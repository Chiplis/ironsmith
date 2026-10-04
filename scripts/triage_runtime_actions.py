#!/usr/bin/env python3
"""Inventory action/choice failures, nested panics and budget-only observations."""
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1] / "reports/runtime-audit"
RUN = "e17a4980b0b92c7a5a4cead2"


def family(status, detail):
    # The trailing fixture description contains generic words such as targets;
    # those must not determine the classification of the actual error.
    detail = detail.split(";", 1)[0]
    if status == "panicked":
        if "TotalCost::costs" in detail:
            return "alternative_cost_access_panic"
        if "unsupported continuous-effect value" in detail:
            return "layer_value_context_panic"
        return "other_panic"
    if status == "budget_exceeded":
        return "decision_budget" if "decision budget" in detail else "priority_or_resolution_step_budget"
    if any(text in detail for text in ("announced distribution", "counter distribution", "cannot divide")):
        return "distribution_choice_not_satisfied"
    if "Ability index no longer valid" in detail:
        return "ability_index_changed"
    if "no legal mana payment plan" in detail:
        return "mana_plan_unavailable"
    if "targets" in detail:
        return "target_choice_not_satisfied"
    if any(text in detail.lower() for text in ("not enough counters", "not enough objects", "failed to pay deferred")):
        return "deferred_cost_resources_unavailable"
    if "X value not set" in detail:
        return "unbound_x_at_announcement"
    return "other_action_or_choice_failure"


def main():
    proof_path = ROOT / "confirmed-outcomes.jsonl"
    panic_proof = {}
    if proof_path.exists():
        for line in proof_path.read_text().splitlines():
            record = json.loads(line)
            if record.get("category") != "runtime_exception":
                continue
            row = record["row"]
            actual = json.dumps(row.get("actual"), ensure_ascii=False)
            if "TotalCost::costs called for an alternative cost" in actual:
                panic_proof.setdefault(row["card"], []).append({
                    "source": record["source"], "row": record["source_row"],
                    "scope": row.get("scope"),
                })
    front_review_path = ROOT / "nested-panic-reviewed-outcomes.json"
    front_review = json.loads(front_review_path.read_text()) if front_review_path.exists() else {}
    front_blocked = {}
    for index, row in enumerate(front_review.get("rows", [])):
        if row.get("status") == "compile_failed" and row.get("attempted_back_face"):
            front_blocked.setdefault(row["attempted_back_face"], []).append({
                "source": front_review_path.name, "row": index, "front_card": row["card"],
                "error": row.get("actual"),
            })
    database = sqlite3.connect(f"file:{ROOT}/actions/results.sqlite3?mode=ro", uri=True)
    database.execute("BEGIN")
    rows = []
    query = """select card_name,json_extract(result_json,'$.execution') from result
               where run_id=? and (result_json like '%action_or_choice_failed%'
               or result_json like '%budget_exceeded%' or result_json like '%panicked%')"""
    for name, raw in database.execute(query, (RUN,)):
        for index, observation in enumerate(json.loads(raw or "[]")):
            if observation["status"] in {"action_or_choice_failed", "budget_exceeded", "panicked"}:
                rows.append({"card": name, "source_observation_index": index,
                    "family": family(observation["status"], observation.get("detail", "")),
                    "review_status": "unreviewed_fixture_limited", "observation": observation})
    database.close()
    counts = Counter(row["observation"]["status"] for row in rows)
    groups = {}
    for row in rows:
        if row["family"] == "alternative_cost_access_panic" and row["card"] in panic_proof:
            row["review_status"] = "independently_reproduced_same_failure_family"
            row["independent_evidence"] = panic_proof[row["card"]]
        elif row["family"] == "layer_value_context_panic" and row["card"] in front_blocked:
            row["review_status"] = "canonical_front_transition_compile_blocked"
            row["reachability_review"] = front_blocked[row["card"]]
        group = groups.setdefault(row["family"], {"cards": set(), "observations": 0})
        group["cards"].add(row["card"])
        group["observations"] += 1
    for group in groups.values():
        group["cards"] = sorted(group["cards"])
    report = {"scope": "Every action/choice failure, nested panic, and budget observation in the complete corrected optimized legal-action campaign, including budget-only cards omitted from its candidate-file selection. This inventory promotes no observation.",
        "generated_at": datetime.now(timezone.utc).isoformat(), "run_id": RUN,
        "status_counts": dict(counts), "families": groups, "rows": rows,
        "review_status_counts": dict(Counter(row["review_status"] for row in rows)),
        "provenance": {"generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "independent_outcomes_sha256": hashlib.sha256(proof_path.read_bytes()).hexdigest() if proof_path.exists() else None,
            "front_transition_review_sha256": hashlib.sha256(front_review_path.read_bytes()).hexdigest() if front_review_path.exists() else None,
            "worker_manifest": json.loads((ROOT / "actions" / RUN / "manifest.json").read_text())},
        "limitations": [
            "Deferred-cost resource failures and unsatisfied distribution/target decisions may be fixture or policy limitations; inspect legality discovery and richer state before a card verdict.",
            "Budget exhaustion may arise from always-accept decisions or game-phase transitions; it does not prove an engine infinite loop.",
            "Nested panics need genuine-state reachability review. Back-face power based on crafted cards requires a producer-aware crafting fixture."]}
    (ROOT / "actions-priority-ledger.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"statuses": dict(counts), "families": {k: {"names": len(v["cards"]), "observations": v["observations"]} for k, v in groups.items()}}))


if __name__ == "__main__":
    main()
