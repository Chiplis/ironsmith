#!/usr/bin/env python3
"""Join all twenty single ReturnToHand cost paths to reviewed paid-action evidence."""
import hashlib
import json
from collections import Counter
from pathlib import Path

ROOT = Path('reports/runtime-audit')


def reference(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    inventory_path = ROOT / 'choose-consume-cost-candidates.json'
    inventory = json.loads(inventory_path.read_text())
    candidates = [r for r in inventory['rows']
                  if r['count_shape'] == 'single' and r['consumer_kind'] == 'ReturnToHandEffect']
    evidence, refs = {}, []
    scenario_count = positive = negative = checks = 0
    for filename in ['single-return-land-reviewed-attribution.json',
                     'single-return-special-reviewed-attribution.json']:
        path = ROOT / filename
        review = json.loads(path.read_text())
        ref = reference(path)
        refs.append(ref)
        for coverage in review['path_coverage']:
            key = (coverage['card'], coverage['path'], coverage['consumer_path'])
            assert key not in evidence, key
            raw_path = Path(coverage['source_report']['path'])
            assert reference(raw_path) == coverage['source_report']
            raw = json.loads(raw_path.read_text())
            assert raw['provenance']['artifacts_unchanged']
            assert all(c['definition_matches_frozen_except_unique_card_ids']
                       for c in raw['compilation'])
            scenarios = []
            for index in coverage['source_rows']:
                row = raw['rows'][index]
                ability_index = int(coverage['path'].split('/')[3])
                assert row['card'] == coverage['card']
                assert row['fixture_evidence']['diagnostic']['ability_index'] == ability_index
                assert row['status'] == 'expected_outcome_passed'
                assert all(c['expected'] == c['observed'] for c in row['checks'])
                scenarios.append({'source_row': index, 'scenario': row['scenario'],
                                  'expected_action_offered': row['expected']['action_offered'],
                                  'actual_action_offered': row['actual']['action_offered'],
                                  'status': row['status']})
                scenario_count += 1
                positive += row['expected']['action_offered'] is True
                negative += row['expected']['action_offered'] is False
                checks += len(row['checks'])
            evidence[key] = {'reviewed_report': ref, 'source_report': coverage['source_report'],
                             'source_ability_index': ability_index,
                             'status': 'scoped_expected_outcomes_passed', 'scenarios': scenarios}
    rows = []
    for candidate in candidates:
        key = (candidate['card'], candidate['path'], candidate['consumer_path'])
        assert key in evidence, f'Missing reviewed path: {key}'
        rows.append({**candidate, 'family': 'single_return_cost', **evidence.pop(key)})
    assert not evidence, f'Reviewed evidence outside inventory: {evidence.keys()}'
    report = {'family': 'single_return_cost', 'scope': __doc__,
              'inventory': {**reference(inventory_path), 'run_id': inventory['run_id'],
                            'records_scanned': inventory['records_scanned'],
                            'retained_definitions': inventory['retained_definitions']},
              'reviewed_sources': refs, 'rows': rows, 'path_count': len(rows),
              'status_counts': dict(Counter(r['status'] for r in rows)),
              'counts': {'paths': len(rows), 'primary_scenarios': scenario_count,
                         'actual_successful_activations': positive,
                         'expected_unavailable_controls': negative, 'oracle_checks': checks,
                         'new_confirmed_cards': 0, 'unrun_paths': 0},
              'limitations': ['This is scoped branch coverage, not whole-card clearance.',
                              'A single-object choose/return helper supports these paths; fixed multi-return failures must not be generalized here.',
                              'Source identity, costs, real resource producers and exact effects are documented in the reviewed reports.',
                              'No unavailable action is forced and no assertion repairs game state.',
                              'Preliminary special-batch report is superseded and excluded from counts.'],
              'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (ROOT / 'single-return-cost-path-coverage.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['counts']))


if __name__ == '__main__':
    main()
