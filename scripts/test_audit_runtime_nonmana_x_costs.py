import unittest
from audit_runtime_nonmana_x_costs import findings


def mana(symbol):
    return {'Mana': {'pips': [[symbol]]}}


def reveal(count='X'):
    return {'Effect': {'kind': 'RevealFromHandEffect', 'payload': {'count': count}}}


def ability(cost):
    return {'abilities': [{'kind': {'Activated': {'mana_cost': cost}}}]}


class NonmanaXCostAuditTests(unittest.TestCase):
    def test_fixed_mana_and_reveal_x(self):
        row, = findings(ability({'kind': {'All': [mana({'Generic': 1}), reveal(), 'SacrificeSelf']}}))
        self.assertEqual(row['route'], 'fixed_mana_clamps_nonmana_x_candidate')

    def test_x_mana_is_not_fixed_mana(self):
        row, = findings(ability({'kind': {'All': [mana('X'), reveal()]}}))
        self.assertEqual(row['route'], 'nonmana_x_without_fixed_mana_clamp_control')

    def test_alternative_branches_are_not_combined(self):
        rows = list(findings(ability({'kind': {'OneOf': [
            {'kind': {'All': [mana({'Generic': 1})]}}, {'kind': {'All': [reveal()]}}]}})))
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['route'], 'nonmana_x_without_fixed_mana_clamp_control')

    def test_constant_reveal_is_not_choosing_x(self):
        row, = findings(ability({'kind': {'All': [mana({'Generic': 1}), reveal({'Fixed': 1})]}}))
        self.assertFalse(row['nonmana_x'])

    def test_spell_cost_not_activation_route(self):
        row, = findings({'additional_cost': {'kind': {'All': [mana({'Generic': 1}), reveal()]}}})
        self.assertEqual(row['route'], 'reveal_or_nonmana_x_other_cost_context')

    def test_transparent_wrapper_delegation(self):
        effect = {'Effect': {'kind': 'TaggedEffect', 'payload': {'effect': reveal()['Effect']}}}
        row, = findings(ability({'kind': {'All': [mana({'Generic': 1}), effect]}}))
        self.assertTrue(row['nonmana_x'])

    def test_sequence_is_not_transparent(self):
        effect = {'Effect': {'kind': 'SequenceEffect', 'payload': {'effects': [reveal()['Effect']]}}}
        self.assertEqual(list(findings(ability({'kind': {'All': [mana({'Generic': 1}), effect]}}))), [])

    def test_flattened_cache_ignored(self):
        self.assertEqual(list(findings({'spell_effect': {'flattened_default_effects': [
            ability({'kind': {'All': [mana({'Generic': 1}), reveal()]}})]}})), [])


if __name__ == '__main__':
    unittest.main()
