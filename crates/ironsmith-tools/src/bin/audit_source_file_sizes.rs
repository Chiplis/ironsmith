use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const HARD_LIMIT: usize = 5_000;
const CHECKED_ROOTS: &[&str] = &["crates", "scripts", "web/ui/src", "web/ui/tests"];
const CHECKED_EXTENSIONS: &[&str] = &[
    "c", "cc", "cpp", "css", "go", "h", "hpp", "java", "js", "jsx", "kt", "mjs", "py", "rs", "sh",
    "ts", "tsx",
];

// Delete entries as the corresponding refactors land. Entries are ratchets: an
// oversized file may shrink, but it may never grow beyond the recorded baseline.
const TEMPORARY_RATCHETS: &[(&str, usize)] = &[];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn collect_files(root: &Path, output: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_files(&path, output);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| CHECKED_EXTENSIONS.contains(&extension))
        {
            output.push(path);
        }
    }
}

fn main() {
    let root = workspace_root();
    let ratchets = TEMPORARY_RATCHETS
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    let mut files = Vec::new();
    for checked_root in CHECKED_ROOTS {
        collect_files(&root.join(checked_root), &mut files);
    }
    files.sort();

    let mut failures = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .expect("checked file below workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let lines = source.lines().count();
        let limit = ratchets
            .get(relative.as_str())
            .copied()
            .unwrap_or(HARD_LIMIT);
        if lines > limit {
            failures.push((relative, lines, limit));
        }
    }

    for ratcheted_path in ratchets.keys() {
        if !root.join(ratcheted_path).is_file() {
            failures.push(((*ratcheted_path).to_string(), 0, HARD_LIMIT));
        }
    }

    if failures.is_empty() {
        println!(
            "source-size audit passed (hard limit {HARD_LIMIT}, {} temporary ratchets)",
            ratchets.len()
        );
        return;
    }

    eprintln!("source-size audit failures:");
    for (path, lines, limit) in failures {
        if lines == 0 {
            eprintln!("  stale ratchet for removed file: {path}");
        } else {
            eprintln!("  {path}: {lines} lines > {limit}");
        }
    }
    std::process::exit(1);
}
