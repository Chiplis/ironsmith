#!/usr/bin/env python3
"""Run Cargo's emitted native test executables with per-binary bounds.

Input is the JSON-lines output of `cargo test --tests --no-run
--message-format=json`. Build failures, missing executables, skipped tests and
timeouts remain visible; assertion failures are candidates requiring review.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("build_log", type=Path)
    parser.add_argument("--out", type=Path, default=Path("reports/runtime-audit/native-tests"))
    parser.add_argument("--timeout", type=float, default=180)
    parser.add_argument("--jobs", type=int, default=2)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    emitted = {}
    build_success = None
    for line in args.build_log.read_text().splitlines():
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if item.get("reason") == "build-finished":
            build_success = item["success"]
        if (item.get("reason") == "compiler-artifact" and item.get("profile", {}).get("test")
                and item.get("executable")):
            emitted[item["executable"]] = item["target"]
    environment = dict(os.environ, RUST_MIN_STACK="16777216")

    def run(entry):
        executable, target = entry
        binary = Path(executable)
        name = target["name"]
        result = {"target": name, "source": target["src_path"], "binary": executable}
        if not binary.exists():
            return dict(result, status="executable_missing")
        digest = hashlib.file_digest(binary.open("rb"), "sha256").hexdigest()
        result["binary_sha256"] = digest
        log = args.out / f"{name}-{digest[:12]}.log"
        started = time.monotonic()
        try:
            process = subprocess.run([executable, "--test-threads=2", "--nocapture"],
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=environment,
                timeout=args.timeout)
            output = process.stdout.decode(errors="replace")
            status, code = ("passed" if process.returncode == 0 else "failed"), process.returncode
        except subprocess.TimeoutExpired as error:
            output = (error.stdout or b"").decode(errors="replace")
            status, code = "timeout", None
        log.write_text(output)
        outcomes = [{"test": match.group(1), "status": match.group(2)} for match in
                    re.finditer(r"^test (.+?) \.\.\. (ok|FAILED|ignored)(?:,.*)?$", output, re.M)]
        result.update(status=status, exit_code=code, elapsed_seconds=round(time.monotonic() - started, 3),
                      tests=outcomes, log=str(log))
        (args.out / f"{name}-{digest[:12]}.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps({"target": name, "status": status, "observed_tests": len(outcomes)}), flush=True)
        return result

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = list(pool.map(run, emitted.items()))
    report = {"build_succeeded": build_success, "emitted_test_executables": len(emitted),
              "timeout_seconds_per_executable": args.timeout, "results": results,
              "limitations": "Emitted executables only. Incomplete builds and timed-out test bodies are uncovered. Failed assertions require fixture/rules review before naming broken cards. Ignored tests are not passed."}
    (args.out / "summary.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
