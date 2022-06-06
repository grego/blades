#![feature(test)]
extern crate test;
use test::{black_box, Bencher};

static SOURCE: &str = "<title>{{title}}</title><h1>{{ title }}</h1><div>{{{body}}}</div>";
const RENDERED_BYTES: u64 =
    "Hello, Ramhorns!This is a really simple test of the rendering!<title></title><h1></h1><div></div>".len() as u64;

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
