#!/usr/bin/env python3
"""Join station source-shape candidates to explicit reviewed scenario evidence."""
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / 'reports/runtime-audit'
REVIEWS = ['ability-index-station-reviewed-attribution.json',
           'station-threshold-static-reviewed-attribution.json',
           'station-threshold-event-reviewed-attribution.json']


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    inventory_path = REPORTS / 'station-striation-candidates.json'
    inventory = json.loads(inventory_path.read_text())
    sources, evidence, raw_cases = [], {}, []
    for name in REVIEWS:
        path = REPORTS / name
        review = json.loads(path.read_text())
        raw_path = ROOT / review['source_report']['path']
        assert sha(raw_path) == review['source_report']['sha256']
        raw = json.loads(raw_path.read_text())
        assert raw['provenance']['artifacts_unchanged']
        sources.append({'review': name, 'review_sha256': sha(path),
                        'raw': str(raw_path.relative_to(ROOT)), 'raw_sha256': sha(raw_path)})
        raw_cases.extend(raw['rows'])
        for finding in review['findings']:
            row = raw['rows'][finding['source_row']]
            assert row['card'] == finding['card']
            assert finding['expected'] == row['expected'] and finding['observed'] == row['actual']
            if finding['defect_subtype'].startswith('station_'):
                evidence.setdefault(finding['card'], []).append({
                    'review': name, 'source_row': finding['source_row'],
                    'outcome_category': finding['outcome_category'], 'scope': finding['scope']})
    rows = [{**candidate, 'status': 'threshold_omission_reproduced' if candidate['card'] in evidence else 'unexercised',
             'reviewed_findings': evidence.get(candidate['card'], [])} for candidate in inventory['rows']]
    out = {'generated_at': datetime.now(timezone.utc).isoformat(),
           'scope': 'Every candidate from this multiline station source shape is mapped to scoped reviewed gameplay. This does not cover all station cards, every symbol, or every branch.',
           'candidate_source': inventory_path.name, 'candidate_sha256': sha(inventory_path),
           'generator_sha256': sha(Path(__file__)), 'candidate_striations': len(rows),
           'candidate_names': len({r['card'] for r in rows}), 'counts': dict(Counter(r['status'] for r in rows)),
           'reviewed_primary_cases': len(raw_cases), 'raw_case_status_counts': dict(Counter(r['status'] for r in raw_cases)),
           'station_threshold_failure_observations': sum(len(v) for v in evidence.values()),
           'sources': sources, 'rows': rows, 'all_cards_verified': False,
           'limitations': ['Entropic threshold-positive resolution throws its separately known simultaneous-action error; it is not a passing control.',
                          'Debris initial below-threshold pump expectation and Seriema declined-Twiddle drafts were excluded before the final reviewed runs.',
                          'Compiler artifact parity does not establish equivalence of current and frozen runtime binaries.']}
    (REPORTS / 'station-striation-family-coverage.json').write_text(json.dumps(out, indent=2) + '\n')
    print(json.dumps({k: out[k] for k in ['candidate_names', 'candidate_striations', 'counts', 'reviewed_primary_cases', 'raw_case_status_counts']}))


if __name__ == '__main__':
    main()
