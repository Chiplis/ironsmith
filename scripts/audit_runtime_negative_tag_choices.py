#!/usr/bin/env python3
"""Screen frozen typed definitions for negative-only tagged-object ChooseSpecs.

This is candidate generation, not dynamic proof. Targeted forms and the special
continuous-effect filter resolver are explicitly separated from generic choices.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def screen(value, path='/definition', effect=None, effect_path=None):
    if isinstance(value, dict):
        if isinstance(value.get('kind'), str) and value['kind'].endswith('Effect'):
            effect, effect_path = value['kind'], path
        obj = value.get('Object')
        if isinstance(obj, dict):
            tags = obj.get('tagged_constraints', [])
            if tags and all(t.get('relation') == 'IsNotTaggedObject' for t in tags):
                relative = path[len(effect_path):] if effect_path else path
                parts = relative.split('/')
                if 'Target' in parts:
                    route = 'targeted_form_excluded'
                elif 'WithCount' in parts or 'WithCountValue' in parts:
                    route = 'counted_choice_route_requires_separate_review'
                elif effect == 'ApplyContinuousEffect' and 'target_spec' in parts:
                    route = 'continuous_effect_whole_battlefield_filter_route'
                elif effect in {'DealDamageEffect', 'MoveToZoneEffect'}:
                    route = 'generic_negative_tag_candidate_pool_risk'
                else:
                    route = 'unreviewed_non_target_consumer'
                yield {'path': path + '/Object', 'effect': effect, 'route': route,
                       'tags': tags, 'filter': obj,
                       'classification': 'structural_candidate_not_a_confirmed_card_defect'}
        for key, child in value.items():
            if key != 'flattened_default_effects':
                yield from screen(child, path + '/' + key, effect, effect_path)
    elif isinstance(value, list):
        for i, child in enumerate(value):
            yield from screen(child, path + '/' + str(i), effect, effect_path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--database', type=Path, default=Path('reports/runtime-audit/actions/results.sqlite3'))
    parser.add_argument('--run-id', default='e17a4980b0b92c7a5a4cead2')
    args = parser.parse_args()
    run = args.database.parent / args.run_id
    inventory = {p['name']: p for p in json.loads((run / 'inventory.json').read_text())['cards']}
    db = sqlite3.connect(f'file:{args.database.resolve()}?mode=ro', uri=True)
    pref = db.execute("select card_name,result_json from result where run_id=? and instr(result_json,'IsNotTaggedObject')>0 order by card_name", (args.run_id,)).fetchall()
    rows = []
    for name, raw in pref:
        result = json.loads(raw)
        for record in screen(result.get('definition', {})):
            rows.append({'card': name, 'artifact_checksum': result.get('artifact_checksum'),
                         'oracle_text': inventory[name]['oracle_text'], **record})
    support = ['Grizzly Bears', 'Typhoid Rats', "Krenko's Command", 'Crimson Wisps', 'Shock']
    names = sorted({r['card'] for r in rows if r['route'] not in {'targeted_form_excluded'}} | set(support))
    inputs = []
    for name in names:
        result = json.loads(db.execute('select result_json from result where run_id=? and card_name=?', (args.run_id, name)).fetchone()[0])
        inputs.append({**inventory[name], 'frozen_definition': result['definition'],
                       'frozen_artifact_checksum': result['artifact_checksum']})
    provenance = {'run': str(run), 'manifest': json.loads((run / 'manifest.json').read_text()),
                  'inventory_sha256': hashlib.sha256((run / 'inventory.json').read_bytes()).hexdigest(),
                  'scanner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    report = {'scope': __doc__, 'records_screened': db.execute('select count(*) from result where run_id=?', (args.run_id,)).fetchone()[0],
              'text_prefilter_definitions': len(pref), 'paths': len(rows),
              'names': len({r['card'] for r in rows}), 'rows': rows, 'provenance': provenance,
              'limitations': ['Only typed ChooseSpec::Object forms with nonempty all-IsNotTaggedObject constraints are enumerated.',
                              'Direct raw ObjectFilters in static rules, counts, All queries or ChooseObjectsEffect are not generic ChooseSpec::Object resolution and are outside this screen.',
                              'Runtime route labels are source-reviewed dispatch classifications, not executed outcomes.',
                              'Duplicated flattened_default_effects are skipped; distinct branches remain distinct paths.']}
    Path('reports/runtime-audit/negative-tag-choice-candidates.json').write_text(json.dumps(report, indent=2) + '\n')
    Path('reports/runtime-audit/negative-tag-choice-frozen-inputs.json').write_text(json.dumps({'provenance': provenance, 'cards': inputs}, indent=2) + '\n')
    print(json.dumps({key: report[key] for key in ['records_screened', 'text_prefilter_definitions', 'paths', 'names']}))


if __name__ == '__main__':
    main()
