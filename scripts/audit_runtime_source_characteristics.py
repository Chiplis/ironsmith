#!/usr/bin/env python3
"""Screen source-P/T reads on printed noncreatures; each path still needs source-binding review."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def references(node, path="definition"):
    if isinstance(node, str) and node in {"SourcePower", "SourceToughness"}:
        yield {"path": path, "value": node}
    elif isinstance(node, list):
        for i, value in enumerate(node):
            yield from references(value, f"{path}[{i}]")
    elif isinstance(node, dict):
        for key, value in node.items():
            if key == "flattened_default_effects" and "segments" in node:
                continue
            yield from references(value, f"{path}.{key}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads((args.run / "manifest.json").read_text())
    names = {row["name"]: row for row in json.loads((args.run / "inventory.json").read_text())["cards"]}
    rows = []
    with sqlite3.connect(f"file:{args.run.parent / 'results.sqlite3'}?mode=ro", uri=True) as db:
        observed = db.execute("select count(*) from result where run_id=?", (args.run.name,)).fetchone()[0]
        query = "select card_name,result_json from result where run_id=? and status='compiled' and (instr(result_json, '\"SourcePower\"')>0 or instr(result_json, '\"SourceToughness\"')>0) order by card_name"
        for name, raw in db.execute(query, (args.run.name,)):
            result = json.loads(raw)
            definition = result.get("definition", {})
            card = definition.get("card", {})
            if "Creature" in card.get("card_types", []) or card.get("power_toughness") is not None:
                continue
            paths = list(references(definition))
            if paths:
                rows.append({"card": name, "status": "candidate", "card_types": card.get("card_types"),
                             "oracle_text": names[name]["oracle_text"], "paths": paths,
                             "artifact_checksum": result.get("artifact_checksum")})
    report = {
        "scope": "Printed noncreatures without P/T whose structured definition reads source P/T. Granted abilities, temporary animation, and source rebinding can make a path legal; no defect is confirmed by this scan.",
        "observed_payloads": observed, "candidates": rows,
        "provenance": {"run": str(args.run.resolve()), "manifest": manifest,
                       "scanner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
    }
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"candidate_cards": len(rows), "observed_payloads": observed, "names": [r["card"] for r in rows]}))


if __name__ == "__main__":
    main()
