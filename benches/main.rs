#![feature(test)]
#![recursion_limit = "256"]
extern crate test;

use blades::{Config, MutSet, Page, Sources, Taxonomy, Templates};
use rayon::prelude::*;
use test::{black_box, Bencher};

static CONFIG: &str = "examples/Blades.toml";

fn parse_sources<'a>(sources: &'a Sources, config: &Config) -> Vec<Page<'a>> {
    sources
        .sources()
        .par_iter()
        .map(|src| Page::new(src, &sources, &config))
        .collect::<Result<Vec<_>, _>>()
        .and_then(|pages| Page::prepare(pages, &config))
        .unwrap()
}

#[bench]
fn a_load_config(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    b.iter(|| black_box(toml::from_slice::<Config>(&file)));
}

#[bench]
fn b_load_sources(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    b.iter(|| black_box(Sources::load(&config)));
}

#[bench]
fn c_load_templates(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    b.iter(|| black_box(Templates::load(&config)));
}

#[bench]
fn d_parse_sources(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    let sources = Sources::load(&config).unwrap();
    b.iter(|| black_box(parse_sources(&sources, &config)));
}

#[bench]
fn da_parse_pages_from_toml(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    let sources = Sources::load(&config).unwrap();
    b.iter(|| {
        black_box(
            sources
                .sources()
                .par_iter()
                .map(|src| Page::new(src, &sources, &config))
                .collect::<Result<Vec<_>, _>>(),
        )
    });
}

#[bench]
fn db_parse_sources_and_load_templates(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    let sources = Sources::load(&config).unwrap();
    b.iter(|| {
        black_box(rayon::join(
            || Templates::load(&config),
            || parse_sources(&sources, &config),
        ))
    });
}

#[bench]
fn e_classify_pages(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    let sources = Sources::load(&config).unwrap();
    let pages = parse_sources(&sources, &config);
    let templates = Templates::load(&config).unwrap();
    b.iter(|| black_box(Taxonomy::classify(&pages, &config, &templates)));
}

#[bench]
fn f_render_pages(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    let sources = Sources::load(&config).unwrap();
    let pages = parse_sources(&sources, &config);
    let templates = Templates::load(&config).unwrap();
    let taxonomies = Taxonomy::classify(&pages, &config, &templates).unwrap();
    b.iter(|| {
        let rendered = MutSet::default();
        black_box(
            pages.par_iter().try_for_each(|page| {
                page.render(&pages, &templates, &config, &taxonomies, &rendered)
            }),
        )
    });
}

#[bench]
fn g_render_classification(b: &mut Bencher) {
    let file = std::fs::read(CONFIG).unwrap();
    let config = toml::from_slice::<Config>(&file).unwrap();
    let sources = Sources::load(&config).unwrap();
    let pages = parse_sources(&sources, &config);
    let templates = Templates::load(&config).unwrap();
    let taxes = Taxonomy::classify(&pages, &config, &templates).unwrap();
    b.iter(|| {
        let rendered = MutSet::default();
        black_box(taxes.par_iter().try_for_each(|(_, tax)| {
            tax.render(&config, &taxes, &pages, &rendered)?;
            tax.keys().par_iter().try_for_each(|(name, tagged)| {
                tax.render_key(name, tagged, &config, &taxes, &pages, &rendered)
            })
        }))
    });
}
