#!/usr/bin/env python3
"""Find same-sequence target-declaration tag overwrites with later typed consumers.

Read-only structural screen of a pinned full corpus. Findings are candidates,
not proof that the earlier target set should survive or that a cast is legal.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3

# These wrappers preserve execution order. Conditional/loop bodies are scanned
# as separate lexical streams; no inference about branch execution is made.
TRANSPARENT_CHILDREN = {
    "SequenceEffect": ("effects",),
    "TaggedEffect": ("effect",),
}
AGGREGATE_KINDS = {
    "ForEachObject", "ForEachTagged", "PutCountersEffect", "RemoveCountersEffect",
    "MoveToZoneEffect", "ReturnToHandEffect", "ReturnFromGraveyardToHandEffect",
    "DestroyEffect", "ExileEffect", "TapEffect", "UntapEffect", "DealDamageEffect",
}
IGNORED_SCRATCH_TAGS = {"__it__", "it", "previous", "__previous__"}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def children(value, path):
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "flattened_default_effects" and "segments" in value:
                continue
            yield child, f"{path}/{key}"
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield child, f"{path}/{index}"


def direct_tag_reads(value):
    """Typed spec/filter references, excluding nested effect bodies/writes."""
    if isinstance(value, dict):
        if isinstance(value.get("kind"), str) and "payload" in value:
            return set()
        out = set()
        for key, child in value.items():
            if key in {"Tagged", "TaggedObjects", "TaggedObject", "tag"} and isinstance(child, str):
                out.add(child)
            else:
                out |= direct_tag_reads(child)
        return out
    if isinstance(value, list):
        return set().union(*(direct_tag_reads(v) for v in value)) if value else set()
    return set()


def lexical_streams(value, path="/definition"):
    """Yield one default program or standalone effect-list execution stream.

    Segmented defaults are one ordered stream. Each self-replacement and each
    nested nontransparent body is independent, preventing alternate branches or
    different abilities from masquerading as repeated writes.
    """
    if isinstance(value, dict) and isinstance(value.get("segments"), list):
        effects = []
        alternatives = []
        for index, segment in enumerate(value["segments"]):
            p = f"{path}/segments/{index}"
            effects.extend((v, f"{p}/default_effects/{i}") for i, v in enumerate(segment.get("default_effects", [])))
            alternatives.extend((v, f"{p}/self_replacements/{i}") for i, v in enumerate(segment.get("self_replacements", [])))
        yield path, effects
        for child, p in alternatives:
            yield from lexical_streams(child, p)
        return
    if isinstance(value, list) and value and all(isinstance(v, dict) and isinstance(v.get("kind"), str) and "payload" in v for v in value):
        yield path, [(v, f"{path}/{i}") for i, v in enumerate(value)]
        return
    for child, p in children(value, path):
        yield from lexical_streams(child, p)


def stream_events(effects):
    events, nested = [], []
    def visit(effect, path):
        kind = effect.get("kind")
        payload = effect.get("payload", {})
        if not isinstance(payload, dict):
            return
        if kind == "TaggedEffect":
            inner = payload.get("effect", {})
            tag = payload.get("tag")
            if isinstance(tag, str) and isinstance(inner, dict) and inner.get("kind") == "TargetOnlyEffect":
                events.append({"type": "target_write", "path": path, "tag": tag,
                               "target": inner.get("payload", {}).get("target"),
                               "explicit_declaration": inner.get("payload", {}).get("explicit_declaration", False)})
                return
        reads = direct_tag_reads(payload)
        if kind in AGGREGATE_KINDS and reads:
            events.append({"type": "consumer", "path": path, "kind": kind, "reads": sorted(reads)})
        transparent = set(TRANSPARENT_CHILDREN.get(kind, ()))
        for key, child in payload.items():
            p = f"{path}/payload/{key}"
            if key in transparent:
                if isinstance(child, list):
                    for i, e in enumerate(child):
                        if isinstance(e, dict):
                            visit(e, f"{p}/{i}")
                elif isinstance(child, dict):
                    visit(child, p)
            else:
                nested.extend(lexical_streams(child, p))
    for effect, path in effects:
        visit(effect, path)
    return events, nested


def scan_definition(definition):
    pending = list(lexical_streams(definition))
    findings, scratch_ignored, streams = [], 0, 0
    while pending:
        stream_path, effects = pending.pop()
        streams += 1
        events, nested = stream_events(effects)
        pending.extend(nested)
        writes = {}
        for event in events:
            if event["type"] == "target_write":
                writes.setdefault(event["tag"], []).append(event)
            else:
                for tag in event["reads"]:
                    prior = writes.get(tag, [])
                    if len(prior) < 2:
                        continue
                    if tag in IGNORED_SCRATCH_TAGS:
                        scratch_ignored += 1
                        continue
                    findings.append({"stream": stream_path, "tag": tag, "writes": prior.copy(),
                                     "consumer": event, "classification": "structural_candidate"})
    return findings, {"lexical_streams": streams, "scratch_consumers_ignored": scratch_ignored}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, default=Path("reports/runtime-audit/actions/results.sqlite3"))
    parser.add_argument("--run-id", default="e17a4980b0b92c7a5a4cead2")
    parser.add_argument("--output", type=Path, default=Path("reports/runtime-audit/target-tag-overwrite-candidates.json"))
    args = parser.parse_args()
    run = args.database.parent / args.run_id
    inventory_path = run / "inventory.json"
    inventory = {r["name"]: r for r in json.loads(inventory_path.read_text())["cards"]}
    db = sqlite3.connect(f"file:{args.database.resolve()}?mode=ro", uri=True)
    statuses = dict(db.execute("SELECT status,count(*) FROM result WHERE run_id=? GROUP BY status", (args.run_id,)))
    rows, prefilt, streams, ignored = [], 0, 0, 0
    # Both strings are necessary typed shape labels; this only saves JSON parsing.
    query = "SELECT card_name,result_json FROM result WHERE run_id=? AND instr(result_json,'TargetOnlyEffect')>0 AND instr(result_json,'TaggedEffect')>0 ORDER BY card_name"
    for name, raw in db.execute(query, (args.run_id,)):
        result = json.loads(raw)
        if not result.get("definition"):
            continue
        prefilt += 1
        findings, stats = scan_definition(result["definition"])
        streams += stats["lexical_streams"]
        ignored += stats["scratch_consumers_ignored"]
        for finding in findings:
            rows.append({"card": name, "artifact_checksum": result.get("artifact_checksum"),
                         "oracle_text": inventory[name].get("oracle_text"),
                         "parse_input": inventory[name].get("parse_input"), **finding})
    report = {
        "scope": "Typed repeated TaggedEffect(TargetOnlyEffect) writes to one nonscratch tag within the same ordered lexical effect stream, followed by a typed aggregate/spec consumer. Duplicate flattened caches skipped; distinct abilities and conditional branches separated. Candidates only, no card promotion.",
        "records_scanned_by_necessary_shape_prefilter": sum(statuses.values()),
        "status_counts": statuses, "definition_candidates_parsed": prefilt,
        "lexical_streams_checked": streams, "ignored_scratch_tag_consumers": ignored,
        "candidate_paths": len(rows), "candidate_names": len({r['card'] for r in rows}),
        "confirmed_added_by_scan": 0,
        "limitations": ["Lexical ordering is not full dataflow or a reachability proof.",
                        "No branch/loop merging; cross-scope target-set losses may be missed.",
                        "Only direct TaggedEffect wrappers around TargetOnlyEffect are writes; other producers are outside scope.",
                        "Consumer whitelist includes potential plural specs; actual plurality and intended union require oracle review.",
                        "Repeated scratch tags are excluded, and an earlier target may intentionally cease to be referenced.",
                        "Names may include face/combined aliases. The pinned corpus is distinct from current concurrent source changes."],
        "provenance": {"database": str(args.database), "run_id": args.run_id,
                       "manifest": json.loads((run / "manifest.json").read_text()),
                       "inventory_sha256": digest(inventory_path), "scanner_sha256": digest(__file__)},
        "rows": rows,
    }
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: report[k] for k in ("records_scanned_by_necessary_shape_prefilter", "definition_candidates_parsed", "lexical_streams_checked", "candidate_paths", "candidate_names")}))


if __name__ == "__main__":
    main()
