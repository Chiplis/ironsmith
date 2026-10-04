#!/usr/bin/env python3
"""Account for every imported scenario, including individual attempts with no result."""
import argparse
import collections
import json
from pathlib import Path

from audit_mage_ports import ROOT, fingerprint


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cumulative', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    cumulative = json.loads(args.cumulative.read_text())
    attempts = collections.defaultdict(list)
    bundles = collections.defaultdict(set)
    for recorded in cumulative['reports']:
        path = ROOT / recorded['path']
        if fingerprint(path)['sha256'] != recorded['sha256']:
            raise ValueError(f'cumulative source report changed: {path}')
        report = json.loads(path.read_text())
        if not report['provenance']['unchanged'] or report['engine_shims'] is not False:
            continue
        for bound in report['provenance']['before']:
            if bound['path'].endswith('.wasm'):
                bundles[bound['path']].add(bound['sha256'])
        for attempt in report.get('attempts', []):
            for scenario_id in attempt['scenario_ids']:
                attempts[scenario_id].append({
                    'source_report': {'path': recorded['path'], 'sha256': recorded['sha256']},
                    'scenario_started': attempt['scenario_started'],
                    'returncode': attempt['returncode'],
                    'wall_timeout': attempt['wall_timeout'],
                    'tap': attempt['tap'],
                    'command': attempt['command'],
                })
    unreported = []
    for scenario in cumulative['scenarios']:
        if scenario['status'] != 'unobserved':
            continue
        evidence = attempts[scenario['scenario_id']]
        unreported.append({
            'scenario_id': scenario['scenario_id'], 'file': scenario['file'], 'test': scenario['test'],
            'classification': ('individually_attempted_without_reported_result' if evidence
                               else 'not_individually_attempted'),
            'execution_start_observed': any(item['scenario_started'] for item in evidence),
            'attempts': evidence,
        })
    result = {
        'scope': 'Coverage accounting, not a claim that cards or failing imported scenarios are correct/defective.',
        'source_cumulative': fingerprint(args.cumulative.resolve()),
        'accounting_script': fingerprint(Path(__file__).resolve()),
        'inventory_files': cumulative['inventory_files'],
        'inventory_scenarios': cumulative['inventory_scenarios'],
        'status_counts': cumulative['status_counts'],
        'classification_counts': cumulative['classification_counts'],
        'bound_wasm_versions': {key: sorted(value) for key, value in bundles.items()},
        'every_inventory_occurrence_accounted_for': all(row['attempts'] for row in unreported),
        'unreported_individual_attempts': unreported,
        'limitations': [
            'Passed scenarios cover implemented assertions only; assertion-unverified passes remain separate.',
            'Skipped scenarios are unavailable coverage; timeouts without a TAP result remain unreported.',
            'Failure classifications are unreviewed candidates until isolated native or other execution establishes attribution.',
            'Each observation is bound to its original harness and artifact hashes; versions may differ across earlier runs.',
        ],
    }
    args.out.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in
                      ['status_counts', 'every_inventory_occurrence_accounted_for']}))


if __name__ == '__main__':
    main()
