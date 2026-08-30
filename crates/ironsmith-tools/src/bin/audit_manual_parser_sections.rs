use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod tooling_paths;

const LEGACY_FALLBACK_PATTERNS: &[&str] = &[
    "LineFamilyRuleHandler::Legacy",
    "LexRuleHandler::Legacy",
    "RuleHandler::Legacy",
    "LegacyRuntime",
    "legacy_handler",
    "legacy_adapter",
    "compatibility_adapter",
];

const ORDER_DEPENDENT_DISPATCH_PATTERNS: &[&str] = &[
    "preempt",
    "first_refusal",
    "first refusal",
    "must_win_before",
    "must win before",
    "must_run_before",
    "must run before",
];

const WHOLE_PROGRAM_PATH_PATTERNS: &[&str] = &[
    "_programs.rs",
    "/program.rs",
    "/programs/",
    "bundle_rules",
    "generic_program_shapes",
    "sequence_pairs",
    "sequence_triples",
    "sequence_quads",
];

const PHRASE_HELPER_PATTERNS: &[&str] = &[
    "words_match_prefix(",
    "words_match_any_prefix(",
    "words_find_phrase(",
    "words_match_suffix(",
    "contains_phrase(",
    "contains_any_phrase(",
    // These aliases are semantically identical to the legacy helpers above.
    // Keep them audited so vocabulary changes cannot masquerade as migration.
    "words_have_phrase(",
    "words_have_phrase_or_empty(",
    "words_have_any_phrase(",
    "words_have_any_phrase_or_empty(",
    "words_start_with(",
    "words_start_with_at(",
    "words_start_with_any(",
    "words_end_with(",
    "words_end_with_any(",
    "tokens_start_with(",
    "tokens_start_with_at(",
    "tokens_start_with_any(",
    "tokens_end_with(",
    "items_start_with(",
    "items_start_with_any(",
    "items_end_with(",
    "items_end_with_any(",
    "word_view_has_prefix(",
    "word_view_has_any_prefix(",
    // Migration-era aliases are still parser-shaped probes. Keep them
    // audited so moving the same recognition behind a new local name does
    // not count as typed-grammar ownership.
    "activated_words_equal(",
    "activated_words_equal_any(",
    "activated_words_contain_all(",
    "activated_words_contain_any(",
    "activated_words_contain_phrase(",
    "activated_words_contain_word(",
    "activated_phrase_start(",
    // Activation/cant-family aliases are the same word-slice recognizers
    // under a namespace-specific spelling. Keep them audited until callers
    // consume typed cant facts instead.
    "activation_word_is_any(",
    "activation_token_word_is(",
    "activation_token_word_is_any(",
    "activation_word_at_is(",
    "activation_word_at_is_any(",
    "activation_words_eq(",
    "activation_words_eq_any(",
    "activation_words_contains(",
    "cant_attack_unless_tail(",
    "cant_attack_or_block_unless_tail(",
    "keyword_words_match_phrase(",
    "strip_prefix_words_ci(",
    "strip_suffix_words_ci(",
    "trigger_control_tail_words(",
    "trigger_subject_control_suffix(",
    "trigger_subject_control_phrase(",
    "keyword_action_word_at_is(",
    "keyword_action_word_at_is_any(",
    "keyword_action_token_is(",
    "PLAYERS_ARE_ATTACKED_TRIGGER_PATTERN",
    "CRAFT_WITH_PREFIX_PATTERN",
    "CRAFT_RED_INSTANT_SORCERY_MATERIAL_TAIL_PATTERN",
    "PAY_LIFE_COST_PATTERN",
    "SINGLE_GRAVEYARD_BOTTOM_LIBRARY_TAIL_PREFIX_PATTERN",
    // Transitional parser DSLs are still manual semantic recognition when
    // consumed outside typed grammar entrypoints. Track both direct builders
    // and their common match APIs so module-level constants cannot hide the
    // recognition from the function-section audit.
    "LexPattern::",
    ".match_clause(",
    ".matches_clause(",
    ".match_prefix(",
    ".match_word_refs(",
    ".matches_word_slice(",
    ".matches_non_article_tokens(",
    ".matches_first_word(",
    ".find_exact_window_range(",
    // LexedClause and pattern-wrapper aliases are still phrase probes when
    // invoked by a family or sentence module. Audit the caller as well as the
    // low-level LexPattern implementation so a wrapper cannot hide ownership.
    ".matches_words(",
    ".matches_any_words(",
    ".matches_clause_first_word(",
    ".contains_word(",
    ".contains_any_word(",
    ".contains_no_words(",
    ".strip_prefix_clause(",
    ".strip_any_prefix_clause(",
    ".strip_suffix_clause(",
    ".strip_any_suffix_clause(",
    "anthem_shape_matches_words(",
    "anthem_shape_matches_word(",
    "anthem_shape_matches_last_word(",
    "anthem_token_matches_shape(",
    "anthem_find_prefix_shape_start(",
    "keyword_static_shape_matches_words(",
    "attached_shape_matches_words(",
    "attached_find_prefix_shape_start(",
    "attached_word_is_any(",
    "attached_word_is(",
    "attached_word_at_is(",
    "attached_token_word_is_any(",
    "attached_token_word_is(",
    "activation_cost_shape_matches_words(",
    "activation_restriction_shape_matches_words(",
    "choice_object_shape_matches_words(",
    "trigger_subject_shape_matches_words(",
    "trigger_clause_shape_matches_words(",
    "token_clause_matches_shape(",
    "shared_util_shape_matches_words(",
    "clause_matches_phrase(",
    "clause_matches_any_phrase(",
    "modal_clause_matches_pattern(",
    "modal_clause_matches_prefix(",
];

const SCAN_HELPER_PATTERNS: &[&str] = &[
    "find_index(",
    "rfind_index(",
    "find_window_index(",
    "find_token_index(",
    "find_word_index(",
    "find_word_sequence_index(",
    "token_index_for_word_index(",
    "locate_index(",
    "locate_index_with(",
    "locate_last_index(",
    "locate_last_index_with(",
    "locate_window_index(",
    "token_start_for_word(",
    "word_slice_find_phrase_start(",
    "word_slice_find_any_phrase_start(",
    "word_slice_find_any_phrase_span(",
    "word_slice_find_word(",
    "word_slice_find_any_word(",
    "word_slice_find_word_where(",
    "word_slice_rfind_word_where(",
    // Generic closure-driven scanners preserve semantic recognition in the
    // caller even when the cursor loop itself lives in grammar/. Require a
    // named typed parser result instead.
    "parse_item_boundary(",
    "parse_item_offset(",
    "parse_last_item_boundary(",
    "parse_last_item_offset(",
    "parse_window_boundary(",
    "parse_window_offset(",
    "parse_phrase_span_tokens(",
    "parse_phrase_token_offsets(",
    "parse_phrase_boundary_words(",
    "parse_phrase_offset_words(",
    ".find_word(",
    ".find_word_any(",
    ".find_phrase_start(",
    ".find_any_phrase_start(",
    ".find_any_phrase_span(",
    "keyword_token_kind_index(",
    "keyword_mana_cost_start(",
    "find_cycling_keyword_word_index(",
    "contains_granted_keyword_before_word(",
    ".position(",
    ".rposition(",
    ".windows(",
];

const LEXED_SCAN_HELPER_PATTERNS: &[&str] = &[".find(", ".rfind("];

const WORD_SLICE_SHAPE_PATTERNS: &[&str] = &[
    "slice_starts_with(",
    "slice_ends_with(",
    "slice_contains(",
    "word_slice_starts_with(",
    "word_slice_ends_with(",
    "word_slice_contains(",
    "words_have(",
    "words_have_any(",
    "words_have_none(",
    "words_have_all(",
    "items_have(",
    "items_have_any(",
    "items_have_all(",
    "word_slice_eq(",
    "word_slice_eq_any(",
    "word_slice_eq_at(",
    "word_slice_eq_any_at(",
    "token_slice_words_eq(",
    "token_slice_words_eq_any(",
    "iter_eq(",
    "== [",
    "!= [",
    ".as_slice() == [",
    ".as_slice() != [",
];

const RAW_STRING_PATTERNS: &[&str] = &[
    ".split_once(",
    ".strip_prefix(",
    ".strip_suffix(",
    ".starts_with(",
    ".ends_with(",
    ".contains(",
];

const RAW_STRING_HELPER_PATTERNS: &[&str] = &[
    "str_contains(",
    "str_contains_char(",
    "str_starts_with(",
    "str_starts_with_char(",
    "str_ends_with(",
    "str_ends_with_char(",
    "str_split_once(",
    "str_split_once_char(",
    "str_find(",
    "str_strip_prefix(",
    "str_strip_suffix(",
    "str_strip_suffix_char(",
];

const CONTROL_FLOW_HELPER_PATTERNS: &[&str] = &[
    concat!("scan_", "helpers::"),
    concat!("lexed", "_words("),
    concat!("render_", "lexed_tokens("),
];

// Method names shared by `LexedClause`, `TokenWordView`, and migration-era
// clause wrappers. The ambiguous names in these lists are classified only
// when their receiver is visibly word/clause-shaped; this avoids treating
// `String::starts_with`, collection `ends_with`, or typed grammar surfaces as
// parser probes.
const RECEIVER_PHRASE_PROBE_METHODS: &[&str] = &[
    "starts_with",
    "starts_with_at",
    "starts_with_any",
    "ends_with",
    "ends_with_any",
    "first_is",
    "first_is_any",
    "last_is",
    "last_is_any",
    "has_phrase",
    "has_any_phrase",
];

const RECEIVER_WORD_SHAPE_PROBE_METHODS: &[&str] = &[
    "at_is",
    "at_is_any",
    "equals_at",
    "equals_any_at",
    "matching_value",
    "strip_prefix_value",
    "strip_first_word_value",
    "strip_suffix_value",
];

// These APIs are specific enough to be audited without receiver inference.
// In particular, the `*_clause` variants are also exposed through
// `SubjectVerbPrimitiveClause`, so auditing only the lexer implementation
// would miss the compatibility wrapper and its callers.
const UNIQUE_PHRASE_PROBE_PATTERNS: &[&str] = &[
    ".first_is_word(",
    ".first_is_any_word(",
    ".strip_prefix_value_clause(",
    ".strip_suffix_value_clause(",
];

const UNIQUE_WORD_SHAPE_PROBE_PATTERNS: &[&str] = &[".contains_all_words("];

// These two wrappers intentionally present phrase-probe APIs outside grammar.
// Their receivers are constructor calls or a generic `ctx`, so identifier
// inference alone cannot distinguish them from typed grammar result objects.
const COMPAT_PHRASE_PROBE_CONTEXTS: &[&str] = &[
    "ActivationRestrictionCompatWords::new(",
    "UnsupportedRewriteLineContext::new(",
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
    ParserOwnership,
    LegacyFallback,
    OrderedDispatch,
    OptionalParserBoundary,
    SilentCandidateFailure,
    WholeProgramRecipe,
    PhraseHelpers,
    ScanHelpers,
    WordSliceShapes,
    RawStringAfterLex,
    ControlFlowHelpers,
}

impl AuditKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::ParserOwnership => "parser_ownership",
            Self::LegacyFallback => "legacy_fallback",
            Self::OrderedDispatch => "ordered_dispatch",
            Self::OptionalParserBoundary => "optional_parser_boundary",
            Self::SilentCandidateFailure => "silent_candidate_failure",
            Self::WholeProgramRecipe => "whole_program_recipe",
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

#[derive(Debug, Default)]
struct Args {
    fail_on_findings: bool,
    enforce_prefixes: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|err| panic!("failed to locate repo root: {err}"));
    let files = tracked_rs_files(&repo_root);

    let mut findings = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()));
        let rel = path
            .strip_prefix(&repo_root)
            .unwrap_or_else(|err| panic!("failed to relativize {}: {err}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");

        if let Some(line) = parser_ownership_module_line(&rel, &source) {
            findings.push(Finding {
                file: rel.clone(),
                name: "<module-parser-ownership>".to_string(),
                line,
                kinds: BTreeSet::from([AuditKind::ParserOwnership]),
            });
        }

        for (line, kind, name) in module_protocol_findings(&rel, &source) {
            findings.push(Finding {
                file: rel.clone(),
                name: name.to_string(),
                line,
                kinds: BTreeSet::from([kind]),
            });
        }

        for function in extract_functions(&rel, &source) {
            let mut kinds = classify_function(&function.file, &function.body);
            kinds.extend(classify_parser_protocol(&function.name, &function.body));
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

    let enforced_findings = enforced_findings(&findings, &args);
    if !enforced_findings.is_empty() {
        eprintln!("\nParser audit enforcement failed:");
        for finding in enforced_findings {
            let kinds = finding
                .kinds
                .iter()
                .map(|kind| kind.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "  {}:{} {} [{}]",
                finding.file, finding.line, finding.name, kinds
            );
        }
        std::process::exit(1);
    }
}

impl Args {
    fn parse() -> Self {
        let mut args = Self::default();
        let mut raw = env::args().skip(1);
        while let Some(arg) = raw.next() {
            match arg.as_str() {
                "--fail-on-findings" => args.fail_on_findings = true,
                "--enforce-prefix" => {
                    let Some(prefix) = raw.next() else {
                        usage_and_exit("--enforce-prefix requires a repo-relative path");
                    };
                    args.enforce_prefixes.push(normalize_repo_prefix(&prefix));
                }
                "--help" | "-h" => usage_and_exit(""),
                _ => usage_and_exit(&format!("unknown argument `{arg}`")),
            }
        }
        args
    }
}

fn usage_and_exit(message: &str) -> ! {
    if !message.is_empty() {
        eprintln!("{message}\n");
    }
    eprintln!(
        "usage: audit_manual_parser_sections [--fail-on-findings] [--enforce-prefix <repo-path>...]"
    );
    std::process::exit(if message.is_empty() { 0 } else { 2 });
}

fn enforced_findings<'a>(findings: &'a [Finding], args: &Args) -> Vec<&'a Finding> {
    findings
        .iter()
        .filter(|finding| {
            args.fail_on_findings
                || args
                    .enforce_prefixes
                    .iter()
                    .any(|prefix| finding_matches_prefix(&finding.file, prefix))
        })
        .collect()
}

fn finding_matches_prefix(file: &str, prefix: &str) -> bool {
    file == prefix
        || file
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}

fn normalize_repo_prefix(prefix: &str) -> String {
    prefix
        .trim_start_matches("./")
        .replace(std::path::MAIN_SEPARATOR, "/")
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
        AuditKind::ParserOwnership,
        AuditKind::LegacyFallback,
        AuditKind::OrderedDispatch,
        AuditKind::OptionalParserBoundary,
        AuditKind::SilentCandidateFailure,
        AuditKind::WholeProgramRecipe,
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

fn tracked_rs_files(repo_root: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed enumerating tracked parser files: {err}"));
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut files = String::from_utf8(output.stdout)
        .expect("git ls-files returned a non-UTF-8 path")
        .lines()
        .filter(|path| path.ends_with(".rs"))
        .filter(|path| {
            (path.starts_with("crates/ironsmith-compiler")
                && !path.starts_with("crates/ironsmith-compiler-runtime/"))
                || path.starts_with("crates/ironsmith-grammar-common/")
        })
        .filter(|path| {
            !path.contains("/tests/")
                && !path.ends_with("/tests.rs")
                && !path.ends_with("_tests.rs")
                && !path.contains("_tests_")
        })
        .map(|path| repo_root.join(path))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    files
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
                    let body = sanitized_lines[active.start_line_index..=line_idx].join("\n");
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

        if let Some(literal_len) = char_literal_len(bytes, i) {
            for byte in &bytes[i..i + literal_len] {
                push_sanitized_char(&mut out, *byte);
            }
            i += literal_len;
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

fn char_literal_len(bytes: &[u8], idx: usize) -> Option<usize> {
    let start = idx;
    let mut cursor = idx;
    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'\'') {
        return None;
    }
    cursor += 1;

    let first = *bytes.get(cursor)?;
    if matches!(first, b'\n' | b'\r' | b'\'') {
        return None;
    }

    if first == b'\\' {
        cursor += 1;
        match *bytes.get(cursor)? {
            b'x' => {
                let high = *bytes.get(cursor + 1)?;
                let low = *bytes.get(cursor + 2)?;
                if !high.is_ascii_hexdigit() || !low.is_ascii_hexdigit() {
                    return None;
                }
                cursor += 3;
            }
            b'u' => {
                if bytes.get(cursor + 1) != Some(&b'{') {
                    return None;
                }
                cursor += 2;
                let digits_start = cursor;
                while bytes
                    .get(cursor)
                    .is_some_and(|byte| byte.is_ascii_hexdigit() || *byte == b'_')
                {
                    cursor += 1;
                }
                if cursor == digits_start || bytes.get(cursor) != Some(&b'}') {
                    return None;
                }
                cursor += 1;
            }
            b'\n' | b'\r' => return None,
            _ => cursor += 1,
        }
    } else {
        let width = utf8_char_width(first)?;
        let tail = bytes.get(cursor..cursor + width)?;
        if width > 1
            && !tail[1..]
                .iter()
                .all(|byte| *byte & 0b1100_0000 == 0b1000_0000)
        {
            return None;
        }
        cursor += width;
    }

    if bytes.get(cursor) != Some(&b'\'') {
        return None;
    }
    Some(cursor + 1 - start)
}

fn utf8_char_width(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7f => Some(1),
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
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

fn contains_rust_identifier(source: &str, expected: &str) -> bool {
    source.match_indices(expected).any(|(start, matched)| {
        let before = source[..start].chars().next_back();
        let after = source[start + matched.len()..].chars().next();
        let is_ident = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
        before.is_none_or(|ch| !is_ident(ch)) && after.is_none_or(|ch| !is_ident(ch))
    })
}

fn module_protocol_findings(file: &str, source: &str) -> Vec<(usize, AuditKind, &'static str)> {
    let sanitized = sanitize_source(source);
    let mut findings = Vec::new();

    if let Some(line) = first_pattern_line(&sanitized, LEGACY_FALLBACK_PATTERNS) {
        findings.push((line, AuditKind::LegacyFallback, "<module-legacy-fallback>"));
    }
    if let Some(line) = first_pattern_line(
        &source.to_ascii_lowercase(),
        ORDER_DEPENDENT_DISPATCH_PATTERNS,
    ) {
        findings.push((
            line,
            AuditKind::OrderedDispatch,
            "<module-ordered-dispatch>",
        ));
    }
    if WHOLE_PROGRAM_PATH_PATTERNS
        .iter()
        .any(|pattern| file.contains(pattern))
    {
        findings.push((
            1,
            AuditKind::WholeProgramRecipe,
            "<module-whole-program-recipe>",
        ));
    }
    findings.extend(
        registry_option_protocol_lines(&sanitized)
            .into_iter()
            .map(|line| {
                (
                    line,
                    AuditKind::OptionalParserBoundary,
                    "<registry-option-protocol>",
                )
            }),
    );

    findings
}

fn registry_option_protocol_lines(source: &str) -> Vec<usize> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut findings = Vec::new();
    let mut index = 0usize;
    let mut registry_type_depth = None::<usize>;
    let mut brace_depth = 0usize;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if (trimmed.contains("enum ") || trimmed.contains("struct "))
            && ["Rule", "Registry", "Recognizer", "Handler"]
                .iter()
                .any(|marker| trimmed.contains(marker))
            && trimmed.contains('{')
        {
            registry_type_depth = Some(brace_depth + line.matches('{').count());
        }

        let type_alias = trimmed.contains("type ")
            && trimmed.contains('=')
            && [
                "Rule",
                "Registry",
                "Recognizer",
                "Handler",
                "Parser",
                "ParseFn",
            ]
            .iter()
            .any(|marker| trimmed.contains(marker));
        let stored_function_pointer = registry_type_depth.is_some() && trimmed.contains("fn(");
        if type_alias || stored_function_pointer {
            let start = index;
            let mut declaration = String::new();
            loop {
                declaration.push_str(lines[index]);
                let complete = if type_alias {
                    lines[index].contains(';')
                } else {
                    lines[index].contains(',') || lines[index].contains(';')
                };
                if complete || index + 1 == lines.len() {
                    break;
                }
                index += 1;
            }
            let compact = declaration
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            if compact.contains("Result<Option<") || compact.contains("->Option<") {
                findings.push(start + 1);
            }
        }

        brace_depth += line.matches('{').count();
        brace_depth = brace_depth.saturating_sub(line.matches('}').count());
        if registry_type_depth.is_some_and(|depth| brace_depth < depth) {
            registry_type_depth = None;
        }
        index += 1;
    }

    findings
}

fn first_pattern_line(source: &str, patterns: &[&str]) -> Option<usize> {
    source
        .lines()
        .position(|line| contains_any(line, patterns))
        .map(|line| line + 1)
}

fn classify_parser_protocol(name: &str, body: &str) -> BTreeSet<AuditKind> {
    let mut kinds = BTreeSet::new();
    let candidate = [
        "parse",
        "recognize",
        "match",
        "candidate",
        "dispatch",
        "handler",
    ]
    .iter()
    .any(|marker| name.contains(marker));
    if !candidate {
        return kinds;
    }

    let sanitized = sanitize_source(body);
    if sanitized.contains(".ok()") {
        kinds.insert(AuditKind::SilentCandidateFailure);
    }
    kinds
}

fn classify_function(file: &str, body: &str) -> BTreeSet<AuditKind> {
    let mut kinds = BTreeSet::new();

    if parser_ownership_function_violation(file, body) {
        kinds.insert(AuditKind::ParserOwnership);
    }

    if contains_any(body, PHRASE_HELPER_PATTERNS)
        || contains_any(body, UNIQUE_PHRASE_PROBE_PATTERNS)
        || contains_receiver_probe(body, RECEIVER_PHRASE_PROBE_METHODS)
        || contains_compat_phrase_probe(body)
    {
        kinds.insert(AuditKind::PhraseHelpers);
    }
    if contains_any(body, SCAN_HELPER_PATTERNS)
        || (contains_any(body, LEXED_CONTEXT_MARKERS)
            && contains_any(body, LEXED_SCAN_HELPER_PATTERNS))
    {
        kinds.insert(AuditKind::ScanHelpers);
    }
    if contains_any(body, WORD_SLICE_SHAPE_PATTERNS)
        || contains_any(body, UNIQUE_WORD_SHAPE_PROBE_PATTERNS)
        || contains_receiver_probe(body, RECEIVER_WORD_SHAPE_PROBE_METHODS)
        || contains_direct_word_slice_match(body)
    {
        kinds.insert(AuditKind::WordSliceShapes);
    }
    if contains_any(body, RAW_STRING_HELPER_PATTERNS) || contains_raw_string_after_lex(body) {
        kinds.insert(AuditKind::RawStringAfterLex);
    }
    if contains_any(body, CONTROL_FLOW_HELPER_PATTERNS) {
        kinds.insert(AuditKind::ControlFlowHelpers);
    }

    kinds
}

fn parser_ownership_module_line(file: &str, source: &str) -> Option<usize> {
    let sanitized = sanitize_source(source);
    let outside_grammar_surface = (!file.contains("/front_end/grammar/"))
        .then(|| {
            [
                "RestrictionSurface",
                "TriggerSurface",
                "ClauseShape",
                "PermissionSequence",
                // Parser-facing compatibility types are ownership leaks even
                // when their methods delegate to grammar helpers. Keep the
                // exact identifiers audited so migrations cannot stop at a
                // renamed word-slice DSL or a front-end repair side channel.
                "CantPattern",
                "ValueHelperCompatWords",
                "UtilWordView",
                "PostpassRepairFacts",
            ]
            .into_iter()
            .find(|marker| contains_rust_identifier(&sanitized, marker))
        })
        .flatten();
    let violation = if file.ends_with("/front_end/leaf.rs") {
        Some("mod")
    } else if let Some(surface) = outside_grammar_surface {
        Some(surface)
    } else if file.contains("/front_end/semantic_line_parsing/")
        && sanitized.contains("use super::lower::*;")
    {
        Some("use super::lower::*;")
    } else {
        None
    }?;

    sanitized
        .lines()
        .position(|line| contains_rust_identifier(line, violation))
        .map_or(Some(1), |line| Some(line + 1))
}

fn parser_ownership_function_violation(file: &str, body: &str) -> bool {
    let lowering_or_postpass = file.contains("/lowering/") || file.contains("/postpasses/");
    lowering_or_postpass
        && [
            "parser_token_word_refs(",
            "token_word_refs(",
            "TokenWordView::",
            "LexStream::",
            "lex_line(",
            "split_lexed_sentences(",
            "render_token_slice(",
            "parse_activated_line(",
            "parse_effect_sentences_lexed(",
            "parse_trigger_clause_lexed(",
            "parse_triggered_line_lexed(",
            "word_slice_",
            "token_slice_",
        ]
        .iter()
        .any(|pattern| body.contains(pattern))
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn contains_receiver_probe(body: &str, methods: &[&str]) -> bool {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");

    methods.iter().any(|method| {
        let marker = format!(".{method}(");
        let mut remaining = compact.as_str();
        while let Some(marker_idx) = remaining.find(&marker) {
            let before = remaining[..marker_idx].trim_end();
            if receiver_is_word_or_clause_shaped(before) {
                return true;
            }
            remaining = &remaining[marker_idx + marker.len()..];
        }
        false
    })
}

fn receiver_is_word_or_clause_shaped(before_method: &str) -> bool {
    if [".words()", ".word_refs()", ".lexed()", ".as_clause()"]
        .iter()
        .any(|suffix| before_method.ends_with(suffix))
    {
        return true;
    }

    let bytes = before_method.as_bytes();
    let mut start = bytes.len();
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    let receiver = &before_method[start..];
    receiver == "words"
        || receiver == "word_refs"
        || receiver == "word_view"
        || receiver == "clause"
        || receiver == "lexed_clause"
        || receiver.ends_with("_words")
        || receiver.ends_with("_word_refs")
        || receiver.ends_with("_word_view")
        || receiver.ends_with("_clause")
}

fn contains_compat_phrase_probe(body: &str) -> bool {
    contains_any(body, COMPAT_PHRASE_PROBE_CONTEXTS)
        && (body.contains(".has_phrase(") || body.contains(".has_any_phrase("))
}

fn contains_direct_word_slice_match(body: &str) -> bool {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");

    for marker in ["matches!(", "match "] {
        let mut remaining = compact.as_str();
        while let Some(marker_idx) = remaining.find(marker) {
            let after_marker = remaining[marker_idx + marker.len()..].trim_start();
            let name_len = after_marker
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            let name = &after_marker[..name_len];
            let after_name = after_marker[name_len..].trim_start();
            let direct_slice = after_name.starts_with(',') || after_name.starts_with('{');
            let as_slice = after_name.strip_prefix(".as_slice()").is_some_and(|tail| {
                let tail = tail.trim_start();
                tail.starts_with(',') || tail.starts_with('{')
            });

            if is_word_slice_name(name) && (direct_slice || as_slice) {
                return true;
            }

            remaining = &remaining[marker_idx + marker.len()..];
        }
    }

    false
}

fn is_word_slice_name(name: &str) -> bool {
    name == "words"
        || name == "word_refs"
        || name.ends_with("_words")
        || name.ends_with("_word_refs")
}

fn contains_raw_string_after_lex(body: &str) -> bool {
    if !contains_any(body, LEXED_CONTEXT_MARKERS) {
        return false;
    }

    for line in body.lines() {
        for needle in RAW_STRING_PATTERNS {
            if line.contains(needle) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_protocol_detection_covers_discarded_candidates_only_at_function_level() {
        let kinds = classify_parser_protocol(
            "parse_candidate",
            "fn parse_candidate() -> Result<Option<Value>, Error> { probe().ok(); todo!() }",
        );
        assert!(kinds.contains(&AuditKind::SilentCandidateFailure));
        assert!(
            classify_parser_protocol(
                "format_value",
                "fn format_value() -> Option<Value> { None }"
            )
            .is_empty()
        );
    }

    #[test]
    fn registry_protocol_detection_covers_stored_option_recognizers_only() {
        let source = r#"
type RuleFn = fn(&Input) -> Result<Option<Value>, Error>;
fn parse_optional_element() -> Option<Value> { None }
enum ParserRule { Structured(fn(&Input) -> Option<Value>) }
"#;
        assert_eq!(registry_option_protocol_lines(source), vec![2, 4]);
    }

    #[test]
    fn module_protocol_detection_cannot_be_hidden_by_renamed_files() {
        let legacy = module_protocol_findings(
            "crates/ironsmith-compiler-grammar/src/registry.rs",
            "const HANDLER: LexRuleHandler = LexRuleHandler::Legacy(parse_old);",
        );
        assert!(
            legacy
                .iter()
                .any(|(_, kind, _)| *kind == AuditKind::LegacyFallback)
        );

        let recipes = module_protocol_findings(
            "crates/ironsmith-compiler-grammar/src/effect_programs.rs",
            "pub fn typed() {}",
        );
        assert!(
            recipes
                .iter()
                .any(|(_, kind, _)| *kind == AuditKind::WholeProgramRecipe)
        );
    }

    #[test]
    fn extracts_lexer_style_functions_after_char_and_raw_literals() {
        let source = r####"
fn normalize_parser_fragment<'a>(slice: &'a str) -> &'a str {
    let _quote = '"';
    let _byte_quote = b'"';
    let _escaped_quote = '\'';
    let _escaped_brace = '\u{7b}';
    let _raw = r#"} fn hidden() { LexPattern::new(&[]) }"#;
    let _raw_byte = br##"{ fn also_hidden() { LexPattern::new(&[]) }"##;
    match slice.chars().next() {
        Some('“' | '”') => slice,
        _ => slice,
    }
}

fn first_is_word() -> bool {
    LexPattern::new(&atoms).matches_prefix(clause)
}
"####;

        let functions = extract_functions("front_end/lexer.rs", source);
        let names = functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["normalize_parser_fragment", "first_is_word"]);

        let first_is_word = functions
            .iter()
            .find(|function| function.name == "first_is_word")
            .expect("function after lexer-style literals must be extracted");
        assert!(
            classify_function(&first_is_word.file, &first_is_word.body)
                .contains(&AuditKind::PhraseHelpers),
            "LexPattern use after lexer-style literals must remain auditable"
        );
    }

    #[test]
    fn classifies_live_lexed_clause_and_word_slice_probe_families() {
        let source = r#"
fn probe(clause: LexedClause<'_>, clause_words: Vec<&str>) {
    let word_view = clause.words();
    let _ = clause.first_is_word("if");
    let _ = clause.matches_words(&["if"]);
    let _ = clause.matches_any_words(&[&["if"], &["unless"]]);
    let _ = clause.contains_word("target");
    let _ = clause.contains_all_words(&["this", "turn"]);
    let _ = clause.strip_prefix_clause(&["if"]);
    let _ = clause.strip_prefix_value_clause(&[(&["if"], 1)]);
    let _ = clause.find_phrase_start(&["this", "way"]);
    let _ = word_view.first_is("if");
    let _ = clause_words.starts_with(&["if"]);
    let _ = clause_words.ends_with(&["turn"]);
    let _ = word_view.at_is(1, "you");
    let _ = word_view.equals_at(0, &["if", "you"]);
    if matches!(
        clause_words.as_slice(),
        ["if", "you", "do"] | ["when", "you", "do"]
    ) {}
}
"#;

        let functions = extract_functions("probe.rs", source);
        let probe = functions.first().expect("probe function");
        let kinds = classify_function(&probe.file, &probe.body);
        assert!(kinds.contains(&AuditKind::PhraseHelpers));
        assert!(kinds.contains(&AuditKind::ScanHelpers));
        assert!(kinds.contains(&AuditKind::WordSliceShapes));
    }

    #[test]
    fn receiver_probe_detection_is_word_and_clause_specific() {
        for body in [
            "words.starts_with(&[\"if\"])",
            "clause_words.starts_with_at(2, &[\"number\", \"of\"])",
            "tail_words.first_is(\"prevent\")",
            "word_view.ends_with_any(PHRASES)",
            "clause.words().has_phrase(&[\"this\", \"way\"])",
        ] {
            assert!(
                contains_receiver_probe(body, RECEIVER_PHRASE_PROBE_METHODS),
                "expected phrase probe in `{body}`"
            );
        }

        for body in [
            "word_view.at_is(0, \"basic\")",
            "filter_words.equals_any_at(1, PHRASES)",
            "clause_words.strip_prefix_value(PHRASES)",
        ] {
            assert!(
                contains_receiver_probe(body, RECEIVER_WORD_SHAPE_PROBE_METHODS),
                "expected word-shape probe in `{body}`"
            );
        }

        for body in [
            "text.starts_with(\"if\")",
            "effects.ends_with(&tail)",
            "prefix_surface.first_is(CreateWord::Time)",
            "modifier_surface.has_phrase(CreatePhrase::HasteGrant)",
        ] {
            assert!(
                !contains_receiver_probe(body, RECEIVER_PHRASE_PROBE_METHODS),
                "typed/string/collection receiver must not be classified in `{body}`"
            );
        }
    }

    #[test]
    fn compatibility_phrase_probe_detection_is_context_specific() {
        assert!(contains_compat_phrase_probe(
            "ActivationRestrictionCompatWords::new(sentence).has_phrase(PHRASE)"
        ));
        assert!(contains_compat_phrase_probe(
            "let ctx = UnsupportedRewriteLineContext::new(tokens); ctx.has_phrase(PHRASE)"
        ));
        assert!(!contains_compat_phrase_probe(
            "CreationWords::new(words).has_phrase(CreatePhrase::HasteGrant)"
        ));
        for probe in [
            "activation_words_eq(words, PHRASE)",
            "activation_word_at_is_any(words, 2, NOUNS)",
            "cant_attack_unless_tail(words)",
        ] {
            assert!(
                contains_any(probe, PHRASE_HELPER_PATTERNS),
                "activation compatibility probe must remain audited: {probe}"
            );
        }
    }

    #[test]
    fn parser_ownership_checks_cover_staged_pipeline_boundaries() {
        assert_eq!(
            parser_ownership_module_line(
                "crates/ironsmith-compiler/src/runtime_backend/families/example.rs",
                "type TriggerSurface<'a> = &'a [&'a str];",
            ),
            Some(1)
        );
        assert_eq!(
            parser_ownership_module_line(
                "crates/ironsmith-compiler/src/runtime_backend/sentences/example.rs",
                "const PREFIX: ClauseShape<'static> = clause_shape!(prefix & [\"if\"]);",
            ),
            Some(1)
        );
        assert_eq!(
            parser_ownership_module_line(
                "crates/ironsmith-compiler/src/runtime_backend/families/example.rs",
                "const PATTERN: PermissionSequence<'static> = PermissionSequence::new(&[]);",
            ),
            Some(1)
        );
        for compatibility_type in [
            "CantPattern",
            "ValueHelperCompatWords",
            "UtilWordView",
            "PostpassRepairFacts",
        ] {
            assert_eq!(
                parser_ownership_module_line(
                    "crates/ironsmith-compiler/src/runtime_backend/families/example.rs",
                    &format!("type Alias = {compatibility_type};"),
                ),
                Some(1),
                "compatibility parser type `{compatibility_type}` must remain auditable"
            );
        }
        for typed_shape in ["CopyClauseShape", "StatementClauseShape"] {
            assert_eq!(
                parser_ownership_module_line(
                    "crates/ironsmith-compiler/src/runtime_backend/sentences/example.rs",
                    &format!("pub struct {typed_shape};"),
                ),
                None,
                "typed shape `{typed_shape}` must not match standalone ClauseShape"
            );
        }
        assert_eq!(
            parser_ownership_module_line(
                "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/example.rs",
                "pub struct TriggerSurface;",
            ),
            None,
            "typed grammar owns its own surface vocabulary"
        );
        assert!(parser_ownership_function_violation(
            "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/example.rs",
            "fn lower(tokens: &[OwnedLexToken]) { parse_activated_line(tokens); }",
        ));
        assert!(!parser_ownership_function_violation(
            "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/example.rs",
            "fn parse(tokens: &[OwnedLexToken]) { token_word_refs(tokens); }",
        ));
    }

    #[test]
    fn direct_word_slice_match_detection_is_receiver_specific() {
        assert!(contains_direct_word_slice_match(
            "matches!(words.as_slice(), [])"
        ));
        assert!(contains_direct_word_slice_match(
            "match filter_words.as_slice() {}"
        ));
        assert!(contains_direct_word_slice_match(
            "matches!(subject_word_refs.as_slice(), [\"you\"] | [\"they\"])"
        ));
        assert!(contains_direct_word_slice_match(
            "match counter_words { [\"a\"] => 1, _ => 0 }"
        ));
        assert!(contains_direct_word_slice_match("matches!(word_refs, [])"));
        assert!(!contains_direct_word_slice_match(
            "matches!(effects.as_slice(), [])"
        ));
        assert!(!contains_direct_word_slice_match(
            "match targets.as_slice() {}"
        ));
    }
}
