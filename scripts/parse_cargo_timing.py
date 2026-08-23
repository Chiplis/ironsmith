#!/usr/bin/env python3
"""Extract stable summary data from Cargo's self-contained timing HTML."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


UNIT_DATA_RE = re.compile(
    r"const UNIT_DATA = (\[.*?\]);\s*const CONCURRENCY_DATA",
    re.DOTALL,
)
DURATION_RE = re.compile(r"\bDURATION = ([0-9.]+);")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("timing_html", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--workspace-prefix", default="ironsmith")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source = args.timing_html.read_text(encoding="utf-8")
    unit_match = UNIT_DATA_RE.search(source)
    duration_match = DURATION_RE.search(source)
    if unit_match is None or duration_match is None:
        raise SystemExit(f"could not parse Cargo timing data from {args.timing_html}")

    units = json.loads(unit_match.group(1))
    workspace_units = [
        {
            "name": unit["name"],
            "target": unit.get("target", ""),
            "startSeconds": unit["start"],
            "durationSeconds": unit["duration"],
            "rmetaSeconds": unit.get("rmeta_time"),
        }
        for unit in units
        if unit["name"].startswith(args.workspace_prefix)
    ]
    workspace_units.sort(key=lambda unit: unit["durationSeconds"], reverse=True)

    payload = {
        "cargoDurationSeconds": float(duration_match.group(1)),
        "workspaceUnits": workspace_units,
        "slowestUnits": [
            {
                "name": unit["name"],
                "target": unit.get("target", ""),
                "startSeconds": unit["start"],
                "durationSeconds": unit["duration"],
                "rmetaSeconds": unit.get("rmeta_time"),
            }
            for unit in sorted(units, key=lambda unit: unit["duration"], reverse=True)[:20]
        ],
    }
    encoded = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.output is None:
        print(encoded, end="")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
