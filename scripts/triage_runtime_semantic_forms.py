#!/usr/bin/env python3
"""Prioritize text-screen candidates without treating rendering differences as card passes."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from audit_runtime_semantics import scan_text


def outside_parentheses(text):
    depth = 0
    result = []
    for char in text:
        if char == "(":
            depth += 1
        elif char == ")" and depth:
            depth -= 1
        elif not depth:
            result.append(char)
    # Malformed text must not silently hide the remainder.
    return text if depth else " ".join("".join(result).split())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--findings", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    counts = Counter()
    rows = []
    with args.findings.open() as source:
        for line in source:
            card = json.loads(line)
            for finding in card.get("semantic_candidates", []):
                outside = outside_parentheses(finding["oracle_ability"])
                narrowed = scan_text(outside, card.get("compiled_text", ""))
                surviving = {check["check"] for row in narrowed for check in row["checks"]}
                for check in finding["checks"]:
                    kind = "persists_outside_parentheses" if check["check"] in surviving else "parenthetical_or_alignment_sensitive"
                    counts[kind] += 1
                    rows.append({
                        "card": card["name"], "classification": kind,
                        "check": check, "oracle_ability": finding["oracle_ability"],
                        "outside_parentheses": outside,
                        "original_compiled_line_candidate": finding["compiled_line_candidate"],
                        "rescan": narrowed,
                    })
    report = {
        "scope": "Text-screen prioritization only. Parenthetical or alignment-sensitive findings remain unverified; no card or keyword is cleared by this classification.",
        "counts": dict(counts), "rows": rows,
        "provenance": {
            "findings": str(args.findings.resolve()),
            "findings_sha256": hashlib.sha256(args.findings.read_bytes()).hexdigest(),
            "classifier_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "scanner_sha256": hashlib.sha256(Path(__file__).with_name("audit_runtime_semantics.py").read_bytes()).hexdigest(),
        },
    }
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(dict(counts)))


if __name__ == "__main__":
    main()
