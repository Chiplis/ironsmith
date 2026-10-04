#!/usr/bin/env python3
"""Screen strict artifacts for literal X in spell-cast-trigger effects.

Candidates only: X in mana symbols with an explicit dynamic value is legitimate,
and effect-value consumers still need oracle/context review and execution.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3

from audit_noncast_x_candidates import ROOT, xpaths


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, default=ROOT / 'reports/runtime-audit/corpus/results.sqlite3')
    parser.add_argument('--run-id')
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/runtime-audit/spell-trigger-x-candidates.json')
    args = parser.parse_args()
    connection = sqlite3.connect(f'file:{args.db.resolve()}?mode=ro', uri=True)
    run = args.run_id or connection.execute('select run_id from result order by recorded_at desc limit 1').fetchone()[0]
    findings = []
    query = "select card_name,result_json from result where run_id=? and json_extract(result_json,'$.definition.abilities') like '%\"X\"%'"
    for name, raw in connection.execute(query, (run,)):
        definition = json.loads(raw)['definition']
        for index, ability in enumerate(definition['abilities']):
            trigger = ability.get('kind', {}).get('Triggered')
            if not trigger or 'SpellCast' not in json.dumps(trigger['trigger']['kind']):
                continue
            paths = list(xpaths(trigger.get('effects')))
            if paths:
                findings.append({'card_name': name, 'ability_index': index,
                                 'x_paths': paths, 'ability': trigger,
                                 'definition_sha256': hashlib.sha256(json.dumps(definition, sort_keys=True).encode()).hexdigest(),
                                 'status': 'candidate_requires_individual_review'})
    report = {'scope': __doc__, 'db': str(args.db.resolve()), 'run_id': run,
              'scanner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'xpaths_source_sha256': hashlib.sha256((ROOT / 'scripts/audit_noncast_x_candidates.py').read_bytes()).hexdigest(),
              'candidates': findings}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'candidate_abilities': len(findings),
                      'card_names': [item['card_name'] for item in findings]}))


if __name__ == '__main__':
    main()
