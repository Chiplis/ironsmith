#!/usr/bin/env python3
"""Find numeric predicate literals absent from executable wire data, including preserved modal text."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sqlite3

from audit_runtime_semantics import normalize
from triage_runtime_semantic_forms import outside_parentheses

PATTERN = re.compile(r"\b(?:exactly|fewer than|more than|less than|greater than|equal to)\s+(\d+)\b")
SURFACE_KEYS = {"card", "source_text", "description", "name", "display_name", "oracle_text", "text", "hints", "id"}


def literals(node, path="definition"):
    if type(node) is int:
        yield node, path
    elif isinstance(node, list):
        for i, child in enumerate(node):
            yield from literals(child, f"{path}[{i}]")
    elif isinstance(node, dict):
        for key, child in node.items():
            if key in SURFACE_KEYS or "surface" in key:
                continue
            if key == "flattened_default_effects" and "segments" in node:
                continue
            yield from literals(child, f"{path}.{key}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    inventory = json.loads((args.run / "inventory.json").read_text())
    rows = []
    with sqlite3.connect(f"file:{args.run.parent / 'results.sqlite3'}?mode=ro", uri=True) as db:
        for card in inventory["cards"]:
            predicates = []
            for line in card["oracle_text"].splitlines():
                for match in PATTERN.finditer(normalize(outside_parentheses(line))):
                    number = int(match.group(1))
                    if number > 1:
                        predicates.append({"oracle_line": line, "number": number, "predicate": match.group()})
            if not predicates:
                continue
            found = db.execute("select result_json from result where run_id=? and card_name=?", (args.run.name, card["name"])).fetchone()
            result = json.loads(found[0]) if found else {}
            definition = result.get("definition")
            if definition is None:
                rows.append({"card": card["name"], "status": "unavailable_definition", "compile_execution_status": result.get("status"), "predicates": predicates})
                continue
            values = list(literals(definition))
            for predicate in predicates:
                matching = [path for value, path in values if value == predicate["number"]]
                rows.append({"card": card["name"], "status": "scalar_absence_candidate" if not matching else "scalar_present_unverified", "predicate": predicate,
                             "matching_executable_scalar_paths": matching, "artifact_checksum": result.get("artifact_checksum")})
    report = {
        "scope": "Numeric predicate screen beyond rendered-text comparison. Absence may reflect implicit keyword semantics, an interpreted string, or alternate encoding; presence may be unrelated. Every result needs typed-path review and execution. No card is cleared.",
        "pattern": PATTERN.pattern, "rows": rows,
        "provenance": {"run": str(args.run.resolve()), "manifest": json.loads((args.run / "manifest.json").read_text()),
                       "scanner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
    }
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"rows": len(rows), "absence_names": sorted({r["card"] for r in rows if r["status"] == "scalar_absence_candidate"})}))


if __name__ == "__main__":
    main()
