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
use std::process::Command;

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
    let modules = tracked_production_modules(&repo_root)
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
        let text = blank_test_modules(&blank_comments_and_literals(&source));
        let count: usize = text.lines().map(key_sites_on_line).sum();
        if count > 0 {
            per_file.push((relative.clone(), count));
            total += count;
        }
    }
    per_file.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    println!("grammar modules covered: {}", modules.iter().filter(|m| is_grammar_module(m)).count());
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


/// Replace comments and string/char literal contents with spaces, keeping
/// every byte offset and newline where it was.
fn blank_comments_and_literals(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    out[i] = b' ';
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    if bytes[i] != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
                i += 2;
            }
            b'"' => {
                let raw_hashes = raw_string_hashes(bytes, i);
                i += 1;
                loop {
                    if i >= bytes.len() {
                        break;
                    }
                    if bytes[i] == b'\\' && raw_hashes.is_none() {
                        out[i] = b' ';
                        if i + 1 < bytes.len() && bytes[i + 1] != b'\n' {
                            out[i + 1] = b' ';
                        }
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        let closes = match raw_hashes {
                            Some(hashes) => bytes[i + 1..].iter().take_while(|byte| **byte == b'#').count() >= hashes,
                            None => true,
                        };
                        if closes {
                            break;
                        }
                    }
                    if bytes[i] != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
            b'\'' => {
                // a char literal is 'x' or '\x'; anything else is a lifetime
                if i + 2 < bytes.len() && bytes[i + 2] == b'\'' {
                    out[i + 1] = b' ';
                    i += 2;
                } else if i + 3 < bytes.len() && bytes[i + 1] == b'\\' && bytes[i + 3] == b'\'' {
                    out[i + 1] = b' ';
                    out[i + 2] = b' ';
                    i += 3;
                }
            }
            _ => {}
        }
        i += 1;
    }
    String::from_utf8(out).expect("blanking keeps the source valid UTF-8")
}

/// Blank every `#[cfg(test)] mod name { ... }` block to spaces.
fn blank_test_modules(text: &str) -> String {
    let mut out = text.as_bytes().to_vec();
    let mut at = 0usize;
    while let Some(offset) = text[at..].find("#[cfg(test)]") {
        let start = at + offset;
        at = start + 12;
        let rest = &text[at..];
        let trimmed = rest.trim_start();
        let Some(after_mod) = trimmed
            .strip_prefix("pub(crate) mod ")
            .or_else(|| trimmed.strip_prefix("pub(super) mod "))
            .or_else(|| trimmed.strip_prefix("pub mod "))
            .or_else(|| trimmed.strip_prefix("mod "))
        else {
            continue;
        };
        let Some(brace_offset) = after_mod.find('{') else {
            continue;
        };
        if after_mod[..brace_offset].contains(';') {
            continue;
        }
        let open = text.len() - after_mod.len() + brace_offset;
        let Some(close) = matching_brace(text.as_bytes(), open) else {
            continue;
        };
        for byte in &mut out[start..=close] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
        at = close + 1;
    }
    String::from_utf8(out).expect("blanking keeps the source valid UTF-8")
}

fn tracked_production_modules(repo_root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard", "--", "*.rs"])
        .output()
        .map_err(|error| format!("failed to run git ls-files: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let mut modules = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .filter(|path| is_parser_module(path) && !is_test_only(path))
        .filter(|path| repo_root.join(path).is_file())
        .collect::<Vec<_>>();
    modules.sort();
    modules.dedup();
    Ok(modules)
}

fn is_parser_module(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    (normalized.starts_with("crates/ironsmith-compiler")
        && !normalized.starts_with("crates/ironsmith-compiler-runtime/"))
        || normalized.starts_with("crates/ironsmith-grammar-common/")
}

fn is_test_only(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.contains("/tests/")
        || normalized.contains("/test_support/")
        || normalized.ends_with("/tests.rs")
        || normalized.ends_with("_tests.rs")
        || normalized.contains("_tests_")
}



/// For a `"` at `quote`, the number of hashes if it opens a raw string.
fn raw_string_hashes(bytes: &[u8], quote: usize) -> Option<usize> {
    let mut i = quote;
    let mut hashes = 0usize;
    while i > 0 && bytes[i - 1] == b'#' {
        hashes += 1;
        i -= 1;
    }
    (i > 0 && bytes[i - 1] == b'r' && (i == 1 || !(bytes[i - 2].is_ascii_alphanumeric() || bytes[i - 2] == b'_')))
        .then_some(hashes)
}

fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
