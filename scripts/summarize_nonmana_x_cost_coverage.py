#!/usr/bin/env python3
"""Join the frozen nonmana-X/reveal-cost inventory to scoped reviewed evidence."""
import hashlib
import json
from pathlib import Path


def main():
    root = Path('reports/runtime-audit')
    inventory_path = root / 'nonmana-x-cost-candidates.json'
    inventory = json.loads(inventory_path.read_text())
    sources = [
        'martyr-reveal-cost-reviewed-attribution.json',
        'nonmana-x-sibling-reviewed-attribution.json',
        'nonmana-x-control-reviewed-classification.json',
        'nonmana-x-spell-reviewed-attribution.json',
        'hand-reveal-cost-reviewed-attribution.json',
    ]
    reviewed, references = [], []
    for name in sources:
        path = root / name
        report = json.loads(path.read_text())
        reference = {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        references.append(reference)
        for key in ['findings', 'controls', 'rows']:
            for index, row in enumerate(report.get(key, [])):
                reviewed.append((row, {**reference, 'section': key, 'row': index}))
    rows, confirmed = [], set()
    for candidate in inventory['rows']:
        evidence = [(row, ref) for row, ref in reviewed if row.get('card') == candidate['card']]
        failures = [(row, ref) for row, ref in evidence if row.get('classification') in {
            'runtime_defect_card_reproduced', 'compiler_semantic_defect_card_reproduced'}]
        controls = [(row, ref) for row, ref in evidence if row.get('classification') in {
            'expected_outcome_passed', 'sampled_semantic_pass'}]
        for row, _ in failures:
            confirmed.update(row.get('confirmed_cards', [row['card']]))
        status = ('scoped_defect_reproduced' if failures else
                  'scoped_expected_outcomes_passed' if controls else
                  'fixed_reveal_only_unrun_outside_nonmana_x_scope')
        rows.append({'card': candidate['card'], 'branch_path': candidate['branch_path'],
                     'structural_route': candidate['route'], 'nonmana_x': candidate['nonmana_x'],
                     'status': status, 'failure_observations': len(failures),
                     'control_observations': len(controls),
                     'evidence': [ref for _, ref in evidence]})
    alias = "Ludevic, Necrogenius // Olag, Ludevic's Hubris"
    confirmed.discard(alias)
    report = {'scope': __doc__, 'inventory': {'path': str(inventory_path),
              'sha256': hashlib.sha256(inventory_path.read_bytes()).hexdigest(),
              'run_id': inventory['run_id'], 'records_scanned': inventory['records_scanned'],
              'retained_definitions': inventory['retained_definitions']},
              'reviewed_sources': references, 'rows': rows,
              'counts': {'paths': len(rows),
                         'paths_with_defect_evidence': sum(r['status'] == 'scoped_defect_reproduced' for r in rows),
                         'paths_with_only_scoped_passing_evidence': sum(r['status'] == 'scoped_expected_outcomes_passed' for r in rows),
                         'unrun_fixed_reveal_only_paths': sum(r['status'].startswith('fixed_reveal') for r in rows),
                         'canonical_confirmed_names': len(confirmed),
                         'primary_scenario_observations': len(reviewed)},
              'confirmed_canonical_cards': sorted(confirmed),
              'alias': {alias: 'Ludevic, Necrogenius'},
              'limitations': ['This ledger adds no new card attribution; it links five independently reviewed reports.',
                              'Passing controls establish only their explicitly tested branches, never whole-card correctness.',
                              'Eleven fixed hand-reveal paths have 59 scoped passing scenarios, including actual TurnRunner timing/repeat controls; they are not whole-card clearances.',
                              'The five Martyrs reached a broken X chooser. Other activated pairs failed earlier legality checks, so their predicted later X bounds remain unexecuted.',
                              'Ordinary spell/flashback cases have distinct cost staging; their passing and failing outcomes are retained separately.',
                              'Twenty additional same-state Firebolt controls are supporting evidence, not extra primary scenarios.'],
              'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (root / 'nonmana-x-cost-family-coverage.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['counts']))


if __name__ == '__main__':
    main()
