// Blades  Copyright (C) 2020  Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>

use custom_error::custom_error;

custom_error! {
/// All possible ways the site generation can fail.
pub Error
    /// Input/output error
    Io{
        /// Source of the error.
        source: std::io::Error
    } 							= "Input/output error: {source}",
    /// TOML deserialization error
    Toml{
        /// Source of the error.
        source: toml::de::Error,
        /// Name of the file where the error occured
        name: Box<str>
    } 							= "Error parsing {name}: {source}",
    /// Ramhorns template error
    Ramhorns{
        /// Source of the error.
        source: ramhorns::Error
    }							= "Ramhorns template error: {source}",
    /// No template with the given name was found
    MissingTemplate{
        /// Name of the template that was not found.
        name: Box<str>
    }							= "Template file {name} not found.",
    /// File name doesn't is not a valid UTF-8 string
    InvalidUtf8{
        /// Name of the file with invalid name.
        name: Box<str>
    }							= "File {name} is not a valid UTF-8 text",
}

/// A convenience wrapper around std Result
pub type Result<T = (), E = Error> = std::result::Result<T, E>;
