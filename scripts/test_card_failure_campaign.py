#!/usr/bin/env python3
"""Standard-library-only tests; no Rust compilation or network required."""
import copy
import gzip
import lzma
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

import card_failure_campaign as campaign


def row(name, status="strict_compiled", **overrides):
    value = {key: None for key in campaign.FIELDS}
    value.update(card_name=name, parse_status=status, parse_error=None,
                 oracle_text="Draw a card.", raw_oracle_text="Draw a card.",
                 normalized_oracle_text="Draw a card.", compiled_text="Draw a card.",
                 similarity_score=1.0, oracle_coverage=1.0, compiled_coverage=1.0,
                 line_delta=0, semantic_mismatch=0, has_unimplemented=0,
                 parse_lossy=0, parse_loss_count=0, parse_loss_reasons="",
                 content_hash="content", compiled_definition_sha256="definition")
    value.update(overrides)
    return value


def snapshot(*rows):
    return {"schema_version": 1, "audit_mode": "authoritative_full_corpus",
            "full_coverage": True, "dataset_sha256": "dataset",
            "canonical_names_sha256": campaign.json_digest(sorted(r["card_name"] for r in rows)),
            "source": {"commit": "test-commit"}, "cards": list(rows),
            "summary": campaign.summarize(list(rows))}


class CorpusTests(unittest.TestCase):
    def test_membership_matches_canonical_loader_rules(self):
        cards = [
            {"name": " A / B ", "card_faces": [{"name": "A"}, {"name": "B"}]},
            {"name": "A // B"},
            {"name": "Digital", "digital": True},
            {"name": "Excluded", "legalities": {"commander": "banned"}},
            {"name": "Legal", "legalities": {"legacy": "legal"}},
            {"card_faces": [{"name": "Face fallback"}]},
            {"name": "No legalities"},
            {"name": "Empty legalities", "legalities": {}},
            {"name": ""},
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "cards.json"
            path.write_text(json.dumps(cards))
            result = campaign.load_corpus(path)
        self.assertEqual([c["card_name"] for c in result["cards"]],
                         ["A // B", "Empty legalities", "Face fallback", "Legal", "No legalities"])
        self.assertEqual(result["source_entry_count"], 9)
        self.assertEqual(result["source_face_count"], 10)
        self.assertEqual(result["canonical_card_count"], 5)
        self.assertEqual(result["exclusions"], {"duplicate_name": 1, "digital": 1,
                                             "outside_supported_formats": 1, "missing_name": 1})

    def test_hash_mismatch_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "cards.json"
            path.write_text('[{"name": "Card"}]')
            path.with_name("cards.json.scryfall-bulk-data.json").write_text('{"filtered_cards_sha256": "wrong"}')
            with self.assertRaisesRegex(ValueError, "sidecar"):
                campaign.load_corpus(path)

    def test_empty_corpus_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "cards.json"
            path.write_text("[]")
            with self.assertRaisesRegex(ValueError, "no canonical"):
                campaign.load_corpus(path)

    def test_restore_exact_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            archive, output = Path(directory) / "cards.gz", Path(directory) / "cards.json"
            raw = b'[{"name":"Card"}]\n'
            archive.write_bytes(gzip.compress(raw, mtime=0))
            import hashlib
            campaign.restore_corpus(archive, output, hashlib.sha256(raw).hexdigest())
            self.assertEqual(output.read_bytes(), raw)
            with self.assertRaisesRegex(ValueError, "overwrite"):
                campaign.restore_corpus(archive, output, hashlib.sha256(raw).hexdigest())

    def test_restore_exact_xz_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            archive, output = Path(directory) / "cards.xz", Path(directory) / "cards.json"
            raw = b'[{"name":"Card"}]\n'
            archive.write_bytes(lzma.compress(raw))
            import hashlib
            campaign.restore_corpus(archive, output, hashlib.sha256(raw).hexdigest())
            self.assertEqual(output.read_bytes(), raw)

    def test_restore_hash_mismatch_leaves_no_partial_output(self):
        with tempfile.TemporaryDirectory() as directory:
            archive, output = Path(directory) / "cards.gz", Path(directory) / "cards.json"
            archive.write_bytes(gzip.compress(b'[]', mtime=0))
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                campaign.restore_corpus(archive, output, "wrong")
            self.assertFalse(output.exists())
            self.assertFalse(output.with_name(output.name + ".partial").exists())


class ClassificationTests(unittest.TestCase):
    def test_unsupported_parser_line_is_not_runtime_gap(self):
        value = row("Card", "parse_failed", parse_error='UnsupportedLine("some clause")')
        self.assertEqual(campaign.classify(value), "parser_failure")

    def test_runtime_unsupported_marker(self):
        value = row("Card", "parse_failed", parse_error="Card compiled but contains unsupported mechanics: marker")
        self.assertEqual(campaign.classify(value), "unsupported_mechanic")

    def test_semantic_marker_is_separate_failure(self):
        value = row("Card", "parse_failed", parse_error="compiled text dropped required semantic marker: tail")
        self.assertEqual(campaign.classify(value), "semantic_output_failure")

    def test_lossy_strict_and_allow_fallback_are_not_fixed(self):
        for value in [row("Lossy", parse_lossy=1), row("Fallback", "compiled_with_allow_unsupported"),
                      row("Marker", has_unimplemented=1), row("Bad", parse_error="unexpected")]:
            self.assertFalse(campaign.is_supported(value))

    def test_panic_is_separate_failure(self):
        self.assertEqual(campaign.classify(row("Card", "parse_failed", parse_error="panic: crash")), "compiler_panic")

    def test_debug_wrappers_preserve_different_root_reasons(self):
        first = row("A", "parse_failed", parse_error='ParseError("unsupported target clause \'first\'")')
        second = row("B", "parse_failed", parse_error='ParseError("unsupported duration clause \'first\'")')
        third = row("C", "parse_failed", parse_error='ParseError("unsupported target clause \'different\'")')
        self.assertNotEqual(campaign.failure_signature(first), campaign.failure_signature(second))
        self.assertEqual(campaign.failure_signature(first), campaign.failure_signature(third))

    def test_both_wrapped_route_reasons_preserved(self):
        value = row("Card", "parse_failed", parse_error='ParseError("first reason"); oracle-only fallback also failed: ParseError("second reason")')
        signature = campaign.failure_signature(value)
        self.assertIn("first reason", signature)
        self.assertIn("second reason", signature)

    def test_groups_do_not_claim_parser_routes(self):
        rows = [row("A", "parse_failed", parse_error="unsupported clause 'first'"),
                row("B", "parse_failed", parse_error="unsupported clause 'second'")]
        summary = campaign.summarize(rows)
        self.assertEqual(summary["failing_card_count"], 2)
        self.assertEqual(summary["diagnostic_group_count"], 1)
        self.assertIsNone(summary["parser_route_count"])

    def test_combined_route_failures_count_as_one_card(self):
        value = row("Card", "parse_failed", parse_error="bad metadata; oracle-only fallback also failed: bad oracle", parse_lossy=1)
        self.assertEqual(campaign.classify(value), "parser_failure")
        summary = campaign.summarize([value])
        self.assertEqual(summary["failing_card_count"], 1)
        self.assertEqual(summary["diagnostic_route_record_count"], 2)
        self.assertEqual(summary["diagnostic_route_counts"], {"oracle_only:failed": 1, "parse_input:failed": 1})

    def test_oracle_fallback_success_preserves_primary_error(self):
        value = row("Card", parse_lossy=1, parse_loss_reasons="oracle_only_fallback: parse input failed before oracle text fallback: bad type")
        routes = campaign.route_diagnostics(value)
        self.assertEqual(routes[0]["diagnostic"], "bad type")
        self.assertEqual(routes[1]["outcome"], "compiled")
        self.assertFalse(campaign.is_supported(value))


class ComparisonTests(unittest.TestCase):
    def test_resolution_and_supported_regression_both_reported(self):
        baseline = snapshot(row("Bad", "parse_failed", parse_error="bad"), row("Good"))
        current = snapshot(row("Bad"), row("Good", "parse_failed", parse_error="new error"))
        result = campaign.compare_snapshots(baseline, current)
        self.assertEqual(result["resolved_baseline_cards"], ["Bad"])
        self.assertEqual(result["regression_card_count"], 1)
        self.assertFalse(result["compile_campaign_complete"])
        self.assertEqual(result["regressions"][0]["card_name"], "Good")

    def test_lossy_success_stays_unresolved(self):
        result = campaign.compare_snapshots(snapshot(row("Card", "parse_failed")), snapshot(row("Card", parse_lossy=1)))
        self.assertEqual(result["remaining_baseline_cards"], ["Card"])
        self.assertFalse(result["compile_campaign_complete"])

    def test_success_requires_no_failures_and_no_regressions(self):
        result = campaign.compare_snapshots(snapshot(row("Card", "parse_failed")), snapshot(row("Card")))
        self.assertTrue(result["compile_campaign_complete"])

    def test_semantic_score_regression_detected(self):
        result = campaign.compare_snapshots(snapshot(row("Card")), snapshot(row("Card", similarity_score=0.9)))
        self.assertEqual(result["regressions"][0]["reasons"], ["similarity_score_decreased"])

    def test_new_semantic_mismatch_detected(self):
        result = campaign.compare_snapshots(snapshot(row("Card")), snapshot(row("Card", semantic_mismatch=1)))
        self.assertEqual(result["regressions"][0]["reasons"], ["new_semantic_mismatch"])

    def test_changed_dataset_rejected(self):
        before = snapshot(row("Card"))
        after = copy.deepcopy(before)
        after["dataset_sha256"] = "new"
        with self.assertRaisesRegex(ValueError, "corpus differ"):
            campaign.compare_snapshots(before, after)

    def test_missing_card_rejected(self):
        with self.assertRaisesRegex(ValueError, "corpus differ"):
            campaign.compare_snapshots(snapshot(row("A"), row("B")), snapshot(row("A")))

    def test_incomplete_snapshot_rejected(self):
        before = snapshot(row("Card"))
        before["full_coverage"] = False
        with self.assertRaisesRegex(ValueError, "incomplete"):
            campaign.compare_snapshots(before, before)

    def test_duplicate_cards_rejected(self):
        before = snapshot(row("Card"), row("Card"))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            campaign.compare_snapshots(before, before)

    def test_forged_supported_boolean_does_not_bypass_status(self):
        value = row("Card", "parse_failed", supported=True)
        result = campaign.compare_snapshots(snapshot(value), snapshot(value))
        self.assertEqual(result["remaining_baseline_card_count"], 1)


class PartitionTests(unittest.TestCase):
    def test_shards_are_disjoint_and_exhaustive(self):
        cards = [{"card_name": f"Card {index}"} for index in range(11)]
        shards = campaign.partition_corpus({"cards": cards}, 4)
        self.assertEqual([len(shard["cards"]) for shard in shards], [3, 3, 3, 2])
        names = [card["card_name"] for shard in shards for card in shard["cards"]]
        self.assertEqual(sorted(names), sorted(card["card_name"] for card in cards))
        self.assertEqual(len(names), len(set(names)))

    def test_shards_deterministic(self):
        corpus = {"cards": [{"card_name": str(index)} for index in range(10)]}
        self.assertEqual(campaign.partition_corpus(corpus, 3), campaign.partition_corpus(corpus, 3))

    def test_no_empty_shards(self):
        shards = campaign.partition_corpus({"cards": [{"card_name": "Only"}]}, 4)
        self.assertEqual(len(shards), 1)

    def test_invalid_process_count(self):
        with self.assertRaisesRegex(ValueError, "at least 1"):
            campaign.partition_corpus({"cards": [{"card_name": "Only"}]}, 0)


class DatabaseTests(unittest.TestCase):
    def database(self, path, rows):
        with sqlite3.connect(path) as conn:
            conn.execute("CREATE TABLE latest_card_compilation (" + ", ".join(campaign.FIELDS) + ", compiled_card_definition)")
            for value in rows:
                conn.execute("INSERT INTO latest_card_compilation VALUES (" + ",".join("?" for _ in range(len(campaign.FIELDS) + 1)) + ")",
                             [value[key] for key in campaign.FIELDS] + ["test definition"])

    def test_missing_db_not_silently_created(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "missing.db"
            with self.assertRaisesRegex(ValueError, "missing status"):
                campaign.read_database(path, {"cards": []})
            self.assertFalse(path.exists())

    def test_full_database_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "status.db"
            self.database(path, [row("A"), row("B")])
            rows = campaign.read_database(path, {"cards": [{"card_name": "A"}, {"card_name": "B"}]})
            self.assertEqual(len(rows), 2)
            self.assertTrue(all(value["supported"] for value in rows))

    def test_partial_database_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "status.db"
            self.database(path, [row("A")])
            with self.assertRaisesRegex(ValueError, "missing=.*B"):
                campaign.read_database(path, {"cards": [{"card_name": "A"}, {"card_name": "B"}]})

    def test_unknown_status_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "status.db"
            self.database(path, [row("A", "ignored")])
            with self.assertRaisesRegex(ValueError, "unknown parse"):
                campaign.read_database(path, {"cards": [{"card_name": "A"}]})


class AuditOrchestrationTests(unittest.TestCase):
    database = DatabaseTests.database

    def test_parallel_audit_exports_exact_full_corpus(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cards, binary, provenance, output = [root / name for name in ("cards.json", "compiler", "build.json", "audit")]
            cards.write_text(json.dumps([{"name": "A"}, {"name": "B"}, {"name": "C"}]))
            binary.write_bytes(b"fixture binary; never executed")
            state = {"commit": "baseline", "tree": "tree", "status": ""}
            provenance.write_text(json.dumps({"source": state, "binary_sha256": campaign.sha256_file(binary), "build_command": ["cargo", "build"]}))
            args = SimpleNamespace(repo=root, cards=cards, out_dir=output, processes=2,
                                   sync_bin=binary, build_manifest=provenance,
                                   expected_commit="baseline", release=False)

            def fake_run(command, repo, env, shard_output):
                self.assertEqual(env["RAYON_NUM_THREADS"], "1")
                self.assertFalse(any(key.startswith("IRONSMITH_") for key in env))
                names = Path(command[command.index("--names-file") + 1]).read_text().splitlines()
                path = Path(command[command.index("--db-path") + 1])
                self.database(path, [row(name) for name in names])
                return 0

            with patch.object(campaign, "source_state", return_value=state), \
                 patch.object(campaign, "git_output", return_value="tree"), \
                 patch.object(campaign.subprocess, "check_output", return_value="fixture toolchain\n"), \
                 patch.object(campaign, "run_logged_command", side_effect=fake_run):
                self.assertEqual(campaign.run_audit(args), 0)
            result = json.loads((output / "snapshot.json").read_text())
            self.assertEqual(sorted(campaign.validate_snapshot(result)), ["A", "B", "C"])
            manifest = json.loads((output / "run.json").read_text())
            self.assertTrue(manifest["completed"])
            self.assertEqual([shard["card_count"] for shard in manifest["shards"]], [2, 1])
            self.assertEqual(manifest["snapshot_sha256"], campaign.sha256_file(output / "snapshot.json"))


if __name__ == "__main__":
    unittest.main()
