#!/usr/bin/env python3
"""Inventory coverage, never a substitute for behavioral verification."""
import collections
import json
from pathlib import Path
from coverage_candidates import merge_candidate_row

root = Path(__file__).resolve().parents[2]
fixtures = root / 'fixtures/custom_mechanics'
seeds = [fixtures / 'revolution.json', fixtures / 'msem-source.json']
records = {}
for path in seeds:
    for row in json.loads(path.read_text())['mechanics']:
        assert row['id'] not in records, f"duplicate seed ID: {row['id']}"
        records[row['id']] = {**row, '_candidate_files': [str(path.relative_to(root))]}
assert len(records) == 971, f"source inventory changed or lost variants: {len(records)}"
original_variant_count = len(records)
source_rules = json.loads((fixtures / 'source-rules.json').read_text())['formats']
additional_rule_ids = set()
derived_candidates = {}
declared_derived_rows = []

# A complete-source candidate explicitly selects the representative-body inventory
# for its source rows after smaller historical scenario cohorts. Original rule
# identity checks below still apply; this does not grant gameplay verification.
def inventory_candidate_order(path):
    fixture = json.loads(path.read_text())
    selected = fixture.get('inventory_selection') == 'complete_source_overlay'
    return (selected, path.name)

for path in sorted(fixtures.glob('*.json'), key=inventory_candidate_order):
    if path in seeds or path.name == 'source-rules.json':
        continue
    for row in json.loads(path.read_text()).get('mechanics', []):
        if row.get('source_inventory_id') and row['source_inventory_id'] != row['id']:
            declared_derived_rows.append((path, row))
            continue
        if row['id'] not in records:
            # Some sourced keyword rules or category sections have real cards
            # but no extracted reminder in the original 971-row inventory. Keep them additional to it.
            scope = 'rev' if row.get('scope') == 'revolution' else row.get('scope')
            matching = [rule for rule in [*source_rules.get(scope, {}).get('rules', []),
                                          *source_rules.get(scope, {}).get('sections', [])]
                        if rule['rule'].casefold() == row['keyword'].casefold()
                        and row.get('rule_reference') in
                        [rule['name'], *[sub['name'] for sub in rule.get('subrules', [])]]]
            assert len(matching) == 1, f"unknown source ID in {path}: {row['id']}"
            assert not row.get('original_reminder'), f"additional rule invented a reminder: {row['id']}"
            records[row['id']] = {**row, 'representative_cards': [], '_candidate_files': []}
            additional_rule_ids.add(row['id'])
        prior = records[row['id']]
        for field in ['keyword', 'original_reminder']:
            assert prior[field] == row[field], f"source altered: {row['id']} {field}"
        merge_candidate_row(prior, row, str(path.relative_to(root)),
                            json.loads(path.read_text()).get('inventory_selection') == 'complete_source_overlay')
        prior['_candidate_files'].append(str(path.relative_to(root)))
# Derived candidates are explicit proposals attached to a source variant,
# not new source rules and never substitutes for the 971 immutable seed IDs.
# Resolve after all real additional rules so file ordering cannot grant or
# remove provenance authority.
for path, row in declared_derived_rows:
    source_id = row['source_inventory_id']
    assert source_id in records, f"unknown source_inventory_id in {path}: {source_id}"
    assert row['id'] not in records, f"derived candidate collides with source ID: {row['id']}"
    source = records[source_id]
    for field in ['keyword', 'original_reminder']:
        assert source[field] == row[field], f"derived source altered: {row['id']} {field}"
    source_scope = 'rev' if source.get('scope') == 'revolution' else source.get('scope')
    derived_scope = 'rev' if row.get('scope') == 'revolution' else row.get('scope')
    assert source_scope == derived_scope, f"derived source scope altered: {row['id']}"
    prior = derived_candidates.get(row['id'])
    if prior is None:
        prior = {**row, 'representative_cards': [], '_candidate_files': []}
        derived_candidates[row['id']] = prior
    else:
        for field in ['source_inventory_id', 'keyword', 'original_reminder']:
            assert prior[field] == row[field], f"derived identity altered: {row['id']} {field}"
    merge_candidate_row(prior, row, str(path.relative_to(root)),
                        json.loads(path.read_text()).get('inventory_selection') == 'complete_source_overlay')
    prior['_candidate_files'].append(str(path.relative_to(root)))
    source.setdefault('_derived_candidate_ids', [])
    if row['id'] not in source['_derived_candidate_ids']:
        source['_derived_candidate_ids'].append(row['id'])

boundary_path = fixtures / 'rule-boundaries.json'
boundary = json.loads(boundary_path.read_text())['entries'] if boundary_path.exists() else []
by_rule = {(entry['scope'], entry['name'].casefold()): entry for entry in boundary if entry['kind'] in {'rules', 'sections'}}
counts = collections.Counter()
rows = []
multiface_layouts = {'doubleface', 'transform', 'transform_like', 'transformLike',
                    'modal_dfc', 'split', 'aftermath', 'flip', 'adventure', 'prepare'}

# Scope applies to exact full-card bodies, independently of the mechanic's
# ordinary reminder operation. Never reclassify all Compose/Reprise sources.
card_scope_exclusions = {}
for scope_path in sorted(fixtures.glob('*.json')):
    scope_overlay = json.loads(scope_path.read_text())
    if scope_overlay.get('user_decision', {}).get('authority') != 'explicit_user_scope_decision':
        continue
    for dependency in scope_overlay.get('full_card_dependencies', []):
        scope = 'rev' if dependency.get('scope') == 'revolution' else dependency.get('scope')
        source_key = dependency['source_card_key']
        identity = (scope, source_key)
        entry = card_scope_exclusions.setdefault(identity, {
            'source_card_key': source_key, 'mechanisms': set(), 'evidence_files': set(),
        })
        entry['mechanisms'].update(dependency['mechanism_dependencies'])
        entry['evidence_files'].add(str(scope_path.relative_to(root)))
        # Some cohorts retain a front-only key for a known linked physical card.
        # Resolve only explicitly enumerated physical names/set suffixes.
        full_name = dependency.get('full_name', '')
        if full_name and source_key.startswith(full_name + '_'):
            suffix = source_key[len(full_name):]
            for face_name in full_name.split('//'):
                alias = (scope, face_name + suffix)
                assert alias not in card_scope_exclusions or card_scope_exclusions[alias] is entry, f'ambiguous excluded source key: {alias}'
                card_scope_exclusions[alias] = entry

def full_card_scope_exclusion(row, card):
    scope = 'rev' if row.get('scope') == 'revolution' else row.get('scope')
    for key in [card.get('source_card_key'), card.get('id')]:
        if not key:
            continue
        for prefix in ['msem:', 'rev:', 'revolution:']:
            if key.startswith(prefix):
                key = key[len(prefix):]
                break
        if (scope, key) in card_scope_exclusions:
            entry = card_scope_exclusions[(scope, key)]
            return {'source_card_key': entry['source_card_key'],
                    'mechanisms': sorted(entry['mechanisms']),
                    'evidence_files': sorted(entry['evidence_files']),
                    'scope': 'excluded_full_card_body_by_explicit_user_decision'}
    return None

def source_body_coverage(row, cards):
    exclusions = [full_card_scope_exclusion(row, card) for card in cards]
    details = [{'id': card.get('id'), 'name': card.get('name'), **exclusion}
               for card, exclusion in zip(cards, exclusions) if exclusion]
    in_scope = [card for card, exclusion in zip(cards, exclusions) if not exclusion]
    return {'excluded_card_bodies': details,
            'in_scope_card_count': len(in_scope),
            'in_scope_full_frontend_candidates': sum(has_full_frontend_input(card) for card in in_scope),
            'mechanic_scope_unchanged': True}

def has_full_frontend_input(card):
    if card.get('rewrite_status') != 'frontend_materialized_unverified' or not card.get('executable_text'):
        return False
    sources = card.get('source_faces') or [card]
    layout = card.get('source_layout') or card.get('source_shape') or 'normal'
    if layout in multiface_layouts and len(sources) < 2:
        return False
    faces = card.get('executable_faces') or ([card] if len(sources) == 1 else [])
    return [face.get('name') for face in faces] == [face.get('name') for face in sources] and all(face.get('executable_text') for face in faces)

for key, row in records.items():
    # Current official Ascend was part of the historical 971-row probe list.
    official_control = row['keyword'].lower() == 'ascend' and "city's blessing" in row['original_reminder'].lower()
    has_reminder = bool(row.get('executable_reminder'))
    cards = row.get('representative_cards', [])
    body_scope = source_body_coverage(row, cards)
    full_cards = bool(cards) and not body_scope['excluded_card_bodies'] and all(has_full_frontend_input(card) for card in cards)
    status = 'official_control' if official_control else 'candidate_full_card' if full_cards else 'candidate_reminder_only' if has_reminder else 'pending_rewrite'
    source_scope = 'rev' if row.get('scope') == 'revolution' else row.get('scope')
    rule = by_rule.get((source_scope, row['keyword'].casefold()))
    if not official_control and rule and rule['classification'] == 'unsupported_requires_new_rules':
        status = 'unsupported_requires_new_rules'
    counts[status] += 1
    rows.append({'id': key, 'keyword': row['keyword'], 'scope': row.get('scope'), 'coverage': status,
                 'derived_candidate_ids': row.get('_derived_candidate_ids', []),
                 'source_body_scope': body_scope,
                 'candidate_merge': {'body_collisions': row.get('_source_body_collisions', []),
                                     'reminder_collisions': row.get('_reminder_collisions', [])},
                 'source_inventory_selection': {
                     'complete_source_overlay': row.get('_selected_complete_source_origin'),
                     'authority': 'explicit_complete_source_overlay' if row.get('_selected_complete_source_origin') else 'merged_candidate_discovery',
                     'exact_source_keys': sorted(card.get('source_card_key') or card.get('id') for card in cards),
                     'exact_source_faces': [
                         {'source_key': card.get('source_card_key') or card.get('id'),
                          'faces': [face.get('name') for face in card.get('source_faces') or [card]]}
                         for card in cards
                     ],
                     'body_collisions': row.get('_source_body_collisions', []),
                 },
                 'gameplay_verified': False, 'rule_boundary': rule['classification'] if rule else 'unmapped', 'unsupported_reason': rule['reason'] if status == 'unsupported_requires_new_rules' else None, 'candidate_files': row['_candidate_files'],
                 'note': 'Verification requires parent-run native evidence and source-equivalence review. Exact excluded card bodies do not exclude the ordinary mechanic operation.'})
derived_rows = []
derived_counts = collections.Counter()
for key, row in derived_candidates.items():
    cards = row.get('representative_cards', [])
    body_scope = source_body_coverage(row, cards)
    status = 'candidate_full_card' if cards and not body_scope['excluded_card_bodies'] and all(has_full_frontend_input(card) for card in cards) else 'candidate_reminder_only' if row.get('executable_reminder') else 'pending_rewrite'
    derived_counts[status] += 1
    derived_rows.append({'id': key, 'source_inventory_id': row['source_inventory_id'],
                         'keyword': row['keyword'], 'scope': row.get('scope'),
                         'coverage': status, 'gameplay_verified': False,
                         'source_body_scope': body_scope,
                         'candidate_merge': {'body_collisions': row.get('_source_body_collisions', []),
                                             'reminder_collisions': row.get('_reminder_collisions', [])},
                         'candidate_files': row['_candidate_files'],
                         'note': 'Derived executable proposal; never counted as an original source variant or additional sourced rule.'})
assert counts['official_control'] == 1, 'Expected one explicitly identified official control'
print(json.dumps({'source_variants': original_variant_count, 'additional_sourced_rules': len(additional_rule_ids),
                  'custom_variants': original_variant_count-counts['official_control'],
                  'counts': counts, 'rows': rows, 'derived_candidates': len(derived_candidates),
                  'derived_candidate_counts': derived_counts, 'derived_candidate_rows': derived_rows}, indent=2))
