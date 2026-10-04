#!/usr/bin/env python3
"""Review 47 specialized WithId additional-cost paths at the first cast-availability gate."""
import hashlib,json
from collections import Counter
from pathlib import Path
P=Path('reports/runtime-audit')
def ref(p):return {'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
def main():
 ip=P/'extended-cost-dependency-candidates.json';inv=json.load(open(ip));inputs=json.load(open(P/'additional-withid-special-frozen-inputs.json'));cases={c['card']:c for c in inputs['cases']};rp=P/'additional-withid-special-execution.json';raw=json.load(open(rp));rr=ref(rp)
 assert len(cases)==47 and len(raw['rows'])==243 and raw['provenance']['artifacts_unchanged'] and all(c['definition_matches_frozen_except_unique_card_ids']for c in raw['compilation'])
 findings=[];controls=[];mapping={c:[]for c in cases}
 for i,r in enumerate(raw['rows']):
  assert r['status']not in ['fixture_error','offered_requires_followthrough'];c=cases[r['card']]; assert r['fixture_evidence']['path']==c['path'] and r['fixture_evidence']['consumer_path']==c['consumer_path'];assert not r['actual']['cast_offered']and not r['actual']['source_action_dispatched']
  failed=[v for v in r['checks']if v['expected']!=v['observed']];want=r['expected']['cast_offered'];common={'card':r['card'],'scenario':r['scenario'],'source_report':rr,'source_row':i,'expected':r['expected'],'observed':r['actual']}
  if want:
   assert len(failed)==1 and failed[0]['check']=='cast_offered'
   early_x=r['card']=="Nahiri's Sacrifice"
   assert r['actual']['total_cost_check']==('Err(Other("Not enough objects to choose (1 needed, 0 available)"))' if early_x else 'Ok(())')
   ctrl=r['actual']['ordinary_control'];assert ctrl['mana_paid']==(1 if r['card']=='Abjure'else 0)and ctrl['resolution_error']is None
   if r['card']!='Abjure':assert ctrl['normal_zero_mana_creature_cast_reaches_battlefield']
   findings.append({**common,'confirmed_cards':[r['card']],'classification':'runtime_defect_card_reproduced','outcome_category':'silent_wrong_result','finding':('With a normally paid mana-value-zero Ornithopter and exact printed {1}{R}, the legal X=0, zero-target cast is absent. The chooser rejects all objects at the earlier ManaValue EqualExpr(X) precheck. The wrapped sacrifice consumer is not reached.' if early_x else 'The normal cast is absent despite actual legal owned resources, printed mana, normal timing and intended targets. The context-aware total additional-cost check succeeds; the independent wrapped sacrifice component rejects the unpopulated tag. A same-state ordinary paid cast succeeds. No source payment or resolution occurs.'),'scope':'Exact additional choose/WithId cost path, first legal-action gate only.'})
  else:
   assert not failed
   controls.append({**common,'confirmed_cards':[],'classification':'expected_outcome_passed','scope':'Only the recorded insufficient, type, controller, damage-history or origin restriction is checked; no source action is dispatched.'})
  mapping[r['card']].append({'source_report':rr,'source_row':i,'scenario':r['scenario'],'intended_legal':want,'source_cast_executed':False})
 paths=[]
 for c in inv['rows']:
  if c['card']not in cases or c['consumer_kind']!='WithIdEffect':continue
  assert c['path']==cases[c['card']]['path'];paths.append({**c,'status':'scoped_legality_failure_reproduced','source_evidence':mapping[c['card']],'scope':'Actual paid canonical resources and producers, exact printed mana, and current legal-action query. No source payment/effect reached. Same-state ordinary cast controls on every intended-legal missing cast.'})
 assert len(paths)==47 and len(findings)==106 and len(controls)==137
 counts={'paths':47,'scenarios':243,'valid_state_missing_cast_observations':106,'unavailable_resource_controls':137,'actual_same_state_cast_controls':106,'actual_source_casts':0,'withid_dependency_paths':46,'earlier_x_filter_paths':1,'strict_frozen_definitions':len(raw['compilation']),'unrun_paths':0}
 limits=['This is cast-availability coverage, not a claim that any of the47 source costs or effects execute. Every unavailable source action is left undispatched.','All64 full canonical definitions strictly load and match frozen definitions except unique CardIds. Every paid resource and graveyard/damage producer is recorded; library contents and initial phase checkpoint are fixtures.','All intended-legal missing actions have an actual same-state ordinary cast: Dispel counters an actually paid pending Shock for Abjure; Ornithopter enters for the others.','Nahiris Sacrifice X=0 fails earlier at its X-dependent mana-value chooser; it is not attributed to the later WithId consumer.','Corpse Cobble and Vicious Betrayal allow zero sacrifices; their zero, wrong-type and opponent-only states are legal zero-payment positives, not controller-boundary negatives.','Treacherous Greed has actual Rabid Bite damage-history positives and no-damage negatives. Opponent-only damage history is not exercised.','Finish is placed in the graveyard by actual Faithless Looting and tested against a hand-origin negative. Aftermath linked-front metadata and other face abilities are outside scope.','The discarded initial attempt is archived under additional-withid-special-snapshots/before-source-hand-timing-fix. That attempt exposed the audited source to cleanup while producing multiple lands and yielded no reviewed raw result. Corrected run creates the source after those turns.']
 review={'scope':__doc__,'findings':findings,'controls':controls,'confirmed_cards':sorted(cases),'counts':counts,'path_coverage':paths,'source_reports':[rr],'provenance':{'artifacts_unchanged':True,'native_source_runs':[{'source_report':rr,'run':raw['provenance'],'attempt':json.load(open(P/'additional-withid-special-attempt.json'))}]},'limitations':limits}
 out=P/'additional-withid-special-reviewed-attribution.json';out.write_text(json.dumps(review,indent=2)+'\n')
 ledger={'family':'additional_cost_withid','subfamily':'special','inventory':ref(ip),'path_count':47,'rows':paths,'counts':counts,'reviewed_sources':[ref(out)],'status_counts':dict(Counter(r['status']for r in paths)),'limitations':limits,'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()};(P/'additional-withid-special-path-coverage.json').write_text(json.dumps(ledger,indent=2)+'\n')
 (P/'additional-withid-special-review.md').write_text('# Specialized WithId additional costs\n\n47 paths:106 legal casts absent,137 unavailable-state controls,106 actual same-state ordinary casts. All source payments and effects remain unreached.\n\n'+'\n'.join('- '+s for s in limits)+'\n');print(json.dumps(counts))
if __name__=='__main__':main()
