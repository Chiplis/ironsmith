#!/usr/bin/env python3
"""Profile one previously unreported imported scenario without changing campaign coverage.

Adds bounded V8 sampling plus existing harness operation/decision/phase traces.
The separate report is diagnostic evidence, never a replacement inventory result.
"""
import argparse
import datetime
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time

from audit_mage_ports import ROOT, fingerprint, parse_tap
from audit_mage_campaign import bound_inputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scenario-id', required=True)
    parser.add_argument('--seconds', type=int, default=15)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    accounting = ROOT / 'reports/runtime-audit/mage-final-accounting.json'
    scenarios = json.loads(accounting.read_text())['unreported_individual_attempts']
    selected = next(item for item in scenarios if item['scenario_id'] == args.scenario_id)
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = output.with_suffix('')
    evidence.mkdir(exist_ok=False)
    sources = [accounting, Path(__file__).resolve(), ROOT / selected['file']]
    before = bound_inputs() + [fingerprint(path) for path in sources]
    environment = {key: val for key, val in os.environ.items() if not key.startswith('MAGE_PORT_')}
    environment.update({key: '1' for key in ['MAGE_PORT_TEST_START_TRACE', 'MAGE_PORT_TRACE',
                                           'MAGE_PORT_DECISION_TRACE', 'MAGE_PORT_ADVANCE_TRACE',
                                           'MAGE_PORT_STACK_TRACE']})
    environment['MAGE_PORT_ALLOW_ENGINE_SHIMS'] = '0'
    command = ['node', '--prof', f'--logfile={evidence}/v8-%p.log', '--test',
               '--test-concurrency=1', f'--test-timeout={args.seconds * 1000}',
               '--test-reporter=tap', f"--test-name-pattern=^{re.escape(selected['test'])}$",
               selected['file']]
    tap = evidence / 'trace.tap'
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    start = time.monotonic()
    wall_timeout = False
    with tap.open('w') as stream:
        child = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=stream,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        try:
            code = child.wait(timeout=args.seconds + 8)
        except subprocess.TimeoutExpired:
            wall_timeout = True
            os.killpg(child.pid, signal.SIGTERM)
            try:
                code = child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                code = child.wait()
    duration = time.monotonic() - start
    text = tap.read_text()
    profiles = []
    for logfile in sorted(evidence.glob('*v8-*.log')):
        analysis = logfile.with_suffix('.summary.txt')
        with analysis.open('w') as stream:
            try:
                result = subprocess.run(['node', '--prof-process', '--nm=/usr/bin/true', str(logfile)], cwd=ROOT,
                                        stdout=stream, stderr=subprocess.STDOUT, timeout=30)
                profile_code = result.returncode
            except subprocess.TimeoutExpired:
                profile_code = 'processing_timeout'
        profiles.append({'raw': fingerprint(logfile), 'summary': fingerprint(analysis),
                         'processing_returncode': profile_code})
    after = bound_inputs() + [fingerprint(path) for path in sources]
    traces = [line for line in text.splitlines() if '[mage-port-' in line]
    report = {'scope': 'Separate diagnostic profiling of one prior unreported scenario. No campaign result replaced and no card defect inferred from timeout alone.',
              'selected': selected, 'command': command, 'environment_overrides': {
                  key: val for key, val in environment.items() if key.startswith('MAGE_PORT_')},
              'started_at': started, 'duration_seconds': duration, 'returncode': code,
              'wall_timeout': wall_timeout, 'tap': fingerprint(tap), 'v8_profiles': profiles,
              'trace_count': len(traces), 'last_traces': traces[-20:],
              'scenario_results': [r for r in parse_tap(text) if r['test'] == selected['test']],
              'provenance': {'before': before, 'after': after, 'unchanged': before == after}}
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'report': str(output.relative_to(ROOT)), 'returncode': code,
                      'duration_seconds': duration, 'trace_count': len(traces),
                      'profiles': len(profiles), 'unchanged': before == after}))


if __name__ == '__main__':
    main()
