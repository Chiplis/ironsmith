//! Source-authored and deliberately unrun (cf8 p04): "Cloak a card from your
//! hand." chooses a card in hand and puts it onto the battlefield face down as
//! a 2/2 creature with ward {2} (CR 701.58a).
use ironsmith::cards::CardDefinition;
use ironsmith::ability::AbilityKind;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/cloak_from_hand.json.fixture")).unwrap();
    let row = &rows[0];
    assert_eq!(row["oracle_id"], "e8aa0d7d-e5f6-4d66-bdc1-315483e1b256");
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

#[test]
fn vannifar_cloaks_a_chosen_hand_card_face_down() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("ManifestObjectsEffect"), "{debug}");
        assert!(debug.contains("cloak: true"), "{debug}");
        assert!(debug.contains("Hand"), "{debug}");
        let modal = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
                _ => None,
            })
            .unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = game.create_object_from_definition(
            &compile_to_runtime_definition("Hand card", "Mana cost: {3}\nType: Artifact", false).unwrap(),
            A,
            Zone::Hand,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in modal.flattened_default_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert!(game.object(card).is_none_or(|object| object.zone != Zone::Hand));
        assert!(game.battlefield.iter().any(|id| *id != source && game.is_face_down(*id)));
    }
}
