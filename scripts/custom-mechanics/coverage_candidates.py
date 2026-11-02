"""Merge reporting candidates by exact physical identity, never by file order.

No runtime or behavioral evidence is inferred. Explicit complete-source overlays
select definitions; contradictory unselected definitions remain visible gaps.
"""
import copy
import json

METADATA = ('supplied_card_definitions', 'custom_keyword_identities', 'custom_keyword_payments', 'custom_subtypes',
            'custom_keyword_counter_definitions')

def namespace(value):
    return 'rev' if value == 'revolution' else value

def source_key(row, card):
    key = card.get('source_card_key') or card.get('id')
    assert isinstance(key, str) and key.strip(), f"unidentified physical source in {row['id']}"
    key = key.strip()
    if ':' in key:
        prefix, tail = key.split(':', 1)
        if prefix in ('msem', 'rev', 'revolution'):
            assert namespace(prefix) == namespace(row.get('scope')), f"source namespace mismatch: {key}"
            key = tail
    assert key.strip(), f"empty physical source identity in {row['id']}"
    return key

def original_faces(card):
    result = []
    for face in card.get('source_faces') or [{'name': card.get('name'), 'original_text': card.get('original_text')}]:
        if face.get('name'):
            result.append(face)
            continue
        # Complete corpus records are another retained schema, not anonymous
        # display labels. Only explicit cardName/cardName2 identify faces.
        for suffix, role in [('', 'front'), ('2', 'back')]:
            name = face.get('cardName' + suffix)
            if not name:
                continue
            normalized = dict(face) if not suffix else {}
            normalized.update(name=name, face=role)
            if not suffix and name == card.get('name') and card.get('original_text') is not None:
                text = card['original_text']
            else:
                lines = []
                for source, label in [('manaCost', 'Mana cost'), ('typeLine', 'Type')]:
                    if face.get(source + suffix):
                        lines.append(f"{label}: {face[source + suffix]}")
                if face.get('power' + suffix) or face.get('toughness' + suffix):
                    lines.append(f"Power/Toughness: {face.get('power' + suffix, '')}/{face.get('toughness' + suffix, '')}")
                for source, label in [('loyalty', 'Loyalty'), ('defense', 'Defense')]:
                    if face.get(source + suffix):
                        lines.append(f"{label}: {face[source + suffix]}")
                if face.get('rulesText' + suffix):
                    lines.append(face['rulesText' + suffix])
                text = '\n'.join(lines)
            normalized['original_text'] = text
            result.append(normalized)
        if not face.get('cardName') and not face.get('cardName2'):
            result.append(face)
    return result

def executable_faces(card):
    faces = card.get('executable_faces') or []
    if not faces and card.get('executable_text'):
        faces = [{'name': card.get('name'), 'executable_text': card['executable_text']}]
    return [face for face in faces if face.get('executable_text')]

def program_signature(card, face):
    value = {'name': face.get('name'), 'executable_text': face['executable_text'],
             'token': face.get('token', card.get('token', False))}
    value.update({key: face.get(key, card.get(key, [])) for key in METADATA})
    return json.dumps(value, sort_keys=True, ensure_ascii=False)

def choose_variant(variants, signature):
    """A unique selected definition wins; equivalent ordinary copies may agree."""
    selected = [item for item in variants if item['selected']]
    pool = selected or variants
    signatures = {signature(item) for item in pool}
    if len(signatures) != 1:
        return None, False
    # This is only a provenance tie-break among equivalent definitions.
    return min(pool, key=lambda item: item['origin']), True

def merged_card(key, variants):
    preferred = [item for item in variants if item['selected']] or variants
    base = copy.deepcopy(min(preferred, key=lambda item: (-len(original_faces(item['card'])), item['origin']))['card'])
    original = {}; programs = {}; collisions = []; blocked = False; unidentified = []
    for item in variants:
        card = item['card']
        for face in original_faces(card):
            if not face.get('name'):
                blocked = True
                unidentified.append(copy.deepcopy(face))
                collisions.append({'source_key': key, 'kind': 'unidentified_source_face', 'candidate_files': [item['origin']], 'resolved_by_selected_overlay': False})
                continue
            original.setdefault(face['name'], []).append({**item, 'face': face})
        for face in executable_faces(card):
            if not face.get('name'):
                blocked = True
                collisions.append({'source_key': key, 'kind': 'unidentified_executable_face', 'candidate_files': [item['origin']], 'resolved_by_selected_overlay': False})
                continue
            programs.setdefault(face['name'], []).append({**item, 'face': face})
    merged_original = []; merged_programs = []
    for name, versions in original.items():
        populated = [item for item in versions if item['face'].get('original_text') is not None] or versions
        signature = lambda item: item['face'].get('original_text')
        chosen, agreed = choose_variant(populated, signature)
        all_values = {signature(item) for item in populated}
        if len(all_values) > 1:
            collisions.append({'source_key': key, 'face': name, 'kind': 'original_source_definition',
                               'candidate_files': sorted({item['origin'] for item in populated}),
                               'resolved_by_selected_overlay': agreed and any(item['selected'] for item in populated)})
        if not agreed:
            blocked = True
            chosen = min(populated, key=lambda item: item['origin'])
        merged_original.append(copy.deepcopy(chosen['face']))
    for name, versions in programs.items():
        signature = lambda item: program_signature(item['card'], item['face'])
        chosen, agreed = choose_variant(versions, signature)
        all_values = {signature(item) for item in versions}
        if len(all_values) > 1:
            collisions.append({'source_key': key, 'face': name, 'kind': 'executable_definition_or_metadata',
                               'candidate_files': sorted({item['origin'] for item in versions}),
                               'resolved_by_selected_overlay': agreed and any(item['selected'] for item in versions)})
        if not agreed:
            blocked = True
            continue
        face = copy.deepcopy(chosen['face'])
        for field in METADATA:
            if field not in face and field in chosen['card']:
                face[field] = copy.deepcopy(chosen['card'][field])
        merged_programs.append(face)
    # Preserve physical front/back ordering from the authoritative original
    # packet. Additional named faces follow in their first recorded order.
    preferred_names = [face['name'] for face in original_faces(base) if face.get('name')]
    rank = {name: index for index, name in enumerate(preferred_names)}
    roles = {'front': 0, 'back': 1}
    merged_original.sort(key=lambda face: (roles.get(face.get('face'), rank.get(face['name'], len(rank))), face['name']))
    ordered_names = [face['name'] for face in merged_original]
    merged_programs.sort(key=lambda face: ordered_names.index(face['name']) if face['name'] in ordered_names else len(ordered_names))
    base['source_card_key'] = key
    base['source_faces'] = [*merged_original, *unidentified]
    base['executable_faces'] = [] if blocked else merged_programs
    first = next((face for face in merged_programs if face['name'] == base.get('name')), None)
    if first is None and merged_programs:
        first = merged_programs[0]
    base['executable_text'] = None if blocked or first is None else first['executable_text']
    if first is not None and not blocked:
        for field in METADATA:
            if field in first:
                base[field] = copy.deepcopy(first[field])
        base['rewrite_status'] = 'frontend_materialized_unverified'
    elif blocked:
        base['rewrite_status'] = 'conflicting_source_candidates'
    return base, collisions

def merge_candidate_row(prior, candidate, origin, selected=False):
    assert prior['id'] == candidate['id'], "candidate row identity changed"
    if prior.get('source_inventory_id') or candidate.get('source_inventory_id'):
        assert prior.get('source_inventory_id', prior['id']) == candidate.get('source_inventory_id', candidate['id']), "derived source identity changed"
    for field in ('keyword', 'original_reminder'):
        assert prior.get(field) == candidate.get(field), f"source altered: {candidate['id']} {field}"
    assert namespace(prior.get('scope')) == namespace(candidate.get('scope')), f"source scope altered: {candidate['id']}"
    if selected:
        previous = prior.get('_selected_complete_source_origin')
        assert not previous or previous == origin, f"conflicting complete-source inventory overlays: {previous} and {origin}"
        prior['_selected_complete_source_origin'] = origin
    candidates = prior.get('_source_body_candidates')
    if candidates is None:
        candidates = {}
        seed_origin = prior.get('_candidate_files', ['<seed>'])[0] if prior.get('_candidate_files') else '<seed>'
        for card in prior.get('representative_cards', []):
            key = source_key(prior, card)
            candidates.setdefault(key, []).append({'card': copy.deepcopy(card), 'origin': seed_origin, 'selected': False})
        prior['_source_body_candidates'] = candidates
    for card in candidate.get('representative_cards', []):
        key = source_key(candidate, card)
        item = {'card': copy.deepcopy(card), 'origin': origin, 'selected': selected}
        if item not in candidates.setdefault(key, []):
            candidates[key].append(item)
    prior['_source_body_collisions'] = []
    prior['representative_cards'] = []
    for key, variants in candidates.items():
        card, collisions = merged_card(key, variants)
        prior['representative_cards'].append(card)
        prior['_source_body_collisions'].extend(collisions)
    reminders = prior.setdefault('_reminder_candidates', [])
    if not reminders and prior.get('executable_reminder'):
        reminders.append({'value': prior['executable_reminder'], 'origin': '<existing>', 'selected': False})
    if candidate.get('executable_reminder'):
        entry = {'value': candidate['executable_reminder'], 'origin': origin, 'selected': selected}
        if entry not in reminders:
            reminders.append(entry)
    chosen, agreed = choose_variant(reminders, lambda item: item['value']) if reminders else (None, True)
    prior['executable_reminder'] = chosen['value'] if chosen and agreed else None
    prior['_reminder_collisions'] = [] if len({item['value'] for item in reminders}) <= 1 else [{
        'candidate_files': sorted({item['origin'] for item in reminders}),
        'resolved_by_selected_overlay': agreed and any(item['selected'] for item in reminders)}]
    return prior
