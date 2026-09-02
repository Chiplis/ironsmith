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
    allowed_dependencies: Vec<AllowedEdge>,
    /// Crates that compose phases instead of owning one.
    ///
    /// The phase graph is only a line if no phase crate can reach another. An
    /// orchestrator is the seam where the phases are finally assembled, so it
    /// is the one place allowed to name more than one — and it must own no
    /// phase itself, or the seam would be a phase with extra reach.
    #[serde(default)]
    orchestrators: Vec<String>,
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
    crates: Vec<String>,
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
    audit_complete_phase_ownership(&manifest, completed_pr, &repo_root, &mut findings);
    audit_cross_crate_path_inclusions(&manifest, completed_pr, &repo_root, &mut findings);
    audit_cargo_dependency_graph(&manifest, completed_pr, &repo_root, &mut findings);
    audit_compatibility_reexports(&manifest, completed_pr, &repo_root, &mut findings);
    audit_parallel_semantic_models(&manifest, completed_pr, &repo_root, &mut findings);
    audit_whole_program_recipe_paths(&manifest, completed_pr, &repo_root, &mut findings);
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

fn audit_complete_phase_ownership(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    for path in production_parser_rust_files(manifest, repo_root) {
        let relative = relative_path(repo_root, &path);
        let mut matching_roots = Vec::new();
        for phase in &manifest.phases {
            for root in &phase.roots {
                if root_owns_path(root, &relative) {
                    matching_roots.push((phase.id.as_str(), root.len()));
                }
            }
        }
        let longest_root = matching_roots.iter().map(|(_, len)| *len).max();
        let owners = matching_roots
            .iter()
            .filter(|(_, len)| Some(*len) == longest_root)
            .map(|(owner, _)| *owner)
            .collect::<BTreeSet<_>>();
        if owners.len() != 1 {
            push_scanned_finding(
                manifest,
                completed_pr,
                findings,
                "invalid_phase_ownership",
                "complete-phase-ownership",
                &relative,
                1,
                &format!(
                    "expected exactly one phase owner, found {}: {}",
                    owners.len(),
                    owners.into_iter().collect::<Vec<_>>().join(", ")
                ),
            );
        }
    }
}

fn root_owns_path(root: &str, relative: &str) -> bool {
    relative == root
        || relative
            .strip_prefix(root)
            .is_some_and(|tail| tail.starts_with('/'))
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
        if phase.crates.is_empty() {
            return Err(format!("phase {} has no owning crates", phase.id));
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
    for edge in &manifest.allowed_dependencies {
        if !phase_ids.contains(edge.from.as_str()) || !phase_ids.contains(edge.to.as_str()) {
            return Err(format!(
                "allowed dependency {} -> {} references an unknown phase",
                edge.from, edge.to
            ));
        }
    }
    let mut crate_owners = BTreeMap::<&str, &str>::new();
    for phase in &manifest.phases {
        for crate_name in &phase.crates {
            if let Some(previous) = crate_owners.insert(crate_name, &phase.id) {
                return Err(format!(
                    "crate {crate_name} is owned by both {previous} and {}",
                    phase.id
                ));
            }
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

fn audit_cross_crate_path_inclusions(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    for path in production_parser_rust_files(manifest, repo_root) {
        let relative = relative_path(repo_root, &path);
        let Some(source_crate) = crate_name_from_relative(&relative) else {
            continue;
        };
        scan_lines(&path, |line_number, line| {
            let Some(path_value) = rust_path_attribute_value(line) else {
                return;
            };
            let target = path
                .parent()
                .expect("Rust source has a parent")
                .join(path_value);
            let Ok(target) = fs::canonicalize(&target) else {
                push_scanned_finding(
                    manifest,
                    completed_pr,
                    findings,
                    "invalid_path_inclusion",
                    "compiled-module-path",
                    &relative,
                    line_number,
                    &format!("path attribute target does not exist: {}", target.display()),
                );
                return;
            };
            let target_relative = relative_path(repo_root, &target);
            let Some(target_crate) = crate_name_from_relative(&target_relative) else {
                return;
            };
            if source_crate != target_crate {
                push_scanned_finding(
                    manifest,
                    completed_pr,
                    findings,
                    "cross_crate_path_inclusion",
                    "compiled-module-path",
                    &relative,
                    line_number,
                    &format!("{source_crate} compiles {target_relative} owned by {target_crate}"),
                );
            }
        });
    }
}

fn rust_path_attribute_value(line: &str) -> Option<&str> {
    let marker = line.find("#[path")?;
    let rest = &line[marker..];
    let quote = rest.find('"')?;
    let value = &rest[quote + 1..];
    let end = value.find('"')?;
    Some(&value[..end])
}

fn crate_name_from_relative(relative: &str) -> Option<&str> {
    let mut parts = relative.split('/');
    (parts.next()? == "crates").then_some(parts.next()?)
}

fn audit_cargo_dependency_graph(
    manifest: &OwnershipManifest,
    _completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    let phase_by_crate = manifest
        .phases
        .iter()
        .flat_map(|phase| {
            phase
                .crates
                .iter()
                .map(move |crate_name| (crate_name.as_str(), phase.id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    let allowed = manifest
        .allowed_dependencies
        .iter()
        .map(|edge| (edge.from.as_str(), edge.to.as_str()))
        .collect::<BTreeSet<_>>();

    for (crate_name, source_phase) in &phase_by_crate {
        let crate_name = *crate_name;
        let source_phase = *source_phase;
        let cargo_path = repo_root.join("crates").join(crate_name).join("Cargo.toml");
        if !cargo_path.is_file() {
            findings.push(Finding {
                kind: "missing_phase_crate".to_string(),
                rule_id: "compiled-cargo-graph".to_string(),
                path: relative_path(repo_root, &cargo_path),
                line: None,
                detail: format!("phase {source_phase} has no physical crate {crate_name}"),
                suppressed_by: None,
            });
            continue;
        }
        let source = fs::read_to_string(&cargo_path)
            .unwrap_or_else(|error| panic!("failed reading {}: {error}", cargo_path.display()));
        let parsed = toml::from_str::<toml::Value>(&source)
            .unwrap_or_else(|error| panic!("failed parsing {}: {error}", cargo_path.display()));
        // Dev-dependencies are checked too, under their own finding kind. A test
        // that spans two phases still couples them, and reading only
        // `[dependencies]` would let that coupling grow unseen.
        let sections = [
            ("dependencies", "forbidden_cargo_dependency"),
            ("dev-dependencies", "forbidden_dev_dependency"),
        ];
        for (section, finding_kind) in sections {
            let Some(dependencies) = parsed.get(section).and_then(toml::Value::as_table) else {
                continue;
            };
            for (dependency_key, dependency_value) in dependencies {
                let dependency_name = dependency_value
                    .as_table()
                    .and_then(|table| table.get("package"))
                    .and_then(toml::Value::as_str)
                    .unwrap_or(dependency_key);
                // Depending on an orchestrator reaches every phase it composes,
                // so it is reported as the edge to the phase furthest from this
                // one. Otherwise a phase crate could route around the graph by
                // going through the seam.
                let orchestrator_reach = || {
                    manifest
                        .orchestrators
                        .iter()
                        .any(|name| name == dependency_name)
                        .then(|| {
                            phase_by_crate
                                .values()
                                .copied()
                                .filter(|phase| !allowed.contains(&(source_phase, *phase)))
                                .max_by_key(|phase| phase.len())
                        })
                        .flatten()
                };
                let Some(target_phase) = phase_by_crate
                    .get(dependency_name)
                    .copied()
                    .or_else(orchestrator_reach)
                else {
                    continue;
                };
                if !allowed.contains(&(source_phase, target_phase)) {
                    findings.push(Finding {
                    kind: finding_kind.to_string(),
                    rule_id: "compiled-cargo-graph".to_string(),
                    path: relative_path(repo_root, &cargo_path),
                    line: cargo_dependency_line(&source, dependency_key),
                    detail: format!(
                        "{crate_name} ({source_phase}) depends on {dependency_name} ({target_phase}) via [{section}]"
                    ),
                    suppressed_by: None,
                });
                }
            }
        }
    }
}

fn cargo_dependency_line(source: &str, dependency: &str) -> Option<usize> {
    let prefix = format!("{dependency} ");
    source
        .lines()
        .position(|line| line.trim_start().starts_with(&prefix))
        .map(|index| index + 1)
}

fn audit_compatibility_reexports(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    for phase in &manifest.phases {
        // This invariant governs the parser/compiler phase stack. Runtime is
        // the terminal consumer in that stack and may compose its own runtime
        // implementation crates without creating a parser compatibility
        // bridge.
        if phase.id == "runtime" {
            continue;
        }
        for crate_name in &phase.crates {
            let path = repo_root.join("crates").join(crate_name).join("src/lib.rs");
            if !path.is_file() {
                continue;
            }
            let relative = relative_path(repo_root, &path);
            scan_lines(&path, |line_number, line| {
                let trimmed = line.trim();
                if trimmed.starts_with("pub use ironsmith_") && trimmed.ends_with("::*;") {
                    push_scanned_finding(
                        manifest,
                        completed_pr,
                        findings,
                        "compatibility_glob_reexport",
                        "physical-layer-exports",
                        &relative,
                        line_number,
                        trimmed,
                    );
                }
            });
        }
    }
}

fn audit_parallel_semantic_models(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    let files = production_parser_rust_files(manifest, repo_root);
    let semantic_definitions = [
        "pub enum GiftTimingAst",
        "pub enum LineAst",
        "pub struct AdditionalCostChoiceOptionAst",
        "pub struct ParsedAbility",
        "pub enum ParsedCardItem",
        "pub struct ParsedLineAst",
        "pub struct ParsedModalAst",
        "pub struct ParsedModalHeader",
        "pub struct ParsedModalActivatedHeader",
        "pub struct ParsedModalModeAst",
        "pub struct ParsedModalGate",
        "pub struct ParsedLevelAbilityAst",
        "pub enum ParsedLevelAbilityItemAst",
    ];
    for definition in semantic_definitions {
        let owners = definition_owners(repo_root, &files, definition);
        if owners.len() > 1 {
            let (path, line) = &owners[0];
            push_scanned_finding(
                manifest,
                completed_pr,
                findings,
                "parallel_semantic_model",
                "single-canonical-ast",
                path,
                *line,
                &format!(
                    "{definition} is defined in {}",
                    owners
                        .iter()
                        .map(|(path, _)| path.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
    }

    let action_owners = definition_owners(repo_root, &files, "pub enum SubjectVerbActionAst");
    let clause_owners = definition_owners(repo_root, &files, "pub struct CompilerClauseAst");
    if let (Some((path, line)), Some((clause_path, _))) =
        (action_owners.first(), clause_owners.first())
    {
        push_scanned_finding(
            manifest,
            completed_pr,
            findings,
            "parallel_semantic_model",
            "single-canonical-effect-ast",
            path,
            *line,
            &format!("SubjectVerbActionAst coexists with CompilerClauseAst in {clause_path}"),
        );
    }
}

fn definition_owners(
    repo_root: &Path,
    files: &[PathBuf],
    definition: &str,
) -> Vec<(String, usize)> {
    files
        .iter()
        .filter_map(|path| {
            let source = fs::read_to_string(path).ok()?;
            let line = source.lines().position(|line| line.contains(definition))? + 1;
            Some((relative_path(repo_root, path), line))
        })
        .collect()
}

fn audit_whole_program_recipe_paths(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    for path in production_parser_rust_files(manifest, repo_root) {
        let relative = relative_path(repo_root, &path);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let recipe_path = file_name.ends_with("_programs.rs")
            || file_name == "program.rs"
            || relative.contains("/bundle_rules")
            || relative.contains("/generic_program_shapes")
            || relative.contains("/sequence_pairs/")
            || relative.contains("/triple_sequence_shapes/")
            || relative.contains("/sequence_quad_shapes/");
        if recipe_path {
            push_scanned_finding(
                manifest,
                completed_pr,
                findings,
                "whole_program_recipe",
                "compositional-grammar-only",
                &relative,
                1,
                "program/bundle/exact-sequence module remains in the production parser tree",
            );
        }
    }
}

fn production_parser_rust_files(manifest: &OwnershipManifest, repo_root: &Path) -> Vec<PathBuf> {
    let mut roots = manifest
        .phases
        .iter()
        .filter(|phase| phase.id != "runtime")
        .flat_map(|phase| phase.crates.iter())
        .map(|crate_name| format!("crates/{crate_name}/src"))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();

    let mut files = roots
        .iter()
        .flat_map(|root| rust_files(repo_root, root, &manifest.audit.excluded_path_fragments))
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files
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
                        .any(|fragment| line_contains_fragment(line, fragment))
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

fn line_contains_fragment(line: &str, fragment: &str) -> bool {
    if fragment == "crate::effects::" {
        return line.match_indices(fragment).any(|(start, _)| {
            let tail = &line[start + fragment.len()..];
            let identifier = tail
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>();
            identifier.ends_with("Effect")
        });
    }
    let requires_identifier_boundary = fragment
        .as_bytes()
        .last()
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
    line.match_indices(fragment).any(|(start, _)| {
        if !requires_identifier_boundary {
            return true;
        }
        let end = start + fragment.len();
        !line
            .as_bytes()
            .get(end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    })
}

fn audit_patterns(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    repo_root: &Path,
    findings: &mut Vec<Finding>,
) {
    let production_files = production_parser_rust_files(manifest, repo_root);
    for rule in &manifest.audit.pattern_rules {
        let files = if invariant_pattern_is_global(&rule.id) {
            production_files.clone()
        } else {
            let mut files = rule
                .roots
                .iter()
                .flat_map(|root| {
                    rust_files(repo_root, root, &manifest.audit.excluded_path_fragments)
                })
                .collect::<Vec<_>>();
            files.sort();
            files.dedup();
            files
        };
        if rule.id == "parser-boundary-option-protocol" {
            audit_registry_option_protocol(
                manifest,
                completed_pr,
                rule,
                repo_root,
                &files,
                findings,
            );
            continue;
        }
        for path in files {
            let relative = relative_path(repo_root, &path);
            scan_lines(&path, |line_number, line| {
                if rule.patterns.iter().any(|pattern| line.contains(pattern))
                    && pattern_rule_applies_at_path(rule, &relative)
                {
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

fn audit_registry_option_protocol(
    manifest: &OwnershipManifest,
    completed_pr: u8,
    rule: &PatternRule,
    repo_root: &Path,
    files: &[PathBuf],
    findings: &mut Vec<Finding>,
) {
    for path in files {
        let relative = relative_path(repo_root, path);
        let source = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for line in registry_option_protocol_lines(&source) {
            push_scanned_finding(
                manifest,
                completed_pr,
                findings,
                &rule.kind,
                &rule.id,
                &relative,
                line,
                "registered recognizer function pointer uses Option as its match protocol",
            );
        }
    }
}

/// Find only type declarations that can be stored in a recognizer registry.
/// Optional leaf values remain valid after a rule has committed; the
/// architecture boundary is the function pointer carried by a `Rule`,
/// `Registry`, `Recognizer`, `Handler`, or explicitly named parser type.
fn registry_option_protocol_lines(source: &str) -> Vec<usize> {
    let sanitized = mask_comments_and_literals(source);
    let lines = sanitized.lines().collect::<Vec<_>>();
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

fn invariant_pattern_is_global(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "literal-reference-tags"
            | "registration-order-semantics"
            | "legacy-registry-handlers-and-adapters"
            | "postparse-semantic-repair"
            | "legacy-runtime-semantic-alternatives"
            | "stringly-semantic-tags"
            | "expired-legacy-registries"
            | "hidden-parser-state"
            | "parser-stack-growth"
    )
}

/// The discarded-error rule protects semantic commitment boundaries, not
/// ordinary fallible leaf conversion (for example, parsing a numeric level
/// header). Leaf grammar modernization is enforced independently by
/// `audit_manual_parser_sections`; keeping the scopes separate prevents a
/// textual `.ok()` scan from misclassifying non-recognizer `Result` values.
fn pattern_rule_applies_at_path(rule: &PatternRule, relative: &str) -> bool {
    if rule.id != "discarded-recognizer-errors" {
        return true;
    }

    relative == "crates/ironsmith-compiler/src/registry.rs"
        || relative == "crates/ironsmith-compiler/src/front_end/rule_engine.rs"
        || relative.starts_with("crates/ironsmith-compiler/src/front_end/document/")
        || relative.rsplit('/').next().is_some_and(|name| {
            name == "registry.rs"
                || name.ends_with("_registry.rs")
                || matches!(
                    name,
                    "line_dispatch.rs" | "clause_dispatch.rs" | "dispatch_entry.rs"
                )
        })
}

fn rust_files(repo_root: &Path, root: &str, excluded: &[String]) -> Vec<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            root,
        ])
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

/// The end of a character literal starting at `start`, if one starts there.
///
/// Returns `None` for a lifetime, which shares the opening byte.
fn char_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start + 1;
    if cursor >= bytes.len() {
        return None;
    }
    if bytes[cursor] == b'\\' {
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor] != b'\'' && bytes[cursor] != b'\n' {
            cursor += 1;
        }
        return (cursor < bytes.len() && bytes[cursor] == b'\'').then_some(cursor + 1);
    }
    // One character, then the closing quote. Anything longer is a lifetime.
    let mut end = cursor;
    while end < bytes.len() && (bytes[end] & 0xC0) == 0x80 {
        end += 1;
    }
    end += 1;
    (end < bytes.len() && bytes[end] == b'\'').then_some(end + 1)
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
        // A character literal may hold a quote or a brace (`.replace('"', "")`).
        // Leaving it unmasked desynchronizes string masking and, with it, the
        // brace matching that decides where a `#[cfg(test)]` item ends.
        // Lifetimes (`&'a str`) start with the same byte and must be left alone.
        if bytes[cursor] == b'\''
            && let Some(end) = char_literal_end(bytes, cursor)
        {
            for slot in cursor..end {
                if bytes[slot] != b'\n' {
                    masked[slot] = b' ';
                }
            }
            cursor = end;
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
