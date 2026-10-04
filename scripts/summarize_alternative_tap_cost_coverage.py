#!/usr/bin/env python3
"""Map each inventoried OneOf choose/tap branch to its exact paid selector scenario.

This review is deliberately fail-closed: it describes the current all-pass cost
campaign, and cannot promote future changed mismatches without manual review.
"""
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path('reports/runtime-audit')


def reference(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def pointer(document, path):
    value = document
    for item in path.lstrip('/').split('/'):
        key = item.replace('~1', '/').replace('~0', '~')
        value = value[int(key)] if isinstance(value, list) else value[key]
    return value


def branch_identity(path):
    match = re.fullmatch(r'/definition/abilities/(\d+)/kind/Activated/mana_cost/kind/OneOf/(\d+)/kind/All/(\d+)', path)
    assert match, f'Unexpected structural cost path: {path}'
    return tuple(map(int, match.groups()))


def validate_candidate(candidate, compilation, row):
    """Require same source ability, alternative branch, selector and cardinality."""
    ability, branch, position = branch_identity(candidate['path'])
    consumer_ability, consumer_branch, consumer_position = branch_identity(candidate['consumer_path'])
    assert (ability, branch, position + 1) == (consumer_ability, consumer_branch, consumer_position)
    assert row['card'] == candidate['card'] == compilation['card']
    assert row['variant'] == 'positive' and row['branch'] == branch > 0
    assert row['fixture_evidence']['diagnostic']['canonical_index'] == ability
    assert compilation['frozen_artifact_checksum'] == candidate['artifact_checksum']
    choice = pointer(compilation, candidate['path'])['Effect']
    consumer = pointer(compilation, candidate['consumer_path'])['Effect']
    assert choice['kind'] == 'ChooseObjectsEffect' and consumer['kind'] == 'TapEffect'
    assert choice['payload']['count']['min'] == choice['payload']['count']['max'] == branch
    assert choice['payload']['tag'] == candidate['tag'] == consumer['payload']['target']['Tagged']
    assert choice['payload']['chooser'] == 'You'
    selector = row['actual']['action']['selector']
    assert len(selector) == 1 and selector[0]['desired_index'] == branch and selector[0]['desired_legal']
    assert next(o for o in selector[0]['options'] if o['index'] == branch)['legal']
    selected = [x for x in row['fixture_evidence']['decision_trace']
                if x['choice'] == 'options' and x['description'].startswith('Choose an activation cost')]
    assert len(selected) == 1 and selected[0]['selected'] == [branch]
    assert row['actual']['action']['mana_paid'] == row['waterbend'] - branch
    assert len(row['actual']['before']['resources']) == branch
    assert all(not r['tapped'] for r in row['actual']['before']['resources'])
    assert all(r['artifact'] or r['creature'] for r in row['actual']['before']['resources'])
    assert all(r['fresh'] for r in row['actual']['before']['resources'] if r['creature'])
    assert all(r['tapped'] for r in row['actual']['after']['resources'])
    return {'source_ability_index': ability, 'alternative_branch_index': branch,
            'selector_description': selector[0]['description'],
            'selected_option': next(o for o in selector[0]['options'] if o['index'] == branch),
            'printed_waterbend': row['waterbend'], 'paid_mana': row['actual']['action']['mana_paid'],
            'actual_tapped_resource_ids': [r['id'] for r in row['actual']['after']['resources']],
            'outcome_scope': row['outcome_scope']}


def main():
    inventory_path = ROOT / 'choose-consume-cost-candidates.json'
    raw_path = ROOT / 'alternative-tap-cost-execution.json'
    input_path = ROOT / 'alternative-tap-cost-frozen-inputs.json'
    inventory = json.loads(inventory_path.read_text())
    raw = json.loads(raw_path.read_text())
    payloads = {p['name']: p for p in json.loads(input_path.read_text())['cards']}
    candidates = [r for r in inventory['rows'] if r['consumer_kind'] == 'TapEffect' and '/OneOf/' in r['path']]
    assert len(candidates) == 79 and len({r['card'] for r in candidates}) == 14
    assert raw['provenance']['artifacts_unchanged']
    assert all(c['definition_matches_frozen_except_unique_card_ids'] for c in raw['compilation'])
    compiled = {c['card']: c for c in raw['compilation']}
    assert len(raw['rows']) == 135
    raw_ref = reference(raw_path)
    scenario_lookup = {}
    controls = []
    for index, row in enumerate(raw['rows']):
        assert row['status'] == 'expected_outcome_passed'
        assert all(c['expected'] == c['observed'] for c in row['checks'])
        key = (row['card'], row['branch'], row['variant'])
        assert key not in scenario_lookup
        scenario_lookup[key] = index
        # Printed oracle quantity is independently retained with the full canonical payload.
        oracle = payloads[row['card']]['oracle_text']
        printed = re.search(r'Waterbend \{(\d+)\}', oracle, re.I)
        assert printed and int(printed.group(1)) == row['waterbend']
        if row['variant'] != 'positive':
            assert not row['actual']['action']['branch_offered']
            assert len(row['actual']['before']['untapped_eligible']) < row['branch']
        controls.append({'card': row['card'], 'scenario': row['scenario'],
                         'classification': 'expected_outcome_passed', 'confirmed_cards': [],
                         'scope': row['outcome_scope'], 'source_report': raw_ref, 'source_row': index,
                         'expected': row['expected'], 'observed': row['actual'],
                         'checks': row['checks']})
    rows = []
    for candidate in candidates:
        _, branch, _ = branch_identity(candidate['path'])
        index = scenario_lookup[(candidate['card'], branch, 'positive')]
        evidence = validate_candidate(candidate, compiled[candidate['card']], raw['rows'][index])
        companion = [i for i, r in enumerate(raw['rows']) if r['card'] == candidate['card'] and (r['branch'] == 0 or r['variant'] != 'positive')]
        rows.append({**candidate, 'family': 'alternative_tap_cost',
                     'status': 'scoped_expected_outcomes_passed', 'source_report': raw_ref,
                     'source_rows': [index], 'companion_control_rows': companion, **evidence})
    assert len({(r['card'], r['path'], r['consumer_path']) for r in rows}) == 79
    counts = {'paths': 79, 'payloads': 14, 'primary_card_names': 13,
              'primary_scenarios': 135, 'actual_successful_activations': 93,
              'inventoried_tap_branch_activations': 79, 'full_mana_branch_activations': 14,
              'expected_unavailable_branch_controls': 42,
              'oracle_checks': sum(len(r['checks']) for r in raw['rows']),
              'strict_canonical_definitions': len(compiled), 'confirmed_cards': 0, 'unrun_paths': 0}
    limitations = [
        'Scoped cost and listed-effect coverage only; no whole-card clearance and no defect promotion.',
        'Each OneOf branch is matched by exact typed source ability and branch path. DFC alias payload is independently compiled and executed.',
        'Aang Swift Savior primary and combined-name payloads have no canonical linked backface metadata. All eighteen positive cost scenarios are covered, but transformation correctness is expressly unverified; no fake linked definition is supplied.',
        'Avatar Kuruk has no printed mana cost. It enters through actual paid Faithless Looting discarding its canonical hand card, then paid Reanimate targeting that graveyard card. Resources are cast before Kuruk to avoid unintended Spirit generation.',
        'Fresh Grizzly Bears, noncreature Tormods Crypt artifacts, and Ornithopter artifact creatures are normally cast and explicitly chosen. Paying with those creatures does not use their own tap-symbol abilities.',
        'Negative controls test the largest alternative per payload with no additional resources, paid noneligible Fervors, or resources tapped through actual paid Twiddle. Any eligible source itself remains counted. They do not individually test all intermediate insufficient-card boundaries.',
        'Mana is a declared initial/checkpoint fixture; every spell and activation follows normal advertised legal actions and real payment. Positive branches start with only the exact printed mana remainder.',
        'The full-mana control and the largest-branch negatives are shared companion evidence, not duplicated primary scenarios for every path.',
        'Iceberg verifies its actual sacrifice; detailed scry arrangement is not asserted. Rallier verifies the revealed eligible creature reaches hand and net library size; exact randomized bottom order is not asserted. Other card abilities remain outside scope.',
        'Choice displays describe tap alternatives with misleading Exile wording. This report records the UI strings but makes no card execution-defect promotion from those labels; actual chosen permanents remain on the battlefield tapped.',
        'Pilot Flexible Waterbender report is superseded by the complete frozen-hash campaign and is not added to scenario totals.',
    ]
    provenance = {'native_source_runs': [{'source_report': raw_ref, 'run': raw['provenance'],
                                         'attempt': json.loads((ROOT/'alternative-tap-cost-attempt.json').read_text())}],
                  'artifacts_unchanged': raw['provenance']['artifacts_unchanged']}
    source_audit = [
        {**reference(Path('crates/ironsmith-engine/src/game_loop/priority_cast.rs')),
         'symbol': 'ActivationStage::ChoosingAlternativeCost',
         'finding': 'Selector enumerates complete alternative branches with explicit original branch indices and per-branch legality.'},
        {**reference(Path('crates/ironsmith-engine/src/game_loop/priority_mana.rs')),
         'symbol': 'apply_alternative_activation_cost_response',
         'finding': 'Chosen branch is rechecked for payment legality, then assigned to pending activation before payment.'},
    ]
    review = {'scope': __doc__, 'findings': [], 'controls': controls, 'confirmed_cards': [],
              'counts': counts, 'source_reports': [raw_ref], 'provenance': provenance,
              'source_audit': source_audit, 'path_coverage': rows, 'limitations': limitations,
              'strict_compilation': [{k: v for k, v in c.items() if k != 'definition'} for c in raw['compilation']]}
    review_path = ROOT / 'alternative-tap-cost-reviewed-attribution.json'
    review_path.write_text(json.dumps(review, indent=2) + '\n')
    ledger = {'family': 'alternative_tap_cost', 'scope': __doc__, 'rows': rows, 'path_count': len(rows),
              'inventory': {**reference(inventory_path), 'run_id': inventory['run_id'],
                            'records_scanned': inventory['records_scanned'],
                            'retained_definitions': inventory['retained_definitions']},
              'reviewed_sources': [reference(review_path)], 'counts': counts,
              'status_counts': dict(Counter(r['status'] for r in rows)),
              'limitations': limitations, 'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (ROOT/'alternative-tap-cost-path-coverage.json').write_text(json.dumps(ledger, indent=2) + '\n')
    names = sorted({r['card'] for r in rows})
    lines = ['# Alternative tap cost audit', '',
             'All 79 inventoried Waterbend choose/tap alternatives passed their exact paid selector scenarios. '
             'The 135-scenario campaign also covers 14 full-mana alternatives and 42 largest-branch unavailable-resource controls. '
             f'{counts["oracle_checks"]} independent expected/observed checks passed. No individual card is newly confirmed or cleared.', '',
             '| Canonical payload | Typed tap paths | Positive activations | Negative controls |', '|---|---:|---:|---:|']
    for name in names:
        card_rows = [r for r in raw['rows'] if r['card'] == name]
        lines.append(f'| {name} | {sum(r["card"] == name for r in rows)} | {sum(r["variant"] == "positive" for r in card_rows)} | {sum(r["variant"] != "positive" for r in card_rows)} |')
    lines += ['', 'Full paid sources and resource producers, exact OneOf selector indices, branch mana remainders, selected permanent IDs, and before/after snapshots are in the raw report. The machine ledger maps each exact source path to one primary scenario.', '']
    lines += [f'- {limit}' for limit in limitations]
    lines += ['', f'Raw report: `{raw_ref["path"]}` (SHA-256 `{raw_ref["sha256"]}`).',
              f'Executable SHA-256: `{provenance["native_source_runs"][0]["attempt"]["executable_sha256"]}`.',
              f'All {len(compiled)} strict definitions match frozen definitions except enumerated unique CardIds. Executable, input, and fixture hashes remained unchanged.', '']
    (ROOT/'alternative-tap-cost-review.md').write_text('\n'.join(lines))
    print(json.dumps(counts))


if __name__ == '__main__':
    main()
