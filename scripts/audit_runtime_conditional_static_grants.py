#!/usr/bin/env python3
"""Enumerate conditional static grants with structured payloads, without claiming defects."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def grants(node, path="definition"):
    if isinstance(node, dict):
        grant = node.get("GrantObjectAbilityForFilter")
        if isinstance(grant, dict) and grant.get("condition") is not None:
            static = grant.get("ability", {}).get("kind", {}).get("Static")
            if static and static.get("payload") != "None":
                yield {"path": path, "condition": grant["condition"], "granted_static_id": static.get("id")}
        for key, child in node.items():
            if key == "flattened_default_effects" and "segments" in node:
                continue
            yield from grants(child, f"{path}.{key}")
    elif isinstance(node, list):
        for index, child in enumerate(node):
            yield from grants(child, f"{path}[{index}]")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    inventory = args.run / "inventory.json"
    canonical = {p["name"]: p for p in json.loads(inventory.read_text())["cards"]}
    candidates = []
    with sqlite3.connect(f"file:{args.run.parent / 'results.sqlite3'}?mode=ro", uri=True) as db:
        count = db.execute("select count(*) from result where run_id=?", (args.run.name,)).fetchone()[0]
        query = "select card_name,result_json from result where run_id=? and status='compiled' and instr(result_json,'GrantObjectAbilityForFilter')>0 order by card_name"
        for name, raw in db.execute(query, (args.run.name,)):
            result = json.loads(raw)
            for grant in grants(result.get("definition", {})):
                candidates.append({"card": name, "status": "structural_candidate", **grant,
                                   "oracle_text": canonical[name]["oracle_text"],
                                   "artifact_checksum": result.get("artifact_checksum")})
    exact = sorted({r["card"] for r in candidates if r["condition"] == "EnchantedPermanentIsEquipment"})
    report = {"scope": "Conditional grants of static abilities with nontrivial serialized payloads. A payload does not imply that generate_effects emits effects; Ward, protection and landwalk are included as conservative candidates. This scan is not a proof of recursion or gameplay correctness.",
              "observed_payloads": count, "candidate_paths": len(candidates), "candidate_names": len({r["card"] for r in candidates}),
              "equipment_condition_names": exact, "rows": candidates, "confirmed_added_by_scan": 0,
              "limitations": ["Only the GrantObjectAbilityForFilter wire shape is screened.", "A card absent from this screen can still have a recursion bug through other condition or ability forms.", "The five Rune Equipment-condition paths have separate paid-action reproductions; their attribution is not inferred here."],
              "provenance": {"run": str(args.run.resolve()), "manifest": json.loads((args.run / "manifest.json").read_text()),
                             "inventory_sha256": hashlib.sha256(inventory.read_bytes()).hexdigest(),
                             "scanner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}}
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ["observed_payloads", "candidate_paths", "candidate_names", "equipment_condition_names"]}))


if __name__ == "__main__":
    main()
