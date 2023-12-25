#![feature(test)]
extern crate test;
use blades::{Page, Parser, Sources};
use std::thread;
use test::{black_box, Bencher};

static SOURCE: &str = "<title>{{title}}</title><h1>{{ title }}</h1><div>{{{body}}}</div>";
const RENDERED_BYTES: u64 =
    "Hello, Ramhorns!This is a really simple test of the rendering!<title></title><h1></h1><div></div>".len() as u64;

#[derive(Default)]
struct Toml;

impl Parser for Toml {
    type Error = toml::de::Error;

    fn from_extension(_ext: &std::ffi::OsStr) -> Option<Self> {
        Some(Toml)
    }

    fn parse<'a>(&self, data: &'a [u8]) -> Result<Page<'a>, Self::Error> {
        toml::de::from_slice(data)
    }
}

#[bench]
fn fnv_hashmap(b: &mut Bencher) {
    let tpl = ramhorns::Template::new(SOURCE).unwrap();

    let mut map: std::collections::HashMap<_, _, fnv::FnvBuildHasher> = Default::default();
    map.insert("title", "Hello, Ramhorns!");
    map.insert("body", "This is a really simple test of the rendering!");

    b.bytes = RENDERED_BYTES;
    b.iter(|| black_box(tpl.render(&map)));
}

#[bench]
fn blades_hashmap(b: &mut Bencher) {
    let tpl = ramhorns::Template::new(SOURCE).unwrap();

    let mut map = blades::HashMap::default();
    map.insert("title", "Hello, Ramhorns!");
    map.insert("body", "This is a really simple test of the rendering!");

    b.bytes = RENDERED_BYTES;
    b.iter(|| black_box(tpl.render(&map)));
}

#[bench]
fn parse_pages(b: &mut Bencher) {
    let sources: Sources<Toml> = Sources::load("examples/content").unwrap();
    b.iter(|| {
        sources
            .sources()
            .iter()
            .map(|src| Page::new(src, &sources))
            .collect::<Vec<_>>()
    });
}

#[bench]
fn spawn_and_join(b: &mut Bencher) {
    b.iter(|| thread::spawn(|| 1 + 2).join());
}
