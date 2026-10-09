"""Registration matching tests that exercise ambiguous lines and exotic rails."""
import importlib.util,pathlib,unittest
spec=importlib.util.spec_from_file_location('history',pathlib.Path(__file__).with_name('register-frame-history.py'))
history=importlib.util.module_from_spec(spec);spec.loader.exec_module(history)
def row(text,y,x=.08,width=.7,height=.025):return dict(text=text,x=x,y=y,width=width,height=height,confidence=.99)
def case(layout='normal'):return dict(id='synthetic',face=None,number='1',source='https://cards.scryfall.io/normal/front/synthetic.jpg',set='tst',layout=layout)
class RegistrationTests(unittest.TestCase):
 def test_headers_cannot_capture_body_references(self):
  printing=dict(name='Returning Bird',type_line='Creature — Bird',power='2',toughness='2',oracle_text='Flying\nWhen Returning Bird dies, draw a card.')
  r,reason=history.build(case(),printing,[row('Returning Bird',.06),row('Creature - Bird',.58),row('Flying',.65),row('When Returning Bird dies,',.72),row('draw a card.',.75),row('2/2',.9,.82,.08)])
  self.assertIsNone(reason)
  self.assertLess(next(f for f in r['fields'] if f['kind']=='name')['bounds']['y'],.1)
  self.assertEqual(len(next(f for f in r['fields'] if f['kind']=='rule' and f['index']==1)['lines']),2)
 def test_omitted_reminder_is_not_missing_rules(self):
  printing=dict(name='Bird',type_line='Creature — Bird',oracle_text='Flying (This creature can only be blocked by creatures with flying or reach.)')
  r,reason=history.build(case(),printing,[row('Bird',.06),row('Creature - Bird',.58),row('Flying',.65)])
  self.assertIsNone(reason)
  self.assertEqual(next(f for f in r['fields'] if f['kind']=='rule')['printedText'],'Flying')
 def test_bottom_type_is_a_hard_region_limit(self):
  printing=dict(name='A Case',type_line='Enchantment — Case',oracle_text='Draw a card.\nSolved — You gain 3 life.')
  r,reason=history.build(case('case'),printing,[row('A Case',.06),row('Draw a card.',.2,.51,.4),row('Solved - You gain 3 life.',.5,.51,.4),row('Enchantment - Case',.86)])
  self.assertIsNone(reason)
  for f in r['fields']:
   if f['kind']=='rule':
    self.assertTrue(f['noFlow'])
    self.assertLess(f['region']['y']+f['region']['height'],.86)
 def test_illusion_and_numeric_creatures_are_not_footer_text(self):
  printing=dict(name='A Saga',type_line='Enchantment — Saga',oracle_text='I — Create a 6/6 legendary Horror.\nII — The illusion lasts forever.')
  r,reason=history.build(case('saga'),printing,[row('A Saga',.06),row('Create a',.3,.13,.3),row('6/6 legendary Horror',.34,.13,.3),row('The illusion lasts forever.',.5,.13,.3),row('Enchantment - Saga',.86)])
  self.assertIsNone(reason)
  rules=[f for f in r['fields'] if f['kind']=='rule']
  self.assertEqual(len(rules[0]['lines']),2)
  self.assertEqual(rules[1]['lines'][0]['text'],'The illusion lasts forever.')
 def test_class_level_cost_segments_belong_to_the_level_field(self):
  printing=dict(name='A Class',type_line='Enchantment — Class',oracle_text='You gain 1 life.\n{1}{B}: Level 2\nDraw a card.')
  r,reason=history.build(case('class'),printing,[row('A Class',.06),row('You gain 1 life.',.2,.51,.4),row('1B:',.4,.51,.08),row('Level 2',.4,.77,.14),row('Draw a card.',.5,.51,.4),row('Enchantment - Class',.86)])
  self.assertIsNone(reason)
  level=next(f for f in r['fields'] if f['kind']=='rule' and f['index']==1)
  self.assertEqual([l['text'] for l in level['lines']],['1B:','Level 2'])
  self.assertEqual(len(next(f for f in r['fields'] if f['kind']=='rule' and f['index']==0)['lines']),1)
 def test_short_keyword_cannot_be_swallowed_by_later_token_rules(self):
  printing=dict(name='Cat',type_line='Creature — Cat',oracle_text='Lifelink\nWhenever this creature mutates, create two Cat creature tokens with lifelink.')
  r,reason=history.build(case('mutate'),printing,[row('Cat',.06),row('Creature - Cat',.58),row('Lifelink',.65),row('Whenever this creature mutates, create',.73),row('two Cat creature tokens with lifelink.',.77)])
  self.assertIsNone(reason)
  self.assertEqual(next(f for f in r['fields'] if f['kind']=='rule' and f['index']==0)['lines'][0]['text'],'Lifelink')
 def test_prepare_spell_headers_and_rules_use_the_right_column(self):
  printing=dict(name='Student // Homework',card_faces=[dict(name='Student',type_line='Creature — Wizard',oracle_text='This creature enters prepared.'),dict(name='Homework',type_line='Sorcery',oracle_text='Draw two cards.')])
  r,reason=history.build(case('prepare'),printing,[row('Student',.06),row('Creature - Wizard',.58),row('This creature enters prepared.',.65,.08,.38),row('Homework',.64,.52,.3),row('Sorcery',.69,.52,.25),row('Draw two cards.',.75,.52,.35)])
  self.assertIsNone(reason)
  spell=[f for f in r['fields'] if f['face']==1]
  self.assertTrue(all(f['bounds']['x']>=.5 for f in spell))
  self.assertTrue(all(f.get('region',f['bounds'])['width']>0 for f in spell))
 def test_split_headers_on_the_same_baseline_do_not_consume_each_other(self):
  printing=dict(name='Fire // Ice',card_faces=[dict(name='Fire',type_line='Instant',oracle_text='Deal 2 damage.'),dict(name='Ice',type_line='Instant',oracle_text='Draw a card.')])
  logical=[row('Fire',.06,.1,.15),row('Ice',.06,.56,.15),row('Instant',.55,.1,.15),row('Instant',.55,.56,.15),row('Deal 2 damage.',.7,.1,.32),row('Draw a card.',.7,.56,.3)]
  original=[{**l,'x':l['y'],'y':1-l['x']-l['width'],'width':l['height'],'height':l['width']} for l in logical]
  r,reason=history.build(case('split'),printing,original)
  self.assertIsNone(reason)
  types=[f for f in r['fields'] if f['kind']=='type']
  self.assertEqual(len(types),2)
  self.assertTrue(all(len(f['lines'])==1 for f in types))
 def test_flip_rules_before_type_and_inverted_secondary_face(self):
  printing=dict(name='Ascendant // Blessing',card_faces=[dict(name='Ascendant',type_line='Creature — Monk',power='1',toughness='1',oracle_text='Flying'),dict(name='Blessing',type_line='Legendary Enchantment',oracle_text='Draw a card.')])
  front=[row('Ascendant',.06),row('Flying',.13),row('Creature - Monk',.27),row('1/1',.26,.83,.08)]
  reverse=[row('Blessing',.06),row('Draw a card.',.13),row('Legendary Enchantment',.27)]
  r,reason=history.build(case('flip'),printing,front,reverse)
  self.assertIsNone(reason)
  second=[f for f in r['fields'] if f['face']==1]
  self.assertTrue(all(f['rotation']==180 for f in second))
  self.assertTrue(all(f['bounds']['y']>.6 for f in second))
 def test_repeated_level_abilities_stay_in_their_printed_band(self):
  printing=dict(name='Student',type_line='Creature — Wizard',power='1',toughness='1',oracle_text='Level up {1}\nLEVEL 2-3\n2/2\n{T}: Copy target spell.\nLEVEL 4+\n3/3\n{T}: Copy target spell twice.')
  rows=[row('Student',.06),row('Creature - Wizard',.58),row('1/1',.66,.83,.08),row('Level up 1',.65,.08,.3),row('LEVEL',.72,.08,.12),row('2-3',.75,.08,.12),row('2/2',.74,.83,.08),row('T: Copy target spell.',.73,.25,.45),row('LEVEL',.82,.08,.12),row('4+',.85,.08,.12),row('3/3',.84,.83,.08),row('T: Copy target spell',.83,.25,.45),row('twice.',.86,.25,.25)]
  r,reason=history.build(case('leveler'),printing,rows)
  self.assertIsNone(reason)
  rules=[f for f in r['fields'] if f['kind']=='rule']
  self.assertEqual([l['text'] for l in rules[-1]['lines']],['T: Copy target spell','twice.'])
  self.assertEqual(len(rules[-2]['lines']),1)
 def test_shared_room_type_is_not_matched_to_an_ability_reference(self):
  printing=dict(name='Study // Parlor',card_faces=[dict(name='Study',type_line='Enchantment — Room',oracle_text='Nonland permanents are enchantments.'),dict(name='Parlor',type_line='Enchantment — Room',oracle_text='Whenever you cast an enchantment spell, draw a card.')])
  logical=[row('Study',.06,.1,.15),row('Parlor',.06,.56,.15),row('Enchantment - Room',.55,.1,.28),row('Nonland permanents are enchantments.',.73,.1,.32),row('Whenever you cast an',.73,.56,.32),row('enchantment spell,',.77,.56,.32),row('draw a card.',.81,.56,.3)]
  original=[{**l,'x':l['y'],'y':1-l['x']-l['width'],'width':l['height'],'height':l['width']} for l in logical]
  original.append(row('2024 Wizards',.96,.3,.3))
  r,reason=history.build(case('split'),printing,original)
  self.assertIsNone(reason)
  self.assertEqual(len([f for f in r['fields'] if f['kind']=='type']),1)
  ability=next(f for f in r['fields'] if f['kind']=='rule' and f['face']==1)
  self.assertEqual(len(ability['lines']),3)
  self.assertFalse(any('Wizards' in l['text'] for f in r['fields'] for l in f['lines']))
 def test_transform_saga_uses_its_side_panel_geometry(self):
  printing=dict(name='Chapter // Creature',card_faces=[dict(name='Chapter',type_line='Enchantment — Saga',oracle_text='I — Draw a card.\nII — You gain 3 life.'),dict(name='Creature',type_line='Creature — Bird',oracle_text='Flying')])
  c={**case('transform'),'face':0}
  r,reason=history.build(c,printing,[row('Chapter',.06),row('Draw a card.',.3,.13,.3),row('You gain 3 life.',.5,.13,.3),row('Enchantment - Saga',.86)])
  self.assertIsNone(reason)
  self.assertEqual(r['layout'],'transform')
  self.assertTrue(all(f['region']['x']+f['region']['width']<.5 for f in r['fields'] if f['kind']=='rule'))
 def test_color_indicator_is_not_part_of_the_type_lettering(self):
  printing=dict(name='Machine',type_line='Legendary Artifact — Vehicle',color_indicator=['B','R'],oracle_text='Flying')
  r,reason=history.build(case(),printing,[row('Machine',.06),row('© Legendary Artifact',.582,.074,.43),row('Vehicle',.579,.56,.14),row('Flying',.67)])
  self.assertIsNone(reason)
  field=next(f for f in r['fields'] if f['kind']=='type')
  self.assertEqual([l['text'] for l in field['lines']],['Legendary Artifact','Vehicle'])
  self.assertEqual(field['bounds']['x'],.125)
 def test_loyalty_cost_badges_do_not_become_paragraph_words(self):
  printing=dict(name='Walker',type_line='Legendary Planeswalker — Walker',loyalty='4',oracle_text='+1: Draw a card.\n-3: Destroy target creature.')
  r,reason=history.build(case(),printing,[row('Walker',.06),row('Legendary Planeswalker - Walker',.58),row('Draw a card.',.69,.18,.5),row('+1',.70,.08,.05),row('Destroy target creature.',.8,.18,.5),row('-3',.81,.08,.05),row('4',.9,.85,.07)])
  self.assertIsNone(reason)
  self.assertEqual([f['text'] for f in r['fields'] if f['kind']=='loyalty-cost'],['+1','-3'])
  self.assertTrue(all(not any(l['text'] in ['+1','-3'] for l in f['lines']) for f in r['fields'] if f['kind']=='rule'))
 def test_separate_mana_ability_introductions_do_not_merge(self):
  printing=dict(name='Grove',type_line='Land',oracle_text='{T}: Add {C}.\n{G/U}, {T}: Add {G}{G}, {G}{U}, or {U}{U}.')
  r,reason=history.build(case(),printing,[row('Grove',.06),row('Land',.86),row('e: Add ◇ to your mana pool.',.65),row('e, e: Add ☀☀, ☀●, or ●● to your',.71),row('mana pool.',.74)])
  self.assertIsNone(reason)
  rules=[f for f in r['fields'] if f['kind']=='rule']
  self.assertEqual(len(rules[0]['lines']),1)
  self.assertEqual(len(rules[1]['lines']),2)
 def test_alternate_printed_title_and_canonical_subtitle_have_separate_fields(self):
  printing=dict(name='Original Monster',flavor_name='Movie Monster',type_line='Creature — Beast',oracle_text='Trample')
  r,reason=history.build(case(),printing,[row('Movie Monster',.06),row('Original Monster',.12,.08,.3,.015),row('Creature - Beast',.58),row('Trample',.65)])
  self.assertIsNone(reason)
  title=next(f for f in r['fields'] if f['kind']=='name')
  self.assertEqual(title['printedText'],'Movie Monster')
  self.assertEqual(next(f for f in r['fields'] if f['kind']=='subtitle')['text'],'Original Monster')
 def test_decorative_headers_have_bounded_changed_label_templates(self):
  printing=dict(name='Mystery',type_line='Instant',set='mp2',oracle_text='Draw a card.')
  r,reason=history.build(case(),printing,[row('DRAW A CARD.',.72)])
  self.assertIsNone(reason)
  headers=[f for f in r['fields'] if f['kind'] in ['name','type']]
  self.assertTrue(all(f['opaqueHeader'] for f in headers))
  self.assertTrue(all(f['bounds']['x']+f['bounds']['width']<.86 for f in headers))
 def test_invocation_stats_keep_their_vertical_fraction(self):
  printing=dict(name='God',type_line='Creature — God',set='mp2',power='5',toughness='5',oracle_text='Indestructible')
  r,reason=history.build(case(),printing,[row('INDESTRUCTIBLE',.68),row('5',.82,.88,.04,.03),row('5',.89,.88,.04,.03)])
  self.assertIsNone(reason)
  stats=next(f for f in r['fields'] if f['kind']=='stats')
  self.assertTrue(stats['stackedStats'])
  self.assertEqual(len(stats['lines']),2)
 def test_centered_short_ability_has_the_full_printed_panel(self):
  printing=dict(name='Reason',type_line='Sorcery',oracle_text='Scry 3.')
  r,reason=history.build(case(),printing,[row('Reason',.06),row('Sorcery',.36),row('Scry 3.',.46,.42,.16)])
  self.assertIsNone(reason)
  rule=next(f for f in r['fields'] if f['kind']=='rule')
  self.assertLess(rule['region']['x'],.1)
  self.assertGreater(rule['region']['width'],.8)
 def test_explicit_textless_printings_have_no_invented_rules_box(self):
  printing=dict(name='A Land',type_line='Land',oracle_text='This land enters tapped.',textless=True)
  r,reason=history.build(case(),printing,[row('A Land',.06),row('Land',.85)])
  self.assertIsNone(reason)
  self.assertFalse(any(f['kind']=='rule' for f in r['fields']))
 def test_retro_planeswalker_inline_costs_are_not_split_into_negative_badges(self):
  printing=dict(name='Old Walker',type_line='Legendary Planeswalker — Walker',loyalty='4',oracle_text='+1: Draw a card.\n−3: Gain 3 life.')
  r,reason=history.build(case(),printing,[row('Old Walker',.06),row('Legendary Planeswalker - Walker',.58),row('+1: Draw a card.',.65,.13),row('-3: Gain 3 life.',.75,.13),row('4',.9,.85,.04)])
  self.assertIsNone(reason)
  self.assertFalse(any(f['kind']=='loyalty-cost' for f in r['fields']))
  self.assertTrue(all(f.get('inlineLoyalty') for f in r['fields'] if f['kind']=='rule'))
 def test_fuzzy_level_label_stays_on_its_marker_rail(self):
  printing=dict(name='Student',type_line='Creature — Wizard',power='1',toughness='1',oracle_text='Level up {3}\nLEVEL 1-2\n2/2\nFlying')
  rows=[row('Student',.06),row('Creature - Wizard',.58),row('Level up 3',.65),row('1/1',.66,.83,.08),row('LEVBL',.74,.1,.08),row('1-2',.77,.1,.08),row('2/2',.75,.83,.08),row('Flying',.75,.25,.15)]
  r,reason=history.build(case('leveler'),printing,rows)
  self.assertIsNone(reason)
  self.assertEqual([l['text'] for f in r['fields'] if f['kind']=='rule' and f['text']=='Flying' for l in f['lines']],['Flying'])
 def test_prototype_stats_are_separate_from_its_reminder(self):
  printing=dict(name='Construct',type_line='Artifact Creature — Construct',power='8',toughness='8',oracle_text='Prototype {2}{G}{G} — 3/3 (You may cast this spell with different mana cost, color, and size. It keeps its abilities and types.)\nTrample')
  rows=[row('Construct',.06),row('Artifact Creature - Construct',.58),row('Prototype (You may cast this spell with',.65),row('different mana cost, color, and size. It',.68),row('keeps its abilities and types.)',.71),row('3/3',.69,.83,.08),row('Trample',.8),row('8/8',.9,.83,.08)]
  r,reason=history.build(case('prototype'),printing,rows)
  self.assertIsNone(reason)
  prototype=next(f for f in r['fields'] if f.get('prototypeRail'))
  self.assertFalse(any(l['text']=='3/3' for l in prototype['lines']))
  self.assertTrue(any(f['kind']=='preview-stats' for f in r['fields']))
 def test_letter_o_color_indicator_preserves_native_icon(self):
  printing=dict(name='Walker',type_line='Legendary Planeswalker — Walker',color_indicator=['W'],oracle_text='Draw a card.')
  r,reason=history.build(case(),printing,[row('Walker',.06),row('O Legendary Planeswalker - Walker',.58,.074,.7),row('Draw a card.',.67)])
  self.assertIsNone(reason)
  field=next(f for f in r['fields'] if f['kind']=='type')
  self.assertEqual(field['lines'][0]['text'],'Legendary Planeswalker - Walker')
  self.assertGreaterEqual(field['lines'][0]['x'],.125)
 def test_battle_back_face_stats_do_not_enter_reminder(self):
  reminder="(As a Siege enters, choose an opponent to protect it. You and others can attack it. When it's defeated, exile it, then cast it transformed.)"
  printing=dict(name='Battle // Bird',card_faces=[dict(name='Battle',type_line='Battle — Siege',defense='5',oracle_text=reminder+'\nDraw a card.'),dict(name='Bird',type_line='Creature — Bird',power='4',toughness='4',oracle_text='Flying')])
  logical=[row('Battle',.06),row('Battle - Siege',.58),row(reminder,.67,.13,.7,.05),row('4/4',.70,.9,.04),row('Draw a card.',.79,.13,.7),row('5',.91,.93,.02)]
  native=[{**l,'x':l['y'],'y':1-l['x']-l['width'],'width':l['height'],'height':l['width']} for l in logical]
  r,reason=history.build({**case('transform'),'face':0},printing,native)
  self.assertIsNone(reason)
  self.assertFalse(any(l['text']=='4/4' for f in r['fields'] if f['kind']=='rule' for l in f['lines']))
  self.assertTrue(any(f['kind']=='preview-stats' and f['text']=='4/4' for f in r['fields']))
 def test_planeswalker_abilities_stay_in_fixed_loyalty_bands(self):
  printing=dict(name='Walker',type_line='Legendary Planeswalker — Walker',loyalty='4',oracle_text='+1: Draw a card.\n−3: You gain 3 life.')
  r,reason=history.build(case(),printing,[row('Walker',.06),row('Legendary Planeswalker - Walker',.58),row('+1',.65,.07,.05),row('Draw a card.',.65,.18,.6),row('-3',.75,.07,.05),row('You gain 3 life.',.75,.18,.6),row('4',.90,.87,.04)])
  self.assertIsNone(reason)
  rules=[f for f in r['fields'] if f['kind']=='rule']
  self.assertTrue(all(f['noFlow'] for f in rules))
  self.assertLessEqual(rules[0]['region']['y']+rules[0]['region']['height'],rules[1]['bounds']['y'])
if __name__=='__main__':unittest.main()
