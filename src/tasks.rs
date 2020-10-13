// Blades  Copyright (C) 2020  Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>

use crate::config::{Config, ASSET_SRC_DIR};
use crate::error::Result;
use crate::page::{Page, Pages};
use crate::taxonomies::{Classification, TaxonList};
use crate::types::{DateTime, MutSet};

use std::fs::{copy, create_dir_all, read_dir, remove_dir_all, remove_file, File};
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Write};
use std::path::{is_separator, Path, PathBuf};

use ramhorns::{Content, Template};

#[inline]
pub(crate) fn render<P, C>(template: &Template, path: P, content: &C, rendered: &MutSet) -> Result
where
    C: Content,
    P: Into<PathBuf>,
{
    let path = path.into();
    template.render_to_file(&path, content)?;
    if let Some(path) = rendered.lock().replace(path) {
        println!("Warning: more paths render to {}", path.to_string_lossy());
    }
    Ok(())
}

#[derive(Content)]
struct Meta<'p, 'r>(
    #[ramhorns(rename = "date")] DateTime,
    #[ramhorns(rename = "pages")] Pages<'p, 'r>,
    #[ramhorns(rename = "taxons")] TaxonList<'p, 'r>,
    #[ramhorns(rename = "site")] &'r Config<'p>,
);

impl<'p> Meta<'p, '_> {
    #[inline]
    fn render(&self, name: &str, template: &str, path: &Path, rendered: &MutSet) -> Result {
        render(&Template::new(template)?, path.join(name), self, rendered)
    }
}

/// Render sitemap, Atom and RSS feeds if enabled in the config.
pub fn render_meta<'p>(
    pages: &[Page<'p>],
    taxons: &Classification<'p, '_>,
    config: &Config<'p>,
    rendered: &MutSet,
) -> Result {
    let pages = Pages::new(pages, 0..pages.len(), 0, &config.url);
    let meta = Meta(DateTime::now(), pages, TaxonList(taxons), config);
    let path = Path::new(config.output_dir.as_ref());

    if config.sitemap {
        let sitemap = include_str!("templates/sitemap.xml");
        meta.render("sitemap.xml", sitemap, path, rendered)?;
    }
    if config.rss {
        let rss = include_str!("templates/rss.xml");
        meta.render("rss.xml", rss, path, rendered)?;
    }
    if config.atom {
        let atom = include_str!("templates/atom.xml");
        meta.render("atom.xml", atom, path, rendered)?;
    }
    Ok(())
}

fn copy_dir(src: &mut PathBuf, dest: &mut PathBuf) -> Result {
    let iter = match read_dir(&src) {
        Ok(iter) => iter,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    create_dir_all(&dest)?;
    for entry in iter.filter_map(Result::ok) {
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        src.push(&file_name);
        dest.push(&file_name);
        if file_type.is_file() {
            copy(&src, &dest)?;
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
pub fn colocate_assets(config: &Config) -> Result {
    let mut output = Path::new(config.output_dir.as_ref()).join(config.assets.as_ref());
    match remove_dir_all(&output) {
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

/// Delete all the pages that were present in the previous render, but not the current one.
/// Then, write all the paths that were rendered to the file `filelist`
pub fn cleanup(rendered: MutSet, filelist: &str) -> Result {
    let rendered = rendered.into_inner();
    if let Ok(f) = File::open(filelist) {
        BufReader::new(f).lines().try_for_each(|filename| {
            let filename = filename?;
            if !rendered.contains(Path::new(&filename)) {
                // Every directory has its index rendered
                if let Some(dir) = filename.strip_suffix("index.html") {
                    if dir.ends_with(is_separator) {
                        return match remove_dir_all(dir) {
                            Ok(_) => Ok(()),
                            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                            Err(e) => Err(e),
                        };
                    }
                }
                match remove_file(&filename) {
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
    for path in rendered {
        // It was already checked that the paths contain valid UTF-8
        writeln!(&mut f, "{}", path.into_os_string().into_string().unwrap())?;
    }

    Ok(())
}
