// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use blades::*;

use chrono::offset::Local;
use ramhorns::{Content, Template};
use rayon::prelude::*;
use std::env::var;
use std::fs::{create_dir_all, read_to_string, write};
use std::io::{stdin, stdout, BufRead, BufReader, Lines, Write};
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use structopt::StructOpt;

static CONFIG_FILE: &str = "Blades.toml";

#[derive(StructOpt)]
/// Blazing fast Dead simple Static site generator
struct Opt {
    /// File to read the site config from
    #[structopt(short, long, default_value = CONFIG_FILE)]
    config: String,
    #[structopt(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(PartialEq, StructOpt)]
enum Cmd {
    /// Initialise the site in the current directory, creating the basic files and folders
    Init,
    /// Start creating a new page
    New,
    /// Build the site according to config, content, templates and themes in the current directory
    Build,
    /// Move the assets from the "assets" directory and from the theme, if one is used,
    /// into the output directory
    Colocate,
    /// Build the site and colocate the assets
    All,
    /// Build the site and (colocate assets only if the theme was switched) [default]
    Lazy,
}

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

/// Get the next line from the standard input after displaying some message
fn next_line<B: BufRead>(lines: &mut Lines<B>, message: &str) -> Result<String, std::io::Error> {
    print!("{} ", message);
    stdout().flush()?;
    lines.next().transpose().map(|s| s.unwrap_or_default())
}

/// Initialise the site
fn init() -> Result<(), Error> {
    println!("Enter the basic site info");
    let stdin = stdin();
    let mut lines = BufReader::new(stdin.lock()).lines();
    let title = next_line(&mut lines, "Name:")?;
    let author = next_line(&mut lines, "Author:")?;
    let config = MockConfig { title, author };
    Template::new(include_str!("templates/Blades.toml"))?.render_to_file(CONFIG_FILE, &config)?;
    write(".watch.toml", include_str!("templates/.watch.toml"))?;
    create_dir_all("content")?;
    create_dir_all("themes").map_err(Into::into)
}

/// Create a new page and edit it if the EDITOR variable is set
fn new_page(config: &Config) -> Result<(), Error> {
    println!("Enter the basic info of the new page");
    let stdin = stdin();
    let mut lines = BufReader::new(stdin.lock()).lines();
    let title = next_line(&mut lines, "Title:")?;
    let slug = next_line(&mut lines, "Slug (short name in the URL):")?;
    let mut path = Path::new(config.content_dir.as_ref()).join(next_line(
        &mut lines,
        "Path (relative to the content directory):",
    )?);
    create_dir_all(&path)?;
    let date = Local::now().format("%Y-%m-%d").to_string();
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

    if let Ok(editor) = var("EDITOR") {
        Command::new(editor).arg(&path).status()?;
    } else {
        println!("Set the EDITOR environment variable to edit new pages immediately");
    }
    Ok(())
}

/// The actual logic of task parallelisation.
/// This is the only place in the crate where Rayon is used.
fn build(config: &Config) -> Result<(), Error> {
    let sources = Sources::load(&config)?;
    let (templates, pages) = rayon::join(
        || Templates::load(&config),
        || {
            sources
                .sources()
                .par_iter()
                .map(|src| Page::new(src, &sources, &config))
                .collect::<Result<Vec<_>, _>>()
                .and_then(|pages| Page::prepare(pages, &config))
        },
    );
    let (templates, pages) = (templates?, pages?);

    let taxonomies = Taxonomy::classify(&pages, &config, &templates)?;

    let rendered = MutSet::default();
    let (res_l, res_r) = rayon::join(
        || {
            pages.par_iter().try_for_each(|page| {
                page.render(&pages, &templates, &config, &taxonomies, &rendered)
            })
        },
        || -> Result<(), Error> {
            taxonomies.par_iter().try_for_each(|(_, taxonomy)| {
                taxonomy.render(&config, &taxonomies, &pages, &rendered)?;
                taxonomy.keys().par_iter().try_for_each(|(name, labeled)| {
                    taxonomy.render_key(name, labeled, &config, &taxonomies, &pages, &rendered)
                })
            })?;
            render_meta(&pages, &taxonomies, &config, &rendered)
        },
    );
    res_l.and(res_r)?;
    cleanup(rendered, FILELIST)
}

fn main() {
    let opt = Opt::from_args();
    let start = Instant::now();

    let config_file = match std::fs::read_to_string(&opt.config) {
        Ok(cf) => cf,
        // Don't need a config file for initialisation.
        Err(_) if opt.cmd == Some(Cmd::Init) => "".to_string(),
        Err(e) => {
            eprintln!("Can't read {}: {}", &opt.config, e);
            return;
        }
    };
    let config: Config = match toml::from_str(&config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error parsing {}: {}", &opt.config, e);
            return;
        }
    };

    if let Err(e) = match opt.cmd {
        Some(Cmd::Init) => {
            if config_file.is_empty() {
                init()
            } else {
                println!("Config file {} already present; exiting", &opt.config);
                Ok(())
            }
        }
        Some(Cmd::New) => new_page(&config),
        Some(Cmd::Build) => build(&config),
        Some(Cmd::Colocate) => colocate_assets(&config),
        Some(Cmd::All) => build(&config).and_then(|_| colocate_assets(&config)),
        Some(Cmd::Lazy) | None => build(&config).and_then(|_| {
            if read_to_string(OLD_THEME)
                .map(|old| old != config.theme)
                .unwrap_or(true)
            {
                colocate_assets(&config)?;
                write(OLD_THEME, config.theme.as_ref()).map_err(Into::into)
            } else {
                Ok(())
            }
        }),
    } {
        eprintln!("{}", e)
    } else if !(opt.cmd == Some(Cmd::Init) || opt.cmd == Some(Cmd::New)) {
        println!("Done in {}ms.", start.elapsed().as_micros() as f64 / 1000.0)
    }
}
