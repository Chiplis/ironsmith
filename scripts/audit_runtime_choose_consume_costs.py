#!/usr/bin/env python3
"""Inventory adjacent typed ChooseObjects -> tagged consumer activation costs.

Every row is structural scope, never an automatic defect prediction. All cost
branches remain separate and flattened effect caches are omitted.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
from audit_runtime_counter_removal_costs import cost_roots
from audit_runtime_self_sacrifice_costs import all_branches


def references(value, tag, path=''):
    if isinstance(value, dict):
        if value.get('Tagged') == tag:
            yield path + '/Tagged'
        if value.get('tag') == tag and value.get('relation') == 'IsTaggedObject':
            yield path + '/tag'
        for k, v in value.items():
            if k != 'flattened_default_effects':
                yield from references(v, tag, path + '/' + k)
    elif isinstance(value, list):
        for i, v in enumerate(value):
            yield from references(v, tag, path + '/' + str(i))


def findings(definition):
    for cost, path in cost_roots(definition):
        if '/Activated/mana_cost' not in path:
            continue
        for components, branch in all_branches(cost, path):
            for i, component in enumerate(components[:-1]):
                choose = component.get('Effect', {}) if isinstance(component, dict) else {}
                if choose.get('kind') != 'ChooseObjectsEffect':
                    continue
                payload = choose['payload']
                nxt = components[i + 1]
                consumer = nxt.get('Effect', {}) if isinstance(nxt, dict) else {}
                refs = list(references(consumer, payload['tag']))
                if not refs:
                    continue
                count = payload['count']
                shape = ('dynamic_x' if count.get('dynamic_x') else
                         'single' if count.get('min') == 1 and count.get('max') == 1 else
                         'fixed_multiple' if count.get('min', 0) > 1 and count.get('max') == count.get('min') else
                         'other_count')
                yield {'path': branch + '/' + str(i), 'consumer_path': branch + '/' + str(i + 1),
                       'tag': payload['tag'], 'count': count, 'count_value': payload.get('count_value'),
                       'count_shape': shape, 'consumer_kind': consumer.get('kind'),
                       'consumer_reference_paths': refs,
                       'choose_flags': {key: payload.get(key) for key in ['chooser','zone','additional_zones','aggregate_constraint','top_only','bottom_only']}}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--database',type=Path,default=Path('reports/runtime-audit/actions/results.sqlite3'))
    p.add_argument('--run-id',default='e17a4980b0b92c7a5a4cead2')
    p.add_argument('--output',type=Path,default=Path('reports/runtime-audit/choose-consume-cost-candidates.json'))
    args=p.parse_args(); rows=[]; records=definitions=0
    with sqlite3.connect(args.database.resolve().as_uri()+'?mode=ro',uri=True) as db:
        for name,raw in db.execute('SELECT card_name,result_json FROM result WHERE run_id=? ORDER BY card_name',(args.run_id,)):
            records+=1;r=json.loads(raw);definition=r.get('definition')
            if not definition:continue
            definitions+=1
            rows.extend({'card':name,'artifact_checksum':r.get('artifact_checksum'),**row} for row in findings(definition))
    from collections import Counter
    output={'scope':'Adjacent ChooseObjectsEffect plus same-tag consumer within one activated TotalCost All branch. Structural candidates only, not engine-support predictions.','database':str(args.database),'run_id':args.run_id,'records_scanned':records,'retained_definitions':definitions,'candidate_paths':len(rows),'candidate_names':len({r['card'] for r in rows}),'groups':dict(Counter(r['count_shape']+' / '+r['consumer_kind'] for r in rows)),'source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'helper_sha256':{n:hashlib.sha256(Path(__file__).with_name(n).read_bytes()).hexdigest() for n in ['audit_runtime_counter_removal_costs.py','audit_runtime_self_sacrifice_costs.py']},'rows':rows,'limitations':['Nested definitions are included; flattened default-effect caches are skipped.','Only immediately adjacent effect costs with typed positive same-tag references are included.','Other cost shapes, spell costs, negative tag references and nonadjacent dependencies remain outside scope.','Do not infer a defect or a control from count shape; paid canonical execution or independently valid legality checks are required.']}
    args.output.write_text(json.dumps(output,indent=2)+'\n');print(json.dumps({k:output[k] for k in ['records_scanned','retained_definitions','candidate_paths','candidate_names','groups']}))

if __name__=='__main__':main()
