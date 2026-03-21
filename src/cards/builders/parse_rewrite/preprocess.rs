#![allow(dead_code)]

use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, LineInfo, MetadataLine, NormalizedLine, OwnedLexToken,
    ParseAnnotations, is_ignorable_unparsed_line, lex_line, normalize_line_for_parse,
    parse_metadata_line, parse_single_word_keyword_action,
};

use super::parser_support::{
    looks_like_spell_resolution_followup_intro, spell_card_prefers_resolution_line_merge,
};
use super::util::tokenize_line;

#[derive(Debug, Clone)]
pub(crate) struct PreprocessedDocument {
    pub(crate) builder: CardDefinitionBuilder,
    pub(crate) annotations: ParseAnnotations,
    pub(crate) items: Vec<PreprocessedItem>,
}

#[derive(Debug, Clone)]
pub(crate) enum PreprocessedItem {
    Metadata(PreprocessedMetadataLine),
    Line(PreprocessedLine),
}

#[derive(Debug, Clone)]
pub(crate) struct PreprocessedMetadataLine {
    pub(crate) info: LineInfo,
    pub(crate) value: MetadataLine,
}

#[derive(Debug, Clone)]
pub(crate) struct PreprocessedLine {
    pub(crate) info: LineInfo,
    pub(crate) tokens: Vec<OwnedLexToken>,
}

fn strip_parenthetical_segments(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len());
    let mut depth = 0u32;

    for ch in line.chars() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_parse_line_variants(line: &str) -> Vec<String> {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("as an additional cost to cast this spell")
        && let Some(period_idx) = line.find('.')
    {
        let first = line[..=period_idx].trim();
        let second = line[period_idx + 1..].trim();
        if !first.is_empty() && !second.is_empty() {
            return vec![first.to_string(), second.to_string()];
        }
    }

    let marker = ". when you spend this mana to cast ";
    let marker_compact = ".when you spend this mana to cast ";
    let split_at = lower.find(marker).or_else(|| lower.find(marker_compact));
    if let Some(idx) = split_at {
        let first = line[..=idx].trim();
        let second = line[idx + 1..].trim();
        if first.contains(':') && !second.is_empty() {
            return vec![first.to_string(), second.to_string()];
        }
    }

    for marker in [
        ". this cost is reduced by ",
        ".this cost is reduced by ",
        ". this ability costs ",
        ".this ability costs ",
        ". this spell costs ",
        ".this spell costs ",
    ] {
        if let Some(idx) = lower.find(marker) {
            let first = line[..=idx].trim();
            let second = line[idx + 1..].trim();
            if !first.is_empty() && !second.is_empty() {
                return vec![first.to_string(), second.to_string()];
            }
        }
    }

    vec![line.to_string()]
}

pub(crate) fn preprocess_document(
    mut builder: CardDefinitionBuilder,
    text: &str,
) -> Result<PreprocessedDocument, CardTextError> {
    fn normalize_card_name_for_self_reference(name: &str) -> String {
        let lower = name.to_ascii_lowercase();
        let bytes = lower.as_bytes();
        if bytes.len() > 2 && bytes[1] == b'-' && bytes[0].is_ascii_alphabetic() {
            lower[2..].to_string()
        } else {
            lower
        }
    }

    fn short_name_for_self_reference(name: &str) -> String {
        fn is_reserved_short_alias(alias_lower: &str) -> bool {
            matches!(
                alias_lower,
                "a" | "an"
                    | "the"
                    | "one"
                    | "two"
                    | "three"
                    | "four"
                    | "five"
                    | "six"
                    | "seven"
                    | "eight"
                    | "nine"
                    | "ten"
                    | "x"
                    | "this"
                    | "that"
                    | "these"
                    | "those"
                    | "you"
                    | "your"
                    | "when"
                    | "whenever"
                    | "if"
                    | "at"
                    | "add"
                    | "move"
                    | "deal"
                    | "draw"
                    | "counter"
                    | "destroy"
                    | "exile"
                    | "untap"
                    | "scry"
                    | "discard"
                    | "transform"
                    | "regenerate"
                    | "mill"
                    | "get"
                    | "reveal"
                    | "look"
                    | "lose"
                    | "gain"
                    | "put"
                    | "sacrifice"
                    | "create"
                    | "investigate"
                    | "attach"
                    | "remove"
                    | "return"
                    | "exchange"
                    | "become"
                    | "switch"
                    | "skip"
                    | "surveil"
                    | "shuffle"
                    | "reorder"
                    | "pay"
                    | "goad"
                    | "power"
                    | "toughness"
                    | "mana"
                    | "life"
                    | "commander"
                    | "player"
                    | "opponent"
                    | "creature"
                    | "artifact"
                    | "enchantment"
                    | "land"
                    | "spell"
                    | "card"
                    | "token"
                    | "permanent"
                    | "library"
                    | "graveyard"
                    | "hand"
                    | "battlefield"
                    | "controller"
                    | "owner"
                    | "planeswalker"
                    | "battle"
                    | "equipment"
                    | "aura"
            ) || parse_single_word_keyword_action(alias_lower).is_some()
        }

        let trimmed = name.trim();
        let comma_short = trimmed.split(',').next().unwrap_or(trimmed).trim();
        if comma_short != trimmed {
            return comma_short.to_string();
        }

        let mut words = trimmed.split_whitespace();
        let Some(first_word) = words.next() else {
            return trimmed.to_string();
        };
        if words.next().is_none() {
            return trimmed.to_string();
        }

        let alias = first_word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-');
        if alias.len() <= 2 {
            return trimmed.to_string();
        }

        let alias_lower = alias.to_ascii_lowercase();
        if is_reserved_short_alias(alias_lower.as_str()) {
            return trimmed.to_string();
        }

        alias.to_string()
    }

    fn normalize_non_metadata_line(
        raw_line: &str,
        line_index: usize,
        full_name: &str,
        short_name: &str,
        annotations: &mut ParseAnnotations,
    ) -> Result<Option<PreprocessedLine>, CardTextError> {
        let stripped = strip_parenthetical_segments(raw_line);
        if stripped.trim().is_empty() {
            return Ok(None);
        }

        let Some(normalized) = normalize_line_for_parse(stripped.as_str(), full_name, short_name)
        else {
            if is_ignorable_unparsed_line(raw_line) {
                return Ok(None);
            }
            return Err(CardTextError::ParseError(format!(
                "rewrite preprocessing could not normalize line: '{raw_line}'"
            )));
        };

        annotations.record_original_line(line_index, &normalized.original);
        annotations.record_normalized_line(line_index, &normalized.normalized);
        annotations.record_char_map(line_index, normalized.char_map.clone());

        let tokens = lex_line(normalized.normalized.as_str(), line_index)?;
        Ok(Some(PreprocessedLine {
            info: LineInfo {
                line_index,
                raw_line: raw_line.trim().to_string(),
                normalized,
            },
            tokens,
        }))
    }

    let card_name = builder.card_builder.name_ref().to_string();
    let front_face_name = card_name
        .split("//")
        .next()
        .unwrap_or(card_name.as_str())
        .trim()
        .to_string();
    let short_name = short_name_for_self_reference(front_face_name.as_str());
    let full_lower = normalize_card_name_for_self_reference(front_face_name.as_str());
    let short_lower = normalize_card_name_for_self_reference(short_name.as_str());
    let mut annotations = ParseAnnotations::default();
    let mut items = Vec::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(meta) = parse_metadata_line(line)? {
            let normalized = NormalizedLine {
                original: line.to_string(),
                normalized: line.to_string(),
                char_map: (0..line.chars().count()).collect(),
            };
            builder = builder.apply_metadata(meta.clone())?;
            annotations.record_original_line(line_index, &normalized.original);
            annotations.record_normalized_line(line_index, &normalized.normalized);
            annotations.record_char_map(line_index, normalized.char_map.clone());
            items.push(PreprocessedItem::Metadata(PreprocessedMetadataLine {
                info: make_line_info(line_index, line, normalized),
                value: meta,
            }));
            continue;
        }

        for (split_index, split_line) in split_parse_line_variants(line).into_iter().enumerate() {
            let virtual_line_index = line_index.saturating_mul(8).saturating_add(split_index);
            if spell_card_prefers_resolution_line_merge(&builder)
                && looks_like_spell_resolution_followup_intro(&tokenize_line(
                    split_line.as_str(),
                    virtual_line_index,
                ))
                && let Some(PreprocessedItem::Line(previous)) = items.last_mut()
            {
                let combined_raw_line =
                    format!("{} {}", previous.info.raw_line.trim(), split_line.trim());
                let Some(normalized) = normalize_line_for_parse(
                    combined_raw_line.as_str(),
                    full_lower.as_str(),
                    short_lower.as_str(),
                ) else {
                    return Err(CardTextError::ParseError(format!(
                        "rewrite preprocessing could not normalize merged line: '{combined_raw_line}'"
                    )));
                };
                annotations.record_original_line(previous.info.line_index, &normalized.original);
                annotations
                    .record_normalized_line(previous.info.line_index, &normalized.normalized);
                annotations.record_char_map(previous.info.line_index, normalized.char_map.clone());
                previous.info.raw_line = combined_raw_line;
                previous.info.normalized = normalized.clone();
                previous.tokens =
                    lex_line(normalized.normalized.as_str(), previous.info.line_index)?;
                continue;
            }
            if let Some(parsed_line) = normalize_non_metadata_line(
                split_line.as_str(),
                virtual_line_index,
                full_lower.as_str(),
                short_lower.as_str(),
                &mut annotations,
            )? {
                items.push(PreprocessedItem::Line(parsed_line));
            }
        }
    }

    if items
        .iter()
        .any(|item| matches!(item, PreprocessedItem::Line(_)))
    {
        let oracle_text = items
            .iter()
            .filter_map(|item| match item {
                PreprocessedItem::Metadata(_) => None,
                PreprocessedItem::Line(line) => Some(line.info.raw_line.as_str()),
            })
            .collect::<Vec<_>>()
            .join("\n");
        let builder = builder.oracle_text(oracle_text);
        return Ok(PreprocessedDocument {
            builder,
            annotations,
            items,
        });
    }

    Ok(PreprocessedDocument {
        builder,
        annotations,
        items,
    })
}

pub(crate) fn make_line_info(
    line_index: usize,
    raw_line: impl Into<String>,
    normalized: NormalizedLine,
) -> LineInfo {
    LineInfo {
        line_index,
        raw_line: raw_line.into(),
        normalized,
    }
}
