use crate::TagKey;
use crate::cards::builders::IT_TAG;
use crate::effect::Value;
use crate::grammar::filters::{parse_counter_type_from_tokens, parse_counter_type_words};
use crate::lexer::synthetic_word_tokens;
use crate::object_filters::parse_object_filter_words;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::util::{
    is_article, source_choose_spec_for_surface, source_reference_surface_for_words,
    this_source_surface_for_words,
};

use super::super::permission_shapes;
use super::value_helper_shapes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ForEachHead {
    item_start: usize,
    other: bool,
}

fn parse_for_each_object_filter_words(
    words: &[&str],
    leading_other: bool,
) -> Option<crate::target::ObjectFilter> {
    // Route all count filters through the lexed grammar entrypoint so
    // independently scoped repeated-`each` domains become typed union arms
    // before the permissive legacy word parser can collapse them. If the
    // count head consumed a leading `other`, restore it as an authored token:
    // in "other Assassins you control and Assassin cards in your graveyard"
    // that qualifier belongs only to the first arm.
    let mut restored = Vec::with_capacity(words.len() + usize::from(leading_other));
    if leading_other {
        restored.push("other");
    }
    restored.extend_from_slice(words);
    let tokens = synthetic_word_tokens(&restored);
    if let Some(filter) =
        crate::grammar::filters::parse_subtype_color_shared_card_union_lexed(&tokens, false)
    {
        return Some(filter);
    }
    crate::object_filters::parse_object_filter_lexed(&tokens, false).ok()
}

pub fn mana_from_source_spent_to_cast_value(source_words: &[&str]) -> Option<Value> {
    mana_from_source_spent_to_cast_value_with_reference(
        source_words,
        ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
    )
}

pub fn mana_from_source_spent_to_cast_value_with_reference(
    source_words: &[&str],
    reference: ironsmith_core::ManaSpentCastReferenceSurface,
) -> Option<Value> {
    let (source_words, include_source_noun) = match source_words {
        [source @ .., "source"] if !source.is_empty() => (source, true),
        source if !source.is_empty() => (source, false),
        _ => return None,
    };
    let source_filter = parse_object_filter_words(source_words, false).ok()?;
    Some(Value::ManaFromSourceSpentToCastThisSpell {
        source_filter,
        include_source_noun,
        reference,
    })
}

fn parse_mana_from_source_spent_count(words: &[&str], item_start: usize) -> Option<(Value, usize)> {
    if words.get(item_start..item_start + 2) != Some(&["mana", "from"][..]) {
        return None;
    }

    for spent_idx in item_start + 3..words.len() {
        if words[spent_idx] != "spent" {
            continue;
        }
        let (consumed, reference) = if words
            .get(spent_idx..spent_idx + 5)
            .is_some_and(|tail| tail == ["spent", "to", "cast", "this", "spell"])
        {
            (
                spent_idx + 5,
                ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
            )
        } else if words
            .get(spent_idx..spent_idx + 5)
            .is_some_and(|tail| tail == ["spent", "to", "cast", "this", "creature"])
        {
            (
                spent_idx + 5,
                ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature,
            )
        } else if words
            .get(spent_idx..spent_idx + 4)
            .is_some_and(|tail| matches!(tail, ["spent", "to", "cast", "it" | "them"]))
        {
            (
                spent_idx + 4,
                ironsmith_core::ManaSpentCastReferenceSurface::It,
            )
        } else {
            continue;
        };

        let mut source_end = spent_idx;
        if words.get(source_end.saturating_sub(2)..source_end) == Some(&["that", "was"][..]) {
            source_end -= 2;
        }
        let source_words = words.get(item_start + 2..source_end)?;
        let value = mana_from_source_spent_to_cast_value_with_reference(source_words, reference)?;
        return Some((value, consumed));
    }
    None
}

pub fn parse_for_each_count_value_words(words: &[&str]) -> Option<(Value, usize)> {
    let head = parse_for_each_head(words)?;
    let idx = head.item_start;

    if let Some(value) = parse_mana_from_source_spent_count(words, idx) {
        return Some(value);
    }

    // Commander products use this count in several otherwise unrelated
    // consumers (spell copies, token creation, P/T changes, and cost
    // reductions).  Recover the history metric before the permissive object
    // filter fallback can reinterpret "commander" as a current object set.
    // The runtime value deliberately counts casts from the command zone, not
    // commanders that happen to exist in a zone now.
    let commander_count_end = value_boundary(&words[idx..]) + idx;
    if let Some(player) = super::value_helper_shapes::parse_commander_cast_count_player(
        &words[idx..commander_count_end],
    ) {
        let mut value = Value::CommanderCastCount(player);
        if words[idx..commander_count_end]
            .windows(2)
            .any(|window| window == ["a", "commander"])
        {
            value = value
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::IndefiniteCommanderReference);
        }
        return Some((value, commander_count_end));
    }

    if words
        .get(idx..idx + 4)
        .is_some_and(|tail| matches!(tail, ["counter" | "counters", "removed", "this", "way"]))
    {
        return Some((
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed),
            )
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay),
            idx + 4,
        ));
    }
    let mut removed_counter_descriptor_start = idx;
    if words
        .get(removed_counter_descriptor_start)
        .is_some_and(|word| is_article(word))
        || permission_shapes::starts_at_words(words, removed_counter_descriptor_start, &["one"])
    {
        removed_counter_descriptor_start += 1;
    }
    if let Some(counter_idx) = first_counter_word(&words[removed_counter_descriptor_start..])
        .map(|relative_idx| removed_counter_descriptor_start + relative_idx)
        .filter(|counter_idx| counter_idx.saturating_sub(removed_counter_descriptor_start) <= 2)
        && words
            .get(counter_idx + 1..counter_idx + 4)
            .is_some_and(|tail| tail == ["removed", "this", "way"])
    {
        let counter_type = (counter_idx > removed_counter_descriptor_start)
            .then(|| {
                parse_counter_type_words(&words[removed_counter_descriptor_start..=counter_idx])
            })
            .flatten();
        return Some((
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed)
                .with_counter_type(counter_type),
            )
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay),
            counter_idx + 4,
        ));
    }

    if let Some(value) = super::value_expr::colored_mana_symbols_in_costs(words) {
        return Some(value);
    }

    let history_end = value_boundary(&words[idx..]) + idx;
    let history_tokens = synthetic_word_tokens(&words[..history_end]);
    if let Some(value) = super::value_semantics::parse_turn_history_count_value(&history_tokens) {
        return Some((value, history_end));
    }

    let mut counter_descriptor_start = idx;
    if words
        .get(counter_descriptor_start)
        .is_some_and(|word| is_article(word))
        || permission_shapes::starts_at_words(words, counter_descriptor_start, &["one"])
    {
        counter_descriptor_start += 1;
    }
    if let Some(counter_idx) = first_counter_word(&words[counter_descriptor_start..])
        .map(|relative_idx| counter_descriptor_start + relative_idx)
        .filter(|counter_idx| counter_idx.saturating_sub(counter_descriptor_start) <= 2)
    {
        let parsed_counter_type = if counter_idx > counter_descriptor_start {
            parse_counter_type_words(&words[counter_descriptor_start..=counter_idx])
        } else {
            None
        };
        if let Some(counter_type) = parsed_counter_type
            && words
                .get(counter_idx + 1..counter_idx + 3)
                .is_some_and(|tail| exact_one_of(tail, &[&["you", "have"], &["you", "ve"]]))
        {
            return Some((
                Value::PlayerCounters(PlayerFilter::You, counter_type),
                counter_idx + 3,
            ));
        }
        if permission_shapes::starts_at_words(words, counter_idx + 1, &["on"]) {
            let reference_start = counter_idx + 2;
            let reference_end = value_boundary(&words[reference_start..]) + reference_start;
            let reference = &words[reference_start..reference_end];
            if is_source_counter_reference(reference) {
                let value = match parsed_counter_type {
                    Some(counter_type) => match this_source_surface_for_words(reference) {
                        Some(surface) => Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        ),
                        None => Value::CountersOnSource(counter_type),
                    },
                    None => Value::CountersOn(
                        Box::new(
                            this_source_surface_for_words(reference)
                                .map(source_choose_spec_for_surface)
                                .unwrap_or(ChooseSpec::Source),
                        ),
                        None,
                    ),
                };
                return Some((value, reference_end));
            }
            if let Some(surface) = source_reference_surface_for_words(reference) {
                let value = Value::CountersOn(
                    Box::new(source_choose_spec_for_surface(surface)),
                    parsed_counter_type,
                );
                return Some((value, reference_end));
            }
            if is_tagged_counter_reference(reference) {
                let value = Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    parsed_counter_type,
                );
                return Some((value, reference_end));
            }
            if let Ok(filter) = parse_object_filter_words(reference, false) {
                return Some((
                    Value::CountersOn(Box::new(ChooseSpec::All(filter)), parsed_counter_type),
                    reference_end,
                ));
            }
        }
    }

    let filter_end = value_boundary(&words[idx..]) + idx;
    let count_words = &words[idx..filter_end];
    if let Some(exact) = parse_exact_dynamic_count_basis(count_words, filter_end) {
        return Some(exact);
    }
    if let Some(value) =
        value_helper_shapes::parse_aggregate_scope_value_words(&words[idx..filter_end])
    {
        return Some((value, filter_end));
    }

    if let Some(relative_this_way) =
        permission_shapes::find_words(&words[idx..filter_end], &["this", "way"])
    {
        let this_way_start = idx + relative_this_way;
        let this_way_subject = &words[idx..this_way_start];
        let died_filter_words =
            [&["that", "died"][..], &["died"][..]]
                .into_iter()
                .find_map(|suffix| {
                    permission_shapes::suffix_words(this_way_subject, suffix).then(|| {
                        &this_way_subject[..this_way_subject.len().saturating_sub(suffix.len())]
                    })
                });
        if let Some(died_filter_words) = died_filter_words
            && !died_filter_words.is_empty()
            && let Some(filter) = parse_for_each_object_filter_words(died_filter_words, head.other)
        {
            return Some((
                Value::PendingPriorEffectMetric(
                    ironsmith_core::PriorEffectMetricQuery::new(
                        ironsmith_core::EffectMetricSource::AffectedObjects,
                        ironsmith_core::EffectMetric::Count,
                    )
                    .with_filter(filter)
                    .with_action(ironsmith_core::PriorEffectAction::Destroyed),
                )
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::DiedThisWay),
                filter_end,
            ));
        }
        if exact_one_of(
            this_way_subject,
            &[
                &["creature", "card", "put", "into", "your", "graveyard"],
                &["creature", "cards", "put", "into", "your", "graveyard"],
            ],
        ) {
            let mut filter = parse_object_filter_words(&["creature", "card"], false).ok()?;
            filter.set_explicit_card_noun(true);
            return Some((
                Value::PendingPriorEffectMetric(
                    ironsmith_core::PriorEffectMetricQuery::new(
                        ironsmith_core::EffectMetricSource::AffectedObjects,
                        ironsmith_core::EffectMetric::Count,
                    )
                    .with_filter(filter)
                    .with_action(ironsmith_core::PriorEffectAction::Milled),
                )
                .with_surface_hint(
                    ironsmith_core::ValueSurfaceHint::CardsPutIntoYourGraveyardThisWay,
                ),
                filter_end,
            ));
        }
        // A typed `... this way` object count binds to the exact producer's
        // captured result memory.  Keep the authored action on the query so
        // both runtime evaluation and compiled text distinguish, for example,
        // cards exiled by this instruction from every card currently in exile.
        if let Some((action, action_start)) =
            value_helper_shapes::parse_prior_effect_action(this_way_subject)
        {
            let mut filter_words = &this_way_subject[..action_start];
            let player = if permission_shapes::suffix_words(filter_words, &["they"])
                || permission_shapes::suffix_words(filter_words, &["their"])
            {
                filter_words = &filter_words[..filter_words.len() - 1];
                Some(PlayerFilter::IteratedPlayer)
            } else if permission_shapes::suffix_words(filter_words, &["you"])
                || permission_shapes::suffix_words(filter_words, &["your"])
            {
                filter_words = &filter_words[..filter_words.len() - 1];
                Some(PlayerFilter::You)
            } else if permission_shapes::suffix_words(filter_words, &["that", "player"]) {
                filter_words = &filter_words[..filter_words.len() - 2];
                Some(PlayerFilter::IteratedPlayer)
            } else {
                None
            };
            let coordinated_stack_filter = matches!(
                filter_words,
                ["spell" | "spells", "and", "ability" | "abilities"]
            )
            .then(|| {
                let mut filter = ObjectFilter::spell_or_ability();
                filter.set_conjunctive_set_surface(true);
                filter
            });
            if !filter_words.is_empty()
                && let Some(mut filter) = coordinated_stack_filter
                    .or_else(|| parse_for_each_object_filter_words(filter_words, head.other))
            {
                let action_words = &this_way_subject[action_start..];
                if action == ironsmith_core::PriorEffectAction::Returned
                    && permission_shapes::suffix_words(
                        action_words,
                        &["returned", "to", "your", "hand"],
                    )
                {
                    filter.owner = Some(PlayerFilter::You);
                } else if action == ironsmith_core::PriorEffectAction::Returned
                    && permission_shapes::suffix_words(
                        action_words,
                        &["returned", "to", "their", "hand"],
                    )
                {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if filter_words
                    .iter()
                    .any(|word| matches!(*word, "card" | "cards"))
                {
                    filter.set_explicit_card_noun(true);
                }
                let mut query = ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::AffectedObjects,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_filter(filter)
                .with_action(action);
                if let Some(player) = player {
                    query = query.with_player(player);
                }
                let value = Value::PendingPriorEffectMetric(query);
                let value = match action {
                    ironsmith_core::PriorEffectAction::Discarded => value
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsDiscardedThisWay),
                    ironsmith_core::PriorEffectAction::Drawn => {
                        value.with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsDrawnThisWay)
                    }
                    ironsmith_core::PriorEffectAction::Revealed => value
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsRevealedThisWay),
                    _ => value,
                };
                return Some((value, filter_end));
            }
        }
        let has_explicit_card_noun = this_way_subject
            .iter()
            .any(|word| matches!(*word, "card" | "cards"));
        for candidate_end in (idx + 1..this_way_start).rev() {
            if let Some(mut filter) =
                parse_for_each_object_filter_words(&words[idx..candidate_end], head.other)
            {
                if has_explicit_card_noun {
                    filter.set_explicit_card_noun(true);
                }
                return Some((
                    Value::Count(
                        filter.match_tagged(
                            TagKey::from(IT_TAG),
                            TaggedOpbjectRelation::IsTaggedObject,
                        ),
                    ),
                    filter_end,
                ));
            }
        }
    }

    if let Some(player) = value_helper_shapes::parse_party_size_player(count_words) {
        return Some((Value::PartySize(player), filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["time", "it", "regenerated", "this", "turn"],
            &["times", "it", "regenerated", "this", "turn"],
        ],
    ) {
        return Some((Value::SourceRegeneratedThisTurnCount, filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["card", "youve", "drawn", "this", "turn"],
            &["cards", "youve", "drawn", "this", "turn"],
            &["card", "you've", "drawn", "this", "turn"],
            &["cards", "you've", "drawn", "this", "turn"],
            &["card", "you", "have", "drawn", "this", "turn"],
            &["cards", "you", "have", "drawn", "this", "turn"],
        ],
    ) {
        return Some((Value::MaxCardsDrawnThisTurn(PlayerFilter::You), filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["card", "an", "opponent", "has", "drawn", "this", "turn"],
            &["cards", "an", "opponent", "has", "drawn", "this", "turn"],
            &["card", "opponents", "have", "drawn", "this", "turn"],
            &["cards", "opponents", "have", "drawn", "this", "turn"],
        ],
    ) {
        return Some((
            Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
            filter_end,
        ));
    }
    if is_kick_count(count_words) {
        return Some((Value::KickCount, filter_end));
    }
    if let Some(counter_idx) =
        first_counter_word(count_words).filter(|counter_idx| *counter_idx <= 2)
    {
        let counter_tokens = synthetic_word_tokens(count_words);
        if let Some(counter_type) = parse_counter_type_from_tokens(&counter_tokens) {
            if count_words
                .get(counter_idx + 1..counter_idx + 3)
                .is_some_and(|tail| exact_one_of(tail, &[&["you", "have"], &["you", "ve"]]))
            {
                return Some((
                    Value::PlayerCounters(PlayerFilter::You, counter_type),
                    filter_end,
                ));
            }
            if permission_shapes::starts_at_words(count_words, counter_idx + 1, &["on"]) {
                let reference = &count_words[counter_idx + 2..];
                if is_source_counter_reference(reference) {
                    if let Some(surface) = this_source_surface_for_words(reference) {
                        return Some((
                            Value::CountersOn(
                                Box::new(source_choose_spec_for_surface(surface)),
                                Some(counter_type),
                            ),
                            filter_end,
                        ));
                    }
                    return Some((Value::CountersOnSource(counter_type), filter_end));
                }
                if let Some(surface) = source_reference_surface_for_words(reference) {
                    return Some((
                        Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        ),
                        filter_end,
                    ));
                }
                if is_tagged_counter_reference(reference) {
                    return Some((
                        Value::CountersOn(
                            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                            Some(counter_type),
                        ),
                        filter_end,
                    ));
                }
                if let Ok(filter) = parse_object_filter_words(reference, false) {
                    return Some((
                        Value::CountersOn(Box::new(ChooseSpec::All(filter)), Some(counter_type)),
                        filter_end,
                    ));
                }
            }
            if permission_shapes::starts_at_words(count_words, counter_idx + 1, &["among"]) {
                let reference = &count_words[counter_idx + 2..];
                if let Ok(filter) = parse_object_filter_words(reference, false) {
                    return Some((
                        Value::CountersOn(Box::new(ChooseSpec::All(filter)), Some(counter_type))
                            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersAmong),
                        filter_end,
                    ));
                }
            }
        }
    }

    let filter = parse_for_each_object_filter_words(&words[idx..filter_end], head.other)?;
    Some((Value::Count(filter), filter_end))
}

fn parse_exact_dynamic_count_basis(words: &[&str], consumed: usize) -> Option<(Value, usize)> {
    let value = if exact_one_of(
        words,
        &[
            &["card", "that", "player", "has", "drawn", "this", "turn"],
            &["cards", "that", "player", "has", "drawn", "this", "turn"],
        ],
    ) {
        Value::MaxCardsDrawnThisTurn(PlayerFilter::IteratedPlayer)
    } else if exact_one_of(
        words,
        &[&["opponent", "you", "have"], &["opponents", "you", "have"]],
    ) {
        Value::CountPlayers(PlayerFilter::Opponent)
    } else if exact_one_of(
        words,
        &[
            &[
                "modified",
                "creature",
                "you",
                "controlled",
                "as",
                "you",
                "cast",
                "this",
                "spell",
            ],
            &[
                "modified",
                "creatures",
                "you",
                "controlled",
                "as",
                "you",
                "cast",
                "this",
                "spell",
            ],
        ],
    ) {
        Value::Count(crate::target::ObjectFilter::tagged(TagKey::from(
            ironsmith_core::CAST_MODIFIED_CREATURES_TAG,
        )))
    } else if exact_one_of(
        words,
        &[
            &["creature", "chosen", "before", "it"],
            &["creatures", "chosen", "before", "it"],
        ],
    ) {
        Value::Count(crate::target::ObjectFilter::tagged(TagKey::from(
            ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG,
        )))
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CreaturesChosenBeforeIt)
    } else if exact_one_of(
        words,
        &[
            &["creature", "blocking", "it"],
            &["creatures", "blocking", "it"],
        ],
    ) {
        Value::EventValueOffset(
            ironsmith_core::EventValueSpec::BlockersBeyondFirst { multiplier: 1 },
            1,
        )
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CreaturesBlockingIt)
    } else if exact_one_of(
        words,
        &[
            &["creature", "blocking", "it", "beyond", "the", "first"],
            &["creatures", "blocking", "it", "beyond", "the", "first"],
        ],
    ) {
        Value::EventValue(ironsmith_core::EventValueSpec::BlockersBeyondFirst { multiplier: 1 })
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CreaturesBlockingIt)
    } else if exact_one_of(
        words,
        &[
            &["card", "looked", "at", "while", "scrying", "this", "way"],
            &["cards", "looked", "at", "while", "scrying", "this", "way"],
        ],
    ) {
        Value::EventValue(ironsmith_core::EventValueSpec::Amount)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsLookedAtWhileScryingThisWay)
    } else {
        return None;
    };
    Some((value, consumed))
}

fn parse_for_each_head(words: &[&str]) -> Option<ForEachHead> {
    let mut item_start = if permission_shapes::prefix_words(words, &["for", "each"]) {
        2
    } else if permission_shapes::prefix_words(words, &["each"]) {
        1
    } else {
        return None;
    };
    if permission_shapes::starts_at_words(words, item_start, &["of"]) {
        item_start += 1;
    }
    if item_start >= words.len() {
        return None;
    }

    let other = permission_shapes::starts_at_words(words, item_start, &["other"])
        || permission_shapes::starts_at_words(words, item_start, &["another"]);
    if other {
        item_start += 1;
    }
    (item_start < words.len()).then_some(ForEachHead { item_start, other })
}

fn value_boundary(words: &[&str]) -> usize {
    ["plus", "minus"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
        .unwrap_or(words.len())
}

fn first_counter_word(words: &[&str]) -> Option<usize> {
    ["counter", "counters"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

fn is_source_counter_reference(words: &[&str]) -> bool {
    exact_one_of(
        words,
        &[
            &["it"],
            &["this"],
            &["this", "card"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
            &["this", "artifact"],
            &["this", "land"],
            &["this", "enchantment"],
        ],
    )
}

fn is_tagged_counter_reference(words: &[&str]) -> bool {
    exact_one_of(
        words,
        &[
            &["that"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those"],
            &["those", "cards"],
            &["those", "creatures"],
            &["those", "permanents"],
        ],
    )
}

fn is_kick_count(words: &[&str]) -> bool {
    let Some((first, rest)) = words.split_first() else {
        return false;
    };
    if !permission_shapes::exact_words(&[*first], &["time"])
        && !permission_shapes::exact_words(&[*first], &["times"])
    {
        return false;
    }
    if rest.len() < 2 || !permission_shapes::suffix_words(rest, &["was", "kicked"]) {
        return false;
    }
    let source_words = &rest[..rest.len() - 2];
    exact_one_of(
        source_words,
        &[
            &["this"],
            &["this", "spell"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "card"],
            &["it"],
        ],
    ) || source_reference_surface_for_words(source_words).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::ObjectFilter;
    use ironsmith_core::ValueSurfaceHint;

    #[test]
    fn parses_for_each_draw_and_kick_counts() {
        assert_eq!(
            parse_for_each_count_value_words(&[
                "for", "each", "card", "youve", "drawn", "this", "turn"
            ]),
            Some((Value::MaxCardsDrawnThisTurn(PlayerFilter::You), 7))
        );
        assert_eq!(
            parse_for_each_count_value_words(&[
                "for", "each", "time", "this", "spell", "was", "kicked"
            ]),
            Some((Value::KickCount, 7))
        );
        assert_eq!(
            parse_for_each_count_value_words(&[
                "for", "each", "time", "this", "creature", "was", "kicked"
            ]),
            Some((Value::KickCount, 7))
        );

        let (drawn_this_way, used) =
            parse_for_each_count_value_words(&["for", "each", "card", "drawn", "this", "way"])
                .expect("drawn-this-way count");
        assert_eq!(used, 6);
        assert!(drawn_this_way.has_surface_hint(ValueSurfaceHint::CardsDrawnThisWay));
        let Value::PendingPriorEffectMetric(query) = drawn_this_way.unhinted() else {
            panic!("expected an exact drawn-card action count, got {drawn_this_way:#?}");
        };
        assert_eq!(query.action, Some(ironsmith_core::PriorEffectAction::Drawn));
        assert_eq!(
            query.source,
            ironsmith_core::EffectMetricSource::AffectedObjects
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
    }

    #[test]
    fn for_each_shared_terminal_subtype_color_card_keeps_union_semantics() {
        let words = ["for", "each", "forest", "and", "green", "card"];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("shared-terminal union count");
        assert_eq!(used, words.len());
        let Value::Count(filter) = value.unhinted() else {
            panic!("expected a typed object count, got {value:#?}");
        };
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert_eq!(filter.any_of[0].subtypes, [crate::Subtype::Forest]);
        assert_eq!(filter.any_of[1].colors, Some(crate::ColorSet::GREEN));
        assert!(filter.has_conjunctive_set_surface());
    }

    #[test]
    fn typed_counter_removed_count_uses_action_count_and_preserves_counter_kind() {
        let words = ["for", "each", "lore", "counter", "removed", "this", "way"];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("typed removed-counter count");
        assert_eq!(used, words.len());
        assert!(value.has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay));
        let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
            panic!("expected an exact prior-action count, got {value:#?}");
        };
        assert_eq!(query.source, ironsmith_core::EffectMetricSource::Outcome);
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Removed)
        );
        assert_eq!(query.counter_type, Some(crate::object::CounterType::Lore));
        assert!(query.filter.is_none());
    }

    #[test]
    fn parses_generic_mana_source_spent_to_cast_counts() {
        let cave_words = [
            "for", "each", "mana", "from", "a", "cave", "spent", "to", "cast", "it",
        ];
        let (cave_value, used) =
            parse_for_each_count_value_words(&cave_words).expect("Cave-spent count should parse");
        assert_eq!(used, cave_words.len());
        let Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun,
            reference,
        } = cave_value
        else {
            panic!("expected a typed mana-source count");
        };
        assert!(!include_source_noun);
        assert_eq!(reference, ironsmith_core::ManaSpentCastReferenceSurface::It);
        assert_eq!(source_filter.subtypes, [crate::types::Subtype::Cave]);

        let artifact_words = [
            "for", "each", "mana", "from", "an", "artifact", "source", "that", "was", "spent",
            "to", "cast", "this", "spell",
        ];
        let (artifact_value, used) = parse_for_each_count_value_words(&artifact_words)
            .expect("artifact-source-spent count should parse");
        assert_eq!(used, artifact_words.len());
        let Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun,
            reference,
        } = artifact_value
        else {
            panic!("expected a typed mana-source count");
        };
        assert!(include_source_noun);
        assert_eq!(
            reference,
            ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell
        );
        assert_eq!(source_filter.card_types, [crate::types::CardType::Artifact]);
    }

    #[test]
    fn commander_cast_history_precedes_generic_commander_object_counts() {
        let words = [
            "for",
            "each",
            "time",
            "you",
            "ve",
            "cast",
            "your",
            "commander",
            "from",
            "the",
            "command",
            "zone",
            "this",
            "game",
        ];
        let (value, used) = parse_for_each_count_value_words(&words)
            .expect("commander cast history count should parse");
        assert_eq!(used, words.len());
        assert_eq!(value, Value::CommanderCastCount(PlayerFilter::You));

        let current_set = ["for", "each", "commander", "you", "control"];
        let (value, used) = parse_for_each_count_value_words(&current_set)
            .expect("ordinary commander object count should remain supported");
        assert_eq!(used, current_set.len());
        assert!(matches!(value.unhinted(), Value::Count(filter) if filter.is_commander));
    }

    #[test]
    fn parses_for_each_creature_in_your_party_as_party_size() {
        assert_eq!(
            parse_for_each_count_value_words(&["for", "each", "creature", "in", "your", "party"]),
            Some((Value::PartySize(PlayerFilter::You), 6))
        );
    }

    #[test]
    fn leading_other_remains_local_to_the_first_scoped_count_arm() {
        let words = [
            "for",
            "each",
            "other",
            "assassin",
            "you",
            "control",
            "and",
            "each",
            "assassin",
            "card",
            "in",
            "your",
            "graveyard",
        ];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("compound count should parse");
        assert_eq!(used, words.len());
        let Value::Count(filter) = value else {
            panic!("expected object count");
        };
        assert_eq!(filter.any_of.len(), 2);
        assert!(filter.any_of[0].other);
        assert!(!filter.any_of[1].other);
    }

    #[test]
    fn repeated_each_keeps_suspended_cards_and_permanents_as_distinct_count_arms() {
        let words = [
            "for",
            "each",
            "suspended",
            "card",
            "you",
            "own",
            "and",
            "each",
            "other",
            "permanent",
            "you",
            "control",
            "with",
            "a",
            "time",
            "counter",
            "on",
            "it",
        ];
        let (value, used) = parse_for_each_count_value_words(&words)
            .expect("compound suspended count should parse");
        assert_eq!(used, words.len());
        let Value::Count(filter) = value else {
            panic!("expected a typed object count, got {value:#?}");
        };
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(
            filter.any_of.iter().any(|arm| {
                arm.zone == Some(crate::zone::Zone::Exile)
                    && arm.owner == Some(PlayerFilter::You)
                    && arm.alternative_cast == Some(crate::filter::AlternativeCastKind::Suspend)
            }),
            "{filter:#?}"
        );
        assert!(
            filter.any_of.iter().any(|arm| {
                arm.zone == Some(crate::zone::Zone::Battlefield)
                    && arm.controller == Some(PlayerFilter::You)
                    && arm.other
                    && arm.with_counter
                        == Some(crate::filter::CounterConstraint::Typed(
                            crate::object::CounterType::Time,
                        ))
            }),
            "{filter:#?}"
        );
    }

    #[test]
    fn parses_for_each_colored_mana_symbol_across_a_filtered_scope() {
        let words = [
            "for",
            "each",
            "white",
            "mana",
            "symbol",
            "in",
            "the",
            "mana",
            "costs",
            "of",
            "permanents",
            "you",
            "control",
        ];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("mana-symbol token count should parse");
        assert_eq!(used, words.len());
        let Value::ManaSymbolsInManaCostOf { spec, color } = value else {
            panic!("expected structured mana-symbol value");
        };
        assert_eq!(color, crate::color::Color::White);
        let ChooseSpec::All(filter) = spec.unhinted() else {
            panic!("expected aggregate object scope");
        };
        assert_eq!(filter.zone, Some(crate::zone::Zone::Battlefield));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn parses_for_each_opponent_you_have_as_a_player_count() {
        for noun in ["opponent", "opponents"] {
            let words = ["for", "each", noun, "you", "have"];
            let (value, used) =
                parse_for_each_count_value_words(&words).expect("opponent count should parse");
            assert_eq!(used, words.len());
            assert_eq!(value, Value::CountPlayers(PlayerFilter::Opponent));
        }
    }

    #[test]
    fn preserves_explicit_card_noun_in_this_way_counts() {
        let (value, used) = parse_for_each_count_value_words(&[
            "for",
            "each",
            "nonland",
            "card",
            "discarded",
            "this",
            "way",
        ])
        .expect("nonland cards discarded this way count");
        assert_eq!(used, 7);
        assert!(value.has_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay));
        let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
            panic!("expected typed discarded-object count");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Discarded)
        );
        assert!(
            query
                .filter
                .as_ref()
                .is_some_and(ObjectFilter::has_explicit_card_noun)
        );
    }

    #[test]
    fn returned_to_your_hand_count_filters_the_exact_result_by_owner() {
        let words = [
            "for", "each", "card", "returned", "to", "your", "hand", "this", "way",
        ];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("returned-to-your-hand count");
        assert_eq!(used, words.len());
        let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
            panic!("expected a filtered prior-effect metric, got {value:?}");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Returned)
        );
        assert_eq!(
            query
                .filter
                .as_ref()
                .and_then(|filter| filter.owner.clone()),
            Some(PlayerFilter::You)
        );
    }

    #[test]
    fn types_plain_tapped_counts_but_leaves_player_partitioned_counts_unbound() {
        let (value, used) =
            parse_for_each_count_value_words(&["for", "each", "creature", "tapped", "this", "way"])
                .expect("creatures tapped this way count");
        assert_eq!(used, 6);
        let Value::PendingPriorEffectMetric(query) = value else {
            panic!("expected typed prior-effect metric");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Tapped)
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
        assert_eq!(
            query.filter.expect("creature filter").card_types,
            [crate::types::CardType::Creature],
        );

        let (partitioned, used) = parse_for_each_count_value_words(&[
            "for",
            "each",
            "creature",
            "they",
            "controlled",
            "that",
            "was",
            "tapped",
            "this",
            "way",
        ])
        .expect("player-partitioned tapped count retains legacy form");
        assert_eq!(used, 10);
        let Value::PendingPriorEffectMetric(query) = partitioned else {
            panic!("expected typed player-partitioned prior-effect metric");
        };
        assert_eq!(query.player, None);
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Tapped)
        );
    }

    #[test]
    fn counts_typed_counters_among_a_filtered_object_set() {
        let words = [
            "for",
            "each",
            "+1/+1",
            "counter",
            "among",
            "other",
            "creatures",
            "you",
            "control",
        ];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("typed counter aggregate");
        assert_eq!(used, words.len());
        assert!(value.has_surface_hint(ValueSurfaceHint::CountersAmong));
        let Value::CountersOn(spec, Some(crate::object::CounterType::PlusOnePlusOne)) =
            value.unhinted()
        else {
            panic!("expected a typed +1/+1 counter aggregate, got {value:#?}");
        };
        let ChooseSpec::All(filter) = spec.unhinted() else {
            panic!("expected an aggregate object filter, got {spec:#?}");
        };
        assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.other);
    }

    #[test]
    fn coordinated_countered_stack_objects_keep_spells_and_abilities() {
        let words = [
            "for",
            "each",
            "spell",
            "and",
            "ability",
            "countered",
            "this",
            "way",
        ];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("coordinated stack count");
        assert_eq!(used, words.len());
        let Value::PendingPriorEffectMetric(query) = value else {
            panic!("expected typed prior-effect count");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Countered)
        );
        let filter = query.filter.expect("stack filter");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::SpellOrAbility)
        );
        assert!(filter.has_conjunctive_set_surface());
    }
}
