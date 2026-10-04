#!/usr/bin/env python3
"""Replay canonical payloads for decoder-gap families, with broad negative controls."""
import argparse
import datetime
import json
from pathlib import Path
import re

from audit_runtime_corpus import Worker, checksum

PATTERN = re.compile(
    r"\bmiracle\s*\{|note the type of mana spent|choose (?:1|one),? (?:2|two),? or (?:3|three) at random|still resolves if its targets? become|adapt.*had no|had no.*counter",
    re.I,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", required=True, type=Path,
                        help="Canonical inventory emitted by audit_runtime_corpus.py; matching manifest.json required")
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--out", type=Path, default=Path("reports/runtime-audit/decoder-family-materialization.json"))
    parser.add_argument("--timeout", type=float, default=45)
    args = parser.parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((args.inventory.parent / "manifest.json").read_text())
    inventory = json.loads(args.inventory.read_text())
    selected = [card for card in inventory["cards"] if PATTERN.search(card["oracle_text"])
                or card["name"] == "Biomancer's Familiar"]
    before = checksum(args.binary)
    worker = Worker(args.binary.resolve(), args.timeout, args.out.with_suffix(".stderr.log"))
    rows = []
    try:
        for original in selected:
            request = dict(original, contracts_only=True, actions_only=False, include_definition=True)
            result = worker.run(request)
            rows.append({"card": request["name"], "oracle_text": request["oracle_text"], "result": result})
            print(request["name"], result["status"], flush=True)
    finally:
        worker.close()
    report = {
        "scope": "Fresh canonical payload compilation and artifact materialization for text-selected decoder-gap families. These are loadability results, not card gameplay outcomes. No whole-card correctness claim.",
        "selection_pattern": PATTERN.pattern, "selected_cards": len(selected), "rows": rows,
        "provenance": {
            "binary": str(args.binary.resolve()), "binary_sha256": before,
            "binary_unchanged": before == checksum(args.binary),
            "inventory": str(args.inventory.resolve()), "inventory_sha256": checksum(args.inventory),
            "cards_sha256": manifest["cards_sha256"], "driver_sha256": checksum(__file__),
            "worker_driver_sha256": checksum(Path(__file__).with_name("audit_runtime_corpus.py")),
            "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        },
    }
    temporary = args.out.with_suffix(".tmp")
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.replace(args.out)


if __name__ == "__main__":
    main()
