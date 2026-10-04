#!/usr/bin/env python3
"""Inventory typed additional-land-play sources for control-transfer probes.

The property is legitimate; inventory membership is not a bug finding.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sqlite3


def walk(value, path='/definition'):
    if isinstance(value, dict):
        if 'AdditionalLandPlays' in value:
            yield {'path': path, 'additional_land_plays': value['AdditionalLandPlays'],
                   'conditional': '/Conditional/' in path}
        for key, child in value.items():
            if key != 'flattened_default_effects':
                yield from walk(child, path + '/' + key)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk(child, path + '/' + str(index))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--database', type=Path, default=Path('reports/runtime-audit/actions/results.sqlite3'))
    parser.add_argument('--run-id', default='e17a4980b0b92c7a5a4cead2')
    parser.add_argument('--out', type=Path, default=Path('reports/runtime-audit/land-play-control-inventory.json'))
    parser.add_argument('--inputs', type=Path, default=Path('reports/runtime-audit/land-play-control-frozen-inputs.json'))
    args = parser.parse_args()
    run = args.database.parent / args.run_id
    inv = {p['name']: p for p in json.loads((run/'inventory.json').read_text())['cards']}
    db = sqlite3.connect(f'file:{args.database.resolve()}?mode=ro', uri=True)
    rows, cards = [], []
    for name, raw in db.execute("select card_name,result_json from result where run_id=? and instr(result_json,'AdditionalLandPlays')>0 order by card_name", (args.run_id,)):
        result = json.loads(raw)
        found = list(walk(result.get('definition', {})))
        if not found:
            continue
        for item in found:
            rows.append({'card': name, 'artifact_checksum': result.get('artifact_checksum'),
                         'card_types': result['definition']['card']['card_types'],
                         'oracle_text': inv[name]['oracle_text'], **item,
                         'classification': 'typed_permission_family_member_not_a_bug_claim',
                         'combined_alias': ' // ' in name})
        cards.append({**inv[name], 'frozen_definition': result['definition'],
                      'frozen_artifact_checksum': result['artifact_checksum'],
                      'additional_land_plays': found[0]['additional_land_plays'],
                      'conditional': any(f['conditional'] for f in found)})
    for name in ['Control Magic', 'Confiscate', 'Act of Treason', 'Typhoid Rats', 'Grizzly Bears']:
        result = json.loads(db.execute('select result_json from result where run_id=?and card_name=?', (args.run_id,name)).fetchone()[0])
        cards.append({**inv[name], 'frozen_definition': result['definition'],
                      'frozen_artifact_checksum': result['artifact_checksum'], 'additional_land_plays':0,
                      'conditional':False})
    for c in cards:
        symbols=re.findall(r'\{([^}]+)\}', c['parse_input'].splitlines()[0])
        c['expected_cast_mana_at_x0'] = sum(
            int(v) if v.isdigit() else 0 if v == 'X' else 1 for v in symbols
        )
    provenance={'run':str(run),'manifest':json.loads((run/'manifest.json').read_text()),
                'inventory_sha256':hashlib.sha256((run/'inventory.json').read_bytes()).hexdigest(),
                'scanner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    report={'scope':__doc__,'records_screened':db.execute('select count(*)from result where run_id=?',(args.run_id,)).fetchone()[0],
            'paths':len(rows),'names':len({r['card']for r in rows}),'rows':rows,'provenance':provenance,
            'limitations':['Only serialized AdditionalLandPlays static payloads; other extra-land permission forms are not covered.',
                           'Conditional sources and combined aliases require separate controls; membership never implies a runtime defect.']}
    args.out.write_text(json.dumps(report,indent=2)+'\n')
    args.inputs.write_text(json.dumps({'provenance':provenance,'cards':cards},indent=2)+'\n')
    print(json.dumps({k:report[k]for k in ['records_screened','paths','names']}))


if __name__=='__main__':main()
