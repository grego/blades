// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use blades::*;

use beef::lean::Cow;
use ramhorns::{Content, Ramhorns, Template};
use serde::Deserialize;
use serde_cmd::CmdBorrowed;

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, stdin, stdout, BufRead, BufReader, BufWriter, ErrorKind, Lines, Write};
use std::path::{self, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Instant, SystemTime};
use std::{cmp, thread};
use thiserror::Error;

static HELP: &str = r#"Blazing fast dead simple static site generator

Usage: blades [COMMAND]

Commands:
  init      Initialize the site in the current directory, creating the basic files and folders
  new       Create a new page
  build     Build the site according to config, content, templates and themes in the current directory
  colocate  Move assets from the "assets" directory and from the theme, if one is used, into the output directory
  all       Build the site and colocate the assets
  lazy      Build the site and (colocate assets only if the theme was switched) [default]
  help      Print this message
  version   Print version information

Environment variables:
  BLADES_CONFIG   File to read the site config from [default: Blades.toml]
"#;
static VAR_CONFIG: &str = "BLADES_CONFIG";
static CONFIG_FILE: &str = "Blades.toml";

const BUFFER_SIZE: usize = 16384;

#[derive(PartialEq, Eq)]
enum Cmd {
    Init,
    New,
    Build,
    Colocate,
    All,
    Lazy,
    Help,
    Version,
    Invalid,
}

/// Main configuration where all the site settings are set.
/// Blades deserializes it from a given TOML file.
#[derive(Default, Deserialize)]
struct Config<'c> {
    /// The directory of the content
    #[serde(borrow, default = "default_content_dir")]
    content_dir: Cow<'c, str>,
    /// The directory where the output should be rendered to
    #[serde(borrow, default = "default_output_dir")]
    output_dir: Cow<'c, str>,
    /// The directory where the themes are
    #[serde(borrow, default = "default_theme_dir")]
    theme_dir: Cow<'c, str>,
    /// Name of the directory of a theme this site is using, empty if none.
    #[serde(borrow, default)]
    theme: Cow<'c, str>,
    /// Taxonomies of the site
    #[serde(default)]
    taxonomies: HashMap<&'c str, TaxonMeta<'c>>,
    /// Generate taxonomies not specified in the config?
    #[serde(default = "default_true")]
    implicit_taxonomies: bool,

    /// Information about the site usable in templates
    #[serde(flatten)]
    site: Site<'c>,

    /// Configuration of plugins for building the site.
    #[serde(default)]
    plugins: Plugins<'c>,
}

/// Plugins to use when building the site.
#[derive(Default, Deserialize)]
struct Plugins<'p> {
    /// Plugins to get the input from, in the form of serialized list of pages.
    #[serde(borrow, default)]
    input: Box<[CmdBorrowed<'p>]>,
    /// Plugins that transform the serialized list of pages.
    #[serde(borrow, default)]
    transform: Box<[CmdBorrowed<'p>]>,
    /// Plugins that get the serialized list of pages and might do something with it.
    #[serde(borrow, default)]
    output: Box<[CmdBorrowed<'p>]>,
    /// Plugins that transform the content of pages.
    /// They are identified by their name and must be enabled for each page.
    #[serde(borrow, default)]
    content: HashMap<&'p str, CmdBorrowed<'p>>,
    /// A list of names of content plugins that should be applied to every page.
    #[serde(default)]
    default: Box<[&'p str]>,
}

#[inline]
const fn default_content_dir() -> Cow<'static, str> {
    Cow::const_str("content")
}

#[inline]
const fn default_output_dir() -> Cow<'static, str> {
    Cow::const_str("public")
}

#[inline]
const fn default_theme_dir() -> Cow<'static, str> {
    Cow::const_str("themes")
}

#[inline]
const fn default_true() -> bool {
    true
}

/// Where the templates are located, relative to the site directrory.
static TEMPLATE_DIR: &str = "templates";
/// Where the assets will be copied from, relative to the site directrory.
static ASSET_SRC_DIR: &str = "assets";
static FILELIST: &str = ".blades";
static OLD_THEME: &str = ".bladestheme";

#[derive(Content)]
struct MockConfig {
    title: String,
    author: String,
}

#[derive(Content)]
struct MockPage {
    title: String,
    slug: String,
    date: String,
}

/// Possible formats of the source.
enum Format {
    Toml,
    Markdown,
}

#[derive(Debug, Error)]
enum ParseError {
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Invalid UTF8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

#[derive(Debug, Error)]
enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Template error: {0}")]
    Ramhorns(#[from] ramhorns::Error),
    #[error("Error parsing {1}: {0}")]
    Parse(ParseError, Box<str>),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Error in plugin {0}: {1}")]
    Plugin(Box<str>, Box<str>),
    #[error("Plugin {0} returned invalid UTF8 data: {1}")]
    Utf8(Box<str>, std::string::FromUtf8Error),
}

impl From<(ParseError, Box<str>)> for Error {
    fn from((e, s): (ParseError, Box<str>)) -> Self {
        Self::Parse(e, s)
    }
}

/// A helper trait to simplify the command logic.
trait OutputResult {
    fn output_result(self, name: &str) -> Result<Vec<u8>, Error>;
}

impl OutputResult for Output {
    fn output_result(self, name: &str) -> Result<Vec<u8>, Error> {
        if self.status.success() {
            Ok(self.stdout)
        } else {
            Err(Error::Plugin(
                name.into(),
                String::from_utf8_lossy(&self.stderr).into(),
            ))
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Format::Toml
    }
}

impl Parser for Format {
    type Error = ParseError;

    /// The kind of parser that should be used, based on the file extension.
    fn from_extension(ext: &OsStr) -> Option<Self> {
        if ext == "toml" {
            Some(Format::Toml)
        } else if ext == "md" {
            Some(Format::Markdown)
        } else {
            None
        }
    }

    /// Parse the binary data into a Page.
    fn parse<'a>(&self, data: &'a [u8]) -> Result<Page<'a>, Self::Error> {
        Ok(match self {
            Format::Toml => toml::from_slice(data)?,
            Format::Markdown => {
                let (header, content) = separate_md_header(data);
                let mut page: Page = toml::from_slice(header)?;
                let content = std::str::from_utf8(content)?;
                page.content = content.trim().into();
                page
            }
        })
    }
}

/// Separate a TOML header in `+++` from the markdown file.
#[inline]
fn separate_md_header(source: &[u8]) -> (&[u8], &[u8]) {
    if source.len() < 4 || &source[..3] != b"+++" {
        return (&[], source);
    }

    enum State {
        None,
        Quote(u8),
    }
    let mut state = State::None;
    for (len, w) in source
        .windows(3)
        .map(|w| [w[0], w[1], w[2]])
        .enumerate()
        .skip(3)
    {
        if (w[1] == b'"' || w[1] == b'\'') && w[0] != b'\\' {
            state = match state {
                State::None => State::Quote(w[1]),
                State::Quote(q) if q == w[1] => State::None,
                State::Quote(r) => State::Quote(r),
            }
        } else if let State::None = state {
            if w == [b'+', b'+', b'+'] {
                if source.len() <= len + 3 {
                    return (&source[3..len], &[]);
                } else {
                    return (&source[3..len], &source[len + 3..]);
                }
            }
        }
    }
    (&source[3..], &[])
}

trait Unwind {
    type Value;

    fn unwind(self) -> Self::Value;
}

impl<T> Unwind for thread::Result<T> {
    type Value = T;

    fn unwind(self) -> T {
        match self {
            Ok(t) => t,
            Err(e) => std::panic::resume_unwind(e),
        }
    }
}

/// Get the next line from the standard input after displaying some message
fn next_line<B: BufRead>(lines: &mut Lines<B>, message: &str) -> Result<String, std::io::Error> {
    print!("{} ", message);
    stdout().flush()?;
    lines.next().transpose().map(|s| s.unwrap_or_default())
}

/// Initialise the site
fn init() -> Result<(), Error> {
    println!("Enter the basic site info");
    let mut lines = BufReader::new(stdin().lock()).lines();
    let title = next_line(&mut lines, "Name:")?;
    let author = next_line(&mut lines, "Author:")?;
    let config = MockConfig { title, author };
    Template::new(include_str!("templates/Blades.toml"))?.render_to_file(CONFIG_FILE, &config)?;
    fs::create_dir_all("content")?;
    fs::create_dir_all("themes").map_err(Into::into)
}

/// Create a new page and edit it if the EDITOR variable is set
fn new_page(config: &Config) -> Result<(), Error> {
    println!("Enter the basic info of the new page");
    let mut lines = BufReader::new(stdin().lock()).lines();
    let title = next_line(&mut lines, "Title:")?;
    let slug = next_line(&mut lines, "Slug (short name in the URL):")?;
    let mut path = Path::new(config.content_dir.as_ref()).join(next_line(
        &mut lines,
        "Path (relative to the content directory):",
    )?);
    fs::create_dir_all(&path)?;

    let date: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
    let date = date.format("%Y-%m-%d").to_string();
    path.push(format!("{}-{}.toml", &date, &slug));

    if path.exists() {
        let mut answer = next_line(
            &mut lines,
            &format!(
                "The path {:?} already exists, do you want to overwrite in? [y/N]",
                &path
            ),
        )?;
        answer.make_ascii_lowercase();
        if answer != "y" || answer != "yes" {
            return Ok(());
        }
    }
    let page = MockPage { title, slug, date };
    Template::new(include_str!("templates/page.toml"))?.render_to_file(&path, &page)?;
    println!("{:?} created", &path);

    if let Ok(editor) = env::var("EDITOR") {
        Command::new(editor).arg(&path).status()?;
    } else {
        println!("Set the EDITOR environment variable to edit new pages immediately");
    }
    Ok(())
}

fn copy_dir(src: &mut PathBuf, dest: &mut PathBuf) -> Result<(), io::Error> {
    let iter = match fs::read_dir(&src) {
        Ok(iter) => iter,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    fs::create_dir_all(&dest)?;
    for entry in iter.filter_map(Result::ok) {
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        src.push(&file_name);
        dest.push(&file_name);
        if file_type.is_file() {
            fs::copy(&src, &dest)?;
        } else if file_type.is_dir() {
            copy_dir(src, dest)?;
        }
        src.pop();
        dest.pop();
    }
    Ok(())
}

/// Place assets located in the `assets` directory or in the `assets` subdirectory of the theme,
/// if used, into a dedicated subdirectory of the output directory specified in the config
/// (defaults to `assets`, too).
fn colocate_assets(config: &Config) -> Result<(), io::Error> {
    let mut output = Path::new(config.output_dir.as_ref()).join(config.site.assets.as_ref());
    match fs::remove_dir_all(&output) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }?;
    let mut src = PathBuf::with_capacity(64);
    if !config.theme.is_empty() {
        src.push(config.theme_dir.as_ref());
        src.push(config.theme.as_ref());
        src.push(ASSET_SRC_DIR);
        copy_dir(&mut src, &mut output)?;
        src.clear();
    }
    src.push(ASSET_SRC_DIR);
    copy_dir(&mut src, &mut output)
}

/// Load the templates from the directories specified by the config.
fn load_templates(config: &Config) -> Result<Ramhorns, ramhorns::Error> {
    fs::create_dir_all(TEMPLATE_DIR)?;
    let mut templates = Ramhorns::from_folder(TEMPLATE_DIR)?;
    if !config.theme.is_empty() {
        let mut theme_path = Path::new(config.theme_dir.as_ref()).join(config.theme.as_ref());
        theme_path.push(TEMPLATE_DIR);
        if theme_path.exists() {
            templates.extend_from_folder(theme_path)?;
        }
    }
    Ok(templates)
}

/// Delete all the pages that were present in the previous render, but not the current one.
/// Then, write all the paths that were rendered to the file `filelist`
fn cleanup(mut rendered: HashMap<PathBuf, u32>, filelist: &str) -> Result<(), io::Error> {
    if let Ok(f) = File::open(filelist) {
        BufReader::new(f).lines().try_for_each(|filename| {
            let filename = filename?;
            if !rendered.contains_key(Path::new(&filename)) {
                // Every directory has its index rendered
                if let Some(dir) = filename.strip_suffix("index.html") {
                    if dir.ends_with(path::is_separator) {
                        return match fs::remove_dir_all(dir) {
                            Ok(_) => Ok(()),
                            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                            Err(e) => Err(e),
                        };
                    }
                }
                match fs::remove_file(&filename) {
                    Ok(_) => Ok(()),
                    Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(e),
                }
            } else {
                Ok(())
            }
        })?;
    };

    let f = File::create(filelist)?;
    let mut f = BufWriter::new(f);
    for (path, count) in rendered.drain() {
        // It was already checked that the paths contain valid UTF-8
        let path = path.into_os_string().into_string().unwrap();
        writeln!(&mut f, "{}", path)?;
        if count > 1 {
            println!("{} paths render to {}", count, path);
        }
    }

    Ok(())
}

/// The actual logic of task parallelisation.
fn build(config: &Config) -> Result<(), Error> {
    const MIN_PER_THREAD: usize = 5;

    let sources: Sources<Format> = Sources::load(config.content_dir.as_ref())?;
    let num_pages = sources.sources().len();
    let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let num_threads = cmp::max(num_threads - 1, num_pages / MIN_PER_THREAD);
    let per_thread = (num_pages / num_threads) + 1;

    let (templates, pages) = thread::scope(|s| {
        let mut threads = Vec::with_capacity(num_threads);
        for chunk in sources.sources().chunks(per_thread) {
            threads.push(s.spawn(|| {
                chunk
                    .iter()
                    .map(|src| Page::new(src, &sources))
                    .collect::<Result<Vec<_>, _>>()
            }));
        }
        let templates = load_templates(config)?;
        let mut pages = Vec::with_capacity(num_pages);
        for thread in threads.drain(..) {
            pages.append(&mut thread.join().unwind()?);
        }
        Ok::<_, Error>((templates, pages))
    })?;

    // Input plugins
    // Store input pages separately, so that we can borrow from the data
    let inputs = config
        .plugins
        .input
        .iter()
        .map(|cmd| cmd.make_command().output()?.output_result(&cmd.path))
        .collect::<Result<Vec<_>, _>>()?;
    let input_pages = inputs
        .iter()
        .map(|input| serde_json::from_slice(input))
        .collect::<Result<Vec<Vec<Page>>, _>>()?;
    let mut pages = pages;
    pages.extend(input_pages.into_iter().flat_map(|ip| ip.into_iter()));

    // Transform plugins
    let mut transformed: Option<Vec<u8>> = None;
    for cmd in config.plugins.transform.iter() {
        let mut child = cmd
            .make_command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().expect("Failed to open child stdin");
        if let Some(ref source) = transformed {
            stdin.write_all(source)?;
        } else {
            serde_json::to_writer(&stdin, &pages)?;
        }
        drop(stdin);
        let output = child.wait_with_output()?.output_result(&cmd.path)?;
        transformed = Some(output);
    }
    let mut pages = pages;
    if let Some(ref source) = transformed {
        pages = serde_json::from_slice(source)?;
    }

    // Content plugins
    if !config.plugins.content.is_empty() {
        pages.iter_mut().try_for_each(|page| {
            let mut output: Option<String> = None;
            for &cmd in config.plugins.default.iter().chain(page.plugins.iter()) {
                let mut child = config.plugins.content[cmd]
                    .make_command()
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                let mut stdin = child.stdin.take().expect("Failed to open child stdin");
                if let Some(ref out) = output {
                    stdin.write_all(out.as_ref())?;
                } else {
                    stdin.write_all(page.content.as_ref().as_ref())?;
                }
                drop(stdin);
                let out = child.wait_with_output()?.output_result(cmd)?;
                output = Some(String::from_utf8(out).map_err(|e| Error::Utf8(cmd.into(), e))?);
            }
            if let Some(out) = output {
                page.content = out.into();
            }
            Ok::<_, Error>(())
        })?;
    }

    let pages = if !inputs.is_empty() || transformed.is_some() {
        pages.sort_unstable();
        Pages::from_external(pages)
    } else {
        Pages::from_sources(pages)
    };

    for page in pages.iter() {
        page.create_directory(config.output_dir.as_ref())?;
    }

    let taxonomies = Taxonomy::classify(
        &pages,
        config.taxonomies.iter(),
        &config.site.url,
        config.implicit_taxonomies,
    );

    let output_dir = config.output_dir.as_ref().as_ref();
    let context = Context(&pages, &config.site, &taxonomies, &templates, output_dir);
    let rendered = thread::scope(|s| {
        let mut threads = Vec::with_capacity(num_threads);
        for chunk in pages.chunks(per_thread) {
            threads.push(s.spawn(|| {
                let mut rendered = HashMap::default();
                let mut buffer = Vec::with_capacity(BUFFER_SIZE);
                for page in chunk.iter() {
                    page.render(context, &mut rendered, &mut buffer)?;
                }
                Ok::<_, Error>(rendered)
            }));
        }

        let mut rendered = HashMap::default();
        let mut buffer = Vec::with_capacity(BUFFER_SIZE);
        for (_, taxonomy) in taxonomies.iter() {
            taxonomy.render(context, &mut rendered, &mut buffer)?;
            for (n, l) in taxonomy.keys().iter() {
                taxonomy.render_key(n, l, context, &mut rendered, &mut buffer)?;
            }
        }
        render_meta(&pages, &config.site, &taxonomies, output_dir, &mut buffer)?;

        for thread in threads.drain(..) {
            let mut other = thread.join().unwind()?;
            for (path, count) in other.drain() {
                rendered
                    .entry(path)
                    .and_modify(|c| *c += count)
                    .or_default();
            }
        }
        Ok::<_, Error>(rendered)
    })?;

    cleanup(rendered, FILELIST)?;

    // Output plugins
    if !config.plugins.output.is_empty() {
        let pagedata = serde_json::to_string(&pages)?;
        for cmd in config.plugins.output.iter() {
            let mut child = cmd
                .make_command()
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            let mut stdin = child.stdin.take().expect("Failed to open child stdin");
            stdin.write_all(pagedata.as_ref())?;
            drop(stdin);
            child
                .wait_with_output()?
                .output_result(&cmd.path)
                .map(drop)?;
        }
    }
    Ok(())
}

fn get_command() -> Cmd {
    let mut args = env::args().skip(1);
    let command = match args.next().as_deref() {
        Some("init") => Cmd::Init,
        Some("new") => Cmd::New,
        Some("build") => Cmd::Build,
        Some("colocate") => Cmd::Colocate,
        Some("all") => Cmd::All,
        Some("lazy") | None => Cmd::Lazy,
        Some("help") => Cmd::Help,
        Some("version") => Cmd::Version,
        _ => Cmd::Invalid,
    };
    if args.next().is_some() {
        Cmd::Invalid
    } else {
        command
    }
}

fn main() {
    let cmd = get_command();
    let config_name: Cow<str> = env::var(VAR_CONFIG)
        .map(Into::into)
        .unwrap_or_else(|_| CONFIG_FILE.into());

    let start = Instant::now();

    match cmd {
        Cmd::Help => {
            print!("{}", HELP);
            return;
        }
        Cmd::Version => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return;
        }
        Cmd::Invalid => {
            eprintln!("Error: invalid arguments");
            print!("{}", HELP);
            return;
        }
        _ => {}
    }

    let config_file = match std::fs::read_to_string(config_name.as_ref()) {
        Ok(cf) => cf,
        // Don't need a config file for initialisation.
        Err(_) if cmd == Cmd::Init => "".to_string(),
        Err(e) => {
            eprintln!("Can't read {}: {}", config_name, e);
            return;
        }
    };
    let config: Config = match toml::from_str(&config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error parsing config file {}: {}", config_name, e);
            return;
        }
    };

    if let Err(e) = match cmd {
        Cmd::Init => {
            if config_file.is_empty() {
                init()
            } else {
                println!("Config file {} already present; exiting", &config_name);
                Ok(())
            }
        }
        Cmd::New => new_page(&config),
        Cmd::Build => build(&config),
        Cmd::Colocate => colocate_assets(&config).map_err(Into::into),
        Cmd::All => build(&config).and_then(|_| colocate_assets(&config).map_err(Into::into)),
        Cmd::Lazy => build(&config).and_then(|_| {
            if fs::read_to_string(OLD_THEME)
                .map(|old| old != config.theme)
                .unwrap_or(true)
            {
                colocate_assets(&config)?;
                fs::write(OLD_THEME, config.theme.as_ref()).map_err(Into::into)
            } else {
                Ok(())
            }
        }),
        _ => {
            unreachable!()
        }
    } {
        eprintln!("{}", e);
        return;
    }

    println!("Done in {}ms.", start.elapsed().as_micros() as f64 / 1000.0)
}
