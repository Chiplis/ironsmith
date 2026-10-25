#!/usr/bin/env python3
"""Maintain the exhaustive CR inventory; source citations never imply compliance."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "reports/cr-complete-audit"
SOURCE = OUT / "MagicCompRules-20260925.txt"
LEDGER = OUT / "ledger.json"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--initialize", action="store_true")
    args = parser.parse_args()
    raw = SOURCE.read_bytes()
    text = raw.decode("utf-8-sig")
    # Ignore the table of contents. Include section rules as well as subrules.
    body = text[text.index("100. General", text.index("100. General") + 1):]
    body = body[:body.index("\nGlossary")]
    # CR 704.5aa has a multi-letter suffix; suffixes are not limited to a-z.
    matches = list(re.finditer(r"^(\d{3}\.(?:\d+[a-z]*)?)(?:\.)? (.+)", body, re.M))
    rules = {}
    for i, match in enumerate(matches):
        key = match[1].rstrip(".")
        end = matches[i + 1].start() if i + 1 < len(matches) else len(body)
        assert key not in rules, f"Duplicate rule: {key}"
        rules[key] = {
            "rule": key,
            "text": body[match.start():end].strip(),
            "status": "pending",
            "evidence": [],
            "tests": [],
            "notes": "",
        }
    if args.initialize:
        if LEDGER.exists():
            raise SystemExit("Refusing to overwrite an existing ledger.")
        ledger = {
            "source_url": "https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt",
            "effective_date": "2026-09-25",
            "sha256": hashlib.sha256(raw).hexdigest(),
            "rules": list(rules.values()),
        }
        LEDGER.write_text(json.dumps(ledger, indent=2, ensure_ascii=False) + "\n")
    ledger = json.loads(LEDGER.read_text())
    assert ledger["sha256"] == hashlib.sha256(raw).hexdigest()
    assert len(ledger["rules"]) == len(rules)
    assert {r["rule"] for r in ledger["rules"]} == set(rules)
    test_source = (ROOT / "crates/ironsmith-engine/tests/cr_complete_audit.rs").read_text()
    test_names = set(re.findall(r"#\[test\]\s*fn\s+(\w+)", test_source))
    log_path = OUT / "test-results.log"
    log = log_path.read_text() if log_path.exists() else ""
    # Cargo's final failure list is stable even when parallel panic output
    # interleaves with individual test-result lines.
    failed_tests = set(re.findall(r"^    (cr_\w+)\s*$", log, re.M))
    valid = {"pending", "in_review", "verified", "mismatch_reproduced",
             "not_engine_behavior", "blocked"}
    for row in ledger["rules"]:
        assert row["text"] == rules[row["rule"]]["text"], row["rule"]
        assert row["status"] in valid, row["rule"]
        assert set(row["tests"]) <= test_names, f"Unknown test for {row['rule']}"
        if row["status"] not in {"pending", "in_review"}:
            assert row["evidence"] and row["notes"], row["rule"]
        if row["status"] == "mismatch_reproduced":
            assert row["tests"], row["rule"]
            assert "test result: FAILED." in log, "A completed test run is required"
            assert set(row["tests"]) & failed_tests, f"No current failing test for {row['rule']}"
    counts = Counter(r["status"] for r in ledger["rules"])
    lines = [
        "# Exhaustive CR audit progress", "",
        "Source: Wizards Comprehensive Rules, effective September 25, 2026.",
        "The JSON ledger contains the full text and individual status of every numbered",
        "rule, subrule, and section heading. Pending rows are **not coverage**.",
        "A citation or an old remediation claim is only a research lead.",
        "",
        f"Inventory: {len(rules)} entries.", "",
        "| Status | Entries |", "|---|---:|",
        *[f"| {key} | {counts[key]} |" for key in sorted(valid)], "",
        "| Section | Total | Pending | In review | Verified | Mismatches | Non-engine | Blocked |",
        "|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for section in sorted({r["rule"].split(".")[0] for r in ledger["rules"]}):
        rows = [r for r in ledger["rules"] if r["rule"].split(".")[0] == section]
        c = Counter(r["status"] for r in rows)
        lines.append(f"| {section} | {len(rows)} | " + " | ".join(str(c[k]) for k in
            ["pending", "in_review", "verified", "mismatch_reproduced", "not_engine_behavior", "blocked"]) + " |")
    scope_path = OUT / "verification-scope.json"
    if scope_path.exists():
        scope = json.loads(scope_path.read_text())
        scope_lines = [
            "Verification scope: **" + scope["kind"].replace("_", " ") + "**.",
            f"Last accepted full run: **{scope['tests']} tests: {scope['passed']} passed / {scope['failed']} intentionally failed**.",
            "See `verification-scope.json` for the source revision, hashes, and later working-tree drift.",
            f"Current authored integration tests: **{len(test_names)}**. A new test is not verified until an accepted run executes it.",
            "The goal remains incomplete.", "",
        ]
        if scope.get("current_worktree_verification_required"):
            scope_lines.insert(2,
                "**Live worktree revalidation is required.** This run uses the last verified production inputs with the updated audit module; concurrent production edits are excluded.")
        lines[2:2] = scope_lines
    (OUT / "progress.md").write_text("\n".join(lines) + "\n")
    print(json.dumps({"entries": len(rules), "statuses": counts}, indent=2))


if __name__ == "__main__":
    main()
