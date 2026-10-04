#!/usr/bin/env python3
"""Finalize a complete durable MAGE retry checkpoint after a reporting failure.

Refuses partial campaigns, changed execution inputs, or changed TAP evidence.
It never executes or invents tests and preserves missing results as unreported.
"""
import argparse
import collections
import datetime
import json
from pathlib import Path

from audit_mage_ports import ROOT, fingerprint, port_inventory


def verify_fingerprints(records):
    observed = []
    for recorded in records:
        actual = fingerprint(ROOT / recorded['path'])
        comparable = {key: actual[key] for key in recorded}
        if comparable != recorded:
            raise ValueError(f'changed evidence: {recorded["path"]}')
        observed.append(comparable)
    return observed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('checkpoint', type=Path)
    parser.add_argument('--reason', required=True)
    args = parser.parse_args()
    checkpoint_path = args.checkpoint.resolve()
    checkpoint = json.loads(checkpoint_path.read_text())
    if checkpoint.get('status') != 'running_unverified_checkpoint':
        parser.error('expected a durable unverified retry checkpoint')
    attempts = checkpoint['attempts']
    selected_ids = {sid for attempt in attempts for sid in attempt['scenario_ids']}
    if len(selected_ids) != checkpoint['selected_scenarios']:
        parser.error('not every planned scenario has an attempted group')
    after = verify_fingerprints(checkpoint['provenance']['before'])
    verify_fingerprints([attempt['tap'] for attempt in attempts])
    inventory = port_inventory(sorted((ROOT / 'scripts/ported-mage-tests').rglob('*.test.mjs')))
    rows = [row for attempt in attempts for row in attempt['rows']]
    if rows != checkpoint['rows']:
        parser.error('checkpoint rows disagree with completed attempt rows')
    observed_ids = {row['scenario_id'] for row in rows}
    checkpoint_evidence = checkpoint_path.with_suffix('.checkpoint.json')
    checkpoint_evidence.write_bytes(checkpoint_path.read_bytes())
    births = [(ROOT / attempt['tap']['path']).stat().st_birthtime for attempt in attempts]
    started_at = datetime.datetime.fromtimestamp(min(births), datetime.timezone.utc).isoformat()
    report = {
        'scope': 'Individual imported MAGE retries finalized from a complete durable checkpoint after a reporting failure',
        'started_at': started_at,
        'started_at_source': 'creation time of earliest preserved TAP file',
        'command': [attempt['command'] for attempt in attempts],
        'engine_shims': checkpoint['engine_shims'],
        'inventory_scenarios': len(inventory),
        'selected_scenarios': len(selected_ids),
        'selected_files': len({row['file'] for row in inventory if row['scenario_id'] in selected_ids}),
        'selected_unreported': [row for row in inventory if row['scenario_id'] in selected_ids - observed_ids],
        'unselected_scenarios': [row for row in inventory if row['scenario_id'] not in selected_ids],
        'provenance': {'before': checkpoint['provenance']['before'], 'after': after, 'unchanged': True},
        'attempts': attempts,
        'rows': rows,
        'excluded_previously_attempted': checkpoint['excluded_previously_attempted'],
        'status_counts': dict(collections.Counter(row['status'] for row in rows)),
        'classification_counts': dict(collections.Counter(row['classification'] for row in rows)),
        'checkpoint_recovery': {'reason': args.reason,
                                'checkpoint': fingerprint(checkpoint_evidence),
                                'recovery_script': fingerprint(Path(__file__).resolve())},
        'limitations': ['Only actual TAP results are coverage; attempts with no result remain unreported.',
                        'All bound inputs and every TAP file were verified unchanged before recovering this report.',
                        'Imported failures are not automatically attributed as engine/card defects.'],
    }
    checkpoint_path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'selected': len(selected_ids), 'unreported': len(report['selected_unreported']),
                      'status_counts': report['status_counts'], 'unchanged': True}))


if __name__ == '__main__':
    main()
