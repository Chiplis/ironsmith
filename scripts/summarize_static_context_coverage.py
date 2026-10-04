#!/usr/bin/env python3
"""Associate reviewed exact outcomes with the narrow static-context candidate family.

The join is by explicit report/card identity and verified immutable source rows.
It never converts an unreviewed screen match into a gameplay defect.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

BASE = Path(__file__).resolve().parents[1]
ROOT = BASE / 'reports/runtime-audit'
REPORTS = (
    ('ability-index-arcades-reviewed-attribution.json', 'ability-index-arcades-reproductions.json'),
    ('static-recipient-condition-reviewed-attribution.json', 'static-recipient-condition-reproductions.json'),
    ('static-tagged-attachment-reviewed-attribution.json', 'static-tagged-attachment-reproductions.json'),
    ('static-combat-predicate-reviewed-attribution.json', 'static-combat-predicate-execution.json'),
)
CLASSES = {'runtime_defect_card_reproduced', 'compiler_semantic_defect_card_reproduced'}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def binding_ok(finding, row, digest, raw_path):
    ref = finding.get('source_report', {})
    return (ref.get('sha256') == digest and Path(ref.get('path', '')).name == raw_path.name
        and finding.get('classification') in CLASSES
        and finding.get('expected') is not None
        and finding.get('expected') == row.get('expected')
        and finding.get('observed') == row.get('actual')
        and finding.get('expected') != finding.get('observed')
        and row.get('card') in finding.get('confirmed_cards', []))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        row = {'card': 'A', 'expected': {'power': 3}, 'actual': {'power': 2}}
        finding = {'classification': 'compiler_semantic_defect_card_reproduced',
            'confirmed_cards': ['A'], 'expected': row['expected'], 'observed': row['actual'],
            'source_report': {'path': 'reports/a.json', 'sha256': 'abc'}}
        assert binding_ok(finding, row, 'abc', Path('a.json'))
        assert not binding_ok(finding, row, 'changed', Path('a.json'))
        assert not binding_ok({**finding, 'confirmed_cards': ['B']}, row, 'abc', Path('a.json'))
        assert not binding_ok({**finding, 'observed': {'power': 1}}, row, 'abc', Path('a.json'))
        assert not binding_ok({**finding, 'classification': 'unreviewed_candidate'}, row, 'abc', Path('a.json'))
        print('Static context coverage binding checks passed.')
        return
    screen_path = ROOT / 'static-execution-condition-candidates.json'
    screen = json.loads(screen_path.read_text())
    names = sorted({r['card'] for r in screen['rows']})
    sources = []
    evidence = {name: [] for name in names}
    all_counts = Counter()
    for reviewed_name, raw_name in REPORTS:
        reviewed_path, raw_path = ROOT / reviewed_name, ROOT / raw_name
        reviewed, raw = json.loads(reviewed_path.read_text()), json.loads(raw_path.read_text())
        digest = sha(raw_path)
        findings = {}
        for finding in reviewed['findings']:
            index = finding['source_row']
            assert binding_ok(finding, raw['rows'][index], digest, raw_path), (reviewed_name, index)
            assert finding['card'] in names
            findings[index] = finding
        for index, row in enumerate(raw['rows']):
            if row['card'] not in names:
                continue
            if index in findings:
                status = 'reviewed_silent_wrong_result'
            elif row.get('expected') is not None and row['expected'] == row.get('actual') and row['status'] == 'expected_result_observed':
                status = 'scoped_expected_result_observed'
            else:
                status = 'unreviewed_or_unfinished'
            evidence[row['card']].append({'raw_report': raw_name, 'raw_sha256': digest, 'source_row': index,
                'scenario': row.get('scenario'), 'classification': status,
                'reviewed_report': reviewed_name if index in findings else None})
            all_counts[status] += 1
        sources.append({'reviewed_report': reviewed_name, 'reviewed_sha256': sha(reviewed_path),
            'raw_report': raw_name, 'raw_sha256': digest, 'provenance': raw.get('provenance'),
            'raw_case_count': len(raw['rows'])})
    rows = []
    for name in names:
        count = Counter(r['classification'] for r in evidence[name])
        rows.append({'card': name, 'candidate_paths': [r for r in screen['rows'] if r['card'] == name],
            'classification': 'reviewed_scoped_static_failure' if count['reviewed_silent_wrong_result'] else
                ('scoped_controls_only' if count['scoped_expected_result_observed'] else 'unexercised'),
            'case_counts': dict(count), 'evidence': evidence[name], 'all_paths_verified': False})
    out = {'generated_at': datetime.now(timezone.utc).isoformat(),
        'scope': 'Explicit reviewed gameplay evidence for names identified by the authored-static-condition source-guard screen. This associates cards and sampled states; it does not validate all candidate condition paths or every printed ability.',
        'candidate_names': len(names), 'candidate_paths': screen['candidate_paths'],
        'classification_counts': dict(Counter(r['classification'] for r in rows)),
        'scenario_counts': dict(all_counts), 'total_scenarios': sum(all_counts.values()),
        'rows': rows, 'all_candidate_names_have_scoped_evidence': all(evidence.values()),
        'all_candidate_branches_verified': False, 'all_cards_correct': False,
        'sources': sources, 'provenance': {'screen': str(screen_path.relative_to(BASE)),
            'screen_sha256': sha(screen_path), 'generator_sha256': sha(Path(__file__))},
        'limitations': ['A confirmed missing conditional P/T bonus does not also prove every adjacent blocking or keyword clause.',
            'Keywords are directly measured; only the reports that record combat outcomes claim those consequences were exercised.',
            'Frozen compiler-definition parity does not establish equivalence of runtime executable versions.',
            'Source guard discovery excludes nested granted conditions and filters/values; this is not coverage of all static evaluation.']}
    (ROOT / 'static-context-family-coverage.json').write_text(json.dumps(out, indent=2) + '\n')
    lines = [f"The narrow static-context screen found {len(names)} candidate names and {screen['candidate_paths']} serialized condition paths. {out['classification_counts'].get('reviewed_scoped_static_failure', 0)} names have individually reviewed failures in sampled states: {out['total_scenarios']} scenarios, {all_counts['reviewed_silent_wrong_result']} wrong results and {all_counts['scoped_expected_result_observed']} controls. This does not validate every condition path or whole cards.",
        '', '| Card | Wrong-result cases | Control cases |', '|---|---:|---:|']
    for row in rows:
        lines.append(f"| {row['card']} | {row['case_counts'].get('reviewed_silent_wrong_result',0)} | {row['case_counts'].get('scoped_expected_result_observed',0)} |")
    lines += ['', '[Full hash-bound coverage ledger](static-context-family-coverage.json), [source-guard candidate screen](static-execution-condition-candidates.json).']
    (ROOT / 'static-context-family-coverage.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps({k: out[k] for k in ('candidate_names','candidate_paths','classification_counts','scenario_counts','total_scenarios')}))


if __name__ == '__main__':
    main()
