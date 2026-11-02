"""Reporting regressions only; parent runs Python unittest, no native work."""
import copy
import json
import unittest
from pathlib import Path
from coverage_candidates import merge_candidate_row
ROOT = Path(__file__).resolve().parents[2]

def card(key, text=None, faces=None):
    value={'id':key,'source_card_key':key,'name':'Shared display name','original_text':'printed body',
           'source_faces':faces or [{'name':'Shared display name','original_text':'printed body'}],
           'executable_text':text,'rewrite_status':'frontend_materialized_unverified' if text else 'pending'}
    if text:value['executable_faces']=[{'name':'Shared display name','executable_text':text}]
    return value

def row(cards=(),reminder=None):
    return {'id':'msem-probe','scope':'msem','keyword':'Probe','original_reminder':'exact reminder',
            'representative_cards':list(cards),'executable_reminder':reminder}

class CandidateMergeTests(unittest.TestCase):
    def test_all_thirty_one_actual_intrinsic_rows_survive_later_null_pending_overlay(self):
        fixtures=ROOT/'fixtures/custom_mechanics'
        seed={r['id']:r for r in json.loads((fixtures/'msem-source.json').read_text())['mechanics']}
        ready=json.loads((fixtures/'msem-intrinsic-characteristics-next.json').read_text())['mechanics']
        pending={r['id']:r for r in json.loads((fixtures/'msem-special-actions.json').read_text())['mechanics']}
        self.assertEqual(len(ready),31)
        for candidate in ready:
            for overlays in [(candidate,pending[candidate['id']]),(pending[candidate['id']],candidate)]:
                actual=copy.deepcopy(seed[candidate['id']])
                for i,overlay in enumerate(overlays):merge_candidate_row(actual,overlay,f'overlay-{i}')
                expected={c['source_card_key']:c for c in candidate['representative_cards']}
                actual_cards={c['source_card_key']:c for c in actual['representative_cards']}
                for key,expected_card in expected.items():
                    self.assertEqual(actual_cards[key]['executable_text'],expected_card['executable_text'],(candidate['id'],key))
                    self.assertTrue(actual_cards[key]['executable_faces'])
                self.assertEqual(actual['executable_reminder'],candidate['executable_reminder'])
                self.assertEqual(actual['keyword'],seed[candidate['id']]['keyword'])
                self.assertEqual(actual['original_reminder'],seed[candidate['id']]['original_reminder'])

    def test_actual_raw_corpus_face_uses_its_explicit_card_name(self):
        fixture=json.loads((ROOT/'fixtures/custom_mechanics/msem-source-collections-next.json').read_text())
        candidate=next(r for r in fixture['mechanics'] if any(c.get('source_card_key')=='Depthlight Captain_EIA' for c in r['representative_cards']))
        actual=copy.deepcopy(candidate);actual['representative_cards']=[]
        merge_candidate_row(actual,candidate,'raw-corpus-fixture')
        captain=next(c for c in actual['representative_cards'] if c['source_card_key']=='Depthlight Captain_EIA')
        self.assertEqual(captain['source_faces'][0]['name'],'Depthlight Captain')
        self.assertEqual(captain['source_faces'][0]['original_text'],captain['original_text'])
        self.assertIsNone(captain['executable_text'])

    def test_raw_back_face_has_only_its_explicit_suffixed_identity_and_body(self):
        physical=card('Linked_SET')
        physical['source_faces']=[{'cardName':'Front','typeLine':'Creature','rulesText':'front words',
                                  'cardName2':'Back','typeLine2':'Land','rulesText2':'back words'}]
        actual=row();merge_candidate_row(actual,row([physical]),'raw-two-face')
        faces=actual['representative_cards'][0]['source_faces']
        self.assertEqual([(face['name'],face['face']) for face in faces],[('Front','front'),('Back','back')])
        self.assertEqual(faces[1]['original_text'],'Type: Land\nback words')

    def test_unidentified_face_stays_fail_closed_without_a_guessed_name(self):
        physical=card('Unknown_SET','You gain 1 life.')
        physical['source_faces']=[{'rulesText':'body without any face identity'}]
        actual=row();merge_candidate_row(actual,row([physical]),'anonymous-face')
        self.assertIsNone(actual['representative_cards'][0]['executable_text'])
        self.assertTrue(any(collision['kind']=='unidentified_source_face' for collision in actual['_source_body_collisions']))
        self.assertNotIn('name',actual['representative_cards'][0]['source_faces'][-1])

    def test_conflicting_nonempty_definitions_are_blocked_in_both_orders(self):
        for versions in [('You gain 1 life.','You gain 2 life.'),('You gain 2 life.','You gain 1 life.')]:
            actual=row()
            for i,text in enumerate(versions):merge_candidate_row(actual,row([card('Real_SET',text)]),str(i))
            self.assertIsNone(actual['representative_cards'][0]['executable_text'])
            self.assertTrue(actual['_source_body_collisions'])
            self.assertFalse(actual['_source_body_collisions'][0]['resolved_by_selected_overlay'])

    def test_conflicting_counter_metadata_is_not_hidden_by_identical_text(self):
        first=card('Real_SET','Put a training counter on this creature.')
        second=copy.deepcopy(first)
        first['custom_keyword_counter_definitions']=[{'name':'training','reminder':'Flying'}]
        second['custom_keyword_counter_definitions']=[{'name':'training','reminder':'Vigilance'}]
        actual=row();merge_candidate_row(actual,row([first]),'first');merge_candidate_row(actual,row([second]),'second')
        self.assertIsNone(actual['representative_cards'][0]['executable_text'])

    def test_explicit_selected_overlay_resolves_program_and_retains_other_source_keys(self):
        actual=row();merge_candidate_row(actual,row([card('A_SET','You gain 1 life.'),card('B_SET','Draw a card.')]),'ordinary')
        merge_candidate_row(actual,row([card('A_SET','You gain 2 life.')]),'selected',selected=True)
        merge_candidate_row(actual,row([card('A_SET')]),'later-pending')
        bodies={c['source_card_key']:c for c in actual['representative_cards']}
        self.assertEqual(set(bodies),{'A_SET','B_SET'})
        self.assertEqual(bodies['A_SET']['executable_text'],'You gain 2 life.')
        self.assertTrue(actual['_source_body_collisions'][0]['resolved_by_selected_overlay'])
        with self.assertRaises(AssertionError):merge_candidate_row(actual,row([]),'another-selected',selected=True)

    def test_physical_faces_merge_without_display_name_identity_fallback(self):
        first=card('Real_SET','Front program.',[{'name':'Front','original_text':'front'}]);first['name']='Front';first['executable_faces']=[{'name':'Front','executable_text':'Front program.'}]
        second=card('Real_SET','Back program.',[{'name':'Back','original_text':'back'}]);second['name']='Back';second['executable_faces']=[{'name':'Back','executable_text':'Back program.'}]
        actual=row();merge_candidate_row(actual,row([first]),'front');merge_candidate_row(actual,row([second,card('Other_SET','Other program.')]),'back')
        self.assertEqual(len(actual['representative_cards']),2)
        linked=next(c for c in actual['representative_cards'] if c['source_card_key']=='Real_SET')
        self.assertEqual({f['name'] for f in linked['source_faces']},{'Front','Back'})
        self.assertEqual({f['name'] for f in linked['executable_faces']},{'Front','Back'})

    def test_scope_seed_and_derived_provenance_are_immutable(self):
        actual=row()
        changed=row();changed['scope']='rev'
        with self.assertRaises(AssertionError):merge_candidate_row(actual,changed,'wrong-scope')
        changed=row();changed['original_reminder']='different'
        with self.assertRaises(AssertionError):merge_candidate_row(actual,changed,'wrong-reminder')
        actual=row();actual['source_inventory_id']='msem-original'
        changed=copy.deepcopy(actual);changed['source_inventory_id']='msem-another'
        with self.assertRaises(AssertionError):merge_candidate_row(actual,changed,'wrong-derived-source')

if __name__=='__main__':unittest.main()
