// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>

use crate::taxonomies::TaxonMeta;
use crate::types::{Any, HashMap};

use beef::lean::Cow;
use ramhorns::Content;
use serde::Deserialize;

// These are pre-defined since the life is easier when they are the same for every theme.
pub(crate) static TEMPLATE_DIR: &str = "templates";
/// Where the assets will be copied from, relative to the site directrory.
pub(crate) static ASSET_SRC_DIR: &str = "assets";

/// Main configuration where all the site settings are set.
/// Razor deserializes it from a given TOML file.
#[derive(Content, Deserialize)]
pub struct Config<'c> {
    /// The directory of the content
    #[serde(borrow, default = "default_content_dir")]
    #[ramhorns(skip)]
    pub content_dir: Cow<'c, str>,
    /// The directory where the output should be rendered to
    #[serde(borrow, default = "default_output_dir")]
    #[ramhorns(skip)]
    pub output_dir: Cow<'c, str>,
    /// The directory where the themes are
    #[serde(borrow, default = "default_theme_dir")]
    #[ramhorns(skip)]
    pub theme_dir: Cow<'c, str>,
    /// Where the assets will be copied to, relative to the site directory.
    #[serde(borrow, default = "default_assets")]
    pub(crate) assets: Cow<'c, str>,
    /// Name of the directory of a theme this site is using, empty if none.
    #[serde(borrow, default)]
    pub theme: Cow<'c, str>,
    #[serde(borrow, default)]
    title: Cow<'c, str>,
    #[serde(borrow, default)]
    description: Cow<'c, str>,
    #[serde(borrow, default)]
    keywords: Cow<'c, str>,
    #[serde(borrow, default)]
    image: Cow<'c, str>,
    #[serde(borrow, default)]
    author: Option<Author<'c>>,
    #[serde(borrow, default)]
    pub(crate) url: Cow<'c, str>,

    #[serde(default = "default_true")]
    pub(crate) sitemap: bool,
    #[serde(default = "default_true")]
    pub(crate) rss: bool,
    #[serde(default = "default_true")]
    pub(crate) atom: bool,
    #[serde(default = "default_true")]
    pub(crate) implicit_taxonomies: bool,
    #[serde(default = "default_true")]
    pub(crate) dates_of_creation: bool,

    #[serde(default)]
    #[ramhorns(skip)]
    pub(crate) taxonomies: HashMap<&'c str, TaxonMeta<'c>>,
    extra: Option<HashMap<&'c str, Any<'c>>>,
}

#[derive(Clone, Content, Default, serde::Deserialize)]
struct Author<'a> {
    #[serde(borrow, default)]
    name: Cow<'a, str>,
    #[serde(borrow, default)]
    email: Cow<'a, str>,
    #[serde(borrow, default)]
    uri: Cow<'a, str>,
    #[serde(borrow, default)]
    avatar: Cow<'a, str>,
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
const fn default_assets() -> Cow<'static, str> {
    Cow::const_str("assets")
}

#[inline]
const fn default_true() -> bool {
    true
}
