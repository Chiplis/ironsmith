//! Native iterator continuation scenarios. Authored for deferred execution.
use super::*;
use crate::effect::Value;
use crate::effects::{EffectExecutor, ForEachObject, ForEachTaggedEffect, ForPlayersEffect,
    ShuffleObjectsIntoLibraryEffect, TagMatchingObjectsEffect};
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::zone::Zone;

fn card(game: &mut GameState, player: PlayerId, zone: Zone, name: &str) -> ObjectId {
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![crate::types::CardType::Creature]).build();
    game.create_object_from_card(&card, player, zone)
}
fn snapshot(game: &GameState, object: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object(game.object(object).unwrap(), game)
}
fn iterator(tagged: bool, tag: &str, effects: Vec<Effect>) -> Effect {
    if tagged { Effect::new(ForEachTaggedEffect::new(tag, effects)) }
    else { Effect::new(ForEachObject::new(ObjectFilter::default()
        .match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject), effects)) }
}

#[derive(Debug, Clone)]
struct AssertBinding { source: ObjectId, controller: PlayerId, object: ObjectId, player: PlayerId, tagged: bool }
impl EffectExecutor for AssertBinding {
    fn execute(&self, _game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        assert_eq!(ctx.source, self.source);
        assert_eq!(ctx.controller, self.controller);
        assert_eq!(ctx.source_snapshot.as_ref().unwrap().object_id, self.source);
        assert_eq!(ctx.iteration.iterated_object, Some(self.object));
        assert_eq!(ctx.iteration.iterated_player, Some(self.player));
        assert_eq!(ctx.get_tagged_all("__it__").unwrap()[0].object_id, self.object);
        if self.tagged {
            assert!(ctx.get_tagged_all(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG).unwrap().is_empty());
        }
        Ok(EffectOutcome::count(0))
    }
}
struct ObserveBoundary {
    random: u64, other_original: ObjectId, controller: PlayerId,
    asks: usize, pause_at: Option<usize>, pending: bool,
}
impl crate::decision::DecisionMaker for ObserveBoundary {
    fn decide_boolean(&mut self, game: &GameState, context: &crate::decisions::context::BooleanContext) -> bool {
        assert_eq!(context.player, self.controller);
        if self.asks == 0 {
            assert_eq!(game.irreversible_random_count(), self.random);
            assert_eq!(game.object(self.other_original).unwrap().zone, Zone::Graveyard);
            assert!(game.player(self.controller).unwrap().hand.is_empty());
        } else {
            assert_eq!(self.asks, 1, "the completed prefix is never replayed");
            assert_eq!(game.irreversible_random_count(), self.random + 2);
            assert!(game.object(self.other_original).is_none());
            assert_eq!(game.player(self.controller).unwrap().hand.len(), 1);
        }
        self.pending = self.pause_at == Some(self.asks);
        self.asks += 1;
        !self.pending
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

#[test]
fn raw_object_and_tagged_replacement_loops_keep_scope_and_draw_after_original_shuffles() {
    for tagged in [false, true] {
    for depth in 0..=2 {
    for dispatched in [false, true] {
    for pause_at in [None, Some(0), Some(1)] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, alice, Zone::Battlefield, "Original source");
        let replacement_source = card(&mut game, bob, Zone::Battlefield, "Replacement source");
        let replaced = card(&mut game, alice, Zone::Graveyard, "Replaced");
        let other = card(&mut game, bob, Zone::Graveyard, "Other original");
        for player in [alice, bob] { for _ in 0..3 { card(&mut game, player, Zone::Library, "Library"); } }
        let binding = Effect::new(AssertBinding { source: replacement_source, controller: bob,
            object: replaced, player: alice, tagged });
        let middle = if depth > 0 { Effect::move_to_zone(ChooseSpec::Iterated, Zone::Exile, false) }
            else { Effect::draw(1) };
        let payload = iterator(tagged, "it", vec![binding.clone(), Effect::may(vec![Effect::gain_life(2)]),
            middle, binding, Effect::may(vec![Effect::gain_life(3)])]);
        let outer = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement_source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Library)),
            ReplacementAction::Instead(vec![payload]),
        ));
        let inner = (depth > 0).then(|| game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement_source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Exile)),
            ReplacementAction::Instead(if depth == 1 { vec![Effect::draw(1)] } else {
                vec![iterator(!tagged, "it", vec![Effect::move_to_zone(ChooseSpec::Iterated, Zone::Hand, false)])]
            }),
        )));
        let nested = (depth == 2).then(|| game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement_source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Hand)),
            ReplacementAction::Instead(vec![Effect::draw(1)]),
        )));
        game.take_pending_trigger_events();
        let original_library = game.players.iter().map(|player| player.library.clone()).collect::<Vec<_>>();
        let before_ids = game.next_object_id_counter();
        let before_random = game.irreversible_random_count();
        let mut dm = ObserveBoundary { random: before_random, other_original: other, controller: bob,
            asks: 0, pause_at, pending: false };
        let shuffle = ShuffleObjectsIntoLibraryEffect::new(
            ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Graveyard)),
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Target)).with_owner_library_destination();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.executing_effect = Some(123456);
        ctx.iteration.iterated_object = Some(source);
        ctx.iteration.iterated_player = Some(bob);
        ctx.tag_object("__it__", snapshot(&game, source));
        ctx.tag_object(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG, snapshot(&game, replacement_source));
        let outcome = if dispatched { crate::effects::execute_effect(&mut game, &Effect::new(shuffle.clone()), &mut ctx) }
            else { shuffle.execute(&mut game, &mut ctx) }.unwrap();
        assert_eq!(ctx.source, source); assert_eq!(ctx.controller, alice);
        assert_eq!(ctx.executing_effect, Some(123456));
        assert_eq!(ctx.iteration.iterated_object, Some(source));
        assert_eq!(ctx.iteration.iterated_player, Some(bob));
        assert_eq!(ctx.get_tagged_all("__it__").unwrap()[0].object_id, source);
        assert_eq!(ctx.get_tagged_all(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG).unwrap()[0].object_id, replacement_source);
        drop(ctx);
        if pause_at.is_some() {
            assert!(outcome.events.is_empty());
            assert_eq!(game.irreversible_random_count(), before_random);
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(game.player(bob).unwrap().hand.is_empty());
            assert_eq!(game.object(replaced).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(other).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.players.iter().map(|player| player.library.clone()).collect::<Vec<_>>(), original_library);
            assert!(game.effect_store.replacement_effects.get_effect(outer).is_some());
            assert!(inner.is_none_or(|id| game.effect_store.replacement_effects.get_effect(id).is_some()));
            assert!(nested.is_none_or(|id| game.effect_store.replacement_effects.get_effect(id).is_some()));
            assert!(game.take_pending_trigger_events().is_empty());
            dm.asks = 0; dm.pause_at = None; dm.pending = false;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            shuffle.execute(&mut game, &mut ctx).unwrap();
        } else {
            assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .filter(|event| event.amount == 2).count(), 1);
        }
        assert_eq!(dm.asks, 2);
        assert_eq!(game.player(bob).unwrap().life, 25);
        assert_eq!(game.player(bob).unwrap().hand.len(), 1);
        assert_eq!(game.irreversible_random_count(), before_random + 2);
    } } } }
}

#[test]
fn native_iterators_share_direct_and_dispatcher_resource_and_error_rollback() {
    for tagged in [false, true] {
    for dispatched in [false, true] {
    for resource_failure in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let first = card(&mut game, alice, Zone::Exile, "First");
        let second = card(&mut game, alice, Zone::Exile, "Second");
        for _ in 0..3 { card(&mut game, alice, Zone::Library, "Library"); }
        let mut effects = vec![Effect::new(crate::effects::InvestigateEffect::you(1)), Effect::shuffle_library(), Effect::draw(1)];
        if !resource_failure { effects.push(Effect::lose_life(Value::X)); }
        let effect = iterator(tagged, "selected", effects);
        game.set_token_creation_limits(crate::effects::tokens::resources::TokenCreationLimits {
            max_created_tokens: 1, ..Default::default()
        });
        game.take_pending_trigger_events();
        let library = game.player(alice).unwrap().library.clone();
        let ids = game.next_object_id_counter();
        let random = game.irreversible_random_count();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_objects("selected", vec![snapshot(&game, first), snapshot(&game, second)]);
        ctx.tag_object("__it__", snapshot(&game, source));
        ctx.iteration.iterated_object = Some(source);
        let result = if dispatched { crate::effects::execute_effect(&mut game, &effect, &mut ctx) }
            else { effect.0.execute(&mut game, &mut ctx) };
        if resource_failure { assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded { .. }))); }
        else { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        assert_eq!(game.battlefield, vec![source]);
        assert_eq!(game.player(alice).unwrap().library, library);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(game.next_object_id_counter(), ids);
        assert_eq!(game.irreversible_random_count(), random);
        assert!(game.take_pending_trigger_events().is_empty());
        assert_eq!(ctx.iteration.iterated_object, Some(source));
        assert_eq!(ctx.get_tagged_all("__it__").unwrap()[0].object_id, source);
    } } }
}

#[test]
fn held_trigger_matching_keeps_real_receipt_proof_across_iterator_continuation() {
    for tagged in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0);
        let source = card(&mut game, alice, Zone::Battlefield, "Observer");
        game.object_mut(source).unwrap().abilities_mut().push(crate::ability::Ability::triggered(
            crate::triggers::Trigger::you_gain_life(), vec![Effect::gain_life(1)]));
        let selected = snapshot(&game, source);
        for _ in 0..2 { card(&mut game, alice, Zone::Library, "Library"); }
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("selected", selected);
        let effect = iterator(tagged, "selected", vec![Effect::gain_life(2), Effect::draw(1)]);
        game.effect_store.trigger_matching_holds += 1;
        let prepared = crate::effects::runtime::prepare_effect_draw_continuation(&mut game, &effect, &mut ctx).unwrap();
        let completion = prepared.completion.expect("native iterator retained draw");
        assert!(game.player(alice).unwrap().hand.is_empty());
        let mut outcome = completion.complete(&mut game, &mut ctx, prepared.outcome).unwrap();
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        assert!(game.take_pending_trigger_entries().is_empty());
        assert!(outcome.events.iter().filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
            .all(|event| !event.triggers_captured()));
        game.effect_store.trigger_matching_holds -= 1;
        crate::effects::capture_triggers_before_added_program(&mut game, &ctx, None, outcome.events.iter_mut()).unwrap();
        assert_eq!(game.take_pending_trigger_entries().len(), 1);
        assert_eq!(game.turn_store.turn_history.total_life_gained_for_players(&[alice]), 2);
        crate::effects::capture_triggers_before_added_program(&mut game, &ctx, None, outcome.events.iter_mut()).unwrap();
        assert!(game.take_pending_trigger_entries().is_empty());
    }
}

#[test]
fn retained_tagged_iterations_keep_previous_objects_and_each_players_counts() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let first = card(&mut game, alice, Zone::Exile, "First");
    let second = card(&mut game, bob, Zone::Exile, "Second");
    for player in [alice, bob] { for _ in 0..3 { card(&mut game, player, Zone::Library, "Library"); } }
    let previous = Value::Count(ObjectFilter::default().match_tagged(
        ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG, TaggedOpbjectRelation::IsTaggedObject));
    let effect = Effect::new(ForEachTaggedEffect::new("selected", vec![
        Effect::with_id(91, Effect::gain_life_player(previous,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer))),
        Effect::target_draws(Value::EffectValue(crate::effect::EffectId(91)), PlayerFilter::IteratedPlayer),
    ]));
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.tag_objects("selected", vec![snapshot(&game, first), snapshot(&game, second)]);
    ctx.tag_object("__it__", snapshot(&game, source));
    ctx.tag_object(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG, snapshot(&game, source));
    let prepared = crate::effects::runtime::prepare_effect_draw_continuation(&mut game, &effect, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(bob).unwrap().life, 21,
        "the first iteration's dynamic zero draw does not delay the second non-draw prefix");
    assert!(game.players.iter().all(|player| player.hand.is_empty()));
    assert_eq!(ctx.get_tagged_all("__it__").unwrap()[0].object_id, source);
    assert_eq!(ctx.get_tagged_all(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG).unwrap()[0].object_id, source);
    ctx.store_outcome(crate::effect::EffectId(91), EffectOutcome::count(99));
    let outcome = prepared.completion.unwrap().complete(&mut game, &mut ctx, prepared.outcome).unwrap();
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert_eq!(game.player(bob).unwrap().hand.len(), 1);
    assert_eq!(outcome.player_counts().unwrap(), &[(alice, 0), (bob, 2)]);
    assert_eq!(ctx.effect_outcomes[&crate::effect::EffectId(91)].count_or_zero(), 1);
    assert_eq!(ctx.get_tagged_all("__it__").unwrap()[0].object_id, source);
    assert_eq!(ctx.get_tagged_all(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG).unwrap()[0].object_id, source);
}

#[test]
fn draw_quantity_is_retained_before_other_originals_change_its_inputs() {
    for tagged in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let selected = card(&mut game, bob, Zone::Exile, "Selected");
        let counted = card(&mut game, alice, Zone::Graveyard, "Counted");
        for _ in 0..3 { card(&mut game, bob, Zone::Library, "Library"); }
        let effect = iterator(tagged, "selected", vec![Effect::target_draws(
            Value::Count(ObjectFilter::default().in_zone(Zone::Graveyard)), PlayerFilter::IteratedPlayer)]);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("selected", snapshot(&game, selected));
        ctx.executing_effect = Some(456789);
        let prepared = crate::effects::runtime::prepare_effect_draw_continuation(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(ctx.executing_effect, Some(456789));
        assert!(game.player(bob).unwrap().hand.is_empty());
        game.remove_object(counted);
        ctx.iteration.iterated_player = Some(alice);
        let outcome = prepared.completion.unwrap().complete(&mut game, &mut ctx, prepared.outcome).unwrap();
        assert_eq!(game.player(bob).unwrap().hand.len(), 1);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(outcome.count_or_zero(), 1);
        assert_eq!(ctx.executing_effect, Some(456789));
    }
}

#[test]
fn empty_and_expired_selections_keep_native_snapshot_and_no_draw_prefix_semantics() {
    for tagged in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let selected = card(&mut game, alice, Zone::Exile, "Expired");
        let expired = snapshot(&game, selected);
        game.remove_object(selected);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = iterator(tagged, "selected", vec![Effect::draw(0), Effect::gain_life(2)]);
        let empty = crate::effects::runtime::prepare_effect_draw_continuation(&mut game, &effect, &mut ctx).unwrap();
        assert!(empty.completion.is_none());
        assert_eq!(game.player(alice).unwrap().life, 20);
        ctx.tag_object("selected", expired);
        let completed = crate::effects::runtime::prepare_effect_draw_continuation(&mut game, &effect, &mut ctx).unwrap();
        assert!(completed.completion.is_none());
        assert_eq!(game.player(alice).unwrap().life, 22,
            "an expired exact tagged snapshot still participates in the native iterator");
        assert_eq!(completed.outcome.count_or_zero(), 2);
    }
}

struct ObserveOrderedTail { asks: usize, original: ObjectId }
impl crate::decision::DecisionMaker for ObserveOrderedTail {
    fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
        assert_eq!(game.player(PlayerId(0)).unwrap().life, 20,
            "the loop's later authored gain cannot commit before the movement's replacement draw");
        assert!(game.player(PlayerId(0)).unwrap().hand.is_empty());
        assert!(game.object(self.original).is_some());
        self.asks += 1;
        true
    }
}
#[test]
fn quantified_multi_instruction_iterators_do_not_overtake_an_indirect_draw() {
    for tagged in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let first = card(&mut game, alice, Zone::Graveyard, "First");
        card(&mut game, bob, Zone::Graveyard, "Second");
        for _ in 0..2 { card(&mut game, alice, Zone::Library, "Library"); }
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(first), Some(Zone::Graveyard), Some(Zone::Exile)),
            ReplacementAction::Instead(vec![Effect::may(vec![Effect::gain_life(0)]), Effect::draw(1)]),
        ));
        let filter = ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(PlayerFilter::IteratedPlayer);
        let body = vec![Effect::move_to_zone(ChooseSpec::tagged("__it__"), Zone::Exile, false),
            Effect::gain_life_player(5, ChooseSpec::Player(PlayerFilter::IteratedPlayer))];
        let effects = if tagged { vec![Effect::new(TagMatchingObjectsEffect::new(filter, "selected")),
            Effect::new(ForEachTaggedEffect::new("selected", body))] }
            else { vec![Effect::new(ForEachObject::new(filter, body))] };
        let mut dm = ObserveOrderedTail { asks: 0, original: first };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ForPlayersEffect::new(PlayerFilter::Any, effects).execute(&mut game, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(dm.asks, 1);
        assert_eq!(game.player(alice).unwrap().life, 25);
        assert_eq!(game.player(bob).unwrap().life, 25);
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }
}

struct ObserveOwnerShuffle { random: u64, owners: Vec<PlayerId>, originals: Vec<ObjectId>, asks: usize }
impl crate::decision::DecisionMaker for ObserveOwnerShuffle {
    fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
        assert_eq!(game.irreversible_random_count(), self.random + self.owners.len() as u64);
        assert!(self.originals.iter().all(|object| game.object(*object).is_none()));
        assert!(!game.has_open_simultaneous_action());
        self.asks += 1;
        true
    }
}
#[test]
fn native_owner_shuffle_finishes_grouped_originals_before_replacement_draws() {
    for dispatched in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let originals = vec![card(&mut game, alice, Zone::Graveyard, "First"),
            card(&mut game, alice, Zone::Graveyard, "Second"), card(&mut game, bob, Zone::Graveyard, "Third")];
        for player in [alice, bob] { for _ in 0..3 { card(&mut game, player, Zone::Library, "Library"); } }
        for original in &originals {
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(*original), Some(Zone::Graveyard), Some(Zone::Library)),
                ReplacementAction::Additionally(vec![Effect::draw(1), Effect::may(vec![Effect::gain_life(1)])]),
            ));
        }
        let effect = ForEachObject::new(ObjectFilter::default().in_zone(Zone::Graveyard), vec![
            Effect::new(crate::effects::SequenceEffect::comma_then(vec![
                Effect::move_to_zone(ChooseSpec::Iterated, Zone::Library, true),
                Effect::shuffle_library_player(PlayerFilter::OwnerOf(crate::target::ObjectRef::tagged("__it__"))),
            ])),
        ]);
        game.take_pending_trigger_events();
        let mut dm = ObserveOwnerShuffle { random: game.irreversible_random_count(), owners: vec![alice, bob],
            originals, asks: 0 };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = if dispatched { crate::effects::execute_effect(&mut game, &Effect::new(effect.clone()), &mut ctx) }
            else { effect.execute(&mut game, &mut ctx) }.unwrap();
        drop(ctx);
        assert_eq!(dm.asks, 3);
        assert_eq!(game.player(alice).unwrap().hand.len(), 3);
        let shuffles = outcome.events.iter().filter(|event| event.downcast::<crate::events::ShuffleLibraryEvent>().is_some())
            .collect::<Vec<_>>();
        assert_eq!(shuffles.len(), 2);
        assert_ne!(shuffles[0].provenance(), shuffles[1].provenance());
        let first_draw = outcome.events.iter().position(|event| event.downcast::<crate::events::CardsDrawnEvent>().is_some()).unwrap();
        let last_shuffle = outcome.events.iter().rposition(|event| event.downcast::<crate::events::ShuffleLibraryEvent>().is_some()).unwrap();
        assert!(last_shuffle < first_draw, "reported receipts preserve physical original-before-draw ordering");
        assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).count(), 3);
    }
}

struct PauseAfterQuantity { random: u64, pending: bool, pause: bool, asks: usize }
impl crate::decision::DecisionMaker for PauseAfterQuantity {
    fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
        assert_eq!(game.irreversible_random_count(), self.random + 2);
        assert_eq!(game.zone_ids(Zone::Graveyard).count(), 1);
        assert_eq!(game.player(PlayerId(1)).unwrap().hand.len(), 2,
            "the reached draw retained its pre-original two-card quantity");
        self.asks += 1; self.pending = self.pause; !self.pending
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
#[test]
fn pending_dynamic_draw_retries_from_the_rolled_back_original_quantity() {
    for tagged in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, bob, Zone::Battlefield, "Source");
        let replaced = card(&mut game, alice, Zone::Graveyard, "Replaced");
        let other = card(&mut game, bob, Zone::Graveyard, "Original");
        for player in [alice, bob] { for _ in 0..3 { card(&mut game, player, Zone::Library, "Library"); } }
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Library)),
            ReplacementAction::Instead(vec![iterator(tagged, "it", vec![
                Effect::draw(Value::Count(ObjectFilter::default().in_zone(Zone::Graveyard))),
                Effect::may(vec![Effect::gain_life(1)]),
            ])]),
        ));
        game.take_pending_trigger_events();
        let before_random = game.irreversible_random_count();
        let before_ids = game.next_object_id_counter();
        let mut dm = PauseAfterQuantity { random: before_random, pending: false, pause: true, asks: 0 };
        let shuffle = ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Graveyard)),
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Target)).with_owner_library_destination();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = shuffle.execute(&mut game, &mut ctx).unwrap();
        assert!(outcome.events.is_empty());
        drop(ctx);
        assert_eq!(dm.asks, 1);
        assert_eq!(game.irreversible_random_count(), before_random);
        assert_eq!(game.next_object_id_counter(), before_ids);
        assert_eq!(game.object(replaced).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(other).unwrap().zone, Zone::Graveyard);
        assert!(game.player(bob).unwrap().hand.is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert!(game.take_pending_trigger_events().is_empty());
        dm.pending = false; dm.pause = false;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        shuffle.execute(&mut game, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(dm.asks, 2);
        assert_eq!(game.player(bob).unwrap().life, 21);
        assert_eq!(game.player(bob).unwrap().hand.len(), 2);
    }
}

#[derive(Debug, Clone)]
struct AssertRetainedWrapperResult {
    id: crate::effect::EffectId, tag: String, source: ObjectId, random: u64, other_original: ObjectId,
}
impl EffectExecutor for AssertRetainedWrapperResult {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        assert_eq!(ctx.source, self.source);
        let player = ctx.iteration.iterated_player.unwrap();
        assert_eq!(game.player(player).unwrap().hand.len(), 2);
        assert_eq!(game.irreversible_random_count(), self.random + 2);
        assert!(game.object(self.other_original).is_none(), "the outer original precedes the wrapped draw tail");
        assert_eq!(ctx.effect_outcomes[&self.id].count_or_zero(), 1,
            "WithId retains the movement quantity, not its appended draw count");
        let tagged = ctx.get_tagged_all(&self.tag).unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].zone, Zone::Hand);
        Ok(EffectOutcome::count(0))
    }
}
struct ObserveIndirectDraw { random: u64, players: Vec<PlayerId> }
impl crate::decision::DecisionMaker for ObserveIndirectDraw {
    fn decide_boolean(&mut self, game: &GameState, context: &crate::decisions::context::BooleanContext) -> bool {
        assert!(!game.has_open_simultaneous_action(), "a retained actual draw must not reopen a synthetic group");
        assert_eq!(game.irreversible_random_count(), self.random + 2);
        assert_eq!(game.player(context.player).unwrap().hand.len(), 2);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(context.player), 1);
        self.players.push(context.player);
        true
    }
}
#[test]
fn quantified_native_and_wrapped_iterators_publish_results_after_their_draw_tail() {
    for tagged in [false, true] {
    for rebound in [false, true] {
    for wrapped_child in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        game.object_mut(source).unwrap().abilities_mut().push(crate::ability::Ability::triggered(
            crate::triggers::Trigger::player_draws_card(PlayerFilter::Any), vec![Effect::gain_life(1)]));
        let rebound_source = card(&mut game, bob, Zone::Battlefield, "Rebound source");
        let replaced = card(&mut game, alice, Zone::Graveyard, "Replaced original");
        let other_original = card(&mut game, bob, Zone::Graveyard, "Other original");
        let selected = [card(&mut game, alice, Zone::Exile, "First selected"),
            card(&mut game, bob, Zone::Exile, "Second selected")];
        for player in [alice, bob] { for _ in 0..3 { card(&mut game, player, Zone::Library, "Library"); } }
        for (original, controller) in selected.into_iter().zip([alice, bob]) {
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, controller, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(original), Some(Zone::Exile), Some(Zone::Hand)),
                ReplacementAction::Additionally(vec![Effect::draw(1), Effect::may(vec![Effect::gain_life(0)])]),
            ));
        }
        let movement = Effect::move_to_zone(ChooseSpec::tagged("__it__"), Zone::Hand, false);
        let movement = if wrapped_child { Effect::with_id(93, movement.tag_all("arrival")) } else { movement };
        let movement = if rebound && wrapped_child { Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::SpecificObject(rebound_source), movement)) } else { movement };
        let filter = ObjectFilter::default().in_zone(Zone::Exile).owned_by(PlayerFilter::IteratedPlayer);
        let loop_effect = if tagged { Effect::new(ForEachTaggedEffect::new("selected", vec![movement])) }
            else { Effect::new(ForEachObject::new(filter.clone(), vec![movement])) };
        // Outer wrappers are owned by ForPlayers' native program scopes;
        // an unwrapped single child exercises the iterator proposal phases.
        let loop_effect = if wrapped_child { loop_effect }
            else { Effect::with_id(93, loop_effect.tag_all("arrival")) };
        let loop_effect = if rebound && !wrapped_child { Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::SpecificObject(rebound_source), loop_effect)) } else { loop_effect };
        let random = game.irreversible_random_count();
        let effects = vec![Effect::new(TagMatchingObjectsEffect::new(filter, "selected")), loop_effect,
            Effect::new(AssertRetainedWrapperResult { id: crate::effect::EffectId(93), tag: "arrival".into(),
                source, random, other_original })];
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(replaced), Some(Zone::Graveyard), Some(Zone::Library)),
            ReplacementAction::Instead(vec![Effect::new(ForPlayersEffect::new(PlayerFilter::Any, effects))]),
        ));
        game.take_pending_trigger_events();
        let mut dm = ObserveIndirectDraw { random, players: Vec::new() };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let shuffle = ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Graveyard)),
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Target)).with_owner_library_destination();
        let outcome = shuffle.execute(&mut game, &mut ctx).unwrap();
        assert!(game.exile.is_empty());
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(bob).unwrap().hand.len(), 2);
        assert!(!ctx.effect_outcomes.contains_key(&crate::effect::EffectId(93)));
        drop(ctx);
        assert_eq!(dm.players, vec![alice, bob]);
        assert_eq!(game.take_pending_trigger_entries().len(), 2);
        assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::CardsDrawnEvent>())
            .map(|event| event.amount()).sum::<u32>(), 2);
    } } }
}

#[test]
fn retained_tagged_iteration_uses_the_captured_historical_controller() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Wall");
    let attacker = card(&mut game, bob, Zone::Battlefield, "Attacker");
    let wall = snapshot(&game, source);
    let attacker_at_block = snapshot(&game, attacker);
    let event = crate::triggers::TriggerEvent::new(
        crate::events::combat::CreatureBlockedEvent::with_snapshots(source, attacker, wall.clone(), attacker_at_block.clone()),
        crate::provenance::ProvNodeId::default());
    game.record_turn_history_event(&event);
    let mut later = attacker_at_block;
    later.controller = alice;
    for _ in 0..3 { card(&mut game, bob, Zone::Library, "Library"); }
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.tag_object("wall", wall);
    ctx.tag_object("selected", later);
    let effect = Effect::new(ForEachTaggedEffect::new("selected", vec![
        Effect::target_draws(1, PlayerFilter::IteratedPlayer),
    ]).with_controller_at_last_blocked_by("wall"));
    let prepared = crate::effects::runtime::prepare_effect_draw_continuation(&mut game, &effect, &mut ctx).unwrap();
    game.remove_object(attacker);
    ctx.tagged_objects.remove(&TagKey::from("wall"));
    let outcome = prepared.completion.unwrap().complete(&mut game, &mut ctx, prepared.outcome).unwrap();
    assert_eq!(game.player(bob).unwrap().hand.len(), 1);
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert_eq!(outcome.player_counts().unwrap(), &[(bob, 1)]);
}

#[test]
fn generic_movement_without_draw_finishes_the_iterator_prefix_immediately() {
    for tagged in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let selected = card(&mut game, alice, Zone::Graveyard, "Selected");
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("selected", snapshot(&game, selected));
        let effect = iterator(tagged, "selected", vec![
            Effect::move_to_zone(ChooseSpec::Iterated, Zone::Exile, false), Effect::gain_life(2),
        ]);
        let prepared = crate::effects::runtime::prepare_effect_draw_continuation(&mut game, &effect, &mut ctx).unwrap();
        assert!(prepared.completion.is_none());
        assert!(game.object(selected).is_none());
        assert_eq!(game.exile.len(), 1);
        assert_eq!(game.player(alice).unwrap().life, 22);
    }
}

#[test]
fn resource_failure_after_retained_iterator_draw_restores_the_entire_shuffle() {
    for tagged in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let original = card(&mut game, alice, Zone::Graveyard, "Original");
        for _ in 0..3 { card(&mut game, alice, Zone::Library, "Library"); }
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, alice, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(original), Some(Zone::Graveyard), Some(Zone::Library)),
            ReplacementAction::Instead(vec![iterator(tagged, "it", vec![
                Effect::new(crate::effects::InvestigateEffect::you(1)), Effect::draw(1),
                Effect::new(crate::effects::InvestigateEffect::you(1)),
            ])]),
        ));
        game.set_token_creation_limits(crate::effects::tokens::resources::TokenCreationLimits {
            max_created_tokens: 1, ..Default::default()
        });
        game.take_pending_trigger_events();
        let before_ids = game.next_object_id_counter();
        let before_random = game.irreversible_random_count();
        let library = game.player(alice).unwrap().library.clone();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let shuffle = ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::all(
            ObjectFilter::default().in_zone(Zone::Graveyard)), PlayerFilter::You);
        assert!(matches!(shuffle.execute(&mut game, &mut ctx), Err(ExecutionError::ResourceLimitExceeded { .. })));
        assert_eq!(game.next_object_id_counter(), before_ids);
        assert_eq!(game.irreversible_random_count(), before_random);
        assert_eq!(game.player(alice).unwrap().library, library);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(game.battlefield, vec![source]);
        assert_eq!(game.object(original).unwrap().zone, Zone::Graveyard);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert!(game.take_pending_trigger_events().is_empty());
    }
}
