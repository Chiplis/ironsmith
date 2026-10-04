#!/usr/bin/env python3
"""Find same-branch SacrificeSelf plus effect costs in frozen definitions.

Candidates are not defects. In particular, ordinary sacrifice and independent
resource payments can commute; source-dependent metadata/resources may not.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
from audit_runtime_counter_removal_costs import cost_roots


def all_branches(cost, path):
    kind = cost['kind']
    if 'All' in kind:
        yield kind['All'], path + '/kind/All'
    else:
        for index, branch in enumerate(kind['OneOf']):
            yield from all_branches(branch, path + '/kind/OneOf/' + str(index))


def findings(definition):
    for cost, path in cost_roots(definition):
        for components, branch_path in all_branches(cost, path):
            if 'SacrificeSelf' not in components:
                continue
            effects = [c['Effect']['kind'] for c in components if isinstance(c, dict) and 'Effect' in c]
            if effects:
                yield {'path': branch_path, 'effect_kinds': effects,
                       'ordered_cost_shapes': [c if isinstance(c, str) else list(c) for c in components]}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--database', type=Path, default=Path('reports/runtime-audit/actions/results.sqlite3'))
    p.add_argument('--run-id', default='e17a4980b0b92c7a5a4cead2')
    p.add_argument('--output', type=Path, default=Path('reports/runtime-audit/self-sacrifice-effect-cost-candidates.json'))
    args = p.parse_args()
    connection = sqlite3.connect(f'file:{args.database.resolve()}?mode=ro', uri=True)
    rows, records, definitions = [], 0, 0
    for name, raw in connection.execute('SELECT card_name,result_json FROM result WHERE run_id=? ORDER BY card_name', (args.run_id,)):
        records += 1
        result = json.loads(raw)
        definition = result.get('definition')
        if not definition:
            continue
        definitions += 1
        rows.extend({'card':name,'artifact_checksum':result.get('artifact_checksum'),**finding} for finding in findings(definition))
    result = {'scope':'Same All branch contains both typed SacrificeSelf and at least one effect cost. Not a defect prediction; independent full canonical paid scenarios required.',
              'database':str(args.database),'run_id':args.run_id,'records_scanned':records,'retained_definitions':definitions,
              'candidate_paths':len(rows),'candidate_names':len({r['card'] for r in rows}),
              'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'cost_root_helper_sha256':hashlib.sha256(Path(__file__).with_name('audit_runtime_counter_removal_costs.py').read_bytes()).hexdigest(),
              'limitations':['OneOf branches remain separate; flattened caches excluded.','Other source-moving costs and non-effect typed costs are outside this screen.','Does not establish whether payment order matters or whether engine stages the effect.'],'rows':rows}
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ['records_scanned','retained_definitions','candidate_paths','candidate_names']}))


if __name__=='__main__':
    main()
