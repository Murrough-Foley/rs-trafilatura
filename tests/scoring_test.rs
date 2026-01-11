use rs_trafilatura::extract;

#[test]
fn extract_penalizes_link_dense_regions() {
    let link_block = (0..30)
        .map(|i| format!("<p><a href='#'>LINK_TEXT_{i}_CLICK_HERE</a></p>"))
        .collect::<String>();

    let sentence = "This is a substantive sentence with meaningful words.";
    let para = sentence.repeat(15);

    let html = format!(
        r#"
        <html><body>
            <div id="maintext">{link_block}</div>
            <div id="storytext">
                <h2>HEADING_MARKER</h2>
                <p>SUBSTANTIVE_MARKER {para}</p>
                <p>{para}</p>
                <p>{para}</p>
            </div>
        </body></html>
    "#
    );

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("SUBSTANTIVE_MARKER"));
            assert!(!result.content_text.contains("LINK_TEXT_0"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_selects_deeply_nested_content_node() {
    let inner_sentence = "This is a substantive sentence with meaningful words.";
    let inner_para = inner_sentence.repeat(20);

    let html = format!(
        r#"
        <html><body>
            <div id="maintext">
                OUTER_NOISE_MARKER
                <div>
                    <div>
                        <div>
                            <div>
                                <div>
                                    <p>INNER_MARKER {inner_para}</p>
                                    <p>{inner_para}</p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </body></html>
    "#
    );

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("INNER_MARKER"));
            assert!(!result.content_text.contains("OUTER_NOISE_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_rewards_sentence_rich_regions() {
    let wordy_block = "WORD ".repeat(400);
    let sentence_block = "This is a sentence.".repeat(120);

    let html = format!(
        r#"
        <html><body>
            <div id="maintext">
                <p>WORDY_MARKER {wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
                <p>{wordy_block}</p>
            </div>
            <div id="storytext">
                <p>SENTENCE_RICH_MARKER {sentence_block}</p>
                <p>{sentence_block}</p>
                <p>{sentence_block}</p>
            </div>
        </body></html>
    "#
    );

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("SENTENCE_RICH_MARKER"));
            assert!(!result.content_text.contains("WORDY_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_rewards_heading_proximity() {
    let plain = "PLAINWORD ".repeat(600);

    let html = format!(
        r#"
        <html><body>
            <div id="maintext">
                <p>PLAIN_MARKER {plain}</p>
                <p>{plain}</p>
                <p>{plain}</p>
                <p>{plain}</p>
            </div>
            <div id="storytext">
                <h2>HEADING_BONUS_MARKER</h2>
                <p>{plain}</p>
                <p>{plain}</p>
                <p>{plain}</p>
                <p>{plain}</p>
            </div>
        </body></html>
    "#
    );

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("HEADING_BONUS_MARKER"));
            assert!(!result.content_text.contains("PLAIN_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_prefers_substantive_paragraphs() {
    let short = "Short sentence.".repeat(3);
    let long = "LONGWORD ".repeat(150);

    let html = format!(
        r#"
        <html><body>
            <div id="maintext">
                <p>SHORT_REGION_MARKER</p>
                <p>{short}</p>
                <p>{short}</p>
                <p>{short}</p>
                <p>{short}</p>
                <p>{short}</p>
                <p>{short}</p>
                <p>{short}</p>
                <p>{short}</p>
                <p>{short}</p>
            </div>
            <div id="storytext">
                <p>LONG_REGION_MARKER</p>
                <p>{long}</p>
                <p>{long}</p>
            </div>
        </body></html>
    "#
    );

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("LONG_REGION_MARKER"));
            assert!(!result.content_text.contains("SHORT_REGION_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_degrades_gracefully_when_selected_node_filters_to_empty() {
    let noise = "NOISE ".repeat(2000);

    let html = format!(
        r#"
        <html><body>
            <div id="maintext"><nav>{noise}</nav></div>
            <div><p>REAL_CONTENT_MARKER</p></div>
        </body></html>
    "#
    );

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("REAL_CONTENT_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
