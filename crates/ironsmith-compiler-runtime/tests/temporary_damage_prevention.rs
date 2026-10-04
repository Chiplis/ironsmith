//! Source-authored scenarios; builds and execution are deferred by the campaign.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::game_loop::{extract_target_requirements_from_program_with_modes, resolve_stack_entry};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}

fn spells(text: &str) -> [CardDefinition; 2] {
    definitions("Unlisted Temporary Shelter", &format!("Mana cost: {{1}}{{W}}\nType: Instant\n{text}"))
}

fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), "Damage probe")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 6)).build(), owner, Zone::Battlefield)
}

fn resolve(game: &mut GameState, definition: &CardDefinition, targets: Vec<Target>) {
    let alice = PlayerId::from_index(0);
    let spell = game.create_object_from_definition(definition, alice, Zone::Stack);
    let requirements = extract_target_requirements_from_program_with_modes(game, definition.spell_effect.as_ref().unwrap(), alice, Some(spell), None);
    let assignments = if targets.is_empty() {
        assert!(requirements.is_empty());
        vec![]
    } else {
        assert_eq!(requirements.len(), 1);
        let requirement = &requirements[0];
        assert!(targets.len() >= requirement.min_targets);
        assert!(requirement.max_targets.is_none_or(|maximum| targets.len() <= maximum));
        for target in &targets { assert!(requirement.legal_targets.contains(target)); }
        vec![TargetAssignment { spec: requirement.spec.clone(), range: 0..targets.len() }]
    };
    game.push_to_stack(StackEntry::new(spell, alice).with_targets(targets).with_target_assignments(assignments));
    resolve_stack_entry(game).unwrap();
}

fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget, combat: bool, unpreventable: bool) -> (u32, u32) {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(game, source, target, 3, combat, unpreventable, EventCause::effect(), None).unwrap();
    let remaining = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented = game.take_pending_trigger_events().iter().filter_map(|event| event.downcast::<DamagePreventedEvent>().map(|event| event.amount)).sum();
    (remaining, prevented)
}

#[test]
fn aggregate_combat_recipients_keep_players_live_filters_and_expiry() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in spells("Prevent all combat damage that would be dealt to you and creatures you control this turn.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = creature(&mut game, bob);
        resolve(&mut game, &definition, vec![]);
        let later = creature(&mut game, alice);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), true, false), (0, 3));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(later), true, false), (0, 3));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(later), false, false), (3, 0));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(later), true, true), (3, 0));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(bob), true, false), (3, 0));
        game.set_current_controller(later, bob).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(later), true, false), (3, 0));
        game.effect_store.prevention_effects.cleanup_end_of_turn();
        assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), true, false), (3, 0));
    }
}

#[test]
fn both_selected_damage_sources_receive_independent_unlimited_shields() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in spells("Prevent all damage one or two target creatures would deal this turn.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let first = creature(&mut game, bob); let second = creature(&mut game, bob);
        let other = creature(&mut game, bob);
        resolve(&mut game, &definition, vec![Target::Object(first), Target::Object(second)]);
        for source in [first, second] {
            assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), true, false), (0, 3));
            assert_eq!(damage(&mut game, source, DamageTarget::Object(other), false, false), (0, 3));
            assert_eq!(damage(&mut game, source, DamageTarget::Player(alice), false, true), (3, 0));
        }
        assert_eq!(damage(&mut game, other, DamageTarget::Player(alice), true, false), (3, 0));
        game.effect_store.prevention_effects.cleanup_end_of_turn();
        assert_eq!(damage(&mut game, second, DamageTarget::Player(alice), true, false), (3, 0));
    }
}

#[test]
fn all_damage_source_filter_includes_later_creatures_without_becoming_a_target() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in spells("Prevent all damage that would be dealt by creatures this turn.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        resolve(&mut game, &definition, vec![]);
        let later = creature(&mut game, bob);
        let noncreature = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Spell probe").card_types(vec![CardType::Sorcery]).build(), bob, Zone::Stack);
        for combat in [true, false] {
            assert_eq!(damage(&mut game, later, DamageTarget::Player(alice), combat, false), (0, 3));
        }
        assert_eq!(damage(&mut game, noncreature, DamageTarget::Player(alice), false, false), (3, 0));
    }
}

#[test]
fn targeted_combat_recipient_keeps_identity_and_does_not_prevent_noncombat_damage() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in spells("Prevent all combat damage that would be dealt to target creature this turn.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let target = creature(&mut game, alice); let source = creature(&mut game, bob);
        resolve(&mut game, &definition, vec![Target::Object(target)]);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(target), true, false), (0, 3));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(target), false, false), (3, 0));
        let other = creature(&mut game, alice);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(other), true, false), (3, 0));
    }
}


#[test]
fn frozen_temporary_prevention_candidates_keep_complete_strict_artifacts() {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/temporary_damage_prevention.json.fixture")).unwrap();
    assert_eq!(cards.len(), 10);
    for card in cards {
        let mut text = format!("Mana cost: {}\nType: {}\n", card["mana_cost"].as_str().unwrap_or(""), card["type_line"].as_str().unwrap());
        if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
            text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
        }
        text.push_str(card["oracle_text"].as_str().unwrap());
        for definition in definitions(card["name"].as_str().unwrap(), &text) {
            assert_eq!(definition.card.name, card["name"].as_str().unwrap());
        }
    }
}

#[test]
fn referenced_untap_group_keeps_all_selected_identities_in_both_directions() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in spells("Untap any number of target creatures. Prevent all combat damage that would be dealt to and dealt by those creatures this turn.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let first = creature(&mut game, alice); let second = creature(&mut game, bob);
        let foreign = creature(&mut game, bob);
        // They are already untapped: protection refers to the declared targets,
        // not only objects whose tapped state actually changed.
        resolve(&mut game, &definition, vec![Target::Object(first), Target::Object(second)]);
        for chosen in [first, second] {
            assert_eq!(damage(&mut game, chosen, DamageTarget::Player(alice), true, false), (0, 3));
            assert_eq!(damage(&mut game, foreign, DamageTarget::Object(chosen), true, false), (0, 3));
            assert_eq!(damage(&mut game, foreign, DamageTarget::Object(chosen), false, false), (3, 0));
        }
        assert_eq!(damage(&mut game, foreign, DamageTarget::Player(alice), true, false), (3, 0));
    }
}

#[test]
fn this_combat_artifact_shield_is_inactive_after_the_combat_boundary() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in spells("Prevent all combat damage that would be dealt to target creature this combat.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::CombatDamage);
        let target = creature(&mut game, alice); let source = creature(&mut game, bob);
        resolve(&mut game, &definition, vec![Target::Object(target)]);
        assert!(game.effect_store.prevention_effects.shields().iter().all(|shield| shield.duration == ironsmith::effect::Until::EndOfCombat));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(target), true, false), (0, 3));
        game.cleanup_effects_end_of_combat();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(target), true, false), (3, 0));
    }
}
