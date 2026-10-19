//! Blocker-count evasion (CR 509.1b/c) on a labeled self line and granted to
//! an authored set of creatures. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/blocker_count_restrictions.json.fixture"))
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

#[test]
fn blocker_count_restrictions_keep_their_counts_and_recipients_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 3);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let debug = format!("{definition:?}");
            match name {
                "Hexmark Destroyer" => {
                    assert!(debug.contains("CantBeBlockedExceptByNOrMore(6)"), "{name}");
                    assert!(!debug.contains("GrantStaticAbility"), "{name}: the source itself");
                }
                "Sonorous Howlbonder" => {
                    assert!(debug.contains("CantBeBlockedExceptByNOrMore(3)"), "{name}");
                    assert!(debug.contains("Menace"), "{name}: granted only to menace creatures");
                }
                "Rocksteady, Crash Courser" => {
                    assert!(debug.matches("CantBeBlockedByMoreThan(1)").count() >= 2, "{name}");
                    assert!(debug.contains("Boar"), "{name}: Boars you control");
                }
                other => panic!("unexpected {other}"),
            }
        }
    }
}

#[test]
fn resilient_roadrunner_protection_and_resolved_evasion_are_independent() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::{GameState, PlayerId, Zone};
    let a = PlayerId(0);
    let b = PlayerId(1);
    let text = "Mana cost: {1}{R}\nType: Creature — Bird\nPower/Toughness: 2/2\nHaste, protection from Coyotes\n{3}: This creature can't be blocked this turn except by creatures with haste.";
    for definition in definitions("Resilient Roadrunner", text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, a, Zone::Battlefield);
        assert!(game.object_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Haste));
        let mut blockers = Vec::new();
        for (subtype, hasty) in [("Bear", false), ("Bear", true), ("Coyote", false), ("Coyote", true)] {
            let text = format!("Type: Creature — {subtype}\nPower/Toughness: 2/2{}", if hasty { "\nHaste" } else { "" });
            let blocker = ironsmith_compiler_runtime::compile_to_runtime_definition("Blocker", text, false).unwrap();
            blockers.push(game.create_object_from_definition(&blocker, b, Zone::Battlefield));
        }
        game.refresh_continuous_state().unwrap();
        // Full combat legality includes protection; the cant-effect cache alone
        // only answers restrictions installed by continuous effects.
        let can_block = |game: &GameState, blocker| {
            ironsmith::rules::combat::can_block(
                game.object(source).unwrap(), game.object(blocker).unwrap(), game,
            )
        };
        for (index, blocker) in blockers.iter().copied().enumerate() {
            assert_eq!(ironsmith::targeting::has_protection_from_source(&game, source, blocker), index >= 2);
            assert_eq!(can_block(&game, blocker), index < 2);
        }
        // Resolve the compiled activation's program directly: this test isolates
        // its blocking semantics; payment/priority are covered by Gingerbrute.
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        }).unwrap();
        let mut context = EffectContext::new_default(source, a);
        for effect in activated.effects.iter() {
            execute_effect(&mut game, effect, &mut context).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        for (index, blocker) in blockers.iter().copied().enumerate() {
            // Haste does not bypass protection from Coyotes.
            assert_eq!(can_block(&game, blocker), index == 1);
        }
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        for (index, blocker) in blockers.iter().copied().enumerate() {
            assert_eq!(can_block(&game, blocker), index < 2);
        }
    }
}

#[test]
fn coordinated_source_and_target_evasion_allows_either_keyword_only_this_turn() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::{GameState, PlayerId, Zone};
    let a = PlayerId(0);
    let b = PlayerId(1);
    let text = "Type: Legendary Creature — Human Ninja\nPower/Toughness: 3/3\n{1}{G/U}: This creature and target Frog you control can't be blocked this turn except by creatures with flying or reach.";
    for definition in definitions("Coordinated evasion", text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, a, Zone::Battlefield);
        let frog_definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Frog", "Type: Creature — Frog\nPower/Toughness: 3/3", false,
        ).unwrap();
        let target = game.create_object_from_definition(&frog_definition, a, Zone::Battlefield);
        let other = game.create_object_from_definition(&frog_definition, a, Zone::Battlefield);
        let mut blockers = Vec::new();
        for keywords in ["", "\nFlying", "\nReach", "\nFlying, reach"] {
            let blocker = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Blocker", format!("Type: Creature — Bear\nPower/Toughness: 2/2{keywords}"), false,
            ).unwrap();
            blockers.push(game.create_object_from_definition(&blocker, b, Zone::Battlefield));
        }
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        }).unwrap();
        let mut context = EffectContext::new_default(source, a)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        for effect in &activated.effects {
            execute_effect(&mut game, effect, &mut context).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        for (index, blocker) in blockers.iter().copied().enumerate() {
            for attacker in [source, target] {
                assert_eq!(game.can_block_attacker(blocker, attacker), index > 0);
            }
            assert!(game.can_block_attacker(blocker, other));
        }
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        for blocker in blockers {
            assert!(game.can_block_attacker(blocker, source));
            assert!(game.can_block_attacker(blocker, target));
        }
    }
}
