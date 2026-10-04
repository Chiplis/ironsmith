#!/usr/bin/env python3
"""Review strict canonical Escape pair payments and independent outcomes."""
import collections,hashlib,json,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit';prefix='extended-trigger-cost'
read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def write(n,d):(p/n).write_text(json.dumps(d,indent=2)+'\n')
def strip(x):
 if isinstance(x,list):return[strip(v)for v in x]
 if isinstance(x,dict):return{k:strip(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
d=read(prefix+'-reproductions.json');proc=read(prefix+'-process.json');assert proc['exit_code']==0 and proc['binary_sha256_before']==proc['binary_sha256_after']==d['provenance']['binary_sha256'];assert sha(root/'crates/ironsmith-tools/tests/runtime_extended_trigger_cost_reproductions.rs')==d['provenance']['source_sha256']
artifacts=read(prefix+'-artifacts.json');parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{p/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for a in artifacts:
   result=db.execute('select result_json from result where run_id=? and card_name=?',(run,a['card'])).fetchone();assert result,a['card'];f=json.loads(result[0]);equal=strip(a['definition'])==strip(f['definition']);assert equal,a['card'];parity.append(dict(card=a['card'],corpus=corpus,run_id=run,artifact_checksum=a['artifact_checksum'],frozen_checksum=f['artifact_checksum'],definition_equal_ignoring_only_card_ids=equal))
write(prefix+'-parity.json',dict(scope='Full strict canonical definitions match both frozen corpora ignoring only definition card IDs. No metadata or engine behavior alterations.',rows=parity))
counts=collections.Counter(r['status']for r in d['rows']);assert counts==dict(expected_result_observed=7,semantic_mismatch=2)
for r in d['rows']:
 if r['status']=='semantic_mismatch':assert r['card']=='Anurid Scavenger'and r['scenario']['variant']in['exact','surplus']and r['actual']['source_zone']=='Graveyard'and r['actual']['selected_material_zone']=='Graveyard'and r['actual']['error']is None
reason='A fully paid canonical Anurid Scavenger reaches its actual next upkeep. Its owner has a graveyard card produced by normally paid One with Nothing, and the fixture chooser is configured to accept a legal payment. The engine never offers that payment, leaves the card in the graveyard, and sacrifices Anurid instead of putting the selected card on the library bottom and keeping Anurid. Exact and surplus cases reproduce this. Empty, deliberate-decline and opponent-only graveyard controls correctly sacrifice it. The strict cost is ChooseObjects(tag library_cost_0) followed by MoveToZone(Tagged library_cost_0,Library,to_top=false). cost::total_cost_has_tagged_choice_consumer recognizes sacrifice/exile but not MoveToZone, so it independently prechecks the consumer before the producer has bound its tag. Phyrexian Dreadnought\'s distinct aggregate-power sacrifice cost has four passing actual ETB controls.'
d.update(summary=dict(paths=2,scenarios=9,controls=7,wrong_outcomes=2,confirmed_cards=['Anurid Scavenger'],strict_definitions=len(artifacts)),confirmed_cards=['Anurid Scavenger'],reviewed_scope='Exact Anurid upkeep and Dreadnought ETB cost paths tested with actual paid sources/resources. Anurid payment is unreachable despite valid own graveyard card; its move-to-library consumer is not executed. Dreadnought decline, two6-power sacrifice, surplus and self12-power sacrifice controls all pass. No fabricated trigger or forced unavailable cost.',parity_report=f'reports/runtime-audit/{prefix}-parity.json',process_report=f'reports/runtime-audit/{prefix}-process.json',excluded_pilots=[])
write(prefix+'-reproductions.json',d);ref=dict(path=f'reports/runtime-audit/{prefix}-reproductions.json',sha256=sha(p/(prefix+'-reproductions.json')))
findings=[dict(card_name=r['card'],confirmed_cards=[r['card']],classification='runtime_defect_card_reproduced',outcome_category='silent_wrong_result',failure_stage='upkeep_optional_cost_payability',defect_subtype='move_to_zone_tag_consumer_checked_before_cost_producer',reason=reason,scenario=r['scenario'],expected=r['expected'],observed=r['actual'],source_report=ref,source_row=i,artifact_checksum=r['artifact_checksum'])for i,r in enumerate(d['rows'])if r['status']=='semantic_mismatch']
paths=[]
for n in sorted({r['card']for r in d['rows']}):
 ix=[i for i,r in enumerate(d['rows'])if r['card']==n];r=d['rows'][ix[0]]
 paths.append(dict(card=n,cost_path=r['scenario']['cost_path'],consumer_path=r['scenario']['consumer_path'],status='upkeep_cost_gate_before_tagged_move_consumer'if n=='Anurid Scavenger'else'paid_trigger_cost_and_scoped_outcome_controls',source_report=ref,source_rows=ix,scope=reason if n=='Anurid Scavenger'else'Actual canonical paid source and two paid6-power Wurms; decline, exact, surplus and legal self-sacrifice controls.'))
write(prefix+'-reviewed-attribution.json',dict(confirmed_cards=d['confirmed_cards'],summary=d['summary'],findings=findings,source_report=ref,reason=reason,path_coverage=paths,parity_report=d['parity_report'],process_report=d['process_report'],limitations=d['limitations'],runtime_attribution_sources=[dict(path=f,sha256=sha(root/f))for f in ['crates/ironsmith-engine/src/cost.rs','crates/ironsmith-engine/src/effects/zones/move_to_zone.rs']],excluded_pilots=[]))
(p/(prefix+'-reproductions.md')).write_text('# Trigger-cost dependency audit\n\nNine cases across two exact paths:seven controls and two Anurid upkeep failures.\n\n'+reason+'\n\n'+d['reviewed_scope']+'\n\n'+f'{len(artifacts)} definitions match both frozen corpora ({len(parity)} comparisons). Stable binary/source hashes verified.\n')
print(d['summary'])
