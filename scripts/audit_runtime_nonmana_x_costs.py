#!/usr/bin/env python3
"""Inventory typed nonmana-X and reveal costs in frozen compiled definitions.

All branches remain separate. Candidate labels describe runtime source routes;
they do not assert that a card is broken or that a positive-X fixture is reachable.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3

from audit_runtime_counter_removal_costs import cost_roots
from audit_runtime_self_sacrifice_costs import all_branches

TRANSPARENT = {'TaggedEffect', 'TagAllEffect', 'WithIdEffect',
               'LocalRewriteEffect', 'ExecuteWithSourceEffect'}


def x_value(value):
    if value == 'X':
        return True
    # Value::Hinted does not change the underlying value.
    if isinstance(value, dict) and 'Hinted' in value:
        hint = value['Hinted']
        return x_value(hint.get('value')) if isinstance(hint, dict) else False
    return False


def effect_info(effect, path):
    wrappers = []
    while effect.get('kind') in TRANSPARENT:
        wrappers.append(effect['kind'])
        effect = effect.get('payload', {}).get('effect', {})
        path += '/payload/effect'
    kind, payload = effect.get('kind'), effect.get('payload', {})
    if kind in {'RevealFromHandEffect', 'SacrificeEffect', 'SacrificePlayerEffect'}:
        references = payload.get('count') == 'X'
    elif kind == 'RemoveAnyCountersFromSourceEffect':
        references = payload.get('display_x') is True
    elif kind == 'ChooseObjectsEffect':
        references = payload.get('count', {}).get('dynamic_x') is True or x_value(
            (payload.get('aggregate_constraint') or {}).get('minimum'))
    else:
        references = False
    return {'path': path, 'effect_kind': kind, 'payload': payload,
            'transparent_wrappers': wrappers, 'references_cost_x': references,
            'reveal_cost': kind in {'RevealFromHandEffect', 'RevealSourceFromHandEffect'}}


def findings(definition):
    for cost, root in cost_roots(definition):
        activated = root.endswith('/kind/Activated/mana_cost')
        for components, branch in all_branches(cost, root):
            effects = [effect_info(c['Effect'], f'{branch}/{i}/Effect')
                       for i, c in enumerate(components)
                       if isinstance(c, dict) and 'Effect' in c]
            relevant = [e for e in effects if e['references_cost_x'] or e['reveal_cost']]
            if not relevant:
                continue
            mana = [c['Mana'] for c in components if isinstance(c, dict) and 'Mana' in c]
            dynamic = [c['DynamicMana'] for c in components if isinstance(c, dict) and 'DynamicMana' in c]
            # assign_pending_activation_cost stores the last Mana component.
            last_mana = mana[-1] if mana else None
            mana_has_x = bool(last_mana and any('X' in pip for pip in last_mana.get('pips', [])))
            nonmana_x = any(e['references_cost_x'] for e in effects)
            if activated and dynamic:
                route = 'dynamic_mana_cost_requires_runtime_review'
            elif activated and nonmana_x and last_mana is not None and not mana_has_x:
                route = 'fixed_mana_clamps_nonmana_x_candidate'
            elif activated and nonmana_x:
                route = 'nonmana_x_without_fixed_mana_clamp_control'
            else:
                route = 'reveal_or_nonmana_x_other_cost_context'
            yield {'cost_root': root, 'branch_path': branch, 'activated': activated,
                   'route': route, 'last_mana': last_mana, 'mana_has_x': mana_has_x,
                   'dynamic_mana': dynamic, 'nonmana_x': nonmana_x,
                   'effects': relevant, 'cost_components': components,
                   'classification': 'structural_candidate_not_a_confirmed_card_defect'}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--database', type=Path, default=Path('reports/runtime-audit/actions/results.sqlite3'))
    p.add_argument('--run-id', default='e17a4980b0b92c7a5a4cead2')
    p.add_argument('--output', type=Path, default=Path('reports/runtime-audit/nonmana-x-cost-candidates.json'))
    args = p.parse_args()
    db = sqlite3.connect(f'file:{args.database.resolve()}?mode=ro', uri=True)
    rows, records, definitions = [], 0, 0
    for name, raw in db.execute('SELECT card_name,result_json FROM result WHERE run_id=? ORDER BY card_name', (args.run_id,)):
        records += 1
        result = json.loads(raw)
        if not result.get('definition'):
            continue
        definitions += 1
        rows.extend({'card': name, 'artifact_checksum': result.get('artifact_checksum'), **r}
                    for r in findings(result['definition']))
    report = {'scope': __doc__, 'database': str(args.database), 'run_id': args.run_id,
              'records_scanned': records, 'retained_definitions': definitions,
              'candidate_paths': len(rows), 'candidate_names': len({r['card'] for r in rows}),
              'rows': rows, 'source_sha256': {name: hashlib.sha256(Path('scripts', name).read_bytes()).hexdigest()
                       for name in ['audit_runtime_nonmana_x_costs.py', 'audit_runtime_counter_removal_costs.py',
                                    'audit_runtime_self_sacrifice_costs.py']},
              'runtime_source_sha256': {name: hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in [
                  'crates/ironsmith-engine/src/game_loop/priority_cast.rs',
                  'crates/ironsmith-engine/src/game_loop/priority_state.rs',
                  'crates/ironsmith-engine/src/player.rs',
                  'crates/ironsmith-engine/src/effects/executor_trait.rs',
                  'crates/ironsmith-engine/src/effects/composition/choose_objects.rs',
                  'crates/ironsmith-engine/src/effects/cards/reveal_from_hand.rs',
                  'crates/ironsmith-engine/src/effects/zones/sacrifice.rs',
                  'crates/ironsmith-engine/src/effects/counters/remove_any_counters_from_source.rs']},
              'limitations': ['Source-reviewed references_cost_x implementations and transparent wrapper delegation only; other X values do not automatically enter ChoosingX.',
                              'Mana cost changes from other cards, affordability, summoning sickness and source restrictions require individual runtime probes.',
                              'Spell costs use a distinct X-bound implementation; no activation-defect inference is made for them.',
                              'No oracle text or string search is used to identify cost semantics; flattened_default_effects caches are skipped.']}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: report[k] for k in ['records_scanned', 'retained_definitions', 'candidate_paths', 'candidate_names']}))


if __name__ == '__main__':
    main()
