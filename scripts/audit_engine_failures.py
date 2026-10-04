#!/usr/bin/env python3
"""Rerun recorded engine failures individually with a fixed time budget.

This deliberately executes an existing test binary without changing/building the
workspace. A nonzero exit is test evidence, not proof that a catalog card fails.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", default="reports/runtime-audit/engine-baseline-summary.json")
    parser.add_argument("--binary", required=True)
    # Keep fresh observations separate from the manually reviewed triage report.
    parser.add_argument("--output", default="reports/runtime-audit/engine-failure-rerun.json")
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--workers", type=int, default=4)
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    summary = json.loads(Path(args.summary).read_text())
    output = Path(args.output).resolve()
    logs = output.parent / "engine-failure-logs"
    logs.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, RUST_MIN_STACK="16777216")

    def run(name):
        suffix = hashlib.sha256(name.encode()).hexdigest()[:10]
        log = logs / f"{name.split('::')[-1]}-{suffix}.log"
        command = [str(binary), "--exact", name, "--nocapture"]
        started = time.monotonic()
        try:
            process = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                     env=environment, timeout=args.timeout, text=True)
            content = process.stdout
            status = "passed" if process.returncode == 0 else "failed"
            code = process.returncode
            if "running 0 tests" in content:
                status = "not_found"
        except subprocess.TimeoutExpired as error:
            content = error.stdout or b""
            if isinstance(content, bytes):
                content = content.decode(errors="replace")
            status, code = "timeout", None
        log.write_text(content)
        return {"test": name, "rerun_status": status, "exit_code": code,
                "elapsed_seconds": round(time.monotonic() - started, 3),
                "command": command, "log": str(log), "classification": "pending_review"}

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        results = list(pool.map(run, summary["failed"]))
    report = {
        "binary": str(binary),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "timeout_seconds": args.timeout,
        "limitations": "Existing test-binary reruns. Card names are candidates until reproduced with their current compiled definitions. No engine changes or rebuilds.",
        "results": results,
        "pending_baseline_tests": summary.get("pending", []),
    }
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"report": str(output), "statuses": {
        key: sum(item["rerun_status"] == key for item in results)
        for key in ["passed", "failed", "timeout", "not_found"]
    }}))


if __name__ == "__main__":
    main()
