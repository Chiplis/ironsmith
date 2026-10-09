//! Counted "Manifest the top N cards" (CR 701.40c: one at a time) and
//! "Open N Attractions" (each opening puts the top Attraction onto the
//! battlefield). Source-authored, deliberately unrun.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{EffectContext, RepeatEffectsEffect, execute_effect};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);

const ETHEREAL_AMBUSH: &str = "Mana cost: {3}{G}{U}\nType: Instant\nManifest the top two cards of your library. (To manifest a card, put it onto the battlefield face down as a 2/2 creature. Turn it face up any time for its mana cost if it's a creature card.)";
const STEP_RIGHT_UP: &str = "Mana cost: {3}{B}\nType: Sorcery\nOpen two Attractions. (Put the top two cards of your Attraction deck onto the battlefield.)";

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

fn effects(definition: &CardDefinition) -> Vec<Effect> {
    definition.spell_effect.as_ref().unwrap().all_effects_owned()
}

fn repeat_count(definition: &CardDefinition) -> Value {
    effects(definition)
        .iter()
        .find_map(|effect| effect.downcast_ref::<RepeatEffectsEffect>().map(|repeat| repeat.count.clone()))
        .expect("counted keyword action repeats its single action")
}

fn resolve(game: &mut GameState, definition: &CardDefinition) {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let mut dm = SelectFirstDecisionMaker;
    for effect in effects(definition) {
        execute_effect(game, &effect, &mut EffectContext::new(source, A, &mut dm)).unwrap();
    }
}

#[test]
fn ethereal_ambush_manifests_the_top_two_cards_one_at_a_time() {
    for definition in routes("Ethereal Ambush", ETHEREAL_AMBUSH) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert_eq!(repeat_count(&definition), Value::Fixed(2));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for index in 0..4 {
            let card = CardDefinitionBuilder::new(CardId::new(), &format!("Library {index}"))
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(5, 5))
                .build();
            game.create_object_from_definition(&card, A, Zone::Library);
        }
        let library_before = game.player(A).unwrap().library.len();
        resolve(&mut game, &definition);
        assert_eq!(game.player(A).unwrap().library.len(), library_before - 2);
        let face_down = game
            .battlefield
            .iter()
            .filter(|&&id| game.is_face_down(id))
            .count();
        assert_eq!(face_down, 2, "two face-down 2/2 manifests");
    }
}

#[test]
fn step_right_up_opens_two_attractions() {
    for definition in routes("Step Right Up", STEP_RIGHT_UP) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert_eq!(repeat_count(&definition), Value::Fixed(2));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let attraction = CardDefinitionBuilder::new(CardId::new(), "Attraction fixture")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![ironsmith::types::Subtype::Attraction])
            .attraction_lights(vec![6])
            .build();
        game.enable_attractions(vec![(
            A,
            ironsmith::game_state::AttractionDeckFormat::Limited,
            vec![attraction.clone(), attraction.clone(), attraction],
        )])
        .unwrap();
        resolve(&mut game, &definition);
        let opened = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|object| object.subtypes.contains(&ironsmith::types::Subtype::Attraction))
            })
            .count();
        assert_eq!(opened, 2);
    }
}

#[test]
fn singular_forms_keep_their_unrepeated_actions() {
    for (name, text) in [
        ("Single manifest", "Mana cost: {1}{G}\nType: Sorcery\nManifest the top card of your library."),
        ("Single attraction", "Mana cost: {1}{B}\nType: Sorcery\nOpen an Attraction."),
    ] {
        for definition in routes(name, text) {
            assert!(!effects(&definition)
                .iter()
                .any(|effect| effect.downcast_ref::<RepeatEffectsEffect>().is_some()));
        }
    }
}
