//! "has hexproof as long as it hasn't dealt damage yet": lifetime damage
//! history of the permanent since it entered. Source-authored, deliberately unrun.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::DamageTarget;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/source_damage_history.json.fixture")).unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
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
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn hexproof_until_first_damage_on_both_routes() {
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("Not(SourceHasDealtDamageSinceEntered)"), "{name}: {debug}");
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let guardian = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let has_hexproof = |game: &GameState| {
                game.object_has_static_ability_id(guardian, StaticAbilityId::Hexproof)
            };
            assert!(has_hexproof(&game), "{name}: no damage dealt yet");
            let victim = game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Victim")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(1, 20))
                    .build(),
                B,
                Zone::Battlefield,
            );
            process_damage_assignments_with_event_with_source_snapshot_opts(
                &mut game,
                guardian,
                DamageTarget::Object(victim),
                1,
                false,
                false,
                EventCause::effect(),
                None,
            )
            .unwrap();
            assert!(game.has_dealt_damage_since_entered(guardian));
            assert!(!has_hexproof(&game), "{name}: hexproof ends once it has dealt damage");
        }
    }
}
