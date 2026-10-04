#!/usr/bin/env python3
"""Read-only typed damage-source binding scan over a frozen all-card campaign."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3

DAMAGE = {"DealDamageEffect", "DealDistributedDamageEffect"}

def base_spec(spec):
    while isinstance(spec, dict) and "SurfaceHinted" in spec:
        spec = spec["SurfaceHinted"]["spec"]
    return spec

def source_kind(spec):
    spec = base_spec(spec)
    if spec == "Source":
        return "enclosing_source"
    if isinstance(spec, dict) and "Tagged" in spec:
        return "tagged_object"
    if isinstance(spec, str):
        return "event_reference" if "Trigger" in spec or "Event" in spec else spec.lower()
    if isinstance(spec, dict) and any(tagged_filter_references(spec)):
        return "tagged_filter_source"
    return "other_object_spec"

def tagged_filter_references(node):
    if isinstance(node, dict):
        for constraint in node.get("tagged_constraints", []):
            if constraint.get("tag"):
                yield constraint["tag"]
        for key, child in node.items():
            if key not in {"hints", "union_surface"}:
                yield from tagged_filter_references(child)
    elif isinstance(node, list):
        for child in node:
            yield from tagged_filter_references(child)

def event_kinds(node):
    if isinstance(node, dict):
        if "ZoneChange" in node:
            z = node["ZoneChange"]
            if z.get("to") == "Battlefield":
                yield "self_etb" if z.get("this") else "other_or_any_etb"
        for k, value in node.items():
            if k not in {"filter", "this_surface", "intro_surface"}:
                yield from event_kinds(value)
    elif isinstance(node, list):
        for value in node:
            yield from event_kinds(value)

def tagged_values(node):
    if isinstance(node, dict):
        if "Tagged" in node:
            yield node["Tagged"]
        for key, value in node.items():
            if key not in {"hints", "union_surface"}:
                yield from tagged_values(value)
    elif isinstance(node, list):
        for value in node:
            yield from tagged_values(value)

def inspect(node, path="definition", ability="spell_or_outer", ability_path=None, trigger=None, binding="Source", binding_path=None):
    if isinstance(node, list):
        for i, value in enumerate(node):
            yield from inspect(value, f"{path}[{i}]", ability, ability_path, trigger, binding, binding_path)
    elif isinstance(node, dict):
        kind = node.get("kind")
        if isinstance(kind, dict) and any(k in kind for k in ("Triggered", "Activated", "Static")):
            for label, body in kind.items():
                yield from inspect(body, f"{path}.kind.{label}", label, path, body.get("trigger") if isinstance(body, dict) else None, "Source", None)
            return
        if kind == "ExecuteWithSourceEffect":
            explicit = node["payload"]["source"]
            if base_spec(explicit) != "Source":
                binding, binding_path = explicit, path
        if isinstance(kind, str) and kind in DAMAGE:
            payload = node["payload"]
            explicit = payload.get("source", "Source")
            effective = binding if base_spec(explicit) == "Source" else explicit
            effective_path = binding_path if base_spec(explicit) == "Source" else path
            yield {"effect_path": path, "effect_kind": kind, "ability_kind": ability,
                   "ability_path": ability_path, "trigger": trigger,
                   "etb_contexts": sorted(set(event_kinds(trigger))),
                   "authored_source": explicit, "effective_source": effective,
                   "effective_source_kind": source_kind(effective), "source_binding_path": effective_path,
                   "source_filter_tags": sorted(set(tagged_filter_references(effective))),
                   "amount": payload.get("amount"),
                   "amount_tag_references": sorted(set(tagged_values(payload.get("amount")))),
                   "status": "typed_candidate_not_executed"}
        for key, value in node.items():
            if key == "flattened_default_effects" and "segments" in node:
                continue
            if key in {"card", "hints", "source_text", "oracle_text", "union_surface"}:
                continue
            yield from inspect(value, f"{path}.{key}", ability, ability_path, trigger, binding, binding_path)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    inventory_raw = (args.run / "inventory.json").read_bytes()
    cards = {r["name"]: r for r in json.loads(inventory_raw)["cards"]}
    rows = []
    with sqlite3.connect(f"file:{args.run.parent / 'results.sqlite3'}?mode=ro", uri=True) as db:
        statuses = dict(db.execute("select status,count(*) from result where run_id=? group by status", (args.run.name,)))
        # instr is only an optimization: every selected definition is traversed by typed kind.
        query = "select card_name,result_json from result where run_id=? and status='compiled' and (instr(result_json,'DealDistributedDamageEffect')>0 or instr(result_json,'DealDamageEffect')>0) order by card_name"
        for name, raw in db.execute(query, (args.run.name,)):
            result = json.loads(raw)
            definition = result.get("definition")
            if definition is None:
                rows.append({"card": name, "status": "definition_unavailable"})
                continue
            for row in inspect(definition):
                row.update(card=name, oracle_text=cards[name]["oracle_text"], artifact_checksum=result.get("artifact_checksum"))
                rows.append(row)
    kinds = Counter(r.get("effective_source_kind", "unavailable") for r in rows)
    candidates = [r for r in rows if r.get("effective_source_kind") in {"tagged_object", "tagged_filter_source", "event_reference"}]
    result = {"scope": "Full frozen typed scan of DealDamageEffect and DealDistributedDamageEffect. Track enclosing ExecuteWithSourceEffect binding separately from explicit Source and tagged/event references; preserve enclosing triggered/activated context and self/other ETB matcher classification.",
              "inventory_payloads": len(cards), "frozen_status_counts": statuses,
              "damage_effects": len(rows), "source_kind_counts": dict(kinds),
              "tagged_or_event_source_effects": len(candidates),
              "tagged_or_event_source_card_count": len({r["card"] for r in candidates}),
              "tagged_or_event_source_cards": sorted({r["card"] for r in candidates}),
              "rows": rows,
              "provenance": {"run": str(args.run.resolve()), "inventory_sha256": hashlib.sha256(inventory_raw).hexdigest(),
                             "scanner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                             "manifest": json.loads((args.run / "manifest.json").read_text())},
              "limitations": "Candidate inventory, not failure evidence. Source may refer to a different object than the ability owner; damage amounts may have separate LKI dependencies. Nested granted abilities are recorded without proving reachability. Flattened duplicates and presentation-only fields are skipped. Other damage effect kinds and implicit runtime source rebinding are outside this scan."}
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: result[k] for k in ["inventory_payloads", "damage_effects", "source_kind_counts", "tagged_or_event_source_effects", "tagged_or_event_source_card_count"]}))

if __name__ == "__main__":
    main()
