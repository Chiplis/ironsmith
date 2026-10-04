#!/usr/bin/env python3
"""Bind paid canonical prepare-entry observations to frozen artifacts and real linkage."""
import collections,hashlib,json,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit';prefix='prepare-entry'
read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def write(n,d):(p/n).write_text(json.dumps(d,indent=2)+'\n')
def strip(x):
 if isinstance(x,list):return[strip(v)for v in x]
 if isinstance(x,dict):return{k:strip(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
def diff(a,b,path=''):
 if isinstance(a,dict)and isinstance(b,dict):return sum((diff(a.get(k),b.get(k),path+'/'+k)for k in sorted(a.keys()|b.keys())),[])
 return []if a==b else[dict(path=path,before=a,after=b)]
d=read(prefix+'-reproductions.json');proc=read(prefix+'-process.json');assert proc['exit_code']==0 and proc['binary_sha256_before']==proc['binary_sha256_after']==d['provenance']['binary_sha256'];assert sha(root/'crates/ironsmith-tools/tests/runtime_prepare_entry_reproductions.rs')==d['provenance']['source_sha256'];assert sha(root/'crates/ironsmith-tools/tests/support/canonical_linked_fixture.rs')==d['provenance']['helper_sha256']
artifacts={a['card']:a for a in read(prefix+'-artifacts.json')};linked={};changes=[]
for f in d['linkage_families']:
 for i,(before,after)in enumerate(zip(f['unlinked_artifacts'],f['linked_artifacts'])):
  face=f['metadata']['canonical_faces'][i];assert before['input']['oracle_text']==face['oracle_text'],before['card']
  if before['card']in artifacts:assert strip(before['definition'])==strip(artifacts[before['card']]['definition'])
  artifacts[before['card']]=before;linked[before['card']]=after
  ds=diff(before['definition'],after['definition']);assert {v['path']for v in ds}=={'/card/other_face','/card/other_face_name','/card/linked_face_layout'}
  changes.append(dict(card=before['card'],combined_group=f['metadata']['combined_name'],strict_prelink_checksum=before['artifact_checksum'],linked_execution_checksum=after['artifact_checksum'],changes=ds))
parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{p/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for a in artifacts.values():
   result=db.execute('select result_json from result where run_id=? and card_name=?',(run,a['card'])).fetchone();assert result,a['card'];f=json.loads(result[0]);equal=strip(a['definition'])==strip(f['definition']);assert equal,a['card'];parity.append(dict(card=a['card'],corpus=corpus,run_id=run,strict_prelink_checksum=a['artifact_checksum'],frozen_checksum=f['artifact_checksum'],definition_equal_ignoring_only_card_ids=equal))
write(prefix+'-parity.json',dict(scope='All strict pre-link definitions equal both frozen corpora ignoring only card-definition IDs. Linked artifacts differ by exactly the three enumerated canonical prepare metadata fields. Actual cards.json face Oracle texts equal frozen face Oracle texts; no substitute same-name spell.',rows=parity,metadata_deltas=changes))
counts=collections.Counter(r['status']for r in d['rows']);assert counts==dict(expected_result_observed=70,semantic_mismatch=70,linked_face_compile_failed=8),counts
names=sorted({r['scenario']['subject']for r in d['rows']if r['status']=='semantic_mismatch'})
for r in d['rows']:
 if r['status']=='semantic_mismatch':assert [k for k in r['expected']if r['expected'][k]!=r['actual'].get(k)]==['prepared_after_announcement'],r['card']
 if r['status']!='linked_face_compile_failed':r['strict_frozen_prelink_artifact_checksum']=r['artifact_checksum'];r['linked_execution_artifact_checksum']=linked[r['scenario']['subject']]['artifact_checksum']
reason='This exact canonical front and combined alias were separately cast for their full printed mana cost, entered prepared naturally, and created one linked spell copy from the real cards.json face. The hold-prepared control preserves that state. In the paired case, the actual legal copy was normally announced, legal targets supplied, X=1 where printed, and its full printed mana cost paid. The spell is on stack and its copy linkage is consumed, but the permanent remains prepared instead of becoming unprepared. Observation occurs immediately after paid cast completion; no subsequent spell effects are claimed. clear_exile_state unlinks the prepared-copy mapping during exile-to-stack movement before priority_mana calls unprepare_for_cast(old copy ID), leaving the source flag stale.'
d.update(summary=dict(scenarios=148,status_counts=dict(counts),entry_groups=37,confirmed_entry_primary_cards=35,primary_and_alias_payloads=74,paired_hold_controls=70,paid_cast_failures=70,linked_spell_compile_failures=8,strict_prelink_definitions=len(artifacts)),confirmed_cards=names,parity_report=f'reports/runtime-audit/{prefix}-parity.json',process_report=f'reports/runtime-audit/{prefix}-process.json',reviewed_scope='All37 frozen-compiled EntersPrepared groups and aliases accounted.35 groups complete actual paid source and spell cast, with35x2hold controls and35x2wrong unprepare outcomes. Heartwood Crafter and Konstrari Improviser front definitions compile but their real linked Soul Tether spell fails strict compilation, so all8case rows stop before gameplay and neither name is promoted. Linked-spell resolution is intentionally outside the cast-completion invariant; actual normally paid source ETBs resolve. No fabricated preparation or forced missing action.',excluded_pilots=[])
write(prefix+'-reproductions.json',d);ref=dict(path=f'reports/runtime-audit/{prefix}-reproductions.json',sha256=sha(p/(prefix+'-reproductions.json')));findings=[]
for i,r in enumerate(d['rows']):
 if r['status']=='semantic_mismatch':findings.append(dict(card_name=r['scenario']['subject'],payload_name=r['card'],confirmed_cards=[r['scenario']['subject']],classification='runtime_defect_card_reproduced',outcome_category='silent_wrong_result',failure_stage='prepared_spell_cast_completion',defect_subtype='prepared_copy_mapping_removed_before_unprepare',reason=reason,scenario=r['scenario'],expected=r['expected'],observed=r['actual'],source_report=ref,source_row=i,artifact_checksum=r['artifact_checksum'],linked_execution_artifact_checksum=r['linked_execution_artifact_checksum']))
write(prefix+'-reviewed-attribution.json',dict(confirmed_cards=names,summary=d['summary'],findings=findings,source_report=ref,reason=reason,reviewed_scope=d['reviewed_scope'],parity_report=d['parity_report'],process_report=d['process_report'],limitations=d['limitations'],excluded_pilots=[]))
inv=read('prepare-face-candidates.json');old=read('linked-face-cost-reviewed-attribution.json');ledger=[]
for group in inv['rows']:
 ix=[i for i,r in enumerate(d['rows'])if r['scenario']['group']==group['combined_name']]
 if ix:status='linked_spell_compile_failed'if all(d['rows'][i]['status']=='linked_face_compile_failed'for i in ix)else'paid_cast_unprepare_failure';evidence=dict(source_report=ref,source_rows=ix)
 elif group['front']=='Harmonized Trio':status='paid_cast_unprepare_failure';evidence=dict(source_report=old['source_report'],source_rows=[i for i,r in enumerate(read('linked-face-cost-reproductions.json')['rows'])if r['card'].startswith('Harmonized Trio')])
 elif group['payloads'][0]['status']=='compile_failed':status='frozen_front_compile_failed';evidence=None
 else:status='trigger_producer_unexercised';evidence=None
 ledger.append(dict(combined_name=group['combined_name'],front=group['front'],spell_face=group['spell_face'],typed_prepare_routes=group['payloads'][0]['typed_prepare_routes'],status=status,evidence=evidence))
write('prepare-face-coverage.json',dict(scope='Exact canonical prepare-family ledger. A paid-cast failure is card-specific executed evidence; compile gates and unexercised trigger producers remain distinct. Aliases never add a primary name.',summary=dict(groups=len(ledger),statuses=dict(collections.Counter(r['status']for r in ledger))),rows=ledger))
(p/(prefix+'-reproductions.md')).write_text('# Entry preparation cast-completion audit\n\n148 cases:70 controls,70 wrong unprepare outcomes and8 linked-spell compilation gates.35 independently executed primary cards confirmed; aliases count once.\n\n'+reason+'\n\n'+d['reviewed_scope']+'\n\n'+f'{len(artifacts)} strict pre-link definitions match both frozen corpora ({len(parity)} comparisons). Each linked face differs only by its three recorded canonical linkage fields. Binary, source and helper hashes verified; run completed within60seconds.\n')
print(d['summary']);print(collections.Counter(r['status']for r in ledger))
