use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ReplacementApplyMode};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Registers a concrete zone-change replacement effect for the currently resolved object(s).
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterZoneReplacementEffect {
    pub target: ChooseSpec,
    pub from_zone: Option<Zone>,
    pub to_zone: Option<Zone>,
    pub replacement_zone: Zone,
    pub mode: ReplacementApplyMode,
}

impl RegisterZoneReplacementEffect {
    pub fn new(
        target: ChooseSpec,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            mode,
        }
    }
}

impl EffectExecutor for RegisterZoneReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        for object_id in &object_ids {
            let replacement = ReplacementEffect::with_matcher(
                ctx.source,
                ctx.controller,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(*object_id),
                    self.from_zone,
                    self.to_zone,
                ),
                ReplacementAction::ChangeDestination(self.replacement_zone),
            );

            match self.mode {
                ReplacementApplyMode::OneShot => {
                    game.replacement_effects.add_one_shot_effect(replacement);
                }
                ReplacementApplyMode::UntilEndOfTurn => {
                    game.replacement_effects
                        .add_until_end_of_turn_effect(replacement);
                }
                ReplacementApplyMode::Resolution => {
                    game.replacement_effects.add_resolution_effect(replacement);
                }
            }
        }

        Ok(EffectOutcome::with_objects(object_ids))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target for replacement"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::OutcomeStatus;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, owner: PlayerId, zone: Zone) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Replacement Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn test_registered_zone_replacement_exiles_matching_death_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move effect to resolve on the creature"
        );

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after replacement");
        assert_eq!(
            game.object(exiled_id)
                .expect("exiled creature should exist")
                .zone,
            Zone::Exile
        );
    }

    #[test]
    fn test_registered_zone_replacement_does_not_apply_to_nonmatching_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Hand,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move-to-hand effect to resolve on the creature"
        );
        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after moving to hand");
        assert_eq!(
            game.object(moved_id)
                .expect("moved creature should exist")
                .zone,
            Zone::Hand
        );
    }
}
