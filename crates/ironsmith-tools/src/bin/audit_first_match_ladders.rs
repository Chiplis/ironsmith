//! Counts first-match ladders in the parser: runs of `if` statements that each
//! try a recognizer and return on its match, so that registration order — the
//! order the `if`s are written — decides which grammar a sentence gets when
//! more than one would accept it.
//!
//! The repair order's item 4 routes every competing grammar through candidate
//! collection with explicit ambiguity detection. This audit is its instrument:
//! a ladder is three or more consecutive top-level `if` statements in one
//! function whose condition calls a recognizer (`parse_*`, `recognize_*`,
//! `probe_*`, `classify_*`, `match_*`, `bind_*`, `split_*`, `read_*`,
//! `open_*`, `try_parse_*`) and whose block returns or continues, with no
//! `else`. The count reports ladders and their rungs per production parser
//! module; the ratchet reads the ladder count.
//!
//! `--rungs` sorts the report by rung count; `--min N` changes the ladder
//! threshold (default 3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod tooling_paths;

const RECOGNIZER_PREFIXES: &[&str] = &[
    "try_parse", "parse", "recognize", "probe", "classify", "match", "bind", "split", "read", "open",
];

struct Ladder {
    path: PathBuf,
    line: usize,
    function: String,
    rungs: usize,
}

fn main() {
    let mut min_rungs = 3usize;
    let mut by_rungs = false;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--min" => {
                min_rungs = iter
                    .next()
                    .and_then(|value| value.parse().ok())
                    .expect("--min requires a positive integer");
            }
            "--rungs" => by_rungs = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|error| panic!("failed to locate repo root: {error}"));
    let modules = tracked_production_modules(&repo_root)
        .unwrap_or_else(|error| panic!("failed to enumerate production parser modules: {error}"));

    let mut ladders = Vec::new();
    for relative in &modules {
        let path = repo_root.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        ladders.extend(scan_module(relative, &source, min_rungs));
    }
    if by_rungs {
        ladders.sort_by(|left, right| {
            right
                .rungs
                .cmp(&left.rungs)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.line.cmp(&right.line))
        });
    } else {
        ladders.sort_by(|left, right| left.path.cmp(&right.path).then_with(|| left.line.cmp(&right.line)));
    }
    let rungs: usize = ladders.iter().map(|ladder| ladder.rungs).sum();
    println!("parser modules covered: {}", modules.len());
    println!("ladder threshold: {min_rungs} rungs");
    println!("first-match ladders: {}", ladders.len());
    println!("first-match rungs: {rungs}");
    for ladder in &ladders {
        println!(
            "{}:{}: {} ({} rungs)",
            ladder.path.display(),
            ladder.line,
            ladder.function,
            ladder.rungs
        );
    }
}

/// The ladders in one module. Comments, string and char literals, and inline
/// test modules are blanked to spaces first so braces inside them do not
/// count and line numbers stay true.
fn scan_module(path: &Path, source: &str, min_rungs: usize) -> Vec<Ladder> {
    let text = blank_test_modules(&blank_comments_and_literals(source));
    let bytes = text.as_bytes();
    let mut ladders = Vec::new();
    let mut search = 0usize;
    while let Some((name, name_end)) = next_function(&text, search) {
        search = name_end;
        let Some(body_start) = function_body_start(bytes, name_end) else {
            continue;
        };
        let Some(body_end) = matching_brace(bytes, body_start) else {
            continue;
        };
        search = body_end;
        let body = &text[body_start..=body_end];
        let mut run = 0usize;
        let mut run_start = 0usize;
        let flush = |run: usize, run_start: usize, ladders: &mut Vec<Ladder>| {
            if run >= min_rungs {
                ladders.push(Ladder {
                    path: path.to_path_buf(),
                    line: line_of(&text, body_start + run_start),
                    function: name.clone(),
                    rungs: run,
                });
            }
        };
        for statement in top_level_ifs(body) {
            if statement.is_decline {
                continue;
            }
            if statement.is_rung {
                if run == 0 {
                    run_start = statement.start;
                }
                run += 1;
            } else {
                flush(run, run_start, &mut ladders);
                run = 0;
            }
        }
        flush(run, run_start, &mut ladders);
    }
    ladders
}

struct IfStatement {
    start: usize,
    is_rung: bool,
    /// A decline: the block gives nothing back (`return Ok(None)`, `return
    /// None`, `return false`). It refuses an input rather than choosing a
    /// reading for it, so it is neither a rung nor a break in a ladder.
    is_decline: bool,
}

/// The `if` statements directly in a function body (brace depth one), in
/// order. A rung tries a recognizer in its condition and leaves the function
/// or the loop from its block, without an `else`.
fn top_level_ifs(body: &str) -> Vec<IfStatement> {
    let bytes = body.as_bytes();
    let mut statements = Vec::new();
    let mut depth = 0i32;
    let mut i = 1usize;
    while i + 3 < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'i' if depth == 0
                && bytes[i + 1] == b'f'
                && bytes[i + 2].is_ascii_whitespace()
                && matches!(bytes[i - 1], b' ' | b'\n' | b'\t' | b';' | b'}' | b'{') =>
            {
                let Some(block_start) = condition_end(bytes, i + 2) else {
                    break;
                };
                let Some(block_end) = matching_brace(bytes, block_start) else {
                    break;
                };
                let condition = &body[i..block_start];
                let block = &body[block_start..=block_end];
                let has_else = body[block_end + 1..]
                    .trim_start()
                    .starts_with("else");
                let is_decline = !has_else && is_decline_block(block);
                statements.push(IfStatement {
                    start: i,
                    is_rung: !has_else
                        && !is_decline
                        && calls_recognizer(condition)
                        && (contains_word(block, "return") || contains_word(block, "continue")),
                    is_decline,
                });
                i = block_end + 1;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    statements
}

/// Index of the brace opening an `if` block: the first `{` outside any
/// parentheses or brackets after the keyword.
fn condition_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' if depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
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

/// The next `fn name` after `from`: the name and the index just past it.
fn next_function(text: &str, from: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let mut at = from;
    while let Some(offset) = text[at..].find("fn ") {
        let start = at + offset;
        at = start + 3;
        if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            continue;
        }
        let name_start = at;
        let mut name_end = name_start;
        while name_end < bytes.len() && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'_') {
            name_end += 1;
        }
        if name_end == name_start {
            continue;
        }
        return Some((text[name_start..name_end].to_string(), name_end));
    }
    None
}

/// The brace opening a function body, or none for a bodiless declaration.
fn function_body_start(bytes: &[u8], from: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' if depth == 0 => return Some(i),
            b';' if depth == 0 => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

fn calls_recognizer(condition: &str) -> bool {
    let bytes = condition.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_')
            || (i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'))
        {
            i += 1;
            continue;
        }
        let mut end = i;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        let word = &condition[i..end];
        let is_recognizer = RECOGNIZER_PREFIXES.iter().any(|prefix| {
            word == *prefix || word.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('_'))
        });
        if is_recognizer {
            let mut after = end;
            // a turbofish may sit between the name and its call
            if condition[after..].starts_with("::<") && let Some(close) = condition[after..].find('>') {
                after += close + 1;
            }
            if condition[after..].trim_start().starts_with('(') {
                return true;
            }
        }
        i = end;
    }
    false
}

/// Whether a block only refuses: its single statement is `return Ok(None);`,
/// `return None;` or `return false;`.
fn is_decline_block(block: &str) -> bool {
    let inner = block.trim().trim_start_matches('{').trim_end_matches('}').trim();
    matches!(inner, "return Ok(None);" | "return None;" | "return false;")
}

fn contains_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while let Some(offset) = text[at..].find(word) {
        let start = at + offset;
        let end = start + word.len();
        let before_ok = start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        let after_ok = end >= bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
        if before_ok && after_ok {
            return true;
        }
        at = end;
    }
    false
}

fn line_of(text: &str, index: usize) -> usize {
    text.as_bytes()[..index].iter().filter(|byte| **byte == b'\n').count() + 1
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_a_run_of_recognizer_ifs_that_return() {
        let source = r#"
fn dispatch(tokens: &[Token]) -> Result<Option<Ast>, Error> {
    if let Some(ast) = parse_a(tokens)? {
        return Ok(Some(ast));
    }
    // a comment with an if { inside
    if tokens.len() == 2 && let Some(ast) = parse_b(tokens)? {
        return Ok(Some(ast));
    }
    let label = "if { not code }";
    if let Some(ast) = recognize_c(tokens) {
        return Ok(Some(ast));
    }
    if label.is_empty() {
        return Ok(None);
    }
    Ok(None)
}
"#;
        let ladders = scan_module(Path::new("x.rs"), source, 3);
        assert_eq!(ladders.len(), 1);
        assert_eq!(ladders[0].rungs, 3);
        assert_eq!(ladders[0].function, "dispatch");
        assert_eq!(ladders[0].line, 3);
    }

    #[test]
    fn declines_are_neither_rungs_nor_breaks() {
        let source = r#"
fn dispatch(tokens: &[Token]) -> Option<Ast> {
    if parse_other_shape(tokens).is_some() {
        return None;
    }
    if let Some(ast) = parse_a(tokens) { return Some(ast); }
    if parse_third_shape(tokens).is_some() {
        return None;
    }
    if let Some(ast) = parse_b(tokens) { return Some(ast); }
    if let Some(ast) = parse_c(tokens) { return Some(ast); }
    None
}
"#;
        let ladders = scan_module(Path::new("x.rs"), source, 3);
        assert_eq!(ladders.len(), 1);
        assert_eq!(ladders[0].rungs, 3);
    }

    #[test]
    fn an_else_breaks_the_run_and_test_modules_do_not_count() {
        let source = r#"
fn dispatch(tokens: &[Token]) -> Option<Ast> {
    if let Some(ast) = parse_a(tokens) {
        return Some(ast);
    } else {
        return None;
    }
    if let Some(ast) = parse_b(tokens) { return Some(ast); }
    if let Some(ast) = parse_c(tokens) { return Some(ast); }
    None
}

#[cfg(test)]
mod tests {
    fn ladder(t: &[Token]) -> Option<Ast> {
        if let Some(a) = parse_a(t) { return Some(a); }
        if let Some(a) = parse_b(t) { return Some(a); }
        if let Some(a) = parse_c(t) { return Some(a); }
        None
    }
}
"#;
        assert!(scan_module(Path::new("x.rs"), source, 3).is_empty());
        assert_eq!(scan_module(Path::new("x.rs"), source, 2).len(), 1);
    }
}
