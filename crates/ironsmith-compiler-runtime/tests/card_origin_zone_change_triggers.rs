//! Exact-card source/origin regressions, deliberately unrun in source-only mode.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{
    EffectContext, EffectExecutor, MillEffect, MoveToZoneEffect, ReturnToHandEffect,
};
use ironsmith::game_loop::{
    generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/card_origin_zone_change_triggers.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let (artifact, direct) = compile_to_artifact(name, row["text"].as_str().unwrap(), false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn has_zone_event(kind: &TriggerKind) -> bool {
    match kind {
        TriggerKind::ZoneChange(_) => true,
        TriggerKind::Either { left, right } => {
            has_zone_event(&left.kind) || has_zone_event(&right.kind)
        }
        _ => false,
    }
}
fn resource(game: &mut GameState, owner: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Card-origin resource", text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn stack(game: &mut GameState) -> usize {
    put_triggers_on_stack_with_dm(
        game,
        &mut TriggerQueue::new(),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState) -> usize {
    let initial = stack(game);
    for _ in 0..24 {
        if game.stack_is_empty() {
            return initial;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        stack(game);
    }
    panic!("card-origin trigger work did not settle");
}
fn execute(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    effect: &dyn EffectExecutor,
) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, controller, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
}
fn counters(game: &GameState, source: ObjectId) -> u32 {
    game.object(source)
        .unwrap()
        .counters
        .get(&ironsmith::object::CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}
fn tokens(game: &GameState) -> usize {
    game.battlefield
        .iter()
        .filter(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token)
        .count()
}

#[test]
fn eleven_exact_card_origin_artifacts_retain_typed_movement_events() {
    assert_eq!(fixtures().len(), 11);
    for fixture in fixtures() {
        for definition in definitions(fixture["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
                if triggered.trigger.compiled_model().is_some_and(|model| has_zone_event(&model.kind)))), "{}", definition.name());
        }
    }
}

#[test]
fn brownscale_uses_its_exact_graveyard_source_when_destination_has_a_new_id() {
    for definition in definitions("Golgari Brownscale") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let original = game.create_object_from_definition(&definition, B, Zone::Graveyard);
        let stable = game.object(original).unwrap().stable_id;
        let effect = ReturnToHandEffect::with_spec(ChooseSpec::SpecificObject(original));
        execute(&mut game, original, B, &effect);
        let hand = *game
            .player(B)
            .unwrap()
            .hand
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        assert_ne!(hand, original);
        assert_eq!(settle(&mut game), 1);
        assert_eq!(game.player(B).unwrap().life, 22);
        assert_eq!(game.player(A).unwrap().life, 20);
        // The hand copy cannot observe an unrelated graveyard card moving.
        let other = resource(
            &mut game,
            B,
            Zone::Graveyard,
            "Type: Creature\nPower/Toughness: 2/2",
        );
        execute(
            &mut game,
            hand,
            B,
            &ReturnToHandEffect::with_spec(ChooseSpec::SpecificObject(other)),
        );
        assert_eq!(settle(&mut game), 0);
        // A battlefield-to-hand return is a different origin, even for Brownscale.
        let field = game.move_object_by_effect(hand, Zone::Battlefield).unwrap();
        settle(&mut game);
        execute(
            &mut game,
            field,
            B,
            &ReturnToHandEffect::with_spec(ChooseSpec::SpecificObject(field)),
        );
        assert_eq!(settle(&mut game), 0);
        assert_eq!(game.player(B).unwrap().life, 22);
    }
}

#[test]
fn titan_observes_opponents_graveyard_and_returns_its_own_current_source() {
    for definition in definitions("Erebos's Titan") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        resource(&mut game, A, Zone::Hand, "Type: Sorcery");
        let wrong_owner = resource(
            &mut game,
            A,
            Zone::Graveyard,
            "Type: Creature\nPower/Toughness: 1/1",
        );
        game.move_object_by_effect(wrong_owner, Zone::Exile)
            .unwrap();
        assert_eq!(settle(&mut game), 0);
        let wrong_type = resource(&mut game, B, Zone::Graveyard, "Type: Artifact");
        game.move_object_by_effect(wrong_type, Zone::Exile).unwrap();
        assert_eq!(settle(&mut game), 0);
        let creature = resource(
            &mut game,
            B,
            Zone::Graveyard,
            "Type: Creature\nPower/Toughness: 1/1",
        );
        game.move_object_by_effect(creature, Zone::Hand).unwrap();
        assert_eq!(settle(&mut game), 1);
        assert!(game.object(source).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(
            game.object(game.player(A).unwrap().hand[0])
                .unwrap()
                .stable_id,
            stable
        );
        assert_eq!(
            game.player(A).unwrap().graveyard.len(),
            1,
            "the discard was actually paid"
        );
    }
}

#[test]
fn disa_follows_the_milled_card_identity_but_excludes_battlefield_and_other_owners() {
    for definition in definitions("Disa the Restless") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let text = "Type: Creature — Lhurgoyf\nPower/Toughness: 2/2";
        let library = resource(&mut game, A, Zone::Library, text);
        let stable = game.object(library).unwrap().stable_id;
        execute(&mut game, source, A, &MillEffect::new(1, PlayerFilter::You));
        assert_eq!(settle(&mut game), 1);
        let returned = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        assert_ne!(returned, library);
        let dead = game
            .move_object_by_effect(returned, Zone::Graveyard)
            .unwrap();
        assert_eq!(settle(&mut game), 0);
        assert!(game.player(A).unwrap().graveyard.contains(&dead));
        let theirs = resource(&mut game, B, Zone::Library, text);
        game.move_object_by_effect(theirs, Zone::Graveyard).unwrap();
        assert_eq!(settle(&mut game), 0);
        let wrong_type = resource(&mut game, A, Zone::Hand, "Type: Kindred Instant — Lhurgoyf");
        game.move_object_by_effect(wrong_type, Zone::Graveyard)
            .unwrap();
        assert_eq!(
            settle(&mut game),
            0,
            "permanent card excludes a matching subtype on an instant"
        );
    }
}

#[test]
fn ultron_union_is_disjoint_for_death_card_origins_owners_and_source_exclusion() {
    for name in ["Ultron the Annihilator", "Ultron's Auxiliary"] {
        for definition in definitions(name) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for zone in [Zone::Battlefield, Zone::Hand, Zone::Library, Zone::Exile] {
                let artifact = resource(&mut game, A, zone, "Type: Artifact");
                game.move_object_by_effect(artifact, Zone::Graveyard)
                    .unwrap();
                assert_eq!(
                    settle(&mut game),
                    1,
                    "{name}: {zone:?} matches exactly one union arm"
                );
            }
            if name == "Ultron the Annihilator" {
                assert_eq!(game.player(B).unwrap().life, 16);
            } else {
                assert_eq!(counters(&game, source), 4);
            }
            let theirs = resource(&mut game, B, Zone::Battlefield, "Type: Artifact");
            game.set_current_controller(theirs, A).unwrap();
            game.move_object_by_effect(theirs, Zone::Graveyard).unwrap();
            assert_eq!(settle(&mut game), 0, "your graveyard names the owner");
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            assert_eq!(
                settle(&mut game),
                0,
                "another excludes the source in the battlefield arm"
            );
        }
    }
}

#[test]
fn crawling_infestation_groups_mill_and_checks_turn_and_frequency() {
    for definition in definitions("Crawling Infestation") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let text = "Type: Creature\nPower/Toughness: 1/1";
        game.turn.active_player = B;
        resource(&mut game, A, Zone::Library, text);
        execute(&mut game, source, A, &MillEffect::you(1));
        assert_eq!(settle(&mut game), 0);
        game.turn.active_player = A;
        resource(&mut game, A, Zone::Library, text);
        resource(&mut game, A, Zone::Library, text);
        execute(&mut game, source, A, &MillEffect::you(2));
        assert_eq!(settle(&mut game), 1);
        assert_eq!(tokens(&game), 1);
        let another = resource(&mut game, A, Zone::Hand, text);
        game.move_object_by_effect(another, Zone::Graveyard)
            .unwrap();
        assert_eq!(settle(&mut game), 0);
        assert_eq!(tokens(&game), 1);
    }
}

#[test]
fn library_destination_groups_distinct_origins_and_never_same_zone_reordering() {
    for name in ["Dutiful Knowledge Seeker", "Wan Shi Tong, All-Knowing"] {
        for definition in definitions(name) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            resource(&mut game, A, Zone::Graveyard, "Type: Sorcery");
            resource(&mut game, B, Zone::Graveyard, "Type: Sorcery");
            let filter = ObjectFilter {
                zone: Some(Zone::Graveyard),
                ..ObjectFilter::default()
            };
            execute(
                &mut game,
                source,
                A,
                &MoveToZoneEffect::new(ChooseSpec::All(filter), Zone::Library, false),
            );
            assert_eq!(settle(&mut game), 1);
            if name == "Dutiful Knowledge Seeker" {
                assert_eq!(counters(&game, source), 1);
            } else {
                assert_eq!(tokens(&game), 2);
            }
            let already = game.player(A).unwrap().library[0];
            execute(
                &mut game,
                source,
                A,
                &MoveToZoneEffect::new(ChooseSpec::SpecificObject(already), Zone::Library, true),
            );
            assert_eq!(
                settle(&mut game),
                0,
                "moving within one library is not entering a zone"
            );
        }
    }
}

#[test]
fn dreadhound_distinguishes_dying_from_library_cards_without_duplicate_union_triggers() {
    for definition in definitions("Dreadhound") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let text = "Type: Creature\nPower/Toughness: 1/1";
        resource(&mut game, B, Zone::Library, text);
        resource(&mut game, B, Zone::Library, text);
        execute(
            &mut game,
            source,
            A,
            &MillEffect::new(2, PlayerFilter::Specific(B)),
        );
        assert_eq!(settle(&mut game), 2);
        let dead = resource(&mut game, B, Zone::Battlefield, text);
        game.move_object_by_effect(dead, Zone::Graveyard).unwrap();
        assert_eq!(settle(&mut game), 1);
        let discarded = resource(&mut game, B, Zone::Hand, text);
        game.move_object_by_effect(discarded, Zone::Graveyard)
            .unwrap();
        assert_eq!(settle(&mut game), 0);
        assert_eq!(game.player(B).unwrap().life, 17);
    }
}

#[test]
fn sylex_exile_trigger_can_pay_and_search_after_source_leaves() {
    for definition in definitions("Urza's Sylex") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let walker = resource(
            &mut game,
            B,
            Zone::Library,
            "Type: Planeswalker — Jace\nLoyalty: 3",
        );
        let stable = game.object(walker).unwrap().stable_id;
        game.player_mut(B)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 2);
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        assert_eq!(settle(&mut game), 1);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert_eq!(
            game.object(game.player(B).unwrap().hand[0])
                .unwrap()
                .stable_id,
            stable
        );
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}

#[test]
fn desert_warfare_remembers_exact_milled_card_until_its_controllers_next_end_step() {
    for definition in definitions("Desert Warfare") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let desert = resource(&mut game, A, Zone::Library, "Type: Land — Desert");
        let stable = game.object(desert).unwrap().stable_id;
        execute(&mut game, source, A, &MillEffect::you(1));
        assert_eq!(settle(&mut game), 1);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(ironsmith::game_state::Step::End);
        game.turn.active_player = B;
        let mut queue = TriggerQueue::new();
        generate_and_queue_step_triggers(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert!(game.stack_is_empty());
        game.turn.active_player = A;
        game.turn.turn_number += 1;
        generate_and_queue_step_triggers(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game);
        let returned = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        assert_ne!(returned, desert);
        assert_eq!(game.controller_of_id(returned), Some(A));
    }
}

#[test]
fn queued_disa_trigger_does_not_follow_a_card_that_left_and_reentered_its_graveyard() {
    for definition in definitions("Disa the Restless") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(
            &mut game,
            A,
            Zone::Library,
            "Type: Creature — Lhurgoyf\nPower/Toughness: 2/2",
        );
        let stable = game.object(original).unwrap().stable_id;
        execute(&mut game, source, A, &MillEffect::you(1));
        assert_eq!(stack(&mut game), 1);
        // Removing the observer prevents an independent trigger for the second
        // graveyard entry, leaving only the queued original to resolve.
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        let first = *game
            .player(A)
            .unwrap()
            .graveyard
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        let exile = game.move_object_by_effect(first, Zone::Exile).unwrap();
        let second = game.move_object_by_effect(exile, Zone::Graveyard).unwrap();
        assert_ne!(first, second);
        settle(&mut game);
        assert!(game.player(A).unwrap().graveyard.contains(&second));
        assert!(
            !game
                .battlefield
                .iter()
                .any(|id| game.object(*id).unwrap().stable_id == stable)
        );
    }
}

#[test]
fn desert_warfare_sacrifice_and_hand_arms_create_distinct_delays_and_pin_new_ids() {
    for definition in definitions("Desert Warfare") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let field = resource(&mut game, A, Zone::Battlefield, "Type: Land — Desert");
        let field_stable = game.object(field).unwrap().stable_id;
        execute(
            &mut game,
            source,
            A,
            &ironsmith::effects::SacrificeEffect::you(ObjectFilter::land(), 1),
        );
        assert_eq!(
            settle(&mut game),
            1,
            "sacrifice and death are not two union matches"
        );
        let hand = resource(&mut game, A, Zone::Hand, "Type: Land — Desert");
        let hand_stable = game.object(hand).unwrap().stable_id;
        let moved = game.move_object_by_effect(hand, Zone::Graveyard).unwrap();
        assert_eq!(settle(&mut game), 1);
        // The delayed reference may not follow a later exile/re-entry, even
        // though its card's stable ID and final zone are the same.
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        let exile = game.move_object_by_effect(moved, Zone::Exile).unwrap();
        let later = game.move_object_by_effect(exile, Zone::Graveyard).unwrap();
        game.turn.active_player = A;
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(ironsmith::game_state::Step::End);
        let mut queue = TriggerQueue::new();
        generate_and_queue_step_triggers(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game);
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.object(*id).unwrap().stable_id == field_stable)
        );
        assert!(game.player(A).unwrap().graveyard.contains(&later));
        assert!(
            !game
                .battlefield
                .iter()
                .any(|id| game.object(*id).unwrap().stable_id == hand_stable)
        );
    }
}
