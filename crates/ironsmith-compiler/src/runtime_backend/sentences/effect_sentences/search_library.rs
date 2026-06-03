use super::super::grammar::primitives::{self as grammar, split_lexed_slices_on_or};
use super::super::grammar::values::parse_value_comparison_tokens;
use super::super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, contains_token_word, find_token_word_sequence_span,
    lex_line, token_slice_starts_with_at, token_word_refs, trim_lexed_commas,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::token_primitives::{
    find_index as find_token_index, parse_simple_restriction_duration_prefix,
    parse_simple_restriction_duration_suffix, rfind_index as rfind_token_index,
};
use super::super::util::{
    helper_tag_for_tokens, is_article, parse_number, parse_number_word_u32, parse_subject,
    parse_target_phrase, parse_zone_word, possessive_normalized_word_refs, span_from_tokens,
    strip_leading_token_words_any, token_index_for_word_index, trim_commas, words,
};
use super::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::sentence_helpers::*;
use super::{
    find_verb, parse_effect_chain, parse_effect_chain_with_subject_verb_primitives,
    parse_effect_clause,
};
use crate::cards::builders::{
    CardTextError, CarryContext, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, PlayerAst, ReturnControllerAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) enum SearchLibraryManaConstraint {
    Equal(u32),
    LessThanOrEqual(u32),
    GreaterThanOrEqual(u32),
    OneOf(Vec<u32>),
}

const SEARCH_FROM_THE_TOP_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "the", "top"]);
const SEARCH_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const SEARCH_FOR_AS_LONG_AS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["for", "as", "long", "as"]]);
const SEARCH_THIS_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "turn"]]);
const SEARCH_SHUFFLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["shuffle"], &["shuffles"]]);
const SEARCH_MAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["may"]);
const SEARCH_EACH_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["each", "player"], &["each", "players"]]);
const SEARCH_INTO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["into"]);
const SEARCH_LIBRARY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["library"], &["libraries"]]);
const SEARCH_SOURCE_AND_GRAVEYARD_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "artifact", "and"],
            &["this", "permanent", "and"],
            &["this", "card", "and"],
        ]
);
const SEARCH_OWNER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "owner", "of"]);
const SEARCH_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const SEARCH_SECOND_ZONE_SKIP_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["cards"], &["from"]]);
const SEARCH_HAND_OR_GRAVEYARD_ZONE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["hand"], &["hands"], &["graveyard"], &["graveyards"],]);
const SEARCH_TARGET_OR_YOUR_OWNER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["target", "player"],
            &["target", "players"],
            &["target", "opponent"],
            &["target", "opponents"],
            &["your"],
        ]
);
const SEARCH_EXILE_OR_EXILES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["exile"], &["exiles"]]);
const SEARCH_EXILED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["exiled", "this", "way"]]);
const SEARCH_EXILED_WITH_THIS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["exiled", "with", "this"]]);
const SEARCH_THEN_PUTS_ALL_PERMANENT_CARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["then", "puts", "all", "permanent", "cards"]]);
const SEARCH_AMONG_THEM_ONTO_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["among", "them", "onto", "battlefield"],
            &["among", "them", "onto", "the", "battlefield"],
        ]]
);
const SEARCH_DESTROYED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["destroyed", "this", "way"]]);
const SEARCH_DIED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["died", "this", "way"]]);
const SEARCH_PUT_INTO_GRAVEYARD_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["put", "into", "a", "graveyard", "this", "way"],
            &["put", "into", "graveyard", "this", "way"],
            &["put", "into", "their", "graveyard", "this", "way"],
            &["put", "into", "its", "graveyard", "this", "way"],
        ]]
);
const SEARCH_REVEAL_UNTIL_CONTROLLER_REVEALS_PREFIX: &[&str] = &[
    "its",
    "controller",
    "reveals",
    "cards",
    "from",
    "the",
    "top",
    "of",
    "their",
    "library",
    "until",
    "they",
    "reveal",
];
const SEARCH_PUTS_THAT_CARD_ONTO_BATTLEFIELD_PHRASE: &[&str] =
    &["puts", "that", "card", "onto", "the", "battlefield"];
const SEARCH_THEN_SHUFFLES_PHRASE: &[&str] = &["then", "shuffles"];
const SEARCH_THEN_PUTS_REST_BOTTOM_PHRASE: &[&str] =
    &["then", "puts", "the", "rest", "on", "the", "bottom"];
const SEARCH_TARGET_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "opponent"]);
const SEARCH_TARGET_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "player"]);
const SEARCH_THEIR_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["their", "graveyard"]);
const SEARCH_CREATURE_CONTROLLED_BY_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature", "they", "control"],
            &["creature", "that", "player", "controls"],
        ]
);
const SEARCH_ENCHANT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["enchant"]);
const SEARCH_EARTHBEND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["earthbend"]);

fn segment_starts_effect_lexed(tokens: &[OwnedLexToken]) -> bool {
    super::lex_chain_helpers::segment_has_effect_head_lexed(tokens)
}

pub(crate) fn parse_search_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::super::grammar::effects::parse_search_library_sentence_with_grammar_entrypoint_lexed(
        tokens,
        segment_starts_effect_lexed,
        super::chain_carry::parse_effect_chain_with_subject_verb_primitives_lexed,
        super::clause_dispatch::parse_effect_clause_lexed,
    )
}

#[allow(dead_code)]
pub(crate) fn word_slice_mentions_nth_from_top(words: &[&str]) -> bool {
    let mut idx = 0usize;
    while idx + 3 < words.len() {
        if SEARCH_FROM_THE_TOP_TAIL_PATTERN.matches_words(&words[idx + 1..idx + 4]) {
            return true;
        }
        idx += 1;
    }
    false
}

fn is_source_reference_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    [
        "this",
        "thiss",
        "source",
        "artifact",
        "creature",
        "permanent",
    ]
    .iter()
    .any(|word| contains_token_word(tokens, word))
}

fn is_as_long_as_you_control_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    contains_token_word(tokens, "you")
        && contains_token_word(tokens, "control")
        && is_source_reference_duration_tokens(tokens)
}

fn is_source_remains_tapped_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    SEARCH_FOR_AS_LONG_AS_PATTERN.matches_words(&token_word_refs(tokens))
        && contains_token_word(tokens, "remains")
        && contains_token_word(tokens, "tapped")
        && is_source_reference_duration_tokens(tokens)
}

fn is_source_remains_battlefield_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    SEARCH_FOR_AS_LONG_AS_PATTERN.matches_words(&token_word_refs(tokens))
        && contains_token_word(tokens, "remains")
        && contains_token_word(tokens, "battlefield")
        && is_source_reference_duration_tokens(tokens)
}

fn remove_this_turn_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut cleaned = Vec::new();
    let mut idx = 0usize;
    while idx < tokens.len() {
        if token_slice_starts_with_at(tokens, idx, &["this", "turn"]) {
            idx += 2;
            continue;
        }
        cleaned.push(tokens[idx].clone());
        idx += 1;
    }
    cleaned
}

#[allow(dead_code)]
pub(crate) fn zone_slice_contains(zones: &[Zone], expected: Zone) -> bool {
    zones.iter().any(|zone| *zone == expected)
}

fn card_type_slice_contains(card_types: &[CardType], expected: CardType) -> bool {
    card_types.iter().any(|card_type| *card_type == expected)
}

fn word_has_fragment(word: &str, fragment: &str) -> bool {
    word.match_indices(fragment).next().is_some()
}

pub(crate) fn parse_search_library_disjunction_filter(
    filter_tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let segments = split_lexed_slices_on_or(filter_tokens);
    if segments.len() < 2 {
        return None;
    }

    let mut branches = Vec::new();
    for segment in segments {
        let trimmed = trim_commas(segment);
        if trimmed.is_empty() {
            return None;
        }
        let Ok(filter) = parse_object_filter_lexed(&trimmed, false) else {
            return None;
        };
        branches.push(filter);
    }

    if branches.len() < 2 {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Some(filter)
}

pub(crate) fn parse_restriction_duration_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    use crate::effect::Until;

    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some((duration, rest)) = parse_simple_restriction_duration_prefix(tokens) {
        return Ok(Some((duration, trim_lexed_commas(rest).to_vec())));
    }

    if token_word_refs(tokens).len() < 2 {
        return Ok(None);
    }

    if grammar::parse_prefix(tokens, grammar::phrase(&["for", "as", "long", "as"])).is_some() {
        if !is_as_long_as_you_control_duration_tokens(tokens) {
            return Ok(None);
        }
        let Some((_before, after)) =
            grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        else {
            return Err(CardTextError::ParseError(
                "missing comma after duration prefix".to_string(),
            ));
        };
        let remainder = trim_lexed_commas(after).to_vec();
        return Ok(Some((Until::YouStopControllingThis, remainder)));
    }

    if let Some((rest, duration)) = parse_simple_restriction_duration_suffix(tokens) {
        let remainder = trim_lexed_commas(rest).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((duration, remainder)));
        }
    }

    if let Some((token_idx, _)) =
        find_token_word_sequence_span(tokens, &["for", "as", "long", "as"])
    {
        let suffix_tokens = &tokens[token_idx..];
        if is_source_remains_tapped_duration_tokens(suffix_tokens) {
            let remainder = trim_lexed_commas(&tokens[..token_idx]).to_vec();
            return Ok(Some((Until::SourceUntaps, remainder)));
        }
        if is_source_remains_battlefield_duration_tokens(suffix_tokens) {
            let remainder = trim_lexed_commas(&tokens[..token_idx]).to_vec();
            return Ok(Some((Until::ThisLeavesTheBattlefield, remainder)));
        }
        if is_as_long_as_you_control_duration_tokens(suffix_tokens) {
            let remainder = trim_lexed_commas(&tokens[..token_idx]).to_vec();
            return Ok(Some((Until::YouStopControllingThis, remainder)));
        }
    }

    if SEARCH_THIS_TURN_PATTERN.matches_words(&token_word_refs(tokens)) {
        let cleaned = remove_this_turn_tokens(tokens);
        let remainder = trim_lexed_commas(&cleaned).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((Until::EndOfTurn, remainder)));
        }
    }

    Ok(None)
}

#[allow(dead_code)]
pub(crate) fn extract_search_library_mana_constraint(
    filter_tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, SearchLibraryManaConstraint)> {
    let (clause_token_start, clause_token_end) =
        find_token_word_sequence_span(filter_tokens, &["with", "mana", "cost"])
            .or_else(|| find_token_word_sequence_span(filter_tokens, &["with", "mana", "value"]))?;
    let base_filter_tokens = trim_commas(&filter_tokens[..clause_token_start]);
    if base_filter_tokens.is_empty() {
        return None;
    }

    let clause_tokens = trim_lexed_commas(&filter_tokens[clause_token_end..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let parse_single_u32_clause = |tokens: &[OwnedLexToken]| -> Option<u32> {
        let (value, used) = parse_number(tokens)?;
        (used == tokens.len()).then_some(value)
    };
    let constraint = if let Some(value) = parse_single_u32_clause(clause_tokens) {
        SearchLibraryManaConstraint::Equal(value)
    } else if let Some((operator, value_tokens)) = parse_value_comparison_tokens(clause_tokens) {
        let value = parse_single_u32_clause(value_tokens)?;
        match operator {
            crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                SearchLibraryManaConstraint::LessThanOrEqual(value)
            }
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                SearchLibraryManaConstraint::GreaterThanOrEqual(value)
            }
            _ => return None,
        }
    } else {
        let [left, middle, right] = clause_tokens else {
            return None;
        };
        if !SEARCH_OR_WORD_PATTERN.matches_token(middle) {
            return None;
        }
        SearchLibraryManaConstraint::OneOf(vec![
            parse_single_u32_clause(std::slice::from_ref(left))?,
            parse_single_u32_clause(std::slice::from_ref(right))?,
        ])
    };

    Some((base_filter_tokens, constraint))
}

#[allow(dead_code)]
pub(crate) fn apply_search_library_mana_constraint(
    filter: &mut ObjectFilter,
    constraint: SearchLibraryManaConstraint,
) {
    if !filter.any_of.is_empty() {
        for nested in &mut filter.any_of {
            apply_search_library_mana_constraint(nested, constraint.clone());
        }
        return;
    }

    let build_branch = |base: &ObjectFilter, mana_value: crate::filter::Comparison| {
        let mut branch = base.clone();
        branch.has_mana_cost = true;
        branch.no_x_in_cost = true;
        branch.mana_value = Some(mana_value);
        branch
    };

    match constraint {
        SearchLibraryManaConstraint::Equal(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::Equal(value as i32));
        }
        SearchLibraryManaConstraint::LessThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::GreaterThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::GreaterThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::OneOf(values) => {
            let base = filter.clone();
            *filter = ObjectFilter::default();
            filter.any_of = values
                .into_iter()
                .map(|value| build_branch(&base, crate::filter::Comparison::Equal(value as i32)))
                .collect();
        }
    }
}

#[allow(dead_code)]
pub(crate) fn split_search_same_name_reference_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let (start_token_idx, end_token_idx) =
        find_token_word_sequence_span(tokens, &["with", "the", "same", "name", "as"])
            .or_else(|| find_token_word_sequence_span(tokens, &["with", "same", "name", "as"]))?;
    let base_filter_tokens = trim_commas(&tokens[..start_token_idx]);
    let reference_tokens = trim_commas(&tokens[end_token_idx..]);
    Some((base_filter_tokens, reference_tokens))
}

#[allow(dead_code)]
pub(crate) fn is_same_name_that_reference_words(words: &[&str]) -> bool {
    matches!(
        words,
        ["that", "card"]
            | ["that", "cards"]
            | ["that", "creature"]
            | ["that", "creatures"]
            | ["that", "artifact"]
            | ["that", "artifacts"]
            | ["that", "enchantment"]
            | ["that", "enchantments"]
            | ["that", "land"]
            | ["that", "lands"]
            | ["that", "permanent"]
            | ["that", "permanents"]
            | ["that", "spell"]
            | ["that", "spells"]
            | ["that", "object"]
            | ["that", "objects"]
            | ["those", "cards"]
            | ["those", "creatures"]
            | ["those", "artifacts"]
            | ["those", "enchantments"]
            | ["those", "lands"]
            | ["those", "permanents"]
            | ["those", "spells"]
            | ["those", "objects"]
    )
}

pub(crate) fn normalize_search_library_filter(filter: &mut ObjectFilter) {
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
                | Subtype::Desert
        )
    }) && !card_type_slice_contains(&filter.card_types, CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    for nested in &mut filter.any_of {
        normalize_search_library_filter(nested);
    }
}

pub(crate) fn parse_shuffle_graveyard_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let trimmed_tokens = trim_commas(tokens);
    let clause_tokens = strip_leading_token_words_any(&trimmed_tokens, &["then", "and"]).to_vec();
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = token_word_refs(&clause_tokens);
    if !clause_words
        .iter()
        .any(|word| SEARCH_SHUFFLE_WORD_PATTERN.matches_word(word))
        || !grammar::contains_word(&clause_tokens, "graveyard")
        || !grammar::contains_word(&clause_tokens, "library")
    {
        return Ok(None);
    }

    let Some(shuffle_idx) = find_token_index(&clause_tokens, |token| {
        SEARCH_SHUFFLE_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    // Keep this primitive focused on shuffle-led clauses so we don't swallow
    // earlier effects in chains like "... then shuffle your graveyard ...".
    if shuffle_idx > 3 {
        return Ok(None);
    }

    let mut subject_tokens = trim_commas(&clause_tokens[..shuffle_idx]);
    let optional_shuffle = subject_tokens
        .last()
        .is_some_and(|token| SEARCH_MAY_WORD_PATTERN.matches_token(token));
    if optional_shuffle {
        subject_tokens.pop();
    }
    let each_player_subject = {
        let subject_words = token_word_refs(&subject_tokens);
        SEARCH_EACH_PLAYER_PREFIX_PATTERN.matches_words(&subject_words)
    };
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else if each_player_subject {
        SubjectAst::Player(PlayerAst::Implicit)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let body_tokens = trim_commas(&clause_tokens[shuffle_idx + 1..]);
    if body_tokens.is_empty() {
        return Ok(None);
    }

    let Some(into_idx) = find_token_index(&body_tokens, |token| {
        SEARCH_INTO_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    if into_idx == 0 {
        return Ok(None);
    }

    let destination_tokens = trim_commas(&body_tokens[into_idx + 1..]);
    let destination_words = token_word_refs(&destination_tokens);
    if !grammar::contains_word(&destination_tokens, "library") {
        return Ok(None);
    }
    let owner_library_destination = destination_words
        .iter()
        .any(|word| word_has_fragment(word, "owner"));
    let trailing_tokens = find_token_index(&destination_tokens, |token| {
        SEARCH_LIBRARY_WORD_PATTERN.matches_token(token)
    })
    .map(|idx| trim_commas(&destination_tokens[idx + 1..]).to_vec())
    .unwrap_or_default();
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
            if each_player_subject {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player(effect, CarryContext::ForEachPlayer);
                }
            } else {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player_with_clause(
                        effect,
                        CarryContext::Player(player),
                        &trailing_tokens,
                    );
                }
            }
            effects.extend(trailing_effects);
            Ok(Some(effects))
        };
    let wrap_optional = |effects: Vec<EffectAst>| -> Vec<EffectAst> {
        if optional_shuffle {
            vec![EffectAst::MayByPlayer { player, effects }]
        } else {
            effects
        }
    };

    let target_tokens = trim_commas(&body_tokens[..into_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target_words = token_word_refs(&target_tokens);
    if !grammar::contains_word(&target_tokens, "graveyard") {
        return Ok(None);
    }

    let has_target_selector = grammar::contains_word(&target_tokens, "target");
    if !has_target_selector {
        let mut effects = Vec::new();
        let has_source_and_graveyard_clause =
            SEARCH_SOURCE_AND_GRAVEYARD_PREFIX_PATTERN.matches_words(&target_words);
        let has_hand_clause = grammar::contains_word(&target_tokens, "hand");
        if has_source_and_graveyard_clause {
            effects.push(EffectAst::subject_verb_move_to_zone(
                TargetAst::Source(None),
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
            if owner_library_destination {
                effects.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ));
            }
        } else if has_hand_clause {
            effects.push(EffectAst::subject_verb_shuffle_hand_and_graveyard_into_library(player));
        } else {
            effects.push(EffectAst::subject_verb_shuffle_graveyard_into_library(
                player,
            ));
        }
        if each_player_subject {
            return append_trailing(vec![EffectAst::ForEachPlayer {
                effects: wrap_optional(effects),
            }]);
        }
        return append_trailing(wrap_optional(effects));
    }

    let mut target = parse_target_phrase(&target_tokens)?;
    apply_shuffle_subject_graveyard_owner_context(&mut target, subject);

    append_trailing(vec![EffectAst::subject_verb_shuffle_objects_into_library(
        player, target,
    )])
}

pub(crate) fn parse_shuffle_object_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let trimmed_tokens = trim_commas(tokens);
    let clause_tokens = strip_leading_token_words_any(&trimmed_tokens, &["then", "and"]).to_vec();
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = token_word_refs(&clause_tokens);
    if !clause_words
        .iter()
        .any(|word| SEARCH_SHUFFLE_WORD_PATTERN.matches_word(word))
        || !grammar::contains_word(&clause_tokens, "library")
        || grammar::contains_word(&clause_tokens, "graveyard")
    {
        return Ok(None);
    }

    let Some(shuffle_idx) = find_token_index(&clause_tokens, |token| {
        SEARCH_SHUFFLE_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(&clause_tokens[..shuffle_idx]);
    let owner_of_subject_target = {
        let subject_words = token_word_refs(&subject_tokens);
        if SEARCH_OWNER_OF_PREFIX_PATTERN.matches_words(&subject_words) {
            let Some(target_start) = token_index_for_word_index(&subject_tokens, 3) else {
                return Ok(None);
            };
            Some(parse_target_phrase(&subject_tokens[target_start..])?)
        } else {
            None
        }
    };
    if shuffle_idx > 3 && owner_of_subject_target.is_none() {
        return Ok(None);
    }

    let subject = if owner_of_subject_target.is_some() {
        SubjectAst::Player(PlayerAst::ItsOwner)
    } else if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let body_tokens = trim_commas(&clause_tokens[shuffle_idx + 1..]);
    let Some(into_idx) = find_token_index(&body_tokens, |token| {
        SEARCH_INTO_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    if into_idx == 0 {
        return Ok(None);
    }

    let destination_tokens = trim_commas(&body_tokens[into_idx + 1..]);
    if !grammar::contains_word(&destination_tokens, "library") {
        return Ok(None);
    }
    let trailing_tokens = find_token_index(&destination_tokens, |token| {
        SEARCH_LIBRARY_WORD_PATTERN.matches_token(token)
    })
    .map(|idx| trim_commas(&destination_tokens[idx + 1..]).to_vec())
    .unwrap_or_default();
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
            for effect in &mut trailing_effects {
                maybe_apply_carried_player_with_clause(
                    effect,
                    CarryContext::Player(player),
                    &trailing_tokens,
                );
            }
            effects.extend(trailing_effects);
            Ok(Some(effects))
        };

    let target_tokens = trim_commas(&body_tokens[..into_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target_words = token_word_refs(&target_tokens);
    if let Some(target) = owner_of_subject_target {
        if matches!(
            target_words.as_slice(),
            ["it"] | ["them"] | ["that"] | ["that", "object"] | ["that", "card"]
        ) {
            if !trailing_tokens.is_empty() {
                return append_trailing(vec![
                    EffectAst::subject_verb_move_to_zone(
                        target,
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::LibraryOwner,
                        PlayerAst::ItsOwner,
                        SubjectVerbActionAst::ShuffleLibrary,
                    ),
                ]);
            }
            return append_trailing(vec![EffectAst::subject_verb_shuffle_objects_into_library(
                PlayerAst::ItsOwner,
                target,
            )]);
        }
        return Ok(None);
    }
    if matches!(subject, SubjectAst::Player(PlayerAst::ItsOwner))
        && matches!(
            target_words.as_slice(),
            ["them"] | ["those", "cards"] | ["those", "objects"] | ["those"]
        )
    {
        return append_trailing(vec![EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
                    Zone::Library,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ),
            ],
        }]);
    }
    let target = parse_target_phrase(&target_tokens)?;

    append_trailing(vec![EffectAst::subject_verb_shuffle_objects_into_library(
        player, target,
    )])
}

pub(crate) fn parse_exile_hand_and_graveyard_bundle_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let trimmed_tokens = trim_commas(tokens);
    let clause_tokens = strip_leading_token_words_any(&trimmed_tokens, &["then", "and"]).to_vec();
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    if grammar::words_match_prefix(&clause_tokens, &["exile", "all", "cards", "from"]).is_none() {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "hand")
        && !grammar::contains_word(&clause_tokens, "hands")
    {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "graveyard")
        && !grammar::contains_word(&clause_tokens, "graveyards")
    {
        return Ok(None);
    }
    let clause_words = token_word_refs(&clause_tokens);

    let first_zone_idx = SEARCH_HAND_OR_GRAVEYARD_ZONE_WORD_PATTERN
        .find_word(&clause_words)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing zone in exile hand+graveyard clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    if first_zone_idx <= 4 {
        return Ok(None);
    }

    let owner_words = possessive_normalized_word_refs(&clause_words[4..first_zone_idx]);
    if !SEARCH_TARGET_OR_YOUR_OWNER_PATTERN.matches_words(&owner_words) {
        return Ok(None);
    }
    let owner = if SEARCH_TARGET_PLAYER_PREFIX_PATTERN.matches_words(&owner_words) {
        PlayerFilter::target_player()
    } else if SEARCH_TARGET_OPPONENT_PREFIX_PATTERN.matches_words(&owner_words) {
        PlayerFilter::target_opponent()
    } else {
        PlayerFilter::You
    };

    let Some(first_zone) = parse_zone_word(clause_words[first_zone_idx]) else {
        return Ok(None);
    };
    if !matches!(first_zone, Zone::Hand | Zone::Graveyard) {
        return Ok(None);
    }

    let Some(and_word) = clause_words.get(first_zone_idx + 1) else {
        return Ok(None);
    };
    if !SEARCH_AND_WORD_PATTERN.matches_word(and_word) {
        return Ok(None);
    }

    let mut second_zone_idx = first_zone_idx + 2;
    while clause_words
        .get(second_zone_idx)
        .is_some_and(|word| SEARCH_SECOND_ZONE_SKIP_WORD_PATTERN.matches_word(word))
    {
        second_zone_idx += 1;
    }
    let Some(second_zone_word) = clause_words.get(second_zone_idx) else {
        return Ok(None);
    };
    if clause_words.len() != second_zone_idx + 1 {
        return Ok(None);
    }
    let Some(second_zone) = parse_zone_word(second_zone_word) else {
        return Ok(None);
    };
    if !matches!(second_zone, Zone::Hand | Zone::Graveyard) || second_zone == first_zone {
        return Ok(None);
    }

    let mut first_filter = ObjectFilter::default().in_zone(first_zone);
    first_filter.owner = Some(owner.clone());
    let mut second_filter = ObjectFilter::default().in_zone(second_zone);
    second_filter.owner = Some(owner);

    Ok(Some(vec![
        EffectAst::subject_verb_exile_all(first_filter, false),
        EffectAst::subject_verb_exile_all(second_filter, false),
    ]))
}

pub(crate) fn parse_target_player_exiles_creature_and_graveyard_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_tokens = trim_commas(tokens);
    let clause_words = token_word_refs(&clause_tokens);
    if clause_words.len() < 8 {
        return Ok(None);
    }

    let (subject_player, subject_filter) =
        if SEARCH_TARGET_OPPONENT_PREFIX_PATTERN.matches_words(&clause_words) {
            (PlayerAst::TargetOpponent, PlayerFilter::target_opponent())
        } else if SEARCH_TARGET_PLAYER_PREFIX_PATTERN.matches_words(&clause_words) {
            (PlayerAst::Target, PlayerFilter::target_player())
        } else {
            return Ok(None);
        };

    let verb_idx = 2usize;
    if !clause_words
        .get(verb_idx)
        .is_some_and(|word| SEARCH_EXILE_OR_EXILES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let tail_words = &clause_words[verb_idx + 1..];
    let Some(and_idx) = SEARCH_AND_WORD_PATTERN.find_word(tail_words) else {
        return Ok(None);
    };
    let creature_words = &tail_words[..and_idx];
    let graveyard_words = &tail_words[and_idx + 1..];

    if !SEARCH_THEIR_GRAVEYARD_PATTERN.matches_words(graveyard_words) {
        return Ok(None);
    }

    let creature_words = if creature_words.first().is_some_and(|word| is_article(word)) {
        &creature_words[1..]
    } else {
        creature_words
    };
    let creature_clause_matches =
        SEARCH_CREATURE_CONTROLLED_BY_PLAYER_PATTERN.matches_words(creature_words);
    if !creature_clause_matches {
        return Ok(None);
    }

    let mut creature_filter = ObjectFilter::creature();
    creature_filter.controller = Some(subject_filter.clone());

    let mut graveyard_filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    graveyard_filter.owner = Some(subject_filter);

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: creature_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: subject_player,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(TagKey::from(IT_TAG), None), false),
        EffectAst::subject_verb_exile_all(graveyard_filter, false),
    ]))
}

pub(crate) fn parse_for_each_exiled_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["for", "each"]).is_none() {
        return Ok(None);
    }
    let words_all = token_word_refs(tokens);
    let refers_to_exiled = SEARCH_EXILED_THIS_WAY_PATTERN.matches_words(&words_all)
        || grammar::words_match_prefix(tokens, &["for", "each", "of", "those", "creatures"])
            .is_some();
    if !refers_to_exiled {
        return Ok(None);
    }
    if grammar::words_match_prefix(
        tokens,
        &["for", "each", "permanent", "exiled", "this", "way"],
    )
    .is_some()
        && grammar::contains_word(tokens, "shares")
        && grammar::contains_word(tokens, "card")
        && grammar::contains_word(tokens, "type")
        && grammar::contains_word(tokens, "library")
        && grammar::contains_word(tokens, "battlefield")
    {
        let filter_tokens = lex_line("a permanent that shares a card type with it", 0)?;
        let filter = parse_object_filter_lexed(&filter_tokens, false)?;
        let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
        let matched_tag = helper_tag_for_tokens(tokens, "chosen");

        return Ok(Some(vec![EffectAst::ForEachTagged {
            tag: IT_TAG.into(),
            effects: vec![
                EffectAst::subject_verb_consult_top_of_library(
                    PlayerAst::Implicit,
                    LibraryConsultModeAst::Reveal,
                    filter,
                    LibraryConsultStopRuleAst::FirstMatch,
                    revealed_tag.clone(),
                    matched_tag.clone(),
                ),
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(matched_tag.clone(), None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    revealed_tag,
                    Some(matched_tag),
                    LibraryBottomOrderAst::Random,
                    PlayerAst::Implicit,
                ),
            ],
        }]));
    }

    let (_before, after_comma) = grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        ))
    })?;
    let effect_tokens = trim_commas(after_comma);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    let effect_clause = LexedClause::new(&effect_tokens);
    if let Some(after_prefix) =
        effect_clause.strip_prefix_clause(SEARCH_REVEAL_UNTIL_CONTROLLER_REVEALS_PREFIX)
        && let Some((filter_clause, put_tail)) =
            after_prefix.split_once_before_phrase(SEARCH_PUTS_THAT_CARD_ONTO_BATTLEFIELD_PHRASE)
        && put_tail.contains_phrase(SEARCH_THEN_SHUFFLES_PHRASE)
    {
        let reveal_filter_tokens = filter_clause.trimmed_tokens();
        if !reveal_filter_tokens.is_empty() {
            let filter = parse_object_filter_lexed(reveal_filter_tokens, false)?;
            let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
            let matched_tag = helper_tag_for_tokens(tokens, "chosen");

            return Ok(Some(vec![EffectAst::ForEachTagged {
                tag: IT_TAG.into(),
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::ItsController,
                        LibraryConsultModeAst::Reveal,
                        filter,
                        LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag,
                        matched_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(matched_tag, None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::LibraryOwner,
                        PlayerAst::ItsController,
                        SubjectVerbActionAst::ShuffleLibrary,
                    ),
                ],
            }]));
        }
    }
    if let Some(after_prefix) =
        effect_clause.strip_prefix_clause(SEARCH_REVEAL_UNTIL_CONTROLLER_REVEALS_PREFIX)
        && let Some((filter_clause, put_tail)) =
            after_prefix.split_once_before_phrase(SEARCH_PUTS_THAT_CARD_ONTO_BATTLEFIELD_PHRASE)
        && put_tail.contains_phrase(SEARCH_THEN_PUTS_REST_BOTTOM_PHRASE)
    {
        let reveal_filter_tokens = filter_clause.trimmed_tokens();
        if !reveal_filter_tokens.is_empty() {
            let filter = parse_object_filter_lexed(reveal_filter_tokens, false)?;
            let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
            let matched_tag = helper_tag_for_tokens(tokens, "chosen");

            return Ok(Some(vec![EffectAst::ForEachTagged {
                tag: IT_TAG.into(),
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::ItsController,
                        LibraryConsultModeAst::Reveal,
                        filter,
                        LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        matched_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(matched_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                        revealed_tag,
                        Some(matched_tag),
                        LibraryBottomOrderAst::Random,
                        PlayerAst::ItsController,
                    ),
                ],
            }]));
        }
    }
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_each_player_put_permanent_cards_exiled_with_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let starts_with_each_player_turns_face_up = grammar::words_match_prefix(
        tokens,
        &["each", "player", "turns", "face", "up", "all", "cards"],
    )
    .is_some();
    if !starts_with_each_player_turns_face_up {
        return Ok(None);
    }
    let token_words = token_word_refs(tokens);
    let has_exiled_with_this = SEARCH_EXILED_WITH_THIS_PATTERN.matches_words(&token_words);
    if !has_exiled_with_this {
        return Ok(None);
    }
    let has_puts_all_permanent_cards =
        SEARCH_THEN_PUTS_ALL_PERMANENT_CARDS_PATTERN.matches_words(&token_words);
    let has_among_them_onto_battlefield =
        SEARCH_AMONG_THEM_ONTO_BATTLEFIELD_PATTERN.matches_words(&token_words);
    if !has_puts_all_permanent_cards || !has_among_them_onto_battlefield {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![EffectAst::subject_verb_return_all_to_battlefield(
            filter,
            false,
            false,
            ReturnControllerAst::Owner,
        )],
    }]))
}

pub(crate) fn parse_for_each_destroyed_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["for", "each"]).is_none() {
        return Ok(None);
    }
    let words_all = token_word_refs(tokens);
    let refers_to_destroyed = SEARCH_DESTROYED_THIS_WAY_PATTERN.matches_words(&words_all);
    let refers_to_died = SEARCH_DIED_THIS_WAY_PATTERN.matches_words(&words_all);
    if !refers_to_destroyed && !refers_to_died {
        return Ok(None);
    }

    let (_before, after_comma) = grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        ))
    })?;
    let effect_tokens = trim_commas(after_comma);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_for_each_put_into_graveyard_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["for", "each"]).is_none() {
        return Ok(None);
    }
    let token_words = token_word_refs(tokens);
    let refers_to_graveyard =
        SEARCH_PUT_INTO_GRAVEYARD_THIS_WAY_PATTERN.matches_words(&token_words);
    if !refers_to_graveyard {
        return Ok(None);
    }

    let (_before, after_comma) = grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    let effect_tokens = trim_commas(after_comma);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_earthbend_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if !words
        .first()
        .is_some_and(|word| SEARCH_EARTHBEND_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let count = parse_number(tokens.get(1..).unwrap_or_default())
        .map(|(value, _)| value)
        .or_else(|| words.get(1).and_then(|word| parse_number_word_u32(word)))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing earthbend count (clause: '{}')",
                words.join(" ")
            ))
        })?;

    Ok(Some(EffectAst::subject_verb_earthbend(count)))
}

pub(crate) fn parse_enchant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if words.is_empty() || !SEARCH_ENCHANT_WORD_PATTERN.matches_words(&words[..1]) {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = match words.get(1..) {
        Some(["player"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::Any),
        Some(["opponent"]) | Some(["an", "opponent"]) => {
            crate::object::AuraAttachmentFilter::Player(PlayerFilter::Opponent)
        }
        Some(["you"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::You),
        _ => crate::object::AuraAttachmentFilter::Object(parse_object_filter(remaining, false)?),
    };
    Ok(Some(EffectAst::subject_verb_enchant(filter)))
}

pub(crate) fn parse_restriction_duration(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    parse_restriction_duration_lexed(tokens)
}
