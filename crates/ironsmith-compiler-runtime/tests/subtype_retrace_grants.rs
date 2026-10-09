//! "<Subtype> and <Subtype> cards in your graveyard have retrace" (CR 702.81).
//! The retrace grammar existed for card-type subjects only and was reachable
//! only from those heads. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId(0);

const DEEPROOT_HISTORIAN: &str = "Mana cost: {3}{G}\nType: Creature — Merfolk Druid\nPower/Toughness: 3/3\nMerfolk and Druid cards in your graveyard have retrace. (You may cast cards with retrace from your graveyard by discarding a land card in addition to paying their other costs.)";

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

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for symbol in [ManaSymbol::Green, ManaSymbol::Blue, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 30);
    }
    game
}

fn card(game: &mut GameState, zone: Zone, name: &str, text: &str) -> ObjectId {
    game.create_object_from_definition(&compile_to_runtime_definition(name, text, false).unwrap(), A, zone)
}

fn casts(game: &GameState, id: ObjectId) -> usize {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .count()
}

#[test]
fn deeproot_historian_grants_retrace_to_merfolk_and_druid_cards() {
    for definition in routes("Deeproot Historian", DEEPROOT_HISTORIAN) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let grants: Vec<String> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .collect();
        assert_eq!(grants.len(), 1, "{grants:#?}");
        assert!(grants[0].contains("Merfolk") && grants[0].contains("Druid"), "{}", grants[0]);

        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        card(&mut game, Zone::Hand, "Discard fodder", "Type: Basic Land — Forest");
        let merfolk = card(&mut game, Zone::Graveyard, "Merfolk card", "Mana cost: {1}{U}\nType: Creature — Merfolk\nPower/Toughness: 2/2");
        let druid = card(&mut game, Zone::Graveyard, "Druid card", "Mana cost: {1}{G}\nType: Creature — Elf Druid\nPower/Toughness: 1/1");
        let bear = card(&mut game, Zone::Graveyard, "Bear card", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
        assert!(casts(&game, merfolk) > 0, "Merfolk card has retrace");
        assert!(casts(&game, druid) > 0, "Druid card has retrace");
        assert_eq!(casts(&game, bear), 0, "other creature cards do not");
    }
}

#[test]
fn card_type_retrace_grants_still_parse_and_mixed_lists_are_rejected() {
    let ok = "Mana cost: {2}{U}\nType: Enchantment\nInstant and sorcery cards in your graveyard have retrace.";
    for definition in routes("Card type retrace", ok) {
        assert_eq!(definition.abilities.len(), 1);
    }
    let mixed = "Mana cost: {2}{U}\nType: Enchantment\nInstant and Wizard cards in your graveyard have retrace.";
    assert!(compile_to_runtime_definition("Mixed retrace", mixed, false).is_err());
}
