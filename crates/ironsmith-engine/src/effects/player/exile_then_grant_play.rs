//! Exile a chosen object, then grant permission to cast or play it from exile.

use crate::effects::CompletedEffectOutputs;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_single_object_for_effect};
use crate::effects::player::grant_by_spec::next_turn_number_for_player;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::{GrantDuration, Grantable};
use crate::grant_registry::GrantSource;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileThenGrantPlayEffect {
    pub target: ChooseSpec,
    pub player: PlayerFilter,
    pub duration: GrantDuration,
    pub available_starting_next_turn: bool,
}

impl ExileThenGrantPlayEffect {
    pub fn new(target: ChooseSpec, player: PlayerFilter, duration: GrantDuration) -> Self {
        Self {
            target,
            player,
            duration,
            available_starting_next_turn: false,
        }
    }

    pub fn starting_next_turn(mut self) -> Self {
        self.available_starting_next_turn = true;
        self
    }
}

impl EffectExecutor for ExileThenGrantPlayEffect {
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
                // A delayed effect tracks the original object, not the card's new
                // incarnation after a zone change. There is nothing left to exile.
                let Some(from_zone) = game.object(target_id).map(|obj| obj.zone) else {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                let player = resolve_player_filter(game, &self.player, ctx)?;
                let expires = match self.duration {
                    GrantDuration::UntilEndOfTurn => game.turn.turn_number,
                    GrantDuration::Forever | GrantDuration::UntilYourNextTurn => u32::MAX,
                    GrantDuration::UntilYourNextTurnEnd => {
                        next_turn_number_for_player(game, player)
                    }
                };
                let request = crate::effects::zones::PreparedZoneMove::capture(
                    game,
                    target_id,
                    from_zone,
                    Zone::Exile,
                    ctx.cause.clone(),
                    None,
                );
                crate::effects::zones::execute_zone_moves_with_outputs(
                    game,
                    ctx,
                    vec![request],
                    |game, ctx, receipts| {
                        let arrived = crate::effects::zones::movement_arrivals(
                            game,
                            target_id,
                            &receipts[0].1,
                        );
                        // Grant only to this instruction's actual arrivals still in exile;
                        // never follow a later object incarnation by stable identity.
                        let exiled_ids = arrived
                            .into_iter()
                            .filter(|id| {
                                game.object(*id)
                                    .is_some_and(|card| card.zone == Zone::Exile)
                            })
                            .collect::<Vec<_>>();

                        for &exiled_id in &exiled_ids {
                            let grant_source = match self.duration {
                                GrantDuration::UntilYourNextTurn => {
                                    GrantSource::until_player_next_turn_start(
                                        ctx.source,
                                        player,
                                        game.turn.turn_number,
                                    )
                                }
                                GrantDuration::UntilYourNextTurnEnd => {
                                    GrantSource::until_player_next_turn_end(
                                        ctx.source, player, expires,
                                    )
                                }
                                GrantDuration::UntilEndOfTurn | GrantDuration::Forever => {
                                    GrantSource::Effect {
                                        source_id: ctx.source,
                                        expires_end_of_turn: expires,
                                    }
                                }
                            };
                            if self.available_starting_next_turn {
                                game.effect_store
                                    .grant_registry
                                    .grant_to_card_starting_on_turn(
                                        exiled_id,
                                        Zone::Exile,
                                        player,
                                        Grantable::PlayFrom,
                                        game.turn.turn_number.saturating_add(1),
                                        grant_source,
                                    );
                            } else {
                                game.effect_store.grant_registry.grant_to_card(
                                    exiled_id,
                                    Zone::Exile,
                                    player,
                                    Grantable::PlayFrom,
                                    grant_source,
                                );
                            }
                        }

                        let original = if exiled_ids.is_empty() {
                            EffectOutcome::count(0)
                        } else {
                            EffectOutcome::with_objects(exiled_ids)
                        };
                        // Permissions are part of the original compound instruction.
                        Ok(original)
                    },
                )
            },
        );
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to exile and grant play permission to"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    #[test]
    fn exiled_card_can_be_played_through_the_players_next_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(1), "Next-turn Exile")
            .card_types(vec![CardType::Sorcery])
            .build();
        let hand_id = game.create_object_from_card(&card, alice, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(hand_id)];
        ExileThenGrantPlayEffect::new(
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
            PlayerFilter::You,
            GrantDuration::UntilYourNextTurnEnd,
        )
        .execute(&mut game, &mut ctx)
        .expect("the next-turn duration must be executable");

        let exiled_id = *game.exile.last().expect("the card should be exiled");
        let grants = game.effect_store.grant_registry.get_grants_for_card(
            &game,
            exiled_id,
            Zone::Exile,
            alice,
        );
        assert_eq!(grants.len(), 1);
        assert_eq!(
            grants[0].source,
            GrantSource::until_player_next_turn_end(source, alice, 3)
        );

        game.turn.turn_number = 3;
        assert_eq!(
            game.effect_store
                .grant_registry
                .get_grants_for_card(&game, exiled_id, Zone::Exile, alice,)
                .len(),
            1,
        );
        game.turn.turn_number = 4;
        assert!(
            game.effect_store
                .grant_registry
                .get_grants_for_card(&game, exiled_id, Zone::Exile, alice,)
                .is_empty()
        );
    }
}

#[cfg(test)]
mod replacement_exile_grant_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId, StableId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    struct Answers { target: StableId, permission_player: PlayerId, pause: bool, pending: bool, calls: usize }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            let arrival = game.find_object_by_stable_id(self.target).unwrap();
            assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
            assert!(game.effect_store.grant_registry.card_can_play_from_zone(game, arrival, Zone::Exile, self.permission_player),
                "the original exile's permission is complete before added actions");
            self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), "Exile grant fixture")
            .card_types(vec![CardType::Artifact]).build(), owner, zone)
    }
    fn check(mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let parent = card(&mut game, alice, Zone::Battlefield);
        let source = card(&mut game, bob, Zone::Battlefield);
        let target = card(&mut game, alice, Zone::Hand);
        let stable = game.object(target).unwrap().stable_id;
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let effects = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it"))),
                Effect::may(vec![Effect::gain_life(0)])],
            _ => vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])],
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(target), Some(Zone::Hand), Some(Zone::Exile)),
            ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events(); let before_ids = game.next_object_id_counter();
        let mut dm = Answers { target: stable, permission_player: alice, pause: mode == 2, pending: false, calls: 0 };
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm); ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let effect = ExileThenGrantPlayEffect::new(ChooseSpec::SpecificObject(target), PlayerFilter::You, GrantDuration::Forever);
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(result.unwrap().events.is_empty()); }
        else {
            let outcome = result.unwrap(); assert_eq!(outcome.objects().unwrap().len(), 1);
            let arrival = outcome.objects().unwrap()[0]; assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
            assert!(game.effect_store.grant_registry.card_can_play_from_zone(&game, arrival, Zone::Exile, alice));
            assert!(!game.effect_store.grant_registry.card_can_play_from_zone(&game, arrival, Zone::Exile, bob));
            assert_eq!(game.player(alice).unwrap().life, 20); assert_eq!(game.player(bob).unwrap().life, if mode == 3 {20} else {27});
            if mode == 3 {
                assert_eq!(game.counter_count(arrival, CounterType::PlusOnePlusOne), 1);
                assert!(outcome.execution_facts.iter().filter_map(|fact| match fact { crate::effect::ExecutionFact::AffectedObjectMemory(memory) => Some(memory.as_slice()), _ => None }).flatten().any(|memory| memory.object_id == arrival && memory.zone == Zone::Exile));
                assert!(!outcome.affected_object_memory().unwrap_or(&[]).iter().any(|memory| memory.object_id == arrival && memory.zone == Zone::Exile), "auxiliary post-move counter memory is not original movement memory");
            } else {
                assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                    .map(|event| (event.player,event.amount)).collect::<Vec<_>>(), vec![(bob,3),(bob,4)]);
            }
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source, parent); assert_eq!(ctx.controller, alice);
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, sentinel.object_id);
        assert_eq!(game.counter_count(parent, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            assert_eq!(game.next_object_id_counter(), before_ids); assert_eq!(game.object(target).unwrap().zone, Zone::Hand);
            assert!(game.exile.is_empty()); assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode == 0 || mode == 3 { assert_eq!(dm.calls, 1); }
        if mode == 2 {
            assert_eq!(dm.calls, 1); dm.pause = false; dm.pending = false;
            let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap(); assert_eq!(outcome.objects().unwrap().len(), 1);
            assert_eq!(game.player(bob).unwrap().life, 27); assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.calls, 2);
        }
    }
    #[test] fn additions_see_permissions_and_preserve_original_summary() { check(0); }
    #[test] fn error_restores_exile_permissions_and_resources() { check(1); }
    #[test] fn pending_replays_entire_exile_and_grant() { check(2); }
    #[test] fn addition_binds_arrival_and_returns_counter_facts() { check(3); }
}
