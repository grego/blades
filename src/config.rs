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
use serde::{Deserialize, Serialize};
use serde_cmd::CmdBorrowed;

// These are pre-defined since the life is easier when they are the same for every theme.
pub(crate) static TEMPLATE_DIR: &str = "templates";
/// Where the assets will be copied from, relative to the site directrory.
pub(crate) static ASSET_SRC_DIR: &str = "assets";

/// Main configuration where all the site settings are set.
/// Razor deserializes it from a given TOML file.
#[derive(Default, Deserialize, Serialize)]
pub struct Config<'c> {
    /// The directory of the content
    #[serde(borrow, default = "default_content_dir")]
    pub content_dir: Cow<'c, str>,
    /// The directory where the output should be rendered to
    #[serde(borrow, default = "default_output_dir")]
    pub output_dir: Cow<'c, str>,
    /// The directory where the themes are
    #[serde(borrow, default = "default_theme_dir")]
    pub theme_dir: Cow<'c, str>,
    /// Name of the directory of a theme this site is using, empty if none.
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub theme: Cow<'c, str>,
    /// Taxonomies of the site
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub taxonomies: HashMap<&'c str, TaxonMeta<'c>>,
    /// Generate taxonomies not specified in the config?
    #[serde(default = "default_true")]
    pub implicit_taxonomies: bool,

    /// Information about the site usable in templates
    #[serde(flatten)]
    pub site: Site<'c>,

    /// Configuration of plugins for building the site.
    #[serde(default, skip_serializing)]
    pub plugins: Plugins<'c>,
}

/// Information about the site usable in templates
#[derive(Content, Default, Deserialize, Serialize)]
pub struct Site<'c> {
    /// Where the assets will be copied to, relative to the site directory.
    #[serde(borrow, default = "default_assets")]
    pub assets: Cow<'c, str>,
    /// Title of the site
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub title: Cow<'c, str>,
    /// Description of the site
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub description: Cow<'c, str>,
    /// Keywords of the site
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub keywords: Cow<'c, str>,
    /// A representative image of the site
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub image: Cow<'c, str>,
    /// Language of the site
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub lang: Cow<'c, str>,
    /// Name of the author of the site
    #[serde(borrow, default)]
    pub author: Cow<'c, str>,
    /// Email of the webmaster
    #[serde(borrow, default)]
    pub email: Cow<'c, str>,
    /// URL of the site
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    pub url: Cow<'c, str>,

    /// Generate a sitemap?
    #[serde(default = "default_true")]
    pub sitemap: bool,
    /// Generate RSS feed?
    #[serde(default = "default_true")]
    pub rss: bool,
    /// Generate Atom feed?
    #[serde(default = "default_true")]
    pub atom: bool,

    #[serde(flatten)]
    #[ramhorns(flatten)]
    pub extra: HashMap<&'c str, Any<'c>>,
}

/// Plugins to use when building the site.
#[derive(Default, Deserialize)]
pub struct Plugins<'p> {
    /// Plugins to get the input from, in the form of serialized list of pages.
    #[serde(borrow, default)]
    pub input: Box<[CmdBorrowed<'p>]>,
    /// Plugins that transform the serialized list of pages.
    #[serde(borrow, default)]
    pub transform: Box<[CmdBorrowed<'p>]>,
    /// Plugins that get the serialized list of pages and might do something with it.
    #[serde(borrow, default)]
    pub output: Box<[CmdBorrowed<'p>]>,
    /// Plugins that transform the content of pages.
    /// They are identified by their name and must be enabled for each page.
    #[serde(borrow, default)]
    pub content: HashMap<&'p str, CmdBorrowed<'p>>,
    /// A list of names of content plugins that should be applied to every page.
    #[serde(default)]
    pub default: Box<[&'p str]>,
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
pub(crate) const fn default_true() -> bool {
    true
}
