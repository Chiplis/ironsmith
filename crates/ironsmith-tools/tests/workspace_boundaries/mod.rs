use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn read_repo_file(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    // Source-boundary checks treat a Rust module as one logical unit. Keep that
    // invariant when a formerly monolithic `module.rs` is split into
    // `module/*.rs`, or when a `mod.rs` delegates to sibling module files.
    let module_dir = if path.file_name().is_some_and(|name| name == "mod.rs") {
        path.parent().map(Path::to_path_buf)
    } else {
        path.file_stem()
            .map(|stem| path.with_file_name(stem))
            .filter(|candidate| candidate.is_dir())
    };
    let Some(module_dir) = module_dir else {
        return source;
    };

    let mut module_files = Vec::new();
    collect_production_rust_files(&module_dir, &path, &mut module_files);
    module_files.sort();
    for module_file in module_files {
        source.push('\n');
        source.push_str(&fs::read_to_string(&module_file).unwrap_or_else(|err| {
            panic!(
                "failed to read split module {}: {err}",
                module_file.display()
            )
        }));
    }
    source
}

fn collect_production_rust_files(root: &Path, primary: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("failed to read {}: {err}", root.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed to enumerate {}: {err}", root.display()));
        let path = entry.path();
        if path == primary {
            continue;
        }
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            collect_production_rust_files(&path, primary, out);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if file_name == "tests.rs" || file_name.ends_with("_tests.rs") {
            continue;
        }
        out.push(path);
    }
}

fn repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("failed to read {}: {err}", root.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed to enumerate {}: {err}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

mod audit_budgets;
mod relex_sites;
mod shard_00;
mod shard_01;
mod shard_02;
