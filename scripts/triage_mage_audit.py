#!/usr/bin/env python3
"""Group imported MAGE failures for review without promoting or clearing cards."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re


def review_family(classification, error, operation=None):
    operation = operation or {}
    error = error.removeprefix("`")
    if error.startswith("unsupported Java test body:"):
        return "untranslated_java_test_body"
    if error.startswith(("unsupported Java statement:", "unknown generated operation:")):
        return "untranslated_java_operation"
    if error.startswith("unsupported decision kind"):
        return "decision_adapter_candidate"
    if error.startswith(("unsupported library fixture:", "unsupported color assertion", "unsupported numeric",
                         "unsupported attacking assertion", "unsupported ability assertion",
                         "unsupported player counter assertion", "unsupported blitz assertion")):
        return "fixture_or_assertion_adapter_candidate"
    if error.startswith(("expected assertion checkpoint", "could not advance to turn", "could not inspect player",
                         "could not advance to priority", "stack did not settle", "one stack object did not resolve")):
        return "turn_priority_or_stack_scheduling_candidate"
    if error.startswith("unknown card name:"):
        return "fixture_or_catalog_lookup_candidate"
    if error.startswith("Missing CHOICE def"):
        return "missing_scripted_decision_candidate"
    if error.startswith("invalid priority action index:"):
        return "priority_action_selector_candidate"
    if error.startswith(("could not find cast action", "could not find activated ability",
                         "could not find playable ability", "could not find play land action",
                         "could not find mana ability", "expected playable ability", "expected payable action")):
        return "action_legality_or_label_candidate"
    if error.startswith("permanent not found:"):
        return "missing_result_or_fixture_candidate"
    if any(s in error for s in ["target decision has no legal targets", "select_objects decision has no legal candidates",
                                "targets do not satisfy", "no legal mana payment plan"]):
        return "decision_or_legality_candidate"
    if error.startswith("invalid blocker") or "Invalid blockers:" in error:
        return "combat_setup_or_legality_candidate"
    if re.search(r"recursive use of an object|memory access out of bounds|unreachable|Maximum call stack", error, re.I):
        return "wasm_execution_trap_candidate"
    if classification == "outcome_or_fixture_mismatch_candidate":
        op = operation.get("op", "")
        if op in {"assertPermanentCount", "assertGraveyardCount", "assertHandCount", "assertExileCount",
                  "assertLibraryCount", "assertLibraryOrder"}:
            return "zone_or_controller_outcome_candidate"
        if op in {"assertAbility", "assertAbilities", "assertSubtype", "assertPowerToughness"}:
            return "calculated_characteristics_candidate"
        if op == "assertCounterCount":
            return "counter_outcome_candidate"
        if op == "assertLife":
            return "life_or_damage_outcome_candidate"
    if classification != "unclassified_failure":
        return classification
    if any(s in error for s in ["missing damage amount (clause:", "triggered CST reached lowering",
                                "in attach object clause without prior tagged object",
                                "spell text effects references PlayerFilter::", "unsupported ",
                                "pending effect metric requires a prior memory-producing effect",
                                "card selection has no resolved source zone or referenced collection"]):
        return "compiler_diagnostic_or_runtime_lowering_candidate"
    return "unclassified_failure"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cumulative", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    raw = args.cumulative.read_bytes()
    source = json.loads(raw)
    rows = []
    grouped = defaultdict(list)
    for scenario in source["scenarios"]:
        if scenario["status"] != "failed":
            continue
        latest = scenario["observations"][-1]
        error = latest["row"].get("error") or ""
        classification = scenario["classification"]
        operation = latest["row"].get("operation")
        family = review_family(classification, error, operation)
        # Keep exact errors per observation; this normalization groups only
        # diagnostic templates, not cards or reviewed correctness verdicts.
        template = re.sub(r"\b\d+\b", "<number>", error.splitlines()[0].strip("`") if error else "")
        row = {"scenario_id": scenario["scenario_id"], "test": scenario["test"], "file": scenario["file"],
               "original_classification": classification, "review_family": family, "status": "unreviewed_candidate",
               "error": error, "operation": operation, "source_report": latest["report"],
               "source_report_sha256": latest["report_sha256"]}
        rows.append(row)
        grouped[(family, template)].append(scenario["scenario_id"])
    groups = [{"family": family, "diagnostic_template": template, "scenario_count": len(ids), "scenario_ids": ids}
              for (family, template), ids in grouped.items()]
    groups.sort(key=lambda row: (-row["scenario_count"], row["family"], row["diagnostic_template"]))
    report = {
        "scope": "Downstream review queue for failed imported scenarios. Categories never prove a fixture defect, compiler defect, or card defect. For example, no legal target may indicate either a bad fixture or an engine legality bug; a missing permanent may be a failed effect. All candidates remain unresolved until attribution is reviewed.",
        "confirmed_cards_added": 0, "cards_cleared": 0,
        "source_inventory_scenarios": source["inventory_scenarios"], "source_status_counts": source["status_counts"],
        "review_family_counts": dict(Counter(row["review_family"] for row in rows)),
        "groups": groups, "rows": rows,
        "provenance": {"cumulative": str(args.cumulative.resolve()), "cumulative_sha256": hashlib.sha256(raw).hexdigest(),
                       "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
    }
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"failed_scenario_candidates": len(rows), "groups": len(groups), "review_family_counts": report["review_family_counts"]}))


if __name__ == "__main__":
    main()
