//! Keyword action event emission effect.
//!
//! Some rules text triggers on a keyword action (e.g., "when you cycle this card").
//! This effect provides a generic way to emit a KeywordActionEvent as part of an
//! effect/cost pipeline so triggers can observe it.

use std::collections::HashMap;

use crate::card::LinkedFaceLayout;
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{
    TraitEventResult, process_trait_event_with_dm_and_applied_effects,
};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::object::ObjectKind;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::EmitKeywordActionEffect;

use super::mechanic_actions::execute_keyword_action_replacement_effects;

fn snapshot_from_memory(game: &GameState, memory: &OutcomeObjectMemory) -> ObjectSnapshot {
    let mut snapshot = game
        .object(memory.object_id)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .unwrap_or_else(|| ObjectSnapshot {
            chosen_subtype: None,
            secret_chosen_subtype: None,
            chosen_object: None,
            object_id: memory.object_id,
            stable_id: memory.stable_id,
            kind: if memory.is_token {
                ObjectKind::Token
            } else {
                ObjectKind::Card
            },
            card: None,
            controller: memory.controller,
            owner: memory.owner,
            name: String::new(),
            first_printed_set_name: None,
            mana_cost: None,
            colors: memory.colors,
            supertypes: Vec::new(),
            card_types: memory.card_types.clone(),
            subtypes: memory.subtypes.clone(),
            compiled_card_text: String::new(),
            ability_labels: Vec::new(),
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            linked_face_mana_value: None,
            power: memory.power,
            toughness: memory.toughness,
            base_power: memory.power,
            base_toughness: memory.toughness,
            loyalty: None,
            defense: None,
            abilities: std::sync::Arc::new(Vec::new()),
            aura_attach_filter: None,
            copiable_values: crate::snapshot::CopiableValues::default(),
            x_value: None,
            cast_order_this_turn: None,
            mana_spent_to_cast: crate::player::ManaPool::default(),
            snow_mana_spent_to_cast: crate::player::ManaPool::default(),
            mana_sources_spent_to_cast: Vec::new(),
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            counters: std::collections::BTreeMap::new(),
            is_token: memory.is_token,
            tapped: false,
            attacking: false,
            goaded: None,
            flipped: false,
            face_down: false,
            transform_count: 0,
            attached_to: None,
            attachments: Vec::new(),
        attachment_snapshots: Vec::new(),
            was_enchanted: false,
            is_monstrous: false,
            is_prepared: false,
            is_commander: false,
            zone: memory.zone,
        });

    snapshot.stable_id = memory.stable_id;
    snapshot.controller = memory.controller;
    snapshot.owner = memory.owner;
    snapshot.zone = memory.zone;
    snapshot.power = memory.power;
    snapshot.toughness = memory.toughness;
    snapshot.card_types = memory.card_types.clone();
    snapshot.colors = memory.colors;
    snapshot.subtypes = memory.subtypes.clone();
    snapshot.is_token = memory.is_token;
    snapshot
}

fn object_tags_from_config(
    effect: &EmitKeywordActionEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<HashMap<TagKey, Vec<ObjectSnapshot>>, ExecutionError> {
    let mut tags: HashMap<TagKey, Vec<ObjectSnapshot>> = HashMap::new();
    for config in &effect.object_tags {
        let outcome = ctx
            .get_outcome(config.effect_id)
            .ok_or(ExecutionError::EffectNotFound(config.effect_id))?;
        let memories = if config.use_affected_memory {
            outcome.affected_object_memory()
        } else {
            outcome.chosen_object_memory()
        };
        let Some(memories) = memories else {
            continue;
        };
        let snapshots = memories
            .iter()
            .map(|memory| snapshot_from_memory(game, memory))
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            tags.entry(config.tag.clone())
                .or_default()
                .extend(snapshots);
        }
    }
    Ok(tags)
}

fn forage_payments(exclude_source: bool) -> [crate::effect::Effect; 2] {
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    let mut graveyard = ObjectFilter::default().owned_by(PlayerFilter::You).in_zone(crate::zone::Zone::Graveyard);
    graveyard.other = exclude_source;
    [
        crate::effect::Effect::new(crate::effects::ExileEffect::with_spec(
            ChooseSpec::Object(graveyard).with_count(crate::effect::ChoiceCount::exactly(3)))),
        crate::effect::Effect::new(crate::effects::SacrificeEffect::you(
            ObjectFilter::default().with_subtype(crate::types::Subtype::Food), 1)),
    ]
}

impl EffectExecutor for EmitKeywordActionEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.action == KeywordActionKind::Forage {
            let mut outcomes = Vec::new();
            for _ in 0..self.amount {
                let payments = forage_payments(false);
                let options: Vec<_> = payments.iter().enumerate().filter(|(_, effect)|
                    effect.0.can_execute_as_cost(game, ctx.source, ctx.controller).is_ok())
                    .map(|(index, _)| (if index == 0 { "Exile three cards from your graveyard" } else { "Sacrifice a Food" }.to_string(), index)).collect();
                if options.is_empty() { return Err(ExecutionError::Impossible("cannot forage".into())); }
                let choice = crate::decisions::ask_choose_one(game, &mut ctx.decision_maker, ctx.controller, ctx.source, &options);
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                let index = choice.unwrap_or(options[0].1);
                let outcome = crate::effects::execute_effect(game, &payments[index], ctx)?;
                if ctx.decision_maker.awaiting_choice() { return Ok(outcome); }
                if outcome.status.is_failure() { return Ok(outcome); }
                outcomes.push(outcome);
                outcomes.push(EffectOutcome::count(1).with_event(TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(self.action, ctx.controller, ctx.source, 1), ctx.provenance)));
            }
            let mut outcome = EffectOutcome::aggregate(outcomes);
            outcome.value = crate::effect::OutcomeValue::Count(self.amount as i32);
            return Ok(outcome);
        }
        if self.action == KeywordActionKind::AssembleContraption {
            // CR 701.45a deliberately does not define the Unstable Contraption
            // procedure. Keep the action typed and observable for an external
            // profile, but never pretend ordinary CR-only play can execute it.
            return Err(ExecutionError::ExternalRulesProfileRequired {
                action: "assembling a Contraption",
                specification: "the Unstable FAQ",
            });
        }
        if self.action == KeywordActionKind::Planeswalk {
            if let Some(planar_roll) = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::other::DieRolledEvent>())
                .filter(|event| event.is_planar)
                && !game.is_face_up_planar_object(planar_roll.source)
            {
                // CR 901.9a: this sourceless ability leaves the plane that was
                // face up when the die was rolled. If that plane has already
                // left the planar zone, the ability does nothing on resolution.
                return Ok(EffectOutcome::count(0));
            }
            let mut outcomes = Vec::with_capacity(self.amount as usize);
            for _ in 0..self.amount {
                let would_event = Event::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Planeswalk,
                        ctx.controller,
                        ctx.source,
                        1,
                    ),
                    ctx.provenance,
                );
                let applied_effects = ctx.replacement.suppressed_replacement_effects.clone();
                let applied_effect_keys =
                    ctx.replacement.suppressed_replacement_effect_keys.clone();
                if applied_effects.is_empty() && applied_effect_keys.is_empty() {
                    game.update_replacement_effects();
                }
                match process_trait_event_with_dm_and_applied_effects(
                    game,
                    would_event,
                    ctx.decision_maker,
                    &applied_effects,
                    &applied_effect_keys,
                ) {
                    TraitEventResult::Replaced {
                        effects, effect_id, ..
                    } => outcomes.push(execute_keyword_action_replacement_effects(
                        game, ctx, effects, effect_id, None,
                    )?),
                    TraitEventResult::Prevented => {}
                    TraitEventResult::NeedsChoice { .. }
                    | TraitEventResult::NeedsInteraction { .. } => {
                        return Ok(EffectOutcome::aggregate_summing_counts(outcomes));
                    }
                    TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                        let action =
                            crate::events::downcast_event::<KeywordActionEvent>(event.inner())
                                .ok_or_else(|| {
                                    ExecutionError::Impossible(
                                        "planeswalk replacement produced a non-planeswalk event"
                                            .to_string(),
                                    )
                                })?;
                        let destination = game
                            .planeswalk(action.player, action.source)
                            .map_err(ExecutionError::Impossible)?;
                        outcomes
                            .push(EffectOutcome::count(1).with_affected_objects(vec![destination]));
                    }
                }
            }
            return Ok(EffectOutcome::aggregate_summing_counts(outcomes));
        }
        if self.action == KeywordActionKind::SetSchemeInMotion {
            let mut schemes = Vec::with_capacity(self.amount as usize);
            for _ in 0..self.amount {
                schemes.push(
                    game.set_scheme_in_motion(ctx.controller)
                        .map_err(ExecutionError::Impossible)?,
                );
            }
            return Ok(EffectOutcome::resolved().with_affected_objects(schemes));
        }
        if self.action == KeywordActionKind::AbandonScheme {
            let scheme = game
                .abandon_scheme(ctx.source)
                .map_err(ExecutionError::Impossible)?;
            return Ok(EffectOutcome::resolved().with_affected_objects(vec![scheme]));
        }
        if self.action == KeywordActionKind::Harness && !game.harness(ctx.source) {
            return Ok(EffectOutcome::count(0));
        }
        let object_tags = object_tags_from_config(self, game, ctx)?;
        if self.action == KeywordActionKind::Exploit {
            // CR 702.110b: "when this exploits a creature" looks back in time.
            // A creature that exploited itself is gone by now, so carry its
            // last-known information for the source filter and its own trigger.
            let on_battlefield = game
                .object(ctx.source)
                .filter(|object| object.zone == crate::zone::Zone::Battlefield);
            let source_snapshot = on_battlefield
                .map(|object| game.cached_object_snapshot_with_calculated_characteristics(object))
                .or_else(|| ctx.source_snapshot.clone());
            let lookback = if on_battlefield.is_none() {
                source_snapshot.iter().cloned().collect()
            } else {
                Vec::new()
            };
            let event = TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(self.action, ctx.controller, ctx.source, self.amount)
                    .with_object_tags(object_tags)
                    .with_snapshot(source_snapshot),
                ctx.provenance,
            )
            .with_lookback_source_snapshots(lookback);
            return Ok(EffectOutcome::resolved().with_event(event));
        }
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(self.action, ctx.controller, ctx.source, self.amount)
                .with_object_tags(object_tags),
            ctx.provenance,
        );
        Ok(EffectOutcome::resolved().with_event(event))
    }

    fn cost_description(&self) -> Option<String> {
        if self.action == KeywordActionKind::Forage { return Some("Forage".into()); }
        // Internal scaffolding effect used to emit trigger-visible events from costs.
        // This should not show up as part of the printed/visible cost.
        Some(String::new())
    }
}

impl CostExecutableEffect for EmitKeywordActionEffect {
    fn can_execute_as_cost(&self, game: &GameState, source: crate::ids::ObjectId, controller: crate::ids::PlayerId) -> Result<(), crate::effects::CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(self, game, source, controller, crate::costs::PaymentReason::Other)
    }
    fn can_execute_as_cost_with_reason(&self, game: &GameState, source: crate::ids::ObjectId, controller: crate::ids::PlayerId, reason: crate::costs::PaymentReason) -> Result<(), crate::effects::CostValidationError> {
        if self.action == KeywordActionKind::Forage && !forage_payments(reason == crate::costs::PaymentReason::CastSpell).iter()
            .any(|effect| effect.0.can_execute_as_cost(game, source, controller).is_ok()) {
            return Err(crate::effects::CostValidationError::NotEnoughCards);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::ColorSet;
    use crate::effect::EffectId;
    use crate::ids::{ObjectId, PlayerId, StableId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn forwards_affected_object_memory_as_event_object_tag() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);
        let sacrificed = ObjectId::from_raw(20);
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect_id = EffectId(7);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::resolved().with_affected_object_memory(vec![OutcomeObjectMemory {
                object_id: sacrificed,
                stable_id: StableId::from(sacrificed),
                name: "Sacrificed Creature".to_string(),
                controller: bob,
                owner: bob,
                zone: Zone::Battlefield,
                power: Some(3),
                toughness: Some(5),
                mana_value: 2,
                card_types: vec![CardType::Creature],
                colors: ColorSet::default(),
                subtypes: Vec::new(),
                is_token: true,
            }]),
        );

        let effect = EmitKeywordActionEffect::new(crate::events::KeywordActionKind::Exploit, 1)
            .with_affected_object_memory_tag(effect_id, crate::tag::EXPLOITED_TAG);
        let outcome = effect.execute(&mut game, &mut ctx).expect("event emitted");
        let event = outcome.events.first().expect("keyword action event");
        let keyword = event
            .downcast::<KeywordActionEvent>()
            .expect("keyword action payload");
        let snapshots = keyword
            .object_tags
            .get(crate::tag::EXPLOITED_TAG)
            .expect("exploited tag");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].object_id, sacrificed);
        assert_eq!(snapshots[0].controller, bob);
        assert_eq!(snapshots[0].zone, Zone::Battlefield);
        assert_eq!(snapshots[0].toughness, Some(5));
    }
}
