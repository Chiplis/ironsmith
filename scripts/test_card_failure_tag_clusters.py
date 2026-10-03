#!/usr/bin/env python3
"""Offline tests for diagnostic-first tag enrichment; no compiler invocation."""
import copy
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import card_failure_campaign as campaign
import card_failure_tag_clusters as clusters


def tag(slug, parents=(), children=(), ids=()):
    return {"object": "tag", "type": "oracle", "id": slug + "-id", "slug": slug,
            "parent_ids": [parent + "-id" for parent in parents],
            "child_ids": [child + "-id" for child in children],
            "taggings": [{"oracle_id": oracle_id, "weight": "median"} for oracle_id in ids]}


def records():
    return [tag("draw", children=["pure-draw"]),
            tag("pure-draw", parents=["draw"], children=["draw-two"], ids=["one"]),
            tag("draw-two", parents=["pure-draw"], ids=["one"]),
            tag("token", ids=["one", "two"]), tag("cycle", ids=["one"])]


def categories():
    return {"draw_family": [{"slug": "draw"}, {"slug": "pure-draw"}],
            "token_family": [{"slug": "token"}]}


def cards():
    return [
        {"name": "Ordinary", "oracle_id": "one", "keywords": ["Flying", "Flying"]},
        {"name": "Reversible / Back", "layout": "reversible_card", "card_faces": [
            {"name": "Reversible", "oracle_id": "one", "keywords": ["Flying"]},
            {"name": "Back", "oracle_id": "one", "keywords": ["Haste"]}]},
        {"name": "Other", "oracle_id": "two"},
        {"name": "Unknown", "oracle_id": "unknown"},
    ]


def row(name, failure=True, **overrides):
    value = {"card_name": name, "parse_status": "parse_failed" if failure else "strict_compiled",
             "parse_error": 'ParseError("unsupported target clause \'some text\'")' if failure else None,
             "parse_loss_reasons": "", "parse_lossy": 0, "has_unimplemented": 0,
             "semantic_mismatch": 0, "oracle_text": "Draw two cards.", "raw_oracle_text": "Draw two cards."}
    value.update(overrides)
    return value


def snapshot(corpus, rows=None):
    if rows is None:
        rows = [row(name) for name in sorted(corpus["cards"])]
    return {"schema_version": 1, "audit_mode": "authoritative_full_corpus", "full_coverage": True,
            "source": {"commit": "a" * 40, "tree": "b" * 40, "status": ""},
            "dataset_sha256": corpus["manifest"]["dataset_sha256"],
            "canonical_names_sha256": campaign.json_digest(sorted(value["card_name"] for value in rows)),
            "cards": rows, "summary": campaign.summarize(rows)}


def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True) + "\n")


def fixture(directory, source=None, raw_tags=None):
    source = cards() if source is None else source
    raw_tags = records() if raw_tags is None else raw_tags
    raw = json.dumps(source).encode()
    archive = gzip.compress(raw, mtime=0)
    (directory / "cards.json.gz").write_bytes(archive)
    names = [campaign.normalize_name(card.get("name") or card["card_faces"][0]["name"]) for card in source]
    manifest = {"schema_version": 1, "archive": "cards.json.gz", "archive_bytes": len(archive),
                "archive_sha256": hashlib.sha256(archive).hexdigest(), "dataset_bytes": len(raw),
                "dataset_sha256": hashlib.sha256(raw).hexdigest(), "source_entry_count": len(source),
                "source_face_count": sum(len(card.get("card_faces") or []) or 1 for card in source),
                "canonical_card_count": len(names), "canonical_names_sha256": campaign.json_digest(sorted(names)),
                "unique_nonnull_oracle_id_count": len({card["oracle_id"] for card in source if card.get("oracle_id")}),
                "entries_without_oracle_id_count": sum(not card.get("oracle_id") for card in source)}
    write_json(directory / "manifest.json", manifest)
    tag_raw = gzip.compress(b"\n".join(json.dumps(record).encode() for record in raw_tags) + b"\n", mtime=0)
    (directory / "tags.jsonl.gz").write_bytes(tag_raw)
    write_json(directory / clusters.CATEGORY_FILE, categories())
    category_raw = (directory / clusters.CATEGORY_FILE).read_bytes()
    metadata = {"schema_version": 1, "raw_file": "tags.jsonl.gz", "raw_sha256": hashlib.sha256(tag_raw).hexdigest(),
                "bulk_metadata": {"type": "oracle_tags", "compressed_size": len(tag_raw), "uri": "test:metadata",
                                  "jsonl_download_uri": "test:snapshot", "updated_at": "2026-10-03T09:00:35Z"},
                "counts": {"tags": len(raw_tags), "taggings": sum(len(record["taggings"]) for record in raw_tags),
                           "annotations": sum("annotation" in tagging for record in raw_tags for tagging in record["taggings"])},
                "artifacts": [{"filename": clusters.CATEGORY_FILE, "bytes": len(category_raw),
                               "sha256": hashlib.sha256(category_raw).hexdigest()}]}
    write_json(directory / "oracle-tags.metadata.json", metadata)
    return metadata


class TagGraphTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name)
        self.metadata = fixture(self.path)
        self.index = clusters.TagIndex.load(self.path)

    def test_parent_hierarchy_is_separate_from_direct_tags(self):
        value = self.index.enrich_card(cards()[0])
        self.assertEqual(value["oracle_tags_direct"], ["cycle", "draw-two", "pure-draw", "token"])
        self.assertEqual(value["oracle_tags_ancestor_only"], ["draw"])
        self.assertEqual(value["functional_categories_direct"], ["draw_family", "token_family"])
        self.assertEqual(value["functional_categories_ancestor_only"], [])
        self.assertEqual(value["scryfall_keywords"], ["Flying"])
        self.assertTrue(set(value["oracle_tags_direct"]).isdisjoint(value["oracle_tags_ancestor_only"]))

    def test_parent_only_functional_label_is_derived(self):
        index = clusters.TagIndex(records(), {"draw_family": [{"slug": "draw"}]}, self.metadata)
        value = index.enrich_card(cards()[0])
        self.assertEqual(value["functional_categories_direct"], [])
        self.assertEqual(value["functional_categories_ancestor_only"], ["draw_family"])

    def test_unknown_identity_is_explicitly_untagged_not_excluded(self):
        value = self.index.enrich_card({"oracle_id": "unknown"})
        self.assertEqual(value["oracle_ids"], ["unknown"])
        self.assertEqual(value["oracle_ids_without_direct_tags"], ["unknown"])
        self.assertEqual(value["oracle_tags_direct"], [])
        self.assertEqual(value["oracle_tags_ancestor_only"], [])
        self.assertEqual(self.index.enrich_card({})["oracle_ids"], [])

    def test_reversible_identity_is_shared_without_duplicate_face_tags(self):
        ordinary, alias = (self.index.enrich_card(card) for card in cards()[:2])
        self.assertEqual(alias["oracle_ids"], ["one"])
        self.assertEqual(alias["oracle_tags_direct"], ordinary["oracle_tags_direct"])
        self.assertEqual(alias["scryfall_keywords"], ["Flying", "Haste"])

    def test_top_level_identity_takes_precedence(self):
        value = {"oracle_id": "two", "card_faces": [{"oracle_id": "one"}]}
        self.assertEqual(clusters.oracle_ids_for_card(value), ["two"])
        self.assertEqual(self.index.enrich_card(value)["oracle_tags_direct"], ["token"])

    def test_distinct_face_ids_are_preserved_without_multiplying_entry(self):
        self.assertEqual(clusters.oracle_ids_for_card({"card_faces": [
            {"oracle_id": "two"}, {"oracle_id": "one"}, {"oracle_id": "two"}]}), ["one", "two"])

    def test_graph_rejects_unknown_edge(self):
        values = records()
        values[0]["parent_ids"] = ["missing"]
        with self.assertRaisesRegex(ValueError, "unknown hierarchy"):
            clusters.TagIndex(values, categories(), self.metadata)

    def test_graph_rejects_nonreciprocal_edge(self):
        values = records()
        values[0]["child_ids"] = []
        with self.assertRaisesRegex(ValueError, "non-reciprocal"):
            clusters.TagIndex(values, categories(), self.metadata)

    def test_graph_rejects_cycle(self):
        values = records()
        values[0]["parent_ids"] = ["draw-two-id"]
        values[2]["child_ids"] = ["draw-id"]
        with self.assertRaisesRegex(ValueError, "cycle"):
            clusters.TagIndex(values, categories(), self.metadata)

    def test_unknown_category_refused(self):
        with self.assertRaisesRegex(ValueError, "unknown functional"):
            clusters.TagIndex(records(), {"family": [{"slug": "absent"}]}, self.metadata)

    def test_duplicate_tag_and_duplicate_membership_refused(self):
        for mutate, message in [
            (lambda values: values.append(copy.deepcopy(values[0])), "duplicate"),
            (lambda values: values[1]["taggings"].append({"oracle_id": "one"}), "duplicate"),
        ]:
            values = records()
            mutate(values)
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                clusters.TagIndex(values, categories(), self.metadata)

    def test_tag_count_refused(self):
        self.metadata["counts"]["tags"] += 1
        with self.assertRaisesRegex(ValueError, "count mismatch"):
            clusters.TagIndex(records(), categories(), self.metadata)


class ReportTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name)
        fixture(self.path)
        self.corpus = clusters.load_pinned_cards(self.path)
        self.index = clusters.TagIndex.load(self.path)
        self.snapshot = snapshot(self.corpus)

    def report(self):
        return clusters.build_report(self.snapshot, self.corpus, self.index)

    def test_entries_and_unique_ids_are_distinct_units(self):
        report = self.report()
        self.assertEqual(report["summary"]["failed_compile_entry_count"], 4)
        self.assertEqual(report["summary"]["failed_unique_oracle_id_count"], 3)
        self.assertEqual(report["summary"]["corpus_reversible_alias_entry_count"], 1)
        self.assertEqual(report["summary"]["oracle_ids_shared_by_multiple_compile_entries_count"], 1)
        self.assertEqual(report["summary"]["failed_entries_without_direct_tags_count"], 1)
        entries = report["failed_compile_entries"]
        self.assertEqual(len({entry["source_row_index"] for entry in entries}), 4)
        self.assertEqual(len({entry["source_card_sha256"] for entry in entries}), 4)
        names = [name for group in report["diagnostic_groups"] for name in group["cards"]]
        self.assertEqual(sorted(names), sorted(self.corpus["cards"]))
        self.assertEqual(len(names), len(set(names)))

    def test_diagnostics_are_primary_even_when_tags_identical(self):
        self.snapshot["cards"][2]["parse_error"] = 'ParseError("unsupported duration clause \'example\'")'
        self.snapshot["summary"] = campaign.summarize(self.snapshot["cards"])
        report = self.report()
        self.assertEqual(len(report["diagnostic_groups"]), 2)
        self.assertEqual(sum(group["failed_entry_count"] for group in report["diagnostic_groups"]), 4)
        signatures = [group["diagnostic_signature"] for group in report["diagnostic_groups"]]
        self.assertTrue(any("duration" in value for value in signatures))
        self.assertTrue(any("target" in value for value in signatures))

    def test_multiple_tags_in_category_count_entry_once(self):
        group = self.report()["diagnostic_groups"][0]
        labels = {value["label"]: value for value in group["functional_categories"]}
        self.assertEqual(labels["draw_family"]["failed_entry_count"], 2)
        self.assertEqual(labels["draw_family"]["failed_unique_oracle_id_count"], 1)
        self.assertEqual(labels["draw_family"]["direct_entry_count"], 2)
        self.assertEqual(labels["draw_family"]["ancestor_only_entry_count"], 0)
        self.assertNotIn("cycle", labels)
        keywords = {value["label"]: value for value in group["scryfall_keywords"]}
        self.assertEqual(keywords["Flying"]["failed_entry_count"], 2)

    def test_tags_cannot_make_supported_or_semantic_only_entry_fail(self):
        rows = [row(name, failure=name == "Unknown", semantic_mismatch=int(name == "Ordinary"))
                for name in sorted(self.corpus["cards"])]
        self.snapshot = snapshot(self.corpus, rows)
        report = self.report()
        self.assertEqual(report["summary"]["failed_compile_entry_count"], 1)
        self.assertEqual(report["summary"]["supported_compile_entry_count"], 3)
        self.assertEqual(report["summary"]["semantic_mismatch_entry_count"], 1)
        self.assertEqual(report["diagnostic_groups"][0]["functional_categories"], [])

    def test_recomputes_classification_and_two_routes_count_one_entry(self):
        self.snapshot["cards"][0].update(parse_error="first; oracle-only fallback also failed: second",
                                          supported=True, category="strict_compiled", route_diagnostics=[])
        self.snapshot["summary"] = campaign.summarize(self.snapshot["cards"])
        report = self.report()
        self.assertEqual(report["summary"]["failed_compile_entry_count"], 4)
        self.assertEqual(report["summary"]["diagnostic_route_record_count"], 5)
        self.assertEqual(report["failed_compile_entries"][0]["category"], "parser_failure")
        self.assertEqual(len(report["failed_compile_entries"][0]["route_diagnostics"]), 2)

    def test_incomplete_and_non_authoritative_snapshots_refused(self):
        for key, value in [("full_coverage", False), ("audit_mode", "strict_only"), ("schema_version", 2)]:
            original = copy.deepcopy(self.snapshot)
            self.snapshot[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.report()
            self.snapshot = original

    def test_changed_dataset_refused(self):
        self.snapshot["dataset_sha256"] = "changed"
        with self.assertRaisesRegex(ValueError, "dataset/membership"):
            self.report()

    def test_partial_snapshot_refused_even_if_internally_consistent(self):
        self.snapshot = snapshot(self.corpus, self.snapshot["cards"][:-1])
        with self.assertRaisesRegex(ValueError, "dataset/membership"):
            self.report()

    def test_duplicate_snapshot_entry_refused(self):
        self.snapshot = snapshot(self.corpus, self.snapshot["cards"] + [self.snapshot["cards"][0]])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.report()

    def test_forged_summary_refused(self):
        self.snapshot["summary"]["failing_card_count"] = 0
        with self.assertRaisesRegex(ValueError, "summary"):
            self.report()

    def test_dirty_or_missing_source_refused(self):
        for source in [{"commit": "a"}, {"commit": "a", "tree": "b", "status": "dirty"}]:
            self.snapshot["source"] = source
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, "clean committed"):
                self.report()

    def test_deterministic_and_does_not_mutate_inputs(self):
        before = copy.deepcopy(self.snapshot)
        first = json.dumps(self.report(), sort_keys=True)
        self.snapshot["cards"].reverse()
        second = json.dumps(self.report(), sort_keys=True)
        # The provenance content digest preserves snapshot row order. Everything
        # else is independent of that order; identical input yields exact bytes.
        self.snapshot["cards"].reverse()
        self.assertEqual(first, json.dumps(self.report(), sort_keys=True))
        self.assertEqual(self.snapshot, before)
        result_a, result_b = json.loads(first), json.loads(second)
        del result_a["provenance"]["snapshot_content_sha256"]
        del result_b["provenance"]["snapshot_content_sha256"]
        self.assertEqual(result_a, result_b)

    def test_cli_is_offline_and_byte_deterministic(self):
        source = self.path / "snapshot.json"
        write_json(source, self.snapshot)
        with patch("socket.socket", side_effect=AssertionError("network forbidden")), patch(
                "subprocess.run", side_effect=AssertionError("compiler forbidden")):
            for name in ["first.json", "second.json"]:
                self.assertEqual(clusters.main(["--snapshot", str(source), "--fixtures", str(self.path),
                                                "--out", str(self.path / name)]), 0)
        self.assertEqual((self.path / "first.json").read_bytes(), (self.path / "second.json").read_bytes())
        self.assertEqual(clusters.main(["--snapshot", str(source), "--fixtures", str(self.path),
                                        "--out", str(self.path / "first.json")]), 2)


class IntegrityTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name)
        fixture(self.path)

    def test_raw_tag_integrity_refused(self):
        with (self.path / "tags.jsonl.gz").open("ab") as stream:
            stream.write(b"corrupt")
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            clusters.TagIndex.load(self.path)

    def test_selected_category_integrity_refused(self):
        write_json(self.path / clusters.CATEGORY_FILE, {})
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            clusters.TagIndex.load(self.path)

    def test_corpus_archive_integrity_refused(self):
        (self.path / "cards.json.gz").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            clusters.load_pinned_cards(self.path)

    def test_restored_corpus_integrity_refused(self):
        restored = self.path / "restored.json"
        restored.write_text("[]")
        with self.assertRaisesRegex(ValueError, "dataset SHA-256"):
            clusters.load_pinned_cards(self.path, restored)

    def test_restored_and_archived_corpus_equivalent(self):
        restored = self.path / "restored.json"
        restored.write_bytes(gzip.decompress((self.path / "cards.json.gz").read_bytes()))
        self.assertEqual(clusters.load_pinned_cards(self.path, restored), clusters.load_pinned_cards(self.path))

    def test_source_count_mismatch_refused(self):
        path = self.path / "manifest.json"
        value = json.loads(path.read_text())
        value["source_entry_count"] += 1
        write_json(path, value)
        with self.assertRaisesRegex(ValueError, "membership/counts"):
            clusters.load_pinned_cards(self.path)

    def test_excluding_or_deduplicating_source_rows_refused(self):
        for source in [[cards()[0], cards()[0]], [{**cards()[0], "digital": True}],
                       [{**cards()[0], "legalities": {"commander": "banned"}}]]:
            fixture(self.path, source=source)
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, "filtering"):
                clusters.load_pinned_cards(self.path)

    def test_no_output_on_invalid_evidence(self):
        corpus = clusters.load_pinned_cards(self.path)
        value = snapshot(corpus)
        value["full_coverage"] = False
        write_json(self.path / "snapshot.json", value)
        output = self.path / "report.json"
        self.assertEqual(clusters.main(["--snapshot", str(self.path / "snapshot.json"),
                                        "--out", str(output), "--fixtures", str(self.path)]), 2)
        self.assertFalse(output.exists())


class PinnedArtifactTests(unittest.TestCase):
    def test_shipped_snapshot_integrity_and_all_71_aliases(self):
        index = clusters.TagIndex.load(clusters.DEFAULT_FIXTURES)
        corpus = clusters.load_pinned_cards(clusters.DEFAULT_FIXTURES)
        self.assertEqual(len(index.catalog), 4560)
        self.assertEqual(len(corpus["cards"]), 32209)
        ordinary = {entry["card"]["oracle_id"]: entry["card"] for entry in corpus["cards"].values()
                    if entry["card"].get("oracle_id")}
        aliases = [entry["card"] for entry in corpus["cards"].values() if not entry["card"].get("oracle_id")]
        self.assertEqual(len(aliases), 71)
        self.assertEqual(len(ordinary), 32138)
        for card in aliases:
            self.assertEqual(card["layout"], "reversible_card")
            ids = clusters.oracle_ids_for_card(card)
            self.assertEqual(len(ids), 1)
            self.assertIn(ids[0], ordinary)
            enriched, original = index.enrich_card(card), index.enrich_card(ordinary[ids[0]])
            self.assertEqual(enriched["oracle_tags_direct"], original["oracle_tags_direct"])
            self.assertEqual(enriched["oracle_tags_ancestor_only"], original["oracle_tags_ancestor_only"])
        self.assertEqual(sum(bool(index.enrich_card(card)["oracle_tags_direct"]) for card in ordinary.values()), 31939)


if __name__ == "__main__":
    unittest.main()
