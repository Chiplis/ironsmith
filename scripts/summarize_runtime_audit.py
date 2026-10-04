#!/usr/bin/env python3
"""Rebuild the runtime audit index without modifying any source evidence.

Only named expected-result reproductions and explicitly reviewed canonical
integration failures establish confirmed card outcomes.
Generic fixture observations, contract failures, unit tests and MAGE ports keep
their own evidence classes. Per-run validity.json always controls runtime use.
"""

import argparse
from collections import Counter
from contextlib import ExitStack
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3
import tempfile


RUNTIME_FAILURES = {
    "resolution_failed", "direct_resolution_failed", "announcement_failed",
    "action_or_choice_failed", "panicked", "invariant_failed",
}
CONFIRMED_FAILURES = {"silent_wrong_result", "runtime_exception", "runtime_nontermination", "reviewed_card_failure"}
EXPECTED_REPORTS = ("semantic-execution.json", "engine-card-reproductions.json",
                    "runtime-value-reproductions.json", "ninjutsu-family.json",
                    "runtime-search-value-reproductions.json", "count-semantic-reproductions.json",
                    "delayed-sacrifice-family.json", "threshold-cast-reproductions.json",
                    "glissa-seifer-reproductions.json", "foundry-value-reproductions.json",
                    "rich-legal-candidate-reproductions.json", "delayed-return-reproductions.json",
                    "deep-cavern-bat-reproduction.json", "x-and-resource-trigger-reproductions.json",
                    "simultaneous-action-execution.json", "gift-family-reproductions.json",
                    "optional-reference-context-reproductions.json", "residual-threshold-reproductions.json",
                    "multiplayer-controller-context-reproductions.json", "reference-value-reproductions.json",
                    "optional-etb-owner-context-reproductions.json", "phase-outcome-execution.json",
                    "trigger-context-reproductions.json", "optional-resource-context-reproductions.json",
                    "misc-value-context-reproductions.json", "remaining-outcome-execution.json",
                    "target-cost-optional-context-reproductions.json", "followup-context-reproductions.json",
                    "path-vote-omen-context-reproductions.json", "sacrifice-cost-execution.json",
                    "numeric-predicate-reproductions.json", "nested-panic-reviewed-outcomes.json",
                    "cast-counter-execution.json", "new-way-forward-damage-reproductions.json",
                    "attachment-outcome-reproductions.json", "remaining-target-zone-delayed-reproductions.json",
                    "activation-gate-reproductions.json", "activation-history-gate-reproductions.json",
                    "remaining-state-gate-reproductions.json", "temporal-activation-reproductions.json",
                    "special-zone-gate-reproductions.json", "combat-activation-gate-reproductions.json",
                    "remaining-planechase-reproductions.json", "numeric-color-gate-reproductions.json",
                    "any-player-gate-reproductions.json", "stack-history-gate-reproductions.json",
                    "remaining-attack-history-gate-reproductions.json", "astral-drift-execution-reproductions.json",
                    "sacrifice-life-value-reproductions.json", "modal-candidate-reproductions.json",
                    "simultaneous-trigger-family-reproductions.json", "trigger-choice-zone-reproductions.json",
                    "trigger-player-context-reproductions.json", "attacking-player-context-reproductions.json",
                    "foretell-candidate-reproductions.json", "simultaneous-activation-family-reproductions.json",
                    "repeated-mana-alternative-reproductions.json",
                    "simultaneous-remaining-family-reproductions.json", "aura-reflexive-context-reproductions.json", "second-trigger-choice-reproductions.json", "backdraft-reproductions.json",
                    "optional-tag-candidate-reproductions.json", "ability-index-reproductions.json", "mana-x-announcement-reproductions.json",
                    "ability-index-followup-reproductions.json", "mana-counter-sibling-reproductions.json",
                    "ability-index-restriction-reproductions.json", "gluntch-choice-reviewed-attribution.json",
                    "ability-index-rule-reproductions.json", "ability-damage-distribution-reproductions.json",
                    "ability-index-anthem-reproductions.json", "damage-source-lki-reproductions.json",
                    "ability-index-mana-wrapper-reproductions.json", "counted-damage-ability-reproductions.json",
                    "damage-source-event-reproductions.json", "ability-index-equipment-grant-reproductions.json",
                    "ability-index-devotion-reproductions.json", "damage-source-etb-batch-reproductions.json",
                    "ability-index-active-grant-reproductions.json", "ability-index-pair-reproductions.json",
                    "damage-source-noncreature-reproductions.json", "ability-index-hand-reproductions.json",
                    "damage-source-aura-reproductions.json", "damage-source-death-reproductions.json", "damage-source-combat-reproductions.json", "flitterwing-delayed-draw-reproductions.json", "ability-index-turn-grant-reproductions.json", "self-sacrifice-effect-cost-reproductions.json", "ability-index-state-grant-reproductions.json", "ability-index-cost-transition-reproductions.json", "ability-index-type-equipment-reproductions.json", "ability-index-counter-timing-reproductions.json", "ability-index-earned-grant-reproductions.json", "ability-index-land-mill-reproductions.json", "ability-index-myojin-reproductions.json", "fixed-tap-cost-reproductions.json", "fixed-tap-sibling-reproductions.json", "ability-index-turn-equipment-reproductions.json", "ability-index-gourmand-reproductions.json", "ability-index-turn-loyalty-reproductions.json")
REVIEWED_REPORTS = ("native-integration-triage.json", "legal-action-triage.json",
                    "legal-action-triage-batch2.json", "static-flagged-legal-triage.json",
                    "simultaneous-legal-action-triage.json", "draw-replacement-reviewed-attribution.json",
                    "mana-restriction-reviewed-attribution.json", "simultaneous-legal-action-triage-batch2.json",
                    "noncompletion-reviewed-outcomes.json", "x-context-reviewed-attribution.json",
                    "noncast-x-reviewed-attribution.json", "mage-followup-reviewed-attribution.json",
                    "rune-reviewed-attribution.json", "spell-trigger-x-reviewed-attribution.json", "player-choice-reviewed-attribution.json", "tap-gift-outcome-reviewed-attribution.json", "reconfigure-reviewed-attribution.json",
                    "mantle-attachment-reviewed-attribution.json", "equipment-cast-restriction-reviewed-attribution.json", "multitarget-reviewed-classification.json", "counted-damage-reviewed-attribution.json",
                    "mage-outcome-followups-reviewed-attribution.json",
                    "dynamic-counted-damage-reviewed-attribution.json", "dynamic-discard-cost-reviewed-attribution.json",
                    "multitarget-optional-reviewed-attribution.json", "multitarget-split-reviewed-attribution.json",
                    "flashback-discard-cost-reviewed-attribution.json", "target-tag-overwrite-reviewed-attribution.json",
                    "ability-index-arcades-reviewed-attribution.json", "negative-tag-choice-reviewed-attribution.json", "static-recipient-condition-reviewed-attribution.json", "counter-cost-final-reviewed-classification.json", "static-tagged-attachment-reviewed-attribution.json", "static-combat-predicate-reviewed-attribution.json", "martyr-reveal-cost-reviewed-attribution.json", "sacrifice-cost-sibling-reviewed-classification.json", "nonmana-x-sibling-reviewed-attribution.json", "rest-tag-reviewed-attribution.json", "rest-library-reviewed-attribution.json", "rest-event-reviewed-attribution.json", "nonmana-x-control-reviewed-classification.json", "nonmana-x-spell-reviewed-attribution.json", "multi-return-cost-reviewed-classification.json", "graveyard-selection-cost-reviewed-attribution.json", "devourer-pregame-reviewed-classification.json", "exile-cost-family-reviewed-attribution.json", "ability-index-station-reviewed-attribution.json", "ability-index-attachment-switch-reviewed-attribution.json", "fixed-exile-graveyard-reviewed-attribution.json", "fixed-exile-remaining-reviewed-attribution.json", "station-threshold-static-reviewed-attribution.json", "station-threshold-event-reviewed-attribution.json", "single-exile-simple-reviewed-attribution.json", "single-exile-craft-reviewed-attribution.json", "unattach-cost-reviewed-attribution.json", "single-exile-special-reviewed-attribution.json", "ability-index-zone-stack-reviewed-attribution.json", "single-exile-stack-hand-reviewed-attribution.json", "mixed-tap-cost-reviewed-classification.json", "alternative-tap-cost-reviewed-attribution.json", "tap-outcome-reviewed-classification.json", "ability-index-counter-level-reviewed-attribution.json", "single-tap-station-reviewed-attribution.json", "ability-index-turn-equipment-reviewed-classification.json", "special-tap-cost-reviewed-classification.json", "shimmer-tap-reviewed-classification.json", "single-exile-battlefield-reviewed-attribution.json", "ability-index-gourmand-reviewed-classification.json", "eladamri-tap-reviewed-classification.json", "single-exile-remaining-reviewed-attribution.json", "single-tap-effect-reviewed-attribution.json", "ability-index-turn-loyalty-reviewed-classification.json", "dermotaxi-tap-reviewed-classification.json", "tap-copy-reviewed-classification.json", "single-tap-mana-reviewed-attribution.json", "weight-tap-reviewed-classification.json", "linked-face-cost-reviewed-attribution.json", "meria-tap-exile-reviewed-attribution.json", "combat-tap-reviewed-classification.json", "prepare-entry-reviewed-attribution.json", "ability-index-speed-discard-reviewed-attribution.json", "single-tap-source-reviewed-attribution.json", "single-land-tap-reviewed-classification.json", "ability-index-linked-transition-reviewed-classification.json", "single-special-seven-reviewed-classification.json", "ability-index-alias-front-reviewed-classification.json", "single-tap-special-six-reviewed-attribution.json", "prepare-trigger-reviewed-attribution.json", "spell-return-land-reviewed-classification.json", "spell-web-return-reviewed-classification.json", "ability-index-island-attack-reviewed-classification.json", "return-morph-kicker-reviewed-classification.json", "extended-escape-reviewed-attribution.json", "extended-alternate-exile-reviewed-attribution.json", "additional-withid-simple-reviewed-attribution.json", "spell-alt-tap-reviewed-classification.json", "ward-waterbend-reviewed-classification.json", "spell-tap-flashback-reviewed-classification.json", "optional-tap-outcome-reviewed-classification.json", "extended-optional-exile-reviewed-attribution.json", "extended-trigger-cost-reviewed-attribution.json", "conditional-land-entry-reviewed-classification.json", "additional-withid-special-reviewed-attribution.json")
REVIEWED_CLASSES = {"runtime_defect_card_reproduced", "compiler_semantic_defect_card_reproduced"}
READ_DESCRIPTORS = {}


def utcnow():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def read_json(path, warnings, default=None):
    if not path.exists():
        return default
    try:
        contents = path.read_bytes()
        parsed = json.loads(contents)
        READ_DESCRIPTORS[path.resolve()] = {"sha256": hashlib.sha256(contents).hexdigest(),
            "size": len(contents), "mtime_ns": path.stat().st_mtime_ns}
        return parsed
    except (OSError, json.JSONDecodeError) as error:
        warnings.append({"source": str(path), "classification": "unknown", "error": str(error)})
        return default


def relative(path, root):
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def descriptor(path, root):
    if path.resolve() in READ_DESCRIPTORS:
        return {"path": relative(path, root), **READ_DESCRIPTORS[path.resolve()]}
    stat = path.stat()
    return {"path": relative(path, root), "sha256": digest(path),
            "size": stat.st_size, "mtime_ns": stat.st_mtime_ns}


def runtime_validity(value, malformed=False):
    """Unknown explicit policies fail closed; no file means no known revocation."""
    if malformed:
        return {"eligible": False, "status": "unreadable_validity_policy"}
    if value is None:
        return {"eligible": True, "status": "not_explicitly_revoked",
                "note": "This permits candidate observations, not correctness claims."}
    status = str(value.get("status", "unknown")).lower()
    invalid = any(word in status for word in (
        "superseded", "collision", "invalid", "quarantin", "require_reproduction", "revoked",
    ))
    explicitly_valid = value.get("runtime_valid") is True or status in {
        "valid", "validated", "runtime_valid", "valid_for_runtime_candidates",
    }
    eligible = explicitly_valid and not invalid and value.get("runtime_valid") is not False
    return {"eligible": eligible, "status": status, "policy": value}


def validity_snapshot(root):
    return {str(path): digest(path) for path in root.rglob("*validity.json")}


def expected_category(row):
    status = row.get("status")
    if status == "reviewed_nontermination":
        return "runtime_nontermination"
    if status == "runtime_nontermination" and row.get("outcome_category") == "runtime_nontermination":
        # This path is used only by explicitly whitelisted reviewed outcome
        # reports. Generic worker timeout/budget rows are never passed here.
        return "runtime_nontermination"
    if status == "confirmed_resolution_failure":
        actual = row.get("actual")
        return "runtime_exception" if isinstance(actual, dict) and actual.get("resolution_error") else "unknown"
    if status == "resolution_completed":
        return "expected_result_observed" if row.get("actual") == row.get("expected") and "expected" in row else "unknown"
    if status == "reviewed_card_failure":
        return status
    if status == "semantic_mismatch":
        actual = row.get("actual")
        if isinstance(actual, dict) and actual.get("resolution_error") is not None:
            return "runtime_exception"
        return "silent_wrong_result"
    if status in RUNTIME_FAILURES:
        return "runtime_exception"
    if status in {"passed", "expected_result_observed"}:
        return "expected_result_observed"
    if status in {"fixture_invalid", "invalid_fixture", "fixture_error"}:
        return "fixture_invalid"
    if status == "compile_failed":
        return "compile_failure"
    if status == "materialization_failed":
        return "materialization_failure"
    return "unknown"


def write_row(stream, value):
    stream.write(json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n")


def summarize_expected(root, warnings, output):
    counts, cards, sources = Counter(), {}, []
    for filename in (*EXPECTED_REPORTS, *REVIEWED_REPORTS):
        path = root / filename
        report = read_json(path, warnings)
        if not isinstance(report, dict):
            continue
        provenance = report.get("provenance", {})
        if not isinstance(provenance, dict):
            warnings.append({"source": str(path), "classification": "unknown",
                             "error": "Unsupported provenance shape; expected an object. Report not promoted."})
            continue
        policy_path = path.with_name(path.stem + ".validity.json")
        policy = read_json(policy_path, warnings)
        validity = runtime_validity(policy, policy_path.exists() and policy is None)
        # Reviews derived from campaign rows inherit their run's revocations.
        # A later source review cannot rehabilitate an invalid fixture run.
        dependencies = []
        source_runs = provenance.get("source_runs", [])
        if not isinstance(source_runs, list) or any(
                not isinstance(run, dict) or not isinstance(run.get("campaign"), str)
                or not isinstance(run.get("run_id"), str) for run in source_runs):
            warnings.append({"source": str(path), "classification": "unknown",
                             "error": "Malformed campaign source_runs metadata; report not promoted."})
            continue
        for run in source_runs:
            run_policy_path = root / run["campaign"] / run["run_id"] / "validity.json"
            run_policy = read_json(run_policy_path, warnings)
            dependency = runtime_validity(run_policy, run_policy_path.exists() and run_policy is None)
            dependencies.append({**run, **dependency})
            if not dependency["eligible"]:
                validity = {**validity, "eligible": False, "status": "source_run_runtime_ineligible"}
        if dependencies:
            validity["source_runs"] = dependencies
        if provenance.get("artifacts_unchanged") is False or provenance.get("unchanged") is False:
            validity = {**validity, "eligible": False, "status": "artifacts_changed_during_run"}
        valid = validity["eligible"]
        source = descriptor(path, root)
        source_rows = report.get("rows", report.get("findings", []))
        source.update({"provenance": provenance, "scope": report.get("scope"),
                       "validity": validity, "rows": len(source_rows)})
        sources.append(source)
        report_rows = []
        status_reviews = {}
        if filename == "gift-family-reproductions.json":
            status_path = root / "gift-family-review.json"
            status_review = read_json(status_path, warnings, {})
            status_reviews = {row.get("card"): row for row in status_review.get("unconfirmed", [])}
            if status_review:
                source["stage_classification_review"] = descriptor(status_path, root)
        for index, row in enumerate(source_rows):
            if row.get("interaction_cards"):
                # Explicit multi-card attribution has its own evidence class.
                continue
            if filename in REVIEWED_REPORTS:
                if row.get("classification") not in REVIEWED_CLASSES:
                    continue
                # Promotion is explicit per row, never inferred from a failing
                # assertion, a candidate mapping, or the report's title.
                for name in row.get("confirmed_cards", []):
                    subtype = row.get("outcome_category")
                    status = {"silent_wrong_result": "semantic_mismatch",
                              "runtime_exception": "resolution_failed",
                              "runtime_nontermination": "reviewed_nontermination"}.get(subtype, "reviewed_card_failure")
                    report_rows.append((index, {**row, "card": name,
                        "status": status, "actual": row.get("observed"),
                        "scope": row.get("fixture_review"),
                        "reviewed_classification": row["classification"]}))
            else:
                reviewed_status = status_reviews.get(row.get("card"))
                if reviewed_status and row.get("status") == reviewed_status.get("raw_status") and row.get("actual") == reviewed_status.get("actual"):
                    row = {**row, "raw_status": row["status"], "status": reviewed_status["status"],
                           "stage_classification_review": reviewed_status.get("review")}
                report_rows.append((index, row))
        if filename in REVIEWED_REPORTS:
            explicit_names = set(report.get("confirmed_cards", []))
            mapped_names = {row.get("card") for _, row in report_rows}
            if explicit_names - mapped_names:
                warnings.append({"source": filename, "classification": "review_schema_requires_attention",
                    "unmapped_explicit_confirmed_cards": sorted(explicit_names - mapped_names),
                    "error": "Top-level confirmed names have no eligible explicit per-row review; no inference or promotion was made."})
        for index, row in report_rows:
            category = expected_category(row) if valid else "fixture_invalid"
            # An expected-result designation still needs both reported values.
            if category in CONFIRMED_FAILURES | {"expected_result_observed"}:
                if row.get("expected") is None or row.get("actual") is None or not row.get("card"):
                    category = "unknown"
            counts[category] += 1
            record = {"source": filename, "source_row": index, "category": category, "row": row}
            write_row(output, record)
            name = row.get("card", "(unnamed)")
            card = cards.setdefault(name, {"card": name, "observations": Counter(), "evidence": []})
            card["observations"][category] += 1
            card["evidence"].append({"source": filename, "row": index,
                                      "category": category, "scope": row.get("scope")})
    failed_cards = sorted(name for name, card in cards.items()
                          if any(card["observations"][key] for key in CONFIRMED_FAILURES))
    return {"sources": sources, "observation_counts": dict(counts),
            "confirmed_failure_cards": failed_cards, "confirmed_failure_card_count": len(failed_cards),
            "cards": [dict(card, observations=dict(card["observations"]))
                      for _, card in sorted(cards.items())],
            "raw": "confirmed-outcomes.jsonl",
            "limitation": "Confirmation applies to the recorded scenarios and artifact versions only."}


def summarize_campaigns(root, warnings, findings, quarantined):
    runs = []
    face_export_path = root / "canonical-face-payloads.json"
    face_export = read_json(face_export_path, warnings, {})
    explicit_faces = {row["name"] for row in face_export.get("cards", [])}
    face_corpus_hash = face_export.get("cards_sha256")
    for database in sorted(root.glob("*/results.sqlite3")):
        connection = None
        try:
            connection = sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True, timeout=10)
            connection.execute("pragma query_only=ON")
            connection.execute("begin")
            for run_id, raw_metadata, created in connection.execute(
                    "select run_id,metadata,created_at from run order by created_at,run_id"):
                run_dir = database.parent / run_id
                metadata = json.loads(raw_metadata)
                policy_path = run_dir / "validity.json"
                policy = read_json(policy_path, warnings)
                validity = runtime_validity(policy, policy_path.exists() and policy is None)
                inventory = read_json(run_dir / "inventory.json", warnings, {})
                expected = inventory.get("payload_count")
                exclusions = inventory.get("exclusions", [])
                face_coverage = {}
                if explicit_faces and face_corpus_hash == metadata.get("cards_sha256"):
                    missing_faces = explicit_faces - {row["name"] for row in inventory.get("cards", [])}
                    face_coverage = {"explicit_face_payloads_expected": len(explicit_faces),
                                     "explicit_face_payloads_missing_count": len(missing_faces),
                                     "explicit_faces_fully_inventoried": not missing_faces,
                                     "face_coverage_source": descriptor(face_export_path, root)}
                del inventory
                source = {"database": relative(database, root), "run_id": run_id}
                run = {**source, "campaign": database.parent.name, "created_at": created,
                       **face_coverage,
                       "manifest": relative(run_dir / "manifest.json", root), "provenance": metadata,
                       "runtime_validity": validity, "canonical_payloads": expected,
                       "excluded_source_names": len(exclusions), "recorded": 0,
                       "worker_status_counts": Counter(), "contract_counts": Counter(),
                       "execution_status_counts": Counter(), "quarantined_execution_status_counts": Counter(),
                       "semantic_candidate_checks": Counter(), "runtime_candidate_cards": set(),
                       "structural_candidate_cards": set(), "parse_lossy_cards": 0,
                       "unimplemented_cards": 0, "last_recorded_at": None}
                for name, status, raw, recorded_at in connection.execute(
                        "select card_name,status,result_json,recorded_at from result where run_id=? order by card_name",
                        (run_id,)):
                    row = json.loads(raw)
                    run["recorded"] += 1
                    run["last_recorded_at"] = max(run["last_recorded_at"] or recorded_at, recorded_at)
                    run["worker_status_counts"][status] += 1
                    base = {**source, "card": name, "recorded_at": recorded_at}
                    if status != "compiled":
                        category = {"compile_failed": "compile_failure",
                                    "materialization_failed": "materialization_failure"}.get(status, "unknown")
                        write_row(findings, {**base, "category": category, "stage": "worker",
                                            "observation": {k: v for k, v in row.items() if k != "definition"},
                                            "runtime_validity": validity})
                    for contract in row.get("contracts", []):
                        run["contract_counts"][contract.get("severity", "unknown") + ":" + contract.get("code", "unknown")] += 1
                        if contract.get("severity") == "error":
                            run["structural_candidate_cards"].add(name)
                            write_row(findings, {**base, "category": "necessary_contract_violation",
                                                "stage": "static", "observation": contract})
                    for observation in row.get("execution", []):
                        observation_status = observation.get("status", "unknown")
                        if not validity["eligible"]:
                            run["quarantined_execution_status_counts"][observation_status] += 1
                            write_row(quarantined, {**base, "category": "fixture_invalid",
                                                   "validity": validity, "observation": observation})
                        else:
                            run["execution_status_counts"][observation_status] += 1
                            if observation_status in RUNTIME_FAILURES:
                                run["runtime_candidate_cards"].add(name)
                                write_row(findings, {**base, "category": "runtime_exception_candidate",
                                                    "stage": "execution", "observation": observation})
                            elif observation_status not in {"executed", "executed_game_ended", "action_discovery"}:
                                category = "fixture_invalid" if observation_status == "fixture_invalid" else "coverage_gap"
                                write_row(findings, {**base, "category": category,
                                                    "stage": "execution", "observation": observation})
                    run["parse_lossy_cards"] += bool(row.get("parse_lossy"))
                    run["unimplemented_cards"] += bool(row.get("has_unimplemented"))
                    if row.get("parse_lossy") or row.get("has_unimplemented"):
                        write_row(findings, {**base, "category": "semantic_candidate", "stage": "compilation",
                                            "parse_loss": row.get("parse_loss"),
                                            "has_unimplemented": row.get("has_unimplemented", False)})
                    for candidate in row.get("semantic_candidates", []):
                        for check in candidate.get("checks", []):
                            run["semantic_candidate_checks"][check.get("check", "unknown")] += 1
                        write_row(findings, {**base, "category": "semantic_candidate",
                                            "stage": "text_or_structure", "observation": candidate})
                for key, value in list(run.items()):
                    if isinstance(value, Counter):
                        run[key] = dict(value)
                    elif isinstance(value, set):
                        run[key] = sorted(value)
                run["remaining"] = max(0, expected - run["recorded"]) if isinstance(expected, int) else None
                run["inventory_fully_recorded"] = expected == run["recorded"] if expected is not None else False
                stopped_path = run_dir / "stopped.json"
                stopped = read_json(stopped_path, warnings)
                if stopped is not None:
                    run["stopped"] = stopped
                    run["stopped_source"] = descriptor(stopped_path, root)
                run["recording_status"] = ("inventory_recorded" if run["inventory_fully_recorded"] else
                                           "stopped_partial" if stopped is not None else "partial_or_active")
                run["all_cards_correct"] = False
                runs.append(run)
            connection.rollback()
        except (OSError, sqlite3.Error, ValueError, KeyError) as error:
            warnings.append({"source": str(database), "classification": "unknown", "error": str(error)})
        finally:
            if connection is not None:
                connection.close()
    # Include created/pending run directories even before inventory or DB registration completes.
    recorded = {(run["campaign"], run["run_id"]) for run in runs}
    for manifest in sorted(root.glob("*/*/manifest.json")):
        campaign, run_id = manifest.parent.parent.name, manifest.parent.name
        if (campaign, run_id) in recorded:
            continue
        policy_path = manifest.parent / "validity.json"
        policy = read_json(policy_path, warnings)
        runs.append({"campaign": campaign, "run_id": run_id, "recorded": 0,
                     "status": "no_database_results_yet", "provenance": read_json(manifest, warnings, {}),
                     "runtime_validity": runtime_validity(policy, policy_path.exists() and policy is None),
                     "manifest": relative(manifest, root), "all_cards_correct": False})
    return {"runs": runs, "raw_findings": "consolidated-findings.jsonl",
            "quarantined_observations": "quarantined-runtime-observations.jsonl",
            "limitation": "Counts are per run; repeated cards and fixture observations are not unique gameplay proofs."}


def summarize_supplemental_legal(root, warnings, findings, quarantined):
    path = root / "legal-reproductions-corrected.json"
    data = read_json(path, warnings)
    if not data:
        return {"status": "not_available"}
    policy_path = path.with_name(path.stem + ".validity.json")
    policy = read_json(policy_path, warnings)
    validity = runtime_validity(policy, policy_path.exists() and policy is None)
    counts, candidate_cards = Counter(), set()
    for row_index, row in enumerate(data.get("rows", [])):
        name = row.get("name", row.get("card", "(unnamed)"))
        for observation in row.get("execution", []):
            status = observation.get("status", "unknown")
            counts[status] += 1
            record = {"source": path.name, "source_row": row_index, "card": name,
                      "artifact_checksum": row.get("artifact_checksum"), "observation": observation}
            if not validity["eligible"]:
                write_row(quarantined, {**record, "category": "fixture_invalid", "validity": validity})
            elif status in RUNTIME_FAILURES:
                candidate_cards.add(name)
                write_row(findings, {**record, "category": "runtime_exception_candidate"})
    review_path = root / "legal-action-triage.json"
    review = read_json(review_path, warnings, {})
    reviewed_cards = sorted({name for row in review.get("rows", [])
                             if row.get("classification") in REVIEWED_CLASSES
                             for name in row.get("confirmed_cards", [])})
    return {"source": descriptor(path, root), "scope": data.get("scope"),
            "provenance": data.get("provenance"), "runtime_validity": validity,
            "observation_counts": dict(counts), "runtime_candidate_cards": sorted(candidate_cards),
            "review_source": descriptor(review_path, root) if review_path.exists() else None,
            "reviewed_failure_cards": reviewed_cards,
            "unreviewed_exception_cards": sorted(candidate_cards - set(reviewed_cards)),
            "limitation": "Corrected fixtures remain candidates until separately reviewed or reproduced with expected outcomes."}


def summarize_baked(root, warnings):
    candidates = []
    for path in root.glob("baked-artifact-contracts-summary*.json"):
        data = read_json(path, warnings)
        if data and data.get("completed_at"):
            candidates.append((data["completed_at"], path, data))
    if not candidates:
        return {"status": "not_available"}
    _, path, data = max(candidates, key=lambda item: item[0])
    return {"source": descriptor(path, root), "audit_source_sha256": data.get("audit_source_sha256"),
            "corpus_sha256": data.get("corpus_sha256"), "completed_at": data.get("completed_at"),
            "compiler_versions": data.get("compiler_versions"),
            **{key: data.get(key) for key in ("artifact_files", "artifact_rows", "unique_card_names",
                "files_without_artifacts", "rows_with_errors", "rows_with_coverage_gaps", "rows_without_findings",
                "unique_cards_with_errors", "finding_counts")},
            "necessary_contract_cards": sorted({error["card"] for error in data.get("errors", [])}),
            "limitation": "Existing baked artifacts checked structurally against the recorded checker; no execution or fresh compilation."}


def summarize_loadability(root, warnings, findings):
    path = root / "decoder-family-materialization.json"
    report = read_json(path, warnings, {})
    counts, failures, sources = Counter(), [], []
    for source_path in (path, root / "corpus-additional-materialization.json"):
        source_report = report if source_path == path else read_json(source_path, warnings, {})
        if source_report:
            sources.append({"source": descriptor(source_path, root), "scope": source_report.get("scope"),
                            "provenance": source_report.get("provenance")})
        for index, row in enumerate(source_report.get("rows", [])):
            result = row.get("result", {})
            status = result.get("status", "unknown")
            counts[status] += 1
            if status == "materialization_failed":
                failure = {"card": row.get("card"), "source": source_path.name, "source_row": index,
                           "error": result.get("error")}
                failures.append(failure)
                write_row(findings, {**failure,
                                    "category": "confirmed_materialization_failure", "stage": "artifact_load"})
            elif status != "compiled":
                write_row(findings, {"source": source_path.name, "source_row": index, "card": row.get("card"),
                                    "category": "compile_failure" if status == "compile_failed" else "unknown",
                                    "stage": "compilation", "observation": result})
    registry_path = root / "decoder-registry.json"
    registry = read_json(registry_path, warnings, {})
    return {"source": descriptor(path, root) if report else None,
            "scope": report.get("scope"), "provenance": report.get("provenance"),
            "selected_cards": sum(counts.values()), "status_counts": dict(counts), "sources": sources,
            "confirmed_failure_cards": sorted({row["card"] for row in failures if row.get("card")}),
            "confirmed_failure_card_count": len({row["card"] for row in failures if row.get("card")}),
            "failures": failures,
            "registry": {"source": descriptor(registry_path, root) if registry else None,
                         **{key: registry.get(key) for key in ("scope", "provenance", "reference_effect_count",
                             "active_route_count", "missing_from_active_routes", "active_routes_missing_shard_implementation",
                             "shard_implementations_missing_routes", "limitations")}},
            "limitation": "Confirmed compiler-to-runtime artifact load failures only. Materialized cards are not proven gameplay-correct; source registration gaps alone remain candidates."}


def summarize_static_replay(root, warnings, findings):
    path = root / "static-flagged-legal-reproductions.json"
    report = read_json(path, warnings, {})
    statuses, observations, exception_names = Counter(), Counter(), set()
    for index, row in enumerate(report.get("rows", [])):
        result = row.get("result", {})
        statuses[result.get("status", "unknown")] += 1
        for observation in result.get("execution", []):
            status = observation.get("status", "unknown")
            observations[status] += 1
            if status in RUNTIME_FAILURES:
                exception_names.add(row["card"])
                write_row(findings, {"source": path.name, "source_row": index, "card": row["card"],
                                    "category": "runtime_exception_candidate", "observation": observation})
    return {"source": descriptor(path, root) if report else None,
            "scope": report.get("scope"), "provenance": report.get("provenance"),
            "selected_cards": report.get("selected_cards"), "recorded": len(report.get("rows", [])),
            "status_counts": dict(statuses), "execution_status_counts": dict(observations),
            "exception_cards": sorted(exception_names), "review": "static-flagged-legal-triage.json",
            "limitation": "A targeted replay of baked-contract candidates; no exception does not establish that a flagged branch executed."}


def summarize_interactions(root, warnings, findings):
    path = root / "damage-batch-execution.json"
    report = read_json(path, warnings, {})
    review_path = root / "damage-batch-rules.json"
    review = read_json(review_path, warnings, {})
    policy_path = path.with_name(path.stem + ".validity.json")
    policy = read_json(policy_path, warnings)
    valid = runtime_validity(policy, policy_path.exists() and policy is None)
    if report.get("provenance", {}).get("artifacts_unchanged") is False:
        valid = {"eligible": False, "status": "artifacts_changed_during_run"}
    rows, counts = [], Counter()
    for index, row in enumerate(report.get("rows", [])):
        category = expected_category(row) if valid["eligible"] and review else "unknown"
        if "actual" not in row or "expected" not in row:
            category = "unknown"
        counts[category] += 1
        item = {"source": path.name, "source_row": index, "category": category,
                "participating_cards": [row.get("card"), *row.get("related_cards", [])], "row": row}
        rows.append(item)
        write_row(findings, {**item, "evidence_kind": "reviewed_interaction"})
    sources = [{"source": descriptor(path, root),
                "review": descriptor(review_path, root) if review else None}] if report else []
    for review_name in ("x-context-reviewed-attribution.json",):
        attribution_path = root / review_name
        attribution = read_json(attribution_path, warnings, {})
        raw_path = root / Path(attribution.get("raw_report", "missing-report")).name
        raw = read_json(raw_path, warnings, {})
        if not attribution or not raw:
            continue
        review_policy_path = attribution_path.with_name(attribution_path.stem + ".validity.json")
        raw_policy_path = raw_path.with_name(raw_path.stem + ".validity.json")
        review_policy = read_json(review_policy_path, warnings)
        raw_policy = read_json(raw_policy_path, warnings)
        eligible = (runtime_validity(review_policy, review_policy_path.exists() and review_policy is None)["eligible"]
                    and runtime_validity(raw_policy, raw_policy_path.exists() and raw_policy is None)["eligible"]
                    and attribution.get("provenance", {}).get("artifacts_unchanged") is not False
                    and raw.get("provenance", {}).get("artifacts_unchanged") is not False
                    and descriptor(raw_path, root)["sha256"] == attribution.get("raw_report_sha256"))
        sources.append({"source": descriptor(raw_path, root), "review": descriptor(attribution_path, root)})
        for item in attribution.get("interaction_findings", []):
            reference, separator, pointer = item.get("reference", "").partition("#")
            if (item.get("classification") != "runtime_defect_interaction_reproduced"
                    or not separator or Path(reference).name != raw_path.name
                    or not pointer.startswith("/rows/")):
                continue
            try:
                index = int(pointer.removeprefix("/rows/"))
                row = raw["rows"][index]
            except (ValueError, IndexError, KeyError):
                warnings.append({"source": review_name, "classification": "unknown", "error": "invalid reviewed interaction reference"})
                continue
            category = expected_category(row) if eligible and "expected" in row and "actual" in row else "unknown"
            counts[category] += 1
            record = {"source": raw_path.name, "source_row": index, "category": category,
                      "participating_cards": item.get("cards", []), "row": row,
                      "attribution_review": item, "review_source": review_name}
            rows.append(record)
            write_row(findings, {**record, "evidence_kind": "reviewed_interaction"})
    # Some isolated interaction reviews refer to one independently bounded
    # native report per scenario. Keep these outside individual-card totals.
    standalone_raw_reports = {}
    for review_name in ("gitrog-control-interaction-reviewed-attribution.json", "land-play-control-reviewed-attribution.json"):
        review_path = root / review_name
        review = read_json(review_path, warnings, {})
        review_policy_path = review_path.with_name(review_path.stem + ".validity.json")
        review_policy = read_json(review_policy_path, warnings)
        review_valid = runtime_validity(review_policy, review_policy_path.exists() and review_policy is None)["eligible"]
        for item in review.get("interaction_findings", []):
            if item.get("classification") != "runtime_defect_interaction_reproduced":
                continue
            reference = item.get("source_report", {})
            raw_path = root / Path(reference.get("path", "missing-report")).name
            if raw_path not in standalone_raw_reports:
                standalone_raw_reports[raw_path] = read_json(raw_path, warnings, {})
            raw = standalone_raw_reports[raw_path]
            raw_policy_path = raw_path.with_name(raw_path.stem + ".validity.json")
            raw_policy = read_json(raw_policy_path, warnings)
            raw_valid = runtime_validity(raw_policy, raw_policy_path.exists() and raw_policy is None)["eligible"]
            try:
                index = int(item["source_row"])
                row = raw["rows"][index]
            except (ValueError, TypeError, IndexError, KeyError):
                warnings.append({"source": review_name, "classification": "unknown", "error": "invalid standalone interaction reference"})
                continue
            eligible = (review_valid and raw_valid
                        and descriptor(raw_path, root)["sha256"] == reference.get("sha256")
                        and raw.get("provenance", {}).get("artifacts_unchanged") is not False
                        and raw.get("provenance", {}).get("unchanged") is not False
                        and item.get("expected") is not None
                        and item.get("observed") is not None
                        and row.get("expected") == item["expected"]
                        and row.get("actual") == item["observed"])
            category = expected_category(row) if eligible else "unknown"
            counts[category] += 1
            sources.append({"source": descriptor(raw_path, root), "review": descriptor(review_path, root)})
            record = {"source": raw_path.name, "source_row": index, "category": category,
                      "participating_cards": item.get("cards", []), "row": row,
                      "attribution_review": item, "review_source": review_name}
            rows.append(record)
            write_row(findings, {**record, "evidence_kind": "reviewed_interaction"})
    plane_path = root / "remaining-planechase-reproductions.json"
    plane = read_json(plane_path, warnings, {})
    plane_policy_path = plane_path.with_name(plane_path.stem + ".validity.json")
    plane_policy = read_json(plane_policy_path, warnings)
    plane_valid = (runtime_validity(plane_policy, plane_policy_path.exists() and plane_policy is None)["eligible"]
                   and plane.get("provenance", {}).get("artifacts_unchanged") is not False)
    if plane:
        sources.append({"source": descriptor(plane_path, root), "review": "embedded reviewed_attribution"})
    for index, row in enumerate(plane.get("rows", [])):
        if not row.get("interaction_cards") or row.get("card") not in plane.get("reviewed_attribution", {}):
            continue
        category = expected_category(row) if plane_valid and "expected" in row and "actual" in row else "unknown"
        counts[category] += 1
        record = {"source": plane_path.name, "source_row": index, "category": category,
                  "participating_cards": row["interaction_cards"], "row": row,
                  "attribution_review": plane["reviewed_attribution"][row["card"]]}
        rows.append(record)
        write_row(findings, {**record, "evidence_kind": "reviewed_interaction"})
    foretell_path = root / "foretell-candidate-reproductions.json"
    foretell = read_json(foretell_path, warnings, {})
    foretell_policy_path = foretell_path.with_name(foretell_path.stem + ".validity.json")
    foretell_policy = read_json(foretell_policy_path, warnings)
    foretell_valid = (runtime_validity(foretell_policy, foretell_policy_path.exists() and foretell_policy is None)["eligible"]
                     and foretell.get("provenance", {}).get("artifacts_unchanged") is not False)
    interaction_review = foretell.get("reviewed_attribution", {}).get("interaction")
    if foretell:
        sources.append({"source": descriptor(foretell_path, root), "review": "embedded reviewed_attribution.interaction"})
    for index, row in enumerate(foretell.get("rows", [])):
        if not row.get("interaction_cards") or not interaction_review:
            continue
        category = expected_category(row) if foretell_valid and "expected" in row and "actual" in row else "unknown"
        counts[category] += 1
        record = {"source": foretell_path.name, "source_row": index, "category": category,
                  "participating_cards": row["interaction_cards"], "row": row,
                  "attribution_review": interaction_review}
        rows.append(record)
        write_row(findings, {**record, "evidence_kind": "reviewed_interaction"})
    return {"source": descriptor(path, root) if report else None, "sources": sources,
            "rules_review": descriptor(review_path, root) if review else None,
            "scope": report.get("scope"), "provenance": report.get("provenance"), "validity": valid,
            "observation_counts": dict(counts),
            "confirmed_failure_cases": sum(counts[c] for c in CONFIRMED_FAILURES), "rows": rows,
            "limitation": "Interaction failures do not establish that each participating card is independently defective; they are excluded from individual card counts."}


def summarize_tests(root, warnings):
    baseline = read_json(root / "engine-baseline-summary.json", warnings, {})
    triage = read_json(root / "engine-failure-triage.json", warnings, {})
    native_review = read_json(root / "native-integration-triage.json", warnings, {})
    master_review_path = root / "master-wasm-regression-review.json"
    master_review = read_json(master_review_path, warnings, {})
    integrity_path = root / "confirmed-compilation-integrity-review.json"
    integrity = read_json(integrity_path, warnings, {})
    noncompletion_path = root / "noncompletion-replays.json"
    noncompletion = read_json(noncompletion_path, warnings, {})
    native, test_statuses = [], Counter()
    for path in sorted((root / "native-tests").glob("*.json")):
        data = read_json(path, warnings)
        if not data:
            continue
        statuses = Counter(row.get("status", "unknown") for row in data.get("tests", []))
        test_statuses.update(statuses)
        native.append({"source": descriptor(path, root), **{key: data.get(key) for key in (
            "target", "binary", "binary_sha256", "status", "exit_code", "elapsed_seconds", "log")},
            "test_status_counts": dict(statuses)})
    mage = []
    for path in sorted(root.glob("mage-*-verified.json")):
        data = read_json(path, warnings)
        if not data:
            continue
        mage.append({"source": descriptor(path, root), **{key: data.get(key) for key in (
            "scope", "started_at", "duration_seconds", "engine_shims", "returncode", "wall_timeout",
            "selected_files", "status_counts", "classification_counts", "provenance", "tap_path", "limitations")}})
    mage_cumulative = []
    for path in (root / "mage-cumulative.json", root / "mage-campaign/cumulative.json", root / "mage-campaign-five/cumulative.json", root / "mage-campaign-five/with-retries.json"):
        data = read_json(path, warnings, {})
        if data:
            mage_cumulative.append({"source": descriptor(path, root), **{key: data.get(key) for key in (
                "scope", "inventory_files", "inventory_scenarios", "inventory_files_manifest_sha256",
                "status_counts", "classification_counts", "limitations")}})
    latest_mage = max(mage_cumulative, key=lambda item: item["source"]["mtime_ns"], default=None)
    accounting_path = root / "mage-final-accounting.json"
    accounting = read_json(accounting_path, warnings, {})
    accounting_bound = bool(accounting and latest_mage and
        accounting.get("source_cumulative", {}).get("sha256") == latest_mage["source"]["sha256"])
    mage_retries = []
    for path in sorted(root.glob("mage-unreported-retry*.json")):
        data = read_json(path, warnings, {})
        if data:
            mage_retries.append({"source": descriptor(path, root), **{key: data.get(key) for key in (
                "scope", "status", "started_at", "duration_seconds", "engine_shims", "selected_scenarios",
                "selected_files", "selected_unreported", "unselected_scenarios", "status_counts",
                "classification_counts", "provenance", "limitations")}})
    return {"baseline": baseline, "baseline_source": "engine-baseline-summary.json",
            "baseline_failure_triage": {key: triage.get(key) for key in (
                "binary", "binary_sha256", "classification_counts", "pending_baseline_tests", "planner_assessment")},
            "baseline_failure_triage_source": "engine-failure-triage.json", "native_runs": native,
            "native_review_status": {key: native_review.get(key) for key in (
                "generated_at", "campaign_status", "completed_targets", "emitted_targets_expected",
                "target_status_counts", "classification_counts", "limitations")},
            "native_test_observation_counts": dict(test_statuses), "mage_verified_runs": mage,
            "latest_mage_cumulative_snapshot": latest_mage,
            "mage_final_accounting": {"source": descriptor(accounting_path, root) if accounting else None,
                "matches_latest_snapshot": accounting_bound,
                **{key: accounting.get(key) for key in ("scope", "inventory_scenarios", "status_counts",
                    "classification_counts", "bound_wasm_versions", "every_inventory_occurrence_accounted_for",
                    "unreported_individual_attempts", "limitations")}},
            "mage_individual_retry_snapshots": mage_retries,
            "master_wasm_regression": {"source": descriptor(master_review_path, root) if master_review else None,
                                       **{key: master_review.get(key) for key in ("scope", "status", "scenario_count",
                                           "direct_assertion_count", "provenance_unchanged", "source_sha256",
                                           "execution_report_sha256", "limitations")}},
            "compilation_integrity": {"source": descriptor(integrity_path, root) if integrity else None,
                                      "checked_card_names": [row["card"] for row in integrity.get("rows", [])],
                                      **{key: integrity.get(key) for key in ("scope", "selected_cards",
                                          "all_full_inputs_compiled_and_materialized", "oracle_only_fallback_observed_cards",
                                          "metadata_mismatch_cards", "parse_input_difference_cards", "cards_requiring_review",
                                          "strict_parse_loss_cards", "helper_parse_loss_cards", "provenance", "limitations")}},
            "noncompletion_replays": {"source": descriptor(noncompletion_path, root) if noncompletion else None,
                                      "scope": noncompletion.get("scope"),
                                      "rows": [{"card": row.get("card"), "attempts": [{"mode": attempt.get("mode"), "status": attempt.get("result", {}).get("status")} for attempt in row.get("attempts", [])]} for row in noncompletion.get("rows", [])],
                                      "localization_source": "noncompletion-localization.json" if (root / "noncompletion-localization.json").exists() else None},
            "mage_inventory": {key: value for key, value in read_json(root / "mage-inventory.json", warnings, {}).items()
                               if key in {"generated_at", "files", "tests", "explicitly_skipped", "no_direct_assert", "limitations"}},
            "limitation": "Assertion failures are test results, not confirmed card defects. Counts may repeat tests across binaries; incomplete or fixture-invalid tests are not passes."}


def summarize_screen_coverage(root, warnings):
    path = root / "activation-gate-coverage.json"
    report = read_json(path, warnings, {})
    activation = {"source": descriptor(path, root) if report else None,
                  **{key: report.get(key) for key in ("scope", "coverage_counts",
                      "directly_sampled_inputs", "identical_front_input_aliases",
                      "all_cards_verified", "all_candidate_branches_verified", "provenance")}}
    sources = []
    for filename in ("activation-front-alias-review.json", "activation-restriction-candidates.json",
                     "numeric-predicate-candidates.json", "numeric-color-gate-reproductions.json",
                     "source-characteristic-candidates.json", "optional-oneof-cost-candidates.json",
                     "noncast-x-reviewed-attribution.json", "semantic-candidate-forms.json",
                     "mage-review-queue.json", "rune-execution-reproductions.json",
                     "rune-execution-reproductions.md", "rune-fixture-validity.json",
                     "spell-trigger-x-candidates.json", "conditional-static-grant-candidates.json",
                     "iterated-player-family-coverage.json", "simultaneous-family-coverage.json",
                     "processor-reviewed-classification.json", "processor-execution.json",
                     "counter-outcome-reviewed-classification.json", "counter-outcome-execution.json",
                     "cohort-reviewed-classification.json", "cohort-reproductions.json",
                     "reconfigure-reproductions.json", "ability-index-structural-candidates.json",
                     "ability-index-followup-frozen-parity.json", "ability-index-restriction-frozen-parity.json", "ability-index-rule-frozen-parity.json",
                     "ability-index-family-coverage.json", "ability-index-static-gate-review.json", "ability-index-remaining-priority.json", "ability-index-anthem-frozen-parity.json",
                     "damage-source-candidates.json", "damage-source-reviewed-coverage.json", "counted-damage-target-candidates.json", "gitrog-timeout-native-reviewed-classification.json", "variable-mana-cost-reviewed-coverage.json",
                     "ability-index-mana-wrapper-frozen-parity.json", "ability-index-equipment-grant-frozen-parity.json", "counted-damage-family-coverage.json",
                     "ability-index-devotion-frozen-parity.json", "ability-index-active-grant-frozen-parity.json", "ability-index-pair-frozen-parity.json",
                     "dynamic-discard-cost-candidates.json", "token-capacity-reviewed-classification.json",
                     "dynamic-discard-all-cost-candidates.json", "dynamic-discard-cost-family-coverage.json",
                     "multitarget-remaining-reviewed-ledger.json", "multitarget-combat-reviewed-classification.json",
                     "target-tag-overwrite-candidates.json", "target-tag-overwrite-frozen-parity.json",
                     "counter-choice-reviewed-classification.json", "counter-choice-frozen-parity.json",
                     "counter-choice-sibling-reviewed-classification.json", "counter-choice-sibling-frozen-parity.json",
                     "counter-removal-cost-candidates.json", "ability-index-hand-frozen-parity.json",
                     "variable-counter-cost-reviewed-classification.json", "variable-counter-cost-frozen-parity.json",
                     "counter-removal-cost-family-coverage.json", "static-target-condition-candidates.json",
                     "counter-cost-misc-reviewed-classification.json", "counter-cost-misc-frozen-parity.json",
                     "counter-cost-remaining-reviewed-attribution.json", "static-execution-condition-candidates.json",
                     "static-context-family-coverage.json", "static-context-family-coverage.md",
                     "self-sacrifice-effect-cost-family-coverage.json", "self-sacrifice-effect-cost-family-coverage.md",
                     "nonmana-x-cost-family-coverage.json", "nonmana-x-cost-candidates.json",
                     "hand-reveal-cost-reviewed-attribution.json", "hand-reveal-cost-final-execution.json",
                     "fixed-tap-cost-reviewed-classification.json", "fixed-tap-cost-reproductions.json",
                     "fixed-tap-sibling-reviewed-classification.json", "fixed-tap-sibling-reproductions.md",
                     "fixed-tap-cost-family-coverage.json", "fixed-tap-existing-control-mappings.json",
                     "fixed-exile-simple-reviewed-attribution.json", "fixed-exile-cost-coverage.json",
                     "choose-consume-cost-family-coverage.json", "choose-consume-cost-family-coverage.md", "choose-consume-cost-candidates.json",
                     "tribal-tap-cost-reviewed-classification.json", "tribal-tap-cost-reproductions.md",
                     "../../scripts/audit_runtime_station_striations.py", "../../scripts/summarize_station_striation_coverage.py",
                     "../../scripts/summarize_choose_consume_cost_coverage.py", "../../scripts/audit_runtime_choose_consume_costs.py",
                     "single-return-land-reviewed-attribution.json", "single-return-special-reviewed-attribution.json", "single-return-cost-path-coverage.json",
                     "single-move-cost-reviewed-classification.json", "single-move-cost-family-coverage.json", "single-move-cost-reproductions.md",
                     "single-exile-cost-coverage.json", "single-exile-simple-reproductions.md", "single-exile-craft-reproductions.md", "single-exile-special-reproductions.md", "single-exile-stack-hand-reproductions.md", "mixed-tap-cost-reproductions.md", "alternative-tap-cost-execution.json", "alternative-tap-cost-path-coverage.json", "single-tap-station-path-coverage.json", "single-tap-station-reviewed-attribution.json", "tap-outcome-reproductions.md", "special-tap-cost-reproductions.md", "shimmer-tap-reproductions.md", "single-exile-battlefield-reproductions.md", "single-exile-remaining-reproductions.md", "eladamri-tap-reproductions.md", "dermotaxi-tap-reproductions.md", "tap-copy-reproductions.md", "single-tap-mana-path-coverage.json", "weight-tap-reproductions.md", "linked-face-cost-reproductions.md", "linked-face-cost-parity.json", "linked-face-materialization-route.json", "prepare-entry-reproductions.md", "ability-index-speed-discard-reproductions.md", "ability-index-linked-transition-reproductions.md", "ability-index-alias-front-reproductions.md", "ability-index-alias-front-event-parity.json", "ability-index-alias-front-actions-parity.json", "single-tap-special-six-path-coverage.json", "prepare-trigger-reproductions.md", "prepare-trigger-parity.json", "spell-return-land-reproductions.md", "ability-index-linked-transition-event-parity.json", "ability-index-linked-transition-actions-parity.json", "ability-index-speed-discard-event-parity.json", "ability-index-speed-discard-actions-parity.json", "single-tap-source-path-coverage.json", "single-land-tap-reproductions.md", "single-special-seven-reproductions.md", "extended-escape-reproductions.md", "extended-alternate-exile-reproductions.md", "extended-exile-cost-coverage.json", "additional-withid-simple-path-coverage.json", "spell-alt-tap-reproductions.md", "ward-waterbend-reproductions.md", "ward-waterbend-path-coverage.json", "ward-waterbend-event-parity.json", "ward-waterbend-actions-parity.json", "spell-tap-flashback-reproductions.md", "optional-tap-outcome-reproductions.md", "extended-optional-exile-reproductions.md", "extended-trigger-cost-reproductions.md", "conditional-land-entry-reproductions.md", "conditional-land-entry-actions-parity.json", "conditional-land-entry-corpus-parity.json", "additional-withid-special-path-coverage.json", "extended-cost-dependency-candidates.json", "extended-cost-dependency-coverage.json", "../../scripts/summarize_extended_cost_coverage.py", "../../scripts/audit_runtime_extended_cost_dependencies.py", "spell-web-return-reproductions.md", "ability-index-island-attack-reproductions.md", "ability-index-island-attack-event-parity.json", "ability-index-island-attack-actions-parity.json", "return-morph-kicker-reproductions.md", "prepare-face-coverage.json", "prepare-entry-parity.json", "snapshots/linked-face-cost-06469a28a64e5e00/manifest.json", "replay-history.json", "meria-tap-exile-path-coverage.json", "meria-tap-exile-execution.json", "combat-tap-reproductions.md", "tap-copy-reproductions.json", "tap-copy-event-parity.json", "tap-copy-actions-parity.json", "single-tap-effect-path-coverage.json", "ability-index-gourmand-reproductions.md", "ability-index-turn-loyalty-reproductions.md", "ability-index-turn-loyalty-event-parity.json", "ability-index-turn-loyalty-actions-parity.json", "ability-index-gourmand-event-parity.json", "ability-index-gourmand-actions-parity.json",
                     "unattach-cost-path-coverage.json", "unattach-cost-reviewed-attribution.json", "unattach-granted-final-execution.json", "unattach-intrinsic-execution.json",
                     "ability-index-cost-transition-frozen-parity.json", "ability-index-type-equipment-frozen-parity.json",
                     "ability-index-station-reproductions.json", "ability-index-station-frozen-parity.json", "ability-index-station-fixture-validity.json",
                     "ability-index-turn-equipment-reproductions.md", "ability-index-turn-equipment-event-parity.json", "ability-index-turn-equipment-actions-parity.json", "ability-index-counter-level-reproductions.json", "ability-index-counter-level-reproductions.md", "ability-index-counter-level-event-parity.json", "ability-index-counter-level-actions-parity.json", "ability-index-zone-stack-reproductions.json", "ability-index-zone-stack-reproductions.md", "ability-index-zone-stack-event-parity.json", "ability-index-zone-stack-actions-parity.json", "ability-index-attachment-switch-reproductions.json", "ability-index-attachment-switch-frozen-parity.json", "station-striation-candidates.json",
                     "station-threshold-static-reproductions.json", "station-threshold-static-reproductions.md", "station-threshold-static-frozen-parity.json",
                     "station-threshold-event-reproductions.json", "station-threshold-event-frozen-parity.json", "station-striation-family-coverage.json",
                     "ability-index-arcades-reproductions.json", "ability-index-arcades-frozen-parity.json",
                     "coat-native-profile-reviewed-classification.json", "mage-timeout-followup-ledger.json",
                     "player-choice-filter-coverage.json", "boreas-choice-reviewed-classification.json",
                     "cultural-choice-reviewed-classification.json", "layer-interaction-reviewed-classification.json",
                     "damage-distribution-reviewed-classification.json", "damage-distribution-reproductions.json",
                     "damage-distribution-additional-reviewed-classification.json", "damage-distribution-additional-reproductions.json"):
        path = root / filename
        if path.exists():
            sources.append(descriptor(path, root))
    return {"activation": activation, "sources": sources,
            "limitation": "Screens are candidate generators. Scoped execution coverage and exact input aliases do not certify whole cards or all choices."}


def render_readme(summary):
    confirmed, baked, tests = (summary[key] for key in ("confirmed_card_outcomes", "baked_contracts", "tests"))
    counts = confirmed["observation_counts"]
    lines = ["# Runtime card audit", "", f"Generated {summary['generated_at']}. **All cards correct: false.**", "",
        "This index separates expected-result reproductions and reviewed integration failures from runtime fixture candidates, necessary structural contracts, and incomplete coverage. It does not infer gameplay correctness from compilation, successful resolution, or an assertion failing in an unverified fixture.", "",
        "## Confirmed scenario outcomes", "",
        f"**{confirmed['confirmed_failure_card_count']} card/face names have a reproduced failure** in named expected-result or reviewed integration reports: "
        f"{counts.get('silent_wrong_result', 0)} silent wrong-result observations and {counts.get('runtime_exception', 0)} runtime exceptions. "
        + (f"{counts['runtime_nontermination']} nonterminating scenarios have explicit reachability and cause reviews. " if counts.get('runtime_nontermination') else "") +
        (f"{counts['reviewed_card_failure']} reviewed failures do not yet specify their failure subtype. " if counts.get('reviewed_card_failure') else "") +
        f"{counts.get('expected_result_observed', 0)} scenarios matched their expected result; this does not prove those cards correct in other states.", "",
        "Table totals include passing controls and inconclusive cases.", "",
        "| Card | Reproduced failure | Total cases |", "| --- | --- | --- |"]
    for card in confirmed["cards"]:
        failures = [key for key in sorted(CONFIRMED_FAILURES) if card["observations"].get(key)]
        if failures:
            lines.append(f"| {card['card']} | {', '.join(failures)} | {sum(card['observations'].values())} |")
    lines += ["", "Exact expected/actual values, scopes and source rows: [confirmed outcomes](confirmed-outcomes.jsonl). "
              "Primary reports: " + ", ".join(f"[{source['path']}]({source['path']})" for source in confirmed["sources"]) + ".", "",
              "## Artifact loadability", ""]
    interactions = summary["confirmed_interactions"]
    if interactions.get("sources"):
        lines[-2:-2] = [f"Separate interaction reports have {interactions['confirmed_failure_cases']} confirmed failing cases, "
                       f"with {interactions['observation_counts'].get('expected_result_observed', 0)} passing controls. "
                       "These failures are excluded from individual-card counts. "
                       + ", ".join(f"[{item['source']['path']}]({item['source']['path']})" for item in interactions["sources"])
                       + ". Attribution and rules reviews are linked in summary.json.", ""]
    loadability = summary["confirmed_loadability_failures"]
    if loadability.get("source"):
        load_counts = loadability["status_counts"]
        lines += [f"**{loadability['confirmed_failure_card_count']} additional loadability failures** were observed after successful compilation "
                  f"across {loadability.get('selected_cards')} selected observations in "
                  + ", ".join(f"[{s['source']['path']}]({s['source']['path']})" for s in loadability['sources']) + ". "
                  f"{load_counts.get('compiled', 0)} cards materialized; {load_counts.get('compile_failed', 0)} failed compilation. "
                  "These are artifact-loading results, counted separately from the gameplay scenarios above.", "",
                  "Failed loads: " + ", ".join(loadability["confirmed_failure_cards"]) + ".", ""]
    registry = loadability["registry"]
    if registry.get("source"):
        lines += [f"The [source decoder parity audit]({registry['source']['path']}) found "
                  f"{len(registry.get('missing_from_active_routes') or [])} reference effect types missing from active routes. "
                  "Its source-registration findings alone are candidates, not executed card failures.", ""]
    lines += ["## Corpus and legal-action campaigns", "",
              "Runs retain their original binaries, manifests and data. A `validity.json` quarantine excludes **all runtime observations from that run**, including apparent passes. Its compilation and structural findings remain available. An eligible run still produces candidates, not confirmed card outcomes.", "",
              "| Campaign / run | Recorded / inventory | Runtime evidence | Runtime failure candidates | Remaining |",
              "| --- | ---: | --- | ---: | ---: |"]
    for run in summary["campaigns"]["runs"]:
        valid = run["runtime_validity"]
        label = "candidate observations" if valid["eligible"] else "**quarantined**"
        if run.get("recording_status") == "stopped_partial":
            label += "; stopped partial"
        if run.get("explicit_face_payloads_missing_count"):
            label += f"; inventory lacks {run['explicit_face_payloads_missing_count']} named faces"
        lines.append(f"| [{run['campaign']} / {run['run_id']}]({run['manifest']}) | "
                     f"{run['recorded']} / {run.get('canonical_payloads', 'unknown')} | {label} | "
                     f"{len(run.get('runtime_candidate_cards', []))} cards | {run.get('remaining', 'unknown')} |")
    lines += ["", "[Compile, materialization, structural and runtime candidate details](consolidated-findings.jsonl). "
              "[Quarantined runtime observations](quarantined-runtime-observations.jsonl). "
              "Worker timeouts, panics before a known stage, protocol failures and incomplete branches remain unknown; they are not automatically card execution bugs.", "",
              "## Existing baked artifacts", ""]
    ledger = summary.get("corpus_triage", {})
    if ledger:
        lines[-2:-2] = [f"The [completed event-corpus triage ledger](corpus-triage-ledger.md) groups "
                       f"{ledger.get('candidate_card_count')} flagged card/face names into {len(ledger.get('families', []))} "
                       "cause families, retaining reviewed family matches, fixture limits, and unreviewed findings. "
                       "[Semantic text-screen prioritization](semantic-candidate-forms.json) preserves the original candidate counts.", ""]
    action_ledger = summary.get("actions_triage", {})
    if action_ledger:
        lines[-2:-2] = [f"The [legal-action ledger](actions-triage-ledger.md) groups {action_ledger.get('candidate_card_count')} "
                       "flagged names. [Separate action/choice, panic, and budget inventory](actions-priority-ledger.json) "
                       "includes budget-only observations absent from the original candidate selection. "
                       "[Isolated noncompletion replay](noncompletion-replays.json) and [diagnostic localization](noncompletion-localization.json) remain separate from confirmed gameplay outcomes.", ""]
    replay = summary["static_candidate_replay"]
    if replay.get("source"):
        lines[-2:-2] = [f"The [baked-candidate legal replay]({replay['source']['path']}) recorded "
                       f"{replay['recorded']} / {replay.get('selected_cards')} selected names and found exceptions for "
                       f"{len(replay['exception_cards'])} names. [Separate reachability review]({replay['review']}) controls promotion. "
                       "Names with no exception are not thereby cleared.", ""]
    supplemental = summary["supplemental_legal_candidates"]
    if supplemental.get("source"):
        unconfirmed_sample = set(supplemental.get("runtime_candidate_cards", [])) - set(confirmed["confirmed_failure_cards"])
        lines[-2:-2] = [f"The [corrected legal-action sample]({supplemental['source']['path']}) has "
            f"{len(supplemental.get('runtime_candidate_cards', []))} cards with exception observations. "
            f"{len(supplemental.get('reviewed_failure_cards', []))} cards have separate reachability reviews; "
            f"{len(unconfirmed_sample)} remain unconfirmed after considering the expected-result reports above. "
            + ("[Legal-action review](legal-action-triage.json)." if supplemental.get("review_source") else ""), ""]
    if baked.get("source"):
        lines += [f"The latest structural scan found **{baked.get('unique_cards_with_errors')} cards with necessary-contract violations** "
                  f"in {baked.get('artifact_rows'):,} artifact rows across {baked.get('artifact_files'):,} files. "
                  f"{baked.get('files_without_artifacts'):,} files had no compiled artifact. These findings are not execution confirmations.", "",
                  f"[Baked scan summary]({baked['source']['path']}); exact pointers and artifact provenance are retained in the corresponding baked report files. "
                  f"{baked.get('rows_with_coverage_gaps'):,} rows have coverage gaps. Zero findings is not proof of correct gameplay.", ""]
    lines += ["## Existing tests and MAGE ports", "",
              f"Native baseline: {tests['baseline'].get('passed', 'unknown')} passed, "
              f"{len(tests['baseline'].get('failed', []))} failed, {len(tests['baseline'].get('pending', []))} pending; "
              f"status `{tests['baseline'].get('status', 'unknown')}`. [Baseline](engine-baseline-summary.json), "
              "[failure triage](engine-failure-triage.json). Unit failures have separate stale-assertion, fixture and runtime classifications; card confirmation requires a named reproduction.", "",
              f"{len(tests['native_runs'])} native test-binary reports are indexed in [summary.json](summary.json). "
              "Their counts are test observations and can include repeated tests across binary versions.", ""]
    review = tests["native_review_status"]
    if review.get("emitted_targets_expected"):
        lines += [f"The reviewed native campaign snapshot reports {review.get('completed_targets')} / "
                  f"{review['emitted_targets_expected']} targets completed, status `{review.get('campaign_status')}` "
                  f"as of {review.get('generated_at')}. [Review](native-integration-triage.json).", ""]
    integrity = tests["compilation_integrity"]
    if integrity.get("source"):
        lines += [f"The [compilation integrity check]({integrity['source']['path']}) compared the scenario helper with strict "
                  f"full-input compilation for {integrity.get('selected_cards')} selected confirmed names. "
                  f"All strict inputs compiled/materialized: {integrity.get('all_full_inputs_compiled_and_materialized')}; "
                  f"fallbacks observed: {len(integrity.get('oracle_only_fallback_observed_cards') or [])}; "
                  f"primary metadata mismatches: {len(integrity.get('metadata_mismatch_cards') or [])}. "
                  "This fresh check does not retrospectively prove historical helper branches.", ""]
        if integrity.get("unchecked_current_confirmed_cards"):
            lines += [f"{len(integrity['unchecked_current_confirmed_cards'])} names confirmed after that integrity snapshot remain listed as unchecked in summary.json.", ""]
    followups = summary.get("targeted_screen_coverage", {})
    activation = followups.get("activation", {})
    if activation.get("source"):
        lines += [f"The [activation-gate coverage ledger]({activation['source']['path']}) associates "
                  f"{activation.get('directly_sampled_inputs')} distinct compilation inputs with scoped reviewed scenarios, plus "
                  f"{activation.get('identical_front_input_aliases')} identical front-input aliases. "
                  "The [alias review](activation-front-alias-review.json) establishes input equivalence only, not linked-face transitions. "
                  "Candidate coverage does not establish that every ability branch or card is correct.", ""]
    if followups.get("sources"):
        lines += ["Additional screens, scoped controls, and review queues: " + ", ".join(
            f"[{source['path']}]({source['path']})" for source in followups["sources"]) + ".", ""]
    for mage in tests["mage_verified_runs"]:
        lines.append(f"- [MAGE report]({mage['source']['path']}): {mage.get('selected_files')} selected files; "
                     f"statuses {mage.get('status_counts')}; classifications {mage.get('classification_counts')}. "
                     f"Artifacts unchanged: {mage.get('provenance', {}).get('unchanged', 'unknown')}. "
                     "Port/fixture mismatches and parser failures are not promoted to confirmed card bugs.")
    latest_mage = tests.get("latest_mage_cumulative_snapshot")
    if latest_mage:
        lines += ["", f"The latest [deduplicated imported-scenario snapshot]({latest_mage['source']['path']}) covers "
                  f"an inventory of {latest_mage.get('inventory_scenarios')} scenarios in {latest_mage.get('inventory_files')} files, "
                  f"with statuses {latest_mage.get('status_counts')}. Passing port tests and unresolved failures are not whole-card verdicts."]
    accounting = tests.get("mage_final_accounting", {})
    if accounting.get("source"):
        lines += ["", f"The [final scenario accounting]({accounting['source']['path']}) binds the immutable WASM versions "
                  f"and records every inventory occurrence, including individually attempted cases with no terminal result. "
                  f"Matches this snapshot: {accounting.get('matches_latest_snapshot')}. "
                  "A completed test without verified direct assertions is not treated as an outcome check."]
    if tests.get("mage_individual_retry_snapshots"):
        lines += ["", "Individual retries of previously unreported imported scenarios: " + ", ".join(
            f"[{run['source']['path']}]({run['source']['path']})" for run in tests["mage_individual_retry_snapshots"]) +
            ". Running checkpoints remain unverified; retry totals are not added to campaign totals without scenario deduplication."]
    master = tests["master_wasm_regression"]
    if master.get("source"):
        lines += ["", f"The [Master of Lake-town WASM regression review]({master['source']['path']}) records "
                  f"{master.get('scenario_count')} authored scenarios and {master.get('direct_assertion_count')} direct assertions, "
                  f"status `{master.get('status')}`; artifacts unchanged: {master.get('provenance_unchanged')}. "
                  "This covers the original life-loss milling fix only; its separate death-trigger failure remains. "
                  "The preserved initial legend-rule fixture mistake is excluded from bug evidence."]
    lines += ["", "## Rebuild this index", "", "```sh", "python3 scripts/summarize_runtime_audit.py", "```", "",
              "The generator reads SQLite snapshots and available JSON reports, writes derived files atomically, and preserves original evidence. Rerun it as campaigns finish. "
              "[Machine-readable summary](summary.json) includes per-run status counts, validity, source hashes, provenance and coverage limitations.", "",
              "Remaining coverage includes unvisited targets and choices, delayed/reflexive chains, multiplayer states, static/replacement interactions, and gameplay semantics without expected-result assertions. "
              "Compile failures, materialization failures, runtime exception candidates, silent wrong results, invalid fixtures and unknown cases remain separate."]
    if summary["warnings"]:
        lines += ["", f"**{len(summary['warnings'])} input warnings** are recorded in summary.json; unavailable or partial evidence was not counted as a pass."]
    return "\n".join(lines) + "\n"


def generate(root, retries=2):
    root.mkdir(parents=True, exist_ok=True)
    READ_DESCRIPTORS.clear()
    policies_before = validity_snapshot(root)
    warnings = []
    temporary = []
    with ExitStack() as stack:
        streams = {}
        for name in ("confirmed-outcomes.jsonl", "consolidated-findings.jsonl", "quarantined-runtime-observations.jsonl"):
            handle = stack.enter_context(tempfile.NamedTemporaryFile("w", encoding="utf-8", dir=root, prefix="." + name, delete=False))
            temporary.append((Path(handle.name), root / name))
            streams[name] = handle
        summary = {"schema_version": 1, "generated_at": utcnow(), "all_cards_correct": False,
                   "generator_sha256": digest(Path(__file__)), "report_root": str(root.resolve()),
                   "confirmed_card_outcomes": summarize_expected(root, warnings, streams["confirmed-outcomes.jsonl"]),
                   "campaigns": summarize_campaigns(root, warnings, streams["consolidated-findings.jsonl"], streams["quarantined-runtime-observations.jsonl"]),
                   "supplemental_legal_candidates": summarize_supplemental_legal(root, warnings, streams["consolidated-findings.jsonl"], streams["quarantined-runtime-observations.jsonl"]),
                   "baked_contracts": summarize_baked(root, warnings), "tests": summarize_tests(root, warnings),
                   "confirmed_loadability_failures": summarize_loadability(root, warnings, streams["consolidated-findings.jsonl"]),
                   "confirmed_interactions": summarize_interactions(root, warnings, streams["consolidated-findings.jsonl"]),
                   "static_candidate_replay": summarize_static_replay(root, warnings, streams["consolidated-findings.jsonl"]),
                   "corpus_triage": read_json(root / "corpus-triage-summary.json", warnings, {}),
                   "actions_triage": read_json(root / "actions-triage-summary.json", warnings, {}),
                   "targeted_screen_coverage": summarize_screen_coverage(root, warnings),
                   "warnings": warnings, "snapshot_completed_at": utcnow()}
        integrity = summary["tests"]["compilation_integrity"]
        integrity["unchecked_current_confirmed_cards"] = sorted(set(summary["confirmed_card_outcomes"]["confirmed_failure_cards"]) - set(integrity["checked_card_names"]))
    # Do not publish runtime candidates under a policy revoked during the scan.
    # Other evidence is versioned by the bytes actually read; validity is a
    # revocation and must be current when this derived report is published.
    if policies_before != validity_snapshot(root):
        for source, _ in temporary:
            source.unlink(missing_ok=True)
        if retries:
            return generate(root, retries - 1)
        raise RuntimeError("Validity policies changed repeatedly; no new report published. Rerun when stable.")
    for source, target in temporary:
        source.replace(target)
    for name, text in (("README.md", render_readme(summary)), ("summary.json", json.dumps(summary, indent=2, ensure_ascii=False) + "\n")):
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", dir=root, prefix="." + name, delete=False) as stream:
            stream.write(text)
            source = Path(stream.name)
        source.replace(root / name)
    return summary


def self_test():
    assert expected_category({"status": "timeout", "actual": {"timeout_seconds": 90}}) == "unknown"
    assert expected_category({"status": "runtime_nontermination", "outcome_category": "runtime_nontermination"}) == "runtime_nontermination"
    assert expected_category({"status": "confirmed_resolution_failure", "actual": {"resolution_error": "missing player"}}) == "runtime_exception"
    assert expected_category({"status": "confirmed_resolution_failure", "actual": {"resolution_error": None}}) == "unknown"
    assert expected_category({"status": "resolution_completed", "expected": {"resolution_error": None}, "actual": {"resolution_error": None}}) == "expected_result_observed"
    # This regression guards the distinction that matters most: an invalid
    # fixture must never contribute runtime failures or apparent passes.
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        campaign = root / "corpus"
        run = campaign / "old"
        run.mkdir(parents=True)
        (run / "validity.json").write_text(json.dumps({"status": "runtime_results_require_reproduction"}))
        (run / "inventory.json").write_text(json.dumps({"payload_count": 2, "exclusions": []}))
        database = sqlite3.connect(campaign / "results.sqlite3")
        database.executescript("create table run(run_id text,metadata text,created_at text); create table result(run_id text,card_name text,status text,result_json text,recorded_at text);")
        database.execute("insert into run values('old','{}','2026-01-01')")
        database.execute("insert into result values(?,?,?,?,?)", ("old", "Fixture card", "compiled", json.dumps({"contracts": [{"severity": "error", "code": "test"}], "execution": [{"status": "resolution_failed"}, {"status": "executed"}]}), "2026-01-01"))
        database.commit()
        database.close()
        (root / "semantic-execution.json").write_text(json.dumps({"rows": [
            {"card": "Real card", "status": "semantic_mismatch", "expected": 2, "actual": 0},
            {"card": "Error card", "status": "semantic_mismatch", "expected": {"resolution_error": None},
             "actual": {"resolution_error": "cannot resolve value"}},
        ]}))
        (root / "native-integration-triage.json").write_text(json.dumps({"rows": [
            {"classification": "runtime_defect_card_reproduced", "confirmed_cards": ["Reviewed card"], "expected": "moved", "observed": "stayed"},
            {"classification": "pending_review", "confirmed_cards": ["Unreviewed card"], "expected": "moved", "observed": "stayed"},
        ]}))
        (root / "legal-action-triage.json").write_text(json.dumps({"rows": [
            {"classification": "compiler_semantic_defect_card_reproduced", "confirmed_cards": ["Legal card"],
             "outcome_category": "runtime_exception", "expected": {"error": None}, "observed": {"error": "missing outcome"}},
            {"classification": "needs_richer_fixture", "confirmed_cards": ["Not proven legal"],
             "outcome_category": "runtime_exception", "expected": {"error": None}, "observed": {"error": "missing outcome"}},
        ]}))
        (root / "legal-action-triage-batch2.json").write_text(json.dumps({
            "provenance": {"source_runs": [{"campaign": "corpus", "run_id": "old"}]},
            "rows": [{"classification": "runtime_defect_card_reproduced", "confirmed_cards": ["Revoked review"],
                      "outcome_category": "runtime_exception", "expected": {"error": None}, "observed": {"error": "failure"}}],
        }))
        (root / "decoder-family-materialization.json").write_text(json.dumps({"selected_cards": 2, "rows": [
            {"card": "Decoder card", "result": {"status": "materialization_failed", "error": "missing decoder"}},
            {"card": "Loaded card", "result": {"status": "compiled"}},
        ]}))
        (root / "remaining-planechase-reproductions.json").write_text(json.dumps({
            "reviewed_attribution": {"Interaction card": "Scoped multi-card failure only."},
            "rows": [{"card": "Interaction card", "interaction_cards": ["Interaction card", "Enabling card"],
                      "status": "resolution_failed", "expected": {"error": None}, "actual": {"error": "failure"}}],
        }))
        interaction_raw = {"rows": [{"card": "Host card", "status": "resolution_failed",
                                      "expected": {"error": None}, "actual": {"error": "failure"}}]}
        interaction_path = root / "x-context-execution.json"
        interaction_path.write_text(json.dumps(interaction_raw))
        (root / "x-context-reviewed-attribution.json").write_text(json.dumps({
            "raw_report": interaction_path.name, "raw_report_sha256": "incorrect hash",
            "interaction_findings": [{"cards": ["Host card", "Copied card"],
                                      "classification": "runtime_defect_interaction_reproduced",
                                      "reference": "x-context-execution.json#/rows/0"}],
        }))
        standalone_path = root / "isolated-interaction.json"
        standalone_row = {"status": "semantic_mismatch", "expected": {"controller": 1}, "actual": {"controller": 0}}
        standalone_path.write_text(json.dumps({"rows": [standalone_row], "provenance": {"artifacts_unchanged": True}}))
        standalone_finding = {"classification": "runtime_defect_interaction_reproduced",
            "cards": ["Interaction source", "Control Aura"], "source_row": 0,
            "source_report": {"path": standalone_path.name, "sha256": digest(standalone_path)},
            "expected": standalone_row["expected"], "observed": standalone_row["actual"]}
        (root / "gitrog-control-interaction-reviewed-attribution.json").write_text(json.dumps({
            "interaction_findings": [standalone_finding, {**standalone_finding, "observed": {"controller": 2}}]}))
        (root / "single-exile-special-reviewed-attribution.json").write_text(json.dumps({
            "provenance": [{"artifacts_unchanged": True}],
            "findings": [{"classification": "runtime_defect_card_reproduced",
                          "confirmed_cards": ["Malformed provenance"], "expected": 1, "observed": 0}],
        }))
        (root / "multitarget-reviewed-classification.json").write_text(json.dumps({
            "provenance": {"source_runs": [{"source_report": "native.json"}]},
            "findings": [{"classification": "runtime_defect_card_reproduced",
                          "confirmed_cards": ["Malformed dependency"], "expected": 1, "observed": 0}],
        }))
        result = generate(root)
        assert any("Malformed campaign source_runs" in warning.get("error", "") for warning in result["warnings"])
        assert any("Unsupported provenance shape" in warning.get("error", "") for warning in result["warnings"])
        assert result["confirmed_card_outcomes"]["confirmed_failure_cards"] == ["Error card", "Legal card", "Real card", "Reviewed card"]
        assert result["confirmed_card_outcomes"]["observation_counts"]["runtime_exception"] == 2
        assert result["confirmed_card_outcomes"]["observation_counts"]["fixture_invalid"] == 1
        assert result["confirmed_loadability_failures"]["confirmed_failure_cards"] == ["Decoder card"]
        assert result["confirmed_interactions"]["confirmed_failure_cases"] == 2
        assert result["confirmed_interactions"]["observation_counts"] == {"unknown": 2, "runtime_exception": 1, "silent_wrong_result": 1}
        run = result["campaigns"]["runs"][0]
        assert run["runtime_candidate_cards"] == [] and run["execution_status_counts"] == {}
        assert run["quarantined_execution_status_counts"] == {"resolution_failed": 1, "executed": 1}
        assert run["structural_candidate_cards"] == ["Fixture card"] and run["remaining"] == 1
        assert len((root / "quarantined-runtime-observations.jsonl").read_text().splitlines()) == 2
        assert not result["all_cards_correct"]
    assert not runtime_validity({"status": "superseded_harness_identity_collision"})["eligible"]
    assert not runtime_validity({"status": "unknown_policy"})["eligible"]
    assert runtime_validity(None)["eligible"]
    print("Report aggregation regression checks passed.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1] / "reports/runtime-audit")
    parser.add_argument("--self-test", action="store_true")
    arguments = parser.parse_args()
    if arguments.self_test:
        self_test()
    else:
        result = generate(arguments.root.resolve())
        print(json.dumps({"generated_at": result["generated_at"], "confirmed_failure_cards": result["confirmed_card_outcomes"]["confirmed_failure_card_count"], "campaign_runs": len(result["campaigns"]["runs"]), "warnings": len(result["warnings"]), "all_cards_correct": False}))
