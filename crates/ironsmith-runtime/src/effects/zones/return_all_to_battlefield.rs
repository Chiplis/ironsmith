//! Return all matching cards to the battlefield.

use super::battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_batch_with_options,
};
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::BattlefieldController;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
pub type ReturnAllToBattlefieldEffect = ironsmith_core::ReturnAllToBattlefieldEffect;

impl EffectExecutor for ReturnAllToBattlefieldEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let spec = ChooseSpec::all(self.filter.clone());
        let objects = resolve_objects_from_spec(game, &spec, ctx)?;

        let mut entries = Vec::new();
        for object_id in objects {
            let options = match self.battlefield_controller {
                BattlefieldController::Preserve => BattlefieldEntryOptions::preserve(self.tapped),
                BattlefieldController::Owner => BattlefieldEntryOptions::owner(self.tapped),
                BattlefieldController::You => {
                    BattlefieldEntryOptions::specific(ctx.controller, self.tapped)
                }
            };
            let Some(object) = game.object(object_id) else {
                continue;
            };
            let memory =
                OutcomeObjectMemory::from_snapshot(&ObjectSnapshot::from_object(object, game));
            if self.face_down
                && let Some(card) = game.object_mut(object_id)
            {
                card.apply_face_down_cast_overlay();
            }
            entries.push((object_id, options, memory));
        }

        let outcomes = move_to_battlefield_batch_with_options(
            game,
            ctx,
            entries
                .iter()
                .map(|(object, options, _)| (*object, options.clone()))
                .collect(),
        );
        let mut returned_count = 0;
        let mut affected_memory = Vec::new();
        for ((object_id, _, memory), outcome) in entries.into_iter().zip(outcomes) {
            match outcome {
                BattlefieldEntryOutcome::Moved(_) => {
                    returned_count += 1;
                    affected_memory.push(memory);
                }
                BattlefieldEntryOutcome::Prevented => {
                    if self.face_down
                        && let Some(card) = game.object_mut(object_id)
                    {
                        card.end_face_down_cast_overlay();
                    }
                }
            }
        }

        let mut outcome = EffectOutcome::count(returned_count);
        if !affected_memory.is_empty() {
            outcome = outcome.with_affected_object_memory(affected_memory);
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::BooleanContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn graveyard_permanent(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        card_type: CardType,
    ) -> ObjectId {
        let id = game.new_object_id();
        let mut builder =
            CardBuilder::new(CardId::from_raw(id.0 as u32), name).card_types(vec![card_type]);
        if card_type == CardType::Creature {
            builder = builder.power_toughness(PowerToughness::fixed(2, 2));
        }
        game.add_object(Object::from_card(
            id,
            &builder.build(),
            owner,
            Zone::Graveyard,
        ));
        id
    }

    fn return_all_permanents(game: &mut GameState, dm: &mut dyn DecisionMaker) {
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let filter = ObjectFilter::permanent()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::Any);
        let mut ctx = ExecutionContext::new(source, alice, dm);
        let outcome = ReturnAllToBattlefieldEffect::new(filter, false)
            .execute(game, &mut ctx)
            .expect("simultaneous return should resolve");
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
    }

    #[test]
    fn simultaneous_entries_do_not_expose_an_entrant_replacement_to_its_companion() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let orb = graveyard_permanent(&mut game, alice, "Prospective Orb", CardType::Artifact);
        game.object_mut(orb)
            .expect("orb should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::permanents_enter_tapped(),
            ));
        graveyard_permanent(&mut game, alice, "Simultaneous Bear", CardType::Creature);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        return_all_permanents(&mut game, &mut dm);

        let bear = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Simultaneous Bear")
            })
            .expect("bear should enter");
        assert!(
            !game.is_tapped(bear),
            "a replacement effect entering in the same event does not already exist"
        );
    }

    #[derive(Default)]
    struct CapturePayLifeOrder {
        players: Vec<PlayerId>,
        battlefield_sizes: Vec<usize>,
    }

    impl DecisionMaker for CapturePayLifeOrder {
        fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
            if ctx.description.to_ascii_lowercase().contains("pay") {
                self.players.push(ctx.player);
                self.battlefield_sizes.push(game.battlefield.len());
            }
            true
        }
    }

    #[test]
    fn simultaneous_entry_replacement_choices_are_collected_in_apnap_order() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;

        // Deliberately create Bob's card first so raw object order disagrees
        // with APNAP order.
        for (owner, name) in [(bob, "Bob Shockland"), (alice, "Alice Shockland")] {
            let land = graveyard_permanent(&mut game, owner, name, CardType::Land);
            game.object_mut(land)
                .expect("land should exist")
                .abilities_mut()
                .push(Ability::static_ability(
                    StaticAbility::pay_life_or_enter_tapped(2),
                ));
        }

        let mut dm = CapturePayLifeOrder::default();
        return_all_permanents(&mut game, &mut dm);

        assert_eq!(dm.players, vec![alice, bob]);
        assert_eq!(dm.battlefield_sizes, vec![0, 0]);
        assert_eq!(game.player(alice).map(|player| player.life), Some(18));
        assert_eq!(game.player(bob).map(|player| player.life), Some(18));
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
    }

    #[test]
    fn simultaneous_entry_choices_preserve_combined_cost_payability() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice should exist").life = 3;

        for name in ["First Shockland", "Second Shockland"] {
            let land = graveyard_permanent(&mut game, alice, name, CardType::Land);
            game.object_mut(land)
                .expect("land should exist")
                .abilities_mut()
                .push(Ability::static_ability(
                    StaticAbility::pay_life_or_enter_tapped(2),
                ));
        }

        let mut dm = CapturePayLifeOrder::default();
        return_all_permanents(&mut game, &mut dm);

        assert_eq!(dm.players, vec![alice]);
        assert_eq!(game.player(alice).map(|player| player.life), Some(1));
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.is_tapped(**id))
                .count(),
            1,
            "the second payment must become unavailable after the first is reserved"
        );
    }
}
