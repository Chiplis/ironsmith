//! Deal damage effect implementation.
//!
//! This module implements the `DealDamage` effect, which deals damage to a target
//! creature, planeswalker, or player.

use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_player_from_spec, resolve_players_from_spec, resolve_value,
};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::LifeLossEvent;
use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
use crate::events::processing::{
    ProcessedDamageResult, SimultaneousDamageEvent,
    process_damage_assignments_with_event_with_source_snapshot_opts_with_dm,
    process_simultaneous_damage_assignments_with_event_with_dm,
};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};
use crate::triggers::AttackEventTarget;
use crate::triggers::TriggerEvent;
use crate::types::CardType;
pub use ironsmith_core::DealDamageEffect;

/// Effect that deals damage to a target creature, planeswalker, or player.
///
/// # Fields
///
/// * `amount` - The amount of damage to deal (can be fixed or variable)
/// * `target` - The target specification (creature, player, or "any target")
/// * `source_is_combat` - Whether this damage is combat damage
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to any target (Lightning Bolt)
/// let effect = DealDamageEffect {
///     amount: Value::Fixed(3),
///     target: ChooseSpec::AnyTarget,
///     source_is_combat: false,
/// };
/// ```
pub(crate) fn apply_processed_damage_outcome(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    initial_target: DamageTarget,
    amount: u32,
    source_is_combat: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> EffectOutcome {
    apply_processed_damage_outcome_opts(
        game,
        source,
        source_snapshot,
        initial_target,
        amount,
        source_is_combat,
        false,
        provenance,
        cause,
        dm,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_processed_damage_outcome_opts(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    initial_target: DamageTarget,
    amount: u32,
    source_is_combat: bool,
    unpreventable: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> EffectOutcome {
    let processed = process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
        game,
        source,
        initial_target,
        amount,
        source_is_combat,
        unpreventable,
        cause.clone(),
        source_snapshot,
        dm,
    );

    apply_processed_damage_results(
        game,
        source,
        source_snapshot,
        std::iter::once(processed),
        None,
        source_is_combat,
        provenance,
        cause,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_simultaneous_damage_outcome_opts(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    initial_targets: Vec<DamageTarget>,
    amount: u32,
    source_is_combat: bool,
    unpreventable: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> EffectOutcome {
    let events = initial_targets
        .into_iter()
        .map(|target| SimultaneousDamageEvent {
            source,
            target,
            amount,
            is_combat: source_is_combat,
            unpreventable,
            cause: cause.clone(),
            source_snapshot: source_snapshot.cloned(),
        })
        .collect::<Vec<_>>();
    let processed = process_simultaneous_damage_assignments_with_event_with_dm(game, &events, dm);
    let simultaneous_batch =
        game.alloc_child_event_provenance(provenance, crate::events::EventKind::Damage);

    apply_processed_damage_results(
        game,
        source,
        source_snapshot,
        processed,
        Some(simultaneous_batch),
        source_is_combat,
        provenance,
        cause,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_processed_damage_results(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    processed_results: impl IntoIterator<Item = ProcessedDamageResult>,
    simultaneous_batch: Option<crate::provenance::ProvNodeId>,
    source_is_combat: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
) -> EffectOutcome {
    let source_controller = source_snapshot
        .map(|snapshot| snapshot.controller)
        .or_else(|| game.object(source).map(|obj| game.controller_of(obj)));

    let keywords = crate::rules::damage::source_damage_keywords(game, source, source_snapshot);
    let mut outcomes = Vec::new();
    let mut total_damage_dealt = 0u32;
    let mut affected_objects = Vec::new();
    let mut any_replacement_prevented = false;
    for processed in processed_results {
        any_replacement_prevented |= processed.replacement_prevented;
        for assignment in processed.assignments {
            let target_snapshot = match assignment.target {
                DamageTarget::Object(object_id) => game.object(object_id).map(|obj| {
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        obj, game,
                    )
                }),
                DamageTarget::Player(_) => None,
            };
            let excess_damage = match assignment.target {
                DamageTarget::Object(object_id) => {
                    excess_damage_to_object(game, object_id, assignment.amount, keywords)
                }
                DamageTarget::Player(_) => 0,
            };
            let applied = crate::rules::damage::apply_processed_damage_assignment(
                game,
                source,
                assignment.target,
                assignment.amount,
                keywords,
                cause.clone(),
            );
            if !applied.applied {
                continue;
            }

            total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
            if let DamageTarget::Object(object_id) = assignment.target {
                affected_objects.push(object_id);
            }
            let mut outcome = EffectOutcome::count(assignment.amount as i32);
            if excess_damage > 0 {
                outcome = outcome
                    .with_execution_fact(ExecutionFact::ExcessDamageDealt)
                    .with_execution_fact(ExecutionFact::ExcessDamage(excess_damage));
            }
            if assignment.amount > 0 {
                let mut damage_event = DamageEvent::with_cause(
                    source,
                    assignment.target,
                    assignment.amount,
                    source_is_combat,
                    cause.clone(),
                );
                if let Some(snapshot) = target_snapshot {
                    damage_event = damage_event.with_target_snapshot(snapshot);
                }
                let mut event = TriggerEvent::new_with_provenance(damage_event, provenance);
                if let Some(batch) = simultaneous_batch {
                    event = event.with_simultaneous_batch(batch);
                }
                if game.object(source).is_none()
                    && let Some(snapshot) = source_snapshot
                {
                    event = event.with_source_snapshot(snapshot.clone());
                }
                outcome = outcome.with_event(event);
            }

            if let DamageTarget::Player(player_id) = assignment.target
                && applied.life_lost > 0
            {
                let mut event = TriggerEvent::new_with_provenance(
                    LifeLossEvent::new(player_id, applied.life_lost, true),
                    provenance,
                );
                if let Some(batch) = simultaneous_batch {
                    event = event.with_simultaneous_batch(batch);
                }
                outcome = outcome.with_event(event);
            }

            outcomes.push(outcome);
        }
    }

    if keywords.has_lifelink
        && total_damage_dealt > 0
        && let Some(controller) = source_controller
    {
        let life_to_gain = crate::events::processing::process_life_gain_with_event(
            game,
            controller,
            total_damage_dealt,
        );
        if life_to_gain > 0 {
            game.gain_life(controller, life_to_gain);
        }
    }

    let mut outcome = if outcomes.is_empty() && any_replacement_prevented {
        EffectOutcome::prevented()
    } else if outcomes.is_empty() {
        EffectOutcome::count(0)
    } else {
        EffectOutcome::aggregate_summing_counts(outcomes)
    };
    if !affected_objects.is_empty() {
        outcome = outcome.with_affected_objects_from_game(game, affected_objects);
    }
    outcome
}

fn excess_damage_to_object(
    game: &GameState,
    target: crate::ids::ObjectId,
    amount: u32,
    keywords: crate::rules::damage::SourceDamageKeywords,
) -> u32 {
    if amount == 0 {
        return 0;
    }
    let Some(object) = game.object(target) else {
        return 0;
    };
    if object.has_card_type(CardType::Creature) {
        let lethal = if keywords.has_deathtouch {
            1
        } else {
            let Some(toughness) = game
                .calculated_toughness(target)
                .or_else(|| object.toughness())
            else {
                return 0;
            };
            (toughness - game.damage_on(target) as i32).max(0) as u32
        };
        return amount.saturating_sub(lethal);
    }
    0
}

fn object_can_be_dealt_damage(object: &crate::object::Object) -> bool {
    object.has_card_type(CardType::Creature)
        || object.has_card_type(CardType::Planeswalker)
        || object.has_card_type(CardType::Battle)
}

impl EffectExecutor for DealDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        // Check if this is targeting IteratedPlayer (used in ForEachOpponent)
        // If so, resolve the target from the context's iterated_player
        if let ChooseSpec::Player(PlayerFilter::IteratedPlayer) = &self.target {
            if let Some(player_id) = ctx.iteration.iterated_player {
                return Ok(apply_processed_damage_outcome_opts(
                    game,
                    ctx.source,
                    ctx.source_snapshot.as_ref(),
                    DamageTarget::Player(player_id),
                    amount,
                    self.source_is_combat,
                    self.unpreventable,
                    ctx.provenance,
                    ctx.cause.clone(),
                    &mut *ctx.decision_maker,
                ));
            }
            return Ok(EffectOutcome::target_invalid());
        }

        if let ChooseSpec::Iterated = &self.target {
            if let Some(object_id) = ctx.iteration.iterated_object {
                if let Some(obj) = game.object(object_id) {
                    if !object_can_be_dealt_damage(obj) {
                        return Ok(EffectOutcome::target_invalid());
                    }
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &mut *ctx.decision_maker,
                    ));
                }
                return Ok(EffectOutcome::target_invalid());
            }
            return Ok(EffectOutcome::target_invalid());
        }

        if let ChooseSpec::AttackedPlayerOrPlaneswalker = &self.target {
            let attacked_target = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| {
                    if let Some(attacked) = event.downcast::<CreatureAttackedEvent>() {
                        return Some(attacked.target);
                    }
                    if let Some(blocked) = event.downcast::<CreatureBecameBlockedEvent>() {
                        return blocked.attack_target;
                    }
                    None
                })
                .or_else(|| ctx.combat.defending_player.map(AttackEventTarget::Player));

            let Some(attacked_target) = attacked_target else {
                return Ok(EffectOutcome::target_invalid());
            };

            match attacked_target {
                AttackEventTarget::Player(player_id) => {
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Player(player_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &mut *ctx.decision_maker,
                    ));
                }
                AttackEventTarget::Planeswalker(object_id) => {
                    if !game
                        .object(object_id)
                        .is_some_and(|obj| obj.has_card_type(CardType::Planeswalker))
                    {
                        return Ok(EffectOutcome::target_invalid());
                    }
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &mut *ctx.decision_maker,
                    ));
                }
                AttackEventTarget::Battle(object_id) => {
                    if !game
                        .object(object_id)
                        .is_some_and(|obj| obj.has_card_type(CardType::Battle))
                    {
                        return Ok(EffectOutcome::target_invalid());
                    }
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &mut *ctx.decision_maker,
                    ));
                }
            }
        }

        // Handle SourceController - deal damage to the controller of the source (e.g., Ancient Tomb)
        if let ChooseSpec::SourceController = &self.target {
            let controller = ctx.controller;
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(controller),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ));
        }

        // "Each player" is one simultaneous damage event per matching player,
        // not a request to resolve the player filter to a single representative.
        // This distinction is observable in shared-life variants (CR 810.9).
        if matches!(self.target.base(), ChooseSpec::EachPlayer(_)) {
            let damage_targets = resolve_players_from_spec(game, &self.target, ctx)?
                .into_iter()
                .map(DamageTarget::Player)
                .collect::<Vec<_>>();
            if damage_targets.is_empty() {
                return Ok(EffectOutcome::count(0));
            }
            return Ok(apply_simultaneous_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                damage_targets,
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ));
        }

        if matches!(
            self.target.base(),
            ChooseSpec::AnyTarget
                | ChooseSpec::AnyOtherTarget
                | ChooseSpec::ObjectOrPlayer(_, _)
                | ChooseSpec::PlayerOrPlaneswalker(_)
        ) {
            let mut found_assignment = false;
            let mut damage_targets = Vec::new();
            for assignment in &ctx.target_assignments {
                if assignment.spec == self.target || assignment.spec.base() == self.target.base() {
                    found_assignment = true;
                    let Some(assigned_targets) = ctx.targets.get(assignment.range.clone()) else {
                        continue;
                    };
                    for target in assigned_targets {
                        match target {
                            ResolvedTarget::Player(player_id) => {
                                if !game
                                    .player(*player_id)
                                    .is_some_and(|player| player.is_in_game())
                                {
                                    continue;
                                }
                                damage_targets.push(DamageTarget::Player(*player_id));
                            }
                            ResolvedTarget::Object(object_id) => {
                                if !game
                                    .object(*object_id)
                                    .is_some_and(object_can_be_dealt_damage)
                                {
                                    continue;
                                }
                                damage_targets.push(DamageTarget::Object(*object_id));
                            }
                        }
                    }
                    // This effect owns one announced target requirement. A later
                    // requirement may have the same surface `ChooseSpec`, but it
                    // belongs to a different effect and must not be consumed here.
                    break;
                }
            }
            if found_assignment {
                return if damage_targets.is_empty() {
                    Ok(EffectOutcome::target_invalid())
                } else {
                    Ok(apply_simultaneous_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        damage_targets,
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &mut *ctx.decision_maker,
                    ))
                };
            }
        }

        let controller_of_tagged = match &self.target {
            ChooseSpec::Player(
                PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag)),
            ) => Some(tag),
            ChooseSpec::Target(inner) => match inner.as_ref() {
                ChooseSpec::Player(
                    PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
                    | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag)),
                ) => Some(tag),
                _ => None,
            },
            _ => None,
        };
        if let Some(tag) = controller_of_tagged {
            let controller = ctx
                .get_tagged(tag)
                .map(|snapshot| snapshot.controller)
                .or_else(|| {
                    ctx.triggering_event
                        .as_ref()
                        .and_then(|event| event.snapshot())
                        .map(|snapshot| snapshot.controller)
                });
            let Some(controller) = controller else {
                return Ok(EffectOutcome::target_invalid());
            };
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(controller),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ));
        }

        let controller_of_specific = match &self.target {
            ChooseSpec::Player(
                PlayerFilter::ControllerOf(ObjectRef::Specific(id))
                | PlayerFilter::AliasedControllerOf(ObjectRef::Specific(id)),
            ) => Some(*id),
            _ => None,
        };
        if let Some(object_id) = controller_of_specific {
            let controller = game
                .object(object_id)
                .map(|object| game.controller_of(object))
                .or_else(|| {
                    ctx.target_snapshots
                        .get(&object_id)
                        .map(|snapshot| snapshot.controller)
                })
                .or_else(|| {
                    ctx.triggering_event
                        .as_ref()
                        .and_then(|event| event.downcast::<DamageEvent>())
                        .filter(|event| event.source == object_id)
                        .and_then(|event| {
                            event.cause.source_controller.or_else(|| {
                                game.object(object_id)
                                    .map(|object| game.controller_of(object))
                            })
                        })
                });
            let Some(controller) = controller else {
                return Ok(EffectOutcome::target_invalid());
            };
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(controller),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ));
        }

        if matches!(
            self.target,
            ChooseSpec::Player(_)
                | ChooseSpec::PlayerOrPlaneswalker(_)
                | ChooseSpec::SourceOwner
                | ChooseSpec::SpecificPlayer(_)
                | ChooseSpec::EachPlayer(_)
        ) && let Ok(player_id) = resolve_player_from_spec(game, &self.target, ctx)
        {
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(player_id),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ));
        }

        if let Ok(object_ids) = resolve_objects_for_effect(game, ctx, &self.target)
            && let Some(object_id) = object_ids.into_iter().find(|object_id| {
                game.object(*object_id).is_some_and(|obj| {
                    obj.zone == crate::zone::Zone::Battlefield && object_can_be_dealt_damage(obj)
                })
            })
        {
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Object(object_id),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ));
        }

        // Otherwise, use pre-resolved targets from ctx.targets
        for target in &ctx.targets {
            match target {
                ResolvedTarget::Player(player_id) => {
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Player(*player_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &mut *ctx.decision_maker,
                    ));
                }
                ResolvedTarget::Object(object_id) => {
                    if let Some(obj) = game.object(*object_id) {
                        if !object_can_be_dealt_damage(obj) {
                            continue;
                        }
                        return Ok(apply_processed_damage_outcome_opts(
                            game,
                            ctx.source,
                            ctx.source_snapshot.as_ref(),
                            DamageTarget::Object(*object_id),
                            amount,
                            self.source_is_combat,
                            self.unpreventable,
                            ctx.provenance,
                            ctx.cause.clone(),
                            &mut *ctx.decision_maker,
                        ));
                    }
                }
            }
        }

        Ok(EffectOutcome::target_invalid())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        // SourceController is deterministic at resolution time (no cast-time selection),
        // but exposing it here keeps downstream wrappers/tests able to inspect
        // what subject this damage effect is bound to.
        if self.target.is_target() || matches!(self.target, ChooseSpec::SourceController) {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target for damage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::cause::CauseFilter;
    use crate::events::counters::matchers::WouldPutCountersMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        power: i32,
        toughness: i32,
        controller: PlayerId,
        abilities: Vec<StaticAbility>,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        for ability in abilities {
            obj.abilities_mut().push(Ability::static_ability(ability));
        }
        game.add_object(obj);
        id
    }

    fn add_doubling_season_like_effect(
        game: &mut GameState,
        controller: PlayerId,
        target: crate::ids::ObjectId,
    ) {
        let source = game.new_object_id();
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                controller,
                WouldPutCountersMatcher::new(
                    ObjectFilter::specific(target),
                    Some(CounterType::MinusOneMinusOne),
                )
                .with_cause_filter(CauseFilter::from_effect()),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );
    }

    fn create_player_aura_attached_to(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        player: PlayerId,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![crate::types::Subtype::Aura])
            .build();
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.attached_to = Some(crate::object::AttachmentTarget::Player(player));
        game.add_object(obj);
        game.player_mut(player)
            .expect("attached player should exist")
            .attachments
            .push(id);
        id
    }

    #[test]
    fn damage_effect_uses_affected_players_tied_replacement_choice() {
        struct ChooseLastReplacement {
            expected_player: PlayerId,
        }

        impl crate::decision::DecisionMaker for ChooseLastReplacement {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.expected_player);
                ctx.options
                    .iter()
                    .rev()
                    .find(|option| option.legal)
                    .map(|option| vec![option.index])
                    .unwrap_or_default()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let add_source = create_creature(&mut game, "Add Replacement", 1, 1, alice, vec![]);
        let double_source = create_creature(&mut game, "Double Replacement", 1, 1, alice, vec![]);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                add_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Add(1)),
            ),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                double_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );

        let mut dm = ChooseLastReplacement {
            expected_player: bob,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let outcome = DealDamageEffect::new(3, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve through both replacements");

        assert_eq!(outcome.count_or_zero(), 7);
        assert_eq!(game.player(bob).expect("bob should exist").life, 13);
    }

    #[test]
    fn damage_to_object_records_affected_object_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let target = create_creature(&mut game, "Target", 2, 2, bob, vec![]);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let outcome = DealDamageEffect::new(1, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve");

        assert_eq!(outcome.affected_objects(), Some([target].as_slice()));
        let memory = outcome
            .affected_object_memory()
            .expect("damaged object memory should be recorded");
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].controller, bob);
        assert_eq!(memory[0].toughness, Some(2));
    }

    #[test]
    fn damage_to_object_records_numeric_excess_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Overkiller", 5, 5, alice, vec![]);
        let target = create_creature(&mut game, "Small Target", 2, 2, bob, vec![]);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let outcome = DealDamageEffect::new(5, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve");

        assert!(
            outcome
                .execution_facts()
                .contains(&ExecutionFact::ExcessDamageDealt)
        );
        assert!(
            outcome
                .execution_facts()
                .contains(&ExecutionFact::ExcessDamage(3))
        );
    }

    #[test]
    fn damage_to_tagged_enchanted_player_resolves_from_aura_attachment() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let aura = create_player_aura_attached_to(&mut game, "Curse", alice, bob);
        let mut ctx = ExecutionContext::new_default(aura, alice);

        let outcome = DealDamageEffect::new(
            6,
            ChooseSpec::Player(PlayerFilter::TaggedPlayer(crate::tag::TagKey::from(
                "enchanted",
            ))),
        )
        .execute(&mut game, &mut ctx)
        .expect("damage should resolve");

        assert_eq!(outcome.count_or_zero(), 6);
        assert_eq!(
            game.player(bob).expect("bob should exist").life,
            14,
            "damage should apply to the player enchanted by the Aura source"
        );
    }

    #[test]
    fn object_damage_resolves_matching_target_instead_of_first_raw_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let bounced = create_creature(&mut game, "Bounced", 2, 4, bob, vec![]);
        let damaged = create_creature(&mut game, "Damaged", 2, 2, bob, vec![]);
        game.move_object_by_effect(bounced, Zone::Hand);

        let mut filter = crate::filter::ObjectFilter::default();
        filter.zone = Some(Zone::Battlefield);
        filter.card_types = vec![CardType::Creature, CardType::Planeswalker];

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(bounced),
            ResolvedTarget::Object(damaged),
        ]);

        DealDamageEffect::new(2, ChooseSpec::target(ChooseSpec::Object(filter)))
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve");

        assert_eq!(game.damage_on(damaged), 2);
        assert_eq!(game.damage_on(bounced), 0);
    }

    #[test]
    fn counted_damage_assignment_hits_every_remaining_legal_target_as_one_batch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(
            &mut game,
            "Lifelink Volley Source",
            2,
            2,
            alice,
            vec![StaticAbility::lifelink()],
        );
        let first = create_creature(&mut game, "First Target", 2, 5, bob, vec![]);
        let illegal = create_creature(&mut game, "Illegal Target", 2, 5, bob, vec![]);
        game.move_object_by_effect(illegal, Zone::Hand);

        let counted_target = ChooseSpec::AnyTarget.with_count(crate::effect::ChoiceCount::up_to(3));
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![
                ResolvedTarget::Object(first),
                ResolvedTarget::Object(illegal),
                ResolvedTarget::Player(bob),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: counted_target.clone(),
                range: 0..3,
            }]);

        let outcome = DealDamageEffect::new(2, counted_target)
            .execute(&mut game, &mut ctx)
            .expect("every remaining legal target should be damaged");

        assert_eq!(game.damage_on(first), 2);
        assert_eq!(game.damage_on(illegal), 0);
        assert_eq!(game.player(bob).expect("Bob should exist").life, 18);
        assert_eq!(game.player(alice).expect("Alice should exist").life, 24);
        assert_eq!(outcome.count_or_zero(), 4);
        assert_eq!(outcome.affected_objects(), Some([first].as_slice()));
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| event.downcast::<DamageEvent>().is_some())
                .count(),
            2
        );
    }

    #[test]
    fn noncombat_infect_damage_to_creature_uses_effect_counter_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(
            &mut game,
            "Infector",
            1,
            1,
            alice,
            vec![StaticAbility::infect()],
        );
        let target = create_creature(&mut game, "Target", 2, 2, bob, vec![]);
        add_doubling_season_like_effect(&mut game, bob, target);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let effect = DealDamageEffect::new(1, ChooseSpec::AnyTarget);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("damage resolves");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), 2);
        assert_eq!(game.damage_on(target), 0);
    }

    #[test]
    fn noncombat_damage_replacement_from_your_source_puts_counters_on_opponent_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let _replacement_source = create_creature(
            &mut game,
            "Soul-Scar Stand-In",
            1,
            2,
            alice,
            vec![StaticAbility::replace_damage_with_counters_instead(
                CounterType::MinusOneMinusOne,
                ObjectFilter::default().controlled_by(PlayerFilter::You),
                ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
                Some(false),
                "If a source you control would deal noncombat damage to a creature an opponent controls, put that many -1/-1 counters on that creature instead.",
            )],
        );
        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let target = create_creature(&mut game, "Target", 2, 2, bob, vec![]);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        DealDamageEffect::new(2, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage replacement should resolve");

        assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), 2);
        assert_eq!(game.damage_on(target), 0);
    }

    #[test]
    fn damage_event_history_uses_source_lki_after_source_leaves() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Departed Pinger", 1, 1, alice, vec![]);
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source should exist"),
            &game,
        );
        game.remove_object(source);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)])
            .with_source_snapshot(source_snapshot);

        let effect = DealDamageEffect::new(1, ChooseSpec::AnyTarget);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("departed source should still deal damage from LKI");

        assert_eq!(outcome.events.len(), 2);
        game.record_turn_history_event(&outcome.events[0]);

        assert_eq!(
            game.turn_store
                .turn_history
                .total_creature_damage_to_player(bob),
            1,
            "damage history should treat the departed source as the creature it last was"
        );
    }
}
