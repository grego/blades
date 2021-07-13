// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use crate::config::Config;
use crate::page::Page;

use std::ffi::OsStr;
use std::fs::{read_dir, File};
use std::io::{self, Read};
use std::ops::Range;
use std::path::PathBuf;
use std::time::SystemTime;

/// A structure that can parse Page from binary data.
/// Is typically a deserializer or an enum of deserializers.
pub trait Parser: Default + Sized {
    /// The error that can happen during parsing.
    type Error: std::error::Error;
    
    /// The kind of parser that should be used, based on the file extension.
    fn from_extension(_extension: &OsStr) -> Option<Self> {
        Some(Self::default())
    }

    /// Parse the binary data into a Page.
    fn parse<'a>(&self, data: &'a [u8]) -> Result<Page<'a>, Self::Error>;
}

/// Data about where the source of a one particular file is located
pub struct Source<P: Parser> {
    /// Range in the slice of data
    pub(crate) source: Range<usize>,
    /// Range in the slice of data
    pub(crate) path: Range<usize>,
    /// Range in the slice of sources
    pub(crate) pages: Range<usize>,
    /// Range in the slice of sources
    pub(crate) subsections: Range<usize>,
    pub(crate) is_section: bool,
    pub(crate) parent: usize,
    pub(crate) date: Option<SystemTime>,
    pub(crate) to_load: Option<PathBuf>,
    pub(crate) format: P,
}

/// All of the site source files
pub struct Sources<P: Parser> {
    /// Binary data read of all the files
    pub(crate) data: Vec<u8>,
    /// Info about where the particular files are loaded
    sources: Vec<Source<P>>,
}

impl<P: Parser> Source<P> {
    #[inline]
    fn new(
        path: Range<usize>,
        src: Range<usize>,
        parent: usize,
        date: Option<SystemTime>,
        format: P,
    ) -> Self {
        Self {
            source: src,
            path,
            pages: 0..0,
            subsections: 0..0,
            is_section: false,
            parent,
            date,
            to_load: None,
            format,
        }
    }

    /// Create a placeholder source, not referencing any data.
    #[inline]
    fn empty(section: PathBuf, parent: usize) -> Self {
        Self {
            source: 0..0,
            path: 0..0,
            pages: 0..0,
            subsections: 0..0,
            is_section: true,
            parent,
            date: None,
            to_load: Some(section),
            format: P::default(),
        }
    }
}

impl<P: Parser> Sources<P> {
    /// Add all the sources from the current directory to `self`.
    #[inline]
    fn step(
        &mut self,
        index: usize,
        path: PathBuf,
        dirs: &mut Vec<PathBuf>,
    ) -> Result<(), io::Error> {
        let start = self.sources.len();
        let mut index_file = None;
        for (path, date, format) in read_dir(&path)?
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
            .map(|entry| {
                let date = entry.metadata().and_then(|m| m.created()).ok();
                (entry.path(), date)
            })
            .filter_map(|(path, date)| {
                let ext = path.extension().unwrap_or_default();
                let format = P::from_extension(ext)?;
                if path.file_stem()? == "index" {
                    index_file = Some((path, date, format));
                    return None;
                };
                Some((path, date, format))
            })
        {
            let start = self.data.len();
            let read = File::open(&path)?.read_to_end(&mut self.data)?;
            let mid = start + read;
            let path = path.to_string_lossy();
            let ext_start = path.rfind('.').unwrap_or_else(|| path.len());
            self.data
                .extend_from_slice(path[..ext_start].as_ref());
            let end = self.data.len();
            self.sources
                .push(Source::new(mid..end, start..mid, index, date, format));
        }
        let end = self.sources.len();

        for dir in dirs.drain(..) {
            self.sources.push(Source::empty(dir, index));
        }
        let len = self.sources.len();

        let source_start = self.data.len();
        let read = if let Some((path, date, format)) = index_file {
            self.sources[index].date = date;
            self.sources[index].format = format;
            File::open(path)?.read_to_end(&mut self.data)?
        } else {
            0
        };
        let mid = source_start + read;
        self.data
            .extend_from_slice(path.to_string_lossy().as_ref().as_ref());
        let source_end = self.data.len();

        self.sources[index].path = mid..source_end;
        self.sources[index].source = source_start..mid;
        self.sources[index].pages = start..end;
        if len > end {
            self.sources[index].subsections = end..len;
        }

        Ok(())
    }

    /// Load all the sources from the directory specified by the config.
    pub fn load(config: &Config) -> Result<Self, io::Error> {
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
            if let Some(path) = sources.sources[i].to_load.take() {
                sources.step(i, path, &mut dirs_buffer)?;
            }
            i += 1;
        }

        Ok(sources)
    }

    /// Get a reference of the innel list of sources.
    pub fn sources(&self) -> &[Source<P>] {
        &self.sources
    }
}
