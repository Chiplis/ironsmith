#!/usr/bin/env python3
"""Review exact own-tap-symbol plus single chosen-resource cost paths."""
import hashlib
import json
from collections import Counter
from pathlib import Path

ROOT = Path("reports/runtime-audit")


def ref(path):
    return {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    inventory = json.loads((ROOT / "single-tap-cost-inventory.json").read_text())
    candidates = [r for r in inventory["rows"] if r["subgroup"] == "source_tap"]
    assert len(candidates) == 20
    assert len({(r["card"], r["path"], r["consumer_path"]) for r in candidates}) == 20
    cases = {r["card"]: r for r in json.loads((ROOT / "single-tap-source-frozen-inputs.json").read_text())["cases"]}
    path = ROOT / "single-tap-source-execution.json"
    raw = json.loads(path.read_text())
    source = ref(path)
    assert raw["provenance"]["artifacts_unchanged"]
    assert len(raw["rows"]) == 120
    assert all(c["definition_matches_frozen_except_unique_card_ids"] for c in raw["compilation"])
    controls, availability = [], []
    mapping = {r["card"]: [] for r in candidates}
    for index, row in enumerate(raw["rows"]):
        assert row["status"] != "fixture_error"
        assert row["fixture_evidence"]["canonical_index"] == cases[row["card"]]["index"]
        failures = [c for c in row["checks"] if c["expected"] != c["observed"]]
        ev = {
            "source_report": source,
            "source_row": index,
            "scenario": row["scenario"],
            "valid_activation_executed": row["expected"]["action_offered"] and row["actual"]["action"] is not None,
            "availability_mismatch": bool(failures),
            "scope": row["scope"],
        }
        mapping[row["card"]].append(ev)
        common = {
            "card": row["card"], "scenario": row["scenario"], "confirmed_cards": [],
            "source_report": source, "source_row": index,
            "expected": row["expected"], "observed": row["actual"],
        }
        if failures:
            assert len(failures) == 1 and failures[0]["check"] == "intended_activation_offered"
            assert row["scenario"] == "zero"
            assert not row["expected"]["action_offered"] and row["actual"]["action_offered"]
            assert "Not enough objects to choose (1 needed, 0 available)" in row["actual"]["action"]["announcement_error"]
            assert row["actual"]["before"] == row["actual"]["after"]
            availability.append({
                **common, "classification": "advertised_unpayable_action_rejected_with_rollback",
                "failed_checks": failures,
                "finding": "The sole source cannot pay both its tap symbol and the required untapped-object cost. Its advertised activation is rejected during payment; checked mana, source/resource taps, life, hand/library, counters, tokens and stack are restored. This is an interface consistency observation, not a reproduced failure of a legally payable ability.",
            })
        else:
            assert row["status"] == "expected_outcome_passed"
            controls.append({**common, "classification": "expected_outcome_passed", "checks": row["checks"], "scope": row["scope"]})
    rows = []
    for candidate in candidates:
        evidence = mapping[candidate["card"]]
        assert {e["scenario"] for e in evidence} == {"zero", "exact", "surplus", "fresh", "source_tapped", "all_tapped"}
        assert len(evidence) == 6
        assert candidate["source_ability_index"] == cases[candidate["card"]]["index"]
        mismatch = any(e["availability_mismatch"] for e in evidence)
        rows.append({
            **candidate,
            "status": "scoped_payment_passed_with_availability_mismatch" if mismatch else "scoped_expected_outcomes_passed",
            "source_evidence": evidence,
            "scope": "Exact source tap plus a distinct selected resource and printed mana. Listed effects checked except transformations without linked back-face metadata; negative-state advertised rejections are separate interface observations.",
        })
    counts = {
        "paths": len(rows), "scenarios": len(raw["rows"]), "passing_scenarios": len(controls),
        "valid_activations": sum(e["valid_activation_executed"] for evidence in mapping.values() for e in evidence),
        "negative_state_advertised_rejections": len(availability),
        "oracle_checks": sum(len(r["checks"]) for r in raw["rows"]),
        "strict_definitions_matching_frozen": len(raw["compilation"]), "confirmed_cards": 0, "unrun_paths": 0,
    }
    limits = [
        "All 33 strict full canonical definitions match the frozen corpus except explicitly enumerated unique CardIds. Inputs, fixture and executable hashes are unchanged within the run.",
        "Sources and resources enter through normal paid casts or legal land plays. Source aging and untapping use actual TurnRunner turns. Resource creatures remain fresh to test their legal use for chosen tap costs.",
        "Exact and surplus payments check one distinct selected resource, source tap, printed mana, and untapped surplus. Fresh sources and paid Twiddle source/all-tapped states provide separate controls.",
        "Effects independently check damage, life, draw/discard counts and selected card, chosen target tap or power/toughness, each recipient's counters, equipment selection with library-bottom placement, and actual token characteristics.",
        "Chosen of Markov and Town Gossipmonger primary/combined payloads compile independently. Their eight positive scenarios cover costs only: linked transform metadata is absent in frozen inputs, so transform correctness is not asserted.",
        "Thirteen zero-resource advertisements are rejected with full measured rollback. They are not legal payable scenarios, and no card-execution defect is promoted from them.",
        "These are scoped ability-path checks, not whole-card clearance or exhaustive choices/target/combat coverage.",
    ]
    review = {
        "scope": __doc__, "findings": [], "confirmed_cards": [], "controls": controls,
        "availability_observations": availability, "counts": counts, "path_coverage": rows,
        "source_reports": [source],
        "provenance": {"artifacts_unchanged": True, "native_source_runs": [{"source_report": source, "run": raw["provenance"], "attempt": json.loads((ROOT / "single-tap-source-attempt.json").read_text())}]},
        "limitations": limits,
    }
    reviewpath = ROOT / "single-tap-source-reviewed-attribution.json"
    reviewpath.write_text(json.dumps(review, indent=2) + "\n")
    ledger = {
        "family": "single_tap_cost", "subfamily": "source_tap", "path_count": len(rows),
        "rows": rows, "counts": counts, "reviewed_sources": [ref(reviewpath)],
        "status_counts": dict(Counter(r["status"] for r in rows)), "limitations": limits,
        "generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    (ROOT / "single-tap-source-path-coverage.json").write_text(json.dumps(ledger, indent=2) + "\n")
    (ROOT / "single-tap-source-review.md").write_text(
        "# Source tap plus chosen-resource cost audit\n\n"
        "All 20 paths have valid payment coverage. All 44 valid activations meet the checked cost/effect outcomes. "
        "Of 120 scenarios, 107 fully pass; 13 zero-resource advertisements reject during payment with measured rollback. "
        "No new card defect is promoted.\n\n" + "\n".join("- " + s for s in limits) + "\n"
    )
    print(json.dumps(counts))


if __name__ == "__main__":
    main()
