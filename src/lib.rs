// Blades  Copyright (C) 2021 Maroš Grego
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
//!
//! # Features
//! Currently, Cargo doesn't support binary-only dependencies. As such, these dependencies are behind
//! the `bin` feature gate, which is enabled by default. When using Blades as a library, they are not
//! necessary, so it is recommended to import blades with `default_features = false`.
#![warn(missing_docs)]
mod config;
mod page;
mod sources;
mod tasks;
mod taxonomies;
mod types;

pub use config::Config;
pub use page::{Page, Pages, Picture};
pub use sources::{Parser, Source, Sources};
pub use tasks::{cleanup, colocate_assets, render_meta};
pub use taxonomies::{Taxonomies, Taxonomy};
pub use types::{Ancestors, Any, DateTime, MutSet, Templates};
