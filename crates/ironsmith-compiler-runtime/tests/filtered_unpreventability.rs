//! Authored only. Builds, compilation and execution remain deferred.
use ironsmith::ability::Ability;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Until;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::{process_damage_assignments_with_event_with_source_snapshot_opts, process_simultaneous_damage_assignments_with_event, SimultaneousDamageEvent};
use ironsmith::prevention::{PreventionShield, PreventionTarget};
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::{ObjectFilter, PlayerFilter};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/filtered_unpreventability.json.fixture")).unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", card["mana_cost"].as_str().unwrap(), card["type_line"].as_str().unwrap(), card["power"].as_str().unwrap(), card["toughness"].as_str().unwrap(), card["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text()); artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn a() -> PlayerId { PlayerId::from_index(0) }
fn b() -> PlayerId { PlayerId::from_index(1) }
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn object(game: &mut GameState, owner: PlayerId, creature: bool) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Damage source probe")
        .card_types(vec![if creature { CardType::Creature } else { CardType::Artifact }])
        .power_toughness(PowerToughness::fixed(2, 5)).build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}
fn shield(game: &mut GameState, amount: Option<u32>) {
    let source = game.new_object_id();
    game.effect_store.prevention_effects.add_shield(PreventionShield::new(source, b(), PreventionTarget::Player(b()), amount, Until::EndOfTurn));
}
fn damage(game: &mut GameState, source: ObjectId, combat: bool, unpreventable: bool, snapshot: Option<&ObjectSnapshot>) -> (u32, u32) {
    game.take_pending_trigger_events();
    let processed = process_damage_assignments_with_event_with_source_snapshot_opts(game, source, DamageTarget::Player(b()), 3, combat, unpreventable, EventCause::effect(), snapshot).unwrap();
    assert!(processed.programs.is_empty(), "these scenarios have no extra replacement payload");
    let remaining = processed.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented = game.take_pending_trigger_events().iter().filter_map(|event| event.downcast::<DamagePreventedEvent>()).map(|event| event.amount).sum();
    (remaining, prevented)
}
fn snapshot(game: &GameState, source: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), game)
}

#[test]
fn both_complete_frozen_cards_round_trip_strict_artifacts() {
    for name in ["Excruciator", "Questing Beast"] {
        for definition in definitions(name) { assert_eq!(definition.card.name, name); }
    }
}

#[test]
fn self_scope_disables_prevention_without_spending_the_shield_or_emitting_prevention() {
    for definition in definitions("Excruciator") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let unrelated = object(&mut game, a(), true);
        shield(&mut game, Some(3));
        assert_eq!(damage(&mut game, host, false, false, None), (3, 0));
        assert_eq!(damage(&mut game, host, true, false, None), (3, 0));
        assert_eq!(game.effect_store.prevention_effects.shields()[0].amount_remaining, Some(3));
        assert_eq!(damage(&mut game, unrelated, false, false, None), (0, 3));
    }
}

#[test]
fn combat_controller_and_creature_scopes_stay_independent_of_explicit_damage_flag() {
    for definition in definitions("Questing Beast") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let own = object(&mut game, a(), true);
        let foreign = object(&mut game, b(), true);
        let artifact = object(&mut game, a(), false);
        shield(&mut game, None);
        assert_eq!(damage(&mut game, own, true, false, None), (3, 0));
        assert_eq!(damage(&mut game, own, false, false, None), (0, 3));
        assert_eq!(damage(&mut game, foreign, true, false, None), (0, 3));
        assert_eq!(damage(&mut game, artifact, true, false, None), (0, 3));
        assert_eq!(damage(&mut game, artifact, false, true, None), (3, 0), "spell-local explicit unpreventability remains independent");
        game.set_current_controller(host, b()).unwrap();
        assert_eq!(damage(&mut game, own, true, false, None), (0, 3));
        assert_eq!(damage(&mut game, foreign, true, false, None), (3, 0));
    }
}

#[test]
fn source_lki_does_not_resurrect_an_inactive_restriction_host() {
    for definition in definitions("Questing Beast") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let source = object(&mut game, a(), true);
        let retained = snapshot(&game, source);
        shield(&mut game, None);
        game.set_current_controller(source, b()).unwrap();
        assert_eq!(damage(&mut game, source, true, false, Some(&retained)), (0, 3), "live controller overrides old LKI");
        game.set_current_controller(source, a()).unwrap();
        game.phase_out(source);
        assert_eq!(damage(&mut game, source, true, false, Some(&retained)), (3, 0), "phased-out damage source uses retained characteristics");
        game.phase_in(source);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, source, true, false, Some(&retained)), (3, 0));
        game.phase_out(host);
        assert_eq!(damage(&mut game, source, true, false, Some(&retained)), (0, 3));
        game.phase_in(host);
        assert_eq!(damage(&mut game, source, true, false, Some(&retained)), (3, 0));
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        assert_eq!(damage(&mut game, source, true, false, Some(&retained)), (0, 3));
    }
    for definition in definitions("Excruciator") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let retained = snapshot(&game, source);
        shield(&mut game, None);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, source, false, false, Some(&retained)), (0, 3), "a source's retained characteristics do not keep its battlefield static active");
    }
}

#[test]
fn finite_shared_shield_is_allocated_only_to_preventable_simultaneous_damage() {
    for definition in definitions("Excruciator") {
        for reverse in [false, true] {
            let mut game = game();
            let unpreventable = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let ordinary = object(&mut game, a(), true);
            shield(&mut game, Some(3));
            let mut events: Vec<_> = [unpreventable, ordinary].into_iter().map(|source| SimultaneousDamageEvent {
                source, target: DamageTarget::Player(b()), amount: 3, is_combat: true,
                unpreventable: false, cause: EventCause::effect(), source_snapshot: None,
            }).collect();
            if reverse { events.reverse(); }
            let processed = process_simultaneous_damage_assignments_with_event(&mut game, &events).unwrap();
            for (event, result) in events.iter().zip(processed) {
                let amount: u32 = result.assignments.iter().map(|assignment| assignment.amount).sum();
                assert_eq!(amount, if event.source == unpreventable { 3 } else { 0 });
                assert!(result.programs.is_empty());
            }
            let prevented: u32 = game.take_pending_trigger_events().iter().filter_map(|event| event.downcast::<DamagePreventedEvent>()).map(|event| event.amount).sum();
            assert_eq!(prevented, 3);
        }
    }
}

#[test]
fn genuine_damage_reduction_still_applies_to_unpreventable_damage() {
    for definition in definitions("Excruciator") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let reducer = object(&mut game, b(), false);
        std::sync::Arc::make_mut(&mut game.object_mut(reducer).unwrap().abilities).push(Ability::static_ability(
            StaticAbility::modify_damage_amount_replacement(ObjectFilter::default(), Some(PlayerFilter::Any), None, -1, "If a source would deal damage to a player, it deals that much damage minus 1 instead.".into())
        ));
        shield(&mut game, Some(3));
        assert_eq!(damage(&mut game, source, false, false, None), (2, 0));
        assert_eq!(game.effect_store.prevention_effects.shields()[0].amount_remaining, Some(3));
    }
}
