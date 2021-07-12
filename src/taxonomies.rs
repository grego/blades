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
use crate::page::{Page, PageRef, Paginate, Pagination, Permalink};
use crate::tasks::render;
use crate::types::{HashMap, MutSet, Templates};

use arrayvec::ArrayVec;
use beef::lean::Cow;
use ramhorns::{encoding::Encoder, traits::ContentSequence, Content, Section, Template};
use serde::Deserialize;

use std::cmp::Reverse;
use std::collections::hash_map::Entry;
use std::fs::create_dir_all;
use std::num::NonZeroUsize;
use std::ops::{Deref, Range};
use std::path::Path;

const DEFAULT_TEMPLATE: &str = "taxonomy.html";
const DEFAULT_KEY_TEMPLATE: &str = "taxonomy_key.html";

/// One class this page is a species of.
#[derive(Clone, Content, Deserialize)]
pub(crate) struct Species<'s>(
    #[ramhorns(rename = "name")]
    #[serde(borrow)]
    Cow<'s, str>,
);

/// All the classes in all taxonomies one page belongs to.
pub(crate) type Taxonomies<'p> = HashMap<&'p str, ArrayVec<Species<'p>, 4>>;

/// Classification of all pages on the site.
pub type Classification<'t, 'r> = HashMap<&'r str, Taxonomy<'t, 'r>>;

/// Information abouth the given taxonomy.
#[derive(Content, Clone, Deserialize)]
pub(crate) struct TaxonMeta<'t> {
    #[serde(borrow, default)]
    title: Cow<'t, str>,
    #[serde(borrow, default)]
    description: Cow<'t, str>,
    #[serde(borrow, default = "default_template")]
    #[ramhorns(skip)]
    template: Cow<'t, str>,
    #[serde(borrow, default = "default_key_template")]
    #[ramhorns(skip)]
    key_template: Cow<'t, str>,
    #[ramhorns(skip)]
    paginate_by: Option<NonZeroUsize>,
    #[serde(default)]
    #[ramhorns(skip)]
    sort_by_weight: bool,
}

/// One taxonomical category of the site (e.g. tags, categories).
#[derive(Content)]
pub struct Taxonomy<'t, 'r> {
    #[ramhorns(flatten)]
    taxonomy: TaxonMeta<'r>,
    slug: &'t str,
    keys: TaxDict<'t, 'r>,
    #[ramhorns(skip)]
    template: &'r Template<'static>,
    #[ramhorns(skip)]
    key_template: &'r Template<'static>,
}

/// All the pages in one taxonomical category, classified by the class name
struct TaxDict<'t, 'r>(HashMap<&'r str, Vec<PageLinked<'t, 'r>>>);

/// One taxonomical key, in the context of the whole site.
#[derive(Content, Clone)]
struct TaxKey<'t, 'r> {
    title: &'r str,
    taxonomy: &'r Taxonomy<'t, 'r>,
    pages: &'r [PageLinked<'t, 'r>],
    index: PageRef<'t, 'r>,
    site: &'r Config<'t>,
    classification: &'r Classification<'t, 'r>,
    pagination: Option<Pagination>,
}

/// One taxonomy in the context of the whole site.
#[derive(Content)]
struct TaxContext<'t, 'r> {
    #[ramhorns(flatten)]
    taxonomy: &'r Taxonomy<'t, 'r>,
    index: PageRef<'t, 'r>,
    site: &'r Config<'t>,
    classification: &'r Classification<'t, 'r>,
}

/// Classification of the whole site that's rendered as a list instead of a map (for sitemap)
pub(crate) struct TaxonList<'t, 'r>(pub(crate) &'r Classification<'t, 'r>);

/// Name of the taxonomical classes and its species
#[derive(Content)]
struct Coupled<'t, 'r>(
    #[ramhorns(rename = "key")] &'r str,
    #[ramhorns(rename = "pages")] &'r [PageLinked<'t, 'r>],
);

/// Reference to a page, coupled with it's permalink
#[derive(Content)]
pub struct PageLinked<'t, 'r>(
    #[ramhorns(flatten)] &'r Page<'t>,
    #[ramhorns(rename = "permalink")] Permalink<'t, 'r>,
);

impl<'t, 'r> Taxonomy<'t, 'r> {
    #[inline]
    fn empty(slug: &'t str, templates: &'r Templates) -> Result<Self> {
        Ok(Self {
            taxonomy: TaxonMeta {
                title: Cow::owned(title_case(slug)),
                description: Cow::const_str(""),
                template: Cow::const_str("taxonomy.html"),
                key_template: Cow::const_str("taxonomy_key.html"),
                paginate_by: None,
                sort_by_weight: false,
            },
            slug,
            keys: TaxDict(HashMap::default()),
            template: templates.get(DEFAULT_TEMPLATE)?,
            key_template: templates.get(DEFAULT_KEY_TEMPLATE)?,
        })
    }

    #[inline]
    fn new(slug: &'t str, other: &'r TaxonMeta<'t>, templates: &'r Templates) -> Result<Self> {
        Ok(Self {
            taxonomy: TaxonMeta {
                title: Cow::const_str(&other.title),
                description: Cow::const_str(&other.description),
                template: Cow::const_str(&other.template),
                key_template: Cow::const_str(&other.key_template),
                paginate_by: other.paginate_by,
                sort_by_weight: other.sort_by_weight,
            },
            slug,
            keys: TaxDict(HashMap::default()),
            template: templates.get(&other.template)?,
            key_template: templates.get(&other.key_template)?,
        })
    }

    #[inline]
    fn add(&mut self, species: &'r str, page: PageLinked<'t, 'r>) {
        self.keys
            .0
            .entry(species)
            .or_insert_with(Vec::new)
            .push(page)
    }

    /// Classify the given pages into taxonomies specified by the config.
    #[inline]
    pub fn classify(
        pages: &'r [Page<'t>],
        config: &'r Config<'t>,
        templates: &'r Templates,
    ) -> Result<Classification<'t, 'r>> {
        let mut named = config
            .taxonomies
            .iter()
            .map(|(&key, tax)| Taxonomy::new(key, tax, templates).map(|t| (key, t)))
            .collect::<Result<HashMap<_, _>, _>>()?;

        for page in pages {
            for (class, family) in &page.taxonomies {
                if let Some(taxon) = named.get_mut(class) {
                    for species in family {
                        taxon.add(species, PageLinked(page, Permalink(page, &config.url)));
                    }
                } else if config.implicit_taxonomies {
                    let taxon = match named.entry(class) {
                        Entry::Occupied(o) => o.into_mut(),
                        Entry::Vacant(v) => {
                            let taxonomy = Taxonomy::empty(class, templates)?;
                            v.insert(taxonomy)
                        }
                    };
                    for species in family {
                        taxon.add(species, PageLinked(page, Permalink(page, &config.url)));
                    }
                }
            }
        }

        for taxon in named.values_mut() {
            if taxon.taxonomy.sort_by_weight {
                taxon
                    .keys
                    .0
                    .iter_mut()
                    .for_each(|(_, pages)| pages.sort_unstable_by_key(|page| page.0.weight))
            } else {
                taxon
                    .keys
                    .0
                    .iter_mut()
                    .for_each(|(_, pages)| pages.sort_unstable_by_key(|page| Reverse(page.0.date)))
            }
        }

        Ok(named)
    }

    /// Get a reference to the key map of the given taxonomy.
    #[inline]
    pub fn keys(&self) -> &HashMap<&'r str, Vec<PageLinked<'t, 'r>>> {
        &self.keys.0
    }

    /// Render this taxonomy into the output directory specified by the config.
    #[inline]
    pub fn render(
        &self,
        config: &Config<'t>,
        classification: &Classification<'t, '_>,
        all: &[Page<'t>],
        rendered: &MutSet,
    ) -> Result {
        let mut path = Path::new(config.output_dir.as_ref()).join(self.slug);
        create_dir_all(&path)?;
        path.push("index.html");

        let contexted = TaxContext {
            taxonomy: self,
            site: config,
            index: all[0].by_ref(all, usize::MAX, &config.url),
            classification,
        };
        render(self.template, path, &contexted, rendered)
    }

    /// Render one key of this taxonomy into the output directory specified by the config.
    #[inline]
    pub fn render_key(
        &self,
        title: &str,
        pages: &[PageLinked<'t, '_>],
        config: &Config<'t>,
        classification: &Classification<'t, '_>,
        all: &[Page<'t>],
        rendered: &MutSet,
    ) -> Result {
        let mut output = Path::new(config.output_dir.as_ref()).join(self.slug);
        output.push(title);
        create_dir_all(&output)?;
        output.push("index.html");

        let contexted = TaxKey {
            title,
            taxonomy: self,
            pages,
            index: all[0].by_ref(all, usize::MAX, &config.url),
            site: config,
            classification,
            pagination: None,
        };

        let by = self
            .taxonomy
            .paginate_by
            .map(NonZeroUsize::get)
            .unwrap_or(0);
        if by > 0 && pages.len() > by {
            contexted.render_paginated(0, pages.len(), by, &mut output, self.key_template, rendered)
        } else {
            render(self.key_template, output, &contexted, rendered)
        }
    }
}

impl<'t, 'r> Paginate for TaxKey<'t, 'r> {
    fn paginate(&self, range: Range<usize>, length: usize, current: usize) -> Self {
        Self {
            pages: &self.pages[range],
            pagination: Some(Pagination::new(length, current)),
            // range in PageRef doesn't implement Copy
            ..self.clone()
        }
    }
}

impl<'t, 'r> Content for TaxDict<'t, 'r> {
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
        for (key, pages) in self.0.iter() {
            section.with(&Coupled(key, pages)).render(encoder)?;
        }
        Ok(())
    }
}

impl<'t, 'r> Content for TaxonList<'t, 'r> {
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
        for taxonomy in self.0.values() {
            section.with(taxonomy).render(encoder)?;
        }
        Ok(())
    }
}

impl<'s> Deref for Species<'s> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

#[inline]
fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[inline]
const fn default_template() -> Cow<'static, str> {
    Cow::const_str(DEFAULT_TEMPLATE)
}

#[inline]
const fn default_key_template() -> Cow<'static, str> {
    Cow::const_str(DEFAULT_KEY_TEMPLATE)
}
