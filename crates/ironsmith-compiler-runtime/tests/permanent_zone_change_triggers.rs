//! Authored movement producers exercise both runtime definitions and serialized
//! artifacts. These regressions are intentionally unrun during the source-only
//! card-coverage campaign.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, EffectExecutor, ReturnToHandEffect};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn has_zone_event(kind: &TriggerKind) -> bool {
    match kind {
        TriggerKind::ZoneChange(_) => true,
        TriggerKind::Either { left, right } => {
            has_zone_event(&left.kind) || has_zone_event(&right.kind)
        }
        _ => false,
    }
}

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/permanent_zone_change_triggers.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let text = row["text"].as_str().unwrap();
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, kind: &str) -> ObjectId {
    let text = if kind == "Creature" {
        "Type: Creature\nPower/Toughness: 2/2".to_owned()
    } else {
        format!("Type: {kind}")
    };
    let definition = compile_to_runtime_definition("Zone-change resource", &text, false).unwrap();
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
    panic!("zone-change trigger work did not settle");
}
fn bounce(game: &mut GameState, source: ObjectId, spec: ChooseSpec) {
    let controller = game.controller_of_id(source).unwrap();
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, controller, &mut dm);
    let outcome = ReturnToHandEffect::with_spec(spec)
        .execute(game, &mut ctx)
        .unwrap();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
}
fn token_count(game: &GameState, owner: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            game.object(**id).is_some_and(|object| {
                object.kind == ironsmith::object::ObjectKind::Token
                    && game.controller_of(object) == owner
            })
        })
        .count()
}

#[test]
fn ten_exact_permanent_zone_cards_retain_materialized_zone_event_programs() {
    assert_eq!(fixtures().len(), 10);
    for fixture in fixtures() {
        for definition in definitions(fixture["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
                if triggered.trigger.compiled_model().is_some_and(|model| has_zone_event(&model.kind)))), "{}", definition.name());
        }
    }
}

#[test]
fn stolen_permanent_uses_origin_controller_but_private_graveyard_owner() {
    let marvel = definitions("Aetherworks Marvel");
    let patron = definitions("Patron of the Nezumi");
    let liability = definitions("Liability");
    for path in 0..2 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&marvel[path], B, Zone::Battlefield);
        game.create_object_from_definition(&patron[path], B, Zone::Battlefield);
        game.create_object_from_definition(&liability[path], B, Zone::Battlefield);
        let victim = card(&mut game, A, Zone::Battlefield, "Artifact");
        game.set_current_controller(victim, B).unwrap();
        let departed = game.move_object_by_effect(victim, Zone::Graveyard).unwrap();
        assert_ne!(departed, victim);
        assert!(game.player(A).unwrap().graveyard.contains(&departed));
        assert_eq!(settle(&mut game), 3);
        assert_eq!(game.player(B).unwrap().energy_counters, 1);
        assert_eq!(
            game.player(A).unwrap().life,
            18,
            "that player is the owner, not the last controller"
        );
        assert_eq!(game.player(B).unwrap().life, 20);
        for zone in [Zone::Hand, Zone::Library, Zone::Exile] {
            let card = card(&mut game, A, zone, "Artifact");
            game.move_object_by_effect(card, Zone::Graveyard).unwrap();
            assert_eq!(
                settle(&mut game),
                0,
                "bare permanent nouns require a battlefield origin"
            );
        }
    }
}

#[test]
fn returned_permanent_binds_its_hand_owner_and_never_a_graveyard_return() {
    for definition in definitions("Warped Devotion") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let victim = card(&mut game, A, Zone::Battlefield, "Artifact");
        game.set_current_controller(victim, B).unwrap();
        let discard = card(&mut game, A, Zone::Hand, "Sorcery");
        let other_hand = card(&mut game, B, Zone::Hand, "Sorcery");
        bounce(&mut game, source, ChooseSpec::SpecificObject(victim));
        assert_eq!(stack(&mut game), 1);
        let returned = *game
            .player(A)
            .unwrap()
            .hand
            .iter()
            .find(|id| **id != discard)
            .unwrap();
        assert_ne!(returned, victim);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert!(game.player(B).unwrap().hand.contains(&other_hand));
        let from_graveyard = card(&mut game, A, Zone::Graveyard, "Artifact");
        bounce(
            &mut game,
            source,
            ChooseSpec::SpecificObject(from_graveyard),
        );
        assert_eq!(settle(&mut game), 0);
    }
}

#[test]
fn grouped_return_draws_once_per_simultaneous_event_and_obeys_frequency() {
    for definition in definitions("Tameshi, Reality Architect") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        card(&mut game, A, Zone::Library, "Sorcery");
        card(&mut game, A, Zone::Library, "Sorcery");
        card(&mut game, A, Zone::Battlefield, "Artifact");
        card(&mut game, B, Zone::Battlefield, "Artifact");
        bounce(&mut game, source, ChooseSpec::All(ObjectFilter::artifact()));
        assert_eq!(
            settle(&mut game),
            1,
            "two owners in one bounce still make one event"
        );
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        let later = card(&mut game, A, Zone::Battlefield, "Artifact");
        bounce(&mut game, source, ChooseSpec::SpecificObject(later));
        assert_eq!(
            settle(&mut game),
            0,
            "the separate event is blocked by once each turn"
        );
        let creature = card(&mut game, A, Zone::Battlefield, "Creature");
        bounce(&mut game, source, ChooseSpec::SpecificObject(creature));
        assert_eq!(settle(&mut game), 0);
    }
}

#[test]
fn simultaneous_self_and_other_return_uses_source_lookback_once_per_creature() {
    for definition in definitions("Stormfront Riders") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        card(&mut game, A, Zone::Battlefield, "Creature");
        // Opponent-owned creatures return to a different hand and don't count.
        card(&mut game, B, Zone::Battlefield, "Creature");
        bounce(&mut game, source, ChooseSpec::All(ObjectFilter::creature()));
        assert!(game.object(source).is_none());
        assert_eq!(settle(&mut game), 2);
        assert_eq!(token_count(&game, A), 2);
        assert_eq!(token_count(&game, B), 0);
    }
}

#[test]
fn leave_battlefield_turn_guard_is_event_time_and_grouping_includes_departing_source() {
    for definition in definitions("Oni-Cult Anvil") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.turn.active_player = B;
        let early = card(&mut game, A, Zone::Battlefield, "Artifact");
        bounce(&mut game, source, ChooseSpec::SpecificObject(early));
        assert_eq!(settle(&mut game), 0);
        game.turn.active_player = A;
        card(&mut game, A, Zone::Battlefield, "Artifact");
        bounce(&mut game, source, ChooseSpec::All(ObjectFilter::artifact()));
        assert_eq!(stack(&mut game), 1);
        game.turn.active_player = B;
        settle(&mut game);
        assert_eq!(
            token_count(&game, A),
            1,
            "event-time during your turn is not an intervening if"
        );
    }
}

#[test]
fn aethermage_optional_payment_uses_returned_hand_owner() {
    for definition in definitions("Azorius Aethermage") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        card(&mut game, B, Zone::Library, "Sorcery");
        game.player_mut(B)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let mine = card(&mut game, B, Zone::Battlefield, "Artifact");
        game.set_current_controller(mine, A).unwrap();
        bounce(&mut game, source, ChooseSpec::SpecificObject(mine));
        assert_eq!(settle(&mut game), 1);
        assert_eq!(game.player(B).unwrap().hand.len(), 2);
        assert!(game.player(A).unwrap().hand.is_empty());
        let theirs = card(&mut game, A, Zone::Battlefield, "Artifact");
        game.set_current_controller(theirs, B).unwrap();
        bounce(&mut game, source, ChooseSpec::SpecificObject(theirs));
        assert_eq!(settle(&mut game), 0);
    }
}

#[test]
fn justice_counts_other_controlled_nonlands_independently_of_owner() {
    for definition in definitions("Justice, Vance Astrovik") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let stolen = card(&mut game, A, Zone::Battlefield, "Artifact");
        game.set_current_controller(stolen, B).unwrap();
        bounce(&mut game, source, ChooseSpec::SpecificObject(stolen));
        assert_eq!(settle(&mut game), 1);
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&ironsmith::object::CounterType::PlusOnePlusOne),
            Some(&1)
        );
        let land = card(&mut game, B, Zone::Battlefield, "Land");
        bounce(&mut game, source, ChooseSpec::SpecificObject(land));
        assert_eq!(settle(&mut game), 0);
        let opponent = card(&mut game, A, Zone::Battlefield, "Artifact");
        bounce(&mut game, source, ChooseSpec::SpecificObject(opponent));
        assert_eq!(settle(&mut game), 0);
        bounce(&mut game, source, ChooseSpec::SpecificObject(source));
        assert_eq!(
            settle(&mut game),
            0,
            "another excludes the departing source identity"
        );
    }
}

#[test]
fn suki_requires_another_controlled_permanent_on_its_controllers_turn() {
    for definition in definitions("Suki, Courageous Rescuer") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.turn.active_player = B;
        let wrong_turn = card(&mut game, A, Zone::Battlefield, "Land");
        bounce(&mut game, source, ChooseSpec::SpecificObject(wrong_turn));
        assert_eq!(settle(&mut game), 0);
        game.turn.active_player = A;
        let wrong_controller = card(&mut game, B, Zone::Battlefield, "Land");
        bounce(
            &mut game,
            source,
            ChooseSpec::SpecificObject(wrong_controller),
        );
        assert_eq!(settle(&mut game), 0);
        let stolen = card(&mut game, B, Zone::Battlefield, "Land");
        game.set_current_controller(stolen, A).unwrap();
        bounce(&mut game, source, ChooseSpec::SpecificObject(stolen));
        assert_eq!(settle(&mut game), 1);
        assert_eq!(token_count(&game, A), 1);
        let later = card(&mut game, A, Zone::Battlefield, "Land");
        bounce(&mut game, source, ChooseSpec::SpecificObject(later));
        assert_eq!(settle(&mut game), 0);
        game.next_turn();
        game.turn.active_player = A;
        bounce(&mut game, source, ChooseSpec::SpecificObject(source));
        assert_eq!(settle(&mut game), 0);
    }
}
