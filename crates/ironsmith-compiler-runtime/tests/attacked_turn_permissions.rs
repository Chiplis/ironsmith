//! Source-authored and deliberately unrun (cf8 p04): "During any turn you
//! attacked with <creatures>, you may play that card." — a lasting
//! permission over the exiled card, usable only during turns in which its
//! player attacked with enough matching creatures (CR 508.1).
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::game_loop::resolve_stack_entry_with;
use ironsmith::game_state::StackEntry;
use ironsmith::grant_registry::GrantSource;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const COHORT: &[(&str, &str, u32)] = &[
    ("Boros Strike-Captain", "30b20932-0d9a-447f-b934-1daa8c44a678", 3),
    ("Goblin Researcher", "c8762dfa-a027-4e1c-a2fa-bfdab1de102a", 1),
    ("Neriv, Crackling Vanguard", "c8f827d7-4740-43c8-b494-c9e0479e0bf7", 1),
    ("Neyali, Suns' Vanguard", "4012b400-7dcd-43d6-8806-39a3cb743d8f", 1),
    ("Robber of the Rich", "4df0d381-fc05-4d30-9307-48963aa2ecfd", 1),
];

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/attacked_turn_permissions.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing fixture row {name}"));
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn frozen_bodies_carry_the_attack_turn_condition() {
    let rows = rows();
    assert_eq!(rows.len(), COHORT.len());
    for (name, oracle_id, minimum) in COHORT {
        assert!(
            rows.iter().any(|row| row["name"] == *name && row["oracle_id"] == *oracle_id),
            "{name}"
        );
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("during_turns_attacked_with: Some"), "{name}: {debug}");
            assert!(debug.contains(&format!("minimum: {minimum}")), "{name}: {debug}");
            assert!(debug.contains("ForAsLongAsExiled"), "{name}: {debug}");
        }
    }
    for definition in definitions("Robber of the Rich") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("AnyColor"), "{debug}");
        assert!(debug.contains("allow_land: false"), "{debug}");
    }
    for definition in definitions("Goblin Researcher") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("source: true"), "attacked with this creature: {debug}");
    }
}

fn card(game: &mut GameState, name: &str, text: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, A, zone)
}

fn record_attack(game: &mut GameState, attacker: ObjectId) {
    let snapshot =
        ironsmith::snapshot::ObjectSnapshot::from_object(game.object(attacker).unwrap(), game);
    game.turn_store
        .turn_history
        .event_records
        .push(ironsmith::turn_history::TurnEventRecord {
            event: ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::combat::CreatureAttackedEvent::new(
                    attacker,
                    ironsmith::triggers::AttackEventTarget::Player(B),
                ),
                Default::default(),
            ),
            object_snapshot: Some(snapshot),
            source_snapshot: None,
        });
}

#[test]
fn goblin_researcher_permission_is_live_only_on_turns_it_attacked() {
    for definition in definitions("Goblin Researcher") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        card(&mut game, "Top land", "Type: Land", Zone::Library);
        let researcher = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = card(
            &mut game,
            "Other attacker",
            "Mana cost: {1}{R}\nType: Creature — Goblin\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        let enters = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
                _ => None,
            })
            .expect("enters trigger");
        game.push_to_stack(StackEntry::ability(researcher, A, enters));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().library.len(), 0, "the top card was exiled");
        let grant = game
            .effect_store
            .grant_registry
            .grants
            .iter()
            .find(|grant| matches!(grant.source, GrantSource::EffectDuringTurnsAttackedWith { .. }))
            .cloned()
            .expect("attack-turn permission");
        let GrantSource::EffectDuringTurnsAttackedWith { player, minimum, .. } = &grant.source
        else {
            unreachable!()
        };
        assert_eq!((*player, *minimum), (A, 1));
        assert!(!grant.source.is_valid(&game), "no attack yet");
        record_attack(&mut game, other);
        assert!(!grant.source.is_valid(&game), "another creature attacking does not count");
        record_attack(&mut game, researcher);
        assert!(grant.source.is_valid(&game), "it attacked this turn");
        game.next_turn();
        assert!(!grant.source.is_valid(&game), "a new turn resets the condition");
    }
}
