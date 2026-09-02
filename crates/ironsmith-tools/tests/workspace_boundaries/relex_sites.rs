//! Where recognition still turns text into tokens.
//!
//! A line is tokenized once, by the document phase. Everything after that has
//! tokens in hand, so a `lex_line` call in recognition means one of three
//! things: text that already had tokens was rendered back and lexed again; a
//! phrase was synthesized from a string template instead of being built as
//! tokens; or a probe was written over `&str` when its caller held tokens. All
//! three are the reparsing that repair-order item 3 removes.
//!
//! This test names every production function that lexes, so the set can only
//! shrink: a new site fails the gate, and a removed site must be taken off the
//! list in the same change.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::{collect_production_rust_files, repo_relative, workspace_root};

const GRAMMAR_SRC: &str = "crates/ironsmith-compiler-grammar/src";

/// Functions that lex, as `path::function`.
///
/// Every entry is the document phase doing its job: turning a line, a
/// fragment of a line being normalized, a mana cost, a type line, or the card
/// name into tokens for the first time. Recognition holds no entry.
const LEXING_FUNCTIONS: &[&str] = &[
    // The tokenizer entry point the document phase calls.
    "util.rs::lex_fragment",
    // Lines and their normalization stages.
    "preprocess.rs::normalize_non_metadata_line",
    "preprocess.rs::stage_tokens",
    "preprocess.rs::make_line_info",
    // Card metadata: mana cost, type line, name.
    "grammar/values.rs::parse_mana_cost_tokens_text",
    "grammar/values.rs::parse_type_line_with",
    "document_parser/named_source_tokens.rs::aliases_for_builder",
    // Text handed to the parser directly, outside a card document.
    "front_end_parser_support.rs::split_sentences_for_parse",
    "front_end_parser_support.rs::split_text_for_parse_with_restrictions",
];

#[test]
pub(super) fn recognition_lexes_only_where_the_list_says() {
    let root = workspace_root();
    let actual = lexing_functions(&root);
    let expected: BTreeSet<String> = LEXING_FUNCTIONS.iter().map(|s| (*s).to_string()).collect();
    let added: Vec<_> = actual.difference(&expected).collect();
    let removed: Vec<_> = expected.difference(&actual).collect();
    assert!(
        added.is_empty(),
        "recognition gained lexing sites; tokens already exist by then, so use them:\n{}",
        added
            .iter()
            .map(|s| format!("  {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        removed.is_empty(),
        "lexing sites were removed; take them off the list so the gate keeps that ground:\n{}",
        removed
            .iter()
            .map(|s| format!("  {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn lexing_functions(root: &Path) -> BTreeSet<String> {
    let src = root.join(GRAMMAR_SRC);
    let mut files = Vec::new();
    collect_production_rust_files(&src, Path::new(""), &mut files);
    let mut found = BTreeSet::new();
    for path in files {
        // Numbered inline test shards (`*_tests_3.rs`) sit beside the modules
        // they test and are compiled only under `cfg(test)`.
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains("_tests"))
        {
            continue;
        }
        let relative = repo_relative(&src, &path);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let lines: Vec<&str> = source.lines().collect();
        let masked = test_only_lines(&lines);
        for (index, line) in lines.iter().enumerate() {
            if masked[index] {
                continue;
            }
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            let calls_lexer = (line.contains("lex_line(") && !line.contains("fn lex_line("))
                || (line.contains("lex_fragment(") && !line.contains("fn lex_fragment("));
            if !calls_lexer {
                continue;
            }
            let function = enclosing_function(&lines, index)
                .unwrap_or_else(|| panic!("{relative}:{}: lexer call outside any fn", index + 1));
            found.insert(format!("{relative}::{function}"));
        }
    }
    found
}

/// Lines inside a `#[cfg(test)]`, `#[cfg(any(test, …))]`, or `#[test]` item.
fn test_only_lines(lines: &[&str]) -> Vec<bool> {
    let mut masked = vec![false; lines.len()];
    let mut index = 0;
    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        if !(trimmed.starts_with("#[cfg(test)]")
            || trimmed.starts_with("#[cfg(any(test,")
            || trimmed.starts_with("#[test]"))
        {
            index += 1;
            continue;
        }
        // Mask from the attribute through the end of the item it decorates:
        // either a `;`-terminated line or a brace-balanced body.
        let start = index;
        let mut depth: i32 = 0;
        let mut seen_brace = false;
        let mut end = start;
        while end < lines.len() {
            let line = lines[end];
            for ch in line.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        seen_brace = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if seen_brace && depth <= 0 {
                break;
            }
            if !seen_brace && line.trim_end().ends_with(';') && !line.trim_start().starts_with('#')
            {
                break;
            }
            end += 1;
        }
        for line in start..=end.min(lines.len() - 1) {
            masked[line] = true;
        }
        index = end + 1;
    }
    masked
}

fn enclosing_function(lines: &[&str], index: usize) -> Option<String> {
    for line in lines[..=index].iter().rev() {
        let trimmed = line.trim_start();
        let rest = trimmed
            .strip_prefix("pub(crate) fn ")
            .or_else(|| trimmed.strip_prefix("pub(super) fn "))
            .or_else(|| trimmed.strip_prefix("pub fn "))
            .or_else(|| trimmed.strip_prefix("fn "));
        if let Some(rest) = rest {
            let name: String = rest
                .chars()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}
