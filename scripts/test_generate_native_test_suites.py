"""Regression checks for suite membership, stable assignment, and coverage guards."""

import importlib.util
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "suites", Path(__file__).with_name("generate-native-test-suites.py")
)
SUITES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUITES)


class NativeTestSuites(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in SUITES.PACKAGES:
            package = self.root / "crates" / name
            (package / "tests").mkdir(parents=True)
            (package / "Cargo.toml").write_text(
                f'[package]\nname = "{name}"\nversion = "0.1.0"\n\n'
                '[dependencies]\n# Keep existing package configuration.\n'
            )
            (package / "tests" / "alpha.rs").write_text("#[test]\nfn example() {}\n")
            (package / "tests" / "beta.rs").write_text("#[test]\nfn another() {}\n")

    def generate(self):
        files = SUITES.outputs(self.root)
        for path, text in files.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        return files

    def test_membership_and_idempotence(self):
        first = self.generate()
        self.assertEqual(first, SUITES.outputs(self.root))
        for package, count in SUITES.PACKAGES.items():
            directory = self.root / "crates" / package
            wrappers = list((directory / "tests" / "suites").glob("*.rs"))
            self.assertEqual(len(wrappers), count)
            members = [s for p in wrappers for s in re.findall(r'mod r#(\w+);', p.read_text())]
            self.assertCountEqual(members, ["alpha", "beta"])
            manifest = (directory / "Cargo.toml").read_text()
            self.assertIn("# Keep existing package configuration.", manifest)
            self.assertIn("autotests = false", manifest)
            self.assertEqual(manifest.count("[[test]]"), count)

    def test_new_file_changes_only_one_suite(self):
        first = self.generate()
        package = self.root / "crates" / "ironsmith-tools"
        (package / "tests" / "gamma.rs").write_text("#[test]\nfn new_test() {}\n")
        second = SUITES.outputs(self.root)
        changed = [p for p in first if first[p] != second[p]]
        self.assertEqual(len(changed), 1)
        self.assertEqual(changed[0].parent.name, "suites")

    def test_directory_targets_and_duplicate_names(self):
        tests = self.root / "crates" / "ironsmith-tools" / "tests"
        (tests / "nested").mkdir()
        (tests / "nested" / "main.rs").write_text("#[test]\nfn nested() {}\n")
        self.assertTrue(any('../nested/main.rs' in s for s in SUITES.outputs(self.root).values()))
        (tests / "nested.rs").write_text("")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            SUITES.outputs(self.root)

    def test_explicit_standalone_and_declared_helpers_do_not_enter_suites(self):
        package = self.root / "crates" / "ironsmith-compiler-runtime"
        manifest = package / "Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[[test]]\nname = "dedicated"\npath = "tests/alpha.rs"\n')
        (package / "tests" / "shared_support.rs").write_text(SUITES.HELPER_MARKER + "\npub fn helper() {}\n")
        files = self.generate()
        members = [member for path, text in files.items() if path.parent.name == "suites" and package in path.parents for member in re.findall(r'mod r#(\w+);', text)]
        self.assertEqual(members, ["beta"])
        self.assertIn('name = "dedicated"', manifest.read_text())
        self.assertEqual(files, SUITES.outputs(self.root))

    def test_helper_role_is_explicit_and_cannot_hide_test_declarations(self):
        tests = self.root / "crates" / "ironsmith-tools" / "tests"
        helper = tests / "ordinary_support.rs"
        helper.write_text("pub fn helper() {}\n")
        self.assertTrue(any('../ordinary_support.rs' in text for text in SUITES.outputs(self.root).values()))
        helper.write_text(SUITES.HELPER_MARKER + "\n#[test]\nfn hidden() {}\n")
        with self.assertRaisesRegex(ValueError, "contains test/bench"):
            SUITES.outputs(self.root)

    def test_duplicate_explicit_source_is_rejected_before_generating_wrappers(self):
        package = self.root / "crates" / "ironsmith-tools"
        manifest = package / "Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[[test]]\nname = "first"\npath = "tests/alpha.rs"\n\n[[test]]\nname = "second"\npath = "tests/alpha.rs"\n')
        with self.assertRaisesRegex(ValueError, "registered twice"):
            SUITES.outputs(self.root)

    def test_guard_counts_standalone_plus_suites_and_excludes_only_declared_helpers(self):
        package = self.root / "crates" / "ironsmith-tools"
        manifest = package / "Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[[test]]\nname = "dedicated"\npath = "tests/alpha.rs"\n')
        helper = package / "tests" / "shared_support.rs"
        helper.write_text(SUITES.HELPER_MARKER + "\npub fn helper() {}\n")
        self.generate()
        binary = self.root / "coverage-union-check"
        subprocess.run(["rustc", "--edition=2024", "--test", str(Path(__file__).with_name("native-test-suite-coverage.rs")), "-o", str(binary)], env=dict(os.environ, CARGO_MANIFEST_DIR=str(package)), check=True, capture_output=True)
        def run():
            return subprocess.run([str(binary)], capture_output=True, text=True)
        self.assertEqual(run().returncode, 0)
        wrapper = package / "tests" / "suites" / "suite_00.rs"
        original = wrapper.read_text()
        wrapper.write_text(original + '\n#[path = "../alpha.rs"]\nmod duplicate_direct;\n')
        self.assertIn("registered twice", run().stdout)
        wrapper.write_text(original + '\n#[path = "../shared_support.rs"]\nmod hidden_helper;\n')
        self.assertIn("helper-only file in suite", run().stdout)

    def test_guard_rejects_missing_and_duplicate_members(self):
        self.generate()
        package = self.root / "crates" / "ironsmith-tools"
        binary = self.root / "coverage-check"
        subprocess.run(
            ["rustc", "--edition=2024", "--test",
             str(Path(__file__).with_name("native-test-suite-coverage.rs")), "-o", str(binary)],
            env=dict(os.environ, CARGO_MANIFEST_DIR=str(package)), check=True,
            capture_output=True,
        )
        def run():
            return subprocess.run([str(binary)], capture_output=True, text=True)

        self.assertEqual(run().returncode, 0)
        missing = package / "tests" / "unregistered.rs"
        missing.write_text("#[test]\nfn forgotten() {}\n")
        result = run()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("test sources changed", result.stdout)
        missing.unlink()
        wrapper = package / "tests" / "suites" / "suite_00.rs"
        with wrapper.open("a") as out:
            out.write('\n#[path = "../alpha.rs"]\nmod duplicate;\n')
        result = run()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("registered twice", result.stdout)


if __name__ == "__main__":
    unittest.main()
