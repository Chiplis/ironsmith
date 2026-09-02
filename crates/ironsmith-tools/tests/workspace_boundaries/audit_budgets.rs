//! Enforce the parser audits instead of merely keeping them wired.
//!
//! Each audit reports debt the migration has not paid off yet, so a
//! zero-findings gate would fail on work that is already known and tracked. A
//! recorded budget is the enforceable form of that: the release gate runs every
//! audit and fails when a finding count rises, so the numbers can only move
//! down. Lower the budget in the same change that removes the findings.

use std::process::Command;

use super::workspace_root;

/// Architecture findings, all four of them phase-crate coupling that survived
/// the lowering extraction.
///
/// One is a production edge: recognition still calls the resolver, because a
/// handful of recognizers resolve a condition while recognizing it. The other
/// three are test-only — the grammar and lowering crates dev-depend on the
/// orchestrator so their recognition-to-runtime tests can name both phases.
/// Those are reported under their own kind rather than waved through: the
/// compiled library graph is acyclic and rustc enforces it, but a test that
/// spans two phases still couples them, and the count keeps that visible.
const ARCHITECTURE_BUDGET: usize = 4;

/// Manual-parser sections still recognized by hand-rolled scans rather than
/// typed leaves.
const MANUAL_PARSER_BUDGET: usize = 38;

/// Production modules over the 1,000-line limit.
const MODULE_SIZE_BUDGET: usize = 92;

fn audit_output(binary: &str, arguments: &[&str]) -> String {
    let root = workspace_root();
    let output = Command::new(binary)
        .args(arguments)
        .current_dir(&root)
        .output()
        .unwrap_or_else(|err| panic!("failed to run {binary}: {err}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        !stdout.trim().is_empty(),
        "{binary} produced no report; the gate cannot enforce an audit it cannot read"
    );
    stdout
}

fn reported_count(report: &str, label: &str) -> usize {
    let line = report
        .lines()
        .find(|line| line.trim_start().starts_with(label))
        .unwrap_or_else(|| panic!("audit report is missing `{label}`:\n{report}"));
    line.rsplit(':')
        .next()
        .and_then(|count| count.trim().parse::<usize>().ok())
        .unwrap_or_else(|| panic!("could not read a count from `{line}`"))
}

fn assert_within_budget(name: &str, count: usize, budget: usize) {
    assert!(
        count <= budget,
        "{name} findings rose from {budget} to {count}. This audit is enforced: either \
         remove the new findings, or, if the increase is intended, say so in the \
         architecture progress doc and raise the budget deliberately."
    );
    assert!(
        count == budget,
        "{name} findings fell from {budget} to {count}. Lower the budget in this change so \
         the gate keeps the ground you just took."
    );
}

#[test]
pub(super) fn parser_architecture_audit_is_enforced() {
    let report = audit_output(env!("CARGO_BIN_EXE_audit_parser_architecture"), &[]);
    assert_within_budget(
        "architecture",
        reported_count(&report, "active findings:"),
        ARCHITECTURE_BUDGET,
    );
}

#[test]
pub(super) fn manual_parser_audit_is_enforced() {
    let report = audit_output(env!("CARGO_BIN_EXE_audit_manual_parser_sections"), &[]);
    assert_within_budget(
        "manual-parser",
        reported_count(&report, "total_sections:"),
        MANUAL_PARSER_BUDGET,
    );
}

#[test]
pub(super) fn parser_module_size_audit_is_enforced() {
    let report = audit_output(env!("CARGO_BIN_EXE_audit_parser_module_sizes"), &[]);
    assert_within_budget(
        "module-size",
        reported_count(&report, "module-size findings:"),
        MODULE_SIZE_BUDGET,
    );
}
