use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use url::Url;

fn html_unescape_basic(s: &str) -> String {
    let s = s.replace("&amp;", "&");
    let s = s.replace("&quot;", "\"");
    let s = s.replace("&#34;", "\"");
    let s = s.replace("&apos;", "'");
    let s = s.replace("&#39;", "'");
    let s = s.replace("&lt;", "<");
    s.replace("&gt;", ">")
}

fn normalize_url_candidate(raw: &str) -> Option<Url> {
    let mut s = html_unescape_basic(raw);
    let s_trimmed = s.trim();
    s = s_trimmed.to_string();

    while s.ends_with(['\"', '\'', ')', ']', '}', '>', ',', '.', ';', ':']) {
        s.pop();
    }

    while s.starts_with(['\"', '\'']) {
        s.remove(0);
    }

    let url = Url::parse(&s).ok()?;
    if (url.scheme() != "http" && url.scheme() != "https") || url.host_str().is_none() {
        return None;
    }

    Some(url)
}

fn normalize_domain(host: &str) -> String {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    host.strip_prefix("www.").unwrap_or(&host).to_string()
}

fn collect_files_rec(dir: &Path, acc: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)?.collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files_rec(&path, acc)?;
        } else if path.is_file() {
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name == "urls_prededup.txt" || file_name == "urls_dedup.txt" {
                continue;
            }

            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let ext = ext.to_ascii_lowercase();
            if ext == "txt" || ext == "html" || ext == "htm" {
                acc.push(path);
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let input_dir = args.next().unwrap_or_else(|| "collect_html_files".to_string());

    let input_dir = PathBuf::from(input_dir);
    if !input_dir.exists() {
        return Err(format!("Input dir does not exist: {}", input_dir.display()).into());
    }

    let prededup_path = input_dir.join("urls_prededup.txt");
    let dedup_path = input_dir.join("urls_dedup.txt");

    let url_re = Regex::new(r#"https?://[^\s"'<>]+"#)?;

    let mut files = Vec::new();
    collect_files_rec(&input_dir, &mut files)?;

    let prededup_file = fs::File::create(&prededup_path)?;
    let dedup_file = fs::File::create(&dedup_path)?;

    let mut prededup_writer = BufWriter::new(prededup_file);
    let mut dedup_writer = BufWriter::new(dedup_file);

    let mut domains_seen: HashSet<String> = HashSet::new();

    let mut url_total: u64 = 0;
    let mut url_valid: u64 = 0;
    let mut url_domains_unique: u64 = 0;

    for path in files {
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to read {}: {e}", path.display());
                continue;
            }
        };

        let content = String::from_utf8_lossy(&bytes);
        for m in url_re.find_iter(&content) {
            url_total = url_total.saturating_add(1);
            let url = match normalize_url_candidate(m.as_str()) {
                Some(u) => u,
                None => continue,
            };
            url_valid = url_valid.saturating_add(1);

            writeln!(prededup_writer, "{}", url.as_str())?;

            if let Some(host) = url.host_str() {
                let domain = normalize_domain(host);
                if domains_seen.insert(domain) {
                    url_domains_unique = url_domains_unique.saturating_add(1);
                    writeln!(dedup_writer, "{}", url.as_str())?;
                }
            }
        }
    }

    prededup_writer.flush()?;
    dedup_writer.flush()?;

    eprintln!("Wrote: {}", prededup_path.display());
    eprintln!("Wrote: {}", dedup_path.display());
    eprintln!("Total URL-like matches: {url_total}");
    eprintln!("Valid http(s) URLs written to prededup: {url_valid}");
    eprintln!("Unique domains written to dedup: {url_domains_unique}");

    Ok(())
}
