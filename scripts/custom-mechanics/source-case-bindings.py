#!/usr/bin/env python3
"""Read-only reviewed source/case bindings; never runs Cargo or infers coverage from names."""
import argparse,hashlib,json,re,tomllib
from pathlib import Path

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def card_sha(c):return hashlib.sha256(json.dumps(c,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
# Each tuple below is an explicit read-only review of the named case's loop,
# selection, source fixture and adapter. Regex discovery below is indexing only.
# target, case, fixture, selector, kind, strict words, routes, adapter
PROFILES=[
('custom_pledge_sources_next','all_eighteen_exact_source_variants_include_every_associated_actual_card','revolution-pledge-exact-source-next.json',{'root_text_only':True},'strict_full_source_compile',False,['native','artifact'],None),
('custom_gallery','every_complete_source_gallery_and_its_work_sections_compile_strictly','msem-gallery.json',{'first_row':True,'root_text_only':True},'strict_full_source_compile',False,['native','artifact'],'cf8_p08/support.rs'),
('custom_module','all_fifteen_complete_source_module_faces_compile_directly_and_through_artifacts','msem-module.json',{'first_row':True,'root_text_only':True},'strict_full_source_compile',False,['native','artifact'],'cf8_p08/support.rs'),
('custom_spirited','complete_spirited_source_variants_compile_without_loss','msem-middle-keywords.json',{'keyword':'Spirited','root_text_only':True},'strict_full_source_compile',True,['artifact'],None),
('custom_arise_action_receipts','full_bugrund_source_arise_observer_attaches_equipment_to_the_new_token','msem-arise.json',{'rows':['msem-846'],'first':True,'root_text_only':True},'gameplay',True,['artifact'],None),
('custom_ability_counter_bindings','exact_upgrade_respawn_source_uses_supplied_program_and_real_counter_removal','msem-growth-actions.json',{'rows':['msem-705'],'first':True,'root_text_only':True},'gameplay',False,['artifact'],None),
('custom_final_abilities','remaining_disgraced_source_variants_and_memory_compile_without_loss','msem-final-abilities.json',{'rows':['msem-884','msem-886','msem-887','msem-917'],'first':True,'root_text_only':True},'strict_full_source_compile',True,['artifact'],None),
('custom_coalesce_composition','all_sixty_one_source_coalesce_programs_round_trip','coalesce-composed-resolution.json',{},'strict_full_source_compile',True,['native','artifact'],'custom_final_next_wave_support.rs'),
('custom_intrinsic_paid_actions','every_source_rhapsody_and_stardom_face_compiles_through_direct_and_artifact_routes','msem-intrinsic-characteristics-next.json',{},'strict_full_source_compile',True,['native','artifact'],'custom_final_next_wave_support.rs'),
('custom_intrinsic_paid_actions','animus_source_exception_is_priority_timing_and_stardom_use_limit_is_structural','msem-intrinsic-characteristics-next.json',{'rows':['msem-809','msem-953'],'first':True},'gameplay',True,['native','artifact'],'custom_final_next_wave_support.rs'),
('custom_intrinsic_paid_actions','serin_compound_grant_supplies_hand_price_and_attacking_static_action_without_battlefield_cast_permission','msem-intrinsic-characteristics-next.json',{'rows':['msem-964'],'first':True},'gameplay',True,['native','artifact'],'custom_final_next_wave_support.rs'),
('custom_abilities_tail','every_materialized_existing_operation_source_compiles_losslessly','msem-abilities-tail.json',{'rewrite_status':'frontend_materialized_unverified'},'strict_full_source_compile',True,['artifact'],None),
('custom_abilities_tail','practiced_full_source_grant_reduces_only_the_next_creature_by_distinct_attackers','msem-abilities-tail.json',{'rows':['msem-946'],'first':True},'gameplay',True,['artifact'],None),
('custom_abilities_tail','compound_copy_exceptions_keep_name_legendary_and_own_other_abilities','msem-abilities-tail.json',{'rows':['msem-937'],'first':True},'gameplay',True,['artifact'],None),
('custom_bless_full_source','all_twenty_six_exact_bless_sources_keep_fifty_complete_faces_and_linked_definitions','revolution-bless-complete.json',{},'strict_full_source_compile',True,['artifact'],None),
('custom_bless_full_source','exact_furies_any_number_player_targets_create_one_complete_blessing_per_selected_player_including_zero','revolution-bless-complete.json',{'names':['Furies of the Natural Order']},'gameplay',False,['artifact'],None),
('custom_linked_face_tokens','full_source_back_copy_is_attached_on_entry_and_front_ability_copy_has_no_back','revolution-experience-source.json',{'first_row':True,'first':True},'gameplay',True,['artifact'],None),
('custom_linked_face_tokens','actual_callaphe_damage_trigger_creates_complete_blessing_only_at_threshold','revolution-experience-source.json',{'first_row':True,'first':True},'gameplay',True,['artifact'],None),
('custom_activation_alternatives','every_complete_dynamize_source_compiles_strictly_and_roundtrips','msem-dynamize-complete.json',{'first_row':True,'root_text_only':True},'strict_full_source_compile',True,['artifact'],None),
('custom_golden_age_designation','all_forty_three_golden_age_source_cards_round_trip','msem-golden-age-designation.json',{},'strict_full_source_compile',True,['artifact'],None),
('custom_endgame_designation','complete_source_endgame_variants_preserve_artifact_and_native_programs','revolution-endgame-designation.json',{},'strict_full_source_compile',True,['artifact'],None),
('custom_optional_text_fragments','full_sources_and_receipt_readers_compile_losslessly_and_transport_all_subsets','msem-glitch-fragments.json',{},'strict_full_source_compile',True,['artifact'],None),
('custom_vision_named_receipts','envisioning_a_creature_is_independent_of_optional_graveyard_move_and_unrelated_reveals','msem-casting-middle.json',{'rows':['msem-575'],'first':True},'gameplay',True,['native','artifact'],None),
('custom_vision_named_receipts','artifact_vision_filters_the_exact_revealed_card_and_does_not_trigger_on_empty_library','msem-casting-middle.json',{'rows':['msem-576'],'first':True},'gameplay',True,['native','artifact'],None),
]

# Reviewed test snapshots: drift requires another read-only review, never restamping.
REVIEWED_TEST_HASHES={'custom_pledge_sources_next': '7a4b6fb22c40ea2d62c1b4794fa4de5b26e0ab7fe644d6acc8b23b8fa2126d7b', 'custom_gallery': '1cecb8ba3e270a6d9672d2b0805067d258602de859f38d74c7b9c9f735ba818c', 'custom_module': '1aecc20e6f61215937864fafc341f9dda5119840bbac5040790721ea2d8d7fe1', 'custom_spirited': 'c4e78628668ea1878e4fd4e664ac7bee7980aad50d40853ffc36ea9b82385c65', 'custom_arise_action_receipts': '85423230a8f04abde91d596931f13ead4e7f1ab343247abe6831395e90659ac6', 'custom_ability_counter_bindings': 'e2bc1ea73bc670a8d4dfc72c819d33bc9608064c90383cda422494bf51375d06', 'custom_final_abilities': '3c9ad0ee761c7d09346dbf7a172851bf8cd4a408e940259a34cda2d04835dc64', 'custom_coalesce_composition': '13a4c515f5421b704fed24be31c7bff876196b82966508642dff2a6795660b77', 'custom_intrinsic_paid_actions': '8e28b310b183df4eb2438cb01cf7c57ae4976f8642898d5f9897844c8234eba2', 'custom_abilities_tail': '5f7bc761362019c7456fa3ba87d45c162058548bafd75194aa4f3d7e9da38df3', 'custom_bless_full_source': 'c96f196a7a278ffe0a0284da54f17b11bd301d44b53117815e288727dd4bf624', 'custom_linked_face_tokens': '7150f39bfd45153050f43e03c60d7bfb13b52f94ec0ac21d9d49e47b88a6473a', 'custom_activation_alternatives': '1e0dd16282b492c918cde2c1a4b5047cc3e201297d92006830258282142fe161', 'custom_golden_age_designation': 'f170771dd42aad54b2e95422b346b9d8a937389825bd815de06844e2c87aec7b', 'custom_endgame_designation': '70584f15753cf83feccf09a4831a9a6dd43f3343eb84a43fd66c736bfd67113f', 'custom_optional_text_fragments': 'c3144f3b7cbc639648acf5d862c87a2dbcfc56fea4c1f1f1d1fe21015f4245eb', 'custom_vision_named_receipts': '1a59c8d02705b71fe0c5b6f263b2b7a5c2556cd474d9f2d140ae434756cb50e6'}

REVIEWED_ADAPTER_HASHES={'custom_final_next_wave_support.rs': 'd7b55d19e594049ebf84c541a761d3c3d27cc81680e6965e838b2c48e4ecca3b', 'cf8_p08/support.rs': '367a64f77cf2988647152bc89f40eafaf205861014fdfcf96925d6cab2f3f7cf'}

def historical_observations(root, descriptors):
 observations=[]
 for descriptor in descriptors:
  payload=json.loads(descriptor.read_text())
  for item in payload.get('results',[]):
   target=item['target'];log=Path(item['log']);log=log if log.is_absolute() else root/log
   errors=[];cases={}
   if not log.is_file() or sha(log)!=item.get('log_sha256'):
    errors.append('log missing or hash differs from captured result')
   else:
    text=re.sub(r'\x1b\[[0-9;]*m','',log.read_text())
    identities=re.findall(r'Running tests/([^\s/]+)\.rs\b',text)
    if identities!=[target]:errors.append('log does not identify exactly the captured target')
    for name,status in re.findall(r'^test (\S+)\s+\.\.\.\s+(ok|FAILED|ignored)\s*$',text,re.MULTILINE):
     if name in cases:errors.append('duplicate test case result: '+name)
     cases[name]={'ok':'passed','FAILED':'failed','ignored':'ignored'}[status]
    summary=re.findall(r'test result: (?:ok|FAILED)\.\s+(\d+) passed;\s+(\d+) failed;',text)
    if len(summary)!=1:errors.append('missing unique native result summary')
    elif tuple(map(int,summary[0]))!=(item.get('passed'),item.get('failed')):errors.append('summary differs from captured counts')
   observations.append({'target':target,'log':str(log),'log_sha256':item.get('log_sha256'),
      'exit_code':item.get('exit'),'captured_result_descriptor':str(descriptor),'captured_result_sha256':sha(descriptor),
      'case_results':cases if not errors else {},'validation_errors':errors,
      'historical_log_integrity_verified':not errors,'source_input_identity_verified':False,
      'current_native_pass':False,
      'qualification_gap':'No tested code fingerprint or input hashes were captured; do not restamp historical logs from current files.'})
 return observations

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--root',type=Path,required=True);ap.add_argument('--targets',type=Path,action='append',required=True);ap.add_argument('--observed-results',type=Path,action='append',default=[]);ap.add_argument('--output-dir',type=Path,required=True);a=ap.parse_args();root=a.root.resolve();out=a.output_dir;out.mkdir(parents=True,exist_ok=True)
 testdir=root/'crates/ironsmith-compiler-runtime/tests';cargo=tomllib.loads((testdir.parent/'Cargo.toml').read_text());registered={t['name'] for t in cargo.get('test',[])}
 names=set()
 for p in a.targets:
  data=json.loads(p.read_text());names.update(data if isinstance(data,list) else data['targets'])
 audits={};bindings=[];problems=[]
 for target in sorted(names):
  file=testdir/(target+'.rs');text=file.read_text() if file.exists() else '';cases=re.findall(r'#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(\w+)',text)
  audits[target]={'target':target,'registered':target in registered,'test_file':str(file.relative_to(root)),'test_file_sha256':sha(file) if file.exists() else None,'cases':[{'case':case,'classification':'unknown_not_yet_source_reviewed','source_bindings':0} for case in cases], 'literal_fixture_references':re.findall(r'include_str!\(\s*"([^"]+)"',text),'note':'Literal references are discovery only; no source/card coverage inferred from names or imports.'}
 for target,case,fixture_name,selector,kind,strict,routes,adapter_name in PROFILES:
  if target not in audits:continue
  audit=audits[target];caseaudit=next((c for c in audit['cases'] if c['case']==case),None)
  if audit['test_file_sha256'] != REVIEWED_TEST_HASHES[target]:
   problems.append({'target':target,'case':case,'error':'test changed after source review; review required before binding'});continue
  if caseaudit is None:problems.append({'target':target,'case':case,'error':'reviewed case absent; no binding emitted'});continue
  if adapter_name and sha(testdir/adapter_name) != REVIEWED_ADAPTER_HASHES[adapter_name]:
   problems.append({'target':target,'case':case,'error':'route helper changed after review; review required before binding'});continue
  fixture=root/'fixtures/custom_mechanics'/fixture_name
  if not fixture.is_file():problems.append({'target':target,'case':case,'error':'reviewed fixture absent'});continue
  fixture_data=json.loads(fixture.read_text());rows=fixture_data['mechanics'];rows=rows[:1] if selector.get('first_row') else rows
  emitted=0
  for row in rows:
   if selector.get('keyword') and row.get('keyword')!=selector['keyword']:continue
   if selector.get('rows') and row['id'] not in selector['rows']:continue
   cards=row.get('representative_cards',[]);cards=cards[:1] if selector.get('first') else cards
   for card in cards:
    if selector.get('names') and card.get('name') not in selector['names']:continue
    if selector.get('rewrite_status') and card.get('rewrite_status')!=selector['rewrite_status']:continue
    raw=card.get('source_card_key') or card.get('id');faces=card.get('source_faces') or [{'name':card.get('name')}]; executable=card.get('executable_faces') or ([{'name':card.get('name'),'executable_text':card.get('executable_text')}] if card.get('executable_text') else [])
    full=bool(executable) and [f.get('name') for f in executable]==[f.get('name') for f in faces] and all(f.get('executable_text') for f in executable)
    if selector.get('root_text_only') and len(faces)!=1:
     problems.append({'target':target,'case':case,'row':row['id'],'source_key':raw,'error':'case compiles representative text only, not every linked face; no full-source binding emitted'});continue
    if not raw or not full:problems.append({'target':target,'case':case,'row':row['id'],'source_key':raw,'error':'exact physical identity or complete executable faces absent; no full-source binding emitted'});continue
    namespace='revolution' if row.get('scope') in ['rev','revolution'] else row.get('scope');key=raw if raw.startswith(('msem:','rev:','revolution:')) else namespace+':'+raw
    binding={'source_inventory_id':row.get('source_inventory_id',row['id']),'source_key':key,'faces':[f['name'] for f in faces],'target':target,'cases':[case],'fixture':str(fixture.relative_to(root)),'fixture_sha256':sha(fixture),'candidate_card_sha256':card_sha(card),'test_file_sha256':audit['test_file_sha256'],'additional_inputs_sha256':{str((testdir/ref).resolve().relative_to(root)):sha((testdir/ref).resolve()) for ref in audit['literal_fixture_references'] if (testdir/ref).is_file()},'kind':kind,'routes':routes,'independent_native_artifact':set(routes)=={'native','artifact'},'review_reference':'source-case-bindings/reviewed-profiles-v1: '+target+'::'+case,'strict_word_claims':strict,'parse_loss_free':True,'complete_source_faces':True,'execution_status':'unrun_no_native_receipt_supplied'}
    if adapter_name:
     adapter=testdir/adapter_name;binding.update(route_adapter=str(adapter.relative_to(root)),route_adapter_sha256=sha(adapter))
    bindings.append(binding);emitted+=1
  caseaudit.update(classification=(('full_source_strict_parser' if strict else 'full_source_lossless_parser_word_claims_unasserted') if kind=='strict_full_source_compile' else 'actual_full_source_gameplay'),source_bindings=emitted,routes=routes,independent_native_artifact=set(routes)=={'native','artifact'},strict_word_claims_asserted=strict)
  if routes==['artifact']:caseaudit['route_note']='The examined adapter uses artifact/wire materialization for both returned definitions; a second JSON materialization is not an independent native adapter.'
 # No fixture literals in these reviewed targets: explicit authored/synthetic
 # controls of generic primitives, not source-card certifications.
 representative={'custom_named_operation_completion','custom_named_operation_replacements','custom_named_action_participants','custom_zero_named_completion','custom_colored_resource_costs','custom_compiled_resource_bindings','custom_collect_partition_replacement','custom_scaled_selection_preflight','custom_counter_copy_dependencies','custom_named_operation_expansion_bindings','custom_resource_binding_preview','custom_external_trigger_classes','custom_trigger_class_choice','custom_granted_parameter_identity'}
 for target in representative & audits.keys():
  for case in audits[target]['cases']:
   if case['classification']=='unknown_not_yet_source_reviewed':case['classification']='representative_mechanical_or_composition_control'
 observations=historical_observations(root,a.observed_results)
 for audit in audits.values():
  for case in audit['cases']:
   case['historical_case_observations']=[{**observation,'case_status':observation['case_results'][case['case']]} for observation in observations if observation['target']==audit['target'] and case['case'] in observation['case_results']]
 for binding in bindings:
  binding['historical_case_observations']=[{'descriptor':observation['captured_result_descriptor'],'log':observation['log'],'log_sha256':observation['log_sha256'],'cases':{case:observation['case_results'].get(case,'unreported') for case in binding['cases']},'source_input_identity_verified':False,'current_native_pass':False} for observation in observations if observation['target']==binding['target']]
 (out/'historical-native-observations.json').write_text(json.dumps({'observations':observations,'current_native_pass_claims':0},indent=2)+'\n')
 result={'schema_version':1,'report_only':True,'bindings':bindings,'validation_problems':problems,'targets':list(audits.values()),'summary':{'targets':len(audits),'bindings':len(bindings),'source_parser_bindings':sum(b['kind']=='strict_full_source_compile' for b in bindings),'strict_word_parser_bindings':sum(b['kind']=='strict_full_source_compile' and b['strict_word_claims'] for b in bindings),'gameplay_bindings':sum(b['kind']=='gameplay' for b in bindings),'independent_route_bindings':sum(b['independent_native_artifact'] for b in bindings),'native_pass_claims':0}}
 templates=[]
 for target,audit in audits.items():
  inputs={audit['test_file']:audit['test_file_sha256']} if audit['test_file_sha256'] else {}
  for b in bindings:
   if b['target']!=target:continue
   inputs[b['fixture']]=b['fixture_sha256'];inputs.update(b.get('additional_inputs_sha256',{}))
   if b.get('route_adapter'):inputs[b['route_adapter']]=b['route_adapter_sha256']
  templates.append({'target':target,'inputs_sha256':inputs,'execution_status':'not_run_template_only'})
 (out/'run-inputs-template.json').write_text(json.dumps({'templates':templates},indent=2)+'\n')
 (out/'source-case-bindings.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
 print(json.dumps(result['summary']));print('validation problems',len(problems))
if __name__=='__main__':main()
