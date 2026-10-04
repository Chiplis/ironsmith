#!/usr/bin/env python3
"""Join the authored-gate screen to explicitly selected reviewed scenario reports."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("reports/runtime-audit"))
    parser.add_argument("--report", action="append", required=True,
                        help="Reviewed report filename; do not select initial or invalidated drafts")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    screen_path = args.root / "activation-restriction-candidates.json"
    candidates = [r for r in json.loads(screen_path.read_text())["rows"]
                  if r["status"] == "conditional_text_without_typed_gate_candidate"]
    aliases = {r["inventory_name"]: r for r in json.loads(
        (args.root / "activation-front-alias-review.json").read_text())["rows"]}
    observations = defaultdict(list)
    sources = []
    for filename in dict.fromkeys(args.report):
        if "initial" in filename or "invalid" in filename:
            raise ValueError(f"Refusing draft/invalid report: {filename}")
        path = args.root / filename
        raw = path.read_bytes()
        sources.append({"path": filename, "sha256": hashlib.sha256(raw).hexdigest()})
        for index, row in enumerate(json.loads(raw).get("rows", [])):
            observations[row["card"]].append({"report": filename, "row_index": index,
                                               "status": row["status"], "scenario": row.get("scenario")})
    rows = []
    for candidate in candidates:
        name = candidate["card"]
        alias = aliases.get(name)
        lookup = alias["compiled_front_name"] if alias and alias["canonical_compile_input_identical"] else name
        matched = observations[lookup]
        good = [r for r in matched if r["status"] in {"expected_result_observed", "semantic_mismatch", "resolution_failed"}]
        status = "scoped_scenarios_observed" if good else "fixture_limited" if matched else "not_exercised"
        rows.append({"card": name, "candidate_path": candidate["path"],
                     "authored_restrictions": candidate["authored_restrictions"],
                     "coverage_status": status, "input_alias_of": lookup if lookup != name else None,
                     "direct_scenario_count": len(good) if lookup == name else 0,
                     "observation_references": matched})
    report = {
        "scope": f"Scoped coverage of {len(candidates)} conditional-text candidates. Rows associate reviewed scenarios with the flagged card/ability; they do not certify every state, target, permission, or branch. Alias mappings cover identical front compilation input only, never linked transitions. This is not an additional confirmed-card list.",
        "all_cards_verified": False, "all_candidate_branches_verified": False,
        "coverage_counts": dict(Counter(r["coverage_status"] for r in rows)),
        "directly_sampled_inputs": sum(r["direct_scenario_count"] > 0 for r in rows),
        "identical_front_input_aliases": sum(r["input_alias_of"] is not None for r in rows),
        "rows": rows, "sources": sources,
        "provenance": {"screen_sha256": hashlib.sha256(screen_path.read_bytes()).hexdigest(),
                       "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
    }
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: report[k] for k in ["coverage_counts", "directly_sampled_inputs", "identical_front_input_aliases"]}))


if __name__ == "__main__":
    main()
