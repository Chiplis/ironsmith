#!/usr/bin/env python3
"""Find cost-selection dependencies outside the adjacent activated-cost inventory.

Structural scope only. A same-tag reference does not prove correct binding,
resource availability, reachability, or the success of any resulting effect.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3
from audit_runtime_counter_removal_costs import cost_roots
from audit_runtime_self_sacrifice_costs import all_branches
from audit_runtime_choose_consume_costs import references


def findings(definition):
    for cost, cost_path in cost_roots(definition):
        for components, branch in all_branches(cost, cost_path):
            for i, component in enumerate(components):
                choose = component.get('Effect', {}) if isinstance(component, dict) else {}
                if choose.get('kind') != 'ChooseObjectsEffect':
                    continue
                payload = choose['payload']; tag = payload['tag']
                for j in range(i + 1, len(components)):
                    after = components[j]
                    consumer = after.get('Effect', {}) if isinstance(after, dict) else {}
                    if consumer.get('kind') == 'ChooseObjectsEffect' and consumer['payload'].get('tag') == tag:
                        break
                    refs = list(references(consumer, tag))
                    if not refs:
                        continue
                    if '/Activated/mana_cost' in cost_path and j == i + 1:
                        continue
                    count = payload['count']
                    shape = ('dynamic_x' if count.get('dynamic_x') else 'single' if count.get('min') == count.get('max') == 1 else 'fixed_multiple' if count.get('min', 0) > 1 and count.get('min') == count.get('max') else 'other_count')
                    yield dict(path=f'{branch}/{i}', consumer_path=f'{branch}/{j}', cost_root=cost_path,
                               distance=j-i, tag=tag, count=count, count_value=payload.get('count_value'),
                               count_shape=shape, consumer_kind=consumer.get('kind'),consumer_reference_paths=refs,
                               cost_context='activated_nonadjacent' if '/Activated/mana_cost' in cost_path else 'other_total_cost',
                               choose_flags={k:payload.get(k) for k in ['chooser','zone','additional_zones','aggregate_constraint','top_only','bottom_only']},
                               intervening_cost_shapes=[c if isinstance(c,str) else list(c) for c in components[i+1:j]])


def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--database',type=Path,default=Path('reports/runtime-audit/actions/results.sqlite3'))
    p.add_argument('--run-id',default='e17a4980b0b92c7a5a4cead2')
    p.add_argument('--output',type=Path,default=Path('reports/runtime-audit/extended-cost-dependency-candidates.json'))
    args=p.parse_args();rows=[];records=definitions=0
    with sqlite3.connect(args.database.resolve().as_uri()+'?mode=ro',uri=True) as db:
        for name,raw in db.execute('SELECT card_name,result_json FROM result WHERE run_id=? ORDER BY card_name',(args.run_id,)):
            records+=1;r=json.loads(raw);definition=r.get('definition')
            if not definition:continue
            definitions+=1
            rows.extend(dict(card=name,artifact_checksum=r.get('artifact_checksum'),**row) for row in findings(definition))
    result=dict(scope=__doc__,database=str(args.database),run_id=args.run_id,records_scanned=records,retained_definitions=definitions,candidate_paths=len(rows),candidate_names=len({r['card'] for r in rows}),groups=dict(Counter(r['cost_context']+' / '+r['count_shape']+' / '+str(r['consumer_kind']) for r in rows)),source_sha256=digest(Path(__file__)),helper_sha256={n:digest(Path(__file__).with_name(n)) for n in ['audit_runtime_counter_removal_costs.py','audit_runtime_self_sacrifice_costs.py','audit_runtime_choose_consume_costs.py']},rows=rows,limitations=['Structural candidates only, never an execution result.','Each OneOf branch stays independent; copied flattened effect caches are skipped.','Earlier adjacent activated pairs are excluded, including all453paths in the original ledger.','Tracks direct ChooseObjectsEffect producers and positive tag references within a TotalCost All branch, ending before a later direct same-tag selection. Nested/conditional producer binding, effect-to-cost dependencies and spell effect outcomes are not certified.'])
    args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:result[k] for k in ['records_scanned','retained_definitions','candidate_paths','candidate_names','groups']},indent=2))
if __name__=='__main__':main()
