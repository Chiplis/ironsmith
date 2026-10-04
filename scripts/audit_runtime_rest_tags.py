#!/usr/bin/env python3
"""Inventory typed literal-rest tag consumers and scope-local producer evidence."""
import hashlib
import json
import sqlite3
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "reports/runtime-audit"
RUNS = (("actions", "e17a4980b0b92c7a5a4cead2"), ("corpus", "267a16aff3b321196397d0b4"))
PRODUCERS = {"TaggedEffect", "ChooseObjectsEffect", "LookAtTopCardsEffect", "RevealTopCardsEffect", "RevealCardsEffect", "RevealHandEffect"}
PRESENTATION = {"hints", "union_surface", "source_text", "oracle_text"}


def inspect(definition):
    refs, producers, unknowns = [], [], []

    def walk(node, path="definition", scope="definition", effect=None, effect_path=None, branches=()):
        if isinstance(node, list):
            for index, child in enumerate(node):
                walk(child, f"{path}/{index}", scope, effect, effect_path, branches)
            return
        if not isinstance(node, dict):
            return
        # Every actual ability (including granted/token abilities) owns its tag
        # scope. Tags produced by a sibling ability cannot satisfy this scope.
        kind = node.get("kind")
        if isinstance(kind, dict) and set(kind) & {"Triggered", "Activated", "Static"}:
            scope = path
        if isinstance(kind, str) and "payload" in node:
            effect, effect_path = kind, path
            payload = node["payload"]
            if isinstance(payload, dict) and isinstance(payload.get("tag"), str):
                record = dict(tag=payload["tag"], effect_kind=kind, effect_path=path, scope=scope, branch_ancestry=list(branches))
                if kind in PRODUCERS or kind.startswith("Tag"):
                    producers.append(record)
                else:
                    unknowns.append(record)
        for key, child in node.items():
            if key in PRESENTATION or (key == "flattened_default_effects" and "segments" in node):
                continue
            at = f"{path}/{key}"
            next_scope = at if key == "spell_effect" else scope
            next_branches = branches + (at,) if key in {"if_true", "if_false", "modes", "alternatives", "failure", "payment"} else branches
            is_tag_constraint = key == "tag" and "relation" in node
            is_tag_reference = key in {"Tagged", "TaggedPlayer", "TaggedPower", "TaggedToughness"}
            if child == "rest" and (is_tag_constraint or is_tag_reference):
                refs.append(dict(reference_path=at, reference_kind="tagged_constraint" if is_tag_constraint else key,
                                 constraint_relation=node.get("relation"), effect_kind=effect,
                                 effect_path=effect_path, scope=scope, branch_ancestry=list(branches)))
            elif child == "rest" and not (key == "tag" and effect_path and path == effect_path + "/payload"):
                unknowns.append(dict(tag="rest", reference_path=at, scope=scope, classification="unclassified_typed_rest_field"))
            walk(child, at, next_scope, effect, effect_path, next_branches)
    walk(definition)
    return refs, producers, unknowns


def main():
    inventory_path = REPORT / "corpus/267a16aff3b321196397d0b4/inventory.json"
    inventory = {r["name"]: r for r in json.loads(inventory_path.read_text())["cards"]}
    summary_path = REPORT / "summary.json"
    prior = set(json.loads(summary_path.read_text())["confirmed_card_outcomes"]["confirmed_failure_cards"])
    rows, runs = [], []
    for family, run in RUNS:
        with sqlite3.connect(f"file:{REPORT / family / 'results.sqlite3'}?mode=ro", uri=True) as db:
            statuses = dict(db.execute("select status,count(*) from result where run_id=? group by status", (run,)))
            # Exact-string SQL prefilter only reduces I/O; all findings below
            # require typed tag nodes and actual containing effect/ability AST.
            query = "select card_name,result_json from result where run_id=? and status='compiled' and instr(result_json,?)>0 order by card_name"
            current = []
            for name, raw in db.execute(query, (run, '"rest"')):
                result = json.loads(raw)
                refs, producers, unknowns = inspect(result.get("definition", {}))
                grouped = defaultdict(list)
                for ref in refs:
                    grouped[(ref["scope"], ref["effect_path"])].append(ref)
                for (scope, effect_path), references in grouped.items():
                    local = [p for p in producers if p["scope"] == scope]
                    literal = [p for p in local if p["tag"] == "rest"]
                    row = dict(card=name, corpus=family, run_id=run, artifact_checksum=result.get("artifact_checksum"),
                               scope=scope, effect_path=effect_path, effect_kind=references[0]["effect_kind"], references=references,
                               scope_local_explicit_tag_producers=local, literal_rest_producers_in_scope=literal,
                               rest_producers_elsewhere=[p for p in producers if p["tag"] == "rest" and p["scope"] != scope],
                               unresolved_typed_fields=[p for p in unknowns if p["scope"] == scope],
                               dataflow_status="no_explicit_rest_producer_in_scope" if not literal else "producer_requires_order_and_branch_review",
                               classification="typed_candidate_not_runtime_confirmation", oracle_text=inventory[name]["oracle_text"],
                               parse_input=inventory[name]["parse_input"], previously_confirmed_in_index=name in prior,
                               separate_root_reproduction=name == "Mount Doom")
                    current.append(row)
            rows.extend(current)
            runs.append(dict(corpus=family, run_id=run, full_status_counts=statuses,
                             candidate_payload_names=len({r["card"] for r in current}), consumer_effect_nodes=len(current),
                             serialized_reference_leaves=sum(len(r["references"]) for r in current)))
    out = dict(scope="Full frozen typed literal-rest tag inventory. Ability/spell scopes and branch paths preserved; sibling producers do not satisfy a consumer. Flattened duplicate effects skipped; mirrored spec/target references deduplicated to one consumer effect.",
               limitations="An absent explicit producer is a static candidate, not an execution failure. Generic implicit runtime binding is not proven by this inventory; runtime source inspection finds no literal-rest special case in effects/core. This scan concerns exactly rest, not all complement/helper tag names. Existing confirmed status is name-level context, not proof of this consumer. Aliases are payloads, not extra primary cards.",
               runs=runs, rows=rows, provenance=dict(scanner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               inventory_sha256=hashlib.sha256(inventory_path.read_bytes()).hexdigest(), prior_index_sha256=hashlib.sha256(summary_path.read_bytes()).hexdigest()))
    (REPORT / "rest-tag-inventory.json").write_text(json.dumps(out, indent=2) + "\n")
    actions = [r for r in rows if r["corpus"] == "actions"]
    md = ["# Typed rest-tag inventory", "", "Static candidates only; no unexecuted name is promoted.", "", "| Payload | Consumer | Prior confirmed name | Scope-local rest producer |", "| --- | --- | --- | --- |"]
    md += [f"| {r['card']} | {r['effect_kind']} | {r['previously_confirmed_in_index']} | {bool(r['literal_rest_producers_in_scope'])} |" for r in actions]
    (REPORT / "rest-tag-inventory.md").write_text("\n".join(md) + "\n")
    print(json.dumps(dict(runs=runs, dataflow_counts=dict(Counter(r["dataflow_status"] for r in rows))), indent=2))


if __name__ == "__main__":
    main()
