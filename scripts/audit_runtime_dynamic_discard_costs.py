#!/usr/bin/env python3
"""Screen pinned compiled definitions for non-Fixed discard additional costs.

Read-only candidate inventory. It cannot establish reachability or correctness.
The observed runtime lead is DiscardEffect cost validation rejecting non-Fixed counts.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def walk(value, path="/definition"):
    if isinstance(value, dict):
        if value.get("kind") == "DiscardEffect":
            count = value["payload"].get("count")
            if not (isinstance(count, dict) and "Fixed" in count):
                yield {"path": path, "count": count, "player": value["payload"].get("player"), "random": value["payload"].get("random", False)}
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
    parser.add_argument("--output", type=Path, default=Path("reports/runtime-audit/dynamic-discard-cost-candidates.json"))
    parser.add_argument("--all-costs", action="store_true", help="Scan every structurally typed TotalCost, including alternatives and abilities")
    args = parser.parse_args()
    connection = sqlite3.connect(f"file:{args.database.resolve()}?mode=ro", uri=True)
    rows, seen, definitions = [], 0, 0
    def cost_roots(value, path="/definition"):
        if isinstance(value, dict):
            kind = value.get("kind")
            if isinstance(kind, dict) and set(kind) in ({"All"}, {"OneOf"}):
                yield value, path
                return
            for key, child in value.items():
                if key != "flattened_default_effects":
                    yield from cost_roots(child, f"{path}/{key}")
        elif isinstance(value, list):
            for index, child in enumerate(value):
                yield from cost_roots(child, f"{path}/{index}")
    for name, raw in connection.execute("SELECT card_name,result_json FROM result WHERE run_id=? ORDER BY card_name", (args.run_id,)):
        seen += 1
        result = json.loads(raw)
        definition = result.get("definition")
        if not definition:
            continue
        definitions += 1
        roots = cost_roots(definition) if args.all_costs else [(definition.get("additional_cost"), "/definition/additional_cost")]
        for cost, cost_path in roots:
            for finding in walk(cost, cost_path):
                rows.append({"card": name, "artifact_checksum": result.get("artifact_checksum"), "cost_root": cost_path, **finding})
    output = {"scope": ("Typed non-Fixed DiscardEffect paths within every structurally typed TotalCost. Structural candidates only; no execution promotion." if args.all_costs else "Typed non-Fixed DiscardEffect paths within mandatory spell additional_cost only. Structural candidates only; no execution promotion."),
              "database": str(args.database), "run_id": args.run_id,
              "records_scanned": seen, "retained_definitions": definitions,
              "candidate_paths": len(rows), "candidate_names": len({r["card"] for r in rows}),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "limitations": ["Default scans mandatory additional_cost only; --all-costs scans structural All/OneOf TotalCost roots, including nested definitions. Non-discard effects are not screened.",
                              "Does not establish action availability or legal X values; independent full canonical fixtures required."],
              "rows": rows}
    args.output.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps({k: output[k] for k in ("records_scanned", "retained_definitions", "candidate_paths", "candidate_names")}))


if __name__ == "__main__":
    main()
