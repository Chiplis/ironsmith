#!/usr/bin/env python3
"""Replay bounded corpus noncompletion in isolated workers without changing original evidence."""
import argparse
import datetime
import json
from pathlib import Path
import sqlite3

from audit_runtime_corpus import Worker, checksum


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", action="append", required=True, type=Path)
    parser.add_argument("--timeout", type=float, default=90)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "scope": "Isolated bounded replay of noncompletion; timeouts remain unknown and are not confirmed gameplay defects.",
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "timeout_seconds": args.timeout,
        "driver_sha256": checksum(__file__),
        "worker_driver_sha256": checksum(Path(__file__).with_name("audit_runtime_corpus.py")),
        "rows": [],
    }
    for run in args.run:
        manifest = json.loads((run / "manifest.json").read_text())
        cards = {card["name"]: card for card in json.loads((run / "inventory.json").read_text())["cards"]}
        binary = run / "audit_runtime_worker"
        binary_hash = checksum(binary)
        with sqlite3.connect(f"file:{run.parent / 'results.sqlite3'}?mode=ro", uri=True) as db:
            originals = db.execute(
                "select card_name,result_json from result where run_id=? and status in ('timeout','worker_crashed','worker_protocol_failed','panicked') order by card_name",
                (run.name,),
            ).fetchall()
        for name, original_json in originals:
            original = json.loads(original_json)
            request = dict(cards[name], include_definition=False)
            # The canonical inventory deliberately defaults flags to false; the manifest owns run mode.
            request["actions_only"] = bool(manifest.get("actions_only", False))
            request["contracts_only"] = bool(manifest.get("contracts_only", False))
            attempts = []
            for mode in ["original", "contracts_only"]:
                if mode == "contracts_only":
                    request.update(actions_only=False, contracts_only=True)
                worker = Worker(binary.resolve(), args.timeout, args.out.with_suffix(".stderr.log"))
                try:
                    result = worker.run(request)
                finally:
                    worker.close()
                attempts.append({"mode": mode, "request": dict(request), "result": result})
                print(run.parent.name, name, mode, result["status"], flush=True)
            report["rows"].append({
                "card": name, "original_run": str(run.resolve()), "original_result": original,
                "attempts": attempts, "binary_sha256": binary_hash,
                "binary_unchanged": checksum(binary) == binary_hash,
                "manifest_sha256": checksum(run / "manifest.json"),
                "inventory_sha256": checksum(run / "inventory.json"),
                "cards_sha256": manifest["cards_sha256"],
            })
            temporary = args.out.with_suffix(".tmp")
            temporary.write_text(json.dumps(report, indent=2) + "\n")
            temporary.replace(args.out)


if __name__ == "__main__":
    main()
