//! "Enchanted creature can attack as though it had haste." (CR 302.6
//! attack exemption granted to the attached creature). Source-authored,
//! deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::object::AttachmentTarget;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);

const INSTILL_ENERGY: &str = "Mana cost: {G}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can attack as though it had haste.\n{0}: Untap enchanted creature. Activate only during your turn and only once each turn.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

#[test]
fn instill_energy_lets_only_the_enchanted_creature_attack_while_summoning_sick() {
    for definition in routes("Instill Energy", INSTILL_ENERGY) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert!(definition
            .abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Activated(_))));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let creature_card = CardDefinitionBuilder::new(CardId::new(), "Fresh creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let enchanted = game.create_object_from_definition(&creature_card, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&creature_card, A, Zone::Battlefield);
        game.set_summoning_sick(enchanted);
        game.set_summoning_sick(other);
        assert!(game.is_summoning_sick(enchanted));
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(enchanted)));
        game.update_cant_effects();
        let can_attack = |game: &GameState, id| {
            ironsmith::rules::combat::can_attack(game.object(id).unwrap(), game)
        };
        assert!(can_attack(&game, enchanted), "attack exemption on the enchanted creature");
        assert!(!can_attack(&game, other), "other summoning-sick creatures still can't attack");
        let view = game.calculated_characteristics(enchanted).unwrap();
        assert!(
            !view.static_abilities.iter().any(|ability| ability.id() == StaticAbilityId::Haste),
            "the permission is not haste itself"
        );
    }
}
