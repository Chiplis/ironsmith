use crate::cards::builders::{PlayerAst, PredicateAst, PlayerPredicateAst, TurnEventPredicateAst};
use winnow::combinator::{alt, eof};
use winnow::prelude::*;

use crate::ability::ActivationTiming;
use crate::color::{Color, ColorSet};
use crate::target::{ObjectFilter, PlayerFilter};

use super::super::super::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::conditions::{
    ControlConditionOptions, parse_control_condition, parse_control_relation_tail_clause,
};
use super::super::{leaf, primitives};
use super::surface::{
    matches_any_prefix_tokens, matches_exact_tokens, matches_prefix_tokens, parse_phrase_words,
    phrase_offset_words,
};
use crate::grammar::shared_util::value_semantics::parse_filter_comparison_tokens;
use crate::util::{parse_less_than_or_equal_quantity_prefix, parse_number};

const ACTIVATE_ONLY_RESTRICTION_PREFIXES: &[&[&str]] =
    &[&["activate", "only"], &["activate", "no", "more", "than"]];
const ACTIVATE_ONLY_INSTANT_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "as", "an", "instant"],
    &["activate", "only", "as", "instant"],
];
const ACTIVATE_ONLY_SORCERY_PREFIXES: &[&[&str]] = &[&["activate", "only", "as", "a", "sorcery"]];
const DURING_OPPONENTS_TURN_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "during", "an", "opponents", "turn"],
    &["activate", "only", "during", "opponents", "turn"],
];
const ANY_PLAYER_DURING_THEIR_TURN_BEFORE_END_STEP: &[&str] = &[
    "any", "player", "may", "activate", "this", "ability", "but", "only", "during", "their",
    "turn", "before", "the", "end", "step",
];
const THIS_ABILITY_TRIGGERS_ONLY_PREFIXES: &[&[&str]] = &[
    &["this", "ability", "triggers", "only"],
    &["do", "this", "only"],
];
const TIMES_EACH_TURN_TAILS: &[&[&str]] = &[
    &["time", "each", "turn"],
    &["times", "each", "turn"],
    &["each", "turn"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivateOnlyTimingMarker {
    OnceEachTurn,
    DuringCombat,
    DuringYourTurn,
    DuringOpponentsTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CountEachTurnShape {
    count_start: usize,
    count_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GraveyardConditionShape<'a> {
    descriptor_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TotalPowerConditionShape<'a> {
    comparison_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourcesDamageConditionShape<'a> {
    source_tokens: &'a [OwnedLexToken],
    threshold_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ControlledCreaturePowerShape<'a> {
    object_tokens: &'a [OwnedLexToken],
    comparison_tokens: &'a [OwnedLexToken],
}

pub fn parse_activate_only_timing_lexed(tokens: &[OwnedLexToken]) -> Option<ActivationTiming> {
    if matches_exact_tokens(tokens, ANY_PLAYER_DURING_THEIR_TURN_BEFORE_END_STEP) {
        return Some(ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep);
    }
    let marker = parse_activate_only_timing_marker(tokens);
    if matches_any_prefix_tokens(tokens, ACTIVATE_ONLY_SORCERY_PREFIXES) {
        return Some(ActivationTiming::SorcerySpeed);
    }
    if matches_prefix_tokens(tokens, &["activate", "only", "once", "each", "turn"])
        || marker == Some(ActivateOnlyTimingMarker::OnceEachTurn)
    {
        return Some(ActivationTiming::OncePerTurn);
    }
    if matches_prefix_tokens(tokens, &["activate", "only", "during", "combat"])
        || marker == Some(ActivateOnlyTimingMarker::DuringCombat)
    {
        return Some(ActivationTiming::DuringCombat);
    }
    if matches_prefix_tokens(tokens, &["activate", "only", "during", "your", "turn"])
        || marker == Some(ActivateOnlyTimingMarker::DuringYourTurn)
    {
        return Some(ActivationTiming::DuringYourTurn);
    }
    if matches_any_prefix_tokens(tokens, DURING_OPPONENTS_TURN_PREFIXES)
        || marker == Some(ActivateOnlyTimingMarker::DuringOpponentsTurn)
    {
        return Some(ActivationTiming::DuringOpponentsTurn);
    }
    None
}

pub fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_prefix_tokens(tokens, ACTIVATE_ONLY_RESTRICTION_PREFIXES)
        || matches_exact_tokens(tokens, ANY_PLAYER_DURING_THEIR_TURN_BEFORE_END_STEP)
}

pub fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_prefix_tokens(
        tokens,
        &["any", "player", "may", "activate", "this", "ability"],
    )
}

pub fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_prefix_tokens(tokens, THIS_ABILITY_TRIGGERS_ONLY_PREFIXES)
}

pub fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(
        &mut input,
        alt((
            |input: &mut primitives::WordSliceInput<'_>| {
                parse_phrase_words(input, &["this", "ability", "triggers", "only"])
            },
            |input: &mut primitives::WordSliceInput<'_>| {
                parse_phrase_words(input, &["do", "this", "only"])
            },
        )),
    )?;
    parse_activation_count_per_turn(input)
}

pub fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    parse_triggered_times_each_turn_from_words(&TokenWordView::new(tokens).word_refs())
}

pub fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    let shape = parse_count_each_turn_shape(words, 0)?;
    let count_words = words.get(shape.count_start..shape.count_end)?;
    let parsed = leaf::parse_leaf_number_prefix_words(count_words)?.into_fixed()?;
    (parsed.1 == count_words.len()).then_some(parsed.0)
}

use crate::recognition::ParseOutcome;
#[path = "activation_conditions/condition_readings.rs"]
mod condition_readings;

pub fn parse_activation_condition_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let input = condition_readings::ActivationCondition {
        tokens,
        read_by_cache: Default::default(),
    };
    match condition_readings::read(&input) {
        ParseOutcome::Match(matched) => return Some(matched.value.value),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(_) => return None,
    }
    let control_tokens = parse_activate_only_if_you_control_tail_tokens(tokens)?;
    if let Some(control_condition) =
        parse_control_condition(control_tokens, ControlConditionOptions::default())
    {
        let count = control_condition.at_least_count()?;
        // The resolved filter is still required as a guard: a subject with no
        // filter ("that player") is not one this clause accepts. What travels
        // on is the recognized player, not the binding.
        let _ = control_condition.player_filter?;
        let player = control_condition.player;
        let mut filter = control_condition.filter;
        // Activation-condition ASTs historically represented adjacent type
        // words in the union field. Keep that established shape while the
        // shared object-filter grammar retains its stricter intersection
        // representation for ordinary object selection.
        if filter.card_types.is_empty() && !filter.all_card_types.is_empty() {
            filter.card_types = std::mem::take(&mut filter.all_card_types);
        }
        if count == 1
            && player == PlayerAst::You
            && crate::slice_primitives::contains(&filter.card_types, &crate::types::CardType::Land)
            && crate::slice_primitives::contains(
                &filter.supertypes,
                &crate::types::Supertype::Basic,
            )
        {
            return Some(PredicateAst::YouControl(filter));
        }
        return Some(PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeast {
            player,
            filter,
            count,
        }));
    }
    parse_land_subtype_control_condition(control_tokens)
}

fn parse_combined_once_and_timing_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = TokenWordView::new(tokens).word_refs();
    phrase_offset_words(&words, &["once", "each", "turn"])?;
    let timing = if phrase_offset_words(&words, &["during", "your", "turn"]).is_some() {
        ActivationTiming::DuringYourTurn
    } else if phrase_offset_words(&words, &["during", "combat"]).is_some() {
        ActivationTiming::DuringCombat
    } else if phrase_offset_words(&words, &["during", "an", "opponents", "turn"]).is_some()
        || phrase_offset_words(&words, &["during", "opponents", "turn"]).is_some()
    {
        ActivationTiming::DuringOpponentsTurn
    } else {
        return None;
    };
    Some(PredicateAst::And(
        Box::new(PredicateAst::MaxActivationsPerTurn(1)),
        Box::new(PredicateAst::ActivationTiming(timing)),
    ))
}

fn parse_repeated_or_if_activation_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let split = phrase_offset_words(&words, &["or", "if"])?;
    if split == 0 || split + 2 >= words.len() {
        return None;
    }

    let left_tokens = token_slice_for_words(tokens, &view, 0, split)?;
    let right_tokens = token_slice_for_words(tokens, &view, split + 2, words.len())?;
    let left = parse_activation_condition_lexed(left_tokens)?;

    let mut prefixed_right = vec![
        OwnedLexToken::synthetic_word("activate"),
        OwnedLexToken::synthetic_word("only"),
        OwnedLexToken::synthetic_word("if"),
    ];
    prefixed_right.extend_from_slice(right_tokens);
    let right = parse_activation_condition_lexed(&prefixed_right)?;

    Some(PredicateAst::Or(Box::new(left), Box::new(right)))
}

fn parse_once_each_turn_and_if_activation_condition(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let split = phrase_offset_words(&words, &["and", "only", "if"])?;
    if split == 0 || split + 3 >= words.len() {
        return None;
    }
    let left = words.get(..split)?;
    if !matches!(
        left,
        ["activate", "only", "once", "each", "turn"]
            | [
                "activate", "this", "ability", "only", "once", "each", "turn"
            ]
    ) {
        return None;
    }

    let right_tokens = token_slice_for_words(tokens, &view, split + 3, words.len())?;
    let mut prefixed_right = vec![
        OwnedLexToken::synthetic_word("activate"),
        OwnedLexToken::synthetic_word("only"),
        OwnedLexToken::synthetic_word("if"),
    ];
    prefixed_right.extend_from_slice(right_tokens);
    let right = parse_activation_condition_lexed(&prefixed_right)?;
    Some(PredicateAst::And(
        Box::new(PredicateAst::MaxActivationsPerTurn(1)),
        Box::new(right),
    ))
}

fn parse_source_entered_this_turn_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition_tokens = parse_activate_only_if_tail_tokens(tokens)?;
    let words = TokenWordView::new(condition_tokens).word_refs();
    let source_end =
        if crate::word_primitives::parse_sequence_suffix(&words, &["entered", "this", "turn"]) {
            words.len().checked_sub(3)?
        } else if crate::word_primitives::parse_sequence_suffix(
            &words,
            &["entered", "the", "battlefield", "this", "turn"],
        ) {
            words.len().checked_sub(5)?
        } else {
            return None;
        };
    let source_words = words.get(..source_end)?;
    let filter = if crate::word_primitives::parse_sequence_complete(source_words, &["it"]) {
        ObjectFilter::source()
    } else {
        ObjectFilter::source_with_surface(leaf::parse_leaf_this_source_reference_words(
            source_words,
        )?)
    };
    Some(PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectEnteredBattlefieldThisTurn(filter)))
}

fn parse_activate_only_timing_marker(tokens: &[OwnedLexToken]) -> Option<ActivateOnlyTimingMarker> {
    let words = TokenWordView::new(tokens).word_refs();
    for (phrase, marker) in [
        (
            &["once", "each", "turn"][..],
            ActivateOnlyTimingMarker::OnceEachTurn,
        ),
        (
            &["during", "combat"][..],
            ActivateOnlyTimingMarker::DuringCombat,
        ),
        (
            &["during", "your", "turn"][..],
            ActivateOnlyTimingMarker::DuringYourTurn,
        ),
        (
            &["during", "an", "opponents", "turn"][..],
            ActivateOnlyTimingMarker::DuringOpponentsTurn,
        ),
        (
            &["during", "opponents", "turn"][..],
            ActivateOnlyTimingMarker::DuringOpponentsTurn,
        ),
    ] {
        if phrase_offset_words(&words, phrase).is_some() {
            return Some(marker);
        }
    }
    None
}

fn parse_count_each_turn_shape(words: &[&str], count_start: usize) -> Option<CountEachTurnShape> {
    let tail_words = words.get(count_start..)?;
    let tail_offset = exact_tail_offset(tail_words, TIMES_EACH_TURN_TAILS)?;
    (tail_offset > 0).then_some(CountEachTurnShape {
        count_start,
        count_end: count_start + tail_offset,
    })
}

fn parse_graveyard_condition_shape(
    tokens: &[OwnedLexToken],
) -> Option<GraveyardConditionShape<'_>> {
    const TAILS: &[&[&str]] = &[
        &["in", "your", "graveyard"],
        &["in", "graveyard"],
        &["in", "the", "graveyard"],
    ];
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    crate::grammar::primitives::take_leaf(
        &mut input,
        alt((
            |input: &mut primitives::WordSliceInput<'_>| {
                parse_phrase_words(input, &["activate", "only", "if", "there", "is"])
            },
            |input: &mut primitives::WordSliceInput<'_>| {
                parse_phrase_words(input, &["activate", "only", "if", "there", "are"])
            },
        )),
    )?;
    let start = words.len().checked_sub(input.len())?;
    let end = start + exact_tail_offset(input, TAILS)?;
    (end > start).then_some(GraveyardConditionShape {
        descriptor_tokens: token_slice_for_words(tokens, &view, start, end)?,
    })
}

fn parse_graveyard_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let parsed = parse_graveyard_condition_shape(tokens)?;
    let descriptor_words = TokenWordView::new(parsed.descriptor_tokens).word_refs();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for word in &descriptor_words {
        if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
        }
        if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
        }
    }
    if card_types.is_empty() && subtypes.is_empty() {
        let (count, used) =
            leaf::parse_leaf_number_prefix_words(&descriptor_words)?.into_fixed()?;
        if descriptor_words.get(used..) != Some(&["or", "more", "cards"][..]) {
            return None;
        }
        return Some(PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeast {
            player: PlayerAst::You,
            filter: ObjectFilter::default()
                .in_zone(crate::zone::Zone::Graveyard)
                .owned_by(PlayerFilter::You),
            count,
        }));
    }
    Some(PredicateAst::CardInYourGraveyard {
        card_types,
        subtypes,
    })
}

fn parse_total_power_condition_shape(
    tokens: &[OwnedLexToken],
) -> Option<TotalPowerConditionShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_phrase_words(
            input,
            &[
                "activate",
                "only",
                "if",
                "creatures",
                "you",
                "control",
                "have",
                "total",
                "power",
            ],
        )
    })?;
    let start = words.len().checked_sub(input.len())?;
    (start < words.len()).then_some(TotalPowerConditionShape {
        comparison_tokens: token_slice_for_words(tokens, &view, start, words.len())?,
    })
}

fn parse_total_power_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let parsed = parse_total_power_condition_shape(tokens)?;
    let comparison_words = TokenWordView::new(parsed.comparison_tokens).word_refs();
    let clause_words = TokenWordView::new(tokens).word_refs();
    let (comparison, used) = crate::grammar::primitives::probe_shape(
        parse_filter_comparison_tokens("power", &comparison_words, &clause_words),
    )??;
    let crate::filter::Comparison::GreaterThanOrEqual(threshold) = comparison else {
        return None;
    };
    (used == comparison_words.len()).then_some(PredicateAst::ControlCreaturesTotalPowerAtLeast(
        crate::util::narrowed_u32(threshold)?,
    ))
}

fn parse_sources_damage_condition_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourcesDamageConditionShape<'_>> {
    const TAILS: &[&[&str]] = &[
        &["or", "more", "noncombat", "damage", "this", "turn"],
        &[
            "or",
            "more",
            "noncombat",
            "damage",
            "this",
            "turn",
            "and",
            "only",
            "as",
            "a",
            "sorcery",
        ],
    ];
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_phrase_words(input, &["activate", "only", "if"])
    })?;
    let body_start = words.len().checked_sub(input.len())?;
    let dealt = phrase_offset_words(input, &["dealt"])?;
    let source_end = body_start + dealt;
    let threshold_start = source_end + 1;
    let threshold_end = threshold_start + exact_tail_offset(words.get(threshold_start..)?, TAILS)?;
    if source_end <= body_start || threshold_end <= threshold_start {
        return None;
    }
    Some(SourcesDamageConditionShape {
        source_tokens: token_slice_for_words(tokens, &view, body_start, source_end)?,
        threshold_tokens: token_slice_for_words(tokens, &view, threshold_start, threshold_end)?,
    })
}

fn parse_sources_damage_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let parsed = parse_sources_damage_condition_shape(tokens)?;
    if !matches_exact_tokens(
        parsed.source_tokens,
        &["red", "sources", "you", "controlled"],
    ) {
        return None;
    }
    let threshold_words = TokenWordView::new(parsed.threshold_tokens).word_refs();
    let (threshold, used) = parse_number(parsed.threshold_tokens)?;
    if used != threshold_words.len() {
        return None;
    }
    Some(PredicateAst::ValueComparison {
        left: crate::effect::Value::NoncombatDamageDealtBySourcesControlledThisTurn {
            player: PlayerFilter::You,
            colors: Some(ColorSet::from_color(Color::Red)),
        },
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(threshold as i32),
    })
}

#[cfg(test)]
#[path = "activation_conditions_inline_tests.rs"]
mod tests;

#[path = "activation_conditions/object_action.rs"]
mod object_action_programs;
use object_action_programs::{
    activate_only_you_control_options, parse_activate_only_if_tail_tokens,
    parse_activate_only_if_you_control_tail_tokens, parse_controlled_creature_power_condition,
    parse_controlled_creature_power_shape, parse_land_subtype_control_condition,
    token_slice_for_words,
};
#[path = "activation_conditions/core.rs"]
mod core_programs;
use core_programs::{exact_tail_offset, matches_exact_word_slice};
#[path = "activation_conditions/condition.rs"]
mod condition_programs;
use condition_programs::{
    parse_activate_count_each_turn_condition, parse_activate_only_count_per_turn_condition,
};
