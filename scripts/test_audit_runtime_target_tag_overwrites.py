import unittest
from audit_runtime_target_tag_overwrites import scan_definition


def target(tag):
    return {"kind": "TaggedEffect", "payload": {"tag": tag, "effect": {
        "kind": "TargetOnlyEffect", "payload": {"target": {"Target": {"Object": {"card_types": ["Creature"]}}}, "explicit_declaration": True}}}}


def consume(tag):
    return {"kind": "ForEachObject", "payload": {"filter": {"tagged_constraints": [{"tag": tag, "relation": "IsTaggedObject"}]}, "effects": []}}


def program(*groups):
    return {"segments": [{"default_effects": g} for g in groups]}


class TargetTagAuditTests(unittest.TestCase):
    def test_declared_target_union_consumed_after_second_segment(self):
        definition = {"spell_effect": program([{"kind": "SequenceEffect", "payload": {"effects": [target("chosen"), target("chosen")]}}], [consume("chosen")])}
        rows, _ = scan_definition(definition)
        self.assertEqual(len(rows), 1)
        self.assertEqual([r["tag"] for r in rows[0]["writes"]], ["chosen", "chosen"])

    def test_rendered_flattened_cache_does_not_create_execution_write(self):
        p = program([target("chosen"), consume("chosen")])
        p["flattened_default_effects"] = [target("chosen"), target("chosen"), consume("chosen")]
        self.assertEqual(scan_definition({"spell_effect": p})[0], [])

    def test_different_abilities_do_not_share_target_bindings(self):
        a = {"effects": program([target("chosen"), consume("chosen")])}
        self.assertEqual(scan_definition({"abilities": [a, a]})[0], [])

    def test_alternate_branches_do_not_form_a_sequence(self):
        conditional = {"kind": "IfEffect", "payload": {"then": [target("chosen")], "else": [target("chosen"), consume("chosen")]}}
        self.assertEqual(scan_definition({"spell_effect": program([conditional])})[0], [])

    def test_scratch_tag_overwrites_are_excluded(self):
        rows, stats = scan_definition(program([target("__it__"), target("__it__"), consume("__it__")]))
        self.assertEqual(rows, [])
        self.assertEqual(stats["scratch_consumers_ignored"], 1)

    def test_consumer_before_repeated_declaration_is_not_a_lead(self):
        self.assertEqual(scan_definition(program([consume("chosen"), target("chosen"), target("chosen")]))[0], [])

    def test_distinct_tags_are_not_overwrites(self):
        self.assertEqual(scan_definition(program([target("first"), target("other"), consume("first")]))[0], [])


if __name__ == "__main__":
    unittest.main()
