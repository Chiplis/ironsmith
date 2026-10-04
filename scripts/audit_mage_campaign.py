#!/usr/bin/env python3
"""Resume bounded batches across the imported MAGE inventory, without engine shims.

Every file is attempted at most once per campaign. Incomplete or timed-out files
remain explicit; a later campaign may retry them after investigating the cause.
"""
import argparse
import datetime
import json
from pathlib import Path
import subprocess
import sys

from audit_mage_ports import ROOT, completed_files_from_reports, fingerprint, port_inventory


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def bound_inputs():
    sources = [ROOT / 'cards.json', ROOT / 'scripts/audit_mage_ports.py',
               ROOT / 'scripts/mage-port-runner.mjs', ROOT / 'scripts/wasm-test-harness.mjs']
    sources += sorted((ROOT / 'scripts/mage-port-runner').glob('*.mjs'))
    sources += sorted((ROOT / 'pkg').glob('*.wasm')) + sorted((ROOT / 'pkg').glob('*.js'))
    return [{key: value for key, value in fingerprint(path).items() if key != 'mtime_ns'}
            for path in sources]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'reports/runtime-audit/mage-campaign')
    parser.add_argument('--resume-from', type=Path, action='append', default=[])
    parser.add_argument('--defer-file', action='append', default=[],
                        help='known blocked file to retain as deferred without another identical retry')
    parser.add_argument('--jobs', type=int, default=3)
    parser.add_argument('--batch-files', type=int, default=32)
    parser.add_argument('--test-timeout-ms', type=int, default=60000)
    parser.add_argument('--wall-timeout-seconds', type=int, default=900)
    parser.add_argument('--max-batches', type=int, default=None)
    args = parser.parse_args()
    if min(args.jobs, args.batch_files, args.test_timeout_ms, args.wall_timeout_seconds) <= 0:
        parser.error('jobs, batch size, and timeouts must be positive')
    args.output_dir = args.output_dir.resolve()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = args.output_dir / 'manifest.json'
    files = sorted((ROOT / 'scripts/ported-mage-tests').rglob('*.test.mjs'))
    inventory = port_inventory(files)
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
        if manifest['bound_inputs'] != bound_inputs():
            parser.error('bound artifact/harness inputs changed; use a new campaign directory')
        for key in ['jobs', 'batch_files', 'test_timeout_ms', 'wall_timeout_seconds']:
            if manifest['configuration'][key] != getattr(args, key):
                parser.error(f'{key} differs from the saved campaign')
    else:
        prior_paths = [path.resolve() for path in args.resume_from]
        prior_reports = [json.loads(path.read_text()) for path in prior_paths]
        completed = completed_files_from_reports(inventory, prior_reports)
        deferred = set(args.defer_file)
        known = {str(path.relative_to(ROOT)) for path in files}
        if not deferred <= known:
            parser.error(f'unknown deferred files: {sorted(deferred - known)}')
        pending = sorted(known - completed - deferred)
        manifest = {'scope': 'Bounded imported MAGE scenario execution; failures remain candidates',
                    'started_at': now(), 'status': 'running', 'bound_inputs': bound_inputs(),
                    'driver': fingerprint(Path(__file__).resolve()),
                    'configuration': {key: getattr(args, key) for key in
                                      ['jobs', 'batch_files', 'test_timeout_ms', 'wall_timeout_seconds']},
                    'inventory_files': len(files), 'inventory_scenarios': len(inventory),
                    'inventory_manifest': [fingerprint(path) for path in files],
                    'prior_reports': [fingerprint(path) for path in prior_paths],
                    'previously_completed_files': sorted(completed),
                    'deferred_files': sorted(deferred),
                    'planned_files': pending, 'batches': []}

    def save():
        manifest['updated_at'] = now()
        temp = manifest_path.with_suffix('.tmp')
        temp.write_text(json.dumps(manifest, indent=2) + '\n')
        temp.replace(manifest_path)

    attempted = {file for batch in manifest['batches'] for file in batch['files']}
    remaining = [file for file in manifest['planned_files'] if file not in attempted]
    save()
    batch_count = 0
    while remaining and (args.max_batches is None or batch_count < args.max_batches):
        if bound_inputs() != manifest['bound_inputs']:
            manifest['status'] = 'stopped_inputs_changed'
            save()
            return 2
        selected, remaining = remaining[:args.batch_files], remaining[args.batch_files:]
        output = args.output_dir / f'batch-{len(manifest["batches"]):04d}.json'
        command = [sys.executable, str(ROOT / 'scripts/audit_mage_ports.py'), *selected,
                   '--jobs', str(args.jobs), '--test-timeout-ms', str(args.test_timeout_ms),
                   '--wall-timeout-seconds', str(args.wall_timeout_seconds), '--output', str(output)]
        batch = {'files': selected, 'started_at': now(), 'status': 'running', 'command': command,
                 'report': str(output.relative_to(ROOT))}
        manifest['batches'].append(batch)
        save()
        child = subprocess.run(command, cwd=ROOT)
        batch['finished_at'] = now()
        batch['returncode'] = child.returncode
        if output.exists():
            report = json.loads(output.read_text())
            batch.update({'status': 'reported', 'report_fingerprint': fingerprint(output),
                          'status_counts': report['status_counts'],
                          'selected_scenarios': report['selected_scenarios'],
                          'unreported_scenarios': len(report['selected_unreported']),
                          'provenance_unchanged': report['provenance']['unchanged']})
            if not report['provenance']['unchanged']:
                manifest['status'] = 'stopped_inputs_changed'
                save()
                return 2
        else:
            batch['status'] = 'missing_report'
        save()
        report_paths = [str(ROOT / record['path']) for record in manifest['prior_reports']]
        report_paths += [str(ROOT / record['report']) for record in manifest['batches']
                         if record['status'] == 'reported']
        subprocess.run([sys.executable, str(ROOT / 'scripts/summarize_mage_audit.py'),
                        *report_paths, '--output', str(args.output_dir / 'cumulative.json')],
                       cwd=ROOT, check=True)
        print(json.dumps({'batch': len(manifest['batches']) - 1, 'remaining_files': len(remaining),
                          'status_counts': batch.get('status_counts'),
                          'unreported_scenarios': batch.get('unreported_scenarios')}), flush=True)
        batch_count += 1
    manifest['status'] = 'all_planned_files_attempted' if not remaining else 'batch_limit_reached'
    save()
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
