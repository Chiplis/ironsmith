//! Authored source proposals; no campaign tests have been executed.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::effects::{EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/target_event_participants.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn object(game: &mut GameState, player: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    yes: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.yes
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        context
            .requirements
            .iter()
            .flat_map(|requirement| {
                let selected: Vec<_> = self
                    .targets
                    .iter()
                    .copied()
                    .filter(|target| requirement.legal_targets.contains(target))
                    .take(requirement.max_targets.unwrap_or(usize::MAX))
                    .collect();
                if selected.len() >= requirement.min_targets {
                    selected
                } else {
                    SelectFirstDecisionMaker.decide_targets(
                        game,
                        &TargetsContext::new(
                            context.player,
                            context.source,
                            "fallback",
                            vec![requirement.clone()],
                        ),
                    )
                }
            })
            .collect()
    }
}
fn action(game: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending without decision: {progress:?}");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn cast(game: &mut GameState, player: PlayerId, spell: ObjectId, dm: &mut Choices) -> ObjectId {
    let stable = game.object(spell).unwrap().stable_id;
    action(
        game,
        player,
        LegalAction::CastSpell {
            spell_id: spell,
            from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        },
        dm,
    );
    game.find_object_by_stable_id(stable).unwrap()
}
fn pending(game: &mut GameState, dm: &mut Choices) {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
    ironsmith::game_loop::check_and_apply_sbas_with(game, &mut TriggerQueue::new(), dm).unwrap();
    pending(game, dm);
}
fn activation_index(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}
#[test]
fn eight_complete_proposals_retain_exact_oracle_and_artifact_round_trip() {
    assert_eq!(fixtures().len(), 8);
    for row in fixtures()
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
    {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
}
#[test]
fn player_target_tax_distinguishes_subject_and_actor_and_counters_the_exact_spell() {
    for definition in definitions("Amulet of Safekeeping") {
        for (caster, target, expected) in [(B, A, 2), (A, A, 1), (B, B, 1)] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let spell = object(
                &mut game,
                caster,
                Zone::Hand,
                "Player benefit",
                "Mana cost: {0}\nType: Instant\nTarget player gains 1 life.",
            );
            let mut dm = Choices {
                targets: vec![Target::Player(target)],
                yes: false,
            };
            let spell = cast(&mut game, caster, spell, &mut dm);
            assert_eq!(game.stack.len(), expected);
            resolve(&mut game, &mut dm);
            if expected == 2 {
                assert!(!game.stack.iter().any(|entry| entry.object_id == spell));
                assert_eq!(game.player(target).unwrap().life, 20);
            } else {
                assert_eq!(game.player(target).unwrap().life, 21);
            }
        }
    }
}
#[test]
fn copied_player_target_pays_separately_and_token_penalty_keeps_toughness() {
    for definition in definitions("Amulet of Safekeeping") {
        let mut game = game();
        let amulet = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spell = object(
            &mut game,
            B,
            Zone::Hand,
            "Player benefit",
            "Mana cost: {0}\nType: Instant\nTarget player gains 1 life.",
        );
        game.player_mut(B)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Colorless, 1);
        let mut dm = Choices {
            targets: vec![Target::Player(A)],
            yes: true,
        };
        let spell = cast(&mut game, B, spell, &mut dm);
        assert_eq!(game.stack.len(), 2);
        resolve(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        let mut ctx = EffectContext::new(amulet, B, &mut dm);
        let outcome =
            ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(spell))
                .execute(&mut game, &mut ctx)
                .unwrap();
        let ironsmith::effect::OutcomeValue::Objects(copies) = outcome.value else {
            panic!("copy");
        };
        dm.yes = false;
        pending(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 3);
        resolve(&mut game, &mut dm);
        assert!(game.stack.iter().any(|entry| entry.object_id == spell));
        assert!(!game.stack.iter().any(|entry| entry.object_id == copies[0]));
        let maker = object(
            &mut game,
            A,
            Zone::Hand,
            "Token maker",
            "Mana cost: {0}\nType: Instant\nCreate a 2/2 white Soldier creature token.",
        );
        cast(&mut game, A, maker, &mut dm);
        resolve(&mut game, &mut dm);
        let token = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|o| o.kind == ironsmith::object::ObjectKind::Token)
            })
            .unwrap();
        assert_eq!(game.calculated_power(token), Some(1));
        assert_eq!(game.calculated_toughness(token), Some(2));
    }
}
#[test]
fn separate_activations_keep_their_identity_after_physical_source_control_change_and_departure() {
    for definition in definitions("Amulet of Safekeeping") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ability = compile_to_runtime_definition(
            "Ability source",
            "Type: Artifact\n{0}: Target player gains 1 life.",
            false,
        )
        .unwrap();
        let source = game.create_object_from_definition(&ability, B, Zone::Battlefield);
        let mut dm = Choices {
            targets: vec![Target::Player(A)],
            yes: false,
        };
        action(
            &mut game,
            B,
            LegalAction::ActivateAbility {
                source,
                ability_index: activation_index(&ability),
            },
            &mut dm,
        );
        let first = game
            .stack
            .iter()
            .find(|entry| entry.object_id == source)
            .unwrap()
            .target_id();
        action(
            &mut game,
            B,
            LegalAction::ActivateAbility {
                source,
                ability_index: activation_index(&ability),
            },
            &mut dm,
        );
        let second = game
            .stack
            .iter()
            .rev()
            .find(|entry| entry.object_id == source)
            .unwrap()
            .target_id();
        assert_ne!(first, second);
        assert_eq!(game.stack.len(), 4);
        game.set_current_controller(source, A).unwrap();
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        resolve(&mut game, &mut dm);
        assert!(game.stack.iter().any(|entry| entry.target_id() == first));
        assert!(!game.stack.iter().any(|entry| entry.target_id() == second));
        resolve(&mut game, &mut dm);
        assert!(game.stack_is_empty());
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}
#[test]
fn dormant_entry_untap_step_and_player_spell_trigger_keep_all_clauses() {
    for definition in definitions("Dormant Gomazoa") {
        let mut game = game();
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Blue, 2);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Colorless, 1);
        let mut dm = Choices {
            yes: true,
            ..Default::default()
        };
        let stable = game.object(hand).unwrap().stable_id;
        cast(&mut game, A, hand, &mut dm);
        resolve(&mut game, &mut dm);
        let dormant = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.is_tapped(dormant));
        assert!(game.object_has_static_ability_id(
            dormant,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        ironsmith::turn::execute_untap_step(&mut game);
        assert!(game.is_tapped(dormant));
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let spell = object(
            &mut game,
            B,
            Zone::Hand,
            "Player benefit",
            "Mana cost: {0}\nType: Instant\nTarget player gains 1 life.",
        );
        dm.targets = vec![Target::Player(A)];
        cast(&mut game, B, spell, &mut dm);
        assert_eq!(game.stack.len(), 2);
        resolve(&mut game, &mut dm);
        assert!(!game.is_tapped(dormant));
        resolve(&mut game, &mut dm);
        game.tap(dormant);
        let ability = compile_to_runtime_definition(
            "Player-target ability",
            "Type: Artifact\n{0}: Target player gains 1 life.",
            false,
        )
        .unwrap();
        let source = game.create_object_from_definition(&ability, B, Zone::Battlefield);
        action(
            &mut game,
            B,
            LegalAction::ActivateAbility {
                source,
                ability_index: activation_index(&ability),
            },
            &mut dm,
        );
        assert_eq!(game.stack.len(), 1, "an ability is not a spell");
        resolve(&mut game, &mut dm);
        assert!(game.is_tapped(dormant));
        let spell = object(
            &mut game,
            B,
            Zone::Hand,
            "Other player benefit",
            "Mana cost: {0}\nType: Instant\nTarget player gains 1 life.",
        );
        dm.targets = vec![Target::Player(B)];
        cast(&mut game, B, spell, &mut dm);
        assert_eq!(
            game.stack.len(),
            1,
            "the opponent being targeted does not untap your Gomazoa"
        );
    }
}
#[test]
fn four_heroic_aliases_trigger_on_own_cast_and_keep_each_complete_body() {
    for name in [
        "Anthousa, Setessan Hero",
        "Brigone, Soldier of Meletis",
        "Cleon, Merry Champion",
        "Rosnakht, Heir of Rohgahh",
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let lands: Vec<_> = (0..3)
                .map(|_| {
                    object(
                        &mut game,
                        A,
                        Zone::Battlefield,
                        "Animation land",
                        "Type: Land",
                    )
                })
                .collect();
            for _ in 0..3 {
                object(&mut game, A, Zone::Library, "Available land", "Type: Land");
            }
            let spell = object(
                &mut game,
                A,
                Zone::Hand,
                "Heroic test spell",
                "Mana cost: {0}\nType: Instant\nTarget creature gets +0/+1 until end of turn.",
            );
            let mut dm = Choices {
                targets: std::iter::once(Target::Object(source))
                    .chain(lands.iter().copied().map(Target::Object))
                    .collect(),
                yes: true,
            };
            cast(&mut game, A, spell, &mut dm);
            assert_eq!(game.stack.len(), 2, "{name}");
            resolve(&mut game, &mut dm);
            match name {
                "Anthousa, Setessan Hero" => {
                    for land in &lands {
                        assert_eq!(game.calculated_power(*land), Some(2));
                        assert_eq!(game.calculated_toughness(*land), Some(2));
                        assert!(game.object_has_card_type(*land, ironsmith::CardType::Land));
                        assert!(game.object_has_card_type(*land, ironsmith::CardType::Creature));
                        assert!(game.current_has_subtype(*land, ironsmith::Subtype::Warrior));
                    }
                }
                "Brigone, Soldier of Meletis" => {
                    assert_eq!(
                        game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne),
                        1
                    );
                    assert!(game.object_has_static_ability_id(
                        source,
                        ironsmith::static_abilities::StaticAbilityId::Vigilance
                    ));
                }
                "Cleon, Merry Champion" => {
                    assert_eq!(game.exile.len(), 1);
                    assert!(game.object_has_static_ability_id(
                        source,
                        ironsmith::static_abilities::StaticAbilityId::DoubleStrike
                    ));
                }
                _ => {
                    let tokens: Vec<_> = game
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|id| {
                            game.object(*id)
                                .is_some_and(|o| o.name == "Kobolds of Kher Keep")
                        })
                        .collect();
                    assert_eq!(tokens.len(), 1);
                    assert_eq!(game.calculated_power(tokens[0]), Some(0));
                    assert_eq!(game.calculated_toughness(tokens[0]), Some(1));
                }
            }
            resolve(&mut game, &mut dm);
            if name == "Brigone, Soldier of Meletis" {
                game.remove_summoning_sickness(source);
                action(
                    &mut game,
                    A,
                    LegalAction::ActivateAbility {
                        source,
                        ability_index: activation_index(&definition),
                    },
                    &mut dm,
                );
                assert_eq!(
                    game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne),
                    0
                );
                assert!(game.is_tapped(source));
                resolve(&mut game, &mut dm);
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
            }
            if name == "Cleon, Merry Champion" {
                game.turn.priority_player = Some(A);
                assert!(
                    ironsmith::decision::compute_actions_for_source(&game, A, Some(game.exile[0]))
                        .unwrap()
                        .iter()
                        .any(|action| matches!(action, LegalAction::PlayLand { .. }))
                );
            }
            if name == "Rosnakht, Heir of Rohgahh" {
                use ironsmith::combat_state::{AttackTarget, CombatState};
                let ally = object(
                    &mut game,
                    A,
                    Zone::Battlefield,
                    "Attacking ally",
                    "Type: Creature\nPower/Toughness: 2/2",
                );
                game.remove_summoning_sickness(source);
                game.remove_summoning_sickness(ally);
                game.turn.phase = ironsmith::Phase::Combat;
                game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
                let mut combat = CombatState::default();
                let mut queue = TriggerQueue::new();
                ironsmith::game_loop::apply_attacker_declarations(
                    &mut game,
                    &mut combat,
                    &mut queue,
                    &[
                        ironsmith::decision::AttackerDeclaration {
                            creature: source,
                            target: AttackTarget::Player(B),
                        },
                        ironsmith::decision::AttackerDeclaration {
                            creature: ally,
                            target: AttackTarget::Player(B),
                        },
                    ],
                )
                .unwrap();
                game.combat = Some(combat);
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                assert_eq!(game.stack.len(), 1);
                resolve(&mut game, &mut dm);
                assert_eq!(game.calculated_power(ally), Some(3));
                assert_eq!(game.calculated_power(source), Some(0));
            }
            ironsmith::turn::execute_cleanup_step(&mut game);
            game.refresh_continuous_state().unwrap();
            if name == "Anthousa, Setessan Hero" {
                for land in lands {
                    assert!(!game.object_has_card_type(land, ironsmith::CardType::Creature));
                }
            }
        }
    }
}

// Source-closed, unrun regression: a granted target trigger exists when the
// target is announced, before sacrificing its grantor to pay the spell's cost.
#[test]
fn kira_grant_survives_sacrificing_kira_to_pay_for_the_already_targeted_spell() {
    for definition in definitions("Kira, Great Glass-Spinner") {
        let mut game = game();
        let kira = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = object(
            &mut game,
            A,
            Zone::Battlefield,
            "Target creature",
            "Type: Creature\nPower/Toughness: 2/2",
        );
        let spell = object(
            &mut game,
            A,
            Zone::Hand,
            "Cost-boundary spell",
            "Mana cost: {0}\nType: Instant\nAs an additional cost to cast this spell, sacrifice an enchantment.\nTarget creature gets +1/+1 until end of turn.",
        );
        // Make Kira the only enchantment so the real sacrifice cost chooses
        // this source, while its grant still applies at target selection.
        game.object_mut(kira)
            .unwrap()
            .card_types
            .push(ironsmith::CardType::Enchantment);
        let mut dm = Choices {
            targets: vec![Target::Object(target)],
            yes: true,
        };
        let spell = cast(&mut game, A, spell, &mut dm);
        assert!(game.object(kira).is_none());
        assert_eq!(
            game.stack.len(),
            2,
            "pre-cost granted observer must survive successful payment"
        );
        resolve(&mut game, &mut dm);
        assert!(!game.stack.iter().any(|entry| entry.object_id == spell));
        assert_eq!(game.calculated_power(target), Some(2));
    }
}

#[test]
fn skophos_full_oracle_keeps_source_qualification_and_complete_body() {
    for definition in definitions("Skophos Maze-Warden") {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
    }
}

#[test]
fn skophos_real_land_activation_fights_the_targeted_creature_and_keeps_paid_pump() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    for definition in definitions("Skophos Maze-Warden") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let land = compile_to_runtime_definition("Labyrinth of Skophos", "Type: Land\n{T}: Add {C}.\n{4}, {T}: Remove target attacking or blocking creature from combat.", false).unwrap();
        let land_id = game.create_object_from_definition(&land, A, Zone::Battlefield);
        let target = object(
            &mut game,
            B,
            Zone::Battlefield,
            "Attacker",
            "Type: Creature\nPower/Toughness: 2/3",
        );
        let spare = object(
            &mut game,
            B,
            Zone::Battlefield,
            "Untargeted creature",
            "Type: Creature\nPower/Toughness: 2/3",
        );
        let mut dm = Choices {
            targets: vec![Target::Object(target)],
            yes: true,
        };
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Colorless, 5);
        action(
            &mut game,
            A,
            LegalAction::ActivateAbility {
                source,
                ability_index: activation_index(&definition),
            },
            &mut dm,
        );
        resolve(&mut game, &mut dm);
        assert_eq!(game.calculated_power(source), Some(4));
        assert_eq!(game.calculated_toughness(source), Some(3));
        game.turn.active_player = B;
        game.turn.phase = ironsmith::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        game.remove_summoning_sickness(target);
        let mut combat = CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[ironsmith::decision::AttackerDeclaration {
                creature: target,
                target: AttackTarget::Player(A),
            }],
        )
        .unwrap();
        game.combat = Some(combat);
        let index = land
            .abilities
            .iter()
            .rposition(|ability| {
                matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
            })
            .unwrap();
        action(
            &mut game,
            A,
            LegalAction::ActivateAbility {
                source: land_id,
                ability_index: index,
            },
            &mut dm,
        );
        assert!(game.is_tapped(land_id));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.stack.len(), 2);
        resolve(&mut game, &mut dm);
        assert!(
            game.object(target).is_none(),
            "the targeted 2/3 fought the 4/3 Warden"
        );
        assert!(game.object(spare).is_some());
        assert!(game.object(source).is_some());
        resolve(&mut game, &mut dm);
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.calculated_power(source), Some(3));
        assert_eq!(game.calculated_toughness(source), Some(4));
    }
}

#[test]
fn kira_uses_first_event_history_even_before_the_grant_and_resets_on_the_next_turn() {
    for definition in definitions("Kira, Great Glass-Spinner") {
        for first_before_grant in [false, true] {
            let mut game = game();
            let target = object(
                &mut game,
                A,
                Zone::Battlefield,
                "Target creature",
                "Type: Creature\nPower/Toughness: 2/2",
            );
            let mut dm = Choices {
                targets: vec![Target::Object(target)],
                yes: true,
            };
            if !first_before_grant {
                game.create_object_from_definition(&definition, A, Zone::Battlefield);
            }
            for cast_index in 0..2 {
                let spell = object(
                    &mut game,
                    A,
                    Zone::Hand,
                    "Two target slots",
                    "Mana cost: {0}\nType: Instant\nTarget creature gets +0/+1 until end of turn. Target creature gets +0/+1 until end of turn.",
                );
                let spell = cast(&mut game, A, spell, &mut dm);
                let should_trigger = !first_before_grant && cast_index == 0;
                assert_eq!(game.stack.len(), if should_trigger { 2 } else { 1 });
                resolve(&mut game, &mut dm);
                if should_trigger {
                    assert!(!game.stack.iter().any(|entry| entry.object_id == spell));
                }
                if first_before_grant && cast_index == 0 {
                    game.create_object_from_definition(&definition, A, Zone::Battlefield);
                }
            }
            game.next_turn();
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let spell = object(
                &mut game,
                A,
                Zone::Hand,
                "New turn target",
                "Mana cost: {0}\nType: Instant\nTarget creature gets +0/+1 until end of turn.",
            );
            cast(&mut game, A, spell, &mut dm);
            assert_eq!(
                game.stack.len(),
                2,
                "first event resets with the actual turn"
            );
        }
    }
}

#[test]
fn dormant_target_trigger_is_captured_before_its_source_is_sacrificed_for_the_spell() {
    for definition in definitions("Dormant Gomazoa") {
        let mut game = game();
        let dormant = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spell = object(
            &mut game,
            A,
            Zone::Hand,
            "Target then sacrifice",
            "Mana cost: {0}\nType: Instant\nAs an additional cost to cast this spell, sacrifice a creature.\nTarget player gains 1 life.",
        );
        let mut dm = Choices {
            targets: vec![Target::Player(A)],
            yes: true,
        };
        cast(&mut game, A, spell, &mut dm);
        assert!(game.object(dormant).is_none());
        assert_eq!(
            game.stack.len(),
            2,
            "the targeting event preceded the sacrifice cost"
        );
        resolve(&mut game, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}
