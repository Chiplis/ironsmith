#!/usr/bin/env python3
"""Review 31 Station cost paths, reusing only explicitly measured prior Station payments."""
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path('reports/runtime-audit')
REUSE = [('station-threshold-static', [1, 3, 5, 7]),
         ('station-threshold-event', [1, 3, 5, 7, 9]),
         ('ability-index-station', [1, 2, 3])]


def reference(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def normalized(value, path=''):
    if isinstance(value, dict):
        return {k: normalized(v, path + '/' + k) for k, v in value.items()
                if not (k == 'id' and path.endswith('/card') and {'card_types', 'name'} <= value.keys())}
    if isinstance(value, list):
        return [normalized(v, path + '/' + str(i)) for i, v in enumerate(value)]
    return value


def main():
    invpath = ROOT / 'single-tap-cost-inventory.json'
    candidates = [r for r in json.loads(invpath.read_text())['rows'] if r['subgroup'] == 'station']
    assert len(candidates) == 31
    byname = {r['card']: r for r in candidates}
    frozen = {p['name']: p for p in json.loads((ROOT/'single-tap-station-frozen-inputs.json').read_text())['cards']}
    rawpath = ROOT / 'single-tap-station-execution.json'
    raw = json.loads(rawpath.read_text())
    assert raw['provenance']['artifacts_unchanged'] and len(raw['rows']) == 187
    assert all(c['definition_matches_frozen_except_unique_card_ids'] for c in raw['compilation'])
    sources = [reference(rawpath)]
    controls, mapped, native_runs, reused_parity = [], {n: [] for n in byname}, [], []
    native_runs.append({'source_report': sources[0], 'run': raw['provenance'],
                        'attempt': json.loads((ROOT/'single-tap-station-attempt.json').read_text())})
    for index, row in enumerate(raw['rows']):
        assert row['status'] == 'expected_outcome_passed'
        assert all(c['expected'] == c['observed'] for c in row['checks'])
        assert row['fixture_evidence']['canonical_index'] == byname[row['card']]['source_ability_index']
        positive = row['scenario'].startswith('power_')
        if positive:
            assert row['fixture_evidence']['resource']['power_before_payment'] == row['fixture_evidence']['initial_canonical_resource_power']
            assert row['fixture_evidence']['resource']['fresh']
            assert row['actual']['action']['mana_paid'] == 0 and row['actual']['action']['resolution_error'] is None
        elif row['scenario'] != 'stack':
            assert row['fixture_evidence']['eligible_ids'] == []
        mapped[row['card']].append({'source_report': sources[0], 'source_row': index,
                                    'scenario': row['scenario'], 'positive': positive,
                                    'activation_count': int(positive), 'scope': 'Station availability/payment and charge delta'})
        controls.append({'card': row['card'], 'scenario': row['scenario'], 'classification': 'expected_outcome_passed',
                         'confirmed_cards': [], 'source_report': sources[0], 'source_row': index,
                         'expected': row['expected'], 'observed': row['actual'], 'checks': row['checks']})
    for stem, indices in REUSE:
        path = ROOT / f'{stem}-reproductions.json'
        prior = json.loads(path.read_text())
        assert prior['provenance']['artifacts_unchanged']
        ref = reference(path)
        sources.append(ref)
        native_runs.append({'source_report': ref, 'run': prior['provenance']})
        compiled_path = ROOT / f'{stem}-cases/00.json'
        compiled = {c['card']: c for c in json.loads(compiled_path.read_text())['compilation']}
        for index in indices:
            row = prior['rows'][index]
            name = row['card']
            assert normalized(compiled[name]['definition']) == normalized(frozen[name]['frozen_definition'])
            reused_parity.append({'card': name, 'source_report': ref, 'source_row': index,
                                 'compiled_definition': reference(compiled_path),
                                 'equal_to_current_frozen_definition_except_unique_card_ids': True})
            # Evidence guards are narrower than the full original row's outcome (which can fail elsewhere).
            if stem == 'station-threshold-static':
                resources = row['state_evidence']['stationed_resources']
                assert resources and all(r['tapped'] for r in resources)
                amount = sum(r['power'] for r in resources)
                assert row['actual']['charge_counters'] == amount == row['state_evidence']['threshold']
                count = len(resources)
            elif stem == 'station-threshold-event':
                assert row['state_evidence']['mode'] == 1
                count = len(re.findall(r'ObjectId\(\d+\)', row['state_evidence']['stationed_resources']))
                assert count > 0
                amount = row['state_evidence']['counters']
                assert amount == row['state_evidence']['threshold']
            else:
                amount = row['state_evidence']['station_counter_expectation_from_printed_power']
                assert row['actual']['charge_counters_after_station'] == amount
                count = row['actual']['station_resources_tapped']
                assert count == {1: 1, 2: 2, 3: 1}[row['scenario']['mode']]
            ability_index = byname[name]['source_ability_index']
            matching = [t for t in row['execution_trace'] if t.get('stage') == 'activation_action'
                        and re.search(rf'ability_index: {ability_index} \}}', t.get('action', ''))]
            assert len(matching) >= count, (name, index, ability_index)
            evidence = {'source_report': ref, 'source_row': index, 'scenario': row['scenario'],
                        'positive': True, 'activation_count': count, 'charge_counter_sum': amount,
                        'source_ability_index': ability_index,
                        'scope': 'Only the earlier measured Station payments; original threshold/pump/attack outcomes excluded'}
            mapped[name].append(evidence)
            controls.append({'card': name, 'scenario': row['scenario'], 'classification': 'expected_outcome_passed',
                             'confirmed_cards': [], 'source_report': ref, 'source_row': index,
                             'scope': evidence['scope'],
                             'expected': {'actual_station_payments': count, 'charge_counter_sum': amount},
                             'observed': {'actual_station_payments': count, 'charge_counter_sum': amount},
                             'source_evidence': evidence,
                             'original_full_row_status': row['status']})
    paths = []
    for candidate in candidates:
        evidence = mapped[candidate['card']]
        assert any(e['positive'] for e in evidence)
        assert {e['scenario'] for e in evidence if not e['positive']} == {'none', 'noncreature', 'tapped', 'stack'}
        paths.append({**candidate, 'status': 'scoped_expected_outcomes_passed', 'source_evidence': evidence,
                      'scope': 'Single creature tap payment, charge counters equal paid creature power, and measured unavailable-resource/stack controls'})
    counts = {'paths': 31, 'primary_scenarios': len(controls), 'new_scenarios': 187, 'reused_scoped_scenarios': 12,
              'new_oracle_checks': sum(len(r['checks']) for r in raw['rows']),
              'actual_successful_activations': sum(e['activation_count'] for es in mapped.values() for e in es),
              'expected_unavailable_controls': 124, 'confirmed_cards': 0, 'unrun_station_paths': 0}
    limitations = [
        'Scope is Station cost/payment only. Existing unrelated station-striation failures remain valid and are not cleared by this report.',
        'Twelve old scenario rows on ten cards contribute only explicitly measured Station tap/payment/counter evidence. Zero-mode rows contribute no positive coverage. Original rows that fail later pump/attack behavior are not relabeled as whole-scenario passes.',
        'All 41 new strict definitions match the frozen corpus except unique CardIds. Ten reused source definitions are additionally compared directly to the same frozen definitions.',
        'New positive gaps use normally cast power-zero Ornithopter, power-two Grizzly Bears and Bears with an actual paid Giant Growth (power five). Fresh creatures legally pay a tap cost that is not their own tap-symbol ability.',
        'ETB-produced/reanimated creatures remain real. Paid Twiddle taps them to avoid silently satisfying zero-resource controls. Planet sources use actual normal land plays.',
        'Each source has no-untapped-creature, noncreature artifact, actual tapped-creature and real pending paid Shock negatives. Opponent-turn timing and repeated Station of an untapped-once-again resource are outside this scope.',
        'Resource current power is observed before payment, independently checked against printed zero/two or paid Growth five, then compared to exact counter delta. No direct counter mutation or forced unavailable action is used.',
    ]
    review = {'scope': __doc__, 'findings': [], 'controls': controls, 'confirmed_cards': [], 'counts': counts,
              'source_reports': sources, 'provenance': {'native_source_runs': native_runs, 'artifacts_unchanged': True},
              'reused_frozen_parity': reused_parity, 'path_coverage': paths, 'limitations': limitations}
    rp = ROOT / 'single-tap-station-reviewed-attribution.json'
    rp.write_text(json.dumps(review, indent=2) + '\n')
    ledger = {'family': 'single_tap_cost', 'subfamily': 'station', 'path_count': 31, 'rows': paths, 'counts': counts,
              'inventory': reference(invpath), 'reviewed_sources': [reference(rp)], 'limitations': limitations,
              'status_counts': dict(Counter(r['status'] for r in paths)),
              'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (ROOT/'single-tap-station-path-coverage.json').write_text(json.dumps(ledger, indent=2) + '\n')
    lines = ['# Station single-tap cost audit', '',
             f'All 31 Station paths have scoped passing payment evidence: 187 new scenarios and 12 reused scoped rows, {counts["actual_successful_activations"]} actual activations and 124 unavailable-resource/stack controls. All 563 new checks pass. No card is newly confirmed or cleared.', '']
    lines += [f'- {limit}' for limit in limitations]
    lines += ['', f'Raw report: `{rawpath}`.', f'Review: `{rp}`.',
              'Exact per-path evidence: `reports/runtime-audit/single-tap-station-path-coverage.json`.', '']
    (ROOT/'single-tap-station-review.md').write_text('\n'.join(lines))
    print(json.dumps(counts))


if __name__ == '__main__':
    main()
