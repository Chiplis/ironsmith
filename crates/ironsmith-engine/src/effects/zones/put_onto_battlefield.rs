//! Put onto battlefield effect implementation.

use super::battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, PreparedBattlefieldEntryBatch,
    prepare_battlefield_entry_batch, resolve_battlefield_entry_counters,
};
use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter};
use crate::effects::{
    CompletedEffectOutputs, EffectExecutor, SimultaneousEffectCommit, SimultaneousEffectProposal,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
pub use ironsmith_core::PutOntoBattlefieldEffect;

/// Effect that puts a target card onto the battlefield.
///
/// This is a general-purpose effect for putting cards from any zone onto
/// the battlefield. Used by various spells and abilities that cheat permanents
/// into play.
///
/// # Fields
///
/// * `target` - Which card to put onto the battlefield
/// * `tapped` - Whether the permanent enters tapped
/// * `controller` - Who controls the permanent when it enters
///
/// # Example
///
/// ```ignore
/// // Put target creature card from your hand onto the battlefield tapped
/// let effect = PutOntoBattlefieldEffect::new(
///     ChooseSpec::creature_card_in_hand(),
///     true,
///     PlayerFilter::You,
/// );
/// ```
impl EffectExecutor for PutOntoBattlefieldEffect {
    fn own_preflight_object_specs(&self) -> Vec<ChooseSpec> {
        vec![self.target.clone()]
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        // Both a tagged set and an object iterator already identify the exact
        // original cards. Capture them before any player's entry commits.
        matches!(
            self.target.base(),
            ChooseSpec::Tagged(_) | ChooseSpec::Iterated
        )
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        let objects = crate::effects::helpers::resolve_objects_from_spec(game, &self.target, ctx)?;
        Ok(Box::new(PutProposal::from_objects(
            self, game, ctx, objects,
        )?))
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let objects = super::resolve_zone_move_objects(game, ctx, &self.target)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let proposal = PutProposal::from_objects(self, game, ctx, objects)?;
                crate::effects::composition::complete_prepared_original_with_outputs(
                    Box::new(proposal),
                    game,
                    ctx,
                    true,
                )
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "card to put onto battlefield"
    }
}

/// Complete entry preparation is shared by standalone and simultaneous puts.
/// Every identity, controller and counter amount belongs to the original
/// instruction; replacement programs cannot reselect another player's cards.
#[derive(Debug)]
struct PutProposal {
    entries: Vec<(
        crate::ids::ObjectId,
        ObjectSnapshot,
        Vec<(crate::object::CounterType, u32)>,
    )>,
    controller: crate::ids::PlayerId,
    tapped: bool,
    prepared: Option<PreparedBattlefieldEntryBatch>,
    draws: super::ZoneInstructionDraws,
}
impl PutProposal {
    fn from_objects(
        effect: &PutOntoBattlefieldEffect,
        game: &GameState,
        ctx: &ExecutionContext,
        objects: Vec<crate::ids::ObjectId>,
    ) -> Result<Self, ExecutionError> {
        let controller = resolve_player_filter(game, &effect.controller, ctx)?;
        let entries = objects
            .into_iter()
            .filter_map(|id| game.object(id).map(|object| (id, object)))
            .map(|(id, object)| {
                Ok((
                    id,
                    ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)?,
                    resolve_battlefield_entry_counters(
                        game,
                        ctx,
                        id,
                        &effect.enters_with_counters,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, ExecutionError>>()?;
        Ok(Self {
            entries,
            controller,
            tapped: effect.tapped,
            prepared: None,
            draws: Default::default(),
        })
    }
}
impl SimultaneousEffectProposal for PutProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.entries.is_empty() {
            return Ok(());
        }
        let requests = self
            .entries
            .iter()
            .map(|(id, _, counters)| {
                (
                    *id,
                    BattlefieldEntryOptions::specific(self.controller, self.tapped)
                        .with_initial_counters(counters.clone()),
                )
            })
            .collect();
        self.prepared = prepare_battlefield_entry_batch(
            game,
            ctx,
            requests,
            Default::default(),
            Some(&mut self.draws),
        )?;
        Ok(())
    }
    fn commit_original(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        if self.entries.is_empty() {
            return Ok(SimultaneousEffectCommit::finished(
                EffectOutcome::target_invalid(),
            ));
        }
        let prepared = self.prepared.ok_or_else(|| {
            ExecutionError::InternalError(
                "battlefield put committed without complete entry preparation".into(),
            )
        })?;
        let outcomes = prepared.commit(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0)));
        }
        if outcomes.len() != self.entries.len() {
            return Err(ExecutionError::InternalError(
                "battlefield batch lost an entry receipt".into(),
            ));
        }
        let mut receipts = Vec::new();
        let mut moved_ids = Vec::new();
        let mut affected_memory = Vec::new();
        let mut prevented = false;
        for ((object_id, memory, _), outcome) in self.entries.into_iter().zip(outcomes) {
            match &outcome.outcome {
                BattlefieldEntryOutcome::Moved(new_id) => {
                    moved_ids.push(*new_id);
                    affected_memory.push(memory);
                }
                BattlefieldEntryOutcome::Redirected(receipt) => {
                    moved_ids.extend(receipt.new_object_ids.iter().copied());
                    affected_memory.push(memory);
                }
                BattlefieldEntryOutcome::Prevented => prevented = true,
            }
            let (original, receipt) = self.draws.retain_entry_receipt(outcome);
            if original != object_id {
                return Err(ExecutionError::InternalError(
                    "battlefield receipt changed original identity".into(),
                ));
            }
            receipts.push((original, receipt));
        }
        let outcome = if !moved_ids.is_empty() {
            EffectOutcome::with_objects(moved_ids).with_affected_object_memory(affected_memory)
        } else if prevented {
            EffectOutcome::impossible()
        } else {
            EffectOutcome::target_invalid()
        };
        Ok(self.draws.finish(outcome, receipts, ctx))
    }
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.commit_original(game, ctx)
            .map(SimultaneousEffectCommit::into_retained)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, true)
            .map(CompletedEffectOutputs::into_outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{ChoiceCount, Effect};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

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

    fn create_creature_in_hand(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Hand);
        game.add_object(obj);
        id
    }

    fn create_creature_in_graveyard(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    fn create_creature_in_library(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj);
        id
    }

    fn create_land_in_library(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_put_onto_battlefield_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_hand(&mut game, "Emrakul", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(Zone::Hand)),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature should be on battlefield and untapped
            assert!(game.battlefield.contains(&new_id));
            assert!(!game.is_tapped(new_id));
            assert_eq!(game.current_controller(new_id), Some(alice));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_put_onto_battlefield_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_hand(&mut game, "Emrakul", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(Zone::Hand)),
            true,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature should be on battlefield and tapped
            assert!(game.battlefield.contains(&new_id));
            assert!(game.is_tapped(new_id));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn put_onto_battlefield_applies_initial_counters_during_entry() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_library(&mut game, "Stunned Arrival", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(Zone::Library)),
            true,
        )
        .with_entry_counter(ironsmith_core::BattlefieldEntryCounterSpec::new(
            crate::object::CounterType::Stun,
            crate::effect::Value::Fixed(1),
            ironsmith_core::BattlefieldEntryCounterSurface::Inline,
        ));
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected moved object result");
        };
        let [entered] = ids.as_slice() else {
            panic!("expected exactly one entered object");
        };

        assert!(game.is_tapped(*entered));
        assert_eq!(
            game.counter_count(*entered, crate::object::CounterType::Stun),
            1
        );
    }

    #[test]
    fn test_put_onto_battlefield_from_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Griselbrand", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(Zone::Graveyard)),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            assert!(game.battlefield.contains(&new_id));
        } else {
            panic!("Expected Objects result");
        }
        // Graveyard should be empty now
        assert!(game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_put_onto_battlefield_opponent_creature_you_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature_in_graveyard(&mut game, "Wurmcoil", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(Zone::Graveyard)),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Alice controls it even though Bob owns it
            let obj = game.object(new_id).unwrap();
            assert_eq!(game.controller_of(obj), alice);
            assert_eq!(obj.owner, bob);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_put_onto_battlefield_iterated_object_from_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_library(&mut game, "Library Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.iteration.iterated_object = Some(creature_id);

        let effect = PutOntoBattlefieldEffect::you_control(ChooseSpec::Iterated, true);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            assert!(game.battlefield.contains(&new_id));
            assert!(game.is_tapped(new_id));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_put_onto_battlefield_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx);
        assert_eq!(result, Err(ExecutionError::InvalidTarget));
    }

    #[test]
    fn test_map_the_frontier_style_sequence_puts_chosen_cards_onto_battlefield_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_land_in_library(&mut game, "Forest", alice);
        create_land_in_library(&mut game, "Desert", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = crate::effects::SequenceEffect::new(vec![
            Effect::choose_objects(
                ObjectFilter::land().in_zone(Zone::Library),
                ChoiceCount::up_to(2),
                PlayerFilter::You,
                "searched_0",
            ),
            Effect::for_each_tagged(
                "searched_0",
                vec![Effect::put_onto_battlefield(
                    ChooseSpec::Iterated,
                    true,
                    PlayerFilter::You,
                )],
            ),
            Effect::shuffle_library_player(PlayerFilter::You),
        ]);

        effect.execute(&mut game, &mut ctx).unwrap();

        assert!(
            game.player(alice).unwrap().library.is_empty(),
            "chosen cards should leave the library"
        );
        assert_eq!(game.battlefield.len(), 2);
        for object_id in game.battlefield.clone() {
            assert!(game.is_tapped(object_id));
            assert_eq!(game.object(object_id).unwrap().zone, Zone::Battlefield);
        }
    }

    #[test]
    fn test_put_onto_battlefield_clone_box() {
        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PutOntoBattlefieldEffect"));
    }

    #[test]
    fn test_put_onto_battlefield_get_target_spec() {
        let targeted_effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            false,
        );
        assert!(targeted_effect.get_target_spec().is_some());

        let non_target_effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        assert!(non_target_effect.get_target_spec().is_none());
    }
}

#[cfg(test)]
mod replacement_battlefield_owner_contract_tests {
    use super::*;
    use crate::ids::{CardId, ObjectId, PlayerId, StableId};
    use crate::target::ObjectFilter;
    use crate::filter::ObjectFilterExt as _;
    use crate::types::CardType;
    use crate::zone::Zone;
    use crate::effect::{Effect, Value};
    use crate::replacement::{ReplacementEffect, ReplacementAction};
    struct Answers { originals: Vec<StableId>, destination: Zone, pause: bool, pending: bool, calls: usize }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            assert!(self.originals.iter().all(|stable| game.find_object_by_stable_id(*stable)
                .is_some_and(|id| game.object(id).unwrap().zone == self.destination)),
                "replacement additions must see the entire original move batch committed");
            self.pending = self.pause;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, name: &str, owner: crate::ids::PlayerId, zone: Zone) -> ObjectId {
        let card = crate::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
        game.create_object_from_card(&card, owner, zone)
    }
    fn check(return_all: bool, mode: u8) { check_controller(return_all, mode, false); }
    fn check_controller(return_all: bool, mode: u8, other_controller: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let parent = card(&mut game, "Battlefield parent", alice, Zone::Battlefield);
        let replacement_source = card(&mut game, "Battlefield replacement", bob, Zone::Battlefield);
        let first = card(&mut game, "Move first", alice, Zone::Graveyard);
        let second = card(&mut game, "Move second", alice, Zone::Graveyard);
        let originals = vec![first, second];
        let stable = originals.iter().map(|id| game.object(*id).unwrap().stable_id).collect::<Vec<_>>();
        let destination = Zone::Battlefield;
        let effects = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(
                crate::object::CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it")))],
            _ => vec![Effect::may(vec![Effect::gain_life(7)])],
        };
        let action = if mode == 4 { ReplacementAction::Prevent } else {ReplacementAction::Additionally(effects)};
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement_source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(first), Some(Zone::Graveyard), Some(destination)), action));
        let snapshots = originals.iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect::<Vec<_>>();
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let before_ids = game.next_object_id_counter(); game.take_pending_trigger_events();
        let mut dm = Answers { originals: stable.clone(), destination, pause: mode == 2, pending: false, calls: 0 };
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
        ctx.set_tagged_objects("selected", snapshots); ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let effect = if return_all {
            Effect::new(crate::effects::ReturnAllToBattlefieldEffect::new(ObjectFilter::creature()
                .in_zone(Zone::Graveyard).owned_by(crate::target::PlayerFilter::You), true))
        } else { Effect::new(PutOntoBattlefieldEffect::new(ChooseSpec::tagged("selected"), true,
            if other_controller { crate::target::PlayerFilter::Specific(bob) } else { crate::target::PlayerFilter::You })) };
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); }
        else {
            let outcome = result.unwrap(); assert_eq!(outcome.output_objects().len(), 2);
            if return_all { assert_eq!(outcome.count_or_zero(), 2); }
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, if mode == 0 {27} else {20});
            let arrived = game.find_object_by_stable_id(stable[0]).unwrap();
            if mode == 3 {
                assert_eq!(game.counter_count(arrived, crate::object::CounterType::PlusOnePlusOne), 1);
                assert_eq!(game.counter_count(parent, crate::object::CounterType::PlusOnePlusOne), 0);
            }
            if mode == 4 { assert_eq!(game.object(first).unwrap().zone, Zone::Graveyard); }
            let second_arrived = game.find_object_by_stable_id(stable[1]).unwrap();
            assert_eq!(game.object(second_arrived).unwrap().zone, destination);
            assert_eq!(game.object(second_arrived).unwrap().owner, alice);
            assert_eq!(game.current_controller(second_arrived), Some(if other_controller { bob } else { alice }));
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, sentinel.object_id);
        if mode == 1 || mode == 2 {
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert_eq!(game.player(alice).unwrap().life, 20); assert_eq!(game.player(bob).unwrap().life, 20);
            for id in &originals { assert_eq!(game.object(*id).unwrap().zone, Zone::Graveyard); }
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            assert!(game.take_pending_trigger_events().is_empty());
        }
        if mode == 2 {
            drop(ctx); assert_eq!(dm.calls, 1); dm.pause = false; dm.pending = false;
            let snapshots = originals.iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect::<Vec<_>>();
            let mut ctx = ExecutionContext::new(parent, alice, &mut dm); ctx.set_tagged_objects("selected", snapshots);
            let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            assert_eq!(outcome.output_objects().len(), 2); assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.calls, 2);
        }
    }
    #[test] fn put_addition_sees_whole_batch() { check(false,0); }
    #[test] fn put_addition_error_restores_owner() { check(false,1); }
    #[test] fn put_addition_pending_replays_owner() { check(false,2); }
    #[test] fn put_addition_binds_arrival() { check(false,3); }
    #[test] fn return_all_addition_sees_whole_batch() { check(true,0); }
    #[test] fn return_all_addition_error_restores_owner() { check(true,1); }
    #[test] fn return_all_addition_pending_replays_owner() { check(true,2); }
    #[test] fn return_all_addition_binds_arrival() { check(true,3); }
    // UNRUN: explicit destination controller is retained by the same native
    // proposal through replacement completion, rollback, and resumed entry.
    #[test] fn relative_controller_addition_sees_whole_batch() { check_controller(false,0,true); }
    #[test] fn relative_controller_late_error_restores_originals() { check_controller(false,1,true); }
    #[test] fn relative_controller_pending_replays_exact_entry() { check_controller(false,2,true); }
    #[test] fn relative_controller_addition_binds_arrival() { check_controller(false,3,true); }

    #[test]
    fn relative_entry_resource_failure_rolls_back_every_original_and_can_retry() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, "Entry source", alice, Zone::Battlefield);
        let originals = [card(&mut game, "First arrival", alice, Zone::Graveyard),
            card(&mut game, "Second arrival", alice, Zone::Graveyard)];
        let stable = originals.map(|id| game.object(id).unwrap().stable_id);
        let mut shields = Vec::new();
        for original in originals {
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(original), Some(Zone::Graveyard), Some(Zone::Battlefield)),
                ReplacementAction::Additionally(vec![Effect::new(crate::effects::InvestigateEffect::you(1))]),
            )));
        }
        let snapshots = originals.iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("entries", snapshots);
        let effect = Effect::new(PutOntoBattlefieldEffect::new(ChooseSpec::tagged("entries"), true,
            crate::target::PlayerFilter::Specific(bob)));
        game.set_token_creation_limits(crate::effects::tokens::resources::TokenCreationLimits {
            max_created_tokens: 1, ..Default::default()
        });
        let before_ids = game.next_object_id_counter();
        game.take_pending_trigger_events();
        assert!(matches!(crate::effects::execute_effect(&mut game, &effect, &mut ctx),
            Err(ExecutionError::ResourceLimitExceeded { .. })));
        assert_eq!(game.next_object_id_counter(), before_ids);
        assert!(originals.iter().all(|id| game.object(*id).unwrap().zone == Zone::Graveyard));
        assert!(shields.iter().all(|id| game.effect_store.replacement_effects.get_effect(*id).is_some()));
        assert!(game.take_pending_trigger_events().is_empty());
        game.set_token_creation_limits(crate::effects::tokens::resources::TokenCreationLimits {
            max_created_tokens: 2, ..Default::default()
        });
        crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        for original in stable {
            let arrived = game.find_object_by_stable_id(original).unwrap();
            assert_eq!(game.current_controller(arrived), Some(bob));
            assert_eq!(game.object(arrived).unwrap().owner, alice);
            assert!(game.is_tapped(arrived));
        }
        assert!(shields.iter().all(|id| game.effect_store.replacement_effects.get_effect(*id).is_none()));
    }

    #[test]
    fn source_zone_qualification_is_an_exact_reference_without_a_selection_prompt() {
        struct NoObjectChoice;
        impl crate::decision::DecisionMaker for NoObjectChoice {
            fn decide_objects(&mut self, _: &GameState, _: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
                panic!("the source reference is fixed, not a choice from its graveyard");
            }
        }
        for zone in [Zone::Graveyard, Zone::Hand] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId(0); let bob = PlayerId(1);
            let source = card(&mut game, "Same name", alice, zone);
            let decoy = card(&mut game, "Same name", alice, Zone::Graveyard);
            let stable = game.object(source).unwrap().stable_id;
            let filter = ObjectFilter::source().in_zone(Zone::Graveyard);
            let effect = Effect::new(PutOntoBattlefieldEffect::new(ChooseSpec::Object(filter), false,
                crate::target::PlayerFilter::Specific(bob)));
            let mut dm = NoObjectChoice;
            let mut ctx = ExecutionContext::new(source, bob, &mut dm);
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, if zone == Zone::Graveyard { Zone::Battlefield } else { Zone::Hand });
            assert_eq!(game.object(decoy).unwrap().zone, Zone::Graveyard);
            if zone == Zone::Graveyard {
                assert_eq!(game.current_controller(current), Some(bob));
                assert_eq!(game.object(current).unwrap().owner, alice);
                // The old source reference cannot follow another incarnation.
                let hand = game.move_object_by_effect(current, Zone::Hand).unwrap();
                let grave = game.move_object_by_effect(hand, Zone::Graveyard).unwrap();
                crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
                assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
            }
        }
    }

    #[test]
    fn missing_player_or_object_binding_returns_a_checked_error_without_moving_cards() {
        for missing_player in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId(0);
            let source = card(&mut game, "Bound source", alice, Zone::Graveyard);
            let mut missing_collection = ObjectFilter::exact_tagged("missing").in_zone(Zone::Graveyard);
            missing_collection.match_captured_public_destination = true;
            let effect = Effect::new(PutOntoBattlefieldEffect::new(
                if missing_player { ChooseSpec::Source } else { ChooseSpec::Object(missing_collection) }, false,
                if missing_player { crate::target::PlayerFilter::AliasedTarget(Box::new(crate::target::PlayerFilter::Opponent)) }
                    else { crate::target::PlayerFilter::You }));
            let before_ids = game.next_object_id_counter();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
            assert!(matches!(result, Err(ExecutionError::InvalidTarget | ExecutionError::IncompleteEvidence(_))));
            assert_eq!(game.object(source).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.next_object_id_counter(), before_ids);
        }
    }
}
