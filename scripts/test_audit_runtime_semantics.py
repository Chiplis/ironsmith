import unittest

from audit_runtime_semantics import normalize, scan_text


class SemanticAuditTests(unittest.TestCase):
    def test_equivalent_number_and_comparison_surfaces_are_not_candidates(self):
        oracle = "If four or more mana was spent, draw a card."
        for compiled in ["If at least 4 mana was spent, draw a card.",
                         "If 4 or greater mana was spent, draw a card."]:
            self.assertEqual(scan_text(oracle, compiled), [])
        self.assertEqual(normalize("one hundred or fewer"), "100 or less")

    def test_other_ability_cannot_supply_missing_constraint(self):
        findings = scan_text(
            "If you control four or fewer lands, draw a card.\n"
            "Whenever this creature attacks, if you control four or fewer lands, gain 2 life.",
            "If you control a land, draw a card.\n"
            "Whenever this creature attacks, if you control four or fewer lands, gain 2 life.",
        )
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["oracle_line_index"], 0)
        self.assertEqual(findings[0]["checks"][0]["check"], "numeric_threshold_not_preserved")
        self.assertEqual(findings[0]["status"], "candidate")

    def test_graveyard_domain_and_threshold_are_both_screened(self):
        findings = scan_text("When this creature dies, draw a card for each graveyard with seven or more cards in it.",
                             "When this creature dies, draw a card for each card in a graveyard.")
        self.assertEqual({check["check"] for check in findings[0]["checks"]},
                         {"numeric_threshold_not_preserved", "graveyard_count_domain_not_preserved"})

    def test_lost_aggregate_and_delayed_timing_are_candidates(self):
        self.assertEqual(scan_text("Exile any number of cards with total mana value 30 or greater: Draw a card.",
                                   "Exile any number of cards with mana value 30 or greater: Draw a card.")[0]
                         ["checks"][0]["check"], "aggregate_constraint_not_preserved")
        self.assertIn("delayed_copy_timing_not_preserved", [c["check"] for c in scan_text(
            "{T}: When you next activate an ability this turn, copy that ability.",
            "{T}: Copy that ability.")[0]["checks"]])


if __name__ == "__main__":
    unittest.main()
