use blades::Page;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut source = Vec::new();
    std::io::stdin().read_to_end(&mut source)?;
    // When deserializing from a slice, zero-copy deserialiation can be used
    let mut pages: Vec<Page> = serde_json::from_slice(&source)?;

    for page in &mut pages {
        if page.content.find("dog").is_some() {
            page.summary = "WARNING! CONTAINS DOGS!".into();
        }
    }

    serde_json::to_writer(std::io::stdout(), &pages)?;
    Ok(())
}
