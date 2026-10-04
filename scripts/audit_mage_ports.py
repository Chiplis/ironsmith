#!/usr/bin/env python3
"""Run selected MAGE WASM ports without engine shims and retain bounded evidence.

Example: python3 scripts/audit_mage_ports.py 'scripts/ported-mage-tests/cards/watchers/*.test.mjs'
Categories describe observed failures, not proof of an engine defect. Passing a
ported test only covers the assertions actually implemented by that port.
"""
import argparse
import collections
import datetime
import glob
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def fingerprint(path):
    path = Path(path)
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    return {'path': str(path.relative_to(ROOT)), 'sha256': digest,
            'size': path.stat().st_size, 'mtime_ns': path.stat().st_mtime_ns}


def classify(error):
    if re.search(r'timed out|cancelled|canceled', error, re.I):
        return 'execution_timeout_or_cancellation'
    if re.search(r'parser does not|unsupported (trigger|predicate)|parse error|parse failed|\[rule-path=', error, re.I):
        return 'parser_failure_before_expected_outcome'
    if 'starting decks and sideboards cannot be changed by manual card injection' in error:
        return 'unsupported_fixture_injection'
    if 'unsupported ' in error:
        return 'unsupported_harness_or_engine_operation'
    if 'distribution must assign' in error:
        return 'decision_fixture_error'
    if re.search(r'Cannot resolve|Resolution failed|dispatch failed', error):
        return 'runtime_error_candidate'
    if re.search(r'^expected |^could not find ', error, re.I):
        return 'outcome_or_fixture_mismatch_candidate'
    return 'unclassified_failure'


def parse_tap(contents):
    rows = []
    chunks = re.split(r'(?=^# Subtest: )', contents, flags=re.M)
    for chunk in chunks:
        if not chunk.startswith('# Subtest: '):
            continue
        lines = chunk.splitlines()
        title = lines[0].removeprefix('# Subtest: ')
        result = next((line for line in lines[1:] if re.match(r'^(?:not )?ok \d+', line)), None)
        if result is None:
            rows.append({'test': title, 'status': 'missing_result', 'classification': 'unavailable'})
            continue
        passed = result.startswith('ok ')
        skipped = '# SKIP' in result
        error = ''
        for index, line in enumerate(lines):
            if line.startswith('  error: '):
                error = line.removeprefix('  error: ')
                if error in ('|-', '|'):
                    error = lines[index + 1].strip() if index + 1 < len(lines) else ''
                elif error.startswith('"'):
                    try:
                        error = json.loads(error)
                    except json.JSONDecodeError:
                        pass
                else:
                    error = error.strip("'")
                break
        duration = re.search(r'^  duration_ms: ([\d.]+)', chunk, flags=re.M)
        source, _, name = title.partition(' :: ')
        rows.append({'test': title, 'source': source, 'scenario': name,
                     'status': 'skipped' if skipped else 'passed' if passed else 'failed',
                     'classification': 'unavailable' if skipped else 'assertions_completed' if passed else classify(error),
                     'error': error or None,
                     'duration_ms': float(duration.group(1)) if duration else None})
    return rows


def port_inventory(files):
    scenarios = []
    for path in files:
        text = path.read_text()
        marker = 'registerPortedMageTests('
        spec = json.loads(text[text.index(marker) + len(marker):text.rindex(');')])
        for index, scenario in enumerate(spec['tests']):
            scenarios.append({'test': spec['sourcePath'] + ' :: ' + scenario['name'],
                              'file': str(path.relative_to(ROOT)),
                              'scenario_id': str(path.relative_to(ROOT)) + '#' + str(index),
                              'upstream_skip': scenario.get('skip'),
                              'direct_assertions': sum(op.get('op', '').startswith('assert')
                                                       for op in scenario.get('operations', []))})
    return scenarios


def completed_files_from_reports(inventory, reports):
    observed = collections.Counter()
    for report in reports:
        if not report.get('provenance', {}).get('unchanged'):
            continue
        counts = collections.Counter(row['test'] for row in report['rows']
                                     if row['status'] in ('passed', 'failed', 'skipped'))
        for name, count in counts.items():
            observed[name] = max(observed[name], count)
    required = collections.defaultdict(collections.Counter)
    for scenario in inventory:
        required[scenario['file']][scenario['test']] += 1
    return {file for file, names in required.items()
            if all(observed[name] >= count for name, count in names.items())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('patterns', nargs='+', help='test files or glob patterns relative to repository')
    parser.add_argument('--jobs', type=int, default=2)
    parser.add_argument('--test-timeout-ms', type=int, default=60000)
    parser.add_argument('--wall-timeout-seconds', type=int, default=900)
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/runtime-audit/mage-execution.json')
    parser.add_argument('--resume-from', type=Path, action='append', default=[],
                        help='skip files whose every scenario has a final result in stable prior reports')
    args = parser.parse_args()
    files = list(dict.fromkeys(Path(path).resolve() for pattern in args.patterns
                    for path in sorted(glob.glob(str(ROOT / pattern), recursive=True))))
    inventory = port_inventory(sorted((ROOT / 'scripts/ported-mage-tests').rglob('*.test.mjs')))
    completed_files = completed_files_from_reports(inventory,
        [json.loads(path.read_text()) for path in args.resume_from])
    files = [file for file in files if str(file.relative_to(ROOT)) not in completed_files]
    if not files:
        parser.error('no matching unreported test files')
    sources = [ROOT / 'scripts/mage-port-runner.mjs',
               ROOT / 'scripts/wasm-test-harness.mjs', Path(__file__).resolve(), ROOT / 'cards.json']
    sources += sorted((ROOT / 'scripts/mage-port-runner').glob('*.mjs'))
    artifacts = sorted((ROOT / 'pkg').glob('*.wasm')) + sorted((ROOT / 'pkg').glob('*.js'))
    before = [fingerprint(path) for path in sources + artifacts + files]
    selected = set(str(path.relative_to(ROOT)) for path in files)
    planned = [scenario for scenario in inventory if scenario['file'] in selected]
    command = ['node', '--test', f'--test-concurrency={args.jobs}',
               f'--test-timeout={args.test_timeout_ms}', '--test-reporter=tap',
               *[str(path.relative_to(ROOT)) for path in files]]
    environment = {key: value for key, value in os.environ.items() if not key.startswith('MAGE_PORT_')}
    environment['MAGE_PORT_ALLOW_ENGINE_SHIMS'] = '0'
    args.output.parent.mkdir(parents=True, exist_ok=True)
    tap = args.output.with_suffix('.tap')
    start = time.monotonic()
    started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
    timed_out = False
    with tap.open('w') as stream:
        child = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=stream,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        try:
            returncode = child.wait(timeout=args.wall_timeout_seconds)
        except subprocess.TimeoutExpired:
            import signal
            timed_out = True
            os.killpg(child.pid, signal.SIGTERM)
            try:
                returncode = child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                returncode = child.wait()
    after = [fingerprint(path) for path in sources + artifacts + files]
    rows = parse_tap(tap.read_text())
    declared = collections.defaultdict(collections.deque)
    for scenario in planned:
        declared[scenario['test']].append(scenario)
    observed = set()
    for row in rows:
        choices = declared.get(row['test'])
        if choices:
            scenario = choices.popleft()
            row['scenario_id'] = scenario['scenario_id']
            observed.add(scenario['scenario_id'])
            row['file'] = scenario['file']
            row['declared_direct_assertions'] = scenario['direct_assertions']
            if row['status'] == 'passed' and not scenario['direct_assertions']:
                row['classification'] = 'test_completed_assertion_coverage_unverified'
    report = {'scope': 'Selected ported MAGE scenarios, not all-card correctness',
              'started_at': started_at, 'duration_seconds': time.monotonic() - start,
              'command': command, 'engine_shims': False, 'returncode': returncode,
              'wall_timeout': timed_out, 'selected_files': len(files),
              'inventory_scenarios': len(inventory), 'selected_scenarios': len(planned),
              'selected_unreported': [scenario for scenario in planned if scenario['scenario_id'] not in observed],
              'unselected_scenarios': [scenario for scenario in inventory if scenario['file'] not in selected],
              'resume_sources': [fingerprint(path.resolve()) for path in args.resume_from],
              'resumed_completed_files': sorted(completed_files),
              'status_counts': dict(collections.Counter(row['status'] for row in rows)),
              'classification_counts': dict(collections.Counter(row['classification'] for row in rows)),
              'provenance': {'before': before, 'after': after, 'unchanged': before == after},
              'tap_path': str(tap), 'rows': rows,
              'limitations': ['Failures require separation of engine bugs from port/fixture differences.',
                              'Passing ports cover implemented assertions, not all semantics.',
                              'Only emitted TAP results are rows; wall timeout or process crash can leave selected scenarios unrun.']}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({key: report[key] for key in ['status_counts', 'classification_counts', 'wall_timeout']}))
    print(args.output)
    return 0 if returncode == 0 and before == after else 1


if __name__ == '__main__':
    raise SystemExit(main())
