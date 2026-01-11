#!/usr/bin/env rust-script
//! Test extraction on a single HTML file and show debug output

use std::env;
use std::fs;
use std::io::Read;
use flate2::read::GzDecoder;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --example test_single <html_file.gz>");
        return;
    }
    
    let file = fs::File::open(&args[1]).expect("Failed to open file");
    let mut decoder = GzDecoder::new(file);
    let mut html = String::new();
    decoder.read_to_string(&mut html).expect("Failed to decompress");
    
    let result = rs_trafilatura::extract(&html);
    match result {
        Ok(r) => {
            println!("=== EXTRACTION RESULT ===");
            println!("Content length: {} chars", r.content_text.len());
            println!("Content preview: {}...", &r.content_text[..500.min(r.content_text.len())]);
            println!("\n=== METADATA ===");
            println!("Title: {:?}", r.metadata.title);
            println!("Author: {:?}", r.metadata.author);
        }
        Err(e) => {
            eprintln!("Extraction failed: {e:?}");
        }
    }
}
