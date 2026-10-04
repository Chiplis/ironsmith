#!/usr/bin/env python3
"""Review canonical face linkage without merging linkage fields into frozen parity."""
import collections,hashlib,json,sqlite3
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];P=ROOT/'reports/runtime-audit';PREFIX='linked-face-cost'
load=lambda n:json.loads((P/n).read_text()); sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def write(n,d): (P/n).write_text(json.dumps(d,indent=2)+'\n')
def strip_ids(x):
 if isinstance(x,list):return [strip_ids(v)for v in x]
 if isinstance(x,dict):return{k:strip_ids(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
def differences(a,b,path=''):
 if isinstance(a,dict)and isinstance(b,dict):return sum((differences(a.get(k),b.get(k),path+'/'+k)for k in sorted(a.keys()|b.keys())),[])
 if a==b:return[]
 return[dict(path=path,before=a,after=b)]
d=load(PREFIX+'-reproductions.json');process=load(PREFIX+'-process.json');assert process['exit_code']==0 and process['binary_sha256_before']==process['binary_sha256_after']==d['provenance']['binary_sha256']
assert sha(ROOT/'crates/ironsmith-tools/tests/runtime_linked_face_cost_reproductions.rs')==d['provenance']['source_sha256'];assert sha(ROOT/'crates/ironsmith-tools/tests/support/canonical_linked_fixture.rs')==d['provenance']['helper_sha256']
artifacts=load(PREFIX+'-artifacts.json');all_artifacts=artifacts[:];deltas=[];linked={}
for f in d['linkage_families']:
 all_artifacts.extend(f['unlinked_artifacts'])
 for base,actual in zip(f['unlinked_artifacts'],f['linked_artifacts']):
  diff=differences(base['definition'],actual['definition']);assert all(v['path']in ['/card/other_face','/card/other_face_name','/card/linked_face_layout','/card/transforming_dfc']for v in diff),diff
  assert len(diff)==(4 if f['metadata']['layout']=='transform'else 3),diff
  deltas.append(dict(card=base['card'],canonical_group=f['metadata']['combined_name'],strict_prelink_checksum=base['artifact_checksum'],linked_execution_checksum=actual['artifact_checksum'],changes=diff,artifact_identity=actual['artifact']['card']))
  linked[base['card']]=actual
unique={}
for a in all_artifacts:
 if a['card']in unique:assert strip_ids(a['definition'])==strip_ids(unique[a['card']]['definition'])
 unique[a['card']]=a
parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{P/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for a in unique.values():
   frozen=json.loads(db.execute('select result_json from result where run_id=? and card_name=?',(run,a['card'])).fetchone()[0]);same=strip_ids(a['definition'])==strip_ids(frozen['definition']);assert same,a['card'];parity.append(dict(card=a['card'],corpus=corpus,run_id=run,strict_prelink_checksum=a['artifact_checksum'],frozen_checksum=frozen['artifact_checksum'],equal_ignoring_only_card_ids=same))
for alias,front in [('Lluwen, Exchange Student // Pest Friend','Lluwen, Exchange Student'),('Harmonized Trio // Brainstorm','Harmonized Trio')]:assert strip_ids(unique[alias]['definition'])==strip_ids(unique[front]['definition'])
write(PREFIX+'-parity.json',dict(scope='Strict pre-link artifact definitions compared unchanged against both frozen corpora; only card-definition IDs ignored. Linked execution artifacts intentionally differ in exactly the enumerated catalog metadata fields; linked equality is NOT claimed against frozen unlinked definitions.',rows=parity,metadata_deltas=deltas))
reason='Real canonical prepare-layout faces were linked from cards.json, artifacts validated/materialized, and both faces registered in GameState through the production linked-face cache. Lluwen enters prepared and creates its Pest Friend copy; Harmonized Trio becomes prepared after actual full paid tap costs. Paid exiled copies of Pest Friend and Brainstorm cast and resolve correctly, but the source remains prepared. The exile-to-stack zone change calls clear_exile_state, which unlinks the reverse prepared-copy mapping without clearing the permanent flag. Later priority_mana calls unprepare_for_cast(old copy ID), whose lookup is now missing. In the real follow-up Lluwen activation, actual creature exile cost pays but no new spell copy is created because set_prepared sees the stale flag. No state or prepared flag was injected.'
short_reason='With only one other owned untapped creature, Harmonized Trio incorrectly advertises its T plus tap-two-creatures action: preflight counts the source as available for the second component before reserving it for T. Normal announcement then rejects with Not enough objects to choose (2 needed,1 available). Costs roll back (source and resource untapped). This is an announcement/preflight failure, separate from prepared-spell unprepare behavior.'
counts=collections.Counter(r['status']for r in d['rows']);assert counts==dict(semantic_mismatch=24,expected_result_observed=15),counts
for row in d['rows']:
 primary=row['card'].split(' // ')[0];row['strict_frozen_prelink_artifact_checksum']=row['artifact_checksum'];row['linked_execution_artifact_checksum']=linked[primary]['artifact_checksum']
d.update(summary=dict(scenarios=39,status_counts=dict(counts),reviewed_unprepare_failures=22,reviewed_announcement_exceptions=2,scoped_controls=15,canonical_prelink_definitions=len(unique),face_groups=3,primary_confirmed_cards=2),confirmed_cards=['Harmonized Trio','Lluwen, Exchange Student'],parity_report=f'reports/runtime-audit/{PREFIX}-parity.json',process_report=f'reports/runtime-audit/{PREFIX}-process.json',reviewed_scope='Bladewheel seven exact-path scenarios pass through actual paid Sawblades2 and craft4 with an actual battlefield artifact: no/one/two/three artifacts, opponent/type and tapped negatives; successful source becomes a5/5 creature and two selected others tap. Harmonized controls five negatives per payload, except the one-creature preflight error; exact/surplus costs and Brainstorm effect succeed before unprepare flag mismatch. Fourteen Lluwen rows stop at the same initial paid Pest cast mismatch before their labeled later cost-resource scenario; two already-prepared rows pay cost and cast, and two explicit follow-up rows prove stale flag prevents reprepare copy. Earlier unlinked reports remain unchanged metadata-limited evidence. No whole prepare-family claim.',excluded_preliminary_report=f'reports/runtime-audit/{PREFIX}-preliminary-before-followups.json')
write(PREFIX+'-reproductions.json',d);ref=dict(path=f'reports/runtime-audit/{PREFIX}-reproductions.json',sha256=sha(P/(PREFIX+'-reproductions.json')));findings=[]
for i,row in enumerate(d['rows']):
 if row['status']!='semantic_mismatch':continue
 short=row['scenario']['mode']=='one_creature';primary=row['card'].split(' // ')[0]
 observed=row['actual']
 if short:observed=dict(observed,normal_announcement=next(t for t in row['execution_trace']if t['stage']=='insufficient_cost_attempt'))
 findings.append(dict(card_name=primary,payload_name=row['card'],confirmed_cards=[primary],classification='runtime_defect_card_reproduced',outcome_category='runtime_exception'if short else'silent_wrong_result',failure_stage='advertised_activation_cost_choice'if short else'prepared_spell_cast_completion',defect_subtype='sequential_tap_source_not_reserved_in_preflight'if short else'prepared_copy_mapping_removed_before_unprepare',reason=short_reason if short else reason,scenario=row['scenario'],expected=row['expected'],observed=observed,source_report=ref,source_row=i,artifact_checksum=row['artifact_checksum'],linked_execution_artifact_checksum=row['linked_execution_artifact_checksum']))
choose=load('choose-consume-cost-candidates.json')['rows'];path_coverage=[]
for n in ['Bladewheel Chariot','Harmonized Trio','Harmonized Trio // Brainstorm','Lluwen, Exchange Student','Lluwen, Exchange Student // Pest Friend']:
 candidates=[r for r in choose if r['card']==n];ix=[i for i,r in enumerate(d['rows'])if r['card']==n]
 for c in candidates:
  relevant=ix if not n.startswith('Lluwen')else[i for i in ix if d['rows'][i]['scenario']['mode']in ['already_prepared','reprepare_after_cast']]
  path_coverage.append(dict(payload_name=n,cost_path=c['path'],source_rows=relevant,source_report=ref,ability_index=1 if n.startswith('Lluwen')else 0,coverage_status='linked_source_cost_and_consumer_controls'if n=='Bladewheel Chariot'else'linked_cost_pass_prepared_cast_failure',scope=d['reviewed_scope']))
write(PREFIX+'-reviewed-attribution.json',dict(confirmed_cards=d['confirmed_cards'],summary=d['summary'],findings=findings,path_coverage=path_coverage,reason=reason,secondary_reason=short_reason,source_report=ref,parity_report=d['parity_report'],process_report=d['process_report'],limitations=d['limitations'],reviewed_scope=d['reviewed_scope'],excluded_preliminary_report=d['excluded_preliminary_report']))
write('linked-face-materialization-route.json',dict(scope='Read-only production route investigation; no engine behavior changes.',canonical_helper='crates/ironsmith-tools/tests/support/canonical_linked_fixture.rs',validated_groups=[f['metadata']['combined_name']for f in d['linkage_families']],parity_report=d['parity_report'],routes=[dict(path='crates/ironsmith-tools/src/tooling.rs',finding='load_card_payloads_by_name/compile_runtime_definition_from_payload decorates transform/split/flip; prepare missing from linked_face_layout_from_card.'),dict(path='crates/ironsmith-compiler-wasm/src/lib.rs',finding='compile_artifact accepts otherFaceId/name, linkedFaceLayout prepare, transform marker; both typed payload and artifact identity carry linkage.'),dict(path='crates/ironsmith-wasm/src/wasm_game_impl/external_registry.rs',finding='materialize_compiled_artifact_batch allocates fresh runtime IDs and remaps paired local IDs, then cache registers linked definitions.'),dict(path='crates/ironsmith-engine/src/game_state.rs',finding='register_linked_face_definition and register_linked_face_family_from_catalog are public production registration APIs used by runtime lookups.')],unverified_candidates=[dict(kind='wasm_prepare_batch_cardinality',detail='external_source_definition_names returns only prepare front, but register_compiled_card_source_artifacts requires artifacts.len equal that one name while a linked batch needs paired face artifacts. Not exercised here; no card or runtime defect promoted for this binding boundary.')]))
(P/(PREFIX+'-reproductions.md')).write_text('# Canonical linked-face cost execution\n\n39 scenarios:15 scoped controls,22 prepared-cast wrong-state observations and2 announced-cost exceptions. Two primary cards confirmed; aliases count once.\n\n'+reason+'\n\n'+short_reason+'\n\n'+d['reviewed_scope']+'\n\n'+f'{len(unique)} strict pre-link definitions match both frozen corpora ({len(parity)} comparisons); exactly four metadata fields for transform and three for prepare change in six actual linked faces. Binary/source/helper hashes verified. Native artifact materialization/cache registration exercised; JS/WASM binding not replayed.\n')
print(d['summary'])
