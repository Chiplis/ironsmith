#!/usr/bin/env python3
"""Compare recorded strict definitions to one immutable campaign's definitions.

Only numeric Card.id fields are excluded from semantic equality. This checks
compiler artifacts, not runtime executable equivalence or gameplay correctness.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def differences(old, fresh, path="$", card_id=False):
    if type(old) is not type(fresh):
        return [{"path": path, "frozen": old, "fresh": fresh, "generated_card_id": False}]
    if isinstance(old, dict):
        rows = []
        for key in sorted(old.keys() | fresh.keys()):
            if key not in old or key not in fresh:
                rows.append({"path": path + "/" + key, "missing_in": "frozen" if key not in old else "fresh", "generated_card_id": False})
            else:
                rows += differences(old[key], fresh[key], path + "/" + key,
                    key == "id" and path.endswith("/card") and "name" in old and "name" in fresh)
        return rows
    if isinstance(old, list):
        if len(old) != len(fresh):
            return [{"path": path, "frozen": old, "fresh": fresh, "generated_card_id": False}]
        return [row for i, (a, b) in enumerate(zip(old, fresh))
                for row in differences(a, b, path + "/" + str(i))]
    if old == fresh:
        return []
    return [{"path": path, "frozen": old, "fresh": fresh,
             "generated_card_id": card_id and type(old) is int and type(fresh) is int}]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--compilation-report", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--database", type=Path, default=Path("reports/runtime-audit/corpus/results.sqlite3"))
    parser.add_argument("--run-id", default="267a16aff3b321196397d0b4")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        assert differences({"card": {"id": 1, "name": "A"}}, {"card": {"id": 2, "name": "A"}})[0]["generated_card_id"]
        assert not differences({"card": {"id": 1}}, {"card": {"id": 2}})[0]["generated_card_id"]
        assert not differences({"effect": {"id": 1}}, {"effect": {"id": 2}})[0]["generated_card_id"]
        assert not differences({"a": None}, {})[0]["generated_card_id"]
        assert not differences({"card": {"id": 1, "name": "A"}}, {"card": {"id": 2, "name": "B"}})[1]["generated_card_id"]
        print("Definition comparison checks passed.")
        return
    if not args.report or not args.compilation_report or not args.output:
        parser.error("--report, --compilation-report, and --output are required")
    report = json.loads(args.report.read_text())
    compiled = json.loads(args.compilation_report.read_text())["compilation"]
    checksums = {row["card"]: row["artifact_checksum"] for row in compiled}
    assert len(checksums) == len(compiled), "duplicate compiled names"
    assert all(row["artifact_checksum"] == checksums[row["card"]]
               for row in report["rows"] if "artifact_checksum" in row), "scenario artifacts differ from compared artifacts"
    rows = []
    with sqlite3.connect(args.database.resolve().as_uri() + "?mode=ro", uri=True) as connection:
        connection.execute("PRAGMA query_only=ON")
        for item in compiled:
            result = connection.execute("SELECT result_json FROM result WHERE run_id=? AND card_name=?",
                                        (args.run_id, item["card"])).fetchone()
            if not result:
                raise ValueError(f"Missing frozen definition: {item['card']}")
            frozen = json.loads(result[0])
            diff = differences(frozen["definition"], item["definition"])
            rows.append({"card": item["card"], "equal_except_generated_card_ids": all(d["generated_card_id"] for d in diff),
                         "differences": diff, "artifact_checksum": item["artifact_checksum"],
                         "frozen_result_sha256": hashlib.sha256(result[0].encode()).hexdigest()})
    out = {"generated_at": datetime.now(timezone.utc).isoformat(),
           "raw_report": str(args.report), "raw_report_sha256": digest(args.report),
           "compiled_definitions": str(args.compilation_report), "compiled_definitions_sha256": digest(args.compilation_report),
           "generator_sha256": digest(Path(__file__)), "run_id": args.run_id,
           "all_scenario_artifact_checksums_match": True,
           "comparison": "Full strict definitions; only numeric Card.id fields on named Card structures are ignored. Every difference is retained. Compiler artifact parity is not runtime executable equivalence.",
           "rows": rows}
    args.output.write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n")
    same = all(row["equal_except_generated_card_ids"] for row in rows)
    print(json.dumps({"definitions": len(rows), "equal_except_generated_card_ids": same}))
    if not same:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
