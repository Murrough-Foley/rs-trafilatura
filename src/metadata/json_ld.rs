//! JSON-LD Metadata Parsing
//!
//! This module ports JSON-LD metadata parsing from go-trafilatura's metadata-json.go.
//! It extracts structured metadata from Schema.org JSON-LD embedded in HTML documents.

use dom_query::{Document, Selection};
use serde_json::Value;
use crate::dom;
use crate::result::Metadata;
use crate::Options;

/// Schema data container with importance scoring.
///
/// Go equivalent: `SchemaData` struct (lines 14-21)
#[derive(Debug, Clone)]
pub struct SchemaData {
    /// Schema @type values
    pub types: Vec<String>,
    /// Raw schema data
    pub data: serde_json::Map<String, Value>,
    /// Importance score (higher = more relevant)
    pub importance: i32,
    /// Parent schema reference (for hierarchy tracking)
    pub parent: Option<Box<SchemaData>>,
}

/// Extract metadata from JSON-LD scripts.
///
/// Go equivalent: `extractJsonLd(opts, doc, originalMetadata)` (lines 23-91)
///
/// # Arguments
/// * `doc` - The HTML document
/// * `original` - Pre-existing metadata to merge with
/// * `opts` - Extraction options
///
/// # Returns
/// * Updated metadata with JSON-LD values merged
#[must_use]
pub fn extract_json_ld(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    // Decode all JSON-LD scripts
    let (persons, organizations, articles) = decode_json_ld(doc);

    // Extract author from persons
    if result.author.is_none() {
        for person in &persons {
            if let Some(name) = get_schema_names(&person.data, "Person", "Author") {
                result.author = Some(name);
                break;
            }
        }
    }

    // Extract sitename from organizations
    if result.sitename.is_none() {
        for org in &organizations {
            if let Some(name) = get_schema_names(&org.data, "Organization", "WebSite") {
                result.sitename = Some(name);
                break;
            }
        }
    }

    // Extract from articles (title, description, categories, etc.)
    for article in &articles {
        if result.title.is_none() {
            if let Some(title) = get_single_string_value(&article.data, "headline") {
                result.title = Some(title);
            } else if let Some(title) = get_single_string_value(&article.data, "name") {
                result.title = Some(title);
            }
        }

        if result.description.is_none() {
            if let Some(desc) = get_single_string_value(&article.data, "description") {
                result.description = Some(desc);
            }
        }

        // Extract date
        if result.date.is_none() {
            if let Some(date_str) = get_single_string_value(&article.data, "datePublished") {
                if let Ok(date) = parse_json_ld_date(&date_str) {
                    result.date = Some(date);
                }
            }
        }

        // Extract image
        if result.image.is_none() {
            if let Some(image) = extract_schema_image(&article.data) {
                result.image = Some(image);
            }
        }

        // Extract categories/keywords
        if result.categories.is_empty() {
            if let Some(keywords) = get_string_values(&article.data, "keywords") {
                result.categories = keywords;
            }
        }
    }

    result
}

/// Parse and categorize JSON-LD scripts into persons, organizations, and articles.
///
/// Go equivalent: `decodeJsonLd(doc, opts)` (lines 93-189)
fn decode_json_ld(
    doc: &Document,
) -> (Vec<SchemaData>, Vec<SchemaData>, Vec<SchemaData>) {
    let mut persons: Vec<SchemaData> = Vec::new();
    let mut organizations: Vec<SchemaData> = Vec::new();
    let mut articles: Vec<SchemaData> = Vec::new();

    // Find all JSON-LD scripts
    for script in doc.select(r#"script[type="application/ld+json"]"#).nodes() {
        let script_sel = Selection::from(*script);
        let json_text = dom::text_content(&script_sel).trim().to_string();

        if json_text.is_empty() {
            continue;
        }

        // Parse JSON
        let data: Value = match serde_json::from_str(&json_text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Process the schema(s)
        process_schema_value(&data, None, 0, &mut persons, &mut organizations, &mut articles);
    }

    // Sort by importance (higher first)
    persons.sort_by(|a, b| b.importance.cmp(&a.importance));
    organizations.sort_by(|a, b| b.importance.cmp(&a.importance));
    articles.sort_by(|a, b| b.importance.cmp(&a.importance));

    (persons, organizations, articles)
}

/// Recursively process schema values.
fn process_schema_value(
    value: &Value,
    parent: Option<&SchemaData>,
    depth: i32,
    persons: &mut Vec<SchemaData>,
    organizations: &mut Vec<SchemaData>,
    articles: &mut Vec<SchemaData>,
) {
    match value {
        Value::Object(map) => {
            // Check if this is a schema object
            let types = get_schema_types_from_value(value, true);

            if types.is_empty() {
                // Not a typed schema, recurse anyway
                for (_, val) in map {
                    process_schema_value(val, parent, depth, persons, organizations, articles);
                }
            } else {
                let importance = calculate_importance(&types, parent, depth);
                let schema_data = SchemaData {
                    types: types.clone(),
                    data: map.clone(),
                    importance,
                    parent: parent.map(|p| Box::new(p.clone())),
                };

                // Categorize by type
                if is_person_type(&types) {
                    persons.push(schema_data.clone());
                } else if is_organization_type(&types) {
                    organizations.push(schema_data.clone());
                } else if is_article_type(&types) {
                    articles.push(schema_data.clone());
                }

                // Recurse into nested objects
                for (_, val) in map {
                    process_schema_value(val, Some(&schema_data), depth + 1, persons, organizations, articles);
                }
            }
        }
        Value::Array(arr) => {
            // Handle @graph arrays
            for item in arr {
                process_schema_value(item, parent, depth, persons, organizations, articles);
            }
        }
        _ => {}
    }
}

/// Get @type values from a schema object.
///
/// Go equivalent: `getSchemaTypes(schema, toLower)` (lines 268-290)
fn get_schema_types_from_value(value: &Value, to_lower: bool) -> Vec<String> {
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };

    let Some(type_val) = obj.get("@type") else {
        return Vec::new();
    };

    let mut types = Vec::new();

    match type_val {
        Value::String(s) => {
            let t = if to_lower { s.to_lowercase() } else { s.clone() };
            types.push(t);
        }
        Value::Array(arr) => {
            for item in arr {
                if let Value::String(s) = item {
                    let t = if to_lower { s.to_lowercase() } else { s.clone() };
                    types.push(t);
                }
            }
        }
        _ => {}
    }

    types
}

/// Extract names from a schema object.
///
/// Go equivalent: `getSchemaNames(v, expectedTypes...)` (lines 191-266)
fn get_schema_names(data: &serde_json::Map<String, Value>, _expected_types: &str, _alt_type: &str) -> Option<String> {
    // Try "name" field
    if let Some(Value::String(name)) = data.get("name") {
        let name = name.trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }

    // Try composed name (givenName + familyName)
    let given = data.get("givenName").and_then(|v| v.as_str()).unwrap_or("");
    let family = data.get("familyName").and_then(|v| v.as_str()).unwrap_or("");

    if !given.is_empty() || !family.is_empty() {
        let full_name = format!("{} {}", given.trim(), family.trim()).trim().to_string();
        if !full_name.is_empty() {
            return Some(full_name);
        }
    }

    None
}

/// Get string values from an object property.
///
/// Go equivalent: `getStringValues(obj, key)` (lines 292-314)
fn get_string_values(data: &serde_json::Map<String, Value>, key: &str) -> Option<Vec<String>> {
    let value = data.get(key)?;

    let mut result = Vec::new();

    match value {
        Value::String(s) => {
            let s = s.trim();
            if !s.is_empty() {
                // Split by comma if contains multiple
                result.extend(s.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
            }
        }
        Value::Array(arr) => {
            for item in arr {
                if let Value::String(s) = item {
                    let s = s.trim();
                    if !s.is_empty() {
                        result.push(s.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Get a single string value from an object property.
///
/// Go equivalent: `getSingleStringValue(obj, key)` (lines 316-327)
fn get_single_string_value(data: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    let value = data.get(key)?;

    match value {
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() { None } else { Some(s.to_string()) }
        }
        Value::Array(arr) => {
            arr.first().and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }
        _ => None,
    }
}

// === Helper Functions ===

fn is_person_type(types: &[String]) -> bool {
    types.iter().any(|t| matches!(t.as_str(), "person" | "author" | "reviewedby" | "creator"))
}

fn is_organization_type(types: &[String]) -> bool {
    types.iter().any(|t| matches!(t.as_str(), "organization" | "newsmediaorganization" | "website" | "publisher"))
}

fn is_article_type(types: &[String]) -> bool {
    types.iter().any(|t| matches!(
        t.as_str(),
        "article" | "newsarticle" | "blogposting" | "webpage" | "report"
        | "techarticle" | "scholarlyarticle" | "socialmediaposting"
    ))
}

fn calculate_importance(types: &[String], parent: Option<&SchemaData>, depth: i32) -> i32 {
    let base = if is_article_type(types) { 100 } else { 50 };
    let depth_penalty = depth * 10;
    let parent_bonus = if parent.is_some() { 5 } else { 0 };

    base - depth_penalty + parent_bonus
}

fn extract_schema_image(data: &serde_json::Map<String, Value>) -> Option<String> {
    if let Some(image) = data.get("image") {
        match image {
            Value::String(s) => return Some(s.clone()),
            Value::Object(obj) => {
                if let Some(Value::String(url)) = obj.get("url") {
                    return Some(url.clone());
                }
            }
            Value::Array(arr) => {
                if let Some(Value::String(s)) = arr.first() {
                    return Some(s.clone());
                }
                if let Some(Value::Object(obj)) = arr.first() {
                    if let Some(Value::String(url)) = obj.get("url") {
                        return Some(url.clone());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_json_ld_date(date_str: &str) -> Result<chrono::DateTime<chrono::Utc>, ()> {
    // Try ISO 8601 format
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.with_timezone(&chrono::Utc));
    }

    // Try other common formats
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt.and_utc());
    }

    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc());
    }

    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_article_schema() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@type": "Article",
                "headline": "Test Article Title",
                "description": "This is the article description.",
                "datePublished": "2024-01-15T10:30:00Z",
                "author": {
                    "@type": "Person",
                    "name": "John Doe"
                }
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_json_ld(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.title, Some("Test Article Title".to_string()));
        assert_eq!(metadata.description, Some("This is the article description.".to_string()));
        assert_eq!(metadata.author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_graph_array_schema() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@graph": [
                    {
                        "@type": "WebSite",
                        "name": "Example Site"
                    },
                    {
                        "@type": "NewsArticle",
                        "headline": "Breaking News"
                    }
                ]
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_json_ld(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.sitename, Some("Example Site".to_string()));
        assert_eq!(metadata.title, Some("Breaking News".to_string()));
    }

    #[test]
    fn test_person_name_composition() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@type": "Person",
                "givenName": "Jane",
                "familyName": "Smith"
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_json_ld(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.author, Some("Jane Smith".to_string()));
    }

    #[test]
    fn test_image_extraction_formats() {
        // Test string format
        let html1 = r#"<script type="application/ld+json">{"@type":"Article","image":"https://example.com/image.jpg"}</script>"#;
        let doc1 = Document::from(html1);
        let m1 = extract_json_ld(&doc1, Metadata::default(), &Options::default());
        assert_eq!(m1.image, Some("https://example.com/image.jpg".to_string()));

        // Test object format
        let html2 = r#"<script type="application/ld+json">{"@type":"Article","image":{"@type":"ImageObject","url":"https://example.com/image2.jpg"}}</script>"#;
        let doc2 = Document::from(html2);
        let m2 = extract_json_ld(&doc2, Metadata::default(), &Options::default());
        assert_eq!(m2.image, Some("https://example.com/image2.jpg".to_string()));
    }

    #[test]
    fn test_keywords_extraction() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@type": "Article",
                "headline": "Tech News",
                "keywords": ["technology", "innovation", "software"]
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_json_ld(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.categories, vec!["technology", "innovation", "software"]);
    }

    #[test]
    fn test_invalid_json_skipped() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            { invalid json here }
            </script>
            <script type="application/ld+json">
            {"@type": "Article", "headline": "Valid Article"}
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_json_ld(&doc, Metadata::default(), &Options::default());

        // Should extract from valid script, skip invalid
        assert_eq!(metadata.title, Some("Valid Article".to_string()));
    }

    #[test]
    fn test_preserves_original_metadata() {
        let html = r#"<script type="application/ld+json">{"@type":"Article","headline":"New Title"}</script>"#;

        let original = Metadata {
            author: Some("Original Author".to_string()),
            ..Metadata::default()
        };

        let doc = Document::from(html);
        let metadata = extract_json_ld(&doc, original, &Options::default());

        // Author preserved, title updated
        assert_eq!(metadata.author, Some("Original Author".to_string()));
        assert_eq!(metadata.title, Some("New Title".to_string()));
    }
}
