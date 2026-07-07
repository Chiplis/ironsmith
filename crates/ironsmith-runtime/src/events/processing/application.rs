use super::TraitApplyResult;
use crate::events::{Event, EventKind};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;
use crate::object::CounterType;
use crate::replacement::{
    EventModification, RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect,
};
use crate::zone::Zone;

pub(super) fn apply_trait_replacement(
    game: &GameState,
    event: Event,
    effect: &ReplacementEffect,
) -> TraitApplyResult {
    match &effect.replacement {
        ReplacementAction::Prevent => TraitApplyResult::Prevented,

        ReplacementAction::Skip => TraitApplyResult::Prevented,

        ReplacementAction::Instead(effects) => TraitApplyResult::Replaced(effects.clone()),

        ReplacementAction::Modify(modification) => {
            let modified = apply_trait_modification(game, &event, modification, effect);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::Double => {
            let modified = apply_trait_double(&event);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::DoubleCounters { counter_type } => {
            let modified = apply_trait_double_counters(&event, *counter_type);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::AddCountersToPlacement {
            counter_type,
            additional,
        } => {
            let modified =
                apply_trait_add_counters_to_placement(&event, *counter_type, *additional);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::ChangeDestination(new_zone) => {
            let modified = apply_trait_change_destination(&event, *new_zone);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::MoveToZoneWithCounters { .. } => TraitApplyResult::Replaced(Vec::new()),

        ReplacementAction::ExileWithSourceLink => TraitApplyResult::Replaced(Vec::new()),

        ReplacementAction::ExileWithSourceLinkThen(effects) => {
            TraitApplyResult::Replaced(effects.clone())
        }

        ReplacementAction::ExileWithSourceLinkCountersThen { effects, .. } => {
            TraitApplyResult::Replaced(effects.clone())
        }

        ReplacementAction::EnterTapped => {
            let modified = apply_trait_enter_tapped(&event);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::EnterUntapped => {
            let modified = apply_trait_enter_untapped(&event);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::EnterUnderControl(controller) => {
            let modified = apply_trait_enter_under_control(&event, *controller);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::EnterWithCounters {
            counter_type,
            count,
            added_subtypes,
            added_abilities,
        } => {
            let resolved_count = resolve_value_for_etb(count, game, effect.source);
            let modified = apply_trait_enter_with_counters(
                &event,
                *counter_type,
                resolved_count,
                added_subtypes,
                added_abilities,
            );
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::EnterWithCounterChoice { counter_types, .. } => {
            if counter_types.is_empty() {
                return TraitApplyResult::Unchanged(event);
            }
            TraitApplyResult::NeedsInteraction {
                decision_ctx: super::counter_choice_context(
                    game,
                    effect.source,
                    effect.controller,
                    counter_types,
                ),
                redirect_zone: Zone::Battlefield,
                effect_id: effect.id,
                object_id: effect.source,
                filter: None,
                destinations: None,
            }
        }

        ReplacementAction::Tribute { count, .. } => {
            let opponents = super::tribute_opponents(game, effect.controller);
            if opponents.is_empty() {
                return TraitApplyResult::Unchanged(event);
            };
            let decision_ctx = if opponents.len() == 1 {
                super::tribute_boolean_context(game, effect.source, opponents[0], *count)
            } else {
                super::tribute_opponent_choice_context(
                    game,
                    effect.source,
                    effect.controller,
                    &opponents,
                )
            };
            TraitApplyResult::NeedsInteraction {
                decision_ctx,
                redirect_zone: Zone::Battlefield,
                effect_id: effect.id,
                object_id: effect.source,
                filter: None,
                destinations: None,
            }
        }

        ReplacementAction::Redirect { target, which } => {
            let modified = apply_trait_redirect(game, &event, target, which, effect.controller);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::RedirectDamageAmount {
            target,
            which,
            amount,
        } => {
            use crate::events::{DamageEvent, DamageTarget, downcast_event};

            if *amount == 0 {
                return TraitApplyResult::Unchanged(event);
            }

            let Some(damage) = downcast_event::<DamageEvent>(event.inner()) else {
                return TraitApplyResult::Unchanged(event);
            };
            let Some(new_target) = resolve_trait_redirect_target(
                game,
                event.inner(),
                target,
                which,
                effect.controller,
            ) else {
                return TraitApplyResult::Unchanged(event);
            };
            let redirected_target = match new_target {
                Target::Player(player_id) => DamageTarget::Player(player_id),
                Target::Object(object_id) => DamageTarget::Object(object_id),
            };

            let redirected_amount = (*amount).min(damage.amount);
            if redirected_amount == 0 {
                return TraitApplyResult::Unchanged(event);
            }

            let mut modified = damage
                .with_target(redirected_target)
                .with_amount(redirected_amount);
            if damage.amount > redirected_amount {
                modified =
                    modified.with_remainder(damage.target, damage.amount - redirected_amount);
            }
            TraitApplyResult::Modified(event.rewrap(modified))
        }

        ReplacementAction::Additionally(_effects) => TraitApplyResult::Modified(event),

        ReplacementAction::AddTokens { token, count } => {
            let modified = apply_trait_add_tokens(&event, *token, *count);
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::ReplaceMana(mana) => {
            use crate::events::{ManaAddedEvent, downcast_event};

            let Some(mana_event) = downcast_event::<ManaAddedEvent>(event.inner()) else {
                return TraitApplyResult::Unchanged(event);
            };
            let replacement_mana = if mana.len() == 1 {
                vec![mana[0].clone(); mana_event.mana.len()]
            } else {
                mana.clone()
            };
            TraitApplyResult::Modified(event.rewrap(mana_event.clone().with_mana(replacement_mana)))
        }

        ReplacementAction::EnterAsCopy {
            source,
            enters_tapped,
            linked_exile_objects,
            additional_counters,
            name_override,
            added_card_types,
            removed_supertypes,
            added_subtypes,
            added_abilities,
            set_base_power_toughness,
        } => {
            let modified = apply_trait_enter_as_copy(
                &event,
                *source,
                *enters_tapped,
                linked_exile_objects,
                additional_counters,
                name_override.clone(),
                added_card_types,
                removed_supertypes,
                added_subtypes,
                added_abilities,
                *set_base_power_toughness,
            );
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::EnterWithCharacteristics {
            added_card_types,
            added_subtypes,
            set_base_power_toughness,
        } => {
            let modified = apply_trait_enter_with_characteristics(
                &event,
                added_card_types,
                added_subtypes,
                *set_base_power_toughness,
            );
            match modified {
                Some(e) => TraitApplyResult::Modified(e),
                None => TraitApplyResult::Unchanged(event),
            }
        }

        ReplacementAction::InteractiveDiscardOrRedirect {
            filter,
            redirect_zone,
        } => {
            let controller = effect.controller;
            let matching_cards = find_matching_cards_in_hand(game, controller, filter);

            if matching_cards.is_empty() {
                let modified = apply_trait_change_destination(&event, *redirect_zone);
                match modified {
                    Some(e) => TraitApplyResult::Modified(e),
                    None => TraitApplyResult::Unchanged(event),
                }
            } else {
                let candidates: Vec<crate::decisions::context::SelectableObject> = matching_cards
                    .iter()
                    .map(|&id| {
                        let name = game
                            .object(id)
                            .map(|o| o.name.to_string())
                            .unwrap_or_else(|| "Unknown".to_string());
                        crate::decisions::context::SelectableObject::new(id, name)
                    })
                    .collect();
                let source_name = game
                    .object(effect.source)
                    .map(|o| o.name.to_string())
                    .unwrap_or_else(|| "permanent".to_string());
                let discard_phrase = describe_discard_filter_card_phrase(filter);
                let redirect_phrase = describe_redirect_zone_phrase(*redirect_zone);
                let decision_ctx = crate::decisions::context::DecisionContext::SelectObjects(
                    crate::decisions::context::SelectObjectsContext::new(
                        controller,
                        Some(effect.source),
                        format!(
                            "Discard {} to put {} onto the battlefield, or it goes to {}",
                            discard_phrase, source_name, redirect_phrase
                        ),
                        candidates,
                        1,
                        Some(1),
                    ),
                );
                TraitApplyResult::NeedsInteraction {
                    decision_ctx,
                    redirect_zone: *redirect_zone,
                    effect_id: effect.id,
                    object_id: effect.source,
                    filter: Some(filter.clone()),
                    destinations: None,
                }
            }
        }

        ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost } => {
            let controller = effect.controller;
            let can_pay = game
                .player(controller)
                .map(|p| p.life >= *life_cost as i32)
                .unwrap_or(false);

            if !can_pay {
                let modified = apply_trait_enter_tapped(&event);
                match modified {
                    Some(e) => TraitApplyResult::Modified(e),
                    None => TraitApplyResult::Unchanged(event),
                }
            } else {
                let source_name = game.object(effect.source).map(|o| o.name.to_string());
                let mut bool_ctx = crate::decisions::context::BooleanContext::new(
                    controller,
                    Some(effect.source),
                    format!("Pay {} life? (If you don't, this enters tapped)", life_cost),
                );
                if let Some(name) = source_name {
                    bool_ctx = bool_ctx.with_source_name(name);
                }
                let decision_ctx = crate::decisions::context::DecisionContext::Boolean(bool_ctx);
                TraitApplyResult::NeedsInteraction {
                    decision_ctx,
                    redirect_zone: Zone::Battlefield,
                    effect_id: effect.id,
                    object_id: effect.source,
                    filter: None,
                    destinations: None,
                }
            }
        }

        ReplacementAction::InteractiveChooseDestination {
            destinations,
            description,
        } => {
            let options: Vec<crate::decisions::context::SelectableOption> = destinations
                .iter()
                .enumerate()
                .map(|(idx, zone)| {
                    let zone_name = match zone {
                        Zone::Library => "Top of library",
                        Zone::Graveyard => "Graveyard",
                        Zone::Hand => "Hand",
                        Zone::Exile => "Exile",
                        Zone::Battlefield => "Battlefield",
                        Zone::Stack => "Stack",
                        Zone::Command => "Command zone",
                        Zone::OutsideGame => "Outside the game",
                    };
                    crate::decisions::context::SelectableOption::new(idx, zone_name.to_string())
                })
                .collect();

            let controller = effect.controller;
            let decision_ctx = crate::decisions::context::DecisionContext::SelectOptions(
                crate::decisions::context::SelectOptionsContext::new(
                    controller,
                    Some(effect.source),
                    description.clone(),
                    options,
                    1,
                    1,
                ),
            );

            let default_zone = destinations.first().copied().unwrap_or(Zone::Graveyard);

            TraitApplyResult::NeedsInteraction {
                decision_ctx,
                redirect_zone: default_zone,
                effect_id: effect.id,
                object_id: effect.source,
                filter: None,
                destinations: Some(destinations.clone()),
            }
        }
    }
}

fn apply_trait_enter_under_control(event: &Event, controller: PlayerId) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(event.rewrap(etb.with_controller_override(controller)))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                Some(
                    event.rewrap(
                        EnterBattlefieldEvent::new(*zone_change.objects.first()?, zone_change.from)
                            .with_controller_override(controller),
                    ),
                )
            } else {
                None
            }
        }
        _ => None,
    }
}

fn indefinite_article(text: &str) -> &'static str {
    let first = text
        .chars()
        .find(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase());
    match first {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn describe_discard_filter_card_phrase(filter: &crate::target::ObjectFilter) -> String {
    let mut phrase = filter.description().trim().to_string();
    if phrase.is_empty() {
        return "a card".to_string();
    }

    let lower = phrase.to_ascii_lowercase();
    let has_determiner = lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("target ")
        || lower.starts_with("another ")
        || lower.starts_with("any ")
        || lower.starts_with("each ");
    if !has_determiner {
        phrase = format!("{} {}", indefinite_article(&phrase), phrase);
    }

    let lower = phrase.to_ascii_lowercase();
    if !lower.contains(" card") && !lower.ends_with("card") {
        phrase.push_str(" card");
    }
    phrase
}

fn describe_redirect_zone_phrase(zone: Zone) -> &'static str {
    match zone {
        Zone::Graveyard => "its owner's graveyard",
        Zone::Hand => "its owner's hand",
        Zone::Library => "its owner's library",
        Zone::Battlefield => "the battlefield",
        Zone::Stack => "the stack",
        Zone::Exile => "exile",
        Zone::Command => "the command zone",
        Zone::OutsideGame => "outside the game",
    }
}

pub(super) fn find_matching_cards_in_hand(
    game: &GameState,
    controller: crate::ids::PlayerId,
    filter: &crate::target::ObjectFilter,
) -> Vec<crate::ids::ObjectId> {
    use crate::target::FilterContext;

    let filter_ctx = FilterContext::new(controller);
    game.player(controller)
        .map(|p| {
            p.hand
                .iter()
                .filter(|&&card_id| {
                    game.object(card_id)
                        .map(|obj| filter.matches(obj, &filter_ctx, game))
                        .unwrap_or(false)
                })
                .copied()
                .collect()
        })
        .unwrap_or_default()
}

fn apply_trait_add_tokens(
    event: &Event,
    token: ironsmith_core::AdditionalTokenKind,
    count: u32,
) -> Option<Event> {
    use crate::events::{CreateTokensEvent, downcast_event};

    if event.kind() != EventKind::CreateTokens || count == 0 {
        return None;
    }
    let create_tokens = downcast_event::<CreateTokensEvent>(event.inner())?;
    Some(event.rewrap(create_tokens.with_additional_tokens(token, count)))
}

fn apply_trait_modification(
    game: &GameState,
    event: &Event,
    modification: &EventModification,
    effect: &ReplacementEffect,
) -> Option<Event> {
    use crate::events::{
        CreateTokensEvent, DamageEvent, DrawEvent, LifeGainEvent, PutCountersEvent, downcast_event,
    };

    match event.kind() {
        EventKind::Damage => {
            let damage = downcast_event::<DamageEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    damage.with_amount(damage.amount.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    damage.with_amount((damage.amount as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => damage.reduced(*delta),
                EventModification::SetTo(value) => damage.with_amount(*value),
                EventModification::SetToAtLeast(value) => {
                    let floor = resolve_value_for_replacement(value, game, effect.source);
                    if damage.amount >= floor {
                        return None;
                    }
                    damage.with_amount(floor)
                }
                EventModification::ReduceToZero => damage.prevented(),
            };
            Some(event.rewrap(modified))
        }
        EventKind::LifeGain => {
            let life_gain = downcast_event::<LifeGainEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    life_gain.with_amount(life_gain.amount.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    life_gain.with_amount((life_gain.amount as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => {
                    life_gain.with_amount(life_gain.amount.saturating_sub(*delta))
                }
                EventModification::SetTo(value) => life_gain.with_amount(*value),
                EventModification::SetToAtLeast(value) => {
                    let floor = resolve_value_for_replacement(value, game, effect.source);
                    life_gain.with_amount(life_gain.amount.max(floor))
                }
                EventModification::ReduceToZero => life_gain.with_amount(0),
            };
            Some(event.rewrap(modified))
        }
        EventKind::PutCounters => {
            let put_counters = downcast_event::<PutCountersEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    put_counters.with_count(put_counters.count.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    put_counters.with_count((put_counters.count as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => {
                    put_counters.with_count(put_counters.count.saturating_sub(*delta))
                }
                EventModification::SetTo(value) => put_counters.with_count(*value),
                EventModification::SetToAtLeast(value) => {
                    let floor = resolve_value_for_replacement(value, game, effect.source);
                    put_counters.with_count(put_counters.count.max(floor))
                }
                EventModification::ReduceToZero => put_counters.with_count(0),
            };
            Some(event.rewrap(modified))
        }
        EventKind::CreateTokens => {
            let create_tokens = downcast_event::<CreateTokensEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    create_tokens.with_count(create_tokens.count.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    create_tokens.with_count((create_tokens.count as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => {
                    create_tokens.with_count(create_tokens.count.saturating_sub(*delta))
                }
                EventModification::SetTo(value) => create_tokens.with_count(*value),
                EventModification::SetToAtLeast(value) => {
                    let floor = resolve_value_for_replacement(value, game, effect.source);
                    create_tokens.with_count(create_tokens.count.max(floor))
                }
                EventModification::ReduceToZero => create_tokens.with_count(0),
            };
            Some(event.rewrap(modified))
        }
        EventKind::Draw => {
            let draw = downcast_event::<DrawEvent>(event.inner())?;
            let modified = match modification {
                EventModification::Multiply(factor) => {
                    draw.with_count(draw.count.saturating_mul(*factor))
                }
                EventModification::Add(delta) => {
                    draw.with_count((draw.count as i32 + delta).max(0) as u32)
                }
                EventModification::Subtract(delta) => {
                    draw.with_count(draw.count.saturating_sub(*delta))
                }
                EventModification::SetTo(value) => draw.with_count(*value),
                EventModification::SetToAtLeast(value) => {
                    let floor = resolve_value_for_replacement(value, game, effect.source);
                    draw.with_count(draw.count.max(floor))
                }
                EventModification::ReduceToZero => draw.with_count(0),
            };
            Some(event.rewrap(modified))
        }
        _ => None,
    }
}

fn apply_trait_double(event: &Event) -> Option<Event> {
    use crate::events::{
        CreateTokensEvent, DamageEvent, DrawEvent, LifeGainEvent, PutCountersEvent, downcast_event,
    };

    match event.kind() {
        EventKind::Damage => {
            let damage = downcast_event::<DamageEvent>(event.inner())?;
            Some(event.rewrap(damage.doubled()))
        }
        EventKind::LifeGain => {
            let life_gain = downcast_event::<LifeGainEvent>(event.inner())?;
            Some(event.rewrap(life_gain.doubled()))
        }
        EventKind::PutCounters => {
            let put_counters = downcast_event::<PutCountersEvent>(event.inner())?;
            Some(event.rewrap(put_counters.doubled()))
        }
        EventKind::CreateTokens => {
            let create_tokens = downcast_event::<CreateTokensEvent>(event.inner())?;
            Some(event.rewrap(create_tokens.doubled()))
        }
        EventKind::Draw => {
            let draw = downcast_event::<DrawEvent>(event.inner())?;
            Some(event.rewrap(draw.doubled()))
        }
        _ => None,
    }
}

fn apply_trait_double_counters(event: &Event, counter_type: Option<CounterType>) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, PutCountersEvent, downcast_event};

    match event.kind() {
        EventKind::PutCounters => {
            let put_counters = downcast_event::<PutCountersEvent>(event.inner())?;
            if counter_type.is_none_or(|ct| ct == put_counters.counter_type) {
                Some(event.rewrap(put_counters.doubled()))
            } else {
                None
            }
        }
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            let mut doubled = etb.clone();
            let mut changed = false;
            for (existing_type, count) in &mut doubled.enters_with_counters {
                if counter_type.is_none_or(|ct| ct == *existing_type) {
                    *count = count.saturating_mul(2);
                    changed = true;
                }
            }
            changed.then(|| event.rewrap(doubled))
        }
        _ => None,
    }
}

fn apply_trait_add_counters_to_placement(
    event: &Event,
    counter_type: Option<CounterType>,
    additional: u32,
) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, PutCountersEvent, downcast_event};

    match event.kind() {
        EventKind::PutCounters => {
            let put_counters = downcast_event::<PutCountersEvent>(event.inner())?;
            if counter_type.is_none_or(|ct| ct == put_counters.counter_type)
                && put_counters.count > 0
            {
                Some(event.rewrap(put_counters.with_additional(additional)))
            } else {
                None
            }
        }
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            let mut increased = etb.clone();
            let mut changed = false;
            for (existing_type, count) in &mut increased.enters_with_counters {
                if *count > 0 && counter_type.is_none_or(|ct| ct == *existing_type) {
                    *count = count.saturating_add(additional);
                    changed = true;
                }
            }
            changed.then(|| event.rewrap(increased))
        }
        _ => None,
    }
}

fn apply_trait_change_destination(event: &Event, new_zone: Zone) -> Option<Event> {
    use crate::events::{DiscardEvent, EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::Discard => {
            let discard = downcast_event::<DiscardEvent>(event.inner())?;
            Some(event.rewrap(discard.with_destination(new_zone)))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            Some(event.rewrap(zone_change.with_destination(new_zone)))
        }
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(event.rewrap(ZoneChangeEvent::with_cause(
                etb.object,
                etb.from,
                new_zone,
                crate::events::cause::EventCause::effect(),
                None,
            )))
        }
        _ => None,
    }
}

pub(super) fn apply_trait_enter_tapped(event: &Event) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(event.rewrap(etb.with_tapped()))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                Some(event.rewrap(EnterBattlefieldEvent::tapped(
                    *zone_change.objects.first()?,
                    zone_change.from,
                )))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn apply_trait_enter_untapped(event: &Event) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            let mut untapped = etb.clone();
            untapped.enters_tapped = false;
            Some(event.rewrap(untapped))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                Some(event.rewrap(EnterBattlefieldEvent::new(
                    *zone_change.objects.first()?,
                    zone_change.from,
                )))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn apply_trait_enter_with_counters(
    event: &Event,
    counter_type: CounterType,
    count: u32,
    added_subtypes: &[crate::types::Subtype],
    added_abilities: &[crate::ability::Ability],
) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(
                event.rewrap(
                    etb.with_counters(counter_type, count)
                        .with_added_subtypes(added_subtypes)
                        .with_added_abilities(added_abilities),
                ),
            )
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                Some(
                    event.rewrap(
                        EnterBattlefieldEvent::new(*zone_change.objects.first()?, zone_change.from)
                            .with_counters(counter_type, count)
                            .with_added_subtypes(added_subtypes)
                            .with_added_abilities(added_abilities),
                    ),
                )
            } else {
                None
            }
        }
        _ => None,
    }
}

fn apply_trait_enter_as_copy(
    event: &Event,
    source_id: crate::ids::ObjectId,
    enters_tapped: bool,
    linked_exile_objects: &[crate::ids::ObjectId],
    additional_counters: &[(CounterType, u32)],
    name_override: Option<String>,
    added_card_types: &[crate::types::CardType],
    removed_supertypes: &[crate::types::Supertype],
    added_subtypes: &[crate::types::Subtype],
    added_abilities: &[crate::ability::Ability],
    set_base_power_toughness: Option<(i32, i32)>,
) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    let apply_copy_modifiers = |mut etb: EnterBattlefieldEvent| {
        etb = etb
            .with_copy_of(source_id)
            .with_linked_exile_objects(linked_exile_objects)
            .with_copy_name_override(name_override.clone())
            .with_added_card_types(added_card_types)
            .with_removed_supertypes(removed_supertypes)
            .with_added_subtypes(added_subtypes)
            .with_added_abilities(added_abilities);
        if let Some((power, toughness)) = set_base_power_toughness {
            etb = etb.with_base_power_toughness(power, toughness);
        }
        for (counter_type, count) in additional_counters {
            etb = etb.with_counters(*counter_type, *count);
        }
        if enters_tapped {
            etb = etb.with_tapped();
        }
        etb
    };

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(event.rewrap(apply_copy_modifiers(etb.clone())))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                Some(
                    event.rewrap(apply_copy_modifiers(EnterBattlefieldEvent::new(
                        *zone_change.objects.first()?,
                        zone_change.from,
                    ))),
                )
            } else {
                None
            }
        }
        _ => None,
    }
}

fn apply_trait_enter_with_characteristics(
    event: &Event,
    added_card_types: &[crate::types::CardType],
    added_subtypes: &[crate::types::Subtype],
    set_base_power_toughness: Option<(i32, i32)>,
) -> Option<Event> {
    use crate::events::{EnterBattlefieldEvent, ZoneChangeEvent, downcast_event};

    let apply = |mut etb: EnterBattlefieldEvent| {
        etb = etb
            .with_added_card_types(added_card_types)
            .with_added_subtypes(added_subtypes);
        if let Some((power, toughness)) = set_base_power_toughness {
            etb = etb.with_base_power_toughness(power, toughness);
        }
        etb
    };

    match event.kind() {
        EventKind::EnterBattlefield => {
            let etb = downcast_event::<EnterBattlefieldEvent>(event.inner())?;
            Some(event.rewrap(apply(etb.clone())))
        }
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event.inner())?;
            if zone_change.to == Zone::Battlefield {
                Some(event.rewrap(apply(EnterBattlefieldEvent::new(
                    *zone_change.objects.first()?,
                    zone_change.from,
                ))))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn apply_trait_redirect(
    game: &GameState,
    event: &Event,
    redirect_target: &RedirectTarget,
    which: &RedirectWhich,
    effect_controller: PlayerId,
) -> Option<Event> {
    let new_target = resolve_trait_redirect_target(
        game,
        event.inner(),
        redirect_target,
        which,
        effect_controller,
    )?;
    let redirectable = event.inner().redirectable_targets();
    let selected = match which {
        RedirectWhich::First => redirectable.first(),
        RedirectWhich::Index(idx) => redirectable.get(*idx),
        RedirectWhich::ByDescription(desc) => redirectable.iter().find(|t| t.description == *desc),
    }?;
    let new_event_box = event
        .inner()
        .with_target_replaced(&selected.target, &new_target)?;
    Some(event.rewrap_boxed(new_event_box))
}

fn resolve_trait_redirect_target(
    game: &GameState,
    event: &dyn crate::events::traits::GameEventType,
    redirect_target: &RedirectTarget,
    which: &RedirectWhich,
    effect_controller: PlayerId,
) -> Option<Target> {
    let redirectable = event.redirectable_targets();
    let selected = match which {
        RedirectWhich::First => redirectable.first(),
        RedirectWhich::Index(idx) => redirectable.get(*idx),
        RedirectWhich::ByDescription(desc) => redirectable.iter().find(|t| t.description == *desc),
    }?;

    let new_target = match redirect_target {
        RedirectTarget::ToController => Target::Player(effect_controller),
        RedirectTarget::ToPlayer(player_id) => Target::Player(*player_id),
        RedirectTarget::ToObject(object_id) => Target::Object(*object_id),
        RedirectTarget::ToSource => Target::Object(event.source_object()?),
        RedirectTarget::ToSourceController => {
            let source = event.source_object()?;
            Target::Player(game.current_controller(source)?)
        }
    };

    if !selected.valid_redirect_types.is_valid(&new_target) {
        return None;
    }
    Some(new_target)
}

fn resolve_value_for_etb(
    count: &crate::effect::Value,
    game: &GameState,
    source: crate::ids::ObjectId,
) -> u32 {
    resolve_value_for_replacement(count, game, source)
}

pub(super) fn resolve_value_for_etb_for_choice(
    count: &crate::effect::Value,
    game: &GameState,
    source: crate::ids::ObjectId,
) -> u32 {
    resolve_value_for_etb(count, game, source)
}

fn resolve_value_for_replacement(
    count: &crate::effect::Value,
    game: &GameState,
    source: crate::ids::ObjectId,
) -> u32 {
    let controller = game
        .object(source)
        .map(|o| game.controller_of(o))
        .unwrap_or(crate::ids::PlayerId::from_index(0));

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, controller, &mut dm);

    if let Some(source_obj) = game.object(source) {
        ctx.optional_costs_paid = source_obj.optional_costs_paid.clone();
        if !source_obj.cast_tagged_objects.is_empty() {
            ctx = ctx.with_tagged_objects(source_obj.cast_tagged_objects.clone());
        }
    }

    crate::effects::helpers::resolve_value(game, count, &ctx)
        .unwrap_or(0)
        .max(0) as u32
}
