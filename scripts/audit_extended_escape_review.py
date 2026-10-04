#!/usr/bin/env python3
"""Review strict canonical Escape pair payments and independent outcomes."""
import collections,hashlib,json,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit';prefix='extended-escape'
read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def write(n,d):(p/n).write_text(json.dumps(d,indent=2)+'\n')
def strip(x):
 if isinstance(x,list):return[strip(v)for v in x]
 if isinstance(x,dict):return{k:strip(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
d=read(prefix+'-reproductions.json');proc=read(prefix+'-process.json');assert proc['exit_code']==0 and proc['binary_sha256_before']==proc['binary_sha256_after']==d['provenance']['binary_sha256'];assert sha(root/'crates/ironsmith-tools/tests/runtime_extended_escape_reproductions.rs')==d['provenance']['source_sha256']
artifacts=read(prefix+'-artifacts.json');parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{p/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for a in artifacts:
   result=db.execute('select result_json from result where run_id=? and card_name=?',(run,a['card'])).fetchone();assert result,a['card'];f=json.loads(result[0]);equal=strip(a['definition'])==strip(f['definition']);assert equal,a['card'];parity.append(dict(card=a['card'],corpus=corpus,run_id=run,artifact_checksum=a['artifact_checksum'],frozen_checksum=f['artifact_checksum'],definition_equal_ignoring_only_card_ids=equal))
write(prefix+'-parity.json',dict(scope='Full strict canonical definitions match both frozen corpora ignoring only definition card IDs. No metadata or engine behavior alterations.',rows=parity))
counts=collections.Counter(r['status']for r in d['rows']);assert counts==dict(expected_result_observed=80,semantic_mismatch=2)
for r in d['rows']:
 if r['status']=='semantic_mismatch':
  assert r['card']=='Alex Wilder, Runaway'and r['actual']['cost']==r['expected']['cost']
  assert r['actual']['source_pt']['power']==1 and r['expected']['source_pt']['power']==3
  assert r['actual']['other']==dict(haste=False)and r['expected']['other']==dict(haste=True)
reason='Actual One with Nothing paidB discards Alex and resources from hand. The normally advertised escape cast pays2R and exiles exactly three other graveyard cards, then Alex enters the battlefield. It remains1/3 without haste rather than becoming3/3 with haste until end of turn. The normal paid1R hand-cast control correctly remains1/3 without haste. The strict intervening-if is ThisSpellWasCastFromNonHand; condition_eval returns false whenever that condition has no execution context, including the external ETB trigger gate. Escape payment itself succeeds; neither its selector nor its consumer is implicated.'
d.update(summary=dict(paths=27,scenarios=82,controls=80,wrong_outcomes=2,actual_escape_payments=54,insufficient_resource_controls=27,normal_hand_control=1,confirmed_cards=['Alex Wilder, Runaway'],strict_definitions=len(artifacts)),confirmed_cards=['Alex Wilder, Runaway'],reviewed_scope='All27 exact Escape additional-cost paths have paid exact/surplus and insufficient-resource observations.26 count-based costs exclude the source itself; Nethergoyf tests distinct card types. All54 actual payments succeed.26 cards have scoped resolution controls; Alex alone has two actual ETB-condition failures with an independent normal-hand control. These cost controls are not whole-card semantic certificates.',parity_report=f'reports/runtime-audit/{prefix}-parity.json',process_report=f'reports/runtime-audit/{prefix}-process.json',preserved_initial_report='reports/runtime-audit/snapshots/extended-escape-initial81-40528521d45e7f82/manifest.json',excluded_pilots=[])
write(prefix+'-reproductions.json',d);ref=dict(path=f'reports/runtime-audit/{prefix}-reproductions.json',sha256=sha(p/(prefix+'-reproductions.json')))
findings=[dict(card_name=r['card'],confirmed_cards=[r['card']],classification='runtime_defect_card_reproduced',outcome_category='silent_wrong_result',failure_stage='escaped_source_etb_intervening_condition',defect_subtype='nonhand_cast_condition_requires_execution_context',reason=reason,scenario=r['scenario'],expected=r['expected'],observed=r['actual'],source_report=ref,source_row=i,artifact_checksum=r['artifact_checksum'])for i,r in enumerate(d['rows'])if r['status']=='semantic_mismatch']
paths=[]
for n in sorted({r['card']for r in d['rows']}):
 rows=[i for i,r in enumerate(d['rows'])if r['card']==n];r=d['rows'][rows[0]]
 paths.append(dict(card=n,cost_path=r['scenario']['cost_path'],consumer_path=r['scenario']['consumer_path'],status='paid_escape_cost_controls_downstream_etb_failure'if n=='Alex Wilder, Runaway'else'paid_escape_cost_and_scoped_outcome_controls',source_report=ref,source_rows=rows,scope='Exact/surplus actual cost payment and insufficient-resource boundary; additional normal-hand conditional control for Alex. See each row and report limitations for the specific resulting state checked.'))
write(prefix+'-reviewed-attribution.json',dict(confirmed_cards=d['confirmed_cards'],summary=d['summary'],findings=findings,source_report=ref,reason=reason,path_coverage=paths,parity_report=d['parity_report'],process_report=d['process_report'],limitations=d['limitations'],runtime_attribution_sources=[dict(path=f,sha256=sha(root/f))for f in ['crates/ironsmith-engine/src/condition_eval.rs','crates/ironsmith-engine/src/condition_eval/context.rs','crates/ironsmith-engine/src/triggers/check.rs']],excluded_pilots=[]))
(p/(prefix+'-reproductions.md')).write_text('# Escape additional-cost audit\n\n82 cases across27 exact paths:54 real escape payments and27 insufficient-resource controls, plus one normal hand-cast control. All payments satisfy the printed cost.80 scoped outcomes match; Alex has two ETB-condition failures.\n\n'+reason+'\n\n'+d['limitations']+'\n\n'+f'{len(artifacts)} definitions match both frozen corpora ({len(parity)} comparisons). Binary and source hashes verified.\n')
print(d['summary'])
