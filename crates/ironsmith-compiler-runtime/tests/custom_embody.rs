//! Generic attached-copy source and recipient-type exceptions in layer 1.
//! Requires the isolated abilities_middle/embody production patch.
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::types::{CardType, Subtype, Supertype};
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const COPY: &str = "Mana cost: {1}{R}\nType: Legendary Enchantment — Aura\nPower/Toughness: 3/4\nEnchant creature\nEnchanted creature is a copy of this permanent except for its card types, supertypes, and subtypes.\nFlying";
fn routes(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact("Attached copy source", text, false)
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn body() -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Snow Goblin",
        "Type: Snow Artifact Creature — Goblin\nPower/Toughness: 1/2\nVigilance",
        false,
    )
    .unwrap()
}
#[test]
fn embody_copies_every_copiable_non_type_category_and_preserves_all_recipient_type_categories() {
    for definition in routes(COPY) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let host = game.create_object_from_definition(&body(), A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(
            game.attach_object_to_target(source, ironsmith::object::AttachmentTarget::Object(host))
        );
        game.add_counters(host, CounterType::PlusOnePlusOne, 2);
        let chars = game.current_characteristics(host).unwrap();
        assert_eq!(chars.power, Some(5));
        assert_eq!(chars.toughness, Some(6));
        assert!(chars.card_types.contains(&CardType::Creature));
        assert!(chars.card_types.contains(&CardType::Artifact));
        assert!(!chars.card_types.contains(&CardType::Enchantment));
        assert_eq!(chars.supertypes.as_slice(), &[Supertype::Snow]);
        assert_eq!(chars.subtypes.as_slice(), &[Subtype::Goblin]);
        assert_eq!(chars.name.as_ref(), definition.card.name);
        let values = game.calculated_copiable_values(host).unwrap();
        assert_eq!(values.power, Some(3));
        assert_eq!(values.toughness, Some(4));
        assert_eq!(
            values.card_types,
            vec![CardType::Artifact, CardType::Creature]
        );
        assert_eq!(values.supertypes, vec![Supertype::Snow]);
        assert_eq!(values.subtypes, vec![Subtype::Goblin]);
        let debug = format!("{:?}", chars.abilities);
        assert!(debug.contains("Flying"), "{debug}");
        assert!(!debug.contains("Vigilance"), "{debug}");
        assert_eq!(values.mana_cost, definition.card.mana_cost);
    }
}
#[test]
fn embody_type_exception_is_copiable_and_ongoing_attachment_changes_restore_previous_host() {
    for definition in routes(COPY) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let first = game.create_object_from_definition(&body(), A, Zone::Battlefield);
        let second_def = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Other host",
            "Type: Creature — Merfolk\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        let second = game.create_object_from_definition(&second_def, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(
            game.attach_object_to_target(
                source,
                ironsmith::object::AttachmentTarget::Object(first)
            )
        );
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(
                ChooseSpec::SpecificObject(first),
                1,
                PlayerFilter::You,
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        let token = *game
            .battlefield
            .iter()
            .find(|id| {
                matches!(
                    game.object(**id).unwrap().kind,
                    ironsmith::object::ObjectKind::Token
                )
            })
            .unwrap();
        let copied = game.current_characteristics(token).unwrap();
        assert!(copied.card_types.contains(&CardType::Artifact));
        assert!(copied.subtypes.contains(&Subtype::Goblin));
        assert_eq!(copied.power, Some(3));
        assert!(
            game.attach_object_to_target(
                source,
                ironsmith::object::AttachmentTarget::Object(second)
            )
        );
        assert_eq!(game.current_power(first), Some(1));
        let chars = game.current_characteristics(second).unwrap();
        assert_eq!(chars.power, Some(3));
        assert_eq!(chars.subtypes.as_slice(), &[Subtype::Merfolk]);
        assert_eq!(chars.supertypes.as_slice(), &[]);
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(source),
                Zone::Graveyard,
                false,
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        assert_eq!(game.current_power(second), Some(2));
        assert_eq!(
            game.current_power(token),
            Some(3),
            "created copy retains actual exceptions after original Aura leaves"
        );
    }
}
