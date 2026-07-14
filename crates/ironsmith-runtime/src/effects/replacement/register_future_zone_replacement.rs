use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterFutureZoneReplacementEffect = ironsmith_core::RegisterFutureZoneReplacementEffect;

impl EffectExecutor for RegisterFutureZoneReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut matcher = crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            self.filter.clone(),
            self.from_zone,
            self.to_zone,
        );
        if let Some(cause_filter) = self.cause_filter.clone() {
            matcher = matcher.with_cause_filter(cause_filter);
        }
        if self.require_cause_source_match {
            matcher = matcher.requiring_cause_source_match();
        }
        let action =
            if self.link_exiled_to_source && self.replacement_zone == crate::zone::Zone::Exile {
                ReplacementAction::ExileWithSourceLink
            } else {
                ReplacementAction::ChangeDestination(self.replacement_zone)
            };
        let replacement =
            ReplacementEffect::with_matcher(ctx.source, ctx.controller, matcher, action);

        match self.mode {
            crate::effects::ReplacementApplyMode::OneShot => {
                game.effect_store
                    .replacement_effects
                    .add_one_shot_effect(replacement);
            }
            crate::effects::ReplacementApplyMode::UntilEndOfTurn => {
                game.effect_store
                    .replacement_effects
                    .add_until_end_of_turn_effect(replacement);
            }
            crate::effects::ReplacementApplyMode::Resolution => {
                game.effect_store
                    .replacement_effects
                    .add_resolution_effect(replacement);
            }
        }

        Ok(EffectOutcome::from_status(OutcomeStatus::Succeeded))
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::{ReplacementApplyMode, execute_effect};
    use crate::ids::{CardId, PlayerId};
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(game: &mut GameState, owner: PlayerId, name: &str) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn move_to_graveyard(
        game: &mut GameState,
        ctx: &mut ExecutionContext<'_>,
        object: crate::ids::ObjectId,
    ) {
        execute_effect(
            game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(object),
                Zone::Graveyard,
                false,
            ),
            ctx,
        )
        .expect("zone change should resolve");
    }

    #[test]
    fn until_end_of_turn_future_replacement_is_multi_use_and_expires() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, "Future Replacement Source");
        let first = create_creature(&mut game, alice, "First Creature");
        let second = create_creature(&mut game, alice, "Second Creature");
        let third = create_creature(&mut game, alice, "Third Creature");
        let first_stable = game.object(first).unwrap().stable_id;
        let second_stable = game.object(second).unwrap().stable_id;
        let third_stable = game.object(third).unwrap().stable_id;

        let effect = RegisterFutureZoneReplacementEffect::new(
            ObjectFilter::creature(),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::UntilEndOfTurn,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        effect
            .execute(&mut game, &mut ctx)
            .expect("future replacement should register");

        move_to_graveyard(&mut game, &mut ctx, first);
        move_to_graveyard(&mut game, &mut ctx, second);
        assert_eq!(
            game.object(game.find_object_by_stable_id(first_stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
        assert_eq!(
            game.object(game.find_object_by_stable_id(second_stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
        assert_eq!(game.effect_store.replacement_effects.effects().len(), 1);

        game.effect_store
            .replacement_effects
            .clear_until_end_of_turn_effects();
        move_to_graveyard(&mut game, &mut ctx, third);
        assert_eq!(
            game.object(game.find_object_by_stable_id(third_stable).unwrap())
                .unwrap()
                .zone,
            Zone::Graveyard
        );
    }

    #[test]
    fn linked_future_replacement_records_every_exiled_object() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, "Linked Replacement Source");
        let first = create_creature(&mut game, alice, "First Linked Creature");
        let second = create_creature(&mut game, alice, "Second Linked Creature");
        let first_stable = game.object(first).unwrap().stable_id;
        let second_stable = game.object(second).unwrap().stable_id;

        let effect = RegisterFutureZoneReplacementEffect::new(
            ObjectFilter::permanent().controlled_by(crate::target::PlayerFilter::You),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::UntilEndOfTurn,
        )
        .linking_exiled_to_source();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        effect
            .execute(&mut game, &mut ctx)
            .expect("linked future replacement should register");

        move_to_graveyard(&mut game, &mut ctx, first);
        move_to_graveyard(&mut game, &mut ctx, second);
        let linked = game.get_exiled_with_source_links(source);
        let first = game.find_object_by_stable_id(first_stable).unwrap();
        let second = game.find_object_by_stable_id(second_stable).unwrap();
        assert!(linked.contains(&first));
        assert!(linked.contains(&second));
        assert!(matches!(
            &game.effect_store.replacement_effects.effects()[0].replacement,
            ReplacementAction::ExileWithSourceLink
        ));
    }
}
