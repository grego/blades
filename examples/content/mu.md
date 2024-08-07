+++
title = "Mu"
date = 2020-09-18
image = "img/mu.jpg"
summary = """A monk asked Joshu, "Has the dog the Buddha nature?" """

[taxonomies]
tags = ["pets"]
+++
A monk asked Joshu, "Has the dog the Buddha nature?"[^cool]
Joshu replied, "Mu"

$$\sum_{n=0}^\infty \frac{1}{n^2}$$

### Mumon's Comment:
> For the pursuit of Zen, you must pass through the barriers (gates) set up by the Zen masters. To attain his mysterious awareness one must completely uproot all the normal workings of one's mind. If you do not pass through the barriers, nor uproot the normal workings of your mind, whatever you do and whatever you think is a tangle of ghost. Now what are the barriers? This one word "Mu" is the sole barrier. This is why it is called the Gateless Gate of Zen. The one who passes through this barrier shall meet with Joshu face to face and also see with the same eyes, hear with the same ears and walk together in the long train of the patriarchs. Wouldn't that be pleasant?  
> Would you like to pass through this barrier? Then concentrate your whole body, with its 360 bones and joints, and 84,000 hair follicles, into this question of what "Mu" is; day and night, without ceasing, hold it before you. It is neither nothingness, nor its relative "not" of "is" and "is not." It must be like gulping a hot iron ball that you can neither swallow nor spit out.

```rust
#[inline]
pub(crate) fn render_content<E: Encoder>(source: &str, encoder: &mut E) -> Result<(), E::Error> {
    let parser = pulldown_cmark::Parser::new_ext(source, pulldown_cmark::Options::all());
    let processed = cmark_syntax::SyntaxPreprocessor::new(parser);
    encoder.write_html(processed)
}
```

[^cool]: This is a really cool story.
