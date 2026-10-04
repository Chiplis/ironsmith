#!/usr/bin/env python3
"""Review Meria's exact two-nontoken-artifact payment and subsequent play permission."""
import json,hashlib
from pathlib import Path
ROOT=Path('reports/runtime-audit')
def ref(p):return {'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
def main():
 p=ROOT/'meria-tap-exile-execution.json';r=json.loads(p.read_text());source=ref(p)
 assert len(r['rows'])==14 and r['provenance']['artifacts_unchanged']
 assert all(c['definition_matches_frozen_except_unique_card_ids']for c in r['compilation'])
 controls=[]
 for i,row in enumerate(r['rows']):
  assert row['status']=='expected_outcome_passed' and all(c['expected']==c['observed']for c in row['checks'])
  assert row['fixture_evidence']['source_ability_index']==1
  controls.append({'card':row['card'],'scenario':row['scenario'],'classification':'expected_outcome_passed','confirmed_cards':[],'source_report':source,'source_row':i,'expected':row['expected'],'observed':row['actual'],'checks':row['checks']})
 inv=json.loads((ROOT/'choose-consume-cost-candidates.json').read_text())
 candidate=next(c for c in inv['rows']if c['card']=='Meria, Scholar of Antiquity'and c['consumer_kind']=='TapEffect'and c['count_shape']=='fixed_multiple')
 assert candidate['count']['min']==candidate['count']['max']==2 and '/abilities/1/'in candidate['path']
 path={**candidate,'family':'fixed_tap_cost','status':'scoped_expected_outcomes_passed','source_report':source,'source_rows':list(range(14)),'scope':'Two real nontoken artifacts tapped, exile exact top card, actual paid casts/land play, land-play limit, duration expiry, source departure and empty-library controls'}
 counts={'paths':1,'primary_scenarios':14,'actual_successful_activations':sum(x['expected']['two_artifact_action_offered']for x in r['rows']),'expected_unavailable_controls':sum(not x['expected']['two_artifact_action_offered']for x in r['rows']),'oracle_checks':sum(len(x['checks'])for x in r['rows']),'strict_definitions':len(r['compilation']),'confirmed_cards':0,'unrun_paths':0}
 limits=['No whole-card clearance; this report covers canonical ability1 and separately records ability0 nontoken availability. The separate mana report proves actual ability0 payment.','Actual full canonical Meria and zero-mana Ornithopter casts provide nontoken artifacts. Actual paid Servo Exhibition supplies precisely two real artifact tokens. Tokens-only and one-nontoken-plus-tokens states do not satisfy the cost.','The two- and three-nontoken states tap exactly two; surplus artifacts and tokens stay untapped. No unavailable action is forced.','The exact exiled Mind Stone and Shock objects are normally cast from Exile at their printed mana costs; Shock deals two damage. The tracked exiled Plains is normally played as a land, and an already-used land play remains unavailable.','Permission expiry is checked at the next own first main after real TurnRunner turns, with mana funded and sorcery timing legal. Source-bounce uses an actual paid Unsummon, after which the exiled artifact remains playable this turn.','An empty library does not prevent activation or cause a resolution exception; the real two-artifact cost is still paid.','All eight strict definitions match frozen artifacts except unique CardIds; executable, input and fixture hashes remain unchanged.']
 review={'scope':__doc__,'findings':[],'confirmed_cards':[],'controls':controls,'counts':counts,'path_coverage':[path],'source_reports':[source],'provenance':{'artifacts_unchanged':True,'native_source_runs':[{'source_report':source,'run':r['provenance'],'attempt':json.loads((ROOT/'meria-tap-exile-attempt.json').read_text())}]},'limitations':limits}
 rp=ROOT/'meria-tap-exile-reviewed-attribution.json';rp.write_text(json.dumps(review,indent=2)+'\n')
 ledger={'family':'fixed_tap_cost','path_count':1,'rows':[path],'counts':counts,'reviewed_sources':[ref(rp)],'status_counts':{'scoped_expected_outcomes_passed':1},'limitations':limits,'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()};(ROOT/'meria-tap-exile-path-coverage.json').write_text(json.dumps(ledger,indent=2)+'\n')
 (ROOT/'meria-tap-exile-review.md').write_text('# Meria two-artifact cost audit\n\nAll14 scenarios passed:10 real activations and4 unavailable-resource controls. Exile/cast/landplay, cost amount, nontoken restriction and permission lifetime match the scoped expectations. No card is promoted or globally cleared.\n\n'+'\n'.join('- '+x for x in limits)+'\n')
 print(json.dumps(counts))
if __name__=='__main__':main()
