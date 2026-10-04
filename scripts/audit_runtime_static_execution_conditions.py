#!/usr/bin/env python3
"""Screen authored static conditions for execution/trigger-context dependencies.

Candidate discovery only: a replacement/cost payload can evaluate its condition
under a different context from a continuous ability. Every hit needs routing and
actual gameplay review. The frozen worker is not changed.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sqlite3
from audit_runtime_static_target_conditions import static_roots

BASE = Path(__file__).resolve().parents[1]
ROOT = BASE / "reports/runtime-audit"
CONDITIONS = BASE / "crates/ironsmith-engine/src/condition_eval.rs"
STATIC_CONTEXT = BASE / "crates/ironsmith-engine/src/static_abilities/continuous.rs"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_guards(source):
    """Record exact source guards; do not infer whole-variant behavior from them."""
    lines = source.splitlines()
    current = None
    result = {}
    for index, line in enumerate(lines):
        match = re.match(r"        Condition::([A-Za-z0-9_]+)", line)
        if match:
            current = (match.group(1), index + 1)
        if (current and "let Some(ctx) = ctx.execution() else" in line
                and "return Ok(false);" in lines[index + 1]):
            result[current[0]] = {"variant_line": current[1], "guard_line": index + 1,
                "reason": "Variant contains a missing-execution-context false guard; earlier external branches still need review."}
    # Direct optional-event accesses, explicitly inspected in source. This list
    # does not infer contracts of similarly named helper functions.
    for variant in ("TriggeringObjectWasEnchanted", "EvolveEnteringCreatureIsLarger", "TriggeringObjectHadCounters"):
        line = next((i+1 for i, text in enumerate(lines)
                     if re.match(r"        Condition::" + variant + r"\b", text)), None)
        if line:
            result[variant] = {"variant_line": line,
                "reason": "Direct triggering_event/snapshot optional access; absent event makes the predicate false."}
    return result


def references(condition, path, variants):
    if isinstance(condition, str):
        if condition in variants:
            yield {"variant": condition, "path": path}
    elif isinstance(condition, dict) and len(condition) == 1:
        name, child = next(iter(condition.items()))
        if name in variants:
            yield {"variant": name, "path": path + "." + name}
        elif name == "Not":
            yield from references(child, path + ".Not", variants)
        elif name in ("And", "Or") and isinstance(child, list):
            for i, node in enumerate(child):
                yield from references(node, path + f".{name}[{i}]", variants)


def screen(definition, variants):
    result = []
    for index, ability in enumerate(definition.get("abilities", [])):
        model = ability.get("kind", {}).get("Static")
        if not isinstance(model, dict):
            continue
        for path, condition, payload in static_roots(model, f"abilities[{index}].kind.Static"):
            refs = list(references(condition, path, variants))
            if refs:
                result.append({"static_index": index, "static_id": model.get("id"),
                    "functional_zones": ability.get("functional_zones"), "payload_variant": payload,
                    "condition_path": path, "condition": condition, "context_references": refs,
                    "classification": "static_context_routing_candidate", "execution_confirmed": False})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, default=ROOT / "corpus/results.sqlite3")
    parser.add_argument("--run-id", default="267a16aff3b321196397d0b4")
    parser.add_argument("--output", type=Path, default=ROOT / "static-execution-condition-candidates.json")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        variants = {"TargetMatches", "EnchantedPermanentAttackedThisTurn"}
        condition = {"And": [{"Not": {"TargetMatches": {}}}, "EnchantedPermanentAttackedThisTurn"]}
        body = {"condition": condition}
        model = {"id": "Anthem", "payload": {"Anthem": body}}
        assert len(list(references(condition, "$", variants))) == 2
        assert len(screen({"abilities": [{"kind": {"Static": model}}]}, variants)) == 1
        assert screen({"abilities": [{"kind": {"Triggered": body}}]}, variants) == []
        assert screen({"abilities": [{"kind": {"Static": {"payload": {"GrantObjectAbilityForFilter": {"ability": model}}}}}]}, variants) == []
        assert list(references({"CountComparison": {"nested": "TargetMatches"}}, "$", variants)) == []
        assert source_guards("        Condition::TargetIsTapped => {\n            let Some(ctx) = ctx.execution() else {\n                return Ok(false);\n            };\n")['TargetIsTapped']['guard_line'] == 2
        print("Static execution condition scope checks passed.")
        return
    condition_bytes = CONDITIONS.read_bytes()
    static_bytes = STATIC_CONTEXT.read_bytes()
    guards = source_guards(condition_bytes.decode())
    pattern = re.compile(r'"(?:' + '|'.join(map(re.escape, guards)) + r')"')
    rows = []
    counts = {"records_screened": 0, "serialized_reference_records": 0, "decoded_definitions": 0}
    with sqlite3.connect(args.database.resolve().as_uri() + "?mode=ro", uri=True) as connection:
        connection.execute("PRAGMA query_only=ON")
        for name, raw in connection.execute("SELECT card_name,result_json FROM result WHERE run_id=?", (args.run_id,)):
            counts['records_screened'] += 1
            if not pattern.search(raw):
                continue
            counts['serialized_reference_records'] += 1
            report = json.loads(raw)
            definition = report.get('definition')
            if not isinstance(definition, dict):
                continue
            counts['decoded_definitions'] += 1
            for row in screen(definition, guards):
                rows.append({"card": name, "artifact_checksum": report.get('artifact_checksum'),
                    "result_json_sha256": hashlib.sha256(raw.encode()).hexdigest(), **row})
    out = {"generated_at": datetime.now(timezone.utc).isoformat(), **counts,
        "candidate_names": len({r['card'] for r in rows}), "candidate_paths": len(rows), "rows": rows,
        "scope": "Every frozen record is screened for exact JSON variant names from the current-source execution guards and three directly inspected event predicates. Matching authored top-level static-condition roots are typed-walked through Boolean combinators and static Conditional wrappers. This is a context-routing candidate inventory, not a defect classification.",
        "source_guard_inventory": guards,
        "confirmed_cards_by_this_screen": [], "all_cards_verified": False,
        "provenance": {"database": str(args.database), "run_id": args.run_id,
            "generator_sha256": sha(Path(__file__)),
            "scope_helper_sha256": sha(BASE / 'scripts/audit_runtime_static_target_conditions.py'),
            "condition_source_sha256": hashlib.sha256(condition_bytes).hexdigest(),
            "static_context_source_sha256": hashlib.sha256(static_bytes).hexdigest(),
            "source_files_unchanged_during_scan": condition_bytes == CONDITIONS.read_bytes() and static_bytes == STATIC_CONTEXT.read_bytes()},
        "limitations": ["An execution guard may have an earlier external-context branch. Each candidate requires exact source routing review.",
            "Static replacement or cost abilities can use execution contexts; a matching serialized condition alone is not proof of a missing binding.",
            "The screen excludes nested granted abilities, resolution effects, filter/value context references and unavailable definitions.",
            "Current source hashes and frozen compiler artifacts identify distinct versions; no runtime equivalence is inferred."]}
    args.output.write_text(json.dumps(out, indent=2) + '\n')
    print(json.dumps({k: out[k] for k in (*counts, 'candidate_names', 'candidate_paths')}))


if __name__ == '__main__':
    main()
