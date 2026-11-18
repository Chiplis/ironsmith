use std::fs;

mod parser_audit_modules;
mod tooling_paths;

const MAX_PRODUCTION_MODULE_LINES: usize = 1_000;

fn main() {
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|error| panic!("failed to locate repo root: {error}"));
    let modules = parser_audit_modules::tracked_production_modules(&repo_root)
        .unwrap_or_else(|error| panic!("failed to enumerate production parser modules: {error}"));

    let mut findings = Vec::new();
    for relative in &modules {
        let path = repo_root.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let lines = source.lines().count();
        if lines > MAX_PRODUCTION_MODULE_LINES {
            findings.push((relative.clone(), lines));
        }
    }

    findings.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    println!("parser modules covered: {}", modules.len());
    println!("automatic production-module limit: {MAX_PRODUCTION_MODULE_LINES} lines");
    println!("module-size findings: {}", findings.len());
    for (path, lines) in &findings {
        println!(
            "{}:{lines}: production parser module exceeds automatic line limit",
            path.display()
        );
    }

    if !findings.is_empty() {
        std::process::exit(1);
    }
}
