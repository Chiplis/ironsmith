#!/usr/bin/env python3
"""Prioritize unexercised index candidates without clearing or promoting them."""
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "reports/runtime-audit"


def main():
    coverage_path = ROOT / "ability-index-family-coverage.json"
    review_path = ROOT / "ability-index-static-gate-review.json"
    inputs = {path: path.read_bytes() for path in (coverage_path, review_path)}
    coverage = json.loads(inputs[coverage_path])
    activity = json.loads(inputs[review_path])
    by_card = defaultdict(list)
    for row in activity["rows"]:
        by_card[row["card"]].append(row)
    rows = []
    gate_or_unknown = {
        "direct_static_activity_gate_declared",
        "wrapped_native_static_activity_gate_declared",
        "requires_payload_or_wrapper_review",
    }
    for row in coverage["rows"]:
        if row["classification"] != "unexercised_by_this_family":
            continue
        gates = by_card[row["card"]]
        if not gates:
            raise ValueError(f"Missing source review for {row['card']}")
        kinds = sorted({gate["classification"] for gate in gates})
        priority = (
            "requires_native_activity_gate_or_wrapper_review"
            if gate_or_unknown.intersection(kinds)
            else "lower_index_suspicion_from_current_source_only"
        )
        rows.append({
            "card": row["card"], "priority": priority,
            "activity_classifications": kinds,
            "static_ids": sorted({gate["static_id"] for gate in gates}),
            "source_review_paths": [{key: gate[key] for key in (
                "static_index", "static_id", "classification", "condition_paths",
                "source_review", "wrapper_review",
            )} for gate in gates],
            "runtime_exercised_by_this_family": False, "card_cleared": False,
        })
    output = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "scope": "Priority-only join of unexercised conditional-index candidates to exact current-source activity-gate review. A lower priority is not an execution pass or proof of frozen binary behavior.",
        "counts": dict(Counter(row["priority"] for row in rows)),
        "rows": rows, "all_cards_correct": False,
        "provenance": {
            "sources": [{"path": str(path), "sha256": hashlib.sha256(data).hexdigest()}
                        for path, data in inputs.items()],
            "generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        },
        "limitations": [
            "No candidate is removed from the original 262-name structural ledger.",
            "A declared activity gate still needs a legal state in which its condition is false. Timing or resource restrictions can prevent an inactive state from exposing an index error.",
            "Source classification and fresh-binary controls do not establish historical runtime equivalence.",
        ],
    }
    path = ROOT / "ability-index-remaining-priority.json"
    path.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps(output["counts"]))


if __name__ == "__main__":
    main()
