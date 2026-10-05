//! Native prepared-entry phase, projection and rollback scenarios. Unrun.
use super::*;
use crate::effects::EffectExecutor;
use crate::effect::Effect;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;

fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str) -> ObjectId {
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3)).build();
    game.create_object_from_card(&card, owner, zone)
}
struct PayEntryLife { questions: usize, pause_second: bool, pending: bool }
impl crate::decision::DecisionMaker for PayEntryLife {
    fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
        self.questions += 1;
        self.pending = self.pause_second && self.questions == 2;
        !self.pending
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn life_replacement(game: &mut GameState, source: ObjectId, object: ObjectId, owner: PlayerId)
    -> crate::replacement::ReplacementEffectId {
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, owner, crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(ObjectFilter::specific(object)),
        ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost: 2 },
    ))
}

#[test]
fn prepared_entry_detaches_its_projection_but_keeps_paid_cost_and_effect_identity() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let original = card(&mut game, alice, Zone::Graveyard, "Printed creature");
    let identity = game.object(original).unwrap().stable_id;
    let shield = life_replacement(&mut game, source, original, alice);
    let mut dm = PayEntryLife { questions: 0, pause_second: false, pending: false };
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let prepared = prepare_battlefield_entry_batch(&mut game, &mut ctx, vec![(original,
        BattlefieldEntryOptions::owner(false).face_down(true).with_entry_modifications(vec![
            crate::continuous::Modification::AddCardTypes(vec![CardType::Artifact]),
        ]))], std::collections::HashMap::new(), None).unwrap().unwrap();
    assert_eq!(game.player(alice).unwrap().life, 18);
    assert_eq!(game.object(original).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.object(original).unwrap().name.as_ref(), "Printed creature");
    assert!(game.object(original).unwrap().face_down_cast_state.is_none());
    assert!(!game.current_has_card_type(original, CardType::Artifact));
    let ids = prepared.provisional_entry_effects[&original].clone();
    assert_eq!(ids.len(), 1);
    assert!(game.effect_store.continuous_effects.effects().iter().all(|effect| !ids.contains(&effect.id)));
    let receipts = prepared.commit(&mut game, &mut ctx).unwrap();
    assert_eq!(receipts.len(), 1);
    let arrived = game.find_object_by_stable_id(identity).unwrap();
    assert_eq!(game.object(arrived).unwrap().zone, Zone::Battlefield);
    assert!(game.object(arrived).unwrap().face_down_cast_state.is_some());
    assert!(game.current_has_card_type(arrived, CardType::Artifact));
    assert!(game.effect_store.continuous_effects.effects().iter().any(|effect| ids.contains(&effect.id)));
    assert_eq!(game.player(alice).unwrap().life, 18, "entry preparation costs are not replayed");
    assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    drop(ctx);
    assert_eq!(dm.questions, 1);
}

#[derive(Debug, Clone)]
struct OtherOriginalStillPrinted { entrant: ObjectId, other: ObjectId }
impl crate::events::ReplacementMatcher for OtherOriginalStillPrinted {
    fn matches_prepared_event(&self, event: &dyn crate::events::GameEventType,
        context: &crate::events::context::PreparedEventContext) -> bool {
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event)
            .is_some_and(|event| event.object == self.entrant)
            && context.game.object(self.other).is_some_and(|other|
                other.zone == Zone::Graveyard && other.name.as_ref() == "First original"
                    && other.face_down_cast_state.is_none())
    }
    fn display(&self) -> String { "While the other original has its printed face".into() }
}

#[test]
fn entry_replacements_see_other_original_faces_before_any_entry_commits() {
    for quantified in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId(0); let bob = PlayerId(1);
        let source = card(&mut game, alice, Zone::Battlefield, "Source");
        let first = card(&mut game, alice, Zone::Graveyard, "First original");
        let second = card(&mut game, bob, Zone::Graveyard, "Second original");
        let first_identity = game.object(first).unwrap().stable_id;
        let second_identity = game.object(second).unwrap().stable_id;
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, alice, OtherOriginalStillPrinted { entrant: second, other: first },
            ReplacementAction::ChangeDestination(Zone::Exile),
        ));
        let mut movement = crate::effects::MoveToZoneEffect::new(ChooseSpec::all(ObjectFilter::creature()
            .in_zone(Zone::Graveyard).owned_by(if quantified { PlayerFilter::IteratedPlayer } else { PlayerFilter::Any })),
            Zone::Battlefield, false);
        movement.enters_face_down = true;
        let mut ctx = ExecutionContext::new_default(source, alice);
        if quantified {
            crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(movement)])
                .execute(&mut game, &mut ctx).unwrap();
        } else { movement.execute(&mut game, &mut ctx).unwrap(); }
        let first = game.find_object_by_stable_id(first_identity).unwrap();
        let second = game.find_object_by_stable_id(second_identity).unwrap();
        assert_eq!(game.object(first).unwrap().zone, Zone::Battlefield);
        assert!(game.object(first).unwrap().face_down_cast_state.is_some());
        assert_eq!(game.object(second).unwrap().zone, Zone::Exile);
        assert_eq!(game.object(second).unwrap().name.as_ref(), "Second original");
        assert!(game.object(second).unwrap().face_down_cast_state.is_none());
    }
}

#[test]
fn pending_later_entry_restores_prepared_costs_source_faces_and_originals() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let first = card(&mut game, alice, Zone::Graveyard, "First original");
    let second = card(&mut game, bob, Zone::Graveyard, "Second original");
    let shields = [life_replacement(&mut game, source, first, alice), life_replacement(&mut game, source, second, bob)];
    let before_ids = game.next_object_id_counter();
    game.take_pending_trigger_events();
    let mut movement = crate::effects::MoveToZoneEffect::new(ChooseSpec::all(ObjectFilter::creature()
        .in_zone(Zone::Graveyard).owned_by(PlayerFilter::IteratedPlayer)), Zone::Battlefield, false);
    movement.enters_face_down = true;
    let effect = crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(movement)]);
    let mut dm = PayEntryLife { questions: 0, pause_second: true, pending: false };
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert!(outcome.events.is_empty());
    drop(ctx);
    assert_eq!(dm.questions, 2);
    assert!(game.players.iter().all(|player| player.life == 20));
    for (object, name) in [(first, "First original"), (second, "Second original")] {
        assert_eq!(game.object(object).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(object).unwrap().name.as_ref(), name);
        assert!(game.object(object).unwrap().face_down_cast_state.is_none());
    }
    assert_eq!(game.next_object_id_counter(), before_ids);
    assert_eq!(game.battlefield, vec![source]);
    assert!(shields.iter().all(|id| game.effect_store.replacement_effects.get_effect(*id).is_some()));
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn one_batch_entry_replacement_survives_all_quantified_preparations() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0); let bob = PlayerId(1);
    let source = card(&mut game, alice, Zone::Battlefield, "Source");
    let first = card(&mut game, alice, Zone::Graveyard, "First");
    let second = card(&mut game, bob, Zone::Graveyard, "Second");
    let identities = [game.object(first).unwrap().stable_id, game.object(second).unwrap().stable_id];
    let shield = game.effect_store.replacement_effects.add_batch_one_shot_effect(
        ReplacementEffect::with_matcher(source, alice,
            crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
            ReplacementAction::EnterTapped));
    let movement = crate::effects::MoveToZoneEffect::new(ChooseSpec::all(ObjectFilter::creature()
        .in_zone(Zone::Graveyard).owned_by(PlayerFilter::IteratedPlayer)), Zone::Battlefield, false);
    let mut ctx = ExecutionContext::new_default(source, alice);
    crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(movement)])
        .execute(&mut game, &mut ctx).unwrap();
    for identity in identities {
        let entered = game.find_object_by_stable_id(identity).unwrap();
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert!(game.is_tapped(entered));
    }
    assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
}
