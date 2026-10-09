use super::grammar::effects::optional_companion_shapes::parse_shared_subject_optional_companion_shape;
use super::grammar::structure::{MetadataLineKind, split_leading_result_prefix_lexed};
use super::grammar::{line_semantic_facts, preprocess as preprocess_grammar};
use super::lexer::{TokenKind, lex_line, render_token_slice, split_lexed_sentences};
use super::parser_support::{
    looks_like_spell_resolution_followup_intro_lexed, spell_card_prefers_resolution_line_merge,
};
use ironsmith_core::card::CardBuilder;

use crate::cards::builders::{
    CardTextError, LineInfo, MetadataLine, NormalizedLine, OwnedLexToken, ParseAnnotations,
};
use crate::model::provenance::{
    ProvenanceStore, ReminderTextDecision, SourceSliceKind, SourceUnitId,
};
use crate::types::CardType;

#[derive(Debug, Clone)]
pub struct PreprocessedDocument {
    pub card: CardBuilder,
    pub annotations: ParseAnnotations,
    pub provenance: ProvenanceStore,
    pub cst: crate::front_end::DocumentCst,
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

/// Authored rules tokens retain original byte spans and casing, but never
/// expose reminder text to downstream semantic parsers. Raw text and CST
/// provenance remain available separately for presentation and diagnostics.
fn authored_rules_tokens(
    raw: &str,
    line_index: usize,
) -> Result<Vec<OwnedLexToken>, CardTextError> {
    let tokens = lex_line(raw, line_index)?;
    match preprocess_grammar::parse_parenthetical_line_surface_tokens(&tokens) {
        Some(preprocess_grammar::ParentheticalLineSurface::FullyWrapped) => {
            // Preserve standalone activation tokens until the document owner
            // can distinguish functional text from a typed CR 305.6 reminder.
            // A reminder is never lowered as a printed ability.
            Ok(crate::util::strip_parenthetical_tokens(
                &tokens[1..tokens.len() - 1],
            ))
        }
        Some(preprocess_grammar::ParentheticalLineSurface::PreserveEnchantmentNotCreature) => {
            // This parenthetical is a functional type-changing instruction.
            let mut kept = Vec::new();
            let mut depth = 0usize;
            let mut start = 0usize;
            for (index, token) in tokens.iter().enumerate() {
                match token.kind {
                    TokenKind::LParen => {
                        if depth == 0 {
                            start = index + 1;
                        }
                        depth += 1;
                    }
                    TokenKind::RParen => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            let body = &tokens[start..index];
                            let words = crate::lexer::parser_token_word_refs(body);
                            if words == ["it's", "not", "a", "creature"]
                                || words == ["its", "not", "a", "creature"]
                            {
                                kept.extend_from_slice(body);
                            }
                        }
                    }
                    _ if depth == 0 => kept.push(token.clone()),
                    _ => {}
                }
            }
            Ok(kept)
        }
        None => Ok(crate::util::strip_parenthetical_tokens(&tokens)),
    }
}

fn supported_sneak_reminder(raw: &str, line_index: usize) -> bool {
    use crate::grammar::keyword_dispatch::{
        KeywordSpecialFormShape, parse_keyword_special_form_shape_tokens,
    };
    if !raw
        .trim_start()
        .get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("sneak"))
    {
        return false;
    }
    lex_line(raw, line_index).ok().is_some_and(|tokens| {
        matches!(
            parse_keyword_special_form_shape_tokens(&tokens),
            Some(KeywordSpecialFormShape::SpellSneak | KeywordSpecialFormShape::PermanentSneak)
        )
    })
}

fn station_reminder_threshold(raw: &str, line_index: usize) -> Option<i32> {
    if !raw.trim_start().get(..7)?.eq_ignore_ascii_case("station") {
        return None;
    }
    let tokens = lex_line(raw, line_index).ok()?;
    crate::grammar::line_families::parse_station_keyword_line(&tokens, &tokens)?.creature_threshold
}

pub(super) fn strip_parenthetical_segments(line: &str) -> String {
    let surface = stage_tokens(line)
        .and_then(|tokens| preprocess_grammar::parse_parenthetical_line_surface_tokens(&tokens));
    match surface {
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

/// The tokens of one stage of the line pipeline. The document phase rewrites
/// text, and each rewritten text is tokenized once for the probes that read
/// it; `None` when the text is not rules text the lexer accepts.
fn stage_tokens(text: &str) -> Option<Vec<OwnedLexToken>> {
    crate::util::lex_fragment(text.trim(), 0)
}

/// The tokens of `tokens` that fall inside `range` of their text.
fn tokens_within(tokens: &[OwnedLexToken], range: std::ops::Range<usize>) -> &[OwnedLexToken] {
    let start = tokens.partition_point(|token| token.span.start < range.start);
    let end = tokens.partition_point(|token| token.span.end <= range.end);
    tokens.get(start..end.max(start)).unwrap_or_default()
}

/// A name the document phase turns into a self reference wherever a line
/// mentions it: the lowercased text that is matched and the tokens the
/// keyword guards read. Card metadata, tokenized once per card; a name the
/// lexer rejects keeps its text and has no tokens.
struct SelfReferenceName {
    text: String,
    tokens: Option<Vec<OwnedLexToken>>,
}

impl SelfReferenceName {
    fn new(text: String) -> Self {
        let tokens = stage_tokens(text.as_str());
        Self { text, tokens }
    }

    fn tokens(&self) -> &[OwnedLexToken] {
        self.tokens.as_deref().unwrap_or_default()
    }

    fn lexable(&self) -> bool {
        self.tokens.is_some()
    }
}

#[cfg(test)]
fn split_parse_line_variants_text(line: &str) -> Vec<String> {
    split_parse_line_variants(line, &stage_tokens(line).unwrap_or_default())
}

/// "This creature has trample as long as you control a Beast, haste as long
/// as you control a Goblin, ..., and "{B}: Regenerate this creature" as long
/// as you control a Zombie." (Tribal Golem): each conditional grant in the
/// list is its own static ability sharing the subject.
fn split_conditional_grant_list(line: &str) -> Option<Vec<String>> {
    const CONDITION: &str = " as long as you control a";
    let has_index = line.find(" has ")?;
    let subject = &line[..has_index];
    if subject.contains(',') || subject.contains('"') {
        return None;
    }
    let rest = line[has_index + " has ".len()..]
        .trim_end()
        .trim_end_matches('.');
    // Split on top-level commas only (never inside a quoted ability).
    let mut segments = Vec::new();
    let mut in_quote = false;
    let mut start = 0usize;
    for (index, ch) in rest.char_indices() {
        match ch {
            '"' | '\u{201C}' | '\u{201D}' => in_quote = !in_quote,
            ',' if !in_quote => {
                segments.push(rest[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    segments.push(rest[start..].trim());
    if segments.len() < 3 {
        return None;
    }
    let segments = segments
        .into_iter()
        .map(|segment| segment.strip_prefix("and ").unwrap_or(segment).trim())
        .collect::<Vec<_>>();
    if !segments.iter().all(|segment| {
        segment.contains(CONDITION)
            && !segment.starts_with(CONDITION.trim_start())
            && segment.matches(CONDITION).count() == 1
    }) {
        return None;
    }
    Some(
        segments
            .into_iter()
            .map(|segment| {
                // A quoted ability reads its condition first so the
                // condition is not absorbed into the quoted text.
                let split = segment.find(CONDITION).unwrap_or(segment.len());
                let (granted, condition) = segment.split_at(split);
                let granted = granted.trim();
                if granted.starts_with(['"', '\u{201C}']) {
                    let mut chars = subject.chars();
                    let subject = chars
                        .next()
                        .map(|first| first.to_lowercase().chain(chars).collect::<String>())
                        .unwrap_or_default();
                    let condition = condition.trim_start();
                    let condition = condition.strip_prefix("as").unwrap_or(condition);
                    let granted = match granted.strip_suffix(['"', '\u{201D}']) {
                        Some(inner) if !inner.ends_with('.') => format!("{inner}.\""),
                        _ => granted.to_string(),
                    };
                    format!("As{condition}, {subject} has {granted}")
                } else {
                    format!("{subject} has {segment}.")
                }
            })
            .collect(),
    )
}

fn split_parse_line_variants(line: &str, line_tokens: &[OwnedLexToken]) -> Vec<String> {
    if let Some(lines) = split_conditional_grant_list(line) {
        return lines;
    }
    if let Some(split) = preprocess_grammar::parse_line_variant_split_tokens(line_tokens) {
        let first = line.get(..split.first_end).unwrap_or_default().trim();
        let second = line.get(split.second_start..).unwrap_or_default().trim();
        let first_tokens = tokens_within(line_tokens, 0..split.first_end);
        let second_tokens = crate::util::strip_parenthetical_tokens(tokens_within(
            line_tokens,
            split.second_start..line.len(),
        ));
        let is_flashback_scoped_cost_adjustment = split.kind
            == preprocess_grammar::LineVariantSplitKind::CostAdjustmentFollowup
            && preprocess_grammar::is_flashback_scoped_cost_adjustment_tokens(
                first_tokens,
                &second_tokens,
            );
        if is_flashback_scoped_cost_adjustment {
            // The flashback parser binds "this way" to the alternative cast.
            // Splitting these sentences first silently broadens the reduction
            // to normal casting as well.
            return vec![line.to_string()];
        }
        if split.kind == preprocess_grammar::LineVariantSplitKind::ManaSpendFollowup
            && preprocess_grammar::is_mana_spend_bonus_followup_tokens(&second_tokens)
        {
            return vec![line.to_string()];
        }
        if let Some(restriction_start) = split.trailing_restriction_start
            && split.kind == preprocess_grammar::LineVariantSplitKind::CostAdjustmentFollowup
            && restriction_start > split.second_start
        {
            let cost = line
                .get(split.second_start..restriction_start)
                .unwrap_or_default()
                .trim();
            let restriction = line.get(restriction_start..).unwrap_or_default().trim();
            if !first.is_empty() && !cost.is_empty() && !restriction.is_empty() {
                return vec![format!("{first} {restriction}"), cost.to_string()];
            }
        }
        if !first.is_empty() && !second.is_empty() {
            return vec![first.to_string(), second.to_string()];
        }
    }

    vec![line.to_string()]
}

fn parse_metadata_line(line: &str) -> Result<Option<MetadataLine>, CardTextError> {
    let Some(surface) = preprocess_grammar::parse_metadata_surface_with(line, stage_tokens) else {
        return Ok(None);
    };

    let metadata = match surface.kind {
        MetadataLineKind::ManaCost => MetadataLine::ManaCost(surface.value),
        MetadataLineKind::TypeLine => MetadataLine::TypeLine(surface.value),
        MetadataLineKind::ColorIndicator => MetadataLine::ColorIndicator(surface.value),
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
        crate::front_end::MetadataLine::ColorIndicator(value) => {
            MetadataLine::ColorIndicator(value.clone())
        }
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
    line_tokens: &[OwnedLexToken],
    full_name: &SelfReferenceName,
    short_name: &SelfReferenceName,
    preserve_source_surfaces: bool,
    typed_subject: &str,
    base_char_offset: usize,
) -> (String, Vec<usize>) {
    fn has_word_boundaries_at(bytes: &[u8], idx: usize, len: usize) -> bool {
        // A byte at or above 0x80 belongs to a multibyte letter: "omer" inside
        // "Éomer" is not a word of its own.
        let is_word = |b: u8| b.is_ascii_alphanumeric() || b >= 0x80;
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

    fn is_single_word_keyword_verb(name: &SelfReferenceName) -> bool {
        preprocess_grammar::parse_single_keyword_verb_tokens(name.tokens()).is_some()
    }

    fn starts_with_typed_keyword_action_statement(tokens: &[OwnedLexToken]) -> bool {
        let Some(statement) = split_lexed_sentences(tokens).into_iter().next() else {
            return false;
        };
        super::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(statement)
            .is_some()
    }

    fn is_keyword_ability_name(name: &SelfReferenceName) -> bool {
        preprocess_grammar::parse_keyword_ability_name_tokens(name.tokens()).is_some()
    }

    /// "The flashback cost is equal to its mana cost." on a card named
    /// Flashback: the keyword's cost, not a self-reference.
    fn followed_by_cost_word(bytes: &[u8], mut idx: usize) -> bool {
        while idx < bytes.len() && !bytes[idx].is_ascii_alphanumeric() {
            idx += 1;
        }
        let start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_alphanumeric() {
            idx += 1;
        }
        matches!(&bytes[start..idx], b"cost" | b"costs")
    }

    fn preceded_by_named_keyword(bytes: &[u8], candidate: usize) -> bool {
        let mut idx = candidate;
        while idx > 0 && !bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        let end = idx;
        while idx > 0 && bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        if idx < end && &bytes[idx..end] == b"named" {
            return true;
        }
        within_named_serial_list(bytes, candidate)
    }

    /// "Equipment named Sword of Kaldra, Shield of Kaldra, and Helm of
    /// Kaldra": a later entry of a comma list of names is still a name, not a
    /// self-reference.
    fn within_named_serial_list(bytes: &[u8], candidate: usize) -> bool {
        let mut cursor = candidate;
        while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
            cursor -= 1;
        }
        if let Some(word) = previous_word(bytes, cursor)
            && matches!(word, b"and" | b"or")
            && cursor >= word.len()
            && &bytes[cursor - word.len()..cursor] == word
        {
            cursor -= word.len();
            while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
                cursor -= 1;
            }
        }
        if cursor == 0 || bytes[cursor - 1] != b',' {
            return false;
        }
        let before = &bytes[..cursor - 1];
        let sentence_start = before
            .iter()
            .rposition(|byte| matches!(*byte, b'.' | b';' | b'"' | b'(' | b':'))
            .map(|at| at + 1)
            .unwrap_or(0);
        let segment = &before[sentence_start..];
        let Some(named_at) = segment
            .windows(7)
            .rposition(|window| window.eq_ignore_ascii_case(b" named "))
        else {
            return false;
        };
        segment[named_at + 7..]
            .split(|byte| byte.is_ascii_whitespace())
            .filter(|word| !word.is_empty())
            .count()
            <= 12
    }

    /// "meld them into Titania, Gaea Incarnate": the meld result's name is
    /// another card's name even when it shares the source's short name.
    /// "Roll a six-sided die" on a card named Six-Sided Die: the die rolled
    /// is a game object of that kind, not the card itself.
    fn is_rolled_die_noun(bytes: &[u8], idx: usize) -> bool {
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
            .is_some_and(|word| matches!(word, b"roll" | b"rolls" | b"reroll" | b"rerolls"))
    }

    fn preceded_by_meld_into(bytes: &[u8], idx: usize) -> bool {
        let start = idx.saturating_sub(24);
        let window = bytes[start..idx].to_ascii_lowercase();
        window.ends_with(b"meld them into ") || window.ends_with(b"meld it into ")
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

    /// "a card exiled with Raphael" (Raphael, Most Attitude): the linked
    /// exile pool of the source names it by its short name.
    fn preceded_by_exiled_with(bytes: &[u8], idx: usize) -> bool {
        if previous_word(bytes, idx) != Some(b"with".as_slice()) {
            return false;
        }
        let mut with_start = idx;
        while with_start > 0 && !bytes[with_start - 1].is_ascii_alphanumeric() {
            with_start -= 1;
        }
        while with_start > 0 && bytes[with_start - 1].is_ascii_alphanumeric() {
            with_start -= 1;
        }
        previous_word(bytes, with_start) == Some(b"exiled".as_slice())
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
        keyword: &SelfReferenceName,
    ) -> bool {
        if !is_single_word_keyword_verb(keyword) {
            return false;
        }
        let Some(slice) = original.as_bytes().get(idx..idx + len) else {
            return false;
        };
        !slice.iter().any(|byte| byte.is_ascii_uppercase())
    }

    fn within_vote_choice_clause(bytes: &[u8], line_tokens: &[OwnedLexToken], idx: usize) -> bool {
        let mut sentence_start = idx;
        while sentence_start > 0 {
            let prev = bytes[sentence_start - 1];
            if prev == b'.' || prev == b';' {
                break;
            }
            sentence_start -= 1;
        }
        let clause = tokens_within(line_tokens, sentence_start..idx);
        preprocess_grammar::parse_vote_choice_surface_tokens(clause).is_some()
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
                    | b"untap"
                    | b"tap"
                    | b"sacrifice"
                    | b"destroy"
                    | b"regenerate"
                    // "You can't cast Rakdos unless ..." (Rakdos, Lord of
                    // Riots): the spell being cast is this card.
                    | b"cast"
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
                    | b"gain"
                    | b"gains"
                    | b"lose"
                    | b"loses"
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
            || preceded_by_exiled_with(bytes, idx)
    }

    fn is_result_optional_companion_short_name_context(
        bytes: &[u8],
        line_tokens: &[OwnedLexToken],
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
        let tokens = tokens_within(line_tokens, sentence_start..sentence_end);
        let Some(prefix) = split_leading_result_prefix_lexed(tokens) else {
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
        first.span.start == idx && last.span.end == idx + len
    }

    fn is_created_token_lifecycle_source(
        bytes: &[u8],
        line_tokens: &[OwnedLexToken],
        idx: usize,
    ) -> bool {
        let sentence_start = crate::slice_primitives::select_last_position(&bytes[..idx], |byte| {
            matches!(*byte, b'.' | b';')
        })
        .map_or(0, |separator| separator + 1);
        let tokens = tokens_within(line_tokens, sentence_start..idx);
        crate::word_primitives::parse_sequence_suffix(
            &crate::lexer::parser_token_word_refs(tokens),
            &["exile", "that", "token", "when"],
        )
    }

    fn should_preserve_source_surface_context(
        bytes: &[u8],
        line_tokens: &[OwnedLexToken],
        idx: usize,
        len: usize,
    ) -> bool {
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
            && is_created_token_lifecycle_source(bytes, line_tokens, idx)
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

    // A starting-life adjective describes a player quantity even when it
    // coincides with the source's short name.
    fn is_starting_life_descriptor(bytes: &[u8], idx: usize, len: usize) -> bool {
        bytes[idx..idx + len].eq_ignore_ascii_case(b"starting")
            && bytes
                .get(idx + len..)
                .is_some_and(|tail| tail.starts_with(b" life total"))
    }

    fn is_base_characteristic_descriptor(bytes: &[u8], idx: usize, len: usize) -> bool {
        bytes[idx..idx + len].eq_ignore_ascii_case(b"base")
            && bytes
                .get(idx + len..)
                .is_some_and(|tail| tail.starts_with(b" power") || tail.starts_with(b" toughness"))
    }

    // "until end of turn" / "this turn" on a card named Turn (Turn // Burn):
    // the game's turn noun after a determiner or "of" is never the source.
    fn is_turn_noun_usage(bytes: &[u8], idx: usize, len: usize) -> bool {
        bytes[idx..idx + len].eq_ignore_ascii_case(b"turn")
            && previous_word(bytes, idx).is_some_and(|word| {
                matches!(
                    word.to_ascii_lowercase().as_slice(),
                    b"of"
                        | b"this"
                        | b"that"
                        | b"each"
                        | b"next"
                        | b"your"
                        | b"their"
                        | b"extra"
                        | b"same"
                        | b"whose"
                        | b"a"
                        | b"an"
                        | b"its"
                        | b"his"
                        | b"her"
                )
            })
    }

    // The planeswalker type in the named keyword action is not a self
    // reference, even on a source with that short name (CR 701.71).
    fn is_excess_damage_descriptor(bytes: &[u8], idx: usize, len: usize) -> bool {
        bytes[idx..idx + len].eq_ignore_ascii_case(b"excess")
            && next_word(bytes, idx + len) == Some(b"damage".as_slice())
    }

    fn is_empower_jace_subtype(bytes: &[u8], idx: usize, len: usize) -> bool {
        previous_word(bytes, idx) == Some(b"empower".as_slice())
            && bytes[idx..idx + len].eq_ignore_ascii_case(b"jace")
    }

    /// "Target Assembly-Worker creature" on a card named Assembly-Worker: a
    /// name spelled as a subtype between a selecting word and a type noun
    /// describes a class of objects, not this object.
    fn is_subtype_descriptor_usage(bytes: &[u8], idx: usize, len: usize) -> bool {
        if next_word(bytes, idx + len)
            .is_some_and(|word| matches!(word, b"planeswalker" | b"planeswalkers"))
            && std::str::from_utf8(&bytes[idx..idx + len])
                .ok()
                .and_then(crate::util::parse_subtype_flexible)
                .is_some_and(|subtype| subtype.is_planeswalker_subtype())
        {
            return true;
        }
        // "for each other attacking Aurochs" on Aurochs: a name spelled as a
        // creature subtype after a combat-state or `other` adjective, ending
        // the phrase, is the class of objects.
        if previous_word(bytes, idx)
            .is_some_and(|word| matches!(word, b"attacking" | b"blocking" | b"other" | b"another"))
            && bytes
                .get(idx + len)
                .is_none_or(|byte| matches!(*byte, b'.' | b',' | b';'))
            && std::str::from_utf8(&bytes[idx..idx + len])
                .ok()
                .is_some_and(|name| {
                    !name.contains(' ') && crate::util::parse_subtype_flexible(name).is_some()
                })
        {
            return true;
        }
        previous_word(bytes, idx).is_some_and(|word| {
            matches!(word, b"target" | b"other" | b"another" | b"each" | b"all")
        }) && next_word(bytes, idx + len).is_some_and(|word| {
            matches!(
                word,
                b"creature"
                    | b"creatures"
                    | b"card"
                    | b"cards"
                    | b"permanent"
                    | b"permanents"
                    | b"token"
                    | b"tokens"
                    | b"spell"
                    | b"spells"
            )
        })
    }

    /// "counters on Beast this turn" on Beast, Erudite Aerialist; "dealt to
    /// Gideon Blackblade" on Gideon Blackblade: a name whose first word is a
    /// creature subtype, right after a preposition and with no determiner, is
    /// the proper name. Keeping the authored surface there lets the filter
    /// grammar read it as "any Beast" / "any Gideon", so normalize it to the
    /// self-reference instead.
    fn is_subtype_leading_name_after_preposition(bytes: &[u8], idx: usize, len: usize) -> bool {
        if !previous_word(bytes, idx).is_some_and(|word| matches!(word, b"on" | b"to")) {
            return false;
        }
        let Ok(name) = std::str::from_utf8(&bytes[idx..idx + len]) else {
            return false;
        };
        let Some(first) = name.split([' ', ',']).next() else {
            return false;
        };
        if crate::util::parse_subtype_flexible(first).is_none() {
            return false;
        }
        // Only where a temporal qualifier follows the name directly ("on Beast
        // this turn", "to Gideon Blackblade during your turn"): there the
        // bare name is the whole object phrase that a context-free filter
        // reader would otherwise take as a subtype.
        bytes.get(idx + len) == Some(&b' ')
            && next_word(bytes, idx + len).is_some_and(|word| matches!(word, b"this" | b"during"))
    }

    /// Inside a quoted granted ability the card's name names the grantor,
    /// never the object that will hold the ability.
    fn within_double_quotes(bytes: &[u8], idx: usize) -> bool {
        bytes[..idx].iter().filter(|byte| **byte == b'"').count() % 2 == 1
    }

    fn push_replacement(
        out: &mut String,
        map: &mut Vec<usize>,
        replacement: &str,
        base: usize,
        name_len: usize,
    ) {
        let name_len = name_len.max(1);
        let len = replacement.chars().count();
        for (j, ch) in replacement.chars().enumerate() {
            out.push(ch);
            // Retain both endpoints, including when the replacement ends the
            // line: its final character must still cover the name's suffix.
            map.push(base + j * (name_len - 1) / len.saturating_sub(1).max(1));
        }
    }

    /// `Equipped creature has "... Return Trusty Boomerang to its owner's
    /// hand."`: inside an ability the Equipment (or Aura) grants to the
    /// permanent it's attached to, the card's own name names the attachment,
    /// not the ability's source (the equipped or enchanted permanent).
    /// The name is the object of an action in the granted ability's effect
    /// ("Return Trusty Boomerang ...", "you may sacrifice Trickster's
    /// Talisman"), not part of its cost ("{T}, Sacrifice Blazing Torch:").
    fn is_attachment_grant_action_object(bytes: &[u8], idx: usize, len: usize) -> bool {
        let before = std::str::from_utf8(&bytes[..idx]).unwrap_or("");
        let words = before.split(|ch: char| !ch.is_ascii_alphanumeric()).filter(|word| !word.is_empty()).collect::<Vec<_>>();
        let after = &bytes[idx + len..]; let end = after.iter().position(|byte| *byte == b'"').unwrap_or(after.len());
        crate::grammar::preprocess::attachment_grant_name_is_operand(
            words.last().copied(), words.len().checked_sub(2).map(|index| words[index]), after[..end].contains(&b':'))
    }

    fn quoted_attachment_grant_host(bytes: &[u8], idx: usize) -> Option<(&'static str, bool)> {
        let text = std::str::from_utf8(bytes).ok()?;
        crate::grammar::preprocess::attachment_grant_quote_scopes(text).into_iter()
            .find(|scope| scope.start <= idx && idx < scope.end)
            .map(|scope| (GRANTING_SOURCE_SURFACE, scope.labeled))
    }

    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let full_bytes = full_name.text.as_bytes();
    let short_bytes = short_name.text.as_bytes();

    let mut out = String::new();
    let mut map = Vec::new();
    let mut idx = 0;
    let mut source_char = base_char_offset;

    while idx < bytes.len() {
        let full_typed_override = preserve_source_surfaces
            && !full_bytes.is_empty()
            && bytes_start_with(&bytes[idx..], full_bytes)
            && has_word_boundaries_at(bytes, idx, full_bytes.len())
            && is_subtype_leading_name_after_preposition(bytes, idx, full_bytes.len())
            && !within_double_quotes(bytes, idx);
        let short_typed_override = preserve_source_surfaces
            && !short_bytes.is_empty()
            && bytes_start_with(&bytes[idx..], short_bytes)
            && has_word_boundaries_at(bytes, idx, short_bytes.len())
            && is_subtype_leading_name_after_preposition(bytes, idx, short_bytes.len())
            && !within_double_quotes(bytes, idx);
        let attachment_name_len = (!full_bytes.is_empty()
            && bytes_start_with(&bytes[idx..], full_bytes)
            && has_word_boundaries_at(bytes, idx, full_bytes.len())
            && is_attachment_grant_action_object(bytes, idx, full_bytes.len()))
        .then_some(full_bytes.len());
        if let Some(name_len) = attachment_name_len
            && let Some((replacement, labeled)) = quoted_attachment_grant_host(bytes, idx)
        {
            let name_chars = lower[idx..idx + name_len].chars().count();
            if labeled {
                // An ability-word line is re-read from its authored tokens;
                // keep the name so the token-level source normalizer
                // rewrites it there without shifting the source map.
                for (offset, ch) in lower[idx..idx + name_len].chars().enumerate() {
                    out.push(ch);
                    map.push(source_char + offset);
                }
            } else {
                push_replacement(&mut out, &mut map, replacement, source_char, name_chars);
            }
            idx += name_len;
            source_char += name_chars;
            continue;
        }
        if !full_bytes.is_empty()
            && bytes_start_with(&bytes[idx..], full_bytes)
            && has_word_boundaries_at(bytes, idx, full_bytes.len())
            && !(idx == 0
                && (is_single_word_keyword_verb(full_name)
                    || starts_with_typed_keyword_action_statement(line_tokens)))
            && !(is_keyword_ability_name(full_name) && preceded_by_ability_grant_word(bytes, idx))
            && !(is_keyword_ability_name(full_name)
                && followed_by_cost_word(bytes, idx + full_bytes.len()))
            && !preceded_by_named_keyword(bytes, idx)
            && !preceded_by_meld_into(bytes, idx)
            && !is_rolled_die_noun(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, full_bytes.len())
            && !within_vote_choice_clause(bytes, line_tokens, idx)
            && !is_indefinite_become_descriptor(bytes, idx)
            && !is_empower_jace_subtype(bytes, idx, full_bytes.len())
            && !is_excess_damage_descriptor(bytes, idx, full_bytes.len())
            && !is_starting_life_descriptor(bytes, idx, full_bytes.len())
            && !is_base_characteristic_descriptor(bytes, idx, full_bytes.len())
            && !is_turn_noun_usage(bytes, idx, full_bytes.len())
            && !is_subtype_descriptor_usage(bytes, idx, full_bytes.len())
            && !(preserve_source_surfaces
                && should_preserve_source_surface_context(
                    bytes,
                    line_tokens,
                    idx,
                    full_bytes.len(),
                )
                && !full_typed_override)
            && !should_preserve_single_word_keyword_verb_usage(
                line,
                idx,
                full_bytes.len(),
                full_name,
            )
        {
            let name_chars = lower[idx..idx + full_bytes.len()].chars().count();
            let replacement = if full_typed_override {
                typed_subject
            } else {
                "this"
            };
            push_replacement(&mut out, &mut map, replacement, source_char, name_chars);
            idx += full_bytes.len();
            source_char += name_chars;
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
                    || starts_with_typed_keyword_action_statement(line_tokens)))
            && !(is_keyword_ability_name(short_name) && preceded_by_ability_grant_word(bytes, idx))
            && !(is_keyword_ability_name(short_name)
                && followed_by_cost_word(bytes, idx + short_bytes.len()))
            && !preceded_by_named_keyword(bytes, idx)
            && !preceded_by_meld_into(bytes, idx)
            && !is_rolled_die_noun(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, short_bytes.len())
            && !within_vote_choice_clause(bytes, line_tokens, idx)
            && !is_indefinite_become_descriptor(bytes, idx)
            && !is_empower_jace_subtype(bytes, idx, short_bytes.len())
            && !is_excess_damage_descriptor(bytes, idx, short_bytes.len())
            && !is_starting_life_descriptor(bytes, idx, short_bytes.len())
            && !is_base_characteristic_descriptor(bytes, idx, short_bytes.len())
            && !is_turn_noun_usage(bytes, idx, short_bytes.len())
            && (is_short_name_self_reference_context(bytes, idx, short_bytes.len())
                || is_result_optional_companion_short_name_context(
                    bytes,
                    line_tokens,
                    idx,
                    short_bytes.len(),
                ))
            && !(preserve_source_surfaces
                && should_preserve_source_surface_context(
                    bytes,
                    line_tokens,
                    idx,
                    short_bytes.len(),
                )
                && !short_typed_override)
            && !should_preserve_single_word_keyword_verb_usage(
                line,
                idx,
                short_bytes.len(),
                short_name,
            )
        {
            let name_chars = lower[idx..idx + short_bytes.len()].chars().count();
            let replacement = if short_typed_override {
                typed_subject
            } else {
                "this"
            };
            push_replacement(&mut out, &mut map, replacement, source_char, name_chars);
            idx += short_bytes.len();
            source_char += name_chars;
            continue;
        }
        let ch = lower[idx..].chars().next().unwrap();
        out.push(ch);
        map.push(source_char);
        idx += ch.len_utf8();
        source_char += 1;
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
    let Some(surface) = stage_tokens(text)
        .and_then(|tokens| preprocess_grammar::parse_labeled_ability_prefix_tokens(&tokens))
    else {
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
    let Some(tokens) = stage_tokens(text) else {
        return (text.to_string(), map.to_vec());
    };
    let Some(surface) = preprocess_grammar::parse_resolution_timing_tail_tokens(&tokens) else {
        return (text.to_string(), map.to_vec());
    };

    let mut out = text[..surface.tail_start].trim_end().to_string();
    let mut out_map = map[..out.chars().count().min(map.len())].to_vec();
    let kept_tokens = tokens_within(&tokens, 0..surface.tail_start);
    if surface.terminal_period && !preprocess_grammar::parse_terminal_period_tokens(kept_tokens) {
        out.push('.');
        let period_source = tokens
            .last()
            .and_then(|token| map.get(text[..token.span.start].chars().count()));
        out_map.push(*period_source.unwrap_or_else(|| map.last().unwrap_or(&0)));
    }
    (out, out_map)
}

#[cfg(test)]
fn normalize_line_for_parse_text(
    line: &str,
    full_name: &str,
    short_name: &str,
    preserve_source_surfaces: bool,
) -> Option<NormalizedLine> {
    normalize_line_for_parse(
        line,
        &SelfReferenceName::new(full_name.to_string()),
        &SelfReferenceName::new(short_name.to_string()),
        preserve_source_surfaces,
        "this",
    )
}

fn normalize_line_for_parse(
    line: &str,
    full_name: &SelfReferenceName,
    short_name: &SelfReferenceName,
    preserve_source_surfaces: bool,
    typed_subject: &str,
) -> Option<NormalizedLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let trimmed_tokens = stage_tokens(trimmed).unwrap_or_default();
    let (replaced, map) = replace_names_with_map(
        trimmed,
        &trimmed_tokens,
        full_name,
        short_name,
        preserve_source_surfaces,
        typed_subject,
        0,
    );
    let (label_stripped, label_map) = strip_labeled_ability_word_prefix_with_map(&replaced, &map);
    let (stripped, stripped_map) = strip_parenthetical_with_map(&label_stripped, &label_map);
    let (stripped, stripped_map) = strip_resolution_timing_tail_with_map(&stripped, &stripped_map);

    if stripped.trim().is_empty() {
        let wrapped =
            preprocess_grammar::parse_wrapped_activation_surface_tokens(trimmed, &trimmed_tokens)?;
        let inner_tokens = stage_tokens(wrapped.inner.as_str()).unwrap_or_default();
        let (inner_replaced, inner_map) = replace_names_with_map(
            wrapped.inner.as_str(),
            &inner_tokens,
            full_name,
            short_name,
            preserve_source_surfaces,
            typed_subject,
            trimmed[..wrapped.inner_start].chars().count(),
        );
        return Some(NormalizedLine::from_char_map(
            trimmed,
            inner_replaced,
            inner_map,
        ));
    }

    Some(NormalizedLine::from_char_map(
        trimmed,
        stripped,
        stripped_map,
    ))
}

fn split_same_is_true_subject_predicate(sentence: &[OwnedLexToken]) -> Option<(String, String)> {
    preprocess_grammar::parse_subject_predicate_surface_tokens(sentence)
        .map(|surface| (surface.subject, surface.predicate))
}

fn find_borrow_ability_source_phrase(sentence: &[OwnedLexToken]) -> Option<&'static str> {
    preprocess_grammar::parse_borrow_ability_surface_tokens(sentence).map(|surface| surface.phrase)
}

/// The sentence with every borrowed-ability phrase replaced by `replacement`.
fn apply_borrow_phrase_occurrences(
    tokens: &[OwnedLexToken],
    occurrences: &preprocess_grammar::BorrowPhraseTokenOccurrences,
    replacement: &str,
) -> Vec<OwnedLexToken> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut cursor = 0usize;
    for range in &occurrences.ranges {
        out.extend_from_slice(&tokens[cursor..range.start]);
        out.extend(crate::lexer::synthetic_phrase_tokens(replacement));
        cursor = range.end;
    }
    out.extend_from_slice(&tokens[cursor..]);
    out
}

fn rewrite_borrow_static_condition(condition: &[OwnedLexToken], ability: &str) -> Option<String> {
    match preprocess_grammar::parse_borrow_static_condition_surface_tokens(condition, ability)? {
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

/// A borrowed-ability static sentence, rewritten to its canonical "as long
/// as there is ..." form; the sentence text as rendered otherwise.
fn borrow_condition_is_anaphoric(sentence: &[OwnedLexToken]) -> bool {
    let Some(preprocess_grammar::BorrowStaticSentenceSurfaceTokens::Leading { condition, .. }) =
        preprocess_grammar::parse_borrow_static_sentence_surface_tokens(sentence)
    else {
        return false;
    };
    let words = crate::lexer::parser_token_word_refs(condition);
    let subject_start = usize::from(words.first() == Some(&"if"));
    matches!(
        words.get(subject_start).copied(),
        Some("it" | "that" | "the")
    )
}

fn rewrite_borrow_static_sentence(sentence: &[OwnedLexToken]) -> String {
    let rendered = || render_token_slice(sentence).trim().to_string();
    let Some(ability) = find_borrow_ability_source_phrase(sentence) else {
        return rendered();
    };
    let rendered_slice = |slice: &[OwnedLexToken]| render_token_slice(slice).trim().to_string();
    match preprocess_grammar::parse_borrow_static_sentence_surface_tokens(sentence) {
        Some(preprocess_grammar::BorrowStaticSentenceSurfaceTokens::Leading {
            condition,
            consequence,
        }) => rewrite_borrow_static_condition(condition, ability)
            .map(|rewritten| format!("as long as {rewritten}, {}", rendered_slice(consequence)))
            .unwrap_or_else(rendered),
        Some(preprocess_grammar::BorrowStaticSentenceSurfaceTokens::Trailing {
            prefix,
            condition,
        }) => rewrite_borrow_static_condition(condition, ability)
            .map(|rewritten| format!("{} as long as {rewritten}", rendered_slice(prefix)))
            .unwrap_or_else(rendered),
        None => rendered(),
    }
}

/// "The same is true for ..." sentences expanded into one sentence per
/// target, over the stage text's tokens — tokenized once for the whole line.
/// "If you cast a spell this way, mana of any type can be spent to cast it."
/// (Bloodsoaked Insight) restates the rider the permission grammar already
/// reads as "Mana of any type can be spent to cast spells this way."
fn rewrite_any_type_cast_rider_line(text: &str) -> String {
    text.replace(
        "if you cast a spell this way, mana of any type can be spent to cast it",
        "mana of any type can be spent to cast spells this way",
    )
}

/// "you recruit" (The Queen of Dale): the keyword action spelled out as its
/// reminder text so the ordinary draw/discard/create grammar executes it.
/// Rewrite gendered personal pronouns that Oracle text uses for named legendary
/// characters ("he gets +1/+1") onto the neutral object pronoun the grammar
/// understands. Possessives ("his", "her") are left alone: "her" is ambiguous
/// and both carry authored source surfaces elsewhere. The object pronoun
/// "him" is left alone for the same reason as "her" — the operand grammar
/// reads `it | him | her` directly, and rewriting only the masculine form
/// erased the authored surface that the feminine one keeps.
fn personal_pronoun_replacement(core: &str, opens_activated_effect: bool) -> Option<&'static str> {
    match core {
        // The effect-opening pronoun names the ability's source, not a cost object.
        "he" | "she" if opens_activated_effect => Some("this"),
        "he" | "she" => Some("it"),
        "he's" | "she's" => Some("it's"),
        "himself" | "herself" => Some("itself"),
        _ => None,
    }
}

/// Contextual recognition may revisit the authored stream to recover a card
/// name. Retain the same pronoun vocabulary as preprocessing without rendering
/// and re-lexing that stream or replacing its original source spans.
pub(super) fn rewrite_personal_pronouns_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut previous_ends_activation_cost = false;
    tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            // The text pass leaves quote-adjacent words untouched, including
            // literal names such as `named "He"`.
            let quote_adjacent = index
                .checked_sub(1)
                .and_then(|previous| tokens.get(previous))
                .is_some_and(|previous| previous.kind == TokenKind::Quote)
                || tokens
                    .get(index + 1)
                    .is_some_and(|next| next.kind == TokenKind::Quote);
            let replacement = (!quote_adjacent)
                .then(|| {
                    token.as_word().and_then(|word| {
                        personal_pronoun_replacement(
                            &word.to_ascii_lowercase(),
                            previous_ends_activation_cost,
                        )
                    })
                })
                .flatten();
            previous_ends_activation_cost = token.kind == TokenKind::Colon;
            replacement.map_or_else(
                || token.clone(),
                |word| OwnedLexToken::word(word, token.span),
            )
        })
        .collect()
}

fn rewrite_personal_pronouns_line(text: &str) -> String {
    let words: Vec<&str> = text.split(' ').collect();
    if !words.iter().any(|word| {
        matches!(
            word.trim_end_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':')),
            "he" | "she" | "he's" | "she's" | "himself" | "herself"
        )
    }) {
        return text.to_string();
    }
    let mut rewritten = Vec::with_capacity(words.len());
    let mut previous_ends_activation_cost = false;
    for word in words {
        let trailing_len = word.len()
            - word
                .trim_end_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':'))
                .len();
        let (core, trailing) = word.split_at(word.len() - trailing_len);
        let opens_activated_effect = previous_ends_activation_cost;
        previous_ends_activation_cost = trailing.contains(':');
        let replacement = personal_pronoun_replacement(core, opens_activated_effect);
        match replacement {
            Some(replacement) => rewritten.push(format!("{replacement}{trailing}")),
            None => rewritten.push(word.to_string()),
        }
    }
    rewritten.join(" ")
}

/// "Counter that spell unless its controller pays {4} instead if this spell
/// was cast using teamwork." names its spell-label condition after the
/// replacement action. The leading form ("If this spell was cast using
/// teamwork, counter that spell ... instead.") is the shape the statement
/// grammar reads as a self replacement of the preceding sentence, so move the
/// condition to the front of that sentence.
/// "if this land would enter, instead sacrifice each other permanent named
/// sheltered valley you control, then put this land onto the battlefield."
/// (Sheltered Valley) is a self entry replacement whose program runs before
/// the permanent enters (CR 614.1c, 614.12) — exactly the "as this land
/// enters, <program>." form. Rewrite it to that form; the returned flag keeps
/// the authored "instead" surface for rendering. Also accepts
/// "..., <program> instead, then put ...".
fn rewrite_entry_instead_then_put_line(text: &str) -> (String, bool) {
    let unchanged = || (text.to_string(), false);
    let Some(rest) = text.strip_prefix("if this ") else {
        return unchanged();
    };
    let Some(would_idx) = rest.find(" would enter") else {
        return unchanged();
    };
    let subject = &rest[..would_idx];
    if subject.is_empty() || subject.contains([',', '.']) {
        return unchanged();
    }
    let after = &rest[would_idx + " would enter".len()..];
    let after = after.strip_prefix(" the battlefield").unwrap_or(after);
    let Some(after) = after.strip_prefix(", ") else {
        return unchanged();
    };
    let body = after.trim_end().trim_end_matches('.');
    let tail = format!(", then put this {subject} onto the battlefield");
    let Some(program) = body.strip_suffix(tail.as_str()) else {
        return unchanged();
    };
    let program = if let Some(program) = program.strip_prefix("instead ") {
        program
    } else if let Some(program) = program.strip_suffix(" instead") {
        program
    } else {
        return unchanged();
    };
    if program.is_empty() || program.contains('.') {
        return unchanged();
    }
    (format!("as this {subject} enters, {program}."), true)
}

fn rewrite_trailing_instead_if_spell_label_line(text: &str) -> String {
    const MARKER: &str = " instead if this spell ";
    let Some(marker_idx) = text.find(MARKER) else {
        return text.to_string();
    };
    let sentence_start = text[..marker_idx].rfind(". ").map_or(0, |idx| idx + 2);
    let action = text[sentence_start..marker_idx].trim();
    let after_marker = &text[marker_idx + MARKER.len()..];
    let Some(period_rel) = after_marker.find('.') else {
        return text.to_string();
    };
    let predicate = after_marker[..period_rel].trim();
    let rest = &after_marker[period_rel..];
    if action.is_empty()
        || predicate.is_empty()
        || action.starts_with("if ")
        || action.contains(", ")
        || predicate.contains(", ")
    {
        return text.to_string();
    }
    format!(
        "{}if this spell {predicate}, {action} instead{rest}",
        &text[..sentence_start]
    )
}

fn expand_recruit_keyword_line(text: &str) -> String {
    const KEYWORD: &str = "recruit";
    const EXPANSION: &str = "draw a card, then discard a card. if you discarded a nonland card this way, create a 1/1 white human soldier creature token";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(KEYWORD) {
        let before = &rest[..idx];
        let tail = &rest[idx + KEYWORD.len()..];
        let word_start = before
            .chars()
            .last()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric());
        let word_end = tail
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric());
        // "Freedom Fighter Recruit's power ..." names a card, not the
        // keyword action.
        let possessive = tail.starts_with('\'') || tail.starts_with('\u{2019}');
        if word_start && word_end && !possessive {
            let trimmed = before.trim_end();
            if let Some(without_you) = trimmed.strip_suffix("you")
                && without_you
                    .chars()
                    .last()
                    .is_none_or(|ch| !ch.is_ascii_alphanumeric())
            {
                out.push_str(without_you);
                if !without_you.is_empty() && !without_you.ends_with(' ') {
                    out.push(' ');
                }
            } else {
                out.push_str(before);
            }
            out.push_str(EXPANSION);
        } else {
            out.push_str(&rest[..idx + KEYWORD.len()]);
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

fn expand_borrow_ability_line(text: &str) -> String {
    let Some(tokens) = stage_tokens(text) else {
        return text.trim().to_string();
    };
    let Some(document) = preprocess_grammar::parse_preprocess_sentence_list_tokens(&tokens) else {
        return rewrite_borrow_static_sentence(&tokens);
    };
    if document.sentences.len() < 2 {
        return rewrite_borrow_static_sentence(&tokens);
    }

    let mut expanded: Vec<String> = Vec::new();
    let mut expanded_tokens: Vec<Vec<OwnedLexToken>> = Vec::new();
    for sentence in document.sentences {
        if let Some(same_is_true) = preprocess_grammar::parse_same_is_true_surface_tokens(sentence)
            && let Some(base_sentence) = expanded_tokens.last().cloned()
        {
            let targets = same_is_true.targets;
            if !targets.is_empty() {
                if let Some(source_phrase) = find_borrow_ability_source_phrase(&base_sentence)
                    && let Some(occurrences) =
                        preprocess_grammar::parse_borrow_phrase_occurrences_tokens(
                            &base_sentence,
                            source_phrase,
                        )
                {
                    for target in &targets {
                        let replaced = apply_borrow_phrase_occurrences(
                            &base_sentence,
                            &occurrences,
                            target.as_str(),
                        );
                        expanded.push(rewrite_borrow_static_sentence(&replaced));
                        expanded_tokens.push(replaced);
                    }
                    continue;
                }

                if let Some((_subject, predicate)) =
                    split_same_is_true_subject_predicate(&base_sentence)
                {
                    for target in &targets {
                        expanded.push(format!("{} {}", target.trim(), predicate));
                        let mut tokens = crate::lexer::synthetic_phrase_tokens(target.trim());
                        tokens.extend(crate::lexer::synthetic_phrase_tokens(&predicate));
                        expanded_tokens.push(tokens);
                    }
                    continue;
                }
            }
        }

        // A follow-up sentence whose condition names an object from the
        // previous sentence ("If the creature you control has trample, ...")
        // is a resolution-time check on that object, not a static condition.
        if !expanded.is_empty() && borrow_condition_is_anaphoric(sentence) {
            expanded.push(render_token_slice(sentence).trim().to_string());
        } else {
            expanded.push(rewrite_borrow_static_sentence(sentence));
        }
        expanded_tokens.push(sentence.to_vec());
    }

    let mut joined = expanded.join(". ");
    if document.terminal_period {
        joined.push('.');
    }
    joined
}

fn rewrite_vote_count_followups_line(text: &str) -> String {
    fn rewrite_vote_count_sentence(sentence: &[OwnedLexToken]) -> String {
        let trimmed = render_token_slice(sentence).trim().to_string();
        match preprocess_grammar::parse_vote_count_rewrite_surface_tokens(sentence) {
            Some(preprocess_grammar::VoteCountRewriteSurface::DrawForEachVote { vote }) => {
                format!("For each {vote} vote, draw a card")
            }
            Some(preprocess_grammar::VoteCountRewriteSurface::SharedSubjectPair {
                subject,
                first_action,
                first_vote,
                second_action,
                second_vote,
            }) => {
                let first = format!("{subject} {first_action}");
                let second = format!("{subject} {second_action}");
                format!(
                    "For each {first_vote} vote, {}. For each {second_vote} vote, {}",
                    first.trim(),
                    second.trim()
                )
            }
            Some(preprocess_grammar::VoteCountRewriteSurface::TrailingForEach { head, vote }) => {
                format!("For each {vote} vote, {head}")
            }
            None => trimmed,
        }
    }

    let Some(tokens) = stage_tokens(text) else {
        return text.to_string();
    };
    let Some(document) = preprocess_grammar::parse_preprocess_sentence_list_tokens(&tokens) else {
        return text.to_string();
    };
    let rewritten = document
        .sentences
        .into_iter()
        .map(rewrite_vote_count_sentence)
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
    stage_tokens(line).is_some_and(|tokens| {
        preprocess_grammar::parse_ignorable_parenthetical_line_tokens(&tokens)
    })
}

/// Surface the preprocess writes for the card's own name inside an ability an
/// Equipment or Aura grants (`Equipped creature has "... Return Trusty
/// Boomerang to its owner's hand."`). The target and sacrifice grammars read
/// it as the object that granted the ability
/// (`CompilerReferenceTag::GrantingSource`), never as a filter.
pub const GRANTING_SOURCE_SURFACE: &str = "granting permanent";

/// [`GRANTING_SOURCE_SURFACE`] as parser words.
pub const GRANTING_SOURCE_SURFACE_WORDS: &[&str] = &["granting", "permanent"];

pub fn preprocess_document(
    card: CardBuilder,
    text: &str,
) -> Result<PreprocessedDocument, CardTextError> {
    let provenance = ProvenanceStore::capture(SourceUnitId(0), text, card.name_ref().trim());
    preprocess_document_with_provenance(card, text, provenance)
}

pub fn preprocess_document_with_provenance(
    mut card: CardBuilder,
    text: &str,
    mut provenance: ProvenanceStore,
) -> Result<PreprocessedDocument, CardTextError> {
    let cst =
        crate::front_end::parse_document_cst(provenance.source().id, text, card.name_ref().trim())?;
    for node in cst.lines.iter().flat_map(|line| &line.nodes) {
        let (kind, reminder_text) = match &node.kind {
            crate::front_end::CstNodeKind::SelfReference(_) => (
                SourceSliceKind::SelfReference,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::Quotation(_) => (
                SourceSliceKind::Quotation,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::AbilityWord { .. } => (
                SourceSliceKind::AbilityWord,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::ReminderText(decision) => {
                (SourceSliceKind::ReminderText, *decision)
            }
            crate::front_end::CstNodeKind::Symbol => (
                SourceSliceKind::Symbol,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::FaceSeparator => (
                SourceSliceKind::FaceSeparator,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::ChapterHeader { .. } => (
                SourceSliceKind::ChapterHeader,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::ClassHeader { .. } => (
                SourceSliceKind::ClassHeader,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::LevelHeader { .. } => (
                SourceSliceKind::LevelHeader,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::ModeMarker(_) => (
                SourceSliceKind::ModeMarker,
                ReminderTextDecision::NotReminderText,
            ),
            crate::front_end::CstNodeKind::Punctuation(_) => (
                SourceSliceKind::Punctuation,
                ReminderTextDecision::NotReminderText,
            ),
        };
        provenance.record_structural_span(kind, node.span, reminder_text);
    }
    fn strip_rounding_instruction_sentence(line: &str) -> String {
        const SENTENCE: &str = "round down each time.";
        let lower = line.to_ascii_lowercase();
        let Some(start) = lower.find(SENTENCE) else {
            return line.to_string();
        };
        let mut out = String::with_capacity(line.len());
        out.push_str(line[..start].trim_end());
        let rest = line[start + SENTENCE.len()..].trim_start();
        if !rest.is_empty() {
            out.push(' ');
            out.push_str(rest);
        }
        out
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

    fn normalize_non_metadata_line(
        raw_line: &str,
        line_index: usize,
        display_line_index: usize,
        full_name: &SelfReferenceName,
        short_name: &SelfReferenceName,
        preserve_source_surfaces: bool,
        typed_subject: &str,
        annotations: &mut ParseAnnotations,
        provenance: &mut ProvenanceStore,
    ) -> Result<Option<PreprocessedLine>, CardTextError> {
        let source_tokens = authored_rules_tokens(raw_line.trim(), line_index)?;
        let stripped = strip_parenthetical_segments(raw_line);
        // "Round down each time." (Hydroid Krasis) restates the default
        // rounding of the "half X" values in the same line.
        let stripped = strip_rounding_instruction_sentence(&stripped);
        if stripped.trim().is_empty() {
            return Ok(None);
        }

        let Some(normalized) = normalize_line_for_parse(
            stripped.as_str(),
            full_name,
            short_name,
            preserve_source_surfaces,
            typed_subject,
        ) else {
            if is_ignorable_unparsed_line(raw_line) {
                return Ok(None);
            }
            return Err(CardTextError::ParseError(format!(
                "rewrite preprocessing could not normalize line: '{raw_line}'"
            )));
        };

        let (entry_rewritten, entry_instead_surface) =
            rewrite_entry_instead_then_put_line(normalized.normalized.as_str());
        let expanded_normalized = expand_borrow_ability_line(entry_rewritten.as_str());
        let expanded_normalized = expand_recruit_keyword_line(expanded_normalized.as_str());
        let expanded_normalized = rewrite_any_type_cast_rider_line(expanded_normalized.as_str());
        let expanded_normalized = rewrite_personal_pronouns_line(expanded_normalized.as_str());
        let expanded_normalized =
            rewrite_trailing_instead_if_spell_label_line(expanded_normalized.as_str());
        let rewritten_normalized = rewrite_vote_count_followups_line(expanded_normalized.as_str());
        // Keep explicit exile/return sentences intact. The effect-sequence bundle
        // parser folds them into one source-leaves runtime effect while retaining
        // that the authored surface used two sentences.
        let normalized = if rewritten_normalized != normalized.normalized {
            let char_map =
                resized_char_map_for_rewrite(&normalized.char_map, &rewritten_normalized);
            NormalizedLine::from_char_map(normalized.original, rewritten_normalized, char_map)
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

        let mut tokens = lex_line(normalized.normalized.as_str(), line_index)?;
        // Normalization may rewrite rules, but casing of an unchanged token
        // remains useful lexical data (in particular for literal card names).
        // Restore only exact case-insensitive matches at the mapped source span.
        for token in &mut tokens {
            let source_span = crate::util::map_span_to_original(
                token.span,
                &normalized.normalized,
                &normalized.original,
                &normalized.char_map,
            );
            if let Some(authored) = normalized.original.get(source_span.start..source_span.end)
                && authored.eq_ignore_ascii_case(&token.slice)
            {
                token.set_literal_surface(authored);
            }
        }
        let mut semantic_facts = line_semantic_facts::parse_line_semantic_facts_tokens(&tokens);
        semantic_facts.intrinsic_basic_land_mana_reminder =
            preprocess_grammar::parse_intrinsic_basic_land_mana_reminder_tokens(
                &lex_line(raw_line.trim(), line_index)?,
            );
        if entry_instead_surface
            && let Some(as_enters) = semantic_facts.statement.as_enters_effect_program.as_mut()
        {
            as_enters.entry_instead_surface = true;
        }
        if let Some(as_enters) = semantic_facts.statement.as_enters_effect_program.as_mut()
            && as_enters.uses_enters_with_counter_surface
            && !as_enters.source_reference_enters_with_counter_surface
        {
            // "As <Name> enters, ... . <Name> enters with ...": the leading
            // name was normalized to `this`, while the follow-up subject kept
            // its proper-name surface. Both name the source.
            let words = crate::lexer::token_word_refs(&tokens)
                .into_iter()
                .map(str::to_ascii_lowercase)
                .collect::<Vec<_>>();
            let words = words.iter().map(String::as_str).collect::<Vec<_>>();
            as_enters.source_reference_enters_with_counter_surface = [full_name, short_name]
                .into_iter()
                .filter(|name| name.lexable())
                .any(|name| {
                    let mut expected = crate::lexer::token_word_refs(name.tokens())
                        .into_iter()
                        .map(str::to_ascii_lowercase)
                        .collect::<Vec<_>>();
                    if expected.is_empty() {
                        return false;
                    }
                    expected.extend(["enters".to_string(), "with".to_string()]);
                    let expected = expected.iter().map(String::as_str).collect::<Vec<_>>();
                    crate::word_primitives::parse_sequence_start(&words, &expected).is_some()
                });
        }
        semantic_facts.supported_sneak_form = supported_sneak_reminder(raw_line.trim(), line_index);
        semantic_facts.station_creature_threshold =
            station_reminder_threshold(raw_line.trim(), line_index);
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

    let card_name = card.name_ref().to_string();
    let front_face_name = card_name
        .split(" // ")
        .next()
        .unwrap_or(card_name.as_str())
        .trim()
        .to_string();
    // The card name is metadata: tokenized here, once per card, for the
    // short alias and for the keyword guards every line's name matching asks.
    let front_face_tokens = stage_tokens(front_face_name.as_str()).unwrap_or_default();
    let short_name = preprocess_grammar::parse_short_self_reference_name_tokens(
        front_face_name.as_str(),
        &front_face_tokens,
    );
    let full_name = SelfReferenceName::new(normalize_card_name_for_self_reference(
        front_face_name.as_str(),
    ));
    let short_name =
        SelfReferenceName::new(normalize_card_name_for_self_reference(short_name.as_str()));
    let source_surface_name_is_lexable =
        full_name.lexable() || (short_name.text != full_name.text && short_name.lexable());
    let mut annotations = ParseAnnotations::default();
    let mut items = Vec::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let structural_line = cst.line(line_index);
        if structural_line.is_some_and(|line| {
            matches!(
                &line.kind,
                crate::front_end::CstLineKind::Blank | crate::front_end::CstLineKind::FaceSeparator
            )
        }) {
            continue;
        }
        if structural_line.is_some_and(|line| {
            matches!(&line.kind, crate::front_end::CstLineKind::ReminderOnly)
                && !line.nodes.iter().any(|node| {
                    matches!(
                        &node.kind,
                        crate::front_end::CstNodeKind::ReminderText(
                            ReminderTextDecision::TreatedAsRulesText
                        )
                    )
                })
        }) && lex_line(raw_line.trim(), line_index).ok()
            .and_then(|tokens| preprocess_grammar::parse_intrinsic_basic_land_mana_reminder_tokens(&tokens))
            .is_none()
        {
            continue;
        }
        // Keep a recognized intrinsic reminder through preprocessing so the
        // document owner can validate its metadata and preserve source evidence.
        // Generic reminder-only lines still follow the CST exclusion above.
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(meta) = structural_line.and_then(|line| match &line.kind {
            crate::front_end::CstLineKind::Metadata(value) => {
                Some(materialize_structural_metadata(value))
            }
            _ => None,
        }) {
            let normalized = NormalizedLine::identity(line);
            card = crate::card_metadata::apply_compiler_metadata_line(card, meta.clone())?;
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

        let line_tokens = stage_tokens(line).unwrap_or_default();
        for (split_index, split_line) in split_parse_line_variants(line, &line_tokens)
            .into_iter()
            .enumerate()
        {
            let preserve_source_surfaces = source_surface_name_is_lexable
                && card.card_types_ref().iter().any(|card_type| {
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
            let typed_subject = crate::document_parser::named_source_subject_for_builder(&card);
            let virtual_line_index = line_index.saturating_mul(8).saturating_add(split_index);
            let split_tokens = lex_line(split_line.as_str(), virtual_line_index).ok();
            let looks_like_resolution_followup = split_tokens
                .as_deref()
                .map(looks_like_spell_resolution_followup_intro_lexed)
                .unwrap_or(false);
            let is_standalone_keyword_action = split_tokens
                .as_deref()
                .and_then(|tokens| {
                    split_lexed_sentences(tokens)
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

            if spell_card_prefers_resolution_line_merge(&card)
                && looks_like_resolution_followup
                && !is_standalone_keyword_action
                && let Some(PreprocessedItem::Line(previous)) = items.last_mut()
            {
                let combined_raw_line =
                    format!("{} {}", previous.info.raw_line.trim(), split_line.trim());
                let Some(normalized) = normalize_line_for_parse(
                    combined_raw_line.as_str(),
                    &full_name,
                    &short_name,
                    preserve_source_surfaces,
                    typed_subject,
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
                previous.tokens =
                    lex_line(normalized.normalized.as_str(), previous.info.line_index)?;
                // The authored stream follows the authored line: readers of
                // `source_tokens` must see the merged line, not the first half.
                previous.info.source_tokens =
                    authored_rules_tokens(combined_raw_line.trim(), previous.info.line_index)?;
                previous.info.raw_line = combined_raw_line;
                previous.info.normalized = normalized.clone();
                continue;
            }
            if let Some(parsed_line) = normalize_non_metadata_line(
                split_line.as_str(),
                virtual_line_index,
                line_index,
                &full_name,
                &short_name,
                preserve_source_surfaces,
                typed_subject,
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
        let card = card.oracle_text(oracle_text);
        return Ok(PreprocessedDocument {
            card,
            annotations,
            provenance,
            cst,
            items,
        });
    }

    Ok(PreprocessedDocument {
        card,
        annotations,
        provenance,
        cst,
        items,
    })
}

pub fn make_line_info(
    line_index: usize,
    raw_line: impl Into<String>,
    normalized: NormalizedLine,
) -> LineInfo {
    let raw_line = raw_line.into();
    let source_tokens = authored_rules_tokens(raw_line.as_str(), line_index).unwrap_or_default();
    let mut semantic_facts = crate::model::facts::LineSemanticFacts::default();
    semantic_facts.intrinsic_basic_land_mana_reminder = lex_line(&raw_line, line_index)
        .ok().and_then(|tokens| preprocess_grammar::parse_intrinsic_basic_land_mana_reminder_tokens(&tokens));
    semantic_facts.station_creature_threshold = station_reminder_threshold(&raw_line, line_index);
    semantic_facts.supported_sneak_form = supported_sneak_reminder(&raw_line, line_index);
    LineInfo {
        line_index,
        display_line_index: line_index,
        raw_line,
        source_tokens,
        normalized,
        semantic_facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CardId;
    use ironsmith_core::card::CardBuilder;


    #[test]
    fn intrinsic_reminder_survives_cst_exclusion_as_typed_source_evidence() {
        let document = preprocess_document(CardBuilder::new(CardId::new(), "Typed reminder"),
            "({T}: Add {G}.)\nType: Land — Forest").unwrap();
        let PreprocessedItem::Line(line) = &document.items[0] else { panic!("typed reminder line"); };
        assert_eq!(line.info.semantic_facts.intrinsic_basic_land_mana_reminder,
            Some(vec![crate::types::Subtype::Forest]));
        assert_eq!(line.info.raw_line, "({T}: Add {G}.)");
        assert!(document.card.oracle_text_ref().contains("({T}: Add {G}.)"));
    }

    #[test]
    fn authored_rules_strip_reminders_before_keyword_recognition() {
        let raw = "Flashback—Sacrifice a Mountain. (You may cast this card from your graveyard for its flashback cost. Then exile it.)";
        let document =
            preprocess_document(CardBuilder::new(CardId::new(), "Lava Dart"), raw).unwrap();
        let PreprocessedItem::Line(line) = &document.items[0] else {
            panic!("expected rules line")
        };
        let full_card = preprocess_document(
            CardBuilder::new(CardId::new(), "Lava Dart"),
            &format!("Lava Dart deals 1 damage to any target.\n{raw}"),
        )
        .unwrap();
        crate::document_parser::recognize_document(&full_card, false)
            .expect("the complete Lava Dart must recognize in strict mode");
        assert_eq!(line.info.raw_line, raw);
        assert_eq!(
            render_token_slice(&line.info.source_tokens),
            render_token_slice(&lex_line("Flashback—Sacrifice a Mountain.", 0).unwrap())
        );
        let keyword = crate::keyword_registry::recognize_keyword_line(line)
            .unwrap()
            .expect("flashback should parse with reminder text");
        let crate::recognized_document::KeywordLinePayload::Ast(ast) = keyword.payload else {
            panic!("expected AST")
        };
        assert!(matches!(
            *ast,
            crate::cards::builders::LineAst::AlternativeCastingMethod(
                crate::model::CompilerAlternativeCastingMethod::Flashback { .. }
            )
        ));
    }

    #[test]
    fn authored_rules_preserve_spans_after_nested_reminders() {
        let raw = "Draw a card. (Reminder (nested).) Then discard a card.";
        let tokens = authored_rules_tokens(raw, 3).unwrap();
        assert!(!render_token_slice(&tokens).contains("Reminder"));
        let then = tokens.iter().find(|token| token.slice == "Then").unwrap();
        assert_eq!(then.span.start, raw.find("Then").unwrap());
        assert_eq!(&raw[then.span.start..then.span.end], "Then");
        let info = make_line_info(3, raw, NormalizedLine::identity(raw));
        assert_eq!(
            render_token_slice(&info.source_tokens),
            render_token_slice(&tokens)
        );
    }

    #[test]
    fn authored_rules_keep_functional_parentheticals_only() {
        let mana = "({T}: Add {G}.)";
        assert_eq!(
            render_token_slice(&authored_rules_tokens(mana, 0).unwrap()),
            "{T}: Add {G}."
        );
        let text = "It's an enchantment in addition to its other types. (It's not a creature.) (Reminder.)";
        let tokens = authored_rules_tokens(text, 0).unwrap();
        let rendered = render_token_slice(&tokens);
        assert!(rendered.contains("not a creature"), "{rendered}");
        assert!(!rendered.contains("Reminder"), "{rendered}");
    }

    #[test]
    fn preprocessing_extracts_station_fact_before_stripping_reminder() {
        let raw = "Station (This artifact is an artifact creature at 12+.)";
        let document =
            preprocess_document(CardBuilder::new(CardId::new(), "Station Test"), raw).unwrap();
        let PreprocessedItem::Line(line) = &document.items[0] else {
            panic!("expected station")
        };
        assert_eq!(render_token_slice(&line.info.source_tokens), "Station");
        assert_eq!(
            line.info.semantic_facts.station_creature_threshold,
            Some(12)
        );
    }

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
        let card = CardBuilder::new(CardId::new(), "Metadata Variant");
        let preprocessed = preprocess_document(
            card,
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
            preprocessed.card.build().first_printed_set_name.as_deref(),
            Some("Antiquities")
        );
    }

    #[test]
    fn created_token_lifecycle_normalizes_named_source_after_token_name() {
        let document = preprocess_document(
            CardBuilder::new(CardId::new(), "Stangg"),
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
            split_parse_line_variants_text(
                "As an additional cost to cast this spell, discard a card. Draw two cards."
            ),
            vec![
                "As an additional cost to cast this spell, discard a card.".to_string(),
                "Draw two cards.".to_string(),
            ]
        );

        let flashback = "Flashback {8}{B}{B}. This spell costs {X} less to cast this way, where X is the greatest mana value of a commander you own on the battlefield or in the command zone.";
        assert_eq!(
            split_parse_line_variants_text(flashback),
            vec![flashback.to_string()],
            "flashback-scoped cost adjustments must reach the compound keyword parser"
        );

        assert_eq!(
            strip_parenthetical_segments(
                "It's an enchantment in addition to its other types. (It's not a creature.)"
            ),
            "It's an enchantment in addition to its other types. It's not a creature."
        );

        let normalized =
            normalize_line_for_parse_text("Draw a card as it resolves.", "", "", false)
                .expect("resolution line should normalize");
        assert_eq!(normalized.normalized, "draw a card.");
    }

    #[test]
    fn preprocess_preserves_empower_jace_keyword_subtype_on_a_jace_source() {
        let line = normalize_line_for_parse_text(
            "Empower Jace X, where X is the number of Islands you control.",
            "jace, reality sculptor",
            "jace",
            false,
        )
        .unwrap();
        assert!(
            line.normalized.starts_with("empower jace x"),
            "{}",
            line.normalized
        );
        let ordinary = normalize_line_for_parse_text(
            "Put a loyalty counter on Jace.",
            "jace, reality sculptor",
            "jace",
            false,
        )
        .unwrap();
        assert!(
            !ordinary.normalized.contains("on jace"),
            "ordinary source references still normalize"
        );
    }

    #[test]
    fn preprocess_preserves_typed_multiword_keyword_action_matching_card_name() {
        let document = preprocess_document(
            CardBuilder::new(CardId::new(), "Manifest Dread"),
            "Manifest dread.",
        )
        .expect("the keyword action should preprocess without becoming a source reference");
        let Some(PreprocessedItem::Line(line)) = document.items.first() else {
            panic!("expected one preprocessed keyword-action line");
        };
        assert_eq!(line.info.normalized.normalized, "manifest dread.");

        let reference = normalize_line_for_parse_text(
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
            CardBuilder::new(CardId::new(), "Coward // Killer")
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
                &stage_tokens(
                    "as long as a creature with flying is in your graveyard, creatures you control have flying"
                )
                .expect("lexes")
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
            CardBuilder::new(CardId::new(), "Loxodon Peacekeeper"),
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
            let document =
                preprocess_document(CardBuilder::new(CardId::new(), "Preprocess Test"), oracle)
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

#[cfg(test)]
#[path = "preprocess_unicode_tests.rs"]
mod unicode_tests;

#[cfg(test)]
#[test]
fn authored_pronoun_retry_keeps_original_spans_and_literal_names() {
    let tokens = lex_line("When Madame Masque enters, she connives.", 7).unwrap();
    let rewritten = rewrite_personal_pronouns_tokens(&tokens);
    assert_eq!(tokens.len(), rewritten.len());
    for (before, after) in tokens.iter().zip(&rewritten) {
        assert_eq!(before.span, after.span);
    }
    assert!(rewritten.iter().any(|token| token.is_word("it")));
    assert!(!rewritten.iter().any(|token| token.is_word("she")));
    let tokens = lex_line("Create a token named \"He\".", 0).unwrap();
    assert_eq!(rewrite_personal_pronouns_tokens(&tokens), tokens);
}
