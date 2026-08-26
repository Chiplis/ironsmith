use super::grammar::effects::optional_companion_shapes::parse_shared_subject_optional_companion_shape;
use super::grammar::structure::{MetadataLineKind, split_leading_result_prefix_lexed};
use super::grammar::{line_semantic_facts, preprocess as preprocess_grammar};
use super::lexer::{lex_line, split_lexed_sentences};
use super::parser_support::{
    looks_like_spell_resolution_followup_intro_lexed, spell_card_prefers_resolution_line_merge,
};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, LineInfo, MetadataLine, NormalizedLine, OwnedLexToken,
    ParseAnnotations,
};
use crate::model::provenance::{
    ProvenanceStore, ReminderTextDecision, SourceSliceKind, SourceUnitId,
};
use crate::types::CardType;

#[derive(Debug, Clone)]
pub struct PreprocessedDocument {
    pub builder: CardDefinitionBuilder,
    pub annotations: ParseAnnotations,
    pub provenance: ProvenanceStore,
    pub structure: crate::front_end::DocumentStructure,
    pub items: Vec<PreprocessedItem>,
}

#[derive(Debug, Clone)]
pub enum PreprocessedItem {
    Metadata(PreprocessedMetadataLine),
    Line(PreprocessedLine),
}

#[derive(Debug, Clone)]
pub struct PreprocessedMetadataLine {
    pub info: LineInfo,
    pub value: MetadataLine,
}

#[derive(Debug, Clone)]
pub struct PreprocessedLine {
    pub info: LineInfo,
    pub tokens: Vec<OwnedLexToken>,
}

fn bytes_start_with(slice: &[u8], prefix: &[u8]) -> bool {
    if prefix.len() > slice.len() {
        return false;
    }
    for (idx, expected) in prefix.iter().enumerate() {
        if slice[idx] != *expected {
            return false;
        }
    }
    true
}

fn collapse_whitespace_runs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(ch);
    }
    out
}

pub(super) fn strip_parenthetical_segments(line: &str) -> String {
    match preprocess_grammar::parse_parenthetical_line_surface(line) {
        Some(preprocess_grammar::ParentheticalLineSurface::FullyWrapped) => {
            return line.to_string();
        }
        Some(preprocess_grammar::ParentheticalLineSurface::PreserveEnchantmentNotCreature) => {
            return line
                .replace("(It's not a creature.)", "It's not a creature.")
                .replace("(It's not a creature)", "It's not a creature")
                .replace("(it's not a creature.)", "it's not a creature.")
                .replace("(it's not a creature)", "it's not a creature")
                .replace("(Its not a creature.)", "Its not a creature.")
                .replace("(Its not a creature)", "Its not a creature")
                .replace("(its not a creature.)", "its not a creature.")
                .replace("(its not a creature)", "its not a creature");
        }
        None => {}
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

    collapse_whitespace_runs(out.as_str())
}

fn split_parse_line_variants(line: &str) -> Vec<String> {
    if let Some(split) = preprocess_grammar::parse_line_variant_split(line) {
        let first = line.get(..split.first_end).unwrap_or_default().trim();
        let second = line.get(split.second_start..).unwrap_or_default().trim();
        let second_without_reminder = strip_parenthetical_segments(second);
        let is_flashback_scoped_cost_adjustment = split.kind
            == preprocess_grammar::LineVariantSplitKind::CostAdjustmentFollowup
            && preprocess_grammar::is_flashback_scoped_cost_adjustment(
                first,
                second_without_reminder.as_str(),
            );
        if is_flashback_scoped_cost_adjustment {
            // The flashback parser binds "this way" to the alternative cast.
            // Splitting these sentences first silently broadens the reduction
            // to normal casting as well.
            return vec![line.to_string()];
        }
        if split.kind == preprocess_grammar::LineVariantSplitKind::ManaSpendFollowup
            && preprocess_grammar::is_mana_spend_bonus_followup(second_without_reminder.as_str())
        {
            return vec![line.to_string()];
        }
        if !first.is_empty() && !second.is_empty() {
            return vec![first.to_string(), second.to_string()];
        }
    }

    vec![line.to_string()]
}

fn parse_metadata_line(line: &str) -> Result<Option<MetadataLine>, CardTextError> {
    let Some(surface) = preprocess_grammar::parse_metadata_surface(line) else {
        return Ok(None);
    };

    let metadata = match surface.kind {
        MetadataLineKind::ManaCost => MetadataLine::ManaCost(surface.value),
        MetadataLineKind::TypeLine => MetadataLine::TypeLine(surface.value),
        MetadataLineKind::FirstPrintedSet => MetadataLine::FirstPrintedSet(surface.value),
        MetadataLineKind::AttractionLights => MetadataLine::AttractionLights(surface.value),
        MetadataLineKind::PowerToughness => MetadataLine::PowerToughness(surface.value),
        MetadataLineKind::Loyalty => MetadataLine::Loyalty(surface.value),
        MetadataLineKind::Defense => MetadataLine::Defense(surface.value),
    };

    Ok(Some(metadata))
}

fn materialize_structural_metadata(value: &crate::front_end::MetadataLine) -> MetadataLine {
    match value {
        crate::front_end::MetadataLine::ManaCost(value) => MetadataLine::ManaCost(value.clone()),
        crate::front_end::MetadataLine::TypeLine(value) => MetadataLine::TypeLine(value.clone()),
        crate::front_end::MetadataLine::FirstPrintedSet(value) => {
            MetadataLine::FirstPrintedSet(value.clone())
        }
        crate::front_end::MetadataLine::AttractionLights(value) => {
            MetadataLine::AttractionLights(value.clone())
        }
        crate::front_end::MetadataLine::PowerToughness(value) => {
            MetadataLine::PowerToughness(value.clone())
        }
        crate::front_end::MetadataLine::Loyalty(value) => MetadataLine::Loyalty(value.clone()),
        crate::front_end::MetadataLine::Defense(value) => MetadataLine::Defense(value.clone()),
    }
}

fn replace_names_with_map(
    line: &str,
    full_name: &str,
    short_name: &str,
    preserve_source_surfaces: bool,
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
        preprocess_grammar::parse_single_keyword_verb(name).is_some()
    }

    fn starts_with_typed_keyword_action_statement(line: &str) -> bool {
        let Ok(tokens) = lex_line(line, 0) else {
            return false;
        };
        let Some(statement) = split_lexed_sentences(&tokens).into_iter().next() else {
            return false;
        };
        super::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(statement)
            .is_some()
    }

    fn is_keyword_ability_name(name: &str) -> bool {
        preprocess_grammar::parse_keyword_ability_name(name).is_some()
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

    fn is_indefinite_become_descriptor(bytes: &[u8], idx: usize) -> bool {
        let Some(article) = previous_word(bytes, idx) else {
            return false;
        };
        if !matches!(article, b"a" | b"an") {
            return false;
        }
        let mut article_start = idx;
        while article_start > 0 && !bytes[article_start - 1].is_ascii_alphanumeric() {
            article_start -= 1;
        }
        while article_start > 0 && bytes[article_start - 1].is_ascii_alphanumeric() {
            article_start -= 1;
        }
        previous_word(bytes, article_start)
            .is_some_and(|word| matches!(word, b"become" | b"becomes" | b"became" | b"becoming"))
    }

    fn token_word_appears_before_sentence_end(bytes: &[u8], mut idx: usize) -> bool {
        while idx < bytes.len() {
            if bytes[idx] == b'.' || bytes[idx] == b';' {
                break;
            }
            if bytes_start_with(&bytes[idx..], b"token")
                && has_word_boundaries_at(bytes, idx, "token".len())
            {
                return true;
            }
            if bytes_start_with(&bytes[idx..], b"tokens")
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

    fn within_vote_choice_clause(bytes: &[u8], idx: usize) -> bool {
        let mut sentence_start = idx;
        while sentence_start > 0 {
            let prev = bytes[sentence_start - 1];
            if prev == b'.' || prev == b';' {
                break;
            }
            sentence_start -= 1;
        }
        let Some(prefix) = std::str::from_utf8(&bytes[sentence_start..idx]).ok() else {
            return false;
        };
        preprocess_grammar::parse_vote_choice_surface(prefix).is_some()
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

    fn is_result_optional_companion_short_name_context(
        bytes: &[u8],
        idx: usize,
        len: usize,
    ) -> bool {
        let sentence_start = crate::slice_primitives::select_last_position(&bytes[..idx], |byte| {
            matches!(*byte, b'.' | b';')
        })
        .map_or(0, |separator| separator + 1);
        let sentence_end = crate::slice_primitives::select_position(&bytes[idx + len..], |byte| {
            matches!(*byte, b'.' | b';')
        })
        .map_or(bytes.len(), |separator| idx + len + separator);
        let Some(sentence) = std::str::from_utf8(&bytes[sentence_start..sentence_end]).ok() else {
            return false;
        };
        let Ok(tokens) = lex_line(sentence, 0) else {
            return false;
        };
        let Some(prefix) = split_leading_result_prefix_lexed(&tokens) else {
            return false;
        };
        let Some(shape) = parse_shared_subject_optional_companion_shape(prefix.trailing_tokens)
        else {
            return false;
        };
        let Some(first) = shape.first_subject_tokens.first() else {
            return false;
        };
        let Some(last) = shape.first_subject_tokens.last() else {
            return false;
        };
        let local_start = idx - sentence_start;
        first.span.start == local_start && last.span.end == local_start + len
    }

    fn is_created_token_lifecycle_source(bytes: &[u8], idx: usize) -> bool {
        let sentence_start = crate::slice_primitives::select_last_position(&bytes[..idx], |byte| {
            matches!(*byte, b'.' | b';')
        })
        .map_or(0, |separator| separator + 1);
        let Some(prefix) = std::str::from_utf8(&bytes[sentence_start..idx]).ok() else {
            return false;
        };
        let Ok(tokens) = lex_line(prefix, 0) else {
            return false;
        };
        crate::word_primitives::parse_sequence_suffix(
            &crate::lexer::parser_token_word_refs(&tokens),
            &["exile", "that", "token", "when"],
        )
    }

    fn should_preserve_source_surface_context(bytes: &[u8], idx: usize, len: usize) -> bool {
        let prev = previous_word(bytes, idx);
        let next = next_word(bytes, idx + len);
        let next_char = bytes.get(idx + len).copied();
        let apostrophe_s = matches!(next_char, Some(b'\''))
            && bytes
                .get(idx + len + 1)
                .is_some_and(|byte| matches!(*byte, b's' | b'S'));

        if prev.is_some_and(|word| word == b"as") {
            return false;
        }

        // The reciprocal created-token lifecycle is represented by a typed
        // source-linked effect. Normalize the proper-name subject here so
        // the lifecycle grammar sees the same source reference as ordinary
        // `this ...` wording; the original source map still retains the
        // authored name surface.
        if prev == Some(&b"when"[..])
            && next == Some(&b"leaves"[..])
            && is_created_token_lifecycle_source(bytes, idx)
        {
            return false;
        }

        if prev.is_some_and(|word| word == b"is") {
            let mut word_start = idx;
            while word_start > 0 && !bytes[word_start - 1].is_ascii_alphanumeric() {
                word_start -= 1;
            }
            while word_start > 0 && bytes[word_start - 1].is_ascii_alphanumeric() {
                word_start -= 1;
            }
            if previous_word(bytes, word_start).is_some_and(|word| word == b"name") {
                return true;
            }
        }

        if prev.is_some_and(|word| word == b"to") {
            let mut word_start = idx;
            while word_start > 0 && !bytes[word_start - 1].is_ascii_alphanumeric() {
                word_start -= 1;
            }
            while word_start > 0 && bytes[word_start - 1].is_ascii_alphanumeric() {
                word_start -= 1;
            }
            if previous_word(bytes, word_start).is_some_and(|word| word == b"attached") {
                return false;
            }
        }

        apostrophe_s
            || prev.is_some_and(|word| {
                matches!(
                    word,
                    b"attach"
                        | b"destroy"
                        | b"exile"
                        | b"transform"
                        | b"convert"
                        | b"regenerate"
                        | b"return"
                        | b"tap"
                        | b"untap"
                        | b"control"
                        | b"of"
                        | b"than"
                        | b"to"
                        | b"on"
                )
            })
            || next.is_some_and(|word| {
                matches!(
                    word,
                    b"become"
                        | b"becomes"
                        | b"becoming"
                        | b"deal"
                        | b"deals"
                        | b"enter"
                        | b"enters"
                        | b"gain"
                        | b"gains"
                        | b"has"
                        | b"have"
                        | b"leave"
                        | b"leaves"
                        | b"remain"
                        | b"remains"
                        | b"power"
                        | b"toughness"
                )
            })
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
            && bytes_start_with(&bytes[idx..], full_bytes)
            && has_word_boundaries_at(bytes, idx, full_bytes.len())
            && !(idx == 0
                && (is_single_word_keyword_verb(full_name)
                    || starts_with_typed_keyword_action_statement(line)))
            && !(is_keyword_ability_name(full_name) && preceded_by_ability_grant_word(bytes, idx))
            && !preceded_by_named_keyword(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, full_bytes.len())
            && !within_vote_choice_clause(bytes, idx)
            && !is_indefinite_become_descriptor(bytes, idx)
            && !(preserve_source_surfaces
                && should_preserve_source_surface_context(bytes, idx, full_bytes.len()))
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
            && bytes_start_with(&bytes[idx..], short_bytes)
            && has_word_boundaries_at(bytes, idx, short_bytes.len())
            && !(preserve_source_surfaces
                && !full_bytes.is_empty()
                && bytes_start_with(&bytes[idx..], full_bytes)
                && has_word_boundaries_at(bytes, idx, full_bytes.len()))
            && !(idx == 0
                && (is_single_word_keyword_verb(short_name)
                    || starts_with_typed_keyword_action_statement(line)))
            && !(is_keyword_ability_name(short_name) && preceded_by_ability_grant_word(bytes, idx))
            && !preceded_by_named_keyword(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, short_bytes.len())
            && !within_vote_choice_clause(bytes, idx)
            && !is_indefinite_become_descriptor(bytes, idx)
            && (is_short_name_self_reference_context(bytes, idx, short_bytes.len())
                || is_result_optional_companion_short_name_context(bytes, idx, short_bytes.len()))
            && !(preserve_source_surfaces
                && should_preserve_source_surface_context(bytes, idx, short_bytes.len()))
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

fn strip_labeled_ability_word_prefix_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
    let Some(surface) = preprocess_grammar::parse_labeled_ability_prefix(text) else {
        return (text.to_string(), map.to_vec());
    };
    let remainder = text[surface.remainder_start..].to_string();
    let remainder_char_start = text[..surface.remainder_start].chars().count();
    let remainder_map = if remainder_char_start < map.len() {
        map[remainder_char_start..].to_vec()
    } else {
        Vec::new()
    };
    (remainder, remainder_map)
}

fn strip_resolution_timing_tail_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
    let Some(surface) = preprocess_grammar::parse_resolution_timing_tail(text) else {
        return (text.to_string(), map.to_vec());
    };

    let mut out = text[..surface.tail_start].trim_end().to_string();
    let mut out_map = map[..out.chars().count().min(map.len())].to_vec();
    if surface.terminal_period && !preprocess_grammar::parse_terminal_period(out.as_str()) {
        out.push('.');
        out_map.push(
            *map.get(surface.tail_start)
                .unwrap_or_else(|| map.last().unwrap_or(&0)),
        );
    }
    (out, out_map)
}

fn normalize_line_for_parse(
    line: &str,
    full_name: &str,
    short_name: &str,
    preserve_source_surfaces: bool,
) -> Option<NormalizedLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (replaced, map) =
        replace_names_with_map(trimmed, full_name, short_name, preserve_source_surfaces, 0);
    let (label_stripped, label_map) = strip_labeled_ability_word_prefix_with_map(&replaced, &map);
    let (stripped, stripped_map) = strip_parenthetical_with_map(&label_stripped, &label_map);
    let (stripped, stripped_map) = strip_resolution_timing_tail_with_map(&stripped, &stripped_map);

    if stripped.trim().is_empty() {
        let wrapped = preprocess_grammar::parse_wrapped_activation_surface(trimmed)?;
        let (inner_replaced, inner_map) = replace_names_with_map(
            wrapped.inner.as_str(),
            full_name,
            short_name,
            preserve_source_surfaces,
            wrapped.inner_start,
        );
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

fn split_same_is_true_subject_predicate(sentence: &str) -> Option<(String, String)> {
    preprocess_grammar::parse_subject_predicate_surface(sentence)
        .map(|surface| (surface.subject, surface.predicate))
}

fn find_borrow_ability_source_phrase(sentence: &str) -> Option<&'static str> {
    preprocess_grammar::parse_borrow_ability_surface(sentence).map(|surface| surface.phrase)
}

fn apply_borrow_phrase_occurrences(
    text: &str,
    occurrences: &preprocess_grammar::BorrowPhraseOccurrencesSurface,
    replacement: &str,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for range in &occurrences.ranges {
        let Some(prefix) = text.get(cursor..range.start) else {
            return text.to_string();
        };
        out.push_str(prefix);
        out.push_str(replacement);
        cursor = range.end;
    }
    let Some(tail) = text.get(cursor..) else {
        return text.to_string();
    };
    out.push_str(tail);
    out
}

fn rewrite_borrow_static_condition(condition: &str, ability: &str) -> Option<String> {
    match preprocess_grammar::parse_borrow_static_condition_surface(condition, ability)? {
        preprocess_grammar::BorrowStaticConditionSurface::ExiledWithAbility {
            subject,
            tail,
            source_noun,
        } => {
            let tail = source_noun
                .map(|noun| format!("exiled with this {noun}"))
                .unwrap_or(tail);
            Some(format!("there is {subject} {tail} with {ability}"))
        }
        preprocess_grammar::BorrowStaticConditionSurface::HasAbility { subject } => {
            Some(format!("there is {subject} with {ability}"))
        }
        preprocess_grammar::BorrowStaticConditionSurface::InZone {
            plural,
            subject,
            zone_tail,
        } => {
            let intro = if plural { "there are" } else { "there is" };
            Some(format!("{intro} {subject} in {zone_tail}"))
        }
    }
}

fn rewrite_borrow_static_sentence(sentence: &str) -> String {
    let Some(ability) = find_borrow_ability_source_phrase(sentence) else {
        return sentence.to_string();
    };
    match preprocess_grammar::parse_borrow_static_sentence_surface(sentence) {
        Some(preprocess_grammar::BorrowStaticSentenceSurface::Leading {
            condition,
            consequence,
        }) => rewrite_borrow_static_condition(condition.as_str(), ability)
            .map(|rewritten| format!("as long as {rewritten}, {consequence}"))
            .unwrap_or_else(|| sentence.to_string()),
        Some(preprocess_grammar::BorrowStaticSentenceSurface::Trailing { prefix, condition }) => {
            rewrite_borrow_static_condition(condition.as_str(), ability)
                .map(|rewritten| format!("{prefix} as long as {rewritten}"))
                .unwrap_or_else(|| sentence.to_string())
        }
        None => sentence.to_string(),
    }
}

fn expand_borrow_ability_line(text: &str) -> String {
    let Some(document) = preprocess_grammar::parse_preprocess_sentence_list(text) else {
        return rewrite_borrow_static_sentence(text.trim());
    };
    if document.sentences.len() < 2 {
        return rewrite_borrow_static_sentence(text.trim());
    }

    let mut expanded: Vec<String> = Vec::new();
    for sentence in document.sentences {
        if let Some(same_is_true) =
            preprocess_grammar::parse_same_is_true_surface(sentence.as_str())
            && let Some(base_sentence) = expanded.last().cloned()
        {
            let targets = same_is_true.targets;
            if !targets.is_empty() {
                if let Some(source_phrase) =
                    find_borrow_ability_source_phrase(base_sentence.as_str())
                    && let Some(occurrences) = preprocess_grammar::parse_borrow_phrase_occurrences(
                        base_sentence.as_str(),
                        source_phrase,
                    )
                {
                    for target in &targets {
                        let replaced = apply_borrow_phrase_occurrences(
                            base_sentence.as_str(),
                            &occurrences,
                            target.as_str(),
                        );
                        expanded.push(rewrite_borrow_static_sentence(replaced.as_str()));
                    }
                    continue;
                }

                if let Some((_subject, predicate)) =
                    split_same_is_true_subject_predicate(base_sentence.as_str())
                {
                    for target in &targets {
                        expanded.push(format!("{} {}", target.trim(), predicate));
                    }
                    continue;
                }
            }
        }

        expanded.push(rewrite_borrow_static_sentence(sentence.as_str()));
    }

    let mut joined = expanded.join(". ");
    if document.terminal_period {
        joined.push('.');
    }
    joined
}

fn rewrite_vote_count_followups_line(text: &str) -> String {
    fn rewrite_vote_count_sentence(sentence: &str) -> String {
        let trimmed = sentence.trim();
        match preprocess_grammar::parse_vote_count_rewrite_surface(trimmed) {
            Some(preprocess_grammar::VoteCountRewriteSurface::DrawForEachVote { vote }) => {
                format!("For each {vote} vote, draw a card")
            }
            Some(preprocess_grammar::VoteCountRewriteSurface::SharedSubjectPair {
                subject,
                first_action,
                first_vote,
                second_action,
                second_vote,
            }) => format!(
                "For each {first_vote} vote, {subject} {first_action}. For each {second_vote} vote, {subject} {second_action}"
            ),
            Some(preprocess_grammar::VoteCountRewriteSurface::TrailingForEach { head, vote }) => {
                format!("For each {vote} vote, {head}")
            }
            None => trimmed.to_string(),
        }
    }

    let Some(document) = preprocess_grammar::parse_preprocess_sentence_list(text) else {
        return text.to_string();
    };
    let rewritten = document
        .sentences
        .into_iter()
        .map(|sentence| rewrite_vote_count_sentence(sentence.as_str()))
        .collect::<Vec<_>>()
        .join(". ");
    if document.terminal_period && !rewritten.is_empty() {
        format!("{rewritten}.")
    } else {
        rewritten
    }
}

fn resized_char_map_for_rewrite(original_map: &[usize], normalized: &str) -> Vec<usize> {
    let target_len = normalized.chars().count();
    if target_len == original_map.len() {
        return original_map.to_vec();
    }

    let mut rewritten = original_map.to_vec();
    let fill = original_map.last().copied().unwrap_or(0);
    rewritten.resize(target_len, fill);
    rewritten
}

fn is_ignorable_unparsed_line(line: &str) -> bool {
    preprocess_grammar::parse_ignorable_parenthetical_line(line)
}

pub fn preprocess_document(
    builder: CardDefinitionBuilder,
    text: &str,
) -> Result<PreprocessedDocument, CardTextError> {
    let provenance = ProvenanceStore::capture(
        SourceUnitId(0),
        text,
        builder.card_builder.name_ref().trim(),
    );
    preprocess_document_with_provenance(builder, text, provenance)
}

pub fn preprocess_document_with_provenance(
    mut builder: CardDefinitionBuilder,
    text: &str,
    mut provenance: ProvenanceStore,
) -> Result<PreprocessedDocument, CardTextError> {
    let structure = crate::front_end::classify_document_structure(
        provenance.source().id,
        text,
        builder.card_builder.name_ref().trim(),
    )?;
    for node in structure.lines.iter().flat_map(|line| &line.nodes) {
        let (kind, reminder_text) = match &node.kind {
            crate::front_end::StructuralNodeKind::SelfReference(_) => (
                SourceSliceKind::SelfReference,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::Quotation(_) => (
                SourceSliceKind::Quotation,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::AbilityWord { .. } => (
                SourceSliceKind::AbilityWord,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::ReminderText(decision) => {
                (SourceSliceKind::ReminderText, *decision)
            }
            crate::front_end::StructuralNodeKind::Symbol => (
                SourceSliceKind::Symbol,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::FaceSeparator => (
                SourceSliceKind::FaceSeparator,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::ChapterHeader { .. } => (
                SourceSliceKind::ChapterHeader,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::ClassHeader { .. } => (
                SourceSliceKind::ClassHeader,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::LevelHeader { .. } => (
                SourceSliceKind::LevelHeader,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::ModeMarker(_) => (
                SourceSliceKind::ModeMarker,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::StructuralNodeKind::Punctuation(_) => (
                SourceSliceKind::Punctuation,
                ReminderTextDecision::NotReminderText,
            ),
        };
        provenance.record_structural_span(kind, node.span, reminder_text);
    }
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
        preprocess_grammar::parse_short_self_reference_name(name)
    }

    fn normalize_non_metadata_line(
        raw_line: &str,
        line_index: usize,
        display_line_index: usize,
        full_name: &str,
        short_name: &str,
        preserve_source_surfaces: bool,
        annotations: &mut ParseAnnotations,
        provenance: &mut ProvenanceStore,
    ) -> Result<Option<PreprocessedLine>, CardTextError> {
        let stripped = strip_parenthetical_segments(raw_line);
        if stripped.trim().is_empty() {
            return Ok(None);
        }

        let Some(normalized) = normalize_line_for_parse(
            stripped.as_str(),
            full_name,
            short_name,
            preserve_source_surfaces,
        ) else {
            if is_ignorable_unparsed_line(raw_line) {
                return Ok(None);
            }
            return Err(CardTextError::ParseError(format!(
                "rewrite preprocessing could not normalize line: '{raw_line}'"
            )));
        };

        let expanded_normalized = expand_borrow_ability_line(normalized.normalized.as_str());
        let rewritten_normalized = rewrite_vote_count_followups_line(expanded_normalized.as_str());
        // Keep explicit exile/return sentences intact. The effect-sequence bundle
        // parser folds them into one source-leaves runtime effect while retaining
        // that the authored surface used two sentences.
        let normalized = if rewritten_normalized != normalized.normalized {
            let char_map =
                resized_char_map_for_rewrite(&normalized.char_map, &rewritten_normalized);
            NormalizedLine {
                original: normalized.original,
                normalized: rewritten_normalized,
                char_map,
            }
        } else {
            normalized
        };

        annotations.record_original_line(line_index, &normalized.original);
        annotations.record_normalized_line(line_index, &normalized.normalized);
        annotations.record_char_map(line_index, normalized.char_map.clone());
        provenance.record_normalized_line(
            display_line_index,
            &normalized.original,
            &normalized.normalized,
            &normalized.char_map,
        );

        let tokens = lex_line(normalized.normalized.as_str(), line_index)?;
        let source_tokens =
            lex_line(raw_line.trim(), line_index).unwrap_or_else(|_| tokens.clone());
        let mut semantic_facts = line_semantic_facts::parse_line_semantic_facts_tokens(&tokens);
        // The normalized parse stream removes the trigger header's leading
        // `unless` clause before later lowering consumes line facts. Retain
        // only this grammar-proven punctuation fact from the authored stream;
        // all semantic parsing continues to use normalized tokens.
        semantic_facts.triggered_ability.leading_unless_surface =
            line_semantic_facts::parse_line_semantic_facts_tokens(&source_tokens)
                .triggered_ability
                .leading_unless_surface;
        Ok(Some(PreprocessedLine {
            info: LineInfo {
                line_index,
                display_line_index,
                raw_line: raw_line.trim().to_string(),
                source_tokens,
                normalized,
                semantic_facts,
            },
            tokens,
        }))
    }

    let card_name = builder.card_builder.name_ref().to_string();
    let front_face_name = card_name
        .split(" // ")
        .next()
        .unwrap_or(card_name.as_str())
        .trim()
        .to_string();
    let short_name = short_name_for_self_reference(front_face_name.as_str());
    let full_lower = normalize_card_name_for_self_reference(front_face_name.as_str());
    let short_lower = normalize_card_name_for_self_reference(short_name.as_str());
    let source_surface_name_is_lexable = lex_line(full_lower.as_str(), 0).is_ok()
        || (short_lower != full_lower && lex_line(short_lower.as_str(), 0).is_ok());
    let mut annotations = ParseAnnotations::default();
    let mut items = Vec::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let structural_line = structure.line(line_index);
        if structural_line.is_some_and(|line| {
            matches!(
                &line.kind,
                crate::front_end::StructuralLineKind::Blank
                    | crate::front_end::StructuralLineKind::FaceSeparator
            )
        }) {
            continue;
        }
        if structural_line.is_some_and(|line| {
            matches!(
                &line.kind,
                crate::front_end::StructuralLineKind::ReminderOnly
            ) && !line.nodes.iter().any(|node| {
                matches!(
                    &node.kind,
                    crate::front_end::StructuralNodeKind::ReminderText(
                        ReminderTextDecision::TreatedAsRulesText
                    )
                )
            })
        }) {
            continue;
        }
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(meta) = structural_line.and_then(|line| match &line.kind {
            crate::front_end::StructuralLineKind::Metadata(value) => {
                Some(materialize_structural_metadata(value))
            }
            _ => None,
        }) {
            let normalized = NormalizedLine {
                original: line.to_string(),
                normalized: line.to_string(),
                char_map: (0..line.chars().count()).collect(),
            };
            builder = builder.apply_compiler_metadata(meta.clone())?;
            annotations.record_original_line(line_index, &normalized.original);
            annotations.record_normalized_line(line_index, &normalized.normalized);
            annotations.record_char_map(line_index, normalized.char_map.clone());
            provenance.record_normalized_line(
                line_index,
                &normalized.original,
                &normalized.normalized,
                &normalized.char_map,
            );
            items.push(PreprocessedItem::Metadata(PreprocessedMetadataLine {
                info: make_line_info(line_index, line, normalized),
                value: meta,
            }));
            continue;
        }

        for (split_index, split_line) in split_parse_line_variants(line).into_iter().enumerate() {
            let preserve_source_surfaces = source_surface_name_is_lexable
                && builder
                    .card_builder
                    .card_types_ref()
                    .iter()
                    .any(|card_type| {
                        matches!(
                            card_type,
                            CardType::Artifact
                                | CardType::Battle
                                | CardType::Creature
                                | CardType::Enchantment
                                | CardType::Land
                                | CardType::Planeswalker
                        )
                    });
            let virtual_line_index = line_index.saturating_mul(8).saturating_add(split_index);
            let looks_like_resolution_followup = lex_line(split_line.as_str(), virtual_line_index)
                .ok()
                .map(|tokens| looks_like_spell_resolution_followup_intro_lexed(tokens.as_slice()))
                .unwrap_or(false);
            let is_standalone_keyword_action = lex_line(split_line.as_str(), virtual_line_index)
                .ok()
                .and_then(|tokens| {
                    split_lexed_sentences(&tokens)
                        .into_iter()
                        .next()
                        .map(|sentence| {
                            super::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(
                                sentence,
                            )
                            .is_some()
                        })
                })
                .unwrap_or(false);

            if spell_card_prefers_resolution_line_merge(&builder)
                && looks_like_resolution_followup
                && !is_standalone_keyword_action
                && let Some(PreprocessedItem::Line(previous)) = items.last_mut()
            {
                let combined_raw_line =
                    format!("{} {}", previous.info.raw_line.trim(), split_line.trim());
                let Some(normalized) = normalize_line_for_parse(
                    combined_raw_line.as_str(),
                    full_lower.as_str(),
                    short_lower.as_str(),
                    preserve_source_surfaces,
                ) else {
                    return Err(CardTextError::ParseError(format!(
                        "rewrite preprocessing could not normalize merged line: '{combined_raw_line}'"
                    )));
                };
                annotations.record_original_line(previous.info.line_index, &normalized.original);
                annotations
                    .record_normalized_line(previous.info.line_index, &normalized.normalized);
                annotations.record_char_map(previous.info.line_index, normalized.char_map.clone());
                provenance.record_normalized_line(
                    previous.info.display_line_index,
                    &normalized.original,
                    &normalized.normalized,
                    &normalized.char_map,
                );
                previous.info.raw_line = combined_raw_line;
                previous.info.normalized = normalized.clone();
                previous.tokens =
                    lex_line(normalized.normalized.as_str(), previous.info.line_index)?;
                continue;
            }
            if let Some(parsed_line) = normalize_non_metadata_line(
                split_line.as_str(),
                virtual_line_index,
                line_index,
                full_lower.as_str(),
                short_lower.as_str(),
                preserve_source_surfaces,
                &mut annotations,
                &mut provenance,
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
            provenance,
            structure,
            items,
        });
    }

    Ok(PreprocessedDocument {
        builder,
        annotations,
        provenance,
        structure,
        items,
    })
}

pub fn make_line_info(
    line_index: usize,
    raw_line: impl Into<String>,
    normalized: NormalizedLine,
) -> LineInfo {
    let raw_line = raw_line.into();
    let source_tokens = lex_line(raw_line.as_str(), line_index).unwrap_or_default();
    LineInfo {
        line_index,
        display_line_index: line_index,
        raw_line,
        source_tokens,
        normalized,
        semantic_facts: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;

    #[test]
    fn parse_metadata_line_routes_supported_labels_through_structure_parser() {
        assert!(matches!(
            parse_metadata_line("Mana Cost: {2}{W}"),
            Ok(Some(MetadataLine::ManaCost(value))) if value == "{2}{W}"
        ));
        assert!(matches!(
            parse_metadata_line("Type: Legendary Creature — Human"),
            Ok(Some(MetadataLine::TypeLine(value))) if value == "Legendary Creature — Human"
        ));
        assert!(matches!(
            parse_metadata_line("First printed set: Antiquities"),
            Ok(Some(MetadataLine::FirstPrintedSet(value))) if value == "Antiquities"
        ));
        assert!(matches!(
            parse_metadata_line(" Power/Toughness: 2/3 "),
            Ok(Some(MetadataLine::PowerToughness(value))) if value == "2/3"
        ));
        assert!(matches!(
            parse_metadata_line("Loyalty: 4"),
            Ok(Some(MetadataLine::Loyalty(value))) if value == "4"
        ));
        assert!(matches!(
            parse_metadata_line("Defense: 5"),
            Ok(Some(MetadataLine::Defense(value))) if value == "5"
        ));
        assert!(matches!(parse_metadata_line("Draw a card."), Ok(None)));
    }

    #[test]
    fn parse_metadata_line_keeps_unlexable_values_by_parsing_only_the_label() {
        assert!(matches!(
            parse_metadata_line("Power/Toughness: */*"),
            Ok(Some(MetadataLine::PowerToughness(value))) if value == "*/*"
        ));
        assert!(matches!(
            parse_metadata_line("Power/Toughness: 1+*/1+*"),
            Ok(Some(MetadataLine::PowerToughness(value))) if value == "1+*/1+*"
        ));
        assert!(matches!(
            parse_metadata_line("Type Line: Artifact // Creature"),
            Ok(Some(MetadataLine::TypeLine(value))) if value == "Artifact // Creature"
        ));
    }

    #[test]
    fn preprocess_document_keeps_metadata_values_after_structure_cutover() {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Metadata Variant");
        let preprocessed = preprocess_document(
            builder,
            "Mana Cost: {2}{W}\nType Line: Legendary Creature — Human\nFirst printed set: Antiquities\nDraw a card.",
        )
        .expect("metadata-bearing text should preprocess");

        assert!(matches!(
            preprocessed.items.first(),
            Some(PreprocessedItem::Metadata(PreprocessedMetadataLine {
                value: MetadataLine::ManaCost(value),
                ..
            })) if value == "{2}{W}"
        ));
        assert!(matches!(
            preprocessed.items.get(1),
            Some(PreprocessedItem::Metadata(PreprocessedMetadataLine {
                value: MetadataLine::TypeLine(value),
                ..
            })) if value == "Legendary Creature — Human"
        ));
        assert!(matches!(
            preprocessed.items.get(2),
            Some(PreprocessedItem::Metadata(PreprocessedMetadataLine {
                value: MetadataLine::FirstPrintedSet(value),
                ..
            })) if value == "Antiquities"
        ));
        assert!(matches!(
            preprocessed.items.get(3),
            Some(PreprocessedItem::Line(_))
        ));
        assert_eq!(
            preprocessed
                .builder
                .build()
                .card
                .first_printed_set_name
                .as_deref(),
            Some("Antiquities")
        );
    }

    #[test]
    fn created_token_lifecycle_normalizes_named_source_after_token_name() {
        let document = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Stangg"),
            "Type: Creature\nWhen Stangg enters, create Stangg Twin, a legendary 3/4 creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.",
        )
        .expect("created-token lifecycle should preprocess");
        let Some(PreprocessedItem::Line(line)) = document.items.get(1) else {
            panic!("expected lifecycle line: {:#?}", document.items);
        };
        assert_eq!(
            line.info.normalized.normalized,
            "when stangg enters, create stangg twin, a legendary 3/4 creature token. exile that token when this leaves the battlefield. sacrifice this when that token leaves the battlefield."
        );
    }

    #[test]
    fn typed_line_shapes_preserve_preprocess_rewrite_behavior() {
        assert_eq!(
            split_parse_line_variants(
                "As an additional cost to cast this spell, discard a card. Draw two cards."
            ),
            vec![
                "As an additional cost to cast this spell, discard a card.".to_string(),
                "Draw two cards.".to_string(),
            ]
        );

        let flashback = "Flashback {8}{B}{B}. This spell costs {X} less to cast this way, where X is the greatest mana value of a commander you own on the battlefield or in the command zone.";
        assert_eq!(
            split_parse_line_variants(flashback),
            vec![flashback.to_string()],
            "flashback-scoped cost adjustments must reach the compound keyword parser"
        );

        assert_eq!(
            strip_parenthetical_segments(
                "It's an enchantment in addition to its other types. (It's not a creature.)"
            ),
            "It's an enchantment in addition to its other types. It's not a creature."
        );

        let normalized = normalize_line_for_parse("Draw a card as it resolves.", "", "", false)
            .expect("resolution line should normalize");
        assert_eq!(normalized.normalized, "draw a card.");
    }

    #[test]
    fn preprocess_preserves_typed_multiword_keyword_action_matching_card_name() {
        let document = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Manifest Dread"),
            "Manifest dread.",
        )
        .expect("the keyword action should preprocess without becoming a source reference");
        let Some(PreprocessedItem::Line(line)) = document.items.first() else {
            panic!("expected one preprocessed keyword-action line");
        };
        assert_eq!(line.info.normalized.normalized, "manifest dread.");

        let reference = normalize_line_for_parse(
            "When Manifest Dread enters, draw a card.",
            "manifest dread",
            "manifest dread",
            false,
        )
        .expect("an ordinary card-name reference should still normalize");
        assert_eq!(reference.normalized, "when this enters, draw a card.");
    }

    #[test]
    fn preprocess_preserves_front_face_name_used_as_become_subtype_descriptor() {
        let document = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Coward // Killer")
                .card_types(vec![CardType::Sorcery]),
            "Target creature can't block this turn and becomes a Coward in addition to its other types until end of turn.\nTime travel.",
        )
        .expect("combined-card face text should preprocess");
        let Some(PreprocessedItem::Line(line)) = document.items.first() else {
            panic!("expected Coward's first rules line: {:#?}", document.items);
        };

        assert_eq!(
            line.info.normalized.normalized,
            "target creature can't block this turn and becomes a coward in addition to its other types until end of turn."
        );
        assert_eq!(
            document.items.len(),
            2,
            "an independently executable keyword action on its own Oracle line must retain that source boundary: {:#?}",
            document.items
        );
    }

    #[test]
    fn typed_borrow_vote_and_return_shapes_drive_textual_rewrites() {
        assert_eq!(
            rewrite_borrow_static_sentence(
                "As long as a creature with flying is in your graveyard, creatures you control have flying"
            ),
            "as long as there is a creature with flying in your graveyard, creatures you control have flying"
        );
        assert_eq!(
            expand_borrow_ability_line(
                "As long as a creature card with flying is in a graveyard, this creature has flying. The same is true for first strike and vigilance."
            ),
            "as long as there is a creature card with flying in a graveyard, this creature has flying. as long as there is a creature card with first strike in a graveyard, this creature has first strike. as long as there is a creature card with vigilance in a graveyard, this creature has vigilance."
        );
        assert_eq!(
            rewrite_vote_count_followups_line("You draw cards equal to the number of truth votes."),
            "For each truth vote, draw a card."
        );
    }

    #[test]
    fn peacekeeper_tie_clause_is_not_fingerprint_dropped() {
        let oracle = "At the beginning of your upkeep, the player with the lowest life total gains control of this creature. If two or more players are tied for lowest life total, you choose one of them, and that player gains control of this creature.";
        let document = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Loxodon Peacekeeper"),
            oracle,
        )
        .expect("Peacekeeper text should preprocess generically");
        let Some(PreprocessedItem::Line(line)) = document.items.first() else {
            panic!("expected one preprocessed line: {:#?}", document.items);
        };
        assert!(
            line.info
                .normalized
                .normalized
                .contains("if two or more players are tied for lowest life total"),
            "the tie clause must remain parser input: {}",
            line.info.normalized.normalized
        );
    }

    #[test]
    fn typed_text_rewrites_keep_source_maps_aligned() {
        for oracle in [
            "As long as a creature with flying is in your graveyard, creatures you control have flying. The same is true for first strike and vigilance.",
            "You draw cards equal to the number of truth votes.",
            "Exile target creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.",
        ] {
            let document = preprocess_document(
                CardDefinitionBuilder::new(CardId::new(), "Preprocess Test"),
                oracle,
            )
            .expect("typed rewrite should preprocess");
            let Some(PreprocessedItem::Line(line)) = document.items.first() else {
                panic!("expected rewritten line: {:#?}", document.items);
            };
            assert_eq!(
                line.info.normalized.char_map.len(),
                line.info.normalized.normalized.chars().count(),
                "source map length must follow rewritten text: {}",
                line.info.normalized.normalized
            );
            assert!(
                line.info
                    .normalized
                    .char_map
                    .iter()
                    .all(|offset| *offset <= oracle.len()),
                "source map offset escaped original line: {:?}",
                line.info.normalized.char_map
            );
        }
    }
}
