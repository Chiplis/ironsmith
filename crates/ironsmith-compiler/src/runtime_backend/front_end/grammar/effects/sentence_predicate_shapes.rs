use super::*;

use crate::filter::CounterConstraint;
use crate::object::CounterType;
use crate::runtime_backend::front_end::grammar::{filters, leaf};
use ironsmith_core::{EffectMetric, EffectMetricSource};
use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailingCounterConstraintShape {
    NoCounters,
    Constraint(CounterConstraint),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PowerDamageSelfShape<'a> {
    pub(crate) source_tokens: &'a [OwnedLexToken],
    pub(crate) first_target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TappedThisWayBindingShape {
    pub(crate) damage_to_active_player: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuraEnchantmentShape<'a> {
    pub(crate) attachment_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
    pub(crate) granted_ability_tokens: Vec<&'a [OwnedLexToken]>,
    pub(crate) attachment_mentions_you_control: bool,
    pub(crate) loses_all_abilities: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedSentenceShape<'a> {
    NextEndStep,
    NextCombat,
    EndOfCombat {
        remainder_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuotedAbilitySentenceShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImmediateSacrificeSentenceShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingIfSentenceShape {
    pub(crate) replacement: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceBlockedLibraryShuffleShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SingleGraveyardLibraryBottomShape {
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WhereXReferenceShape {
    Source,
    Target,
    TaggedIt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WhereXMetricShape {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SacrificeCostObjectKindShape {
    CardType(crate::types::CardType),
    Permanent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WhereXValueShape {
    CommanderManaValueChoice,
    ReferenceMetric {
        reference: WhereXReferenceShape,
        metric: WhereXMetricShape,
    },
    TapCostPower,
    CommanderCastCount,
    CardTypesInYourGraveyard,
    SacrificeCostManaValue {
        object_kind: SacrificeCostObjectKindShape,
    },
    ColorsAmongSacrificed {
        object_kind: String,
    },
    TwoPlusSacrificedManaValue,
    SourceExiledManaValue,
    PriorEffectMetric {
        source: EffectMetricSource,
        metric: EffectMetric,
    },
    RemovedCountersThisWay,
    CountersOn {
        reference: WhereXReferenceShape,
        counter_type: Option<CounterType>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct WhereXSentenceShape<'a> {
    pub(crate) stripped_tokens: &'a [OwnedLexToken],
    pub(crate) where_tokens: &'a [OwnedLexToken],
    pub(crate) comma_tail_has_effect_clause: bool,
    pub(crate) stripped_references_target: bool,
    pub(crate) stripped_starts_search: bool,
    where_segments: Vec<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone)]
pub(crate) struct WhereXLayout<'a> {
    pub(crate) primary_where_tokens: &'a [OwnedLexToken],
    pub(crate) trailing_after_where: Vec<OwnedLexToken>,
}

impl<'a> WhereXSentenceShape<'a> {
    pub(crate) fn layout(&self, full_where_is_count_value: bool) -> WhereXLayout<'a> {
        if full_where_is_count_value {
            return WhereXLayout {
                primary_where_tokens: self.where_tokens,
                trailing_after_where: Vec::new(),
            };
        }

        let primary_where_tokens = self
            .where_segments
            .first()
            .copied()
            .unwrap_or(self.where_tokens);
        let mut trailing_after_where = Vec::new();
        for (index, segment) in self.where_segments.iter().enumerate().skip(1) {
            if index > 1 {
                trailing_after_where.push(OwnedLexToken::comma(TextSpan::synthetic()));
            }
            trailing_after_where.extend(segment.iter().cloned());
        }
        WhereXLayout {
            primary_where_tokens,
            trailing_after_where,
        }
    }
}

fn marker_anywhere<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

fn counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .void()
        .parse_next(input)
}

fn parse_trailing_counter_constraint_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TrailingCounterConstraintShape> {
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::kw("with"))
        .void()
        .parse_next(input)?;
    let tail = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    let no_counters = primitives::parse_all(
        tail,
        (primitives::kw("no"), eof).void(),
        "trailing no-counter constraint",
    )
    .is_ok();
    if no_counters {
        return Ok(TrailingCounterConstraintShape::NoCounters);
    }

    let words = parser_token_word_refs(tail);
    let (constraint, consumed) = filters::parse_filter_counter_constraint_words(&words)
        .ok_or_else(|| primitives::backtrack_err("counter constraint", "typed counter clause"))?;
    if consumed != words.len() {
        return Err(primitives::backtrack_err(
            "counter constraint",
            "complete counter clause",
        ));
    }
    Ok(TrailingCounterConstraintShape::Constraint(constraint))
}

pub(crate) fn parse_trailing_counter_constraint_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TrailingCounterConstraintShape> {
    primitives::parse_all(
        tokens,
        parse_trailing_counter_constraint_lexed,
        "trailing counter constraint",
    )
    .ok()
}

fn damage_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("deals"), primitives::kw("deal")))
        .void()
        .parse_next(input)
}

fn parse_power_damage_self_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PowerDamageSelfShape<'a>> {
    let source_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((damage_verb, primitives::phrase(&["x", "damage", "to"]))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    damage_verb.parse_next(input)?;
    primitives::phrase(&["x", "damage", "to"]).parse_next(input)?;
    let first_target_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["and", "x", "damage", "to"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["and", "x", "damage", "to"]).parse_next(input)?;
    alt((primitives::kw("itself"), primitives::kw("it"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["where", "x", "is", "its", "power"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PowerDamageSelfShape {
        source_tokens,
        first_target_tokens,
    })
}

pub(crate) fn parse_power_damage_self_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PowerDamageSelfShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_power_damage_self_lexed,
        "power damage to another target and self",
    )
    .ok()
}

fn parse_tapped_this_way_where_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["where", "x", "is", "the", "number", "of"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["tapped", "this", "way"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["tapped", "this", "way"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

pub(crate) fn parse_tapped_this_way_binding_tokens(
    stripped_tokens: &[OwnedLexToken],
    where_tokens: &[OwnedLexToken],
) -> Option<TappedThisWayBindingShape> {
    primitives::parse_all(
        where_tokens,
        parse_tapped_this_way_where_lexed,
        "where X is number tapped this way",
    )
    .ok()?;
    let damage_to_active_player = marker_anywhere(
        stripped_tokens,
        primitives::phrase(&["to", "the", "player"]),
    );
    Some(TappedThisWayBindingShape {
        damage_to_active_player,
    })
}

fn aura_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (primitives::kw("it's"), primitives::kw("an")).void(),
        (primitives::kw("it’s"), primitives::kw("an")).void(),
        (primitives::kw("its"), primitives::kw("an")).void(),
        primitives::phrase(&["it", "s", "an"]),
        primitives::phrase(&["it", "is", "an"]),
    ))
    .parse_next(input)
}

fn apostrophe<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::token_kind(TokenKind::Apostrophe)
        .void()
        .parse_next(input)
}

fn quoted_token_group<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (
            primitives::quote(),
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::quote())).void(),
            primitives::quote(),
        )
            .void(),
        (
            apostrophe,
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(apostrophe)).void(),
            apostrophe,
        )
            .void(),
    ))
    .parse_next(input)
}

fn attachment_token<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((quoted_token_group, any.void())).parse_next(input)
}

fn parse_aura_with_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let attachment_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., attachment_token, peek(primitives::kw("and")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let tail_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((attachment_tokens, tail_tokens))
}

fn parse_aura_without_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let attachment_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((attachment_tokens, &[]))
}

fn parse_aura_enchantment_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    aura_prefix.parse_next(input)?;
    primitives::phrase(&["aura", "enchantment", "with", "enchant"]).parse_next(input)?;
    alt((parse_aura_with_tail_lexed, parse_aura_without_tail_lexed)).parse_next(input)
}

fn parse_apostrophe_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), apostrophe)
        .void()
        .parse_next(input)?;
    let segment = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(apostrophe))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    apostrophe.parse_next(input)?;
    Ok(segment)
}

fn aura_ability_tokens<'a>(tail_tokens: &'a [OwnedLexToken]) -> Vec<&'a [OwnedLexToken]> {
    let Some((_, ability_tokens)) = primitives::parse_prefix(
        tail_tokens,
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::kw("has")).void(),
    ) else {
        return Vec::new();
    };
    let mut input = LexStream::new(ability_tokens);
    let quoted: Vec<&[OwnedLexToken]> = repeat(1.., parse_apostrophe_segment_lexed)
        .parse_next(&mut input)
        .unwrap_or_default();
    if quoted.is_empty() {
        vec![ability_tokens]
    } else {
        quoted
    }
}

pub(crate) fn parse_aura_enchantment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AuraEnchantmentShape<'_>> {
    let (attachment_tokens, tail_tokens) = primitives::parse_all(
        tokens,
        parse_aura_enchantment_lexed,
        "it is an aura enchantment",
    )
    .ok()?;
    let attachment_mentions_you_control = primitives::find_prefix(attachment_tokens, || {
        primitives::phrase(&["you", "control"]).void()
    })
    .is_some();
    let loses_all_abilities = marker_anywhere(
        tail_tokens,
        alt((
            primitives::phrase(&["loses", "all", "other", "abilities"]),
            primitives::phrase(&["loses", "all", "abilities"]),
        )),
    );
    Some(AuraEnchantmentShape {
        attachment_tokens,
        tail_tokens,
        granted_ability_tokens: aura_ability_tokens(tail_tokens),
        attachment_mentions_you_control,
        loses_all_abilities,
    })
}

fn parse_cant_gain_life_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("if").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["would", "gain", "life", "this", "turn"]),
    )
    .void()
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(alt((
            primitives::phrase(&["gains", "no", "life", "instead"]),
            primitives::phrase(&["gain", "no", "life", "instead"]),
        ))),
    )
    .void()
    .parse_next(input)?;
    alt((
        primitives::phrase(&["gains", "no", "life", "instead"]),
        primitives::phrase(&["gain", "no", "life", "instead"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

pub(crate) fn parses_cant_gain_life_replacement_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_cant_gain_life_lexed,
        "cant gain life replacement",
    )
    .is_ok()
}

fn parse_next_end_step_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["at", "the", "beginning", "of"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["next", "end", "step"]),
    )
    .void()
    .parse_next(input)
}

fn parse_next_combat_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "combat"]).parse_next(input)
}

fn parse_end_combat_delayed_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["at", "this"]).parse_next(input)?;
    let timing_tokens = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::phrase(&["end", "of", "combat"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    if !marker_anywhere(timing_tokens, primitives::kw("next")) {
        return Err(primitives::backtrack_err(
            "delayed end of combat",
            "next timing marker",
        ));
    }
    primitives::phrase(&["end", "of", "combat"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)
}

pub(crate) fn parse_delayed_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DelayedSentenceShape<'_>> {
    if primitives::parse_prefix(tokens, parse_next_end_step_prefix_lexed).is_some() {
        return Some(DelayedSentenceShape::NextEndStep);
    }
    if primitives::parse_prefix(tokens, parse_next_combat_prefix_lexed).is_some() {
        return Some(DelayedSentenceShape::NextCombat);
    }
    primitives::parse_all(
        tokens,
        parse_end_combat_delayed_lexed,
        "delayed end of combat sentence",
    )
    .ok()
    .map(|remainder_tokens| DelayedSentenceShape::EndOfCombat { remainder_tokens })
}

fn quote_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::token_kind(TokenKind::Quote)
        .void()
        .parse_next(input)
}

fn ability_action<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("gain"),
        primitives::kw("gains"),
        primitives::kw("has"),
        primitives::kw("have"),
        primitives::kw("lose"),
        primitives::kw("loses"),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_quoted_ability_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<QuotedAbilitySentenceShape> {
    (marker_anywhere(tokens, quote_marker) && marker_anywhere(tokens, ability_action))
        .then_some(QuotedAbilitySentenceShape)
}

fn delayed_lifecycle_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "end", "step"]),
        primitives::phrase(&["at", "the", "beginning", "of", "next", "end", "step"]),
        primitives::phrase(&["at", "end", "of", "combat"]),
        primitives::phrase(&["at", "the", "end", "of", "combat"]),
    ))
    .parse_next(input)
}

pub(crate) fn parse_immediate_sacrifice_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ImmediateSacrificeSentenceShape> {
    let (_, tail) = primitives::parse_prefix(tokens, primitives::kw("sacrifice"))?;
    let counted = primitives::parse_prefix(
        tail,
        alt((
            primitives::phrase(&["any", "number"]),
            primitives::phrase(&["one", "or", "more"]),
        )),
    )
    .is_some();
    (!counted && !marker_anywhere(tokens, delayed_lifecycle_marker))
        .then_some(ImmediateSacrificeSentenceShape)
}

pub(crate) fn parse_leading_if_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeadingIfSentenceShape> {
    primitives::parse_prefix(tokens, primitives::kw("if"))?;
    Some(LeadingIfSentenceShape {
        replacement: marker_anywhere(tokens, primitives::kw("would")),
    })
}

fn owner_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("owner"),
        primitives::kw("owners"),
        primitives::kw("owner's"),
        primitives::kw("owners'"),
    ))
    .void()
    .parse_next(input)
}

fn library_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("library"), primitives::kw("libraries")))
        .void()
        .parse_next(input)
}

fn parse_source_blocked_library_shuffle_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("put").parse_next(input)?;
    primitives::phrase(&[
        "this", "creature", "and", "each", "creature", "it's", "blocking",
    ])
    .parse_next(input)?;
    primitives::phrase(&["on", "top", "of"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), owner_word)
        .void()
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), library_word)
        .void()
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["then", "those", "players", "shuffle"]),
    )
    .void()
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

pub(crate) fn parse_source_blocked_library_shuffle_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceBlockedLibraryShuffleShape> {
    primitives::parse_all(
        tokens,
        parse_source_blocked_library_shuffle_lexed,
        "source and blocked creatures library shuffle",
    )
    .ok()
    .map(|()| SourceBlockedLibraryShuffleShape)
}

fn parse_single_graveyard_library_bottom_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    primitives::kw("put").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    if count == 0 {
        return Err(primitives::backtrack_err(
            "graveyard card count",
            "positive count",
        ));
    }
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    primitives::phrase(&[
        "from",
        "a",
        "single",
        "graveyard",
        "on",
        "the",
        "bottom",
        "of",
    ])
    .parse_next(input)?;
    alt((primitives::kw("its"), primitives::kw("their"))).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), owner_word)
        .void()
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), library_word)
        .void()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(count)
}

pub(crate) fn parse_single_graveyard_library_bottom_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SingleGraveyardLibraryBottomShape> {
    primitives::parse_all(
        tokens,
        parse_single_graveyard_library_bottom_lexed,
        "single graveyard cards to library bottom",
    )
    .ok()
    .map(|count| SingleGraveyardLibraryBottomShape { count })
}

fn where_x_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["where", "x", "is"]).parse_next(input)
}

fn segment_has_effect_clause(tokens: &[OwnedLexToken]) -> bool {
    marker_anywhere(tokens, primitives::kw("then"))
        || super::chain_splitting::find_chain_verb_tokens(tokens).is_some()
}

pub(crate) fn parse_where_x_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXSentenceShape<'_>> {
    let (stripped_tokens, where_tokens) = primitives::parse_prefix(
        tokens,
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(where_x_prefix))
            .map(|((), _)| ())
            .take(),
    )?;
    let where_segments = primitives::split_lexed_slices_on_commas_or_semicolons(where_tokens);
    let comma_tail_has_effect_clause = where_segments
        .iter()
        .skip(1)
        .copied()
        .any(segment_has_effect_clause);
    let stripped_references_target = marker_anywhere(stripped_tokens, primitives::kw("target"));
    let stripped_starts_search =
        primitives::parse_prefix(stripped_tokens, primitives::kw("search")).is_some();
    Some(WhereXSentenceShape {
        stripped_tokens,
        where_tokens,
        comma_tail_has_effect_clause,
        stripped_references_target,
        stripped_starts_search,
        where_segments,
    })
}

pub(crate) fn parse_before_activation_time_tokens(
    where_tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (prefix, _) = primitives::parse_prefix(
        where_tokens,
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("as")))
            .map(|((), _)| ())
            .take(),
    )?;
    Some(prefix)
}

fn articles<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            primitives::kw("a"),
            primitives::kw("an"),
            primitives::kw("the"),
        )),
    )
    .void()
    .parse_next(input)
}

fn article_then<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (articles, primitives::kw(expected)).void()
}

fn parse_commander_choice_where_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    for word in [
        "mana",
        "value",
        "of",
        "commander",
        "you",
        "own",
        "on",
        "battlefield",
        "or",
        "in",
        "command",
        "zone",
    ] {
        article_then(word).parse_next(input)?;
    }
    primitives::sentence_end().parse_next(input)?;
    Ok(WhereXValueShape::CommanderManaValueChoice)
}

fn metric<'a>(input: &mut LexStream<'a>) -> WResult<WhereXMetricShape> {
    alt((
        primitives::kw("power").value(WhereXMetricShape::Power),
        primitives::kw("toughness").value(WhereXMetricShape::Toughness),
        primitives::phrase(&["mana", "value"]).value(WhereXMetricShape::ManaValue),
    ))
    .parse_next(input)
}

fn parse_reference_metric_where_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(WhereXMetricShape, ReferenceSurface)> {
    where_x_prefix.parse_next(input)?;
    let (surface, parsed_metric) = alt((
        (primitives::kw("its"), metric).map(|(_, metric)| (ReferenceSurface::Its, metric)),
        (primitives::phrase(&["this", "creatures"]), metric)
            .map(|(_, metric)| (ReferenceSurface::ThisCreature, metric)),
        (
            primitives::kw("that"),
            alt((
                primitives::kw("spell"),
                primitives::kw("spell's"),
                primitives::kw("spells"),
            )),
            metric,
        )
            .map(|(_, _, metric)| (ReferenceSurface::ThatSpell, metric)),
        (
            primitives::kw("that"),
            alt((
                primitives::kw("creature"),
                primitives::kw("creature's"),
                primitives::kw("creatures"),
            )),
            metric,
        )
            .map(|(_, _, metric)| (ReferenceSurface::ThatCreature, metric)),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((parsed_metric, surface))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceSurface {
    Its,
    ThisCreature,
    ThatSpell,
    ThatCreature,
}

fn parse_tap_cost_power_where_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["power", "of", "the", "creature", "tapped", "this", "way"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(WhereXValueShape::TapCostPower)
}

fn parse_commander_cast_count_where_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    primitives::phrase(&["the", "number", "of", "times"]).parse_next(input)?;
    alt((
        primitives::phrase(&["it's", "been"]),
        primitives::phrase(&["its", "been"]),
        primitives::phrase(&["it", "has", "been"]),
    ))
    .parse_next(input)?;
    primitives::phrase(&["cast", "from", "the", "command", "zone", "this", "game"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(WhereXValueShape::CommanderCastCount)
}

fn parse_card_types_in_your_graveyard_where_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    primitives::phrase(&["the", "number", "of"]).parse_next(input)?;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    alt((primitives::kw("type"), primitives::kw("types"))).parse_next(input)?;
    primitives::phrase(&["among", "cards", "in", "your", "graveyard"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(WhereXValueShape::CardTypesInYourGraveyard)
}

fn parse_sacrifice_cost_where_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("sacrificed").parse_next(input)?;
    let object_kind_token = primitives::word_text.parse_next(input)?;
    let object_kind = match leaf::parse_leaf_demonstrative_object_head_complete(
        leaf::strip_leaf_source_possessive_suffix(object_kind_token),
    ) {
        Ok(leaf::LeafDemonstrativeObjectHead::CardType(card_type)) => {
            SacrificeCostObjectKindShape::CardType(card_type)
        }
        Ok(leaf::LeafDemonstrativeObjectHead::Permanent) => SacrificeCostObjectKindShape::Permanent,
        _ => {
            return Err(primitives::backtrack_err(
                "sacrifice-cost value",
                "possessive permanent or card type",
            ));
        }
    };
    primitives::phrase(&["mana", "value"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(WhereXValueShape::SacrificeCostManaValue { object_kind })
}

fn parse_colors_among_where_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    primitives::phrase(&["the", "number", "of"]).parse_next(input)?;
    alt((primitives::kw("color"), primitives::kw("colors"))).parse_next(input)?;
    alt((primitives::kw("that"), primitives::kw("the"))).parse_next(input)?;
    let object_kind = primitives::word_text.parse_next(input)?.to_string();
    alt((primitives::kw("was"), primitives::kw("were"))).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(WhereXValueShape::ColorsAmongSacrificed { object_kind })
}

fn parse_two_plus_sacrificed_where_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    alt((primitives::kw("2"), primitives::kw("two"))).parse_next(input)?;
    primitives::kw("plus").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("sacrificed").parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    primitives::phrase(&["mana", "value"]).parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(WhereXValueShape::TwoPlusSacrificedManaValue)
}

fn chosen_memory_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("chosen").parse_next(input)?;
    let mut probe = input.clone();
    if alt((primitives::kw("type"), primitives::kw("color")))
        .parse_next(&mut probe)
        .is_ok()
    {
        return Err(primitives::backtrack_err(
            "chosen memory",
            "chosen object rather than chosen type or color",
        ));
    }
    Ok(())
}

fn affected_memory_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "way"]),
        primitives::kw("destroyed").void(),
        primitives::kw("discarded").void(),
        primitives::kw("exiled").void(),
        primitives::kw("milled").void(),
        primitives::kw("revealed").void(),
        primitives::kw("sacrificed").void(),
        primitives::kw("searched").void(),
    ))
    .parse_next(input)
}

fn prior_effect_source(tokens: &[OwnedLexToken]) -> Option<EffectMetricSource> {
    if marker_anywhere(tokens, chosen_memory_marker) {
        Some(EffectMetricSource::ChosenObjects)
    } else if marker_anywhere(tokens, affected_memory_marker) {
        Some(EffectMetricSource::AffectedObjects)
    } else {
        None
    }
}

fn exact_exiled_card_reference(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            alt((
                primitives::phrase(&["the", "exiled", "card"]),
                primitives::phrase(&["the", "exiled", "cards"]),
                primitives::phrase(&["exiled", "card"]),
                primitives::phrase(&["exiled", "cards"]),
            )),
            eof,
        )
            .void(),
        "exiled card reference",
    )
    .is_ok()
}

fn removed_counters_this_way(tokens: &[OwnedLexToken]) -> bool {
    marker_anywhere(tokens, counter_noun)
        && marker_anywhere(tokens, primitives::kw("removed"))
        && marker_anywhere(tokens, primitives::phrase(&["this", "way"]))
}

fn parse_prior_effect_where_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let parsed_metric = alt((
        primitives::kw("power").value(Some(EffectMetric::FirstPower)),
        primitives::kw("toughness").value(Some(EffectMetric::FirstToughness)),
        primitives::phrase(&["mana", "value"]).value(Some(EffectMetric::FirstManaValue)),
        primitives::phrase(&["number", "of"]).value(None),
    ))
    .parse_next(input)?;
    if parsed_metric.is_some() {
        primitives::kw("of").parse_next(input)?;
    }
    let reference_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    if parsed_metric == Some(EffectMetric::FirstManaValue)
        && exact_exiled_card_reference(reference_tokens)
    {
        return Ok(WhereXValueShape::SourceExiledManaValue);
    }
    if parsed_metric.is_none() && removed_counters_this_way(reference_tokens) {
        return Ok(WhereXValueShape::RemovedCountersThisWay);
    }
    let source = prior_effect_source(reference_tokens)
        .ok_or_else(|| primitives::backtrack_err("prior effect reference", "remembered objects"))?;
    Ok(WhereXValueShape::PriorEffectMetric {
        source,
        metric: parsed_metric.unwrap_or(EffectMetric::Count),
    })
}

fn parse_counter_reference_where_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXValueShape> {
    where_x_prefix.parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of"]).parse_next(input)?;
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
        primitives::kw("one"),
    )))
    .parse_next(input)?;
    let descriptor = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(counter_noun))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    counter_noun.parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let reference_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let counter_type = if descriptor.is_empty() {
        None
    } else {
        filters::parse_counter_type_from_tokens(descriptor)
    };
    if !descriptor.is_empty() && counter_type.is_none() {
        return Err(primitives::backtrack_err(
            "counter type",
            "known single-word counter type",
        ));
    }

    let reference_words = parser_token_word_refs(reference_tokens);
    let reference = if leaf::parse_leaf_source_anaphor_words(&reference_words).is_some() {
        WhereXReferenceShape::Source
    } else {
        primitives::parse_all(
            reference_tokens,
            (
                alt((primitives::kw("that"), primitives::kw("those"))),
                opt(alt((
                    primitives::kw("card"),
                    primitives::kw("cards"),
                    primitives::kw("creature"),
                    primitives::kw("creatures"),
                    primitives::kw("object"),
                    primitives::kw("objects"),
                    primitives::kw("permanent"),
                    primitives::kw("permanents"),
                ))),
                eof,
            )
                .void(),
            "tagged counter reference",
        )
        .map_err(|_| {
            primitives::backtrack_err("counter reference", "source or tagged object reference")
        })?;
        WhereXReferenceShape::TaggedIt
    };
    Ok(WhereXValueShape::CountersOn {
        reference,
        counter_type,
    })
}

pub(crate) fn parse_where_x_value_shape_tokens(
    where_tokens: &[OwnedLexToken],
    stripped_references_target: bool,
) -> Option<WhereXValueShape> {
    if let Ok(shape) = primitives::parse_all(
        where_tokens,
        parse_commander_choice_where_lexed,
        "where X commander mana value choice",
    ) {
        return Some(shape);
    }
    if let Ok(shape) = primitives::parse_all(
        where_tokens,
        parse_tap_cost_power_where_lexed,
        "where X tap cost power",
    ) {
        return Some(shape);
    }
    if let Ok(shape) = primitives::parse_all(
        where_tokens,
        parse_prior_effect_where_lexed,
        "where X prior effect metric",
    ) {
        return Some(shape);
    }
    if let Ok((metric, surface)) = primitives::parse_all(
        where_tokens,
        parse_reference_metric_where_lexed,
        "where X reference metric",
    ) {
        let reference = match surface {
            ReferenceSurface::Its => {
                if stripped_references_target {
                    WhereXReferenceShape::Target
                } else {
                    WhereXReferenceShape::Source
                }
            }
            ReferenceSurface::ThisCreature => WhereXReferenceShape::Source,
            ReferenceSurface::ThatSpell => WhereXReferenceShape::TaggedIt,
            ReferenceSurface::ThatCreature => {
                if stripped_references_target {
                    WhereXReferenceShape::Target
                } else {
                    WhereXReferenceShape::TaggedIt
                }
            }
        };
        return Some(WhereXValueShape::ReferenceMetric { reference, metric });
    }
    for parser in [
        parse_commander_cast_count_where_lexed
            as for<'a> fn(&mut LexStream<'a>) -> WResult<WhereXValueShape>,
        parse_card_types_in_your_graveyard_where_lexed,
        parse_sacrifice_cost_where_lexed,
        parse_colors_among_where_lexed,
        parse_two_plus_sacrificed_where_lexed,
        parse_counter_reference_where_lexed,
    ] {
        if let Ok(shape) = primitives::parse_all(where_tokens, parser, "typed where X value") {
            return Some(shape);
        }
    }
    None
}

#[cfg(test)]
#[path = "sentence_predicate_shapes/tests.rs"]
mod tests;
