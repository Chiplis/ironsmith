#!/usr/bin/env python3
"""Supplemental source-face inventory and frozen CLI diagnostics, never a success gate.

The baseline name loader is first-wins. This script records its exact selected
source context and reports shadowed routes as omissions. It does not duplicate
the compiler or treat compare-text output as strict/non-lossy compilation.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

LINKED_LAYOUTS = {"transform", "split", "flip"}
FORMATS = ("commander", "standard", "modern", "pioneer", "legacy", "vintage")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    with Path(path).open("rb") as f:
        h = hashlib.sha256()
        for block in iter(lambda: f.read(1024 * 1024), b""):
            h.update(block)
        return h.hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n")


def normalize(name):
    name = name.strip()
    return name.replace(" / ", " // ", 1) if " // " not in name else name


def postprocess(text):
    """Mirror public postprocess_oracle_text solely to verify CLI text identity."""
    lines = []
    for line in text.splitlines():
        out, depth = "", 0
        for char in line:
            if char == "(":
                if not depth:
                    out = out.rstrip(" \t")
                depth += 1
            elif char == ")":
                depth = max(0, depth - 1)
            elif not depth:
                out += char
        out = re.sub(r" ([.,;:!?])", r"\1", " ".join(out.split())).strip()
        if out:
            lines.append(out)
    return "\n".join(lines)


def is_legal(card):
    legalities = card.get("legalities") or {}
    return not legalities or any(legalities.get(f) == "legal" for f in FORMATS)


def route_id(index, face):
    # The source digest scopes these stable indexes; names/Oracle IDs can repeat.
    return f"source:{index}/face:{face}"


def selected_payload(card, index, face_index):
    face = card.get("card_faces", [])[face_index] if face_index is not None else (card.get("card_faces") or [{}])[0]
    if face_index is None:
        name = normalize(card.get("name", face.get("name", "")))
        raw = card.get("oracle_text")
        if raw is None:
            raw = face.get("oracle_text", "")
    else:
        name = face["name"].strip()
        raw = face.get("oracle_text", "")
    return {"source_index": index, "face_index": face_index,
            "route_id": route_id(index, face_index) if face_index is not None else None,
            "source_name": card.get("name"), "source_id": card.get("id"),
            "name": name, "oracle_text": postprocess(raw or ""),
            "oracle_id": face.get("oracle_id") or card.get("oracle_id")}


def make_inventory(cards, source_sha256):
    if not isinstance(cards, list) or any(not isinstance(c, dict) for c in cards):
        raise ValueError("source must be a Scryfall JSON array")
    routes = []
    for index, card in enumerate(cards):
        for face_index, face in enumerate(card.get("card_faces") or []):
            name = face.get("name")
            if not isinstance(name, str) or not name.strip():
                raise ValueError(f"missing face name at source {index}, face {face_index}")
            routes.append({"route_id": route_id(index, face_index), "source_index": index,
                           "source_name": card.get("name"), "source_id": card.get("id"),
                           "source_layout": card.get("layout"), "face_index": face_index,
                           "face_name": name.strip(), "lookup_name": normalize(name),
                           "raw_oracle_text": (face.get("oracle_text") or "").strip(),
                           "oracle_id": face.get("oracle_id") or card.get("oracle_id"),
                           "top_level_oracle_id": card.get("oracle_id"),
                           "digital": card.get("digital") is True})
    requested = {r["lookup_name"] for r in routes}
    selected = {}
    # Exact first-wins traversal in load_card_payloads_by_names. Legal checks
    # apply only to canonical-record construction, not explicit face matches.
    for index, card in enumerate(cards):
        if card.get("digital") is True:
            continue
        name = normalize(card.get("name", ""))
        faces = card.get("card_faces") or []
        if name in requested and name not in selected:
            if card.get("layout") in LINKED_LAYOUTS and faces:
                selected[name] = [selected_payload(card, index, j) for j, f in enumerate(faces) if f.get("name", "").strip()]
            elif name and is_legal(card):
                selected[name] = [selected_payload(card, index, None)]
        for j, face in enumerate(faces):
            name = normalize(face.get("name", ""))
            if name in requested and name not in selected:
                selected[name] = [selected_payload(card, index, j)]
    for route in routes:
        choices = selected.get(route["lookup_name"], [])
        route["selected_route_ids"] = [x["route_id"] for x in choices]
        route["lookup_disposition"] = ("exact_face" if route["route_id"] in route["selected_route_ids"] else
                                       "digital_omitted" if route["digital"] else
                                       "name_shadowed" if choices else "missing_payload")
    omitted = [r for r in routes if r["lookup_disposition"] != "exact_face"]
    return {"schema_version": 1, "source_sha256": source_sha256,
            "scope": "Supplemental exact source-face routes, separate from canonical baseline.",
            "summary": {
                "source_entry_count": len(cards),
                "unique_source_oracle_card_count": len({c["oracle_id"] for c in cards if c.get("oracle_id")}),
                "multiface_source_entry_count": len({r["source_index"] for r in routes}),
                "multiface_unique_oracle_card_count": len({r["oracle_id"] for r in routes if r["oracle_id"]}),
                "face_route_count": len(routes), "distinct_face_lookup_name_count": len(requested),
                "selected_lookup_query_count": len(selected),
                "selected_payload_count": sum(map(len, selected.values())),
                "exact_face_route_count": len(routes) - len(omitted),
                "omitted_face_route_count": len(omitted),
                "omitted_routes_by_layout": dict(sorted(Counter(r["source_layout"] for r in omitted).items())),
                "selected_canonical_payload_count": sum(p["face_index"] is None for ps in selected.values() for p in ps),
                "product_baker_linked_face_route_count": sum(r["source_layout"] in {"transform", "modal_dfc", "adventure", "prepare", "flip", "split"} for r in routes),
                "face_routes_by_layout": dict(sorted(Counter(r["source_layout"] for r in routes).items()))},
            "queries": [{"name": name, "selected_payloads": selected.get(name, [])} for name in sorted(requested)],
            "routes": routes}


def load_inventory_source(path):
    data = Path(path).read_bytes()
    return make_inventory(json.loads(data), digest(data))


def parse_stdout(text):
    records = []
    chunks = re.split(r"(?m)^Name: ", text)
    if chunks[0].strip():
        raise ValueError("unexpected stdout preamble")
    for chunk in chunks[1:]:
        name, _, body = chunk.partition("\n")
        match = re.fullmatch(r"Similarity: ([^\n]+)\nSemantic mismatch: (true|false)\nOriginal oracle text:\n(.*?)\nCompiled oracle text:\n(.*)", body.rstrip("\n"), re.S)
        if not match:
            raise ValueError(f"malformed compare-text output for {name}")
        score = float(match[1])
        if not 0 <= score <= 1:
            raise ValueError(f"invalid similarity for {name}")
        records.append({"name": name, "similarity_score": score, "semantic_mismatch": match[2] == "true",
                        "oracle_text": match[3], "compiled_text": match[4]})
    return records


def parse_stderr(text):
    rows = {}
    for match in re.finditer(r"(?m)^Name: ([^\n]+)\nError: ([^\n]+)$", text):
        if match[1] in rows:
            raise ValueError(f"duplicate error query {match[1]}")
        rows[match[1]] = match[2]
    return rows


def analyze(inventory, stdout, stderr):
    outputs = parse_stdout(stdout)
    errors = parse_stderr(stderr)
    queries = {q["name"]: q for q in inventory["queries"]}
    if set(errors) - set(queries):
        raise ValueError("stderr contains unexpected names")
    by_name = {}
    for row in outputs:
        by_name.setdefault(row["name"], []).append(row)
    observations = {}
    query_rows = []
    for name, query in queries.items():
        payloads = query["selected_payloads"]
        if not payloads:
            raise ValueError(f"cannot attribute CLI fallback for missing lookup {name}")
        if len(payloads) != 1:
            raise ValueError(f"multi-payload query cannot be independently attributed: {name}")
        selected = payloads[0]
        matches = by_name.pop(selected["name"], [])
        if name in errors:
            if matches:
                raise ValueError(f"query produced both output and error: {name}")
            error = errors[name]
            if not error.startswith(f"parse failed for {selected['name']}: "):
                raise ValueError(f"unattributed compilation error for {name}: {error}")
            row = {"status": "failed", "error": error}
        elif len(matches) == 1:
            row = matches[0]
            if row["oracle_text"] != selected["oracle_text"]:
                raise ValueError(f"selected Oracle text differs for {name}")
            row = {**row, "status": "cli_compiled_unverified"}
        else:
            raise ValueError(f"expected exactly one outcome for {name}, found {len(matches)}")
        record = {"query_name": name, "selected_payload": selected, **row,
                  "strict_nonlossy_supported": None}
        query_rows.append(record)
        if selected["route_id"] is not None:
            observations[selected["route_id"]] = record
    if by_name:
        raise ValueError("stdout contains unexpected names")
    route_rows = []
    for route in inventory["routes"]:
        observation = observations.get(route["route_id"])
        route_rows.append({**route, "outcome": observation or {"status": "omitted", "reason": route["lookup_disposition"]},
                           "strict_nonlossy_supported": None})
    failed = [r for r in route_rows if r["outcome"]["status"] == "failed"]
    return {"schema_version": 1, "source_sha256": inventory["source_sha256"],
            "evidence_kind": "frozen_cli_diagnostic_only",
            "limitations": ["Resolver membership is source-derived and selected name/Oracle text checked against CLI output; error paths expose name only.",
                            "compare-text does not expose strict/permissive status, parse loss, or unsupported markers; output is not success.",
                            "Name collisions omit exact source-face contexts; no omitted route inherits another route's outcome.",
                            "Gameplay and artifact-baker behavior have not been tested by this diagnostic run."],
            "all_face_routes_supported": False,
            "summary": {**inventory["summary"], "lookup_outcome_counts": dict(Counter(q["status"] for q in query_rows)),
                        "face_route_outcome_counts": dict(Counter(r["outcome"]["status"] for r in route_rows)),
                        "failed_face_route_count": len(failed),
                        "unique_failed_source_entry_count": len({r["source_index"] for r in failed}),
                        "unique_failed_oracle_card_count": len({r["oracle_id"] for r in failed if r["oracle_id"]}),
                        "strict_supported_face_route_count": None},
            "queries": query_rows, "routes": route_rows}


def run_cli(args):
    out = args.out_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inventory = load_inventory_source(args.cards)
    write_json(out / "inventory.json", inventory)
    names = out / "face-names.txt"
    names.write_text("".join(q["name"] + "\n" for q in inventory["queries"]))
    binary = args.compile_bin.resolve()
    binary_hash = file_digest(binary)
    manifest = json.loads(args.build_manifest.read_text())
    bound = manifest.get("binary_sha256") == binary_hash or manifest.get("binaries", {}).get(binary.name, {}).get("sha256") == binary_hash
    if not bound:
        raise ValueError("compiler binary digest is not bound to the supplied build manifest")
    command = [str(binary), "--names", str(names), "--cards", str(args.cards.resolve()), "--compare-text", "--continue-on-error"]
    env = {k: v for k, v in os.environ.items() if not k.startswith("IRONSMITH_")}
    env["RAYON_NUM_THREADS"] = "1"
    record = {"schema_version": 1, "started_at": datetime.now(timezone.utc).isoformat(),
              "command": command, "source_sha256": inventory["source_sha256"],
              "binary_sha256": binary_hash, "names_sha256": file_digest(names),
              "build_manifest": manifest, "binary_bound_to_build_manifest": bound,
              "environment_policy": "clear inherited IRONSMITH_*; RAYON_NUM_THREADS=1; empty stdin",
              "harness_sha256": file_digest(__file__), "inventory_sha256": file_digest(out / "inventory.json"), "complete": False}
    (out / "harness.py").write_bytes(Path(__file__).read_bytes())
    write_json(out / "run.json", record)
    with (out / "stdout.log").open("w") as stdout, (out / "stderr.log").open("w") as stderr:
        result = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, env=env, check=False)
    record.update({"returncode": result.returncode, "finished_at": datetime.now(timezone.utc).isoformat(),
                   "stdout_sha256": file_digest(out / "stdout.log"), "stderr_sha256": file_digest(out / "stderr.log")})
    try:
        if result.returncode:
            raise ValueError(f"compiler exited {result.returncode}")
        if file_digest(binary) != binary_hash or file_digest(args.cards) != inventory["source_sha256"] or file_digest(names) != record["names_sha256"]:
            raise ValueError("input or binary changed during audit")
        report = analyze(inventory, (out / "stdout.log").read_text(), (out / "stderr.log").read_text())
        write_json(out / "diagnostics.json", report)
        record["complete"] = True
        print(json.dumps(report["summary"], indent=2))
    except Exception as error:
        record["error"] = str(error)
        raise
    finally:
        write_json(out / "run.json", record)



def supported(snapshot):
    return (snapshot["parse_status"] == "strict_compiled" and not snapshot["parse_error"]
            and not snapshot["has_unimplemented"] and not snapshot["parse_lossy"]
            and snapshot["parse_loss_count"] == 0 and not snapshot["parse_loss_reasons"]
            and snapshot["compiled_text"] is not None and snapshot["compiled_card_definition"] is not None)


def analyze_exact(inventory, records):
    records = iter(records)
    header = next(records, {})
    if header.get("kind") != "header" or header.get("schema_version") != 1:
        raise ValueError("missing exact audit header")
    if header.get("source_sha256") != inventory["source_sha256"]:
        raise ValueError("exact audit source digest differs")
    expected = {r["route_id"]: r for r in inventory["routes"]}
    if header.get("face_route_count") != len(expected):
        raise ValueError("header face count differs")
    rows, footer = {}, None
    snapshot_fields = {"card_name", "oracle_text", "raw_oracle_text", "parse_status", "parse_error",
                       "normalized_oracle_text", "compiled_text", "compiled_card_definition",
                       "compiled_card_definition_sha256", "oracle_coverage", "compiled_coverage",
                       "similarity_score", "line_delta", "semantic_mismatch", "has_unimplemented",
                       "parse_lossy", "parse_loss_reasons", "parse_loss_count", "content_hash"}
    for record in records:
        if footer is not None:
            raise ValueError("records follow completion footer")
        if record.get("kind") == "complete":
            footer = record
            continue
        key = record.get("route_id")
        if record.get("kind") != "face" or key not in expected or key in rows:
            raise ValueError(f"unexpected or duplicate exact face route {key}")
        route = expected[key]
        for field in ("source_index", "source_name", "source_id", "source_layout", "face_index", "face_name", "oracle_id", "top_level_oracle_id"):
            if record.get(field) != route[field]:
                raise ValueError(f"exact face identity differs: {key} {field}")
        payload = record.get("payload") or {}
        if payload.get("name") != route["face_name"] or payload.get("raw_oracle_text") != route["raw_oracle_text"]:
            raise ValueError(f"exact payload identity differs: {key}")
        if payload.get("parse_name") is not None:
            raise ValueError(f"unexpected face parse name: {key}")
        snapshot = record.get("snapshot")
        if header.get("inventory_only") is True:
            if snapshot is not None:
                raise ValueError("inventory-only audit contains compilation result")
        else:
            if not isinstance(snapshot, dict) or snapshot_fields - snapshot.keys():
                raise ValueError(f"missing strict snapshot fields: {key}")
            if snapshot["card_name"] != payload["name"] or snapshot["raw_oracle_text"] != payload["raw_oracle_text"]:
                raise ValueError(f"snapshot identity differs: {key}")
            if snapshot["parse_status"] not in {"strict_compiled", "compiled_with_allow_unsupported", "parse_failed"}:
                raise ValueError(f"unknown snapshot status: {key}")
            definition = snapshot["compiled_card_definition"]
            if snapshot["compiled_card_definition_sha256"] != (digest(definition.encode()) if definition is not None else None):
                raise ValueError(f"definition digest differs: {key}")
            for field in ("has_unimplemented", "parse_lossy", "semantic_mismatch"):
                if type(snapshot[field]) is not bool:
                    raise ValueError(f"invalid boolean {field}: {key}")
        rows[key] = record
    if footer is None or footer.get("face_route_count") != len(expected) or footer.get("source_sha256") != inventory["source_sha256"] or set(rows) != set(expected):
        raise ValueError("missing/incomplete exact face coverage")
    compiled = header.get("inventory_only") is False
    failing = [r for r in rows.values() if compiled and not supported(r["snapshot"])]
    return {"schema_version": 1, "source_sha256": inventory["source_sha256"],
            "evidence_kind": "exact_authoritative_snapshots" if compiled else "exact_payload_inventory_only",
            "all_face_routes_supported": compiled and not failing,
            "gameplay_verified": False, "artifact_baker_verified": False,
            "summary": {"face_route_count": len(rows), "omitted_face_route_count": 0,
                        "strict_supported_face_route_count": len(rows) - len(failing) if compiled else None,
                        "failed_face_route_count": len(failing) if compiled else None,
                        "unique_failed_source_entry_count": len({r["source_index"] for r in failing}) if compiled else None,
                        "unique_failed_oracle_card_count": len({r["oracle_id"] for r in failing if r["oracle_id"]}) if compiled else None},
            "routes": list(rows.values())}


def compare_exact(baseline, current):
    if baseline.get("evidence_kind") != "exact_authoritative_snapshots" or current.get("evidence_kind") != "exact_authoritative_snapshots":
        raise ValueError("only exact authoritative face snapshots can be compared")
    if baseline["source_sha256"] != current["source_sha256"]:
        raise ValueError("face audit source changed")
    before = {r["route_id"]: r for r in baseline["routes"]}
    after = {r["route_id"]: r for r in current["routes"]}
    if len(before) != len(baseline["routes"]) or len(after) != len(current["routes"]) or set(before) != set(after):
        raise ValueError("face route membership changed")
    unresolved, regressions, changed = [], [], []
    for key, old in before.items():
        new = after[key]
        if old["payload"] != new["payload"]:
            raise ValueError(f"face payload context changed: {key}; review route changes explicitly")
        a, b = old["snapshot"], new["snapshot"]
        if not supported(b):
            unresolved.append(key)
        reasons = []
        if supported(a):
            if not supported(b):
                reasons.append("support_regression")
            if not a["semantic_mismatch"] and b["semantic_mismatch"]:
                reasons.append("new_semantic_mismatch")
            if b["similarity_score"] + 1e-6 < a["similarity_score"]:
                reasons.append("similarity_score_decreased")
            if a["compiled_card_definition_sha256"] != b["compiled_card_definition_sha256"]:
                changed.append(key)
        if reasons:
            regressions.append({"route_id": key, "reasons": reasons})
    return {"source_sha256": baseline["source_sha256"], "face_compile_gate_complete": not unresolved and not regressions,
            "unresolved_route_ids": unresolved, "regressions": regressions, "changed_supported_definition_routes": changed,
            "gameplay_verified": False, "artifact_baker_verified": False}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    plan = sub.add_parser("inventory")
    plan.add_argument("--cards", required=True, type=Path)
    plan.add_argument("--out", required=True, type=Path)
    run = sub.add_parser("run-cli")
    run.add_argument("--cards", required=True, type=Path)
    run.add_argument("--compile-bin", required=True, type=Path)
    run.add_argument("--build-manifest", required=True, type=Path)
    run.add_argument("--out-dir", required=True, type=Path)
    analysis = sub.add_parser("analyze-cli")
    analysis.add_argument("--run-dir", required=True, type=Path)
    exact = sub.add_parser("analyze-exact")
    exact.add_argument("--cards", required=True, type=Path)
    exact.add_argument("--jsonl", required=True, type=Path)
    exact.add_argument("--out", required=True, type=Path)
    compare = sub.add_parser("compare-exact")
    compare.add_argument("--baseline", required=True, type=Path)
    compare.add_argument("--current", required=True, type=Path)
    compare.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    try:
        if args.command == "inventory":
            inventory = load_inventory_source(args.cards)
            write_json(args.out, inventory)
            print(json.dumps(inventory["summary"], indent=2))
        elif args.command == "run-cli":
            run_cli(args)
        elif args.command == "analyze-exact":
            inventory = load_inventory_source(args.cards)
            with args.jsonl.open() as stream:
                report = analyze_exact(inventory, (json.loads(line) for line in stream))
            report["jsonl_sha256"] = file_digest(args.jsonl)
            write_json(args.out, report)
            print(json.dumps(report["summary"], indent=2))
        elif args.command == "compare-exact":
            report = compare_exact(json.loads(args.baseline.read_text()), json.loads(args.current.read_text()))
            write_json(args.out, report)
            return 0 if report["face_compile_gate_complete"] else 1
        else:
            path = args.run_dir
            record = json.loads((path / "run.json").read_text())
            if record.get("returncode") != 0:
                raise ValueError("run has no successful process exit")
            for field, filename in [("stdout_sha256", "stdout.log"), ("stderr_sha256", "stderr.log"), ("names_sha256", "face-names.txt"), ("inventory_sha256", "inventory.json")]:
                if file_digest(path / filename) != record[field]:
                    raise ValueError(f"changed {filename}")
            inventory = json.loads((path / "inventory.json").read_text())
            if inventory["source_sha256"] != record["source_sha256"]:
                raise ValueError("inventory source digest changed")
            report = analyze(inventory, (path / "stdout.log").read_text(), (path / "stderr.log").read_text())
            write_json(path / "diagnostics.json", report)
            print(json.dumps(report["summary"], indent=2))
    except (ValueError, OSError, KeyError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
