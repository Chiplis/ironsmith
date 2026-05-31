use winnow::combinator::{alt, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};
use crate::target::PlayerFilter;
use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;
use ironsmith_core::Value;

use super::super::activation_and_restrictions::{
    normalize_cant_words, parse_cant_restriction_clause, parse_cant_restrictions,
};
use super::super::grammar::structure::{IfClausePredicateSpec, split_if_clause_lexed};
use super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, parser_token_word_positions, parser_token_word_refs,
    split_lexed_sentences, token_slice_all_are_kind, token_slice_at_is, token_slice_first_is,
    token_slice_starts_with, token_word_refs, trim_lexed_commas,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::search_library_support::{
    apply_search_library_mana_constraint, extract_search_library_mana_constraint,
    is_same_name_that_reference_words, normalize_search_library_filter,
    parse_restriction_duration_lexed, parse_search_library_disjunction_filter,
    split_search_different_name_reference_filter, split_search_library_count_value_clause_lexed,
    split_search_same_name_reference_filter, word_slice_mentions_nth_from_top, zone_slice_contains,
};
use super::super::token_primitives::{
    find_index as find_token_index, rfind_index as rfind_token_index,
};
use super::super::util::{
    is_article, parse_choice_count_token_prefix_consumed, parse_number, parse_subject,
    parse_target_phrase, span_from_tokens, trim_commas,
};
use super::primitives;

#[path = "effects/search_library.rs"]
mod search_library;
pub(crate) use search_library::*;
#[path = "effects/unsupported_shapes.rs"]
mod unsupported_shapes;
pub(crate) use unsupported_shapes::*;

const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const DESCEND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["descend"]);
const CANT_NEGATION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["can't"], &["cant"], &["cannot"]]);
const DONT_NEGATION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["doesn't"], &["doesnt"], &["don't"], &["dont"]]);
const DOES_DO_CAN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["does"], &["do"], &["can"]]);
const DOES_OR_DO_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["does"], &["do"]]);
const NOT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not"]);
const IF_YOU_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if", "you"]);
const CONTROL_OR_OWN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"], &["own"], &["owns"]]);
const THEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then"]);
const WHO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["who"]);
const THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "turn"]);
const THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "way"]);
const COMPACT_NEGATED_ACTION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["doesnt"], &["didnt"], &["doesn't"], &["didn't"]]);
const SPLIT_NEGATED_ACTION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["do", "not"], &["did", "not"]]);
const THAT_WOULD_BE_DEALT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "would", "be", "dealt"]);
const WOULD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["would"]);
const PREVENT_DAMAGE_SOURCE_HEAD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target"], &["that"], &["this"], &["it"]]);
const DEAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["deal"]);
const PREVENT_DAMAGE_REFERENCE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["target", "this", "that", "it"]]);
const PREVENT_DAMAGE_PLAYERS_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["player"], &["players"]]);
const YOU_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const LOSE_MANA_STEPS_PHASES_END_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["lose", "mana", "steps", "phases", "end"]);
const THAT_MANY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "many"]);
const REMAINS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["remains"]);
const TAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["tapped"]);
const PREVENT_DAMAGE_EXPLICIT_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["that"], &["it"]]);
const SOURCE_REFERENCE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this"],
            &["source"],
            &["artifact"],
            &["creature"],
            &["permanent"]
        ]
);
const EFFECT_DISCARD_OR_DISCARDED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["discard"], &["discarded"]]);

pub(crate) fn cant_sentence_clause_tokens_for_restriction_scan_lexed(
    clause_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    split_lexed_sentences(clause_tokens)
        .into_iter()
        .next()
        .unwrap_or(clause_tokens)
        .to_vec()
}

pub(crate) fn cant_sentence_has_supported_negation_gate_lexed(
    clause_tokens: &[OwnedLexToken],
) -> bool {
    let Some((neg_start, _)) = find_cant_sentence_negation_span_lexed(clause_tokens) else {
        return false;
    };

    !clause_tokens[..neg_start]
        .iter()
        .any(|token| AND_WORD_PATTERN.matches_token(token))
}

pub(crate) fn find_cant_sentence_negation_span_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(usize, usize)> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if CANT_NEGATION_WORD_PATTERN.matches_token(token) {
            return Some((cursor, cursor + 1));
        }
        if DONT_NEGATION_WORD_PATTERN.matches_token(token) {
            if cursor >= 2
                && IF_YOU_PREFIX_PATTERN
                    .matches_words(&token_word_refs(&tokens[cursor - 2..cursor]))
            {
                cursor += 1;
                continue;
            }
            if tokens
                .get(cursor + 1)
                .is_some_and(|next| CONTROL_OR_OWN_WORD_PATTERN.matches_token(next))
            {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 1));
        }
        if DOES_DO_CAN_WORD_PATTERN.matches_token(token)
            && tokens
                .get(cursor + 1)
                .is_some_and(|next| NOT_WORD_PATTERN.matches_token(next))
        {
            if cursor >= 2
                && IF_YOU_PREFIX_PATTERN
                    .matches_words(&token_word_refs(&tokens[cursor - 2..cursor]))
            {
                cursor += 2;
                continue;
            }
            if DOES_OR_DO_WORD_PATTERN.matches_token(token)
                && tokens
                    .get(cursor + 2)
                    .is_some_and(|next| CONTROL_OR_OWN_WORD_PATTERN.matches_token(next))
            {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 2));
        }
        cursor += 1;
    }

    None
}

fn cant_sentence_next_turn_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["during", "that", "players", "next", "turn"]),
        primitives::phrase(&["during", "that", "player's", "next", "turn"]),
        primitives::phrase(&["during", "that", "player", "s", "next", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn cant_sentence_for_as_long_as_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

pub(crate) fn split_cant_sentence_next_turn_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let Some((_, rest)) =
            primitives::parse_prefix(&tokens[cursor..], cant_sentence_next_turn_suffix)
        else {
            cursor += 1;
            continue;
        };
        if token_slice_all_are_kind(rest, TokenKind::Period) {
            return Some(tokens[..cursor].to_vec());
        }
        cursor += 1;
    }

    None
}

#[derive(Debug, Clone)]
pub(crate) struct CantSentencePreparedClause {
    pub(crate) duration: crate::effect::Until,
    pub(crate) clause_tokens: Vec<OwnedLexToken>,
}

pub(crate) fn prepare_cant_sentence_restriction_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<CantSentencePreparedClause>, CardTextError> {
    let Some((duration, clause_tokens)) = parse_restriction_duration_lexed(tokens)? else {
        return Ok(None);
    };
    if clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restriction clause missing body".to_string(),
        ));
    }
    if clause_tokens
        .first()
        .is_some_and(|token| IF_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }

    let clause_tokens = cant_sentence_clause_tokens_for_restriction_scan_lexed(&clause_tokens);
    if !cant_sentence_has_supported_negation_gate_lexed(&clause_tokens) {
        return Ok(None);
    }

    Ok(Some(CantSentencePreparedClause {
        duration,
        clause_tokens,
    }))
}

fn conditional_label_delimiter<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::token_kind(TokenKind::Dash).void(),
        primitives::token_kind(TokenKind::EmDash).void(),
    ))
    .parse_next(input)
}

fn labeled_effect_prefix<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    (conditional_label_phrase, conditional_label_delimiter)
        .void()
        .parse_next(input)
}

pub(crate) fn split_labeled_effect_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, labeled_effect_prefix)?;
    Some(rest)
}

fn labeled_prefix_words(prefix: &str) -> Vec<&str> {
    prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect()
}

pub(crate) fn is_labeled_ability_prefix_text(prefix: &str) -> bool {
    let words = labeled_prefix_words(prefix);
    if words.is_empty() {
        return false;
    }

    if words.len() == 2
        && DESCEND_WORD_PATTERN.matches_word(words[0])
        && words[1].chars().all(|ch| ch.is_ascii_digit())
    {
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
            | "harmonize"
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

pub(crate) fn preserve_labeled_ability_prefix_for_parse_text(prefix: &str) -> bool {
    let words = labeled_prefix_words(prefix);
    let Some(first) = words.first().copied() else {
        return false;
    };

    matches!(
        first,
        "buyback"
            | "bestow"
            | "cumulative"
            | "cycling"
            | "echo"
            | "equip"
            | "escape"
            | "flashback"
            | "harmonize"
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

fn is_generic_ability_label_prefix_text(prefix: &str) -> bool {
    let words = labeled_prefix_words(prefix);
    if words.is_empty() || words.len() > 4 {
        return false;
    }

    words.iter().all(|word| {
        word.chars().all(|ch| ch.is_ascii_alphanumeric())
            && word.chars().any(|ch| ch.is_ascii_alphabetic())
    })
}

fn starts_with_if_clause_text(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed
        .split_whitespace()
        .next()
        .is_some_and(|word| IF_WORD_PATTERN.matches_word(word))
}

pub(crate) fn should_strip_labeled_ability_prefix_text(prefix: &str, remainder: &str) -> bool {
    is_labeled_ability_prefix_text(prefix)
        || (starts_with_if_clause_text(remainder) && is_generic_ability_label_prefix_text(prefix))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChooseNewTargetsClauseSplit<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) count: Option<ChoiceCount>,
    pub(crate) explicit_target: bool,
    pub(crate) reference_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangeTargetClauseSplit {
    pub(crate) target_tokens: Vec<OwnedLexToken>,
    pub(crate) fixed_to_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForEachDoesntClauseSplit<'a> {
    pub(crate) inner_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
    pub(crate) negation_idx: usize,
    pub(crate) negation_len: usize,
}

const CHOOSE_NEW_TARGET_PREFIXES: &[&[&str]] = &[
    &["choose", "new", "targets", "for"],
    &["chooses", "new", "targets", "for"],
    &["choose", "a", "new", "target", "for"],
    &["chooses", "a", "new", "target", "for"],
];
const CHOOSE_NEW_TARGET_REFERENCE_PREFIXES: &[&[&str]] = &[
    &["it"],
    &["them"],
    &["the", "copy"],
    &["the", "copies"],
    &["that", "copy"],
    &["those", "copies"],
    &["the", "spell"],
    &["that", "spell"],
];
const CHANGE_TARGET_PREFIXES: &[&[&str]] = &[
    &["change", "the", "target", "of"],
    &["change", "the", "targets", "of"],
    &["change", "a", "target", "of"],
];
const FOR_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
    &["each", "opponent"],
    &["each", "opponents"],
];
const FOR_EACH_PLAYER_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["each", "player"],
    &["each", "players"],
];

pub(crate) fn split_choose_new_targets_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ChooseNewTargetsClauseSplit<'_>> {
    let (_, mut tail_tokens) =
        primitives::strip_lexed_prefix_phrases(tokens, CHOOSE_NEW_TARGET_PREFIXES)?;
    if tail_tokens.is_empty() {
        return None;
    }

    if let Some(if_idx) =
        find_token_index(tail_tokens, |token| IF_WORD_PATTERN.matches_token(token))
    {
        tail_tokens = &tail_tokens[..if_idx];
    }
    if tail_tokens.is_empty() {
        return None;
    }

    if primitives::starts_with_any_phrase(tail_tokens, CHOOSE_NEW_TARGET_REFERENCE_PREFIXES) {
        return Some(ChooseNewTargetsClauseSplit {
            target_tokens: tail_tokens,
            count: None,
            explicit_target: false,
            reference_target: true,
        });
    }

    if let Some((prefix, rest)) = primitives::strip_lexed_prefix_phrases(
        tail_tokens,
        &[&["any", "number", "of"], &["target"]],
    ) {
        return Some(ChooseNewTargetsClauseSplit {
            target_tokens: rest,
            count: (prefix.len() == 3).then_some(ChoiceCount::any_number()),
            explicit_target: prefix.len() != 3,
            reference_target: false,
        });
    }

    Some(ChooseNewTargetsClauseSplit {
        target_tokens: tail_tokens,
        count: None,
        explicit_target: false,
        reference_target: false,
    })
}

pub(crate) fn split_change_target_unless_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    primitives::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        primitives::kw("unless").void()
    })
    .map(|(main, unless)| (trim_lexed_commas(main), trim_lexed_commas(unless)))
}

pub(crate) fn split_change_target_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ChangeTargetClauseSplit> {
    let (_, after_prefix_tokens) =
        primitives::strip_lexed_prefix_phrases(tokens, CHANGE_TARGET_PREFIXES)?;
    if after_prefix_tokens.is_empty() {
        return None;
    }

    let mut tail_tokens = trim_commas(after_prefix_tokens).to_vec();
    let mut fixed_to_source = false;
    if let Some((before_to, to_tail)) =
        primitives::split_lexed_once_on_separator(&tail_tokens, || {
            use winnow::Parser as _;
            primitives::kw("to").void()
        })
        && token_slice_first_is(to_tail, "this")
    {
        fixed_to_source = true;
        tail_tokens.truncate(before_to.len());
    }

    Some(ChangeTargetClauseSplit {
        target_tokens: tail_tokens,
        fixed_to_source,
    })
}

pub(crate) fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    if let Some(idx) = COMPACT_NEGATED_ACTION_WORD_PATTERN.find_word(words) {
        return Some((idx, 1));
    }
    if let Some(idx) = SPLIT_NEGATED_ACTION_WORD_PATTERN.find_exact_window(words, 2) {
        return Some((idx, 2));
    }
    None
}

fn split_for_each_doesnt_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    prefixes: &'static [&'static [&'static str]],
) -> Option<ForEachDoesntClauseSplit<'a>> {
    let mut clause_tokens = tokens;
    if token_word_refs(clause_tokens)
        .first()
        .is_some_and(|word| THEN_WORD_PATTERN.matches_word(word))
    {
        clause_tokens = &clause_tokens[1..];
    }
    let start = primitives::words_match_any_prefix(clause_tokens, prefixes)?
        .0
        .len();
    let inner_tokens = trim_lexed_commas(&clause_tokens[start..]);
    let inner_words = token_word_refs(inner_tokens);
    if !inner_words
        .first()
        .is_some_and(|word| WHO_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let (negation_idx, negation_len) = negated_action_word_index(&inner_words)?;
    let effect_token_start =
        if let Some(comma_idx) = find_token_index(inner_tokens, |token| token.is_comma()) {
            comma_idx + 1
        } else if let Some(this_way_idx) = THIS_WAY_PATTERN.find_exact_window(&inner_words, 2) {
            parser_token_word_positions(inner_tokens)
                .get(this_way_idx + 2)
                .map(|(idx, _)| *idx)
                .unwrap_or(inner_tokens.len())
        } else {
            parser_token_word_positions(inner_tokens)
                .get(negation_idx + negation_len)
                .map(|(idx, _)| *idx)
                .unwrap_or(inner_tokens.len())
        };
    let effect_tokens = trim_lexed_commas(&inner_tokens[effect_token_start..]);
    (!effect_tokens.is_empty()).then_some(ForEachDoesntClauseSplit {
        inner_tokens,
        effect_tokens,
        negation_idx,
        negation_len,
    })
}

pub(crate) fn split_for_each_opponent_doesnt_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDoesntClauseSplit<'_>> {
    split_for_each_doesnt_clause_lexed(tokens, FOR_EACH_OPPONENT_PREFIXES)
}

pub(crate) fn split_for_each_player_doesnt_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDoesntClauseSplit<'_>> {
    split_for_each_doesnt_clause_lexed(tokens, FOR_EACH_PLAYER_PREFIXES)
}

pub(crate) fn split_negated_who_this_way_filter_tokens_lexed(
    inner_tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let inner_words = token_word_refs(inner_tokens);
    if !inner_words
        .first()
        .is_some_and(|word| WHO_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let this_way_idx = THIS_WAY_PATTERN.find_exact_window(&inner_words, 2)?;
    let (negation_idx, negation_len) = negated_action_word_index(&inner_words)?;
    let verb_idx = negation_idx + negation_len;
    let verb = inner_words.get(verb_idx).copied().unwrap_or("");
    if !EFFECT_DISCARD_OR_DISCARDED_WORD_PATTERN.matches_word(verb) || this_way_idx <= verb_idx + 1
    {
        return None;
    }

    let parser_words = parser_token_word_positions(inner_tokens);
    let filter_start = parser_words.get(verb_idx + 1).map(|(idx, _)| *idx)?;
    let filter_end = parser_words.get(this_way_idx).map(|(idx, _)| *idx)?;
    let filter_tokens = trim_lexed_commas(&inner_tokens[filter_start..filter_end]);
    (!filter_tokens.is_empty()).then_some(filter_tokens)
}

const PREVENT_DAMAGE_BY_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "by"]];
const PREVENT_DAMAGE_TO_AND_BY_PREFIXES: &[&[&str]] =
    &[&["that", "would", "be", "dealt", "to", "and", "dealt", "by"]];
const PREVENT_DAMAGE_TO_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "to"]];

pub(crate) fn parse_prevent_damage_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    let prefix = ["prevent", "all", "combat", "damage"];
    if primitives::words_match_prefix(tokens, &prefix).is_none() {
        return Ok(None);
    }

    let Some(this_turn_idx) = THIS_TURN_PATTERN.find_exact_window(&words, 2) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    };
    if THIS_TURN_PATTERN.matches_words(&words[this_turn_idx + 2..]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }
    if this_turn_idx < prefix.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut core_words = Vec::with_capacity(words.len() - prefix.len() - 2);
    core_words.extend_from_slice(&words[prefix.len()..this_turn_idx]);
    core_words.extend_from_slice(&words[this_turn_idx + 2..]);
    let mut core_tokens = Vec::with_capacity(tokens.len() - prefix.len() - 2);
    core_tokens.extend_from_slice(&tokens[prefix.len()..this_turn_idx]);
    core_tokens.extend_from_slice(&tokens[this_turn_idx + 2..]);

    if THAT_WOULD_BE_DEALT_PATTERN.matches_words(&core_words) {
        return Ok(Some(EffectAst::subject_verb_prevent_all_combat_damage(
            crate::effect::Until::EndOfTurn,
        )));
    }

    if primitives::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_BY_PREFIXES).is_some() {
        let source_tokens = &core_tokens[5..];
        if source_tokens
            .first()
            .is_some_and(|token| PREVENT_DAMAGE_SOURCE_HEAD_WORD_PATTERN.matches_token(token))
        {
            let (source, has_color_condition) =
                parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
            return Ok(Some(prevent_damage_effect_with_optional_condition(
                source,
                has_color_condition,
            )));
        }
        if let Ok(source_filter) = parse_object_filter(source_tokens, false) {
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter(
                    source_filter,
                    crate::effect::Until::EndOfTurn,
                ),
            ));
        }
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
        )));
    }

    if primitives::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_TO_AND_BY_PREFIXES).is_some()
    {
        let source_tokens = &core_tokens[8..];
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
        )));
    }

    if primitives::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_TO_PREFIXES).is_some() {
        return parse_prevent_damage_target_scope_lexed(&core_tokens[5..], &words);
    }

    if let Some(would_idx) = WOULD_WORD_PATTERN.find_word(&core_words)
        && core_words
            .get(would_idx + 1)
            .is_some_and(|word| DEAL_WORD_PATTERN.matches_word(word))
    {
        let source_tokens = &core_tokens[..would_idx];
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        let has_color_condition = has_color_condition
            || prevent_damage_shares_color_clause_lexed(&core_tokens[would_idx + 2..]);
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all-combat-damage clause tail (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_prevent_damage_source_target_lexed(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(TargetAst, bool), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all source target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (tokens, has_color_condition) = strip_prevent_damage_shares_color_clause_lexed(tokens);
    let source_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    let is_explicit_reference = PREVENT_DAMAGE_REFERENCE_MARKER_PATTERN
        .matches_words(&source_words)
        || source_words
            .first()
            .is_some_and(|word| PREVENT_DAMAGE_EXPLICIT_REFERENCE_WORD_PATTERN.matches_word(word));
    if !is_explicit_reference {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            source_words.join(" ")
        )));
    }

    let source = parse_target_phrase(tokens)?;
    match source {
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _) => {
            Ok((source, has_color_condition))
        }
        _ => Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            token_word_refs(tokens).join(" ")
        ))),
    }
}

fn prevent_damage_effect_with_optional_condition(
    source: TargetAst,
    has_color_condition: bool,
) -> EffectAst {
    let condition_filter = match &source {
        TargetAst::Object(filter, _, _) => Some(filter.clone()),
        _ => None,
    };
    let prevent = EffectAst::subject_verb_prevent_all_combat_damage_from_source(
        source,
        crate::effect::Until::EndOfTurn,
    );
    if has_color_condition {
        let predicate = condition_filter.map_or_else(
            || {
                PredicateAst::TargetMatches(
                    ObjectFilter::default().shares_color_with_tagged(TagKey::from(IT_TAG)),
                )
            },
            |filter| {
                PredicateAst::TargetMatches(filter.shares_color_with_tagged(TagKey::from(IT_TAG)))
            },
        );
        EffectAst::Conditional {
            predicate,
            if_true: vec![prevent],
            if_false: Vec::new(),
        }
    } else {
        prevent
    }
}

fn prevent_damage_shares_color_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    matches!(
        words.as_slice(),
        ["if", "it", "shares", "color", "with", "that", "permanent"]
            | ["if", "it", "shares", "color", "with", "that", "object"]
            | ["if", "it", "shares", "color", "with", "that", "creature"]
            | ["if", "it", "shares", "color", "with", "it"]
    )
}

fn strip_prevent_damage_shares_color_clause_lexed(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], bool) {
    let Some(if_idx) = tokens
        .iter()
        .rposition(|token| IF_WORD_PATTERN.matches_token(token))
    else {
        return (tokens, false);
    };
    if prevent_damage_shares_color_clause_lexed(&tokens[if_idx..]) {
        return (&tokens[..if_idx], true);
    }
    (tokens, false)
}

pub(crate) fn parse_prevent_damage_target_scope_lexed(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    if PREVENT_DAMAGE_PLAYERS_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_combat_damage_to_players(
                crate::effect::Until::EndOfTurn,
            ),
        ));
    }
    if YOU_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_combat_damage_to_you(
                crate::effect::Until::EndOfTurn,
            ),
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all target scope '{}'",
        token_word_refs(tokens).join(" ")
    )))
}

fn conditional_sentence_family_head<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["then", "if"]),
        (
            conditional_label_phrase,
            opt(conditional_label_delimiter),
            primitives::kw("if"),
        )
            .void(),
        primitives::kw("if").void(),
    ))
    .parse_next(input)
}

pub(crate) fn split_conditional_sentence_family_head_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, conditional_sentence_family_head)?;
    let consumed = tokens.len().checked_sub(rest.len())?;
    consumed.checked_sub(1).map(|if_idx| &tokens[if_idx..])
}

pub(crate) fn parse_conditional_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let split = split_if_clause_lexed(tokens, parse_effect_chain_lexed)?;

    Ok(vec![match split.predicate {
        IfClausePredicateSpec::Conditional(predicate) => EffectAst::Conditional {
            predicate,
            if_true: split.effects,
            if_false: Vec::new(),
        },
        IfClausePredicateSpec::Result(predicate) => EffectAst::IfResult {
            predicate,
            effects: split.effects,
        },
    }])
}

pub(crate) fn parse_conditional_sentence_family_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(conditional_tokens) = split_conditional_sentence_family_head_lexed(tokens) else {
        return Ok(None);
    };

    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        conditional_tokens,
        parse_effect_chain_lexed,
    )
    .map(Some)
}

pub(crate) fn parse_cant_effect_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(prefix_tokens) = split_cant_sentence_next_turn_prefix_lexed(tokens) {
        let prefix_tokens = prefix_tokens.as_slice();
        if let Some(parsed) = parse_cant_restriction_clause(prefix_tokens)? {
            let next_turn_effects = match parsed.restriction {
                crate::effect::Restriction::CastSpellsMatching(player, spell_filter) => {
                    let nested = crate::effect::Restriction::cast_spells_matching(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }])
                        }
                        _ => None,
                    }
                }
                crate::effect::Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => {
                    let nested = crate::effect::Restriction::CastMoreThanOneSpellEachTurn(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }])
                        }
                        _ => None,
                    }
                }
                _ => None,
            };

            if let Some(next_turn_effects) = next_turn_effects {
                return Ok(Some(next_turn_effects));
            }
        }
    }

    let source_tapped_duration = cant_sentence_has_source_remains_tapped_duration(tokens);
    let words = token_word_refs(tokens);
    if LOSE_MANA_STEPS_PHASES_END_PATTERN.matches_words(&words) {
        return Ok(Some(vec![
            EffectAst::subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn(),
        ]));
    }
    let Some(prepared_clause) = prepare_cant_sentence_restriction_clause_lexed(tokens)? else {
        return Ok(None);
    };
    let duration = prepared_clause.duration;
    let clause_tokens = prepared_clause.clause_tokens;

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported restriction clause body (clause: '{}')",
            token_word_refs(&clause_tokens).join(" ")
        )));
    };

    let mut target: Option<crate::cards::builders::TargetAst> = None;
    let mut effects = Vec::new();
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target {
                if *existing != parsed_target {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mixed restriction targets (clause: '{}')",
                        token_word_refs(&clause_tokens).join(" ")
                    )));
                }
            } else {
                target = Some(parsed_target);
            }
        }
        effects.push(EffectAst::subject_verb_cant(
            parsed.restriction,
            duration.clone(),
            source_tapped_duration.then_some(crate::ConditionExpr::SourceIsTapped),
        ));
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_cant_effect_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence_with_grammar_entrypoint_lexed(tokens)
}

pub(crate) fn parse_search_library_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    subject_starts_effect_lexed: fn(&[OwnedLexToken]) -> bool,
    parse_leading_effects_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
    parse_effect_clause_lexed: fn(&[OwnedLexToken]) -> Result<EffectAst, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn has_trailing_that_player_shuffle(tokens: &[OwnedLexToken]) -> bool {
        super::primitives::words_find_phrase(tokens, &["then", "that", "player", "shuffle"])
            .is_some()
            || super::primitives::words_find_phrase(tokens, &["then", "that", "player", "shuffles"])
                .is_some()
            || super::primitives::words_find_phrase(tokens, &["that", "player", "shuffle"])
                .is_some()
            || super::primitives::words_find_phrase(tokens, &["that", "player", "shuffles"])
                .is_some()
    }

    let words_all = parser_token_word_refs(tokens);
    let Some(head_split) = split_search_library_sentence_head_lexed(tokens) else {
        return Ok(None);
    };

    let subject_prelude = parse_search_library_leading_effect_prelude_lexed(
        head_split.subject_tokens,
        subject_starts_effect_lexed,
        parse_leading_effects_lexed,
    )?;
    let subject_tokens = subject_prelude.subject_tokens;
    let sentence_has_direct_may = head_split.sentence_has_direct_may;
    let mut leading_effects = subject_prelude.leading_effects;
    let wrap_each_target_player =
        search_library_subject_wraps_each_target_player_lexed(subject_tokens);
    let iterated_subject_filter =
        parse_search_library_iterated_object_subject_lexed(subject_tokens)?;
    let chooser = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb=Search subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));

    let search_tokens = head_split.search_tokens;
    if !search_library_starts_with_search_verb_lexed(search_tokens) {
        return Ok(None);
    }
    let search_words = parser_token_word_refs(search_tokens);
    if search_words.is_empty() {
        return Ok(None);
    }
    let Some(subject_routing) = derive_search_library_subject_routing_lexed(search_tokens, chooser)
    else {
        return Ok(None);
    };
    let player = subject_routing.player;
    let search_player_target = subject_routing.search_player_target;
    let forced_library_owner = subject_routing.forced_library_owner;
    let search_zones_override = subject_routing.search_zones_override;
    if search_library_has_unsupported_top_position_probe(&search_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library top-position clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let clause_markers = scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned search-library clause marker scan should produce defaults");
    let for_idx = clause_markers.for_idx;
    let put_idx = clause_markers.put_idx;
    let has_explicit_destination = clause_markers.has_explicit_destination;
    let filter_boundary = clause_markers.filter_boundary;

    let filter_end =
        find_search_library_filter_boundary_lexed(search_tokens, for_idx, filter_boundary)
            .filter_end;

    if filter_end <= for_idx + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing search filter in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let count_tokens = &search_tokens[for_idx + 1..filter_end];
    let count_prefix = parse_search_library_count_prefix_lexed(count_tokens);
    let mut count = count_prefix.count;
    let search_mode = count_prefix.search_mode;
    let count_used = count_prefix.count_used;
    let mut prefix_count_value = count_prefix.count_value;

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Err(CardTextError::ParseError(format!(
            "missing object selector in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let mut raw_filter_tokens = trim_commas(&search_tokens[filter_start..filter_end]).to_vec();
    if THAT_MANY_PREFIX_PATTERN.matches_words(&token_word_refs(&raw_filter_tokens)) {
        prefix_count_value.get_or_insert(Value::Count(ObjectFilter::tagged(TagKey::from(IT_TAG))));
        count = if search_mode == SearchSelectionMode::Optional {
            ChoiceCount::up_to_dynamic_x()
        } else {
            ChoiceCount::dynamic_x()
        };
        raw_filter_tokens.drain(0..2);
    }
    let (filter_tokens, count_value) = if let Some((base_filter_tokens, count_value)) =
        split_search_library_count_value_clause_lexed(&raw_filter_tokens)?
    {
        (base_filter_tokens, Some(count_value))
    } else {
        (raw_filter_tokens, prefix_count_value)
    };
    let (filter_tokens, mana_constraint) = if let Some((base_filter_tokens, mana_constraint)) =
        extract_search_library_mana_constraint(&filter_tokens)
    {
        (base_filter_tokens, Some(mana_constraint))
    } else {
        (filter_tokens.to_vec(), None)
    };
    let (filter_tokens, distinct_names) =
        strip_search_library_different_names_clause_lexed(&filter_tokens);
    let same_name_split = parse_search_library_same_name_reference_lexed(
        &filter_tokens,
        filter_tokens.clone(),
        &words_all,
    )?;
    let filter_tokens = same_name_split.filter_tokens;
    let same_name_reference = same_name_split.same_name_reference;
    let same_name_relation = same_name_split.same_name_relation;
    let same_name_reference_requires_setup = matches!(
        same_name_reference,
        Some(SearchLibrarySameNameReference::Target(_))
            | Some(SearchLibrarySameNameReference::Choose { .. })
    );

    let named_filters = if count_used == 0 {
        split_search_named_item_filters_lexed(&filter_tokens, &words_all)?
    } else {
        None
    };
    let mut filter = parse_search_library_object_filter_lexed(&filter_tokens, &words_all)?;
    filter.distinct_names = distinct_names;
    if let Some(same_name_tag) = same_name_reference
        .as_ref()
        .map(|reference| match reference {
            SearchLibrarySameNameReference::Tagged(tag) => tag.clone(),
            SearchLibrarySameNameReference::Target(_) => TagKey::from(IT_TAG),
            SearchLibrarySameNameReference::Choose { tag, .. } => tag.clone(),
        })
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: same_name_tag.clone(),
            relation: same_name_relation,
        });
    }
    if filter.owner.is_none()
        && let Some(owner) = forced_library_owner.clone()
    {
        filter.owner = Some(owner);
    }
    normalize_search_library_filter(&mut filter);
    if let Some(mana_constraint) = mana_constraint {
        apply_search_library_mana_constraint(&mut filter, mana_constraint);
    }
    let search_zones_are_library_only = match search_zones_override.as_ref() {
        None => true,
        Some(zones) => zones.len() == 1 && zones[0] == Zone::Library,
    };
    if search_zones_are_library_only {
        filter.zone = Some(Zone::Library);
    }

    let discard_before_shuffle_followup =
        find_search_library_discard_before_shuffle_followup_lexed(search_tokens, put_idx);
    let trailing_discard_before_shuffle = discard_before_shuffle_followup.is_some();
    let effect_routing = derive_search_library_effect_routing_lexed(
        tokens,
        search_tokens,
        clause_markers,
        trailing_discard_before_shuffle,
    );
    let destination = effect_routing.destination;
    let reveal = effect_routing.reveal;
    let face_down_exile = effect_routing.face_down_exile;
    let original_shuffle = effect_routing.shuffle;
    let trailing_create_followup = find_search_library_trailing_create_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    );
    let shuffle = original_shuffle && trailing_create_followup.is_none();
    let split_battlefield_and_hand = effect_routing.split_battlefield_and_hand;
    let library_position_from_top = effect_routing.library_position_from_top.clone();
    let mut handled_direct_may_in_iterated_search = false;
    let mut effects = if let Some(iterated_filter) = iterated_subject_filter.clone()
        && has_explicit_destination
        && named_filters.is_none()
        && !split_battlefield_and_hand
        && !(destination == Zone::Exile && face_down_exile)
    {
        let searched_tag: TagKey = "searched".into();
        let search_zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        // Always use the search subject `player` so the shuffle references
        // the searcher, not the last-referenced player from a preceding effect.
        let shuffle_player = player;

        let mut per_object_effects = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: searched_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if sentence_has_direct_may {
            handled_direct_may_in_iterated_search = true;
            per_object_effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
                EffectAst::May {
                    effects: per_object_effects,
                }
            } else {
                EffectAst::MayByPlayer {
                    player: chooser,
                    effects: per_object_effects,
                }
            }];
        }

        let mut sequence = vec![EffectAst::ForEachObject {
            filter: iterated_filter,
            effects: per_object_effects,
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
        }
        if shuffle
            && destination == Zone::Library
            && zone_slice_contains(&search_zones, Zone::Library)
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: searched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                destination,
                matches!(destination, Zone::Library),
                ReturnControllerAst::Preserve,
                battlefield_tapped,
                None,
            )],
        });
        if shuffle
            && !(destination == Zone::Library && zone_slice_contains(&search_zones, Zone::Library))
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if let Some(named_filters) = named_filters {
        let searched_tag: TagKey = "searched_named".into();
        let zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let mut sequence = Vec::new();
        for mut named_filter in named_filters {
            if named_filter.owner.is_none()
                && let Some(owner) = forced_library_owner.clone()
            {
                named_filter.owner = Some(owner);
            }
            normalize_search_library_filter(&mut named_filter);
            sequence.push(EffectAst::ChooseObjectsAcrossZones {
                filter: named_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: chooser,
                tag: searched_tag.clone(),
                zones: zones.clone(),
                search_mode: Some(SearchSelectionMode::Exact),
            });
        }
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
        }
        sequence.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
            destination,
            matches!(destination, Zone::Library),
            ReturnControllerAst::Preserve,
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier,
            None,
        ));
        if shuffle && zones.contains(&Zone::Library) {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if !has_explicit_destination {
        let chosen_tag: TagKey = "searched".into();
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones_override.unwrap_or_else(|| vec![Zone::Library]),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()));
        }
        if shuffle {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if let Some(search_zones) = search_zones_override.clone() {
        let chosen_tag: TagKey = "searched_multi_zone".into();
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        // Use the search subject `player` (e.g. Implicit/You) rather than
        // PlayerAst::That, which would resolve to the last referenced player
        // in a preceding effect (e.g. "target player" from a damage clause).
        let shuffle_player = player;
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()));
        }
        if shuffle
            && destination == Zone::Library
            && zone_slice_contains(&search_zones, Zone::Library)
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(chosen_tag, span_from_tokens(tokens)),
                destination,
                matches!(destination, Zone::Library),
                ReturnControllerAst::Preserve,
                battlefield_tapped,
                None,
            )],
        });
        if shuffle
            && !(destination == Zone::Library && zone_slice_contains(&search_zones, Zone::Library))
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if split_battlefield_and_hand {
        let battlefield_tapped = effect_routing.has_tapped_modifier;
        vec![
            EffectAst::subject_verb_search_library(
                filter.clone(),
                Zone::Battlefield,
                chooser,
                player,
                search_mode,
                reveal,
                false,
                ChoiceCount::up_to(1),
                None,
                None,
                battlefield_tapped,
            ),
            EffectAst::subject_verb_search_library(
                filter,
                Zone::Hand,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle,
                ChoiceCount::up_to(1),
                None,
                None,
                false,
            ),
        ]
    } else if destination == Zone::Exile && face_down_exile {
        let searched_tag: TagKey = "searched_face_down".into();
        let mut sequence = vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value: count_value.clone(),
                player: chooser,
                tag: searched_tag.clone(),
                zones: vec![Zone::Library],
                search_mode: Some(search_mode),
            },
            EffectAst::subject_verb_exile(
                TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                true,
            ),
        ];
        if shuffle {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else {
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        vec![EffectAst::subject_verb_search_library(
            filter,
            destination,
            chooser,
            player,
            search_mode,
            reveal,
            shuffle,
            count,
            count_value.clone(),
            library_position_from_top,
            battlefield_tapped,
        )]
    };

    if let Some(discard_followup) = discard_before_shuffle_followup {
        let discard_tokens =
            trim_commas(&search_tokens[discard_followup.discard_idx..discard_followup.discard_end]);
        if !discard_tokens.is_empty() {
            effects.push(parse_effect_clause_lexed(&discard_tokens)?);
        }
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    if has_trailing_that_player_shuffle(tokens) {
        let mut rewrote_existing_shuffle = false;
        for effect in &mut effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect
                && matches!(subject_verb.action, SubjectVerbActionAst::ShuffleLibrary)
                && matches!(
                    subject_verb.subject.player,
                    PlayerAst::You | PlayerAst::Implicit
                )
            {
                subject_verb.subject.player = PlayerAst::That;
                rewrote_existing_shuffle = true;
            }
        }
        if !rewrote_existing_shuffle {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if let Some(target) = search_player_target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }

    if let Some(trailing_tokens) = find_search_library_trailing_life_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    ) {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
    }

    if let Some(trailing_tokens) = trailing_create_followup {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
        if original_shuffle {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if let Some(reference) = same_name_reference {
        match reference {
            SearchLibrarySameNameReference::Tagged(_) => {}
            SearchLibrarySameNameReference::Target(target) => {
                effects.insert(0, EffectAst::subject_verb_target_only(target));
            }
            SearchLibrarySameNameReference::Choose { filter, tag } => {
                if same_name_relation == TaggedOpbjectRelation::DifferentNameFromTagged {
                    effects.insert(
                        0,
                        EffectAst::subject_verb_tag_matching_objects(
                            filter,
                            vec![Zone::Battlefield],
                            tag,
                        ),
                    );
                } else {
                    effects.insert(
                        0,
                        EffectAst::ChooseObjects {
                            filter,
                            count: ChoiceCount::exactly(1),
                            count_value: None,
                            player,
                            tag,
                        },
                    );
                }
            }
        }
    }

    if sentence_has_direct_may && !handled_direct_may_in_iterated_search {
        effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
            EffectAst::May { effects }
        } else {
            EffectAst::MayByPlayer {
                player: chooser,
                effects,
            }
        }];
    }

    if let Some(iterated_filter) = iterated_subject_filter
        && !has_explicit_destination
        && !same_name_reference_requires_setup
    {
        effects = vec![EffectAst::ForEachObject {
            filter: iterated_filter,
            effects,
        }];
    }

    if !leading_effects.is_empty() {
        leading_effects.extend(effects);
        return Ok(Some(leading_effects));
    }

    if wrap_each_target_player {
        effects = vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::target_player(),
            effects,
        }];
    }

    Ok(Some(effects))
}
pub(crate) fn cant_sentence_has_source_remains_tapped_duration(tokens: &[OwnedLexToken]) -> bool {
    let mut has_for_as_long_as = false;
    let mut has_remains = false;
    let mut has_tapped = false;
    let mut has_source_word = false;
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        if !has_for_as_long_as
            && primitives::parse_prefix(&tokens[cursor..], cant_sentence_for_as_long_as_marker)
                .is_some()
        {
            has_for_as_long_as = true;
        }

        let token = &tokens[cursor];
        has_remains |= REMAINS_WORD_PATTERN.matches_token(token);
        has_tapped |= TAPPED_WORD_PATTERN.matches_token(token);
        has_source_word |= SOURCE_REFERENCE_WORD_PATTERN.matches_token(token);
        cursor += 1;
    }

    has_for_as_long_as && has_remains && has_tapped && has_source_word
}
