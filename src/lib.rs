// Blades  Copyright (C) 2020  Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>

//! Blazing fast
//!  dead simple
//!   static site generator.
#![warn(missing_docs)]
mod config;
mod error;
mod page;
mod sources;
mod tasks;
mod taxonomies;
mod types;

pub use config::Config;
pub use error::{Error, Result};
pub use page::Page;
pub use sources::{Source, Sources};
pub use tasks::{cleanup, colocate_assets, render_meta};
pub use taxonomies::Taxonomy;
pub use types::{MutSet, Templates};
