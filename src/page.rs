// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>

use crate::config::Config;
use crate::error::Result;
use crate::sources::{Source, Sources};
use crate::tasks::render;
use crate::taxonomies::{Classification, Taxonomies};
use crate::types::{Ancestors, Any, DateTime, HashMap, MutSet, Templates};

use beef::lean::Cow;
use ramhorns::{encoding::Encoder, traits::ContentSequence, Content, Section, Template};
use serde::Deserialize;

use std::cmp::{min, Reverse};
use std::fs::create_dir_all;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::{is_separator, Path, PathBuf};

/// All the information regarding one page
#[derive(Content, Deserialize)]
pub struct Page<'p> {
    #[serde(borrow, default)]
    title: Cow<'p, str>,
    #[serde(borrow, default)]
    slug: Cow<'p, str>,
    #[serde(borrow, default)]
    path: Ancestors<'p>,
    #[serde(default)]
    alternative_paths: Cow<'p, [&'p str]>,
    #[serde(default)]
    #[ramhorns(skip)]
    pub(crate) weight: i64,
    #[serde(borrow, default)]
    #[ramhorns(skip)]
    template: Cow<'p, str>,
    #[serde(borrow, default = "default_page")]
    #[ramhorns(skip)]
    page_template: Cow<'p, str>,
    #[serde(borrow, default = "default_section")]
    #[ramhorns(skip)]
    section_template: Cow<'p, str>,
    #[serde(borrow, default = "default_gallery")]
    #[ramhorns(skip)]
    gallery_template: Cow<'p, str>,
    #[ramhorns(skip)]
    paginate_by: Option<NonZeroUsize>,
    #[serde(default)]
    #[ramhorns(skip)]
    pictures: Cow<'p, [Picture<'p>]>,
    #[serde(borrow, default)]
    image: Cow<'p, str>,
    #[serde(borrow, default)]
    summary: Cow<'p, str>,
    #[serde(borrow, default)]
    #[md]
    content: Cow<'p, str>,

    pub(crate) date: Option<DateTime>,

    #[serde(default)]
    #[ramhorns(skip)]
    sort_by_weight: bool,
    #[serde(skip)]
    is_section: bool,
    #[serde(default)]
    hidden: bool,

    #[serde(skip, default = "default_range")]
    #[ramhorns(skip)]
    pages: Range<usize>,
    #[serde(skip, default = "default_range")]
    #[ramhorns(skip)]
    subsections: Range<usize>,
    #[serde(skip)]
    #[ramhorns(skip)]
    parent: usize,
    #[serde(skip)]
    #[ramhorns(skip)]
    previous: usize,
    #[serde(skip)]
    #[ramhorns(skip)]
    next: usize,
    /// Priority of this page in the sitemap
    #[serde(skip, default = "default_priority")]
    priority: f32,

    #[serde(default)]
    pub(crate) taxonomies: Taxonomies<'p>,
    #[serde(default)]
    extra: HashMap<&'p str, Any<'p>>,

    /// A unique number to determine whether this is the active page
    #[serde(skip)]
    #[ramhorns(skip)]
    id: usize,
}

#[derive(Clone, Content, Deserialize)]
pub(crate) struct Picture<'p> {
    #[serde(borrow, default)]
    alt: Cow<'p, str>,
    #[serde(borrow, default)]
    #[md]
    caption: Cow<'p, str>,
    #[serde(borrow)]
    file: Cow<'p, str>,
    #[serde(borrow)]
    pid: Cow<'p, str>,
    taken: Option<DateTime>,
}

/// Page bundled with references to its subpages and subsections for rendering
#[derive(Clone, Content)]
pub(crate) struct PageRef<'p, 'r> {
    pages: Pages<'p, 'r>,
    subsections: Pages<'p, 'r>,
    pictures: Pictures<'p, 'r>,
    permalink: Permalink<'p, 'r>,
    active: bool,
    #[ramhorns(flatten)]
    page: &'r Page<'p>,
}

/// Page bundled with the context of the whole site for rendering
#[derive(Clone, Content)]
struct PageContext<'p, 'r> {
    pages: Pages<'p, 'r>,
    subsections: Pages<'p, 'r>,
    previous: Option<PageRef<'p, 'r>>,
    next: Option<PageRef<'p, 'r>>,
    parent: PageRef<'p, 'r>,
    pictures: Pictures<'p, 'r>,
    index: PageRef<'p, 'r>,
    pagination: Option<Pagination>,
    permalink: Permalink<'p, 'r>,
    site: &'r Config<'p>,
    classification: &'r Classification<'p, 'r>,
    /// Always true, because this is the current page
    active: Active,
    #[ramhorns(flatten)]
    page: &'r Page<'p>,
}

/// (all pages, range we are interested in, id of the active page)
/// Uses special Content implementation to render the given range of pages in context.
#[derive(Clone)]
pub(crate) struct Pages<'p, 'r> {
    all: &'r [Page<'p>],
    range: Range<usize>,
    active: usize,
    site_url: &'r str,
}

/// Information about the current position in pagination
#[derive(Clone, Content, Copy)]
pub(crate) struct Pagination {
    previous: Option<usize>,
    next: Option<usize>,
    current: usize,
    length: usize,
}

/// A view of one picture on some page
#[derive(Content)]
struct PictureRef<'p, 'r> {
    #[ramhorns(flatten)]
    picture: Picture<'r>,
    permalink: PicturePermalink<'p, 'r>,
}

/// One picture bundled with the context of the whole site for rendering
#[derive(Content)]
struct PictureView<'p, 'r> {
    current: PictureRef<'p, 'r>,
    previous: PictureRef<'p, 'r>,
    next: PictureRef<'p, 'r>,
    parent: PageRef<'p, 'r>,
    index: PageRef<'p, 'r>,
    site: &'r Config<'p>,
    classification: &'r Classification<'p, 'r>,
}

/// A list of pictures
/// The last str is the site URL, kept for generating permalinks
#[derive(Clone)]
struct Pictures<'p, 'r>(&'r [Picture<'p>], &'r Page<'p>, &'r str);

#[derive(Clone)]
struct Active;
impl Content for Active {}

/// A struct to generate the full link for the given page
// (page, site_url)
#[derive(Clone)]
pub struct Permalink<'p, 'r>(pub(crate) &'r Page<'p>, pub(crate) &'r str);

/// A struct to generate the full link for the given page
/// (page, site_url, pid)
struct PicturePermalink<'p, 'r>(&'r Page<'p>, &'r str, &'r str);

/// Trait representing types that can be rendered with some of their subpages separately
pub(crate) trait Paginate: Content + Sized {
    /// Return `self`, but only with pages in the given range.
    fn paginate(&self, pages: Range<usize>, length: usize, current: usize) -> Self;

    /// Render `self` into separate pages where each can view just a subslice of `self`'s subpages.
    fn render_paginated(
        &self,
        mut first: usize,
        last: usize,
        by: usize,
        path: &mut PathBuf,
        tpl: &Template,
        rendered: &MutSet,
    ) -> Result<()> {
        let count = last - first;
        let by = min(by, count);
        let len = count / by + ((count % by != 0) as usize);
        render(
            tpl,
            &path,
            &self.paginate(first..(first + by), len, 1),
            rendered,
        )?;
        for i in 0..len {
            path.pop();
            path.push((i + 1).to_string());
            path.set_extension("html");
            let end = min(first + by, last);
            render(tpl, &path, &self.paginate(first..end, len, i + 1), rendered)?;
            first = end;
        }
        Ok(())
    }
}

impl<'p> Page<'p> {
    /// Construct a new page from the source.
    #[inline]
    pub fn new(source: &'p Source, data: &'p Sources, config: &Config) -> Result<Self> {
        let mut page: Page = toml::from_slice(&data.data[source.source.clone()])
            .map_err(|e| (e, source.path.clone()))?;

        let is_section = source.is_section;
        page.is_section = is_section;
        page.pages = source.pages.clone();
        page.subsections = source.subsections.clone();
        page.parent = source.parent;
        if config.dates_of_creation {
            page.date = page.date.or_else(|| source.date.map(|d| d.into()));
        }

        let path = &source.path;
        let path = path
            .strip_prefix(config.content_dir.as_ref())
            .unwrap_or(path);
        let path = path.strip_prefix(is_separator).unwrap_or(path);
        if is_section || page.slug.is_empty() || page.slug.contains(is_separator) {
            let slug = path.rsplit(is_separator).next().unwrap_or_default();
            let slug = slug.strip_suffix(".toml").unwrap_or(slug);
            page.slug = Cow::const_str(slug);
        }
        let page_path = page.path.as_ref();
        if is_section || page_path.is_empty() || Path::new(page_path).is_absolute() {
            let path = &path[0..path.rfind(is_separator).unwrap_or_default()];
            page.path = Cow::const_str(path).into();
        } else if page_path == "." {
            page.path = Cow::const_str("").into();
        }

        Ok(page)
    }

    /// Appropriately sort the given vector of pages and create all the directories where
    /// they will be rendered to.
    #[inline]
    pub fn prepare(mut pages: Vec<Self>, config: &Config) -> Result<Vec<Self>> {
        let output_dir = Path::new(config.output_dir.as_ref());
        for i in 0..pages.len() {
            let page = &pages[i];

            let subpages = page.pages.clone();
            let subsections = page.subsections.clone();
            if page.sort_by_weight {
                pages[subpages.clone()].sort_unstable_by_key(|p| p.weight);
                pages[subsections.clone()].sort_unstable_by_key(|p| p.weight);
            } else {
                pages[subpages.clone()].sort_unstable_by_key(|p| Reverse(p.date));
                pages[subsections.clone()].sort_unstable_by_key(|p| Reverse(p.date));
            }

            for i in subpages.clone().skip(1) {
                pages[i].previous = i - 1;
            }
            for i in subpages.clone().take_while(|i| *i != subpages.end - 1) {
                pages[i].next = i + 1;
            }
            for i in subsections.clone().skip(1) {
                pages[i].previous = i - 1;
            }
            for i in subsections
                .clone()
                .take_while(|i| *i != subsections.end - 1)
            {
                pages[i].next = i + 1;
            }

            let page = &pages[i];
            if page.is_section || !page.pictures.is_empty() {
                let mut path = output_dir.join(page.path.as_ref());
                path.push(page.slug.as_ref());
                create_dir_all(path)?;
            }

            for path in page.alternative_paths.iter() {
                let path = output_dir.join(path);
                create_dir_all(path)?;
            }

            // Assign a unique identifier
            pages[i].id = i;
        }

        Ok(pages)
    }

    /// Get a reference of the page, in context of its subpages and subsections.
    #[inline]
    pub(crate) fn by_ref<'r>(&'r self, all: &'r [Self], i: usize, url: &'r str) -> PageRef<'p, 'r> {
        PageRef {
            pages: Pages::new(all, self.pages.clone(), i, url),
            subsections: Pages::new(all, self.subsections.clone(), i, url),
            pictures: Pictures(&self.pictures, self, url),
            page: self,
            permalink: Permalink(self, url),
            active: self.id == i,
        }
    }

    /// Get a reference of the page bundled with the context of the whole site.
    #[inline]
    fn in_context<'r>(
        &'r self,
        all: &'r [Self],
        site: &'r Config<'p>,
        classification: &'r Classification<'p, 'r>,
    ) -> PageContext<'p, 'r> {
        PageContext {
            pages: Pages::new(all, self.pages.clone(), self.id, &site.url),
            subsections: Pages::new(all, self.subsections.clone(), self.id, &site.url),
            previous: Some(self.previous)
                .filter(|&i| i != 0)
                .map(|i| all[i].by_ref(all, self.id, &site.url)),
            next: Some(self.next)
                .filter(|&i| i != 0)
                .map(|i| all[i].by_ref(all, self.id, &site.url)),
            parent: all[self.parent].by_ref(all, self.id, &site.url),
            pictures: Pictures(&self.pictures, self, &site.url),
            permalink: Permalink(self, &site.url),
            index: all[0].by_ref(all, self.id, &site.url),
            pagination: None,
            classification,
            site,
            active: Active,
            page: self,
        }
    }

    /// Render the page to the output directory specified by the config.
    #[inline]
    pub fn render(
        &self,
        all: &[Self],
        templates: &Templates,
        config: &Config<'p>,
        classification: &Classification<'p, '_>,
        rendered: &MutSet,
    ) -> Result {
        let output_dir = Path::new(config.output_dir.as_ref());
        let mut output = output_dir.join(self.path.as_ref());
        output.push(self.slug.as_ref());
        if self.is_section {
            output.push("index");
        }
        output.set_extension("html");

        let template = if self.template.is_empty() {
            if self.is_section {
                &all[self.parent].section_template
            } else {
                &all[self.parent].page_template
            }
        } else {
            &self.template
        };
        let template = templates.get(template)?;

        let page = self.in_context(all, config, classification);
        let by = self.paginate_by.map(NonZeroUsize::get).unwrap_or(0);
        if by > 0 && self.pages.len() > by {
            let (start, end) = (self.pages.start, self.pages.end);
            page.render_paginated(start, end, by, &mut output, &template, rendered)?
        } else if !self.pictures.is_empty() {
            render(template, &output, &page, rendered)?;

            if self.is_section {
                output.pop();
            } else {
                output.set_extension("");
            };

            let template = templates.get(&self.gallery_template)?;
            // Make gallery circular, with the last photo referencing the first and vice-versa
            let pictures = &self.pictures;
            let last = pictures.len() - 1;
            for i in 0..=last {
                let page = PictureView {
                    current: pictures[i].by_ref(self, &config.url),
                    previous: pictures[if i == 0 { last } else { i - 1 }].by_ref(self, &config.url),
                    next: pictures[if i == last { 0 } else { i + 1 }].by_ref(self, &config.url),
                    parent: self.by_ref(all, self.id, &config.url),
                    index: all[0].by_ref(all, self.id, &config.url),
                    site: config,
                    classification,
                };
                output.push(pictures[i].pid.as_ref());
                output.set_extension("html");
                render(template, &output, &page, rendered)?;
                output.pop();
            }
        } else {
            render(template, output, &page, rendered)?;
        }

        for path in self.alternative_paths.iter() {
            let mut output = output_dir.join(path);
            output.push("index.html");
            render(template, output, &page, rendered)?;
        }
        Ok(())
    }
}

impl<'p, 'r> Paginate for PageContext<'p, 'r> {
    #[inline]
    fn paginate(&self, pages: Range<usize>, length: usize, current: usize) -> Self {
        let old = &self.pages;
        Self {
            pages: Pages::new(old.all, pages, old.active, old.site_url),
            pagination: Some(Pagination::new(length, current)),
            ..self.clone()
        }
    }
}

impl Pagination {
    #[inline]
    pub(crate) fn new(length: usize, current: usize) -> Self {
        Self {
            length,
            current,
            previous: Some(current - 1).filter(|&i| i > 0),
            next: Some(current + 1).filter(|&i| i <= length),
        }
    }
}

impl<'p> Picture<'p> {
    fn by_ref<'r>(&'r self, page: &'r Page<'p>, site_url: &'r str) -> PictureRef<'p, 'r> {
        PictureRef {
            picture: Picture {
                alt: self.alt.as_ref().into(),
                caption: self.caption.as_ref().into(),
                pid: self.pid.as_ref().into(),
                file: self.file.as_ref().into(),
                taken: self.taken,
            },
            permalink: PicturePermalink(page, site_url, &self.pid),
        }
    }
}

impl<'p, 'r> Pages<'p, 'r> {
    pub(crate) fn new(all: &'r [Page<'p>], range: Range<usize>, id: usize, url: &'r str) -> Self {
        Self {
            all,
            range,
            active: id,
            site_url: url,
        }
    }
}

impl<'p, 'r> Content for Pages<'p, 'r> {
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.range.is_empty()
    }

    #[inline]
    fn render_section<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        let range = self.range.clone();
        for page in self.all[range].iter().filter(|p| !p.hidden) {
            page.by_ref(self.all, self.active, self.site_url)
                .render_section(section, encoder)?;
        }

        Ok(())
    }
}

impl<'p, 'r> Content for Permalink<'p, 'r> {
    #[inline]
    fn is_truthy(&self) -> bool {
        true
    }

    #[inline]
    fn render_escaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        encoder.write_escaped(self.1)?;
        self.0.path.render_escaped(encoder)?;
        if !self.0.slug.is_empty() {
            encoder.write_unescaped("/")?;
            self.0.slug.render_escaped(encoder)?;
        }
        if self.0.is_section {
            encoder.write_unescaped("/")
        } else {
            encoder.write_unescaped(".html")
        }
    }

    #[inline]
    fn render_unescaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        encoder.write_unescaped(self.1)?;
        self.0.path.render_unescaped(encoder)?;
        if !self.0.slug.is_empty() {
            encoder.write_unescaped("/")?;
            self.0.slug.render_unescaped(encoder)?;
        }
        if self.0.is_section {
            encoder.write_unescaped("/")
        } else {
            encoder.write_unescaped(".html")
        }
    }
}

impl<'p, 'r> Content for Pictures<'p, 'r> {
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.0.is_empty()
    }

    #[inline]
    fn render_section<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        for picture in self.0 {
            picture
                .by_ref(self.1, self.2)
                .render_section(section, encoder)?;
        }

        Ok(())
    }
}

impl<'p, 'r> Content for PicturePermalink<'p, 'r> {
    #[inline]
    fn is_truthy(&self) -> bool {
        true
    }

    #[inline]
    fn render_escaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        encoder.write_escaped(self.1)?;
        self.0.path.render_escaped(encoder)?;
        if !self.0.slug.is_empty() {
            encoder.write_unescaped("/")?;
            self.0.slug.render_escaped(encoder)?;
        }
        encoder.write_unescaped("/")?;
        encoder.write_escaped(self.2)?;
        encoder.write_unescaped(".html")
    }

    #[inline]
    fn render_unescaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        encoder.write_unescaped(self.1)?;
        self.0.path.render_unescaped(encoder)?;
        if !self.0.slug.is_empty() {
            encoder.write_unescaped("/")?;
            self.0.slug.render_unescaped(encoder)?;
        }
        encoder.write_unescaped("/")?;
        encoder.write_unescaped(self.2)?;
        encoder.write_unescaped(".html")
    }
}

#[inline]
const fn default_priority() -> f32 {
    0.5
}

#[inline]
const fn default_range() -> Range<usize> {
    0..0
}

#[inline]
const fn default_gallery() -> Cow<'static, str> {
    Cow::const_str("gallery.html")
}

#[inline]
const fn default_page() -> Cow<'static, str> {
    Cow::const_str("page.html")
}

#[inline]
const fn default_section() -> Cow<'static, str> {
    Cow::const_str("section.html")
}
