#!/usr/bin/env python3
"""Conservative path-scoped coverage for frozen fixed multi-object tap costs."""
import collections
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / 'reports/runtime-audit'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    source = REPORTS / 'choose-consume-cost-candidates.json'
    candidates = json.loads(source.read_text())
    rows = [dict(r, status='unexercised_path', coverage=[]) for r in candidates['rows']
            if r['count_shape'] == 'fixed_multiple' and r['consumer_kind'] == 'TapEffect']
    def attach(card, path, evidence):
        matches = [r for r in rows if r['card'] == card and r['path'] == path]
        assert len(matches) == 1, (card, path)
        matches[0]['coverage'].append(evidence)
        previous=matches[0]['status']
        is_partial=lambda status: 'not_measured' in status or 'announcement_only' in status
        if previous=='unexercised_path' or not is_partial(evidence['status']) or is_partial(previous):
            matches[0]['status'] = evidence['status']
    for name in ['fixed-tap-cost-reviewed-classification.json', 'fixed-tap-sibling-reviewed-classification.json', 'tribal-tap-cost-reviewed-classification.json', 'mixed-tap-cost-reviewed-classification.json', 'tap-outcome-reviewed-classification.json', 'special-tap-cost-reviewed-classification.json', 'shimmer-tap-reviewed-classification.json', 'eladamri-tap-reviewed-classification.json', 'dermotaxi-tap-reviewed-classification.json', 'weight-tap-reviewed-classification.json', 'tap-copy-reviewed-classification.json', 'combat-tap-reviewed-classification.json']:
        file = REPORTS / name
        if not file.exists():
            continue
        reviewed = json.loads(file.read_text())
        raw = REPORTS / reviewed['source_report']
        assert digest(raw) == reviewed['source_sha256']
        for entry in reviewed['path_coverage']:
            attach(entry['card'], entry['path'], dict(entry, reviewed_report=str(file.relative_to(ROOT)),
                   reviewed_sha256=digest(file), raw_report=str(raw.relative_to(ROOT)), raw_sha256=digest(raw)))
    # This independently reviewed full resolution includes the exact two-creature tap cost.
    file = REPORTS / 'sacrifice-cost-sibling-reviewed-classification.json'
    review = json.loads(file.read_text()); raw = REPORTS / review['source_report']
    assert digest(raw) == review['source_sha256']
    data = json.loads(raw.read_text()); row = data['rows'][3]
    assert row['card'] == 'Grove of the Guardian' and row['actual'] == row['expected']
    assert row['actual']['mana_paid'] == 5 and row['actual']['source_graveyard']
    assert len([o for o in row['actual']['permanents'] if o['tapped']]) == 2
    attach(row['card'], '/definition/abilities/1/kind/Activated/mana_cost/kind/All/2',
           dict(status='one_scoped_resolution_control', reviewed_report=str(file.relative_to(ROOT)),
                reviewed_sha256=digest(file), raw_report=str(raw.relative_to(ROOT)), raw_sha256=digest(raw),
                source_rows=[3], scope='Actual land play; two paid fresh creatures tapped, source tapped/sacrificed, five mana paid; exact 8/8 green-white vigilance Elemental. No insufficient or surplus resource variants.'))
    file = REPORTS / 'hand-reveal-cost-reviewed-attribution.json'
    raw = REPORTS / 'hand-reveal-cost-final-execution.json'
    if file.exists() and raw.exists():
        data = json.loads(raw.read_text())
        for i in range(27,33):
            row = data['rows'][i]
            assert row['card'] == 'Sky Hussar' and row['status'] == 'expected_outcome_passed'
            assert all(c['expected'] == c['observed'] for c in row['checks'])
        attach('Sky Hussar', '/definition/abilities/2/kind/Activated/mana_cost/kind/All/0',
               dict(status='scoped_controls_only', reviewed_report=str(file.relative_to(ROOT)),
                    reviewed_sha256=digest(file), raw_report=str(raw.relative_to(ROOT)), raw_sha256=digest(raw),
                    source_rows=list(range(27,33)), scope='Actual paid Raise the Alarm Soldiers, normal TurnRunner own upkeep forecast reveal + exactly two taps + draw; repeat/timing/resource negatives and next-upkeep reset. Soldiers aged through a turn; fresh-creature payment is not claimed.'))
    supplemental = REPORTS / 'fixed-tap-existing-control-mappings.json'
    if supplemental.exists():
        data=json.loads(supplemental.read_text())
        for entry in data['path_coverage']:
            raw=ROOT/entry['raw_report']; assert digest(raw)==entry['raw_sha256']
            attach(entry['card'],entry['path'],entry)
    alternative = REPORTS / 'alternative-tap-cost-path-coverage.json'
    if alternative.exists():
        data=json.loads(alternative.read_text())
        for entry in data['rows']:
            if entry['count_shape'] != 'fixed_multiple': continue
            raw=ROOT/entry['source_report']['path']; assert digest(raw)==entry['source_report']['sha256']
            attach(entry['card'],entry['path'],dict(entry,ledger=str(alternative.relative_to(ROOT)),ledger_sha256=digest(alternative)))
    linked = REPORTS / 'linked-face-cost-reviewed-attribution.json'
    if linked.exists():
        data=json.loads(linked.read_text())
        for entry in data['path_coverage']:
            card=entry['payload_name'];path=entry['cost_path']
            matches=[r for r in rows if r['card']==card and r['path']==path]
            if not matches: continue
            raw=ROOT/entry['source_report']['path'];assert digest(raw)==entry['source_report']['sha256']
            attach(card,path,dict(entry,card=card,path=path,consumer_path=matches[0]['consumer_path'],status=entry['coverage_status'],reviewed_report=str(linked.relative_to(ROOT)),reviewed_sha256=digest(linked)))
    meria = REPORTS / 'meria-tap-exile-path-coverage.json'
    if meria.exists():
        data=json.loads(meria.read_text())
        for entry in data['rows']:
            raw=ROOT/entry['source_report']['path'];assert digest(raw)==entry['source_report']['sha256']
            attach(entry['card'],entry['path'],dict(entry,ledger=str(meria.relative_to(ROOT)),ledger_sha256=digest(meria)))
    output = dict(scope='All fixed-multiple ChooseObjectsEffect + same-tag TapEffect activation-cost paths in the frozen corpus; each OneOf branch remains independent.',
        candidate_source=dict(path=str(source.relative_to(ROOT)),sha256=digest(source),run_id=candidates['run_id']),
        summary=dict(paths=len(rows),payload_names=len({r['card'] for r in rows}),
                     status_counts=dict(collections.Counter(r['status'] for r in rows))),rows=rows,
        limitations=['Passing one path never certifies another branch, the whole card or all possible resource selections.',
                     'Unexercised includes cases that may have unrelated card-name observations elsewhere; only explicit path mappings receive credit.',
                     'Cost availability or early failure does not imply downstream costs or effects executed.'],
        generator_sha256=digest(Path(__file__)))
    (REPORTS/'fixed-tap-cost-family-coverage.json').write_text(json.dumps(output,indent=2)+'\n')
    lines=['# Fixed multiple-object tap cost coverage','','| Payload | Cost path | Required count | Status |','| --- | --- | ---: | --- |']
    lines += [f"| {r['card']} | `{r['path']}` | {r['count']['min']} | {r['status']} |" for r in rows]
    (REPORTS/'fixed-tap-cost-family-coverage.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps(output['summary']))

if __name__ == '__main__':
    main()
