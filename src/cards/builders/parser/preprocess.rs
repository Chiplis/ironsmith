use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, LineInfo, MetadataLine, NormalizedLine, OwnedLexToken,
    ParseAnnotations,
};

use super::activation_and_restrictions::parse_single_word_keyword_action;
use super::lexer::lex_line;
use super::parser_support::{
    looks_like_spell_resolution_followup_intro_lexed, spell_card_prefers_resolution_line_merge,
};

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

fn parse_metadata_line(line: &str) -> Result<Option<MetadataLine>, CardTextError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("mana cost:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::ManaCost(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("type line:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::TypeLine(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("type:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::TypeLine(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("power/toughness:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::PowerToughness(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("loyalty:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::Loyalty(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("defense:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::Defense(value.to_string())));
    }

    Ok(None)
}

fn replace_names_with_map(
    line: &str,
    full_name: &str,
    short_name: &str,
    base_offset: usize,
) -> (String, Vec<usize>) {
    fn has_word_boundaries_at(bytes: &[u8], idx: usize, len: usize) -> bool {
        let is_word = |b: u8| b.is_ascii_alphanumeric();
        let start_ok = if idx == 0 {
            true
        } else {
            !is_word(bytes[idx - 1])
        };
        let end = idx + len;
        let end_ok = if end >= bytes.len() {
            true
        } else {
            !is_word(bytes[end])
        };
        start_ok && end_ok
    }

    fn is_single_word_keyword_verb(name: &str) -> bool {
        !name.contains(' ')
            && matches!(
                name,
                "add"
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
                    | "lose"
                    | "gain"
                    | "put"
                    | "sacrifice"
                    | "create"
                    | "investigate"
                    | "remove"
                    | "return"
                    | "exchange"
                    | "become"
                    | "switch"
                    | "skip"
                    | "surveil"
                    | "pay"
            )
    }

    fn is_keyword_ability_name(name: &str) -> bool {
        if name == "first strike" || name == "double strike" || name == "ward" {
            return true;
        }
        if name.contains(' ') {
            return false;
        }
        parse_single_word_keyword_action(name).is_some()
    }

    fn preceded_by_named_keyword(bytes: &[u8], mut idx: usize) -> bool {
        while idx > 0 && !bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        let end = idx;
        while idx > 0 && bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        idx < end && &bytes[idx..end] == b"named"
    }

    fn previous_word(bytes: &[u8], mut idx: usize) -> Option<&[u8]> {
        while idx > 0 && !bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        let end = idx;
        while idx > 0 && bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        (idx < end).then_some(&bytes[idx..end])
    }

    fn next_word(bytes: &[u8], mut idx: usize) -> Option<&[u8]> {
        while idx < bytes.len() && !bytes[idx].is_ascii_alphanumeric() {
            idx += 1;
        }
        let start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_alphanumeric() {
            idx += 1;
        }
        (start < idx).then_some(&bytes[start..idx])
    }

    fn preceded_by_ability_grant_word(bytes: &[u8], idx: usize) -> bool {
        previous_word(bytes, idx)
            .is_some_and(|word| matches!(word, b"has" | b"have" | b"gain" | b"gains"))
    }

    fn token_word_appears_before_sentence_end(bytes: &[u8], mut idx: usize) -> bool {
        while idx < bytes.len() {
            if bytes[idx] == b'.' || bytes[idx] == b';' {
                break;
            }
            if bytes[idx..].starts_with(b"token")
                && has_word_boundaries_at(bytes, idx, "token".len())
            {
                return true;
            }
            if bytes[idx..].starts_with(b"tokens")
                && has_word_boundaries_at(bytes, idx, "tokens".len())
            {
                return true;
            }
            idx += 1;
        }
        false
    }

    fn appears_to_be_created_token_name(bytes: &[u8], idx: usize, name_len: usize) -> bool {
        let Some(prev_word) = previous_word(bytes, idx) else {
            return false;
        };
        if prev_word != b"create" && prev_word != b"creates" {
            return false;
        }
        token_word_appears_before_sentence_end(bytes, idx + name_len)
    }

    fn should_preserve_single_word_keyword_verb_usage(
        original: &str,
        idx: usize,
        len: usize,
        keyword: &str,
    ) -> bool {
        if !is_single_word_keyword_verb(keyword) {
            return false;
        }
        let Some(slice) = original.as_bytes().get(idx..idx + len) else {
            return false;
        };
        !slice.iter().any(|byte| byte.is_ascii_uppercase())
    }

    fn is_short_name_self_reference_context(bytes: &[u8], idx: usize, len: usize) -> bool {
        let prev = previous_word(bytes, idx);
        let next = next_word(bytes, idx + len);
        let next_char = bytes.get(idx + len).copied();
        let apostrophe_s = matches!(next_char, Some(b'\''))
            && bytes
                .get(idx + len + 1)
                .is_some_and(|byte| matches!(*byte, b's' | b'S'));

        prev.is_some_and(|word| {
            matches!(
                word,
                b"when"
                    | b"whenever"
                    | b"if"
                    | b"as"
                    | b"until"
                    | b"during"
                    | b"at"
                    | b"after"
                    | b"before"
                    | b"transform"
                    | b"transformed"
                    | b"exile"
                    | b"return"
                    | b"put"
                    | b"on"
                    | b"to"
            )
        }) || next.is_some_and(|word| {
            matches!(
                word,
                b"enter"
                    | b"enters"
                    | b"leave"
                    | b"leaves"
                    | b"die"
                    | b"dies"
                    | b"attack"
                    | b"attacks"
                    | b"block"
                    | b"blocks"
                    | b"become"
                    | b"becomes"
                    | b"becoming"
                    | b"is"
                    | b"has"
                    | b"have"
                    | b"get"
                    | b"gets"
                    | b"deal"
                    | b"deals"
                    | b"dealt"
                    | b"can"
                    | b"cant"
                    | b"would"
                    | b"remains"
                    | b"onto"
                    | b"power"
                    | b"toughness"
                    | b"s"
            )
        }) || apostrophe_s
    }

    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let full_bytes = full_name.as_bytes();
    let short_bytes = short_name.as_bytes();

    let mut out = String::new();
    let mut map = Vec::new();
    let mut idx = 0;

    while idx < bytes.len() {
        if !full_bytes.is_empty()
            && bytes[idx..].starts_with(full_bytes)
            && has_word_boundaries_at(bytes, idx, full_bytes.len())
            && !(idx == 0 && is_single_word_keyword_verb(full_name))
            && !(is_keyword_ability_name(full_name) && preceded_by_ability_grant_word(bytes, idx))
            && !preceded_by_named_keyword(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, full_bytes.len())
            && !should_preserve_single_word_keyword_verb_usage(
                line,
                idx,
                full_bytes.len(),
                full_name,
            )
        {
            let name_len = full_bytes.len().max(1);
            for j in 0..4 {
                out.push("this".chars().nth(j).unwrap());
                let mapped = base_offset + idx + (j * name_len / 4);
                map.push(mapped);
            }
            idx += full_bytes.len();
            continue;
        }
        if !short_bytes.is_empty()
            && bytes[idx..].starts_with(short_bytes)
            && has_word_boundaries_at(bytes, idx, short_bytes.len())
            && !(idx == 0 && is_single_word_keyword_verb(short_name))
            && !(is_keyword_ability_name(short_name) && preceded_by_ability_grant_word(bytes, idx))
            && !preceded_by_named_keyword(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, short_bytes.len())
            && is_short_name_self_reference_context(bytes, idx, short_bytes.len())
            && !should_preserve_single_word_keyword_verb_usage(
                line,
                idx,
                short_bytes.len(),
                short_name,
            )
        {
            let name_len = short_bytes.len().max(1);
            for j in 0..4 {
                out.push("this".chars().nth(j).unwrap());
                let mapped = base_offset + idx + (j * name_len / 4);
                map.push(mapped);
            }
            idx += short_bytes.len();
            continue;
        }

        let ch = lower[idx..].chars().next().unwrap();
        out.push(ch);
        map.push(base_offset + idx);
        idx += ch.len_utf8();
    }

    (out, map)
}

fn strip_parenthetical_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
    let mut out = String::new();
    let mut out_map = Vec::new();
    let mut depth = 0u32;
    let mut char_idx = 0usize;

    for ch in text.chars() {
        if ch == '(' {
            depth += 1;
            char_idx += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            char_idx += 1;
            continue;
        }
        if depth == 0 {
            out.push(ch);
            if let Some(mapped) = map.get(char_idx).copied() {
                out_map.push(mapped);
            }
        }
        char_idx += 1;
    }

    (out, out_map)
}

fn is_labeled_ability_word_prefix(prefix: &str) -> bool {
    let words: Vec<&str> = prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return false;
    }

    if words.len() == 2 && words[0] == "descend" && words[1].chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }

    if matches!(
        words.as_slice(),
        ["spell", "mastery"]
            | ["totem", "armor"]
            | ["fateful", "hour"]
            | ["join", "forces"]
            | ["pack", "tactics"]
            | ["max", "speed"]
            | ["leading", "from", "the", "front"]
            | ["summary", "execution"]
            | ["will", "of", "the", "council"]
            | ["guardian", "protocols"]
            | ["jolly", "gutpipes"]
            | ["protection", "fighting", "style"]
            | ["relentless", "march"]
            | ["secret", "of", "the", "soul"]
            | ["secrets", "of", "the", "soul"]
            | ["flurry", "of", "blows"]
            | ["gust", "of", "wind"]
            | ["reverberating", "summons"]
    ) {
        return true;
    }

    matches!(
        words[0],
        "adamant"
            | "addendum"
            | "alliance"
            | "ascend"
            | "battalion"
            | "enrage"
            | "boast"
            | "buyback"
            | "cycling"
            | "bloodrush"
            | "channel"
            | "chroma"
            | "cohort"
            | "constellation"
            | "converge"
            | "corrupted"
            | "coven"
            | "eerie"
            | "equip"
            | "escape"
            | "exhaust"
            | "flashback"
            | "delirium"
            | "domain"
            | "ferocious"
            | "flurry"
            | "formidable"
            | "hellbent"
            | "heroic"
            | "imprint"
            | "inspired"
            | "landfall"
            | "lieutenant"
            | "magecraft"
            | "metalcraft"
            | "morbid"
            | "parley"
            | "partner"
            | "protector"
            | "radiance"
            | "raid"
            | "renew"
            | "replicate"
            | "revolt"
            | "suspend"
            | "spectacle"
            | "strive"
            | "surge"
            | "threshold"
            | "undergrowth"
            | "ward"
    )
}

fn preserve_keyword_prefix_for_parse(prefix: &str) -> bool {
    let words: Vec<&str> = prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    let Some(first) = words.first().copied() else {
        return false;
    };

    matches!(
        first,
        "buyback"
            | "bestow"
            | "cycling"
            | "echo"
            | "equip"
            | "escape"
            | "flashback"
            | "boast"
            | "modular"
            | "replicate"
            | "reinforce"
            | "renew"
            | "spectacle"
            | "strive"
            | "surge"
            | "suspend"
            | "ward"
    )
}

fn starts_with_if_clause(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed == "if" || trimmed.starts_with("if ")
}

fn is_generic_ability_label_prefix(prefix: &str) -> bool {
    let words: Vec<&str> = prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() || words.len() > 4 {
        return false;
    }

    words.iter().all(|word| {
        word.chars().all(|ch| ch.is_ascii_alphanumeric())
            && word.chars().any(|ch| ch.is_ascii_alphabetic())
    })
}

fn strip_labeled_ability_word_prefix_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
    let separator = text
        .find('—')
        .map(|idx| (idx, '—'.len_utf8()))
        .or_else(|| text.find(" - ").map(|idx| (idx, " - ".len())));
    let Some((sep_idx, sep_len)) = separator else {
        return (text.to_string(), map.to_vec());
    };

    let prefix = text[..sep_idx].trim();
    if preserve_keyword_prefix_for_parse(prefix) {
        return (text.to_string(), map.to_vec());
    }

    let mut remainder_start = sep_idx + sep_len;
    while remainder_start < text.len() {
        let ch = text[remainder_start..]
            .chars()
            .next()
            .expect("character must exist");
        if ch.is_whitespace() {
            remainder_start += ch.len_utf8();
        } else {
            break;
        }
    }
    if remainder_start >= text.len() {
        return (text.to_string(), map.to_vec());
    }

    let remainder = text[remainder_start..].to_string();
    let strip_known_label = is_labeled_ability_word_prefix(prefix);
    let strip_generic_conditional_label =
        starts_with_if_clause(&remainder) && is_generic_ability_label_prefix(prefix);
    if !strip_known_label && !strip_generic_conditional_label {
        return (text.to_string(), map.to_vec());
    }

    let remainder_char_start = text[..remainder_start].chars().count();
    let remainder_map = if remainder_char_start < map.len() {
        map[remainder_char_start..].to_vec()
    } else {
        Vec::new()
    };
    (remainder, remainder_map)
}

fn normalize_line_for_parse(
    line: &str,
    full_name: &str,
    short_name: &str,
) -> Option<NormalizedLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (replaced, map) = replace_names_with_map(trimmed, full_name, short_name, 0);
    let (label_stripped, label_map) = strip_labeled_ability_word_prefix_with_map(&replaced, &map);
    let (stripped, stripped_map) = strip_parenthetical_with_map(&label_stripped, &label_map);

    if stripped.trim().is_empty() {
        let is_wrapped = trimmed.starts_with('(') && trimmed.ends_with(')');
        if !is_wrapped {
            return None;
        }
        let inner = trimmed.trim_start_matches('(').trim_end_matches(')').trim();
        if inner.is_empty() {
            return None;
        }
        let should_parse = inner.contains(':');
        if !should_parse {
            return None;
        }
        let base_offset = trimmed.find(inner).unwrap_or(0);
        let (inner_replaced, inner_map) =
            replace_names_with_map(inner, full_name, short_name, base_offset);
        return Some(NormalizedLine {
            original: trimmed.to_string(),
            normalized: inner_replaced,
            char_map: inner_map,
        });
    }

    Some(NormalizedLine {
        original: trimmed.to_string(),
        normalized: stripped,
        char_map: stripped_map,
    })
}

fn is_ignorable_unparsed_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.starts_with('(') && trimmed.ends_with(')')
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
                && lex_line(split_line.as_str(), virtual_line_index)
                    .ok()
                    .is_some_and(|tokens| looks_like_spell_resolution_followup_intro_lexed(&tokens))
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
