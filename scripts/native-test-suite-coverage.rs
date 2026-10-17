//! Catch new, deleted, or multiply registered integration-test sources.
//! Shared by suite_00 in each package that disables Cargo's test autodiscovery.

#[test]
fn every_integration_test_source_is_registered_once() {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tests = root.join("tests");
    let mut expected = BTreeSet::new();
    for entry in fs::read_dir(&tests).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            expected.insert(path.file_name().unwrap().to_str().unwrap().to_owned());
        } else if path.is_dir() && path.join("main.rs").is_file() {
            expected.insert(format!(
                "{}/main.rs",
                path.file_name().unwrap().to_str().unwrap()
            ));
        }
    }
    assert!(!expected.is_empty(), "no integration-test sources found");
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let mut registered = BTreeSet::new();
    for entry in fs::read_dir(tests.join("suites")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap().to_str().unwrap();
        assert!(
            manifest.contains(&format!("path = {relative:?}")),
            "unregistered suite: {relative}"
        );
        for line in fs::read_to_string(&path).unwrap().lines() {
            if let Some(source) = line
                .strip_prefix("#[path = \"../")
                .and_then(|s| s.strip_suffix("\"]"))
            {
                if source.starts_with("../") {
                    continue; // This shared coverage check, outside the test directory.
                }
                assert!(
                    registered.insert(source.to_owned()),
                    "test source registered twice: {source}"
                );
            }
        }
    }
    assert_eq!(
        expected, registered,
        "test sources changed; run python3 scripts/generate-native-test-suites.py"
    );
}
