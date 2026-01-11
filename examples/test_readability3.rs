use dom_smoothie::Readability;
use std::fs;

fn main() {
    let html = fs::read_to_string(
        "/home/foley/Documents/rs-trafilatura/benchmarks/web-content-extraction-benchmark/datasets/combined/html/cleaneval/0490d8a5dfe65ffee2750495a86d1acd0af932a8a67d91c4b6b30d567e317598.html"
    ).unwrap();

    match Readability::new(html.clone(), None, None) {
        Ok(mut reader) => {
            match reader.parse() {
                Ok(article) => {
                    let text: String = article.text_content.into();
                    println!("Readability extracted {} chars", text.len());
                    println!("\nFirst 500 chars:\n{}", &text[..text.len().min(500)]);
                }
                Err(e) => println!("Parse error: {e:?}"),
            }
        }
        Err(e) => println!("Init error: {e:?}"),
    }
}
