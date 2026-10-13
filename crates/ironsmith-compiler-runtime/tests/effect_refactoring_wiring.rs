//! Representative public payloads still compile and materialize after runtime
//! owner migration. Gameplay contracts are tested separately at their owners.
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[test]
fn refactored_effect_families_compile_real_cards_without_loss_and_round_trip() {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/effect_refactoring_wiring.json.fixture"
    ))
    .unwrap();
    for card in cards {
        let name = card["name"].as_str().unwrap();
        let mut lines = vec![
            format!("Mana cost: {}", card["mana_cost"].as_str().unwrap()),
            format!("Type: {}", card["type_line"].as_str().unwrap()),
        ];
        if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str())
        {
            lines.push(format!("Power/Toughness: {power}/{toughness}"));
        }
        if let Some(loyalty) = card["loyalty"].as_str() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        lines.push(card["oracle_text"].as_str().unwrap().to_string());
        let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler_runtime::compile_to_artifact(name, &lines.join("\n"), false)
        });
        let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        restored.validate().unwrap();
        let materialized =
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
                .unwrap();
        for definition in [direct, materialized] {
            assert!(
                !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
                "{name}"
            );
        }
    }
}

#[test]
fn composed_choice_and_stat_adapters_keep_real_card_compilation_and_artifact_wiring() {
    for (name, text) in [
        (
            "Giant Growth",
            "Mana cost: {G}\nType: Instant\nTarget creature gets +3/+3 until end of turn.",
        ),
        (
            "Pulling Teeth",
            "Mana cost: {1}{B}\nType: Sorcery\nClash with an opponent. If you win, target player discards two cards. Otherwise, that player discards a card.",
        ),
    ] {
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
        });
        let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        restored.validate().unwrap();
        let materialized =
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
                .unwrap();
        for definition in [direct, materialized] {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            if name == "Giant Growth" {
                use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let player = ironsmith::PlayerId(0);
                let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Creature")
                    .card_types(vec![ironsmith::CardType::Creature])
                    .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
                    .build();
                let creature =
                    game.create_object_from_card(&card, player, ironsmith::Zone::Battlefield);
                let source = game.new_object_id();
                let mut ctx = EffectContext::new_default(source, player)
                    .with_targets(vec![ResolvedTarget::Object(creature)]);
                for effect in definition.spell_effect.as_ref().unwrap().iter() {
                    execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                assert_eq!(game.calculated_power(creature), Some(5));
                assert_eq!(game.calculated_toughness(creature), Some(5));
            }
        }
    }
}

#[test]
fn remaining_executor_adapters_keep_real_card_artifact_wiring() {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/effect_remaining_wiring.json.fixture"
    )).unwrap();
    let mut failures = Vec::new();
    for card in cards {
        let name = card["name"].as_str().unwrap();
        let mut lines = vec![format!("Mana cost: {}", card["mana_cost"].as_str().unwrap()),
            format!("Type: {}", card["type_line"].as_str().unwrap())];
        if let (Some(power),Some(toughness)) = (card["power"].as_str(),card["toughness"].as_str()) {
            lines.push(format!("Power/Toughness: {power}/{toughness}"));
        }
        lines.push(card["oracle_text"].as_str().unwrap().to_string());
        let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
            ironsmith_compiler_runtime::compile_to_artifact(name, &lines.join("\n"), false));
        let (artifact, direct) = match compiled {
            Ok(result) if !loss.is_lossy() => result,
            other => { failures.push(format!("{name}: {other:?}; {}", loss.reasons_text())); continue; }
        };
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        restored.validate().unwrap();
        let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
        for definition in [direct,materialized] {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition), "{name}");
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
