//! "deals N damage to <one recipient> for each <count>" scales the amount,
//! never the number of recipients. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{DealDamageEffect, EffectContext, execute_effect};
use ironsmith::{CardId, CardType, GameState, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/damage_for_each_multiplier.json.fixture"))
        .unwrap()
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

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

fn triggered_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    for ability in &definition.abilities {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() {
                collect(effect, &mut all);
            }
        }
    }
    all
}

#[test]
fn the_count_scales_the_amount_for_a_single_recipient_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let all = triggered_effects(&definition);
            let damage: Vec<_> = all
                .iter()
                .filter_map(|effect| effect.downcast_ref::<DealDamageEffect>())
                .collect();
            assert_eq!(damage.len(), 1, "{name}");
            let target = format!("{:?}", damage[0].target);
            assert!(!target.contains("WithCountValue"), "{name}: one recipient, {target}");
            match (name, damage[0].amount.unhinted()) {
                ("Black Market Tycoon", Value::Scaled(count, 2)) => {
                    let count = format!("{count:?}");
                    assert!(count.contains("Treasure"), "{name}: {count}");
                }
                ("Lotleth Giant", Value::Count(filter)) => {
                    assert_eq!(filter.zone, Some(Zone::Graveyard), "{name}");
                    assert!(filter.card_types.contains(&CardType::Creature), "{name}");
                }
                (_, other) => panic!("{name}: unexpected amount {other:?}"),
            }
        }
    }
}

#[test]
fn black_market_tycoon_deals_two_per_treasure_to_its_controller() {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == "Black Market Tycoon")
        .unwrap();
    for definition in definitions("Black Market Tycoon", row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let treasure = CardBuilder::new(CardId::new(), "Treasure")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Treasure])
            .build();
        for _ in 0..3 {
            game.create_object_from_card(&treasure, A, Zone::Battlefield);
        }
        let other = CardBuilder::new(CardId::new(), "Bystander")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&other, A, Zone::Battlefield);
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .unwrap();
        let mut dm = SelectFirstDecisionMaker;
        for effect in triggered.effects.flattened_default_effects() {
            execute_effect(&mut game, effect, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        }
        assert_eq!(game.player(A).unwrap().life, 14, "2 damage for each of three Treasures");
    }
}
