//! Per-player action limits: "each opponent can't block with more than one
//! creature this combat" (Mirri, Weatherlight Duelist; CR 509.1c) and "each
//! opponent can't venture into the dungeon more than once each turn"
//! (Keen-Eared Sentry; CR 701.49). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/per_player_action_limits.json.fixture"))
        .unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
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
fn mirri_limits_each_opponent_blockers_this_combat_and_caps_attackers_while_tapped() {
    for definition in definitions("Mirri, Weatherlight Duelist") {
        let debug = format!("{:?}", definition.abilities);
        assert!(
            debug.contains("BlockWithMoreThan { player: Opponent, maximum: 1 }"),
            "{debug}"
        );
        assert!(debug.contains("EndOfCombat"), "{debug}");
        assert!(debug.contains("MaxCreaturesCanAttackYouEachCombat(1)"), "{debug}");
        assert!(debug.contains("SourceIsTapped") || debug.contains("Tapped"), "{debug}");
    }
}

#[test]
fn mirri_attack_trigger_rejects_a_second_blocker_from_the_opponent() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState, declare_blockers};
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::{GameState, PlayerId, Zone};

    const A: PlayerId = PlayerId::from_index(0);
    const B: PlayerId = PlayerId::from_index(1);
    for definition in definitions("Mirri, Weatherlight Duelist") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mirri = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bear = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Bear",
            "Type: Creature — Bear\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        let first = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let second = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let combat = CombatState {
            attackers: vec![AttackerInfo { creature: mirri, target: AttackTarget::Player(B) }],
            ..Default::default()
        };
        assert!(
            declare_blockers(&game, &mut combat.clone(), vec![(first, mirri), (second, mirri)])
                .is_ok(),
            "two blockers are legal before the trigger resolves"
        );
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("attack trigger");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(mirri, A, &mut dm);
        for effect in triggered.effects.flattened_default_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.max_blocking_creatures_for_player(B), Some(1));
        assert_eq!(game.max_blocking_creatures_for_player(A), None, "only opponents");
        assert!(
            declare_blockers(&game, &mut combat.clone(), vec![(first, mirri), (second, mirri)])
                .is_err(),
            "Bob can't block with more than one creature this combat"
        );
        assert!(declare_blockers(&game, &mut combat.clone(), vec![(first, mirri)]).is_ok());
    }
}

#[test]
fn keen_eared_sentry_limits_each_opponent_to_one_venture_each_turn() {
    use ironsmith::{GameState, PlayerId, Zone};

    const A: PlayerId = PlayerId::from_index(0);
    const B: PlayerId = PlayerId::from_index(1);
    for definition in definitions("Keen-Eared Sentry") {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("VentureMoreThanOnceEachTurn(Opponent)"), "{debug}");
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(game.can_venture_into_dungeon(B), "no venture yet this turn");
        assert!(game.can_venture_into_dungeon(A), "the controller is not limited");
        assert_eq!(game.player_venture_count_this_turn(B), 0);
    }
}

#[test]
fn mirri_caps_attackers_against_you_only_while_tapped() {
    use ironsmith::combat_state::{AttackTarget, declare_attackers, new_combat};
    use ironsmith::{GameState, PlayerId, Zone};

    const A: PlayerId = PlayerId::from_index(0);
    const B: PlayerId = PlayerId::from_index(1);
    for definition in definitions("Mirri, Weatherlight Duelist") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mirri = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bear = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Bear",
            "Type: Creature — Bear\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        let first = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let second = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        game.remove_summoning_sickness(first);
        game.remove_summoning_sickness(second);
        game.turn.active_player = B;
        game.refresh_continuous_state().unwrap();
        let both = vec![(first, AttackTarget::Player(A)), (second, AttackTarget::Player(A))];
        assert!(
            declare_attackers(&mut game.clone(), &mut new_combat(), both.clone()).is_ok(),
            "untapped Mirri imposes no cap (CR 604.2)"
        );
        game.tap(mirri);
        game.refresh_continuous_state().unwrap();
        assert!(
            declare_attackers(&mut game.clone(), &mut new_combat(), both).is_err(),
            "no more than one creature can attack Alice while Mirri is tapped"
        );
        assert!(
            declare_attackers(
                &mut game.clone(),
                &mut new_combat(),
                vec![(first, AttackTarget::Player(A))]
            )
            .is_ok()
        );
    }
}
