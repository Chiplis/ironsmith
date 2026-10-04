#!/usr/bin/env python3
"""Inventory typed player choices and candidate empty/exhausted-choice branches."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1]


def choices(value, path=""):
    if isinstance(value, dict):
        if value.get("kind") == "ChoosePlayerEffect":
            yield path, value["payload"]
        for key, child in value.items():
            if key != "flattened_default_effects":
                yield from choices(child, f"{path}/{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from choices(child, f"{path}/{index}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, default=ROOT / "reports/runtime-audit/corpus/results.sqlite3")
    parser.add_argument("--run-id", default="267a16aff3b321196397d0b4")
    parser.add_argument("--output", type=Path, default=ROOT / "reports/runtime-audit/player-choice-filter-candidates.json")
    args = parser.parse_args()
    db = sqlite3.connect(f"file:{args.database}?mode=ro", uri=True)
    db.execute("BEGIN")
    inventory_count = db.execute("select count(*) from result where run_id=?", (args.run_id,)).fetchone()[0]
    definitions = 0
    rows = []
    for name, raw in db.execute("select card_name,result_json from result where run_id=? order by card_name", (args.run_id,)):
        record = json.loads(raw)
        definition = record.get("definition")
        if definition is None:
            continue
        definitions += 1
        for path, payload in choices(definition):
            filter_value = payload.get("filter")
            filter_kind = filter_value if isinstance(filter_value, str) else next(iter(filter_value or {}), "missing")
            exclusions = payload.get("excluded_tags", [])
            reasons = []
            if exclusions:
                reasons.append("prior distinct choices can exhaust the remaining player set")
            if filter_kind in {"CastCardTypeThisTurn", "OpponentWithMoreControlledObjectsThan"}:
                reasons.append("qualifying history or comparison can have no matching player")
            rows.append({
                "card": name, "path": path, "artifact_checksum": record.get("artifact_checksum"),
                "filter_kind": filter_kind, "filter": filter_value,
                "tag": payload.get("tag"), "excluded_tags": exclusions,
                "review_reasons": reasons, "status": "structural_candidate_only" if reasons else "inventory_only",
            })
    db.close()
    candidate_names = sorted({row["card"] for row in rows if row["review_reasons"]})
    report = {
        "scope": "Typed ChoosePlayerEffect inventory and conservative empty/distinct-choice exhaustion screen. No runtime verdict or card clearance.",
        "run_id": args.run_id, "payloads_inspected": inventory_count,
        "retained_definitions_inspected": definitions, "choice_paths": len(rows),
        "choice_cards": len({row["card"] for row in rows}),
        "filter_path_counts": dict(Counter(row["filter_kind"] for row in rows)),
        "candidate_names": candidate_names, "rows": rows,
        "provenance": {"database": str(args.database), "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
        "limitations": [
            "A candidate can already be guarded or safely handle an empty choice; only actual legal gameplay establishes a defect.",
            "Consumers and cross-effect tag lifetimes are not inferred by this screen.",
            "Other choice-effect kinds, uncommon player filters, player departure and range-of-influence rules require separate coverage.",
            "Absence of a retained definition is not an inspected or passing card. Duplicate flattened effect caches are skipped.",
        ],
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ("payloads_inspected", "retained_definitions_inspected", "choice_paths", "choice_cards", "candidate_names")}))


if __name__ == "__main__":
    main()
