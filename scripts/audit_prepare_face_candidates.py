#!/usr/bin/env python3
"""Inventory real prepare layouts and typed preparation producers, never infer execution."""
import json,sqlite3,hashlib,collections
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit'
raw=(root/'cards.json').read_bytes();cards=json.loads(raw);inventory_path=p/'corpus/267a16aff3b321196397d0b4/inventory.json';inv={r['name']:r for r in json.loads(inventory_path.read_bytes())['cards']}
groups={}
for c in cards:
 if c.get('layout')=='prepare' and len(c.get('card_faces',[]))==2:
  key=c['name']; names=[f['name']for f in c['card_faces']]
  if key in groups:assert [f['name']for f in groups[key]['card_faces']]==names
  else:groups[key]=c

def walk(x,path=''):
 if isinstance(x,dict):
  if x.get('kind')=='PrepareEffect':yield dict(path=path,kind='PrepareEffect',payload=x.get('payload'))
  if x.get('id')=='EntersPrepared':yield dict(path=path,kind='EntersPrepared')
  for k,v in x.items():
   if k=='flattened_default_effects':continue # Rendering copy duplicates actual segments.
   yield from walk(v,path+'/'+k)
 elif isinstance(x,list):
  for i,v in enumerate(x):yield from walk(v,path+'/'+str(i))
rows=[]
with sqlite3.connect(f'file:{p/"actions/results.sqlite3"}?mode=ro',uri=True)as db:
 for name,c in sorted(groups.items()):
  front,back=c['card_faces']; payloads=[]
  for n,role in [(front['name'],'front'),(name,'combined_alias'),(back['name'],'linked_spell_name')]:
   r=db.execute('select status,result_json from result where run_id=? and card_name=?',('e17a4980b0b92c7a5a4cead2',n)).fetchone();f=json.loads(r[1])if r else{};definition=f.get('definition',{});routes=[]
   for i,a in enumerate(definition.get('abilities',[])):
    kind=next(iter(a.get('kind',{})),'unknown');prepare=list(walk(a,'/definition/abilities/'+str(i)))
    for t in prepare:routes.append(dict(ability_index=i,ability_kind=kind,functional_zones=a.get('functional_zones'),**t))
   payloads.append(dict(name=n,role=role,in_frozen_inventory=n in inv,status=r[0]if r else'absent',artifact_checksum=f.get('artifact_checksum'),typed_prepare_routes=routes,parse_input=inv.get(n,{}).get('parse_input'),execution_status='not_exercised_by_this_inventory'))
  enters=any(t['kind']=='EntersPrepared'for t in payloads[0]['typed_prepare_routes']);kinds=sorted({t['ability_kind']for t in payloads[0]['typed_prepare_routes']})
  rows.append(dict(combined_name=name,catalog_id=c['id'],layout='prepare',front=front['name'],spell_face=back['name'],front_oracle=front.get('oracle_text'),spell_oracle=back.get('oracle_text'),spell_mana_cost=back.get('mana_cost'),entry_prepared=enters,typed_producer_kinds=kinds,payloads=payloads,reviewed_status='existing_linked_reproduction'if front['name']in ['Lluwen, Exchange Student','Harmonized Trio']else'unexercised_prepare_family_candidate'))
summary=dict(canonical_groups=len(rows),canonical_front_names=len({r['front']for r in rows}),payload_roles=len(rows)*3,distinct_payload_names=len({p['name']for r in rows for p in r['payloads']}),entry_prepared=sum(r['entry_prepared']for r in rows),producer_kind_sets=dict(collections.Counter('+'.join(r['typed_producer_kinds'])or'none'for r in rows)))
out=dict(scope='All actual cards.json prepare relationships with typed preparation routes from frozen action definitions. No inferred card failure or execution coverage from family membership. Back-face standalone names may also be ordinary cards; role retained. Frozen full inputs included; no fabricated relationship.',cards_json_sha256=hashlib.sha256(raw).hexdigest(),inventory_sha256=hashlib.sha256(inventory_path.read_bytes()).hexdigest(),database='reports/runtime-audit/actions/results.sqlite3',run_id='e17a4980b0b92c7a5a4cead2',summary=summary,rows=rows)
(p/'prepare-face-candidates.json').write_text(json.dumps(out,indent=2)+'\n')
(p/'prepare-face-candidates.md').write_text('# Canonical prepare family\n\n'+json.dumps(summary,indent=2)+'\n\n| Front | Spell face | Enters prepared | Typed producers |\n|---|---|---|---|\n'+'\n'.join('|'+r['front']+'|'+r['spell_face']+'|'+str(r['entry_prepared'])+'|'+','.join(r['typed_producer_kinds'])+'|'for r in rows)+'\n\nCandidate inventory only. Actual linked Lluwen/Harmonized evidence remains separately reviewed.\n')
print(json.dumps(summary,indent=2))
for r in rows:print(r['front'],r['entry_prepared'],r['typed_producer_kinds'],'SPELL',r['spell_face'],r['spell_mana_cost'])
