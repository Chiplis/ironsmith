#!/usr/bin/env python3
"""Reviewed first-gate reproductions for all eight typed single unattach-cost paths."""
import hashlib
import json
from pathlib import Path

ROOT = Path('reports/runtime-audit')
SOURCES = [('unattach-granted-final-execution.json', 'unattach-granted-final-attempt.json'),
           ('unattach-intrinsic-execution.json', 'unattach-intrinsic-attempt.json')]
INTENDED = {
    'Heartseeker': 'Destroy the targeted opposing Hill Giant.',
    'Leonin Bola': 'Tap the targeted opposing Hill Giant.',
    'Razor Boomerang': 'Deal one damage to Bob and return the Equipment to its owner’s hand.',
    'Shuriken': 'Deal two damage to the opposing 3/3, then transfer Equipment control except when unattached from the actual Ninja host.',
    'Surestrike Trident': 'Deal host power (two, or five after actual Giant Growth) to Bob.',
    "Toralf's Hammer": 'Deal three damage to Bob and return the Equipment to its owner’s hand.',
    'Captain America, First Avenger': 'Divide Heartseeker’s mana value of four among one, two or three targets; Captain becomes base4/4 after unattachment.',
    'Sunforger': 'Find and freely cast eligible Shock, or fail to find when no qualifying card exists, then actually shuffle. Library filter and free-cast outcomes are unreached.'}


def reference(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    inventory_path = ROOT / 'choose-consume-cost-candidates.json'
    inventory = json.loads(inventory_path.read_text())
    candidates = [r for r in inventory['rows'] if r['consumer_kind'] == 'UnattachObjectsEffect']
    assert len(candidates) == 8 and {r['card'] for r in candidates} == set(INTENDED)
    findings, controls, refs, coverage, provenance = [], [], [], {}, []
    for filename, attempt_name in SOURCES:
        path = ROOT / filename
        raw = json.loads(path.read_text())
        ref = reference(path)
        refs.append(ref)
        assert raw['provenance']['artifacts_unchanged']
        assert all(c['definition_matches_frozen_except_unique_card_ids'] for c in raw['compilation'])
        provenance.append({'source_report': ref, 'run': raw['provenance'],
                           'attempt': json.loads((ROOT / attempt_name).read_text()),
                           'strict_compilation': [{k: v for k, v in c.items() if k != 'definition'}
                                                  for c in raw['compilation']]})
        for i, row in enumerate(raw['rows']):
            assert row['status'] != 'fixture_error'
            actual, expected = row['actual'], row['expected']
            diag = row['fixture_evidence']['diagnostic']
            observed = {'action_offered': actual['action_offered'],
                        'activation_attempted': actual['action'] is not None,
                        'equipment': actual['before']['equipment'], 'host': actual['before']['host'],
                        'state_unchanged': actual['before'] == actual['after'],
                        'component_results': [x['check'] for x in diag['component_checks']]}
            index_key = ('selected_live_host_ability_index' if 'selected_live_host_ability_index' in diag
                         else 'selected_live_source_ability_index')
            observed[index_key] = diag[index_key]
            reviewed = {'card': row['card'], 'scenario': row['scenario'],
                        'source_report': ref, 'source_row': i, 'expected': expected,
                        'observed': observed, 'paid_preparation': row['fixture_evidence']['history']}
            if expected['action_offered']:
                # Review is deliberately restricted to the observed and source-audited first gate.
                # Any different outcome requires a new review; do not automatically promote it.
                assert row['status'] == 'outcome_mismatch' and not actual['action_offered']
                assert actual['action'] is None and observed['state_unchanged']
                assert actual['before']['equipment']['attached_to_host']
                assert diag[index_key] is not None
                assert observed['component_results'][-1] == 'Err(Other("unattach cost has no chosen object"))'
                assert all(x == 'Ok(())' for x in observed['component_results'][:-1])
                assert any(h.get('producer') == 'normal paid equip' for h in reviewed['paid_preparation'])
                if index_key == 'selected_live_host_ability_index':
                    assert not observed['host']['tapped'] and not observed['host']['summoning_sick']
                reviewed.update(confirmed_cards=[row['card']],
                                classification='runtime_defect_card_reproduced',
                                outcome_category='silent_wrong_result',
                                finding='A normal paid and equipped source with legal costs/targets has its exact live ability, but compute_legal_actions omits it. The unattach consumer precheck runs without the choice tag that its preceding cost selection would publish.',
                                first_reached_gate='normal activation legality; no action forced',
                                intended_unexecuted_effect=INTENDED[row['card']])
                findings.append(reviewed)
            else:
                assert row['status'] == 'expected_outcome_passed' and not actual['action_offered']
                reviewed.update(confirmed_cards=[], classification='expected_outcome_passed',
                                limitation='Unattached/fresh/tapped controls have consistent absence. The shared unattach defect can also suppress these actions, so this does not independently clear those restrictions.')
                controls.append(reviewed)
        for item in raw['path_coverage']:
            key = (item['card'], item['path'], item['consumer_path'])
            assert key not in coverage
            coverage[key] = {'source_report': ref, 'source_rows': item['source_rows']}
    assert len(findings) == 19 and len(controls) == 20
    source_audit = [
        {**reference(Path('crates/ironsmith-engine/src/game_loop/priority_state.rs')),
         'lines': [470, 684, 694], 'finding': 'Adjacent ChooseObjects costs are coalesced by choose_tagged_cost_step for selected supported consumer kinds, including single ReturnToHand. There is no UnattachObjectsEffect case.'},
        {**reference(Path('crates/ironsmith-engine/src/costs/cost_effect.rs')),
         'lines': [241, 255, 363, 411], 'finding': 'tagged_unattach_cost_precheck requires an already-published tagged object; absent/empty tags return the exact missing chosen object diagnostic before payment.'},
        {**reference(Path('crates/ironsmith-engine/src/static_abilities/continuous.rs')),
         'lines': [5970, 6131], 'finding': 'Canonical AttachedAbilityGrant is materialized for the Equipment. The fixture queries that exact grant and matches its cost/timing to the live host ability, requiring exactly one match.'}]
    report = {'scope': __doc__, 'findings': findings, 'controls': controls,
              'confirmed_cards': sorted(INTENDED),
              'counts': {'paths': 8, 'scenarios': 39, 'missing_legal_activation_observations': 19,
                         'consistent_unavailable_controls': 20, 'reproduced_card_names': 8,
                         'resolved_unattach_activations': 0},
              'source_reports': refs, 'provenance': {
                  'native_source_runs': provenance,
                  'artifacts_unchanged': all(p['run']['artifacts_unchanged'] for p in provenance),
              }, 'source_audit': source_audit,
              'superseded': [{**reference(ROOT / 'unattach-granted-execution.json'),
                             'reason': 'Preliminary report used canonical Hammer equip index2, whereas its current filtered ability list exposes the exact Equip at1. Four Hammer fixture errors are excluded. Final fixture finds Equip by canonical cost plus AttachObjectsEffect and succeeds normally; all27 rows are valid.'}],
              'limitations': ['These findings concern the first legal-action gate only. No absent action was forced; no downstream damage, destruction, tap, control-transfer, return, library search, free cast or shuffle outcome is claimed.',
                              'Frozen full canonical definitions and unique CardIds are used. Both runs have19 full definitions matching the frozen artifacts except enumerated CardDefinition IDs; binary/input/fixture hashes remain unchanged.',
                              'Equipment and creatures are normally paid casts; Equip is normally activated and paid. Six granted paths age through actual TurnRunner turns/untaps; fresh and actual paid Twiddle-tapped negatives are separate.',
                              'Current host grant indices are selected by the exact canonical grant cost/timing, not by last/first available action. Grizzly grants are0 except Trident1; Ninja Shuriken is2. Hammer equip is actual1 while canonical2 because of its filtered static ability.',
                              'Captain and Sunforger have no tap cost: fresh and actually tapped hosts should both permit activation. Captain uses actual paid/equipped Heartseeker (mana value4).',
                              'Sunforger activation remains legal even when its library has no qualifying card. Those prepared library variants reproduce only the same earlier cost gate, not search behavior.',
                              'Shuriken Ninja, Trident actual Giant Growth, and Hammer legendary-host variants reach the same missing-choice gate. Their special resolution branches remain unexecuted. Existing unrelated static/conditional-index findings are not newly attributed here.',
                              'Canonical source-only filters inside some granted costs may need separate binding review after this first gate is fixed; this report does not promote an unexecuted later defect.',
                              'Card names may already be present in the global confirmed index for other defects; reproduced_card_names is not a claim of eight newly added global names.']}
    review_path = ROOT / 'unattach-cost-reviewed-attribution.json'
    review_path.write_text(json.dumps(report, indent=2) + '\n')
    rows = []
    for candidate in candidates:
        key = (candidate['card'], candidate['path'], candidate['consumer_path'])
        item = coverage.pop(key)
        links = [f for f in findings if f['card'] == candidate['card']]
        assert links
        rows.append({**candidate, 'family': 'single_unattach_cost',
                     'source_kind': 'granted_host_activation' if '/AttachedAbilityGrant/' in candidate['path'] else 'intrinsic_activation',
                     'status': 'runtime_defect_card_reproduced', 'first_reached_gate': 'activation_legality',
                     'downstream_outcomes_executed': False, **item,
                     'reviewed_report': reference(review_path),
                     'reviewed_findings': [i for i, f in enumerate(findings) if f['card'] == candidate['card']]})
    assert not coverage
    ledger = {'family': 'single_unattach_cost', 'inventory': {**reference(inventory_path),
              'run_id': inventory['run_id'], 'records_scanned': inventory['records_scanned'],
              'retained_definitions': inventory['retained_definitions']}, 'rows': rows,
              'counts': report['counts'], 'unrun_paths': 0,
              'limitations': report['limitations'],
              'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (ROOT / 'unattach-cost-path-coverage.json').write_text(json.dumps(ledger, indent=2) + '\n')
    print(json.dumps(report['counts']))


if __name__ == '__main__':
    main()
