//! Attach arbitrary objects to a target permanent.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that attaches one or more objects to a destination object.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachObjectsEffect {
    /// Objects to attach.
    pub objects: ChooseSpec,
    /// Destination to attach objects to.
    pub target: ChooseSpec,
}

impl AttachObjectsEffect {
    /// Create a new attach-objects effect.
    pub fn new(objects: ChooseSpec, target: ChooseSpec) -> Self {
        Self { objects, target }
    }
}

impl EffectExecutor for AttachObjectsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_ids = resolve_objects_from_spec(game, &self.target, ctx)?;
        let Some(target_id) = target_ids.first().copied() else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };
        if game
            .object(target_id)
            .is_none_or(|target| target.zone != Zone::Battlefield)
        {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let object_ids = resolve_objects_from_spec(game, &self.objects, ctx)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut attached_count = 0i32;
        for object_id in object_ids {
            if object_id == target_id {
                continue;
            }
            let Some(attached_obj) = game.object(object_id) else {
                continue;
            };
            if attached_obj.zone != Zone::Battlefield {
                continue;
            }

            let previous_parent = attached_obj.attached_to;
            if let Some(previous_parent) = previous_parent
                && let Some(parent) = game.object_mut(previous_parent)
            {
                parent.attachments.retain(|id| *id != object_id);
            }

            if let Some(object_mut) = game.object_mut(object_id) {
                object_mut.attached_to = Some(target_id);
            } else {
                continue;
            }

            if let Some(target_mut) = game.object_mut(target_id)
                && !target_mut.attachments.contains(&object_id)
            {
                target_mut.attachments.push(object_id);
            }
            attached_count += 1;
        }

        Ok(EffectOutcome::count(attached_count))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to attach to"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_equipment(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn attach_objects_moves_equipment_to_new_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_creature = create_creature(&mut game, "Source", alice);
        let new_creature = create_creature(&mut game, "New", alice);
        let equipment = create_equipment(&mut game, "Sword", alice);

        if let Some(eq) = game.object_mut(equipment) {
            eq.attached_to = Some(source_creature);
        }
        if let Some(src) = game.object_mut(source_creature) {
            src.attachments.push(equipment);
        }

        let effect = AttachObjectsEffect::new(
            ChooseSpec::All(ObjectFilter::specific(equipment)),
            ChooseSpec::SpecificObject(new_creature),
        );
        let mut ctx = ExecutionContext::new_default(equipment, alice);
        let outcome = effect.execute(&mut game, &mut ctx).expect("attach should resolve");

        assert_eq!(outcome.result, EffectResult::Count(1));
        assert_eq!(
            game.object(equipment).and_then(|o| o.attached_to),
            Some(new_creature)
        );
        assert!(
            game.object(new_creature)
                .expect("new creature should exist")
                .attachments
                .contains(&equipment)
        );
        assert!(
            !game
                .object(source_creature)
                .expect("source creature should exist")
                .attachments
                .contains(&equipment)
        );
    }
}
