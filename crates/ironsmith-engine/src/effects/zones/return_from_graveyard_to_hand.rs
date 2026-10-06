//! Return from graveyard to hand effect implementation.

use super::movement_instruction::{SelectedZoneMovement, ZoneMovementInstruction};
use crate::effect::EffectOutcome;
use crate::effects::CompletedEffectOutputs;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub use ironsmith_core::ReturnFromGraveyardToHandEffect;

impl EffectExecutor for ReturnFromGraveyardToHandEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Returned)
    }
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(super::movement_instruction::prepare_movement_instruction(
            self.clone(),
            ctx,
        ))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::movement_instruction::execute_movement_instruction(self.clone(), game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.random {
            None
        } else if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.random {
            None
        } else if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "card in graveyard to return"
    }
}

impl ZoneMovementInstruction for ReturnFromGraveyardToHandEffect {
    fn select(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SelectedZoneMovement, ExecutionError> {
        let targets = if self.random {
            let ChooseSpec::Object(filter) = self.target.base() else {
                return Ok(SelectedZoneMovement::Finished(EffectOutcome::impossible()));
            };
            if filter.zone != Some(Zone::Graveyard) {
                return Ok(SelectedZoneMovement::Finished(EffectOutcome::impossible()));
            }
            let count = self.target.count();
            let requested = if count.is_dynamic_x() {
                0
            } else {
                count.max.unwrap_or(count.min)
            };
            if requested == 0 {
                return Ok(SelectedZoneMovement::Finished(EffectOutcome::with_objects(
                    Vec::new(),
                )));
            }
            let filter_ctx = ctx.filter_context(game);
            let mut candidates = game
                .players
                .iter()
                .flat_map(|player| player.graveyard.iter().copied())
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                })
                .collect::<Vec<_>>();
            game.shuffle_slice(&mut candidates);
            candidates.into_iter().take(requested).collect::<Vec<_>>()
        } else {
            match resolve_objects_for_effect(game, ctx, &self.target) {
                Ok(objects) => objects,
                Err(ExecutionError::InvalidTarget) => {
                    return Ok(SelectedZoneMovement::Finished(
                        EffectOutcome::target_invalid(),
                    ));
                }
                Err(error) => return Err(error),
            }
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedZoneMovement::Finished(EffectOutcome::count(0)));
        }
        let requests = targets
            .into_iter()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.zone == Zone::Graveyard)
            })
            .map(|id| {
                super::PreparedZoneMove::capture(
                    game,
                    id,
                    Zone::Graveyard,
                    Zone::Hand,
                    ctx.cause.clone(),
                    None,
                )
            })
            .collect();

        let effect = self.clone();
        Ok(SelectedZoneMovement::moves(
            requests,
            move |game, _ctx, receipts, _pending_start| {
                let returned = receipts
                    .iter()
                    .filter_map(|(_, receipt)| match &receipt.original {
                        EventOutcome::Proceed(change) => change.new_object_id,
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if returned.is_empty() && !effect.random {
                    return Ok(EffectOutcome::target_invalid());
                }
                Ok(EffectOutcome::with_objects(returned.clone())
                    .with_affected_objects_from_game(game, returned))
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::zones::matchers::WouldGoToHandMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
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
    fn test_return_tagged_target_without_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_in_graveyard(&mut game, "Eternal Witness", alice);
        let snapshot = ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("return_target", snapshot);

        let effect =
            ReturnFromGraveyardToHandEffect::new(ChooseSpec::Tagged("return_target".into()), false);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        assert!(game.players[0].hand.contains(&ids[0]));
        assert!(game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_return_from_graveyard_to_hand_respects_replacement_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_in_graveyard(&mut game, "Reassembling Skeleton", alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGoToHandMatcher::you(),
                ReplacementAction::ChangeDestination(Zone::Exile),
            ),
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect =
            ReturnFromGraveyardToHandEffect::new(ChooseSpec::SpecificObject(creature_id), false);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        assert!(game.players[0].hand.is_empty());
        assert!(game.players[0].graveyard.is_empty());
        assert!(game.exile.contains(&ids[0]));
    }
}
