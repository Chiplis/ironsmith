#!/usr/bin/env python3
"""Exact typed-path coverage of frozen single-object choose/exile activation costs."""
import collections,hashlib,json,sqlite3
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];P=ROOT/'reports/runtime-audit'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def at(x,path):
 for k in path.strip('/').split('/'):x=x[int(k)]if isinstance(x,list)else x[k]
 return x
def kinds(x):
 result=[]
 if isinstance(x,dict):
  if isinstance(x.get('kind'),str):result.append(x['kind'])
  for v in x.values():result.extend(kinds(v))
 elif isinstance(x,list):
  for v in x:result.extend(kinds(v))
 return list(dict.fromkeys(result))
src=P/'choose-consume-cost-candidates.json';data=json.load(open(src));rows=[]
with sqlite3.connect(f'file:{P/"actions/results.sqlite3"}?mode=ro',uri=True)as db:
 for c in data['rows']:
  if c['count_shape']!='single'or c['consumer_kind']!='ExileEffect':continue
  raw=json.loads(db.execute('select result_json from result where run_id=? and card_name=?',(data['run_id'],c['card'])).fetchone()[0]);ch=at(raw,c['path'])['Effect']['payload'];index=int(c['path'].split('/')[3]);ability=raw['definition']['abilities'][index];activated=ability['kind']['Activated'];f=ch['filter'];flags={k:v for k,v in ch.items()if k not in ['description','filter','tag','count']and v not in [None,False,[],{}]};ff={k:v for k,v in f.items()if k not in ['union_surface']and v not in [None,False,[],{},0]};effect_kinds=kinds(activated['effects']['flattened_default_effects']);group=('craft'if any('Craft'in k for k in effect_kinds)or'craft'in json.dumps(activated['mana_cost']).lower()else f.get('zone')or ch.get('zone')or'unspecified');rows.append(dict(c,ability_index=index,source_functional_zones=ability['functional_zones'],selection_flags=flags,filter_flags=ff,effect_kinds=effect_kinds,group=group,status='unexercised_path',coverage=[]))
# Reviewed evidence is explicitly mapped here; a same-name observation is never enough.
prior=[('Mechtitan Core','fixed-exile-remaining-reviewed-attribution.json','/definition/abilities/0/kind/Activated/mana_cost/kind/All/1','Successful exact4/extra5 activation also commits the separate single source-exile cost, creates token and returns materials after actual paid Disenchant. Other-four shortage controls are not a source-exile availability test.'),('Mechtitan Core // Mechtitan Core','fixed-exile-remaining-reviewed-attribution.json','/definition/abilities/0/kind/Activated/mana_cost/kind/All/1','Independent alias payload; same actual whole activation proves single source-exile cost, no extra primary name.'),('Say Its Name','fixed-exile-graveyard-reviewed-attribution.json','/definition/abilities/0/kind/Activated/mana_cost/kind/All/0','Actual paid Say Its Name source plus2/3 actually cast copies: separate single source cost commits Exile, then Altanak searched from hand enters. Other-copy shortage is not a source availability test.')]
for name,report,path,scope in prior:
 review=json.load(open(P/report));coverage=next(c for c in review['path_coverage']if c['payload_name']==name);ref=coverage['source_report'];raw=json.load(open(ROOT/ref['path']));assert sha(ROOT/ref['path'])==ref['sha256'];ix=[i for i in coverage['source_rows']if raw['rows'][i]['actual'].get('legal_activation_available')and raw['rows'][i]['status']=='expected_result_observed'];assert len(ix)==2
 row=next(r for r in rows if r['card']==name and r['path']==path);row['status']='reused_actual_source_exile_cost_and_outcome';row['coverage'].append(dict(reviewed_report=str((P/report).relative_to(ROOT)),reviewed_sha256=sha(P/report),source_report=ref,source_rows=ix,scope=scope))
# Braided Net prior exact-single Craft control proves cost announcement only.
f=P/'exile-cost-family-reviewed-attribution.json';review=json.load(open(f));raw_path=P/'exile-cost-family-reproductions.json';raw=json.load(open(raw_path));observations=[21,22]
for i in observations:
 o=raw['rows'][i];assert o['card']=='Braided Net'and o['scenario']['ability_index']==2 and o['expected']==o['actual']
r=next(r for r in rows if r['card']=='Braided Net');r['status']='reused_cost_announcement_only';r['coverage'].append(dict(reviewed_report=str(f.relative_to(ROOT)),reviewed_sha256=sha(f),source_report=dict(path=str(raw_path.relative_to(ROOT)),sha256=sha(raw_path)),source_rows=observations,scope='Actual paid3 source and0 Ornithopter. Zero material disallows; exact one exiles material+self, pays2 and stacks Craft. Resolution/transformation was deliberately not attempted; no full outcome coverage.'))
# Merge only explicit per-path reviewed additions. Validate raw bindings and selected ability.
for report in sorted(P.glob('single-exile-*-reviewed-attribution.json')):
 d=json.load(open(report))
 for c in d.get('path_coverage',[]):
  r=next(r for r in rows if r['card']==c['payload_name']and r['path']==c['cost_path']);ref=c['source_report'];raw=json.load(open(ROOT/ref['path']));assert sha(ROOT/ref['path'])==ref['sha256']
  for i in c['source_rows']:
   actual=raw['rows'][i];assert actual['card']==r['card'];assert actual['scenario']['ability_index']==r['ability_index']
  r['status']=c['coverage_status'];r['coverage'].append(dict(c,reviewed_report=str(report.relative_to(ROOT)),reviewed_sha256=sha(report)))
out=dict(scope='All90 frozen single-count ChooseObjects followed by same-tag ExileEffect activation-cost paths. Explicit path/ability/source-zone/flags; no name-only or whole-card certification.',candidate_source=dict(path=str(src.relative_to(ROOT)),sha256=sha(src),run_id=data['run_id']),summary=dict(paths=len(rows),payload_names=len({r['card']for r in rows}),primary_names=len({r['card'].split(' // ')[0]for r in rows}),groups=dict(collections.Counter(r['group']for r in rows)),status_counts=dict(collections.Counter(r['status']for r in rows)),unexercised_paths=sum(r['status']=='unexercised_path'for r in rows)),rows=rows,limitations='Canonical combined-name payloads are independent; no fabricated back face. Successful cost announcement alone is not a resolved transformation claim. Nested path classification preserved if present; current90 paths are top-level printed activated abilities.',scanner_sha256=sha(Path(__file__)))
(P/'single-exile-cost-coverage.json').write_text(json.dumps(out,indent=2)+'\n');md=['# Single-object exile cost coverage','','| Payload | Ability | Source zone | Selection family | Status |','| --- | ---: | --- | --- | --- |']
for r in rows:md.append(f"| {r['card']} | {r['ability_index']} | {','.join(r['source_functional_zones'])} | {r['group']} | {r['status']} |")
(P/'single-exile-cost-coverage.md').write_text('\n'.join(md)+'\n');print(json.dumps(out['summary'],indent=2))
