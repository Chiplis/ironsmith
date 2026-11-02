//! Catch unregistered, deleted or multiply registered integration-test sources.
//! Suite sources and explicit standalone targets share one exact source universe.
fn native_suite_helper_only(path: &std::path::Path) -> bool {
    let text = std::fs::read_to_string(path).unwrap();
    if text.lines().next() != Some("// native-test-suite: helper-only") { return false; }
    let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    for attribute in ["#[test]", "#[bench]", "#[tokio::test]", "#[async_std::test]"] {
        assert!(!compact.contains(attribute), "helper-only source contains tests: {}", path.display());
    }
    true
}
fn native_suite_string<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (actual, value) = line.split_once('=')?;
    if actual.trim() != key { return None; }
    let value = value.trim();
    let quote = value.chars().next().expect("missing manifest string");
    assert!(quote == '"' || quote == '\'', "test name/path must be a quoted string");
    let rest = &value[1..];
    let end = rest.find(quote).expect("unterminated manifest string");
    let tail = rest[end + 1..].trim();
    assert!(tail.is_empty() || tail.starts_with('#'), "trailing manifest string syntax");
    let string = &rest[..end];
    assert!(!string.contains('\\'), "test name/path must use portable literal paths");
    Some(string)
}
#[test]
fn every_integration_test_source_is_registered_once() {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tests = root.join("tests");
    let mut expected = BTreeSet::new();
    let mut helpers = BTreeSet::new();
    for entry in fs::read_dir(&tests).unwrap() {
        let path = entry.unwrap().path();
        let source = if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            Some((path.file_name().unwrap().to_str().unwrap().to_owned(), path.clone()))
        } else if path.is_dir() && path.join("main.rs").is_file() {
            Some((format!("{}/main.rs", path.file_name().unwrap().to_str().unwrap()), path.join("main.rs")))
        } else { None };
        if let Some((name, path)) = source {
            if native_suite_helper_only(&path) { helpers.insert(name); }
            else { expected.insert(name); }
        }
    }
    assert!(!expected.is_empty(), "no integration-test sources found");
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let mut registered = BTreeSet::new();
    let mut declared_suites = BTreeSet::new();
    let mut add_target = |name: Option<String>, path: Option<String>| {
        let path = path.unwrap_or_else(|| {
            let name = name.expect("test target needs name or path");
            let candidates = [format!("tests/{name}.rs"), format!("tests/{name}/main.rs")];
            let existing: Vec<_> = candidates.into_iter().filter(|path| root.join(path).is_file()).collect();
            assert_eq!(existing.len(), 1, "explicit test needs one unambiguous source");
            existing.into_iter().next().unwrap()
        });
        let source = path.strip_prefix("tests/").expect("explicit test source is outside tests/");
        assert!(!Path::new(source).components().any(|part| matches!(part, std::path::Component::ParentDir)), "nonportable test path");
        if source.starts_with("suites/") { assert!(declared_suites.insert(source.to_owned()), "suite declared twice: {source}"); }
        else {
            assert!(!helpers.contains(source), "helper-only file registered as test: {source}");
            assert!(registered.insert(source.to_owned()), "test source registered twice: {source}");
        }
    };
    let (mut in_test, mut name, mut path) = (false, None, None);
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            if in_test { add_target(name.take(), path.take()); }
            in_test = line == "[[test]]";
        } else if in_test {
            if let Some(value) = native_suite_string(line, "name") { name = Some(value.to_owned()); }
            if let Some(value) = native_suite_string(line, "path") { path = Some(value.to_owned()); }
        }
    }
    if in_test { add_target(name, path); }
    let mut actual_suites = BTreeSet::new();
    for entry in fs::read_dir(tests.join("suites")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "rs") { continue; }
        let relative = path.strip_prefix(&tests).unwrap().to_str().unwrap();
        assert!(declared_suites.contains(relative), "unregistered suite: {relative}");
        actual_suites.insert(relative.to_owned());
        for line in fs::read_to_string(&path).unwrap().lines() {
            if let Some(source) = line.strip_prefix("#[path = \"../").and_then(|s| s.strip_suffix("\"]")) {
                if source.starts_with("../") { continue; }
                assert!(!helpers.contains(source), "helper-only file in suite: {source}");
                assert!(registered.insert(source.to_owned()), "test source registered twice: {source}");
            }
        }
    }
    assert_eq!(declared_suites, actual_suites, "declared suite missing; regenerate native suites");
    assert_eq!(expected, registered, "test sources changed; run python3 scripts/generate-native-test-suites.py");
}
