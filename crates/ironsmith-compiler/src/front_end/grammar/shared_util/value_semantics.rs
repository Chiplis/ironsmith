use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::{ObjectFilter, Zone};
use ironsmith_core::EffectMetric;
use ironsmith_core::TurnHistoryCount;
use ironsmith_core::ValueSurfaceHint;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use crate::object_filters::{
    parse_object_filter, parse_object_filter_lexed, parse_object_filter_words,
};
use crate::util::{
    parse_greater_than_or_equal_quantity_prefix, possessive_normalized_word_refs, trim_commas,
    trim_edge_punctuation, trim_edge_punctuation_tokens,
};

use super::super::super::lexer::{OwnedLexToken, trim_lexed_commas};
use super::super::leaf;
use super::super::primitives::{self, TokenWordView, WordSliceInput};
use super::super::values::parse_value_comparison_words;
pub use super::super::values::{parse_number_prefix_lexed, parse_value_prefix_lexed};
use super::value_expr;
use super::value_helper_shapes;
use super::value_shapes::{self, AggregateValueMetric};

const SOURCE_LINKED_EXILED_CARD_PHRASES: &[&[&str]] = &[
    &["the", "exiled", "card"],
    &["the", "exiled", "cards"],
    &["exiled", "card"],
    &["exiled", "cards"],
];
const CREATURES_DIED_THIS_TURN_PHRASES: &[&[&str]] = &[
    &["creature", "that", "died", "this", "turn"],
    &["creatures", "that", "died", "this", "turn"],
];
const EQUAL_TO_PHRASE: &[&str] = &["equal", "to"];

/// Parse an authored mana-symbol payment total such as
/// `the amount of {S} spent to cast this spell`.
///
/// This must stay lexed: mana groups intentionally do not appear in
/// `TokenWordView`, so a word-only value parser cannot preserve which symbol
/// the count refers to.
pub fn parse_mana_symbol_spent_to_cast_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let mut mana_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| token.mana_group_inner().map(|_| index));
    let mana_index = mana_indices.next()?;
    if mana_indices.next().is_some() {
        return None;
    }

    let prefix = TokenWordView::new(&tokens[..mana_index]).to_word_refs();
    if !matches!(
        prefix.as_slice(),
        ["the", "amount", "of"]
            | ["amount", "of"]
            | ["where", "x", "is", "the", "amount", "of"]
            | ["where", "x", "is", "amount", "of"]
    ) {
        return None;
    }

    let suffix = TokenWordView::new(&tokens[mana_index + 1..]).to_word_refs();
    let reference = match suffix.as_slice() {
        ["spent", "to", "cast", "it"] => ironsmith_core::ManaSpentCastReferenceSurface::It,
        ["spent", "to", "cast", "this", "spell"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell
        }
        ["spent", "to", "cast", "this", "creature"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature
        }
        _ => return None,
    };
    // Reject hidden punctuation or other non-word tokens in an otherwise
    // matching surface instead of silently treating them as absent.
    if prefix.len() + suffix.len() + 1 != tokens.len() {
        return None;
    }

    let symbols =
        super::super::values::parse_mana_symbol_group(tokens[mana_index].parser_text()).ok()?;
    let [symbol] = symbols.as_slice() else {
        return None;
    };
    if !matches!(
        symbol,
        crate::mana::ManaSymbol::White
            | crate::mana::ManaSymbol::Blue
            | crate::mana::ManaSymbol::Black
            | crate::mana::ManaSymbol::Red
            | crate::mana::ManaSymbol::Green
            | crate::mana::ManaSymbol::Colorless
            | crate::mana::ManaSymbol::Snow
    ) {
        return None;
    }

    Some(Value::ManaSymbolSpentToCastThisSpell {
        symbol: *symbol,
        reference,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EqualToStart {
    start: usize,
    after: usize,
}

fn parse_equal_to_start(words: &[&str]) -> Option<EqualToStart> {
    let mut input: WordSliceInput<'_> = words;
    parse_equal_to_start_words.parse_next(&mut input).ok()
}

fn parse_equal_to_start_words(
    input: &mut WordSliceInput<'_>,
) -> Result<EqualToStart, ErrMode<ContextError>> {
    let initial_len = input.len();
    loop {
        let checkpoint = *input;
        if (
            primitives::word_slice_exact("equal"),
            primitives::word_slice_exact("to"),
        )
            .void()
            .parse_next(input)
            .is_ok()
        {
            let after = initial_len.saturating_sub(input.len());
            return Ok(EqualToStart {
                start: after.saturating_sub(EQUAL_TO_PHRASE.len()),
                after,
            });
        }
        *input = checkpoint;
        any.void().parse_next(input)?;
    }
}

fn words_match_any_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::parse_word_sequence_complete(words, phrase).is_some())
}

fn counter_reference_shape_value(shape: value_helper_shapes::CounterReferenceValueShape) -> Value {
    match shape.reference {
        value_helper_shapes::CounterValueReference::Source(surface) => {
            Value::counters_on_source_reference(shape.counter_type, surface)
        }
        value_helper_shapes::CounterValueReference::Tagged => Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            shape.counter_type,
        ),
    }
}

const COMMANDER_YOU_OWN_BATTLEFIELD_OR_COMMAND_ZONE_PHRASE: &[&str] = &[
    "commander",
    "you",
    "own",
    "on",
    "battlefield",
    "or",
    "in",
    "command",
    "zone",
];
const COMMANDER_ITERATED_PLAYER_OWNS_BATTLEFIELD_OR_COMMAND_ZONE_PHRASES: &[&[&str]] = &[
    &[
        "commander",
        "they",
        "own",
        "on",
        "battlefield",
        "or",
        "in",
        "command",
        "zone",
    ],
    &[
        "commander",
        "that",
        "player",
        "owns",
        "on",
        "battlefield",
        "or",
        "in",
        "command",
        "zone",
    ],
];
pub fn parse_aggregate_scope_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let surface = value_shapes::parse_aggregate_value_surface(&words)?;
    let scope_start = words.len().checked_sub(surface.scope_words.len())?;
    let scope_token_range = word_view.token_span_for_words(scope_start, words.len())?;
    let scope_tokens = trim_edge_punctuation_tokens(&tokens[scope_token_range]);
    let filter = parse_object_filter_lexed(scope_tokens, false).ok()?;

    match surface.metric {
        AggregateValueMetric::BasicLandTypes => Some(Value::BasicLandTypesAmong(filter)),
        AggregateValueMetric::CreatureTypes => Some(Value::CreatureTypesAmong(filter)),
        AggregateValueMetric::Colors => Some(Value::ColorsAmong(filter)),
        AggregateValueMetric::ColorPairs => Some(Value::ColorPairsAmong(filter)),
        AggregateValueMetric::DistinctNames => Some(Value::DistinctNames(filter)),
        AggregateValueMetric::DistinctPowers => Some(Value::DistinctPowers(filter)),
        AggregateValueMetric::Counters => Some(
            Value::CountersOn(Box::new(crate::target::ChooseSpec::All(filter)), None)
                .with_surface_hint(ValueSurfaceHint::CountersAmong),
        ),
    }
}

fn is_power_toughness_axis_word(word: &str) -> bool {
    matches!(word, "power" | "toughness")
}

fn is_plus_minus_word(word: &str) -> bool {
    matches!(word, "plus" | "minus")
}

fn is_and_or_word(word: &str) -> bool {
    matches!(word, "and" | "or" | "and/or")
}

fn is_comparison_tail_word(word: &str) -> bool {
    matches!(word, "less" | "fewer" | "greater" | "more")
}

fn is_less_or_fewer_word(word: &str) -> bool {
    matches!(word, "less" | "fewer")
}

fn aggregate_effect_metric(
    aggregate: value_helper_shapes::AggregateKind,
    value_kind: value_helper_shapes::AggregateValueKind,
) -> EffectMetric {
    use value_helper_shapes::{AggregateKind, AggregateValueKind};

    match (aggregate, value_kind) {
        (AggregateKind::Total, AggregateValueKind::Power) => EffectMetric::TotalPower,
        (AggregateKind::Total, AggregateValueKind::Toughness) => EffectMetric::TotalToughness,
        (AggregateKind::Total, AggregateValueKind::ManaValue) => EffectMetric::TotalManaValue,
        (AggregateKind::Greatest, AggregateValueKind::Power) => EffectMetric::GreatestPower,
        (AggregateKind::Greatest, AggregateValueKind::Toughness) => EffectMetric::GreatestToughness,
        (AggregateKind::Greatest, AggregateValueKind::ManaValue) => EffectMetric::GreatestManaValue,
    }
}

fn pending_aggregate_metric_value(
    aggregate: value_helper_shapes::AggregateKind,
    value_kind: value_helper_shapes::AggregateValueKind,
    object_words: &[&str],
) -> Option<Value> {
    let source = value_helper_shapes::parse_prior_effect_metric_source(object_words)?;
    let metric = aggregate_effect_metric(aggregate, value_kind);
    if let Some(value) = parse_prior_effect_aggregate_metric_value(metric, object_words) {
        return Some(value);
    }
    Some(Value::PendingEffectMetric { source, metric })
}

pub fn parse_prior_effect_aggregate_metric_value(
    metric: EffectMetric,
    object_words: &[&str],
) -> Option<Value> {
    let source = value_helper_shapes::parse_prior_effect_metric_source(object_words)?;
    let this_way_start = object_words
        .windows(2)
        .position(|window| window == ["this", "way"]);
    if let Some(this_way_start) = this_way_start {
        let subject = &object_words[..this_way_start];
        if let Some((action, action_start)) =
            value_helper_shapes::parse_prior_effect_action(subject)
        {
            let mut query =
                ironsmith_core::PriorEffectMetricQuery::new(source, metric).with_action(action);
            let filter_words = &subject[..action_start];
            if !filter_words.is_empty() {
                let mut filter = parse_object_filter_words(filter_words, false).ok()?;
                if filter_words
                    .iter()
                    .any(|word| matches!(*word, "card" | "cards"))
                {
                    filter.set_explicit_card_noun(true);
                }
                query = query.with_filter(filter);
            }
            return Some(Value::PendingPriorEffectMetric(query));
        }
    }
    None
}

fn aggregate_filter_value(
    aggregate: value_helper_shapes::AggregateKind,
    value_kind: value_helper_shapes::AggregateValueKind,
    filter: ObjectFilter,
) -> Value {
    use value_helper_shapes::{AggregateKind, AggregateValueKind};

    match (aggregate, value_kind) {
        (AggregateKind::Total, AggregateValueKind::Power) => Value::TotalPower(filter),
        (AggregateKind::Total, AggregateValueKind::Toughness) => Value::TotalToughness(filter),
        (AggregateKind::Total, AggregateValueKind::ManaValue) => Value::TotalManaValue(filter),
        (AggregateKind::Greatest, AggregateValueKind::Power) => Value::GreatestPower(filter),
        (AggregateKind::Greatest, AggregateValueKind::Toughness) => {
            Value::GreatestToughness(filter)
        }
        (AggregateKind::Greatest, AggregateValueKind::ManaValue) => {
            Value::GreatestManaValue(filter)
        }
    }
}

fn source_linked_exiled_mana_value(object_words: &[&str]) -> Option<Value> {
    if words_match_any_phrase(object_words, SOURCE_LINKED_EXILED_CARD_PHRASES) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        ))));
    }
    None
}

fn history_filter_from_word_prefix(
    tokens: &[OwnedLexToken],
    words: &TokenWordView<'_>,
    end_word: usize,
) -> Option<ObjectFilter> {
    let range = words.token_span_for_words(0, end_word)?;
    let mut filter = parse_object_filter(&trim_edge_punctuation(&tokens[range]), false).ok()?;
    // Historical values match the event snapshot, not the object's current
    // zone.  Zone transitions are carried by the query variant itself.
    filter.zone = None;
    // A bare `spell` noun is represented by the stack-kind discriminator.
    // The general object-filter parser also supplies `has_mana_cost`, but that
    // would incorrectly exclude spells without mana costs from cast history.
    // Keep the spell discriminator as structured surface/semantic metadata and
    // remove only the accidental mana-cost restriction.
    if filter.stack_kind == Some(crate::filter::StackObjectKind::Spell) {
        filter.has_mana_cost = false;
    }
    Some(filter)
}

fn suffix_start(words: &[&str], suffix: &[&str]) -> Option<usize> {
    words
        .ends_with(suffix)
        .then_some(words.len().saturating_sub(suffix.len()))
}

fn parse_spell_cast_history_count(
    tokens: &[OwnedLexToken],
    word_view: &TokenWordView<'_>,
    words: &[&str],
) -> Option<Value> {
    let suffixes: &[(&[&str], PlayerFilter, bool)] = &[
        (
            &["youve", "cast", "before", "it", "this", "turn"],
            PlayerFilter::You,
            true,
        ),
        (
            &["you've", "cast", "before", "it", "this", "turn"],
            PlayerFilter::You,
            true,
        ),
        (
            &["you", "have", "cast", "before", "it", "this", "turn"],
            PlayerFilter::You,
            true,
        ),
        (
            &["cast", "before", "that", "spell", "this", "turn"],
            PlayerFilter::Any,
            true,
        ),
        (
            &["cast", "before", "this", "spell", "this", "turn"],
            PlayerFilter::Any,
            true,
        ),
        (
            &["cast", "before", "it", "this", "turn"],
            PlayerFilter::Any,
            true,
        ),
        (&["youve", "cast", "this", "turn"], PlayerFilter::You, false),
        (
            &["you've", "cast", "this", "turn"],
            PlayerFilter::You,
            false,
        ),
        (
            &["you", "have", "cast", "this", "turn"],
            PlayerFilter::You,
            false,
        ),
        (&["you", "cast", "this", "turn"], PlayerFilter::You, false),
        (&["cast", "this", "turn"], PlayerFilter::Any, false),
    ];

    for (suffix, player, before_triggering_spell) in suffixes {
        let Some(end) = suffix_start(words, suffix) else {
            continue;
        };
        if end == 0 {
            continue;
        }
        let prefix_words = &words[..end];
        if !prefix_words
            .iter()
            .any(|word| matches!(*word, "spell" | "spells"))
        {
            continue;
        }
        let mut filter = history_filter_from_word_prefix(tokens, word_view, end)?;
        let exclude_source = filter.other || prefix_words.contains(&"other");
        // `other` is relative to the cast being evaluated, not to the source
        // permanent of a triggered ability. Keep that relation in the query.
        filter.other = false;
        return Some(Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
            player: player.clone(),
            filter,
            from_zone: None,
            from_outside_hand: false,
            exclude_source,
            before_triggering_spell: *before_triggering_spell,
        }));
    }
    None
}

/// Build the historical spell count used by an ordinal triggering-spell gate.
///
/// Unlike an ordinary "spells you've cast this turn" value, this count stops
/// at the cast event which caused the current trigger. That event boundary is
/// important: spells cast while the trigger is waiting on the stack must not
/// change whether the triggering spell was first, second, and so on.
pub fn parse_triggering_spell_history_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation(tokens);
    let word_view = TokenWordView::new(&tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let mut filter = history_filter_from_word_prefix(&tokens, &word_view, words.len())?;
    let exclude_source = filter.other || words.contains(&"other");
    filter.other = false;
    Some(Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
        player: PlayerFilter::You,
        filter,
        from_zone: None,
        from_outside_hand: false,
        exclude_source,
        before_triggering_spell: true,
    }))
}

/// Parse noun phrases whose numeric meaning comes from retained turn events.
/// Callers may pass either the bare noun phrase or a leading "for each".
pub fn parse_turn_history_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let mut tokens = trim_edge_punctuation(tokens);
    let leading = TokenWordView::new(&tokens);
    let leading_words = leading.to_word_refs();
    if leading_words.starts_with(&["for", "each"])
        && let Some(range) = leading.token_span_for_words(2, leading.len())
    {
        tokens = trim_edge_punctuation(&tokens[range]);
    }

    let word_view = TokenWordView::new(&tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let turn_start_untapped_lands_player = match words.as_slice() {
        [
            "untapped",
            "land" | "lands",
            "they",
            "controlled",
            "at",
            "the",
            "beginning",
            "of",
            "this",
            "turn",
        ] => Some(PlayerFilter::IteratedPlayer),
        [
            "untapped",
            "land" | "lands",
            "you",
            "controlled",
            "at",
            "the",
            "beginning",
            "of",
            "this",
            "turn",
        ] => Some(PlayerFilter::You),
        _ => None,
    };
    if let Some(player) = turn_start_untapped_lands_player {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::UntappedLandsAtTurnStart(player),
        ));
    }

    if matches!(
        words.as_slice(),
        [
            "attraction" | "attractions",
            "youve" | "you've",
            "visited",
            "this",
            "turn"
        ]
    ) {
        return Some(Value::AttractionsVisitedThisTurn(PlayerFilter::You));
    }

    if matches!(
        words.as_slice(),
        ["time" | "times", "you", "descended", "this", "turn"]
    ) {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::Descended(
            PlayerFilter::You,
        )));
    }

    // Keep the exact, unqualified creature-death wording on the dedicated
    // value variant. Richer historical filters still use TurnHistoryCount.
    if let Some(value) = parse_creatures_died_this_turn_count_value(&tokens) {
        return Some(value);
    }

    // This composite value ends with the same `spells you've cast this turn`
    // suffix as an ordinary spell-history count. Recognize the whole phrase
    // first so the generic suffix parser does not reinterpret
    // `colors among permanents you control and spells` as an object filter.
    if words
        == [
            "colors",
            "among",
            "permanents",
            "you",
            "control",
            "and",
            "spells",
            "youve",
            "cast",
            "this",
            "turn",
        ]
        || words
            == [
                "colors",
                "among",
                "permanents",
                "you",
                "control",
                "and",
                "spells",
                "you've",
                "cast",
                "this",
                "turn",
            ]
    {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(PlayerFilter::You),
        ));
    }

    if let Some(value) = parse_spell_cast_history_count(&tokens, &word_view, &words) {
        return Some(value);
    }

    for (suffix, controller, default_surface) in [
        (
            &["that", "died", "this", "turn"][..],
            None,
            ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
        ),
        (
            &["that", "died", "under", "your", "control", "this", "turn"][..],
            Some(PlayerFilter::You),
            ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
        ),
    ] {
        if let Some(end) = suffix_start(&words, suffix) {
            let mut filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            let has_suffix_controller = controller.is_some();
            if let Some(controller) = controller {
                filter.controller = Some(controller);
            }
            let controller_surface = if !has_suffix_controller && filter.controller.is_some() {
                ironsmith_core::DeathHistoryControllerSurface::ControlledThenDied
            } else {
                default_surface
            };
            return Some(Value::TurnHistoryCount(TurnHistoryCount::Died {
                filter,
                controller_surface,
            }));
        }
    }

    for suffix in [
        &[
            "that",
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ][..],
        &[
            "you",
            "had",
            "enter",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ][..],
        &[
            "you",
            "had",
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ][..],
    ] {
        if let Some(end) = suffix_start(&words, suffix) {
            let mut filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            filter.controller = Some(PlayerFilter::You);
            return Some(Value::TurnHistoryCount(
                TurnHistoryCount::EnteredBattlefield(filter),
            ));
        }
    }

    if matches!(
        words.as_slice(),
        ["token" | "tokens", "you", "created", "this", "turn"]
            | [
                "token" | "tokens",
                "youve" | "you've",
                "created",
                "this",
                "turn"
            ]
    ) {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::TokensCreated(
            PlayerFilter::You,
        )));
    }

    if matches!(
        words.as_slice(),
        [
            "card" | "cards",
            "youve" | "you've",
            "cycled",
            "or",
            "discarded",
            "this",
            "turn"
        ] | [
            "card" | "cards",
            "you",
            "have",
            "cycled",
            "or",
            "discarded",
            "this",
            "turn"
        ] | [
            "card" | "cards",
            "youve" | "you've",
            "discarded",
            "or",
            "cycled",
            "this",
            "turn"
        ] | [
            "card" | "cards",
            "you",
            "have",
            "discarded",
            "or",
            "cycled",
            "this",
            "turn"
        ]
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::DiscardedOrCycled(PlayerFilter::You),
        ));
    }

    let graveyard_put = words.iter().position(|word| *word == "put");
    let graveyard_prefix = graveyard_put.and_then(|put| words.get(..put));
    let graveyard_tail = graveyard_put.and_then(|put| words.get(put..));
    let valid_graveyard_card_prefix = matches!(
        graveyard_prefix,
        Some(["card" | "cards"] | ["card" | "cards", "that", "were"])
    );
    if valid_graveyard_card_prefix
        && matches!(
            graveyard_tail,
            Some([
                "put",
                "into",
                "your",
                "graveyard",
                "from",
                "your",
                "hand",
                "or",
                "library",
                "this",
                "turn"
            ])
        )
    {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PutIntoGraveyard {
                owner: PlayerFilter::You,
                from: vec![Zone::Hand, Zone::Library],
            },
        ));
    }
    if valid_graveyard_card_prefix
        && matches!(
            graveyard_tail,
            Some([
                "put",
                "into",
                "their",
                "graveyard",
                "from",
                "anywhere",
                "this",
                "turn"
            ])
        )
    {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PutIntoGraveyard {
                owner: PlayerFilter::IteratedPlayer,
                from: Vec::new(),
            },
        ));
    }

    for suffix in [
        &["youve", "sacrificed", "this", "turn"][..],
        &["you've", "sacrificed", "this", "turn"][..],
        &["you", "have", "sacrificed", "this", "turn"][..],
    ] {
        if let Some(end) = suffix_start(&words, suffix) {
            let filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            return Some(Value::TurnHistoryCount(TurnHistoryCount::Sacrificed {
                player: PlayerFilter::You,
                filter,
            }));
        }
    }

    if matches!(
        words.as_slice(),
        ["opponent" | "opponents", "you", "attacked", "this", "turn"]
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::OpponentsAttacked(PlayerFilter::You),
        ));
    }

    if let Some(end) = suffix_start(&words, &["you", "attacked", "with", "this", "turn"]) {
        let filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::CreaturesAttackedWith {
                player: PlayerFilter::You,
                filter,
            },
        ));
    }

    if matches!(
        words.as_slice(),
        [
            "player" | "players",
            "who",
            "discarded",
            "a",
            "card",
            "this",
            "turn"
        ]
    ) {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::PlayersDiscarded(
            PlayerFilter::Any,
        )));
    }
    if matches!(
        words.as_slice(),
        [
            "opponent" | "opponents",
            "who" | "that",
            "was" | "were",
            "dealt",
            "damage",
            "this",
            "turn"
        ]
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PlayersDealtDamage(PlayerFilter::Opponent),
        ));
    }
    if matches!(
        words.as_slice(),
        [
            "opponent" | "opponents",
            "who" | "that",
            "was" | "were",
            "dealt",
            "combat",
            "damage",
            "this",
            "turn"
        ]
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PlayersDealtCombatDamageBy {
                players: PlayerFilter::Opponent,
                sources: ObjectFilter::default(),
            },
        ));
    }
    if matches!(
        words.as_slice(),
        [
            "opponent" | "opponents",
            "who",
            "lost",
            "life",
            "this",
            "turn"
        ] | [
            "your",
            "opponent" | "opponents",
            "who",
            "lost",
            "life",
            "this",
            "turn"
        ] | [
            "of",
            "your",
            "opponent" | "opponents",
            "who",
            "lost",
            "life",
            "this",
            "turn"
        ]
    ) {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::PlayersLostLife(
            PlayerFilter::Opponent,
        )));
    }

    if words.starts_with(&[
        "your",
        "opponents",
        "who",
        "were",
        "dealt",
        "combat",
        "damage",
        "by",
    ]) && words.ends_with(&["this", "turn"])
    {
        let start = 8;
        let end = words.len().saturating_sub(2);
        let range = word_view.token_span_for_words(start, end)?;
        let sources = parse_object_filter(&trim_edge_punctuation(&tokens[range]), false).ok()?;
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PlayersDealtCombatDamageBy {
                players: PlayerFilter::Opponent,
                sources,
            },
        ));
    }

    let outside_hand_suffixes: &[&[&str]] = &[
        &[
            "youve", "cast", "from", "anywhere", "other", "than", "your", "hand", "this", "turn",
        ],
        &[
            "you've", "cast", "from", "anywhere", "other", "than", "your", "hand", "this", "turn",
        ],
        &[
            "you", "have", "cast", "from", "anywhere", "other", "than", "your", "hand", "this",
            "turn",
        ],
        &[
            "youve", "cast", "this", "turn", "from", "anywhere", "other", "than", "your", "hand",
        ],
        &[
            "you've", "cast", "this", "turn", "from", "anywhere", "other", "than", "your", "hand",
        ],
        &[
            "you", "have", "cast", "this", "turn", "from", "anywhere", "other", "than", "your",
            "hand",
        ],
    ];
    for suffix in outside_hand_suffixes {
        if let Some(end) = suffix_start(&words, suffix) {
            let filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            return Some(Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
                player: PlayerFilter::You,
                filter,
                from_zone: None,
                from_outside_hand: true,
                exclude_source: false,
                before_triggering_spell: false,
            }));
        }
    }

    if words.len() >= 8
        && matches!(words.first(), Some(&"+1/+1"))
        && matches!(words.get(1), Some(&"counter") | Some(&"counters"))
        && words.ends_with(&["under", "your", "control", "this", "turn"])
        && let Some(put_on) = words.windows(2).position(|window| window == ["put", "on"])
    {
        let start = put_on + 2;
        let end = words.len().saturating_sub(5);
        let range = word_view.token_span_for_words(start, end)?;
        let mut filter = parse_object_filter(&trim_edge_punctuation(&tokens[range]), false).ok()?;
        filter.zone = None;
        filter.controller = Some(PlayerFilter::You);
        return Some(Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
            counter_type: Some(crate::object::CounterType::PlusOnePlusOne),
            filter,
        }));
    }

    None
}

/// Parse a complete `where X is ...` binding whose value is backed by turn
/// history. This deliberately runs before generic object-count parsing: words
/// such as `graveyard`, `hand`, and `battlefield` describe event provenance in
/// these clauses, not the current zones of objects to count.
pub fn parse_turn_history_value_binding(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation(tokens);
    let word_view = TokenWordView::new(&tokens);
    let words = word_view.to_word_refs();
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let body_range = word_view.token_span_for_words(3, word_view.len())?;
    let body_tokens = trim_edge_punctuation(&tokens[body_range]);
    let body_view = TokenWordView::new(&body_tokens);
    let body_words = body_view.to_word_refs();

    if matches!(
        body_words.as_slice(),
        [
            "the" | "total",
            "amount",
            "of",
            "damage",
            "dealt",
            "to",
            "it",
            "this",
            "turn"
        ] | [
            "the", "total", "amount", "of", "damage", "dealt", "to", "it", "this", "turn"
        ]
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::DamageDealtToSource,
        ));
    }

    for prefix in [
        &["the", "number", "of"][..],
        &["number", "of"][..],
        &["equal", "to", "the", "number", "of"][..],
    ] {
        if body_words.starts_with(prefix) {
            let history_range = body_view.token_span_for_words(prefix.len(), body_view.len())?;
            return parse_turn_history_count_value(&body_tokens[history_range]);
        }
    }

    let plus_word = body_words.iter().position(|word| *word == "plus")?;
    let history_prefix = body_words.get(plus_word..plus_word + 4)?;
    if history_prefix != ["plus", "the", "number", "of"] {
        return None;
    }

    let fixed_range = body_view.token_span_for_words(0, plus_word)?;
    let fixed_tokens = trim_edge_punctuation(&body_tokens[fixed_range]);
    let (fixed, used) = parse_number_prefix_lexed(&fixed_tokens)?;
    if used != fixed_tokens.len() {
        return None;
    }

    let history_range =
        body_view.token_span_for_words(plus_word + history_prefix.len(), body_view.len())?;
    let history = parse_turn_history_count_value(&body_tokens[history_range])?;
    Some(Value::Add(
        Box::new(Value::Fixed(fixed as i32)),
        Box::new(history),
    ))
}

fn parse_spells_cast_this_turn_matching_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let filter_words = word_view.to_word_refs();
    let surface = value_helper_shapes::parse_spell_cast_this_turn_surface(&filter_words)?;
    let filter_token_range = word_view.token_span_for_words(0, surface.filter_end)?;
    let filter_tokens = trim_commas(&tokens[filter_token_range]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::SpellsCastThisTurnMatching {
        player: surface.player,
        filter,
        exclude_source: surface.exclude_source,
    })
}

fn parse_creatures_died_this_turn_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    if words_match_any_phrase(&word_view.to_word_refs(), CREATURES_DIED_THIS_TURN_PHRASES) {
        Some(Value::CreaturesDiedThisTurn)
    } else {
        None
    }
}

fn parse_cards_discarded_this_turn_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = TokenWordView::new(tokens).to_word_refs();
    value_helper_shapes::parse_cards_discarded_this_turn_player(&words)
        .map(Value::CardsDiscardedThisTurn)
}

pub fn parse_commander_cast_count_player(tokens: &[OwnedLexToken]) -> Option<PlayerFilter> {
    let words = TokenWordView::new(tokens).to_word_refs();
    value_helper_shapes::parse_commander_cast_count_player(&words)
}

pub fn parse_equal_to_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let words_all = word_view.to_word_refs();
    // Callers that have already split an `equal to` clause pass only the
    // amount tail (`the number of ...`). Accept that typed amount directly as
    // well as the unsplit authored clause.
    let prefix_start = parse_equal_to_start(&words_all)
        .map(|start| start.after)
        .unwrap_or(0);
    let suffix_refs = words_all.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let number_word_idx = prefix_start + matched.number_of_start;

    let value_range = word_view.token_span_for_words(number_word_idx, word_view.len())?;
    let value_tokens = trim_edge_punctuation(&tokens[value_range]);
    let filter_start_word_idx = number_word_idx + 2;
    let filter_range = word_view.token_span_for_words(filter_start_word_idx, word_view.len())?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_range]);
    let filter_word_view = TokenWordView::new(&filter_tokens);
    let filter_words = filter_word_view.to_word_refs();
    // A relative controller clause scopes the counted set to the object
    // targeted by this same effect. Parse the set independently from the
    // back-reference so characteristic words in `that creature's controller`
    // cannot leak into the counted filter as an additional Creature type.
    if let Some(that_idx) = filter_words.iter().rposition(|word| *word == "that") {
        let relative = possessive_normalized_word_refs(&filter_words[that_idx..]);
        let relative_noun = relative.get(1).map(|word| word.trim_end_matches('s'));
        if relative.len() == 4
            && relative[0] == "that"
            && matches!(
                relative_noun,
                Some("creature" | "permanent" | "object" | "planeswalker")
            )
            && relative[2] == "controller"
            && relative[3] == "controls"
            && that_idx > 0
        {
            let base_range = filter_word_view.token_span_for_words(0, that_idx)?;
            let mut filter =
                parse_object_filter(&trim_edge_punctuation(&filter_tokens[base_range]), false)
                    .ok()?;
            filter.controller = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
            return Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }
    if let Some(value) = parse_turn_history_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = parse_cards_discarded_this_turn_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some((players, minimum)) = parse_players_with_cards_in_hand_at_least(&filter_tokens) {
        return Some(
            Value::CountPlayersWithCardsInHandAtLeast(players, minimum)
                .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    if let Some(player) = value_helper_shapes::parse_cards_in_hand_player(&filter_words) {
        let mut value = Value::CardsInHand(player).with_surface_hint(ValueSurfaceHint::EqualTo);
        if value_helper_shapes::has_that_player_possessive(&filter_words) {
            value = value.with_surface_hint(ValueSurfaceHint::ThatPlayerPossessive);
        }
        return Some(value);
    }
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(player) = value_helper_shapes::parse_party_size_player(&filter_words) {
        return Some(Value::PartySize(player).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    let mut for_each_words = vec!["for", "each"];
    for_each_words.extend(filter_words.iter().copied());
    if let Some((value @ Value::PendingPriorEffectMetric(_), used)) =
        super::count_shapes::parse_for_each_count_value_words(&for_each_words)
        && used == for_each_words.len()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(distinct_filter_tokens) =
        primitives::parse_word_sequence_prefix(&filter_words, &["differently", "named"]).and_then(
            |remaining| {
                let consumed = filter_words.len().saturating_sub(remaining.len());
                filter_word_view
                    .token_span_for_words(consumed, filter_word_view.len())
                    .map(|range| &filter_tokens[range])
            },
        )
    {
        let filter = parse_object_filter(distinct_filter_tokens, false).ok()?;
        return Some(Value::DistinctNames(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some((value, used)) = value_expr::parse_value_expr_tokens(&value_tokens)
        && TokenWordView::new(&value_tokens[used..]).is_empty()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo))
}

pub fn parse_players_with_cards_in_hand_at_least(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerFilter, u32)> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let with_idx = words.iter().position(|word| *word == "with")?;
    let players = match &words[..with_idx] {
        ["your", "opponents"] | ["opponents"] => PlayerFilter::Opponent,
        ["players"] | ["each", "player"] => PlayerFilter::Any,
        ["other", "players"] => PlayerFilter::NotYou,
        ["you"] => PlayerFilter::You,
        _ => return None,
    };
    let threshold_range = word_view.token_span_for_words(with_idx + 1, word_view.len())?;
    let threshold_tokens = trim_edge_punctuation(&tokens[threshold_range]);
    let (minimum, used) = parse_greater_than_or_equal_quantity_prefix(
        &threshold_tokens,
        false,
        false,
        "player hand-size count",
    )
    .ok()
    .flatten()?;
    let remainder = TokenWordView::new(&threshold_tokens[used..]).to_word_refs();
    matches!(
        remainder.as_slice(),
        ["card" | "cards", "in", "hand"] | ["card" | "cards", "in", "their", "hand"]
    )
    .then_some((players, minimum))
}

pub fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let clause_words = word_view.to_word_refs();
    if parse_equal_to_start(&clause_words).is_none_or(|parsed| parsed.start != 0) {
        return None;
    }

    let suffix_refs = clause_words.get(EQUAL_TO_PHRASE.len()..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let filter_start_word_idx = EQUAL_TO_PHRASE.len() + matched.consumed;
    let operator_word_idx =
        word_view.find_any_word_from(&["plus", "minus"], filter_start_word_idx + 1)?;
    let operator = clause_words[operator_word_idx];

    let filter_range = word_view.token_span_for_words(filter_start_word_idx, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_range]);
    let base_value = if let Some(value) = parse_turn_history_count_value(&filter_tokens) {
        value
    } else if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens) {
        value
    } else if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        value
    } else if let Some(player) = value_helper_shapes::parse_party_size_player(
        &TokenWordView::new(&filter_tokens).to_word_refs(),
    ) {
        Value::PartySize(player)
    } else {
        Value::Count(parse_object_filter(&filter_tokens, false).ok()?)
    };

    let offset_range = word_view.token_span_for_words(operator_word_idx + 1, word_view.len())?;
    let offset_tokens = trim_commas(&tokens[offset_range]);
    let (offset_value, used) =
        leaf::parse_leaf_number_prefix_tokens(&offset_tokens)?.into_fixed()?;
    if !TokenWordView::new(&offset_tokens[used..]).is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(
        Value::Add(Box::new(base_value), Box::new(Value::Fixed(signed_offset)))
            .with_surface_hint(ValueSurfaceHint::EqualTo),
    )
}

pub fn parse_equal_to_number_of_opponents_you_have_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if value_helper_shapes::starts_equal_to_opponents_you_have(&clause_refs) {
        return Some(
            Value::CountPlayers(PlayerFilter::Opponent)
                .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    None
}

pub fn parse_equal_to_number_of_counters_on_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let shape = value_helper_shapes::parse_counter_reference_value_shape(&words)?;
    Some(counter_reference_shape_value(shape).with_surface_hint(ValueSurfaceHint::EqualTo))
}

pub fn parse_equal_to_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let prefix_start = parse_equal_to_start(&clause_refs)?.after;
    let suffix_refs = clause_refs.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_aggregate_prefix(suffix_refs)?;
    let aggregate = matched.aggregate;
    let value_kind = matched.value_kind;
    let idx = prefix_start + matched.consumed;

    if aggregate == value_helper_shapes::AggregateKind::Greatest
        && value_kind == value_helper_shapes::AggregateValueKind::ManaValue
        && let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx)
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }

    let filter_range = clause_words.token_span_for_words(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if aggregate == value_helper_shapes::AggregateKind::Total
        && value_kind == value_helper_shapes::AggregateValueKind::ManaValue
        && let Some(Value::SpellsCastThisTurnMatching {
            player,
            mut filter,
            exclude_source,
        }) = parse_spells_cast_this_turn_matching_count_value(filter_tokens)
    {
        // `other` in this history phrase is relative to the spell whose value
        // is being evaluated. It is carried explicitly by `exclude_source`;
        // leaving it on the snapshot filter would apply a second, context-
        // dependent object relation.
        filter.other = false;
        return Some(
            Value::TotalManaValueOfSpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source,
            }
            .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    if value_kind == value_helper_shapes::AggregateValueKind::ManaValue
        && let Some(value) = source_linked_exiled_mana_value(object_words)
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = pending_aggregate_metric_value(aggregate, value_kind, object_words) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    let mut filter = parse_object_filter(filter_tokens, false).ok()?;
    if object_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
    {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }

    Some(
        aggregate_filter_value(aggregate, value_kind, filter)
            .with_surface_hint(ValueSurfaceHint::EqualTo),
    )
}

pub fn parse_where_x_greatest_commander_mana_value(
    tokens: &[OwnedLexToken],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let words = TokenWordView::new(tokens);
    let commander_range = words.token_span_for_words(commander_start_word_idx, words.len())?;
    let commander_words = crate::lexer::token_word_refs(&tokens[commander_range]);
    let normalized = commander_words
        .iter()
        .copied()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect::<Vec<_>>();
    let owner = commander_owner_from_battlefield_or_command_zone_words(&normalized)?;

    let mut battlefield_commander = ObjectFilter::default();
    battlefield_commander.zone = Some(Zone::Battlefield);
    battlefield_commander.is_commander = true;
    battlefield_commander.owner = Some(owner);

    let mut command_zone_commander = battlefield_commander.clone();
    command_zone_commander.zone = Some(Zone::Command);

    let mut combined = ObjectFilter::default();
    combined.any_of = vec![battlefield_commander, command_zone_commander];

    Some(Value::GreatestManaValue(combined))
}

fn commander_owner_from_battlefield_or_command_zone_words(words: &[&str]) -> Option<PlayerFilter> {
    if words == COMMANDER_YOU_OWN_BATTLEFIELD_OR_COMMAND_ZONE_PHRASE {
        return Some(PlayerFilter::You);
    }
    if words_match_any_phrase(
        words,
        COMMANDER_ITERATED_PLAYER_OWNS_BATTLEFIELD_OR_COMMAND_ZONE_PHRASES,
    ) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    None
}

pub fn parse_spells_cast_this_turn_matching_count_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_words = TokenWordView::new(tokens);
    let word_refs = filter_words.to_word_refs();
    let surface = value_helper_shapes::parse_spell_cast_this_turn_surface(&word_refs)?;
    let filter_token_range = filter_words.token_span_for_words(0, surface.filter_end)?;
    let filter_tokens = trim_lexed_commas(&tokens[filter_token_range]);
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(Value::SpellsCastThisTurnMatching {
        player: surface.player,
        filter,
        exclude_source: surface.exclude_source,
    })
}

pub fn starts_explicit_ordered_comparison(
    tokens: &[&str],
    operator: ValueComparisonOperator,
) -> bool {
    match operator {
        ValueComparisonOperator::LessThanOrEqual => matches!(
            tokens,
            ["less", "than", "or", "equal", "to", ..]
                | ["is", "less", "than", "or", "equal", "to", ..]
        ),
        ValueComparisonOperator::GreaterThanOrEqual => matches!(
            tokens,
            ["greater", "than", "or", "equal", "to", ..]
                | ["is", "greater", "than", "or", "equal", "to", ..]
        ),
        _ => false,
    }
}

pub fn parse_filter_comparison_tokens(
    axis: &str,
    tokens: &[&str],
    clause_words: &[&str],
) -> Result<Option<(crate::filter::Comparison, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if is_power_toughness_axis_word(axis) && value_helper_shapes::starts_or_power_toughness(tokens)
    {
        return Ok(None);
    }

    let to_comparison = |operator: ValueComparisonOperator,
                         operand: Value|
     -> crate::filter::Comparison {
        use crate::filter::Comparison;

        match (operator, operand) {
            (ValueComparisonOperator::Equal, Value::Fixed(value)) => Comparison::Equal(value),
            (ValueComparisonOperator::NotEqual, Value::Fixed(value)) => Comparison::NotEqual(value),
            (ValueComparisonOperator::LessThan, Value::Fixed(value)) => Comparison::LessThan(value),
            (ValueComparisonOperator::LessThanOrEqual, Value::Fixed(value)) => {
                Comparison::LessThanOrEqual(value)
            }
            (ValueComparisonOperator::GreaterThan, Value::Fixed(value)) => {
                Comparison::GreaterThan(value)
            }
            (ValueComparisonOperator::GreaterThanOrEqual, Value::Fixed(value)) => {
                Comparison::GreaterThanOrEqual(value)
            }
            (ValueComparisonOperator::Equal, operand) => Comparison::EqualExpr(Box::new(operand)),
            (ValueComparisonOperator::NotEqual, operand) => {
                Comparison::NotEqualExpr(Box::new(operand))
            }
            (ValueComparisonOperator::LessThan, operand) => {
                Comparison::LessThanExpr(Box::new(operand))
            }
            (ValueComparisonOperator::LessThanOrEqual, operand) => {
                Comparison::LessThanOrEqualExpr(Box::new(operand))
            }
            (ValueComparisonOperator::GreaterThan, operand) => {
                Comparison::GreaterThanExpr(Box::new(operand))
            }
            (ValueComparisonOperator::GreaterThanOrEqual, operand) => {
                Comparison::GreaterThanOrEqualExpr(Box::new(operand))
            }
        }
    };

    let parse_operand = |operand_tokens: &[&str],
                         operator: ValueComparisonOperator|
     -> Result<(crate::filter::Comparison, usize), CardTextError> {
        let Some((operand, used)) = value_expr::parse_value_expr_words(operand_tokens) else {
            let quoted = operand_tokens
                .first()
                .copied()
                .unwrap_or_default()
                .to_string();
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        Ok((to_comparison(operator, operand), used))
    };

    let parse_numeric_token = |word: &str| -> Option<i32> {
        if let Ok(value) = word.parse::<i32>() {
            return Some(value);
        }
        leaf::parse_number_i32_complete(word).ok()
    };

    let first = tokens[0];
    if let Some(value) = parse_numeric_token(first) {
        if tokens.get(1).is_some_and(|word| is_plus_minus_word(word)) {
            let (cmp, used) = parse_operand(tokens, ValueComparisonOperator::Equal)?;
            return Ok(Some((cmp, used)));
        }
        let mut values = vec![value];
        let mut consumed = 1usize;
        while consumed < tokens.len() {
            let token = tokens[consumed];
            if is_and_or_word(token) {
                consumed += 1;
                continue;
            }
            if let Some(next_value) = parse_numeric_token(token) {
                values.push(next_value);
                consumed += 1;
                continue;
            }
            break;
        }
        if values.len() > 1 {
            return Ok(Some((crate::filter::Comparison::OneOf(values), consumed)));
        }
        if tokens.len() == 1 {
            return Ok(Some((crate::filter::Comparison::Equal(value), 1)));
        }
    }

    if let Some((operator, operand_words, consumed_base)) = parse_value_comparison_words(tokens) {
        if operand_words.is_empty() {
            let consumed_phrase = consumed_base;
            let phrase = tokens[..consumed_phrase].join(" ");
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{phrase}' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (operand, used) =
            value_expr::parse_value_expr_words(operand_words).ok_or_else(|| {
                let quoted = operand_words.first().copied().unwrap_or_default();
                CardTextError::ParseError(format!(
                    "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let operand = if starts_explicit_ordered_comparison(tokens, operator)
            && !matches!(operand.unhinted(), Value::Fixed(_))
        {
            operand.with_surface_hint(ValueSurfaceHint::ExplicitComparison)
        } else {
            operand
        };
        let consumed = consumed_base + used;
        return Ok(Some((to_comparison(operator, operand), consumed)));
    }

    if let Some((value, used)) = value_expr::parse_value_expr_words(tokens) {
        if tokens.get(used).copied() == Some("or")
            && let Some(next) = tokens.get(used + 1)
            && is_comparison_tail_word(next)
        {
            let operator = if is_less_or_fewer_word(next) {
                ValueComparisonOperator::LessThanOrEqual
            } else {
                ValueComparisonOperator::GreaterThanOrEqual
            };
            return Ok(Some((to_comparison(operator, value), used + 2)));
        }
        if let Value::Fixed(fixed) = value
            && used == 1
        {
            return Ok(Some((crate::filter::Comparison::Equal(fixed), used)));
        }
        return Ok(Some((
            crate::filter::Comparison::EqualExpr(Box::new(value)),
            used,
        )));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardType;

    fn lex_words(text: &str) -> Vec<OwnedLexToken> {
        let mut tokens = crate::lexer::lex_line(text, 0).expect("test phrase should lex");
        for token in &mut tokens {
            token.lowercase_word();
        }
        tokens
    }

    #[test]
    fn equal_to_parser_returns_typed_word_boundaries() {
        assert_eq!(
            parse_equal_to_start(&["where", "x", "is", "equal", "to", "the"]),
            Some(EqualToStart { start: 3, after: 5 })
        );
        assert_eq!(parse_equal_to_start(&["not", "equal"]), None);
    }

    #[test]
    fn counted_set_keeps_the_same_effect_target_controller_relation() {
        let value = parse_equal_to_number_of_filter_value(&lex_words(
            "equal to the number of nonbasic lands that creature's controller controls",
        ))
        .expect("relative target-controller count should parse");
        let Value::SurfaceHinted { value, .. } = value else {
            panic!("equal-to surface should be retained: {value:?}");
        };
        let Value::Count(filter) = *value else {
            panic!("expected an object count: {value:?}");
        };
        assert_eq!(filter.card_types, vec![CardType::Land]);
        assert!(!filter.card_types.contains(&CardType::Creature));
        assert!(
            filter
                .excluded_supertypes
                .contains(&crate::Supertype::Basic)
        );
        assert_eq!(
            filter.controller,
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
        );
    }

    #[test]
    fn mana_symbol_spent_value_preserves_symbol_and_cast_reference() {
        for (text, expected_symbol, expected_reference) in [
            (
                "where X is the amount of {S} spent to cast this spell",
                crate::mana::ManaSymbol::Snow,
                ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
            ),
            (
                "the amount of {U} spent to cast it",
                crate::mana::ManaSymbol::Blue,
                ironsmith_core::ManaSpentCastReferenceSurface::It,
            ),
            (
                "amount of {G} spent to cast this creature",
                crate::mana::ManaSymbol::Green,
                ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature,
            ),
        ] {
            assert_eq!(
                parse_mana_symbol_spent_to_cast_value(&lex_words(text)),
                Some(Value::ManaSymbolSpentToCastThisSpell {
                    symbol: expected_symbol,
                    reference: expected_reference,
                }),
                "{text}",
            );
        }
    }

    #[test]
    fn mana_symbol_spent_value_rejects_non_exact_or_multi_symbol_surfaces() {
        for text in [
            "where X is the amount of {S}{S} spent to cast this spell",
            "where X is the amount of {S} spent to cast this spell, then draw a card",
            "where X is the number of {S} spent to cast this spell",
        ] {
            assert!(
                parse_mana_symbol_spent_to_cast_value(&lex_words(text)).is_none(),
                "{text}",
            );
        }
    }

    #[test]
    fn dynamic_filter_comparisons_preserve_prefix_vs_postfix_surface() {
        let prefix = ["less", "than", "or", "equal", "to", "your", "life", "total"];
        let (prefix_comparison, prefix_used) =
            parse_filter_comparison_tokens("power", &prefix, &prefix)
                .expect("comparison parse should succeed")
                .expect("explicit comparison should parse");
        let crate::filter::Comparison::LessThanOrEqualExpr(prefix_value) = prefix_comparison else {
            panic!("expected dynamic less-than-or-equal comparison");
        };
        assert_eq!(prefix_used, prefix.len());
        assert!(prefix_value.has_surface_hint(ValueSurfaceHint::ExplicitComparison));

        let postfix = ["your", "life", "total", "or", "less"];
        let (postfix_comparison, postfix_used) =
            parse_filter_comparison_tokens("power", &postfix, &postfix)
                .expect("comparison parse should succeed")
                .expect("postfix comparison should parse");
        let crate::filter::Comparison::LessThanOrEqualExpr(postfix_value) = postfix_comparison
        else {
            panic!("expected dynamic less-than-or-equal comparison");
        };
        assert_eq!(postfix_used, postfix.len());
        assert!(!postfix_value.has_surface_hint(ValueSurfaceHint::ExplicitComparison));

        let greater_prefix = [
            "is", "greater", "than", "or", "equal", "to", "your", "life", "total",
        ];
        let (greater_comparison, _) =
            parse_filter_comparison_tokens("power", &greater_prefix, &greater_prefix)
                .expect("comparison parse should succeed")
                .expect("explicit greater-than comparison should parse");
        let crate::filter::Comparison::GreaterThanOrEqualExpr(greater_value) = greater_comparison
        else {
            panic!("expected dynamic greater-than-or-equal comparison");
        };
        assert!(greater_value.has_surface_hint(ValueSurfaceHint::ExplicitComparison));
    }

    #[test]
    fn parse_aggregate_scope_value_lexed_uses_captured_metric_and_scope() {
        let color_tokens = lex_words("colors among creatures you control");
        let color_value = parse_aggregate_scope_value_lexed(&color_tokens)
            .expect("colors-among aggregate should parse");
        let Value::ColorsAmong(color_filter) = color_value else {
            panic!("expected colors-among value, got {color_value:?}");
        };
        assert_eq!(color_filter.card_types, vec![CardType::Creature]);
        assert_eq!(color_filter.controller, Some(PlayerFilter::You));

        let power_tokens = lex_words("different powers among creatures you control");
        let power_value = parse_aggregate_scope_value_lexed(&power_tokens)
            .expect("distinct-powers aggregate should parse");
        let Value::DistinctPowers(power_filter) = power_value else {
            panic!("expected distinct-powers value, got {power_value:?}");
        };
        assert_eq!(power_filter.card_types, vec![CardType::Creature]);
        assert_eq!(power_filter.controller, Some(PlayerFilter::You));

        let name_tokens = lex_words("differently named lands you control");
        let name_value = parse_aggregate_scope_value_lexed(&name_tokens)
            .expect("distinct-name aggregate should parse");
        let Value::DistinctNames(name_filter) = name_value else {
            panic!("expected distinct-name value, got {name_value:?}");
        };
        assert_eq!(name_filter.card_types, vec![CardType::Land]);
        assert_eq!(name_filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn parse_spells_cast_this_turn_matching_count_value_lexed_uses_captured_suffix() {
        let tokens = lex_words("other creature spells an opponent has cast this turn");
        let value = parse_spells_cast_this_turn_matching_count_value_lexed(&tokens)
            .expect("spell-cast count should parse");
        let Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } = value
        else {
            panic!("expected spell-cast matching value, got {value:?}");
        };
        assert_eq!(player, PlayerFilter::Opponent);
        assert!(exclude_source);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
    }

    #[test]
    fn parse_total_mana_value_of_other_spells_cast_this_turn_as_history_aggregate() {
        let tokens =
            lex_words("equal to the total mana value of other spells you've cast this turn");
        let value = parse_equal_to_aggregate_filter_value(&tokens)
            .expect("spell-cast mana-value aggregate should parse");
        let Value::SurfaceHinted { value, .. } = value else {
            panic!("expected equal-to surface hint");
        };
        let Value::TotalManaValueOfSpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } = value.as_ref()
        else {
            panic!("expected spell-history mana-value aggregate, got {value:?}");
        };
        assert_eq!(*player, PlayerFilter::You);
        assert!(*exclude_source);
        assert!(!filter.other, "source exclusion is carried by the query");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
    }

    #[test]
    fn turn_history_counts_keep_event_metric_and_typed_filters() {
        let cases = [
            ("Zubera that died this turn", "Died"),
            (
                "nontoken creatures that died under your control this turn",
                "Died",
            ),
            (
                "nontoken creatures you controlled that died this turn",
                "Died",
            ),
            ("tokens you created this turn", "TokensCreated"),
            (
                "lands that entered the battlefield under your control this turn",
                "EnteredBattlefield",
            ),
            (
                "cards that were put into your graveyard from your hand or library this turn",
                "PutIntoGraveyard",
            ),
            (
                "spells you've cast from anywhere other than your hand this turn",
                "SpellsCast",
            ),
            (
                "instant and sorcery spells you've cast this turn",
                "SpellsCast",
            ),
            (
                "instant and sorcery spells cast before that spell this turn",
                "SpellsCast",
            ),
            (
                "colors among permanents you control and spells you've cast this turn",
                "ColorsAmongPermanentsAndSpellsCast",
            ),
            (
                "+1/+1 counters you've put on creatures under your control this turn",
                "CountersPutOn",
            ),
            (
                "untapped lands they controlled at the beginning of this turn",
                "UntappedLandsAtTurnStart",
            ),
            ("times you descended this turn", "Descended"),
        ];

        for (text, expected) in cases {
            let value = parse_turn_history_count_value(&lex_words(text))
                .unwrap_or_else(|| panic!("history count should parse: {text}"));
            let debug = format!("{value:?}");
            assert!(
                debug.contains("TurnHistoryCount") && debug.contains(expected),
                "{text}: {debug}"
            );
        }
    }

    #[test]
    fn death_history_counts_preserve_authored_controller_order() {
        for (text, expected_surface) in [
            (
                "nontoken creatures that died under your control this turn",
                ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
            ),
            (
                "nontoken creatures you controlled that died this turn",
                ironsmith_core::DeathHistoryControllerSurface::ControlledThenDied,
            ),
        ] {
            let value = parse_turn_history_count_value(&lex_words(text))
                .unwrap_or_else(|| panic!("history count should parse: {text}"));
            let Value::TurnHistoryCount(TurnHistoryCount::Died {
                filter,
                controller_surface,
            }) = value
            else {
                panic!("expected a typed death-history count for {text}: {value:?}");
            };
            assert_eq!(filter.controller, Some(PlayerFilter::You), "{text}");
            assert_eq!(controller_surface, expected_surface, "{text}");
        }
    }

    #[test]
    fn spell_cast_history_distinguishes_turn_counts_from_trigger_boundaries() {
        let cases = [
            (
                "instant and sorcery spells you've cast this turn",
                PlayerFilter::You,
                false,
                false,
            ),
            (
                "other spells you've cast this turn",
                PlayerFilter::You,
                true,
                false,
            ),
            (
                "instant and sorcery spells cast before that spell this turn",
                PlayerFilter::Any,
                false,
                true,
            ),
            (
                "other instant and sorcery spells you've cast before it this turn",
                PlayerFilter::You,
                true,
                true,
            ),
        ];

        for (text, expected_player, expected_other, expected_boundary) in cases {
            let value = parse_turn_history_count_value(&lex_words(text))
                .unwrap_or_else(|| panic!("spell history should parse: {text}"));
            let Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
                player,
                filter,
                exclude_source,
                before_triggering_spell,
                ..
            }) = value
            else {
                panic!("expected spell-cast history for {text}: {value:?}");
            };
            assert_eq!(player, expected_player, "{text}");
            assert_eq!(exclude_source, expected_other, "{text}");
            assert_eq!(before_triggering_spell, expected_boundary, "{text}");
            assert!(!filter.other, "other belongs to the history query: {text}");
            assert_eq!(
                filter.stack_kind,
                Some(crate::filter::StackObjectKind::Spell),
                "{text}"
            );
        }
    }

    #[test]
    fn fixed_plus_spell_history_bindings_cover_rionya_and_thunder_surfaces() {
        for (text, expected_fixed, expected_other) in [
            (
                "where X is one plus the number of instant and sorcery spells you've cast this turn",
                1,
                false,
            ),
            (
                "where X is 2 plus the number of other spells you've cast this turn",
                2,
                true,
            ),
        ] {
            let parsed = parse_turn_history_value_binding(&lex_words(text))
                .unwrap_or_else(|| panic!("fixed-plus cast history should parse: {text}"));
            let Value::Add(fixed, history) = parsed else {
                panic!("expected fixed-plus value for {text}: {parsed:?}");
            };
            assert_eq!(*fixed, Value::Fixed(expected_fixed));
            assert!(matches!(
                *history,
                Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
                    player: PlayerFilter::You,
                    exclude_source,
                    before_triggering_spell: false,
                    ..
                }) if exclude_source == expected_other
            ));
        }
    }

    #[test]
    fn turn_history_where_bindings_precede_current_zone_counts() {
        let attractions = parse_turn_history_value_binding(&lex_words(
            "where X is the number of Attractions you've visited this turn",
        ))
        .expect("Attraction visit history should parse");
        assert_eq!(
            attractions,
            Value::AttractionsVisitedThisTurn(PlayerFilter::You)
        );

        let graveyard = parse_turn_history_value_binding(&lex_words(
            "where X is the number of cards put into their graveyard from anywhere this turn",
        ))
        .expect("graveyard provenance count should parse");
        let Value::TurnHistoryCount(TurnHistoryCount::PutIntoGraveyard { owner, from }) = graveyard
        else {
            panic!("expected graveyard-history value, got {graveyard:?}");
        };
        assert_eq!(owner, PlayerFilter::IteratedPlayer);
        assert!(from.is_empty());

        let spells = parse_turn_history_value_binding(&lex_words(
            "where X is 1 plus the number of spells you've cast from anywhere other than your hand this turn",
        ))
        .expect("fixed-plus spell provenance count should parse");
        let Value::Add(fixed, history) = spells else {
            panic!("expected fixed-plus history value, got {spells:?}");
        };
        assert_eq!(*fixed, Value::Fixed(1));
        let Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
            player,
            filter,
            from_outside_hand,
            ..
        }) = *history
        else {
            panic!("expected spell-cast history value, got {history:?}");
        };
        assert_eq!(player, PlayerFilter::You);
        assert!(from_outside_hand);
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
        assert!(!filter.has_mana_cost);
    }

    #[test]
    fn dynamic_token_where_bindings_use_typed_turn_history_values() {
        let descended = parse_turn_history_value_binding(&lex_words(
            "where X is the number of times you descended this turn",
        ))
        .expect("descend count should parse");
        assert!(matches!(
            descended,
            Value::TurnHistoryCount(TurnHistoryCount::Descended(PlayerFilter::You))
        ));

        let damage = parse_turn_history_value_binding(&lex_words(
            "where X is the amount of damage dealt to it this turn",
        ))
        .expect("source damage total should parse");
        assert!(matches!(
            damage,
            Value::TurnHistoryCount(TurnHistoryCount::DamageDealtToSource)
        ));
    }

    #[test]
    fn parses_opponents_dealt_combat_damage_without_a_source_qualifier() {
        let value = parse_turn_history_count_value(&lex_words(
            "opponents that were dealt combat damage this turn",
        ))
        .expect("combat-damaged opponent count should parse");
        assert!(matches!(
            value,
            Value::TurnHistoryCount(TurnHistoryCount::PlayersDealtCombatDamageBy {
                players: PlayerFilter::Opponent,
                sources,
            }) if sources == ObjectFilter::default()
        ));
    }

    #[test]
    fn parses_authored_possessive_opponents_who_lost_life_count() {
        let value = parse_turn_history_count_value(&lex_words(
            "for each of your opponents who lost life this turn",
        ))
        .expect("distinct opponents who lost life should parse");
        assert!(matches!(
            value,
            Value::TurnHistoryCount(TurnHistoryCount::PlayersLostLife(PlayerFilter::Opponent))
        ));
    }

    #[test]
    fn turn_history_values_require_complete_supported_provenance_surfaces() {
        assert!(
            parse_turn_history_count_value(&lex_words(
                "Zubera that died this turn among creatures you control"
            ))
            .is_none()
        );
        assert!(
            parse_turn_history_count_value(&lex_words(
                "cards with flying put into your graveyard from your hand or library this turn"
            ))
            .is_none()
        );
        assert!(
            parse_turn_history_value_binding(&lex_words(
                "where X is the number of cards put into their graveyard from anywhere this turn plus one"
            ))
            .is_none()
        );
        assert!(
            parse_turn_history_count_value(&lex_words("Treasure tokens you created this turn"))
                .is_none(),
            "typed created-token counts require a token-filter/creator-aware model"
        );
    }

    #[test]
    fn equal_to_number_of_differently_named_objects_keeps_distinctness() {
        let tokens =
            lex_words("equal to the number of differently named creature tokens you control");
        let value = parse_equal_to_number_of_filter_value(&tokens)
            .expect("Audience with Trostani count should parse");
        let Value::SurfaceHinted { value, hints } = value else {
            panic!("expected equal-to surface hint");
        };
        assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
        let Value::DistinctNames(filter) = *value else {
            panic!("expected a distinct-name count");
        };
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.token);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.name, None);
    }

    #[test]
    fn equal_to_hand_count_preserves_authored_that_player_possessive() {
        for (text, expected_hint) in [
            ("equal to the number of cards in that player's hand", true),
            ("equal to the number of cards in their hand", false),
        ] {
            let value = parse_equal_to_number_of_filter_value(&lex_words(text))
                .unwrap_or_else(|| panic!("player-relative hand count should parse: {text}"));
            assert!(
                value.has_surface_hint(ValueSurfaceHint::EqualTo),
                "{value:#?}"
            );
            assert_eq!(
                value.has_surface_hint(ValueSurfaceHint::ThatPlayerPossessive),
                expected_hint,
                "{text}: {value:#?}"
            );
            assert!(matches!(
                value.unhinted(),
                Value::CardsInHand(PlayerFilter::IteratedPlayer)
            ));
        }
    }

    #[test]
    fn equal_to_number_of_players_with_minimum_hand_size_keeps_both_filters() {
        let value = parse_equal_to_number_of_filter_value(&lex_words(
            "equal to the number of your opponents with four or more cards in hand",
        ))
        .expect("qualified opponent count should parse");
        let Value::SurfaceHinted { value, hints } = value else {
            panic!("expected equal-to surface hint");
        };
        assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
        assert_eq!(
            *value,
            Value::CountPlayersWithCardsInHandAtLeast(PlayerFilter::Opponent, 4)
        );
    }

    #[test]
    fn minimum_hand_size_player_count_does_not_claim_other_count_domains() {
        for text in [
            "creatures with four or more cards in hand",
            "your opponents with four or fewer cards in hand",
            "your opponents with four or more cards in graveyard",
            "cards in your opponents' hands",
        ] {
            assert!(
                parse_players_with_cards_in_hand_at_least(&lex_words(text)).is_none(),
                "the qualified-player parser must not claim {text:?}"
            );
        }
    }

    #[test]
    fn equal_to_number_of_tapped_this_way_keeps_typed_action() {
        let value = parse_equal_to_number_of_filter_value(&lex_words(
            "equal to the number of creatures tapped this way",
        ))
        .expect("tapped-this-way equal count should parse");
        let Value::SurfaceHinted { value, hints } = value else {
            panic!("expected equal-to surface hint");
        };
        assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
        let Value::PendingPriorEffectMetric(query) = *value else {
            panic!("expected typed tapped prior-effect metric");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Tapped)
        );
        assert_eq!(query.metric, EffectMetric::Count);
    }

    #[test]
    fn equal_to_party_count_plus_fixed_keeps_typed_party_value() {
        let value = parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&lex_words(
            "equal to the number of creatures in your party plus two",
        ))
        .expect("equal-to party offset should parse");
        let Value::SurfaceHinted { value, hints } = value else {
            panic!("expected equal-to surface hint");
        };
        assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
        assert_eq!(
            *value,
            Value::Add(
                Box::new(Value::PartySize(PlayerFilter::You)),
                Box::new(Value::Fixed(2)),
            )
        );
    }
}
