//! Scoped "spend mana as though it were mana of any color" permissions
//! (CR 609.4b): casting-scoped (Oath of Nissa) and one-color source-activation
//! scoped (Quicksilver Elemental). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId(0);

const OATH_OF_NISSA: &str = "Mana cost: {G}\nType: Legendary Enchantment\nWhen Oath of Nissa enters, look at the top three cards of your library. You may reveal a creature, land, or planeswalker card from among them and put it into your hand. Put the rest on the bottom of your library in any order.\nYou may spend mana as though it were mana of any color to cast planeswalker spells.";
const QUICKSILVER_ELEMENTAL: &str = "Mana cost: {3}{U}{U}\nType: Creature — Elemental\nPower/Toughness: 3/4\n{U}: This creature gains all activated abilities of target creature until end of turn. (If any of the abilities use that creature's name, use this creature's name instead.)\nYou may spend blue mana as though it were mana of any color to pay the activation costs of this creature's abilities.";

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

fn statics(definition: &CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(format!("{ability:?}")),
            _ => None,
        })
        .collect()
}

fn casts(game: &GameState, id: ObjectId) -> usize {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .count()
}

#[test]
fn oath_of_nissa_converts_mana_only_for_planeswalker_spells() {
    for definition in routes("Oath of Nissa", OATH_OF_NISSA) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let statics = statics(&definition);
        assert!(statics.iter().any(|s| s.contains("CastingSpellsMatching") && s.contains("Planeswalker")), "{statics:#?}");

        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 2);
        let walker = game.create_object_from_definition(
            &compile_to_runtime_definition(
                "Blue walker",
                "Mana cost: {U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 3\n+1: Draw a card.",
                false,
            )
            .unwrap(),
            A,
            Zone::Hand,
        );
        let sorcery = game.create_object_from_definition(
            &compile_to_runtime_definition("Blue sorcery", "Mana cost: {U}{U}\nType: Sorcery\nDraw a card.", false)
                .unwrap(),
            A,
            Zone::Hand,
        );
        assert!(casts(&game, walker) > 0, "green mana pays a planeswalker's {{U}}{{U}}");
        assert_eq!(casts(&game, sorcery), 0, "but not other spells");
    }
}

#[test]
fn quicksilver_elemental_converts_only_blue_mana_for_its_own_activations() {
    for definition in routes("Quicksilver Elemental", QUICKSILVER_ELEMENTAL) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let statics = statics(&definition);
        assert!(
            statics.iter().any(|s| s.contains("ActivationCostsOf") && s.contains("any_color_mana_symbol: Some(Blue)")),
            "{statics:#?}"
        );
    }
}
