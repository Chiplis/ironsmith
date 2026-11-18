//! Counts the grammar's string reference keys: the sites where the recognizer
//! mints or compares a `TagKey` (the string identity a semantic reference
//! carries today) instead of binding a scoped symbol. The repair order's item
//! 6 makes `SymbolId` the only semantic reference identity; this audit is its
//! instrument, and the ratchet reads the total.
//!
//! A site is a string-identity operation on a reference: `.key()` on a
//! reference tag enum (minting a key without binding its symbol),
//! `.as_str()` (comparing keys as text), `is_sentence_helper_tag(`,
//! `TagKey::new(` or `TagKey::from(`. Binding (`.bind()`, `helper_tag_for_tokens`)
//! is not counted: it declares the symbol. Comments, literals and inline test
//! modules are blanked first. `--by-file` prints the per-file counts.

use std::fs;
use std::path::{Path, PathBuf};

mod parser_audit_modules;
mod parser_audit_source;
mod tooling_paths;

const KEY_SITE_PATTERNS: &[&str] = &[
    ".key()",
    ".as_str()",
    "is_sentence_helper_tag(",
    "TagKey::new(",
    "TagKey::from(",
    "TagRef::of(",
];

/// Key sites on one line: every pattern counts, except that `.as_str()` is a
/// key site only where the line also speaks of a tag (plain strings are
/// compared as text everywhere else in the grammar).
fn key_sites_on_line(line: &str) -> usize {
    let tag_line = line.contains("tag") || line.contains("Tag");
    KEY_SITE_PATTERNS
        .iter()
        .filter(|pattern| **pattern != ".as_str()" || tag_line)
        .map(|pattern| line.matches(pattern).count())
        .sum()
}

fn main() {
    let mut by_file = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--by-file" => by_file = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|error| panic!("failed to locate repo root: {error}"));
    let modules = parser_audit_modules::tracked_production_modules(&repo_root)
        .unwrap_or_else(|error| panic!("failed to enumerate production parser modules: {error}"));
    let mut per_file: Vec<(PathBuf, usize)> = Vec::new();
    let mut total = 0usize;
    for relative in &modules {
        if !is_grammar_module(relative) {
            continue;
        }
        let path = repo_root.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let text = parser_audit_source::production_source(&source);
        let count: usize = text.lines().map(key_sites_on_line).sum();
        if count > 0 {
            per_file.push((relative.clone(), count));
            total += count;
        }
    }
    per_file.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    println!(
        "grammar modules covered: {}",
        modules.iter().filter(|m| is_grammar_module(m)).count()
    );
    println!("modules with key sites: {}", per_file.len());
    println!("reference key sites: {total}");
    if by_file {
        for (path, count) in &per_file {
            println!("{count:6}  {}", path.display());
        }
    }
}

fn is_grammar_module(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.starts_with("crates/ironsmith-compiler-grammar/")
        || normalized.starts_with("crates/ironsmith-grammar-common/")
}
