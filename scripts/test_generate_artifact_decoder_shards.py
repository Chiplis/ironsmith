"""Authored source-parity gates; UNRUN in the source-only repair.

These read source literals without importing or executing the generator. They
are not regeneration, Rust compilation, semantic validation, or provenance.
Run only when the validation gate is opened. Real regeneration and runtime
checks remain required; see reports/decoder-generator-source-20261008/REPORT.md.
"""

from __future__ import annotations

import ast
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
DECODER = ROOT / "crates/ironsmith-artifact-effect-decoder/src"
GRAPH_MARKER = "/// Remap typed card references"
TEST_MARKER = "#[cfg(test)]\nmod tests {"


class DecoderGeneratorSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = (ROOT / "scripts/generate_artifact_decoder_shards.py").read_text()
        cls.tree = ast.parse(cls.source)
        cls.rust = (DECODER / "lib.rs").read_text()

    def literal(self, name):
        assignments = [
            node for node in self.tree.body
            if isinstance(node, ast.Assign)
            and any(isinstance(target, ast.Name) and target.id == name for target in node.targets)
        ]
        self.assertEqual(len(assignments), 1, name)
        value = ast.literal_eval(assignments[0].value)
        self.assertIsInstance(value, str)
        return value

    def function(self, name):
        return next(node for node in self.tree.body if isinstance(node, ast.FunctionDef) and node.name == name)

    def test_graph_template_matches_retained_owner_and_all_graph_tests(self):
        graph = self.literal("CARD_GRAPH_SOURCE")
        self.assertEqual(graph, self.rust[self.rust.index(GRAPH_MARKER):])
        for required in (
            "pub fn authored_definition_graph", "authored_definition: false",
            "authored_definition: true", "retained_definition", "generated_definitions",
            "normalized_definitions", "authored_graph_tracks_only_typed_opaque_definitions_and_preserves_pair_aliases",
            "authored_graph_omits_nested_presentation_without_mutating_transport_data",
        ):
            self.assertIn(required, graph)
        self.assertNotIn('self.name == Some("RetainedCardPayload")', graph)

    def test_facade_template_preserves_whole_retained_test_block(self):
        tests = self.literal("FACADE_TEST_SOURCE")
        self.assertEqual(tests, self.rust[self.rust.index(TEST_MARKER):self.rust.index(GRAPH_MARKER)].rstrip())
        self.assertIn("fn routes_representative_effects_to_domain_families", tests)
        self.assertIn("fn source_counter_payload_decodes_and_normalizes_inside_owned_cost", tests)

    def test_facade_emits_both_templates_and_keeps_external_counter_tests(self):
        facade = self.function("write_facade")
        emitted = [
            node.value.id for node in ast.walk(facade)
            if isinstance(node, ast.FormattedValue) and isinstance(node.value, ast.Name)
        ]
        self.assertEqual(emitted.count("FACADE_TEST_SOURCE"), 1)
        self.assertEqual(emitted.count("CARD_GRAPH_SOURCE"), 1)
        literals = [node.value for node in ast.walk(facade) if isinstance(node, ast.Constant) and isinstance(node.value, str)]
        registration = "#[cfg(test)]\nmod counter_exile_permission_tests;"
        self.assertTrue(any(registration in value for value in literals))
        self.assertIn(registration, self.rust)
        self.assertTrue((DECODER / "counter_exile_permission_tests.rs").is_file())

    def test_counter_validation_helper_and_both_typed_arms_are_retained(self):
        shard = self.function("write_shard")
        literals = [node.value for node in ast.walk(shard) if isinstance(node, ast.Constant) and isinstance(node.value, str)]
        helper = next(value for value in literals if value.startswith("fn validated_counter_effect("))
        retained = (DECODER / "stack_event.rs").read_text()
        self.assertIn(helper, retained)
        self.assertIn("effect.exile_permission_target_is_supported()", helper)
        for arm in (
            '"CounterEffect" => validated_counter_effect(payload)',
            'validated_counter_effect(payload.clone())?;',
        ):
            self.assertTrue(any(arm in value for value in literals))
            self.assertIn(arm, retained)


if __name__ == "__main__":
    unittest.main()
