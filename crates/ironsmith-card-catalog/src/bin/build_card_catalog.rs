use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut cards_dir = None;
    let mut output = None;
    let mut quality = ironsmith_card_catalog::builder::DEFAULT_QUALITY;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cards-dir" => {
                cards_dir = Some(PathBuf::from(args.next().ok_or("missing cards directory")?))
            }
            "--output" => output = Some(PathBuf::from(args.next().ok_or("missing output path")?)),
            "--quality" => quality = args.next().ok_or("missing brotli quality")?.parse()?,
            "--help" | "-h" => {
                println!("build_card_catalog --cards-dir PATH --output PATH [--quality 0-11]");
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    let stats = ironsmith_card_catalog::builder::build_directory_with_quality(
        &cards_dir.ok_or("--cards-dir is required")?,
        &output.ok_or("--output is required")?,
        quality,
    )?;
    println!(
        "{} card catalog: {} routes, {} source bytes, {} deduplicated bytes, {} compressed bundle bytes",
        if stats.cached { "cached" } else { "built" },
        stats.routes,
        stats.source_bytes,
        stats.unique_bytes,
        stats.bundle_bytes
    );
    Ok(())
}
