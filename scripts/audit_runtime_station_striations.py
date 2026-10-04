#!/usr/bin/env python3
"""Find canonical station striations spanning multiple source-text lines.

This enumerates a source shape; it does not classify the compiled result or
claim that every station card has been checked at runtime.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INVENTORY = ROOT / 'reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json'
MARKER = re.compile(r'^(\d+)\+\s*\|\s*(.*)$')


def striations(text):
    if not re.search(r'\bStation\b', text):
        return []
    lines = text.splitlines()
    markers = [(i, MARKER.match(line)) for i, line in enumerate(lines) if MARKER.match(line)]
    result = []
    for ordinal, (start, match) in enumerate(markers):
        end = markers[ordinal + 1][0] if ordinal + 1 < len(markers) else len(lines)
        following = [line for line in lines[start + 1:end] if line.strip()]
        if following:
            result.append({'threshold': int(match[1]), 'station_line': start,
                           'station_symbol_line': lines[start], 'following_lines': following})
    return result


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/runtime-audit/station-striation-candidates.json')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        assert striations('Station\n8+ | Flying') == []
        assert striations('8+ | Flying\n{R}: Pump.') == []
        assert striations('Station\n8+ | {R}: Pump.') == []
        assert len(striations('Station\n8+ | Flying\n{R}: Pump.')) == 1
        rows = striations('Station\n1+ | Draw.\nExtra first.\n8+ | Flying\nExtra second.')
        assert [r['threshold'] for r in rows] == [1, 8]
        assert rows[0]['following_lines'] == ['Extra first.']
        assert striations('Station\n8+ | Flying\n\n') == []
        print('Seven station-source-shape checks passed.')
        return
    inventory = json.loads(args.inventory.read_text())
    rows = []
    for card in inventory['cards']:
        for block in striations(card['oracle_text']):
            rows.append({'card': card['name'], **block, 'parse_input': card['parse_input'],
                         'status': 'structural_candidate_only'})
    out = {'generated_at': datetime.now(timezone.utc).isoformat(),
           'scope': 'All explicit station striations with another nonempty source line before the next station symbol or end of text. Structural candidates only; no automatic defect attribution.',
           'inventory': str(args.inventory.relative_to(ROOT)), 'inventory_sha256': sha(args.inventory),
           'generator': str(Path(__file__).resolve().relative_to(ROOT)), 'generator_sha256': sha(Path(__file__)),
           'records_scanned': len(inventory['cards']), 'candidate_names': len({r['card'] for r in rows}),
           'candidate_striations': len(rows), 'rows': rows, 'all_cards_verified': False}
    args.output.write_text(json.dumps(out, indent=2) + '\n')
    print(json.dumps({'records': out['records_scanned'], 'candidate_names': out['candidate_names'], 'striations': len(rows)}))


if __name__ == '__main__':
    main()
