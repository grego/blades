<img src="https://raw.githubusercontent.com/grego/blades/master/logo.svg?sanitize=true" alt="Blades logo" width="250" align="right">

# Blades

[![Crates.io status](https://badgen.net/crates/v/blades)](https://crates.io/crates/blades)
[![Docs](https://docs.rs/blades/badge.svg)](https://docs.rs/blades)

```
blazing fast
 dead simple
  static site generator
```

[User manual](http://www.getblades.org)

Blades is made to do one job and do it well - generate HTML files from the provided
content using the provided templates.  
Thanks to [zero-copy](https://serde.rs/lifetimes.html#borrowing-data-in-a-derived-impl) deserialization
and the [Ramhorns](https://github.com/maciejhirsz/ramhorns) templating engine,
it renders the whole site in milliseconds, possibly more than
[20 times](https://github.com/grego/ssg-bench) faster than other generators like Hugo.

It's made for easy setup and use. A static site generator should be a no brainer.
It uses [mustache](https://mustache.github.io/mustache.5.html) templates with extremely minimal
and obvious syntax (like 7 rules!), providing the necessary building blocks
to let you focus on your content.

## Features
* Powerful plugin system
* Themes
* Image gallery generation
* [CommonMark](https://commonmark.org) markdown with tables and footnotes for content
* Automatic syntax highlighting using [cmark-syntax](https://github.com/grego/cmark-syntax)
  (with a possibility of turning LaTeX formulas into [MathML](https://developer.mozilla.org/docs/Web/MathML))
* Customizable taxonomies (like categories or tags)
* Pagination
* Breadcrumbs
* Asset colocation
* Table of contents with access to all of the site data
* Automatic sitemap, Atom and RSS feed generation

## Why not _`blades`_?
Unlike other monolithic generators, Blades is modest in scope. All it does is to generate a site.
It doesn't do any fancy stuff like transpiling Haskell to minified Javascript, or ever
watching the site for changes. For that, you can use a dedicated tool like
[caretaker](https://github.com/grego/caretaker).

Nevertheless, if you have a feature request or ran into some issue using Blades, please submit an
[issue](https://github.com/grego/blades). Any contribution is welcome! `:)`

## Why _`blades`_?
They shave the [mustache](https://mustache.github.io/mustache.5.html) off.

## Installing
With the Rust toolchain installed, you can install Blades from [crates.io](https://crates.io/crates/blades)
```bash
cargo install blades
```

Or from its repository
```bash
git clone https://github.com/grego/blades
cd blades
cargo install --path .
```

## macOS
Using the package manager [MacPorts](https://www.macports.org)
```bash
sudo port install blades
```

## Running
Then, you can run the executable `blades` with the following subcommands:
* `init`: Initialize the site in the current directory, creating the basic files and folders
* `build`: Build the site according to config, content, templates and themes in the current directory
* `colocate`: Move the assets from the "assets" directory and from the theme, if one is used, into the output directory
* `all`: Build the site and colocate the assets
* `lazy`: Build the site and (colocate assets only if the theme was switched) [default]
* `new`: Create a new page

## Plugins
There are 4 types of plugins that can be used with Blades.
* **input** - they put a JSON-serialised list of [pages](https://www.getblades.org/pages.html) on the standard output, can be used
  to get pages from different sources
* **output** - they receive a JSON-serialised list of [pages](https://www.getblades.org/pages.html) on the standard input and can be
  used to generate further page data, such as processing images
* **transform** - they receive a JSON-serialised list of [pages](https://www.getblades.org/pages.html) on the standard output and output
  another such list on the standard output, can transform anything on the pages
* **content** - they receive a markdown content of one page on standard input and output markdown on the standard output; they are enabled
  on per-page basis and can be used e.g. to render LaTeX formulas or highlight syntax

Any code in any language can be used, as only using the standard input and output is assumed. For Rust, Blades also provides a
[library](https://docs.rs/blades) for automatic serialisation and deserialisation pages.

### Example
Example plugin configuration can be found in [examples](examples/Blades.toml), as well as an
example toy [transform plugin](examples/transform_plugin.rs).
To try it, first downolad the [Casper](https://blades-casper.netlify.app/) theme as a submodule
```bash
git submodule update --init
```
Then build the plugin:
```bash
cargo build --release transform_plugin
```
Then run Blades in the `examples` directory:
```bash
cargo run --release
```

For more on plugins, check their [documentation](https://www.getblades.org/making-plugins.html) and
[existing plugins](https://www.getblades.org/plugins/)

## Themes
When you specify a theme in the [config](https://www.getblades.org/config.html), templates and assets from the theme are used.
Every site that doesn't use a theme can be used as a theme for another site.
Therefore, the easiest way to use a theme is to just clone the corresponding theme's repository
into the `themes` directory. A list of available themes can be found [here](https://www.getblades.org/themes/).

To overwrite the theme, simply use the files in the `templates`, resp. `assets` subdirectories of the
page root directory.

## Assets
All the files from the `assets` directory (and from the theme) are moved into the directory
specified in the [config](https://www.getblades.org/config.html), which is emptied before. This is a subdirectory of the
output directory (defaults to `assets`).

Blades takes of the pages it rendered before and if some of them is deleted, the corresponding
files in the output directory will be deleted, too. The other files in the output directory
are left intact. This way, you can place anything in the output directory and (as long as its name
differs from all the page names and it's not in the assets subdirectory), Blades won't touch it.

## Meta
Blades renders [sitemap](https://www.sitemaps.org) (into `sitemap.xml`), [Atom](https://en.wikipedia.org/wiki/Atom_(Web_standard)) (into `atom.xml`)
and [RSS](https://en.wikipedia.org/wiki/RSS) (into `rss.xml`) feeds, unless explicitly disabled in the [config](https://www.getblades.org/config.html).

## Using Blades as a library
Main components of Blades are also exported as a library. They are parser agnostic, so they can be used
to generate a website using any format that implements `serde::Deserialize`.
Currently, Cargo doesn't support binary-only dependencies. As such, these dependencies are behind
the `bin` feature gate, which is enabled by default. When using Blades as a library, they are not
necessary, so it is recommended to import blades with `default_features = false`.

## Contribution
If you found a bug or would like to see some feature in Blades, you are the most welcome to submit an issue
or a pull request! Likewise if you found something in this documentation not clear or imprecise.

## License
Blades is free software, and is released under the terms of the GNU General Public
License version 3. See [LICENSE](LICENSE).
