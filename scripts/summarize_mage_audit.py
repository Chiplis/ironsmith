#!/usr/bin/env python3
"""Merge stable MAGE reports by scenario occurrence, preserving every observation."""
import argparse
import collections
import hashlib
import json
from pathlib import Path

from audit_mage_ports import ROOT, fingerprint, port_inventory


def match_report_rows(inventory, rows):
    """Prefer recorded occurrence IDs; legacy rows retain ordered matching."""
    by_id = {item['scenario_id']: item for item in inventory}
    choices = collections.defaultdict(collections.deque)
    for item in inventory:
        choices[item['test']].append(item)
    seen = set()
    for row in rows:
        if row.get('scenario_id'):
            scenario = by_id.get(row['scenario_id'])
            if scenario and scenario['test'] != row['test']:
                scenario = None
        else:
            matches = choices.get(row['test'])
            while matches and matches[0]['scenario_id'] in seen:
                matches.popleft()
            scenario = matches.popleft() if matches else None
        if scenario and scenario['scenario_id'] in seen:
            scenario = None
        if scenario:
            seen.add(scenario['scenario_id'])
        yield scenario, row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reports', type=Path, nargs='+')
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/runtime-audit/mage-cumulative.json')
    args = parser.parse_args()
    files = sorted((ROOT / 'scripts/ported-mage-tests').rglob('*.test.mjs'))
    inventory = port_inventory(files)
    observations = collections.defaultdict(list)
    reports = []
    excluded = []
    framework_rows = []
    for path in args.reports:
        report = json.loads(path.read_text())
        record = {**fingerprint(path.resolve()), 'started_at': report.get('started_at'),
                  'command': report['command'], 'engine_shims': report.get('engine_shims'),
                  'provenance_unchanged': report['provenance']['unchanged']}
        reports.append(record)
        if not record['provenance_unchanged'] or record['engine_shims'] is not False:
            excluded.append(record)
            continue
        for scenario, row in match_report_rows(inventory, report['rows']):
            if scenario is None:
                framework_rows.append({'report': record['path'], 'row': row})
                continue
            observations[scenario['scenario_id']].append({
                'report': record['path'], 'report_sha256': record['sha256'],
                'started_at': record['started_at'], 'row': row,
            })
    scenarios = []
    for scenario in inventory:
        history = sorted(observations[scenario['scenario_id']], key=lambda item: item['started_at'] or '')
        latest = history[-1]['row'] if history else None
        scenarios.append({**scenario, 'status': latest['status'] if latest else 'unobserved',
                          'classification': latest['classification'] if latest else 'not_exercised',
                          'observations': history})
    input_files = [fingerprint(path) for path in files]
    inventory_sha = hashlib.sha256(json.dumps(input_files, sort_keys=True).encode()).hexdigest()
    result = {'scope': 'Unique imported MAGE scenario occurrences across stable no-shim runs; latest observed result per scenario, not exhaustive card correctness',
              'aggregator': fingerprint(Path(__file__).resolve()),
              'reports': reports, 'excluded_reports': excluded,
              'inventory_files': len(files), 'inventory_scenarios': len(inventory),
              'inventory_files_manifest_sha256': inventory_sha,
              'status_counts': dict(collections.Counter(scenario['status'] for scenario in scenarios)),
              'classification_counts': dict(collections.Counter(scenario['classification'] for scenario in scenarios)),
              'framework_or_noninventory_rows': framework_rows, 'scenarios': scenarios,
              'limitations': ['Rows from different artifact/harness versions remain bound to their original report hashes.',
                              'A pass covers implemented assertions only; skipped, unobserved, and assertion-unverified coverage remains explicit.',
                              'Failure classifications are candidates until native or otherwise isolated reproduction confirms attribution.']}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ['status_counts', 'classification_counts']}))
    print(args.output)


if __name__ == '__main__':
    main()
