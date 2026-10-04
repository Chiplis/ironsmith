#!/usr/bin/env python3
"""Path-scoped ledger for all frozen fixed-multiple tagged exile costs."""
import json,hashlib,sqlite3,collections
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];P=ROOT/'reports/runtime-audit'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def at(d,path):
 for k in path.strip('/').split('/'):
  d=d[int(k)]if isinstance(d,list)else d[k]
 return d
src=P/'choose-consume-cost-candidates.json';candidate=json.load(open(src));rows=[]
with sqlite3.connect(f'file:{P/"actions/results.sqlite3"}?mode=ro',uri=True)as db:
 for c in candidate['rows']:
  if c['count_shape']!='fixed_multiple'or c['consumer_kind']!='ExileEffect':continue
  raw=json.loads(db.execute('select result_json from result where run_id=? and card_name=?',(candidate['run_id'],c['card'])).fetchone()[0]);choose=at(raw,c['path'])['Effect']['payload'];r=dict(c,ability_index=int(c['path'].split('/')[3]),status='unexercised_path',coverage=[],filter_flags={k:choose['filter'].get(k)for k in ['zone','single_graveyard','other','controller','owner','card_types','historic','supertypes','name']});rows.append(r)
  if c['card']=='Coin of Fate':
   f=P/'sacrifice-cost-sibling-reviewed-classification.json';review=json.load(open(f));raw_path=P/review['source_report'];data=json.load(open(raw_path));assert digest(raw_path)==review['source_sha256']
   for i in [0,1]:
    v=review['rows'][i];e=data['rows'][i]['fixture_evidence'];assert v['card']=='Coin of Fate'and v['expected']==v['observed']and e['ability_index']==r['ability_index']and e['activation']['mana_paid']==4
    r['coverage'].append(dict(reviewed_report=str(f.relative_to(ROOT)),reviewed_sha256=digest(f),reviewed_row=i,raw_report=str(raw_path.relative_to(ROOT)),raw_sha256=digest(raw_path),scope='Exact2 creature exile +tap+self-sacrifice paid; opponent choice variants and exact return/library/monarch outcomes. Required-minus-one and extra-resource cases not yet covered.'))
   r['status']='two_exact_cost_and_outcome_controls'
# Merge only owned reviewed execution reports with explicit matching candidate path.
for report in sorted(P.glob('fixed-exile-*-reviewed-attribution.json')):
 d=json.load(open(report))
 for finding in d.get('path_coverage',[]):
  matches=[r for r in rows if r['card']==finding['payload_name']and r['path']==finding['cost_path']]
  assert len(matches)==1,(report,finding)
  source=finding['source_report'];raw_path=ROOT/source['path'];assert digest(raw_path)==source['sha256'];raw=json.load(open(raw_path))
  for i in finding['source_rows']:
   observation=raw['rows'][i];assert observation['card']==finding['payload_name'];assert observation['scenario']['cost_path']==finding['cost_path'];assert 'spend_mode'not in observation['scenario']
  r=matches[0];r['coverage'].append(dict(finding,reviewed_report=str(report.relative_to(ROOT)),reviewed_sha256=digest(report)));r['status']=finding['coverage_status']
scoped_evidence=set();ancillary_evidence=set();confirmed_cards=set()
for r in rows:
 for evidence in r['coverage']:
  if 'source_report'in evidence:
   ref=evidence['source_report'];scoped_evidence.update((ref['path'],i)for i in evidence['source_rows']);ancillary_evidence.update((ref['path'],i)for i in evidence.get('ancillary_source_rows',[]))
  else:scoped_evidence.add((evidence['raw_report'],evidence['reviewed_row']))
for f in P.glob('fixed-exile-*-reviewed-attribution.json'):confirmed_cards.update(json.load(open(f)).get('confirmed_cards',[]))
out=dict(confirmed_cards=sorted(confirmed_cards),scope='All26 frozen fixed-multiple ChooseObjects+same-tag ExileEffect activated-cost paths. A scoped outcome is tied to its exact ability/cost path, never inferred from another ability/card-name observation.',candidate_source=dict(path=str(src.relative_to(ROOT)),sha256=digest(src),run_id=candidate['run_id']),summary=dict(paths=len(rows),payload_names=len({r['card']for r in rows}),primary_names=len({r['card'].split(' // ')[0]for r in rows}),status_counts=dict(collections.Counter(r['status']for r in rows)),unexercised_paths=sum(r['status']=='unexercised_path'for r in rows),path_scoped_execution_rows=len(scoped_evidence),ancillary_execution_rows=len(ancillary_evidence),confirmed_primary_cards=len(confirmed_cards)),rows=rows,excluded_unrelated_evidence=[dict(card='Mines of Moria',report='ability-index-static-gate-review.json',reason='Static activity-gate observation does not execute this three-card exile cost; no coverage credited.')],limitations='No blanket promotion or whole-card certification. Combined-name payloads are independent compilation cases but not extra primary card names.',scanner_sha256=digest(Path(__file__)))
(P/'fixed-exile-cost-coverage.json').write_text(json.dumps(out,indent=2)+'\n')
md=['# Fixed-multiple exile cost coverage','',f"All{len(rows)} payload paths ({out['summary']['primary_names']} primary names) have explicit execution accounting: {len(scoped_evidence)} path-scoped rows plus{len(ancillary_evidence)} ancillary Sunken Palace qualification controls. Scope is per path; Sunken cost payment passes despite its sibling trigger defect.",'','| Payload | Ability | Required count | Single graveyard | Reviewed status |','| --- | ---: | ---: | --- | --- |']
for r in rows:md.append(f"| {r['card']} | {r['ability_index']} | {r['count']['min']} | {r['filter_flags']['single_graveyard']} | {r['status']} |")
(P/'fixed-exile-cost-coverage.md').write_text('\n'.join(md)+'\n');print(json.dumps(out['summary'],indent=2))
