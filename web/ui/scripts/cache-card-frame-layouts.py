#!/usr/bin/env python3
"""Cache pinned audit scans. Usage: python scripts/cache-card-frame-layouts.py DIRECTORY"""
import concurrent.futures
import json
import pathlib
import sys
import time
import urllib.request

root = pathlib.Path(__file__).resolve().parents[3]
cases = json.loads((root / 'web/ui/tests/card-frame-layout-cases.json').read_text())
regressions = json.loads((root / 'web/ui/tests/card-frame-regression-cases.json').read_text())
cases = list({case['slug']: case for case in cases + regressions}.values())
destination = pathlib.Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
catalog = {card['id']: card for card in json.loads((root / 'cards.json').read_text())}

def fetch(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'Ironsmith card-frame regression audit', 'Accept': 'application/json,image/jpeg'})
    with urllib.request.urlopen(request, timeout=45) as response:
        return response.read()

jobs = []
for case in cases:
    path = destination / (case['slug'] + '.json')
    if not path.exists():
        card = catalog.get(case['id'])
        if card is None:
            card = json.loads(fetch('https://api.scryfall.com/cards/' + case['id']))
            time.sleep(.12)
        path.write_text(json.dumps(card, indent=2))
    card = json.loads(path.read_text())
    face = card['card_faces'][case['face']] if 'face' in case else card
    for variant in ['normal', 'art_crop']:
        image = destination / (case['slug'] + '-' + variant + '.jpg')
        url = face.get('image_uris', {}).get(variant)
        if url and not image.exists():
            jobs.append((image, url))

def download(job):
    path, url = job
    for attempt in range(3):
        try:
            path.write_bytes(fetch(url))
            return path.name
        except Exception:
            if attempt == 2:
                raise
            time.sleep(attempt + 1)

with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    for index, name in enumerate(pool.map(download, jobs)):
        if index % 10 == 0:
            print(f'{index + 1}/{len(jobs)} {name}', flush=True)
print(f'Cached {len(cases)} card faces in {destination}', flush=True)
