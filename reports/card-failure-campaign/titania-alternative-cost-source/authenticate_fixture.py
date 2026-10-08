#!/usr/bin/env python3
"""Offline byte/metadata authentication only; never invokes the compiler or tests."""
import hashlib
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[3]
fixture = json.loads((root / 'fixtures/titania_alternative_cost.json.fixture').read_bytes())
source = Path(sys.argv[1])
raw = source.read_bytes()
assert hashlib.sha256(raw).hexdigest() == fixture['source_dataset_sha256']
rows = [card for card in json.loads(raw) if card.get('oracle_id') == fixture['card']['oracle_id']]
assert len(rows) == 1
card = rows[0]
assert card == fixture['card'], 'entire retained official record must agree'
expected = '\n'.join([
    f"Mana cost: {card['mana_cost']}", f"Type: {card['type_line']}",
    f"Power/Toughness: {card['power']}/{card['toughness']}", card['oracle_text'],
])
assert fixture['text'] == expected
print('Authenticated exact official record, metadata and complete raw body; no compiler or tests executed.')
