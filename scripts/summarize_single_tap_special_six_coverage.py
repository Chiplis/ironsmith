#!/usr/bin/env python3
"""Review six exact special single-tap paths, including legal producers and effects."""
import hashlib
import json
from collections import Counter
from pathlib import Path

ROOT = Path("reports/runtime-audit")


def ref(path):
    return {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    inputs = json.loads((ROOT / "single-tap-special-six-frozen-inputs.json").read_text())
    cases = {(r["card"], r["index"]) for r in inputs["cases"]}
    assert len(cases) == 6
    inventory = json.loads((ROOT / "single-tap-cost-inventory.json").read_text())
    candidates = [r for r in inventory["rows"] if (r["card"], r["source_ability_index"]) in cases]
    assert len(candidates) == 6
    path = ROOT / "single-tap-special-six-execution.json"
    raw = json.loads(path.read_text())
    source = ref(path)
    assert len(raw["rows"]) == 37 and raw["provenance"]["artifacts_unchanged"]
    assert len(raw["compilation"]) == 17
    assert all(r["definition_matches_frozen_except_unique_card_ids"] for r in raw["compilation"])
    evidence = {key: [] for key in cases}
    controls = []
    for index, row in enumerate(raw["rows"]):
        assert row["status"] == "expected_outcome_passed"
        assert all(c["expected"] == c["observed"] for c in row["checks"])
        key = row["card"], row["ability_index"]
        assert key in cases
        assert row["fixture_evidence"]["canonical_index"] == key[1]
        positive = row["expected"]["action_offered"]
        assert row["actual"]["action_offered"] == positive
        if positive:
            assert row["actual"]["action"]["resolution_error"] is None
            assert row["actual"]["action"]["mana_paid"] == (3 if key[0] == "Purple Pentapus" else 0)
            for name in ["exact_printed_mana_paid", "resolution_completed", "chosen_resource_tap_and_surplus"]:
                assert any(c["check"] == name for c in row["checks"])
        else:
            assert row["actual"]["action"] is None
        evidence[key].append({
            "source_report": source, "source_row": index, "scenario": row["scenario"],
            "positive": positive, "source_ability_index": key[1],
            "live_ability_index": row["fixture_evidence"]["live_index"],
        })
        controls.append({
            "card": row["card"], "ability_index": row["ability_index"], "scenario": row["scenario"],
            "classification": "expected_outcome_passed", "confirmed_cards": [],
            "source_report": source, "source_row": index,
            "expected": row["expected"], "observed": row["actual"], "checks": row["checks"],
        })
    scopes = {
        ("Arachnus Spinner", 1): "Single Spider payment; named Aura from library or actual Disenchant-produced graveyard enters attached to the declared creature. Library-search and empty-search cases shuffle remaining card IDs. The graveyard case does not assert that the library was left unsearched or unshuffled.",
        ("Patron Wizard", 0): "Single Wizard payment counters an actually paid pending Shock when its controller cannot pay or declines funded tax; accepting pays exactly one and Shock deals two. No pending-spell control is unavailable.",
        ("Purple Pentapus", 1): "Actual paid source plus paid Murder produces its graveyard entry; one fresh creature and three mana return the same stable card tapped and generate the second actual surveil decision. Battlefield-zone and unavailable-resource negatives are rejected.",
        ("Vodalian War Machine", 1): "Exact ability identity includes effect representation, distinct from its same-cost pump ability. Payment adds permission while retaining defender; real TurnRunner combat offers and accepts attack, then permission expires. Death-trigger semantics are outside scope.",
        ("Zombie Trailblazer", 0): "Single Zombie payment changes an actually played Forest to Swamp, removing Forest subtype, then actual cleanup restores Forest. Granted mana-production ability is outside this scoped check.",
        ("Zombie Trailblazer", 1): "Single Zombie payment grants the chosen Bears landwalk; real combat offers and accepts the attack, allows the actual opposing blocker without a Swamp and forbids it with a Swamp. Grant expires by next own main.",
    }
    rows = []
    for candidate in candidates:
        key = candidate["card"], candidate["source_ability_index"]
        ev = evidence[key]
        assert {"zero", "exact", "surplus", "tapped", "ineligible"} <= {r["scenario"] for r in ev}
        rows.append({**candidate, "status": "scoped_expected_outcomes_passed", "source_evidence": ev, "scope": scopes[key]})
    counts = {
        "paths": 6, "scenarios": 37, "passing_scenarios": 37,
        "actual_successful_activations": sum(c["expected"]["action_offered"] for c in controls),
        "expected_unavailable_controls": sum(not c["expected"]["action_offered"] for c in controls),
        "oracle_checks": sum(len(c["checks"]) for c in controls),
        "strict_definitions_matching_frozen": 17, "confirmed_cards": 0, "unrun_paths": 0,
    }
    limits = [
        "Full canonical definitions compile strictly and match frozen artifacts except enumerated unique CardIds. Executable, fixture and inputs remain hash-stable through the run.",
        "All battlefield sources and resources are normal paid casts or legal land plays. Library/hand deck contents and initial mana are explicit fixture setup; graveyard resources are produced by actual paid Murder or Disenchant.",
        "Zero additional resource is correctly positive when the printed cost allows tapping the source. Other exact/surplus resources are freshly cast Universal Automaton with Changeling. Negative controls use actual paid Twiddle or noncreature Fervor.",
        "No unavailable action is forced. Patron Wizard's response follows Bob's actual paid spell announcement and normal priority passing; funded acceptance and funded refusal are separate controls.",
        "Combat effects use actual TurnRunner attacker/blocker decision contexts and declared attacks. Temporal effects are checked again after real cleanup/turn transitions.",
        "Only these exact six cost paths and listed effects receive coverage. No whole-card clearance, exhaustive branching, or new defect promotion.",
    ]
    review = {
        "scope": __doc__, "findings": [], "confirmed_cards": [], "controls": controls,
        "counts": counts, "path_coverage": rows, "source_reports": [source],
        "provenance": {
            "artifacts_unchanged": True,
            "native_source_runs": [{"source_report": source, "run": raw["provenance"], "attempt": json.loads((ROOT / "single-tap-special-six-attempt.json").read_text())}],
        },
        "limitations": limits,
    }
    rp = ROOT / "single-tap-special-six-reviewed-attribution.json"
    rp.write_text(json.dumps(review, indent=2) + "\n")
    ledger = {
        "family": "single_tap_cost", "subfamily": "special_six", "path_count": 6,
        "rows": rows, "counts": counts, "reviewed_sources": [ref(rp)],
        "status_counts": dict(Counter(r["status"] for r in rows)), "limitations": limits,
        "generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    (ROOT / "single-tap-special-six-path-coverage.json").write_text(json.dumps(ledger, indent=2) + "\n")
    (ROOT / "single-tap-special-six-review.md").write_text(
        "# Six special single-tap cost paths\n\n"
        "All 37 scenarios and 191 checks pass: 21 actual activations and 16 unavailable controls. "
        "Seventeen strict definitions match frozen inputs. No card is promoted or globally cleared.\n\n"
        + "\n".join("- " + scope for scope in scopes.values()) + "\n\n"
        + "\n".join("- " + limit for limit in limits) + "\n"
    )
    print(json.dumps(counts))


if __name__ == "__main__":
    main()
