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
use crate::events::KeywordActionEvent;
use crate::game_state::GameState;
use crate::object::ObjectKind;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::EmitKeywordActionEffect;

fn snapshot_from_memory(game: &GameState, memory: &OutcomeObjectMemory) -> ObjectSnapshot {
    let mut snapshot = game
        .object(memory.object_id)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .unwrap_or_else(|| ObjectSnapshot {
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
            mana_cost: None,
            colors: memory.colors,
            supertypes: Vec::new(),
            card_types: memory.card_types.clone(),
            subtypes: memory.subtypes.clone(),
            compiled_card_text: String::new(),
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            power: memory.power,
            toughness: memory.toughness,
            base_power: memory.power,
            base_toughness: memory.toughness,
            loyalty: None,
            defense: None,
            abilities: Vec::new(),
            aura_attach_filter: None,
            x_value: None,
            cast_order_this_turn: None,
            counters: HashMap::new(),
            is_token: memory.is_token,
            tapped: false,
            flipped: false,
            face_down: false,
            transform_count: 0,
            attached_to: None,
            attachments: Vec::new(),
            was_enchanted: false,
            is_monstrous: false,
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
        let object_tags = object_tags_from_config(self, game, ctx)?;
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(self.action, ctx.controller, ctx.source, self.amount)
                .with_object_tags(object_tags),
            ctx.provenance,
        );
        Ok(EffectOutcome::resolved().with_event(event))
    }

    fn cost_description(&self) -> Option<String> {
        // Internal scaffolding effect used to emit trigger-visible events from costs.
        // This should not show up as part of the printed/visible cost.
        Some(String::new())
    }
}

impl CostExecutableEffect for EmitKeywordActionEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
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
