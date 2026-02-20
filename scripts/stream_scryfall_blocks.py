import argparse
import json
from pathlib import Path


def iter_json_array(path):
    decoder = json.JSONDecoder()
    buf = ""
    with open(path, "r", encoding="utf-8") as f:
        # find array start
        while True:
            ch = f.read(1)
            if not ch:
                return
            if ch.isspace():
                continue
            if ch == "[":
                break
        while True:
            chunk = f.read(65536)
            if not chunk:
                return
            buf += chunk
            while True:
                buf = buf.lstrip()
                if not buf:
                    break
                if buf.startswith("]"):
                    return
                if buf.startswith(","):
                    buf = buf[1:]
                    continue
                try:
                    obj, idx = decoder.raw_decode(buf)
                except json.JSONDecodeError:
                    break
                yield obj
                buf = buf[idx:]


def pick_field(card, face, key):
    value = card.get(key)
    if value is not None:
        return value
    if face is not None:
        return face.get(key)
    return None


DIGITAL_ONLY_ORACLE_MARKERS = (
    "boon",
    "conjure",
    "double team",
    "draft",
    "heist",
    "incorporate",
    "intensity",
    "intensify",
    "perpetually",
    "seek",
    "specialize",
    "spellbook",
)


def contains_marker_with_boundaries(text, marker):
    start = text.find(marker)
    while start != -1:
        before = text[start - 1] if start > 0 else ""
        after_index = start + len(marker)
        after = text[after_index] if after_index < len(text) else ""
        before_is_letter = before.isalpha()
        after_is_letter = after.isalpha()
        if not before_is_letter and not after_is_letter:
            return True
        start = text.find(marker, start + 1)
    return False


def has_digital_only_oracle_marker(oracle_text):
    lower = oracle_text.lower()
    return any(
        contains_marker_with_boundaries(lower, marker)
        for marker in DIGITAL_ONLY_ORACLE_MARKERS
    )


def is_non_playable(card, type_line, oracle_text):
    border_color = (card.get("border_color") or "").strip().lower()
    if border_color == "silver":
        return True

    if card.get("has_acorn"):
        return True

    legalities = card.get("legalities") or {}
    if legalities and all(value == "not_legal" for value in legalities.values()):
        return True

    layout = (card.get("layout") or "").strip().lower()
    if layout in {
        "token",
        "double_faced_token",
        "emblem",
        "planar",
        "scheme",
        "vanguard",
        "art_series",
        "reversible_card",
    }:
        return True

    if type_line:
        non_playable_types = (
            "Token",
            "Emblem",
            "Plane",
            "Scheme",
            "Vanguard",
            "Phenomenon",
            "Conspiracy",
            "Dungeon",
            "Attraction",
            "Contraption",
        )
        if any(label in type_line for label in non_playable_types):
            return True

        # Jumpstart theme cards show up as "Type: Card".
        if type_line.strip() == "Card":
            return True

    if oracle_text and "Theme color" in oracle_text:
        return True
    if oracle_text and has_digital_only_oracle_marker(oracle_text):
        return True

    return False


def build_block(card):
    faces = card.get("card_faces") or []
    face = faces[0] if faces else None

    name = card.get("name") or (face or {}).get("name")
    if not name:
        return None

    mana_cost = pick_field(card, face, "mana_cost")
    type_line = pick_field(card, face, "type_line")
    oracle_text = pick_field(card, face, "oracle_text")
    power = pick_field(card, face, "power")
    toughness = pick_field(card, face, "toughness")
    loyalty = pick_field(card, face, "loyalty")
    defense = pick_field(card, face, "defense")

    if is_non_playable(card, type_line, oracle_text):
        return None

    lines = [f"Name: {name}"]
    if mana_cost:
        lines.append(f"Mana cost: {mana_cost}")
    if type_line:
        lines.append(f"Type: {type_line}")
    if power is not None and toughness is not None:
        lines.append(f"Power/Toughness: {power}/{toughness}")
    if loyalty is not None:
        lines.append(f"Loyalty: {loyalty}")
    if defense is not None:
        lines.append(f"Defense: {defense}")
    if oracle_text:
        lines.append(oracle_text)
    return "\n".join(lines)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Stream playable Scryfall cards as parser input blocks."
    )
    parser.add_argument(
        "--cards",
        default="cards.json",
        help="Path to the cards JSON file (default: cards.json)",
    )
    return parser.parse_args()


def main():
    args = parse_args()
    path = Path(args.cards)
    for card in iter_json_array(path):
        block = build_block(card)
        if not block:
            continue
        print(block)
        print("---")


if __name__ == "__main__":
    main()
