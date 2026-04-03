use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const ALLOWLIST_PATH: &str = "src/cards/builders/parser/migration_audit_allowlist";
const MANUAL_SCAN_PATTERNS: &[Pattern] = &[
    Pattern::with_leading_boundary("words("),
    Pattern::new(".windows("),
    Pattern::new(".position("),
    Pattern::new(".rposition("),
    Pattern::new(".find("),
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum AuditKind {
    ManualScan,
    RawStringAfterLex,
    ControlFlowHelpers,
}

impl AuditKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::ManualScan => "manual_scan",
            Self::RawStringAfterLex => "raw_string_after_lex",
            Self::ControlFlowHelpers => "control_flow_helpers",
        }
    }

    fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "manual_scan" => Some(Self::ManualScan),
            "raw_string_after_lex" => Some(Self::RawStringAfterLex),
            "control_flow_helpers" => Some(Self::ControlFlowHelpers),
            _ => None,
        }
    }

    fn raw_string_patterns(self) -> &'static [&'static str] {
        match self {
            Self::ManualScan => &[],
            Self::RawStringAfterLex => RAW_STRING_PATTERNS,
            Self::ControlFlowHelpers => CONTROL_FLOW_HELPER_PATTERNS,
        }
    }

    fn requires_lexed_context(self) -> bool {
        matches!(self, Self::RawStringAfterLex)
    }
}

#[derive(Debug)]
struct AllowlistEntry {
    max_hits: usize,
    remove_when: String,
}

#[derive(Debug)]
struct AuditSummary {
    total_files: usize,
    total_hits: usize,
}

#[derive(Clone, Copy)]
struct Pattern {
    needle: &'static str,
    require_leading_boundary: bool,
}

impl Pattern {
    const fn new(needle: &'static str) -> Self {
        Self {
            needle,
            require_leading_boundary: false,
        }
    }

    const fn with_leading_boundary(needle: &'static str) -> Self {
        Self {
            needle,
            require_leading_boundary: true,
        }
    }
}

#[test]
fn parser_migration_audit_matches_allowlist() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let allowlist = parse_allowlist(&repo_root);
    let manual = collect_findings(&repo_root, AuditKind::ManualScan);
    let raw = collect_findings(&repo_root, AuditKind::RawStringAfterLex);
    let helpers = collect_findings(&repo_root, AuditKind::ControlFlowHelpers);

    print_summary(AuditKind::ManualScan, &manual);
    print_summary(AuditKind::RawStringAfterLex, &raw);
    print_summary(AuditKind::ControlFlowHelpers, &helpers);

    let mut failures = Vec::new();
    for (kind, findings) in [
        (AuditKind::ManualScan, &manual),
        (AuditKind::RawStringAfterLex, &raw),
        (AuditKind::ControlFlowHelpers, &helpers),
    ] {
        let allowed_entries = match allowlist.get(&kind) {
            Some(entries) => entries,
            None if findings.is_empty() => continue,
            None => {
                failures.push(format!(
                    "{} is missing from {}",
                    kind.as_str(),
                    allowlist_path(&repo_root).display()
                ));
                continue;
            }
        };

        for (path, hits) in findings {
            match allowed_entries.get(path) {
                Some(entry) if *hits <= entry.max_hits => {}
                Some(entry) => failures.push(format!(
                    "{} {} now has {} hits (allowlist max {}). Remove when: {}",
                    kind.as_str(),
                    path,
                    hits,
                    entry.max_hits,
                    entry.remove_when
                )),
                None => failures.push(format!(
                    "{} {} has {} hits and is not allowlisted",
                    kind.as_str(),
                    path,
                    hits
                )),
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "parser migration audit failed:\n{}\n\nUpdate {} only when the new debt is intentional and has a named removal plan.",
            failures.join("\n"),
            allowlist_path(&repo_root).display()
        );
    }
}

fn print_summary(kind: AuditKind, findings: &BTreeMap<String, usize>) {
    let summary = summarize(findings);
    let mut hotspots = findings
        .iter()
        .map(|(path, hits)| (path.as_str(), *hits))
        .collect::<Vec<_>>();
    hotspots.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));

    println!(
        "{}: {} files / {} hits",
        kind.as_str(),
        summary.total_files,
        summary.total_hits
    );
    for (path, hits) in hotspots.into_iter().take(10) {
        println!("  {hits:>4}  {path}");
    }
}

fn summarize(findings: &BTreeMap<String, usize>) -> AuditSummary {
    AuditSummary {
        total_files: findings.len(),
        total_hits: findings.values().sum(),
    }
}

fn collect_findings(repo_root: &Path, kind: AuditKind) -> BTreeMap<String, usize> {
    let parser_root = repo_root.join("src/cards/builders/parser");
    let mut files = Vec::new();
    collect_parser_files(&parser_root, &mut files);

    let mut findings = BTreeMap::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()));
        if kind.requires_lexed_context() && !has_lexed_context(&source) {
            continue;
        }

        let hits = match kind {
            AuditKind::ManualScan => count_manual_patterns(&source),
            AuditKind::RawStringAfterLex => {
                count_string_patterns(&source, kind.raw_string_patterns())
            }
            AuditKind::ControlFlowHelpers => {
                count_string_patterns(&source, kind.raw_string_patterns())
            }
        };
        if hits == 0 {
            continue;
        }

        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or_else(|err| panic!("failed to relativize {}: {err}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");
        findings.insert(rel, hits);
    }

    findings
}

fn collect_parser_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed reading directory {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("failed reading directory entry: {err}"));
        let path = entry.path();
        if path.is_dir() {
            collect_parser_files(&path, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("tests.rs") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("migration_audit.rs") {
            continue;
        }
        out.push(path);
    }
    out.sort();
}

fn count_manual_patterns(source: &str) -> usize {
    MANUAL_SCAN_PATTERNS
        .iter()
        .map(|pattern| count_pattern(source, *pattern))
        .sum()
}

fn count_string_patterns(source: &str, patterns: &[&str]) -> usize {
    patterns
        .iter()
        .map(|pattern| source.match_indices(pattern).count())
        .sum()
}

fn count_pattern(source: &str, pattern: Pattern) -> usize {
    source
        .match_indices(pattern.needle)
        .filter(|(start, _)| {
            !pattern.require_leading_boundary || {
                let prefix = &source[..*start];
                prefix
                    .chars()
                    .next_back()
                    .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
            }
        })
        .count()
}

fn has_lexed_context(source: &str) -> bool {
    LEXED_CONTEXT_MARKERS
        .iter()
        .any(|marker| source.contains(marker))
}

fn parse_allowlist(repo_root: &Path) -> BTreeMap<AuditKind, BTreeMap<String, AllowlistEntry>> {
    let text = fs::read_to_string(allowlist_path(repo_root)).unwrap_or_else(|err| {
        panic!(
            "failed reading parser migration allowlist {}: {err}",
            allowlist_path(repo_root).display()
        )
    });
    let mut entries: BTreeMap<AuditKind, BTreeMap<String, AllowlistEntry>> = BTreeMap::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.splitn(4, '|');
        let kind = parts
            .next()
            .and_then(AuditKind::from_str)
            .unwrap_or_else(|| panic!("invalid audit kind on line {}", line_index + 1));
        let path = parts
            .next()
            .unwrap_or_else(|| panic!("missing path on line {}", line_index + 1))
            .trim();
        let max_hits = parts
            .next()
            .unwrap_or_else(|| panic!("missing max_hits on line {}", line_index + 1))
            .trim()
            .parse::<usize>()
            .unwrap_or_else(|err| panic!("invalid max_hits on line {}: {err}", line_index + 1));
        let remove_when = parts
            .next()
            .unwrap_or_else(|| panic!("missing removal note on line {}", line_index + 1))
            .trim();

        assert!(
            !remove_when.is_empty(),
            "empty removal note on line {}",
            line_index + 1
        );
        assert!(
            max_hits > 0,
            "max_hits must be positive on line {}",
            line_index + 1
        );

        let entry = AllowlistEntry {
            max_hits,
            remove_when: remove_when.to_string(),
        };
        let prior = entries
            .entry(kind)
            .or_default()
            .insert(path.to_string(), entry);
        assert!(
            prior.is_none(),
            "duplicate allowlist entry for {} {}",
            kind.as_str(),
            path
        );
    }

    entries
}

fn allowlist_path(repo_root: &Path) -> PathBuf {
    repo_root.join(ALLOWLIST_PATH)
}
