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
    assert_eq!(cards.len(), 18);
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
        "If damage would be dealt to this creature, prevent that damage. When damage is prevented this way, this creature deals that much damage to any other target.",
        "If damage would be dealt to you, prevent that damage. You gain life equal to the damage prevented this way. Draw a card.",
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


#[test]
fn spell_reduction_applies_to_each_simultaneous_recipient_without_affecting_creatures() {
    use ironsmith::events::processing::{
        SimultaneousDamageEvent, process_simultaneous_damage_assignments_with_event,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in barrier("If a spell would deal damage to a permanent or player, prevent 1 damage that spell would deal to that permanent or player.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell_card = CardBuilder::new(CardId::new(), "Simultaneous damage spell")
            .card_types(vec![CardType::Sorcery]).build();
        let spell = game.create_object_from_card(&spell_card, bob, Zone::Stack);
        let own = creature(&mut game, alice, Subtype::Cleric);
        let other = creature(&mut game, bob, Subtype::Warrior);
        let events = [DamageTarget::Player(alice), DamageTarget::Player(bob),
            DamageTarget::Object(own), DamageTarget::Object(other)]
            .into_iter().map(|target| SimultaneousDamageEvent {
                source: spell, target, amount: 3, is_combat: false,
                unpreventable: false, cause: EventCause::effect(), source_snapshot: None,
            }).collect::<Vec<_>>();
        game.take_pending_trigger_events();
        let results = process_simultaneous_damage_assignments_with_event(&mut game, &events).unwrap();
        assert_eq!(results.len(), 4);
        assert!(results.iter().all(|result| result.assignments.iter().map(|damage| damage.amount).sum::<u32>() == 2));
        let prevented = game.take_pending_trigger_events().into_iter()
            .filter_map(|event| event.downcast::<DamagePreventedEvent>()
                .map(|event| (event.amount, event.prevention_source)))
            .collect::<Vec<_>>();
        assert_eq!(prevented.len(), 4);
        assert!(prevented.iter().all(|entry| *entry == (1, host)));
        assert_eq!(damage(&mut game, other, DamageTarget::Player(alice), 3, false, false), (3, vec![]));
        assert_eq!(damage(&mut game, spell, DamageTarget::Player(alice), 3, false, true), (3, vec![]));
    }
}


#[test]
fn counter_followups_use_actual_prevention_and_preserve_the_recipient() {
    use ironsmith::object::CounterType;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Unlisted Minus Counter Creature", "Mana cost: {3}{G}{G}\nType: Creature — Hydra\nPower/Toughness: 7/7\nIf damage would be dealt to this creature, prevent that damage. Put a -1/-1 counter on this creature for each 1 damage prevented this way.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, false, false), (0, vec![(3, host, alice)]));
        assert_eq!(game.counter_count(host, CounterType::MinusOneMinusOne), 3);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 2, false, true), (2, vec![]));
        assert_eq!(game.counter_count(host, CounterType::MinusOneMinusOne), 3,
            "an additional effect scaled by actual prevention adds zero counters for unpreventable damage");
    }
    for definition in definitions("Unlisted Allied Counter Creature", "Mana cost: {3}{G}{G}{G}\nType: Creature — Elemental\nPower/Toughness: 6/6\nIf damage would be dealt to another creature you control, prevent that damage. Put a +1/+1 counter on that creature for each 1 damage prevented this way.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        let protected = creature(&mut game, alice, Subtype::Cleric);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(protected), 3, false, false), (0, vec![(3, host, alice)]));
        assert_eq!(game.counter_count(protected, CounterType::PlusOnePlusOne), 3);
        assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), 0);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, false, false), (3, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(source), 3, false, false), (3, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(protected), 2, false, true), (2, vec![]));
        assert_eq!(game.counter_count(protected, CounterType::PlusOnePlusOne), 3);
    }
}


#[test]
fn prevention_life_followup_uses_actual_prevention_and_noncombat_scope() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in barrier("If noncombat damage would be dealt to you, prevent that damage. You gain life equal to the damage prevented this way.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), 3, false, false), (0, vec![(3, host, alice)]));
        assert_eq!(game.player(alice).unwrap().life, 23);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), 2, false, true), (2, vec![]));
        assert_eq!(game.player(alice).unwrap().life, 23, "actual prevention is zero");
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), 2, true, false), (2, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(bob), 2, false, false), (2, vec![]));
    }
}

#[test]
fn source_controller_draw_followup_retains_live_or_supplied_damage_source_lki() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let text = "Mana cost: {2}{W}{U}\nType: Creature — Bird Spirit\nPower/Toughness: 4/3\nIf a source would deal damage to this creature, prevent that damage. The source's controller draws cards equal to the damage prevented this way.";
    for definition in definitions("Unlisted Drawing Bird", text) {
        for departed in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let source = creature(&mut game, bob, Subtype::Warrior);
            for index in 0..5 {
                let card = CardBuilder::new(CardId::new(), format!("Source-controller draw {index}"))
                    .card_types(vec![CardType::Artifact]).build();
                game.create_object_from_card(&card, bob, Zone::Library);
            }
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(), &game,
            );
            if departed {
                game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                // Prove the explicitly supplied snapshot survives the full queue;
                // a stale turn-history lookup is not an adequate substitute.
                game.turn_store.turn_history.clear_for_new_turn();
            }
            game.take_pending_trigger_events();
            let processed = process_damage_assignments_with_event_with_source_snapshot_opts(
                &mut game, source, DamageTarget::Object(host), 3, false, false,
                EventCause::effect(), Some(&snapshot),
            ).unwrap();
            assert_eq!(processed.assignments.iter().map(|damage| damage.amount).sum::<u32>(), 0);
            assert_eq!(game.player(bob).unwrap().hand.len(), 3);
            assert!(game.player(alice).unwrap().hand.is_empty(), "draw belongs to the damage source's controller");
            let prevented = game.take_pending_trigger_events().into_iter()
                .filter_map(|event| event.downcast::<DamagePreventedEvent>().map(|event| event.amount))
                .collect::<Vec<_>>();
            assert_eq!(prevented, vec![3]);
            let processed = process_damage_assignments_with_event_with_source_snapshot_opts(
                &mut game, source, DamageTarget::Object(host), 2, false, true,
                EventCause::effect(), Some(&snapshot),
            ).unwrap();
            assert_eq!(processed.assignments.iter().map(|damage| damage.amount).sum::<u32>(), 2);
            assert_eq!(game.player(bob).unwrap().hand.len(), 3, "unpreventable damage draws zero cards");
            assert!(!game.take_pending_trigger_events().iter().any(|event| event.downcast::<DamagePreventedEvent>().is_some()));
        }
    }
}

#[test]
fn prevented_damage_token_followup_preserves_created_characteristics_and_source_scope() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let text = "If a spell you control would deal damage to an opponent, prevent that damage. Create a 3/1 red Elemental Shaman creature token with haste for each 1 damage prevented this way.";
    for definition in barrier(text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell_card = CardBuilder::new(CardId::new(), "Owned damage spell")
            .card_types(vec![CardType::Sorcery]).build();
        let spell = game.create_object_from_card(&spell_card, alice, Zone::Stack);
        assert_eq!(damage(&mut game, spell, DamageTarget::Player(bob), 3, false, false), (0, vec![(3, host, alice)]));
        let tokens = game.battlefield.iter().copied().filter(|id| *id != host).collect::<Vec<_>>();
        assert_eq!(tokens.len(), 3);
        for token in &tokens {
            assert_eq!(game.current_power(*token), Some(3));
            assert_eq!(game.current_toughness(*token), Some(1));
            assert_eq!(game.current_controller(*token), Some(alice));
            assert!(game.current_has_subtype(*token, Subtype::Elemental));
            assert!(game.current_has_subtype(*token, Subtype::Shaman));
            assert!(game.current_has_static_ability_id(*token, ironsmith::static_abilities::StaticAbilityId::Haste));
        }
        assert_eq!(damage(&mut game, spell, DamageTarget::Player(bob), 2, false, true), (2, vec![]));
        assert_eq!(game.battlefield.len(), 4, "zero actual prevention creates zero tokens");
        let foreign_spell = game.create_object_from_card(&spell_card, bob, Zone::Stack);
        assert_eq!(damage(&mut game, foreign_spell, DamageTarget::Player(bob), 2, false, false), (2, vec![]));
        assert_eq!(damage(&mut game, spell, DamageTarget::Player(alice), 2, false, false), (2, vec![]));
        assert_eq!(game.battlefield.len(), 4);
    }
}


fn fill_library(game: &mut GameState, owner: PlayerId, count: usize) {
    for index in 0..count {
        let card = CardBuilder::new(CardId::new(), format!("Milling probe {index}"))
            .card_types(vec![CardType::Artifact]).build();
        game.create_object_from_card(&card, owner, Zone::Library);
    }
}

#[test]
fn conjoined_milling_uses_proposed_damage_even_when_prevention_is_prohibited() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in barrier("If damage would be dealt to you, prevent that damage and mill twice that many cards.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        fill_library(&mut game, alice, 10);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), 3, false, false), (0, vec![(3, host, alice)]));
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 6);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), 2, true, true), (2, vec![]));
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 10, "unpreventable proposed damage still mills twice its amount");
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), 1, false, false), (0, vec![(1, host, alice)]));
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 10, "empty library does not stop prevention");
        assert_eq!(damage(&mut game, source, DamageTarget::Player(bob), 3, false, false), (3, vec![]));
    }
}

#[test]
fn conjoined_opponent_milling_keeps_source_scope_and_all_opponents() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for definition in barrier("If a source you control would deal damage to an opponent, prevent that damage and each opponent mills that many cards.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, alice, Subtype::Warrior);
        let foreign = creature(&mut game, bob, Subtype::Warrior);
        for player in [alice, bob, carol] { fill_library(&mut game, player, 10); }
        assert_eq!(damage(&mut game, source, DamageTarget::Player(bob), 3, false, false), (0, vec![(3, host, alice)]));
        for player in [bob, carol] { assert_eq!(game.player(player).unwrap().graveyard.len(), 3); }
        assert!(game.player(alice).unwrap().graveyard.is_empty());
        assert_eq!(damage(&mut game, source, DamageTarget::Player(carol), 2, true, true), (2, vec![]));
        for player in [bob, carol] { assert_eq!(game.player(player).unwrap().graveyard.len(), 5); }
        assert_eq!(damage(&mut game, foreign, DamageTarget::Player(carol), 2, false, false), (2, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), 2, false, false), (2, vec![]));
        for player in [bob, carol] { assert_eq!(game.player(player).unwrap().graveyard.len(), 5); }
        // With preventable damage, the first chosen prevention removes the event;
        // a second instance must not mill a second time.
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(bob), 1, false, false).0, 0);
        for player in [bob, carol] { assert_eq!(game.player(player).unwrap().graveyard.len(), 6); }
    }
}


#[test]
fn fixed_counter_removal_is_true_prevention_with_an_independent_additional_part() {
    use ironsmith::object::CounterType;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Unlisted Counter Horde", "Mana cost: {2}{B}
Type: Creature — Zombie
Power/Toughness: 2/10
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = creature(&mut game, bob, Subtype::Warrior);
        game.add_counters(host, CounterType::PlusOnePlusOne, 2);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 5, false, false), (0, vec![(5, host, alice)]));
        assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), 1);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, false, true), (3, vec![]));
        assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), 0, "the fixed additional action still occurs for unpreventable damage");
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 7, true, false), (0, vec![(7, host, alice)]), "prevention is not conditional on having a counter to remove");
        assert_eq!(damage(&mut game, source, DamageTarget::Object(source), 1, true, false), (1, vec![]));
    }
}


#[test]
fn damage_to_counter_replacement_and_two_prevention_amounts_stay_distinct() {
    use ironsmith::object::CounterType;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (ability, is_prevention, uses_proposed) in [
        ("If damage would be dealt to this creature, put that many +1/+1 counters on it instead.", false, true),
        ("If damage would be dealt to this creature, prevent that damage and put that many +1/+1 counters on it.", true, true),
        ("If damage would be dealt to this creature, prevent that damage. Put a +1/+1 counter on this creature for each 1 damage prevented this way.", true, false),
    ] {
        for definition in definitions("Unlisted Counter Semantics", &format!("Mana cost: {{2}}{{G}}\nType: Creature — Plant\nPower/Toughness: 2/10\n{ability}")) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let source = creature(&mut game, bob, Subtype::Warrior);
            assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, false, false),
                (0, if is_prevention { vec![(3, host, alice)] } else { vec![] }));
            assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), 3);
            assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 2, false, true),
                (if is_prevention { 2 } else { 0 }, vec![]),
                "a genuine replacement still replaces unpreventable damage; prevention cannot");
            assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), if uses_proposed { 5 } else { 3 });
            assert_eq!(damage(&mut game, source, DamageTarget::Object(source), 1, false, false), (1, vec![]));
        }
    }
}


#[test]
fn horde_complete_entry_count_and_prevention_keep_each_zone_and_controller() {
    use ironsmith::object::CounterType;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let text = "Mana cost: {2}{B}\nType: Creature — Zombie\nPower/Toughness: 0/0\nThis creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.\nIf this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.";
    for definition in definitions("Unlisted Whole Horde", text) {
        for from in [Zone::Stack, Zone::Graveyard] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            creature(&mut game, alice, Subtype::Zombie);
            creature(&mut game, alice, Subtype::Zombie);
            creature(&mut game, bob, Subtype::Zombie);
            creature(&mut game, alice, Subtype::Warrior);
            let zombie = CardBuilder::new(CardId::new(), "Graveyard Zombie")
                .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Zombie]).build();
            for _ in 0..3 { game.create_object_from_card(&zombie, alice, Zone::Graveyard); }
            game.create_object_from_card(&zombie, bob, Zone::Graveyard);
            let irrelevant = CardBuilder::new(CardId::new(), "Graveyard Warrior")
                .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Warrior]).build();
            game.create_object_from_card(&irrelevant, alice, Zone::Graveyard);
            let old_host = game.create_object_from_definition(&definition, alice, from);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let receipt = game.move_object_with_etb_processing_with_dm(old_host, Zone::Battlefield, &mut dm).unwrap();
            assert!(!receipt.pending);
            assert!(receipt.programs.is_empty(), "fixture must not discard added entry instructions");
            let entry = receipt.original.into_result().unwrap();
            let host = entry.new_id;
            // A direct graveyard entry counts the entering card in that old
            // zone, before the replacement-modified entry is committed.
            let expected = if from == Zone::Graveyard { 6 } else { 5 };
            assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), expected);
            let source = creature(&mut game, bob, Subtype::Warrior);
            assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, false, false), (0, vec![(3, host, alice)]));
            assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), expected - 1);
        }
    }
}
