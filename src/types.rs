// Blades  Copyright (C) 2021 Maroš Grego
//
// This file is part of Blades. This program comes with ABSOLUTELY NO WARRANTY;
// This is free software, and you are welcome to redistribute it under the
// conditions of the GNU General Public License version 3.0.
//
// You should have received a copy of the GNU General Public License
// along with Blades.  If not, see <http://www.gnu.org/licenses/>
use beef::lean::Cow;
use chrono::{DateTime as CDateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, Timelike};
use ramhorns::encoding::Encoder;
use ramhorns::traits::ContentSequence;
use ramhorns::{Content, Section};
use serde::de::{self, Deserialize, Deserializer, Visitor};

use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::path::{is_separator, PathBuf};
use std::time::SystemTime;

/// A set of all rendered paths. Behind a mutex, so it can be written from multiple threads.
pub type MutSet<T = PathBuf> = parking_lot::Mutex<HashSet<T, ahash::RandomState>>;

/// A hash map wrapper that can render fields directly by the hash.
#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct HashMap<K: Hash + Eq, V>(pub(crate) hashbrown::HashMap<K, V, fnv::FnvBuildHasher>);

/// A wrapper around the `chrono::NaiveDateTime`, used for rendering of dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct DateTime(pub NaiveDateTime);

/// A wrapper around a `str` representing path, used to derive `Content` implementation
/// that acts like an iterator over the path segmets.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct Ancestors<'a>(#[serde(borrow)] pub Cow<'a, str>);

/// One segment of a path.
#[derive(Content)]
struct Segment<'a>(
    /// This segment.
    #[ramhorns(rename = "name")]
    &'a str,
    /// Full path up to this segment.
    #[ramhorns(rename = "full")]
    &'a str,
);

/// A sum of all the types that can be used in a TOML file.
#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Any<'a> {
    /// A string.
    String(#[serde(borrow)] Cow<'a, str>),
    /// A number.
    Number(f64),
    /// A boolean value.
    Bool(bool),
    /// Date and time data.
    DateTime(DateTime),
    /// A list.
    List(Vec<Any<'a>>),
    /// A key-value map.
    Map(HashMap<&'a str, Any<'a>>),
}

impl<'a> Content for Ancestors<'a> {
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.0.is_empty()
    }

    #[inline]
    fn render_escaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        // The path was stripped of leading separators.
        if !self.0.is_empty() {
            encoder.write_unescaped("/")?;
            encoder.write_escaped(&self.0)?;
        }
        Ok(())
    }

    #[inline]
    fn render_unescaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        if !self.0.is_empty() {
            encoder.write_unescaped("/")?;
            encoder.write_unescaped(&self.0)?;
        }
        Ok(())
    }

    #[inline]
    fn render_section<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        let s = self.0.as_ref();
        if s.is_empty() {
            return Ok(());
        }

        let mut previous = 0;
        for (i, sep) in s.match_indices(is_separator) {
            section
                .with(&Segment(&s[previous..i], &s[0..i]))
                .render(encoder)?;
            previous = i + sep.len();
        }
        if !s.contains(is_separator) {
            section.with(&Segment(s, s)).render(encoder)?;
        }
        Ok(())
    }
}

impl AsRef<str> for Ancestors<'_> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> Default for Ancestors<'a> {
    #[inline]
    fn default() -> Self {
        Ancestors(Cow::const_str(""))
    }
}

impl<'a> From<Cow<'a, str>> for Ancestors<'a> {
    #[inline]
    fn from(s: Cow<'a, str>) -> Self {
        Ancestors(s)
    }
}

#[inline]
fn content_without_paragraphs<E: Encoder>(source: &str, encoder: &mut E) -> Result<(), E::Error> {
    use pulldown_cmark::{Event, Tag};
    let parser =
        pulldown_cmark::Parser::new_ext(source, pulldown_cmark::Options::all()).filter(|event| {
            !matches!(
                event,
                Event::Start(Tag::Paragraph) | Event::End(Tag::Paragraph),
            )
        });
    let processed = cmark_syntax::SyntaxPreprocessor::new(parser);
    encoder.write_html(processed)
}

impl<'a> Content for Any<'a> {
    #[inline]
    fn is_truthy(&self) -> bool {
        match self {
            Any::Bool(b) => *b,
            Any::List(vec) => !vec.is_empty(),
            Any::Map(map) => !map.is_empty(),
            Any::String(s) => !s.is_empty(),
            Any::Number(n) => n.abs() > f64::EPSILON,
            _ => true,
        }
    }

    #[inline]
    fn render_escaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        match self {
            Any::Bool(b) => b.render_escaped(encoder),
            Any::String(ref s) => content_without_paragraphs(s, encoder),
            Any::Number(n) => n.render_escaped(encoder),
            Any::DateTime(dt) => dt.render_escaped(encoder),
            Any::List(vec) => vec.render_escaped(encoder),
            Any::Map(map) => map.render_escaped(encoder),
        }
    }

    #[inline]
    fn render_unescaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        match self {
            Any::Bool(b) => b.render_unescaped(encoder),
            Any::String(s) => s.render_unescaped(encoder),
            Any::Number(n) => n.render_unescaped(encoder),
            Any::DateTime(dt) => dt.render_unescaped(encoder),
            Any::List(vec) => vec.render_unescaped(encoder),
            Any::Map(map) => map.render_unescaped(encoder),
        }
    }

    #[inline]
    fn render_section<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        match self {
            Any::List(vec) => vec.render_section(section, encoder),
            Any::Map(map) => map.render_section(section, encoder),
            Any::DateTime(dt) => dt.render_section(section, encoder),
            _ => {
                if self.is_truthy() {
                    section.render(encoder)
                } else {
                    Ok(())
                }
            }
        }
    }

    #[inline]
    fn render_field_escaped<E>(&self, h: u64, name: &str, enc: &mut E) -> Result<bool, E::Error>
    where
        E: Encoder,
    {
        match self {
            Any::Map(map) => map.render_field_escaped(h, name, enc),
            _ => Ok(false),
        }
    }

    #[inline]
    fn render_field_unescaped<E>(&self, h: u64, name: &str, enc: &mut E) -> Result<bool, E::Error>
    where
        E: Encoder,
    {
        match self {
            Any::Map(map) => map.render_field_unescaped(h, name, enc),
            _ => Ok(false),
        }
    }

    #[inline]
    fn render_field_section<C, E>(
        &self,
        hash: u64,
        name: &str,
        section: Section<C>,
        encoder: &mut E,
    ) -> Result<bool, E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        match self {
            Any::Map(map) => map.render_field_section(hash, name, section, encoder),
            _ => Ok(false),
        }
    }

    #[inline]
    fn render_field_inverse<C, E>(
        &self,
        hash: u64,
        name: &str,
        section: Section<C>,
        encoder: &mut E,
    ) -> Result<bool, E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        match self {
            Any::Map(map) => map.render_field_inverse(hash, name, section, encoder),
            _ => Ok(false),
        }
    }
}

impl DateTime {
    /// The date and time right now.
    pub fn now() -> Self {
        SystemTime::now().into()
    }
}

impl Content for DateTime {
    #[inline]
    fn render_section<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        section.with(self).render(encoder)
    }

    #[inline]
    fn render_field_escaped<E>(&self, _: u64, name: &str, enc: &mut E) -> Result<bool, E::Error>
    where
        E: Encoder,
    {
        if name.len() != 1 {
            return Ok(false);
        }

        const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        const NUMS: [&str; 60] = [
            "00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13",
            "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27",
            "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41",
            "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55",
            "56", "57", "58", "59",
        ];

        match name.bytes().next().unwrap_or(0) {
            b'y' => self.0.year().render_unescaped(enc).map(|_| true),
            b'm' => enc
                .write_unescaped(NUMS[self.0.month() as usize])
                .map(|_| true),
            b'd' => enc
                .write_unescaped(NUMS[self.0.day() as usize])
                .map(|_| true),
            b'e' => self.0.day().render_unescaped(enc).map(|_| true),
            b'H' => enc
                .write_unescaped(NUMS[self.0.hour() as usize])
                .map(|_| true),
            b'M' => enc
                .write_unescaped(NUMS[self.0.minute() as usize])
                .map(|_| true),
            b'S' => enc
                .write_unescaped(NUMS[self.0.second() as usize])
                .map(|_| true),
            b'a' => enc
                .write_unescaped(WEEKDAYS[self.0.weekday().num_days_from_sunday() as usize])
                .map(|_| true),
            b'b' => enc
                .write_unescaped(MONTHS[self.0.month0() as usize])
                .map(|_| true),
            _ => Ok(false),
        }
    }

    #[inline]
    fn render_field_unescaped<E>(&self, h: u64, name: &str, enc: &mut E) -> Result<bool, E::Error>
    where
        E: Encoder,
    {
        self.render_field_escaped(h, name, enc)
    }
}

// Toml crate currently doesn't supprot deserializing dates into types other than String,
// so an ugly hack based on its `Deserializer` private fields needs to be used.
const FIELD: &str = "$__toml_private_datetime";

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<DateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DateTimeKey;
        struct DateTimeVisitor;

        impl<'de> Deserialize<'de> for DateTimeKey {
            fn deserialize<D>(deserializer: D) -> Result<DateTimeKey, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> de::Visitor<'de> for FieldVisitor {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a valid datetime field")
                    }

                    fn visit_str<E>(self, s: &str) -> Result<(), E>
                    where
                        E: de::Error,
                    {
                        if s == FIELD {
                            Ok(())
                        } else {
                            Err(de::Error::custom("expected field with a custom name"))
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)?;
                Ok(DateTimeKey)
            }
        }

        impl<'de> Visitor<'de> for DateTimeVisitor {
            type Value = DateTime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a TOML datetime")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<DateTime, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let value = visitor.next_key::<DateTimeKey>()?;
                if value.is_none() {
                    return Err(de::Error::custom("datetime key not found"));
                }
                let v: &str = visitor.next_value()?;
                self.visit_str(v)
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse::<NaiveDateTime>()
                    .or_else(|_| v.parse::<NaiveDate>().map(|d| d.and_hms(0, 0, 0)))
                    .or_else(|_| NaiveDateTime::parse_from_str(v, "%F %T%.f"))
                    .or_else(|_| v.parse::<CDateTime<FixedOffset>>().map(|d| d.naive_utc()))
                    .map(DateTime)
                    .map_err(|_| {
                        de::Error::custom(format!("unable to parse date and time from {}", v))
                    })
            }
        }

        deserializer.deserialize_str(DateTimeVisitor)
    }
}

impl From<SystemTime> for DateTime {
    fn from(st: SystemTime) -> Self {
        let time: chrono::DateTime<chrono::Utc> = st.into();
        DateTime(time.naive_utc())
    }
}

impl<K: Borrow<str> + Hash + Eq, V: Content> Content for HashMap<K, V> {
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }

    /// Render a section with self.
    #[inline]
    fn render_section<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        if self.is_truthy() {
            section.with(self).render(encoder)
        } else {
            Ok(())
        }
    }

    #[inline]
    fn render_field_escaped<E>(
        &self,
        hash: u64,
        _name: &str,
        encoder: &mut E,
    ) -> Result<bool, E::Error>
    where
        E: Encoder,
    {
        match self.raw_entry().from_hash(hash, |_| true) {
            Some((_, v)) => v.render_escaped(encoder).map(|_| true),
            None => Ok(false),
        }
    }

    #[inline]
    fn render_field_unescaped<E>(
        &self,
        hash: u64,
        _name: &str,
        encoder: &mut E,
    ) -> Result<bool, E::Error>
    where
        E: Encoder,
    {
        match self.raw_entry().from_hash(hash, |_| true) {
            Some((_, v)) => v.render_unescaped(encoder).map(|_| true),
            None => Ok(false),
        }
    }

    #[inline]
    fn render_field_section<C, E>(
        &self,
        hash: u64,
        _name: &str,
        section: Section<C>,
        encoder: &mut E,
    ) -> Result<bool, E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        match self.raw_entry().from_hash(hash, |_| true) {
            Some((_, v)) => v.render_section(section, encoder).map(|_| true),
            None => Ok(false),
        }
    }

    #[inline]
    fn render_field_inverse<C, E>(
        &self,
        hash: u64,
        _name: &str,
        section: Section<C>,
        encoder: &mut E,
    ) -> Result<bool, E::Error>
    where
        C: ContentSequence,
        E: Encoder,
    {
        match self.raw_entry().from_hash(hash, |_| true) {
            Some((_, v)) => v.render_inverse(section, encoder).map(|_| true),
            None => Ok(false),
        }
    }
}

impl<K: Hash + Eq, V> Default for HashMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: Hash + Eq, V> Deref for HashMap<K, V> {
    type Target = hashbrown::HashMap<K, V, fnv::FnvBuildHasher>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: Hash + Eq, V> DerefMut for HashMap<K, V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn simple_render_hash_map() {
        use super::HashMap;

        let source = "<title>{{title}}</title><h1>{{ title }}</h1><div>{{body}}</div>";
        let tpl = ramhorns::Template::new(source).unwrap();

        let mut map = HashMap::default();

        map.insert("title", "Hello, Ramhorns!");
        map.insert(
            "body",
            "This is a test of rendering a template with a HashMap Content!",
        );

        let rendered = tpl.render(&map);

        assert_eq!(
            &rendered,
            "<title>Hello, Ramhorns!</title><h1>Hello, Ramhorns!</h1>\
         <div>This is a test of rendering a template with a HashMap Content!</div>"
        );
    }
}
