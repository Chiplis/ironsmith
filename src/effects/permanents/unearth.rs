//! Unearth effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, EffectResult, Until};
use crate::effects::zones::MoveToZoneEffect;
use crate::effects::{
    ApplyContinuousEffect, ApplyReplacementEffect, EffectExecutor, ScheduleDelayedTriggerEffect,
};
use crate::events::zones::matchers::WouldLeaveBattlefieldMatcher;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::Trigger;
use crate::zone::Zone;

/// Effect that executes the rules text for Unearth.
///
/// "Return this card from your graveyard to the battlefield. It gains haste.
/// Exile it at the beginning of the next end step or if it would leave the
/// battlefield."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnearthEffect;

impl UnearthEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for UnearthEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = ctx.source;
        let Some(source_obj) = game.object(source_id) else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };
        if source_obj.zone != Zone::Graveyard {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let move_to_battlefield =
            Effect::new(MoveToZoneEffect::new(ChooseSpec::Source, Zone::Battlefield, false).under_owner_control());
        let move_outcome = execute_effect(game, &move_to_battlefield, ctx)?;
        let EffectOutcome { result, events } = move_outcome;

        let new_id = match result {
            EffectResult::Objects(ids) => {
                let Some(&id) = ids.first() else {
                    return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)
                        .with_events(events));
                };
                id
            }
            other => return Ok(EffectOutcome::from_result(other).with_events(events)),
        };

        // Grant haste until end of turn to the returned permanent.
        let haste_effect = ApplyContinuousEffect::new(
            EffectTarget::Specific(new_id),
            Modification::AddAbility(StaticAbility::haste()),
            Until::EndOfTurn,
        )
        .with_source_type(EffectSourceType::Resolution {
            locked_targets: vec![new_id],
        });
        let _ = execute_effect(game, &Effect::new(haste_effect), ctx)?;

        // "If it would leave the battlefield, exile it instead."
        let replacement = ReplacementEffect::with_matcher(
            new_id,
            ctx.controller,
            WouldLeaveBattlefieldMatcher::new(ObjectFilter::specific(new_id)),
            ReplacementAction::ChangeDestination(Zone::Exile),
        )
        .self_replacing();
        let _ = execute_effect(
            game,
            &Effect::new(ApplyReplacementEffect::one_shot(replacement)),
            ctx,
        )?;

        // "Exile it at the beginning of the next end step."
        let schedule = ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![Effect::exile(ChooseSpec::SpecificObject(new_id))],
            true,
            vec![new_id],
            PlayerFilter::Specific(ctx.controller),
        );
        let _ = execute_effect(game, &Effect::new(schedule), ctx)?;

        Ok(EffectOutcome::from_result(EffectResult::Objects(vec![new_id])).with_events(events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_in_graveyard(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_unearth_effect_returns_from_graveyard_and_sets_cleanup() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = create_creature_in_graveyard(&mut game, "Unearth Tester", alice);
        let mut ctx = ExecutionContext::new_default(source_id, alice);

        let result = UnearthEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("unearth should resolve");

        let EffectResult::Objects(ids) = result.result else {
            panic!("expected returned battlefield object");
        };
        let returned_id = ids[0];

        assert!(
            game.battlefield.contains(&returned_id),
            "unearthed card should be on battlefield"
        );
        assert_eq!(
            game.delayed_triggers.len(),
            1,
            "unearthed card should have next end-step exile trigger"
        );
        assert_eq!(
            game.replacement_effects.one_shot_effects_snapshot().len(),
            1,
            "unearthed card should register one-shot leave-battlefield replacement"
        );
    }

    #[test]
    fn test_unearth_leave_battlefield_replacement_exiles_instead() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = create_creature_in_graveyard(&mut game, "Unearth Tester", alice);
        let mut ctx = ExecutionContext::new_default(source_id, alice);

        let result = UnearthEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("unearth should resolve");
        let returned_id = match result.result {
            EffectResult::Objects(ids) => ids[0],
            other => panic!("expected returned battlefield object, got {other:?}"),
        };

        let stable_id = game
            .object(returned_id)
            .expect("returned object should exist")
            .stable_id;

        let mut move_ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        let move_to_hand = Effect::new(MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(returned_id),
            Zone::Hand,
            false,
        ));
        let _ = execute_effect(&mut game, &move_to_hand, &mut move_ctx)
            .expect("move should resolve through replacement processing");

        let in_hand = game.players[0]
            .hand
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|obj| obj.stable_id == stable_id);
        let in_exile = game
            .exile
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|obj| obj.stable_id == stable_id);

        assert!(
            !in_hand,
            "unearthed card should not go to hand when leaving battlefield"
        );
        assert!(
            in_exile,
            "unearthed card should be exiled when it would leave battlefield"
        );
    }
}
