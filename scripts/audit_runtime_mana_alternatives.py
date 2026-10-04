#!/usr/bin/env python3
"""Find canonical payloads with repeated mana-symbol alternatives; candidates only."""
import argparse
import hashlib
import json
import re
from pathlib import Path


def scan(document):
    matches = []
    for card in document['cards']:
        lines = [line for line in card['oracle_text'].splitlines()
                 if re.search(r'\b[aA]dd\b', line)
                 and re.search(r'((?:\{[WUBRGC]\}){2,}).*\bor\b.*\{[WUBRGC]\}', line)]
        if lines:
            matches.append({'card': card['name'], 'matched_lines': lines,
                            'classification': 'textual_candidate_requires_execution'})
    return matches


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    raw = args.inventory.read_bytes()
    document = json.loads(raw)
    matches = scan(document)
    report = {'inventory': str(args.inventory),
              'inventory_sha256': hashlib.sha256(raw).hexdigest(),
              'scanned_payloads': len(document['cards']),
              'candidate_payloads': len(matches),
              'scope': 'An Add clause containing consecutive ordinary WUBRGC symbols and an or alternative with mana symbols on the same Oracle line.',
              'limitations': 'Does not cover number words, hybrid or variable symbols, cross-line choices, or dynamic amounts. Candidates are not confirmed defects.',
              'candidates': matches}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'scanned_payloads': report['scanned_payloads'],
                      'candidate_payloads': len(matches)}))


if __name__ == '__main__':
    main()
