#!/usr/bin/env python3
"""Audit handwritten card definitions without putting policy work in build.rs."""

from __future__ import annotations

import argparse
from pathlib import Path


FORBIDDEN = (
    "use crate::cards::builders::CardDefinitionBuilder;",
    "use crate::cards::CardDefinitionBuilder;",
    "crate::cards::builders::CardDefinitionBuilder::new(",
    "crate::cards::CardDefinitionBuilder::new(",
    ".with_ability(",
    ".with_abilities(",
    ".with_etb(",
    ".with_dies_trigger(",
    ".with_upkeep_trigger(",
    ".with_trigger(",
    ".with_targeted_etb(",
    ".with_optional_trigger(",
    ".with_activated(",
    ".with_tap_ability(",
    ".with_spell_effect(",
    ".with_chapter(",
    ".with_chapters(",
    ".with_level_abilities(",
    ".spell_effect =",
    ".abilities =",
    ".alternative_casts =",
    ".optional_costs =",
    ".aura_attach_filter =",
    ".has_fuse =",
    ".additional_cost =",
    "CardDefinition::spell(",
    "CardDefinition::spell_with_abilities(",
    "CardDefinition::with_abilities(",
)


def uncommented_source(path: Path) -> str:
    lines: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        lines.append(line.split("//", 1)[0])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1]
        / "crates"
        / "ironsmith-engine"
        / "src"
        / "cards"
        / "definitions",
    )
    args = parser.parse_args()

    violations: list[str] = []
    for path in sorted(args.root.rglob("*.rs")):
        if path.name == "builder.rs":
            continue
        source = uncommented_source(path)
        hits = [needle for needle in FORBIDDEN if needle in source]
        if hits:
            violations.append(f"{path}:\n  " + "\n  ".join(hits))
    if violations:
        print("handwritten definitions crossed the parser/compiled-text boundary:")
        print("\n".join(violations))
        return 1
    print(f"definition boundary audit passed: {args.root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
