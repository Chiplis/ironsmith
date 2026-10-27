import unittest
from keyword_reminder_selection import select_representatives, selection_report


class RepresentativeSelectionTests(unittest.TestCase):
    def test_complete_haste_definition_wins_over_attack_only(self):
        texts = [
            'This creature can attack as soon as it comes under your control.',
            'This creature can attack and {T} as soon as it comes under your control.',
            'It can attack and {T} this turn.',
        ]
        probes = [dict(rule='702.10', text=text, baseline='Haste', name='Example') for text in texts]
        catalog = [dict(rule='702.10', title='Haste', aliases=['haste'])]
        self.assertEqual(select_representatives(probes, catalog)[0]['text'], texts[1])
        self.assertEqual(select_representatives(list(reversed(probes)), catalog)[0]['text'], texts[1])

    def test_granted_scope_is_not_a_standalone_definition(self):
        probes = [
            dict(rule='702.3', baseline='Defender', text="This creature can't attack.", name='A'),
            dict(rule='702.3', baseline='Creatures with gold counters have defender.',
                 text="Those creatures with gold counters can't attack.", name='B'),
        ]
        catalog = [dict(rule='702.3', title='Defender', aliases=['defender'])]
        self.assertEqual(select_representatives(probes, catalog)[0]['text'], probes[0]['text'])

    def test_missing_samples_are_not_synthesized(self):
        self.assertEqual(select_representatives([], [dict(rule='701.2', title='Activate')]), [])

    def test_two_instances_do_not_define_one_keyword(self):
        probes = [
            dict(rule='702.108', baseline='Prowess', text='Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.', name='A'),
            dict(rule='702.108', baseline='Prowess, prowess', text='Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn twice.', name='B'),
        ]
        catalog = [dict(rule='702.108', title='Prowess', aliases=['prowess'])]
        self.assertEqual(select_representatives(probes, catalog)[0]['text'], probes[0]['text'])

    def test_longer_warp_exception_never_defines_warp(self):
        probes = [
            dict(rule='702.185', baseline='Warp {1}{G}', text='You may cast this card from your hand for its warp cost.', name='Ordinary'),
            dict(rule='702.185', baseline='Warp—{B}, Pay 2 life.', text='You may cast this card from your hand or graveyard for its warp cost. If you do, exile it later.', name='Exception'),
        ]
        catalog = [dict(rule='702.185', title='Warp', aliases=['warp'])]
        self.assertEqual(select_representatives(probes, catalog)[0]['name'], 'Ordinary')
        report = selection_report(probes, catalog)[0]
        self.assertIn('graveyard', report['rejected'][0]['reasons'][0])
        self.assertEqual(probes[1]['text'], 'You may cast this card from your hand or graveyard for its warp cost. If you do, exile it later.')

    def test_combined_definitions_do_not_win_by_length(self):
        cases = [
            ('701.22', 'Scry', 'Scry 1.', 'Look at the top card of your library. You may put it on the bottom.',
             'Scry 1 and create a Treasure token.', 'Look at the top card of your library. You may put it on the bottom. A Treasure token is an artifact with an ability.'),
            ('701.67', 'Waterbend', 'Waterbend {5}', 'While paying this cost, tap your artifacts and creatures to help.',
             'Exhaust — Waterbend {20}: Take an extra turn.', 'While paying this cost, tap your artifacts and creatures to help. Activate each exhaust ability only once.'),
            ('702.43', 'Modular', 'Modular 3', 'This enters with three counters. Move them when it dies.',
             'Modular—Sunburst', 'This enters with a counter for each color spent to cast it. Move them when it dies.'),
            ('702.184', 'Station', 'Station', 'Tap another creature: Put counters equal to its power on this Spacecraft.',
             'Station', "Tap another creature: Put counters equal to its power on this Spacecraft. It's an artifact creature at 20+."),
        ]
        for rule, title, good_decl, good_text, bad_decl, bad_text in cases:
            with self.subTest(rule=rule):
                probes = [dict(rule=rule, baseline=decl, text=text, name=name) for decl, text, name in
                          [(good_decl, good_text, 'Generic'), (bad_decl, bad_text, 'Combined')]]
                catalog = [dict(rule=rule, title=title, aliases=[title.lower()])]
                selected = select_representatives(probes, catalog)[0]
                if rule == '702.184':
                    self.assertIn('this permanent', selected['text'])
                    self.assertNotIn('Spacecraft', selected['text'])
                    self.assertNotIn('20+', selected['text'])
                else:
                    self.assertEqual(selected['name'], 'Generic')

    def test_base_keyword_is_not_replaced_by_a_longer_variant(self):
        probes = [
            dict(rule='702.49', baseline='Ninjutsu {B}', text='Return an attacker: Put this card from your hand onto the battlefield.', name='Base'),
            dict(rule='702.49', baseline='Commander ninjutsu {U}{B}', text='Return an attacker: Put this card from your hand or the command zone onto the battlefield.', name='Variant'),
        ]
        catalog = [dict(rule='702.49', title='Ninjutsu', aliases=['ninjutsu', 'commander ninjutsu'])]
        self.assertEqual(select_representatives(probes, catalog)[0]['name'], 'Base')

    def test_exception_only_family_does_not_silently_fall_back(self):
        probes = [dict(rule='702.185', baseline='Warp {B}', text='You may cast this card from your hand or graveyard for its warp cost.', name='Exception')]
        catalog = [dict(rule='702.185', title='Warp', aliases=['warp'])]
        self.assertEqual(select_representatives(probes, catalog), [])
        self.assertIn('No eligible generic', selection_report(probes, catalog)[0]['reason'])

    def test_grant_only_observation_uses_explicit_rule_derived_definition(self):
        probes = [dict(rule='702.64', baseline='All Sliver creatures have absorb 1.', text='If a source would deal damage to a Sliver, prevent 1 of that damage.', name='Grant')]
        catalog = [dict(rule='702.64', title='Absorb', aliases=['absorb'])]
        result = select_representatives(probes, catalog)[0]
        self.assertEqual(result['baseline'], 'Absorb 1')
        self.assertIn('this creature', result['text'])
        self.assertEqual(result['definition_source'], 'official-rule-derived')
        self.assertEqual(result['observed_text'], probes[0]['text'])

    def test_copy_definition_keeps_source_types_instead_of_sample_types(self):
        probes = [dict(rule='702.128', baseline='Embalm {3}', text="{3}: Create a white Zombie Cat copy.", name='Cat')]
        catalog = [dict(rule='702.128', title='Embalm', aliases=['embalm'])]
        result = select_representatives(probes, catalog)[0]
        self.assertNotIn('Cat', result['text'])
        self.assertIn('in addition to its other types', result['text'])

    def test_named_source_is_not_a_reusable_definition(self):
        probes = [
            dict(rule='702.3', baseline='Defender', text="This creature can't attack.", name='Ordinary'),
            dict(rule='702.3', baseline='Defender', text="The Long-Named Guardian can't attack, even during an extra combat phase.", name='The Long-Named Guardian'),
        ]
        catalog = [dict(rule='702.3', title='Defender', aliases=['defender'])]
        self.assertEqual(select_representatives(probes, catalog)[0]['name'], 'Ordinary')

    def test_card_type_dependent_rules_keep_both_branches(self):
        cases = [
            ('702.44', 'Sunburst', 'This artifact enters with a charge counter for each color spent.', ['+1/+1 counter', 'charge counter']),
            ('702.55', 'Haunt', 'When this spell resolves, exile it haunting a creature.', ['from the battlefield', 'during its resolution']),
            ('702.174', 'Gift a card', 'They draw a card before its other effects.', ['when it enters', 'before its other effects']),
        ]
        for rule, title, text, branches in cases:
            with self.subTest(rule=rule):
                probes = [dict(rule=rule, baseline=title, text=text, name='Printed example')]
                result = select_representatives(probes, [dict(rule=rule, title=title, aliases=[title])])[0]
                self.assertEqual(result['definition_source'], 'official-rule-derived')
                self.assertEqual(result['observed_text'], text)
                for branch in branches:
                    self.assertIn(branch, result['text'])

    def test_reviewed_fallback_provenance_is_independent_of_input_order(self):
        probes = [dict(rule='702.64', baseline='All Slivers have absorb 1.', text=text, name=name)
                  for text, name in [('Prevent damage to any Sliver.', 'B'), ('Prevent 1 damage to each Sliver.', 'A')]]
        catalog = [dict(rule='702.64', title='Absorb', aliases=['absorb'])]
        self.assertEqual(select_representatives(probes, catalog), select_representatives(list(reversed(probes)), catalog))


if __name__ == '__main__':
    unittest.main()
