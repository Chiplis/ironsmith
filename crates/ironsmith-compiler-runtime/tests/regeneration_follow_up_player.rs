//! "Choose target opponent. Regenerate this creature. When it regenerates
//! this way, that player may draw a card." (Soldevi Sentry): the activation's
//! chosen player is fixed when the shield is created (`follow_up_player`) and
//! the reflexive trigger (CR 701.19, 603.12) names it as the iterated player
//! of a one-player loop. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effect::Effect;
use ironsmith::effects::{RegenerateEffect, ResolvedTarget};
use ironsmith::target::PlayerFilter;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/regeneration_follow_up_player.json.fixture"
    ))
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

fn soldevi() -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|row| row["name"] == "Soldevi Sentry").unwrap();
    definitions("Soldevi Sentry", row["text"].as_str().unwrap())
}

#[test]
fn soldevi_sentry_carries_the_chosen_opponent_into_the_shield() {
    for definition in soldevi() {
        let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
            panic!("regeneration ability");
        };
        let mut all = Vec::new();
        for effect in activated.effects.all_effects() {
            collect(effect, &mut all);
        }
        let regenerate = all
            .iter()
            .find_map(|effect| effect.downcast_ref::<RegenerateEffect>())
            .expect("regeneration shield");
        assert!(
            matches!(regenerate.follow_up_player, Some(PlayerFilter::Target(_))),
            "{:?}",
            regenerate.follow_up_player
        );
        let body = format!("{:?}", regenerate.follow_up_effects);
        assert!(body.contains("ReflexiveTriggerEffect"), "{body}");
        assert!(body.contains("IteratedPlayer"), "{body}");
        assert!(body.contains("Draw"), "{body}");
    }
}

struct AcceptAll;

impl DecisionMaker for AcceptAll {
    fn decide_boolean(&mut self, _game: &ironsmith::GameState, _ctx: &BooleanContext) -> bool {
        true
    }
}

#[test]
fn soldevi_sentry_lets_the_chosen_opponent_draw_when_it_regenerates() {
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::events::processing::{DestroyResult, process_destroy_full};
    use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{GameState, PlayerId, Zone};

    const A: PlayerId = PlayerId::from_index(0);
    const B: PlayerId = PlayerId::from_index(1);
    for definition in soldevi() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let filler = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Library card",
            "Type: Artifact",
            false,
        )
        .unwrap();
        for _ in 0..3 {
            game.create_object_from_definition(&filler, B, Zone::Library);
        }
        let sentry = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
            panic!("regeneration ability");
        };
        let mut dm = AcceptAll;
        {
            let mut ctx = EffectContext::new(sentry, A, &mut dm)
                .with_targets(vec![ResolvedTarget::Player(B)]);
            for effect in activated.effects.flattened_default_effects() {
                execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        let hand_before = game.player(B).unwrap().hand.len();
        assert_eq!(
            process_destroy_full(&mut game, sentry, None).unwrap(),
            DestroyResult::Replaced
        );
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "the reflexive trigger waits for the stack");
        assert_eq!(game.stack.last().unwrap().controller, A);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.player(B).unwrap().hand.len(),
            hand_before + 1,
            "the opponent chosen on activation draws"
        );
        assert_eq!(game.player(A).unwrap().hand.len(), 0, "not the shield's controller");
    }
}
