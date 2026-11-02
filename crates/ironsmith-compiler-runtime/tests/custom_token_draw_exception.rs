//! Source draw triggers use captured own-step facts, not the current global step.
use ironsmith::ability::AbilityKind;
use ironsmith::events::CardsDrawnEvent;
use ironsmith::triggers::{TriggerContext, TriggerEvent};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

#[test]
fn thought_police_excludes_every_draw_in_drawers_own_draw_step() {
    let text = "Mana cost: {3}{U}\nType: Creature — Human Wizard\nPower/Toughness: 2/3\nWhenever an opponent draws a card except during their draw step, Create a white Aura enchantment token with \"Enchant creature\" and \"Sacrifice this token: Untap enchanted creature.\" attached to target creature.\nWhenever an enchanted creature you control deals combat damage to a player, draw a card.";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Thought Police", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(ability) => Some(ability), _ => None,
        }).unwrap();
        for (drawer, own_draw_step, prior, expected) in [(B, true, 0, false), (B, true, 3, false), (B, false, 0, true), (A, false, 0, false)] {
            let event = TriggerEvent::new(CardsDrawnEvent::new_with_step_context(
                drawer, vec![ObjectId::from_raw(50), ObjectId::from_raw(51)], false,
                own_draw_step, prior,
            ), Default::default());
            let context = TriggerContext::for_source(source, A, &game);
            assert_eq!(triggered.trigger.matches(&event, &context), expected);
            if expected { assert_eq!(triggered.trigger.trigger_count(&event), 2); }
        }
    }
}
