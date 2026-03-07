use std::fs;
use std::path::{Path, PathBuf};

fn builders_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("cards")
        .join("builders")
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", root.display());
    });

    for entry in entries {
        let entry = entry.unwrap_or_else(|err| {
            panic!("failed to enumerate {}: {err}", root.display());
        });
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "rs") && !path.ends_with("tests.rs") {
            out.push(path);
        }
    }
}

fn production_builder_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(&builders_dir(), &mut files);
    files.sort();
    files
}

fn parse_boundary_files() -> Vec<PathBuf> {
    production_builder_files()
        .into_iter()
        .filter(|path| {
            path.ends_with("parser.rs")
                || path
                    .components()
                    .any(|component| component.as_os_str() == "parse_parsing")
        })
        .collect()
}

fn strip_cfg_test_modules(content: &str) -> String {
    fn brace_delta(line: &str) -> i32 {
        let opens = line.bytes().filter(|b| *b == b'{').count() as i32;
        let closes = line.bytes().filter(|b| *b == b'}').count() as i32;
        opens - closes
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut out = String::new();
    let mut idx = 0usize;

    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        let inline_cfg_test_mod = trimmed.starts_with("#[cfg(test)]")
            && trimmed.contains("mod ")
            && trimmed.contains('{');
        if inline_cfg_test_mod {
            let mut depth = brace_delta(lines[idx]);
            idx += 1;
            while idx < lines.len() && depth > 0 {
                depth += brace_delta(lines[idx]);
                idx += 1;
            }
            continue;
        }

        if trimmed == "#[cfg(test)]" {
            let mut lookahead = idx + 1;
            while lookahead < lines.len() && lines[lookahead].trim().is_empty() {
                lookahead += 1;
            }
            if lookahead < lines.len() && lines[lookahead].trim_start().starts_with("mod ") {
                let mut depth = brace_delta(lines[lookahead]);
                idx = lookahead + 1;
                while idx < lines.len() && depth > 0 {
                    depth += brace_delta(lines[idx]);
                    idx += 1;
                }
                continue;
            }
        }

        out.push_str(lines[idx]);
        out.push('\n');
        idx += 1;
    }

    out
}

fn stripped_file(path: &Path) -> String {
    let raw = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    });
    strip_cfg_test_modules(&raw)
}

fn forbidden_prefixed_calls(content: &str, prefix: &str) -> Vec<String> {
    let mut hits = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let mut scan_from = 0usize;
        while let Some(offset) = line[scan_from..].find(prefix) {
            let start = scan_from + offset;
            let previous = line[..start].chars().last();
            if previous.is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
                scan_from = start + 1;
                continue;
            }

            let rest = &line[start..];
            let ident_len = rest
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            let after_ident = rest[ident_len..].trim_start();
            let before_ident = line[..start].trim_end();
            let is_definition = before_ident.ends_with("fn");

            if after_ident.starts_with('(') && !is_definition {
                hits.push(format!("{}:{}", line_no + 1, trimmed));
                break;
            }

            scan_from = start + 1;
        }
    }

    hits
}

fn forbidden_downcasts(content: &str) -> Vec<String> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line_no, line)| {
            let trimmed = line.trim();
            if trimmed.contains("downcast_ref") || trimmed.contains("downcast_mut") {
                Some(format!("{}:{}", line_no + 1, trimmed))
            } else {
                None
            }
        })
        .collect()
}

fn whitespace_squashed(content: &str) -> String {
    content.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn parse_modules_do_not_call_compile_or_lower_helpers() {
    let mut violations = Vec::new();

    for path in parse_boundary_files() {
        let content = stripped_file(&path);
        let compile_hits = forbidden_prefixed_calls(&content, "compile_");
        let lower_hits = forbidden_prefixed_calls(&content, "lower_");

        if !compile_hits.is_empty() || !lower_hits.is_empty() {
            violations.push(format!(
                "{}\n{}\n{}",
                path.display(),
                compile_hits.join("\n"),
                lower_hits.join("\n")
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "parse/lower boundary violations:\n{}",
        violations.join("\n\n")
    );
}

#[test]
fn parse_modules_do_not_downcast_compiled_effects() {
    let mut violations = Vec::new();

    for path in parse_boundary_files() {
        let content = stripped_file(&path);
        let hits = forbidden_downcasts(&content);
        if !hits.is_empty() {
            violations.push(format!("{}\n{}", path.display(), hits.join("\n")));
        }
    }

    assert!(
        violations.is_empty(),
        "compiled-effect downcasts remain in parse modules:\n{}",
        violations.join("\n\n")
    );
}

#[test]
fn ability_lowering_avoids_raw_compile_effect_entrypoints() {
    let path = builders_dir().join("ability_lowering.rs");
    let content = stripped_file(&path);
    let forbidden = [
        "compile_statement_effects(",
        "compile_statement_effects_with_imports(",
        "compile_trigger_effects(",
        "compile_trigger_effects_with_imports(",
        "compile_trigger_effects_with_intervening_if(",
        "compile_trigger_effects_with_intervening_if_imports(",
    ];

    let hits: Vec<&str> = forbidden
        .into_iter()
        .filter(|needle| content.contains(needle))
        .collect();

    assert!(
        hits.is_empty(),
        "ability_lowering.rs still calls raw compile entrypoints:\n{}",
        hits.join("\n")
    );
}

#[test]
fn builders_pipeline_has_no_card_name_branching() {
    let mut violations = Vec::new();

    for path in production_builder_files() {
        let content = stripped_file(&path);
        let squashed = whitespace_squashed(&content);
        let branched = squashed.contains("if builder.card_builder.name_ref()")
            || squashed.contains("match builder.card_builder.name_ref()")
            || squashed.contains("matches!(builder.card_builder.name_ref()");
        if branched {
            violations.push(path.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "card-name branching remains in builders pipeline:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_builders_do_not_use_super_wildcards() {
    let mut violations = Vec::new();

    for path in production_builder_files() {
        let content = stripped_file(&path);
        for (line_no, line) in content.lines().enumerate() {
            if line.trim() == "use super::*;" {
                violations.push(format!("{}:{}", path.display(), line_no + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "production wildcard imports remain:\n{}",
        violations.join("\n")
    );
}
