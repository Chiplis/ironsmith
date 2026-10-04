#!/usr/bin/env python3
"""Bind paid canonical prepare-trigger observations to frozen artifacts and real linkage."""
import collections,hashlib,json,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit';prefix='prepare-trigger'
read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def write(n,d):(p/n).write_text(json.dumps(d,indent=2)+'\n')
def strip(x):
 if isinstance(x,list):return[strip(v)for v in x]
 if isinstance(x,dict):return{k:strip(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
def diff(a,b,path=''):
 if isinstance(a,dict)and isinstance(b,dict):return sum((diff(a.get(k),b.get(k),path+'/'+k)for k in sorted(a.keys()|b.keys())),[])
 return []if a==b else[dict(path=path,before=a,after=b)]
d=read(prefix+'-reproductions.json');proc=read(prefix+'-process.json');assert proc['exit_code']==0 and proc['binary_sha256_before']==proc['binary_sha256_after']==d['provenance']['binary_sha256'];assert sha(root/'crates/ironsmith-tools/tests/runtime_prepare_trigger_reproductions.rs')==d['provenance']['source_sha256'];assert sha(root/'crates/ironsmith-tools/tests/support/canonical_linked_fixture.rs')==d['provenance']['helper_sha256']
artifacts={a['card']:a for a in read(prefix+'-artifacts.json')};linked={};changes=[];oracle_differences=[]
for f in d['linkage_families']:
 for i,(before,after)in enumerate(zip(f['unlinked_artifacts'],f['linked_artifacts'])):
  face=f['metadata']['canonical_faces'][i]
  if before['input']['oracle_text']!=face['oracle_text']:
   assert before['card']=='Replenish' and before['input']['oracle_text']==face['oracle_text']+' (Auras with nothing to enchant remain in your graveyard.)'
   oracle_differences.append(dict(card=before['card'],frozen_full_oracle=before['input']['oracle_text'],catalog_face_oracle=face['oracle_text'],difference='Frozen ordinary printing includes the Aura reminder sentence omitted by the catalog linked face. Rules text and printed mana cost agree. The unchanged full frozen definition is used; only cast completion is observed, not this spell resolution.'))
  if before['card']in artifacts:assert strip(before['definition'])==strip(artifacts[before['card']]['definition'])
  artifacts[before['card']]=before;linked[before['card']]=after
  ds=diff(before['definition'],after['definition']);assert {v['path']for v in ds}=={'/card/other_face','/card/other_face_name','/card/linked_face_layout'}
  changes.append(dict(card=before['card'],combined_group=f['metadata']['combined_name'],strict_prelink_checksum=before['artifact_checksum'],linked_execution_checksum=after['artifact_checksum'],changes=ds))
parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{p/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for a in artifacts.values():
   result=db.execute('select result_json from result where run_id=? and card_name=?',(run,a['card'])).fetchone();assert result,a['card'];f=json.loads(result[0]);equal=strip(a['definition'])==strip(f['definition']);assert equal,a['card'];parity.append(dict(card=a['card'],corpus=corpus,run_id=run,strict_prelink_checksum=a['artifact_checksum'],frozen_checksum=f['artifact_checksum'],definition_equal_ignoring_only_card_ids=equal))
write(prefix+'-parity.json',dict(scope='All strict pre-link definitions equal both frozen corpora ignoring only card-definition IDs. Linked artifacts differ by exactly the three enumerated canonical prepare metadata fields. All face rules text agrees. Replenish has a separately enumerated printing reminder difference; its actual paid cast completes before any effect resolution is observed.',rows=parity,metadata_deltas=changes,catalog_oracle_differences=oracle_differences))
counts=collections.Counter(r['status']for r in d['rows'])
assert counts==dict(expected_result_observed=80,semantic_mismatch=44,linked_face_compile_failed=6),counts
names=sorted({r['scenario']['subject']for r in d['rows']if r['status']=='semantic_mismatch'})
cast_rows=[];gate_rows=[]
for i,r in enumerate(d['rows']):
 if r['status']!='linked_face_compile_failed':
  r['strict_frozen_prelink_artifact_checksum']=r['artifact_checksum']
  r['linked_execution_artifact_checksum']=linked[r['scenario']['subject']]['artifact_checksum']
 if r['status']!='semantic_mismatch':continue
 changed={k for k in r['expected']if r['expected'][k]!=r['actual'].get(k)}
 if changed=={'prepared_after_announcement'}:cast_rows.append(i)
 else:
  assert r['scenario']['subject']=='Naktamun Lorespinner'and changed=={'copy_count','prepared_after_producer'},r
  hands=[t['hands']for t in r['execution_trace']if t['stage']=='actual_phase_event'and t['active']==0 and t['step']=='Some(Upkeep)']
  assert hands==[[0,1,1]],hands
  gate_rows.append(i)
assert len(cast_rows)==40 and len(gate_rows)==4
# The same source and alias each also have an actual all-players-over-one negative.
for r in d['rows']:
 if r['scenario']['subject']=='Naktamun Lorespinner'and r['scenario']['mode']=='no_prepare':
  hands=[t['hands']for t in r['execution_trace']if t['stage']=='actual_phase_event'and t['active']==0 and t['step']=='Some(Upkeep)']
  assert hands==[[2,3,3]]and r['status']=='expected_result_observed',hands
cast_reason='The exact canonical source and combined alias were separately cast for their printed mana cost and became prepared through the recorded real gameplay producer. Hold and applicable negative-producer controls accompany each family. The actual linked spell was normally announced with legal targets, X=1 where printed, and its mana cost paid (including Dirgur Focusmage\'s printed reduction). At paid cast completion the spell is on stack and its copy linkage is consumed, but its source remains prepared. Only this cast-completion invariant is claimed, before linked-spell resolution. clear_exile_state removes the copy-to-source mapping on exile-to-stack movement before priority_mana calls unprepare_for_cast(old copy ID), leaving the prepared flag stale.'
gate_reason='The fully paid canonical source reaches its actual next upkeep through normal turn advancement and generated phase events. Recorded hands are Alice0/Bob1/Cara1, so a player has one or fewer cards and the source should become prepared. It stays unprepared and creates no linked spell. In the negative control all hands are greater than1 (2/3/3), and no preparation correctly occurs. The strict intervening-if is PlayerCardsInHandOrFewer{player:Any,count:1}. condition_eval uses the singular resolve_player path; external resolution maps Any to None and returns false instead of testing whether any player meets the threshold. Prepared-spell casting is unreached in these rows.'
excluded=[dict(path='reports/runtime-audit/excluded-pilots/prepare-trigger-initial-phase-ordinal/validity.json',reason='Initial first-main fixture omitted the started-main-phase ordinal. The pilot falsely missed Scheming Silvertongue\'s second-main trigger. All130 pilot observations are excluded, with original raw/input/source/helper/executable preserved before the corrected full replay.')]
d.update(summary=dict(scenarios=len(d['rows']),status_counts=dict(counts),trigger_groups=22,confirmed_primary_cards=21,paid_cast_unprepare_primary_cards=20,earlier_condition_gate_primary_cards=1,primary_and_alias_payloads=44,controls=80,paid_cast_failures=40,earlier_condition_failures=4,linked_spell_compile_failures=6,strict_prelink_definitions=len(artifacts)),confirmed_cards=names,parity_report=f'reports/runtime-audit/{prefix}-parity.json',process_report=f'reports/runtime-audit/{prefix}-process.json',reviewed_scope='All22 frozen Triggered-only prepare source groups and their aliases are accounted.20 groups reach actual paid prepared-spell casts and exhibit stale prepared state. Naktamun fails its Any-player hand condition before preparation; its cast consumer is not reached. Yavimaya Bloomsage\'s actual linked Channel spell fails strict compilation, so its six scenarios stop before gameplay and it is not promoted.80 controls verify scoped hold/negative outcomes. Inspired Skypainter uses its ETB branch only; its token-combat branch is not separately certified. Leech Collector uses the first own-life-gain event and an other-player negative; its second-life-gain suppression is not certified. Other unexercised timing combinations and linked-spell resolution remain outside this report. No injected prepared state or forced missing action.',excluded_pilots=excluded)
write(prefix+'-reproductions.json',d)
ref=dict(path=f'reports/runtime-audit/{prefix}-reproductions.json',sha256=sha(p/(prefix+'-reproductions.json')))
findings=[]
for i in cast_rows+gate_rows:
 r=d['rows'][i];cast=i in cast_rows
 findings.append(dict(card_name=r['scenario']['subject'],payload_name=r['card'],confirmed_cards=[r['scenario']['subject']],classification='runtime_defect_card_reproduced',outcome_category='silent_wrong_result',failure_stage='prepared_spell_cast_completion'if cast else'upkeep_intervening_condition',defect_subtype='prepared_copy_mapping_removed_before_unprepare'if cast else'any_player_hand_threshold_singular_resolver',reason=cast_reason if cast else gate_reason,scenario=r['scenario'],expected=r['expected'],observed=r['actual'],source_report=ref,source_row=i,artifact_checksum=r['artifact_checksum'],linked_execution_artifact_checksum=r['linked_execution_artifact_checksum']))
code_paths=['crates/ironsmith-engine/src/condition_eval.rs','crates/ironsmith-engine/src/condition_eval/context.rs','crates/ironsmith-engine/src/triggers/check.rs','crates/ironsmith-engine/src/triggers/phase_step/beginning_of_main_phase.rs','crates/ironsmith-engine/src/game_state/object_state_and_events.rs','crates/ironsmith-engine/src/game_loop/priority_mana.rs']
write(prefix+'-reviewed-attribution.json',dict(confirmed_cards=names,summary=d['summary'],findings=findings,source_report=ref,reasons=dict(paid_cast=cast_reason,earlier_condition=gate_reason),reviewed_scope=d['reviewed_scope'],parity_report=d['parity_report'],process_report=d['process_report'],limitations=d['limitations'],excluded_pilots=excluded,catalog_oracle_differences=oracle_differences,reviewed_runtime_sources=[dict(path=f,sha256=sha(root/f))for f in code_paths]))
inv=read('prepare-face-candidates.json');entry=read('prepare-entry-reproductions.json');entry_ref=read('prepare-entry-reviewed-attribution.json')['source_report'];old=read('linked-face-cost-reviewed-attribution.json');ledger=[]
for group in inv['rows']:
 ix=[i for i,r in enumerate(d['rows'])if r['scenario']['group']==group['combined_name']]
 ex=[i for i,r in enumerate(entry['rows'])if r['scenario']['group']==group['combined_name']]
 if ix:
  if all(d['rows'][i]['status']=='linked_face_compile_failed'for i in ix):status='linked_spell_compile_failed'
  elif group['front']=='Naktamun Lorespinner':status='preparation_condition_failure_cast_unreached'
  else:status='paid_cast_unprepare_failure'
  evidence=dict(source_report=ref,source_rows=ix)
 elif ex:
  status='linked_spell_compile_failed'if all(entry['rows'][i]['status']=='linked_face_compile_failed'for i in ex)else'paid_cast_unprepare_failure'
  evidence=dict(source_report=entry_ref,source_rows=ex)
 elif group['front']=='Harmonized Trio':
  status='paid_cast_unprepare_failure';evidence=dict(source_report=old['source_report'],source_rows=[i for i,r in enumerate(read('linked-face-cost-reproductions.json')['rows'])if r['card'].startswith('Harmonized Trio')])
 elif group['payloads'][0]['status']=='compile_failed':status='frozen_front_compile_failed';evidence=None
 else:raise AssertionError(group['front'])
 ledger.append(dict(combined_name=group['combined_name'],front=group['front'],spell_face=group['spell_face'],typed_prepare_routes=group['payloads'][0]['typed_prepare_routes'],status=status,evidence=evidence,scope='At least one normal canonical preparation route and actual paid copy cast where preparation is reached. Other producer branches or repeated preparation are not globally certified.'))
write('prepare-face-coverage.json',dict(scope='All67 canonical prepare groups explicitly accounted.56 have individual actual paid-cast unprepare failures, one has an earlier actual upkeep condition failure, three stop at linked-spell compilation and seven at frozen-front compilation. Aliases never add a primary name. This is not a whole-mechanic pass certificate or blanket family promotion.',summary=dict(groups=len(ledger),statuses=dict(collections.Counter(r['status']for r in ledger))),rows=ledger))
(p/(prefix+'-reproductions.md')).write_text('# Trigger preparation cast-completion audit\n\n130 cases:80 scoped controls,40 wrong unprepare outcomes,4 earlier Naktamun condition failures and6 linked-spell compilation gates.21 independently executed primary cards confirmed; aliases count once.\n\n'+cast_reason+'\n\n'+gate_reason+'\n\n'+d['reviewed_scope']+'\n\n'+f'{len(artifacts)} strict pre-link definitions match both frozen corpora ({len(parity)} comparisons). Each linked face differs only by its three enumerated real catalog linkage fields. Corrected full run completed within60seconds. The excluded initial phase-ordinal pilot, its source and executable are preserved separately.\n')
print(d['summary']);print(collections.Counter(r['status']for r in ledger))
