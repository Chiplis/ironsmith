#!/usr/bin/env python3
"""Isolate legal canonical Rune casts and equip activations; never promote a hang alone."""
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
NAMES = ["Rune of Flight", "Rune of Might", "Rune of Mortality", "Rune of Speed", "Rune of Sustenance"]


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1048576), b""):
            h.update(block)
    return h.hexdigest()


def main():
    builds = [json.loads(line) for line in (REPORT / "rune-build.jsonl").read_text().splitlines()]
    artifact = next(a for a in reversed(builds) if a.get("reason") == "compiler-artifact" and a["target"]["name"] == "runtime_rune_reproductions")
    compiled = Path(artifact["executable"])
    folder = REPORT / "rune-native" / digest(compiled) / "selected-input-rerun"
    folder.mkdir(parents=True, exist_ok=True)
    binary = folder / "runtime_rune_reproductions"
    if not binary.exists():
        shutil.copy2(compiled, binary)
    inventory = REPORT / "corpus/267a16aff3b321196397d0b4/inventory.json"
    selected = [p for p in json.loads(inventory.read_text())["cards"] if p["name"] in NAMES + ["Bonesplitter"]]
    assert len(selected) == 6
    inputs = folder / "exact-selected-inputs.json"
    inputs.write_text(json.dumps({"cards": selected}, ensure_ascii=False) + "\n")
    source = ROOT / "crates/ironsmith-tools/tests/runtime_rune_reproductions.rs"
    before = {str(p): digest(p) for p in (binary, source, inventory, inputs)}
    cases = [(n, f) for n in NAMES for f in ["creature", "land", "equipment_before", "equipment_after", "equipment_unequipped"]]
    cases += [("Rune of Flight", "equipment_control")]

    def run(case):
        name, fixture = case
        label = name.replace(" ", "_") + "-" + fixture
        active = fixture in {"creature", "equipment_before", "equipment_after"}
        equipped = fixture in {"equipment_before", "equipment_after", "equipment_control"}
        expected = {"keyword": active, "power": 2 + 2 * equipped + int(active and name in {"Rune of Might", "Rune of Speed"}),
                    "toughness": 6 + int(active and name == "Rune of Might"), "hand": int(fixture != "equipment_control"), "stack": 0}
        case_input = folder / f"{label}.inputs.json"
        case_input.write_text(json.dumps({"cards": [p for p in selected if p["name"] in {name, "Bonesplitter"}]}, ensure_ascii=False) + "\n")
        env = dict(os.environ, AUDIT_RUNE_NAME=name, AUDIT_RUNE_FIXTURE=fixture,
                   AUDIT_RUNE_INPUT=str(case_input), RUST_MIN_STACK="16777216", RUST_BACKTRACE="1")
        stdout_path, stderr_path = folder / f"{label}.stdout", folder / f"{label}.stderr"
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            proc = subprocess.Popen([str(binary), "--ignored", "--nocapture"], cwd=ROOT, env=env, stdout=stdout, stderr=stderr)
            try:
                code = proc.wait(timeout=45)
                status = "child_completed" if code == 0 else "child_failed"
            except subprocess.TimeoutExpired:
                profile = folder / f"{label}.sample.txt"
                subprocess.run(["sample", str(proc.pid), "1", "-file", str(profile)], capture_output=True, timeout=8)
                proc.kill()
                code = proc.wait()
                status = "child_timeout"
        stages = [json.loads(line.partition("RUNE_STAGE ")[2]) for line in stdout_path.read_text(errors="replace").splitlines() if "RUNE_STAGE " in line]
        finish = next((s["data"] for s in reversed(stages) if s["stage"] == "finished"), {})
        actual = finish.get("actual")
        if status == "child_completed":
            status = "expected_result_observed" if actual == expected else "semantic_mismatch" if actual is not None else "execution_or_fixture_error"
        print(name, fixture, status, flush=True)
        return {"card": name, "scenario": fixture, "status": status, "expected": expected, "actual": actual,
                "case_input_sha256": digest(case_input),
                "finish": finish, "child_returncode": code, "stages": stages, "stdout": str(stdout_path), "stderr": str(stderr_path)}

    with ThreadPoolExecutor(max_workers=2) as pool:
        rows = list(pool.map(run, cases))
    after = {str(p): digest(p) for p in (binary, source, inventory, inputs)}
    report = {"generated_at": datetime.now(timezone.utc).isoformat(), "rows": rows,
              "scope": "Strict canonical Rune and Bonesplitter artifacts. Actual paid Rune casts and Equip activations, normal priority-pass resolution including ETB events. Established legal board, inert library. Isolated process failures require attribution review.",
              "provenance": {"before": before, "after": after, "artifacts_unchanged": before == after,
                             "driver_sha256": digest(Path(__file__)), "timeout_seconds": 45, "stack_bytes": 16777216, "parallel_children": 2},
              "limitations": ["No whole-card correctness is inferred from a passing scenario.", "Timeouts and aborts are observations pending attribution, not automatic confirmed defects."]}
    (REPORT / "rune-execution-reproductions.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
