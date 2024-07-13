// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use crate::render::render;
use crate::site::{default_true, Site};
use crate::sources::{Parser, Source, Sources};
use crate::taxonomies::{Classification, Taxonomies};
use crate::types::{Ancestors, Any, DateTime, HashMap};

use beef::lean::Cow;
use chrono::NaiveDate;
use ramhorns::{
    encoding::Encoder, traits::ContentSequence, Content, Error, Ramhorns, Section, Template,
};
use serde::{Deserialize, Serialize};

use std::cmp::{min, Ordering, Reverse};
use std::fs::create_dir_all;
use std::io;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::ops::{Deref, DerefMut};
use std::path::{is_separator, Path, PathBuf};

/// All the information regarding one page
#[derive(Content, Default, Deserialize, Serialize)]
pub struct Page<'p> {
    /// Title of the page.
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub title: Cow<'p, str>,
    /// The file name this page is rendered into, without the .html extension.
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub slug: Cow<'p, str>,
    /// The path in the output directory this page is rendered into.
    #[serde(borrow, default, skip_serializing_if = "is_ancestors_empty")]
    pub path: Ancestors<'p>,
    /// A list of alternative paths to render this page in, relative to the output directory.
    #[serde(default, skip_serializing_if = "is_slice_empty")]
    pub alternative_paths: Box<[&'p str]>,
    /// A weight of the page, used if a collection this page is in is sorted by weight.
    #[serde(default, skip_serializing_if = "equal_zero")]
    #[ramhorns(skip)]
    pub weight: i64,
    /// A template to render this page with.
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    #[ramhorns(skip)]
    pub template: Cow<'p, str>,
    /// A template to render every subpage with (unless it specifies another template).
    #[serde(borrow, default = "def_page", skip_serializing_if = "eq_def_page")]
    #[ramhorns(skip)]
    pub page_template: Cow<'p, str>,
    /// A template to render every subsection with (unless it specifies another template).
    #[serde(borrow, default = "def_section", skip_serializing_if = "eq_def_sect")]
    #[ramhorns(skip)]
    pub section_template: Cow<'p, str>,
    /// A template to render the gallery pictures with.
    #[serde(borrow, default = "def_gallery", skip_serializing_if = "eq_def_gall")]
    #[ramhorns(skip)]
    pub gallery_template: Cow<'p, str>,
    /// A number of pages to paginate by, if this number is exceeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ramhorns(skip)]
    pub paginate_by: Option<NonZeroUsize>,
    /// An image representing the page.
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub image: Cow<'p, str>,
    /// A brief summary of the page content.
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub summary: Cow<'p, str>,
    /// The main content of the page.
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    #[ramhorns(callback = render_content)]
    pub content: Cow<'p, str>,

    /// Date when the page was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<DateTime>,

    /// Whether to sort subpages and subsetions by their provided weight.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ramhorns(skip)]
    pub sort_by_weight: bool,
    /// Is this page a section?
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_section: bool,
    /// Hide the page from the list of its parent's subpages or subsections.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,

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
    #[serde(skip, default = "default_true")]
    #[ramhorns(skip)]
    nonstandard_path: bool,
    /// Priority of this page in the sitemap
    #[serde(skip, default = "default_priority")]
    pub priority: f32,

    /// A list of pictures associated with this page.
    #[serde(default, skip_serializing_if = "is_slice_empty")]
    #[ramhorns(skip)]
    pub pictures: Box<[Picture<'p>]>,

    /// A map of lists to classify this page with.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub taxonomies: Taxonomies<'p>,
    /// A list of plugins to use to transform the content of this page.
    #[serde(default, skip_serializing_if = "is_slice_empty")]
    #[ramhorns(skip)]
    pub plugins: Box<[&'p str]>,

    /// A unique number to determine whether this is the active page
    #[serde(skip)]
    #[ramhorns(skip)]
    id: usize,

    /// Any "key = value" of any type can be used here for templates.
    #[serde(flatten)]
    #[ramhorns(flatten)]
    pub extra: HashMap<&'p str, Any<'p>>,
}

/// A list of pages properly sorted and linked within. It dereferences to `[Page]`.
#[derive(Serialize)]
#[serde(transparent)]
pub struct Pages<'p>(Box<[Page<'p>]>);

/// A single picture on a page.
#[derive(Clone, Content, Deserialize, Serialize)]
pub struct Picture<'p> {
    /// An alternative text displayed when the image can't be loaded or for accessibility.
    #[serde(borrow, default)]
    pub alt: Cow<'p, str>,
    /// An associated caption of the picture.
    #[serde(borrow, default)]
    pub caption: Cow<'p, str>,
    /// File name of the image.
    #[serde(borrow)]
    pub file: Cow<'p, str>,
    /// Id string of the picture, used for the generated URL in the gallery page.
    #[serde(borrow)]
    pub pid: Cow<'p, str>,
    /// Date and time of when the image was taken.
    pub taken: Option<DateTime>,
}

/// Whole context for rendering the site
#[derive(Clone, Copy)]
pub struct Context<'p, 'r>(
    pub &'r Pages<'p>,
    pub &'r Site<'p>,
    pub &'r Classification<'p, 'r>,
    pub &'r Ramhorns,
    pub &'r Path,
);

/// Page bundled with references to its subpages and subsections for rendering
#[derive(Clone, Content)]
pub(crate) struct PageRef<'p, 'r> {
    pages: PageList<'p, 'r>,
    subsections: PageList<'p, 'r>,
    pictures: Pictures<'p, 'r>,
    permalink: Permalink<'p, 'r>,
    active: bool,
    #[ramhorns(flatten)]
    page: &'r Page<'p>,
}

/// Page bundled with the context of the whole site for rendering
#[derive(Clone, Content)]
struct PageContext<'p, 'r> {
    pages: PageList<'p, 'r>,
    subsections: PageList<'p, 'r>,
    previous: Option<PageRef<'p, 'r>>,
    next: Option<PageRef<'p, 'r>>,
    parent: PageRef<'p, 'r>,
    pictures: Pictures<'p, 'r>,
    index: PageRef<'p, 'r>,
    pagination: Option<Pagination>,
    permalink: Permalink<'p, 'r>,
    site: &'r Site<'p>,
    classification: &'r Classification<'p, 'r>,
    /// Always true, because this is the current page
    active: Active,
    #[ramhorns(flatten)]
    page: &'r Page<'p>,
}

/// (all pages, range we are interested in, id of the active page)
/// Uses special Content implementation to render the given range of pages in context.
#[derive(Clone)]
pub(crate) struct PageList<'p, 'r> {
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
    site: &'r Site<'p>,
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
        range: Range<usize>,
        by: usize,
        path: &mut PathBuf,
        tpl: &Template,
        rendered: &mut Vec<PathBuf>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), io::Error> {
        let (mut first, last) = (range.start, range.end);
        let count = last - first;
        let by = min(by, count);
        let len = count / by + ((count % by != 0) as usize);
        render(
            tpl,
            &path,
            &self.paginate(first..(first + by), len, 1),
            rendered,
            buffer,
        )?;
        for i in 0..len {
            path.pop();
            path.push((i + 1).to_string());
            path.set_extension("html");
            let end = min(first + by, last);
            render(
                tpl,
                &path,
                &self.paginate(first..end, len, i + 1),
                rendered,
                buffer,
            )?;
            first = end;
        }
        Ok(())
    }
}

impl<'p> Page<'p> {
    /// Construct a new page from the source.
    #[inline]
    pub fn new<P: Parser>(
        source: &'p Source<P>,
        data: &'p Sources<P>,
    ) -> Result<Self, (P::Error, Box<str>)> {
        let path = std::str::from_utf8(&data.data[source.path.clone()]).unwrap();
        let mut page = source
            .format
            .parse(&data.data[source.source.clone()])
            .map_err(|e| (e, path.into()))?;

        let is_section = source.is_section;
        page.is_section = is_section;
        page.pages = source.pages.clone();
        page.subsections = source.subsections.clone();
        page.parent = source.parent;

        let slug = path.rsplit(is_separator).next().unwrap_or_default();
        page.date = page
            .date
            .or_else(|| {
                slug.get(..10).and_then(|p| {
                    p.parse::<NaiveDate>()
                        .map(|d| DateTime(d.and_hms_opt(0, 0, 0).unwrap()))
                        .ok()
                })
            })
            .or_else(|| source.date.map(|d| d.into()));

        if is_section || page.slug.is_empty() || page.slug.contains(is_separator) {
            let slug = path.rsplit(is_separator).next().unwrap_or_default();
            page.slug = Cow::const_str(slug);
        }
        let page_path = page.path.as_ref();
        if is_section || page_path.is_empty() || Path::new(page_path).is_absolute() {
            let path = &path[0..path.rfind(is_separator).unwrap_or_default()];
            page.path = Cow::const_str(path).into();
            page.nonstandard_path = false;
        } else if page_path == "." {
            page.path = Cow::const_str("").into();
        }

        Ok(page)
    }

    /// Get a reference of the page, in context of its subpages and subsections.
    #[inline]
    pub(crate) fn by_ref<'r>(&'r self, all: &'r [Self], i: usize, url: &'r str) -> PageRef<'p, 'r> {
        PageRef {
            pages: PageList::new(all, self.pages.clone(), i, url),
            subsections: PageList::new(all, self.subsections.clone(), i, url),
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
        site: &'r Site<'p>,
        classification: &'r Classification<'p, 'r>,
    ) -> PageContext<'p, 'r> {
        PageContext {
            pages: PageList::new(all, self.pages.clone(), self.id, &site.url),
            subsections: PageList::new(all, self.subsections.clone(), self.id, &site.url),
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

    /// If the page is section, create a directory where it will be rendered to.
    /// Also creates the directories specified in `alternative_paths`.
    pub fn create_directory<P: AsRef<Path>>(&self, output_dir: P) -> Result<(), io::Error> {
        let output_dir = output_dir.as_ref();

        for path in self.alternative_paths.iter() {
            let path = output_dir.join(path);
            create_dir_all(path)?;
        }

        if self.is_section || !self.pictures.is_empty() {
            let mut path = output_dir.join(self.path.as_ref());
            path.push(self.slug.as_ref());
            create_dir_all(path)
        } else if self.nonstandard_path {
            let path = output_dir.join(self.path.as_ref());
            create_dir_all(path)
        } else {
            Ok(())
        }
    }

    /// Render the page to the output directory specified by the config.
    /// `buffer` is used to store the result before writing it to the disk and expected to be empty.
    #[inline]
    pub fn render(
        &self,
        Context(all, site, classification, templates, output_dir): Context<'p, '_>,
        rendered: &mut Vec<PathBuf>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), Error> {
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
        let template = templates
            .get(template)
            .ok_or_else(|| Error::NotFound(template.as_ref().into()))?;

        let page = self.in_context(all, site, classification);
        let by = self.paginate_by.map(NonZeroUsize::get).unwrap_or(0);
        if by > 0 && self.pages.len() > by {
            let (start, end) = (self.pages.start, self.pages.end);
            page.render_paginated(start..end, by, &mut output, template, rendered, buffer)?
        } else if !self.pictures.is_empty() {
            render(template, &output, &page, rendered, buffer)?;

            if self.is_section {
                output.pop();
            } else {
                output.set_extension("");
            };

            let template = templates
                .get(&self.gallery_template)
                .ok_or_else(|| Error::NotFound(self.gallery_template.as_ref().into()))?;

            // Make gallery circular, with the last photo referencing the first and vice-versa
            let pictures = &self.pictures;
            let last = pictures.len() - 1;
            for i in 0..=last {
                let page = PictureView {
                    current: pictures[i].by_ref(self, &site.url),
                    previous: pictures[if i == 0 { last } else { i - 1 }].by_ref(self, &site.url),
                    next: pictures[if i == last { 0 } else { i + 1 }].by_ref(self, &site.url),
                    parent: self.by_ref(all, self.id, &site.url),
                    index: all[0].by_ref(all, self.id, &site.url),
                    site,
                    classification,
                };
                output.push(pictures[i].pid.as_ref());
                output.set_extension("html");
                render(template, &output, &page, rendered, buffer)?;
                output.pop();
            }
        } else {
            render(template, output, &page, rendered, buffer)?;
        }

        for path in self.alternative_paths.iter() {
            let mut output = output_dir.join(path);
            output.push("index.html");
            render(template, output, &page, rendered, buffer)?;
        }
        Ok(())
    }
}

impl<'p> Pages<'p> {
    /// Appropriately sort the given vector of pages and create all the directories where
    /// they will be rendered to.
    /// When applied to pages from external sources, the pages will not have any content
    /// hierarchy (subpages, subsections).
    #[inline]
    pub fn from_sources(mut pages: Vec<Page<'p>>) -> Self {
        for i in 0..pages.len() {
            let page = &pages[i];

            let subpages = page.pages.clone();
            let subsects = page.subsections.clone();
            if page.sort_by_weight {
                pages[subpages.clone()].sort_unstable_by_key(|p| p.weight);
                pages[subsects.clone()].sort_unstable_by_key(|p| p.weight);
            } else {
                pages[subpages.clone()].sort_unstable_by_key(|p| Reverse(p.date));
                pages[subsects.clone()].sort_unstable_by_key(|p| Reverse(p.date));
            }

            for i in subpages.clone().skip(1) {
                pages[i].previous = i - 1;
            }
            for i in subpages.clone().take_while(|i| *i != subpages.end - 1) {
                pages[i].next = i + 1;
            }
            for i in subsects.clone().skip(1) {
                pages[i].previous = i - 1;
            }
            for i in subsects.clone().take_while(|i| *i != subsects.end - 1) {
                pages[i].next = i + 1;
            }

            // Assign a unique identifier
            pages[i].id = i;
        }
        Pages(pages.into())
    }

    /// Build up the internal hierarchical structure of pages loaded from the external source.
    /// The vector of pages MUST be sorted before (using the `Ord` implementation of `Page`),
    /// otherwise the hierarchy will be incomplete.
    #[inline]
    #[allow(clippy::needless_range_loop)]
    pub fn from_external(mut pages: Vec<Page<'p>>) -> Self {
        #[inline]
        fn is_subpage(path: &str, section_path: &str, section_slug: &str) -> bool {
            path.strip_suffix(is_separator)
                .unwrap_or(path)
                .strip_suffix(section_slug)
                .and_then(|p| p.strip_prefix(section_path))
                .filter(|s| s.chars().all(is_separator))
                .is_some()
        }

        // Pages are sorted in a way that makes subpages and subsections adjacent
        for i in 0..pages.len() {
            if !pages[i].is_section {
                continue;
            }

            let (mut found, mut subpage_found) = (false, false);
            // subsections
            let (mut start, mut end) = (0, 0);
            // subpages
            let (mut pstart, mut pend) = (0, 0);
            for j in i + 1..pages.len() {
                if !is_subpage(&pages[j].path.0, &pages[i].path.0, &pages[i].slug) {
                    if !found {
                        continue;
                    } else {
                        if subpage_found {
                            pend = j;
                        } else {
                            end = j;
                        }
                        break;
                    }
                }
                if !found {
                    found = true;
                    if !pages[j].is_section {
                        subpage_found = true;
                        pstart = j;
                    } else {
                        start = j;
                    }
                } else if !subpage_found && !pages[j].is_section {
                    subpage_found = true;
                    pstart = j;
                    end = j;
                }
                pages[j].parent = i;
            }

            if end == 0 && start != 0 {
                end = pages.len()
            } else if pend == 0 && pstart != 0 {
                pend = pages.len()
            }

            pages[i].subsections = start..end;
            pages[i].pages = pstart..pend;
            if pages[i].sort_by_weight {
                pages[start..end].sort_unstable_by_key(|p| p.weight);
                pages[pstart..pend].sort_unstable_by_key(|p| p.weight);
            }
            if end != 0 {
                for j in start + 1..end {
                    pages[j].previous = j - 1;
                }
                for j in start..end - 1 {
                    pages[j].next = j + 1;
                }
            }
            if pend != 0 {
                for j in pstart + 1..pend {
                    pages[j].previous = j - 1;
                }
                for j in pstart..pend - 1 {
                    pages[j].next = j + 1;
                }
            }

            // Assign a unique identifier
            pages[i].id = i;
        }
        Pages(pages.into())
    }
}

impl<'p> Ord for Page<'p> {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut self_paths = self.path.0.split(is_separator);
        let mut other_paths = other.path.0.split(is_separator);
        loop {
            match (self_paths.next(), other_paths.next()) {
                (Some(s1), Some(s2)) => match s1.cmp(s2) {
                    Ordering::Less => return Ordering::Less,
                    Ordering::Greater => return Ordering::Greater,
                    _ => continue,
                },
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (None, None) => break,
            }
        }
        match (self.is_section, other.is_section) {
            (true, true) => self.slug.cmp(&other.slug),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => Reverse(self.date).cmp(&Reverse(other.date)),
        }
    }
}

impl<'p> PartialOrd for Page<'p> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'p> PartialEq for Page<'p> {
    fn eq(&self, other: &Self) -> bool {
        self.path.0 == other.path.0 && self.slug == other.slug && self.id == other.id
    }
}

impl<'p> Eq for Page<'p> {}

impl<'p, 'r> Paginate for PageContext<'p, 'r> {
    #[inline]
    fn paginate(&self, pages: Range<usize>, length: usize, current: usize) -> Self {
        let old = &self.pages;
        Self {
            pages: PageList::new(old.all, pages, old.active, old.site_url),
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

impl<'p, 'r> PageList<'p, 'r> {
    pub(crate) fn new(all: &'r [Page<'p>], range: Range<usize>, id: usize, url: &'r str) -> Self {
        Self {
            all,
            range,
            active: id,
            site_url: url,
        }
    }
}

#[inline]
fn render_content<E: Encoder>(source: &str, encoder: &mut E) -> Result<(), E::Error> {
    use pulldown_cmark::Options;

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM;
    let parser = pulldown_cmark::Parser::new_ext(source, options);
    let processed = cmark_syntax::SyntaxPreprocessor::new(parser);
    encoder.write_html(processed)
}

impl<'p, 'r> Content for PageList<'p, 'r> {
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

impl<'p> Deref for Pages<'p> {
    type Target = [Page<'p>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'p> DerefMut for Pages<'p> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
const fn def_gallery() -> Cow<'static, str> {
    Cow::const_str("gallery.html")
}

#[inline]
const fn def_page() -> Cow<'static, str> {
    Cow::const_str("page.html")
}

#[inline]
const fn def_section() -> Cow<'static, str> {
    Cow::const_str("section.html")
}

#[inline]
fn eq_def_gall(c: &str) -> bool {
    c == "gallery.html"
}

#[inline]
fn eq_def_page(c: &str) -> bool {
    c == "page.html"
}

#[inline]
fn eq_def_sect(c: &str) -> bool {
    c == "section.html"
}

#[inline]
const fn equal_zero(i: &i64) -> bool {
    *i == 0
}

#[inline]
fn is_slice_empty<T>(s: &[T]) -> bool {
    s.is_empty()
}

#[inline]
fn is_ancestors_empty(s: &Ancestors) -> bool {
    s.0.is_empty()
}
