#!/usr/bin/env python3
"""Find TargetMatches references in authored static-condition roots.

This is a candidate screen, not a card verdict. Triggered/activated effects and
conditions nested inside granted abilities are deliberately outside its scope.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3

BASE = Path(__file__).resolve().parents[1]
ROOT = BASE / "reports/runtime-audit"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def target_paths(value, path):
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "TargetMatches":
                yield path + ".TargetMatches"
            else:
                yield from target_paths(child, path + "." + key)
    elif isinstance(value, list):
        for i, child in enumerate(value):
            yield from target_paths(child, path + f"[{i}]")


def static_roots(model, path):
    payload = model.get("payload")
    if not isinstance(payload, dict):
        return
    for variant, body in payload.items():
        if not isinstance(body, dict):
            continue
        node_path = path + ".payload." + variant
        # Only the static's own condition is included. A filter, granted
        # ability, or nested effect can have a different evaluation contract.
        if body.get("condition") is not None:
            yield node_path + ".condition", body["condition"], variant
        if variant == "Conditional" and isinstance(body.get("ability"), dict):
            yield from static_roots(body["ability"], node_path + ".ability")


def screen(definition):
    findings = []
    for i, ability in enumerate(definition.get("abilities", [])):
        model = ability.get("kind", {}).get("Static")
        if not isinstance(model, dict):
            continue
        for path, condition, variant in static_roots(model, f"abilities[{i}].kind.Static"):
            refs = list(target_paths(condition, path))
            if refs:
                findings.append({"static_index": i, "static_id": model.get("id"),
                    "functional_zones": ability.get("functional_zones"),
                    "payload_variant": variant, "condition_path": path,
                    "condition": condition, "target_reference_paths": refs,
                    "classification": "static_target_context_candidate", "execution_confirmed": False})
    return findings


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, default=ROOT / "corpus/results.sqlite3")
    parser.add_argument("--run-id", default="267a16aff3b321196397d0b4")
    parser.add_argument("--output", type=Path, default=ROOT / "static-target-condition-candidates.json")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        root = {"condition": {"Not": {"TargetMatches": {"attacking": True}}}}
        row = {"abilities": [{"kind": {"Static": {"id": "Anthem", "payload": {"Anthem": root}}}}]}
        assert len(screen(row)) == 1
        assert screen({"abilities": [{"kind": {"Triggered": root}}]}) == []
        assert screen({"abilities": [{"kind": {"Static": {"payload": {"GrantObjectAbilityForFilter": {"ability": row["abilities"][0]}}}}}]}) == []
        wrapper = {"abilities": [{"kind": {"Static": {"payload": {"Conditional": {"ability": row["abilities"][0]["kind"]["Static"], "condition": "Always"}}}}}]}
        assert len(screen(wrapper)) == 1
        assert "Conditional.ability" in screen(wrapper)[0]["condition_path"]
        print("Static-condition scope checks passed.")
        return
    findings = []
    with sqlite3.connect(args.database.resolve().as_uri() + "?mode=ro", uri=True) as connection:
        connection.execute("PRAGMA query_only=ON")
        status_counts = dict(connection.execute("SELECT status,count(*) FROM result WHERE run_id=? GROUP BY status", (args.run_id,)))
        candidates = connection.execute("SELECT card_name,result_json FROM result WHERE run_id=? AND instr(result_json, '\"TargetMatches\"') > 0", (args.run_id,))
        prefiltered = 0
        for name, raw in candidates:
            prefiltered += 1
            result = json.loads(raw)
            definition = result.get("definition")
            if not isinstance(definition, dict):
                continue
            for finding in screen(definition):
                findings.append({"card": name, "artifact_checksum": result.get("artifact_checksum"),
                    "result_json_sha256": hashlib.sha256(raw.encode()).hexdigest(), **finding})
    files = ["crates/ironsmith-engine/src/static_abilities/continuous.rs", "crates/ironsmith-engine/src/condition_eval.rs"]
    out = {"generated_at": datetime.now(timezone.utc).isoformat(),
        "scope": "All frozen records are searched for the exact serialized TargetMatches variant; matching definitions are decoded and only authored static-condition roots inspected. No condition in a granted ability, filter or resolution effect is treated as the parent's condition.",
        "records_screened": sum(status_counts.values()), "status_counts": status_counts,
        "text_prefilter_records": prefiltered, "candidate_paths": len(findings),
        "candidate_names": len({f['card'] for f in findings}), "rows": findings,
        "confirmed_cards_by_this_screen": [], "all_cards_verified": False,
        "provenance": {"database": str(args.database), "run_id": args.run_id,
            "generator_sha256": sha(Path(__file__)),
            "current_source_references": [{"path": f, "sha256": sha(BASE / f)} for f in files]},
        "limits": ["The screen cannot inspect definitions that did not compile or were not retained.",
            "TargetMatches in a static external context needs separate binding and gameplay review. The screen does not infer the resulting Boolean value or promote any card.",
            "The reproduced Arcades case motivated this screen; other payload classes can have different runtime contracts.",
            "Nested granted static conditions, effect conditions and other context-reference variants are outside this narrow screen."]}
    args.output.write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: out[k] for k in ("records_screened", "text_prefilter_records", "candidate_paths", "candidate_names")}, indent=2))


if __name__ == "__main__":
    main()
