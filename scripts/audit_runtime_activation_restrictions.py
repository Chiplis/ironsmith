#!/usr/bin/env python3
"""Inventory authored activation gates and their typed wire representation, without clearing cards."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def walk(node, path="definition"):
    if isinstance(node, dict):
        if node.get("additional_restrictions"):
            yield path, node
        for key, value in node.items():
            if key == "flattened_default_effects" and "segments" in node:
                continue
            yield from walk(value, f"{path}.{key}")
    elif isinstance(node, list):
        for i, value in enumerate(node):
            yield from walk(value, f"{path}[{i}]")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    rows = []
    with sqlite3.connect(f"file:{args.run.parent / 'results.sqlite3'}?mode=ro", uri=True) as db:
        observed = db.execute("select count(*) from result where run_id=?", (args.run.name,)).fetchone()[0]
        query = "select card_name,result_json from result where run_id=? and status='compiled' and instr(result_json,'additional_restrictions')>0 order by card_name"
        for name, raw in db.execute(query, (args.run.name,)):
            result = json.loads(raw)
            for path, value in walk(result.get("definition")):
                authored = [r for r in value["additional_restrictions"] if isinstance(r, str) and not r.startswith("__")]
                if not authored:
                    continue
                conditional = any(any(token in text for token in ["only if ", "before ", "only during any ", "only during their "])
                                  for text in authored)
                absent = value.get("activation_condition") is None and not value.get("activation_restrictions")
                rows.append({"card": name, "path": path, "authored_restrictions": authored,
                             "activation_condition": value.get("activation_condition"),
                             "activation_restrictions": value.get("activation_restrictions"),
                             "timing": value.get("timing"),
                             "status": "conditional_text_without_typed_gate_candidate" if conditional and absent else "encoding_unverified",
                             "artifact_checksum": result.get("artifact_checksum")})
    report = {
        "scope": "Complete compiled-wire textual activation-gate inventory. Typed timing, functional zones, and recognized textual fallbacks can enforce restrictions without activation_condition. Presence of a typed field does not prove the full condition; absence does not confirm a defect. Runtime boundary tests are required.",
        "observed_payloads": observed, "rows": rows,
        "provenance": {"run": str(args.run.resolve()), "manifest": json.loads((args.run / "manifest.json").read_text()),
                       "scanner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
    }
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"rows": len(rows), "conditional_absence_candidates": sum(r["status"] == "conditional_text_without_typed_gate_candidate" for r in rows)}))


if __name__ == "__main__":
    main()
