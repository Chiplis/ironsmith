#!/usr/bin/env python3
"""Generate crates/ironsmith-engine/src/cards/generated_oracle_face_types.txt.

CR 201.4a-f: a card name chosen "with certain characteristics" is checked
against the Oracle characteristics of the named card (or the selected half /
face / alternative characteristics). The engine reads this table to validate
such choices without a loaded card registry.

Format: lines starting with '#' carry the supertypes and card types of the
names that follow (the type line before the em dash); every other line is one
Oracle card or face name. A name with several Oracle records appears once per
distinct type line.

Usage: scripts/generate_oracle_face_types.py [cards.json] [output]
"""
import json
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
cards_path = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "cards.json"
out_path = (
    Path(sys.argv[2])
    if len(sys.argv) > 2
    else ROOT / "crates/ironsmith-engine/src/cards/generated_oracle_face_types.txt"
)


def type_prefix(type_line):
    return (type_line or "").split("—")[0].split(" - ")[0].strip()


groups = defaultdict(set)
for card in json.load(open(cards_path, encoding="utf-8")):
    if card.get("layout") in ("token", "double_faced_token", "emblem", "art_series"):
        continue
    faces = [face for face in card.get("card_faces") or [] if face.get("name")]
    if faces:
        records = [(face["name"], face.get("type_line")) for face in faces]
    else:
        records = [(card["name"], card.get("type_line"))]
    for name, type_line in records:
        prefix = type_prefix(type_line)
        # Sticker sheets are not Magic cards (CR 123.2a); their names are not card names.
        if name and " // " not in name and prefix and prefix != "Stickers":
            groups[prefix].add(name)

with open(out_path, "w", encoding="utf-8") as out:
    for prefix in sorted(groups):
        out.write(f"#{prefix}\n")
        for name in sorted(groups[prefix]):
            out.write(f"{name}\n")
print(f"wrote {out_path} ({sum(map(len, groups.values()))} names, {len(groups)} type lines)")
