#!/usr/bin/env python3
"""Inventory exact non-OneOf single choose/tap cost paths from frozen strict definitions."""
import hashlib
import json
import sqlite3
from collections import Counter
from pathlib import Path

ROOT = Path('reports/runtime-audit')
RUN = 'e17a4980b0b92c7a5a4cead2'


def reference(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    source = ROOT / 'choose-consume-cost-candidates.json'
    inv = json.loads(source.read_text())
    rows = [r for r in inv['rows'] if r['consumer_kind'] == 'TapEffect'
            and r['count_shape'] == 'single' and '/OneOf/' not in r['path']]
    assert len(rows) == 115 and len({r['card'] for r in rows}) == 113
    inventory = ROOT / f'actions/{RUN}/inventory.json'
    payloads = {c['name']: c for c in json.loads(inventory.read_text())['cards']}
    db = sqlite3.connect(f'file:{ROOT}/actions/results.sqlite3?mode=ro', uri=True)
    results, cards = [], []
    for name in sorted({r['card'] for r in rows}):
        data = json.loads(db.execute('select result_json from result where run_id=? and card_name=?', (RUN, name)).fetchone()[0])
        card = {**payloads[name], 'frozen_definition': data['definition'], 'frozen_artifact_checksum': data['artifact_checksum']}
        cards.append(card)
        for row in [r for r in rows if r['card'] == name]:
            index = int(row['path'].split('/')[3])
            ability = data['definition']['abilities'][index]['kind']['Activated']
            component_kinds = [next(iter(c)) for c in ability['mana_cost']['kind']['All']]
            effects = [e['kind'] for s in ability['effects']['segments'] for e in s['default_effects']]
            # This is a scheduling subgroup, never a proof that Station or its effect is correct.
            subgroup = ('station' if 'Station (' in card['oracle_text'] and effects == ['PutCountersEffect']
                        else 'mana' if ability['mana_output'] is not None
                        else 'source_tap' if 'T' in component_kinds else 'other_nonmana_no_source_tap')
            results.append({**row, 'family': 'single_tap_cost', 'subgroup': subgroup,
                            'source_ability_index': index, 'cost_component_kinds': component_kinds,
                            'effect_kinds': effects, 'oracle_text': card['oracle_text'],
                            'parse_name': card['parse_name'], 'parse_input': card['parse_input'], 'status': 'unrun'})
    report = {'scope': __doc__, 'path_count': 115, 'payload_count': 113,
              'groups': dict(Counter(r['subgroup'] for r in results)), 'rows': results,
              'provenance': {'inventory': reference(inventory), 'candidate_source': reference(source),
                             'run_id': RUN, 'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}}
    (ROOT / 'single-tap-cost-inventory.json').write_text(json.dumps(report, indent=2) + '\n')
    (ROOT / 'single-tap-cost-frozen-inputs.json').write_text(json.dumps({'source_run_id': RUN, 'cards': cards}, indent=2) + '\n')
    print(json.dumps(report['groups']))


if __name__ == '__main__':
    main()
