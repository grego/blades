<img src="https://raw.githubusercontent.com/grego/blades/master/logo.svg?sanitize=true" alt="Blades logo" width="250" align="right">

```
blazing fast
 dead simple
  static site generator
```

[![Crates.io status](https://badgen.net/crates/v/blades)](https://crates.io/crates/blades)
[![Docs](https://docs.rs/blades/badge.svg)](https://docs.rs/blades)

[User manual](https://blades.github.io)

Blades is made to do one job and do it well - generate HTML files from the provided
content using the provided templates.  
Thanks to the amazing [Ramhorns](https://github.com/maciejhirsz/ramhorns) templating engine,
[zero-copy](https://serde.rs/lifetimes.html#borrowing-data-in-a-derived-impl) deserialisation
and [rayon](https://github.com/rayon-rs/rayon) parallel iterators, it renders the whole site in
milliseconds, possibly up to [10 times](https://github.com/grego/ssg-bench) faster than other generators like Hugo.

It's made for easy setup and use. A static site generator should be a no brainer.
It uses [mustache](https://mustache.github.io/mustache.5.html) templates with extremely minimal
and obvious syntax (really, like 7 rules!), while providing the necessary building blocks
to let you focus on your content.

You may categorise your pages into taxonomies (like categories or tags), use the usual features
like pagination or breadcrumbs and even generate image galleries.  
Sitemap and RSS feed are automatically rendered.  
All done without hassle, faster than you blink.

## Why not _`blades`_?
Unlike other monolithic generators, Blades is modest in its scope. All it does is generating your site.
It doesn't do any fancy stuff like transpiling Haskell to minified Javascript, or ever
watching the site for changes. For that, you can use a dedicated tool like
[caretaker](https://github.com/grego/caretaker).

Nevertheless, if you have a feature request or ran into some issue using Blades, please submit an
[issue](https://github.com/grego/blades). It is a hobby project, so any contribution
is welcome! `:)`

## Why _`blades`_?
They shave the [mustache](https://mustache.github.io/mustache.5.html) off.

## Installing

If you have the Rust toolchain installed, you can install Blades from [Crates.io](https://crates.io/crates/blades)
``` bash
cargo install blades
```
If you would like to have it included in your favourite package repository, [submit an issue](https://github.com/grego/blades).

Then, you can run the executable `blades` with the following subcommands:
* `init`: Initialise the site in the current directory, creating the basic files and folders
* `build`: Build the site according to config, content, templates and themes in the currend directory
* `colocate`: Move the assets from the "assets" directory and from the theme, if one is used, into the output directory
* `all`: Build the site and colocate the assets
* `lazy`: Build the site and (colocate assets only if the theme was switched) [default]
* `new`: Create a new page

## Themes
When you specify a theme in the [config](config.html), templates and assets from the theme are used.
Every site that doesn't use a theme can be used as a theme for another site.
Therefore, the easiest way to use a theme is to just clone the corresponding theme's repository
into the `themes` directory. A list of available themes can be found [here](/themes).

To overwrite the theme, simply use the files in the `templates`, resp. `assets` subdirectories of the
page root directory.

## Assets
All the files from the `assets` directory (and from the theme) are moved into the directory
specified in the [config](config.html), which is emptied before. This is a subdirectory of the
output directory (defaults to `assets`).

Blades takes of the pages it rendered before and if some of them is deleted, the corresponding
files in the output directory will be deleted, too. The other files in the output directory
are left intact. This way, you can place anything in the output directory and (as long as its name
differs from all the page names and it's not in the assets subdirectory), Blades won't touch it.

## Meta
Blades renders [sitemap](https://www.sitemaps.org) (into `sitemap.xml`), [Atom](https://en.wikipedia.org/wiki/Atom_(Web_standard)) (into `atom.xml`)
and [RSS](https://en.wikipedia.org/wiki/RSS) (into `rss.xml`) feeds, unless explicitly disabled in the [config](config.html).

## Contribution
If you found a bug or would like to see some feature in Blades, you are the most welcome to submit an issue
or a pull request! Likewise if you found something in this documentation not clear or imprecise.

## License

Blades is free software, and is released under the terms of the GNU General Public
License version 3. See [LICENSE](LICENSE).
