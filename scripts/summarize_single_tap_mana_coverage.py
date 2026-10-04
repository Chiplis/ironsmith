#!/usr/bin/env python3
"""Review mana single-tap outcomes separately from advertised-but-unpayable negative states."""
import hashlib
import json
from collections import Counter
from pathlib import Path
ROOT=Path('reports/runtime-audit')

def ref(path): return {'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}

def main():
    cases={c['card']:c for c in json.loads((ROOT/'single-tap-mana-frozen-inputs.json').read_text())['cases']}
    inventory=json.loads((ROOT/'single-tap-cost-inventory.json').read_text())
    candidates=[r for r in inventory['rows'] if r['subgroup']=='mana']
    assert len(candidates)==16
    path=ROOT/'single-tap-mana-execution.json'; edgepath=ROOT/'single-tap-mana-edge-execution.json'
    raw=json.loads(path.read_text());edge=json.loads(edgepath.read_text());sources=[ref(path),ref(edgepath)]
    assert len(raw['rows'])==96 and len(edge['rows'])==11
    for data in [raw,edge]:
        assert data['provenance']['artifacts_unchanged']
        assert all(c['definition_matches_frozen_except_unique_card_ids'] for c in data['compilation'])
    controls=[]; availability=[]; mapping={c:[] for c in cases}
    for i,row in enumerate(raw['rows']):
        assert row['status']!='fixture_error'
        failed=[c for c in row['checks'] if c['expected']!=c['observed']]
        assert all(c['check'] in ['exact_mana_ability_offered','restricted_mana_cannot_cast_nonartifact'] for c in failed)
        assert row['fixture_evidence']['canonical_index']==cases[row['card']]['index']
        mapping[row['card']].append({'source_report':sources[0],'source_row':i,'scenario':row['scenario'],'actual_activation_executed':row['actual']['action'] is not None,'availability_mismatch':bool(failed)})
        if not failed:
            controls.append({'card':row['card'],'scenario':row['scenario'],'classification':'expected_outcome_passed','confirmed_cards':[],'source_report':sources[0],'source_row':i,'expected':row['expected'],'observed':row['actual'],'checks':row['checks']})
        else:
            availability.append({'card':row['card'],'scenario':row['scenario'],'classification':'negative_state_availability_inconsistency','confirmed_cards':[],'source_report':sources[0],'source_row':i,'expected':row['expected'],'observed':row['actual'],'failed_checks':failed,'scope':'Not a legal payable gameplay scenario; no card-resolution defect promotion. Corresponding actual advertised follow-up payment is separately retained.'})
    for i,row in enumerate(edge['rows']):
        assert row['status']=='outcome_mismatch'
        if row['card']=='Grand Architect':
            spent=row['actual']['spending_controls']
            assert spent['nonartifact_cast_offered']
            assert 'NoLegalPlan' in spent['actual_nonartifact_attempt']['announcement_error']
            assert spent['grizzly_battlefield_before']==spent['grizzly_battlefield_after']==0
            assert 'green: 1, colorless: 2' in spent['remaining_pool']
            conclusion='The restricted mana is enforced by payment: the advertised nonartifact spell cancels before payment/entry. Valid artifact casts in the main run succeed. No restriction bypass is reproduced.'
        else:
            assert row['scenario']=='zero' and row['actual']['action_offered']
            assert 'Not enough objects to choose (1 needed, 0 available)' in row['actual']['action']['resolution_error']
            assert row['actual']['source_tapped_after'] is False
            assert row['actual']['pool_after_activation']=='ManaPool { white: 0, blue: 0, black: 0, red: 0, green: 0, colorless: 0 }'
            conclusion='The sole permanent cannot satisfy both its tap symbol and another untapped-object payment. The advertised action fails, produces no mana and restores its untapped state; no invalid resource payment succeeds.'
        availability.append({'card':row['card'],'scenario':row['scenario'],'classification':'advertised_unpayable_action_rejected_with_rollback','confirmed_cards':[],'source_report':sources[1],'source_row':i,'expected':row['expected'],'observed':row['actual'],'finding':conclusion})
        mapping[row['card']].append({'source_report':sources[1],'source_row':i,'scenario':row['scenario'],'edge_followup':True,'scope':conclusion})
    paths=[]
    for c in candidates:
        ev=mapping[c['card']]
        mismatch=any(r.get('availability_mismatch')for r in ev)
        paths.append({**c,'status':'scoped_payment_passed_with_availability_mismatch' if mismatch else 'scoped_expected_outcomes_passed','source_evidence':ev,'scope':'Mana amount/color and actual resource payment; malformed zero-resource advertisements are kept as separate negative-state observations, not legal card-runtime failures.'})
    counts={'paths':16,'primary_scenarios':96,'edge_followups':11,'passing_primary_scenarios':len(controls),'negative_state_availability_mismatch_primary_scenarios':11,'valid_mana_activations':sum(r['expected']['action_offered'] and r['actual']['action'] is not None for r in raw['rows']),'primary_oracle_checks':sum(len(r['checks'])for r in raw['rows']),'confirmed_cards':0,'unrun_paths':0}
    limits=['All25 full canonical definitions in both binaries match frozen definitions except enumerated unique CardIds. Provenance remains stable within each run.','Sources and resources are normal paid casts or real land plays. Aging uses real TurnRunner turns/untaps. Fresh-resource, source-tapped, zero/exact/surplus and all-tapped branches are distinct.','Grand Architect produces restricted CC correctly; five actual artifact-spend controls pass. Five advertised nonartifact casts are rejected with NoLegalPlan, preserving mana and preventing battlefield entry. This is not a restriction-bypass finding.','Six sole-source tap-symbol plus chosen-tap advertisements are rejected during payment and rolled back. They are negative fixture states, not proof of failure of a legal card ability.','Urza zero-additional-resource positive uses its normally created artifact Construct. Meria uses nontoken Ornithopters; token eligibility is covered by a separate Meria two-artifact follow-up.','No whole-card clearance. The seven paths with advertised-unpayable inconsistencies carry a distinct status; interface consistency remains unresolved.']
    review={'scope':__doc__,'findings':[],'confirmed_cards':[],'controls':controls,'availability_observations':availability,'counts':counts,'path_coverage':paths,'source_reports':sources,'provenance':{'artifacts_unchanged':True,'native_source_runs':[{'source_report':sources[0],'run':raw['provenance'],'attempt':json.loads((ROOT/'single-tap-mana-attempt.json').read_text())},{'source_report':sources[1],'run':edge['provenance'],'attempt':json.loads((ROOT/'single-tap-mana-edge-attempt.json').read_text())}]},'limitations':limits}
    rp=ROOT/'single-tap-mana-reviewed-attribution.json';rp.write_text(json.dumps(review,indent=2)+'\n')
    ledger={'family':'single_tap_cost','subfamily':'mana','path_count':16,'rows':paths,'counts':counts,'reviewed_sources':[ref(rp)],'status_counts':dict(Counter(r['status']for r in paths)),'limitations':limits,'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (ROOT/'single-tap-mana-path-coverage.json').write_text(json.dumps(ledger,indent=2)+'\n')
    (ROOT/'single-tap-mana-review.md').write_text('# Mana single-tap cost audit\n\nAll16 paths have valid payment coverage. Of96 primary scenarios,85 fully match and11 expose negative-state action advertisements. Eleven bounded advertised-action follow-ups reject the unpayable actions correctly. No new card defect is promoted.\n\n'+'\n'.join('- '+s for s in limits)+'\n')
    print(json.dumps(counts))
if __name__=='__main__':main()
