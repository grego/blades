// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>

use crate::config::Config;
use crate::error::{Error, Result};

use std::fs::{read_dir, File};
use std::io::{ErrorKind, Read};
use std::ops::Range;
use std::path::PathBuf;
use std::time::SystemTime;

/// Data about where the source of a one particular file is located
pub struct Source {
    /// Range in the slice of data
    pub(crate) source: Range<usize>,
    pub(crate) path: Box<str>,
    /// Range in the slice of sources
    pub(crate) pages: Range<usize>,
    /// Range in the slice of sources
    pub(crate) subsections: Range<usize>,
    pub(crate) is_section: bool,
    pub(crate) parent: usize,
    pub(crate) date: Option<SystemTime>,
    pub(crate) to_load: Option<PathBuf>,
}

/// All of the site source files
pub struct Sources {
    /// Binary data read of all the files
    pub(crate) data: Vec<u8>,
    /// Info about where the particular files are loaded
    sources: Vec<Source>,
}

impl Source {
    #[inline]
    fn new(path: PathBuf, src: Range<usize>, parent: usize, date: SystemTime) -> Result<Self> {
        Ok(Self {
            source: src,
            path: path_to_string(path)?.into(),
            pages: 0..0,
            subsections: 0..0,
            is_section: false,
            parent,
            date: Some(date),
            to_load: None,
        })
    }

    /// Create a placeholder source, not referencing any data.
    #[inline]
    fn empty(section: PathBuf, parent: usize) -> Self {
        Self {
            source: 0..0,
            path: "".into(),
            pages: 0..0,
            subsections: 0..0,
            is_section: true,
            parent,
            date: None,
            to_load: Some(section),
        }
    }
}

impl Sources {
    /// Add all the sources from the current directory to `self`.
    #[inline]
    fn step(&mut self, index: usize, dirs: &mut Vec<PathBuf>) -> Result<()> {
        let start = self.sources.len();
        let mut path = self.sources[index].to_load.take().unwrap();
        for (path, date) in read_dir(&path)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|ft| {
                        if ft.is_dir() {
                            dirs.push(entry.path());
                            false
                        } else {
                            ft.is_file()
                        }
                    })
                    .unwrap_or(false)
            })
            .filter_map(|entry| {
                entry
                    .metadata()
                    .and_then(|m| m.created())
                    .map(|date| (entry.path(), date))
                    .ok()
            })
            .filter(|(path, _)| {
                path.file_stem()
                    .and_then(|s| path.extension().map(|e| (s, e)))
                    .map(|(stem, extension)| extension == "toml" && stem != "index")
                    .unwrap_or(false)
            })
        {
            let start = self.data.len();
            let read = File::open(&path)?.read_to_end(&mut self.data)?;
            self.sources
                .push(Source::new(path, start..(start + read), index, date)?);
        }
        let end = self.sources.len();

        for dir in dirs.drain(..) {
            self.sources.push(Source::empty(dir, index));
        }
        let len = self.sources.len();

        let source_start = self.data.len();
        path.push("index.toml");
        let (read, date) = match File::open(&path) {
            Ok(mut file) => (
                file.read_to_end(&mut self.data)?,
                file.metadata()?.created().ok(),
            ),
            Err(e) if e.kind() == ErrorKind::NotFound => (0, None),
            Err(e) => return Err(e.into()),
        };
        path.pop();

        self.sources[index].path = path_to_string(path)?.into();
        self.sources[index].source = source_start..(source_start + read);
        self.sources[index].pages = start..end;
        self.sources[index].date = date;
        if len > end {
            self.sources[index].subsections = end..len;
        }

        Ok(())
    }

    /// Load all the sources from the directory specified by the config.
    pub fn load(config: &Config) -> Result<Self> {
        let mut sources = Self {
            data: Vec::with_capacity(65536),
            sources: Vec::with_capacity(64),
        };
        // The first source directory is the one specified in config.
        sources
            .sources
            .push(Source::empty(config.content_dir.as_ref().into(), 0));

        let mut dirs_buffer = Vec::new();
        let mut i = 0;
        // Check all the sources whether they contain something more to load.
        while i < sources.sources.len() {
            if sources.sources[i].to_load.is_some() {
                sources.step(i, &mut dirs_buffer)?;
            }
            i += 1;
        }

        Ok(sources)
    }

    /// Get a reference of the innel list of sources.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
}

#[inline]
fn path_to_string(path: PathBuf) -> Result<String> {
    path.into_os_string()
        .into_string()
        .map_err(|s| Error::InvalidUtf8(s.to_string_lossy().as_ref().into()))
}
