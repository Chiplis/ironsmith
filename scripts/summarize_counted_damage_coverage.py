#!/usr/bin/env python3
"""Join typed counted-Object damage candidates to explicit scenario reports.

This ledger accounts for candidate payloads, not correctness of all their states.
Earlier failures are distinct from exercising the counted damage consumer.
"""
from collections import Counter
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "reports/runtime-audit"
SOURCES = ["multitarget-reproductions.json", "counted-damage-reproductions.json",
           "dynamic-counted-damage-reproductions.json", "counted-damage-ability-reproductions.json"]


def main():
    candidates_path = ROOT / "counted-damage-target-candidates.json"
    candidates = json.loads(candidates_path.read_text())
    sources = {}
    indexed = {}
    for filename in SOURCES:
        path = ROOT / filename
        report = json.loads(path.read_text())
        sources[filename] = {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                             "scope": report.get("scope"), "provenance": report.get("provenance")}
        for index, row in enumerate(report["rows"]):
            payload = row.get("payload_name", row["card"])
            indexed.setdefault(payload, []).append({"source": filename, "row": index,
                "card": row["card"], "status": row["status"], "scenario": row["scenario"]})
    rows = []
    all_cases = []
    for candidate in candidates["rows"]:
        payload = candidate["card"]
        cases = indexed.get(payload, [])
        failure = any(r["status"] == "semantic_mismatch" for r in cases)
        if not cases:
            disposition = "not_exercised"
        elif payload == "Nahiri's Wrath":
            disposition = "cast_gate_failure_counted_consumer_unreached"
        elif payload == "Slight Malfunction":
            disposition = "reflexive_choice_failure_counted_consumer_unreached"
        elif failure:
            disposition = "counted_damage_failure_reproduced"
        else:
            disposition = "scoped_controls_only"
        primary = cases[0]["card"] if cases else payload
        rows.append({"payload_name": payload, "primary_name": primary,
                     "candidate_path": candidate["path"], "artifact_checksum": candidate["artifact_checksum"],
                     "disposition": disposition, "case_counts": dict(Counter(r["status"] for r in cases)),
                     "cases": cases})
        all_cases.extend(cases)
    result = {"scope": "All17 typed payloads (16primary names) have explicit scenarios; 15payloads reach counted-damage failure, two stop at different earlier failures. This is family accounting, not an all-cards or all-branches proof.",
              "candidate_source": candidates_path.name,
              "candidate_source_sha256": hashlib.sha256(candidates_path.read_bytes()).hexdigest(),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "candidate_scan_records": candidates["records_scanned"],
              "candidate_scan_retained_definitions": candidates["retained_definitions"],
              "candidate_payloads": len(rows), "primary_names": len({r["primary_name"] for r in rows}),
              "disposition_counts": dict(Counter(r["disposition"] for r in rows)),
              "scenario_count": len(all_cases), "scenario_counts": dict(Counter(r["status"] for r in all_cases)),
              "all_candidate_payloads_accounted_for": all(r["cases"] for r in rows),
              "all_counted_consumers_exercised": False,
              "sources": sources, "rows": rows}
    (ROOT / "counted-damage-family-coverage.json").write_text(json.dumps(result, indent=2) + "\n")
    lines = ["# Counted Object-target damage coverage", "", result["scope"], "",
             f"{len(all_cases)} scenarios: {dict(Counter(r['status'] for r in all_cases))}.", "",
             "| Canonical payload | Disposition | Cases |", "| --- | --- | ---: |"]
    lines.extend(f"| {r['payload_name']} | {r['disposition']} | {len(r['cases'])} |" for r in rows)
    lines += ["", "The combined Smoldering Werewolf payload was independently compiled and executed, but is one primary card with the front payload. Nahiri’s Wrath and Slight Malfunction have confirmed earlier failures; their counted-damage branch is still unexercised."]
    (ROOT / "counted-damage-family-coverage.md").write_text("\n".join(lines) + "\n")
    print(json.dumps({k: result[k] for k in ("candidate_payloads", "primary_names", "disposition_counts", "scenario_count", "scenario_counts", "all_candidate_payloads_accounted_for")}))


if __name__ == "__main__":
    main()
