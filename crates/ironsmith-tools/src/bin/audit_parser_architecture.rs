use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod tooling_paths;

const MANIFEST_PATH: &str = "architecture/parser-ownership-manifest.json";

#[derive(Debug, Deserialize)]
struct OwnershipManifest {
    schema_version: u32,
    program: Program,
    phases: Vec<Phase>,
    allowed_edges: Vec<AllowedEdge>,
    bridges: Vec<Bridge>,
    exceptions: Vec<Exception>,
    audit: AuditConfiguration,
}

#[derive(Debug, Deserialize)]
struct Program {
    specification: String,
    implementation_prs: u8,
    integration_pr: u8,
    audit_default_completed_pr: u8,
}

#[derive(Debug, Deserialize)]
struct Phase {
    id: String,
    roots: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AllowedEdge {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct Bridge {
    id: String,
    paths: Vec<String>,
    removal_pr: u8,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct Exception {
    id: String,
    finding_kinds: Vec<String>,
    path_prefixes: Vec<String>,
    removal_pr: u8,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct AuditConfiguration {
    excluded_path_fragments: Vec<String>,
    forbidden_imports: Vec<ForbiddenImportRule>,
    pattern_rules: Vec<PatternRule>,
    legacy_paths: Vec<LegacyPath>,
}

#[derive(Debug, Deserialize)]
struct ForbiddenImportRule {
    id: String,
    source_prefixes: Vec<String>,
    fragments: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PatternRule {
    id: String,
    kind: String,
    roots: Vec<String>,
    patterns: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LegacyPath {
    id: String,
    path: String,
    removal_pr: u8,
}

#[derive(Debug)]
struct Finding {
    kind: String,
    rule_id: String,
    path: String,
    line: Option<usize>,
    detail: String,
    suppressed_by: Option<String>,
}

#[derive(Debug)]
struct Arguments {
    completed_pr: Option<u8>,
    fail_on_findings: bool,
    verbose: bool,
}

fn main() {
    let arguments = parse_arguments().unwrap_or_else(|error| {
        eprintln!("{error}");
        print_usage();
        std::process::exit(2);
    });
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|error| panic!("failed to locate repository root: {error}"));
    let manifest = read_manifest(&repo_root)
        .unwrap_or_else(|error| panic!("failed to read parser ownership manifest: {error}"));
    validate_manifest(&manifest)
        .unwrap_or_else(|error| panic!("invalid parser ownership manifest: {error}"));

    let completed_pr = arguments
        .completed_pr
        .unwrap_or(manifest.program.audit_default_completed_pr);
    let mut findings = Vec::new();
    audit_expired_migration_state(&manifest, completed_pr, &repo_root, &mut findings);
    audit_forbidden_imports(&manifest, completed_pr, &repo_root, &mut findings);
    audit_patterns(&manifest, completed_pr, &repo_root, &mut findings);
    print_report(&manifest, completed_pr, &findings, arguments.verbose);

    if arguments.fail_on_findings
        && findings
            .iter()
            .any(|finding| finding.suppressed_by.is_none())
    {
        std::process::exit(1);
    }
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut completed_pr = None;
    let mut fail_on_findings = false;
    let mut verbose = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--completed-pr" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--completed-pr requires a value".to_string())?;
                completed_pr = Some(
                    value
                        .parse::<u8>()
                        .map_err(|_| format!("invalid PR number: {value}"))?,
                );
            }
            "--fail-on-findings" => fail_on_findings = true,
            "--verbose" => verbose = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(Arguments {
        completed_pr,
        fail_on_findings,
        verbose,
    })
}

fn print_usage() {
    eprintln!(
        "usage: audit_parser_architecture [--completed-pr <1..34>] [--fail-on-findings] [--verbose]"
    );
}

fn read_manifest(repo_root: &Path) -> Result<OwnershipManifest, String> {
    let path = repo_root.join(MANIFEST_PATH);
    let source =
        fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&source).map_err(|error| format!("{}: {error}", path.display()))
}

fn validate_manifest(manifest: &OwnershipManifest) -> Result<(), String> {
    if manifest.schema_version != 1 {
        return Err(format!(
            "unsupported schema version {}",
            manifest.schema_version
        ));
    }
    if manifest.program.implementation_prs != 33 || manifest.program.integration_pr != 34 {
        return Err("migration program must contain PR-01 through PR-34".to_string());
    }
    if manifest.program.specification.trim().is_empty() {
        return Err("program specification path cannot be empty".to_string());
    }

    let mut ids = BTreeSet::new();
    let phase_ids: BTreeSet<_> = manifest
        .phases
        .iter()
        .map(|phase| phase.id.as_str())
        .collect();
    if phase_ids.len() != manifest.phases.len() {
        return Err("phase IDs must be unique".to_string());
    }
    for phase in &manifest.phases {
        if phase.roots.is_empty() {
            return Err(format!("phase {} has no ownership roots", phase.id));
        }
    }
    for edge in &manifest.allowed_edges {
        if !phase_ids.contains(edge.from.as_str()) || !phase_ids.contains(edge.to.as_str()) {
            return Err(format!(
                "allowed edge {} -> {} references an unknown phase",
                edge.from, edge.to
            ));
        }
    }
    for bridge in &manifest.bridges {
        validate_migration_item(
            "bridge",
            &bridge.id,
            bridge.removal_pr,
            &bridge.reason,
            &mut ids,
        )?;
        if bridge.paths.is_empty() {
            return Err(format!("bridge {} has no paths", bridge.id));
        }
    }
    for exception in &manifest.exceptions {
        validate_migration_item(
            "exception",
            &exception.id,
            exception.removal_pr,
            &exception.reason,
            &mut ids,
        )?;
        if exception.finding_kinds.is_empty() || exception.path_prefixes.is_empty() {
            return Err(format!(
                "exception {} must name finding kinds and path prefixes",
                exception.id
            ));
        }
    }
    Ok(())
}

fn validate_migration_item(
    item_kind: &str,
    id: &str,
    removal_pr: u8,
    reason: &str,
    ids: &mut BTreeSet<String>,
) -> Result<(), String> {
    if !ids.insert(id.to_string()) {
        return Err(format!("duplicate migration ID: {id}"));
    }
    if !(2..=33).contains(&removal_pr) {
        return Err(format!(
            "{item_kind} {id} has invalid removal PR {removal_pr}"
        ));
    }
    if reason.trim().is_empty() {
        return Err(format!("{item_kind} {id} has no reason"));
    }
    Ok(())
}

fn audit_expired_migration_state(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    for bridge in &manifest.bridges {
        if completed_pr >= bridge.removal_pr {
            findings.push(Finding {
                kind: "expired_bridge".to_string(),
                rule_id: bridge.id.clone(),
                path: bridge.paths.join(", "),
                line: None,
                detail: format!(
                    "bridge remained after its PR-{:02} removal deadline",
                    bridge.removal_pr
                ),
                suppressed_by: None,
            });
        }
    }
    for exception in &manifest.exceptions {
        if completed_pr >= exception.removal_pr {
            findings.push(Finding {
                kind: "expired_exception".to_string(),
                rule_id: exception.id.clone(),
                path: exception.path_prefixes.join(", "),
                line: None,
                detail: format!(
                    "exception remained after its PR-{:02} removal deadline",
                    exception.removal_pr
                ),
                suppressed_by: None,
            });
        }
    }
    for legacy in &manifest.audit.legacy_paths {
        if tracked_path_exists(repo_root, &legacy.path) {
            let suppressed_by = (completed_pr < legacy.removal_pr)
                .then(|| format!("scheduled for PR-{:02}", legacy.removal_pr));
            findings.push(Finding {
                kind: "legacy_path".to_string(),
                rule_id: legacy.id.clone(),
                path: legacy.path.clone(),
                line: None,
                detail: format!(
                    "legacy path still exists (deadline PR-{:02})",
                    legacy.removal_pr
                ),
                suppressed_by,
            });
        }
    }
}

fn audit_forbidden_imports(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    for rule in &manifest.audit.forbidden_imports {
        for source_prefix in &rule.source_prefixes {
            for path in rust_files(
                repo_root,
                source_prefix,
                &manifest.audit.excluded_path_fragments,
            ) {
                let relative = relative_path(repo_root, &path);
                scan_lines(&path, |line_number, line| {
                    if rule
                        .fragments
                        .iter()
                        .any(|fragment| line.contains(fragment))
                    {
                        push_scanned_finding(
                            manifest,
                            completed_pr,
                            findings,
                            "forbidden_import",
                            &rule.id,
                            &relative,
                            line_number,
                            line.trim(),
                        );
                    }
                });
            }
        }
    }
}

fn audit_patterns(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    for rule in &manifest.audit.pattern_rules {
        for root in &rule.roots {
            for path in rust_files(repo_root, root, &manifest.audit.excluded_path_fragments) {
                let relative = relative_path(repo_root, &path);
                scan_lines(&path, |line_number, line| {
                    if rule.patterns.iter().any(|pattern| line.contains(pattern)) {
                        push_scanned_finding(
                            manifest,
                            completed_pr,
                            findings,
                            &rule.kind,
                            &rule.id,
                            &relative,
                            line_number,
                            line.trim(),
                        );
                    }
                });
            }
        }
    }
}

fn rust_files(repo_root: &Path, root: &str, excluded: &[String]) -> Vec<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files", "--", root])
        .output()
        .unwrap_or_else(|error| panic!("failed to enumerate tracked parser sources: {error}"));
    assert!(
        output.status.success(),
        "git ls-files failed while enumerating {root}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut files = String::from_utf8(output.stdout)
        .expect("git ls-files returned a non-UTF-8 path")
        .lines()
        .filter(|relative| relative.ends_with(".rs"))
        .filter(|relative| !excluded.iter().any(|fragment| relative.contains(fragment)))
        .map(|relative| repo_root.join(relative))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files
}

fn tracked_path_exists(repo_root: &Path, relative: &str) -> bool {
    !rust_files(repo_root, relative, &[]).is_empty()
}

fn scan_lines(path: &Path, mut inspect: impl FnMut(usize, &str)) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let test_lines = cfg_test_lines(&source);
    for (index, line) in source.lines().enumerate() {
        if !test_lines.get(index).copied().unwrap_or(false) {
            inspect(index + 1, line);
        }
    }
}

fn cfg_test_lines(source: &str) -> Vec<bool> {
    let masked = mask_comments_and_literals(source);
    let bytes = masked.as_bytes();
    let mut ranges = Vec::new();
    let mut cursor = 0;
    while cursor + 1 < bytes.len() {
        let Some(relative) = masked[cursor..].find("#[") else {
            break;
        };
        let start = cursor + relative;
        let end = matching_delimiter(bytes, start + 1, b'[', b']');
        let attribute = &masked[start..end];
        if attribute.contains("cfg")
            && attribute
                .split(|ch: char| !ch.is_alphanumeric())
                .any(|word| word == "test")
        {
            ranges.push((start, cfg_item_end(&masked, end)));
            cursor = ranges.last().expect("range just pushed").1;
        } else {
            cursor = end;
        }
    }

    let mut test_lines = vec![false; source.lines().count()];
    let mut line_starts = vec![0usize];
    line_starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
    for (start, end) in ranges {
        let first = line_starts
            .partition_point(|offset| *offset <= start)
            .saturating_sub(1);
        let last = line_starts.partition_point(|offset| *offset < end);
        for line in first..last.min(test_lines.len()) {
            test_lines[line] = true;
        }
    }
    test_lines
}

fn cfg_item_end(masked: &str, attribute_end: usize) -> usize {
    let bytes = masked.as_bytes();
    let mut cursor = skip_attributes(masked, attribute_end);
    let head_end = (cursor + 300).min(masked.len());
    let braced_item = masked[cursor..head_end]
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .take(12)
        .any(|word| {
            matches!(
                word,
                "fn" | "mod" | "impl" | "struct" | "enum" | "union" | "trait" | "macro_rules"
            )
        });
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' if paren_depth == 0 && bracket_depth == 0 => {
                let end = matching_delimiter(bytes, cursor, b'{', b'}');
                if braced_item {
                    return end;
                }
                cursor = end;
                continue;
            }
            b';' if paren_depth == 0 && bracket_depth == 0 => return cursor + 1,
            b',' if paren_depth == 0 && bracket_depth == 0 && !braced_item => return cursor + 1,
            _ => {}
        }
        cursor += 1;
    }
    bytes.len()
}

fn skip_attributes(masked: &str, mut cursor: usize) -> usize {
    let bytes = masked.as_bytes();
    loop {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if bytes.get(cursor..cursor + 2) != Some(b"#[") {
            return cursor;
        }
        cursor = matching_delimiter(bytes, cursor + 1, b'[', b']');
    }
}

fn matching_delimiter(bytes: &[u8], start: usize, opening: u8, closing: u8) -> usize {
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate().skip(start) {
        if *byte == opening {
            depth += 1;
        } else if *byte == closing {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return index + 1;
            }
        }
    }
    bytes.len()
}

fn mask_comments_and_literals(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut cursor = 0usize;
    let mut block_depth = 0usize;
    while cursor < bytes.len() {
        if block_depth > 0 {
            if bytes.get(cursor..cursor + 2) == Some(b"/*") {
                masked[cursor..cursor + 2].fill(b' ');
                block_depth += 1;
                cursor += 2;
            } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
                masked[cursor..cursor + 2].fill(b' ');
                block_depth -= 1;
                cursor += 2;
            } else {
                if bytes[cursor] != b'\n' {
                    masked[cursor] = b' ';
                }
                cursor += 1;
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"//") {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                masked[cursor] = b' ';
                cursor += 1;
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            masked[cursor..cursor + 2].fill(b' ');
            block_depth = 1;
            cursor += 2;
            continue;
        }
        if bytes[cursor] == b'"' {
            masked[cursor] = b' ';
            cursor += 1;
            let mut escaped = false;
            while cursor < bytes.len() {
                let byte = bytes[cursor];
                if byte != b'\n' {
                    masked[cursor] = b' ';
                }
                cursor += 1;
                if byte == b'"' && !escaped {
                    break;
                }
                escaped = byte == b'\\' && !escaped;
                if byte != b'\\' {
                    escaped = false;
                }
            }
            continue;
        }
        cursor += 1;
    }
    String::from_utf8(masked).expect("mask preserved UTF-8 source bytes")
}

#[allow(clippy::too_many_arguments)]
fn push_scanned_finding(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    findings: &mut Vec<Finding>,
    kind: &str,
    rule_id: &str,
    path: &str,
    line: usize,
    detail: &str,
) {
    let suppressed_by = manifest
        .exceptions
        .iter()
        .find(|exception| {
            completed_pr < exception.removal_pr
                && exception
                    .finding_kinds
                    .iter()
                    .any(|candidate| candidate == kind)
                && exception
                    .path_prefixes
                    .iter()
                    .any(|prefix| path.starts_with(prefix))
        })
        .map(|exception| format!("{} until PR-{:02}", exception.id, exception.removal_pr));
    findings.push(Finding {
        kind: kind.to_string(),
        rule_id: rule_id.to_string(),
        path: path.to_string(),
        line: Some(line),
        detail: detail.to_string(),
        suppressed_by,
    });
}

fn relative_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn print_report(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    findings: &[Finding],
    verbose: bool,
) {
    let active: Vec<_> = findings
        .iter()
        .filter(|finding| finding.suppressed_by.is_none())
        .collect();
    let suppressed: Vec<_> = findings
        .iter()
        .filter(|finding| finding.suppressed_by.is_some())
        .collect();
    let mut counts = BTreeMap::<&str, usize>::new();
    for finding in &active {
        *counts.entry(&finding.kind).or_default() += 1;
    }

    println!("parser architecture audit (completed PR-{completed_pr:02})");
    println!("manifest schema: {}", manifest.schema_version);
    println!("active findings: {}", active.len());
    println!("scheduled findings: {}", suppressed.len());
    for (kind, count) in counts {
        println!("  {kind}: {count}");
    }

    let display_limit = if verbose { usize::MAX } else { 50 };
    for finding in active.iter().take(display_limit) {
        let location = finding
            .line
            .map(|line| format!("{}:{line}", finding.path))
            .unwrap_or_else(|| finding.path.clone());
        println!(
            "{} [{}] {}: {}",
            finding.kind, finding.rule_id, location, finding.detail
        );
    }
    if active.len() > display_limit {
        println!(
            "{} additional active findings omitted; pass --verbose to show all",
            active.len() - display_limit
        );
    }
}
