use crate::PtValue;
use crate::ability::ActivationTiming;
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, LineAst, ParseAnnotations, ParsedLevelAbilityItemAst,
    PredicateAst, TextSpan,
};
use winnow::Parser;
use winnow::error::ModalResult as WResult;
use winnow::stream::Stream;
use winnow::token::any;

use super::activation_and_restrictions::keyword_activated_lines::{
    parse_channel_line_lexed, parse_cycling_line_lexed, parse_equip_line_lexed,
};
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::cst::{
    ActivatedLineCst, KeywordLineKindCst, LevelHeaderCst, LevelItemCst, LevelItemKindCst,
    MetadataLineCst, ModalBlockCst, ModalModeCst, RewriteDocumentCst, RewriteLineCst,
    SagaChapterLineCst, StatementLineCst, StaticLineCst, TriggerIntroCst, TriggeredLineCst,
    UnsupportedLineCst,
};
use super::cst_lowering::lower_non_metadata_rewrite_line_cst;
use super::grammar::abilities::{
    is_activate_only_once_each_turn_line_lexed,
    is_doesnt_untap_during_your_untap_step_line_lexed,
    is_land_reveal_enters_static_line_lexed,
    is_land_reveal_enters_tapped_followup_line_lexed,
    is_opening_hand_begin_game_static_line_lexed, is_ward_or_echo_static_prefix_line_lexed,
    split_nested_combat_whenever_clause_lexed,
};
use super::grammar::primitives as grammar;
use super::grammar::structure::split_lexed_sentences;
use super::ir::{RewriteSemanticDocument, RewriteSemanticItem};
use super::keyword_registry::{parse_keyword_line_cst, rewrite_keyword_dash_parse_tokens};
use super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed;
use super::leaf::{lower_activation_cost_cst, parse_activation_cost_tokens_rewrite};
use super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, lex_line, render_token_slice,
    token_word_refs, trim_lexed_commas,
};
use super::preprocess::{
    PreprocessedDocument, PreprocessedItem, PreprocessedLine, preprocess_document,
};
use super::rule_engine::{LexRuleHeadHint, LexRuleHintIndex, build_lex_rule_hint_index};
use super::token_primitives::{
    clone_sentence_chunk_tokens, find_index as find_token_index, lexed_head_words,
    lexed_tokens_contain_non_prefix_instead, remove_copy_exception_type_removal_lexed,
    rewrite_followup_intro_to_if_lexed, split_em_dash_label_prefix,
    split_em_dash_label_prefix_tokens, str_contains, str_ends_with, str_ends_with_char,
    str_split_once, str_split_once_char, str_starts_with, str_starts_with_char, str_strip_prefix,
    str_strip_suffix,
};
use super::util::{
    parse_level_header, parse_level_up_line_lexed, parse_power_toughness,
    parse_saga_chapter_prefix, parser_trace, parser_trace_enabled,
    preserve_keyword_prefix_for_parse,
};
use std::sync::LazyLock;

mod block_parsing;
mod line_cst_parsing;
mod line_dispatch;
mod line_family_handlers;
mod statement_cst_support;
mod unsupported;

use block_parsing::{try_parse_level_header_block, try_parse_modal_bullet_block};
use line_cst_parsing::{
    parse_level_item_cst, parse_modal_mode_cst, parse_saga_chapter_line_cst, parse_static_line_cst,
    parse_triggered_line_cst, strict_unsupported_triggered_line_error,
};
use line_dispatch::{LineDispatchResult, dispatch_standard_line_cst};
#[cfg(test)]
use statement_cst_support::looks_like_statement_line;
use statement_cst_support::{
    extend_triggered_line_with_result_followups, looks_like_statement_line_lexed,
    normalize_statement_parse_groups_lexed, parse_colon_nonactivation_statement_fallback,
    parse_statement_line_cst,
};
use unsupported::diagnose_known_unsupported_rewrite_line;

fn lexed_tokens(text: &str, line_index: usize) -> Result<Vec<OwnedLexToken>, CardTextError> {
    lex_line(text, line_index)
}

fn is_bullet_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if str_starts_with_char(trimmed, '•') || str_starts_with_char(trimmed, '*') {
        return true;
    }
    if let Some(rest) = str_strip_prefix(trimmed, "-") {
        let next = rest.chars().next();
        if next.is_some_and(|ch| ch.is_ascii_digit()) {
            return false;
        }
        return true;
    }
    false
}

fn parse_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> Option<TriggerIntroCst> {
    if let Some((intro, _)) = grammar::parse_prefix(
        tokens,
        winnow::combinator::alt((
            grammar::kw("when").value(TriggerIntroCst::When),
            grammar::kw("whenever").value(TriggerIntroCst::Whenever),
        )),
    ) {
        return Some(intro);
    }

    super::parser_support::is_at_trigger_intro_lexed(tokens, 0).then_some(TriggerIntroCst::At)
}

fn strip_trigger_frequency_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    if let Some((_, rest)) = grammar::strip_lexed_suffix_phrases(
        tokens,
        &[
            &["for", "the", "first", "time", "each", "turn"][..],
            &["for", "the", "first", "time", "this", "turn"][..],
        ],
    ) {
        return (rest, Some(1));
    }

    (tokens, None)
}

fn strip_trailing_trigger_cap_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    let cap_suffixes = [
        &[
            "this", "ability", "triggers", "only", "once", "each", "turn",
        ][..],
        &[
            "this", "ability", "triggers", "only", "twice", "each", "turn",
        ][..],
        &["do", "this", "only", "once", "each", "turn"][..],
        &["do", "this", "only", "twice", "each", "turn"][..],
    ];
    let Some((phrase, head)) = grammar::strip_lexed_suffix_phrases(tokens, &cap_suffixes) else {
        return (tokens, None);
    };
    let count = if phrase
        == [
            "this", "ability", "triggers", "only", "once", "each", "turn",
        ] {
        1
    } else if phrase == ["do", "this", "only", "once", "each", "turn"] {
        1
    } else {
        2
    };
    if !head.last().is_some_and(|token| token.is_period()) {
        return (tokens, None);
    }
    (&head[..head.len() - 1], Some(count))
}

fn line_starts_with_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    if super::parser_support::looks_like_reflexive_followup_intro_lexed(tokens) {
        return false;
    }
    parse_trigger_intro_tokens(tokens).is_some()
}

fn is_if_you_do_exile_followup_tokens(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        (
            grammar::phrase(&["if", "you", "do"]),
            winnow::combinator::opt(grammar::comma()),
            grammar::kw("exile"),
        )
            .void()
            .parse_next(input)
    })
    .is_some()
}

fn should_try_combined_static_tokens(
    line_tokens: &[OwnedLexToken],
    next_line_tokens: &[OwnedLexToken],
) -> bool {
    (is_land_reveal_enters_static_line_lexed(line_tokens)
        && is_land_reveal_enters_tapped_followup_line_lexed(next_line_tokens))
        || (is_opening_hand_begin_game_static_line_lexed(line_tokens)
            && is_if_you_do_exile_followup_tokens(next_line_tokens))
}

#[derive(Debug, Clone)]
struct TriggeredSplitCandidate {
    trigger_text: String,
    trigger_parse_tokens: Vec<OwnedLexToken>,
    effect_text: String,
    effect_parse_tokens: Vec<OwnedLexToken>,
    intervening_if: Option<PredicateAst>,
    max_triggers_per_turn: Option<u32>,
}

impl TriggeredSplitCandidate {
    fn into_cst(
        self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> TriggeredLineCst {
        TriggeredLineCst {
            info: line.info.clone(),
            full_text: render_token_slice(full_parse_tokens).trim().to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            trigger_text: self.trigger_text,
            trigger_parse_tokens: self.trigger_parse_tokens,
            effect_text: self.effect_text,
            effect_parse_tokens: self.effect_parse_tokens,
            intervening_if: self.intervening_if,
            max_triggers_per_turn: self.max_triggers_per_turn,
            chosen_option_label: None,
        }
    }
}

#[derive(Debug, Clone)]
enum TriggeredSplitProbe {
    Empty,
    Supported(TriggeredSplitCandidate),
    Unsupported {
        candidate: TriggeredSplitCandidate,
        trigger_error: Option<CardTextError>,
        effect_error: Option<CardTextError>,
    },
}

impl TriggeredSplitProbe {
    fn supported_cst(
        &self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> Option<TriggeredLineCst> {
        match self {
            Self::Supported(candidate) => Some(candidate.clone().into_cst(line, full_parse_tokens)),
            _ => None,
        }
    }

    fn fallback_cst(
        &self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> Option<TriggeredLineCst> {
        match self {
            Self::Supported(candidate) => Some(candidate.clone().into_cst(line, full_parse_tokens)),
            Self::Unsupported { candidate, .. } => {
                Some(candidate.clone().into_cst(line, full_parse_tokens))
            }
            Self::Empty => None,
        }
    }

    fn preferred_error(&self) -> Option<CardTextError> {
        match self {
            Self::Unsupported {
                trigger_error,
                effect_error,
                ..
            } => effect_error.clone().or_else(|| trigger_error.clone()),
            Self::Empty | Self::Supported(_) => None,
        }
    }
}

fn render_triggered_split_candidate(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    trailing_cap: Option<u32>,
) -> Option<TriggeredSplitCandidate> {
    let trigger_candidate_tokens = trim_lexed_commas(trigger_tokens);
    let effect_candidate_tokens = trim_lexed_commas(effect_tokens);
    if trigger_candidate_tokens.is_empty() || effect_candidate_tokens.is_empty() {
        return None;
    }

    let (trigger_tokens, max_triggers_per_turn) =
        strip_trigger_frequency_suffix_tokens(trigger_candidate_tokens);
    let trigger_text = render_token_slice(trigger_tokens).trim().to_string();
    let effect_text = render_token_slice(effect_candidate_tokens)
        .trim()
        .to_string();
    if trigger_text.is_empty() || effect_text.is_empty() {
        return None;
    }

    Some(TriggeredSplitCandidate {
        trigger_text,
        trigger_parse_tokens: trigger_tokens.to_vec(),
        effect_text,
        effect_parse_tokens: effect_candidate_tokens.to_vec(),
        intervening_if,
        max_triggers_per_turn: max_triggers_per_turn.or(trailing_cap),
    })
}

fn probe_triggered_split(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    trailing_cap: Option<u32>,
) -> TriggeredSplitProbe {
    let trigger_candidate_tokens = trim_lexed_commas(trigger_tokens);
    let effect_candidate_tokens = trim_lexed_commas(effect_tokens);
    let Some(candidate) = render_triggered_split_candidate(
        trigger_tokens,
        effect_tokens,
        intervening_if,
        trailing_cap,
    ) else {
        return TriggeredSplitProbe::Empty;
    };
    let (trigger_tokens, _) = strip_trigger_frequency_suffix_tokens(trigger_candidate_tokens);

    let trigger_error = parse_trigger_clause_lexed(trigger_tokens).err();
    let effect_error = parse_effect_sentences_lexed(effect_candidate_tokens).err();
    if trigger_error.is_none() && effect_error.is_none() {
        TriggeredSplitProbe::Supported(candidate)
    } else {
        TriggeredSplitProbe::Unsupported {
            candidate,
            trigger_error,
            effect_error,
        }
    }
}

fn token_words_match(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens);
    words.len() == expected.len() && words.slice_eq(0, expected)
}

fn token_words_match_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_words_match(tokens, phrase))
}

fn token_words_have_prefix(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    TokenWordView::new(tokens).starts_with(expected)
}

fn token_words_have_any_prefix(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_words_have_prefix(tokens, phrase))
}

fn token_words_have_suffix(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens);
    words.len() >= expected.len() && words.slice_eq(words.len() - expected.len(), expected)
}

pub(crate) fn split_compound_buff_and_unblockable_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let words = TokenWordView::new(tokens);
    let gets_idx = words.find_word("gets")?;
    let and_idx = words.find_phrase_start(&["and", "cant", "be", "blocked"])?;
    if and_idx + 4 != words.len() {
        return None;
    }

    let subject_token_end = words.token_index_for_word_index(gets_idx)?;
    let and_token_idx = words.token_index_for_word_index(and_idx)?;
    let cant_token_idx = words.token_index_for_word_index(and_idx + 1)?;
    if subject_token_end == 0
        || subject_token_end >= and_token_idx
        || cant_token_idx <= and_token_idx
    {
        return None;
    }

    let left_tokens = tokens[..and_token_idx].to_vec();
    let mut right_tokens =
        Vec::with_capacity(subject_token_end + tokens.len().saturating_sub(cant_token_idx));
    right_tokens.extend_from_slice(&tokens[..subject_token_end]);
    right_tokens.extend_from_slice(&tokens[cant_token_idx..]);
    Some((left_tokens, right_tokens))
}

fn parse_former_section9_unsupported_line_cst(
    line: &PreprocessedLine,
) -> Option<UnsupportedLineCst> {
    let normalized = line.info.normalized.normalized.as_str();
    let canonical = normalized.replace(['\'', '’'], "");
    let canonical_without_hyphens = canonical.replace('-', "");
    let matches_former_section9_gap = canonical
        == "destroy target creature if its white. a creature destroyed this way cant be regenerated."
        || canonical
            == "create two 1/1 white kithkin soldier creature tokens if {w} was spent to cast this spell. counter up to one target creature spell if {u} was spent to cast this spell."
        || canonical == "{t}, sacrifice x goats: add x mana of any one color. you gain x life."
        || canonical
            == "shuffle your library, then exile the top four cards. you may cast any number of spells with mana value 5 or less from among them without paying their mana costs. lands you control dont untap during your next untap step."
        || canonical_without_hyphens
            == "exile target nontoken creature you own and the top two cards of your library in a facedown pile, shuffle that pile, then cloak those cards. they enter tapped."
        || canonical
            == "destroy target creature unless its controller pays life equal to its toughness. a creature destroyed this way cant be regenerated."
        || canonical
            == "destroy all lands or all creatures. creatures destroyed this way cant be regenerated."
        || canonical
            == "destroy two target nonblack creatures unless either one is a color the other isnt. they cant be regenerated.";
    matches_former_section9_gap.then(|| UnsupportedLineCst {
        info: line.info.clone(),
        reason_code: "former-section9-line-not-yet-supported",
    })
}

fn strip_non_keyword_label_prefix_lexed(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if looks_like_numeric_result_prefix_lexed(tokens) {
        return tokens;
    }
    while let Some((label, body_tokens)) = split_label_prefix_lexed(tokens) {
        if preserve_keyword_prefix_for_parse(label.as_str()) {
            break;
        }
        tokens = body_tokens;
    }

    tokens
}

fn tokens_after_non_keyword_label_prefix(line: &PreprocessedLine) -> Option<&[OwnedLexToken]> {
    let stripped = strip_non_keyword_label_prefix_lexed(&line.tokens);
    (stripped.len() != line.tokens.len()).then_some(stripped)
}

fn looks_like_numeric_result_prefix_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Number)
    ) && matches!(
        tokens.get(1).map(|token| token.kind),
        Some(TokenKind::Dash | TokenKind::EmDash)
    ) && matches!(
        tokens.get(2).map(|token| token.kind),
        Some(TokenKind::Number)
    ) && tokens
        .iter()
        .skip(3)
        .any(|token| token.kind == TokenKind::Pipe)
}

fn should_skip_keyword_action_static_probe(tokens: &[OwnedLexToken]) -> bool {
    token_words_have_suffix(tokens, &["cant", "be", "blocked"])
        && !token_words_have_any_prefix(tokens, &[&["this"], &["it"]])
}

fn looks_like_activation_cost_prefix(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    if str_starts_with_char(trimmed, '{')
        || str_starts_with(trimmed, "+")
        || str_starts_with(trimmed, "-")
        || str_starts_with_char(trimmed, '−')
    {
        return true;
    }
    let first = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '\'')
        .to_ascii_lowercase();
    matches!(
        first.as_str(),
        "tap"
            | "untap"
            | "pay"
            | "discard"
            | "sacrifice"
            | "exile"
            | "mill"
            | "remove"
            | "put"
            | "return"
    )
}

#[cfg(test)]
fn looks_like_static_line(normalized: &str) -> bool {
    lex_line(normalized, 0)
        .ok()
        .is_some_and(|tokens| looks_like_static_line_tokens(&tokens))
}

fn looks_like_static_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        super::grammar::structure::classify_static_line_family_lexed(tokens),
        Some(
            super::grammar::structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep
                | super::grammar::structure::StaticLineFamily::Generic
        )
    )
}

fn looks_like_static_line_lexed(line: &PreprocessedLine) -> bool {
    if let Some(tokens) = tokens_after_non_keyword_label_prefix(line) {
        return looks_like_static_line_tokens(&tokens);
    }
    looks_like_static_line_tokens(&line.tokens)
}

fn parse_segment_len_until_colon_outside_quotes<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let initial_len = input.len();
    let mut inside_quotes = false;

    while let Some(token) = input.peek_token() {
        if token.kind == TokenKind::Quote {
            grammar::quote().parse_next(input)?;
            inside_quotes = !inside_quotes;
            continue;
        }
        if token.kind == TokenKind::Colon && !inside_quotes {
            return Ok(initial_len - input.len());
        }

        any.parse_next(input)?;
    }

    Err(grammar::backtrack_err("colon", "colon outside quotes"))
}

pub(crate) fn split_lexed_once_on_colon_outside_quotes(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (left_len, rest) =
        grammar::parse_prefix(tokens, parse_segment_len_until_colon_outside_quotes)?;
    let (_, right_tokens) = grammar::parse_prefix(rest, grammar::colon())?;
    Some((&tokens[..left_len], right_tokens))
}

fn split_label_prefix(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    let (label, body) = str_split_once_char(trimmed, '—')?;
    let label = label.trim();
    let body = body.trim();
    (!label.is_empty() && !body.is_empty() && !str_contains(label, ".")).then_some((label, body))
}

fn split_label_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<(String, &[OwnedLexToken])> {
    if looks_like_numeric_result_prefix_lexed(tokens) {
        return None;
    }
    split_em_dash_label_prefix(tokens)
}

fn is_nonkeyword_choice_labeled_line(line: &PreprocessedLine) -> bool {
    split_label_prefix_lexed(&line.tokens)
        .is_some_and(|(label, _)| !preserve_keyword_prefix_for_parse(label.as_str()))
}

fn labeled_choice_block_has_peer(items: &[PreprocessedItem], idx: usize) -> bool {
    let mut probe = idx;
    while probe > 0 {
        probe -= 1;
        match items.get(probe) {
            Some(PreprocessedItem::Line(line)) if is_nonkeyword_choice_labeled_line(line) => {
                return true;
            }
            Some(PreprocessedItem::Line(_)) => break,
            Some(PreprocessedItem::Metadata(_)) => continue,
            None => break,
        }
    }

    let mut probe = idx + 1;
    while let Some(item) = items.get(probe) {
        match item {
            PreprocessedItem::Line(line) if is_nonkeyword_choice_labeled_line(line) => {
                return true;
            }
            PreprocessedItem::Line(_) => break,
            PreprocessedItem::Metadata(_) => {
                probe += 1;
                continue;
            }
        }
    }

    false
}

fn normalize_trailing_keyword_activation_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return None;
    }

    for split_idx in 1..sentences.len() {
        let prefix = clone_sentence_chunk_tokens(tokens, &sentences[..split_idx])?;
        let suffix = clone_sentence_chunk_tokens(tokens, &sentences[split_idx..])?;
        let Some((label, body_tokens)) = split_label_prefix_lexed(&suffix) else {
            continue;
        };
        if !preserve_keyword_prefix_for_parse(label.as_str())
            || split_lexed_once_on_colon_outside_quotes(body_tokens).is_none()
        {
            continue;
        }
        return Some((prefix, suffix));
    }

    None
}

fn preflight_known_strict_unsupported(text: &str) -> Option<CardTextError> {
    let normalized = text.to_ascii_lowercase();
    if str_contains(
        normalized.as_str(),
        "if your life total is less than or equal to half your starting life total plus one",
    ) {
        return Some(CardTextError::ParseError(
            "unsupported predicate".to_string(),
        ));
    }
    None
}

fn normalize_named_source_sentence_for_builder(
    builder: &CardDefinitionBuilder,
    text: &str,
) -> Option<String> {
    let trimmed = text.trim();
    let subject = if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Creature)
    {
        "this creature"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Land)
    {
        "this land"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Artifact)
    {
        "this artifact"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Enchantment)
    {
        "this enchantment"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Planeswalker)
    {
        "this planeswalker"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Battle)
    {
        "this battle"
    } else {
        "this permanent"
    };
    let lower = trimmed.to_ascii_lowercase();

    let name = builder.card_builder.name_ref();
    if !name.is_empty() {
        let name_lower = name.to_ascii_lowercase();
        if let Some(remainder) = str_strip_prefix(lower.as_str(), &(name_lower + " ")) {
            return Some(format!("{subject} {remainder}"));
        }
    }

    let (_, rest) = str_split_once(lower.as_str(), " enters ")?;
    Some(format!("{subject} enters {rest}"))
}

fn normalize_named_source_trigger_for_builder(
    builder: &CardDefinitionBuilder,
    text: &str,
) -> Option<String> {
    let trimmed = text.trim();
    let subject = if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Creature)
    {
        "this creature"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Land)
    {
        "this land"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Artifact)
    {
        "this artifact"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Enchantment)
    {
        "this enchantment"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Planeswalker)
    {
        "this planeswalker"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Battle)
    {
        "this battle"
    } else {
        "this permanent"
    };

    let name = builder.card_builder.name_ref();
    if name.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    let name_lower = name.to_ascii_lowercase();
    for intro in ["whenever", "when", "at"] {
        let prefix = format!("{intro} {name_lower} ");
        if let Some(rest) = str_strip_prefix(lower.as_str(), prefix.as_str()) {
            return Some(format!("{intro} {subject} {rest}"));
        }
    }

    None
}

fn strip_non_keyword_label_prefix(text: &str) -> &str {
    let mut current = text.trim();
    if looks_like_numeric_result_prefix_text(current) {
        return current;
    }
    while let Some((label, body)) = split_label_prefix(current) {
        if preserve_keyword_prefix_for_parse(label) {
            break;
        }
        current = body.trim();
    }
    current
}

fn looks_like_numeric_result_prefix_text(text: &str) -> bool {
    let trimmed = text.trim_start();
    let Some((head, rest)) = trimmed.split_once('—').or_else(|| trimmed.split_once('-')) else {
        return false;
    };
    if !head.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    let Some((range_end, _)) = rest.split_once('|') else {
        return false;
    };
    range_end.trim().chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
    use crate::ids::CardId;
    use crate::types::CardType;

    use super::super::grammar::structure::{
        StatementLineFamily, StaticLineFamily, classify_statement_line_family_lexed,
        classify_static_line_family_lexed,
    };
    use super::{
        PreprocessedItem, TriggeredSplitProbe, classify_unsupported_line_reason,
        diagnose_known_unsupported_rewrite_line,
        is_doesnt_untap_during_your_untap_step_line_lexed, is_if_you_do_exile_followup_tokens,
        is_land_reveal_enters_static_line_lexed,
        is_land_reveal_enters_tapped_followup_line_lexed,
        is_opening_hand_begin_game_static_line_lexed, is_ward_or_echo_static_prefix_line_lexed,
        lex_line, looks_like_statement_line,
        looks_like_statement_line_lexed, looks_like_static_line, looks_like_static_line_lexed,
        normalize_statement_parse_groups_lexed,
        normalize_trailing_keyword_activation_sentence_lexed,
        parse_colon_nonactivation_statement_fallback, parse_keyword_line_cst, parse_level_item_cst,
        parse_statement_line_cst, parse_static_line_cst, parse_triggered_line_cst,
        preprocess_document, probe_triggered_split, render_token_slice,
        rewrite_keyword_dash_parse_tokens, split_activation_text_parts_lexed, split_label_prefix,
        split_label_prefix_lexed, split_reveal_first_draw_line_rewrite_lexed,
        split_trigger_sentence_chunks_rewrite_lexed, strip_non_keyword_label_prefix,
        strip_trailing_trigger_cap_suffix_tokens, tokens_after_non_keyword_label_prefix,
    };

    fn single_preprocessed_line(text: &str) -> super::PreprocessedLine {
        let document = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Document Parser Test")
                .card_types(vec![CardType::Creature]),
            text,
        )
        .expect("expected preprocess_document to keep test line");
        match document
            .items
            .into_iter()
            .next()
            .expect("expected one preprocessed item")
        {
            PreprocessedItem::Line(line) => line,
            other => panic!("expected preprocessed line, got {other:?}"),
        }
    }

    #[test]
    fn strip_non_keyword_label_prefix_removes_chained_mode_name_and_cost() {
        assert_eq!(
            strip_non_keyword_label_prefix(
                "Meteor Strikes — {2} — Double target creature's power and toughness until end of turn."
            ),
            "Double target creature's power and toughness until end of turn."
        );
        assert_eq!(
            strip_non_keyword_label_prefix(
                "Final Heaven — {6}{G} — Triple target creature's power and toughness until end of turn."
            ),
            "Triple target creature's power and toughness until end of turn."
        );
    }

    #[test]
    fn split_label_prefix_lexed_reuses_existing_body_tokens() {
        let tokens = lex_line(
            "Secret Council — Each player votes for death or torture.",
            0,
        )
        .expect("rewrite lexer should classify labeled line");

        let (label, body_tokens) =
            split_label_prefix_lexed(&tokens).expect("expected token label prefix split");

        assert_eq!(label, "Secret Council");
        assert_eq!(
            render_token_slice(body_tokens),
            "Each player votes for death or torture."
        );
    }

    #[test]
    fn tokens_after_non_keyword_label_prefix_reuses_chained_body_tokens() {
        let line = single_preprocessed_line(
            "Meteor Strikes — {2} — Double target creature's power and toughness until end of turn.",
        );

        let tokens = tokens_after_non_keyword_label_prefix(&line)
            .expect("expected chained non-keyword label prefix to strip");

        assert_eq!(
            render_token_slice(&tokens),
            "double target creature's power and toughness until end of turn."
        );
    }

    #[test]
    fn rewrite_keyword_dash_parse_tokens_drops_council_label_body_only() {
        let tokens = lex_line(
            "secret council — each player votes for death or torture.",
            0,
        )
        .expect("rewrite lexer should classify council label line");

        let rewritten = rewrite_keyword_dash_parse_tokens(&tokens);

        assert_eq!(
            render_token_slice(&rewritten),
            "each player votes for death or torture."
        );
    }

    #[test]
    fn rewrite_keyword_dash_parse_tokens_keeps_keyword_label_without_dash() {
        let tokens = lex_line("cycling — {2}, discard this card: draw a card.", 0)
            .expect("rewrite lexer should classify keyword label line");

        let rewritten = rewrite_keyword_dash_parse_tokens(&tokens);

        assert_eq!(
            render_token_slice(&rewritten),
            "cycling {2}, discard this card: draw a card."
        );
    }

    #[test]
    fn strip_trailing_trigger_cap_suffix_tokens_supports_do_this_only_once_each_turn() {
        let tokens = lex_line(
            "Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.",
            0,
        )
        .expect("rewrite lexer should classify capped trigger line");

        let (stripped, max_triggers_per_turn) = strip_trailing_trigger_cap_suffix_tokens(&tokens);

        assert_eq!(max_triggers_per_turn, Some(1));
        assert_eq!(
            render_token_slice(stripped),
            "Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle"
        );
    }

    #[test]
    fn keyword_line_cst_stores_rewritten_parse_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Cycling — {2}, Discard this card: Draw a card.");

        let parsed =
            parse_keyword_line_cst(&line)?.expect("expected cycling line to parse as keyword");

        assert_eq!(
            render_token_slice(&parsed.parse_tokens),
            "cycling {2}, discard this card: draw a card."
        );

        Ok(())
    }

    #[test]
    fn ward_and_echo_static_prefixes_are_token_classified() {
        let ward = lex_line("Ward — Pay 3 life.", 0)
            .expect("rewrite lexer should classify ward static prefix");
        let echo =
            lex_line("Echo {2}{R}", 0).expect("rewrite lexer should classify echo static prefix");

        assert!(is_ward_or_echo_static_prefix_line_lexed(&ward));
        assert!(is_ward_or_echo_static_prefix_line_lexed(&echo));
    }

    #[test]
    fn land_reveal_combined_static_pair_is_token_classified() {
        let first = lex_line(
            "As this land enters, you may reveal an Island card from your hand.",
            0,
        )
        .expect("rewrite lexer should classify first static line");
        let second = lex_line("If you don't, it enters tapped.", 0)
            .expect("rewrite lexer should classify followup static line");

        assert!(is_land_reveal_enters_static_line_lexed(&first));
        assert!(is_land_reveal_enters_tapped_followup_line_lexed(&second));
    }

    #[test]
    fn opening_hand_begin_game_combined_static_pair_is_token_classified() {
        let first = lex_line(
            "If this card is in your opening hand, you may begin the game with it on the battlefield.",
            0,
        )
        .expect("rewrite lexer should classify opening-hand static line");
        let second = lex_line("If you do exile a card from your hand.", 0)
            .expect("rewrite lexer should classify if-you-do followup line");

        assert!(is_opening_hand_begin_game_static_line_lexed(&first));
        assert!(is_if_you_do_exile_followup_tokens(&second));
    }

    #[test]
    fn parse_document_cst_merges_numeric_result_followups_into_triggered_line()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Aberrant Mind Sorcerer")
                .card_types(vec![CardType::Creature]),
            "Psionic Spells — When this creature enters, choose target instant or sorcery card in your graveyard, then roll a d20.\n1—9 | You may put that card on top of your library.\n10—20 | Return that card to your hand.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [super::RewriteLineCst::Triggered(triggered)] => {
                assert!(
                    triggered.effect_text.contains("roll a d20"),
                    "expected initial roll clause in triggered effect text, got {:?}",
                    triggered.effect_text
                );
                assert!(
                    triggered.effect_text.contains("1—9")
                        && triggered.effect_text.contains("10—20"),
                    "expected numeric result followups to merge into triggered line, got {:?}",
                    triggered.effect_text
                );
            }
            other => panic!("expected one merged triggered line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn keyword_line_cst_recognizes_gift_family_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)",
        );

        let parsed =
            parse_keyword_line_cst(&line)?.expect("expected gift line to parse as keyword");

        assert!(matches!(parsed.kind, super::KeywordLineKindCst::Gift));

        Ok(())
    }

    #[test]
    fn keyword_line_cst_recognizes_exert_attack_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "You may exert this creature as it attacks. (An exerted creature won't untap during your next untap step.)",
        );

        let parsed =
            parse_keyword_line_cst(&line)?.expect("expected exert line to parse as keyword");

        assert!(matches!(
            parsed.kind,
            super::KeywordLineKindCst::ExertAttack
        ));

        Ok(())
    }

    #[test]
    fn static_line_cst_recognizes_compound_unblockable_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Enchanted creature gets +2/+2 and can't be blocked.");

        assert!(parse_static_line_cst(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn statement_line_cst_recognizes_exile_then_play_costs_more_from_tokens()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast.",
        );

        assert!(parse_statement_line_cst(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn unsupported_line_reason_recognizes_modal_header_from_tokens() {
        let line = single_preprocessed_line("Choose one —");

        assert_eq!(
            classify_unsupported_line_reason(&line),
            "modal-header-not-yet-supported"
        );
    }

    #[test]
    fn art_rating_statement_routes_to_unsupported_reason_from_tokens() -> Result<(), CardTextError>
    {
        let line = single_preprocessed_line(
            "Ask a person outside the game to rate its new art on a scale from 1 to 5.",
        );

        assert!(parse_statement_line_cst(&line)?.is_none());
        assert_eq!(
            classify_unsupported_line_reason(&line),
            "outside-the-game-rating-not-supported"
        );

        Ok(())
    }

    #[test]
    fn level_item_cst_stores_parsed_payload() -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Flying");

        let parsed = parse_level_item_cst(&line)?.expect("expected flying to parse as level item");

        assert_eq!(parsed.text, "flying");
        match &parsed.parsed {
            crate::cards::builders::ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                assert!(!actions.is_empty());
            }
            other => panic!("expected keyword-actions payload, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn saga_chapter_cst_stores_effects_ast() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Saga Parse Tokens Test")
                .card_types(vec![CardType::Enchantment]),
            "I, II — Draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [super::RewriteLineCst::SagaChapter(saga)] => {
                assert_eq!(saga.text, "draw a card.");
                assert!(!saga.effects_ast.is_empty());
            }
            other => panic!("expected one saga chapter line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn level_header_lowering_keeps_parsed_level_items() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Level Lowering Parse Tokens Test")
                .card_types(vec![CardType::Creature]),
            "Level up {1}\nLEVEL 1-2\nFlying\n3/3",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;
        let semantic = super::lower_document_cst(preprocessed, cst, false)?;

        let level = semantic
            .items
            .iter()
            .find_map(|item| match item {
                super::RewriteSemanticItem::LevelHeader(level) => Some(level),
                _ => None,
            })
            .expect("expected lowered semantic document to contain a level header");

        match level.items.as_slice() {
            [item] => match &item.parsed {
                crate::cards::builders::ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                    assert!(!actions.is_empty());
                }
                other => panic!("expected keyword-actions level item, got {other:?}"),
            },
            other => panic!("expected one lowered level item, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn saga_chapter_lowering_keeps_effects_ast() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Saga Lowering Parse Tokens Test")
                .card_types(vec![CardType::Enchantment]),
            "I, II — Draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;
        let semantic = super::lower_document_cst(preprocessed, cst, false)?;

        let saga = semantic
            .items
            .iter()
            .find_map(|item| match item {
                super::RewriteSemanticItem::SagaChapter(saga) => Some(saga),
                _ => None,
            })
            .expect("expected lowered semantic document to contain a saga chapter");

        assert_eq!(saga.text, "draw a card.");
        assert!(!saga.effects_ast.is_empty());

        Ok(())
    }

    #[test]
    fn statement_parse_groups_lexed_strip_labels_and_rewrite_followups() {
        let line = single_preprocessed_line(
            "Meteor Strikes — Exile target artifact. When you do, draw a card.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec!["exile target artifact. if you do, draw a card.".to_string()]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_split_instead_followup_into_separate_chunks() {
        let line = single_preprocessed_line(
            "Exile target creature. Return that card to the battlefield under its owner's control instead, then scry 1.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "exile target creature.".to_string(),
                "return that card to the battlefield under its owner's control instead, then scry 1."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_rewrites_copy_exception_without_relex() {
        let line = single_preprocessed_line(
            "Target artifact becomes a copy of target enchantment, except it's an artifact and it loses all other card types.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "target artifact becomes a copy of target enchantment, except it's an artifact."
                    .to_string()
            ]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_keep_broken_visage_followups_in_one_effect_group() {
        let line = single_preprocessed_line(
            "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "destroy target nonartifact attacking creature. it can't be regenerated. create a black spirit creature token. its power is equal to that creature's power and its toughness is equal to that creature's toughness. sacrifice the token at the beginning of the next end step.".to_string()
            ]
        );
    }

    #[test]
    fn parse_statement_line_cst_does_not_abort_on_broken_visage_static_probe_error()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        );

        assert!(parse_statement_line_cst(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn looks_like_statement_line_recognizes_vote_leads() {
        for text in [
            "Starting with you, each player votes for death or torture. If death gets more votes, each opponent sacrifices a creature of their choice. If torture gets more votes or the vote is tied, each opponent loses 4 life.",
            "Secret council — Each player secretly votes for truth or consequences, then those votes are revealed. For each truth vote, draw a card. Then choose an opponent at random. For each consequences vote, Truth or Consequences deals 3 damage to that player.",
        ] {
            let helper_text = split_label_prefix(text)
                .map(|(_, body)| body.trim())
                .unwrap_or(text);
            let tokens =
                lex_line(helper_text, 0).expect("rewrite lexer should classify vote line body");
            assert_eq!(
                classify_statement_line_family_lexed(&tokens),
                Some(StatementLineFamily::Vote)
            );
            assert!(
                looks_like_statement_line(text.to_ascii_lowercase().as_str()),
                "expected vote line to classify as a statement: {text}"
            );
        }
    }

    #[test]
    fn looks_like_statement_line_recognizes_next_turn_cast_lock() {
        let text =
            "Each opponent can't cast instant or sorcery spells during that player's next turn.";
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify next-turn cast-lock line");
        assert_eq!(
            classify_statement_line_family_lexed(&tokens),
            Some(StatementLineFamily::NextTurnCantCast)
        );
        assert!(looks_like_statement_line(
            text.to_ascii_lowercase().as_str()
        ));
    }

    #[test]
    fn looks_like_statement_line_recognizes_generic_heads() {
        for text in [
            "Draw a card.",
            "Each player discards a card.",
            "That target player sacrifices a creature.",
            "This spell deals 3 damage to any target.",
            "Target creature gets +2/+2 until end of turn.",
        ] {
            let tokens =
                lex_line(text, 0).expect("rewrite lexer should classify generic statement head");
            assert_eq!(
                classify_statement_line_family_lexed(&tokens),
                Some(StatementLineFamily::Generic)
            );
            assert!(looks_like_statement_line(
                text.to_ascii_lowercase().as_str()
            ));
        }
    }

    #[test]
    fn looks_like_lexed_line_family_helpers_handle_nonkeyword_labels() {
        let statement = single_preprocessed_line("Battle Plan — Each player discards a card.");
        let static_line = single_preprocessed_line("Mystic Aura — Enchanted creature gets +1/+1.");

        assert!(looks_like_statement_line_lexed(&statement));
        assert!(looks_like_static_line_lexed(&static_line));
    }

    #[test]
    fn looks_like_static_line_recognizes_generic_heads() {
        for text in [
            "This creature has flying.",
            "Enchanted creature gets +1/+1.",
            "As long as you control an artifact, this creature has hexproof.",
            "Your maximum hand size is reduced by four.",
        ] {
            let tokens =
                lex_line(text, 0).expect("rewrite lexer should classify generic static head");
            assert_eq!(
                classify_static_line_family_lexed(&tokens),
                Some(StaticLineFamily::Generic)
            );
            assert!(looks_like_static_line(text.to_ascii_lowercase().as_str()));
        }
    }

    #[test]
    fn triggered_split_probe_preserves_failed_effect_parse_details() {
        let line = single_preprocessed_line(
            "Whenever this creature attacks, search your library for artifact card named.",
        );
        let comma_idx = line
            .tokens
            .iter()
            .enumerate()
            .find_map(|(idx, token)| token.is_comma().then_some(idx))
            .expect("expected triggered probe line to contain a comma");
        let probe = probe_triggered_split(
            &line.tokens[1..comma_idx],
            &line.tokens[comma_idx + 1..],
            None,
            None,
        );
        let fallback = probe.fallback_cst(&line, &line.tokens);

        match probe {
            TriggeredSplitProbe::Unsupported {
                trigger_error,
                effect_error,
                ..
            } => {
                assert!(trigger_error.is_none());
                assert!(effect_error.is_some());
                assert!(fallback.is_some());
            }
            other => panic!("expected unsupported triggered split probe, got {other:?}"),
        }
    }

    #[test]
    fn triggered_conditional_split_preserves_full_text_for_lowering() {
        let line = single_preprocessed_line(
            "At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.",
        );

        let parsed =
            parse_triggered_line_cst(&line).expect("expected triggered conditional line to parse");

        assert_eq!(
            parsed.full_text,
            "at the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. put that card into your hand and the rest on the bottom of your library in a random order."
        );
        assert_eq!(
            parsed.trigger_text,
            "the beginning of your second main phase"
        );
        assert_eq!(
            parsed.effect_text,
            "reveal cards from the top of your library until you reveal a land card. put that card into your hand and the rest on the bottom of your library in a random order."
        );
        assert_eq!(
            render_token_slice(&parsed.full_parse_tokens),
            parsed.full_text
        );
        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            parsed.trigger_text
        );
        assert_eq!(
            render_token_slice(&parsed.effect_parse_tokens),
            parsed.effect_text
        );
    }

    #[test]
    fn colon_nonactivation_statement_fallback_reuses_split_token_slice() {
        let line = single_preprocessed_line("Reveal this card from your hand: Draw a card.");

        let parsed = parse_colon_nonactivation_statement_fallback(&line)
            .expect("expected fallback parse to succeed")
            .expect("expected reveal-prefix fallback to produce a statement");

        assert_eq!(parsed.text, "reveal this card from your hand");
        assert_eq!(
            parsed.info.normalized.normalized,
            "reveal this card from your hand"
        );
    }

    #[test]
    fn trigger_sentence_chunk_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "Whenever this creature attacks, draw a card. Whenever it deals combat damage to a player, create a Treasure token.",
            0,
        )
        .expect("rewrite lexer should classify trigger chunk line");

        let chunks = split_trigger_sentence_chunks_rewrite_lexed(&tokens)
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Whenever this creature attacks, draw a card".to_string(),
                "Whenever it deals combat damage to a player, create a Treasure token".to_string(),
            ]
        );
    }

    #[test]
    fn reveal_first_draw_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "Reveal the first card you draw each turn. Whenever you reveal an instant card this way, draw a card.",
            0,
        )
        .expect("rewrite lexer should classify reveal-first-draw line");

        let chunks = split_reveal_first_draw_line_rewrite_lexed(&tokens)
            .expect("expected reveal-first-draw splitter to match")
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Reveal the first card you draw each turn".to_string(),
                "Whenever you reveal an instant card this way, draw a card".to_string(),
            ]
        );
    }

    #[test]
    fn trailing_keyword_activation_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "draw a card. cycling — {2}, discard this card: draw a card.",
            0,
        )
        .expect("rewrite lexer should classify trailing keyword activation line");

        let (prefix, suffix) = normalize_trailing_keyword_activation_sentence_lexed(&tokens)
            .expect("expected trailing keyword activation split");

        assert_eq!(render_token_slice(&prefix), "draw a card");
        assert_eq!(
            render_token_slice(&suffix),
            "cycling — {2}, discard this card: draw a card"
        );
    }

    #[test]
    fn activation_text_parts_lexed_reuse_existing_token_split() {
        let tokens = lex_line("{2}, discard this card: draw a card.", 0)
            .expect("rewrite lexer should classify activation text");

        let (cost_tokens, effect_text) =
            split_activation_text_parts_lexed(&tokens).expect("expected activation text split");

        assert_eq!(render_token_slice(&cost_tokens), "{2}, discard this card");
        assert_eq!(effect_text, "draw a card.");
    }

    #[test]
    fn activated_line_cst_stores_cost_and_effect_parse_tokens() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Activated Parse Tokens Test")
                .card_types(vec![CardType::Artifact]),
            "{T}: Draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [super::RewriteLineCst::Activated(activated)] => {
                assert_eq!(render_token_slice(&activated.cost_parse_tokens), "{t}");
                assert_eq!(activated.effect_text, "draw a card.");
                assert_eq!(
                    render_token_slice(&activated.effect_parse_tokens),
                    activated.effect_text
                );
            }
            other => panic!("expected one activated line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn reveal_first_draw_line_family_parses_through_document_cst() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Reveal First Draw Split Test")
                .card_types(vec![CardType::Enchantment]),
            "Reveal the first card you draw each turn. Whenever you reveal an instant card this way, draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [
                super::RewriteLineCst::Static(static_line),
                super::RewriteLineCst::Triggered(triggered),
            ] => {
                assert_eq!(static_line.text, "reveal the first card you draw each turn");
                assert_eq!(
                    triggered.trigger_text,
                    "you reveal an instant card this way"
                );
                assert_eq!(triggered.effect_text, "draw a card");
            }
            other => {
                panic!("expected static plus triggered reveal-first-draw split, got {other:?}")
            }
        }

        Ok(())
    }

    #[test]
    fn modal_mode_cst_stores_parsed_effects_ast() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Modal Parse Tokens Test")
                .card_types(vec![CardType::Instant]),
            "Choose one —\n• Meteor Strikes — Draw a card.\n• Final Heaven — Gain 3 life.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [super::RewriteLineCst::Modal(modal)] => {
                assert_eq!(modal.modes.len(), 2);
                assert_eq!(modal.modes[0].text, "Draw a card.");
                assert_eq!(modal.modes[1].text, "Gain 3 life.");
                assert!(!modal.modes[0].effects_ast.is_empty());
                assert!(!modal.modes[1].effects_ast.is_empty());
            }
            other => panic!("expected one modal block, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn unsupported_line_diagnostics_use_token_normalization() {
        let landwalk = single_preprocessed_line(
            "Creatures with islandwalk can be blocked as though they didn’t have islandwalk.",
        );
        let aura_copy = single_preprocessed_line(
            "Create a token that’s a copy of that Aura attached to that creature.",
        );
        let face_down = single_preprocessed_line("Target face-down creature can block this turn.");

        let landwalk_error = diagnose_known_unsupported_rewrite_line(&landwalk.tokens)
            .expect("expected landwalk override diagnostic");
        let aura_copy_error = diagnose_known_unsupported_rewrite_line(&aura_copy.tokens)
            .expect("expected aura-copy diagnostic");
        let face_down_error = diagnose_known_unsupported_rewrite_line(&face_down.tokens)
            .expect("expected face-down diagnostic");

        assert_eq!(
            landwalk_error.to_string(),
            "unsupported landwalk override clause"
        );
        assert_eq!(
            aura_copy_error.to_string(),
            "unsupported aura-copy attachment fanout clause"
        );
        assert_eq!(face_down_error.to_string(), "unsupported face-down clause");
    }

    #[test]
    fn looks_like_divvy_statement_probe_recognizes_pile_lines() {
        let text = "Separate all creatures target player controls into two piles. Destroy all creatures in the pile of your choice.";
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify divvy pile line");
        assert_eq!(
            classify_statement_line_family_lexed(&tokens),
            Some(StatementLineFamily::Divvy)
        );
    }

    #[test]
    fn untap_shape_probes_recognize_expected_token_patterns() {
        let your_step = lex_line("Lands you control don't untap during your untap step.", 0)
            .expect("rewrite lexer should classify your-untap-step probe");
        assert!(is_doesnt_untap_during_your_untap_step_line_lexed(&your_step));

        let your_step_do_not = lex_line(
            "Artifacts you control do not untap during your untap step.",
            0,
        )
        .expect("rewrite lexer should classify do-not untap-step probe");
        assert!(is_doesnt_untap_during_your_untap_step_line_lexed(
            &your_step_do_not
        ));

        let other_players_text =
            "Untap all permanents you control during each other player's untap step.";
        let other_players = lex_line(other_players_text, 0)
            .expect("rewrite lexer should classify other-players untap-step probe");
        assert_eq!(
            classify_static_line_family_lexed(&other_players),
            Some(StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
        );
        assert!(!looks_like_statement_line(
            other_players_text.to_ascii_lowercase().as_str()
        ));
        assert!(looks_like_static_line(
            other_players_text.to_ascii_lowercase().as_str()
        ));

        let singular_other_players_text =
            "Untap this artifact during each other player's untap step.";
        let singular_other_players = lex_line(singular_other_players_text, 0)
            .expect("rewrite lexer should classify singular other-players untap-step probe");
        assert_eq!(
            classify_static_line_family_lexed(&singular_other_players),
            Some(StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
        );
        assert!(!looks_like_statement_line(
            singular_other_players_text.to_ascii_lowercase().as_str()
        ));
        assert!(looks_like_static_line(
            singular_other_players_text.to_ascii_lowercase().as_str()
        ));
    }

    #[test]
    fn pact_shape_probe_recognizes_next_upkeep_lose_game_line() {
        let tokens = lex_line(
            "At the beginning of your next upkeep, pay {2}{U}{U}. If you don't, you lose the game.",
            0,
        )
        .expect("rewrite lexer should classify pact next-upkeep statement line");
        assert_eq!(
            classify_statement_line_family_lexed(&tokens),
            Some(StatementLineFamily::PactNextUpkeep)
        );
    }
}

fn is_named_ability_label(label: &str) -> bool {
    matches!(
        label.to_ascii_lowercase().as_str(),
        "alliance"
            | "astral projection"
            | "bigby's hand"
            | "body-print"
            | "boast"
            | "cohort"
            | "devouring monster"
            | "diana"
            | "exhaust"
            | "gooooaaaalll!"
            | "hero's sundering"
            | "hunt for heresy"
            | "machina"
            | "mage hand"
            | "megamorph"
            | "morph"
            | "psychic blades"
            | "raid"
            | "renew"
            | "rope dart"
            | "scorching ray"
            | "share"
            | "shieldwall"
            | "sleight of hand"
            | "smear campaign"
            | "stunning strike"
            | "teleport"
            | "trance"
            | "valiant"
            | "waterbend"
    )
}

fn rewrite_line_normalized(
    line: &PreprocessedLine,
    normalized: &str,
) -> Result<PreprocessedLine, CardTextError> {
    let mut rewritten = line.clone();
    rewritten.info.normalized.original = normalized.to_string();
    rewritten.info.normalized.normalized = normalized.to_string();
    rewritten.info.normalized.char_map = (0..normalized.len()).collect();
    rewritten.tokens = super::lexer::lex_line(normalized, line.info.line_index)?;
    Ok(rewritten)
}

fn rewrite_line_tokens(line: &PreprocessedLine, tokens: &[OwnedLexToken]) -> PreprocessedLine {
    let normalized = render_token_slice(tokens);
    let mut rewritten = line.clone();
    rewritten.info.normalized.original = normalized.clone();
    rewritten.info.normalized.normalized = normalized.clone();
    rewritten.info.normalized.char_map = (0..normalized.len()).collect();
    rewritten.tokens = tokens.to_vec();
    rewritten
}

fn try_parse_triggered_line_with_named_source_rewrite(
    builder: &CardDefinitionBuilder,
    line: &PreprocessedLine,
    text: &str,
) -> Result<Option<TriggeredLineCst>, CardTextError> {
    let Some(rewritten) = normalize_named_source_trigger_for_builder(builder, text) else {
        return Ok(None);
    };
    let rewritten_line = rewrite_line_normalized(line, rewritten.as_str())?;
    match parse_triggered_line_cst(&rewritten_line) {
        Ok(triggered) => Ok(Some(triggered)),
        Err(_) => Ok(None),
    }
}

fn is_fully_parenthetical_line(text: &str) -> bool {
    let trimmed = text.trim();
    str_starts_with_char(trimmed, '(') && str_ends_with_char(trimmed, ')')
}

fn split_trigger_sentence_chunks_rewrite_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    let sentence_tokens = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentence_tokens.len() <= 1 {
        return sentence_tokens
            .into_iter()
            .map(|sentence| sentence.to_vec())
            .collect();
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_starts_with_trigger = false;

    for sentence_tokens in sentence_tokens {
        let sentence_starts_with_trigger = line_starts_with_trigger_intro_tokens(sentence_tokens);
        if !current.is_empty() && current_starts_with_trigger && sentence_starts_with_trigger {
            if let Some(chunk) = clone_sentence_chunk_tokens(tokens, &current) {
                chunks.push(chunk);
            }
            current.clear();
            current_starts_with_trigger = false;
        }
        if current.is_empty() {
            current_starts_with_trigger = sentence_starts_with_trigger;
        }
        current.push(sentence_tokens);
    }

    if !current.is_empty() {
        if let Some(chunk) = clone_sentence_chunk_tokens(tokens, &current) {
            chunks.push(chunk);
        }
    }

    chunks
}

fn split_reveal_first_draw_line_rewrite_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<Vec<OwnedLexToken>>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return None;
    }

    let first_tokens = *sentences.first()?;
    let first_is_reveal_first_draw = token_words_match_any(
        first_tokens,
        &[
            &[
                "reveal", "the", "first", "card", "you", "draw", "each", "turn",
            ],
            &[
                "reveal", "the", "first", "card", "you", "draw", "on", "each", "of", "your",
                "turns",
            ],
            &[
                "you", "may", "reveal", "the", "first", "card", "you", "draw", "each", "turn",
                "as", "you", "draw", "it",
            ],
            &[
                "you", "may", "reveal", "the", "first", "card", "you", "draw", "on", "each", "of",
                "your", "turns", "as", "you", "draw", "it",
            ],
        ],
    );
    if !first_is_reveal_first_draw {
        return None;
    }

    let tail_tokens = clone_sentence_chunk_tokens(tokens, &sentences[1..])?;
    if !token_words_have_prefix(&tail_tokens, &["whenever", "you", "reveal"]) {
        return None;
    }

    Some(vec![first_tokens.to_vec(), tail_tokens])
}

fn classify_unsupported_line_reason(line: &PreprocessedLine) -> &'static str {
    let classification_tokens = tokens_after_non_keyword_label_prefix(line).unwrap_or(&line.tokens);

    if is_bullet_line(line.info.raw_line.as_str()) {
        return "bullet-line-without-modal-header";
    }
    if line_starts_with_trigger_intro_tokens(&line.tokens) {
        return "triggered-line-not-yet-supported";
    }
    if split_lexed_once_on_colon_outside_quotes(&line.tokens).is_some() {
        return "activated-line-not-yet-supported";
    }
    if token_words_have_prefix(classification_tokens, &["choose"]) {
        return "modal-header-not-yet-supported";
    }
    if matches!(
        super::grammar::structure::classify_statement_line_family_lexed(&line.tokens),
        Some(super::grammar::structure::StatementLineFamily::ArtRating)
    ) {
        return "outside-the-game-rating-not-supported";
    }
    if looks_like_statement_line_lexed(line) {
        return "statement-line-not-yet-supported";
    }
    if looks_like_static_line_lexed(line) {
        return "static-line-not-yet-supported";
    }
    "unclassified-line-family"
}

fn try_parse_labeled_line_dispatch(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some((label, body_tokens)) = split_label_prefix_lexed(&line.tokens) else {
        return Ok(None);
    };

    let is_named_label = is_named_ability_label(label.as_str());
    let preserve_as_choice_label = labeled_choice_block_has_peer(&preprocessed.items, idx);
    if preserve_keyword_prefix_for_parse(label.as_str()) {
        return Ok(None);
    }

    let body_line = rewrite_line_tokens(line, body_tokens);
    let labeled_activation = if (!str_starts_with_char(line.info.raw_line.trim_start(), '(')
        || is_fully_parenthetical_line(line.info.raw_line.as_str()))
        && let Some((cost_tokens, effect_parse_tokens)) =
            split_activation_text_tokens_lexed(&body_line.tokens)
    {
        let cost_text = render_token_slice(&cost_tokens);
        let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        Some((cost_tokens, effect_parse_tokens, cost_text, effect_text))
    } else {
        None
    };
    let prefer_activation = labeled_activation
        .as_ref()
        .is_some_and(|(_, _, cost_text, _)| looks_like_activation_cost_prefix(cost_text.as_str()));

    if line_starts_with_trigger_intro_tokens(&body_line.tokens) {
        if let Ok(mut triggered) = parse_triggered_line_cst(&body_line) {
            if preserve_as_choice_label {
                triggered.chosen_option_label = Some(label.to_ascii_lowercase());
            }
            let (triggered, next_idx) =
                extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
            return Ok(Some(LineDispatchResult::single(
                RewriteLineCst::Triggered(triggered),
                next_idx,
            )));
        }
        if let Some(mut triggered) = try_parse_triggered_line_with_named_source_rewrite(
            &preprocessed.builder,
            line,
            body_line.info.normalized.normalized.as_str(),
        )? {
            if preserve_as_choice_label {
                triggered.chosen_option_label = Some(label.to_ascii_lowercase());
            }
            let (triggered, next_idx) =
                extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
            return Ok(Some(LineDispatchResult::single(
                RewriteLineCst::Triggered(triggered),
                next_idx,
            )));
        }
        if allow_unsupported && is_named_label {
            return Ok(Some(LineDispatchResult::single(
                RewriteLineCst::Unsupported(UnsupportedLineCst {
                    info: line.info.clone(),
                    reason_code: "triggered-line-not-yet-supported",
                }),
                idx + 1,
            )));
        }
        if is_named_label {
            return Err(parse_triggered_line_cst(&body_line)
                .err()
                .unwrap_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported triggered line: '{}'",
                        body_line.info.normalized.normalized.as_str()
                    ))
                }));
        }
    }

    if prefer_activation
        && let Some((cost_tokens, effect_parse_tokens, cost_text, effect_text)) =
            labeled_activation.clone()
    {
        match parse_activation_cost_tokens_rewrite(&cost_tokens) {
            Ok(cost) => {
                return Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Activated(ActivatedLineCst {
                        info: line.info.clone(),
                        cost,
                        cost_parse_tokens: cost_tokens,
                        effect_text,
                        effect_parse_tokens,
                        chosen_option_label: preserve_as_choice_label
                            .then(|| label.to_ascii_lowercase()),
                    }),
                    idx + 1,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(cost_text.as_str()) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    if is_named_label && let Some(keyword_line) = parse_keyword_line_cst(&body_line)? {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Keyword(keyword_line),
            idx + 1,
        )));
    }

    if let Some(mut static_line) = parse_static_line_cst(&body_line)? {
        if preserve_as_choice_label {
            static_line.chosen_option_label = Some(label.to_ascii_lowercase());
        }
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Static(static_line),
            idx + 1,
        )));
    }

    if let Some(rewritten_body) = normalize_named_source_sentence_for_builder(
        &preprocessed.builder,
        body_line.info.normalized.normalized.as_str(),
    ) {
        let rewritten_body_line = rewrite_line_normalized(line, rewritten_body.as_str())?;
        if let Some(mut static_line) = parse_static_line_cst(&rewritten_body_line)? {
            if preserve_as_choice_label {
                static_line.chosen_option_label = Some(label.to_ascii_lowercase());
            }
            return Ok(Some(LineDispatchResult::single(
                RewriteLineCst::Static(static_line),
                idx + 1,
            )));
        }
    }

    if let Some((cost_tokens, effect_parse_tokens, cost_text, effect_text)) = labeled_activation {
        match parse_activation_cost_tokens_rewrite(&cost_tokens) {
            Ok(cost) => {
                return Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Activated(ActivatedLineCst {
                        info: line.info.clone(),
                        cost,
                        cost_parse_tokens: cost_tokens,
                        effect_text,
                        effect_parse_tokens,
                        chosen_option_label: preserve_as_choice_label
                            .then(|| label.to_ascii_lowercase()),
                    }),
                    idx + 1,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(cost_text.as_str()) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    if let Some(statement_line) = parse_statement_line_cst(&body_line)? {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Statement(statement_line),
            idx + 1,
        )));
    }

    Ok(None)
}

fn try_parse_triggered_line_dispatch(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !line_starts_with_trigger_intro_tokens(&line.tokens) {
        return Ok(None);
    }

    let trigger_chunks = split_trigger_sentence_chunks_rewrite_lexed(&line.tokens);
    if trigger_chunks.len() > 1 {
        let mut lines = Vec::with_capacity(trigger_chunks.len());
        for chunk_tokens in trigger_chunks {
            let chunk_line = rewrite_line_tokens(line, &chunk_tokens);
            match parse_triggered_line_cst(&chunk_line) {
                Ok(triggered) => lines.push(RewriteLineCst::Triggered(triggered)),
                Err(_) => {
                    if let Some(triggered) = try_parse_triggered_line_with_named_source_rewrite(
                        &preprocessed.builder,
                        line,
                        chunk_line.info.normalized.normalized.as_str(),
                    )? {
                        lines.push(RewriteLineCst::Triggered(triggered));
                        continue;
                    }
                    if allow_unsupported {
                        lines.push(RewriteLineCst::Unsupported(UnsupportedLineCst {
                            info: line.info.clone(),
                            reason_code: "triggered-line-not-yet-supported",
                        }));
                    } else {
                        return Err(strict_unsupported_triggered_line_error(
                            chunk_line.info.normalized.normalized.as_str(),
                            parse_triggered_line_cst(&chunk_line).err(),
                        ));
                    }
                }
            }
        }
        return Ok(Some(LineDispatchResult {
            lines,
            next_idx: idx + 1,
        }));
    }

    match parse_triggered_line_cst(line) {
        Ok(triggered) => {
            let (triggered, next_idx) =
                extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
            Ok(Some(LineDispatchResult::single(
                RewriteLineCst::Triggered(triggered),
                next_idx,
            )))
        }
        Err(_) => {
            if let Some(triggered) = try_parse_triggered_line_with_named_source_rewrite(
                &preprocessed.builder,
                line,
                &line.info.normalized.normalized,
            )? {
                let (triggered, next_idx) = extend_triggered_line_with_result_followups(
                    &preprocessed.items,
                    idx,
                    triggered,
                );
                Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Triggered(triggered),
                    next_idx,
                )))
            } else if allow_unsupported {
                Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Unsupported(UnsupportedLineCst {
                        info: line.info.clone(),
                        reason_code: "triggered-line-not-yet-supported",
                    }),
                    idx + 1,
                )))
            } else {
                Err(strict_unsupported_triggered_line_error(
                    &line.info.raw_line,
                    parse_triggered_line_cst(line).err(),
                ))
            }
        }
    }
}

#[cfg(test)]
fn split_activation_text_parts_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String)> {
    let (cost_tokens, effect_tokens) = split_activation_text_tokens_lexed(tokens)?;
    Some((
        cost_tokens,
        render_token_slice(&effect_tokens).trim().to_string(),
    ))
}

fn split_activation_text_tokens_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let Some((cost_tokens, effect_tokens)) = split_lexed_once_on_colon_outside_quotes(tokens)
    else {
        return None;
    };

    let cost_tokens = trim_lexed_commas(cost_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return None;
    }

    Some((cost_tokens.to_vec(), effect_tokens.to_vec()))
}

pub(crate) fn parse_text_to_semantic_document(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:start card={:?} allow_unsupported={} lines={}",
            builder.card_builder.name_ref(),
            allow_unsupported,
            text.lines().count()
        );
    }
    if !allow_unsupported && let Some(err) = preflight_known_strict_unsupported(text.as_str()) {
        return Err(err);
    }
    let preprocessed = preprocess_document(builder, text.as_str())?;
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:preprocessed items={}",
            preprocessed.items.len()
        );
    }
    let cst = parse_document_cst(&preprocessed, allow_unsupported)?;
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:cst lines={}",
            cst.lines.len()
        );
    }
    let semantic = lower_document_cst(preprocessed, cst, allow_unsupported)?;
    let annotations = semantic.annotations.clone();
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:done items={}",
            semantic.items.len()
        );
    }
    Ok((semantic, annotations))
}

pub(crate) fn parse_document_cst(
    preprocessed: &PreprocessedDocument,
    allow_unsupported: bool,
) -> Result<RewriteDocumentCst, CardTextError> {
    let mut lines = Vec::with_capacity(preprocessed.items.len());
    let mut idx = 0usize;
    while idx < preprocessed.items.len() {
        let item = &preprocessed.items[idx];
        match item {
            PreprocessedItem::Metadata(meta) => {
                lines.push(RewriteLineCst::Metadata(metadata_line_cst(
                    meta.info.clone(),
                    meta.value.clone(),
                )?));
                idx += 1;
            }
            PreprocessedItem::Line(line) => {
                parser_trace("parse_document_cst:line", &line.tokens);
                if let Some((level_block, next_idx)) =
                    try_parse_level_header_block(preprocessed, idx, line, allow_unsupported)?
                {
                    lines.push(level_block);
                    idx = next_idx;
                    continue;
                }

                if let Some((chapters, text)) =
                    parse_saga_chapter_prefix(&line.info.normalized.normalized)
                {
                    lines.push(RewriteLineCst::SagaChapter(parse_saga_chapter_line_cst(
                        line, chapters, text,
                    )?));
                    idx += 1;
                    continue;
                }

                if let Some((modal_block, next_idx)) =
                    try_parse_modal_bullet_block(preprocessed, idx, line)?
                {
                    lines.push(modal_block);
                    idx = next_idx;
                    continue;
                }

                let normalized = line.info.normalized.normalized.as_str();
                if let Some(chunks) = split_reveal_first_draw_line_rewrite_lexed(&line.tokens) {
                    for chunk_tokens in chunks {
                        let chunk_line = rewrite_line_tokens(line, &chunk_tokens);
                        if line_starts_with_trigger_intro_tokens(&chunk_line.tokens) {
                            for trigger_chunk in
                                split_trigger_sentence_chunks_rewrite_lexed(&chunk_line.tokens)
                            {
                                let trigger_line = rewrite_line_tokens(&chunk_line, &trigger_chunk);
                                lines.push(RewriteLineCst::Triggered(parse_triggered_line_cst(
                                    &trigger_line,
                                )?));
                            }
                        } else if let Some(static_line) = parse_static_line_cst(&chunk_line)? {
                            lines.push(RewriteLineCst::Static(static_line));
                        } else {
                            return Err(CardTextError::ParseError(format!(
                                "parser could not split reveal-first-draw line family: '{}'",
                                line.info.raw_line
                            )));
                        }
                    }
                    idx += 1;
                    continue;
                }
                if let Some(unsupported) = parse_former_section9_unsupported_line_cst(line) {
                    lines.push(RewriteLineCst::Unsupported(unsupported));
                    idx += 1;
                    continue;
                }
                if let Some((prefix_tokens, suffix_tokens)) =
                    normalize_trailing_keyword_activation_sentence_lexed(&line.tokens)
                {
                    let prefix_line = rewrite_line_tokens(line, &prefix_tokens);
                    if let Some(statement_line) = parse_statement_line_cst(&prefix_line)? {
                        lines.push(RewriteLineCst::Statement(statement_line));
                    } else if let Some(rewritten_prefix) =
                        normalize_named_source_sentence_for_builder(
                            &preprocessed.builder,
                            prefix_line.info.normalized.normalized.as_str(),
                        )
                    {
                        let rewritten_prefix_line =
                            rewrite_line_normalized(line, rewritten_prefix.as_str())?;
                        if let Some(statement_line) =
                            parse_statement_line_cst(&rewritten_prefix_line)?
                        {
                            lines.push(RewriteLineCst::Statement(statement_line));
                        } else if let Some(static_line) =
                            parse_static_line_cst(&rewritten_prefix_line)?
                        {
                            lines.push(RewriteLineCst::Static(static_line));
                        } else {
                            return Err(CardTextError::ParseError(format!(
                                "parser could not split leading sentence before keyword ability: '{}'",
                                line.info.raw_line
                            )));
                        }
                    } else if let Some(static_line) = parse_static_line_cst(&prefix_line)? {
                        lines.push(RewriteLineCst::Static(static_line));
                    } else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not split leading sentence before keyword ability: '{}'",
                            line.info.raw_line
                        )));
                    }

                    let suffix_line = rewrite_line_tokens(line, &suffix_tokens);
                    let Some((_label, body_tokens)) = split_label_prefix_lexed(&suffix_line.tokens)
                    else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not recover keyword activation suffix: '{}'",
                            line.info.raw_line
                        )));
                    };
                    let Some((cost_tokens, effect_parse_tokens)) =
                        split_activation_text_tokens_lexed(body_tokens)
                    else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not recover activation suffix: '{}'",
                            line.info.raw_line
                        )));
                    };
                    let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
                    let cost = parse_activation_cost_tokens_rewrite(&cost_tokens)?;
                    lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                        info: suffix_line.info.clone(),
                        cost,
                        cost_parse_tokens: cost_tokens,
                        effect_text,
                        effect_parse_tokens,
                        chosen_option_label: None,
                    }));
                    idx += 1;
                    continue;
                }
                if normalized
                    == "this effect can't reduce the mana in that cost to less than one mana."
                {
                    idx += 1;
                    continue;
                }
                if !allow_unsupported
                    && let Some(err) = diagnose_known_unsupported_rewrite_line(&line.tokens)
                {
                    return Err(err);
                }
                let dispatch =
                    dispatch_standard_line_cst(preprocessed, idx, line, allow_unsupported)?;
                lines.extend(dispatch.lines);
                idx = dispatch.next_idx;
                continue;
            }
        }
    }

    Ok(RewriteDocumentCst { lines })
}

fn lower_document_cst(
    preprocessed: PreprocessedDocument,
    cst: RewriteDocumentCst,
    allow_unsupported: bool,
) -> Result<RewriteSemanticDocument, CardTextError> {
    let mut builder = preprocessed.builder;
    let mut items = Vec::with_capacity(cst.lines.len());

    for line in cst.lines {
        match line {
            RewriteLineCst::Metadata(MetadataLineCst { value }) => {
                builder = builder.apply_metadata(value.clone())?;
                items.push(RewriteSemanticItem::Metadata);
            }
            other => items.push(lower_non_metadata_rewrite_line_cst(
                other,
                allow_unsupported,
            )?),
        }
    }

    Ok(RewriteSemanticDocument {
        builder,
        annotations: preprocessed.annotations,
        items,
        allow_unsupported,
    })
}

pub(crate) fn metadata_line_cst(
    info: crate::cards::builders::LineInfo,
    value: crate::cards::builders::MetadataLine,
) -> Result<MetadataLineCst, CardTextError> {
    let _ = info;
    Ok(MetadataLineCst { value })
}
