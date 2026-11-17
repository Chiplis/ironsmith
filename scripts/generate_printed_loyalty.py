#!/usr/bin/env python3
"""Generate the printed loyalty/defense table used by the Manabrew adapter.

The Manabrew deck protocol has no field for a planeswalker's printed loyalty
or a battle's printed defense, and neither can be derived from the type line
or rules text. The adapter looks them up by face name in this table when a
host does not supply them.

Usage: python3 scripts/generate_printed_loyalty.py [cards.json] [output.tsv]
Rows: <face name>\t<L|D>\t<printed value>, sorted by name.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "crates/ironsmith-wasm/src/wasm_game_impl/printed_loyalty.tsv"


def printed_stats(cards):
    rows = {}
    for card in cards:
        if card.get("lang", "en") != "en":
            continue
        # Each face of a multi-faced card carries its own printed value.
        for face in card.get("card_faces") or [card]:
            name = (face.get("name") or "").strip()
            if not name or "\t" in name:
                continue
            for field, kind in (("loyalty", "L"), ("defense", "D")):
                value = face.get(field)
                if value is None or not str(value).strip():
                    continue
                key = (name, kind)
                value = str(value).strip()
                if rows.setdefault(key, value) != value:
                    raise SystemExit(f"conflicting {field} for {name}: {rows[key]} vs {value}")
    return rows


def main():
    cards_path = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "cards.json"
    output = Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_OUTPUT
    with open(cards_path, encoding="utf-8") as handle:
        rows = printed_stats(json.load(handle))
    lines = [f"{name}\t{kind}\t{value}" for (name, kind), value in sorted(rows.items())]
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines)} rows to {output}")


if __name__ == "__main__":
    main()
