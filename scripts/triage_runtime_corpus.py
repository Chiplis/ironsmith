#!/usr/bin/env python3
"""Group completed runtime-corpus candidates without converting smoke tests into proof.

This derived ledger is intentionally conservative. Reviewed error families can
be associated with independent canonical reproductions, but a confirmed defect
in one ability never clears the other findings on the same card.
"""
import argparse
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sqlite3

from summarize_runtime_audit import CONFIRMED_FAILURES, RUNTIME_FAILURES

ROOT = Path(__file__).resolve().parents[1] / "reports/runtime-audit"
DEFAULT_RUN = "267a16aff3b321196397d0b4"
ACTION_RUN = "e17a4980b0b92c7a5a4cead2"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def runtime_family(text):
    patterns = (
        ("generic each-player action lacks simultaneous proposal support", "unsupported_simultaneous_action"),
        ("requires an explicit search zone", "missing_choice_zone"),
        ("Effect EffectId(", "missing_effect_outcome"),
        ("Ability index no longer valid", "ability_index_mismatch"),
        ("Selected action is not an activated ability", "ability_index_mismatch"),
        ("announced distribution must assign exactly", "invalid_announced_distribution"),
        ("IteratedPlayer not set", "unbound_iterated_player"),
        ("X value not set", "unbound_x"),
        ("dynamic X mana cost has no X value", "unbound_x"),
        ("Opponent filter requires a targeted player", "unbound_targeted_opponent"),
        ("TaggedPlayer requires a tagged player", "unbound_tagged_player"),
        ("AttackingPlayer not set", "unbound_attacking_player"),
        ("triggering event missing object", "unbound_event_object"),
        ("requires a triggering event", "unbound_trigger_event"),
        ("ChosenPlayer requires a previously chosen player", "unbound_chosen_player"),
        ("delayed prevention metric requires a prior prevention shield", "missing_prevention_producer"),
    )
    for fragment, family in patterns:
        if fragment in text:
            return family
    if re.search(r"Tag '.+' not found", text):
        return "missing_object_tag"
    return "other_runtime_exception"


def contract_family(finding):
    if finding["code"] == "unbound_context":
        message = finding["message"]
        for word, family in (("IteratedPlayer", "unbound_iterated_player"),
                             ("EventValue", "unbound_trigger_event")):
            if word in message:
                return family
    return finding["code"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--campaign", choices=("corpus", "actions"), default="corpus")
    parser.add_argument("--run-id")
    args = parser.parse_args()
    root, campaign = args.root, args.campaign
    run = args.run_id or (DEFAULT_RUN if campaign == "corpus" else ACTION_RUN)
    source = root / campaign / run / "findings.jsonl"
    candidates = [json.loads(line) for line in source.read_text().splitlines()]
    summary = json.loads((root / "summary.json").read_text())
    forms_path = root / "semantic-candidate-forms.json"
    forms = json.loads(forms_path.read_text()) if forms_path.exists() else {}
    form_classification = {(r["card"], r["oracle_ability"], json.dumps(r["check"], sort_keys=True)): r["classification"]
                           for r in forms.get("rows", [])}
    fixture_review_path = root / "optional-resource-context-triage.json"
    fixture_review = json.loads(fixture_review_path.read_text()) if fixture_review_path.exists() else {}
    fixture_notes = {r["card"]: r for r in fixture_review.get("rows", [])}
    reviewed_controls = defaultdict(list)
    reviewed_control_sources = []
    for filename in ("processor-reviewed-classification.json", "counter-outcome-reviewed-classification.json",
                     "tap-gift-outcome-reviewed-attribution.json", "cohort-reviewed-classification.json",
                     "damage-distribution-reviewed-classification.json", "damage-distribution-additional-reviewed-classification.json"):
        path = root / filename
        if not path.exists():
            continue
        data = json.loads(path.read_text())
        reviewed_control_sources.append({"path": filename, "sha256": sha(path)})
        for index, review in enumerate(data.get("candidate_reviews", data.get("rows", []))):
            name = review.get("card_name", review.get("card"))
            if name:
                reviewed_controls[name].append({"source": filename, "review_index": index,
                    "review": review, "scope": data.get("scope"),
                    "limitation": "Scoped control evidence does not clear unrelated abilities, branches or whole cards."})
    confirmed_names = set(summary["confirmed_card_outcomes"]["confirmed_failure_cards"])
    proof = defaultdict(list)
    outcome_path = root / "confirmed-outcomes.jsonl"
    for line in outcome_path.read_text().splitlines():
        record = json.loads(line)
        if record["category"] != "runtime_exception":
            continue
        row = record["row"]
        # Only observed errors may associate a family; expected values and
        # explanatory prose cannot serve as a substitute for the actual trace.
        actual = row.get("actual", row.get("observed"))
        family = runtime_family(json.dumps(actual, ensure_ascii=False))
        if family != "other_runtime_exception":
            proof[(row["card"], family)].append({"source": record["source"], "row": record["source_row"]})
    loadable = set(summary["confirmed_loadability_failures"]["confirmed_failure_cards"])
    action_db = root / "actions/results.sqlite3"
    action = sqlite3.connect(f"file:{action_db}?mode=ro", uri=True)
    action.execute("BEGIN")
    rows = []
    families = defaultdict(lambda: {"cards": set(), "observations": 0, "triage_counts": Counter(), "kinds": Counter()})
    card_statuses = Counter()
    for result in candidates:
        name = result["name"]
        findings = []
        for index, contract in enumerate(result.get("contracts", [])):
            if contract["severity"] == "error":
                findings.append({"kind": "necessary_contract", "family": contract_family(contract),
                                 "source_path": f"contracts[{index}]", "observation": contract})
        for index, observation in enumerate(result.get("execution", [])):
            if observation["status"] in RUNTIME_FAILURES:
                family = "scenario_panic" if observation["status"] == "panicked" else runtime_family(observation.get("detail", ""))
                findings.append({"kind": "legal_action_execution" if campaign == "actions" else "synthetic_execution", "family": family,
                                 "source_path": f"execution[{index}]", "observation": observation})
        for index, candidate in enumerate(result.get("semantic_candidates", [])):
            for check in candidate["checks"]:
                findings.append({"kind": "semantic_text_candidate", "family": check["check"],
                                 "source_path": f"semantic_candidates[{index}]", "observation": candidate,
                                 "text_screen_priority": form_classification.get((name, candidate["oracle_ability"], json.dumps(check, sort_keys=True)), "not_classified")})
        if result["status"] not in ("compiled", "compile_failed"):
            findings.append({"kind": "compilation_or_materialization", "family": result["status"],
                             "source_path": "status", "observation": {"status": result["status"], "error": result.get("error")}})
        for flag in ("parse_lossy", "has_unimplemented"):
            if result.get(flag):
                findings.append({"kind": "compilation_coverage", "family": flag,
                                 "source_path": flag, "observation": result.get("parse_loss") if flag == "parse_lossy" else True})
        raw_action = action.execute("SELECT result_json FROM result WHERE run_id=? AND card_name=?", (ACTION_RUN, name)).fetchone() if campaign != "actions" else None
        action_result = result if campaign == "actions" else json.loads(raw_action[0]) if raw_action else None
        legal_failures = [{"index": i, **o} for i, o in enumerate((action_result or {}).get("execution", []))
                          if o["status"] in RUNTIME_FAILURES]
        for finding in findings:
            family = finding["family"]
            references = proof.get((name, family), [])
            if references:
                status = "reviewed_family_reproduced"
            elif family == "materialization_failed" and name in loadable:
                status = "reviewed_loadability_failure"
                references = [{"source": "summary.json", "section": "confirmed_loadability_failures"}]
            elif finding["kind"] in {"synthetic_execution", "legal_action_execution"}:
                status = "fixture_limited"
            else:
                status = "unreviewed"
            finding.update(triage_status=status, related_reviewed_evidence=references,
                           this_specific_occurrence_verified=False,
                           legal_action_exception_observations=[o["index"] for o in legal_failures
                                                               if runtime_family(o.get("detail", "")) == family])
            if family == "invalid_announced_distribution" and "casting_method: Normal" in finding.get("observation", {}).get("detail", ""):
                distribution_reviews = [item for item in reviewed_controls.get(name, [])
                    if item["source"].startswith("damage-distribution-")]
                if distribution_reviews:
                    finding["reviewed_fixture_limitation"] = {
                        "classification": "missing_valid_distribution_in_original_responder",
                        "evidence": distribution_reviews,
                        "scope": "The frozen validator rejected the original incomplete distribution. Separate fresh-binary normal-cost probes supplied explicit valid splits and observed exact outcomes; original observations remain visible.",
                        "limitation": "Full-definition parity does not prove runtime binary equivalence. This review does not clear alternative costs, other branches, or whole cards."}
            group = families[family]
            group["cards"].add(name)
            group["observations"] += 1
            group["triage_counts"][status] += 1
            group["kinds"][finding["kind"]] += 1
        statuses = {f["triage_status"] for f in findings}
        reviewed = {s for s in statuses if s.startswith("reviewed_")}
        card_status = ("all_families_have_reviewed_matches" if statuses and statuses == reviewed else
                       "partially_reviewed" if reviewed else
                       "fixture_limited" if statuses == {"fixture_limited"} else "unreviewed")
        card_statuses[card_status] += 1
        rows.append({"card": name, "status": result["status"], "artifact_checksum": result.get("artifact_checksum"),
                     "triage_status": card_status, "has_independent_confirmed_card_outcome": name in confirmed_names,
                     "related_fixture_review": {"source": fixture_review_path.name, "row": fixture_notes[name],
                                                "limitation": fixture_review.get("limitation")} if name in fixture_notes else None,
                     "reviewed_control_evidence": reviewed_controls.get(name, []),
                     "findings": findings, "legal_action_coverage": {"run_id": ACTION_RUN,
                         "recorded": action_result is not None, "status": (action_result or {}).get("status"),
                         "execution_status_counts": dict(Counter(o["status"] for o in (action_result or {}).get("execution", []))),
                         "exception_observations": legal_failures}})
    action.close()
    table = [{"family": name, "card_count": len(value["cards"]), "cards": sorted(value["cards"]),
              **{k: dict(v) if isinstance(v, Counter) else v for k, v in value.items() if k != "cards"}}
             for name, value in sorted(families.items(), key=lambda item: (-len(item[1]["cards"]), item[0]))]
    report = {"generated_at": datetime.now(timezone.utc).isoformat(), "run_id": run, "campaign": campaign,
              "candidate_card_count": len(rows), "card_triage_counts": dict(card_statuses), "families": table,
              "semantic_text_priority": {"source": forms_path.name if forms else None,
                                         "counts": forms.get("counts", {}), "scope": forms.get("scope")},
              "all_cards_correct": False, "scope": f"Every flagged row in the completed {campaign} campaign, grouped by necessary contract, observed exception, semantic candidate, or noncompletion cause.",
              "provenance": {"findings_path": str(source), "findings_sha256": sha(source),
                  "summary_sha256": sha(root / "summary.json"), "confirmed_outcomes_sha256": sha(outcome_path),
                  "generator_sha256": sha(Path(__file__)), "action_run_snapshot": ACTION_RUN},
              "limitations": ["Reviewed-family association means a separately reviewed reproduction observed the same error family on the same exact card/face name; it does not prove that every flagged ability or branch failed.",
                  "A confirmed defect on a card never automatically reviews its other findings; exact face aliases are not automatically merged.",
                  "Synthetic event/direct-resolution failures remain fixture-limited until independently reproduced. Legal-action exceptions remain candidates unless explicitly reviewed.",
                  "Semantic text screens, including reminder text and line-alignment artifacts, remain unreviewed until typed-IR inspection and expected-result reproduction.",
                  "Coverage gaps, budget-only observations, and ordinary compile failures are outside the original candidate list and remain separately visible in the complete campaign summary.",
                  "A reviewed or unflagged entry is not a whole-card correctness verdict."]}
    report["provenance"]["reviewed_control_sources"] = reviewed_control_sources
    if forms:
        report["provenance"]["semantic_forms_sha256"] = sha(forms_path)
    if fixture_review:
        report["provenance"]["resource_fixture_review_sha256"] = sha(fixture_review_path)
    (root / f"{campaign}-triage-ledger.jsonl").write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows))
    (root / f"{campaign}-triage-summary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    lines = [f"# Completed {campaign} finding ledger", "", f"{len(rows)} flagged card/face rows; {len(table)} cause families. Every row remains traceable to the immutable campaign findings.", "",
             "Reviewed family matches, fixture-limited observations, and unreviewed findings remain distinct. Cards can have several families; table counts overlap. No whole-card correctness claim is made.", "",
             "| Family | Names | Observations | Reviewed family/load | Fixture limited | Unreviewed |",
             "|---|---:|---:|---:|---:|---:|"]
    for group in table:
        c = group["triage_counts"]
        lines.append(f"| {group['family']} | {group['card_count']} | {group['observations']} | {c.get('reviewed_family_reproduced',0)+c.get('reviewed_loadability_failure',0)} | {c.get('fixture_limited',0)} | {c.get('unreviewed',0)} |")
    lines += ["", f"[Full per-card ledger]({campaign}-triage-ledger.jsonl) · [Structured summary and provenance]({campaign}-triage-summary.json)", ""]
    if forms:
        lines += [f"The [downstream text screen](semantic-candidate-forms.json) labels {forms.get('counts')} markers. These are priority labels only: the original findings remain present and no keyword or card is cleared.", ""]
    (root / f"{campaign}-triage-ledger.md").write_text("\n".join(lines))
    print(json.dumps({"candidates": len(rows), "families": len(table), "card_triage_counts": dict(card_statuses)}))


if __name__ == "__main__":
    main()
