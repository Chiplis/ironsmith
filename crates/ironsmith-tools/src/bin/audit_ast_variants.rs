//! Counts the variants of the compiler's largest AST enums (item 7 of the
//! parser architecture migration: splitting the god AST into typed families).
//!
//! A variant that wraps a family enum (`Counters(CounterActionAst)`) counts as
//! one; the family's own variants are listed separately with `--families`.

use std::fs;
use std::path::Path;

mod tooling_paths;

const ENUMS: &[(&str, &str)] = &[
    ("SubjectVerbActionAst", "crates/ironsmith-compiler-semantic/src/model_impl/ast/actions.rs"),
    ("EffectAst", "crates/ironsmith-compiler-semantic/src/model_impl/ast/effects.rs"),
    ("PredicateAst", "crates/ironsmith-compiler-semantic/src/model_impl/ast/predicates.rs"),
];

fn main() {
    let mut families = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--families" => families = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|error| panic!("failed to locate repo root: {error}"));
    for (name, relative) in ENUMS {
        let source = fs::read_to_string(repo_root.join(relative))
            .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"));
        let variants = enum_variants(&source, name);
        println!("{name} variants: {}", variants.len());
    }
    if families {
        for family_dir in ["actions", "effects", "predicates"] {
            let dir = repo_root.join(format!(
                "crates/ironsmith-compiler-semantic/src/model_impl/ast/{family_dir}"
            ));
            if let Ok(entries) = fs::read_dir(&dir) {
                let mut paths: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
                paths.sort();
                for path in paths {
                    report_family(&path);
                }
            }
        }
    }
}

fn report_family(path: &Path) {
    let source = fs::read_to_string(path).unwrap_or_default();
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("pub enum ") {
            let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
            println!("  {name} variants: {}", enum_variants(&source, name).len());
        }
    }
}

/// The variant names of `pub enum {name}` in `source`: lines at indent 4
/// starting with a capital letter, inside the enum's braces.
fn enum_variants(source: &str, name: &str) -> Vec<String> {
    let header = format!("pub enum {name}");
    let Some(start) = source.find(&header) else {
        return Vec::new();
    };
    let body_start = source[start..].find('{').map(|i| start + i + 1).unwrap_or(start);
    let mut depth = 1usize;
    let mut end = body_start;
    for (offset, c) in source[body_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = body_start + offset;
                    break;
                }
            }
            _ => {}
        }
    }
    source[body_start..end]
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("    ")?;
            let first = rest.chars().next()?;
            if first.is_ascii_uppercase() && !line.starts_with("     ") {
                Some(
                    rest.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            } else {
                None
            }
        })
        .collect()
}
