use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}
#[test]
fn nonhand_origin_cost_qualifier_is_strict_and_changes_only_eligible_cast_prices() {
    let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
    for reducer in definitions("Origin reducer", "Type: Enchantment\nSpells you cast from anywhere other than your hand cost {1} less to cast.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&reducer, a, Zone::Battlefield);
        let price = ManaCost::from_symbols(vec![ManaSymbol::Generic(2), ManaSymbol::Green]);
        let spell = CardDefinitionBuilder::new(CardId::new(), "Origin cost probe")
            .card_types(vec![CardType::Sorcery]).mana_cost(price.clone()).build();
        for (player, zone, expected) in [(a, Zone::Hand, 3), (a, Zone::Graveyard, 2), (a, Zone::Exile, 2), (b, Zone::Exile, 3)] {
            let id = game.create_object_from_definition(&spell, player, zone);
            let method = if zone == Zone::Hand { CastingMethod::Normal }
                else { CastingMethod::PlayFrom { source, zone, use_alternative: None } };
            let actual = ironsmith::decision::calculate_effective_mana_cost_for_casting_method(
                &game, player, game.object(id).unwrap(), &price, &method,
            );
            assert_eq!(actual.mana_value(), expected, "{player:?} from {zone:?}");
            assert_eq!(actual.to_oracle().contains("{G}"), true, "only generic mana is reduced");
        }
    }
}
#[test]
fn exact_weaver_source_compiles_strictly_with_entry_permission_and_origin_reduction() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-special-actions.json")).unwrap();
    let row = fixture["mechanics"].as_array().unwrap().iter().find(|row| row["id"] == "msem-326").unwrap();
    for card in row["representative_cards"].as_array().unwrap() {
        for definition in definitions(card["name"].as_str().unwrap(), card["executable_text"].as_str().unwrap()) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("DescribedFaceDownCastPermission"), "{debug}");
            assert!(debug.contains("CostReduction"), "{debug}");
        }
    }
}
