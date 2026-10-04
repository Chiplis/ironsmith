#!/usr/bin/env python3
"""Retry unreported MAGE scenarios individually, without changing the bound harness.

Node's file timeout can prevent later tests in a long file from reporting. An
exact test-name filter isolates those remaining scenarios; unrelated name-filter
skips never become coverage. A timeout without a TAP result stays unreported.
"""
import argparse
import collections
from concurrent.futures import ThreadPoolExecutor
import datetime
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time

from audit_mage_ports import ROOT, fingerprint, parse_tap, port_inventory
from audit_mage_campaign import bound_inputs


def missing_scenarios(inventory, reports):
    by_name = collections.defaultdict(list)
    for item in inventory:
        by_name[item['test']].append(item)
    completed = set()
    pending = set()
    for report in reports:
        if not report.get('provenance', {}).get('unchanged') or report.get('engine_shims') is not False:
            continue
        for item in report.get('selected_unreported', []):
            if item.get('scenario_id'):
                pending.add(item['scenario_id'])
            else:
                pending.update(match['scenario_id'] for match in by_name[item['test']])
        occurrences = collections.Counter()
        for row in report['rows']:
            occurrence = occurrences[row['test']]
            occurrences[row['test']] += 1
            if row['status'] not in ('passed', 'failed', 'skipped'):
                continue
            if row.get('scenario_id'):
                completed.add(row['scenario_id'])
            elif occurrence < len(by_name[row['test']]):
                completed.add(by_name[row['test']][occurrence]['scenario_id'])
    return [item for item in inventory if item['scenario_id'] in pending - completed]


def selected_occurrence_rows(all_occurrences, selected, matching_rows):
    """A name filter runs every duplicate occurrence, even unselected ones."""
    selected_ids = {item['scenario_id'] for item in selected}
    return [(item, row) for item, row in zip(all_occurrences, matching_rows)
            if item['scenario_id'] in selected_ids]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--campaign', type=Path, required=True)
    parser.add_argument('--resume-from', type=Path, action='append', default=[])
    parser.add_argument('--jobs', type=int, default=3)
    parser.add_argument('--test-timeout-ms', type=int, default=60000)
    parser.add_argument('--wall-timeout-seconds', type=int, default=75)
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/runtime-audit/mage-unreported-retry.json')
    parser.add_argument('--exclude-attempted-from', type=Path, action='append', default=[],
                        help='retain stable individual timeouts as unreported without an identical retry')
    args = parser.parse_args()
    manifest = json.loads(args.campaign.read_text())
    paths = [ROOT / item['path'] for item in manifest['prior_reports']]
    paths += [ROOT / item['report'] for item in manifest['batches'] if item['status'] == 'reported']
    paths += [path.resolve() for path in args.resume_from]
    reports = [json.loads(path.read_text()) for path in paths]
    inventory = port_inventory(sorted((ROOT / 'scripts/ported-mage-tests').rglob('*.test.mjs')))
    planned = missing_scenarios(inventory, reports)
    current_bound = bound_inputs()
    excluded_ids = set()
    for prior in args.exclude_attempted_from:
        report = json.loads(prior.read_text())
        if (report.get('engine_shims') is False
                and report.get('provenance', {}).get('unchanged')
                and report['provenance']['before'][:len(current_bound)] == current_bound):
            excluded_ids.update(sid for attempt in report.get('attempts', [])
                                for sid in attempt['scenario_ids'])
    excluded = [item for item in planned if item['scenario_id'] in excluded_ids]
    planned = [item for item in planned if item['scenario_id'] not in excluded_ids]
    groups = collections.defaultdict(list)
    for item in planned:
        groups[(item['file'], item['test'])].append(item)
    args.output = args.output.resolve()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    tap_dir = args.output.with_suffix('')
    tap_dir.mkdir(exist_ok=True)
    sources = [Path(__file__).resolve(), ROOT / 'scripts/audit_mage_campaign.py']
    sources += [ROOT / file for file in sorted({item['file'] for item in planned})]
    before = current_bound + [fingerprint(path) for path in sources]
    environment = {key: value for key, value in os.environ.items() if not key.startswith('MAGE_PORT_')}
    environment.update(MAGE_PORT_ALLOW_ENGINE_SHIMS='0', MAGE_PORT_TEST_START_TRACE='1')
    started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
    start = time.monotonic()

    def run_group(task):
        index, ((file, title), scenarios) = task
        tap = tap_dir / f'{index:04d}.tap'
        command = ['node', '--test', '--test-concurrency=1',
                   f'--test-timeout={args.test_timeout_ms}', '--test-reporter=tap',
                   f'--test-name-pattern=^{re.escape(title)}$', file]
        wall_timeout = False
        with tap.open('w') as stream:
            child = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=stream,
                                     stderr=subprocess.STDOUT, start_new_session=True)
            try:
                returncode = child.wait(timeout=args.wall_timeout_seconds)
            except subprocess.TimeoutExpired:
                wall_timeout = True
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    returncode = child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    returncode = child.wait()
        contents = tap.read_text()
        all_rows = parse_tap(contents)
        matches = [row for row in all_rows if row['test'] == title]
        rows = []
        all_occurrences = [item for item in inventory
                           if item['file'] == file and item['test'] == title]
        for item, row in selected_occurrence_rows(all_occurrences, scenarios, matches):
            if row['status'] == 'missing_result':
                continue
            if row['status'] == 'skipped' and not item.get('upstream_skip'):
                # A name-filter mismatch must never masquerade as an authored
                # skip/completed occurrence, even if an escaping bug caused it.
                continue
            row.update(scenario_id=item['scenario_id'], file=file,
                       declared_direct_assertions=item['direct_assertions'])
            if row['status'] == 'passed' and not item['direct_assertions']:
                row['classification'] = 'test_completed_assertion_coverage_unverified'
            rows.append(row)
        return {'command': command, 'returncode': returncode, 'wall_timeout': wall_timeout,
                'scenario_ids': [item['scenario_id'] for item in scenarios],
                'scenario_started': f'[mage-port-test-start] {title}' in contents,
                'tap': fingerprint(tap), 'rows': rows,
                'nonselected_or_framework_rows': [row for row in all_rows if row['test'] != title]}

    attempts = []
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for attempt in pool.map(run_group, enumerate(groups.items())):
            attempts.append(attempt)
            # A crash-safe checkpoint preserves genuine scenario rows, but is
            # deliberately ineligible for coverage until final hash validation.
            checkpoint_rows = [row for done in attempts for row in done['rows']]
            checkpoint = {'status': 'running_unverified_checkpoint', 'engine_shims': False,
                          'provenance': {'before': before, 'after': None, 'unchanged': False},
                          'rows': checkpoint_rows, 'attempts': attempts,
                          'selected_scenarios': len(planned),
                          'excluded_previously_attempted': excluded}
            checkpoint_path = args.output.with_suffix('.tmp')
            checkpoint_path.write_text(json.dumps(checkpoint, indent=2) + '\n')
            checkpoint_path.replace(args.output)
            print(json.dumps({'completed_groups': len(attempts), 'groups': len(groups),
                              'results': len(attempt['rows']),
                              'scenario_started': attempt['scenario_started']}), flush=True)
    after = bound_inputs() + [fingerprint(path) for path in sources]
    rows = [row for attempt in attempts for row in attempt['rows']]
    observed = {row['scenario_id'] for row in rows}
    selected = {item['scenario_id'] for item in planned}
    report = {'scope': 'Individual retries of previously unreported imported MAGE scenarios; unrelated name-filter skips are excluded',
              'started_at': started_at, 'duration_seconds': time.monotonic() - start,
              'command': [attempt['command'] for attempt in attempts], 'engine_shims': False,
              'inventory_scenarios': len(inventory), 'selected_scenarios': len(planned),
              'selected_files': len({item['file'] for item in planned}),
              'selected_unreported': [item for item in planned if item['scenario_id'] not in observed],
              'unselected_scenarios': [item for item in inventory if item['scenario_id'] not in selected],
              'provenance': {'before': before, 'after': after, 'unchanged': before == after},
              'resume_sources': [fingerprint(path) for path in paths],
              'excluded_previously_attempted': excluded,
              'excluded_attempt_sources': [fingerprint(path.resolve()) for path in args.exclude_attempted_from],
              'attempts': attempts,
              'rows': rows, 'status_counts': dict(collections.Counter(row['status'] for row in rows)),
              'classification_counts': dict(collections.Counter(row['classification'] for row in rows)),
              'limitations': ['Timeouts without an actual TAP scenario result remain unreported, even when the start trace proves the scenario began.',
                              'Duplicate scenario titles within one file execute together and remain separate occurrences.',
                              'No engine defect is inferred merely from an imported test failure.']}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'selected': len(planned), 'unreported': len(report['selected_unreported']),
                      'status_counts': report['status_counts'], 'unchanged': before == after}))


if __name__ == '__main__':
    main()
