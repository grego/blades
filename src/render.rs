// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use crate::page::{Page, PageList};
use crate::site::Site;
use crate::taxonomies::{Classification, TaxonList};
use crate::types::{DateTime, HashMap};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ramhorns::{Content, Template};

#[inline]
pub(crate) fn render<P, C>(
    template: &Template,
    path: P,
    content: &C,
    rendered: &mut HashMap<PathBuf, u32>,
    buffer: &mut Vec<u8>,
) -> Result<(), io::Error>
where
    C: Content,
    P: Into<PathBuf>,
{
    let path = path.into();
    // Can't fail
    let _ = template.render_to_writer(buffer, content);
    fs::write(&path, &buffer)?;
    buffer.clear();
    let count = rendered.entry(path).or_default();
    *count += 1;
    Ok(())
}

#[derive(Content)]
struct Meta<'p, 'r>(
    #[ramhorns(rename = "date")] DateTime,
    #[ramhorns(rename = "pages")] PageList<'p, 'r>,
    #[ramhorns(rename = "taxons")] TaxonList<'p, 'r>,
    #[ramhorns(rename = "site")] &'r Site<'p>,
);

impl<'p> Meta<'p, '_> {
    #[inline]
    fn render(
        &self,
        name: &str,
        template: &str,
        path: &Path,
        buffer: &mut Vec<u8>,
    ) -> Result<(), ramhorns::Error> {
        let _ = Template::new(template)?.render_to_writer(buffer, self);
        fs::write(path.join(name), &buffer)?;
        buffer.clear();
        Ok(())
    }
}

/// Render sitemap, Atom and RSS feeds if enabled in the config.
pub fn render_meta<'p>(
    pages: &[Page<'p>],
    site: &Site<'p>,
    taxons: &Classification<'p, '_>,
    output_dir: &Path,
    buffer: &mut Vec<u8>,
) -> Result<(), ramhorns::Error> {
    let pages = PageList::new(pages, 0..pages.len(), 0, &site.url);
    let meta = Meta(DateTime::now(), pages, TaxonList(taxons), site);

    if site.sitemap {
        let sitemap = include_str!("templates/sitemap.xml");
        meta.render("sitemap.xml", sitemap, output_dir, buffer)?;
    }
    if site.rss {
        let rss = include_str!("templates/rss.xml");
        meta.render("rss.xml", rss, output_dir, buffer)?;
    }
    if site.atom {
        let atom = include_str!("templates/atom.xml");
        meta.render("atom.xml", atom, output_dir, buffer)?;
    }
    Ok(())
}
