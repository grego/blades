// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use crate::types::{Any, HashMap};

use beef::lean::Cow;
use ramhorns::Content;
use serde::{Deserialize, Serialize};

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

    /// Extra values provided by the user
    #[serde(flatten)]
    #[ramhorns(flatten)]
    pub extra: HashMap<&'c str, Any<'c>>,
}

#[inline]
const fn default_assets() -> Cow<'static, str> {
    Cow::const_str("assets")
}

#[inline]
pub(crate) const fn default_true() -> bool {
    true
}
