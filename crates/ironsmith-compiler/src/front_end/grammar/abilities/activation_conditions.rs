use winnow::combinator::{alt, eof};
use winnow::prelude::*;

use crate::ConditionExpr;
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

pub(crate) fn parse_activate_only_timing_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivationTiming> {
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

pub(crate) fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_prefix_tokens(tokens, ACTIVATE_ONLY_RESTRICTION_PREFIXES)
        || matches_exact_tokens(tokens, ANY_PLAYER_DURING_THEIR_TURN_BEFORE_END_STEP)
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_prefix_tokens(
        tokens,
        &["any", "player", "may", "activate", "this", "ability"],
    )
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_prefix_tokens(tokens, THIS_ABILITY_TRIGGERS_ONLY_PREFIXES)
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    let mut input: primitives::WordSliceInput<'_> = words;
    alt((
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["this", "ability", "triggers", "only"])
        },
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["do", "this", "only"])
        },
    ))
    .parse_next(&mut input)
    .ok()?;
    parse_activation_count_per_turn(input)
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    parse_triggered_times_each_turn_from_words(&TokenWordView::new(tokens).word_refs())
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    let shape = parse_count_each_turn_shape(words, 0)?;
    let count_words = words.get(shape.count_start..shape.count_end)?;
    let parsed = leaf::parse_leaf_number_prefix_words(count_words)?.into_fixed()?;
    (parsed.1 == count_words.len()).then_some(parsed.0)
}

pub(crate) fn parse_activation_condition_lexed(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    if let Some(condition) = parse_repeated_or_if_activation_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_once_each_turn_and_if_activation_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_combined_once_and_timing_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_activate_count_each_turn_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_activate_only_count_per_turn_condition(tokens) {
        return Some(condition);
    }
    if matches_any_prefix_tokens(tokens, ACTIVATE_ONLY_INSTANT_PREFIXES) {
        return Some(ConditionExpr::ActivationTiming(ActivationTiming::AnyTime));
    }
    if let Some(condition) = parse_graveyard_condition(tokens) {
        return Some(condition);
    }
    if let Some(status_tokens) = tokens.get(3..)
        && let Some(condition) =
            super::super::conditions::parse_player_status_condition(status_tokens)
        && condition.status == super::super::conditions::PlayerStatusAst::MaxSpeed
    {
        return Some(condition.condition_expr());
    }
    if let Some(condition) = parse_total_power_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_sources_damage_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_controlled_creature_power_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_source_entered_this_turn_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) =
        super::super::restriction_normalization::parse_text_only_activation_restriction_tokens(
            tokens,
        )
    {
        return Some(match condition {
            super::super::restriction_normalization::TextOnlyActivationRestriction::SourceDidNotAttackThisTurn => {
                ConditionExpr::Not(Box::new(ConditionExpr::SourceAttackedThisTurn))
            }
            super::super::restriction_normalization::TextOnlyActivationRestriction::SourceAttackedThisTurn => {
                ConditionExpr::SourceAttackedThisTurn
            }
        });
    }
    if let Some(condition_tokens) = parse_activate_only_if_tail_tokens(tokens)
        && let Ok(predicate) = super::super::filters::parse_predicate(condition_tokens)
    {
        match predicate {
            crate::cards::builders::PredicateAst::SourceHasCounterAtLeast {
                counter_type,
                count,
                surface,
            } => {
                return Some(ConditionExpr::SourceHasCounterAtLeast {
                    counter_type,
                    count,
                    surface,
                });
            }
            crate::cards::builders::PredicateAst::SourceMatches(filter) => {
                return Some(ConditionExpr::SourceMatches(filter));
            }
            _ => {}
        }
    }
    let control_tokens = parse_activate_only_if_you_control_tail_tokens(tokens)?;
    if let Some(control_condition) =
        parse_control_condition(control_tokens, ControlConditionOptions::default())
    {
        let count = control_condition.at_least_count()?;
        let player = control_condition.player_filter?;
        let mut filter = control_condition.filter;
        // Activation-condition ASTs historically represented adjacent type
        // words in the union field. Keep that established shape while the
        // shared object-filter grammar retains its stricter intersection
        // representation for ordinary object selection.
        if filter.card_types.is_empty() && !filter.all_card_types.is_empty() {
            filter.card_types = std::mem::take(&mut filter.all_card_types);
        }
        if count == 1
            && player == PlayerFilter::You
            && filter.card_types.contains(&crate::types::CardType::Land)
            && filter.supertypes.contains(&crate::types::Supertype::Basic)
        {
            return Some(ConditionExpr::YouControl(filter));
        }
        return Some(ConditionExpr::PlayerHasAtLeast {
            player,
            filter,
            count,
        });
    }
    parse_land_subtype_control_condition(control_tokens)
}

fn parse_combined_once_and_timing_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let words = TokenWordView::new(tokens).word_refs();
    if phrase_offset_words(&words, &["once", "each", "turn"]).is_none() {
        return None;
    }
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
    Some(ConditionExpr::And(
        Box::new(ConditionExpr::MaxActivationsPerTurn(1)),
        Box::new(ConditionExpr::ActivationTiming(timing)),
    ))
}

fn parse_repeated_or_if_activation_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
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

    Some(ConditionExpr::Or(Box::new(left), Box::new(right)))
}

fn parse_once_each_turn_and_if_activation_condition(
    tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
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
    Some(ConditionExpr::And(
        Box::new(ConditionExpr::MaxActivationsPerTurn(1)),
        Box::new(right),
    ))
}

fn parse_source_entered_this_turn_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let condition_tokens = parse_activate_only_if_tail_tokens(tokens)?;
    let words = TokenWordView::new(condition_tokens).word_refs();
    let source_end = if words.ends_with(&["entered", "this", "turn"]) {
        words.len().checked_sub(3)?
    } else if words.ends_with(&["entered", "the", "battlefield", "this", "turn"]) {
        words.len().checked_sub(5)?
    } else {
        return None;
    };
    let source_words = words.get(..source_end)?;
    let filter = if matches_exact_word_slice(source_words, &["it"]) {
        ObjectFilter::source()
    } else {
        ObjectFilter::source_with_surface(leaf::parse_leaf_this_source_reference_words(
            source_words,
        )?)
    };
    Some(ConditionExpr::ObjectEnteredBattlefieldThisTurn(filter))
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
    alt((
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["activate", "only", "if", "there", "is"])
        },
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["activate", "only", "if", "there", "are"])
        },
    ))
    .parse_next(&mut input)
    .ok()?;
    let start = words.len().checked_sub(input.len())?;
    let end = start + exact_tail_offset(input, TAILS)?;
    (end > start).then_some(GraveyardConditionShape {
        descriptor_tokens: token_slice_for_words(tokens, &view, start, end)?,
    })
}

fn parse_graveyard_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
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
        return Some(ConditionExpr::PlayerHasAtLeast {
            player: PlayerFilter::You,
            filter: ObjectFilter::default()
                .in_zone(crate::zone::Zone::Graveyard)
                .owned_by(PlayerFilter::You),
            count,
        });
    }
    Some(ConditionExpr::CardInYourGraveyard {
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
    parse_phrase_words(
        &mut input,
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
    .ok()?;
    let start = words.len().checked_sub(input.len())?;
    (start < words.len()).then_some(TotalPowerConditionShape {
        comparison_tokens: token_slice_for_words(tokens, &view, start, words.len())?,
    })
}

fn parse_total_power_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let parsed = parse_total_power_condition_shape(tokens)?;
    let comparison_words = TokenWordView::new(parsed.comparison_tokens).word_refs();
    let clause_words = TokenWordView::new(tokens).word_refs();
    let (comparison, used) =
        parse_filter_comparison_tokens("power", &comparison_words, &clause_words).ok()??;
    let crate::filter::Comparison::GreaterThanOrEqual(threshold) = comparison else {
        return None;
    };
    (used == comparison_words.len()).then_some(ConditionExpr::ControlCreaturesTotalPowerAtLeast(
        u32::try_from(threshold).ok()?,
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
    parse_phrase_words(&mut input, &["activate", "only", "if"]).ok()?;
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

fn parse_sources_damage_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
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
    Some(ConditionExpr::ValueComparison {
        left: crate::effect::Value::NoncombatDamageDealtBySourcesControlledThisTurn {
            player: PlayerFilter::You,
            colors: Some(ColorSet::from_color(Color::Red)),
        },
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(threshold as i32),
    })
}

fn parse_controlled_creature_power_shape(
    tokens: &[OwnedLexToken],
) -> Option<ControlledCreaturePowerShape<'_>> {
    let condition_tokens = parse_activate_only_if_you_control_tail_tokens(tokens)?;
    let tail =
        parse_control_relation_tail_clause(condition_tokens, activate_only_you_control_options())?;
    let view = TokenWordView::new(tail.tokens());
    let words = view.word_refs();
    let power = phrase_offset_words(&words, &["with", "power"])?;
    let comparison_start = power + 2;
    if power == 0 || comparison_start >= words.len() {
        return None;
    }
    Some(ControlledCreaturePowerShape {
        object_tokens: token_slice_for_words(tail.tokens(), &view, 0, power)?,
        comparison_tokens: token_slice_for_words(
            tail.tokens(),
            &view,
            comparison_start,
            words.len(),
        )?,
    })
}

fn parse_controlled_creature_power_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let parsed = parse_controlled_creature_power_shape(tokens)?;
    if !matches_exact_tokens(parsed.object_tokens, &["creature"])
        && !matches_exact_tokens(parsed.object_tokens, &["a", "creature"])
        && !matches_exact_tokens(parsed.object_tokens, &["an", "creature"])
    {
        return None;
    }
    let comparison_words = TokenWordView::new(parsed.comparison_tokens).word_refs();
    let clause_words = TokenWordView::new(tokens).word_refs();
    let (comparison, used) =
        parse_filter_comparison_tokens("power", &comparison_words, &clause_words).ok()??;
    (used == comparison_words.len()).then_some(ConditionExpr::YouControl(
        ObjectFilter::creature().with_power(comparison),
    ))
}

fn parse_activate_only_count_per_turn_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    parse_phrase_words(&mut input, &["activate", "only"]).ok()?;
    let start = words.len().checked_sub(input.len())?;
    let shape = parse_count_each_turn_shape(&words, start)?;
    let count_tokens = token_slice_for_words(tokens, &view, shape.count_start, shape.count_end)?;
    let count_words = words.get(shape.count_start..shape.count_end)?;
    let (count, used) = parse_number(count_tokens)?;
    (used == count_words.len()).then_some(ConditionExpr::MaxActivationsPerTurn(count))
}

fn parse_activate_only_if_you_control_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let condition = parse_activate_only_if_tail_tokens(tokens)?;
    parse_control_relation_tail_clause(condition, activate_only_you_control_options())?;
    Some(condition)
}

fn parse_activate_only_if_tail_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    parse_phrase_words(&mut input, &["activate", "only", "if"]).ok()?;
    let start = words.len().checked_sub(input.len())?;
    (start < words.len()).then_some(())?;
    token_slice_for_words(tokens, &view, start, words.len())
}

fn parse_land_subtype_control_condition(control_tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let object =
        parse_control_relation_tail_clause(control_tokens, activate_only_you_control_options())?;
    let mut subtypes = Vec::new();
    for word in TokenWordView::new(object.tokens()).word_refs() {
        if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
        }
    }
    if subtypes.is_empty() {
        return None;
    }
    let mut combined = None;
    for subtype in subtypes {
        let next = ConditionExpr::YouControl(
            ObjectFilter::default()
                .with_type(crate::types::CardType::Land)
                .with_subtype(subtype),
        );
        combined = Some(match combined {
            Some(existing) => ConditionExpr::Or(Box::new(existing), Box::new(next)),
            None => next,
        });
    }
    combined
}

fn parse_activate_count_each_turn_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    parse_phrase_words(&mut input, &["activate"]).ok()?;
    let start = words.len().checked_sub(input.len())?;
    let shape = parse_count_each_turn_shape(&words, start)?;
    let count_tokens = token_slice_for_words(tokens, &view, shape.count_start, shape.count_end)?;
    let count_words = words.get(shape.count_start..shape.count_end)?;
    let count = parse_less_than_or_equal_quantity_prefix(
        count_tokens,
        false,
        false,
        "activation frequency condition",
    )
    .ok()
    .flatten()
    .and_then(|(count, used)| (used == count_words.len()).then_some(count))?;
    Some(ConditionExpr::MaxActivationsPerTurn(count))
}

fn exact_tail_offset(words: &[&str], tails: &[&[&str]]) -> Option<usize> {
    let mut best = None;
    for tail in tails {
        let Some(offset) = phrase_offset_words(words, tail) else {
            continue;
        };
        if !matches_exact_word_slice(words.get(offset..)?, tail) {
            continue;
        }
        best = Some(best.map_or(offset, |current: usize| current.min(offset)));
    }
    best
}

fn matches_exact_word_slice(words: &[&str], phrase: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        |input: &mut primitives::WordSliceInput<'_>| parse_phrase_words(input, phrase),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

fn token_slice_for_words<'a>(
    tokens: &'a [OwnedLexToken],
    view: &TokenWordView<'a>,
    start: usize,
    end: usize,
) -> Option<&'a [OwnedLexToken]> {
    Some(trim_lexed_commas(
        tokens.get(view.token_span_for_words(start, end)?)?,
    ))
}

fn activate_only_you_control_options() -> ControlConditionOptions {
    ControlConditionOptions {
        allow_that_player: false,
        ..ControlConditionOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn timing_and_frequency_shapes_return_typed_values() {
        assert_eq!(
            parse_activate_only_timing_lexed(&lex("Activate only during combat.")),
            Some(ActivationTiming::DuringCombat)
        );
        assert_eq!(
            parse_triggered_times_each_turn_from_words(&[
                "this", "ability", "triggers", "only", "two", "times", "each", "turn"
            ]),
            Some(2)
        );
        assert_eq!(
            parse_activation_count_per_turn(&["three", "times", "each", "turn"]),
            Some(3)
        );
    }

    #[test]
    fn activation_conditions_preserve_existing_semantics() {
        assert_eq!(
            parse_activation_condition_lexed(&lex(
                "Activate only if creatures you control have total power 8 or greater."
            )),
            Some(ConditionExpr::ControlCreaturesTotalPowerAtLeast(8))
        );
        assert_eq!(
            parse_activation_condition_lexed(&lex("Activate only twice each turn.")),
            Some(ConditionExpr::MaxActivationsPerTurn(2))
        );
        assert!(matches!(
            parse_activation_condition_lexed(&lex(
                "Activate only if there are three or more brick counters on this artifact."
            )),
            Some(ConditionExpr::SourceHasCounterAtLeast {
                counter_type: crate::CounterType::Named("brick"),
                count: 3,
                ..
            })
        ));
        assert_eq!(
            parse_activation_condition_lexed(&lex(
                "Activate only if this permanent is a creature."
            )),
            Some(ConditionExpr::SourceMatches(ObjectFilter::creature()))
        );
    }

    #[test]
    fn combined_once_and_turn_timing_keeps_both_constraints() {
        assert_eq!(
            parse_activation_condition_lexed(&lex(
                "Activate only during your turn and only once each turn."
            )),
            Some(ConditionExpr::And(
                Box::new(ConditionExpr::MaxActivationsPerTurn(1)),
                Box::new(ConditionExpr::ActivationTiming(
                    ActivationTiming::DuringYourTurn
                )),
            ))
        );
        assert_eq!(
            parse_activation_condition_lexed(&lex("Activate only once each turn.")),
            Some(ConditionExpr::MaxActivationsPerTurn(1))
        );
    }

    #[test]
    fn combined_once_and_owned_graveyard_threshold_keeps_both_constraints() {
        let parsed = parse_activation_condition_lexed(&lex(
            "Activate only once each turn and only if there are seven or more cards in your graveyard.",
        ))
        .expect("combined frequency and graveyard threshold should parse");
        let ConditionExpr::And(frequency, threshold) = parsed else {
            panic!("expected a typed conjunction: {parsed:#?}");
        };
        assert_eq!(*frequency, ConditionExpr::MaxActivationsPerTurn(1));
        let ConditionExpr::PlayerHasAtLeast {
            player,
            filter,
            count,
        } = *threshold
        else {
            panic!("expected an owned-graveyard cardinality condition: {threshold:#?}");
        };
        assert_eq!(player, PlayerFilter::You);
        assert_eq!(count, 7);
        assert_eq!(filter.zone, Some(crate::zone::Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));

        assert!(
            parse_activation_condition_lexed(&lex(
                "Activate only once each turn and only if there are seven cards in a graveyard.",
            ))
            .is_none(),
            "the owned-graveyard threshold must not claim a different zone-owner surface"
        );
    }

    #[test]
    fn activation_condition_composes_repeated_or_if_with_typed_source_and_basic_land() {
        let parsed = parse_activation_condition_lexed(&lex(
            "Activate only if this land entered this turn or if you control a basic land.",
        ))
        .expect("repeated or-if activation condition should parse");

        let ConditionExpr::Or(left, right) = parsed else {
            panic!("expected a typed disjunction");
        };
        let ConditionExpr::ObjectEnteredBattlefieldThisTurn(source_filter) = left.as_ref() else {
            panic!("expected source-entered-this-turn left branch, got {left:?}");
        };
        assert!(source_filter.source);
        assert_eq!(
            source_filter.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this land".to_string()
            ))
        );

        let ConditionExpr::YouControl(basic_land_filter) = right.as_ref() else {
            panic!("expected basic-land control right branch, got {right:?}");
        };
        assert!(
            basic_land_filter
                .card_types
                .contains(&crate::types::CardType::Land)
        );
        assert!(
            basic_land_filter
                .supertypes
                .contains(&crate::types::Supertype::Basic)
        );
    }

    #[test]
    fn activation_condition_or_if_composition_reuses_existing_branch_parsers() {
        let parsed = parse_activation_condition_lexed(&lex(
            "Activate only if you control an artifact or if you control a creature.",
        ))
        .expect("generic repeated or-if control branches should parse");

        assert!(matches!(parsed, ConditionExpr::Or(_, _)));
        assert!(matches!(
            parse_activation_condition_lexed(&lex(
                "Activate only if you control a Plains or a Swamp."
            )),
            Some(ConditionExpr::PlayerHasAtLeast { .. }) | Some(ConditionExpr::Or(_, _))
        ));
    }
}
