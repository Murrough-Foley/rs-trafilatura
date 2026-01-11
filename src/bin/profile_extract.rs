/// Profiling binary for flamegraph analysis
/// Processes multiple HTML files to generate sufficient samples
use rs_trafilatura::extract;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: profile_extract <html_dir> [limit]");
        eprintln!("  html_dir: Directory containing HTML files");
        eprintln!("  limit: Optional max number of files to process (default: all)");
        std::process::exit(1);
    }

    let dir = &args[1];
    let limit: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "html")
        })
        .collect();

    entries.sort_by_key(|e| e.path());

    let total = entries.len().min(limit);
    eprintln!("Processing {} HTML files from {}", total, dir);

    let mut success = 0;
    let mut failed = 0;
    let mut total_bytes = 0usize;

    for entry in entries.into_iter().take(limit) {
        let path = entry.path();
        match fs::read_to_string(&path) {
            Ok(html) => {
                total_bytes += html.len();
                match extract(&html) {
                    Ok(_result) => {
                        success += 1;
                    }
                    Err(_e) => {
                        failed += 1;
                    }
                }
            }
            Err(_e) => {
                failed += 1;
            }
        }
    }

    eprintln!(
        "Processed {} files ({} MB): {} success, {} failed",
        total,
        total_bytes / 1_000_000,
        success,
        failed
    );

    Ok(())
}
