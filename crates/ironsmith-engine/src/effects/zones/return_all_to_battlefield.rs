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
    fn supports_simultaneous_player_action(&self) -> bool { true }
    fn prepare_simultaneous_player_action(&self, game: &GameState, ctx: &mut ExecutionContext)
        -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(ReturnAllProposal { effect: self.clone(),
            objects: resolve_objects_from_spec(game, &ChooseSpec::all(self.filter.clone()), ctx)? }))
    }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_from_spec(game, &ChooseSpec::all(self.filter.clone()), ctx)?;
        commit_return_all(self, objects, game, ctx, false).map(|commit| commit.outcome)
    }
}

#[derive(Debug)]
struct ReturnAllProposal { effect: ReturnAllToBattlefieldEffect, objects: Vec<crate::ids::ObjectId> }
impl crate::effects::SimultaneousEffectProposal for ReturnAllProposal {
    fn commit_original(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        commit_return_all(&self.effect, self.objects, game, ctx, true)
    }
    fn commit(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        commit_return_all(&self.effect, self.objects, game, ctx, false).map(|commit| commit.outcome)
    }
}
struct ReturnAllCompletion {
    receipts: Option<Vec<(crate::ids::ObjectId, crate::events::processing::PreparedEventOutcome<super::AppliedZoneChange>)>>,
    frozen: Option<super::FrozenZoneChangeReceipts>,
}
impl crate::effects::SimultaneousEffectCompletion for ReturnAllCompletion {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        let receipts = self.receipts.take().ok_or_else(|| ExecutionError::InternalError("return receipts already frozen".into()))?;
        self.frozen = Some(super::freeze_zone_change_receipts(game, receipts));
        Ok(())
    }
    fn complete(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext, original: EffectOutcome)
        -> Result<EffectOutcome, ExecutionError> {
        let frozen = self.frozen.ok_or_else(|| ExecutionError::InternalError("return completion requires the completed original batch".into()))?;
        super::finish_zone_change_receipts_frozen(game, ctx, original, frozen)
    }
}
fn commit_return_all(effect: &ReturnAllToBattlefieldEffect, objects: Vec<crate::ids::ObjectId>,
    game: &mut GameState, ctx: &mut ExecutionContext, defer_additions: bool)
    -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    use crate::effects::SimultaneousEffectCommit;
    if ctx.decision_maker.awaiting_choice() { return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0))); }
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let instruction = (|| -> Result<SimultaneousEffectCommit, ExecutionError> {
        let mut entries = Vec::new();
        for object_id in objects {
            let options = match effect.battlefield_controller {
                BattlefieldController::Preserve => BattlefieldEntryOptions::preserve(effect.tapped),
                BattlefieldController::Owner => BattlefieldEntryOptions::owner(effect.tapped),
                BattlefieldController::You => {
                    BattlefieldEntryOptions::specific(ctx.controller, effect.tapped)
                }
            };
            let Some(object) = game.object(object_id) else {
                continue;
            };
            let memory =
                OutcomeObjectMemory::from_snapshot(&ObjectSnapshot::from_object(object, game));
            if effect.face_down
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
        )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0))); }
        if outcomes.len() != entries.len() { return Err(ExecutionError::InternalError("battlefield batch lost an entry receipt".into())); }
        let mut receipts = Vec::new();
        let mut returned_count = 0;
        let mut returned_ids = Vec::new();
        let mut affected_memory = Vec::new();
        for ((object_id, _, memory), outcome) in entries.into_iter().zip(outcomes) {
            match &outcome.outcome {
                BattlefieldEntryOutcome::Moved(new_id) => {
                    returned_count += 1;
                    returned_ids.push(*new_id);
                    affected_memory.push(memory);
                }
                BattlefieldEntryOutcome::Redirected(receipt) => {
                    returned_count += i32::try_from(receipt.new_object_ids.len()).unwrap_or(i32::MAX);
                    returned_ids.extend(receipt.new_object_ids.iter().copied());
                    affected_memory.push(memory);
                }
                BattlefieldEntryOutcome::Prevented => {
                    if effect.face_down
                        && let Some(card) = game.object_mut(object_id)
                    {
                        card.end_face_down_cast_overlay();
                    }
                }
            }
            let (original, receipt) = outcome.into_zone_receipt();
            if original != object_id { return Err(ExecutionError::InternalError("battlefield receipt changed original identity".into())); }
            receipts.push((original, receipt));
        }

        let mut outcome = EffectOutcome::count(returned_count).with_result_objects(returned_ids);
        if !affected_memory.is_empty() {
            outcome = outcome.with_affected_object_memory(affected_memory);
        }

        if defer_additions {
            Ok(SimultaneousEffectCommit { outcome, completion: Some(Box::new(ReturnAllCompletion { receipts: Some(receipts), frozen: None })) })
        } else {
            super::finish_zone_change_receipts(game, ctx, outcome, receipts).map(SimultaneousEffectCommit::finished)
        }
    })();
    let pending = ctx.decision_maker.awaiting_choice();
    if pending || instruction.is_err() { *game = checkpoint; context_checkpoint.restore(ctx); }
    if pending { return instruction.map(|_| SimultaneousEffectCommit::finished(EffectOutcome::count(0))); }
    instruction
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
