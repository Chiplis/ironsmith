#!/usr/bin/env python3
"""Review strict canonical Escape pair payments and independent outcomes."""
import collections,hashlib,json,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit';prefix='extended-alternate-exile'
read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def write(n,d):(p/n).write_text(json.dumps(d,indent=2)+'\n')
def strip(x):
 if isinstance(x,list):return[strip(v)for v in x]
 if isinstance(x,dict):return{k:strip(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
d=read(prefix+'-reproductions.json');proc=read(prefix+'-process.json');assert proc['exit_code']==0 and proc['binary_sha256_before']==proc['binary_sha256_after']==d['provenance']['binary_sha256'];assert sha(root/'crates/ironsmith-tools/tests/runtime_extended_alternate_exile_reproductions.rs')==d['provenance']['source_sha256']
artifacts=read(prefix+'-artifacts.json');parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{p/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for a in artifacts:
   result=db.execute('select result_json from result where run_id=? and card_name=?',(run,a['card'])).fetchone();assert result,a['card'];f=json.loads(result[0]);equal=strip(a['definition'])==strip(f['definition']);assert equal,a['card'];parity.append(dict(card=a['card'],corpus=corpus,run_id=run,artifact_checksum=a['artifact_checksum'],frozen_checksum=f['artifact_checksum'],definition_equal_ignoring_only_card_ids=equal))
write(prefix+'-parity.json',dict(scope='Full strict canonical definitions match both frozen corpora ignoring only definition card IDs. No metadata or engine behavior alterations.',rows=parity))
counts=collections.Counter(r['status']for r in d['rows']);assert counts==dict(expected_result_observed=21,semantic_mismatch=8)
names=sorted({r['card']for r in d['rows']if r['status']=='semantic_mismatch'});assert len(names)==4
for r in d['rows']:
 if r['status']=='semantic_mismatch':assert r['card'].endswith('Shoal')and r['scenario']['variant']in['exact','surplus']and r['expected']['selected_method_offered']and not r['actual']['selected_method_offered']
reason='The exact canonical spell and an owned matching-color mana-value2 card are in hand, with a legal target (including an actual paid opposing mana-value2 creature spell on the stack for Disrupting Shoal). No alternate cast action is offered for either one or two matching material cards. The actual ordinary X2 cast pays4 and produces the expected effect; missing/wrong-color material negatives also behave correctly. The compiled alternative is Composed index0 with a ChooseObjects filter mana_value EqualExpr(X). Alternative legality checks that producer before an X announcement; cost_candidate_count creates an execution context without X, so the constrained card is not matched. The intended alternate action was never forced and its payment/consumer is unreached.'
d.update(summary=dict(paths=6,scenarios=29,controls=21,missing_legal_alternate_actions=8,confirmed_cards=names,normal_paid_controls=6,successful_alternate_payments=4,strict_definitions=len(artifacts)),confirmed_cards=names,reviewed_scope='Four Shoal paths are blocked before announcement with sufficient resources; only ordinary paid controls reach their effects. Stalwart Valkyrie and Spinning Darkness have scoped exact/surplus actual alternate-cost and outcome controls, plus ordinary mana payment and insufficient/wrong-type controls. Spinning Darkness selects the top three black cards of its actual graveyard with interleaved nonblack cards. No absent action dispatched.',parity_report=f'reports/runtime-audit/{prefix}-parity.json',process_report=f'reports/runtime-audit/{prefix}-process.json',excluded_pilots=[])
write(prefix+'-reproductions.json',d);ref=dict(path=f'reports/runtime-audit/{prefix}-reproductions.json',sha256=sha(p/(prefix+'-reproductions.json')))
findings=[dict(card_name=r['card'],confirmed_cards=[r['card']],classification='runtime_defect_card_reproduced',outcome_category='silent_wrong_result',failure_stage='alternate_cast_legality_before_x_announcement',defect_subtype='x_mana_value_cost_filter_checked_before_x_is_bound',reason=reason,scenario=r['scenario'],expected=r['expected'],observed=r['actual'],source_report=ref,source_row=i,artifact_checksum=r['artifact_checksum'])for i,r in enumerate(d['rows'])if r['status']=='semantic_mismatch']
paths=[]
for n in sorted({r['card']for r in d['rows']}):
 ix=[i for i,r in enumerate(d['rows'])if r['card']==n];r=d['rows'][ix[0]]
 paths.append(dict(card=n,cost_path=r['scenario']['cost_path'],consumer_path=r['scenario']['consumer_path'],status='alternate_legality_gate_before_x_cost_consumer'if n in names else'paid_alternate_exile_cost_and_scoped_outcome_controls',source_report=ref,source_rows=ix,scope='Exact/surplus matching resource cases and ordinary paid spell controls. Shoal alternate payment remains unreached; its normal-paid controls are not credited to the alternate cost consumer.'))
write(prefix+'-reviewed-attribution.json',dict(confirmed_cards=names,summary=d['summary'],findings=findings,source_report=ref,reason=reason,path_coverage=paths,parity_report=d['parity_report'],process_report=d['process_report'],limitations=d['limitations'],runtime_attribution_sources=[dict(path=f,sha256=sha(root/f))for f in ['crates/ironsmith-engine/src/decision/mana.rs','crates/ironsmith-engine/src/effects/composition/choose_objects.rs']],excluded_pilots=[]))
(p/(prefix+'-reproductions.md')).write_text('# Alternate exile-cost audit\n\n29 cases across six exact paths:21 scoped controls and eight missing legal alternate cast actions across four Shoals.\n\n'+reason+'\n\n'+d['reviewed_scope']+'\n\n'+f'{len(artifacts)} definitions match both frozen corpora ({len(parity)} comparisons). Stable binary and source hashes verified.\n')
print(d['summary'])
