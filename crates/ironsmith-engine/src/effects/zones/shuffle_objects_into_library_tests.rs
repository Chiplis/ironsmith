//! Source-authored shuffle transaction scenarios; execution is deferred.
use super::*;
use crate::effect::{Effect, Value};
use crate::effects::{ForPlayersEffect, ResolvedTarget};
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ObjectFilter;

fn card(game: &mut GameState, player: PlayerId, zone: Zone, name: &str) -> ObjectId {
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![crate::types::CardType::Creature]).build();
    game.create_object_from_card(&card, player, zone)
}

struct ObserveAddedProgram {
    random_before: u64,
    pending: bool,
    suspend: bool,
    players: Vec<PlayerId>,
}
impl crate::decision::DecisionMaker for ObserveAddedProgram {
    fn decide_boolean(&mut self, game: &GameState, context: &crate::decisions::context::BooleanContext) -> bool {
        assert_eq!(game.irreversible_random_count(), self.random_before + 2,
            "both original libraries were randomized before any added program");
        for player in [PlayerId(0), PlayerId(1)] {
            assert!(game.player(player).unwrap().graveyard.is_empty());
            assert_eq!(game.player(player).unwrap().library.len(), 3);
        }
        self.players.push(context.player);
        self.pending = self.suspend;
        !self.suspend
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

#[test]
fn simultaneous_shuffle_originals_precede_added_programs_and_retry_atomically() {
    for suspend in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        game.turn.active_player = bob;
        let source = card(&mut game, alice, Zone::Battlefield, "Shuffle source");
        let mut originals = Vec::new();
        let mut shields = Vec::new();
        for player in [alice, bob] {
            for _ in 0..2 { card(&mut game, player, Zone::Library, "Library"); }
            let object = card(&mut game, player, Zone::Graveyard, "Grave");
            originals.push(object);
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(source, player,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(object), Some(Zone::Graveyard), Some(Zone::Library),
                    ), ReplacementAction::Additionally(vec![Effect::may(vec![Effect::gain_life(2)])])),
            ));
        }
        let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(
            ShuffleObjectsIntoLibraryEffect::new(
                ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::IteratedPlayer)), PlayerFilter::IteratedPlayer,
            ),
        )]);
        game.take_pending_trigger_events();
        let random_before = game.irreversible_random_count();
        let next_id = game.next_object_id_counter();
        let mut dm = ObserveAddedProgram { random_before, pending: false, suspend, players: Vec::new() };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(ctx.decision_maker.awaiting_choice(), suspend);
        drop(ctx);
        if suspend {
            assert!(outcome.events.is_empty());
            assert_eq!(dm.players, vec![bob]);
            assert_eq!(game.irreversible_random_count(), random_before);
            assert_eq!(game.next_object_id_counter(), next_id);
            assert!(game.take_pending_trigger_events().is_empty());
            assert!(originals.iter().all(|id| game.object(*id).is_some_and(|o| o.zone == Zone::Graveyard)));
            assert!(shields.iter().all(|id| game.effect_store.replacement_effects.get_effect(*id).is_some()));
            dm.pending = false; dm.suspend = false; dm.players.clear();
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            effect.execute(&mut game, &mut ctx).unwrap();
        } else {
            assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<ShuffleLibraryEvent>())
                .map(|e| e.player).collect::<Vec<_>>(), vec![bob, alice]);
        }
        assert_eq!(dm.players, vec![bob, alice]);
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert_eq!(game.player(bob).unwrap().life, 22);
        assert!(shields.iter().all(|id| game.effect_store.replacement_effects.get_effect(*id).is_none()));
    }
}

#[test]
fn appended_shuffle_failure_restores_randomization_objects_and_replacement_resources() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let original = card(&mut game, alice, Zone::Graveyard, "Original");
    for _ in 0..3 { card(&mut game, alice, Zone::Library, "Library"); }
    let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            ObjectFilter::specific(original), Some(Zone::Graveyard), Some(Zone::Library),
        ), ReplacementAction::Additionally(vec![Effect::gain_life(2), Effect::lose_life(Value::X)]),
    ));
    let before_library = game.player(alice).unwrap().library.clone();
    let before_random = game.irreversible_random_count();
    let before_ids = game.next_object_id_counter();
    game.take_pending_trigger_events();
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.targets = vec![ResolvedTarget::Object(original)];
    let effect = ShuffleObjectsIntoLibraryEffect::new(
        ChooseSpec::Object(ObjectFilter::specific(original).in_zone(Zone::Graveyard)), PlayerFilter::You,
    );
    assert!(matches!(effect.execute(&mut game, &mut ctx), Err(ExecutionError::UnresolvableValue(_))));
    assert_eq!(game.player(alice).unwrap().library, before_library);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.object(original).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.irreversible_random_count(), before_random);
    assert_eq!(game.next_object_id_counter(), before_ids);
    assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn direct_shuffle_shares_one_resource_budget_across_added_programs() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let originals = (0..2).map(|_| card(&mut game, alice, Zone::Graveyard, "Grave")).collect::<Vec<_>>();
    for _ in 0..2 { card(&mut game, alice, Zone::Library, "Library"); }
    let mut shields = Vec::new();
    for original in &originals {
        shields.push(game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(*original), Some(Zone::Graveyard), Some(Zone::Library),
            ), ReplacementAction::Additionally(vec![Effect::new(crate::effects::InvestigateEffect::you(1))]),
        )));
    }
    game.set_token_creation_limits(crate::effects::tokens::resources::TokenCreationLimits {
        max_created_tokens: 1, ..Default::default()
    });
    let before_ids = game.next_object_id_counter();
    let before_library = game.player(alice).unwrap().library.clone();
    let before_random = game.irreversible_random_count();
    game.take_pending_trigger_events();
    let effect = ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::all(
        ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(PlayerFilter::You)), PlayerFilter::You);
    let mut ctx = ExecutionContext::new_default(source, alice);
    assert!(matches!(effect.execute(&mut game, &mut ctx), Err(ExecutionError::ResourceLimitExceeded { .. })));
    assert_eq!(game.battlefield, vec![source]);
    assert_eq!(game.next_object_id_counter(), before_ids);
    assert_eq!(game.player(alice).unwrap().library, before_library);
    assert_eq!(game.irreversible_random_count(), before_random);
    assert!(originals.iter().all(|id| game.object(*id).unwrap().zone == Zone::Graveyard));
    assert!(shields.iter().all(|id| game.effect_store.replacement_effects.get_effect(*id).is_some()));
    assert!(game.take_pending_trigger_events().is_empty());
    game.set_token_creation_limits(crate::effects::tokens::resources::TokenCreationLimits {
        max_created_tokens: 2, ..Default::default()
    });
    effect.execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.battlefield.len(), 3);
    assert_eq!(game.player(alice).unwrap().library.len(), 4);
}

#[test]
fn each_physical_shuffle_keeps_a_distinct_history_occurrence() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    for player in [alice, bob] {
        card(&mut game, player, Zone::Graveyard, "Grave");
        card(&mut game, player, Zone::Library, "Library");
    }
    let parent = game.alloc_child_event_provenance(crate::provenance::ProvNodeId::default(),
        crate::events::EventKind::ShuffleLibrary);
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.provenance = parent;
    let effect = ShuffleObjectsIntoLibraryEffect::new(
        ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Graveyard)),
        PlayerFilter::OwnerOf(crate::target::ObjectRef::Target),
    ).with_owner_library_destination();
    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
    let shuffles = outcome.events.iter().filter(|event| event.downcast::<ShuffleLibraryEvent>().is_some()).collect::<Vec<_>>();
    assert_eq!(shuffles.len(), 2);
    assert_ne!(shuffles[0].provenance(), shuffles[1].provenance());
    for event in &shuffles { game.stage_turn_history_event(event); }
    let staged = game.turn_store.turn_history.staged_event_records.iter()
        .filter_map(|record| record.event.downcast::<ShuffleLibraryEvent>()).count();
    assert_eq!(staged, 2);
}

struct ObserveDrawBoundary {
    random_before: u64,
    original: ObjectId,
    controller: PlayerId,
    observations: usize,
    pending: bool,
    suspend_after_draw: bool,
}
impl crate::decision::DecisionMaker for ObserveDrawBoundary {
    fn decide_boolean(&mut self, game: &GameState, context: &crate::decisions::context::BooleanContext) -> bool {
        assert_eq!(context.player, self.controller, "replacement source controls its own choices");
        if self.observations == 0 {
            assert_eq!(game.irreversible_random_count(), self.random_before);
            assert!(game.object(self.original).is_some(), "non-draw prefix precedes other original moves");
        } else {
            assert_eq!(game.irreversible_random_count(), self.random_before + 2);
            assert!(game.object(self.original).is_none(), "both participants completed their originals before the draw");
            assert_eq!(game.player(self.controller).unwrap().hand.len(), 1);
            self.pending = self.suspend_after_draw;
        }
        self.observations += 1;
        !self.pending
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

#[test]
fn replaced_shuffle_draw_keeps_prefix_scope_and_waits_for_all_original_libraries() {
    // Plain Instead, typed exile-followup, and a nested typed move whose
    // generated exile is itself replaced all cross the same draw boundary.
    for mode in 0..9 {
        for suspend in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId(0); let bob = PlayerId(1);
            let source = card(&mut game, bob, Zone::Battlefield, "Replacement source");
            let replaced = card(&mut game, alice, Zone::Graveyard, "Replaced grave");
            let original = card(&mut game, bob, Zone::Graveyard, "Other original");
            for player in [alice, bob] { for _ in 0..3 { card(&mut game, player, Zone::Library, "Library"); } }
            let payload = vec![Effect::may(vec![Effect::gain_life(2)]), Effect::draw(1),
                Effect::may(vec![Effect::gain_life(3)])];
            let outer = if mode == 7 { ReplacementAction::ChangeDestination(Zone::Battlefield) }
                else if mode == 8 { ReplacementAction::Instead(vec![Effect::move_to_zone(
                    ChooseSpec::tagged("it"), Zone::Battlefield, false)]) }
                else if mode == 0 { ReplacementAction::Instead(payload.clone()) }
                else if mode == 1 { ReplacementAction::ExileWithSourceLinkThen(payload.clone()) }
                else if mode == 2 { ReplacementAction::ExileWithSourceLinkThen(vec![Effect::gain_life(5)]) }
                else if mode == 3 { ReplacementAction::Instead(vec![Effect::move_to_zone(
                    ChooseSpec::tagged("it"), Zone::Exile, false)]) }
                else if mode == 4 { ReplacementAction::Instead(vec![Effect::exile(ChooseSpec::tagged("it"))]) }
                else {
                    let filter = ObjectFilter::default().in_zone(Zone::Graveyard)
                        .owned_by(PlayerFilter::IteratedPlayer);
                    let movement = if mode == 5 { Effect::move_to_zone(ChooseSpec::all(filter), Zone::Exile, false) }
                        else { Effect::new(crate::effects::ExileEffect::all(filter)) };
                    ReplacementAction::Instead(vec![Effect::new(ForPlayersEffect::new(PlayerFilter::Any, vec![movement]))])
                };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Library)), outer,
            ));
            let inner = (mode >= 2).then(|| {
                let replacement = if mode >= 7 {
                    ReplacementEffect::with_matcher(source, bob,
                        crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(ObjectFilter::specific(replaced)),
                        ReplacementAction::Instead(payload.clone()))
                } else {
                    ReplacementEffect::with_matcher(source, bob,
                        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                            ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Exile)),
                        ReplacementAction::Instead(payload.clone()))
                };
                game.effect_store.replacement_effects.add_one_shot_effect(replacement)
            });
            game.take_pending_trigger_events();
            let before_random = game.irreversible_random_count();
            let before_ids = game.next_object_id_counter();
            let mut dm = ObserveDrawBoundary { random_before: before_random, original, controller: bob,
                observations: 0, pending: false, suspend_after_draw: suspend };
            let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(
                ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::all(ObjectFilter::default()
                    .in_zone(Zone::Graveyard).owned_by(PlayerFilter::IteratedPlayer)), PlayerFilter::IteratedPlayer),
            )]);
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(ctx.source, source); assert_eq!(ctx.controller, alice);
            assert_eq!(ctx.decision_maker.awaiting_choice(), suspend);
            drop(ctx);
            assert_eq!(dm.observations, 2);
            if suspend {
                assert_eq!(game.irreversible_random_count(), before_random);
                assert_eq!(game.next_object_id_counter(), before_ids);
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert!(game.player(bob).unwrap().hand.is_empty());
                assert_eq!(game.object(replaced).unwrap().zone, Zone::Graveyard);
                assert_eq!(game.object(original).unwrap().zone, Zone::Graveyard);
                assert!(outcome.events.is_empty());
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
                assert!(inner.is_none_or(|id| game.effect_store.replacement_effects.get_effect(id).is_some()));
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                assert_eq!(game.player(bob).unwrap().life, if mode == 2 { 30 } else { 25 });
                assert_eq!(game.player(bob).unwrap().library.len(), if matches!(mode, 5 | 6) { 2 } else { 3 });
                assert_eq!(game.player(bob).unwrap().hand.len(), 1);
                assert_eq!(game.player(alice).unwrap().library.len(), 3);
                assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>())
                    .filter(|e| e.amount == 2).count(), 1, "prefix appears in final evidence once");
            }
        }
    }
}

#[test]
fn native_single_and_for_players_shuffle_capture_suspended_life_prefix_once() {
    for multi in [false, true] {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0);
    let source = card(&mut game, alice, Zone::Battlefield, "Life observer");
    game.object_mut(source).unwrap().abilities_mut().push(crate::ability::Ability::triggered(
        crate::triggers::Trigger::you_gain_life(), vec![Effect::gain_life(1)],
    ));
    let original = card(&mut game, alice, Zone::Graveyard, "Original");
    for _ in 0..3 { card(&mut game, alice, Zone::Library, "Library"); }
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            ObjectFilter::specific(original), Some(Zone::Graveyard), Some(Zone::Library)),
        ReplacementAction::Instead(vec![Effect::gain_life(2)]),
    ));
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, alice, crate::events::WouldGainLifeMatcher::new(PlayerFilter::You),
        ReplacementAction::Additionally(vec![Effect::draw(1)]),
    ));
    game.take_pending_trigger_events();
    let mut ctx = ExecutionContext::new_default(source, alice);
    let participant = if multi { PlayerFilter::IteratedPlayer } else { PlayerFilter::You };
    let shuffle = ShuffleObjectsIntoLibraryEffect::new(
        ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(participant.clone())),
        participant,
    );
    let outcome = if multi {
        ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(shuffle)]).execute(&mut game, &mut ctx)
    } else { shuffle.execute(&mut game, &mut ctx) }.unwrap();
    assert_eq!(game.player(alice).unwrap().life, 22);
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(game.take_pending_trigger_entries().len(), 1,
        "the original gain's observer fires once across suspension and resumption");
    assert_eq!(game.turn_store.turn_history.total_life_gained_for_players(&[alice]), 2);
    assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).count(), 1);
}
}

#[test]
fn generic_movement_without_a_draw_keeps_the_remaining_prefix_before_shuffle() {
    for use_exile in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, bob, Zone::Battlefield, "Source");
        let replaced = card(&mut game, alice, Zone::Graveyard, "Replaced");
        let other = card(&mut game, bob, Zone::Graveyard, "Other original");
        for player in [alice, bob] { for _ in 0..2 { card(&mut game, player, Zone::Library, "Library"); } }
        let movement = if use_exile { Effect::exile(ChooseSpec::tagged("it")) }
            else { Effect::move_to_zone(ChooseSpec::tagged("it"), Zone::Exile, false) };
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Library)),
            ReplacementAction::Instead(vec![movement, Effect::may(vec![Effect::gain_life(2)])]),
        ));
        let before_random = game.irreversible_random_count();
        let mut dm = ObserveDrawBoundary { random_before: before_random, original: other, controller: bob,
            observations: 0, pending: false, suspend_after_draw: false };
        let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(
            ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::all(ObjectFilter::default()
                .in_zone(Zone::Graveyard).owned_by(PlayerFilter::IteratedPlayer)), PlayerFilter::IteratedPlayer),
        )]);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        effect.execute(&mut game, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(dm.observations, 1, "plain movement must not manufacture a draw boundary");
        assert_eq!(game.player(bob).unwrap().life, 22);
        assert_eq!(game.exile.len(), 1);
        assert_eq!(game.object(game.exile[0]).unwrap().name.as_ref(), "Replaced");
        assert_eq!(game.irreversible_random_count(), before_random + 2);
    }
}

#[test]
fn resumed_quantified_movement_keeps_each_players_result_for_the_next_draw() {
    for use_exile in [false, true] {
        for fails in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId(0); let bob = PlayerId(1);
            let source = card(&mut game, bob, Zone::Battlefield, "Source");
            let mut graves = Vec::new();
            for (player, count) in [(alice, 2), (bob, 3)] {
                for _ in 0..count { graves.push(card(&mut game, player, Zone::Graveyard, "Grave")); }
                for _ in 0..7 { card(&mut game, player, Zone::Library, "Library"); }
            }
            let filter = ObjectFilter::default().in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::IteratedPlayer);
            let movement = if use_exile { Effect::new(crate::effects::ExileEffect::all(filter)) }
                else { Effect::move_to_zone(ChooseSpec::all(filter), Zone::Exile, false) };
            let mut payload = vec![Effect::new(ForPlayersEffect::new(PlayerFilter::Any, vec![
                Effect::with_id(91, movement),
                Effect::target_draws(Value::EffectValue(crate::effect::EffectId(91)), PlayerFilter::IteratedPlayer),
            ]))];
            if fails { payload.push(Effect::lose_life(Value::X)); }
            let outer = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(graves[0]), Some(Zone::Graveyard), Some(Zone::Library)),
                ReplacementAction::Instead(payload),
            ));
            let inner = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(graves[0]), Some(Zone::Graveyard), Some(Zone::Exile)),
                ReplacementAction::Instead(vec![Effect::draw(1)]),
            ));
            game.take_pending_trigger_events();
            let before_random = game.irreversible_random_count();
            let before_ids = game.next_object_id_counter();
            let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(
                ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::all(ObjectFilter::default()
                    .in_zone(Zone::Graveyard).owned_by(PlayerFilter::IteratedPlayer)), PlayerFilter::IteratedPlayer),
            )]);
            let mut ctx = ExecutionContext::new_default(source, alice);
            let result = effect.execute(&mut game, &mut ctx);
            if fails {
                assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
                assert_eq!(game.irreversible_random_count(), before_random);
                assert_eq!(game.next_object_id_counter(), before_ids);
                assert!(graves.iter().all(|id| game.object(*id).unwrap().zone == Zone::Graveyard));
                assert!(game.exile.is_empty());
                assert!(game.players.iter().all(|player| player.hand.is_empty() && player.library.len() == 7));
                assert!(game.effect_store.replacement_effects.get_effect(outer).is_some());
                assert!(game.effect_store.replacement_effects.get_effect(inner).is_some());
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                result.unwrap();
                assert_eq!(game.player(alice).unwrap().hand.len(), 1,
                    "Alice draws for the one card her original movement actually moved");
                assert_eq!(game.player(bob).unwrap().hand.len(), 4,
                    "Bob keeps the replacement draw plus his own three-card result");
                assert_eq!(game.exile.len(), 4);
                assert_eq!(game.object(graves[0]).unwrap().zone, Zone::Graveyard);
                assert_eq!(game.irreversible_random_count(), before_random + 2);
                assert!(!ctx.effect_outcomes.contains_key(&crate::effect::EffectId(91)),
                    "replacement-local counts do not leak into the enclosing shuffle context");
            }
        }
    }
}

#[derive(Debug, Clone)]
struct WhileOtherGraveRemains { target: ObjectId, other: ObjectId }
impl crate::events::ReplacementMatcher for WhileOtherGraveRemains {
    fn matches_prepared_event(&self, event: &dyn crate::events::GameEventType,
        context: &crate::events::context::PreparedEventContext) -> bool {
        crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event).is_some_and(|event|
            event.objects == vec![self.target] && event.from == Zone::Graveyard && event.to == Zone::Exile)
            && context.game.object(self.other).is_some_and(|object| object.zone == Zone::Graveyard)
    }
    fn display(&self) -> String { "While the other original remains in its graveyard".into() }
}

#[test]
fn simultaneous_exile_prepares_every_replacement_before_original_movements() {
    for use_move in [false, true] {
    for quantified in [false, true] {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let first = card(&mut game, alice, Zone::Graveyard, "First original");
    let second = card(&mut game, bob, Zone::Graveyard, "Second original");
    let second_identity = game.object(second).unwrap().stable_id;
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, alice, WhileOtherGraveRemains { target: second, other: first },
        ReplacementAction::ChangeDestination(Zone::Library),
    ));
    let filter = ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(
        if quantified { PlayerFilter::IteratedPlayer } else { PlayerFilter::Any });
    let action = if use_move { Effect::move_to_zone(ChooseSpec::all(filter), Zone::Exile, false) }
        else { Effect::new(crate::effects::ExileEffect::all(filter)) };
    let mut ctx = ExecutionContext::new_default(source, alice);
    if quantified { ForPlayersEffect::new(PlayerFilter::Any, vec![action]).execute(&mut game, &mut ctx).unwrap(); }
    else { crate::effects::execute_effect(&mut game, &action, &mut ctx).unwrap(); }
    assert_eq!(game.exile.len(), 1);
    let moved = game.find_object_by_stable_id(second_identity).unwrap();
    assert_eq!(game.object(moved).unwrap().zone, Zone::Library,
        "the second replacement qualified before the first original left its graveyard");
}
}
}

struct PauseHiddenSelection { asks: usize, pending: bool }
impl crate::decision::DecisionMaker for PauseHiddenSelection {
    fn decide_objects(&mut self, _: &GameState, context: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(context.reveal_policy, crate::decisions::context::SelectionRevealPolicy::Public);
        assert!(context.require_explicit_choice);
        self.asks += 1; self.pending = true; Vec::new()
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
#[test]
fn native_exile_preflight_waits_for_hidden_identity_before_freezing_participants() {
    for use_move in [false, true] {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let first = card(&mut game, alice, Zone::Hand, "Known creature");
    let hidden = game.create_hidden_card_placeholder(bob, Zone::Hand, 0, "slot-0".into());
    let filter = ObjectFilter::creature().in_zone(Zone::Hand).owned_by(PlayerFilter::IteratedPlayer);
    let action = if use_move { Effect::move_to_zone(ChooseSpec::all(filter), Zone::Exile, false) }
        else { Effect::new(crate::effects::ExileEffect::all(filter)) };
    let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![action]);
    game.take_pending_trigger_events();
    let before_ids = game.next_object_id_counter();
    let mut dm = PauseHiddenSelection { asks: 0, pending: false };
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert!(outcome.events.is_empty());
    drop(ctx);
    assert_eq!(dm.asks, 1);
    assert_eq!(game.object(first).unwrap().zone, Zone::Hand);
    assert_eq!(game.object(hidden).unwrap().zone, Zone::Hand);
    assert!(!game.is_publicly_revealed_hidden_card(hidden));
    assert!(game.exile.is_empty());
    assert_eq!(game.next_object_id_counter(), before_ids);
}
}

#[test]
fn all_exile_participants_select_before_another_players_replacement_prefix() {
    for use_move in [false, true] {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let first = card(&mut game, alice, Zone::Graveyard, "First original");
    let second = card(&mut game, bob, Zone::Graveyard, "Second original");
    let extra = card(&mut game, bob, Zone::Exile, "Later graveyard card");
    let extra_identity = game.object(extra).unwrap().stable_id;
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            ObjectFilter::specific(first), Some(Zone::Graveyard), Some(Zone::Exile)),
        ReplacementAction::Instead(vec![Effect::move_to_zone(
            ChooseSpec::SpecificObject(extra), Zone::Graveyard, false)]),
    ));
    let filter = ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(PlayerFilter::IteratedPlayer);
    let action = if use_move { Effect::move_to_zone(ChooseSpec::all(filter), Zone::Exile, false) }
        else { Effect::new(crate::effects::ExileEffect::all(filter)) };
    let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![action]);
    let mut ctx = ExecutionContext::new_default(source, alice);
    effect.execute(&mut game, &mut ctx).unwrap();
    let extra = game.find_object_by_stable_id(extra_identity).unwrap();
    assert_eq!(game.object(extra).unwrap().zone, Zone::Graveyard,
        "a new candidate created by Alice's prefix is outside Bob's frozen original set");
    assert!(game.object(second).is_none());
    assert_eq!(game.exile.len(), 1);
}
}
