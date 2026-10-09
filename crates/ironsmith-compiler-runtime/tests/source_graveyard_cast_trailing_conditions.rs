//! "You may cast this card from your graveyard if/as long as <condition>."
//! The trailing condition gates the same graveyard permission as the leading
//! "As long as ..., you may cast this card from your graveyard" form
//! (CR 601.3). "<object> died this turn" reads CR 700.4 death history through
//! the shared object filter. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

const CARDS: &[(&str, &str, &str)] = &[
    (
        "Oathsworn Vampire",
        "e1fb696f-1df8-4571-9710-0d79001326d3",
        "Mana cost: {1}{B}\nType: Creature — Vampire Knight\nPower/Toughness: 2/2\nThis creature enters tapped.\nYou may cast this card from your graveyard if you gained life this turn.",
    ),
    (
        "Ebondeath, Dracolich",
        "1a6fce11-e77c-41f1-9ff7-9a93b18071db",
        "Mana cost: {2}{B}{B}\nType: Legendary Creature — Zombie Dragon\nPower/Toughness: 5/2\nFlash\nFlying\nEbondeath enters tapped.\nYou may cast this card from your graveyard if a creature not named Ebondeath, Dracolich died this turn.",
    ),
    (
        "The Indomitable",
        "276dc5c8-c8cf-4b9c-ad75-31876e6e040a",
        "Mana cost: {2}{U}{U}\nType: Legendary Artifact — Vehicle\nPower/Toughness: 6/6\nTrample\nWhenever a creature you control deals combat damage to a player, draw a card.\nCrew 3\nYou may cast this card from your graveyard as long as you control three or more tapped Pirates and/or Vehicles.",
    ),
    (
        "Undead Sprinter",
        "3416ac84-c5ef-44be-894f-ab3b89e592ce",
        "Mana cost: {B}{R}\nType: Creature — Zombie\nPower/Toughness: 2/2\nTrample, haste\nYou may cast this card from your graveyard if a non-Zombie creature died this turn. If you do, this creature enters with a +1/+1 counter on it.",
    ),
];

fn text(name: &str) -> &'static str {
    CARDS.iter().find(|(card, _, _)| *card == name).unwrap().2
}

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

fn graveyard_permissions(definition: &CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter(|ability| ability.functional_zones == vec![Zone::Graveyard])
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(format!("{ability:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn trailing_condition_permissions_compile_as_conditional_graveyard_grants() {
    for &(name, _oracle_id, text) in CARDS {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let permissions = graveyard_permissions(&definition);
            assert_eq!(permissions.len(), 1, "{name}: {permissions:#?}");
            assert!(permissions[0].contains("Conditional"), "{name}: {}", permissions[0]);
            assert!(permissions[0].contains("PlayFrom"), "{name}: {}", permissions[0]);
            if name == "Undead Sprinter" {
                assert!(permissions[0].contains("PlusOnePlusOne"), "cast-this-way rider: {}", permissions[0]);
            }
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(rendered.contains("from your graveyard"), "{rendered}");
        }
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 30);
    }
    game
}

fn casts(game: &GameState, id: ObjectId) -> usize {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .count()
}

fn permanent(game: &mut GameState, name: &str, type_line: &str) -> ObjectId {
    let text = format!("Mana cost: {{1}}\nType: {type_line}\nPower/Toughness: 2/2");
    game.create_object_from_definition(
        &compile_to_runtime_definition(name, &text, false).unwrap(),
        A,
        Zone::Battlefield,
    )
}

/// CR 700.4: record a battlefield-to-graveyard move in the turn history.
fn dies(game: &mut GameState, id: ObjectId) {
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), game);
    game.move_object_by_effect(id, Zone::Graveyard).unwrap();
    let event = ironsmith::events::RawEvent::new(
        ironsmith::events::ZoneChangeEvent::with_cause(
            id,
            Zone::Battlefield,
            Zone::Graveyard,
            ironsmith::events::cause::EventCause::effect(),
            Some(snapshot.clone()),
        ),
        game.provenance_graph_mut()
            .alloc_root_event(ironsmith::events::EventKind::ZoneChange),
    );
    game.turn_store.turn_history.record_event(&event, Some(snapshot), None);
}

#[test]
fn filtered_death_history_gates_the_graveyard_permission() {
    for name in ["Ebondeath, Dracolich", "Undead Sprinter"] {
        for definition in routes(name, text(name)) {
            let mut game = game();
            let card = game.create_object_from_definition(&definition, A, Zone::Graveyard);
            assert_eq!(casts(&game, card), 0, "{name}: no death yet");
            if name == "Undead Sprinter" {
                // A Zombie's death is outside the "non-Zombie creature" filter.
                let zombie = permanent(&mut game, "Zombie witness", "Creature — Zombie");
                dies(&mut game, zombie);
                assert_eq!(casts(&game, card), 0, "{name}: a Zombie died");
            } else {
                // A creature named Ebondeath, Dracolich is excluded by name.
                let namesake = permanent(&mut game, "Ebondeath, Dracolich", "Creature — Zombie Dragon");
                dies(&mut game, namesake);
                assert_eq!(casts(&game, card), 0, "{name}: only its namesake died");
            }
            let bear = permanent(&mut game, "Bear witness", "Creature — Bear");
            dies(&mut game, bear);
            assert!(casts(&game, card) > 0, "{name}: a matching creature died this turn");
        }
    }
}

#[test]
fn tapped_pirates_and_vehicles_gate_the_indomitable() {
    for definition in routes("The Indomitable", text("The Indomitable")) {
        let mut game = game();
        let card = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let crew = [
            permanent(&mut game, "Pirate one", "Creature — Human Pirate"),
            permanent(&mut game, "Pirate two", "Creature — Human Pirate"),
            permanent(&mut game, "Vehicle", "Artifact — Vehicle"),
        ];
        assert_eq!(casts(&game, card), 0, "untapped Pirates/Vehicles do not count");
        game.tap(crew[0]);
        game.tap(crew[1]);
        assert_eq!(casts(&game, card), 0, "only two are tapped");
        game.tap(crew[2]);
        assert!(casts(&game, card) > 0, "three tapped Pirates and/or Vehicles");
        let opposing = game.create_object_from_definition(
            &compile_to_runtime_definition(
                "Opposing pirate",
                "Mana cost: {1}\nType: Creature — Human Pirate\nPower/Toughness: 2/2",
                false,
            )
            .unwrap(),
            B,
            Zone::Battlefield,
        );
        game.tap(opposing);
        game.untap(crew[2]);
        assert_eq!(casts(&game, card), 0, "an opponent's tapped Pirate does not count");
    }
}

#[test]
fn ordinary_unconditioned_graveyard_permission_is_unchanged() {
    let text = "Mana cost: {B}\nType: Creature — Zombie\nPower/Toughness: 1/1\nYou may cast this card from your graveyard.";
    for definition in routes("Unconditioned recursion", text) {
        let permissions = graveyard_permissions(&definition);
        assert_eq!(permissions.len(), 1);
        assert!(!permissions[0].contains("Conditional"), "{}", permissions[0]);
    }
}
