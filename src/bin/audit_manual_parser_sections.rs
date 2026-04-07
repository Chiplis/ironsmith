use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const PARSER_ROOT: &str = "src/cards/builders/parser";

const PHRASE_HELPER_PATTERNS: &[&str] = &[
    "words_match_prefix(",
    "words_match_any_prefix(",
    "words_find_phrase(",
    "words_match_suffix(",
    "contains_phrase(",
    "contains_any_phrase(",
];

const SCAN_HELPER_PATTERNS: &[&str] = &[
    "find_index(",
    "rfind_index(",
    "find_window_index(",
    "find_token_index(",
    "find_word_index(",
    "find_word_sequence_index(",
    "token_index_for_word_index(",
    ".position(",
    ".rposition(",
    ".windows(",
];

const WORD_SLICE_SHAPE_PATTERNS: &[&str] = &[
    "slice_starts_with(",
    "slice_ends_with(",
    "slice_contains(",
    "word_slice_starts_with(",
    "word_slice_ends_with(",
    "word_slice_contains(",
    "== [",
    "!= [",
    ".as_slice() == [",
    ".as_slice() != [",
];

const RAW_STRING_PATTERNS: &[&str] = &[
    ".split_once(",
    ".find(",
    ".strip_prefix(",
    ".strip_suffix(",
    ".starts_with(",
    ".ends_with(",
    ".contains(",
];

const RAW_STRING_LINE_EXCLUSIONS: &[&str] = &["TokenWordView"];
const RAW_STRING_RECEIVER_EXCLUSIONS: &[&str] = &["words"];

const CONTROL_FLOW_HELPER_PATTERNS: &[&str] = &[
    concat!("scan_", "helpers::"),
    concat!("lexed", "_words("),
    concat!("render_", "lexed_tokens("),
];

const LEXED_CONTEXT_MARKERS: &[&str] = &[
    "lex_line(",
    "OwnedLexToken",
    "LexStream",
    "TokenSlice",
    "line.tokens",
    "lexed",
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum AuditKind {
    PhraseHelpers,
    ScanHelpers,
    WordSliceShapes,
    RawStringAfterLex,
    ControlFlowHelpers,
}

impl AuditKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::PhraseHelpers => "phrase_helpers",
            Self::ScanHelpers => "scan_helpers",
            Self::WordSliceShapes => "word_slice_shapes",
            Self::RawStringAfterLex => "raw_string_after_lex",
            Self::ControlFlowHelpers => "control_flow_helpers",
        }
    }
}

#[derive(Debug)]
struct FunctionSection {
    file: String,
    name: String,
    line: usize,
    body: String,
}

#[derive(Debug)]
struct Finding {
    file: String,
    name: String,
    line: usize,
    kinds: BTreeSet<AuditKind>,
}

#[derive(Debug)]
struct ModuleContext {
    end_depth: usize,
    is_test: bool,
}

#[derive(Debug)]
struct PendingFunction {
    name: String,
    line: usize,
    is_test: bool,
}

#[derive(Debug)]
struct ActiveFunction {
    name: String,
    line: usize,
    is_test: bool,
    start_line_index: usize,
    body_depth: usize,
}

fn main() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_root = repo_root.join(PARSER_ROOT);
    let mut files = Vec::new();
    collect_rs_files(&parser_root, &mut files);

    let mut findings = Vec::new();
    for path in files {
        if path.file_name().and_then(|name| name.to_str()) == Some("migration_audit.rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()));
        let rel = path
            .strip_prefix(&repo_root)
            .unwrap_or_else(|err| panic!("failed to relativize {}: {err}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");

        for function in extract_functions(&rel, &source) {
            let kinds = classify_function(&function.body);
            if kinds.is_empty() {
                continue;
            }
            findings.push(Finding {
                file: function.file,
                name: function.name,
                line: function.line,
                kinds,
            });
        }
    }

    print_report(&findings);
}

fn print_report(findings: &[Finding]) {
    println!("total_sections: {}", findings.len());

    let mut by_kind = BTreeMap::<AuditKind, usize>::new();
    let mut by_file = BTreeMap::<String, usize>::new();
    for finding in findings {
        *by_file.entry(finding.file.clone()).or_default() += 1;
        for kind in &finding.kinds {
            *by_kind.entry(*kind).or_default() += 1;
        }
    }

    println!("counts_by_kind:");
    for kind in [
        AuditKind::PhraseHelpers,
        AuditKind::ScanHelpers,
        AuditKind::WordSliceShapes,
        AuditKind::RawStringAfterLex,
        AuditKind::ControlFlowHelpers,
    ] {
        println!(
            "  {}: {}",
            kind.as_str(),
            by_kind.get(&kind).copied().unwrap_or(0)
        );
    }

    println!("counts_by_file:");
    let mut by_file_sorted = by_file.into_iter().collect::<Vec<_>>();
    by_file_sorted.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (file, count) in by_file_sorted {
        println!("  {}: {}", file, count);
    }

    println!("sections:");
    for finding in findings {
        let kinds = finding
            .kinds
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  {}:{} {} [{}]",
            finding.file, finding.line, finding.name, kinds
        );
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed reading directory {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("failed reading directory entry: {err}"));
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn extract_functions(file: &str, source: &str) -> Vec<FunctionSection> {
    let sanitized = sanitize_source(source);
    let source_lines = source.lines().collect::<Vec<_>>();
    let sanitized_lines = sanitized.lines().collect::<Vec<_>>();

    let mut functions = Vec::new();
    let mut brace_depth = 0usize;
    let mut module_stack = Vec::<ModuleContext>::new();
    let mut pending_cfg_test = false;
    let mut pending_test_attr = false;
    let mut pending_function = None::<PendingFunction>;
    let mut active_function = None::<ActiveFunction>;

    for (line_idx, (_source_line, sanitized_line)) in source_lines
        .iter()
        .zip(sanitized_lines.iter().copied().chain(std::iter::repeat("")))
        .enumerate()
    {
        while module_stack
            .last()
            .is_some_and(|module| brace_depth < module.end_depth)
        {
            module_stack.pop();
        }

        let trimmed = sanitized_line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
        }
        if trimmed.starts_with("#[test]") || trimmed.starts_with("#[tokio::test") {
            pending_test_attr = true;
        }

        if pending_function.is_none() && active_function.is_none() {
            if let Some(module_name) = parse_module_name(trimmed) {
                let is_test_module = module_name == "tests" || pending_cfg_test;
                if count_char(sanitized_line, '{') > count_char(sanitized_line, '}') {
                    module_stack.push(ModuleContext {
                        end_depth: brace_depth + 1,
                        is_test: is_test_module,
                    });
                }
                pending_cfg_test = false;
                pending_test_attr = false;
            } else if let Some(function_name) = parse_function_name(trimmed) {
                let in_test_module = module_stack.iter().any(|module| module.is_test);
                pending_function = Some(PendingFunction {
                    name: function_name,
                    line: line_idx + 1,
                    is_test: pending_cfg_test || pending_test_attr || in_test_module,
                });
                pending_cfg_test = false;
                pending_test_attr = false;
            } else if !trimmed.is_empty() && !trimmed.starts_with("#") {
                pending_cfg_test = false;
                pending_test_attr = false;
            }
        }

        if let Some(pending) = pending_function.take() {
            if sanitized_line.contains('{') {
                active_function = Some(ActiveFunction {
                    name: pending.name,
                    line: pending.line,
                    is_test: pending.is_test,
                    start_line_index: pending.line - 1,
                    body_depth: brace_depth + 1,
                });
            } else {
                pending_function = Some(pending);
            }
        }

        let opens = count_char(sanitized_line, '{');
        let closes = count_char(sanitized_line, '}');
        brace_depth += opens;
        brace_depth = brace_depth.saturating_sub(closes);

        if let Some(active) = active_function.take() {
            if brace_depth < active.body_depth {
                if !active.is_test {
                    let body = source_lines[active.start_line_index..=line_idx].join("\n");
                    functions.push(FunctionSection {
                        file: file.to_string(),
                        name: active.name,
                        line: active.line,
                        body,
                    });
                }
            } else {
                active_function = Some(active);
            }
        }
    }

    functions
}

fn sanitize_source(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0usize;
    let mut block_comment_depth = 0usize;

    while i < bytes.len() {
        if block_comment_depth > 0 {
            if bytes[i..].starts_with(b"/*") {
                block_comment_depth += 1;
                out.push(' ');
                out.push(' ');
                i += 2;
            } else if bytes[i..].starts_with(b"*/") {
                block_comment_depth -= 1;
                out.push(' ');
                out.push(' ');
                i += 2;
            } else {
                push_sanitized_char(&mut out, bytes[i]);
                i += 1;
            }
            continue;
        }

        if bytes[i..].starts_with(b"//") {
            out.push(' ');
            out.push(' ');
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }

        if bytes[i..].starts_with(b"/*") {
            block_comment_depth = 1;
            out.push(' ');
            out.push(' ');
            i += 2;
            continue;
        }

        if let Some((prefix_len, hash_count)) = raw_string_start(bytes, i) {
            for _ in 0..prefix_len {
                out.push(' ');
            }
            i += prefix_len;
            while i < bytes.len() {
                if bytes[i] == b'"' && raw_string_end(bytes, i, hash_count) {
                    out.push(' ');
                    i += 1;
                    for _ in 0..hash_count {
                        out.push(' ');
                        i += 1;
                    }
                    break;
                }
                push_sanitized_char(&mut out, bytes[i]);
                i += 1;
            }
            continue;
        }

        if string_start(bytes, i) {
            out.push(' ');
            i += if bytes[i] == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                out.push(' ');
                2
            } else {
                1
            };
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    out.push(' ');
                    i += 1;
                    if i < bytes.len() {
                        push_sanitized_char(&mut out, bytes[i]);
                        i += 1;
                    }
                    continue;
                }
                if bytes[i] == b'"' {
                    out.push(' ');
                    i += 1;
                    break;
                }
                push_sanitized_char(&mut out, bytes[i]);
                i += 1;
            }
            continue;
        }

        out.push(bytes[i] as char);
        i += 1;
    }

    out
}

fn string_start(bytes: &[u8], idx: usize) -> bool {
    bytes[idx] == b'"' || (bytes[idx] == b'b' && idx + 1 < bytes.len() && bytes[idx + 1] == b'"')
}

fn raw_string_start(bytes: &[u8], idx: usize) -> Option<(usize, usize)> {
    let mut cursor = idx;
    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }
    cursor += 1;
    let mut hash_count = 0usize;
    while bytes.get(cursor) == Some(&b'#') {
        hash_count += 1;
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    Some((cursor - idx + 1, hash_count))
}

fn raw_string_end(bytes: &[u8], quote_idx: usize, hash_count: usize) -> bool {
    (0..hash_count).all(|offset| bytes.get(quote_idx + 1 + offset) == Some(&b'#'))
}

fn push_sanitized_char(out: &mut String, byte: u8) {
    if byte == b'\n' {
        out.push('\n');
    } else {
        out.push(' ');
    }
}

fn parse_module_name(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("mod ")?;
    let name = rest
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn parse_function_name(trimmed: &str) -> Option<String> {
    let fn_idx = trimmed.find("fn ")?;
    let rest = &trimmed[fn_idx + 3..];
    let name = rest
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn count_char(line: &str, expected: char) -> usize {
    line.chars().filter(|ch| *ch == expected).count()
}

fn classify_function(body: &str) -> BTreeSet<AuditKind> {
    let mut kinds = BTreeSet::new();

    if contains_any(body, PHRASE_HELPER_PATTERNS) {
        kinds.insert(AuditKind::PhraseHelpers);
    }
    if contains_any(body, SCAN_HELPER_PATTERNS) {
        kinds.insert(AuditKind::ScanHelpers);
    }
    if contains_any(body, WORD_SLICE_SHAPE_PATTERNS) {
        kinds.insert(AuditKind::WordSliceShapes);
    }
    if contains_raw_string_after_lex(body) {
        kinds.insert(AuditKind::RawStringAfterLex);
    }
    if contains_any(body, CONTROL_FLOW_HELPER_PATTERNS) {
        kinds.insert(AuditKind::ControlFlowHelpers);
    }

    kinds
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn contains_raw_string_after_lex(body: &str) -> bool {
    if !contains_any(body, LEXED_CONTEXT_MARKERS) {
        return false;
    }

    for line in body.lines() {
        if RAW_STRING_LINE_EXCLUSIONS
            .iter()
            .any(|needle| line.contains(needle))
        {
            continue;
        }
        for needle in RAW_STRING_PATTERNS {
            let mut search_start = 0usize;
            while let Some(idx) = line[search_start..].find(needle) {
                let absolute = search_start + idx;
                if !receiver_excluded(line, absolute) {
                    return true;
                }
                search_start = absolute + needle.len();
            }
        }
    }

    false
}

fn receiver_excluded(line: &str, needle_idx: usize) -> bool {
    RAW_STRING_RECEIVER_EXCLUSIONS.iter().any(|receiver| {
        let prefix = format!("{receiver}");
        line[..needle_idx].trim_end().ends_with(&prefix)
    })
}
