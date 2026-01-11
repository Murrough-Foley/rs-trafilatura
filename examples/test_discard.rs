use rs_trafilatura::selector::discard;
use rs_trafilatura::dom;

fn main() {
    // Test if MenuItem class would be discarded
    let doc = dom::parse(r#"<p class="MenuItem">Canberra</p>"#);
    let p = doc.select("p");

    println!("Testing <p class='MenuItem'>...");
    println!("Rule 1 matches: {}", discard::overall_discarded_content_rule_1(&p));
    println!("Rule 2 matches: {}", discard::overall_discarded_content_rule_2(&p));
    println!("Should discard: {}", discard::should_discard(&p));
}
