//! Source-composed resource actions retain outer conditions across quoted rules.
use ironsmith::ability::AbilityKind;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::types::Subtype;
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions() -> [CardDefinition; 2] {
    let text = "Mana cost: {4}{U}\nType: Creature — Crab Pirate\nPower/Toughness: 3/4\nFlying\nWhen Sky-Torrent Navigator enters, Create a tapped colorless Omen artifact token with \"{T}, Discard a card, Sacrifice this artifact: Draw a card.\". If you control two or more Pirates, Create a tapped colorless Omen artifact token with \"{T}, Discard a card, Sacrifice this artifact: Draw a card.\".";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Sky-Torrent Navigator", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}

#[test]
fn source_repeated_omen_creation_keeps_condition_and_controller_scope() {
    for definition in definitions() {
        for (allied, enemy, expected) in [(0, 0, 1), (1, 0, 2), (0, 2, 1)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let pirate = ironsmith_compiler_runtime::compile_to_runtime_definition("Pirate", "Type: Creature — Pirate\nPower/Toughness: 1/1", false).unwrap();
            for _ in 0..allied { game.create_object_from_definition(&pirate, A, Zone::Battlefield); }
            for _ in 0..enemy { game.create_object_from_definition(&pirate, B, Zone::Battlefield); }
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let effects = definition.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(ability) => Some(ability.effects.all_effects()), _ => None,
            }).unwrap();
            let mut context = EffectContext::new_default(source, A);
            for effect in effects { execute_effect(&mut game, effect, &mut context).unwrap(); }
            let omens: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().subtypes.contains(&Subtype::ArtifactOmen)).collect();
            assert_eq!(omens.len(), expected);
            for omen in omens {
                assert!(game.is_tapped(omen));
                assert_eq!(game.controller_of(game.object(omen).unwrap()), A);
                assert!(game.object(omen).unwrap().abilities.iter().any(|ability| matches!(ability.kind, AbilityKind::Activated(_))));
            }
        }
    }
}
