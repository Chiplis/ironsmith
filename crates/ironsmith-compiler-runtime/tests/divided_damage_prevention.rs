//! "Prevent the next N damage that would be dealt this turn to any number of
//! targets, divided as you choose." The amount is divided as the spell is
//! cast (CR 601.2d) and each target gets a shield of its share (CR 615.7).
//! Source-authored, deliberately unrun.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_state::{Target, TargetDistribution};
use ironsmith::prevention::PreventionTarget;
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const EMBOLDEN: &str = "Mana cost: {2}{W}\nType: Instant\nPrevent the next 4 damage that would be dealt this turn to any number of targets, divided as you choose.\nFlashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)";
const REMEDY: &str = "Mana cost: {1}{W}\nType: Instant\nPrevent the next 5 damage that would be dealt this turn to any number of targets, divided as you choose.";

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
fn divided_prevention_compiles_as_one_announced_division() {
    for (name, text, amount) in [("Embolden", EMBOLDEN, "Fixed(4)"), ("Remedy", REMEDY, "Fixed(5)")] {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
            assert_eq!(effects.len(), 1, "{name}");
            let prevent = effects[0]
                .downcast_ref::<ironsmith::effects::PreventDamageEffect>()
                .unwrap_or_else(|| panic!("{name}: {effects:#?}"));
            assert!(prevent.divided, "{name}");
            assert!(format!("{:?}", prevent.amount).contains(amount), "{name}");
            assert_eq!(prevent.target.count().min, 0, "any number of targets");
            assert_eq!(prevent.target.count().max, None, "any number of targets");
            if name == "Embolden" {
                assert!(!definition.alternative_casts.is_empty(), "flashback retained");
            }
        }
    }
}

#[test]
fn each_target_gets_a_shield_of_its_announced_share() {
    for definition in routes("Embolden", EMBOLDEN) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let bear = CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let effect = definition.spell_effect.as_ref().unwrap().all_effects_owned().remove(0);
        let spec = effect
            .downcast_ref::<ironsmith::effects::PreventDamageEffect>()
            .unwrap()
            .target
            .clone();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut dm).with_target_distributions(vec![
            TargetDistribution {
                spec,
                range: 0..2,
                allocations: vec![(Target::Player(A), 3), (Target::Object(creature), 1)],
            },
        ]);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();
        let shields = game.effect_store.prevention_effects.shields();
        assert_eq!(shields.len(), 2);
        assert!(shields.iter().any(|shield| shield.protected == PreventionTarget::Player(A)
            && shield.amount_remaining == Some(3)));
        assert!(shields.iter().any(|shield| shield.protected == PreventionTarget::Permanent(creature)
            && shield.amount_remaining == Some(1)));
    }
}
