#!/usr/bin/env python3
"""Generate Rust source with parser-backed card registry entries from cards.json."""

from __future__ import annotations

import argparse
import json
import os
import struct
from pathlib import Path
from typing import Dict, List, Tuple

from stream_scryfall_blocks import (
    build_block,
    has_digital_only_oracle_marker,
    is_non_paper_print,
    is_non_playable,
    iter_json_array,
)


ROOT = Path(__file__).resolve().parents[1]
CARDS_JSON = ROOT / "cards.json"
OUT_FILE = ROOT / "src" / "cards" / "generated_registry.rs"
PAYLOAD_FILE_NAME = "generated_registry_payload.bin"

def card_oracle_text(card: dict) -> str | None:
    oracle_text = card.get("oracle_text")
    if isinstance(oracle_text, str):
        return oracle_text
    faces = card.get("card_faces")
    if isinstance(faces, list) and faces:
        first = faces[0]
        if isinstance(first, dict):
            face_oracle = first.get("oracle_text")
            if isinstance(face_oracle, str):
                return face_oracle
    return None


def rust_raw_literal(value: str) -> str:
    """Return a Rust string literal (raw when possible)."""
    for hashes in range(0, 12):
        marks = "#" * hashes
        end = f'"{marks}'
        if end not in value:
            return f'r{marks}"{value}"{marks}'
    return json.dumps(value)


def load_excluded_name_keys() -> set[str]:
    raw_path = os.environ.get("IRONSMITH_GENERATED_REGISTRY_SKIP_NAMES_FILE", "").strip()
    if not raw_path:
        return set()

    path = Path(raw_path)
    if not path.exists():
        print(f"[generate_baked_registry] skip-list file not found: {path}")
        return set()

    excluded: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        name = line.strip()
        if name:
            excluded.add(name.casefold())
    return excluded


FlipPair = Tuple[str, str, str, str, str]


def collect_unique_blocks(
    excluded_name_keys: set[str],
) -> Tuple[Dict[str, Tuple[str, str]], List[FlipPair]]:
    unique: Dict[str, Tuple[str, str]] = {}
    flips: List[FlipPair] = []
    for card in iter_json_array(CARDS_JSON):
        oracle_text = card_oracle_text(card)
        if (
            oracle_text
            and has_digital_only_oracle_marker(oracle_text)
            and is_non_paper_print(card)
        ):
            continue

        layout = (card.get("layout") or "").strip().lower()
        faces = card.get("card_faces") or []

        # Flip cards need both faces available at runtime for the Flip effect.
        # We generate both faces and wire `Card.other_face` on the front face.
        if layout == "flip" and isinstance(faces, list) and len(faces) >= 2:
            front = faces[0]
            back = faces[1]
            combined_name = (card.get("name") or "").strip()
            if not isinstance(front, dict) or not isinstance(back, dict) or not combined_name:
                continue

            def parse_block_for_face(face: dict) -> Tuple[str, str] | None:
                name = (face.get("name") or "").strip()
                mana_cost = face.get("mana_cost")
                type_line = face.get("type_line")
                oracle_text = face.get("oracle_text")
                power = face.get("power")
                toughness = face.get("toughness")
                loyalty = face.get("loyalty")
                defense = face.get("defense")

                if not name or not type_line or not oracle_text:
                    return None
                if is_non_playable(card, type_line, oracle_text):
                    return None

                lines = []
                if mana_cost:
                    lines.append(f"Mana cost: {mana_cost}")
                lines.append(f"Type: {type_line}")
                if power is not None and toughness is not None:
                    lines.append(f"Power/Toughness: {power}/{toughness}")
                if loyalty is not None:
                    lines.append(f"Loyalty: {loyalty}")
                if defense is not None:
                    lines.append(f"Defense: {defense}")
                lines.append(oracle_text)
                return (name, "\n".join(lines).strip())

            front_pair = parse_block_for_face(front)
            back_pair = parse_block_for_face(back)
            if not front_pair or not back_pair:
                continue

            front_name, front_parse_block = front_pair
            back_name, back_parse_block = back_pair

            if (
                front_name.casefold() in excluded_name_keys
                or back_name.casefold() in excluded_name_keys
                or combined_name.casefold() in excluded_name_keys
            ):
                continue

            flips.append(
                (front_name, front_parse_block, back_name, back_parse_block, combined_name)
            )
            continue

        block = build_block(card)
        if not block:
            continue
        lines = block.splitlines()
        first_line = lines[0] if lines else ""
        if not first_line.startswith("Name: "):
            continue
        name = first_line.removeprefix("Name: ").strip()
        if not name:
            continue
        # Keep metadata + oracle text only. Name is provided by the Rust builder.
        # Leaving "Name:" inside the parse input causes strict parser failures.
        parse_block = "\n".join(lines[1:]).strip()
        key = name.casefold()
        if key in excluded_name_keys:
            continue
        if key not in unique:
            unique[key] = (name, parse_block)
    return unique, flips


def write_generated_source(
    cards: Dict[str, Tuple[str, str]], flips: List[FlipPair], output_path: Path
) -> None:
    ordered = sorted(cards.values(), key=lambda pair: pair[0].casefold())
    flips_ordered = sorted(flips, key=lambda pair: pair[0].casefold())
    payload_path = output_path.parent / PAYLOAD_FILE_NAME
    write_generated_payload(ordered, flips_ordered, payload_path)

    lines = []
    lines.append("// @generated by scripts/generate_baked_registry.py")
    lines.append("// Do not edit manually.")
    lines.append("")
    lines.append("use super::{CardDefinition, CardDefinitionBuilder, CardRegistry};")
    lines.append("use crate::ids::CardId;")
    lines.append("use std::sync::OnceLock;")
    lines.append("")
    lines.append(
        f"pub const GENERATED_PARSER_CARD_SOURCE_COUNT: usize = {len(ordered) + 2 * len(flips_ordered)};"
    )
    lines.append("")
    lines.append(
        f'const GENERATED_REGISTRY_PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/{PAYLOAD_FILE_NAME}"));'
    )
    lines.append("")
    lines.append("#[derive(Clone)]")
    lines.append("struct SingleCardText {")
    lines.append("    name: String,")
    lines.append("    block: String,")
    lines.append("}")
    lines.append("")
    lines.append("#[derive(Clone)]")
    lines.append("struct FlipCardText {")
    lines.append("    front_name: String,")
    lines.append("    front_block: String,")
    lines.append("    back_name: String,")
    lines.append("    back_block: String,")
    lines.append("    combined_name: String,")
    lines.append("}")
    lines.append("")
    lines.append("struct GeneratedCardTexts {")
    lines.append("    singles: Vec<SingleCardText>,")
    lines.append("    flips: Vec<FlipCardText>,")
    lines.append("}")
    lines.append("")
    lines.append("fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {")
    lines.append("    let end = cursor.checked_add(4)?;")
    lines.append("    let chunk = bytes.get(*cursor..end)?;")
    lines.append("    let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);")
    lines.append("    *cursor = end;")
    lines.append("    Some(value)")
    lines.append("}")
    lines.append("")
    lines.append("fn read_string(bytes: &[u8], cursor: &mut usize) -> Option<String> {")
    lines.append("    let length = read_u32(bytes, cursor)? as usize;")
    lines.append("    let end = cursor.checked_add(length)?;")
    lines.append("    let chunk = bytes.get(*cursor..end)?;")
    lines.append("    let text = std::str::from_utf8(chunk).ok()?.to_string();")
    lines.append("    *cursor = end;")
    lines.append("    Some(text)")
    lines.append("}")
    lines.append("")
    lines.append("fn decode_generated_registry_payload() -> GeneratedCardTexts {")
    lines.append("    let bytes = GENERATED_REGISTRY_PAYLOAD;")
    lines.append('    assert!(bytes.starts_with(b"MGR1"), "invalid generated registry payload magic");')
    lines.append("")
    lines.append("    let mut cursor = 4usize;")
    lines.append('    let singles_count = read_u32(bytes, &mut cursor).expect("missing singles count");')
    lines.append("    let mut singles = Vec::with_capacity(singles_count as usize);")
    lines.append("    for _ in 0..singles_count {")
    lines.append('        let name = read_string(bytes, &mut cursor).expect("missing single-card name");')
    lines.append('        let block = read_string(bytes, &mut cursor).expect("missing single-card block");')
    lines.append("        singles.push(SingleCardText { name, block });")
    lines.append("    }")
    lines.append("")
    lines.append('    let flips_count = read_u32(bytes, &mut cursor).expect("missing flip-card count");')
    lines.append("    let mut flips = Vec::with_capacity(flips_count as usize);")
    lines.append("    for _ in 0..flips_count {")
    lines.append('        let front_name = read_string(bytes, &mut cursor).expect("missing flip front name");')
    lines.append(
        '        let front_block = read_string(bytes, &mut cursor).expect("missing flip front block");'
    )
    lines.append('        let back_name = read_string(bytes, &mut cursor).expect("missing flip back name");')
    lines.append(
        '        let back_block = read_string(bytes, &mut cursor).expect("missing flip back block");'
    )
    lines.append(
        '        let combined_name = read_string(bytes, &mut cursor).expect("missing flip combined name");'
    )
    lines.append("        flips.push(FlipCardText {")
    lines.append("            front_name,")
    lines.append("            front_block,")
    lines.append("            back_name,")
    lines.append("            back_block,")
    lines.append("            combined_name,")
    lines.append("        });")
    lines.append("    }")
    lines.append("")
    lines.append("    assert_eq!(")
    lines.append("        cursor,")
    lines.append("        bytes.len(),")
    lines.append('        "generated registry payload has trailing bytes"')
    lines.append("    );")
    lines.append("")
    lines.append("    GeneratedCardTexts { singles, flips }")
    lines.append("}")
    lines.append("")
    lines.append("fn generated_card_texts() -> &'static GeneratedCardTexts {")
    lines.append("    static TEXTS: OnceLock<GeneratedCardTexts> = OnceLock::new();")
    lines.append("    TEXTS.get_or_init(decode_generated_registry_payload)")
    lines.append("}")
    lines.append("")
    lines.append("fn parse_generated_card(cards: &mut Vec<CardDefinition>, name: &str, block: &str) {")
    lines.append("    let builder = CardDefinitionBuilder::new(CardId::new(), name);")
    lines.append("    if let Ok(definition) = builder.parse_text(block.to_string()) {")
    lines.append("        if super::generated_definition_is_supported(&definition) {")
    lines.append("            cards.push(definition);")
    lines.append("        }")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    lines.append("fn parse_generated_flip_card(")
    lines.append("    cards: &mut Vec<CardDefinition>,")
    lines.append("    front_name: &str,")
    lines.append("    front_block: &str,")
    lines.append("    back_name: &str,")
    lines.append("    back_block: &str,")
    lines.append(") {")
    lines.append("    let front_id = CardId::new();")
    lines.append("    let back_id = CardId::new();")
    lines.append("    let front_builder = CardDefinitionBuilder::new(front_id, front_name);")
    lines.append("    let back_builder = CardDefinitionBuilder::new(back_id, back_name);")
    lines.append("    let Ok(mut front) = front_builder.parse_text(front_block.to_string()) else { return; };")
    lines.append("    let Ok(back) = back_builder.parse_text(back_block.to_string()) else { return; };")
    lines.append("    if !super::generated_definition_is_supported(&front) { return; }")
    lines.append("    if !super::generated_definition_is_supported(&back) { return; }")
    lines.append("    front.card.other_face = Some(back_id);")
    lines.append("    cards.push(front);")
    lines.append("    cards.push(back);")
    lines.append("}")
    lines.append("")
    lines.append("fn parsed_generated_cards() -> &'static Vec<CardDefinition> {")
    lines.append("    static PARSED: OnceLock<Vec<CardDefinition>> = OnceLock::new();")
    lines.append("    PARSED.get_or_init(|| {")
    lines.append("        let mut cards = Vec::new();")
    lines.append("        let texts = generated_card_texts();")
    lines.append("        for entry in &texts.singles {")
    lines.append("            parse_generated_card(&mut cards, entry.name.as_str(), entry.block.as_str());")
    lines.append("        }")
    lines.append("        for entry in &texts.flips {")
    lines.append(
        "            parse_generated_flip_card(&mut cards, entry.front_name.as_str(), entry.front_block.as_str(), entry.back_name.as_str(), entry.back_block.as_str());"
    )
    lines.append("        }")
    lines.append("        cards")
    lines.append("    })")
    lines.append("}")
    lines.append("")
    lines.append("fn register_parsed_cards(registry: &mut CardRegistry, parsed: Vec<CardDefinition>) {")
    lines.append("    for definition in parsed {")
    lines.append("        if registry.get(definition.card.name.as_str()).is_none() {")
    lines.append("            registry.register(definition);")
    lines.append("        }")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    lines.append("pub fn register_generated_parser_cards(registry: &mut CardRegistry) {")
    lines.append("    register_parsed_cards(registry, parsed_generated_cards().clone());")
    lines.append("    for entry in &generated_card_texts().flips {")
    lines.append(
        "        registry.register_alias(entry.combined_name.as_str(), entry.front_name.as_str());"
    )
    lines.append("    }")
    lines.append("}")
    lines.append("")
    lines.append("pub fn generated_parser_entry_count() -> usize {")
    lines.append("    let texts = generated_card_texts();")
    lines.append("    texts.singles.len() + texts.flips.len()")
    lines.append("}")
    lines.append("")
    lines.append("pub fn generated_parser_card_names() -> Vec<String> {")
    lines.append("    let texts = generated_card_texts();")
    lines.append("    let mut names = Vec::with_capacity(texts.singles.len() + texts.flips.len());")
    lines.append("    for entry in &texts.singles {")
    lines.append("        names.push(entry.name.clone());")
    lines.append("    }")
    lines.append("    for entry in &texts.flips {")
    lines.append("        names.push(entry.front_name.clone());")
    lines.append("    }")
    lines.append("    names")
    lines.append("}")
    lines.append("")
    lines.append(
        "pub fn register_generated_parser_cards_chunk(registry: &mut CardRegistry, cursor: usize, chunk_size: usize) -> usize {"
    )
    lines.append("    let texts = generated_card_texts();")
    lines.append("    let singles_len = texts.singles.len();")
    lines.append("    let total = singles_len + texts.flips.len();")
    lines.append("    if total == 0 {")
    lines.append("        return 0;")
    lines.append("    }")
    lines.append("    let mut index = cursor.min(total);")
    lines.append("    if index >= total {")
    lines.append("        return total;")
    lines.append("    }")
    lines.append("    let step = chunk_size.max(1);")
    lines.append("    let end = index.saturating_add(step).min(total);")
    lines.append("    while index < end {")
    lines.append("        if index < singles_len {")
    lines.append("            let entry = &texts.singles[index];")
    lines.append("            let mut parsed = Vec::new();")
    lines.append(
        "            parse_generated_card(&mut parsed, entry.name.as_str(), entry.block.as_str());"
    )
    lines.append("            register_parsed_cards(registry, parsed);")
    lines.append("        } else {")
    lines.append("            let entry = &texts.flips[index - singles_len];")
    lines.append("            let mut parsed = Vec::new();")
    lines.append(
        "            parse_generated_flip_card(&mut parsed, entry.front_name.as_str(), entry.front_block.as_str(), entry.back_name.as_str(), entry.back_block.as_str());"
    )
    lines.append("            register_parsed_cards(registry, parsed);")
    lines.append(
        "            registry.register_alias(entry.combined_name.as_str(), entry.front_name.as_str());"
    )
    lines.append("        }")
    lines.append("        index += 1;")
    lines.append("    }")
    lines.append("    index")
    lines.append("}")
    lines.append("")
    lines.append("pub fn register_generated_parser_cards_if_name<F>(")
    lines.append("    registry: &mut CardRegistry,")
    lines.append("    mut include_name: F,")
    lines.append(") where")
    lines.append("    F: FnMut(&str) -> bool,")
    lines.append("{")
    lines.append("    let texts = generated_card_texts();")
    lines.append("    for entry in &texts.singles {")
    lines.append("        if !include_name(entry.name.as_str()) {")
    lines.append("            continue;")
    lines.append("        }")
    lines.append("        let mut parsed = Vec::new();")
    lines.append(
        "        parse_generated_card(&mut parsed, entry.name.as_str(), entry.block.as_str());"
    )
    lines.append("        register_parsed_cards(registry, parsed);")
    lines.append("    }")
    lines.append("    for entry in &texts.flips {")
    lines.append("        if !include_name(entry.front_name.as_str())")
    lines.append("            && !include_name(entry.back_name.as_str())")
    lines.append("            && !include_name(entry.combined_name.as_str())")
    lines.append("        {")
    lines.append("            continue;")
    lines.append("        }")
    lines.append("        let mut parsed = Vec::new();")
    lines.append(
        "        parse_generated_flip_card(&mut parsed, entry.front_name.as_str(), entry.front_block.as_str(), entry.back_name.as_str(), entry.back_block.as_str());"
    )
    lines.append("        register_parsed_cards(registry, parsed);")
    lines.append(
        "        registry.register_alias(entry.combined_name.as_str(), entry.front_name.as_str());"
    )
    lines.append("    }")
    lines.append("}")
    lines.append("")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text("\n".join(lines), encoding="utf-8")


def append_u32(buffer: bytearray, value: int) -> None:
    buffer.extend(struct.pack("<I", value))


def append_string(buffer: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    append_u32(buffer, len(encoded))
    buffer.extend(encoded)


def write_generated_payload(
    ordered: List[Tuple[str, str]], flips_ordered: List[FlipPair], payload_path: Path
) -> None:
    payload = bytearray()
    payload.extend(b"MGR1")
    append_u32(payload, len(ordered))
    for name, block in ordered:
        append_string(payload, name)
        append_string(payload, block)

    append_u32(payload, len(flips_ordered))
    for front_name, front_block, back_name, back_block, combined_name in flips_ordered:
        append_string(payload, front_name)
        append_string(payload, front_block)
        append_string(payload, back_name)
        append_string(payload, back_block)
        append_string(payload, combined_name)

    payload_path.write_bytes(payload)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate baked parser registry source from cards.json"
    )
    parser.add_argument(
        "--out",
        dest="out",
        default=str(OUT_FILE),
        help="Output Rust source file path",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    output_path = Path(args.out)
    excluded_name_keys = load_excluded_name_keys()
    cards, flips = collect_unique_blocks(excluded_name_keys)
    write_generated_source(cards, flips, output_path)
    print(
        f"wrote {output_path} with {len(cards) + 2 * len(flips)} source cards "
        f"(excluded by threshold: {len(excluded_name_keys)})"
    )


if __name__ == "__main__":
    main()
