//! Source-authored regression scenarios. Execution is deferred by the campaign workflow.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::object::AttachmentTarget;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, text, false)
    });
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    [direct, materialize_artifact(&decoded).unwrap()]
}

fn barrier(text: &str) -> [CardDefinition; 2] {
    definitions("Unlisted Barrier", &format!("Mana cost: {{2}}\nType: Enchantment\n{text}"))
}

fn creature(game: &mut GameState, player: PlayerId, subtype: Subtype) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Recipient probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 10))
        .build();
    game.create_object_from_card(&card, player, Zone::Battlefield)
}

fn damage(
    game: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    amount: u32,
    combat: bool,
    unpreventable: bool,
) -> (u32, Vec<(u32, ObjectId, PlayerId)>) {
    game.take_pending_trigger_events();
    let processed = process_damage_assignments_with_event_with_source_snapshot_opts(
        game, source, target, amount, combat, unpreventable, EventCause::effect(), None,
    ).unwrap();
    let remaining = processed.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented = game.take_pending_trigger_events().into_iter().filter_map(|event| {
        event.downcast::<DamagePreventedEvent>()
            .map(|event| (event.amount, event.prevention_source, event.prevention_controller))
    }).collect();
    (remaining, prevented)
}

#[test]
fn complete_frozen_prevention_candidates_keep_strict_artifact_semantics() {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/static_damage_prevention.json.fixture"
    )).unwrap();
    assert_eq!(cards.len(), 9);
    for card in cards {
        let mut text = format!("Mana cost: {}\nType: {}\n",
            card["mana_cost"].as_str().unwrap_or(""), card["type_line"].as_str().unwrap());
        if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
            text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
        }
        text.push_str(card["oracle_text"].as_str().unwrap());
        for definition in definitions(card["name"].as_str().unwrap(), &text) {
            assert_eq!(definition.card.name, card["name"].as_str().unwrap());
            assert!(!definition.abilities.is_empty());
        }
    }
}

#[test]
fn fixed_prevention_filters_live_recipients_and_reports_actual_prevention() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in barrier("If a source would deal damage to a Cleric creature you control, prevent 1 of that damage.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let attacker = creature(&mut game, bob, Subtype::Warrior);
        let own_cleric = creature(&mut game, alice, Subtype::Cleric);
        let own_other = creature(&mut game, alice, Subtype::Wizard);
        let opposing_cleric = creature(&mut game, bob, Subtype::Cleric);
        for combat in [false, true] {
            assert_eq!(damage(&mut game, attacker, DamageTarget::Object(own_cleric), 3, combat, false),
                (2, vec![(1, host, alice)]));
            assert_eq!(damage(&mut game, attacker, DamageTarget::Object(own_other), 3, combat, false).0, 3);
            assert_eq!(damage(&mut game, attacker, DamageTarget::Object(opposing_cleric), 3, combat, false).0, 3);
        }
        assert_eq!(damage(&mut game, attacker, DamageTarget::Object(own_cleric), 1, false, false),
            (0, vec![(1, host, alice)]));
        assert_eq!(damage(&mut game, attacker, DamageTarget::Object(own_cleric), 3, false, true), (3, vec![]));
        game.set_current_controller(host, bob).unwrap();
        assert_eq!(damage(&mut game, attacker, DamageTarget::Object(own_cleric), 3, false, false).0, 3);
        assert_eq!(damage(&mut game, attacker, DamageTarget::Object(opposing_cleric), 3, false, false),
            (2, vec![(1, host, bob)]));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, attacker, DamageTarget::Object(opposing_cleric), 3, false, false).0, 3);
    }
}

#[test]
fn all_but_prevents_excess_without_replacing_unpreventable_damage() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in barrier("If a source would deal damage to you or a Hero you control, prevent all but 1 of that damage.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        let hero = creature(&mut game, alice, Subtype::Hero);
        let opposing_hero = creature(&mut game, bob, Subtype::Hero);
        for target in [DamageTarget::Player(alice), DamageTarget::Object(hero)] {
            assert_eq!(damage(&mut game, source, target, 1, false, false), (1, vec![]));
            assert_eq!(damage(&mut game, source, target, 5, false, false), (1, vec![(4, host, alice)]));
            assert_eq!(damage(&mut game, source, target, 5, true, true), (5, vec![]));
        }
        assert_eq!(damage(&mut game, source, DamageTarget::Player(bob), 5, false, false).0, 5);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(opposing_hero), 5, false, false).0, 5);
    }
}

#[test]
fn threshold_prevention_uses_each_proposed_event_amount_and_source_identity() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let text = "Mana cost: {4}{R}{R}\nType: Creature — Giant\nPower/Toughness: 4/4\nIf a source would deal 3 or less damage to this creature, prevent that damage.";
    for definition in definitions("Unnamed Threshold Giant", text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let giant = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        let other = creature(&mut game, alice, Subtype::Giant);
        for amount in [1, 2, 3] {
            assert_eq!(damage(&mut game, source, DamageTarget::Object(giant), amount, false, false),
                (0, vec![(amount, giant, alice)]));
        }
        assert_eq!(damage(&mut game, source, DamageTarget::Object(giant), 4, false, false), (4, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(giant), 3, false, true), (3, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(other), 3, false, false), (3, vec![]));
    }
}

#[test]
fn dynamic_equipment_prevention_tracks_attachment_and_ability_controller() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let text = "Mana cost: {1}\nType: Artifact — Equipment\nIf a source would deal damage to equipped creature, prevent X of that damage, where X is the number of creatures you control.\nEquip {2}";
    for definition in definitions("Unnamed Counted Shield", text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let shield = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        let protected = creature(&mut game, alice, Subtype::Cleric);
        let other = creature(&mut game, alice, Subtype::Wizard);
        assert!(game.attach_object_to_target(shield, AttachmentTarget::Object(protected)));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(protected), 5, false, false), (3, vec![(2, shield, alice)]));
        let _later = creature(&mut game, alice, Subtype::Cleric);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(protected), 5, false, false), (2, vec![(3, shield, alice)]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(other), 5, false, false), (5, vec![]));
        game.set_current_controller(shield, bob).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(protected), 5, false, false), (4, vec![(1, shield, bob)]));
        assert!(game.attach_object_to_target(shield, AttachmentTarget::Object(source)));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(protected), 5, false, false), (5, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(source), 5, false, false), (4, vec![(1, shield, bob)]));
    }
}

#[test]
fn typed_source_and_combat_qualifiers_are_not_erased() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in barrier("If a creature an opponent controls would deal combat damage to a Cleric creature you control, prevent 2 of that damage.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let own_source = creature(&mut game, alice, Subtype::Warrior);
        let opposing_source = creature(&mut game, bob, Subtype::Warrior);
        let cleric = creature(&mut game, alice, Subtype::Cleric);
        assert_eq!(damage(&mut game, opposing_source, DamageTarget::Object(cleric), 3, true, false), (1, vec![(2, host, alice)]));
        assert_eq!(damage(&mut game, opposing_source, DamageTarget::Object(cleric), 3, false, false), (3, vec![]));
        assert_eq!(damage(&mut game, own_source, DamageTarget::Object(cleric), 3, true, false), (3, vec![]));
    }
}

#[test]
fn unimplemented_optional_and_follow_up_prevention_stays_fail_closed() {
    for text in [
        "If a spell you control would deal damage to an opponent, prevent that damage. Create a 3/1 red Elemental Shaman creature token with haste for each 1 damage prevented this way.",
        "If a source would deal damage to this creature, prevent that damage. The source's controller draws cards equal to the damage prevented this way.",
        "If a source would deal damage to a player, you may prevent X of that damage, where X is the number of Clerics you control.",
    ] {
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
            compile_to_artifact("Unclaimed follow-up", format!(
                "Mana cost: {{3}}\nType: Creature — Elemental\nPower/Toughness: 3/3\n{text}"
            ), false)
        });
        assert!(result.is_err() || loss.is_lossy(),
            "the simple replacement must not turn an unimplemented optional/follow-up program into apparent support: {text}");
    }
}
