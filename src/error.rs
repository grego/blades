// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use derive_more::{Display, Error};

/// All possible ways the site generation can fail.
#[derive(Debug, Display, Error)]
pub enum Error {
    /// Input/output error
    #[display(fmt = "Input/output error: {}", "_0")]
    Io(std::io::Error),
    /// TOML deserialization error
    #[display(fmt = "Error parsing {}: {}", name, source)]
    Toml {
        /// Source of the error.
        source: toml::de::Error,
        /// Name of the file where the error occured
        #[error(ignore)]
        name: Box<str>,
    },
    /// Ramhorns template error
    #[display(fmt = "Ramhorns template error: {}", "_0")]
    Ramhorns(ramhorns::Error),
    /// No template with the given name was found
    #[display(fmt = "Template {} not found", "_0")]
    MissingTemplate(#[error(ignore)] Box<str>),
    /// File name doesn't is not a valid UTF-8 string
    #[display(fmt = "File {} is not a valid UTF-8 text", "_0")]
    InvalidUtf8(#[error(ignore)] Box<str>),
}

/// A convenience wrapper around std Result
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

impl From<std::io::Error> for Error {
    fn from(other: std::io::Error) -> Self {
        Self::Io(other)
    }
}

impl From<ramhorns::Error> for Error {
    fn from(other: ramhorns::Error) -> Self {
        Self::Ramhorns(other)
    }
}

impl<T: Into<Box<str>>> From<(toml::de::Error, T)> for Error {
    fn from((source, name): (toml::de::Error, T)) -> Self {
        Self::Toml {
            source,
            name: name.into(),
        }
    }
}
