#!/usr/bin/env python3
"""Bind four already-reviewed additional WithId paths to their exact historical paid scenarios."""
import hashlib,json
from pathlib import Path
P=Path('reports/runtime-audit')
def ref(p):return {'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
def get(v,path):
 for k in path.strip('/').split('/'):v=v[int(k)]if isinstance(v,list)else v[k]
 return v

def main():
 names={'Devastating Summons','Eliminate the Competition','Immoral Bargain','Tectonic Split'};ip=P/'extended-cost-dependency-candidates.json';rawp=P/'nonmana-x-spell-final-execution.json';reviewp=P/'nonmana-x-spell-reviewed-attribution.json';inp=P/'nonmana-x-spell-frozen-inputs.json'
 inv=json.load(open(ip));raw=json.load(open(rawp));review=json.load(open(reviewp));inputs=json.load(open(inp));compiled={c['card']:c for c in raw['compilation']}
 assert raw['provenance']['artifacts_unchanged']
 immutable=[]
 for entry in raw['provenance']['before'][1:]:
  p=Path(entry['path']);assert ref(p)['sha256']==entry['sha256'];immutable.append(ref(p))
 rows=[]
 for c in inv['rows']:
  if c['card']not in names or c['consumer_kind']!='WithIdEffect':continue
  n=c['card'];d=compiled[n];assert d['definition_matches_frozen_except_unique_card_ids']and d['frozen_artifact_checksum']==c['artifact_checksum']
  node={'definition':d['definition']};choose=get(node,c['path']);consume=get(node,c['consumer_path']);assert choose['Effect']['kind']=='ChooseObjectsEffect'and consume['Effect']['kind']=='WithIdEffect'and consume['Effect']['payload']['effect']['kind']=='SacrificePlayerEffect';assert choose['Effect']['payload']['tag']==c['tag']
  evidence=[]
  for i,r in enumerate(raw['rows']):
   if r['card']!=n:continue
   f=[f for f in review['findings']if f['source_row']==i and f['source_report']['sha256']==ref(rawp)['sha256']and f['confirmed_cards']==[n]];assert len(f)==1
   assert r['status']=='expected_legal_cast_unavailable' and r['expected']['cast_offered'] and not r['actual']['cast_offered']and r['actual']['mana_paid']==0 and r['actual']['objects_unchanged']and r['fixture_evidence']['probe_action']is None
   assert f[0]['same_state_fixed_flashback_control_passed']
   evidence.append({'source_report':ref(rawp),'source_row':i,'scenario':r['scenario'],'expected':r['expected'],'observed':f[0]['observed'],'source_cast_executed':False,'supporting_paid_producer_history':r['fixture_evidence']['paid_source_and_resource_producers'],'reviewed_source':ref(reviewp),'scope':'First legal-action gate; same-state actual normal Firebolt and fixed flashback control pass. Historical executable only.'})
  assert len(evidence)==(2 if n=='Tectonic Split'else 3)
  rows.append({**c,'status':'scoped_legality_failure_reproduced','source_evidence':evidence,'runtime_provenance':raw['provenance'],'exact_definition_path_verified':True,'scope':'Exact additional-cost path matches immutable historical compiled definition and frozen artifact; no fresh runtime claim, payment or resolution claim.'})
 assert len(rows)==4
 limits=['Reuse binds the exact choose and wrapped-sacrifice cost nodes, tag, frozen artifact checksum, raw report and source/input hashes. It does not claim the older executable equals the current checkout.','The historical immutable executable checksum is preserved in provenance even if a subsequent build replaced that local executable path.','No source cast was dispatched in these11 cases. Actual resources and same-state Firebolt controls support only the missing-legal-cast finding.','Eliminate the Competition and Immoral Bargain always have two actually paid own Hill Giants as targets. The resources_available value counts Grizzly Bears only, so their zero-Bears states are not zero-total-creature boundary controls.','Devastating Summons has actual Mountain land-play states0/1 and intendedX0/1. Tectonic Split has actual0/1lands; larger rounded-up boundaries and spell/ETB behavior are not covered.']
 out={'family':'additional_cost_withid','subfamily':'historical_nonmana_x','inventory':ref(ip),'path_count':4,'rows':rows,'reviewed_sources':[ref(reviewp)],'counts':{'paths':4,'scenarios':11,'valid_state_missing_cast_observations':11,'actual_source_casts':0,'new_promotions':0,'unrun_paths':0},'status_counts':{'scoped_legality_failure_reproduced':4},'verified_unchanged_historical_inputs_and_source':immutable,'limitations':limits,'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
 (P/'additional-withid-historical-path-coverage.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out['counts']))
if __name__=='__main__':main()
