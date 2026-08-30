use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod tooling_paths;

const MAX_PRODUCTION_MODULE_LINES: usize = 1_000;

fn main() {
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|error| panic!("failed to locate repo root: {error}"));
    let modules = tracked_production_modules(&repo_root)
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

fn tracked_production_modules(repo_root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ])
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
    use super::{is_parser_module, is_test_only};
    use std::path::Path;

    #[test]
    fn covers_split_crates_and_current_parser_owners() {
        assert!(is_parser_module(Path::new(
            "crates/ironsmith-compiler/src/front_end/grammar/leaf.rs"
        )));
        assert!(is_parser_module(Path::new(
            "crates/ironsmith-compiler-syntax/src/lib.rs"
        )));
        assert!(!is_parser_module(Path::new(
            "crates/ironsmith-engine/src/lib.rs"
        )));
    }

    #[test]
    fn excludes_only_standalone_test_modules() {
        assert!(is_test_only(Path::new(
            "crates/ironsmith-compiler/src/front_end/tests/parser.rs"
        )));
        assert!(is_test_only(Path::new(
            "crates/ironsmith-compiler/src/model/reference_tests.rs"
        )));
        assert!(!is_test_only(Path::new(
            "crates/ironsmith-compiler/src/front_end/grammar/trigger.rs"
        )));
    }
}
