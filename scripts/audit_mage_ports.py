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
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
TERMINAL_STATUSES = {'passed', 'failed', 'skipped'}
WASM_TRAP = re.compile(
    r'^(?:memory access out of bounds|unreachable|maximum call stack size exceeded|recursive use of an object.*)$',
    re.I,
)


def fingerprint(path):
    path = Path(path).resolve()
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    return {'path': str(path.relative_to(ROOT) if path.is_relative_to(ROOT) else path), 'sha256': digest,
            'size': path.stat().st_size, 'mtime_ns': path.stat().st_mtime_ns}


def test_artifacts():
    package = ROOT / (os.environ.get('IRONSMITH_TEST_WASM_PKG') or 'pkg')
    return sorted(package.glob('*.wasm')) + sorted(package.glob('*.js')) + [package / 'package.json']


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
        title = unescape_tap(lines[0].removeprefix('# Subtest: '))
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
                elif error.startswith("'") and error.endswith("'"):
                    error = error[1:-1].replace("''", "'")
                break
        duration = re.search(r'^  duration_ms: ([\d.]+)', chunk, flags=re.M)
        failure = re.search(r"^  failureType: ['\"]?([^'\"\r\n]+)", chunk, flags=re.M)
        stage = re.search(r'^\s*# mage-stage: (.+)$', chunk, flags=re.M)
        operation = re.search(r'^\s*# mage-operation: (.+)$', chunk, flags=re.M)
        operation_record = None
        if operation:
            diagnostic = unescape_tap(operation.group(1))
            try:
                operation_record = json.loads(diagnostic)
            except json.JSONDecodeError:
                operation_record = {'unparsed_diagnostic': diagnostic}
        source, _, name = title.partition(' :: ')
        rows.append({'test': title, 'source': source, 'scenario': name,
                     'status': 'skipped' if skipped else 'passed' if passed else 'failed',
                     'classification': 'unavailable' if skipped else 'assertions_completed' if passed else classify(error),
                     'error': error or None,
                     'failure_type': failure.group(1) if failure else None,
                     'failure_stage': stage.group(1) if stage else None,
                     'operation': operation_record,
                     'duration_ms': float(duration.group(1)) if duration else None})
        if skipped:
            rows[-1]['skip_reason'] = unescape_tap(result.partition('# SKIP')[2].strip()) or None
    return rows


def unescape_tap(value):
    # Node escapes both literal backslashes and hash marks in diagnostic lines.
    return re.sub(r'\\([\\#])', r'\1', value)


def match_planned_rows(rows, planned):
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
            if row['status'] == 'passed':
                if scenario.get('has_ported_operations') is False:
                    error = 'unsupported Java test body: port would execute no operations'
                    row.update(status='failed', raw_status='passed', error=error,
                               classification=classify(error))
                elif not scenario['direct_assertions']:
                    row['classification'] = 'test_completed_assertion_coverage_unverified'
    return [scenario for scenario in planned if scenario['scenario_id'] not in observed]


def port_inventory(files):
    def count_assertions(operations):
        return sum((op['op'].startswith('assert') or
                    (op['op'] == 'attack' and 'expectedLegal' in op) or
                    (op['op'] == 'block' and 'expectedRejection' in op)) +
                   count_assertions(op.get('operations', []))
                   for op in operations)

    scenarios = []
    for path in files:
        text = path.read_text()
        marker = 'registerPortedMageTests('
        spec = json.loads(text[text.index(marker) + len(marker):text.rindex(');')])
        fixtures = {}
        for scenario in spec['tests']:
            fixtures.setdefault(scenario['name'], scenario)
        for index, scenario in enumerate(spec['tests']):
            repeated = scenario.get('repeat')
            fixture = fixtures.get(repeated['name'], scenario) if repeated else scenario
            operations = [*spec.get('setupOperations', []), *fixture.get('setupOperations', []),
                          *fixture.get('operations', [])]
            runs_fixture = repeated is None or repeated.get('seeds') != []
            scenarios.append({'test': spec['sourcePath'] + ' :: ' + scenario['name'],
                              'file': str(path.relative_to(ROOT)),
                              'scenario_id': str(path.relative_to(ROOT)) + '#' + str(index),
                              'upstream_skip': scenario.get('skip'),
                              'has_ported_operations': bool(operations) and runs_fixture,
                              'direct_assertions': count_assertions(operations) if runs_fixture else 0})
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


def partition_after_wasm_trap(rows, *, shared_process=True):
    trusted = []
    invalidated = []
    first_traps = {}
    for row in rows:
        scenario_id = row.get('scenario_id')
        process = None if shared_process else row.get('file')
        if scenario_id and process in first_traps and row['status'] != 'skipped':
            invalidated.append(row)
            continue
        trusted.append(row)
        if scenario_id and row['status'] == 'failed' and WASM_TRAP.fullmatch(row.get('error') or ''):
            first_traps[process] = scenario_id
    return trusted, invalidated, next(iter(first_traps.values()), None)


def replay_occurrences(planned):
    occurrences = collections.defaultdict(list)
    for scenario in planned:
        source = scenario['test'].partition(' :: ')[0]
        occurrences[source].append(int(scenario['scenario_id'].rsplit('#', 1)[1]))
    return json.dumps(occurrences)


def run_tap(command, environment, tap, timeout):
    with tap.open('w') as stream:
        child = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=stream,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        try:
            return child.wait(timeout=timeout), False
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGTERM)
            try:
                return child.wait(timeout=5), True
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                return child.wait(), True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('patterns', nargs='+', help='test files or glob patterns relative to repository')
    parser.add_argument('--jobs', type=int, default=2)
    parser.add_argument('--shared-process', action='store_true',
                        help='reuse one WASM runtime for this bounded batch of files')
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
               ROOT / 'scripts/wasm-test-harness.mjs', ROOT / 'cards.json',
               ROOT / 'web/ui/src/lib/sync-commands.js', ROOT / 'web/ui/src/lib/wire-json.js']
    sources += sorted((ROOT / 'scripts/mage-port-runner').glob('*.mjs'))
    selected = set(str(path.relative_to(ROOT)) for path in files)
    planned = [scenario for scenario in inventory if scenario['file'] in selected]
    java_fixtures = sorted({ROOT / scenario['test'].partition(' :: ')[0] for scenario in planned})
    inputs = sources + test_artifacts() + files + [path for path in java_fixtures if path.is_file()]
    before = [fingerprint(path) for path in inputs]
    command = ['node', '--test', f'--test-concurrency={args.jobs}',
               f'--test-timeout={args.test_timeout_ms}', '--test-reporter=tap']
    if args.shared_process:
        command.append('--test-isolation=none')
    command.extend(str(path.relative_to(ROOT)) for path in files)
    environment = {key: value for key, value in os.environ.items() if not key.startswith('MAGE_PORT_')}
    environment['MAGE_PORT_ALLOW_ENGINE_SHIMS'] = '0'
    args.output.parent.mkdir(parents=True, exist_ok=True)
    tap = args.output.with_suffix('.tap')
    start = time.monotonic()
    started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
    returncode, timed_out = run_tap(command, environment, tap, args.wall_timeout_seconds)
    rows = parse_tap(tap.read_text())
    match_planned_rows(rows, planned)
    recovery = []
    rows, invalidated, trap = partition_after_wasm_trap(rows, shared_process=args.shared_process)
    if trap:
        rows = [row for row in rows if row['status'] in TERMINAL_STATUSES]
        recovery.append({'tap_path': str(tap), 'trap_scenario': trap, 'invalidated_rows': invalidated})
    while trap:
        accepted = {row.get('scenario_id') for row in rows if row['status'] in TERMINAL_STATUSES}
        pending = [case for case in planned if case['scenario_id'] not in accepted]
        remaining = args.wall_timeout_seconds - (time.monotonic() - start)
        if not pending or remaining <= 0:
            break
        retry_tap = args.output.with_suffix(f'.retry-{len(recovery):02d}.tap')
        retry_files = sorted({case['file'] for case in pending})
        retry_command = ['node', '--test', '--test-concurrency=1', '--test-isolation=none',
                         f'--test-timeout={args.test_timeout_ms}', '--test-reporter=tap', *retry_files]
        retry_env = {**environment, 'MAGE_PORT_REPLAY_OCCURRENCES': replay_occurrences(pending)}
        _, retry_timeout = run_tap(retry_command, retry_env, retry_tap, remaining)
        timed_out |= retry_timeout
        retry_rows = parse_tap(retry_tap.read_text())
        match_planned_rows(retry_rows, pending)
        trusted, invalidated, trap = partition_after_wasm_trap(retry_rows)
        rows.extend(row for row in trusted if row.get('scenario_id') and row['status'] in TERMINAL_STATUSES)
        recovery.append({'tap_path': str(retry_tap), 'trap_scenario': trap,
                         'invalidated_rows': invalidated, 'selected_scenarios': len(pending)})
    after = [fingerprint(path) for path in inputs]
    observed = {row.get('scenario_id') for row in rows if row['status'] in TERMINAL_STATUSES}
    unreported = [case for case in planned if case['scenario_id'] not in observed]
    report = {'scope': 'Selected ported MAGE scenarios, not all-card correctness',
              'started_at': started_at, 'duration_seconds': time.monotonic() - start,
              'node_version': subprocess.check_output(['node', '--version'], text=True).strip(),
              'wasm_package': os.environ.get('IRONSMITH_TEST_WASM_PKG', 'pkg'),
              'reporter': fingerprint(Path(__file__)),
              'command': command, 'engine_shims': False, 'returncode': returncode,
              'wall_timeout': timed_out, 'selected_files': len(files),
              'inventory_scenarios': len(inventory), 'selected_scenarios': len(planned),
              'selected_unreported': unreported,
              'wasm_trap_recovery': recovery,
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
