#!/usr/bin/env python3
"""Review 29 direct-effect single-tap cost paths with paid resource and independent outcome checks."""
import hashlib
import json
from collections import Counter
from pathlib import Path

ROOT = Path('reports/runtime-audit')


def ref(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    cases = {c['card']: c for c in json.loads((ROOT/'single-tap-effect-frozen-inputs.json').read_text())['cases']}
    inventory = json.loads((ROOT/'single-tap-cost-inventory.json').read_text())
    candidates = [r for r in inventory['rows'] if r['card'] in cases and r['source_ability_index'] == cases[r['card']]['index']]
    assert len(candidates) == 29
    mapped = {r['card']: [] for r in candidates}
    sources, runs, controls, excluded = [], [], [], []
    for filename, attempt in [('single-tap-effect-execution.json', 'single-tap-effect-attempt.json'),
                              ('single-tap-effect-vodalian-followup.json', 'single-tap-effect-vodalian-attempt.json')]:
        path = ROOT / filename
        raw = json.loads(path.read_text())
        assert raw['provenance']['artifacts_unchanged']
        assert all(c['definition_matches_frozen_except_unique_card_ids'] for c in raw['compilation'])
        source = ref(path)
        sources.append(source)
        runs.append({'source_report': source, 'run': raw['provenance'],
                     'attempt': json.loads((ROOT/attempt).read_text()),
                     'strict_compilation': [{k: v for k, v in c.items() if k != 'definition'} for c in raw['compilation']]})
        for i, row in enumerate(raw['rows']):
            if filename == 'single-tap-effect-execution.json' and row['card'] == 'Vodalian War Machine':
                assert row['status'] == 'fixture_error' and row['actual']['error'] == 'canonical intrinsic cost match not unique:[1, 2]'
                excluded.append({'source_report': source, 'source_row': i, 'reason': 'Two printed Vodalian abilities share a cost; the original helper correctly stopped on ambiguous identity. Follow-up compares exact cost, timing and full runtime effect representation and reruns these five cases.'})
                continue
            assert row['status'] == 'expected_outcome_passed'
            assert all(c['expected'] == c['observed'] for c in row['checks'])
            assert row['fixture_evidence']['canonical_index'] == cases[row['card']]['index']
            positive = row['expected']['action_offered']
            if positive:
                assert row['actual']['action']['resolution_error'] is None
                assert row['fixture_evidence']['selected_resource'] is not None
                assert row['actual']['action']['mana_paid'] == (1 if row['card'] == 'Black Oak of Odunos' else 2 if row['card'] == "Volrath's Gardens" else 0)
                if row['scenario'] in ['exact', 'surplus']:
                    assert all(row['fixture_evidence']['resource_fresh'])
            mapped[row['card']].append({'source_report': source, 'source_row': i,
                                        'scenario': row['scenario'], 'positive': positive,
                                        'source_ability_index': row['fixture_evidence']['canonical_index'],
                                        'live_ability_index': row['fixture_evidence']['live_index']})
            controls.append({'card': row['card'], 'scenario': row['scenario'],
                             'classification': 'expected_outcome_passed', 'confirmed_cards': [],
                             'source_report': source, 'source_row': i,
                             'expected': row['expected'], 'observed': row['actual'], 'checks': row['checks']})
    paths = []
    for candidate in candidates:
        evidence = mapped[candidate['card']]
        assert len(evidence) == 5 and {r['scenario'] for r in evidence} == {'zero', 'exact', 'surplus', 'tapped', 'ineligible'}
        paths.append({**candidate, 'status': 'scoped_expected_outcomes_passed',
                      'source_evidence': evidence,
                      'scope': 'Exact current ability, resource payment, mana amount and listed direct effect; zero-other-resource self-payment only when Oracle allows'})
    counts = {'paths': 29, 'primary_scenarios': 145,
              'actual_successful_activations': sum(c['expected']['action_offered'] for c in controls),
              'expected_unavailable_controls': sum(not c['expected']['action_offered'] for c in controls),
              'oracle_checks': sum(len(c['checks']) for c in controls),
              'fixture_errors_excluded': len(excluded), 'confirmed_cards': 0, 'unrun_paths': 0}
    limitations = [
        'Scoped payment and listed resolution outcomes only. No individual card is cleared globally and no new defect is promoted.',
        'Each source and resource is normally cast from its full canonical strict definition. Fresh non-tap-symbol payment is legal; no unavailable action is forced.',
        'Zero additional resource is a positive self-payment case when the printed cost permits the source. Other/tribal/artifact constraints use explicit independent scenario choices, not a blanket zero-resource negative.',
        'Exact and surplus cases choose one known resource; the second surplus resource must remain untapped. Actual paid Twiddle taps resources and any potentially eligible source for negative cases.',
        'Universal Automaton is a real normally cast Changeling tribal resource. Goblin Piker provides a red creature for Impelled Giant; Ornithopter provides an artifact for Lodestone Myr.',
        'Field Surgeon and Master Apothecary prevention are tested against actual paid Shock; the effect must prevent precisely one or two damage. Bob Fleetfeather Cockatrice is a normally paid flash creature where an opponent target is needed.',
        'The five initial Vodalian ambiguous-identity fixture rows are excluded and replaced by five bounded follow-up rows. Other 140 passing rows were preserved instead of rerun.',
        'Activated lifetime expiration, removal of sources during resolution, opponents turn timing for Volraths Gardens, and other abilities are outside scope.',
    ]
    review = {'scope': __doc__, 'findings': [], 'controls': controls, 'confirmed_cards': [],
              'counts': counts, 'source_reports': sources,
              'provenance': {'native_source_runs': runs, 'artifacts_unchanged': True},
              'path_coverage': paths, 'excluded_fixture_rows': excluded, 'limitations': limitations}
    rp = ROOT / 'single-tap-effect-reviewed-attribution.json'
    rp.write_text(json.dumps(review, indent=2) + '\n')
    ledger = {'family': 'single_tap_cost', 'subfamily': 'direct_effects', 'path_count': 29,
              'rows': paths, 'counts': counts, 'reviewed_sources': [ref(rp)],
              'status_counts': dict(Counter(r['status'] for r in paths)),
              'limitations': limitations, 'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (ROOT/'single-tap-effect-path-coverage.json').write_text(json.dumps(ledger, indent=2) + '\n')
    lines = ['# Direct-effect single-tap cost audit', '',
             f'All 29 exact cost paths passed. {counts["primary_scenarios"]} scenarios verify {counts["actual_successful_activations"]} actual activations and {counts["expected_unavailable_controls"]} unavailable controls; {counts["oracle_checks"]} checks pass. Five ambiguous Vodalian fixture rows are excluded and replaced by exact-effect follow-ups. No card is promoted or globally cleared.', '']
    lines += [f'- {limit}' for limit in limitations]
    lines += ['', 'Path ledger: `reports/runtime-audit/single-tap-effect-path-coverage.json`.',
              'Reviewed attribution: `reports/runtime-audit/single-tap-effect-reviewed-attribution.json`.', '']
    (ROOT/'single-tap-effect-review.md').write_text('\n'.join(lines))
    print(json.dumps(counts))


if __name__ == '__main__':
    main()
