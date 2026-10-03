"""Face-audit fail-closed accounting tests; no Rust compilation required."""
import copy
import unittest

import card_failure_faces as faces


def fixture_cards():
    return [
        {"id": "ordinary", "name": "Shared", "oracle_id": "o1", "oracle_text": "Draw a card.", "layout": "normal"},
        {"id": "prepared", "name": "Scholar // Shared", "oracle_id": "o2", "layout": "prepare", "card_faces": [
            {"name": "Scholar", "oracle_text": "Flying"}, {"name": "Shared", "oracle_text": "Draw two cards."}]},
        {"id": "reversible", "name": "Twin // Twin", "layout": "reversible_card", "card_faces": [
            {"name": "Twin", "oracle_id": "o3", "oracle_text": "Vigilance"},
            {"name": "Twin", "oracle_id": "o3", "oracle_text": "Flying"}]},
    ]


def stdout(name, oracle, compiled=None):
    return f"Name: {name}\nSimilarity: 1\nSemantic mismatch: false\nOriginal oracle text:\n{oracle}\nCompiled oracle text:\n{compiled or oracle}\n"


def snapshot(name, raw):
    return {"card_name": name, "oracle_text": raw, "raw_oracle_text": raw,
            "parse_status": "strict_compiled", "parse_error": None, "normalized_oracle_text": raw,
            "compiled_text": raw, "compiled_card_definition": "definition",
            "compiled_card_definition_sha256": faces.digest(b"definition"),
            "oracle_coverage": 1, "compiled_coverage": 1, "similarity_score": 1,
            "line_delta": 0, "semantic_mismatch": False, "has_unimplemented": False,
            "parse_lossy": False, "parse_loss_reasons": "", "parse_loss_count": 0, "content_hash": "content"}


def exact_records(inventory, inventory_only=False):
    records = [{"kind": "header", "schema_version": 1, "source_sha256": "source", "face_route_count": 4, "inventory_only": inventory_only}]
    for route in inventory["routes"]:
        records.append({**route, "kind": "face",
                        "payload": {"name": route["face_name"], "parse_name": None, "raw_oracle_text": route["raw_oracle_text"]},
                        "snapshot": None if inventory_only else snapshot(route["face_name"], route["raw_oracle_text"])})
    records.append({"kind": "complete", "source_sha256": "source", "face_route_count": 4})
    return records


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.inventory = faces.make_inventory(fixture_cards(), "source")

    def test_source_routes_are_not_unique_face_names(self):
        summary = self.inventory["summary"]
        self.assertEqual(summary["face_route_count"], 4)
        self.assertEqual(summary["distinct_face_lookup_name_count"], 3)
        self.assertEqual(summary["exact_face_route_count"], 2)
        self.assertEqual(summary["omitted_face_route_count"], 2)
        self.assertEqual(summary["multiface_unique_oracle_card_count"], 2)
        self.assertEqual(summary["product_baker_linked_face_route_count"], 2)

    def test_canonical_collision_is_not_face_coverage(self):
        shared = next(q for q in self.inventory["queries"] if q["name"] == "Shared")
        self.assertIsNone(shared["selected_payloads"][0]["route_id"])
        self.assertEqual(self.inventory["routes"][1]["lookup_disposition"], "name_shadowed")

    def test_reversible_same_name_second_face_remains_omitted(self):
        self.assertEqual(self.inventory["routes"][2]["lookup_disposition"], "exact_face")
        self.assertEqual(self.inventory["routes"][3]["lookup_disposition"], "name_shadowed")

    def test_parenthetical_postprocess_matches_punctuation(self):
        self.assertEqual(faces.postprocess("Flying (reminder), vigilance\n(a (nested) note)\n Draw  a card ."), "Flying, vigilance\nDraw a card.")

    def test_name_normalization_matches_single_replacement(self):
        self.assertEqual(faces.normalize(" A / B / C "), "A // B / C")
        self.assertEqual(faces.normalize("A // B / C"), "A // B / C")


class DiagnosticTests(unittest.TestCase):
    def setUp(self):
        self.inventory = faces.make_inventory(fixture_cards(), "source")
        self.output = stdout("Scholar", "Flying") + "\n" + stdout("Shared", "Draw a card.")
        self.errors = "Name: Twin\nError: parse failed for Twin: failure\n"

    def test_failed_card_and_failed_route_counts_separate(self):
        report = faces.analyze(self.inventory, self.output, self.errors)
        self.assertEqual(report["summary"]["failed_face_route_count"], 1)
        self.assertEqual(report["summary"]["unique_failed_oracle_card_count"], 1)
        self.assertEqual(report["summary"]["face_route_outcome_counts"]["omitted"], 2)
        self.assertIsNone(report["summary"]["strict_supported_face_route_count"])
        self.assertFalse(report["all_face_routes_supported"])

    def test_omitted_route_does_not_inherit_failed_twin_result(self):
        report = faces.analyze(self.inventory, self.output, self.errors)
        self.assertEqual(report["routes"][3]["outcome"]["status"], "omitted")

    def test_missing_duplicate_and_unexpected_output_rejected(self):
        for output in ("", self.output + stdout("Shared", "Draw a card."), self.output + stdout("Unexpected", "Flying")):
            with self.subTest(output=output):
                with self.assertRaises(ValueError):
                    faces.analyze(self.inventory, output, self.errors)

    def test_wrong_context_oracle_text_rejected(self):
        with self.assertRaisesRegex(ValueError, "Oracle text differs"):
            faces.analyze(self.inventory, self.output.replace("Draw a card.", "Draw two cards."), self.errors)

    def test_unattributed_error_rejected(self):
        with self.assertRaisesRegex(ValueError, "unattributed"):
            faces.analyze(self.inventory, self.output, self.errors.replace("for Twin", "for Other"))


class ExactAuditTests(unittest.TestCase):
    def setUp(self):
        self.inventory = faces.make_inventory(fixture_cards(), "source")
        self.records = exact_records(self.inventory)

    def test_exact_inventory_is_not_compile_success(self):
        result = faces.analyze_exact(self.inventory, exact_records(self.inventory, True))
        self.assertFalse(result["all_face_routes_supported"])
        self.assertIsNone(result["summary"]["strict_supported_face_route_count"])

    def test_exact_coverage_keeps_all_duplicate_contexts(self):
        result = faces.analyze_exact(self.inventory, self.records)
        self.assertEqual(result["summary"]["face_route_count"], 4)
        self.assertTrue(result["all_face_routes_supported"])
        self.assertFalse(result["artifact_baker_verified"])

    def test_missing_duplicate_footer_and_trailing_data_rejected(self):
        for records in (self.records[:-1], self.records[:2] + self.records[3:], self.records[:-1] + [self.records[1], self.records[-1]], self.records + [self.records[1]]):
            with self.assertRaises(ValueError):
                faces.analyze_exact(self.inventory, records)

    def test_missing_snapshot_fields_rejected(self):
        del self.records[1]["snapshot"]["parse_lossy"]
        with self.assertRaisesRegex(ValueError, "missing strict"):
            faces.analyze_exact(self.inventory, self.records)

    def test_face_and_definition_identity_checked(self):
        for change in (lambda r: r[1].update(face_index=9), lambda r: r[1]["snapshot"].update(compiled_card_definition="changed")):
            records = copy.deepcopy(self.records)
            change(records)
            with self.assertRaises(ValueError):
                faces.analyze_exact(self.inventory, records)

    def test_lossy_and_permissive_are_not_supported(self):
        for change in ({"parse_lossy": True}, {"parse_loss_count": 1}, {"parse_loss_reasons": "fallback"}, {"parse_status": "compiled_with_allow_unsupported"}, {"has_unimplemented": True}, {"compiled_text": None}):
            records = copy.deepcopy(self.records)
            records[1]["snapshot"].update(change)
            result = faces.analyze_exact(self.inventory, records)
            self.assertFalse(result["all_face_routes_supported"])
            self.assertEqual(result["summary"]["failed_face_route_count"], 1)

    def test_regression_comparison(self):
        baseline = copy.deepcopy(faces.analyze_exact(self.inventory, self.records))
        self.records[1]["snapshot"].update(similarity_score=0.5, semantic_mismatch=True)
        current = faces.analyze_exact(self.inventory, self.records)
        result = faces.compare_exact(baseline, current)
        self.assertFalse(result["face_compile_gate_complete"])
        self.assertEqual(result["regressions"][0]["reasons"], ["new_semantic_mismatch", "similarity_score_decreased"])

    def test_context_change_cannot_silently_pass(self):
        baseline = faces.analyze_exact(self.inventory, self.records)
        current = copy.deepcopy(baseline)
        current["routes"][0]["payload"]["parse_input"] = "changed"
        with self.assertRaisesRegex(ValueError, "context changed"):
            faces.compare_exact(baseline, current)

    def test_diagnostic_evidence_cannot_enter_exact_gate(self):
        with self.assertRaisesRegex(ValueError, "only exact"):
            faces.compare_exact({"evidence_kind": "frozen_cli_diagnostic_only"}, {})


if __name__ == "__main__":
    unittest.main()
