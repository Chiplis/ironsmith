#!/usr/bin/env python3
"""Run explicit legal scenarios in isolated, time-bounded native test children."""
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "reports/runtime-audit"
CASES = {
    "goddric_prior0": ("Goddric, Cloaked Reveler", {"power": 3, "toughness": 3, "stack": 0}),
    "goddric_prior1": ("Goddric, Cloaked Reveler", {"power": 4, "toughness": 4, "stack": 0}),
    "grist_soldier_then_insect": ("Grist, the Hunger Tide", {"insect_tokens": 1, "library_remaining": 1, "stack": 0}),
    "grist_insect_then_soldier": ("Grist, the Hunger Tide", {"insect_tokens": 2, "library_remaining": 0, "stack": 0}),
    "plague_decline": ("Plague of Vermin", {"life": [20, 20, 20], "players_in_game": 3, "stack": 0}),
    "plague_pay20": ("Plague of Vermin", {"players_in_game": 0}),
    "triska_life13_mode0": ("Triskaidekaphobia", {"players_in_game": 0}),
    "triska_life13_mode1": ("Triskaidekaphobia", {"players_in_game": 0}),
    "triska_life20_mode0": ("Triskaidekaphobia", {"life": [21, 21, 21], "players_in_game": 3, "stack": 0}),
    "triska_life20_mode1": ("Triskaidekaphobia", {"life": [19, 19, 19], "players_in_game": 3, "stack": 0}),
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    artifacts = [json.loads(line) for line in (REPORT / "noncompletion-build.jsonl").read_text().splitlines()]
    artifact = next(a for a in reversed(artifacts) if a.get("reason") == "compiler-artifact" and a["target"]["name"] == "runtime_noncompletion_reproductions")
    compiled = Path(artifact["executable"])
    folder = REPORT / "noncompletion-native" / digest(compiled)
    folder.mkdir(parents=True, exist_ok=True)
    binary = folder / "runtime_noncompletion_reproductions"
    if not binary.exists():
        shutil.copy2(compiled, binary)
    inventory = REPORT / "corpus/267a16aff3b321196397d0b4/inventory.json"
    source = ROOT / "crates/ironsmith-tools/tests/runtime_noncompletion_reproductions.rs"
    before = {str(p): digest(p) for p in (binary, source, inventory)}

    def run(item):
        case, (name, expected) = item
        env = dict(os.environ, AUDIT_NONCOMPLETION_CASE=case,
                   AUDIT_RUNTIME_INVENTORY=str(inventory), RUST_MIN_STACK="33554432")
        try:
            child = subprocess.run([str(binary), "--ignored", "--nocapture"], cwd=ROOT,
                                   env=env, capture_output=True, timeout=20)
            stdout, stderr, returncode = child.stdout, child.stderr, child.returncode
            status = "child_completed" if returncode == 0 else "child_failed"
        except subprocess.TimeoutExpired as error:
            stdout, stderr, returncode = error.stdout or b"", error.stderr or b"", None
            status = "child_timeout"
        (folder / f"{case}.stdout").write_bytes(stdout)
        (folder / f"{case}.stderr").write_bytes(stderr)
        stages = [json.loads(line.partition("AUDIT_STAGE ")[2]) for line in stdout.decode(errors="replace").splitlines() if "AUDIT_STAGE " in line]
        finish = next((row["data"] for row in reversed(stages) if row["stage"] == "finished"), {})
        actual = finish.get("actual")
        if status == "child_completed":
            if actual is not None:
                status = "expected_result_observed" if all(actual.get(k) == v for k, v in expected.items()) else "semantic_mismatch"
            else:
                status = "fixture_or_execution_error"
        print(case, status, flush=True)
        return {"card": name, "scenario": case, "status": status, "expected": expected,
                "actual": actual, "child_returncode": returncode, "stages": stages,
                "finish": finish, "stdout": str(folder / f"{case}.stdout"), "stderr": str(folder / f"{case}.stderr"),
                "scope": "Strict canonical artifact; explicit legal paid cast/activation or genuine upkeep event; small expected state, stage-traced child process. Timeout/abort requires separate review before promotion."}

    with ThreadPoolExecutor(max_workers=2) as pool:
        rows = list(pool.map(run, CASES.items()))
    after = {str(p): digest(p) for p in (binary, source, inventory)}
    report = {"generated_at": datetime.now(timezone.utc).isoformat(), "rows": rows,
              "provenance": {"before": before, "after": after, "artifacts_unchanged": before == after,
                             "driver_sha256": digest(Path(__file__)), "timeout_seconds": 20,
                             "stack_bytes": 33554432, "parallel_children": 2},
              "limitations": ["Timeouts and process failures are stage-traced observations, not automatically confirmed card defects.",
                              "Expected state comparisons concern only named properties; no whole-card correctness is inferred."]}
    (REPORT / "noncompletion-expected-reproductions.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
