#!/usr/bin/env python3
"""Screen canonical text for variable resources paid into mana abilities (candidates only)."""
import argparse
import hashlib
import json
import re
from pathlib import Path


def scan(cards):
    candidates = []
    for card in cards:
        for line in card["oracle_text"].splitlines():
            if ":" not in line:
                continue
            cost, effect = line.split(":", 1)
            if not re.search(r"\badd\b", effect, re.I):
                continue
            counter = bool(re.search(r"remove (?:X |any number of |one or more ).*?counters?", cost, re.I))
            sacrifice = bool(re.search(r"sacrifice X\b", cost, re.I))
            mana_x = "{X}" in cost
            if counter or sacrifice or mana_x:
                candidates.append({"card": card["name"], "line": line,
                                   "variable_counter_cost": counter,
                                   "variable_sacrifice_cost": sacrifice,
                                   "variable_mana_cost": mana_x})
    return candidates


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    raw = args.inventory.read_bytes()
    inventory = json.loads(raw)
    candidates = scan(inventory["cards"])
    report = {"scope": "Same-line variable counter, sacrifice, or X mana costs before an Add clause; textual candidates, not proof of mana-ability classification or a defect.",
              "inventory": str(args.inventory), "inventory_sha256": hashlib.sha256(raw).hexdigest(),
              "payload_count": len(inventory["cards"]), "candidate_count": len(candidates),
              "candidates": candidates,
              "limitations": "Oracle-text heuristic; implicit quantities, multi-line references, and unusual wording can be missed. The full compiled program and actual legal scenarios must validate each candidate."}
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"{len(candidates)} candidate abilities across {len(inventory['cards'])} canonical payloads")


if __name__ == "__main__":
    main()
