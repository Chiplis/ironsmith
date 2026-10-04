#!/usr/bin/env python3
"""Screen pinned compiled definitions for plural Object-target damage consumers.

Read-only candidate inventory. It cannot establish reachability or correctness.
The observed runtime lead is DealDamageEffect's Object path selecting one object,
while counted AnyTarget/ObjectOrPlayer has separate batch handling.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def counted_object_target(target):
    counts = []
    while isinstance(target, dict):
        if "WithCount" in target:
            target, count = target["WithCount"]
            counts.append(count)
        elif "Target" in target:
            target = target["Target"]
        elif "SurfaceHinted" in target:
            target = target["SurfaceHinted"]["spec"]
        else:
            break
    plural = any(c.get("max") is None or c.get("max", 0) > 1
                 or c.get("dynamic_x") or c.get("up_to_x") for c in counts)
    return counts if plural and isinstance(target, dict) and "Object" in target else None


def walk(value, path="/definition"):
    if isinstance(value, dict):
        if value.get("kind") == "DealDamageEffect":
            counts = counted_object_target(value["payload"].get("target"))
            if counts:
                yield {"path": path, "counts": counts,
                       "target": value["payload"]["target"],
                       "amount": value["payload"].get("amount")}
        for key, child in value.items():
            if key != "flattened_default_effects":
                yield from walk(child, f"{path}/{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk(child, f"{path}/{index}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, default=Path("reports/runtime-audit/actions/results.sqlite3"))
    parser.add_argument("--run-id", default="e17a4980b0b92c7a5a4cead2")
    parser.add_argument("--output", type=Path, default=Path("reports/runtime-audit/counted-damage-target-candidates.json"))
    args = parser.parse_args()
    connection = sqlite3.connect(f"file:{args.database.resolve()}?mode=ro", uri=True)
    rows, seen, definitions = [], 0, 0
    for name, raw in connection.execute("SELECT card_name,result_json FROM result WHERE run_id=? ORDER BY card_name", (args.run_id,)):
        seen += 1
        result = json.loads(raw)
        definition = result.get("definition")
        if not definition:
            continue
        definitions += 1
        for finding in walk(definition):
            rows.append({"card": name, "artifact_checksum": result.get("artifact_checksum"), **finding})
    output = {"scope": "Typed plural Object-target DealDamageEffect paths; duplicate flattened effect cache skipped. Structural candidates only; no execution promotion.",
              "database": str(args.database), "run_id": args.run_id,
              "records_scanned": seen, "retained_definitions": definitions,
              "candidate_paths": len(rows), "candidate_names": len({r["card"] for r in rows}),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "limitations": ["Does not infer legal multiple-target reachability, branch conditions or source presence.",
                              "Does not screen other damage effect kinds, implicit plural targets, or fixed single targets.",
                              "Names can include combined/front-face aliases; promote only explicit legal runtime evidence."],
              "rows": rows}
    args.output.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps({k: output[k] for k in ("records_scanned", "retained_definitions", "candidate_paths", "candidate_names")}))


if __name__ == "__main__":
    main()
