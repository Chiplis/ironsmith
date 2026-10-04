#!/usr/bin/env python3
"""Review 59 typed WithId sacrifice-cost cast-legality reproductions, without claiming resolution."""
import hashlib,json
from collections import Counter
from pathlib import Path
P=Path('reports/runtime-audit')
def ref(p):return {'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
def main():
 invpath=P/'extended-cost-dependency-candidates.json';inv=json.load(open(invpath));inputs=json.load(open(P/'additional-withid-simple-frozen-inputs.json'));cases={r['card']:r for r in inputs['cases']}
 candidates=[r for r in inv['rows']if r['card']in cases and r['consumer_kind']=='WithIdEffect'];assert len(candidates)==59
 path=P/'additional-withid-simple-execution.json';raw=json.load(open(path));cp=P/'additional-withid-simple-controls-execution.json';edge=json.load(open(cp));sources=[ref(path),ref(cp)]
 assert len(raw['rows'])==310 and len(edge['rows'])==26
 for data in [raw,edge]:assert data['provenance']['artifacts_unchanged']and all(c['definition_matches_frozen_except_unique_card_ids']for c in data['compilation'])
 findings=[];controls=[];map={n:[]for n in cases}
 for i,r in enumerate(raw['rows']):
  assert r['status']!='fixture_error';c=cases[r['card']];assert r['fixture_evidence']['path']==c['path']and r['fixture_evidence']['consumer_path']==c['consumer_path'];assert not r['actual']['cast_offered']and r['actual']['action']is None
  failed=[v for v in r['checks']if v['expected']!=v['observed']];want=r['expected']['cast_offered']
  common={'card':r['card'],'scenario':r['scenario'],'source_report':sources[0],'source_row':i,'expected':r['expected'],'observed':r['actual']}
  if want:
   assert len(failed)==1 and failed[0]['check']=='cast_offered';assert r['actual']['component_checks'][-1]=='Err(NoValidSacrificeTarget)'
   no_zone=r['card']in ['Goblin Grenade','Fodder Launch']
   assert ('ChooseObjectsEffect requires an explicit search zone' in r['actual']['total_cost_check']) if no_zone else r['actual']['total_cost_check']=='Ok(())'
   canonical="Kazuul's Fury" if r['card']=="Kazuul's Fury // Kazuul's Cliffs"else r['card']
   findings.append({**common,'confirmed_cards':[canonical],'classification':'runtime_defect_card_reproduced','outcome_category':'silent_wrong_result','finding':('The normal cast is absent despite a normally paid Goblin Piker available to sacrifice. The subtype-only ChooseObjects cost omits both filter.zone and choose.zone, causing the earlier explicit-search-zone failure. The later wrapped consumer is not reached.' if no_zone else 'The normal cast is absent despite printed mana and actual owned controlled sacrifice resources. The total additional-cost checker succeeds; the wrapped sacrifice consumer fails independent component validation. Casting never starts, so sacrifice payment and the spell effect are unexecuted.'),'scope':'Exact additional choose→WithId sacrifice path at the cast-availability gate; not a resolution claim.'})
  else:
   assert not failed
   controls.append({**common,'confirmed_cards':[],'classification':'expected_outcome_passed','scope':'Insufficient resources, wrong type or only opponent-controlled resources correctly prevent casting; no source action dispatched.'})
  map[r['card']].append({'source_report':sources[0],'source_row':i,'scenario':r['scenario'],'intended_legal':want,'cast_executed':False})
 for i,r in enumerate(edge['rows']):
  assert r['status']!='fixture_error'
  if r['expected']['cast_offered']:
   assert r['actual']['ordinary_control']['normal_zero_mana_creature_cast_reaches_battlefield']and r['actual']['ordinary_control']['source_card_still_in_hand']
   map[r['card']].append({'source_report':sources[1],'source_row':i,'scenario':r['scenario'],'scope':'Same state actual normal Ornithopter cast succeeds after the unavailable audited cast is observed; no extra source defect counted.'})
 paths=[]
 for c in candidates:
  evidence=map[c['card']];assert {'zero','exact','surplus','opponent_only'}<={e['scenario']for e in evidence}
  paths.append({**c,'status':'scoped_legality_failure_reproduced','source_evidence':evidence,'scope':'Actual resource producers and exact printed mana; advertised normal cast missing. No cost payment or spell/ETB resolution reached. Alternative type variants only as individually listed.'})
 counts={'paths':59,'primary_scenarios':310,'valid_state_missing_cast_observations':len(findings),'unavailable_resource_controls':len(controls),'supporting_same_state_scenarios':26,'actual_supporting_cast_controls':11,'actual_source_casts':0,'canonical_confirmed_names':len({n for f in findings for n in f['confirmed_cards']}),'unrun_paths':0,'missing_choose_zone_paths':2,'withid_dependency_paths':57}
 sourceaudit=[{'source':ref(Path('crates/ironsmith-engine/src/decision/mana.rs')),'lines':[3144,3984,4019,4047],'finding':'can_pay_non_mana_cost_sequence_for_cast tracks ChooseObjects tags but tagged_dependency_satisfied_by_prior_cost inspects only bare SacrificeEffect/SacrificePlayerEffect/ExileEffect/ReturnToHandEffect; it does not peel WithIdEffect.'},{'source':ref(Path('crates/ironsmith-engine/src/costs/cost_effect.rs')),'lines':[70,100],'finding':'The individual wrapped sacrifice precheck does peel transparent wrappers, then rejects absent tag state. The separate total-cost checker is context-aware and returns Ok for129 positive recorded states. Four positive Goblin Grenade/Fodder Launch states fail earlier because the chooser has no explicit zone.'}]
 limits=['No unavailable action is forced; all source effects and ETBs remain unexecuted behind the legality gate. Goblin Grenade and Fodder Launch fail earlier at a zone-less chooser and are not attributed to execution of the later wrapped consumer.','Resources are full canonical normal paid casts or legal land plays, including real turn transitions for multiple lands. All70 strict definitions match frozen input except unique CardIds.','Printed mana is funded exactly after resources are produced. The source is the exact hand object inspected for legal cast actions.','Five representative payloads have11 same-state successful normal Ornithopter casts; these are supporting controls, not extra distinct defects.','The combined Kazuul payload is independently compiled and exercised but attributed to the primary canonical card; its back face is outside scope.','Buyback, flashback, additional target/type variants and other branches not explicitly listed remain outside this scoped proof.','The source inspection is fingerprinted against the current checkout; each executed report is separately pinned to its executable and input/source hashes.']
 review={'scope':__doc__,'findings':findings,'controls':controls,'confirmed_cards':sorted({n for f in findings for n in f['confirmed_cards']}),'counts':counts,'path_coverage':paths,'source_reports':sources,'source_audit':sourceaudit,'provenance':{'artifacts_unchanged':True,'native_source_runs':[{'source_report':sources[i],'run':d['provenance'],'attempt':json.load(open(P/attempt))}for i,(d,attempt)in enumerate([(raw,'additional-withid-simple-attempt.json'),(edge,'additional-withid-simple-controls-attempt.json')])]},'limitations':limits}
 rp=P/'additional-withid-simple-reviewed-attribution.json';rp.write_text(json.dumps(review,indent=2)+'\n')
 ledger={'family':'additional_cost_withid','subfamily':'fixed_simple','inventory':ref(invpath),'path_count':len(paths),'rows':paths,'counts':counts,'reviewed_sources':[ref(rp)],'status_counts':dict(Counter(r['status']for r in paths)),'limitations':limits,'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()};(P/'additional-withid-simple-path-coverage.json').write_text(json.dumps(ledger,indent=2)+'\n')
 (P/'additional-withid-simple-review.md').write_text('# Fixed WithId additional sacrifice costs\n\n59 exact paths reproduce unavailable normal casting:133 valid resource states,177 negative controls. Eleven supporting casts succeed in the same states. No source payment or resolution is reached.\n\n'+'\n'.join('- '+s for s in limits)+'\n');print(json.dumps(counts))
if __name__=='__main__':main()
