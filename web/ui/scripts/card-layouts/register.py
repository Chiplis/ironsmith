#!/usr/bin/env python3
"""Generate printing/face text registrations from local Vision OCR.

Inputs are cached Scryfall scans/metadata, not card-name-specific layout code.
Run recognize.m on the cached normal images into ocr.jsonl first.
"""
import argparse
import difflib
import json
import re
from pathlib import Path


def normalized(text):
    return re.sub(r'[^a-z0-9]+', ' ', re.sub(r'\{[^}]+\}', ' ', text.lower())).strip()


def similarity(needle, paragraph):
    a, b = normalized(needle), normalized(paragraph)
    if not a or not b:
        return 0
    if a == b:
        return 2
    if a in b:
        return 1
    aw, bw = a.split(), b.split()
    best = 0
    for size in range(max(1, len(aw)-1), min(len(bw), len(aw)+2)+1):
        for start in range(len(bw)-size+1):
            best = max(best, difflib.SequenceMatcher(None, a, ' '.join(bw[start:start+size])).ratio())
    return best


def fields_for(printing, face_index):
    faces = printing.get('card_faces') or [printing]
    if face_index is not None:
        faces = [faces[face_index]]
    result = []
    for i, face in enumerate(faces):
        for kind, key in [('name', 'name'), ('type', 'type_line'), ('flavor', 'flavor_text')]:
            if face.get(key):
                result.append(dict(kind=kind, text=face[key], face=face_index if face_index is not None else i, lines=[]))
        if face.get('power') is not None and face.get('toughness') is not None:
            result.append(dict(kind='stats', text=face['power']+'/'+face['toughness'], face=i, lines=[]))
        for index, text in enumerate(face.get('oracle_text', '').split('\n')):
            if text.strip():
                result.append(dict(kind='rule', text=text, face=face_index if face_index is not None else i, index=index, lines=[]))
    return result


def attach_errata(fields, unmatched):
    """Errata leaves printed wording that no longer matches the current text.

    Those lines still occupy the rules box; attach them to the rules field that
    found no print so the live text masks and replaces them instead of leaving
    the stale paragraph visible.
    """
    lines_of = lambda kind: [line for field in fields if field['kind'] == kind for line in field['lines']]
    type_lines, stats_lines, flavor_lines = lines_of('type'), lines_of('stats'), lines_of('flavor')
    empty = sorted((field for field in fields if field['kind'] == 'rule' and not field['lines']), key=lambda field: field['index'])
    if not type_lines or not empty:
        return
    top = max(line['y'] + line['height'] for line in type_lines)
    bottom = min([line['y'] for line in stats_lines] + [line['y'] for line in flavor_lines] + [.94])
    footer = re.compile(r'illus|©|™|wizards of the coast', re.I)
    candidates = sorted((line for line in unmatched if top < line['y'] and line['y'] + line['height'] <= bottom
                         and line['x'] < .5 and line['width'] > .15 and not footer.search(line['text'])), key=lambda line: line['y'])
    if not candidates:
        return
    paragraphs = [[candidates[0]]]
    pitches = [b['y'] - a['y'] for a, b in zip(candidates, candidates[1:])]
    pitch = sorted(pitches)[len(pitches) // 2] if pitches else 0
    for previous, line in zip(candidates, candidates[1:]):
        if len(empty) > len(paragraphs) and line['y'] - previous['y'] > pitch * 1.6:
            paragraphs.append([])
        paragraphs[-1].append(line)
    for field, paragraph in zip(empty, paragraphs):
        field['errata'] = True
        field['lines'].extend({key: line[key] for key in ['text', 'x', 'y', 'width', 'height']} for line in paragraph)


def register(case, printing, observations):
    fields = fields_for(printing, case.get('face'))
    unmatched = []
    for line in observations:
        if line['confidence'] < .25 or not line['text'].strip():
            continue
        scores = [(max(similarity(line['text'], field['text']), similarity(line['text'], re.sub(r'this (?:creature|artifact|enchantment|Saga|token)', printing['name'].split(' // ')[0], field['text'], flags=re.I))), index) for index, field in enumerate(fields)]
        if not scores:
            continue
        score, index = max(scores, key=lambda pair:(pair[0], -len(fields[pair[1]]['text']), not fields[pair[1]]['lines'], -pair[1]))
        if score < .57:
            unmatched.append(line)
            continue
        field = fields[index]
        # Footer artist/copyright words can occur in long rules paragraphs.
        if line['y'] > .94 and field['kind'] != 'stats':
            continue
        # A generic mana cost digit is a substring of most P/T values.
        if field['kind'] == 'stats' and line['y'] < .75:
            unmatched.append(line)
            continue
        field['lines'].append({key:line[key] for key in ['text','x','y','width','height']})
    attach_errata(fields, unmatched)
    # Translated names may run past the printed name: record where the mana
    # cost (or other print on the same row) begins so the field can grow to it.
    for field in fields:
        if field['kind'] != 'name' or not field['lines']:
            continue
        row = field['lines'][0]
        blockers = [line['x'] for line in unmatched if line['x'] >= row['x'] + row['width'] - .01
                    and line['y'] < row['y'] + row['height'] and line['y'] + line['height'] > row['y']]
        if blockers:
            field['limit'] = min(blockers)
    for field in fields:
        rows = field['lines']
        if rows:
            x, y = min(r['x'] for r in rows), min(r['y'] for r in rows)
            field['bounds'] = dict(x=x,y=y,width=max(r['x']+r['width'] for r in rows)-x,height=max(r['y']+r['height'] for r in rows)-y)
    return dict(id=case['id'], face=case.get('face'), source=case['source'], layout=case['layout'],
                set=case.get('set'), collector_number=case.get('collector_number'), fields=fields)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('cache', type=Path)
    parser.add_argument('--output', type=Path, default=Path(__file__).resolve().parents[2]/'src/lib/card-region-catalog.generated.js')
    args = parser.parse_args()
    manifest = json.loads((Path(__file__).resolve().parents[2]/'tests/card-frame-layout-cases.json').read_text())
    observations = {Path(item['path']).name:item['lines'] for item in map(json.loads,(args.cache/'ocr.jsonl').read_text().splitlines())}
    registrations = []
    for case in manifest:
        printing = json.loads((args.cache/(case['slug']+'.json')).read_text())
        registrations.append(register(case,printing,observations[case['slug']+'-normal.jpg']))
    args.output.write_text('// Generated by scripts/card-layouts/register.py from pinned scans and metadata.\nexport default '+json.dumps(registrations,separators=(',',':'))+';\n')
    missing = [(c['id'],f['kind'],f['text']) for c in registrations for f in c['fields'] if not f.get('bounds')]
    (args.cache/'registration-gaps.json').write_text(json.dumps(missing,indent=2))
    print(f'{len(registrations)} registrations; {len(missing)} fields need review')

if __name__ == '__main__':
    main()
