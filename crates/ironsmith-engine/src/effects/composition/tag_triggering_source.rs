//! Tag the source object from the triggering event for later reference.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::TagTriggeringSourceEffect;

impl EffectExecutor for TagTriggeringSourceEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn as_resolution_prelude(&self) -> Option<&dyn crate::effects::ResolutionPreludeBinding> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::ResolutionPreludeBinding::bind_resolution_prelude(self, game, ctx)
    }
}

impl crate::effects::ResolutionPreludeBinding for TagTriggeringSourceEffect {
    fn bind_resolution_prelude(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let event = ctx.triggering_event.as_ref().ok_or_else(|| {
            ExecutionError::UnresolvableValue("missing triggering event".to_string())
        })?;
        let source_id = event.source_object().ok_or_else(|| {
            ExecutionError::UnresolvableValue("triggering event missing source".to_string())
        })?;
        // "That spell or ability" after "becomes the target of a spell or
        // ability" is the targeting stack object. An ability exists
        // independently of its source (CR 113.7a): name its own stack entry,
        // controlled by the ability's controller, even after its source left.
        if let Some(targeted) = event.downcast::<crate::events::spells::BecomesTargetedEvent>()
            && targeted.by_ability
        {
            let Some(ability_id) = targeted.stack_ability else {
                return Ok(EffectOutcome::count(0));
            };
            let Some(entry) = game
                .stack
                .iter()
                .find(|entry| entry.is_ability && entry.target_id() == ability_id)
            else {
                // Once that exact ability is gone, never redirect the reference
                // to a sibling activation or its physical source permanent.
                return Ok(EffectOutcome::count(0));
            };
            let snapshot = game
                .object(source_id)
                .map(|source| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(source, game)
                })
                .or_else(|| entry.source_snapshot.clone());
            if let Some(mut snapshot) = snapshot {
                snapshot.object_id = ability_id;
                snapshot.controller = entry.controller;
                ctx.set_tagged_objects(self.tag.as_str(), vec![snapshot]);
                return Ok(EffectOutcome::count(1));
            }
            return Ok(EffectOutcome::count(0));
        }
        let snapshot = game
            .object(source_id)
            .filter(|_| !game.is_phased_out(source_id))
            .map(|source| ObjectSnapshot::from_object_with_calculated_characteristics(source, game))
            .or_else(|| {
                event
                    .source_snapshot()
                    .filter(|snapshot| snapshot.object_id == source_id)
                    .cloned()
            })
            .or_else(|| game.source_last_known_snapshot(source_id).cloned());
        let Some(snapshot) = snapshot else {
            return Ok(EffectOutcome::count(0));
        };
        ctx.set_tagged_objects(self.tag.as_str(), vec![snapshot]);
        Ok(EffectOutcome::count(1))
    }
}

#[cfg(test)]
mod targeting_tests {
    use super::*;
    use crate::{card::CardBuilder, ids::{CardId, PlayerId}, game_state::{StackEntry, Target}, zone::Zone};
    #[test]
    fn targeting_reference_uses_exact_ability_entry_including_copy_proxy() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Activated source").build();
        let source = game.create_object_from_card(&card, a, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
        let mut first = StackEntry::ability(source, b, crate::resolution::ResolutionProgram::from_effects(vec![]));
        first.source_snapshot = Some(snapshot.clone());
        game.push_to_stack(first);
        let first = game.stack.last().unwrap().clone();
        let mut second = StackEntry::ability(source, a, crate::resolution::ResolutionProgram::from_effects(vec![]));
        second.source_snapshot = Some(snapshot.clone()); game.push_to_stack(second);
        let copy = game.create_object_from_card(&card, b, Zone::Stack);
        let mut copied = StackEntry::ability(copy, b, crate::resolution::ResolutionProgram::from_effects(vec![]));
        copied.source_snapshot = Some(snapshot); game.stack.push(copied);
        let copied = game.stack.last().unwrap().clone();
        assert_eq!(copied.target_id(), copy);
        for entry in [first, copied] {
            let targeted = crate::events::BecomesTargetedEvent::from_stack_entry(Target::Player(a), &entry);
            assert_eq!(targeted.source, source);
            let mut ctx = ExecutionContext::new_default(source, a);
            ctx.triggering_event = Some(crate::triggers::TriggerEvent::new_with_provenance(targeted, Default::default()));
            TagTriggeringSourceEffect::new("targeting").execute(&mut game, &mut ctx).unwrap();
            let tagged = ctx.get_tagged_all("targeting").unwrap();
            assert_eq!(tagged[0].object_id, entry.target_id()); assert_eq!(tagged[0].controller, b);
            game.stack.retain(|candidate| candidate.target_id() != entry.target_id());
            let mut absent = ExecutionContext::new_default(source, a);
            absent.triggering_event = ctx.triggering_event;
            TagTriggeringSourceEffect::new("targeting").execute(&mut game, &mut absent).unwrap();
            assert!(absent.get_tagged_all("targeting").is_none());
        }
    }
}
