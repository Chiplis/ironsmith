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
SCORES_FILE_ENV = "IRONSMITH_GENERATED_REGISTRY_SCORES_FILE"

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


def load_semantic_scores() -> tuple[Dict[str, float], bool]:
    """Load card-name -> similarity-score map.

    Supported formats:
    - audits report object with `entries` list containing `name` + `similarity_score`
    - plain object mapping `{ "Card Name": 0.98, ... }`
    - list of objects containing `name` + `similarity_score`
    """
    raw_path = os.environ.get(SCORES_FILE_ENV, "").strip()
    if not raw_path:
        return {}, False

    path = Path(raw_path)
    if not path.exists():
        raise FileNotFoundError(
            f"[generate_baked_registry] scores file not found: {path}"
        )

    payload = json.loads(path.read_text(encoding="utf-8"))
    score_map: Dict[str, float] = {}

    def coerce_score(raw: object) -> float | None:
        try:
            score = float(raw)  # type: ignore[arg-type]
        except (TypeError, ValueError):
            return None
        return max(0.0, min(1.0, score))

    def maybe_insert(name_raw: object, score_raw: object) -> None:
        if not isinstance(name_raw, str):
            return
        name = name_raw.strip()
        score = coerce_score(score_raw)
        if not name or score is None:
            return
        key = name.casefold()
        prev = score_map.get(key)
        if prev is None or score > prev:
            score_map[key] = score

    if isinstance(payload, dict):
        entries = payload.get("entries")
        if isinstance(entries, list):
            for entry in entries:
                if not isinstance(entry, dict):
                    continue
                parse_error = entry.get("parse_error")
                has_unimplemented = bool(entry.get("has_unimplemented", False))
                if parse_error is not None or has_unimplemented:
                    continue
                maybe_insert(entry.get("name"), entry.get("similarity_score"))
        else:
            for name, score in payload.items():
                maybe_insert(name, score)
    elif isinstance(payload, list):
        for entry in payload:
            if not isinstance(entry, dict):
                continue
            maybe_insert(entry.get("name"), entry.get("similarity_score"))

    return score_map, True


SingleEntry = Tuple[str, str, float]
FlipPair = Tuple[str, str, float, str, str, float, str]


def collect_unique_blocks(
    semantic_scores: Dict[str, float],
    strict_scores: bool,
) -> Tuple[Dict[str, SingleEntry], List[FlipPair]]:
    unique: Dict[str, SingleEntry] = {}
    flips: List[FlipPair] = []
    missing_scores: List[str] = []

    def resolve_score(*candidates: str) -> float | None:
        for name in candidates:
            key = (name or "").strip().casefold()
            if not key:
                continue
            score = semantic_scores.get(key)
            if score is not None:
                return score
        return None

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

            front_score = resolve_score(front_name, combined_name)
            back_score = resolve_score(back_name, combined_name)
            if front_score is None:
                missing_scores.append(front_name)
                front_score = 1.0
            if back_score is None:
                missing_scores.append(back_name)
                back_score = 1.0

            flips.append(
                (
                    front_name,
                    front_parse_block,
                    front_score,
                    back_name,
                    back_parse_block,
                    back_score,
                    combined_name,
                )
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
        score = resolve_score(name)
        if score is None:
            missing_scores.append(name)
            score = 1.0
        if key not in unique:
            unique[key] = (name, parse_block, score)

    if strict_scores and missing_scores:
        unique_missing = sorted(set(missing_scores))
        preview = ", ".join(unique_missing[:12])
        suffix = "" if len(unique_missing) <= 12 else f", ... (+{len(unique_missing) - 12} more)"
        raise RuntimeError(
            f"[generate_baked_registry] semantic scores missing for {len(unique_missing)} card(s): {preview}{suffix}"
        )

    return unique, flips


def write_generated_source(
    cards: Dict[str, SingleEntry], flips: List[FlipPair], output_path: Path
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
    lines.append("use std::collections::HashMap;")
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
    lines.append("    score: f32,")
    lines.append("}")
    lines.append("")
    lines.append("#[derive(Clone)]")
    lines.append("struct FlipCardText {")
    lines.append("    front_name: String,")
    lines.append("    front_block: String,")
    lines.append("    front_score: f32,")
    lines.append("    back_name: String,")
    lines.append("    back_block: String,")
    lines.append("    back_score: f32,")
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
    lines.append("fn read_f32(bytes: &[u8], cursor: &mut usize) -> Option<f32> {")
    lines.append("    let end = cursor.checked_add(4)?;")
    lines.append("    let chunk = bytes.get(*cursor..end)?;")
    lines.append("    let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);")
    lines.append("    *cursor = end;")
    lines.append("    Some(value)")
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
    lines.append('        let score = read_f32(bytes, &mut cursor).expect("missing single-card score");')
    lines.append("        singles.push(SingleCardText { name, block, score });")
    lines.append("    }")
    lines.append("")
    lines.append('    let flips_count = read_u32(bytes, &mut cursor).expect("missing flip-card count");')
    lines.append("    let mut flips = Vec::with_capacity(flips_count as usize);")
    lines.append("    for _ in 0..flips_count {")
    lines.append('        let front_name = read_string(bytes, &mut cursor).expect("missing flip front name");')
    lines.append(
        '        let front_block = read_string(bytes, &mut cursor).expect("missing flip front block");'
    )
    lines.append(
        '        let front_score = read_f32(bytes, &mut cursor).expect("missing flip front score");'
    )
    lines.append('        let back_name = read_string(bytes, &mut cursor).expect("missing flip back name");')
    lines.append(
        '        let back_block = read_string(bytes, &mut cursor).expect("missing flip back block");'
    )
    lines.append(
        '        let back_score = read_f32(bytes, &mut cursor).expect("missing flip back score");'
    )
    lines.append(
        '        let combined_name = read_string(bytes, &mut cursor).expect("missing flip combined name");'
    )
    lines.append("        flips.push(FlipCardText {")
    lines.append("            front_name,")
    lines.append("            front_block,")
    lines.append("            front_score,")
    lines.append("            back_name,")
    lines.append("            back_block,")
    lines.append("            back_score,")
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
    lines.append("struct GeneratedSemanticData {")
    lines.append("    scores_by_name: HashMap<String, f32>,")
    lines.append("    threshold_counts: [usize; 100],")
    lines.append("}")
    lines.append("")
    lines.append("fn generated_semantic_data() -> &'static GeneratedSemanticData {")
    lines.append("    static DATA: OnceLock<GeneratedSemanticData> = OnceLock::new();")
    lines.append("    DATA.get_or_init(|| {")
    lines.append("        let texts = generated_card_texts();")
    lines.append("        let mut scores_by_name: HashMap<String, f32> = HashMap::new();")
    lines.append("        for entry in &texts.singles {")
    lines.append("            scores_by_name")
    lines.append("                .entry(entry.name.to_lowercase())")
    lines.append("                .and_modify(|score| *score = (*score).max(entry.score))")
    lines.append("                .or_insert(entry.score);")
    lines.append("        }")
    lines.append("        for entry in &texts.flips {")
    lines.append("            scores_by_name")
    lines.append("                .entry(entry.front_name.to_lowercase())")
    lines.append("                .and_modify(|score| *score = (*score).max(entry.front_score))")
    lines.append("                .or_insert(entry.front_score);")
    lines.append("            scores_by_name")
    lines.append("                .entry(entry.back_name.to_lowercase())")
    lines.append("                .and_modify(|score| *score = (*score).max(entry.back_score))")
    lines.append("                .or_insert(entry.back_score);")
    lines.append("            scores_by_name")
    lines.append("                .entry(entry.combined_name.to_lowercase())")
    lines.append("                .and_modify(|score| *score = (*score).max(entry.front_score))")
    lines.append("                .or_insert(entry.front_score);")
    lines.append("        }")
    lines.append("")
    lines.append("        let mut threshold_counts = [0usize; 100];")
    lines.append("        for score in scores_by_name.values().copied() {")
    lines.append("            let clamped = score.clamp(0.0, 1.0);")
    lines.append("            for threshold_index in 0..100usize {")
    lines.append("                let threshold = (threshold_index + 1) as f32 / 100.0;")
    lines.append("                if clamped >= threshold {")
    lines.append("                    threshold_counts[threshold_index] += 1;")
    lines.append("                }")
    lines.append("            }")
    lines.append("        }")
    lines.append("")
    lines.append("        GeneratedSemanticData {")
    lines.append("            scores_by_name,")
    lines.append("            threshold_counts,")
    lines.append("        }")
    lines.append("    })")
    lines.append("}")
    lines.append("")
    lines.append("pub fn generated_parser_semantic_score(name: &str) -> Option<f32> {")
    lines.append("    let normalized = name.trim().to_lowercase();")
    lines.append("    if normalized.is_empty() {")
    lines.append("        return None;")
    lines.append("    }")
    lines.append("    generated_semantic_data().scores_by_name.get(&normalized).copied()")
    lines.append("}")
    lines.append("")
    lines.append("pub fn generated_parser_semantic_threshold_counts() -> [usize; 100] {")
    lines.append("    generated_semantic_data().threshold_counts")
    lines.append("}")
    lines.append("")
    lines.append("pub fn generated_parser_semantic_scored_count() -> usize {")
    lines.append("    generated_semantic_data().scores_by_name.len()")
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
    lines.append("pub fn try_compile_card_by_name(name: &str) -> Result<CardDefinition, String> {")
    lines.append("    let texts = generated_card_texts();")
    lines.append("    let normalized = name.trim();")
    lines.append("")
    lines.append("    for entry in &texts.singles {")
    lines.append("        if entry.name.eq_ignore_ascii_case(normalized) {")
    lines.append("            let builder = CardDefinitionBuilder::new(CardId::new(), &entry.name);")
    lines.append("            match builder.parse_text(entry.block.clone()) {")
    lines.append("                Ok(def) => {")
    lines.append("                    if !super::generated_definition_is_supported(&def) {")
    lines.append(
        '                        return Err("card compiled but contains unsupported mechanics".to_string());'
    )
    lines.append("                    }")
    lines.append("                    return Ok(def);")
    lines.append("                }")
    lines.append("                Err(e) => return Err(format!(\"{e:?}\")),")
    lines.append("            }")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append("    for entry in &texts.flips {")
    lines.append("        if entry.front_name.eq_ignore_ascii_case(normalized)")
    lines.append("            || entry.back_name.eq_ignore_ascii_case(normalized)")
    lines.append("            || entry.combined_name.eq_ignore_ascii_case(normalized)")
    lines.append("        {")
    lines.append("            let front_builder = CardDefinitionBuilder::new(CardId::new(), &entry.front_name);")
    lines.append(
        "            let front = front_builder.parse_text(entry.front_block.clone())"
    )
    lines.append(
        '                .map_err(|e| format!("front face: {e:?}"))?;'
    )
    lines.append("            let back_builder = CardDefinitionBuilder::new(CardId::new(), &entry.back_name);")
    lines.append(
        "            back_builder.parse_text(entry.back_block.clone())"
    )
    lines.append(
        '                .map_err(|e| format!("back face: {e:?}"))?;'
    )
    lines.append("            if !super::generated_definition_is_supported(&front) {")
    lines.append(
        '                return Err("card compiled but contains unsupported mechanics".to_string());'
    )
    lines.append("            }")
    lines.append("            return Ok(front);")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append(
        '    Err(format!("card \'{}\' not found in card database", name))'
    )
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


def append_f32(buffer: bytearray, value: float) -> None:
    buffer.extend(struct.pack("<f", value))


def write_generated_payload(
    ordered: List[SingleEntry], flips_ordered: List[FlipPair], payload_path: Path
) -> None:
    payload = bytearray()
    payload.extend(b"MGR1")
    append_u32(payload, len(ordered))
    for name, block, score in ordered:
        append_string(payload, name)
        append_string(payload, block)
        append_f32(payload, score)

    append_u32(payload, len(flips_ordered))
    for (
        front_name,
        front_block,
        front_score,
        back_name,
        back_block,
        back_score,
        combined_name,
    ) in flips_ordered:
        append_string(payload, front_name)
        append_string(payload, front_block)
        append_f32(payload, front_score)
        append_string(payload, back_name)
        append_string(payload, back_block)
        append_f32(payload, back_score)
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
    semantic_scores, strict_scores = load_semantic_scores()
    cards, flips = collect_unique_blocks(semantic_scores, strict_scores)
    write_generated_source(cards, flips, output_path)
    print(
        f"wrote {output_path} with {len(cards) + 2 * len(flips)} source cards "
        f"(semantic scores loaded: {len(semantic_scores)})"
    )


if __name__ == "__main__":
    main()
