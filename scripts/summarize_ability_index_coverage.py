#!/usr/bin/env python3
"""Associate the conditional-static index screen with explicitly scoped probes.

This does not turn structurally similar definitions or a clean control into
whole-card correctness. Only the enumerated runtime reports are eligible.
"""
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / 'reports/runtime-audit'
REPORTS = (
    'ability-index-reproductions.json',
    'ability-index-followup-reproductions.json',
    'ability-index-restriction-reproductions.json',
    'ability-index-rule-reproductions.json',
    'ability-index-anthem-reproductions.json',
    'ability-index-mana-wrapper-reproductions.json',
    'ability-index-equipment-grant-reproductions.json',
    'ability-index-devotion-reproductions.json',
    'ability-index-active-grant-reproductions.json',
    'ability-index-pair-reproductions.json',
    'ability-index-hand-reproductions.json',
    'ability-index-arcades-reproductions.json',
    'ability-index-turn-grant-reproductions.json',
    'ability-index-state-grant-reproductions.json',
    'ability-index-cost-transition-reproductions.json',
    'ability-index-type-equipment-reproductions.json',
    'ability-index-counter-timing-reproductions.json',
    'ability-index-earned-grant-reproductions.json',
    'ability-index-land-mill-reproductions.json',
    'ability-index-myojin-reproductions.json',
    'ability-index-station-reproductions.json',
    'ability-index-attachment-switch-reproductions.json',
    'ability-index-zone-stack-reproductions.json',
    'ability-index-counter-level-reproductions.json',
    'ability-index-turn-equipment-reproductions.json',
    'ability-index-gourmand-reproductions.json',
    'ability-index-turn-loyalty-reproductions.json',
    'ability-index-speed-discard-reproductions.json',
    'ability-index-linked-transition-reproductions.json',
    'ability-index-alias-front-reproductions.json',
    'ability-index-island-attack-reproductions.json',
)
NON_INDEX_SEMANTIC_REPORTS = {'ability-index-arcades-reproductions.json', 'ability-index-station-reproductions.json', 'ability-index-attachment-switch-reproductions.json', 'ability-index-counter-level-reproductions.json', 'ability-index-speed-discard-reproductions.json'}
FAILURES = {'action_or_choice_failed', 'resolution_failed', 'semantic_mismatch'}
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    screen_path = ROOT / 'ability-index-structural-candidates.json'
    screen = json.loads(screen_path.read_text())
    paths = defaultdict(list)
    for row in screen['rows']:
        paths[row['card']].append(row)
    evidence = defaultdict(list)
    sources = []
    for filename in REPORTS:
        path = ROOT / filename
        if not path.exists():
            continue
        report = json.loads(path.read_text())
        if report.get('provenance', {}).get('artifacts_unchanged') is not True:
            raise ValueError(f'Unverified provenance: {filename}')
        sources.append({'path': filename, 'sha256': sha(path)})
        for i, row in enumerate(report['rows']):
            valid = row['status'] in FAILURES | {'expected_result_observed'}
            if valid and (row.get('actual') is None or row.get('expected') is None):
                raise ValueError(f'Missing comparison: {filename} row {i}')
            evidence[row['card']].append({'source': filename, 'row': i,
                'status': row['status'], 'scenario': row.get('scenario'),
                'earlier_legal_action_failure': filename == 'ability-index-zone-stack-reproductions.json' and row['status'] == 'semantic_mismatch' and row.get('actual', {}).get('activation', {}).get('offered') is False,
                'other_semantic_failure_with_dispatch_passed': filename in NON_INDEX_SEMANTIC_REPORTS and row['status'] == 'semantic_mismatch',
                'attack_rule_only': filename == 'ability-index-island-attack-reproductions.json',
                'has_expected_actual_comparison': valid})
    for filename in ['conditional-land-entry-reviewed-classification.json']:
        path = ROOT / filename
        if not path.exists():
            continue
        review = json.loads(path.read_text())
        raw_path = ROOT / review['source_report']
        if sha(raw_path) != review['source_sha256']:
            raise ValueError(f'Stale source binding: {filename}')
        raw = json.loads(raw_path.read_text())
        process_path = ROOT / review['process_report']
        process = json.loads(process_path.read_text())
        if process['status'] != 'completed' or process['exit_code'] != 0 or process.get('binary_unchanged') is not True or process['binary_sha256'] != raw['provenance']['binary_sha256']:
            raise ValueError(f'Unverified native process: {filename}')
        sources.extend({'path': p.name, 'sha256': sha(p)} for p in [path, raw_path, process_path])
        for entry in review['candidate_evidence']:
            if not any(p['static_index'] == entry['static_index'] and p['static_id'] == entry['static_id'] for p in paths[entry['card']]):
                raise ValueError(f'Unknown static candidate: {entry}')
            for i in entry['source_rows']:
                row = raw['rows'][i]
                if row['card'] != entry['card'] or row['status'] != 'expected_result_observed' or row.get('expected') != row.get('actual'):
                    raise ValueError(f'Unverified control: {filename} row {i}')
                evidence[entry['card']].append({'source': raw_path.name, 'row': i,
                    'status': row['status'], 'scenario': row.get('scenario'),
                    'has_expected_actual_comparison': True, 'scope': entry['scope'],
                    'static_index': entry['static_index'], 'mana_ability_index': entry['mana_ability_index']})
    reality = ROOT / 'reconfigure-reviewed-attribution.json'
    if reality.exists():
        report = json.loads(reality.read_text())
        sources.append({'path': reality.name, 'sha256': sha(reality)})
        for i, row in enumerate(report.get('findings', report.get('rows', []))):
            if row.get('classification') not in {'runtime_defect_card_reproduced', 'compiler_semantic_defect_card_reproduced'}:
                continue
            for name in row.get('confirmed_cards', []):
                evidence[name].append({'source': reality.name, 'row': i,
                    'status': 'action_or_choice_failed', 'scenario': row.get('scenario'),
                    'has_expected_actual_comparison': row.get('expected') is not None and row.get('observed') is not None})
    rows = []
    for name, candidates in sorted(paths.items()):
        cases = evidence.get(name, [])
        valid = [x for x in cases if x['has_expected_actual_comparison']]
        failed = [x for x in valid if x['status'] in FAILURES and not x.get('other_semantic_failure_with_dispatch_passed') and not x.get('earlier_legal_action_failure')]
        earlier_failures = [x for x in valid if x.get('earlier_legal_action_failure')]
        other_failures = [x for x in valid if x.get('other_semantic_failure_with_dispatch_passed')]
        status = ('sampled_attack_rule_only_activation_unexercised' if valid and all(x.get('attack_rule_only') for x in valid) else
                  'reviewed_index_failure_in_sampled_state' if failed else
                  'sampled_other_legality_failure_with_dispatch_controls' if earlier_failures else
                  'sampled_dispatch_passed_with_other_semantic_failure' if other_failures else
                  'sampled_controls_with_unfinished_cases' if valid and len(valid) != len(cases) else
                  'sampled_controls_only' if valid else
                  'setup_limited' if cases else 'unexercised_by_this_family')
        rows.append({'card': name, 'candidate_paths': candidates,
            'classification': status, 'cases': cases,
            'all_candidate_paths_executed': False,
            'whole_card_verified': False})
    summary = Counter(r['classification'] for r in rows)
    rule = [r for r in rows if any(x['static_id'] == 'RuleRestriction' for x in r['candidate_paths'])]
    out = {
        'generated_at': datetime.now(timezone.utc).isoformat(),
        'scope': 'Typed conditional Static-before-Activated/Mana candidates associated only with named paid-action index probes. The screen is broader than actual static filtering semantics.',
        'candidate_names': len(rows), 'candidate_paths': len(screen['rows']),
        'counts': dict(summary), 'rule_restriction_subgroup': {
            'candidate_names': len(rule), 'counts': dict(Counter(r['classification'] for r in rule)),
            'names': [r['card'] for r in rule]},
        'rows': rows, 'all_cards_correct': False,
        'all_candidate_branches_verified': False,
        'provenance': {'screen': screen_path.name, 'screen_sha256': sha(screen_path),
            'generator_sha256': sha(Path(__file__)), 'reports': sources},
        'limitations': [
            'A runtime mismatch in one state does not prove every structurally flagged path fails.',
            'A passing action checks only the explicit report assertions and does not clear the card.',
            'Reports use recorded fresh executable versions. Definition parity is not runtime-binary equivalence.',
            'Names confirmed for unrelated families remain unexercised here unless this explicit report set covers them.',
            'Linked-face transitions and input aliases receive no automatic coverage credit.',
            'Arcades has an explicitly reviewed recipient-buff mismatch while every recorded pump dispatch succeeds; those failures are not ability-index failures.',
            'Fugitive Droid missing legal actions are a reviewed target-filter defect before dispatch; separate own-spell controls exercise both current indices. The omissions are not promoted as ability-index defects.',
            'Seven Island-restricted sources have only actual attack-rule observations here; their activated ability indices remain unexecuted.',
            'Own-turn Equipment controls verify off-state characteristics and absence of sorcery-speed Equip; they do not force an illegal off-state dispatch.',
            'Implicit static activation conditions and nested granted ability lists were outside the original typed screen.'
        ]}
    target = ROOT / 'ability-index-family-coverage.json'
    target.write_text(json.dumps(out, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({k: out[k] for k in ('candidate_names', 'candidate_paths', 'counts', 'rule_restriction_subgroup')}, indent=2))

if __name__ == '__main__':
    main()
