#!/usr/bin/env python3
"""Screen strict corpus artifacts for X entry consumers; candidates are not defects."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1]


def xpaths(value, path=''):
    if value == 'X':
        yield path
    if isinstance(value, dict):
        for key, child in value.items():
            yield from xpaths(child, path + '/' + key)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from xpaths(child, path + '/' + str(index))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, default=ROOT / 'reports/runtime-audit/corpus/results.sqlite3')
    parser.add_argument('--run-id')
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/runtime-audit/noncast-x-candidates.json')
    args = parser.parse_args()
    connection = sqlite3.connect(f'file:{args.db.resolve()}?mode=ro', uri=True)
    run = args.run_id or connection.execute('select run_id from result order by recorded_at desc limit 1').fetchone()[0]
    rows = connection.execute("select card_name,result_json from result where run_id=? and json_extract(result_json,'$.definition.card.mana_cost') like '%\"X\"%'", (run,))
    findings = []
    definitions = 0
    for name, raw in rows:
        definition = json.loads(raw)['definition']
        definitions += 1
        for index, ability in enumerate(definition['abilities']):
            kind, family, payload = ability['kind'], None, None
            if 'Triggered' in kind:
                trigger = kind['Triggered']['trigger']['kind']
                change = trigger.get('ZoneChange', {}) if isinstance(trigger, dict) else {}
                if change.get('to') == 'Battlefield' and change.get('this') is True:
                    family, payload = 'self_etb_trigger', kind['Triggered']
            if 'Static' in kind and 'Enter' in kind['Static'].get('id', ''):
                family, payload = 'entry_static', kind['Static']
            paths = list(xpaths(payload)) if payload else []
            if paths:
                findings.append({'card_name': name, 'ability_index': index, 'family': family,
                                 'x_paths': paths, 'card_types': definition['card']['card_types'],
                                 'subtypes': definition['card']['subtypes'],
                                 'definition_sha256': hashlib.sha256(json.dumps(definition, sort_keys=True).encode()).hexdigest(),
                                 'status': 'noncast_x_candidate_not_executed'})
    optional = []
    for name, raw in connection.execute("select card_name,result_json from result where run_id=? and json_extract(result_json,'$.definition.optional_costs') like '%\"X\"%'", (run,)):
        definition = json.loads(raw)['definition']
        optional.append({'card_name': name, 'optional_costs': definition['optional_costs'],
                         'base_mana_cost': definition['card']['mana_cost'],
                         'result_json_sha256': hashlib.sha256(raw.encode()).hexdigest()})
    report = {'scope': 'Strict compile-success corpus definitions with X in base mana cost and literal X in a self-ETB trigger or entry static ability. Candidates only; oracle-defined X, attachments, conditions and context require review. Does not cover every dynamic_x choice or nonliteral computed value.',
              'db': str(args.db.resolve()), 'run_id': run,
              'scanner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'base_x_definition_count': definitions, 'candidates': findings,
              'card_names': sorted({item['card_name'] for item in findings}),
              'optional_x_cost_family': optional}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'base_x_definitions': definitions, 'candidate_abilities': len(findings),
                      'candidate_payload_names': len(report['card_names']),
                      'optional_x_cost_names': [item['card_name'] for item in optional]}))


if __name__ == '__main__':
    main()
