use super::*;

pub(super) fn value_has_surface_hint(value: &Value, hint: ValueSurfaceHint) -> bool {
    value.has_surface_hint(hint)
}

pub(super) fn value_prefers_where_x(value: &Value) -> bool {
    value_has_surface_hint(value, ValueSurfaceHint::WhereXIs)
}

pub(super) fn value_prefers_equal_to(value: &Value) -> bool {
    value_has_surface_hint(value, ValueSurfaceHint::EqualTo)
}

pub(super) fn describe_consult_stop_text(
    selection: &str,
    stop_rule: &crate::effects::ConsultTopOfLibraryStopRule,
    max_exposed: Option<&Value>,
) -> String {
    let match_text = match stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            ensure_indefinite_article(selection)
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count)
            if value_prefers_where_x(count) =>
        {
            if let Some(basis) = describe_where_x_basis(count) {
                format!("X {}, where X is {basis}", pluralize_noun_phrase(selection))
            } else {
                describe_counted_consult_stop(count, selection)
            }
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, selection)
        }
    };
    if let Some(max_exposed) = max_exposed {
        format!(
            "{match_text} or {} cards, whichever comes first",
            describe_value(max_exposed)
        )
    } else {
        match_text
    }
}

pub(super) const STATION_THRESHOLD_RESTRICTION_PREFIX: &str = "__ironsmith_station_threshold:";

pub(super) fn station_threshold_prefix(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    let threshold = activated
        .additional_restrictions
        .iter()
        .find_map(|restriction| {
            restriction
                .strip_prefix(STATION_THRESHOLD_RESTRICTION_PREFIX)
                .and_then(|value| value.parse::<i32>().ok())
        })?;
    let crate::effect::Condition::ValueComparison {
        left,
        operator,
        right,
    } = activated.activation_condition.as_ref()?
    else {
        return None;
    };
    if !matches!(left, Value::CountersOnSource(crate::CounterType::Charge))
        || !matches!(
            operator,
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual
        )
    {
        return None;
    }
    let Value::Fixed(condition_threshold) = right else {
        return None;
    };
    if threshold != *condition_threshold {
        return None;
    }
    Some(format!("{threshold}+"))
}

pub(super) fn prefix_rendered_ability_body(line: String, prefix: &str) -> String {
    if let Some((heading, body)) = line.split_once(": ")
        && is_render_heading_prefix(heading)
    {
        return format!("{heading}: {prefix}{body}");
    }
    format!("{prefix}{line}")
}

pub(super) fn describe_pay_any_energy_amount(
    pay_any_energy: &crate::effects::PayAnyEnergyEffect,
) -> Option<&'static str> {
    match pay_any_energy.min_amount {
        0 => Some("any amount of {E}"),
        1 => Some("one or more {E}"),
        _ => None,
    }
}

pub(super) fn is_target_permanent_or_suspended_card(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Target(inner) = spec else {
        return false;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    let permanent = crate::target::ObjectFilter::permanent();
    let suspended = crate::target::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Exile)
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
        .with_counter_type(crate::object::CounterType::Time);
    filter.any_of.len() == 2
        && filter.zone.is_none()
        && filter.any_of.iter().any(|arm| arm == &permanent)
        && filter.any_of.iter().any(|arm| arm == &suspended)
}

pub(super) fn is_target_nonland_permanent_or_suspended_card(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Target(inner) = spec else {
        return false;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    let mut permanent = crate::target::ObjectFilter::permanent();
    permanent
        .excluded_card_types
        .push(crate::types::CardType::Land);
    let suspended = crate::target::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Exile)
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
        .with_counter_type(crate::object::CounterType::Time);
    let mut outer = filter.clone();
    outer.any_of.clear();
    filter.any_of.len() == 2
        && outer == crate::target::ObjectFilter::default()
        && filter.any_of.iter().any(|arm| arm == &permanent)
        && filter.any_of.iter().any(|arm| arm == &suspended)
}

pub(super) fn is_source_damaged_death_graveyard_card_spec(spec: &ChooseSpec) -> bool {
    if !spec.count().is_single() {
        return false;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return false;
    };
    filter.zone == Some(Zone::Graveyard)
        && filter.card_types.as_slice() == [CardType::Creature]
        && filter.entered_graveyard_from_battlefield_this_turn
        && filter.dealt_damage_by_source_this_turn.is_some()
}

pub(super) fn is_time_travel_object_set(spec: &ChooseSpec) -> bool {
    let ChooseSpec::All(filter) = spec else {
        return false;
    };
    let permanent = crate::target::ObjectFilter::permanent()
        .you_control()
        .with_counter_type(crate::object::CounterType::Time);
    let suspended = crate::target::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Exile)
        .owned_by(crate::target::PlayerFilter::You)
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
        .with_counter_type(crate::object::CounterType::Time);
    filter.any_of.len() == 2
        && filter.zone.is_none()
        && filter.any_of.iter().any(|arm| arm == &permanent)
        && filter.any_of.iter().any(|arm| arm == &suspended)
}

/// Rejoin the implicit object-choice scaffold used by “Sacrifice any number
/// of ..., then add that much ...”. The sacrifice result ID is the executable
/// proof that the mana amount counts the chosen set actually sacrificed.
pub(super) fn describe_sacrifice_any_number_then_add_that_much_mana(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, mana_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice_with_id = sacrifice_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = sacrifice_view(&sacrifice_with_id.effect)?;
    let mana = structural_unwrap_render_wrappers(mana_effect)
        .downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    let [symbol] = mana.mana.as_slice() else {
        return None;
    };
    if choose.chooser != PlayerFilter::You
        || sacrifice.player != &PlayerFilter::You
        || mana.player != PlayerFilter::You
        || !matches!(
            mana.amount.unhinted(),
            Value::EffectValue(id) if *id == sacrifice_with_id.id
        )
    {
        return None;
    }
    let sacrifice_text = describe_choose_then_sacrifice(choose, sacrifice)?;
    if !sacrifice_text
        .to_ascii_lowercase()
        .contains("sacrifice any number of")
    {
        return None;
    }
    Some(format!(
        "{}, then add that much {}",
        capitalize_first(&sacrifice_text),
        describe_mana_symbol(*symbol)
    ))
}

#[cfg(test)]
mod sacrifice_any_number_then_add_mana_tests {
    use super::*;

    fn effects(result_id: crate::effect::EffectId) -> Vec<Effect> {
        let tag = TagKey::from("chosen_lands");
        let choose = Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::land()
                    .you_control()
                    .in_zone(Zone::Battlefield),
                ChoiceCount::any_number(),
                PlayerFilter::You,
                tag.clone(),
            )
            .in_zone(Zone::Battlefield),
        );
        let chosen = ObjectFilter::tagged(tag);
        let sacrifice = Effect::with_id(
            7,
            Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
                chosen.clone(),
                Value::Count(chosen),
                PlayerFilter::You,
            )),
        );
        let mana = Effect::new(crate::effects::AddScaledManaEffect::new(
            vec![ManaSymbol::Colorless],
            Value::EffectValue(result_id),
            PlayerFilter::You,
        ));
        vec![choose, sacrifice, mana]
    }

    #[test]
    fn sacrifice_result_identity_controls_that_much_mana_surface() {
        assert_eq!(
            describe_sacrifice_any_number_then_add_that_much_mana(&effects(
                crate::effect::EffectId(7)
            )),
            Some("You sacrifice any number of lands, then add that much {C}".to_string())
        );
        assert!(
            describe_sacrifice_any_number_then_add_that_much_mana(&effects(
                crate::effect::EffectId(8)
            ))
            .is_none(),
            "an unrelated result ID must not acquire the 'that much' surface"
        );
    }
}

pub(in crate::compiled_text) fn describe_discard_hand_add_mana_draw_sequence(
    effects: &[&Effect],
) -> Option<String> {
    let [discard_effect, mana_effect, draw_effect] = effects else {
        return None;
    };
    let discard_with_id = discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let discard = discard_with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let mana_with_id = mana_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let mana = mana_with_id
        .effect
        .downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;

    let hand_filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    };
    let discards_entire_hand = match discard.count.unhinted() {
        Value::CardsInHand(player) => player == &PlayerFilter::You && discard.card_filter.is_none(),
        Value::Count(count_filter) => {
            count_filter == &hand_filter && discard.card_filter.as_ref() == Some(&hand_filter)
        }
        _ => false,
    };
    if discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
        || !discards_entire_hand
        || mana.player != PlayerFilter::You
        || draw.player != PlayerFilter::You
    {
        return None;
    }

    let mana_counts_discarded_cards = match mana.amount.unhinted() {
        Value::EffectMetric {
            effect_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        } => *effect_id == discard_with_id.id,
        Value::PriorEffectMetric { effect_id, query } => {
            *effect_id == discard_with_id.id
                && query.source == crate::effect::EffectMetricSource::AffectedObjects
                && query.metric == crate::effect::EffectMetric::Count
                && query.action == Some(crate::effect::PriorEffectAction::Discarded)
        }
        _ => false,
    };
    if !mana_counts_discarded_cards {
        return None;
    }
    let Value::EffectValueOffset(draw_effect_id, draw_offset) = &draw.count else {
        return None;
    };
    if *draw_effect_id != mana_with_id.id || *draw_offset < 0 {
        return None;
    }

    let mana_text = mana
        .mana
        .iter()
        .copied()
        .map(describe_mana_symbol)
        .collect::<Vec<_>>()
        .join("");
    if mana_text.is_empty() {
        return None;
    }

    let draw_text = match *draw_offset {
        0 => "that many cards".to_string(),
        1 => "that many cards plus one".to_string(),
        offset => format!("that many cards plus {offset}"),
    };

    Some(format!(
        "Discard all the cards in your hand. Add {mana_text} for each card discarded this way, then draw {draw_text}"
    ))
}

/// Keep a hand-to-library move and a draw linked to that move's outcome in
/// the authored ordered clause. The effect ID is the semantic link, so this
/// covers both "the cards in your hand" and "any number of cards" variants
/// without relying on a card name or retained source text.
pub(in crate::compiled_text) fn describe_move_hand_to_library_then_draw(
    effects: &[&Effect],
) -> Option<String> {
    let [move_effect, draw_effect] = effects else {
        return None;
    };
    let with_id = move_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let move_to_zone = unwrap_basic_tag_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;

    if move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
        || move_to_zone.destination_player_surface.as_ref() != Some(&PlayerFilter::You)
        || draw.player != PlayerFilter::You
    {
        return None;
    }

    enum HandMove {
        All,
        AnyNumber,
    }

    let (filter, hand_move) = match move_to_zone.target.unhinted() {
        ChooseSpec::All(filter) => (filter, HandMove::All),
        ChooseSpec::WithCount(spec, count)
            if *count == crate::effect::ChoiceCount::any_number() =>
        {
            let ChooseSpec::Object(filter) = spec.unhinted() else {
                return None;
            };
            (filter, HandMove::AnyNumber)
        }
        _ => return None,
    };
    let expected_filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    };
    if filter != &expected_filter {
        return None;
    }

    let draw_offset = if is_effect_count_reference(&draw.count, Some(with_id.id)) {
        0
    } else {
        effect_count_reference_offset(&draw.count, Some(with_id.id))?
    };
    if draw_offset < 0 {
        return None;
    }
    let draw_count = match draw_offset {
        0 => "that many cards".to_string(),
        1 => "that many cards plus one".to_string(),
        offset => format!("that many cards plus {offset}"),
    };

    let move_clause = match hand_move {
        HandMove::All => {
            if move_to_zone.library_order
                != Some(crate::effects::LibraryPlacementOrder::ChosenBy(
                    PlayerFilter::You,
                ))
            {
                return None;
            }
            "Put the cards in your hand on the bottom of your library in any order".to_string()
        }
        HandMove::AnyNumber => {
            if move_to_zone.library_order.is_some() {
                return None;
            }
            "Put any number of cards from your hand on the bottom of your library".to_string()
        }
    };

    Some(format!("{move_clause}, then draw {draw_count}"))
}

pub(super) fn value_is_discarded_count_for_effect(
    value: &Value,
    id: crate::effect::EffectId,
) -> bool {
    let value = value.unhinted();
    matches!(
        value,
        Value::EffectMetric {
            effect_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        } if *effect_id == id
    ) || value_is_affected_count_for_effect(value, id)
}

fn discard_sequence_subject(player: &PlayerFilter) -> String {
    match player {
        PlayerFilter::DamagedPlayer => "that player".to_string(),
        other => describe_player_filter(other),
    }
}

pub(super) fn discard_sequence_count(discard: &crate::effects::DiscardEffect) -> String {
    if discard
        .count
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand)
        || matches!(
            discard.count.unhinted(),
            Value::CardsInHand(owner)
                if player_filters_refer_to_same_player(owner, &discard.player)
        )
    {
        let possessive = if discard.player == PlayerFilter::You {
            "your"
        } else {
            "their"
        };
        return format!("all the cards in {possessive} hand");
    }
    if discard
        .count
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::OneOrMoreChoice)
    {
        return discard.card_filter.as_ref().map_or_else(
            || "one or more cards".to_string(),
            |filter| {
                format!(
                    "one or more {}",
                    pluralize_discard_card_phrase(&describe_discard_card_phrase(filter))
                )
            },
        );
    }
    let count = describe_discard_count(&discard.count, discard.card_filter.as_ref());
    if !discard.any_number {
        return count;
    }
    match discard.count.unhinted() {
        Value::Fixed(0) if discard.card_filter.is_none() => "any number of cards".to_string(),
        Value::Fixed(0) => format!(
            "any number of {}",
            pluralize_discard_card_phrase(&describe_discard_card_phrase(
                discard.card_filter.as_ref().expect("filter checked")
            ))
        ),
        _ => format!("up to {count}"),
    }
}

fn prior_count_draw_phrase(value: &Value, id: crate::effect::EffectId) -> Option<String> {
    let offset = effect_count_reference_offset(value, Some(id))
        .or_else(|| is_effect_count_reference(value, Some(id)).then_some(0))?;
    if offset == 0 && value.has_surface_hint(ValueSurfaceHint::AsManyCardsThisWay) {
        return Some("as many cards as they discarded this way".to_string());
    }
    Some(match offset {
        0 => "that many cards".to_string(),
        1 => "that many cards plus one".to_string(),
        -1 => "that many cards minus one".to_string(),
        amount if amount > 1 => format!(
            "that many cards plus {}",
            small_number_word(amount as u32).unwrap_or_else(|| amount.to_string())
        ),
        amount => format!(
            "that many cards minus {}",
            small_number_word((-amount) as u32).unwrap_or_else(|| (-amount).to_string())
        ),
    })
}

fn describe_discard_draw_pair(first: &Effect, second: &Effect) -> Option<String> {
    let draw =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::DrawCardsEffect>()?;

    if let Some(discard_hand) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::DiscardHandEffect>()
    {
        if discard_hand.player != draw.player {
            return None;
        }
        let subject = discard_sequence_subject(&discard_hand.player);
        let draw_count = describe_card_count(&draw.count);
        return Some(if discard_hand.player == PlayerFilter::You {
            format!("Discard your hand, then draw {draw_count}")
        } else {
            format!(
                "{subject} {} their hand, then {} {draw_count}",
                player_verb(&subject, "discard", "discards"),
                player_verb(&subject, "draw", "draws")
            )
        });
    }

    if first
        .downcast_ref::<crate::effects::WithIdEffect>()
        .is_none()
        && let Some(discard) =
            unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::DiscardEffect>()
        && discard.player == draw.player
        && matches!(discard.count.unhinted(), Value::Fixed(_))
        && matches!(draw.count.unhinted(), Value::Fixed(_))
    {
        let discard_count = discard_sequence_count(discard);
        let draw_count = describe_card_count(&draw.count);
        let random = if discard.random { " at random" } else { "" };
        let subject = discard_sequence_subject(&discard.player);
        return Some(if discard.player == PlayerFilter::You {
            format!("Discard {discard_count}{random}, then draw {draw_count}")
        } else {
            format!(
                "{subject} {} {discard_count}{random}, then {} {draw_count}",
                player_verb(&subject, "discard", "discards"),
                player_verb(&subject, "draw", "draws")
            )
        });
    }

    let with_id = first.downcast_ref::<crate::effects::WithIdEffect>()?;
    let discard = unwrap_basic_tag_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.player != draw.player {
        return None;
    }
    let draw_count = prior_count_draw_phrase(&draw.count, with_id.id).or_else(|| {
        matches!(discard.count.unhinted(), Value::Fixed(_))
            .then(|| describe_card_count(&draw.count))
    })?;
    let discard_count = discard_sequence_count(discard);
    let random = if discard.random { " at random" } else { "" };
    let subject = discard_sequence_subject(&discard.player);
    Some(if discard.player == PlayerFilter::You {
        format!("Discard {discard_count}{random}, then draw {draw_count}")
    } else {
        format!(
            "{subject} {} {discard_count}{random}, then {} {draw_count}",
            player_verb(&subject, "discard", "discards"),
            player_verb(&subject, "draw", "draws")
        )
    })
}

/// Preserve a discard/draw amount dependency as a single oracle-style
/// sequence, including when the pair is embedded in a longer effect list.
pub(super) fn describe_discard_then_draw_amount_sequence(effects: &[Effect]) -> Option<String> {
    for index in 0..effects.len().saturating_sub(1) {
        let Some(pair) = describe_discard_draw_pair(&effects[index], &effects[index + 1]) else {
            continue;
        };

        let mut sentences = Vec::new();
        if index > 0 {
            let prefix = describe_effect_list(&effects[..index]);
            if !prefix.trim().is_empty() {
                sentences.push(prefix.trim().trim_end_matches('.').to_string());
            }
        }
        sentences.push(if sentences.is_empty() {
            pair
        } else {
            capitalize_first(&pair)
        });
        if index + 2 < effects.len() {
            let suffix = describe_effect_list(&effects[index + 2..]);
            if !suffix.trim().is_empty() {
                sentences.push(capitalize_first(suffix.trim().trim_end_matches('.')));
            }
        }
        return Some(sentences.join(". "));
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PriorCountAction {
    Destroyed,
    Exiled,
    Sacrificed,
    Discarded,
    Removed,
    ReturnedToHand,
    PutIntoGraveyard,
}

impl PriorCountAction {
    fn past_tense(self) -> &'static str {
        match self {
            Self::Destroyed => "destroyed",
            Self::Exiled => "exiled",
            Self::Sacrificed => "sacrificed",
            Self::Discarded => "discarded",
            Self::Removed => "removed",
            Self::ReturnedToHand => "returned to your hand",
            Self::PutIntoGraveyard => "put into a graveyard",
        }
    }
}

fn prior_count_subject_from_filter(
    filter: &ObjectFilter,
    cards_outside_battlefield: bool,
    fallback: &'static str,
) -> String {
    let Some(card_type) = filter.card_types.as_slice().first().copied() else {
        return fallback.to_string();
    };
    if filter.card_types.len() != 1 {
        return fallback.to_string();
    }

    let subject = card_type.name().to_ascii_lowercase();
    if cards_outside_battlefield && filter.zone != Some(Zone::Battlefield) {
        format!("{subject} card")
    } else {
        subject
    }
}

fn prior_count_action_surface(effect: &Effect) -> Option<(PriorCountAction, String)> {
    let effect = unwrap_basic_tag_wrappers(effect);
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut actions = sequence
            .effects
            .iter()
            .filter_map(prior_count_action_surface);
        let action = actions.next()?;
        return actions.next().is_none().then_some(action);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        let mut actions = may.effects.iter().filter_map(prior_count_action_surface);
        let action = actions.next()?;
        return actions.next().is_none().then_some(action);
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        let (ChooseSpec::All(filter) | ChooseSpec::Object(filter)) = destroy.spec.base() else {
            return None;
        };
        return Some((
            PriorCountAction::Destroyed,
            prior_count_subject_from_filter(filter, false, "permanent"),
        ));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        let (ChooseSpec::All(filter) | ChooseSpec::Object(filter)) = destroy.spec.base() else {
            return None;
        };
        return Some((
            PriorCountAction::Destroyed,
            prior_count_subject_from_filter(filter, false, "permanent"),
        ));
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        let (ChooseSpec::All(filter) | ChooseSpec::Object(filter)) = exile.spec.base() else {
            return None;
        };
        return Some((
            PriorCountAction::Exiled,
            prior_count_subject_from_filter(filter, true, "card"),
        ));
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>() {
        return Some((
            PriorCountAction::Sacrificed,
            prior_count_subject_from_filter(&sacrifice.filter, false, "permanent"),
        ));
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        return Some((
            PriorCountAction::Sacrificed,
            prior_count_subject_from_filter(&sacrifice.filter, false, "permanent"),
        ));
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        return Some((
            PriorCountAction::Discarded,
            discard
                .card_filter
                .as_ref()
                .map(|filter| prior_count_subject_from_filter(filter, true, "card"))
                .unwrap_or_else(|| "card".to_string()),
        ));
    }
    if effect
        .downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
        .is_some()
    {
        return Some((PriorCountAction::Removed, "counter".to_string()));
    }
    if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveUpToCountersEffect>() {
        return Some((
            PriorCountAction::Removed,
            format!("{} counter", describe_counter_type(remove.counter_type)),
        ));
    }
    if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() {
        return Some((
            PriorCountAction::Removed,
            format!("{} counter", describe_counter_type(remove.counter_type)),
        ));
    }
    if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>() {
        let subject = remove
            .counter_type
            .map(describe_counter_type)
            .map(|counter_type| format!("{counter_type} counter"))
            .unwrap_or_else(|| "counter".to_string());
        return Some((PriorCountAction::Removed, subject));
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        let (ChooseSpec::All(filter) | ChooseSpec::Object(filter)) = return_to_hand.spec.base()
        else {
            return None;
        };
        return Some((
            PriorCountAction::ReturnedToHand,
            prior_count_subject_from_filter(filter, true, "card"),
        ));
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Graveyard
    {
        let (ChooseSpec::All(filter) | ChooseSpec::Object(filter)) = move_to_zone.target.base()
        else {
            return None;
        };
        return Some((
            PriorCountAction::PutIntoGraveyard,
            prior_count_subject_from_filter(filter, true, "card"),
        ));
    }
    None
}

fn exact_prior_effect_count_multiplier(value: &Value, id: crate::effect::EffectId) -> Option<i32> {
    match value {
        // `ForEach` confirms the same one-producer count surface this helper
        // reconstructs. Other authored surfaces remain with their dedicated
        // renderers.
        Value::SurfaceHinted { value, hints }
            if hints.iter().all(|hint| *hint == ValueSurfaceHint::ForEach) =>
        {
            exact_prior_effect_count_multiplier(value, id)
        }
        Value::SurfaceHinted { .. } => None,
        Value::Add(left, right) => Some(
            exact_prior_effect_count_multiplier(left, id)?
                + exact_prior_effect_count_multiplier(right, id)?,
        ),
        Value::Scaled(inner, multiplier) if *multiplier > 0 => {
            Some(exact_prior_effect_count_multiplier(inner, id)? * multiplier)
        }
        value if is_effect_count_reference(value, Some(id)) => Some(1),
        _ => None,
    }
}

fn prior_effect_count_restricts_owner(
    value: &Value,
    id: crate::effect::EffectId,
    owner: &PlayerFilter,
) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } | Value::Scaled(value, _) => {
            prior_effect_count_restricts_owner(value, id, owner)
        }
        Value::Add(left, right) => {
            prior_effect_count_restricts_owner(left, id, owner)
                && prior_effect_count_restricts_owner(right, id, owner)
        }
        Value::PriorEffectMetric { effect_id, query } if *effect_id == id => {
            query
                .filter
                .as_ref()
                .and_then(|filter| filter.owner.as_ref())
                == Some(owner)
        }
        _ => false,
    }
}

/// Render an exact prior-action result link from the structured producer ID.
///
/// This intentionally accepts only one producer and one consumer (plus optional
/// target-selection scaffolding). Multi-producer and filtered-subset clauses do
/// not have enough structural provenance to choose a safe "this way" surface.
pub(super) fn describe_id_backed_prior_action_count_consumer(effects: &[Effect]) -> Option<String> {
    let (producer_effect, consumer_effect) = match effects {
        [producer, consumer] => (producer, consumer),
        [target_only, producer, consumer]
            if target_only
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            (producer, consumer)
        }
        _ => return None,
    };
    let with_id = producer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let (action, subject) = prior_count_action_surface(&with_id.effect)?;
    let counted = format!("{subject} {} this way", action.past_tense());

    let consumer = if let Some(draw) =
        unwrap_basic_tag_wrappers(consumer_effect).downcast_ref::<crate::effects::DrawCardsEffect>()
    {
        if action == PriorCountAction::ReturnedToHand
            && !prior_effect_count_restricts_owner(&draw.count, with_id.id, &PlayerFilter::You)
        {
            return None;
        }
        let multiplier = exact_prior_effect_count_multiplier(&draw.count, with_id.id)?;
        let player = describe_player_filter(&draw.player);
        if draw
            .count
            .has_surface_hint(ValueSurfaceHint::AsManyCardsThisWay)
            && multiplier == 1
            && with_id
                .effect
                .downcast_ref::<crate::effects::DiscardEffect>()
                .is_some_and(|discard| discard.player == draw.player)
        {
            let producer = describe_effect(producer_effect);
            let producer = producer.trim().trim_end_matches('.');
            let verb = player_verb(&player, "draw", "draws");
            return (!producer.is_empty()).then(|| {
                format!("{producer}, then {verb} as many cards as they discarded this way")
            });
        }
        let cards = if multiplier == 1 {
            "a card".to_string()
        } else {
            format!("{multiplier} cards")
        };
        format!(
            "{} {} {cards} for each {counted}",
            capitalize_first(&player),
            player_verb(&player, "draw", "draws")
        )
    } else if let Some(gain) =
        unwrap_basic_tag_wrappers(consumer_effect).downcast_ref::<crate::effects::GainLifeEffect>()
    {
        // A filtered discarded-card subset is not represented by the discard
        // outcome ID alone (for example, land cards discarded this way).
        if action == PriorCountAction::Discarded {
            return None;
        }
        let multiplier = exact_prior_effect_count_multiplier(&gain.amount, with_id.id)?;
        let player = describe_choose_spec(&gain.player);
        format!(
            "{} {} {multiplier} life for each {counted}",
            capitalize_first(&player),
            player_verb(&player, "gain", "gains")
        )
    } else if let Some(create) = unwrap_basic_tag_wrappers(consumer_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()
    {
        let multiplier = exact_prior_effect_count_multiplier(&create.count, with_id.id)?;
        let mut per_result = create.clone();
        per_result.count = Value::Fixed(multiplier);
        let per_result = describe_effect(&Effect::new(per_result));
        format!(
            "{} for each {counted}",
            capitalize_first(per_result.trim().trim_end_matches('.'))
        )
    } else if let Some(add_mana) = unwrap_basic_tag_wrappers(consumer_effect)
        .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
    {
        if add_mana.distinct_colors {
            return None;
        }
        let multiplier = exact_prior_effect_count_multiplier(&add_mana.amount, with_id.id)?;
        let colors = add_mana.available_colors.as_ref()?;
        if colors.is_empty() {
            return None;
        }
        let options = colors
            .iter()
            .copied()
            .map(crate::mana::ManaSymbol::from_color)
            .collect::<Vec<_>>();
        let destination = describe_add_mana_destination_suffix(&add_mana.player);
        if multiplier == 1 {
            format!(
                "Add {} for each {counted}{destination}",
                describe_mana_alternatives(&options)
            )
        } else {
            let amount =
                small_number_word(multiplier as u32).unwrap_or_else(|| multiplier.to_string());
            format!(
                "Add {amount} mana in any combination of {} for each {counted}{destination}",
                options
                    .iter()
                    .copied()
                    .map(describe_mana_symbol)
                    .collect::<Vec<_>>()
                    .join(" and/or ")
            )
        }
    } else if let Some(add_mana) = unwrap_basic_tag_wrappers(consumer_effect)
        .downcast_ref::<crate::effects::AddScaledManaEffect>()
    {
        if action != PriorCountAction::Removed
            || !add_mana
                .amount
                .has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay)
            || exact_prior_effect_count_multiplier(add_mana.amount.unhinted(), with_id.id)? != 1
            || add_mana.mana.is_empty()
        {
            return None;
        }
        let mana = add_mana
            .mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        format!(
            "Add {mana} for each {counted}{}",
            describe_add_mana_destination_suffix(&add_mana.player)
        )
    } else if let Some(repeat) = unwrap_basic_tag_wrappers(consumer_effect)
        .downcast_ref::<crate::effects::RepeatEffectsEffect>()
    {
        let multiplier = exact_prior_effect_count_multiplier(&repeat.count, with_id.id)?;
        let repeated = describe_effect_list(&repeat.effects);
        let repeated = capitalize_first(repeated.trim().trim_end_matches('.'));
        if repeated.is_empty() {
            return None;
        }
        if multiplier == 1 {
            format!("{repeated} for each {counted}")
        } else {
            format!("{repeated} {multiplier} times for each {counted}")
        }
    } else {
        return None;
    };

    let producer = describe_effect(producer_effect);
    let producer = capitalize_first(producer.trim().trim_end_matches('.'));
    (!producer.is_empty()).then(|| format!("{producer}. {consumer}"))
}

pub(super) fn plural_discard_subject(filter: &PlayerFilter) -> Option<&'static str> {
    match filter {
        PlayerFilter::Any => Some("Each player"),
        PlayerFilter::Opponent => Some("Each opponent"),
        PlayerFilter::NotYou => Some("Each other player"),
        PlayerFilter::You => Some("You"),
        _ => None,
    }
}

pub(super) fn describe_discard_then_draw_for_discarded(effects: &[Effect]) -> Option<String> {
    let [discard_effect, draw_effect] = effects else {
        return None;
    };
    let with_id = discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !value_is_discarded_count_for_effect(&draw.count, with_id.id)
    {
        return None;
    }

    let (subject, discard) = if let Some(discard) = with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()
    {
        (plural_discard_subject(&discard.player)?, discard)
    } else {
        let for_players = with_id
            .effect
            .downcast_ref::<crate::effects::ForPlayersEffect>()?;
        if for_players.starting_with_controller {
            return None;
        }
        let [iterated_effect] = for_players.effects.as_slice() else {
            return None;
        };
        let discard = iterated_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
        if discard.player != PlayerFilter::IteratedPlayer {
            return None;
        }
        (describe_for_players_subject(&for_players.filter)?, discard)
    };

    if discard.any_number {
        return None;
    }

    let random_suffix = if discard.random { " at random" } else { "" };
    Some(format!(
        "{subject} {} {}{random_suffix}. You draw a card for each card discarded this way",
        if subject == "You" {
            "discard"
        } else {
            "discards"
        },
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    ))
}

pub(super) fn describe_for_players_may_discard_then_draw_if_discarded(
    effects: &[Effect],
) -> Option<String> {
    let [may_discard_effect, draw_if_discarded_effect] = effects else {
        return None;
    };
    let may_discard_with_id = may_discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let for_players = may_discard_with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.starting_with_controller {
        return None;
    }
    let subject = describe_for_players_subject(&for_players.filter)?;
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let [discard_effect] = may.effects.as_slice() else {
        return None;
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.player != PlayerFilter::IteratedPlayer
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }
    let draw_if_discarded = draw_if_discarded_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if draw_if_discarded.condition != may_discard_with_id.id
        || draw_if_discarded.predicate != EffectPredicate::Happened
        || !draw_if_discarded.else_.is_empty()
    {
        return None;
    }
    let [draw_effect] = draw_if_discarded.then.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.count != Value::Fixed(1) || draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    Some(format!(
        "{subject} may discard a card, then each player who discarded a card this way draws a card"
    ))
}

/// "Each player may draw a card, then each player who drew a card this way
/// gains N life." — the draw/gain-life sibling of the discard/draw shape.
/// The lowering nests the whole program inside the per-player iteration:
/// ForPlayers[May[Sequence[WithId(Draw), If(Happened)[GainLife]]]].
pub(super) fn describe_for_players_may_draw_then_gain_if_drew(
    effects: &[Effect],
) -> Option<String> {
    let [for_players_effect] = effects else {
        return None;
    };
    let for_players = structural_unwrap_render_wrappers(for_players_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.starting_with_controller {
        return None;
    }
    let subject = describe_for_players_subject(&for_players.filter)?;
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let [sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [draw_effect, gain_if_drew_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let draw_with_id = draw_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let draw = draw_with_id
        .effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.count != Value::Fixed(1) || draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let gain_if_drew = gain_if_drew_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if gain_if_drew.condition != draw_with_id.id
        || gain_if_drew.predicate != EffectPredicate::Happened
        || !gain_if_drew.else_.is_empty()
    {
        return None;
    }
    let [gain_effect] = gain_if_drew.then.as_slice() else {
        return None;
    };
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let Value::Fixed(amount) = gain.amount else {
        return None;
    };
    Some(format!(
        "{subject} may draw a card, then each player who drew a card this way gains {amount} life"
    ))
}

pub(super) fn describe_look_reorder_then_may_shuffle(effects: &[Effect]) -> Option<String> {
    let [look_effect, reorder_effect, may_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reorder = reorder_effect.downcast_ref::<crate::effects::ReorderLibraryTopEffect>()?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let (target_only, shuffle_effect) = match may.effects.as_slice() {
        [shuffle_effect] => (None, shuffle_effect),
        [target_effect, shuffle_effect] => {
            let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            (Some(target_only), shuffle_effect)
        }
        _ => return None,
    };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if look.reveal
        || reorder.tag != look.tag
        || !matches!(
            &look.player,
            PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any)
        )
        || shuffle.player != look.player
        || !matches!(may.decider, None | Some(PlayerFilter::You))
    {
        return None;
    }
    if let Some(target_only) = target_only
        && target_only.target != ChooseSpec::target_player()
    {
        return None;
    }

    let look_text = describe_effect(look_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{look_text}, then put them back in any order. You may have that player shuffle"
    ))
}

pub(super) fn may_action_this_way_phrase(action: &str) -> Option<String> {
    let action = action.trim().trim_end_matches('.');
    let lower = action.to_ascii_lowercase();
    let phrase = if lower == "draw a card" {
        "drew a card this way".to_string()
    } else if let Some(rest) = lower.strip_prefix("draw ") {
        format!("drew {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("discard ") {
        format!("discarded {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("gain ") {
        format!("gained {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("lose ") {
        format!("lost {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("sacrifice ") {
        format!("sacrificed {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("pay ") {
        format!("paid {rest} this way")
    } else if lower.starts_with("search their library")
        || lower.starts_with("search that player's library")
        || lower.starts_with("search your library")
    {
        "searched their library this way".to_string()
    } else {
        return None;
    };
    Some(phrase)
}

pub(super) fn describe_for_players_happened_followup(effects: &[Effect]) -> Option<String> {
    let mut followup = describe_effect_list(effects)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if followup.is_empty() {
        return None;
    }
    if let Some(rest) = followup.strip_prefix("that player ") {
        followup = rest.to_string();
    } else if let Some(rest) = followup.strip_prefix("you ") {
        followup = rest.to_string();
    }
    followup = normalize_third_person_verb_phrase(&followup);
    Some(lowercase_first(&followup))
}

pub(super) fn describe_for_players_may_action(
    _filter: &PlayerFilter,
    effects: &[Effect],
) -> Option<String> {
    let mut action = describe_effect_list(effects);
    if let Some(rest) = action.strip_prefix("that player ") {
        action = rest.to_string();
    }
    if let Some(rest) = action.strip_prefix("you ") {
        action = rest.to_string();
    }
    if let Some(rest) = action.strip_prefix("they ") {
        action = rest.to_string();
    }
    let action = normalize_you_verb_phrase(&action);
    Some(lowercase_may_clause(&action))
}

fn describe_contextualized_iterated_player_may_action(
    filter: &PlayerFilter,
    effects: &[Effect],
) -> Option<String> {
    fn join_dynamic_search_move_before_where_x(search: String) -> String {
        let Some((search_head, where_and_move)) = search.split_once(", where X is ") else {
            return search;
        };
        let Some((where_basis, move_clause)) = where_and_move.split_once(". Put ") else {
            return search;
        };
        let move_clause = move_clause
            .strip_prefix("those cards ")
            .map(|rest| format!("them {rest}"))
            .or_else(|| {
                move_clause
                    .strip_prefix("that card ")
                    .map(|rest| format!("it {rest}"))
            })
            .unwrap_or_else(|| move_clause.to_string());
        let where_basis = where_basis
            .split_once(" minus ")
            .and_then(|(left, right)| {
                left.parse::<i32>()
                    .ok()
                    .and_then(number_word)
                    .map(|left| format!("{left} minus {right}"))
            })
            .unwrap_or_else(|| where_basis.to_string());
        format!("{search_head} and put {move_clause}, where X is {where_basis}")
    }

    let iterated_search = match effects {
        [sequence_effect] => sequence_effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .and_then(super::search_reveal_and_sacrifice::describe_iterated_player_search_sequence),
        effects => {
            super::search_reveal_and_sacrifice::describe_iterated_player_search_effects(effects)
        }
    };
    if let Some(search) = iterated_search {
        let search = join_dynamic_search_move_before_where_x(search);
        let search =
            super::search_reveal_and_sacrifice::rewrite_iterated_player_references(&search);
        let search = search
            .strip_prefix("Search ")
            .map_or(search.clone(), |rest| format!("search {rest}"));
        return Some(
            search
                .replace(
                    ", put it into their hand",
                    " and put that card into their hand",
                )
                .replace(
                    ", put them into their hand",
                    " and put those cards into their hand",
                )
                .replace(
                    ", put it onto the battlefield",
                    " and put it onto the battlefield",
                )
                .replace(
                    ", put them onto the battlefield",
                    " and put them onto the battlefield",
                ),
        );
    }
    describe_for_players_may_action(filter, effects)
}

pub(super) fn describe_sequential_any_player_may_action(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if !for_players.starting_with_controller || !for_players.stop_after_first_happened {
        return None;
    }
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let subject = match &for_players.filter {
        PlayerFilter::Any => "Any player".to_string(),
        PlayerFilter::Opponent => "Any opponent".to_string(),
        PlayerFilter::NotYou => "Any player other than you".to_string(),
        filter => format!("Any {}", describe_for_each_player_filter(filter)),
    };

    // Source-attributed damage is causative: the offered player decides
    // whether the spell/permanent deals damage to them. Rendering the nested
    // action as an imperative loses both the source and that relationship.
    let visible_effects = may
        .effects
        .iter()
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                    .is_none()
        })
        .collect::<Vec<_>>();
    if let [effect] = visible_effects.as_slice() {
        let effect = structural_unwrap_render_wrappers(effect);
        let damage = effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .or_else(|| {
                effect
                    .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
                    .and_then(|execute| {
                        structural_unwrap_render_wrappers(&execute.effect)
                            .downcast_ref::<crate::effects::DealDamageEffect>()
                    })
            });
        if let Some(damage) = damage
            && !damage.source_is_combat
            && !damage.unpreventable
            && damage.target == ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        {
            return Some(format!(
                "{subject} may have it deal {} damage to them",
                describe_value(&damage.amount)
            ));
        }
    }

    let action =
        describe_contextualized_iterated_player_may_action(&for_players.filter, &may.effects)?;
    Some(format!("{subject} may {action}"))
}

pub(super) fn describe_for_players_may_happened_sequence(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if let Some(compact) = describe_for_players_may_copy_spell_and_choose_new_targets(for_players) {
        return Some(compact);
    }
    if let Some(compact) = describe_for_players_conditional_may_happened_sequence(for_players) {
        return Some(compact);
    }

    if for_players.starting_with_controller {
        return None;
    }

    // The correlated result may live outside the optional action (the older
    // `[WithId(May), If]` lowering) or inside the same per-player optional
    // block (`May([WithId(Action), If])`). Both represent the same rules:
    // only players who performed the optional action receive the follow-up.
    let (with_id, may, if_effect, action_effects): (
        &crate::effects::WithIdEffect,
        &crate::effects::MayEffect,
        &crate::effects::IfEffect,
        &[Effect],
    ) = match for_players.effects.as_slice() {
        [with_id_effect, if_effect] => {
            let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
            let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
            let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
            (with_id, may, if_effect, &may.effects)
        }
        [may_effect] => {
            let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
            // The pair may sit directly in the optional block or inside one
            // authored comma-then sequence ("may draw a card, then ...").
            let inner: &[Effect] = match may.effects.as_slice() {
                [single] => single
                    .downcast_ref::<crate::effects::SequenceEffect>()
                    .map(|sequence| sequence.effects.as_slice())
                    .unwrap_or(std::slice::from_ref(single)),
                other => other,
            };
            let [with_id_effect, if_effect] = inner else {
                return None;
            };
            let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
            let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
            (
                with_id,
                may,
                if_effect,
                std::slice::from_ref(&with_id.effect),
            )
        }
        _ => return None,
    };
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let subject = describe_for_players_subject(&for_players.filter)?.to_string();
    let mut each_player =
        strip_leading_article(&describe_for_each_player_filter(&for_players.filter)).to_string();
    // The subject clause already scoped the group ("Each player on your team
    // may ..."); a wordy filter repeated here reads badly, and the "who
    // {did_action} this way" restriction keeps the reference unambiguous.
    if each_player.contains(' ') {
        each_player = "player".to_string();
    }
    let action = describe_for_players_may_action(&for_players.filter, action_effects)?;
    let did_action = may_action_this_way_phrase(&action)?;
    let followup = describe_for_players_happened_followup(&if_effect.then)?;
    Some(format!(
        "{subject} may {action}, then each {each_player} who {did_action} {followup}"
    ))
}

fn describe_for_players_conditional_may_happened_sequence(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [with_id_effect, if_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let conditional = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let relative =
        super::structural_bundles::relative_iterated_player_condition(&conditional.condition)?;
    let subject = describe_for_players_subject(&for_players.filter)?.to_string();
    let action =
        describe_contextualized_iterated_player_may_action(&for_players.filter, &may.effects)?;
    let did_action = may_action_this_way_phrase(&action)?;
    let mut followup = describe_for_players_happened_followup(&if_effect.then)?;
    if did_action == "searched their library this way"
        && matches!(
            followup.as_str(),
            "shuffle that player's library"
                | "shuffle their library"
                | "shuffles that player's library"
                | "shuffles their library"
        )
    {
        followup = "shuffles".to_string();
    }
    let each_player = if subject == "Each opponent" {
        "opponent"
    } else {
        "player"
    };
    Some(format!(
        "{subject} who {relative} may {action}. Then each {each_player} who {did_action} {followup}"
    ))
}

fn wrapped_with_id_effect(effect: &Effect) -> Option<&crate::effects::WithIdEffect> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Some(with_id);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return wrapped_with_id_effect(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return wrapped_with_id_effect(&tag_all.effect);
    }
    None
}

/// Keep a copied spell and the optional retargeting of that same copy in one
/// per-player action. The effect ID is the semantic link; the presentation is
/// therefore safe for any spell-copy effect with this lowering shape.
fn describe_for_players_may_copy_spell_and_choose_new_targets(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let [copy_effect, choose_targets_effect] = may.effects.as_slice() else {
        return None;
    };
    let with_id = wrapped_with_id_effect(copy_effect)?;
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    let choose_targets =
        choose_targets_effect.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if copy.count != Value::Fixed(1)
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
        || copy.copier != PlayerFilter::IteratedPlayer
        || choose_targets.from_effect != with_id.id
        || !choose_targets.may
        || choose_targets.chooser != Some(PlayerFilter::IteratedPlayer)
        || choose_targets.single_target_surface
    {
        return None;
    }

    let subject = describe_for_players_subject(&for_players.filter)?;
    let copied_spell = describe_stack_object_copy_target(&copy.target);
    Some(format!(
        "{subject} may copy {copied_spell} and may choose new targets for the copy they control"
    ))
}

#[cfg(test)]
mod per_player_stack_copy_tests {
    use super::*;

    #[test]
    fn triggering_spell_controller_exclusion_keeps_each_other_player_surface() {
        let copy_id = crate::effect::EffectId(9);
        let copy = Effect::with_id(
            copy_id.0,
            Effect::new(crate::effects::CopySpellEffect::new_for_player(
                ChooseSpec::Tagged(TagKey::from("triggering")),
                Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )),
        )
        .tag(TagKey::from("__copied_stack_object__"));
        let retarget = Effect::new(crate::effects::ChooseNewTargetsEffect::may_for_player(
            copy_id,
            PlayerFilter::IteratedPlayer,
        ));
        let may = Effect::may_player(PlayerFilter::IteratedPlayer, vec![copy, retarget]);
        let for_players = crate::effects::ForPlayersEffect::new(
            PlayerFilter::excluding(
                PlayerFilter::Any,
                PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::tagged("triggering")),
            ),
            vec![may],
        );

        assert_eq!(
            describe_for_players_may_copy_spell_and_choose_new_targets(&for_players).as_deref(),
            Some(
                "Each other player may copy that spell and may choose new targets for the copy they control"
            )
        );
    }
}

pub(crate) fn describe_with_id_then_for_players_if_happened(
    with_id: &crate::effects::WithIdEffect,
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let antecedent = with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if antecedent.filter != for_players.filter
        || antecedent.starting_with_controller
        || for_players.starting_with_controller
        || for_players.effects.len() != 1
    {
        return None;
    }
    let if_effect = for_players.effects[0].downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let (subject, each_player, action) = describe_for_players_may_clause(antecedent)?;
    let did_action = may_action_this_way_phrase(&action)?;
    let followup = describe_for_players_happened_followup(&if_effect.then)?;
    Some(format!(
        "{subject} may {action}, then each {each_player} who {did_action} {followup}"
    ))
}

pub(super) fn apply_grants_chosen_color_protection(
    apply: &crate::effects::ApplyContinuousEffect,
) -> bool {
    if apply.until != Until::EndOfTurn
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.condition.is_some()
    {
        return false;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return false;
    };
    matches!(
        ability.protection_from(),
        Some(crate::ability::ProtectionFrom::ChosenColor)
    )
}

pub(super) fn is_radiance_target_creature_spec(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Target(inner) = spec else {
        return false;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    filter.card_types.as_slice() == [CardType::Creature]
        && filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && !filter.other
        && filter.tagged_constraints.is_empty()
        && filter.any_of.is_empty()
}

pub(super) fn shared_color_other_creature_filter(filter: &ObjectFilter) -> Option<&crate::TagKey> {
    if filter.card_types.as_slice() != [CardType::Creature]
        || filter.zone != Some(Zone::Battlefield)
        || filter.other
    {
        return None;
    }
    let shares = filter.tagged_constraints.iter().find(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
    })?;
    let excludes_same = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == shares.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    });
    excludes_same.then_some(&shares.tag)
}

pub(super) fn describe_choose_color_target_and_shared_color_protection(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, target_grant_effect, shared_grant_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseColorEffect>()?;
    if choose.chooser != PlayerFilter::You {
        return None;
    }

    let target_grant = unwrap_basic_tag_wrappers(target_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let shared_grant = unwrap_basic_tag_wrappers(shared_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_grants_chosen_color_protection(target_grant)
        || !apply_grants_chosen_color_protection(shared_grant)
    {
        return None;
    }
    if !matches!(target_grant.target, crate::continuous::EffectTarget::Source)
        || !target_grant
            .target_spec
            .as_ref()
            .is_some_and(is_radiance_target_creature_spec)
    {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(shared_filter) = &shared_grant.target else {
        return None;
    };
    shared_color_other_creature_filter(shared_filter)?;

    Some(
        "Radiance — Choose a color. Target creature and each other creature that shares a color with it gain protection from the chosen color until end of turn"
            .to_string(),
    )
}

pub(super) fn apply_grants_inline_ability_until_eot(
    apply: &crate::effects::ApplyContinuousEffect,
) -> Option<(String, bool)> {
    if apply.until != Until::EndOfTurn
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.condition.is_some()
    {
        return None;
    }
    let (rendered, is_keyword) = match &apply.modification {
        Some(crate::continuous::Modification::AddAbilityGeneric(ability)) => match &ability.kind {
            AbilityKind::Static(static_ability) if static_ability.is_keyword() => {
                (static_ability.display().to_ascii_lowercase(), true)
            }
            _ => (describe_inline_ability(ability), false),
        },
        Some(crate::continuous::Modification::AddAbility(ability)) if ability.is_keyword() => {
            (ability.display().to_ascii_lowercase(), true)
        }
        Some(crate::continuous::Modification::AddAbility(ability)) => (
            describe_static_ability_with_subject(ability, "This creature"),
            false,
        ),
        _ => return None,
    };
    Some((
        rendered.trim().trim_end_matches('.').to_string(),
        is_keyword,
    ))
}

pub(super) fn describe_target_and_shared_color_pt_change(effects: &[Effect]) -> Option<String> {
    let visible = effects.iter().filter(|effect| structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::TagMatchingObjectsEffect>().is_none()).collect::<Vec<_>>();
    let [first_effect, second_effect] = visible.as_slice() else { return None; };
    let first = structural_unwrap_render_wrappers(first_effect).downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let second = structural_unwrap_render_wrappers(second_effect).downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let target = first.target_spec.as_ref()?;
    if !is_radiance_target_creature_spec(target) || first.until != second.until
        || first.condition.is_some() || second.condition.is_some()
        || first.modification.is_some() || second.modification.is_some()
        || !first.additional_modifications.is_empty() || !second.additional_modifications.is_empty()
        || first.runtime_modifications != second.runtime_modifications
    { return None; }
    let [crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness }] = first.runtime_modifications.as_slice() else { return None; };
    let crate::continuous::EffectTarget::Filter(filter) = &second.target else { return None; };
    let tag = shared_color_other_creature_filter(filter)?;
    if wrapped_effect_tag(first_effect)? != tag { return None; }
    let mut plain = filter.clone();
    plain.tagged_constraints.clear();
    plain.set_explicit_card_type_noun(None);
    if plain != ObjectFilter::creature() { return None; }
    Some(format!("Radiance — Target creature and each other creature that shares a color with it get {}/{} {}",
        describe_signed_value(power), describe_signed_value(toughness), describe_until(&first.until)))
}

pub(super) fn describe_target_and_shared_color_inline_ability_grant(
    effects: &[Effect],
) -> Option<String> {
    let mut visible = effects.iter().filter(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
            .is_none()
    });
    let target_grant_effect = visible.next()?;
    let shared_grant_effect = visible.next()?;
    if visible.next().is_some() {
        return None;
    }
    let target_grant = unwrap_basic_tag_wrappers(target_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let shared_grant = unwrap_basic_tag_wrappers(shared_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let (ability_text, is_keyword) = apply_grants_inline_ability_until_eot(target_grant)?;
    if apply_grants_inline_ability_until_eot(shared_grant)? != (ability_text.clone(), is_keyword) {
        return None;
    }
    if !matches!(target_grant.target, crate::continuous::EffectTarget::Source)
        || !target_grant
            .target_spec
            .as_ref()
            .is_some_and(is_radiance_target_creature_spec)
    {
        return None;
    };
    let crate::continuous::EffectTarget::Filter(shared_filter) = &shared_grant.target else {
        return None;
    };
    shared_color_other_creature_filter(shared_filter)?;

    if is_keyword {
        Some(format!(
            "Radiance — Target creature and each other creature that shares a color with it gain {ability_text} until end of turn"
        ))
    } else {
        Some(format!(
            "Radiance — Until end of turn, target creature and each other creature that shares a color with it gain \"{ability_text}.\""
        ))
    }
}

pub(super) fn describe_choose_phase_then_skip_chosen_this_turn(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, conditional_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseNamedOptionEffect>()?;
    let options = choose
        .options
        .iter()
        .map(|option| option.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if options != ["draw step", "main phase", "combat phase"] {
        return None;
    }

    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::SourceChosenOption(draw_option) = &conditional.condition else {
        return None;
    };
    if !draw_option.eq_ignore_ascii_case("draw step") || conditional.if_true.len() != 1 {
        return None;
    }
    let draw_skip = conditional.if_true[0].downcast_ref::<crate::effects::SkipDrawStepEffect>()?;
    if draw_skip.player != choose.chooser || conditional.if_false.len() != 1 {
        return None;
    }

    let main_conditional =
        conditional.if_false[0].downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::SourceChosenOption(main_option) = &main_conditional.condition
    else {
        return None;
    };
    if !main_option.eq_ignore_ascii_case("main phase")
        || main_conditional.if_true.len() != 1
        || main_conditional.if_false.len() != 1
    {
        return None;
    }
    let main_skip = main_conditional.if_true[0]
        .downcast_ref::<crate::effects::SkipMainPhasesThisTurnEffect>()?;
    let combat_skip = main_conditional.if_false[0]
        .downcast_ref::<crate::effects::SkipCombatPhasesThisTurnEffect>()?;
    if main_skip.player != choose.chooser || combat_skip.player != choose.chooser {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let skip_subject = if chooser == "that player" {
        "The player".to_string()
    } else {
        capitalize_first(&chooser)
    };
    Some(format!(
        "{chooser} {choose_verb} draw step, main phase, or combat phase. {skip_subject} skips each instance of the chosen step or phase this turn"
    ))
}

pub(super) fn title_case_vote_option(option: &str) -> String {
    option
        .split_whitespace()
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0 && matches!(word, "a" | "an" | "and" | "of" | "or" | "the") {
                return word.to_string();
            }
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn describe_search_basic_land_battlefield_tapped_shuffle(
    effects: &[Effect],
) -> Option<String> {
    let [search_effect, shuffle_effect] = effects else {
        return None;
    };
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sequence = search_with_id
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, put_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = put_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = shuffle.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if !choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.owner != Some(PlayerFilter::You)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
        || !choose.count.is_single()
        || put_each.tag != choose.tag
        || !matches!(put.target, ChooseSpec::Iterated)
        || !put.tapped
        || put.controller != PlayerFilter::You
        || shuffle.condition != search_with_id.id
        || shuffle.predicate != EffectPredicate::Happened
        || !shuffle.else_.is_empty()
        || shuffle_library.player != PlayerFilter::You
    {
        return None;
    }

    Some(
        "search your library for a basic land card and put it onto the battlefield tapped. If you search your library this way, shuffle"
            .to_string(),
    )
}

pub(super) fn describe_may_search_basic_land_then_shuffle(
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = search_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let actor = may.decider.as_ref()?;
    let search_effects = if let [sequence_effect] = may.effects.as_slice()
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        may.effects.as_slice()
    };
    let [choose_effect, put_effect] = search_effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let if_effect = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = if_effect.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if if_effect.condition != search_with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || choose.chooser != *actor
        || choose.filter.owner.as_ref() != Some(actor)
        || shuffle_library.player != *actor
        || !choose.is_search
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
    {
        return None;
    }

    let mut compact =
        describe_search_choose_for_each(choose, put_each, Some(shuffle_library), false)?;
    compact = compact
        .replace("its controller's library", "their library")
        .replace("that object's controller's library", "their library")
        .replace("its owner's library", "their library")
        .replace("that object's owner's library", "their library");
    let rest = compact.strip_prefix("Search ")?;
    Some(format!(
        "{} may search {}",
        capitalize_first(&describe_player_filter(actor)),
        lowercase_first(rest)
    ))
}

pub(crate) fn same_search_player_filter(left: &PlayerFilter, right: &PlayerFilter) -> bool {
    player_filters_refer_to_same_player(left, right)
        || matches!(
            (left, right),
            (
                PlayerFilter::ControllerOf(_) | PlayerFilter::AliasedControllerOf(_),
                PlayerFilter::ControllerOf(_) | PlayerFilter::AliasedControllerOf(_),
            ) | (
                PlayerFilter::OwnerOf(_) | PlayerFilter::AliasedOwnerOf(_),
                PlayerFilter::OwnerOf(_) | PlayerFilter::AliasedOwnerOf(_),
            )
        )
}

pub(super) fn normalize_actor_owned_search_origin(actor: &PlayerFilter, text: String) -> String {
    match actor {
        PlayerFilter::ControllerOf(_) => text
            .replace("its controller's library", "their library")
            .replace("that object's controller's library", "their library"),
        PlayerFilter::OwnerOf(_) => text
            .replace("its owner's library", "their library")
            .replace("that object's owner's library", "their library"),
        _ => text,
    }
}

pub(super) fn describe_may_search_sequence_then_shuffle(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    let actor = may.decider.as_ref()?;
    let search_effects = if let [sequence_effect] = may.effects.as_slice()
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        may.effects.as_slice()
    };
    let [choose_effect, for_each_effect, shuffle_effect] = search_effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let search_owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);

    if !same_search_player_filter(&choose.chooser, actor)
        || !same_search_player_filter(search_owner, actor)
        || !same_search_player_filter(&shuffle.player, actor)
    {
        return None;
    }

    let compact = describe_search_choose_for_each(choose, for_each, Some(shuffle), false)?;
    let compact = normalize_actor_owned_search_origin(actor, compact)
        .replace(". Then that player shuffles", ", then shuffle")
        .replace(". Then they shuffle", ", then shuffle");
    let rest = compact.strip_prefix("Search ")?;
    Some(format!(
        "{} may search {}",
        capitalize_first(&describe_player_filter(actor)),
        lowercase_first(rest)
    ))
}

pub(super) fn describe_search_sequence_then_shuffle(
    sequence: &crate::effects::SequenceEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    let [choose_effect, for_each_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    describe_search_choose_for_each(choose, for_each, Some(shuffle), false)
}

pub(super) fn describe_wrapped_search_for_each_then_conditional_shuffle(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let search_effect = *effects.first()?;
    let (search_with_id, choose, for_each, shuffle_effect, consumed) = if let Some(choose) =
        search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
    {
        let (Some(search_with_id), for_each) = for_each_tagged_for_compaction(*effects.get(1)?)?
        else {
            return None;
        };
        (search_with_id, choose, for_each, *effects.get(2)?, 3)
    } else {
        let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        if let Some(sequence) = search_with_id
            .effect
            .downcast_ref::<crate::effects::SequenceEffect>()
        {
            let [choose_effect, for_each_effect] = sequence.effects.as_slice() else {
                return None;
            };
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, for_each) = for_each_tagged_for_compaction(for_each_effect)?;
            (search_with_id, choose, for_each, *effects.get(1)?, 2)
        } else {
            let choose = search_with_id
                .effect
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, for_each) = for_each_tagged_for_compaction(*effects.get(1)?)?;
            (search_with_id, choose, for_each, *effects.get(2)?, 3)
        }
    };
    let if_effect = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = if_effect.then.as_slice() else {
        return None;
    };
    let shuffle = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if if_effect.condition != search_with_id.id
        || if_effect.predicate != EffectPredicate::SearchedLibrary
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    Some((
        describe_search_choose_for_each(choose, for_each, Some(shuffle), false)?,
        consumed,
    ))
}

pub(super) fn unwrap_tags_for_from_the_ashes_shape(effect: &Effect) -> &Effect {
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_tags_for_from_the_ashes_shape(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_tags_for_from_the_ashes_shape(&tagged.effect);
    }
    effect
}

pub(super) fn is_destroy_all_nonbasic_lands_effect(effect: &Effect) -> bool {
    let Some(destroy) = unwrap_tags_for_from_the_ashes_shape(effect)
        .downcast_ref::<crate::effects::DestroyEffect>()
    else {
        return false;
    };
    let ChooseSpec::All(filter) = &destroy.spec else {
        return false;
    };
    filter.card_types.as_slice() == [CardType::Land]
        && filter.excluded_supertypes.contains(&Supertype::Basic)
}

pub(super) fn describe_destroyed_land_controller_basic_search_then_player_shuffle(
    destroy_effect: &Effect,
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    if !is_destroy_all_nonbasic_lands_effect(destroy_effect) {
        return None;
    }

    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let destroyed_loop = search_with_id
        .effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let (may_effect, lki_iterated_tag) =
        destroyed_land_controller_search_may_effect(destroyed_loop.effects.as_slice())?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let actor = may.decider.as_ref()?;
    let [search_sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let search_sequence =
        search_sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, put_effect] = search_sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = put_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = shuffle.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if !destroyed_land_controller_search_actor(actor, lki_iterated_tag)
        || !choose.is_search
        || choose.chooser != *actor
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.owner.as_ref() != Some(actor)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
        || !choose.count.is_single()
        || put_each.tag != choose.tag
        || !matches!(put.target, ChooseSpec::Iterated)
        || put.tapped
        || put.controller != *actor
        || shuffle.condition != search_with_id.id
        || shuffle.predicate != EffectPredicate::Happened
        || !shuffle.else_.is_empty()
        || shuffle_library.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }

    Some("Destroy all nonbasic lands. For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles".to_string())
}

fn destroyed_land_controller_search_may_effect(
    effects: &[Effect],
) -> Option<(&Effect, Option<&TagKey>)> {
    if let [may_effect] = effects
        && may_effect
            .downcast_ref::<crate::effects::MayEffect>()
            .is_some()
    {
        return Some((may_effect, None));
    }

    let [conditional_effect] = effects else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatchedLastKnown(iterated_tag, filter) = &conditional.condition
    else {
        return None;
    };
    let mut plain_land_lki = ObjectFilter::land();
    plain_land_lki.zone = None;
    if iterated_tag.as_str() != "__it__"
        || filter != &plain_land_lki
        || !conditional.if_false.is_empty()
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
    {
        return None;
    }
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    may_effect
        .downcast_ref::<crate::effects::MayEffect>()
        .map(|_| (may_effect, Some(iterated_tag)))
}

fn destroyed_land_controller_search_actor(
    actor: &PlayerFilter,
    lki_iterated_tag: Option<&TagKey>,
) -> bool {
    if actor == &PlayerFilter::IteratedPlayer {
        return true;
    }
    let Some(expected_tag) = lki_iterated_tag else {
        return false;
    };
    matches!(
        actor,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(actor_tag))
            if actor_tag == expected_tag
    )
}

#[cfg(test)]
mod destroyed_land_controller_search_lki_tests {
    use super::*;

    fn may_search_effect(actor: PlayerFilter) -> Effect {
        Effect::may_player(actor, vec![Effect::draw(1)])
    }

    #[test]
    fn exact_land_lki_guard_exposes_its_controller_may_effect() {
        let iterated_tag = TagKey::from("__it__");
        let actor =
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(iterated_tag.clone()));
        let mut land_lki = ObjectFilter::land();
        land_lki.zone = None;
        let guarded = Effect::conditional_only(
            Condition::TaggedObjectMatchedLastKnown(iterated_tag.clone(), land_lki),
            vec![may_search_effect(actor.clone())],
        );

        let (may_effect, lki_tag) =
            destroyed_land_controller_search_may_effect(std::slice::from_ref(&guarded))
                .expect("exact destroyed-land LKI guard should unwrap");
        assert!(
            may_effect
                .downcast_ref::<crate::effects::MayEffect>()
                .is_some()
        );
        assert_eq!(lki_tag, Some(&iterated_tag));
        assert!(destroyed_land_controller_search_actor(&actor, lki_tag));
    }

    #[test]
    fn near_miss_lki_guard_does_not_claim_the_destroyed_land_surface() {
        let iterated_tag = TagKey::from("__it__");
        let actor =
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(iterated_tag.clone()));
        let wrong_filter = Effect::conditional_only(
            Condition::TaggedObjectMatchedLastKnown(iterated_tag.clone(), ObjectFilter::creature()),
            vec![may_search_effect(actor.clone())],
        );
        assert!(
            destroyed_land_controller_search_may_effect(std::slice::from_ref(&wrong_filter))
                .is_none()
        );

        let mismatched_actor = PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(
            TagKey::from("other_object"),
        ));
        assert!(!destroyed_land_controller_search_actor(
            &mismatched_actor,
            Some(&iterated_tag),
        ));

        let zoned_land = Effect::conditional_only(
            Condition::TaggedObjectMatchedLastKnown(iterated_tag, ObjectFilter::land()),
            vec![may_search_effect(actor)],
        );
        assert!(
            destroyed_land_controller_search_may_effect(std::slice::from_ref(&zoned_land))
                .is_none(),
            "the LKI terminal filter must not carry a live battlefield-zone constraint"
        );
    }
}

pub(super) fn describe_destroyed_land_basic_search_then_player_shuffle(
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let search_effects = if let Some(destroyed_loop) = search_with_id
        .effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>(
    ) {
        destroyed_loop.effects.as_slice()
    } else if let Some(destroyed_loop) = search_with_id
        .effect
        .downcast_ref::<crate::effects::ForEachObject>()
    {
        destroyed_loop.effects.as_slice()
    } else {
        return None;
    };

    describe_optional_basic_land_search_effects(search_effects)?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = shuffle.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if shuffle.condition != search_with_id.id
        || shuffle.predicate != EffectPredicate::Happened
        || !shuffle.else_.is_empty()
        || shuffle_library.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }

    Some("For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles".to_string())
}

pub(super) fn player_filter_is_target_opponentish(player: &PlayerFilter) -> bool {
    matches!(player, PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Opponent) || player_filter_is_target_opponentish(inner))
}

pub(super) fn search_target_opponent_library_to_graveyard_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<()> {
    let [choose_effect, move_each_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_each = move_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [move_effect] = move_each.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    if !choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose
            .filter
            .owner
            .as_ref()
            .is_none_or(|owner| !player_filter_is_target_opponentish(owner))
        || choose.filter.card_types.as_slice() != [CardType::Creature]
        || choose.count.min != 0
        || choose.count.max != Some(3)
        || move_each.tag != choose.tag
        || !matches!(move_to_zone.target, ChooseSpec::Iterated)
        || move_to_zone.zone != Zone::Graveyard
    {
        return None;
    }

    Some(())
}

pub(super) fn describe_destroy_then_search_target_opponent_to_graveyard_then_shuffle(
    destroy_effect: &Effect,
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    if !matches!(destroy.spec.base(), ChooseSpec::All(_)) {
        return None;
    }
    let sequence = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    search_target_opponent_library_to_graveyard_sequence(sequence)?;
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !player_filter_is_target_opponentish(&shuffle.player) {
        return None;
    }

    let destroy_text = capitalize_first(describe_effect(destroy_effect).trim_end_matches('.'));
    if destroy_text.is_empty() {
        return None;
    }
    let mut search_text = lowercase_first(describe_effect(search_effect).trim_end_matches('.'));
    search_text = search_text.replace("up to 3", "up to three");
    search_text = search_text.replace("target opponent's graveyard", "their graveyard");
    if let Some(replaced) = search_text.strip_suffix(", put them into their graveyard") {
        search_text = format!("{replaced} and put them into their graveyard");
    }

    Some(format!(
        "{destroy_text}, then {search_text}. Then that player shuffles"
    ))
}

pub(super) fn describe_destroy_then_target_opponent_search_to_graveyard_then_shuffle(
    destroy_effect: &Effect,
    target_effect: &Effect,
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.explicit_declaration
        || target.chooser.is_some()
        || target.target != ChooseSpec::target_opponent()
    {
        return None;
    }
    describe_destroy_then_search_target_opponent_to_graveyard_then_shuffle(
        destroy_effect,
        search_effect,
        shuffle_effect,
    )
}

pub(super) fn describe_for_each_tagged_optional_basic_land_search(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    describe_optional_basic_land_search_effects(for_each.effects.as_slice())
}

pub(super) fn describe_optional_basic_land_search_effects(effects: &[Effect]) -> Option<String> {
    let search_effects = if let [may_effect] = effects
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
    {
        may.effects.as_slice()
    } else {
        effects
    };
    let search_effects = if let [sequence_effect] = search_effects
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        search_effects
    };
    let (choose_effect, put_effect) = if let [may_effect, put_effect] = search_effects
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
    {
        let may_effects = if let [sequence_effect] = may.effects.as_slice()
            && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            may.effects.as_slice()
        };
        let [choose_effect] = may_effects else {
            return None;
        };
        (choose_effect, put_effect)
    } else {
        let [choose_effect, put_effect] = search_effects else {
            return None;
        };
        (choose_effect, put_effect)
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = put_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let puts_searched_object = matches!(put.target, ChooseSpec::Iterated)
        || matches!(&put.target, ChooseSpec::Tagged(tag) if tag == &choose.tag);

    if !choose.is_search
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
        || !choose.count.is_single()
        || put_each.tag != choose.tag
        || !puts_searched_object
        || put.tapped
    {
        return None;
    }

    Some("For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield".to_string())
}

pub(super) fn describe_council_dilemma_named_vote_sequence(effects: &[Effect]) -> Option<String> {
    let [vote_effect, repeat_effects @ ..] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || !vote.starting_with_controller
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || repeat_effects.len() < options.len()
        || options
            .iter()
            .any(|option| !option.effects_per_vote.is_empty())
    {
        return None;
    }

    let mut clauses = Vec::new();
    let mut subject_last_clauses = Vec::new();
    let (repeat_effects, trailing_effects) = repeat_effects.split_at(options.len());
    for (option, repeat_effect) in options.iter().zip(repeat_effects.iter()) {
        let repeat = repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
        let Value::VoteCount(repeat_option) = &repeat.count else {
            return None;
        };
        if !repeat_option.eq_ignore_ascii_case(&option.name) {
            return None;
        }

        let body = describe_search_basic_land_battlefield_tapped_shuffle(&repeat.effects)
            .unwrap_or_else(|| {
                let mut body = describe_effect_list(&repeat.effects)
                    .trim()
                    .trim_end_matches('.')
                    .to_string();
                body = body.replace(
                    ", put it onto the battlefield tapped",
                    " and put it onto the battlefield tapped",
                );
                body
            });
        clauses.push(format!(
            "For each {} vote, {}",
            title_case_vote_option(&option.name),
            lowercase_first(&body)
        ));
        subject_last_clauses.push(format!(
            "{} for each {} vote",
            lowercase_first(&body),
            option.name.to_ascii_lowercase()
        ));
    }

    let shared_quantified_subject = [
        "each opponent ",
        "each player ",
        "each other player ",
        "you ",
    ]
    .iter()
    .any(|prefix| {
        subject_last_clauses
            .iter()
            .all(|clause| clause.starts_with(prefix))
    });
    let option_names = options
        .iter()
        .map(|option| {
            if shared_quantified_subject {
                option.name.to_ascii_lowercase()
            } else {
                title_case_vote_option(&option.name)
            }
        })
        .collect::<Vec<_>>();
    let mut text = format!(
        "Council's dilemma — Starting with you, each player votes for {}",
        join_with_or(&option_names)
    );
    if shared_quantified_subject {
        text.push_str(". ");
        text.push_str(&capitalize_first(&compact_repeated_vote_clause_subjects(
            &subject_last_clauses,
        )));
    } else if !clauses.is_empty() {
        text.push_str(". ");
        text.push_str(&clauses.join(". "));
    }
    if !trailing_effects.is_empty() {
        let trailing = describe_effect_list(trailing_effects);
        if !trailing.trim().is_empty() {
            text.push_str(". ");
            text.push_str(&capitalize_first(trailing.trim().trim_end_matches('.')));
        }
    }
    Some(text)
}

/// Reassemble the typed vote program used by a council's-dilemma option that
/// repeats a reveal-until-one procedure, then moves the accumulated matches
/// and disposes of the revealed remainder once. Lowering intentionally keeps
/// the move and remainder effects outside the repeat, so the ordinary
/// contiguous-per-option renderer cannot recover this authored surface.
pub(super) fn describe_named_vote_repeated_consult_collection_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [
        vote_effect,
        consult_repeat_effect,
        collection_effects @ ..,
        other_repeat_effect,
    ] = effects
    else {
        return None;
    };
    let (move_effect, shuffle_effect) = match collection_effects {
        [move_effect, shuffle_effect] => (move_effect, shuffle_effect),
        [sequence_effect] => {
            let sequence = unwrap_basic_tag_wrappers(sequence_effect)
                .downcast_ref::<crate::effects::SequenceEffect>()?;
            let [move_effect, shuffle_effect] = sequence.effects.as_slice() else {
                return None;
            };
            (move_effect, shuffle_effect)
        }
        _ => return None,
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    let [consult_option, other_option] = options.as_slice() else {
        return None;
    };
    if vote.secret
        || !vote.starting_with_controller
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || !consult_option.effects_per_vote.is_empty()
        || !other_option.effects_per_vote.is_empty()
    {
        return None;
    }

    let consult_repeat =
        consult_repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let Value::VoteCount(consult_option_name) = &consult_repeat.count else {
        return None;
    };
    if !consult_option_name.eq_ignore_ascii_case(&consult_option.name) {
        return None;
    }
    let [consult_effect] = consult_repeat.effects.as_slice() else {
        return None;
    };
    let consult = unwrap_basic_tag_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.max_exposed.is_some()
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
    {
        return None;
    }

    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || !choose_spec_references_tagged_object(&move_to_zone.target, &consult.match_tag)
    {
        return None;
    }

    let shuffle = unwrap_basic_tag_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
    let remainder_is_linked = matches!(
        shuffle.target.base(),
        ChooseSpec::Tagged(tag)
            if tag == &consult.all_tag || tag.as_str().eq_ignore_ascii_case("rest")
    );
    if !remainder_is_linked
        || shuffle.player != PlayerFilter::You
        || shuffle.owner_library_destination
    {
        return None;
    }

    let other_repeat = other_repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let Value::VoteCount(other_option_name) = &other_repeat.count else {
        return None;
    };
    if !other_option_name.eq_ignore_ascii_case(&other_option.name)
        || other_repeat.effects.is_empty()
    {
        return None;
    }

    let consult_text = describe_effect(consult_effect);
    let consult_text = consult_text
        .trim()
        .trim_end_matches('.')
        .strip_prefix("You ")
        .or_else(|| {
            consult_text
                .trim()
                .trim_end_matches('.')
                .strip_prefix("you ")
        })
        .unwrap_or_else(|| consult_text.trim().trim_end_matches('.'));
    let selected = describe_library_consult_selection_with_cards(&consult.filter);
    let selected_plural = pluralize_noun_phrase(strip_leading_article(&selected));
    let other_body = describe_effect_list(&other_repeat.effects);
    let other_body = other_body.trim().trim_end_matches('.');
    let other_clause = if let Some(body) = other_body
        .strip_prefix("you ")
        .or_else(|| other_body.strip_prefix("You "))
    {
        format!(
            "You {body} for each {} vote",
            other_option.name.to_ascii_lowercase()
        )
    } else {
        format!(
            "For each {} vote, {}",
            other_option.name.to_ascii_lowercase(),
            lowercase_first(other_body)
        )
    };

    Some(format!(
        "Council's dilemma — Starting with you, each player votes for {}. {} for each {} vote. Put those {selected_plural} onto the battlefield, then shuffle the rest into your library. {other_clause}",
        join_with_or(
            &options
                .iter()
                .map(|option| option.name.to_ascii_lowercase())
                .collect::<Vec<_>>()
        ),
        capitalize_first(consult_text),
        consult_option.name.to_ascii_lowercase(),
    ))
}

/// Some per-vote clauses need the current voter and are embedded directly in
/// the vote effect, while voter-independent clauses are lowered as explicit
/// `RepeatEffectsEffect`s. Recombine those two executable representations so a
/// single council's-dilemma sentence does not lose either option.
pub(super) fn describe_hybrid_named_vote_per_vote_sequence(effects: &[Effect]) -> Option<String> {
    let [vote_effect, followups @ ..] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || !vote.starting_with_controller
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || !options
            .iter()
            .any(|option| !option.effects_per_vote.is_empty())
        || !options
            .iter()
            .any(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let mut merged = vote.clone();
    let crate::effects::VoteChoice::NamedOptions(merged_options) = &mut merged.choice else {
        unreachable!("choice shape checked above");
    };
    let mut consumed_followups = 0usize;
    for followup in followups {
        let Some(repeat) = followup.downcast_ref::<crate::effects::RepeatEffectsEffect>() else {
            break;
        };
        let Value::VoteCount(option_name) = &repeat.count else {
            break;
        };
        let option = merged_options
            .iter_mut()
            .find(|option| option.name.eq_ignore_ascii_case(option_name))?;
        if !option.effects_per_vote.is_empty() {
            return None;
        }
        option.effects_per_vote = repeat.effects.clone();
        consumed_followups += 1;
    }
    if merged_options
        .iter()
        .any(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let mut text = describe_named_vote_per_vote_effects(&merged)?;
    if consumed_followups < followups.len() {
        let trailing = describe_effect_list(&followups[consumed_followups..]);
        if !trailing.trim().is_empty() {
            text.push_str(". ");
            text.push_str(&capitalize_first(trailing.trim().trim_end_matches('.')));
        }
    }
    Some(text)
}

pub(super) fn describe_named_vote_per_vote_effects(
    vote: &crate::effects::VoteEffect,
) -> Option<String> {
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || !vote.starting_with_controller
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || options
            .iter()
            .any(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let option_names = options
        .iter()
        .map(|option| option.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut clauses = Vec::new();
    for option in options {
        let option_effects = if let [effect] = option.effects_per_vote.as_slice()
            && let Some(sequence) =
                unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            option.effects_per_vote.as_slice()
        };
        if let [effect] = option_effects
            && let Some(extra_turn) =
                unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::ExtraTurnEffect>()
            && extra_turn.player == PlayerFilter::You
        {
            clauses.push(format!(
                "For each {} vote, take an extra turn after this one",
                option.name.to_ascii_lowercase()
            ));
            continue;
        }
        if let [choose_effect, control_effect] = option_effects
            && let Some(choose) = unwrap_basic_tag_wrappers(choose_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(control) = unwrap_basic_tag_wrappers(control_effect)
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            && choose.chooser == PlayerFilter::You
            && choose.count.min == 1
            && choose.count.max == Some(1)
            && choose.count_value.is_none()
            && choose.filter.owner == Some(PlayerFilter::IteratedPlayer)
            && is_permanent_filter_in_zone(&choose.filter, Zone::Battlefield)
            && control.until == Until::Forever
            && control.condition.is_none()
            && control.modification.is_none()
            && control.additional_modifications.is_empty()
            && matches!(
                control.runtime_modifications.as_slice(),
                [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
            )
            && control
                .target_spec
                .as_ref()
                .is_some_and(|target| match target.base() {
                    // Lowering may retain the explicit tag produced by the
                    // voter-relative choice, while normalized fixtures can
                    // use the current iterated result directly. Both are the
                    // same executable dependency only when the tag agrees.
                    ChooseSpec::Tagged(tag) => tag == &choose.tag,
                    ChooseSpec::Iterated => true,
                    _ => false,
                })
        {
            clauses.push(format!(
                "For each {} vote, choose a permanent owned by the voter and gain control of it",
                option.name.to_ascii_lowercase()
            ));
            continue;
        }
        let body = describe_effect_list(option_effects)
            .trim()
            .trim_end_matches('.')
            .to_string();
        if body.is_empty() {
            return None;
        }
        clauses.push(format!(
            "{} for each {} vote",
            lowercase_first(&body),
            option.name.to_ascii_lowercase()
        ));
    }

    let combined = compact_repeated_vote_clause_subjects(&clauses);
    Some(format!(
        "Council's dilemma — Starting with you, each player votes for {}. {}",
        join_with_or(&option_names),
        if clauses.iter().all(|clause| clause.starts_with("For each ")) {
            clauses.join(". ")
        } else {
            capitalize_first(&combined)
        }
    ))
}

fn council_conditional_body(effects: &[Effect]) -> Option<String> {
    // Each-player graveyard return followed by exiling the source
    // ("If return gets more votes, each player returns ..., then you exile ~").
    if let [for_players_effect] = effects
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let [return_effect, exile_effect] = for_players.effects.as_slice()
        && let Some(move_to_zone) = unwrap_basic_tag_wrappers(exile_effect)
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Exile
        && matches!(move_to_zone.target.base(), ChooseSpec::Source)
        && let Some(return_text) = {
            let mut return_only = for_players.clone();
            return_only.effects = vec![return_effect.clone()];
            super::search_reveal_and_sacrifice::describe_each_player_return_from_graveyard_to_hand(
                &return_only,
            )
        }
    {
        let exile_text = describe_effect(exile_effect)
            .trim()
            .trim_end_matches('.')
            .to_string();
        // "then exile this" is rewritten to "then you exile {card name}" by
        // rewrite_inline_spell_self_exile in the spell-resolution pass.
        return Some(format!(
            "{return_text}, then {}",
            lowercase_first(&exile_text)
        ));
    }
    // A single optional per-player wheel keeps its one-action surface
    // ("each player may discard their hand and draw seven cards").
    if let [for_players_effect] = effects
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let [may_effect] = for_players.effects.as_slice()
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
        && may
            .decider
            .as_ref()
            .is_none_or(|decider| *decider == PlayerFilter::IteratedPlayer)
        && let [discard_effect, draw_effect] = may.effects.as_slice()
        && discard_effect
            .downcast_ref::<crate::effects::DiscardHandEffect>()
            .is_some_and(|discard| discard.player == PlayerFilter::IteratedPlayer)
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && draw.player == PlayerFilter::IteratedPlayer
    {
        let subject = describe_for_players_subject(&for_players.filter)?;
        return Some(format!(
            "{} may discard their hand and draw {}",
            lowercase_first(subject),
            describe_card_count(&draw.count)
        ));
    }
    let [sacrifice_effect, destroy_effect] = effects else {
        return None;
    };
    let sacrifice = unwrap_basic_tag_wrappers(sacrifice_effect)
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target.base(), ChooseSpec::Source) {
        return None;
    }
    unwrap_basic_tag_wrappers(destroy_effect).downcast_ref::<crate::effects::DestroyEffect>()?;
    let sacrifice_text = describe_effect(sacrifice_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let destroy_text = describe_effect(destroy_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{sacrifice_text} and {}",
        lowercase_first(&destroy_text)
    ))
}

pub(super) fn describe_secret_named_vote_repeat_followup(
    option: &str,
    repeat: &crate::effects::RepeatEffectsEffect,
) -> Option<String> {
    let option = option.to_ascii_lowercase();
    if let [effect] = repeat.effects.as_slice()
        && let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && draw.player == PlayerFilter::You
        && draw.count == Value::Fixed(1)
    {
        return Some(format!(
            "You draw cards equal to the number of {option} votes"
        ));
    }

    let mut body = describe_effect_list(&repeat.effects)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if body.is_empty() {
        return None;
    }
    if let Some(rest) = body.strip_prefix("Deal ") {
        body = format!("This deals {rest}");
    }
    Some(format!(
        "{} for each {option} vote",
        capitalize_first(&body)
    ))
}

pub(super) fn describe_secret_named_vote_intervening_followup(effect: &Effect) -> Option<String> {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()
        && choose.chooser == PlayerFilter::You
        && choose.filter == PlayerFilter::Opponent
        && choose.random
    {
        return Some("Then choose an opponent at random".to_string());
    }
    None
}

/// Secret votes can execute a voter-relative choice immediately, then apply
/// linked set effects after every ballot has been collected. Reconstruct that
/// relationship only when the chosen tag, gained-control tag, and granted
/// ability target all agree.
pub(super) fn describe_secret_vote_voter_choice_control_sequence(
    effects: &[Effect],
) -> Option<String> {
    let (vote_effect, control_effect, grant_effect, repeat_effect) = match effects {
        [vote_effect, control_effect, grant_effect, repeat_effect] => {
            (vote_effect, control_effect, grant_effect, repeat_effect)
        }
        [vote_effect, coordinated_effect, repeat_effect] => {
            let sequence = coordinated_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
                return None;
            }
            let [control_effect, grant_effect] = sequence.effects.as_slice() else {
                return None;
            };
            (vote_effect, control_effect, grant_effect, repeat_effect)
        }
        _ => return None,
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if !vote.secret
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() != 2
    {
        return None;
    }
    let choice_option = options
        .iter()
        .find(|option| !option.effects_per_vote.is_empty())?;
    let followup_option = options
        .iter()
        .find(|option| option.effects_per_vote.is_empty())?;
    let [choice_effect] = choice_option.effects_per_vote.as_slice() else {
        return None;
    };
    let choose = unwrap_basic_tag_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::IteratedPlayer
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count_value.is_some()
        || choose.filter.zone != Some(Zone::Battlefield)
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || choose.filter.card_types.as_slice() != [CardType::Creature]
    {
        return None;
    }

    let control_tag = direct_wrapped_effect_tag(control_effect)?;
    let control = unwrap_basic_tag_wrappers(control_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let aggregated_vote_choice_tag = TagKey::from("__chosen_objects__");
    if control.until != Until::Forever
        || control.condition.is_some()
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || !matches!(
            control.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
        || !control.target_spec.as_ref().is_some_and(|target| {
            choose_spec_references_tagged_object(target, &choose.tag)
                || choose_spec_references_tagged_object(target, &aggregated_vote_choice_tag)
        })
    {
        return None;
    }

    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.until != Until::Forever
        || grant.condition.is_some()
        || !grant.additional_modifications.is_empty()
        || !grant.runtime_modifications.is_empty()
        || !grant
            .target_spec
            .as_ref()
            .is_some_and(|target| choose_spec_references_tagged_object(target, control_tag))
    {
        return None;
    }
    let grants_cant_attack_its_owner = match &grant.modification {
        Some(crate::continuous::Modification::AddAbility(ability)) => {
            ability.id() == crate::static_abilities::StaticAbilityId::CantAttackItsOwner
        }
        Some(crate::continuous::Modification::AddAbilityGeneric(ability)) => matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::CantAttackItsOwner
        ),
        _ => false,
    };
    if !grants_cant_attack_its_owner {
        return None;
    }

    let repeat = repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let Value::VoteCount(repeat_option) = &repeat.count else {
        return None;
    };
    if !repeat_option.eq_ignore_ascii_case(&followup_option.name) {
        return None;
    }
    let repeat_body = describe_effect_list(&repeat.effects)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if repeat_body.is_empty() {
        return None;
    }

    Some(format!(
        "Secret council — Each player secretly votes for {}, then those votes are revealed. For each {} vote, the voter chooses a creature they control. You gain control of each creature chosen this way, and they gain \"This creature can't attack its owner.\" Then for each {} vote, {}",
        join_with_or(
            &options
                .iter()
                .map(|option| option.name.to_ascii_lowercase())
                .collect::<Vec<_>>()
        ),
        choice_option.name.to_ascii_lowercase(),
        followup_option.name.to_ascii_lowercase(),
        lowercase_first(&repeat_body)
    ))
}

pub(super) fn describe_secret_named_vote_followup_sequence(effects: &[Effect]) -> Option<String> {
    let [vote_effect, rest @ ..] = effects else {
        return None;
    };
    if rest.is_empty() {
        return None;
    }
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if !vote.secret
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || options
            .iter()
            .any(|option| !option.effects_per_vote.is_empty())
    {
        return None;
    }

    let option_names = options
        .iter()
        .map(|option| option.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut parts = vec![format!(
        "Secret council — Each player secretly votes for {}, then those votes are revealed",
        join_with_or(&option_names)
    )];

    for effect in rest {
        if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
            let Value::VoteCount(option) = &repeat.count else {
                return None;
            };
            if !option_names
                .iter()
                .any(|known| known.eq_ignore_ascii_case(option))
            {
                return None;
            }
            parts.push(describe_secret_named_vote_repeat_followup(option, repeat)?);
            continue;
        }
        parts.push(describe_secret_named_vote_intervening_followup(effect)?);
    }

    Some(parts.join(". "))
}

pub(super) fn compact_repeated_vote_clause_subjects(clauses: &[String]) -> String {
    for prefix in [
        "each opponent ",
        "each player ",
        "each other player ",
        "you ",
    ] {
        if clauses.iter().all(|clause| clause.starts_with(prefix)) {
            let rests = clauses
                .iter()
                .map(|clause| clause[prefix.len()..].to_string())
                .collect::<Vec<_>>();
            return format!("{prefix}{}", join_with_and(&rests));
        }
    }
    join_with_and(clauses)
}

pub(in crate::compiled_text) fn describe_planeswalk_chaos_vote_sequence(
    effects: &[&Effect],
) -> Option<String> {
    let [vote_effect, planeswalk_effect, chaos_effect] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let ironsmith_core::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || !vote.starting_with_controller
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() != 2
        || options[0].name != "planeswalk"
        || options[1].name != "chaos"
        || !options
            .iter()
            .all(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let planeswalk = planeswalk_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let chaos = chaos_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !planeswalk.if_false.is_empty()
        || !chaos.if_false.is_empty()
        || !matches!(
            &planeswalk.condition,
            Condition::VoteOptionGetsMoreVotes(option) if option == "planeswalk"
        )
        || !matches!(
            &chaos.condition,
            Condition::VoteOptionGetsMoreVotesOrTied(option) if option == "chaos"
        )
    {
        return None;
    }

    let [planeswalk_action] = planeswalk.if_true.as_slice() else {
        return None;
    };
    let [chaos_action] = chaos.if_true.as_slice() else {
        return None;
    };
    let planeswalk_emit =
        planeswalk_action.downcast_ref::<crate::effects::EmitKeywordActionEffect>()?;
    let chaos_emit = chaos_action.downcast_ref::<crate::effects::EmitKeywordActionEffect>()?;
    if planeswalk_emit.action != crate::events::KeywordActionKind::Planeswalk
        || planeswalk_emit.amount != 1
        || chaos_emit.action != crate::events::KeywordActionKind::ChaosEnsues
        || chaos_emit.amount != 1
    {
        return None;
    }

    Some(
        "Will of the Planeswalkers — Starting with you, each player votes for planeswalk or chaos. If planeswalk gets more votes, planeswalk. If chaos gets more votes or the vote is tied, chaos ensues"
            .to_string(),
    )
}

pub(in crate::compiled_text) fn describe_named_vote_conditional_sequence(
    effects: &[&Effect],
) -> Option<String> {
    let [vote_effect, followups @ ..] = effects else {
        return None;
    };
    if followups.is_empty() {
        return None;
    }

    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let ironsmith_core::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || !vote.starting_with_controller
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || !options
            .iter()
            .all(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let option_names = options
        .iter()
        .map(|option| option.name.to_string())
        .collect::<Vec<_>>();
    let mut clauses = Vec::new();
    for effect in followups {
        let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
        if !conditional.if_false.is_empty() {
            return None;
        }
        let condition_option = match &conditional.condition {
            Condition::VoteOptionGetsMoreVotes(option)
            | Condition::VoteOptionGetsMoreVotesOrTied(option) => option,
            _ => return None,
        };
        if !option_names
            .iter()
            .any(|option| option.eq_ignore_ascii_case(condition_option))
        {
            return None;
        }
        let body = council_conditional_body(&conditional.if_true)
            .or_else(|| describe_effect_clause_list(&conditional.if_true))
            .unwrap_or_else(|| describe_effect_list(&conditional.if_true))
            .trim()
            .trim_end_matches('.')
            .to_string();
        if body.is_empty() {
            return None;
        }
        clauses.push(format!(
            "If {}, {}",
            describe_condition(&conditional.condition),
            lowercase_first(&body)
        ));
    }

    let mut text = format!(
        "Will of the council — Starting with you, each player votes for {}",
        join_with_or(&option_names)
    );
    text.push_str(". ");
    text.push_str(&clauses.join(". "));
    Some(text)
}

/// A named vote can conditionally move the cards linked to the source that
/// exiled them. Preserve the vote/result sentence boundary without inventing
/// an ability word: not every rules object using this structure is a
/// will-of-the-council ability.
pub(super) fn describe_source_exiled_named_vote_conditional_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [vote_effect, conditional_effect] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let ironsmith_core::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || !vote.starting_with_controller
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || !options
            .iter()
            .all(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let condition_option = match &conditional.condition {
        Condition::VoteOptionGetsMoreVotes(option)
        | Condition::VoteOptionGetsMoreVotesOrTied(option) => option,
        _ => return None,
    };
    let option_names = options
        .iter()
        .map(|option| option.name.to_string())
        .collect::<Vec<_>>();
    if !option_names
        .iter()
        .any(|option| option.eq_ignore_ascii_case(condition_option))
    {
        return None;
    }

    let [move_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let targets_source_exiled_cards = match move_to_zone.target.base() {
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => {
            is_source_exiled_cards_filter(filter)
        }
        ChooseSpec::Tagged(tag) => tag.as_str() == crate::tag::SOURCE_EXILED_TAG,
        _ => false,
    };
    let surface = move_to_zone.exiled_with_source_surface.as_ref()?;
    if move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || !targets_source_exiled_cards
        || surface.subject != ironsmith_core::ExiledWithSourceSubjectSurface::OwnerOfEachCard
        || surface.destination != ironsmith_core::ExiledWithSourceDestinationSurface::TheirOwner
        || !matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Source(_)
        )
    {
        return None;
    }

    let action = describe_effect(move_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "Starting with you, each player votes for {}. If {}, {}",
        join_with_or(&option_names),
        describe_condition(&conditional.condition),
        lowercase_first(&action),
    ))
}

/// A declared spell target can precede a named vote whose result branches act
/// on that same spell. Preserve the declaration as its own sentence and use
/// the definite reference in every structurally linked vote branch.
pub(super) fn describe_targeted_named_vote_conditional_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, vote_effect, rest @ ..] = effects else {
        return None;
    };
    let (_, target_only) = tagged_target_only_effect(target_effect)?;
    if !matches!(target_only.target, ChooseSpec::Target(_)) {
        return None;
    }
    let conditional_count = rest
        .iter()
        .take_while(|effect| {
            effect
                .downcast_ref::<crate::effects::ConditionalEffect>()
                .is_some()
        })
        .count();
    if conditional_count == 0 {
        return None;
    }
    let conditionals = &rest[..conditional_count];
    let mut vote_refs = Vec::with_capacity(conditional_count + 1);
    vote_refs.push(vote_effect);
    vote_refs.extend(conditionals.iter());
    let vote_text =
        describe_named_vote_conditional_sequence(&vote_refs)?.replace("target spell", "the spell");
    let target_text = describe_effect(target_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let mut text = format!("{target_text}. {vote_text}");
    let trailing = &rest[conditional_count..];
    if !trailing.is_empty() {
        let trailing_text = describe_effect_list(trailing);
        if !trailing_text.trim().is_empty() {
            text.push_str(". ");
            text.push_str(&capitalize_first(
                trailing_text.trim().trim_end_matches('.'),
            ));
        }
    }
    Some(text)
}

pub(super) fn is_you_and_target_opponent_participants(
    choice: &crate::effects::SecretChoiceEffect,
) -> bool {
    matches!(
        choice.participants.as_slice(),
        [
            PlayerFilter::You,
            PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner)
        ] if **inner == PlayerFilter::Opponent
    )
}

pub(in crate::compiled_text) fn describe_secret_choice_match_sequence(
    effects: &[Effect],
) -> Option<String> {
    let (choice_effect, conditional, if_false) = match effects {
        [choice_effect, conditional_effect] => {
            let conditional = structural_unwrap_render_wrappers(conditional_effect)
                .downcast_ref::<crate::effects::ConditionalEffect>()?;
            (choice_effect, conditional, conditional.if_false.as_slice())
        }
        [choice_effect, success_effect, fallback_effect] => {
            // Authored "otherwise" branches can lower as a successful
            // conditional result carrying an ID followed by a DidNotHappen
            // fallback. Preserve the sentence only when that result link is
            // exact; an unrelated fallback must remain a separate effect.
            let success_with_id = success_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
            let conditional = structural_unwrap_render_wrappers(&success_with_id.effect)
                .downcast_ref::<crate::effects::ConditionalEffect>()?;
            let fallback = structural_unwrap_render_wrappers(fallback_effect)
                .downcast_ref::<crate::effects::IfEffect>()?;
            if !conditional.if_false.is_empty()
                || fallback.condition != success_with_id.id
                || fallback.predicate != EffectPredicate::DidNotHappen
                || fallback.then.is_empty()
                || !fallback.else_.is_empty()
            {
                return None;
            }
            (choice_effect, conditional, fallback.then.as_slice())
        }
        _ => return None,
    };
    // Result annotation can assign an ID to the secret-choice producer so
    // later predicates can refer to it. The ID wrapper is semantic metadata,
    // not a different choice shape, and must not hide the producer from this
    // structural sentence renderer.
    let choice = structural_unwrap_render_wrappers(choice_effect)
        .downcast_ref::<crate::effects::SecretChoiceEffect>()?;
    if !is_you_and_target_opponent_participants(choice) {
        return None;
    }
    if !matches!(conditional.condition, Condition::SecretChoicesMatch)
        || conditional.if_true.is_empty()
        || if_false.is_empty()
    {
        return None;
    }

    let option_names = choice.options.clone();
    let if_true = describe_sacrifice_then_put_source_exiled_into_hands(&conditional.if_true)
        .or_else(|| describe_effect_clause_list(&conditional.if_true))
        .unwrap_or_else(|| describe_effect_list(&conditional.if_true))
        .trim()
        .trim_end_matches('.')
        .to_string();
    let if_false = describe_effect_clause_list(if_false)
        .unwrap_or_else(|| describe_effect_list(if_false))
        .trim()
        .trim_end_matches('.')
        .to_string();
    if if_true.is_empty() || if_false.is_empty() {
        return None;
    }

    Some(format!(
        "You and target opponent each secretly choose {}. Then those choices are revealed. If they match, {}. Otherwise, {}",
        join_with_or(&option_names),
        lowercase_first(&if_true),
        lowercase_first(&if_false)
    ))
}

pub(super) fn describe_sacrifice_then_put_source_exiled_into_hands(
    effects: &[Effect],
) -> Option<String> {
    let effects = if let [sequence_effect] = effects
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        effects
    };
    let [sacrifice_effect, move_effect] = effects else {
        return None;
    };
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target.base(), ChooseSpec::Source) {
        return None;
    }
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::All(filter) = move_to_zone.target.base() else {
        return None;
    };
    if move_to_zone.zone != Zone::Hand
        || filter.zone != Some(Zone::Exile)
        || !filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG)
    {
        return None;
    }
    Some(
        "Sacrifice this artifact and put all cards exiled with it into their owners' hands"
            .to_string(),
    )
}

pub(super) fn library_position_from_top_text(position: &Value, one_as_on_top: bool) -> String {
    if let Value::Fixed(value) = position
        && let Ok(value) = u32::try_from(*value)
        && let Some(ordinal) = ordinal_word(value)
    {
        if value == 1 && one_as_on_top {
            "on top".to_string()
        } else {
            format!("{ordinal} from the top")
        }
    } else {
        format!("{} from the top", describe_value(position))
    }
}

/// Compile a list of effects to human-readable text (for stack ability display).
pub fn compile_effect_list(effects: &[Effect]) -> String {
    // Runtime prompts render nested effect programs on ordinary worker
    // threads, whose stacks are often much smaller than the main thread's.
    // Some legitimate delayed-effect programs are structurally deep enough
    // to exhaust that stack even though the renderer is not recursing
    // infinitely. Match the full-card rendering entry point's guarded stack.
    crate::perf::maybe_grow(4 * 1024 * 1024, 16 * 1024 * 1024, || {
        normalize_compile_effect_list_surface(&describe_effect_list(effects))
    })
}

pub(crate) fn describe_spell_mastery_reanimation_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let effects = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .collect::<Vec<_>>();
    describe_spell_mastery_reanimation_effects(&effects)
}

pub(crate) fn unwrap_basic_tag_wrappers(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_basic_tag_wrappers(&with_id.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_basic_tag_wrappers(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_basic_tag_wrappers(&tagged.effect);
    }
    effect
}

pub(super) fn direct_wrapped_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| &tagged.tag)
}

pub(super) fn wrapped_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return wrapped_effect_tag(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    None
}

pub(in crate::compiled_text) fn describe_power_damage_exchange_clause(
    effects: &[Effect],
) -> Option<String> {
    fn power_value_references(value: &Value, spec: &ChooseSpec) -> bool {
        matches!(
            value.unhinted(),
            Value::PowerOf(power_spec) if power_spec.unhinted() == spec.unhinted()
        )
    }

    fn demonstrative_reference_for_target(spec: &ChooseSpec) -> Option<&'static str> {
        let ChooseSpec::Target(inner) = spec.unhinted() else {
            return None;
        };
        let ChooseSpec::Object(filter) = inner.unhinted() else {
            return None;
        };
        if filter.card_types.contains(&CardType::Creature) {
            Some("that creature")
        } else if filter.card_types.contains(&CardType::Artifact) {
            Some("that artifact")
        } else if filter.card_types.contains(&CardType::Enchantment) {
            Some("that enchantment")
        } else if filter.card_types.contains(&CardType::Planeswalker) {
            Some("that planeswalker")
        } else if filter.card_types.contains(&CardType::Battle) {
            Some("that battle")
        } else if filter.card_types.contains(&CardType::Land) {
            Some("that land")
        } else {
            Some("that permanent")
        }
    }

    let [first_effect, tagged_target_effect, reciprocal_effect] = effects else {
        return None;
    };
    let (first_effect, first_result_tag) =
        if let Some(tagged) = first_effect.downcast_ref::<crate::effects::TaggedEffect>() {
            (tagged.effect.as_ref(), Some(&tagged.tag))
        } else {
            (first_effect, None)
        };
    let first_exec = first_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let first_damage = first_exec
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !power_value_references(&first_damage.amount, &first_exec.source) {
        return None;
    }

    let tagged = tagged_target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if first_damage.target.unhinted() != target_only.target.unhinted() {
        return None;
    }

    let reciprocal_exec =
        reciprocal_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(&reciprocal_exec.source, ChooseSpec::Tagged(tag) if tag == &tagged.tag) {
        return None;
    }
    let reciprocal_damage = reciprocal_exec
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let Value::PowerOf(power_spec) = reciprocal_damage.amount.unhinted() else {
        return None;
    };
    let ChooseSpec::Tagged(power_tag) = power_spec.unhinted() else {
        return None;
    };
    // Lowering may tag either the explicit target-only node or the first
    // damage result. The latter is the same chosen object only because the
    // exact target equality above has already been established.
    if power_tag != &tagged.tag
        && first_result_tag.is_none_or(|first_result_tag| power_tag != first_result_tag)
    {
        return None;
    }
    if reciprocal_damage.target.unhinted() != first_exec.source.unhinted() {
        return None;
    }

    let source_text = describe_choose_spec(&first_exec.source);
    let target_text = describe_choose_spec(&first_damage.target);
    let reciprocal_source = demonstrative_reference_for_target(&target_only.target)?;
    let reciprocal_target = describe_choose_spec(&reciprocal_damage.target);
    Some(format!(
        "{source_text} deals damage equal to its power to {target_text}, then {reciprocal_source} deals damage equal to its power to {reciprocal_target}"
    ))
}

#[cfg(test)]
mod power_damage_exchange_tests {
    use super::*;

    fn reciprocal_damage_effects(reciprocal_power_tag: TagKey) -> Vec<Effect> {
        let first_result_tag = TagKey::from("damaged_0");
        let target_tag = TagKey::from("damage_source_1");
        let source = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            ),
        );
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
        vec![
            Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                source.clone(),
                Effect::deal_damage(Value::PowerOf(Box::new(source.clone())), target.clone()),
            ))
            .tag(first_result_tag),
            Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(target_tag.clone()),
            Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(target_tag),
                Effect::deal_damage(
                    Value::PowerOf(Box::new(ChooseSpec::Tagged(reciprocal_power_tag))),
                    source,
                ),
            )),
        ]
    }

    #[test]
    fn reciprocal_power_may_reference_first_damage_result_for_the_same_target() {
        let effects = reciprocal_damage_effects(TagKey::from("damaged_0"));

        assert_eq!(
            describe_power_damage_exchange_clause(&effects).as_deref(),
            Some(
                "this creature deals damage equal to its power to target creature, then that creature deals damage equal to its power to this creature"
            )
        );
    }

    #[test]
    fn reciprocal_power_rejects_an_unrelated_result_tag() {
        let effects = reciprocal_damage_effects(TagKey::from("unrelated_0"));

        assert_eq!(describe_power_damage_exchange_clause(&effects), None);
    }
}

pub(super) fn describe_copy_tagged_then_may_cast_copy(effects: &[Effect]) -> Option<String> {
    let [copy_effect, may_effect] = effects else {
        return None;
    };

    let copy_spell =
        unwrap_basic_tag_wrappers(copy_effect).downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy_spell.count != Value::Fixed(1)
        || !copy_spell.removed_supertypes.is_empty()
        || copy_spell.has_characteristic_modifiers()
    {
        return None;
    }
    let ChooseSpec::Tagged(copy_tag) = &copy_spell.target else {
        return None;
    };

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast_tagged = unwrap_basic_tag_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if !cast_tagged.as_copy || &cast_tagged.tag != copy_tag {
        return None;
    }

    let verb = if cast_tagged.allow_land {
        "play"
    } else {
        "cast"
    };
    let mut text = format!("Copy it. You may {verb} the copy");
    if cast_tagged.without_paying_mana_cost {
        text.push_str(" without paying its mana cost");
    }
    if let Some(reduction) = cast_tagged.cost_reduction.as_ref() {
        text.push_str(&format!(
            ". That copy costs {} less to cast",
            reduction.to_oracle()
        ));
    }
    Some(text)
}

pub(super) fn may_draws_one_for_you(effect: &Effect) -> Option<()> {
    let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, Some(PlayerFilter::You)) || may.effects.len() != 1 {
        return None;
    }
    let draw = may.effects[0].downcast_ref::<crate::effects::DrawCardsEffect>()?;
    (draw.player == PlayerFilter::You && draw.count.unhinted() == &Value::Fixed(1)).then_some(())
}

pub(super) fn describe_may_draw_then_source_enchanted_additional_draw(
    effects: &[Effect],
) -> Option<String> {
    let [first, second] = effects else {
        return None;
    };
    may_draws_one_for_you(first)?;

    let conditional = second.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.condition != crate::effect::Condition::SourceIsEnchanted
        || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
    {
        return None;
    }
    may_draws_one_for_you(&conditional.if_true[0])?;

    Some(format!(
        "You may draw a card. You may draw an additional card if {}",
        lowercase_first(&describe_condition(&conditional.condition))
    ))
}

pub(super) fn describe_spell_mastery_reanimation_effects(effects: &[&Effect]) -> Option<String> {
    let (move_effect, conditional_effect) = match effects {
        [move_effect] => (*move_effect, None),
        [move_effect, conditional_effect] => (*move_effect, Some(*conditional_effect)),
        _ => return None,
    };

    let move_tag = direct_wrapped_effect_tag(move_effect)?;
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
        || move_to_zone.enters_tapped
    {
        return None;
    }
    let target_filter = match move_to_zone.target.base() {
        ChooseSpec::Object(filter) => filter,
        ChooseSpec::WithCount(inner, count) if count.is_single() => {
            let ChooseSpec::Object(filter) = inner.base() else {
                return None;
            };
            filter
        }
        _ => return None,
    };
    if target_filter.zone != Some(Zone::Graveyard)
        || !target_filter.card_types.contains(&CardType::Creature)
    {
        return None;
    }

    let (condition, counter_type, counter_amount, fused_entry_counter) =
        if let Some(conditional_effect) = conditional_effect {
            let conditional =
                conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
            if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
                return None;
            }
            let put_counters = unwrap_basic_tag_wrappers(&conditional.if_true[0])
                .downcast_ref::<crate::effects::PutCountersEffect>()?;
            if put_counters.distributed
                || put_counters.target_count.is_some()
                || !matches!(&put_counters.target, ChooseSpec::Tagged(tag) if tag == move_tag)
            {
                return None;
            }
            (
                &conditional.condition,
                put_counters.counter_type,
                &put_counters.amount,
                false,
            )
        } else {
            let [entry_counter] = move_to_zone.enters_with_counters.as_slice() else {
                return None;
            };
            if entry_counter.surface
                != ironsmith_core::BattlefieldEntryCounterSurface::ThatObjectEntersIfCondition
            {
                return None;
            }
            (
                entry_counter.condition.as_ref()?,
                entry_counter.counter_type,
                &entry_counter.amount,
                true,
            )
        };

    let Condition::ValueComparison {
        left: Value::Count(condition_filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(2),
    } = condition
    else {
        return None;
    };
    if condition_filter.zone != Some(Zone::Graveyard)
        || condition_filter.owner != Some(PlayerFilter::You)
        || condition_filter.card_types != vec![CardType::Instant, CardType::Sorcery]
    {
        return None;
    }

    let move_text = describe_effect(move_effect).replace(" in a graveyard", " from a graveyard");
    if fused_entry_counter {
        let (base, condition_tail) = move_text.split_once(". If ")?;
        return Some(format!("{base}. Spell mastery — If {condition_tail}"));
    }

    let counter_type = describe_counter_type(counter_type);
    let counter_suffix = match counter_amount.unhinted() {
        Value::Fixed(1) => format!("an additional {counter_type} counter"),
        Value::Fixed(amount) => {
            let count_text = number_word(*amount).unwrap_or_else(|| amount.to_string());
            format!("{count_text} additional {counter_type} counters")
        }
        _ => return None,
    };
    Some(format!(
        "{move_text}. Spell mastery — If there are two or more instant and/or sorcery cards in \
         your graveyard, that creature enters with {counter_suffix} on it"
    ))
}

pub(super) fn normalize_compile_effect_list_surface(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if lower
        == "each opponent chooses a creature card, then put it onto the battlefield under your control"
    {
        return "Each opponent chooses a creature card in their graveyard. Put those cards onto the battlefield under your control".to_string();
    }
    if lower
        == "destroy all nonbasic lands. for each land destroyed this way, its controller may search its controller's library for a basic land card. for each tagged 'searched' object, put them onto the battlefield. if you do, shuffle that player's library"
    {
        return "Destroy all nonbasic lands. For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles".to_string();
    }
    line.to_string()
}

pub(super) fn describe_gain_life_then_distribute_creatures_died_counters(
    effects: &[Effect],
) -> Option<String> {
    let [gain_effect, put_effect] = effects else {
        return None;
    };
    let gain =
        unwrap_basic_tag_wrappers(gain_effect).downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !matches!(gain.amount, Value::CreaturesDiedThisTurn)
        || !matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
    {
        return None;
    }

    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !put.distributed
        || !matches!(put.amount, Value::CreaturesDiedThisTurn)
        || put.counter_type != crate::object::CounterType::PlusOnePlusOne
    {
        return None;
    }
    let ChooseSpec::WithCount(inner, count) = &put.target else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    if count.min != 0
        || count.max.is_some()
        || put.target_count != Some(*count)
        || filter.zone != Some(Zone::Battlefield)
        || filter.controller != Some(PlayerFilter::You)
        || !filter.card_types.contains(&CardType::Creature)
    {
        return None;
    }

    Some(format!(
        "You gain that much life and distribute that many {} counters among {}",
        describe_counter_type(put.counter_type),
        describe_choose_spec(&put.target)
    ))
}

pub(super) fn describe_gain_life_then_put_same_x_counters(effects: &[Effect]) -> Option<String> {
    let [gain_effect, put_effect] = effects else {
        return None;
    };
    let gain =
        unwrap_basic_tag_wrappers(gain_effect).downcast_ref::<crate::effects::GainLifeEffect>()?;
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You)
        || put.distributed
        || put.counter_type != crate::object::CounterType::PlusOnePlusOne
        || gain.amount.unhinted() != put.amount.unhinted()
    {
        return None;
    }
    let where_x = describe_where_x_basis(&gain.amount)?;
    Some(format!(
        "You gain X life and put X {} counters on {}, where X is {where_x}",
        describe_counter_type(put.counter_type),
        describe_choose_spec(&put.target)
    ))
}

pub(super) fn is_effect_count_reference(
    value: &Value,
    effect_id: Option<crate::effect::EffectId>,
) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } => is_effect_count_reference(value, effect_id),
        Value::EffectValue(id) => effect_id.is_none_or(|expected| *id == expected),
        Value::EventValue(EventValueSpec::Amount) => true,
        Value::EffectMetric {
            effect_id: id,
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            ..
        } => effect_id.is_none_or(|expected| *id == expected),
        Value::PendingEffectMetric {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            ..
        } => effect_id.is_none(),
        Value::PriorEffectMetric {
            effect_id: id,
            query,
        } if matches!(
            query.metric,
            crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount
        ) =>
        {
            effect_id.is_none_or(|expected| *id == expected)
        }
        Value::PendingPriorEffectMetric(query)
            if matches!(
                query.metric,
                crate::effect::EffectMetric::Count
                    | crate::effect::EffectMetric::ChosenCount
                    | crate::effect::EffectMetric::AffectedCount
            ) =>
        {
            effect_id.is_none()
        }
        _ => false,
    }
}

pub(super) fn effect_count_reference_offset(
    value: &Value,
    effect_id: Option<crate::effect::EffectId>,
) -> Option<i32> {
    match value {
        Value::SurfaceHinted { value, .. } => effect_count_reference_offset(value, effect_id),
        Value::EffectValueOffset(id, offset)
            if effect_id.is_none_or(|expected| *id == expected) =>
        {
            Some(*offset)
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset) => Some(*offset),
        Value::EffectMetricOffset {
            effect_id: id,
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            offset,
            ..
        } if effect_id.is_none_or(|expected| *id == expected) => Some(*offset),
        Value::PendingEffectMetricOffset {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            offset,
            ..
        } if effect_id.is_none() => Some(*offset),
        _ => None,
    }
}

pub(super) fn describe_each_object_subject(target: &ChooseSpec) -> Option<String> {
    match target {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            Some(format!("Each {}", filter.description()))
        }
        _ => None,
    }
}

pub(super) fn possessive_subject(subject: &str) -> String {
    if subject.ends_with('s') {
        format!("{subject}'")
    } else {
        format!("{subject}'s")
    }
}

pub(super) fn possessive_object_subject(subject: &str) -> String {
    match subject {
        "it" => "its".to_string(),
        "them" | "they" => "their".to_string(),
        _ => format!("{subject}'s"),
    }
}

pub(super) fn dynamic_pt_scale_multiplier_for_target(
    value: &Value,
    target: &ChooseSpec,
    power_axis: bool,
) -> Option<i32> {
    match value.unhinted() {
        Value::SourcePower if power_axis && choose_spec_references_source_object(target) => Some(1),
        Value::SourceToughness if !power_axis && choose_spec_references_source_object(target) => {
            Some(1)
        }
        Value::PowerOf(spec) if power_axis => {
            dynamic_pt_choose_specs_equivalent(spec, target).then_some(1)
        }
        Value::ToughnessOf(spec) if !power_axis => {
            dynamic_pt_choose_specs_equivalent(spec, target).then_some(1)
        }
        Value::Scaled(inner, multiplier) if *multiplier > 0 => {
            dynamic_pt_scale_multiplier_for_target(inner, target, power_axis)
                .map(|base| base * *multiplier)
        }
        _ => None,
    }
}

pub(super) fn choose_spec_references_source_object(spec: &ChooseSpec) -> bool {
    match spec.unhinted() {
        ChooseSpec::Source => true,
        ChooseSpec::Object(filter) => filter.source,
        _ => false,
    }
}

pub(super) fn dynamic_pt_choose_specs_equivalent(left: &ChooseSpec, right: &ChooseSpec) -> bool {
    choose_specs_equivalent_ignoring_source_surface(left, right)
        || (choose_spec_references_source_object(left)
            && choose_spec_references_source_object(right))
}

pub(super) fn describe_dynamic_pt_scale_action(
    target: &ChooseSpec,
    power: &Value,
    toughness: &Value,
    duration: &Until,
) -> Option<String> {
    if power.has_surface_hint(ValueSurfaceHint::WhereXIs)
        || toughness.has_surface_hint(ValueSurfaceHint::WhereXIs)
    {
        return None;
    }
    let power_multiplier = dynamic_pt_scale_multiplier_for_target(power, target, true);
    let toughness_multiplier = dynamic_pt_scale_multiplier_for_target(toughness, target, false);
    let (multiplier, stat) = match (power_multiplier, toughness_multiplier) {
        (Some(multiplier), None) if matches!(toughness.unhinted(), Value::Fixed(0)) => {
            (multiplier, "power")
        }
        (None, Some(multiplier)) if matches!(power.unhinted(), Value::Fixed(0)) => {
            (multiplier, "toughness")
        }
        (Some(power_multiplier), Some(toughness_multiplier))
            if power_multiplier == toughness_multiplier =>
        {
            (power_multiplier, "power and toughness")
        }
        _ => return None,
    };
    let verb = match multiplier + 1 {
        2 => "Double",
        3 => "Triple",
        _ => return None,
    };
    let target_text = describe_choose_spec(target);
    Some(format!(
        "{verb} {} {stat} {}",
        possessive_object_subject(&target_text),
        describe_until(duration)
    ))
}

pub(super) fn may_causative_clause(inner: &str) -> Option<String> {
    let trimmed = inner.trim();
    let lower = trimmed.to_ascii_lowercase();
    if ![
        "a ", "an ", "all ", "another ", "each ", "it ", "other ", "that ", "the ", "this ",
        "those ", "target ", "two ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
    {
        return None;
    }

    let base_pt_marker = " has base power and toughness ";
    if let Some(idx) = lower.find(base_pt_marker) {
        let subject = lowercase_first(trimmed[..idx].trim());
        let rest = trimmed[idx + base_pt_marker.len()..].trim();
        if !subject.is_empty() && !rest.is_empty() {
            return Some(format!(
                "have {} base power and toughness become {rest}",
                possessive_subject(&subject)
            ));
        }
    }

    let replacements = [
        (" becomes ", "become"),
        (" gets ", "get"),
        (" gains ", "gain"),
        (" has ", "have"),
        (" loses ", "lose"),
        (" reveals ", "reveal"),
        (" returns ", "return"),
        (" fights ", "fight"),
        (" deals ", "deal"),
        (" exchange ", "exchange"),
    ];
    replacements.iter().find_map(|(from, to)| {
        lower.find(from).and_then(|idx| {
            let subject = lowercase_first(trimmed[..idx].trim());
            let rest = trimmed[idx + from.len()..].trim();
            if subject.is_empty() || rest.is_empty() {
                None
            } else {
                Some(format!("have {subject} {to} {rest}"))
            }
        })
    })
}

fn may_causative_prefix(decider: &PlayerFilter) -> String {
    let who = describe_player_filter(decider);
    if who == "you" {
        "You may have".to_string()
    } else {
        format!("{who} may have")
    }
}

fn describe_may_causative_discard(may: &crate::effects::MayEffect) -> Option<String> {
    let decider = may.decider.as_ref()?;
    let discard = may
        .effects
        .last()?
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if &discard.player == decider || discard.any_number {
        return None;
    }
    let actor = describe_player_filter(&discard.player);
    let count = describe_discard_count(&discard.count, discard.card_filter.as_ref());
    let random = if discard.random { " at random" } else { "" };
    Some(format!(
        "{} {actor} discard {count}{random}",
        may_causative_prefix(decider)
    ))
}

fn describe_may_causative_sacrifice(may: &crate::effects::MayEffect) -> Option<String> {
    let decider = may.decider.as_ref()?;
    let [.., choose_effect, sacrifice_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    if &choose.chooser == decider
        || sacrifice.player != &choose.chooser
        || !matches!(sacrifice.count.unhinted(), Value::Fixed(1))
        || !exact_count(&choose.count, 1)
    {
        return None;
    }
    let actor = describe_player_filter(&choose.chooser);
    let mut filter = choose.filter.clone();
    if filter.controller.as_ref() == Some(&choose.chooser) {
        filter.controller = None;
    }
    let object = with_indefinite_article(strip_leading_article(&filter.description()));
    Some(format!(
        "{} {actor} sacrifice {object} of their choice",
        may_causative_prefix(decider)
    ))
}

fn describe_may_causative_pt_change(may: &crate::effects::MayEffect) -> Option<String> {
    let decider = may.decider.as_ref()?;
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    let apply = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.condition.is_some()
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = apply.runtime_modifications.as_slice()
    else {
        return None;
    };
    let spec = apply.target_spec.as_ref()?;
    let mut target = describe_choose_spec(spec);
    if decider != &PlayerFilter::You && spec.is_target() {
        target.push_str(" of their choice");
    }
    Some(format!(
        "{} {target} get {}/{} {}",
        may_causative_prefix(decider),
        describe_signed_value(power),
        describe_toughness_delta_with_power_context(power, toughness),
        describe_until(&apply.until)
    ))
}

fn describe_may_causative_fight(may: &crate::effects::MayEffect) -> Option<String> {
    let decider = may.decider.as_ref()?;
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    let fight = effect.downcast_ref::<crate::effects::FightEffect>()?;
    if !matches!(fight.creature1.base(), ChooseSpec::Source) {
        return None;
    }
    Some(format!(
        "{} {} fight {}",
        may_causative_prefix(decider),
        describe_choose_spec(&fight.creature1),
        describe_choose_spec(&fight.creature2)
    ))
}

fn describe_may_causative_source_damage(may: &crate::effects::MayEffect) -> Option<String> {
    let decider = may.decider.as_ref()?;
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    let damage =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat || damage.unpreventable {
        return None;
    }
    let target = if choose_spec_player_filter(&damage.target).as_ref() == Some(decider) {
        "them".to_string()
    } else {
        describe_damage_target(&damage.target)
    };
    // This formatter renders the amount itself, so it needs the same
    // "damage … equal to" rule the main damage clause applies — otherwise a
    // characteristic or count basis becomes an inline determiner ("deal the
    // exiled card's mana value damage to it").
    let amount_text = describe_value(&damage.amount);
    if value_prefers_equal_to(&damage.amount)
        || power_damage_prefers_equal_to(&damage.amount)
        || (!value_prefers_where_x(&damage.amount) && count_damage_prefers_equal_to(&damage.amount))
    {
        return Some(format!(
            "{} this source deal damage to {target} equal to {amount_text}",
            may_causative_prefix(decider)
        ));
    }
    Some(format!(
        "{} this source deal {amount_text} damage to {target}",
        may_causative_prefix(decider)
    ))
}

fn describe_may_causative_grant_all(may: &crate::effects::MayEffect) -> Option<String> {
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    let effect = unwrap_basic_tag_wrappers(effect);
    let decider = may.decider.as_ref().unwrap_or(&PlayerFilter::You);

    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        let crate::continuous::EffectTarget::Filter(_) = &apply.target else {
            return None;
        };
        if apply.target_spec.is_some()
            || !apply.runtime_modifications.is_empty()
            || apply.condition.is_some()
        {
            return None;
        }
        let modifications = apply
            .modification
            .iter()
            .chain(apply.additional_modifications.iter())
            .collect::<Vec<_>>();
        if modifications.is_empty()
            || modifications.iter().any(|modification| {
                !matches!(
                    modification,
                    crate::continuous::Modification::AddAbility(_)
                        | crate::continuous::Modification::AddAbilityGeneric(_)
                )
            })
        {
            return None;
        }
        let mut grant = describe_apply_continuous_effect(apply)?;
        if let crate::continuous::EffectTarget::Filter(filter) = &apply.target
            && filter.card_types.as_slice() == [CardType::Creature]
            && filter.explicit_card_type_noun() != Some(CardType::Creature)
            && let [subtype] = filter.subtypes.as_slice()
        {
            // In a causative clause the creature type is itself the plural
            // subject ("have Allies you control gain ..."), rather than an
            // attributive modifier on "creatures" — unless the authored text
            // spelled the noun out ("have Ally creatures you control gain ...").
            let attributive = format!("{subtype} creatures");
            if let Some(rest) = grant.strip_prefix(&attributive) {
                grant = format!("{}{}", pluralize_word(&subtype.to_string()), rest);
            }
        }
        return Some(format!("{} {grant}", may_causative_prefix(decider)));
    }

    let grant = effect.downcast_ref::<crate::effects::GrantAbilitiesAllEffect>()?;
    if grant.abilities.is_empty() {
        return None;
    }
    let self_subject = granted_ability_self_subject_for_filter(&grant.filter);
    let abilities = grant
        .abilities
        .iter()
        .map(|ability| {
            ability
                .granted_inline_ability()
                .map(|inline| describe_granted_ability_phrase(inline, self_subject))
                .unwrap_or_else(|| {
                    strip_redundant_granted_subject(
                        lowercase_first(&ability.display()),
                        self_subject,
                    )
                })
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "{} {} gain {abilities} {}",
        may_causative_prefix(decider),
        grant.filter.description(),
        describe_until(&grant.duration)
    ))
}

fn describe_may_causative_become_color_choice(may: &crate::effects::MayEffect) -> Option<String> {
    let decider = may.decider.as_ref().unwrap_or(&PlayerFilter::You);
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::BecomeColorChoiceEffect>()?;
    let rendered = describe_effect(effect);
    let (subject, predicate) = rendered.split_once(" becomes ")?;
    Some(format!(
        "{} {} become {predicate}",
        may_causative_prefix(decider),
        lowercase_first(subject)
    ))
}

fn describe_may_causative_continuous_change(may: &crate::effects::MayEffect) -> Option<String> {
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    let apply = unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let rendered = describe_apply_continuous_effect(apply)?;
    let causative = may_causative_clause(&rendered)?;
    let decider = may.decider.as_ref().unwrap_or(&PlayerFilter::You);
    Some(format!(
        "{} may {causative}",
        capitalize_first(&describe_player_filter(decider))
    ))
}

/// Render optional causatives from the typed chooser/actor relationship. A
/// `MayEffect` answers who decides; its child effect still identifies who or
/// what performs the action.
pub(super) fn describe_typed_may_causative(may: &crate::effects::MayEffect) -> Option<String> {
    describe_may_causative_discard(may)
        .or_else(|| describe_may_causative_sacrifice(may))
        .or_else(|| describe_may_causative_pt_change(may))
        .or_else(|| describe_may_causative_fight(may))
        .or_else(|| describe_may_causative_source_damage(may))
        .or_else(|| describe_may_causative_grant_all(may))
        .or_else(|| describe_may_causative_become_color_choice(may))
        .or_else(|| describe_may_causative_continuous_change(may))
}

#[cfg(test)]
mod may_grant_all_causative_tests {
    use super::*;

    #[test]
    fn group_wide_ability_grant_uses_typed_have_causative() {
        let grant = Effect::new(crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Filter(
                ObjectFilter::creature()
                    .with_subtype(Subtype::Ally)
                    .you_control(),
            ),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::lifelink(),
            ),
            Until::EndOfTurn,
        ));
        let may = crate::effects::MayEffect::new(vec![grant]);

        assert_eq!(
            describe_typed_may_causative(&may).as_deref(),
            Some("You may have Allies you control gain lifelink until end of turn")
        );
    }

    #[test]
    fn ordinary_optional_non_grant_is_not_claimed_as_a_group_causative() {
        let may = crate::effects::MayEffect::new(vec![Effect::draw(Value::Fixed(1))]);

        assert_eq!(describe_may_causative_grant_all(&may), None);
        assert_eq!(describe_typed_may_causative(&may), None);

        let type_change = Effect::new(crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Filter(ObjectFilter::creature()),
            crate::continuous::Modification::AddSubtypes(vec![Subtype::Ally]),
            Until::EndOfTurn,
        ));
        assert_eq!(
            describe_may_causative_grant_all(&crate::effects::MayEffect::new(vec![type_change])),
            None
        );
    }

    #[test]
    fn optional_color_choice_uses_typed_have_causative() {
        let target = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            ),
        );
        let change = Effect::new(
            crate::effects::BecomeColorChoiceEffect::new(target, Until::Forever)
                .with_multiple_colors(true),
        );
        let may = crate::effects::MayEffect::new(vec![change]);

        assert_eq!(
            describe_typed_may_causative(&may).as_deref(),
            Some("You may have this creature become the color or colors of your choice")
        );
    }
}

#[cfg(test)]
mod may_continuous_change_causative_tests {
    use super::*;

    fn source_animation() -> crate::effects::ApplyContinuousEffect {
        let spec = ChooseSpec::SurfaceHinted {
            spec: Box::new(ChooseSpec::Source),
            hints: vec![crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this Equipment".to_string(),
                ),
            )],
        };
        let mut animation = crate::effects::ApplyContinuousEffect::with_spec(
            spec,
            crate::continuous::Modification::AddCardTypes(vec![
                CardType::Creature,
                CardType::Artifact,
            ]),
            Until::Forever,
        )
        .with_animation_pt_surface(Some(
            ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
        ));
        animation.additional_modifications.extend([
            crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(2),
                toughness: Value::Fixed(1),
                sublayer: crate::continuous::PtSublayer::Setting,
            },
            crate::continuous::Modification::AddSubtypes(vec![Subtype::Construct]),
        ]);
        animation
    }

    #[test]
    fn optional_source_animation_uses_have_causative() {
        let may = crate::effects::MayEffect::new(vec![Effect::new(source_animation())]);

        assert_eq!(
            describe_typed_may_causative(&may).as_deref(),
            Some("You may have this Equipment become a 2/1 Construct artifact creature")
        );
    }

    #[test]
    fn ordinary_nonoptional_animation_does_not_inherit_may_causative() {
        assert_eq!(
            describe_effect(&Effect::new(source_animation())),
            "This Equipment becomes a 2/1 Construct artifact creature"
        );
    }
}

pub(super) fn describe_typed_unless_source_damage(
    unless_action: &crate::effects::UnlessActionEffect,
) -> Option<String> {
    let [alternative] = unless_action.alternative.as_slice() else {
        return None;
    };
    let damage = unwrap_basic_tag_wrappers(alternative)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let damage_player = choose_spec_player_filter(&damage.target)?;
    if damage.source_is_combat
        || damage.unpreventable
        || !unless_damage_player_is_decider_alias(
            &damage_player,
            &unless_action.player,
            &unless_action.effects,
        )
    {
        return None;
    }
    let inner = describe_effect_list(&unless_action.effects);
    let player = describe_player_filter(&unless_action.player);
    if damage
        .amount
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
    {
        let amount = damage
            .amount
            .clone()
            .without_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo);
        return Some(format!(
            "{inner} unless {player} has this source deal damage to them equal to {}",
            describe_value(&amount)
        ));
    }
    Some(format!(
        "{inner} unless {player} has this source deal {} damage to them",
        describe_value(&damage.amount)
    ))
}

fn unless_damage_player_is_decider_alias(
    damage_player: &PlayerFilter,
    decider: &PlayerFilter,
    primary_effects: &[Effect],
) -> bool {
    if player_filters_refer_to_same_player(damage_player, decider) {
        return true;
    }
    let [primary] = primary_effects else {
        return false;
    };
    let Some(tag) = wrapped_effect_tag(primary) else {
        return false;
    };
    if !primary
        .0
        .get_target_spec()
        .is_some_and(ChooseSpec::is_target)
    {
        return false;
    }

    use crate::filter::ObjectRef;
    matches!(
        (damage_player, decider),
        (
            PlayerFilter::ControllerOf(ObjectRef::Target),
            PlayerFilter::ControllerOf(ObjectRef::Tagged(decider_tag)),
        ) | (
            PlayerFilter::OwnerOf(ObjectRef::Target),
            PlayerFilter::OwnerOf(ObjectRef::Tagged(decider_tag)),
        ) | (
            PlayerFilter::AliasedControllerOf(ObjectRef::Target),
            PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(decider_tag)),
        ) | (
            PlayerFilter::AliasedOwnerOf(ObjectRef::Target),
            PlayerFilter::AliasedOwnerOf(ObjectRef::Tagged(decider_tag)),
        ) if decider_tag == tag
    )
}

pub(super) fn prevention_put_counters_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::PutCountersEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() {
        return None;
    }
    if !is_effect_count_reference(&put.amount, None) {
        return None;
    }
    if !matches!(put.target, ChooseSpec::AnyTarget) {
        return None;
    }
    Some(put)
}

pub(super) fn prevention_gain_life_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::GainLifeEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let gain = effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !matches!(
        gain.player,
        ChooseSpec::Player(crate::target::PlayerFilter::You)
    ) {
        return None;
    }
    if !matches!(
        gain.amount.unhinted(),
        Value::EventValue(EventValueSpec::Amount)
    ) {
        return None;
    }
    Some(gain)
}

pub(super) fn prevention_exile_prevented_top_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::ExileTopOfLibraryEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let exile = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if !matches!(exile.player, PlayerFilter::You) {
        return None;
    }
    if !matches!(
        exile.count.unhinted(),
        Value::EventValue(EventValueSpec::Amount)
    ) {
        return None;
    }
    Some(exile)
}

pub(super) fn prevention_damage_any_target_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::DealDamageEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let damage =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        damage.amount.unhinted(),
        Value::EventValue(EventValueSpec::Amount)
    ) {
        return None;
    }
    if !matches!(damage.target, ChooseSpec::AnyTarget) {
        return None;
    }
    Some(damage)
}

pub(super) fn describe_implicit_source_combat_damage_prevention(
    source: &ChooseSpec,
    until: &Until,
) -> Option<String> {
    if !matches!(until, Until::EndOfTurn) {
        return None;
    }
    match source {
        ChooseSpec::Tagged(tag) if is_implicit_reference_tag(tag.as_str()) => {
            Some("Prevent all combat damage that would be dealt by it this turn".to_string())
        }
        _ => None,
    }
}

pub(super) fn put_counters_effect_for_source(
    effect: &Effect,
) -> Option<&crate::effects::PutCountersEffect> {
    if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        return Some(put_counters);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return put_counters_effect_for_source(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return put_counters_effect_for_source(&with_id.effect);
    }
    None
}

pub(super) fn describe_source_exile_with_counters_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let exile_effect = unwrap_basic_tag_wrappers(first);
    let exile_target =
        if let Some(exile) = exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            (exile.zone == Zone::Exile).then_some(&exile.target)?
        } else if let Some(exile) = exile_effect.downcast_ref::<crate::effects::ExileEffect>() {
            // Face-down exile has additional visible semantics and must retain its
            // ordinary renderer. The compact form is only for the exact typed
            // "exile ... with counters on it" pair.
            (!exile.face_down).then_some(&exile.spec)?
        } else {
            return None;
        };
    let put_counters = put_counters_effect_for_source(second)?;
    if put_counters.distributed || put_counters.target_count.is_some() {
        return None;
    }

    let counters_follow_exiled_object = matches!(exile_target.base(), ChooseSpec::Source)
        && matches!(put_counters.target.base(), ChooseSpec::Source)
        || matches!(
            (exile_target.base(), put_counters.target.base()),
            (ChooseSpec::Tagged(exile_tag), ChooseSpec::Tagged(counter_tag))
                if exile_tag == counter_tag
        )
        || wrapped_effect_tag(first)
            .is_some_and(|tag| choose_spec_references_tagged_object(&put_counters.target, tag))
        || matches!(
            put_counters.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        );
    if !counters_follow_exiled_object {
        return None;
    }

    let exile_text = describe_effect_impl(first);
    let subject = exile_text
        .strip_prefix("Exile ")
        .map(|text| text.trim_end_matches('.').to_string())
        .unwrap_or_else(|| describe_choose_spec(exile_target));
    Some(format!(
        "Exile {subject} with {} on it",
        describe_put_counter_phrase(&put_counters.amount, put_counters.counter_type),
    ))
}

pub(super) fn value_is_source_exiled_mana_value(value: &Value) -> bool {
    matches!(
        value.unhinted(),
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    )
}

pub(super) fn is_source_exiled_cards_filter(filter: &ObjectFilter) -> bool {
    if filter.zone != Some(Zone::Exile)
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        })
    {
        return false;
    }

    let mut base = filter.clone();
    base.zone = None;
    base.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    });
    base == ObjectFilter::default()
}

pub(super) fn describe_exiled_card_copy_target_filter(
    filter: &ObjectFilter,
) -> Option<&'static str> {
    if filter.zone != Some(Zone::Exile) {
        return None;
    }

    let mut base = filter.clone();
    let face_down = base.face_down;
    base.zone = None;
    base.face_down = None;
    base.attacking = false;
    base.attacking_player_or_planeswalker_controlled_by = None;

    if base != ObjectFilter::default() {
        return None;
    }

    Some(match face_down {
        Some(false) => "the face-up exiled card",
        Some(true) => "the face-down exiled card",
        None => "the exiled card",
    })
}

pub(super) fn describe_consult_exile_may_cast_rest_bottom_sequence(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Exile {
        return None;
    }

    let may = effects[1].downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != consult.match_tag || cast.allow_land || cast.as_copy {
        return None;
    }
    if let Some(decider) = may.decider.as_ref()
        && decider != &cast.player
    {
        return None;
    }

    let move_to_zone =
        unwrap_basic_tag_wrappers(effects[2]).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Object(filter)
                if describe_exiled_card_copy_target_filter(filter).is_some()
        )
    {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let subject_verb = player_verb(&player, "exile", "exiles");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text =
        describe_consult_stop_text(&selection, &consult.stop_rule, consult.max_exposed.as_ref());
    let caster = describe_player_filter(&cast.player);
    let free_cast_suffix = if cast.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };

    Some(format!(
        "{player} {subject_verb} cards from the top of {library_owner} library until {pronoun} exile {stop_text}. {caster} may cast that card{free_cast_suffix}. Then {player} {put_verb} the exiled cards that weren't cast this way on the bottom of {library_owner} library in a random order"
    ))
}

pub(super) fn describe_library_consult_selection_with_cards(filter: &ObjectFilter) -> String {
    let mut display_filter = filter.clone();
    display_filter.zone = None;
    if filter.zone == Some(Zone::Battlefield) && display_filter == ObjectFilter::default() {
        return "a permanent card".to_string();
    }
    if filter_explicitly_selects_permanent_cards(&display_filter) {
        return describe_single_search_filter_in_zone(&display_filter, Zone::Library);
    }
    let mut selection = display_filter.description();
    if display_filter.card_types.is_empty()
        && display_filter.all_card_types.is_empty()
        && display_filter.excluded_card_types == vec![CardType::Land]
    {
        if selection == "nonland permanent" {
            selection = "a nonland card".to_string();
        } else if let Some(rest) = selection.strip_prefix("nonland permanent with ") {
            selection = format!("a nonland card with {rest}");
        } else if let Some(rest) = selection.strip_prefix("nonland card with ") {
            selection = format!("a nonland card with {rest}");
        }
    }
    let selection = describe_search_selection_with_cards(&selection);
    if selection.ends_with(" card") {
        with_indefinite_article(&selection)
    } else {
        selection
    }
}

/// A cast trigger already supplies the comparison object for "lesser mana
/// value".  Preserve that concise Oracle surface only when the consult filter
/// explicitly compares against the typed triggering-object tag; other tagged
/// comparisons still need their rendered antecedent.
fn describe_triggering_spell_lesser_mana_consult_selection(
    filter: &ObjectFilter,
) -> Option<String> {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            constraint.tag.as_str() == "triggering"
                && constraint.relation == crate::filter::TaggedOpbjectRelation::ManaValueLtTagged
        })
        .then_some(())?;

    let selection = describe_library_consult_selection_with_cards(filter);
    let head = selection.strip_suffix(" with lesser mana value than it")?;
    Some(format!(
        "{} with lesser mana value",
        with_indefinite_article(strip_leading_article(head))
    ))
}

pub(super) fn describe_consult_may_cast_remainder_bottom_sequence(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let may = effects[1].downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != consult.match_tag || cast.allow_land || cast.as_copy {
        return None;
    }
    if let Some(decider) = may.decider.as_ref()
        && decider != &cast.player
    {
        return None;
    }

    let remainder =
        effects[2].downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.player != consult.player
        || remainder
            .keep_tagged
            .as_ref()
            .is_some_and(|tag| tag != &consult.match_tag)
    {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let concise_triggering_spell_selection =
        describe_triggering_spell_lesser_mana_consult_selection(&consult.filter);
    let (subject_verb, followup_verb, default_remainder_subject) = match consult.mode {
        crate::effects::consult_helpers::LibraryConsultMode::Reveal => (
            player_verb(&player, "reveal", "reveals"),
            "reveal",
            "all revealed cards not cast this way",
        ),
        crate::effects::consult_helpers::LibraryConsultMode::Exile => (
            player_verb(&player, "exile", "exiles"),
            "exile",
            "the exiled cards that weren't cast this way",
        ),
    };
    let pronoun = if player == "you" { "you" } else { "they" };
    let selection = concise_triggering_spell_selection
        .as_ref()
        .cloned()
        .unwrap_or_else(|| describe_library_consult_selection_with_cards(&consult.filter));
    let remainder_subject = if concise_triggering_spell_selection.is_some()
        && remainder.surface == ironsmith_core::LibraryRemainderSurface::Rest
    {
        "the rest"
    } else {
        default_remainder_subject
    };
    let stop_text =
        describe_consult_stop_text(&selection, &consult.stop_rule, consult.max_exposed.as_ref());
    let caster = describe_player_filter(&cast.player);
    let free_cast_suffix = if cast.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };
    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => format!(
            " in an order chosen by {}",
            describe_player_filter(&remainder.player)
        ),
    };

    Some(format!(
        "{player} {subject_verb} cards from the top of {library_owner} library until {pronoun} {followup_verb} {stop_text}. {caster} may cast that card{free_cast_suffix}. Put {remainder_subject} on the bottom of {library_owner} library{order_text}"
    ))
}

pub(super) fn source_and_source_exiled_return_text(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() != 2 {
        return None;
    }

    let mut source_text = None;
    let mut has_source_exiled = false;
    for branch in &filter.any_of {
        if is_source_exiled_cards_filter(branch) {
            has_source_exiled = true;
            continue;
        }

        if branch.source {
            let mut base = branch.clone();
            base.source = false;
            let is_saga = base.subtypes == [Subtype::Saga];
            if is_saga {
                base.subtypes.clear();
            }
            if base == ObjectFilter::default() {
                source_text = Some(if is_saga { "this Saga" } else { "this card" });
            }
        }
    }

    if has_source_exiled {
        source_text.map(|source_text| {
            format!("Return {source_text} and the exiled card to their owner's hand")
        })
    } else {
        None
    }
}

pub(super) fn has_vote_winners_tag(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == crate::effects::VOTE_WINNERS_TAG
    })
}

pub(super) fn describe_return_all_to_battlefield_effect(
    return_all: &crate::effects::ReturnAllToBattlefieldEffect,
) -> String {
    let helper_exile_tag = |tag: &str| {
        crate::cards::is_sentence_helper_tag(tag, "exiled")
            || tag
                .strip_prefix("__sentence_helper_exiled_aggregate_")
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                })
    };
    let source_linked_exile = return_all.filter.zone == Some(Zone::Exile)
        && return_all
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            });
    let helper_linked_exile = if return_all.filter.zone == Some(Zone::Exile) {
        let mut base = return_all.filter.clone();
        base.zone = None;
        base.tagged_constraints.retain(|constraint| {
            !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && helper_exile_tag(constraint.tag.as_str()))
        });
        base == ObjectFilter::default()
            && return_all
                .filter
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && helper_exile_tag(constraint.tag.as_str())
                })
    } else {
        false
    };
    let mut filter_text =
        if helper_linked_exile && return_all.filter.has_plural_pronoun_reference_surface() {
            "them".to_string()
        } else if helper_linked_exile {
            "the exiled cards".to_string()
        } else if source_linked_exile
            && return_all.filter.card_types.len() == 1
            && return_all.filter.card_types[0] == CardType::Creature
            && return_all.filter.subtypes.is_empty()
        {
            // The source-linked exile tag is shared by creatures, artifacts,
            // enchantments, and other permanents. The effect itself has no source
            // type context, so claiming a specific permanent type here can change
            // the meaning (for example, a creature source rendered as an
            // enchantment). Keep the reference type-neutral.
            "creature cards exiled with this card".to_string()
        } else {
            describe_for_each_filter(&return_all.filter)
        };
    filter_text = filter_text
        .replace("permanent card exiled", "permanent cards exiled")
        .replace("card exiled", "cards exiled")
        .replace("card milled", "cards milled")
        .replace(" card in your hand", " cards in your hand")
        .replace(" card in your graveyard", " cards from your graveyard")
        .replace(" card in a graveyard", " cards in graveyards");
    let controller_suffix = match return_all.battlefield_controller {
        crate::effects::BattlefieldController::Preserve
        | crate::effects::BattlefieldController::Owner => {
            if filter_text.contains(" from your graveyard") {
                ""
            } else {
                " under their owners' control"
            }
        }
        crate::effects::BattlefieldController::You if !return_all.controller_surface_explicit => "",
        crate::effects::BattlefieldController::You => " under your control",
    };
    let face_down_suffix = if return_all.face_down {
        " face down"
    } else {
        ""
    };
    let (verb, destination) = match return_all.verb_surface {
        ironsmith_core::MoveToZoneVerbSurface::Put => ("Put", "onto"),
        ironsmith_core::MoveToZoneVerbSurface::Canonical
        | ironsmith_core::MoveToZoneVerbSurface::Return => ("Return", "to"),
    };
    format!(
        "{verb}{}{filter_text} {destination} the battlefield{}{}{}",
        if helper_linked_exile { " " } else { " all " },
        if return_all.tapped { " tapped" } else { "" },
        face_down_suffix,
        controller_suffix,
    )
}

#[cfg(test)]
mod helper_linked_return_pronoun_tests {
    use super::*;

    #[test]
    fn exact_plural_surface_renders_them_and_plain_reference_keeps_the_noun() {
        let mut filter = ObjectFilter::tagged(TagKey::from("__sentence_helper_exiled_aggregate_7"))
            .in_zone(Zone::Exile);
        filter.set_plural_pronoun_reference_surface(true);
        let returned = crate::effects::ReturnAllToBattlefieldEffect::new(filter.clone(), false)
            .under_owner_control();

        assert_eq!(
            describe_return_all_to_battlefield_effect(&returned),
            "Return them to the battlefield under their owners' control"
        );

        filter.set_plural_pronoun_reference_surface(false);
        let plain =
            crate::effects::ReturnAllToBattlefieldEffect::new(filter, false).under_owner_control();
        assert_eq!(
            describe_return_all_to_battlefield_effect(&plain),
            "Return the exiled cards to the battlefield under their owners' control"
        );
    }
}

pub(super) fn effect_exiles_triggering_object(effect: &Effect) -> bool {
    let triggering = TagKey::from("triggering");
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return move_to_zone.zone == Zone::Exile
            && choose_spec_references_exact_tag(&move_to_zone.target, &triggering);
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return choose_spec_references_exact_tag(&exile.spec, &triggering);
    }
    false
}

pub(super) fn describe_exile_it_then_return_all_to_battlefield(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    if !effect_exiles_triggering_object(first) {
        return None;
    }
    let return_all = second.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    Some(format!(
        "Exile it, then {}",
        lowercase_first(&describe_return_all_to_battlefield_effect(return_all))
    ))
}

pub(super) fn describe_source_sacrifice_then_return_source_exiled(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let with_id = first.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = with_id
        .effect
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target, ChooseSpec::Source) {
        return None;
    }

    let if_effect = second.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [return_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let return_all =
        return_effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    if !is_source_exiled_cards_filter(&return_all.filter)
        || return_all.tapped
        || return_all.face_down
        || return_all.battlefield_controller != crate::effects::BattlefieldController::Owner
    {
        return None;
    }

    Some(
        "sacrifice it. If you do, return those cards to the battlefield under their owners' control"
            .to_string(),
    )
}

pub(super) fn describe_prevention_follow_up_target(target: &ChooseSpec) -> &'static str {
    let described = describe_choose_spec(target);
    if described.contains("creature") {
        "that creature"
    } else if described.contains("player") {
        "that player"
    } else if described.contains("permanent") {
        "that permanent"
    } else {
        "it"
    }
}

pub(super) fn render_iterative_library_repeat_process(
    repeat: &crate::effects::RepeatProcessEffect,
) -> Option<String> {
    if repeat.predicate != ironsmith_core::EffectPredicate::WasDeclined {
        return None;
    }
    let [exile_effect, conditional_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let exile_effect = exile_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map(|with_id| with_id.effect.as_ref())
        .unwrap_or(exile_effect);
    let conditional_effect = conditional_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map(|with_id| with_id.effect.as_ref())
        .unwrap_or(conditional_effect);
    let exile = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile.count != Value::Fixed(1)
        || exile.player != PlayerFilter::You
        || exile.moved_tags.len() != 1
        || exile.accumulated_tags.len() != 1
    {
        return None;
    }
    let current_tag = &exile.moved_tags[0];
    let exiled_tag = &exile.accumulated_tags[0];
    if current_tag.as_str() != "iterative_library_current"
        || exiled_tag.as_str() != "iterative_library_exiled"
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let [may_move_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may_move = may_move_effect.downcast_ref::<crate::effects::MayMoveToZoneEffect>()?;
    if may_move.zone != Zone::Hand
        || may_move.decider != PlayerFilter::You
        || !matches!(&may_move.target, ChooseSpec::Tagged(tag) if tag == current_tag)
    {
        return None;
    }

    Some(
        "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first"
            .to_string(),
    )
}

pub(in crate::compiled_text) fn choose_primary_zone(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<Zone> {
    choose.filter.zone.or(choose.zone)
}

pub(super) fn object_filter_has_tag(filter: &ObjectFilter, tag: &crate::tag::TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == *tag
    })
}

pub(super) fn choose_spec_player_filter(spec: &ChooseSpec) -> Option<PlayerFilter> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => choose_spec_player_filter(spec),
        ChooseSpec::Target(inner) => Some(PlayerFilter::Target(Box::new(
            choose_spec_player_filter(inner)?,
        ))),
        ChooseSpec::Player(filter) => Some(filter.clone()),
        _ => None,
    }
}

pub(super) fn hand_choice_selection_from_it(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.controller = None;
    filter.tagged_constraints.clear();
    let mut selection = if choose_primary_zone(choose) == Some(Zone::Hand)
        && choose.filter.card_types.is_empty()
        && choose.filter.excluded_card_types == vec![CardType::Land]
    {
        "nonland card".to_string()
    } else if choose_primary_zone(choose) == Some(Zone::Hand)
        && choose.filter.card_types.is_empty()
        && choose.filter.excluded_card_types.is_empty()
        && choose.filter.subtypes.is_empty()
        && choose.filter.colors.is_none()
        && choose.filter.mana_value.is_none()
    {
        "card".to_string()
    } else {
        filter.description()
    };
    if !selection.contains("card") {
        selection.push_str(" card");
    }
    with_indefinite_article(&selection)
}

pub(super) fn describe_discard_reveal_hand_choose_discard_chosen(
    effects: &[&Effect],
) -> Option<String> {
    let [
        discard_cost_effect,
        look_effect,
        choose_effect,
        discard_chosen_effect,
    ] = effects
    else {
        return None;
    };
    let discard_cost = discard_cost_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let discarded_tag = discard_cost.tag.as_ref()?;
    if discard_cost.player != PlayerFilter::You
        || discard_cost.count != Value::Fixed(0)
        || !discard_cost.any_number
        || discard_cost.random
        || discard_cost.card_filter.is_some()
    {
        return None;
    }

    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_dynamic_x()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !choose
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look_player))
        || !matches!(
            choose.count_value.as_ref(),
            Some(Value::Count(filter)) if object_filter_has_tag(filter, discarded_tag)
        )
    {
        return None;
    }

    let discard_chosen = discard_chosen_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard_chosen.random
        || discard_chosen.any_number
        || !player_filters_refer_to_same_player(&discard_chosen.player, &look_player)
        || !matches!(&discard_chosen.count, Value::Count(filter) if object_filter_has_tag(filter, &choose.tag))
        || !discard_chosen
            .card_filter
            .as_ref()
            .is_some_and(|filter| object_filter_has_tag(filter, &choose.tag))
    {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let selection = hand_choice_selection_from_it(choose);
    Some(format!(
        "Discard any number of cards. {} {} their hand, then you choose {selection} from it for each card discarded this way. That player discards those cards",
        capitalize_first(&revealer),
        reveal_verb
    ))
}

pub(super) fn describe_reveal_hand_subset_choose_then_discard(
    effects: &[&Effect],
) -> Option<String> {
    let [reveal_effect, choose_effect, discard_effect] = effects else {
        return None;
    };
    let reveal = reveal_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !reveal.reveal
        || reveal.is_search
        || choose.is_search
        || choose_primary_zone(reveal) != Some(Zone::Hand)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !reveal
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &reveal.chooser))
        || !choose
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &reveal.chooser))
        || !player_filters_refer_to_same_player(&discard.player, &reveal.chooser)
        || discard.random
        || discard.any_number
    {
        return None;
    }
    let (count_text, count_suffix) = if reveal.count.dynamic_x {
        match reveal.count_value.as_ref() {
            Some(count_value) if value_prefers_where_x(count_value) => (
                "X".to_string(),
                format!(", where X is {}", describe_value(count_value)),
            ),
            Some(count_value) => (
                "a number of".to_string(),
                format!(" equal to {}", describe_value(count_value)),
            ),
            None => ("X".to_string(), String::new()),
        }
    } else {
        let reveal_count = reveal.count.max.filter(|max| *max == reveal.count.min)?;
        (
            small_number_word(reveal_count as u32).unwrap_or_else(|| reveal_count.to_string()),
            String::new(),
        )
    };
    let card_filter = discard.card_filter.as_ref()?;
    let chooses_revealed = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == reveal.tag
    });
    let card_filter_discards_chosen = card_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == choose.tag
    });
    let count_discards_chosen = matches!(
        &discard.count,
        Value::Count(filter) if object_filter_has_tag(filter, &choose.tag)
    );
    if !chooses_revealed
        || !card_filter_discards_chosen
        || !(count_discards_chosen
            || (choose.count.is_single() && discard.count == Value::Fixed(1)))
    {
        return None;
    }

    let player = describe_player_filter(&reveal.chooser);
    let verb = player_verb(&player, "reveal", "reveals");
    let (chosen_quantity, chosen_plural) = if choose.count.is_dynamic_x() {
        (
            if choose.count.is_up_to_dynamic_x() {
                "up to X".to_string()
            } else {
                "X".to_string()
            },
            true,
        )
    } else {
        let chosen_count = choose.count.max.filter(|max| *max == choose.count.min)?;
        (
            small_number_word(chosen_count as u32).unwrap_or_else(|| chosen_count.to_string()),
            chosen_count != 1,
        )
    };
    let followup = if chosen_plural {
        format!(". You choose {chosen_quantity} of them")
    } else {
        match reveal.count_value.as_ref() {
            Some(count_value) if value_prefers_where_x(count_value) => {
                ". You choose one of those cards".to_string()
            }
            Some(_) => ". You choose one of them".to_string(),
            None => " and you choose one of them".to_string(),
        }
    };
    let discarded_reference = if chosen_plural {
        "those cards"
    } else {
        "that card"
    };
    Some(format!(
        "{} {} {count_text} cards from their hand{count_suffix}{followup}. That player discards {discarded_reference}",
        capitalize_first(&player),
        verb
    ))
}

fn tagged_selection_discard_matches(
    discard: &crate::effects::DiscardEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    expected_player: &PlayerFilter,
) -> bool {
    !discard.random
        && !discard.any_number
        && player_filters_refer_to_same_player(&discard.player, expected_player)
        && matches!(
            &discard.count,
            Value::Count(filter) if object_filter_has_tag(filter, &choose.tag)
        )
        && discard
            .card_filter
            .as_ref()
            .is_some_and(|filter| object_filter_has_tag(filter, &choose.tag))
}

fn counted_hand_choice_from_it_text(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<(String, String)> {
    let selection = hand_choice_from_it_text(choose)?;
    if choose.count.is_single() {
        return Some((selection, String::new()));
    }

    let (quantity, plural) = if choose.count.is_dynamic_x() {
        (
            if choose.count.is_up_to_dynamic_x() {
                "up to X".to_string()
            } else {
                "X".to_string()
            },
            true,
        )
    } else {
        match (choose.count.min, choose.count.max) {
            (0, Some(max)) => (
                format!(
                    "up to {}",
                    small_number_word(max as u32).unwrap_or_else(|| max.to_string())
                ),
                max != 1,
            ),
            (min, Some(max)) if min == max => (
                small_number_word(max as u32).unwrap_or_else(|| max.to_string()),
                max != 1,
            ),
            (0, None) => ("any number of".to_string(), true),
            _ => return None,
        }
    };
    let noun = strip_leading_article(&selection);
    let noun = if plural {
        pluralize_noun_phrase(noun)
    } else {
        noun.to_string()
    };
    let where_clause = choose
        .count_value
        .as_ref()
        .map(|value| format!(", where X is {}", describe_value(value)))
        .unwrap_or_default();
    Some((format!("{quantity} {noun}"), where_clause))
}

pub(super) fn describe_reveal_hand_choose_two_filters_then_discard(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, first_effect, second_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let first = first_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second = second_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let discard = unwrap_basic_tag_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let looked_player = choose_spec_player_filter(&look.target)?;
    if !look.reveal
        || first.is_search
        || second.is_search
        || first.chooser != PlayerFilter::You
        || second.chooser != PlayerFilter::You
        || !first.count.is_single()
        || !second.count.is_single()
        || first.tag != second.tag
        || first.replace_tagged_objects
        || second.replace_tagged_objects
        || first.filter == second.filter
        || choose_primary_zone(first) != Some(Zone::Hand)
        || choose_primary_zone(second) != Some(Zone::Hand)
        || !first
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &looked_player))
        || !second
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &looked_player))
        || !tagged_selection_discard_matches(discard, second, &looked_player)
    {
        return None;
    }

    let first_selection = hand_choice_from_it_text(first)?;
    let second_selection = hand_choice_from_it_text(second)?;
    Some(format!(
        "{}. You choose from it {first_selection} and {second_selection}. That player discards those cards",
        describe_effect(look_effect).trim_end_matches('.')
    ))
}

pub(in crate::compiled_text) fn describe_look_hand_choose_then_discard_or_exile(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, choose_effect, action_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let action_effect = unwrap_basic_tag_wrappers(action_effect);

    if let Some(discard) = action_effect.downcast_ref::<crate::effects::DiscardEffect>()
        && !choose.count.is_single()
        && !choose.is_search
        && choose.chooser == PlayerFilter::You
        && choose_primary_zone(choose) == Some(Zone::Hand)
    {
        let look_player = choose_spec_player_filter(&look.target)?;
        let owner_matches = choose
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look_player));
        if owner_matches && tagged_selection_discard_matches(discard, choose, &look_player) {
            let (selection, where_clause) = counted_hand_choice_from_it_text(choose)?;
            let look_text = describe_effect(look_effect)
                .trim()
                .trim_end_matches('.')
                .to_string();
            return Some(if look.reveal {
                format!(
                    "{look_text}. You choose {selection} from it{where_clause}. That player discards those cards"
                )
            } else {
                format!(
                    "{look_text} and choose {selection} from it{where_clause}. That player discards those cards"
                )
            });
        }
    }

    if let Some((reveal_text, choice_text, look_player)) =
        describe_reveal_hand_choose_from_it_or_graveyard(look, choose)
    {
        if let Some(discard) = action_effect.downcast_ref::<crate::effects::DiscardEffect>() {
            if !discard_discards_chosen_card(discard, choose, &look_player) {
                return None;
            }
            return Some(format!(
                "{reveal_text}. You choose {choice_text}. That player discards that card"
            ));
        }
        if let Some(exile) = action_effect.downcast_ref::<crate::effects::ExileEffect>()
            && exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
        {
            return Some(format!(
                "{reveal_text}. You choose {choice_text}. Exile that card"
            ));
        }
        if let Some(move_to_zone) = action_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            && move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str())
        {
            return Some(format!(
                "{reveal_text}. You choose {choice_text}. Exile that card"
            ));
        }
        return None;
    }

    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;
    let choice_from_it = card_choice_from_it_text(&choice_text);

    if let Some(discard) = action_effect.downcast_ref::<crate::effects::DiscardEffect>() {
        if !discard_discards_chosen_card(discard, choose, &look_player) {
            return None;
        }
        return Some(format!(
            "{reveal_text}. You choose {choice_from_it}. That player discards that card"
        ));
    }

    if let Some(exile) = action_effect.downcast_ref::<crate::effects::ExileEffect>()
        && exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
    {
        return Some(format!(
            "{reveal_text}. You choose {choice_from_it} and exile that card"
        ));
    }
    if let Some(move_to_zone) = action_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        return Some(format!(
            "{reveal_text}. You choose {choice_from_it} and exile that card"
        ));
    }
    None
}

pub(in crate::compiled_text) fn describe_reveal_hand_choose_discard_inline(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, _, action_effect] = effects else {
        return None;
    };
    if !look_effect
        .downcast_ref::<crate::effects::LookAtHandEffect>()?
        .reveal
        || unwrap_basic_tag_wrappers(action_effect)
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_none()
    {
        return None;
    }
    let rendered = describe_look_hand_choose_then_discard_or_exile(effects)?;
    let (reveal, rest) = rendered.split_once(". You choose ")?;
    let (choice, discard) = rest.split_once(". That player discards ")?;
    Some(format!(
        "{reveal}, you choose {choice}, then that player discards {discard}"
    ))
}

pub(super) fn describe_reveal_hand_optional_choice_discard_else_exile(
    effects: &[&Effect],
) -> Option<String> {
    let [
        look_effect,
        may_choose_effect,
        discard_if_effect,
        fallback_if_effect,
    ] = effects
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let looked_player = choose_spec_player_filter(&look.target)?;
    if !look.reveal || looked_player == PlayerFilter::You {
        return None;
    }

    let may_choose_with_id = may_choose_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may_choose = may_choose_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [choose_effect] = may_choose.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if may_choose.decider.as_ref() != Some(&PlayerFilter::You)
        || may_choose.fallback != crate::decision::FallbackStrategy::Decline
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.excluded_card_types.as_slice() != [CardType::Land]
        || !choose
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &looked_player))
    {
        return None;
    }

    let discard_with_id = discard_if_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let discard_if = discard_with_id
        .effect
        .downcast_ref::<crate::effects::IfEffect>()?;
    let [discard_effect] = discard_if.then.as_slice() else {
        return None;
    };
    let discard = unwrap_basic_tag_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard_if.condition != may_choose_with_id.id
        || discard_if.predicate != EffectPredicate::Happened
        || !discard_if.else_.is_empty()
        || !discard_discards_chosen_card(discard, choose, &looked_player)
    {
        return None;
    }

    let fallback_if = fallback_if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [fallback_may_effect] = fallback_if.then.as_slice() else {
        return None;
    };
    let fallback_may = fallback_may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [move_effect] = fallback_may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::Object(exiled_filter) = move_to_zone.target.base() else {
        return None;
    };
    if fallback_if.condition != may_choose_with_id.id
        || fallback_if.predicate != EffectPredicate::DidNotHappen
        || !fallback_if.else_.is_empty()
        || fallback_may.decider.as_ref() != Some(&PlayerFilter::You)
        || fallback_may.fallback != crate::decision::FallbackStrategy::Decline
        || move_to_zone.zone != Zone::Graveyard
        || move_to_zone.to_top
        || move_to_zone.target_plural_surface
        || exiled_filter.zone != Some(Zone::Exile)
        || exiled_filter.face_down != Some(false)
        || !exiled_filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &looked_player))
    {
        return None;
    }

    Some(format!(
        "{}. You may choose {} from it. If you do, that player discards that card. Otherwise, you may put a face-up exiled card they own into their graveyard",
        describe_effect(look_effect).trim_end_matches('.'),
        hand_choice_from_it_text(choose)?
    ))
}

pub(super) fn describe_look_hand_choose_then_discard(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if look.reveal {
        return None;
    }
    let (choose, optional) =
        if let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            (choose, false)
        } else {
            let may = choose_effect.downcast_ref::<crate::effects::MayEffect>()?;
            if may
                .decider
                .as_ref()
                .is_some_and(|decider| decider != &PlayerFilter::You)
            {
                return None;
            }
            let [inner] = may.effects.as_slice() else {
                return None;
            };
            (
                inner.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
                true,
            )
        };
    let looked_player = choose_spec_player_filter(&look.target)?;
    if choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !choose
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &looked_player))
    {
        return None;
    }
    let discard = unwrap_basic_tag_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if !discard_discards_chosen_card(discard, choose, &looked_player) {
        return None;
    }
    let look_text = describe_effect(look_effect);
    let choice = hand_choice_from_it_text(choose)?;
    let discard_subject = if look_text
        .to_ascii_lowercase()
        .contains("that player's hand")
    {
        "The player"
    } else {
        "That player"
    };
    Some(if optional {
        format!(
            "{look_text}. You may choose {choice} from it. {discard_subject} discards that card"
        )
    } else {
        format!("{look_text} and choose {choice} from it. {discard_subject} discards that card")
    })
}

pub(in crate::compiled_text) fn describe_player_damage_then_same_player_discards(
    effects: &[&Effect],
) -> Option<String> {
    let [damage_effect, discard_effect] = effects else {
        return None;
    };
    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if matches!(
        damage.target.unhinted(),
        ChooseSpec::PlayerOrPlaneswalker(_)
    ) && discard.player == PlayerFilter::TargetPlayerOrControllerOfTarget
        && !discard.random
        && !discard.any_number
    {
        return Some(format!(
            "{}. That player or that planeswalker's controller discards {}",
            describe_effect(damage_effect).trim_end_matches('.'),
            describe_discard_count(&discard.count, discard.card_filter.as_ref())
        ));
    }

    let damaged_player = choose_spec_player_filter(&damage.target)?;
    if !matches!(damaged_player, PlayerFilter::Target(_)) {
        return None;
    }
    if discard.random
        || discard.any_number
        || !player_filters_refer_to_same_player(&damaged_player, &discard.player)
    {
        return None;
    }
    Some(format!(
        "{}. That player discards {}",
        describe_effect(damage_effect).trim_end_matches('.'),
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    ))
}

pub(super) fn describe_target_player_sacrifice_then_gain_toughness(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, target_effect, gain_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    let target_player = choose_spec_player_filter(&target_only.target)?;
    let gain_player = choose_spec_player_filter(&gain.player)?;
    if !player_filters_refer_to_same_player(&choose.chooser, sacrifice.player)
        || !player_filters_refer_to_same_player(&choose.chooser, &target_player)
        || !player_filters_refer_to_same_player(&choose.chooser, &gain_player)
        || !matches!(
            gain.amount.unhinted(),
            Value::ToughnessOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        )
    {
        return None;
    }
    let sacrifice_text = describe_choose_then_sacrifice(choose, sacrifice)?;
    Some(format!(
        "{sacrifice_text}, then gains life equal to that creature's toughness"
    ))
}

pub(super) fn describe_target_player_reveal_top(
    target_effect: &Effect,
    reveal_effect: &Effect,
) -> Option<String> {
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let selected_player = choose_spec_player_filter(&target_only.target)?;
    if matches!(selected_player, PlayerFilter::You) {
        return None;
    }
    if let Some(reveal) = reveal_effect.downcast_ref::<crate::effects::RevealTopEffect>() {
        if !player_filters_refer_to_same_player(&selected_player, &reveal.player) {
            return None;
        }
        if selected_player == PlayerFilter::Target(Box::new(PlayerFilter::Any)) {
            return Some("Target player reveals the top card of their library".to_string());
        }
        return Some(format!(
            "Reveal the top card of {} library",
            describe_possessive_player_filter(&selected_player)
        ));
    }
    let reveal = reveal_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if !reveal.reveal || !player_filters_refer_to_same_player(&selected_player, &reveal.player) {
        return None;
    }
    let player = describe_player_filter(&selected_player);
    let (count, noun, where_clause) = describe_top_count_noun_and_where_clause(&reveal.count);
    let top = if reveal.count == Value::Fixed(1) {
        format!("top {noun}")
    } else {
        format!("top {count} {noun}")
    };
    Some(format!(
        "{} {} the {top} of their library{where_clause}",
        capitalize_first(&player),
        player_verb(&player, "reveal", "reveals")
    ))
}

#[cfg(test)]
mod target_player_reveal_top_tests {
    use super::*;

    #[test]
    fn exact_target_player_reveal_uses_the_target_as_grammatical_subject() {
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
            ChooseSpec::Player(PlayerFilter::Any),
        )));
        let reveal = Effect::new(crate::effects::RevealTopEffect::new(
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
            None,
        ));

        assert_eq!(
            describe_target_player_reveal_top(&target, &reveal).as_deref(),
            Some("Target player reveals the top card of their library")
        );

        let mismatched = Effect::new(crate::effects::RevealTopEffect::new(
            PlayerFilter::Opponent,
            None,
        ));
        assert!(describe_target_player_reveal_top(&target, &mismatched).is_none());
    }
}

pub(super) fn describe_reveal_hand_then_gain_for_that_players_hand(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, gain_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let looked_player = choose_spec_player_filter(&look.target)?;
    if !matches!(looked_player, PlayerFilter::Target(_)) {
        return None;
    }
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if choose_spec_player_filter(&gain.player)? != PlayerFilter::You {
        return None;
    }
    let Value::Count(filter) = gain.amount.unhinted() else {
        return None;
    };
    let owner = filter.owner.as_ref()?;
    let mut plain_hand = filter.clone();
    plain_hand.zone = None;
    plain_hand.owner = None;
    if filter.zone != Some(Zone::Hand)
        || plain_hand != ObjectFilter::default()
        || !player_filters_refer_to_same_player(&looked_player, owner)
    {
        return None;
    }
    let player = describe_player_filter(&looked_player);
    Some(format!(
        "{} {} their hand. You gain life equal to the number of cards in that player's hand",
        capitalize_first(&player),
        player_verb(&player, "reveal", "reveals")
    ))
}

pub(super) fn describe_target_player_look_top_may_move_that_card(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, look_effect, may_effect] = effects else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let selected_player = choose_spec_player_filter(&target_only.target)?;
    if !matches!(selected_player, PlayerFilter::Target(_)) {
        return None;
    }
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.reveal
        || look.count != Value::Fixed(1)
        || !player_filters_refer_to_same_player(&selected_player, &look.player)
    {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| decider != &PlayerFilter::You)
    {
        return None;
    }
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard
        || move_to_zone.to_top
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &look.tag)
        || !move_to_zone
            .destination_player_surface
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &selected_player))
    {
        return None;
    }

    let possessive = if move_to_zone.destination_player_reference_surface
        == Some(ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer)
    {
        "that player's"
    } else {
        "their"
    };
    Some(format!(
        "{}. You may put that card into {possessive} graveyard",
        describe_effect(look_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_target_player_consult_exile_shuffle_may_cast(
    effects: &[&Effect],
) -> Option<String> {
    let [
        target_effect,
        consult_effect,
        move_effect,
        shuffle_effect,
        may_effect,
    ] = effects
    else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let selected_player = choose_spec_player_filter(&target_only.target)?;
    if !matches!(selected_player, PlayerFilter::Target(_)) {
        return None;
    }
    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        || !player_filters_refer_to_same_player(&selected_player, &consult.player)
    {
        return None;
    }
    let move_to_exile = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_exile.zone != Zone::Exile
        || move_to_exile.enters_face_down
        || !matches!(move_to_exile.target.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
    {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.target_spec.is_some()
        || !player_filters_refer_to_same_player(&selected_player, &shuffle.player)
    {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| decider != &PlayerFilter::You)
    {
        return None;
    }
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != consult.match_tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || cast.cost_reduction.is_some()
    {
        return None;
    }
    let free_cast = if cast.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };

    Some(format!(
        "{}. Exile that card, then that player shuffles. You may cast that exiled card{free_cast}",
        describe_effect(consult_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_reveal_hand_then_same_player_discards(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let discard = unwrap_basic_tag_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if !look.reveal {
        return None;
    }
    let looked_player = choose_spec_player_filter(&look.target)?;
    if !player_filters_refer_to_same_player(&looked_player, &discard.player) {
        return None;
    }

    let player = describe_player_filter(&looked_player);
    let discard_count = if discard.any_number {
        match discard.card_filter.as_ref() {
            Some(filter) => format!(
                "any number of {}",
                pluralize_discard_card_phrase(&describe_discard_card_phrase(filter))
            ),
            None => "any number of cards".to_string(),
        }
    } else {
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    };
    let random_suffix = if discard.random { " at random" } else { "" };
    Some(format!(
        "{} {} their hand and {} {discard_count}{random_suffix}",
        capitalize_first(&player),
        player_verb(&player, "reveal", "reveals"),
        player_verb(&player, "discard", "discards"),
    ))
}

pub(super) fn describe_reveal_hand_choose_from_it_or_graveyard(
    look: &crate::effects::LookAtHandEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<(String, String, PlayerFilter)> {
    if !look.reveal
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose.zone != Some(Zone::Hand)
        || choose.filter.any_of.len() != 2
    {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    let hand_arm = choose.filter.any_of.iter().find(|option| {
        option.zone == Some(Zone::Hand)
            && option.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::effects::REVEALED_THIS_WAY_TAG
                    && matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    )
            })
    })?;
    let graveyard_arm = choose.filter.any_of.iter().find(|option| {
        option.zone == Some(Zone::Graveyard)
            && option
                .owner
                .as_ref()
                .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look_player))
    })?;

    let hand_choice = describe_card_choice_without_zone(hand_arm)?;
    let graveyard_choice = describe_card_choice_without_zone(graveyard_arm)?;
    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    Some((
        format!("{} {} their hand", capitalize_first(&revealer), reveal_verb),
        format!("{hand_choice} from it or {graveyard_choice} from their graveyard"),
        look_player,
    ))
}

pub(super) fn describe_card_choice_without_zone(filter: &ObjectFilter) -> Option<String> {
    let mut display = filter.clone();
    display.zone = None;
    display.owner = None;
    display.controller = None;
    display.tagged_constraints.clear();
    display.any_of.clear();
    let mut choice =
        if display.card_types.is_empty() && display.excluded_card_types == vec![CardType::Land] {
            "nonland card".to_string()
        } else if display.card_types.is_empty()
            && display.excluded_card_types.is_empty()
            && display.subtypes.is_empty()
            && display.colors.is_none()
            && display.mana_value.is_none()
            && display.supertypes.is_empty()
        {
            "card".to_string()
        } else {
            display.description()
        };
    if !choice.contains("card") {
        choice.push_str(" card");
    }
    Some(with_indefinite_article(&choice))
}

pub(super) fn describe_reveal_hand_choose_from_it(
    look: &crate::effects::LookAtHandEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<(String, String, PlayerFilter)> {
    if !look.reveal
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
    {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    if !choose
        .filter
        .owner
        .as_ref()
        .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look_player))
    {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let choice_text = hand_choice_from_it_text(choose)?;
    Some((
        format!("{} {} their hand", capitalize_first(&revealer), reveal_verb),
        choice_text,
        look_player,
    ))
}

pub(super) fn player_filters_refer_to_same_player(
    left: &PlayerFilter,
    right: &PlayerFilter,
) -> bool {
    if left == right {
        return true;
    }
    match (left, right) {
        (PlayerFilter::ControllerOf(left), PlayerFilter::AliasedControllerOf(right))
        | (PlayerFilter::AliasedControllerOf(left), PlayerFilter::ControllerOf(right))
        | (PlayerFilter::OwnerOf(left), PlayerFilter::AliasedOwnerOf(right))
        | (PlayerFilter::AliasedOwnerOf(left), PlayerFilter::OwnerOf(right)) => left == right,
        (PlayerFilter::Target(inner), other)
        | (other, PlayerFilter::Target(inner))
        | (PlayerFilter::AliasedTarget(inner), other)
        | (other, PlayerFilter::AliasedTarget(inner)) => {
            player_filters_refer_to_same_player(inner, other)
        }
        _ => false,
    }
}

pub(super) fn discard_discards_chosen_card(
    discard: &crate::effects::DiscardEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    expected_player: &PlayerFilter,
) -> bool {
    discard.count == Value::Fixed(1)
        && !discard.random
        && !discard.any_number
        && player_filters_refer_to_same_player(&discard.player, expected_player)
        && discard
            .card_filter
            .as_ref()
            .is_some_and(|filter| object_filter_has_tag(filter, &choose.tag))
}

fn referenced_player_action(effect: &Effect) -> Option<(PlayerFilter, String)> {
    let effect = unwrap_basic_tag_wrappers(effect);
    let actor = if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        discard.player.clone()
    } else if let Some(mill) = effect.downcast_ref::<crate::effects::MillEffect>() {
        mill.player.clone()
    } else if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        choose_spec_player_filter(&lose.player)?
    } else if let Some(pay) = effect.downcast_ref::<crate::effects::PayLifeEffect>() {
        choose_spec_player_filter(&pay.player)?
    } else if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        choose_spec_player_filter(&gain.player)?
    } else if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        draw.player.clone()
    } else if let Some(look) = effect.downcast_ref::<crate::effects::LookAtHandEffect>() {
        if !look.reveal {
            return None;
        }
        choose_spec_player_filter(&look.target)?
    } else if let Some(sacrifice) = sacrifice_view(effect) {
        sacrifice.player.clone()
    } else {
        return None;
    };

    if matches!(actor, PlayerFilter::You) {
        return None;
    }
    let rendered = describe_effect(effect);
    let subject = describe_player_filter(&actor);
    let action = rendered
        .strip_prefix(&subject)
        .or_else(|| rendered.strip_prefix(&capitalize_first(&subject)))?
        .trim_start()
        .to_string();
    (!action.is_empty()).then_some((actor, action))
}

fn result_id_is_used_by_later_effects(effect: &Effect, later: &[&Effect]) -> bool {
    let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return false;
    };
    later.iter().any(|later_effect| {
        later_effect
            .downcast_ref::<crate::effects::IfEffect>()
            .is_some_and(|if_effect| if_effect.condition == with_id.id)
    })
}

pub(super) fn describe_same_referenced_player_action_sequence(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    if result_id_is_used_by_later_effects(*effects.first()?, &effects[1..]) {
        return None;
    }
    let (first_actor, first_action) = referenced_player_action(*effects.first()?)?;
    if !matches!(
        first_actor,
        PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_)
    ) {
        return None;
    }

    let mut actions = vec![first_action];
    let mut consumed = 1;
    for (offset, effect) in effects[1..].iter().enumerate() {
        if result_id_is_used_by_later_effects(effect, &effects[offset + 2..]) {
            break;
        }
        let Some((actor, action)) = referenced_player_action(effect) else {
            break;
        };
        if !player_filters_refer_to_same_player(&first_actor, &actor) {
            break;
        }
        actions.push(action);
        consumed += 1;
    }
    if consumed < 2 {
        return None;
    }

    let mut shared_where_x = None;
    let mut stripped_actions = Vec::with_capacity(actions.len());
    for action in actions {
        if let Some((head, tail)) = action.split_once(", where X is ") {
            let suffix = format!(", where X is {tail}");
            if shared_where_x
                .as_ref()
                .is_some_and(|known| known != &suffix)
            {
                shared_where_x = None;
                stripped_actions.clear();
                break;
            }
            shared_where_x = Some(suffix);
            stripped_actions.push(head.to_string());
        } else {
            shared_where_x = None;
            stripped_actions.clear();
            break;
        }
    }
    let actions = if stripped_actions.len() == consumed {
        stripped_actions
    } else {
        effects[..consumed]
            .iter()
            .map(|effect| referenced_player_action(effect).map(|(_, action)| action))
            .collect::<Option<Vec<_>>>()?
    };
    let joined_actions = if actions.len() == 2
        && actions
            .iter()
            .all(|action| action.starts_with("sacrifices ") && action.ends_with(" of their choice"))
    {
        let first = actions[0]
            .trim_start_matches("sacrifices ")
            .trim_end_matches(" of their choice");
        let second = actions[1]
            .trim_start_matches("sacrifices ")
            .trim_end_matches(" of their choice");
        format!("sacrifices {first} and {second} of their choice")
    } else if actions.len() == 2
        && unwrap_basic_tag_wrappers(effects[0])
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_some()
        && (unwrap_basic_tag_wrappers(effects[1])
            .downcast_ref::<crate::effects::MillEffect>()
            .is_some()
            || unwrap_basic_tag_wrappers(effects[1])
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some())
    {
        format!("{}, then {}", actions[0], actions[1])
    } else if actions.len() == 2 {
        format!("{} and {}", actions[0], actions[1])
    } else {
        join_with_and(&actions)
    };
    let subject = if matches!(
        &first_actor,
        PlayerFilter::Target(inner)
            if matches!(inner.as_ref(), PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_))
    ) {
        "That player".to_string()
    } else {
        capitalize_first(&describe_player_filter(&first_actor))
    };
    Some((
        format!(
            "{subject} {joined_actions}{}",
            shared_where_x.unwrap_or_default()
        ),
        consumed,
    ))
}

pub(super) fn describe_choose_sacrifice_then_same_player_actions(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [
        choose_effect,
        sacrifice_effect,
        first_action_effect,
        second_action_effect,
        ..,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    let sacrifice_text = describe_choose_then_sacrifice(choose, sacrifice)?;
    let (first_actor, first_action) = referenced_player_action(first_action_effect)?;
    let (second_actor, second_action) = referenced_player_action(second_action_effect)?;
    if !player_filters_refer_to_same_player(&choose.chooser, &first_actor)
        || !player_filters_refer_to_same_player(&first_actor, &second_actor)
    {
        return None;
    }
    Some((
        format!("{sacrifice_text}, {first_action}, and {second_action}"),
        4,
    ))
}

pub(super) fn describe_two_choose_sacrifices_same_player(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [
        first_choose_effect,
        first_sacrifice_effect,
        second_choose_effect,
        second_sacrifice_effect,
        ..,
    ] = effects
    else {
        return None;
    };
    let first_choose = first_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second_choose =
        second_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !player_filters_refer_to_same_player(&first_choose.chooser, &second_choose.chooser) {
        return None;
    }
    let first =
        describe_choose_then_sacrifice(first_choose, sacrifice_view(first_sacrifice_effect)?)?;
    let second =
        describe_choose_then_sacrifice(second_choose, sacrifice_view(second_sacrifice_effect)?)?;
    let subject_text = describe_player_filter(&first_choose.chooser);
    let first_object = first
        .strip_prefix(&subject_text)?
        .trim_start()
        .strip_prefix("sacrifices ")?
        .strip_suffix(" of their choice")?;
    let second_subject = describe_player_filter(&second_choose.chooser);
    let second_object = second
        .strip_prefix(&second_subject)?
        .trim_start()
        .strip_prefix("sacrifices ")?
        .strip_suffix(" of their choice")?;
    let subject = capitalize_first(&subject_text);
    Some((
        format!("{subject} sacrifices {first_object} and {second_object} of their choice"),
        4,
    ))
}

pub(super) fn describe_discard_then_exile_same_player_graveyard(
    discard_effect: &Effect,
    exile_effect: &Effect,
) -> Option<String> {
    let discard = unwrap_basic_tag_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let exile =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down {
        return None;
    }
    let ChooseSpec::All(filter) = &exile.spec else {
        return None;
    };
    let owner = filter.owner.as_ref()?;
    if filter.zone != Some(Zone::Graveyard)
        || !player_filters_refer_to_same_player(&discard.player, owner)
        || !filter.card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.tagged_constraints.is_empty()
        || !filter.any_of.is_empty()
    {
        return None;
    }
    Some(format!(
        "{}. Then exile that player's graveyard",
        describe_effect(discard_effect).trim_end_matches('.')
    ))
}

pub(super) fn hand_choice_from_it_text(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.controller = None;
    filter.tagged_constraints.clear();
    let mut choice = if choose_card_name_excludes_only_basic_lands(&filter) {
        "card other than a basic land card".to_string()
    } else if filter == ObjectFilter::default() {
        "card".to_string()
    } else if filter.card_types.is_empty()
        && filter.excluded_card_types == vec![CardType::Land]
        && filter.mana_value.is_none()
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.colors.is_none()
        && filter.any_of.is_empty()
    {
        "nonland card".to_string()
    } else {
        filter.description()
    };
    if choice == "nonland permanent" {
        choice = "nonland card".to_string();
    } else if let Some(rest) = choice.strip_prefix("nonland permanent with ") {
        choice = format!("nonland card with {rest}");
    } else if let Some(rest) = choice.strip_prefix("permanent with ") {
        choice = format!("card with {rest}");
    } else if let Some(rest) = choice.strip_prefix("with ") {
        choice = format!("card with {rest}");
    }
    if !choice.contains("card") {
        choice.push_str(" card");
    }
    Some(with_indefinite_article(&choice))
}

pub(super) fn card_choice_from_it_text(choice: &str) -> String {
    if let Some((noun, qualifier)) = choice.split_once(" with ") {
        format!("{noun} from it with {qualifier}")
    } else {
        format!("{choice} from it")
    }
}

pub(super) fn tagged_move_to_library_nth_from_effect(
    effect: &Effect,
) -> Option<&crate::effects::MoveToLibraryNthFromTopEffect> {
    unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToLibraryNthFromTopEffect>()
}

pub(super) fn choose_owner_matches_looked_player(
    choose: &crate::effects::ChooseObjectsEffect,
    looked_player: &str,
) -> bool {
    choose.filter.owner.as_ref().is_none_or(|owner| {
        let owner_text = describe_player_filter(owner);
        owner_text == looked_player || owner_text == format!("target {looked_player}")
    })
}

pub(super) fn describe_hand_choose_then_library_placement(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, move_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You || choose_primary_zone(choose) != Some(Zone::Hand) {
        return None;
    }

    let looked_player = describe_choose_spec(&look.target);
    if !choose_owner_matches_looked_player(choose, &looked_player) {
        return None;
    }

    let selection = hand_choice_selection_from_it(choose);
    if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_library_uses_chosen_tag(move_to_zone, choose.tag.as_str())
        && move_to_zone.to_top
    {
        let look_verb = if look.reveal {
            player_verb(&looked_player, "reveal", "reveals")
        } else {
            "look at"
        };
        let opener = if look.reveal {
            format!(
                "{} {look_verb} their hand",
                capitalize_first(&looked_player)
            )
        } else {
            format!("Look at {looked_player}'s hand")
        };
        let moved_reference = if choose.count.is_single() {
            "that card"
        } else {
            "them"
        };
        let order_suffix = match move_to_zone.library_order.as_ref() {
            Some(crate::effects::LibraryPlacementOrder::Random) => " in a random order",
            Some(crate::effects::LibraryPlacementOrder::ChosenBy(_)) => " in any order",
            None => "",
        };
        return Some(format!(
            "{opener} and choose {selection} from it. Put {moved_reference} on top of that player's library{order_suffix}"
        ));
    }

    if let Some(move_to_library) = tagged_move_to_library_nth_from_effect(move_effect)
        && matches!(&move_to_library.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        let position = library_position_from_top_text(&move_to_library.position, true);
        let reveal_verb = player_verb(&looked_player, "reveal", "reveals");
        return Some(format!(
            "{} {reveal_verb} their hand. You choose {selection} from it. That player puts that card into their library {position}",
            capitalize_first(&looked_player)
        ));
    }

    None
}

pub(super) fn describe_target_player_choose_hand_top_library_any_order(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, choose_effect, move_effect] = effects else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.target != ChooseSpec::target_player() {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search
        || choose.chooser != PlayerFilter::target_player()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.owner.as_ref() != Some(&PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_library_uses_chosen_tag(move_to_zone, choose.tag.as_str()) || !move_to_zone.to_top {
        return None;
    }

    let chosen = describe_choose_selection(choose);
    let moved_ref = if choose.count.is_single() {
        "it"
    } else {
        "them"
    };
    let order_suffix = if choose.count.is_single() {
        ""
    } else {
        match move_to_zone.library_order.as_ref() {
            Some(crate::effects::LibraryPlacementOrder::Random) => " in a random order",
            Some(crate::effects::LibraryPlacementOrder::ChosenBy(_)) | None => " in any order",
        }
    };
    Some(format!(
        "Target player chooses {chosen} from their hand and puts {moved_ref} on top of their library{order_suffix}"
    ))
}

pub(super) fn choose_spec_is_target_permanent(spec: &ChooseSpec) -> bool {
    describe_choose_spec(spec) == "target permanent"
}

pub(super) fn card_types_are_permanent_card_types(card_types: &[CardType]) -> bool {
    let required = [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    card_types.len() == required.len()
        && required
            .iter()
            .all(|card_type| card_types.contains(card_type))
}

pub(super) fn object_filter_is_plain_permanent_card(filter: &ObjectFilter) -> bool {
    if filter.zone.is_some() || !card_types_are_permanent_card_types(&filter.card_types) {
        return false;
    }
    let mut rest = filter.clone();
    rest.card_types.clear();
    rest == ObjectFilter::default()
}

pub(super) fn player_filter_is_owner_of_tag(player: &PlayerFilter, tag: &TagKey) -> bool {
    matches!(
        player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(owner_tag)) if owner_tag == tag
    )
}

pub(super) fn move_revealed_tag_to_battlefield(effect: &Effect, tag: &TagKey) -> bool {
    let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    move_to_zone.zone == Zone::Battlefield
        && !move_to_zone.to_top
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(move_tag) if move_tag == tag)
}

pub(super) fn describe_target_permanent_shuffle_reveal_permanent_card(
    effects: &[Effect],
) -> Option<String> {
    let [
        move_effect,
        shuffle_effect,
        reveal_effect,
        conditional_effect,
    ] = effects
    else {
        return None;
    };
    let moved_tag = wrapped_effect_tag(move_effect)?;
    let move_to_library = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_library.zone != Zone::Library
        || move_to_library.to_top
        || !choose_spec_is_target_permanent(&move_to_library.target)
    {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !player_filter_is_owner_of_tag(&shuffle.player, moved_tag) {
        return None;
    }
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    let reveal_tag = reveal.tag.as_ref()?;
    if !player_filter_is_owner_of_tag(&reveal.player, moved_tag) {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    if condition_tag != reveal_tag
        || !object_filter_is_plain_permanent_card(filter)
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let [move_revealed] = conditional.if_true.as_slice() else {
        return None;
    };
    if !move_revealed_tag_to_battlefield(move_revealed, reveal_tag) {
        return None;
    }

    Some(
        "The owner of target permanent shuffles it into their library, then reveals the top card of their library. If it's a permanent card, they put it onto the battlefield"
            .to_string(),
    )
}

pub(super) fn describe_reveal_hand_choose_discard_then_scry(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, discard_effect, scry_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;
    let choice_from_it = card_choice_from_it_text(&choice_text);
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !discard_discards_chosen_card(discard, choose, &look_player) {
        return None;
    }
    let scry = scry_effect.downcast_ref::<crate::effects::ScryEffect>()?;
    if scry.player != PlayerFilter::You {
        return None;
    }
    Some(format!(
        "{reveal_text}. You choose {choice_from_it}. That player discards that card. Scry {}",
        describe_value(&scry.count)
    ))
}

pub(in crate::compiled_text) fn describe_reveal_hand_choose_discard_then_adventure_move(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, choose_effect, discard_effect, may_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;
    let choice_from_it = card_choice_from_it_text(&choice_text);
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !discard_discards_chosen_card(discard, choose, &look_player) {
        return None;
    }

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You) {
        return None;
    }
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_effect = move_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(move_effect);
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard || move_to_zone.enters_tapped {
        return None;
    }
    let ChooseSpec::WithCount(spec, count) = &move_to_zone.target else {
        return None;
    };
    if !count.is_single() {
        return None;
    }
    let ChooseSpec::Object(filter) = spec.as_ref() else {
        return None;
    };
    if filter.zone != Some(Zone::Exile)
        || !filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look_player))
        || filter.subtypes != vec![Subtype::Adventure]
    {
        return None;
    }

    Some(format!(
        "{reveal_text}. You choose {choice_from_it}. That player discards that card. You may put a card that has an Adventure that player owns from exile into that player's graveyard"
    ))
}

pub(super) fn describe_reveal_hand_choose_gain_toughness_then_discard(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, choose_effect, gain_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;
    let choice_from_it = card_choice_from_it_text(&choice_text);
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let Value::ToughnessOf(spec) = &gain.amount else {
        return None;
    };
    let ChooseSpec::Object(filter) = spec.as_ref() else {
        return None;
    };
    if !object_filter_has_tag(filter, &choose.tag) {
        return None;
    }
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !discard_discards_chosen_card(discard, choose, &look_player) {
        return None;
    }

    Some(format!(
        "{reveal_text}. You choose {choice_from_it}. You gain life equal to that creature card's toughness, then that player discards that card"
    ))
}

pub(in crate::compiled_text) fn describe_reveal_hand_choose_graveyard_or_hand_exile(
    effects: &[&Effect],
) -> Option<String> {
    let (look_effect, choose_effect, move_effect, trailing_effect) = match effects {
        [look_effect, choose_effect, move_effect] => {
            (*look_effect, *choose_effect, *move_effect, None)
        }
        [look_effect, choose_effect, move_effect, trailing_effect] => (
            *look_effect,
            *choose_effect,
            *move_effect,
            Some(*trailing_effect),
        ),
        _ => return None,
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let mut text = if let Some((reveal_text, choice_text, _)) =
        describe_reveal_hand_choose_from_it_or_graveyard(look, choose)
    {
        format!("{reveal_text}. You choose {choice_text}. Exile that card")
    } else {
        if choose.is_search
            || choose.chooser != PlayerFilter::You
            || choose_exact_count(choose) != Some(1)
            || choose_primary_zone(choose) != Some(Zone::Graveyard)
            || choose.additional_zones != vec![Zone::Hand]
            || choose
                .filter
                .owner
                .as_ref()
                .is_none_or(|owner| !player_filters_refer_to_same_player(owner, &look_player))
            || choose.filter.excluded_card_types != vec![CardType::Land]
        {
            return None;
        }
        format!(
            "{} {} their hand. You choose a nonland card from that player's graveyard or hand and exile it",
            capitalize_first(&revealer),
            reveal_verb
        )
    };
    if let Some(trailing_effect) = trailing_effect {
        if let Some(lose) = trailing_effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
            if lose.player != ChooseSpec::Player(PlayerFilter::You) {
                return None;
            }
            text.push_str(&format!(
                ". You lose {}",
                describe_life_amount_phrase(&lose.amount)
            ));
        } else {
            let grant = trailing_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
            if grant.tag.as_str() != "__source_exiled__"
                || grant.player != PlayerFilter::You
                || grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
                || grant.allow_land
                || !grant.allow_any_color_for_cast
                || grant.while_on_top_of_library
                || grant.filter.is_some()
                || grant.cast_pool_is_plural
            {
                return None;
            }
            text.push_str(". ");
            text.push_str(&capitalize_first(
                describe_effect(trailing_effect).trim_end_matches('.'),
            ));
        }
    }
    Some(text)
}
