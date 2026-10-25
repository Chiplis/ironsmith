//! CR audit (in-review completion pass, package q08): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::{DecisionMaker, EffectExecutor, PlayerFilter};

fn q08_complete_action<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: ironsmith::LegalAction,
    dm: &mut D,
    queue: &mut ironsmith::triggers::TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        ironsmith::LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    assert_eq!(game.turn.priority_player, Some(player));
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if let Some(pending) = &state.pending_method_selection {
            let index = pending
                .available_methods
                .iter()
                .position(|choice| Some(&choice.method) == requested_method.as_ref())
                .ok_or_else(|| "requested casting method lost during method selection".to_string())?;
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending native action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    if state.pending_cast.is_some()
        || state.pending_method_selection.is_some()
        || state.pending_activation.is_some()
    {
        return Err("native action exceeded decision bound".into());
    }
    Ok(())
}

fn q08_creature(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    power: i32,
    toughness: i32,
) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let def = CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(power, toughness))
        .build();
    let id = game.create_object_from_definition(&def, owner, ironsmith::Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

/// CR 702.91b: two battle cry instances on one creature trigger separately and
/// each pumps the other attacking creature.
#[test]
fn cr_702_91b_two_battle_cry_instances_trigger_separately() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::decision::AttackerDeclaration;
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{
        CardId, CardType, Phase, PowerToughness, Step, TurnAction, TurnRunner, TurnRunnerState,
        Zone,
    };
    for instances in [1usize, 2] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Battle cry source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 4));
        for _ in 0..instances {
            builder = builder.battle_cry();
        }
        let source = game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let ally = q08_creature(&mut game, alice, "Battle cry ally", 1, 5);
        let mut queue = TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMainPriority);
        runner.priority_done();
        let mut declared = false;
        let mut triggers_seen = None;
        let mut ally_power_after = None;
        for _ in 0..200 {
            match runner.advance(&mut game, &mut queue).unwrap() {
                TurnAction::Continue => {}
                TurnAction::Decision(DecisionContext::Attackers(_)) => {
                    runner.respond_attackers(vec![
                        AttackerDeclaration {
                            creature: source,
                            target: AttackTarget::Player(bob),
                        },
                        AttackerDeclaration {
                            creature: ally,
                            target: AttackTarget::Player(bob),
                        },
                    ]);
                    declared = true;
                }
                TurnAction::Decision(DecisionContext::Blockers(c)) => {
                    runner.respond_blockers(vec![], c.player);
                }
                TurnAction::Decision(other) => panic!("unexpected decision {other:?}"),
                TurnAction::RunPriority => {
                    ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm)
                        .unwrap();
                    if declared
                        && triggers_seen.is_none()
                        && game.turn.step == Some(Step::DeclareAttackers)
                    {
                        triggers_seen = Some(
                            game.stack
                                .iter()
                                .filter(|e| e.is_ability && e.object_id == source)
                                .count(),
                        );
                        // Resolve the whole stack through ordinary passes.
                        for _ in 0..20 {
                            if game.stack.is_empty() {
                                break;
                            }
                            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                            ironsmith::game_loop::advance_priority_with_dm(
                                &mut game, &mut queue, &mut dm,
                            )
                            .unwrap();
                        }
                        ally_power_after = game.current_power(ally);
                        assert_eq!(
                            game.current_power(source),
                            Some(2),
                            "CR 702.91a: battle cry does not pump its own source"
                        );
                        break;
                    }
                    runner.priority_done();
                }
                TurnAction::TurnComplete => break,
                other => panic!("unexpected runner action {other:?}"),
            }
        }
        assert_eq!(
            triggers_seen,
            Some(instances),
            "CR 702.91b: each battle cry instance triggers separately"
        );
        assert_eq!(
            ally_power_after,
            Some(1 + instances as i32),
            "CR 702.91b: each resolved battle cry trigger gives the other attacker +1/+0"
        );
    }
}

/// CR 704.4: SBAs ignore what happens during resolution — a player brought to
/// 0 life and a creature dealt lethal damage mid-resolution survive when the
/// same resolution later fixes the state.
#[test]
fn cr_704_4_sbas_ignore_intermediate_states_during_resolution() {
    use ironsmith::{ChooseSpec, Effect, Phase, StackEntry, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    let creature = q08_creature(&mut game, bob, "Intermediate victim", 2, 2);
    let source = q08_creature(&mut game, alice, "Intermediate source", 1, 1);
    game.stack.push(StackEntry::ability(
        source,
        alice,
        vec![
            Effect::lose_life(20),
            Effect::deal_damage(3, ChooseSpec::SpecificObject(creature)),
            Effect::gain_life(5),
            Effect::pump(0, 5, ChooseSpec::SpecificObject(creature), Until::EndOfTurn),
        ],
    ));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    let a = game.player(alice).unwrap();
    assert_eq!(a.life, 5, "fixture: life went 20 -> 0 -> 5 during one resolution");
    assert!(
        !a.has_lost,
        "CR 704.4: 0 life during resolution is ignored by SBAs checked afterwards"
    );
    assert_eq!(
        game.object(creature).map(|o| o.zone),
        Some(Zone::Battlefield),
        "CR 704.4: lethal damage that stops being lethal before SBAs are checked does not destroy"
    );
}

/// CR 702.103a/b: casting bestowed pays the bestow cost, and the spell is an
/// Aura enchantment (not a creature) with enchant creature on the stack and as
/// it resolves attached to a creature.
#[test]
fn cr_702_103a_b_bestow_pays_bestow_cost_and_is_an_enchant_creature_aura() {
    use ironsmith::decisions::context::TargetsContext;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{
        CardType, CastingMethod, LegalAction, ManaSymbol, ObjectId, Phase, Subtype, Target, Zone,
    };
    struct Pick {
        host: ObjectId,
        artifact: ObjectId,
        saw_requirement: bool,
        artifact_offered: bool,
    }
    impl DecisionMaker for Pick {
        fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            self.saw_requirement = true;
            for req in &ctx.requirements {
                if req.legal_targets.contains(&Target::Object(self.artifact)) {
                    self.artifact_offered = true;
                }
            }
            vec![Target::Object(self.host)]
        }
    }
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Hopeful Eidolon",
        "Mana cost: {W}\nType: Enchantment Creature — Spirit\nPower/Toughness: 1/1\nBestow {3}{W} (If you cast this card for its bestow cost, it's an Aura spell with enchant creature. It becomes a creature again if it's not attached to a creature.)\nLifelink\nEnchanted creature gets +1/+1 and has lifelink.",
        false,
    )
    .unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&def));
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let host = q08_creature(&mut game, alice, "Bestow host", 2, 2);
    let artifact_def = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Noncreature artifact",
    )
    .card_types(vec![CardType::Artifact])
    .build();
    let artifact = game.create_object_from_definition(&artifact_def, alice, Zone::Battlefield);
    let card = game.create_object_from_definition(&def, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::White, 5);
    let action = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| {
            matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(_), .. } if *spell_id == card)
        })
        .expect("CR 702.103a: bestow is offered as an alternative cast from hand");
    let mut queue = TriggerQueue::new();
    let mut dm = Pick {
        host,
        artifact,
        saw_requirement: false,
        artifact_offered: false,
    };
    q08_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
    assert!(dm.saw_requirement, "CR 702.103b: the bestowed Aura spell targets as an Aura");
    assert!(
        !dm.artifact_offered,
        "CR 702.103b: a bestowed spell has enchant creature, so a noncreature artifact is not a legal target"
    );
    assert_eq!(
        game.player(alice).unwrap().mana_pool.amount(ManaSymbol::White),
        1,
        "CR 702.103a: the bestow cost {{3}}{{W}} is paid rather than the mana cost {{W}}"
    );
    assert_eq!(game.stack.len(), 1);
    let spell = game.stack[0].object_id;
    let spell_obj = game.object(spell).unwrap();
    assert!(spell_obj.subtypes.contains(&Subtype::Aura), "CR 702.103b: bestowed spell is an Aura");
    assert!(
        !spell_obj.card_types.contains(&CardType::Creature),
        "CR 702.103b: bestowed spell is not a creature spell"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let perm = game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).is_some_and(|o| o.name == "Hopeful Eidolon"))
        .expect("bestowed Aura resolves onto the battlefield");
    let o = game.object(perm).unwrap();
    assert!(o.subtypes.contains(&Subtype::Aura) && !o.card_types.contains(&CardType::Creature));
    assert_eq!(
        o.attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(host)),
        "CR 702.103b: the bestowed Aura enters attached to its target"
    );
    assert_eq!(game.current_power(host), Some(3), "Enchanted creature gets +1/+1");
}

/// CR 702.118b: skulk compares power when blocks are declared, including
/// continuous changes applied before the declaration.
#[test]
fn cr_702_118b_skulk_uses_power_modified_before_blocks() {
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Phase, StaticAbility, Step, Until};
    for (attacker_pump, blocker_pump) in [(0, 0), (0, 2), (2, 2), (3, 0)] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let attacker = q08_creature(&mut game, alice, "Skulker", 2, 2);
        let blocker = q08_creature(&mut game, bob, "Blocker", 2, 2);
        let ctx_source = attacker;
        ApplyContinuousEffect::new(
            EffectTarget::Specific(attacker),
            Modification::AddAbility(StaticAbility::skulk()),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(ctx_source, alice))
        .unwrap();
        for (id, amount) in [(attacker, attacker_pump), (blocker, blocker_pump)] {
            if amount != 0 {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(id),
                    Modification::ModifyPowerToughness {
                        power: amount,
                        toughness: 0,
                    },
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(ctx_source, alice))
                .unwrap();
            }
        }
        game.refresh_continuous_state().unwrap();
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker, AttackTarget::Player(bob))],
        )
        .unwrap();
        game.turn.step = Some(Step::DeclareBlockers);
        let legal = 2 + blocker_pump <= 2 + attacker_pump;
        let result = combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)]);
        assert_eq!(
            result.is_ok(),
            legal,
            "CR 702.118b: attacker +{attacker_pump}, blocker +{blocker_pump}"
        );
    }
}

/// CR 702.111b: menace is checked per attacker when several attackers are blocked.
#[test]
fn cr_702_111b_menace_is_checked_per_attacker_with_multiple_attackers() {
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Phase, StaticAbility, Step, Until};
    // (a_blockers, b_blockers) counts drawn from 3 blockers; both attackers have menace.
    for (split, legal) in [((1usize, 1usize), false), ((2, 0), true), ((2, 1), false), ((0, 0), true), ((0, 2), true)] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let a = q08_creature(&mut game, alice, "Menace A", 3, 3);
        let b = q08_creature(&mut game, alice, "Menace B", 3, 3);
        for id in [a, b] {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(id),
                Modification::AddAbility(StaticAbility::menace()),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(id, alice))
            .unwrap();
        }
        let blockers: Vec<_> = (0..3)
            .map(|i| q08_creature(&mut game, bob, &format!("Blocker {i}"), 1, 1))
            .collect();
        game.refresh_continuous_state().unwrap();
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(a, AttackTarget::Player(bob)), (b, AttackTarget::Player(bob))],
        )
        .unwrap();
        game.turn.step = Some(Step::DeclareBlockers);
        let mut decl = Vec::new();
        let mut it = blockers.iter();
        for _ in 0..split.0 {
            decl.push((*it.next().unwrap(), a));
        }
        for _ in 0..split.1 {
            decl.push((*it.next().unwrap(), b));
        }
        let result = combat_state::declare_blockers(&game, &mut combat, decl);
        assert_eq!(result.is_ok(), legal, "CR 702.111b: blockers split {split:?}");
    }
}

/// CR 704.5x: a battle being attacked is left alone; with no legal protector
/// available it goes to its owner's graveyard; otherwise a choice is required.
#[test]
fn cr_704_5x_attacked_and_no_eligible_protector_branches() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::rules::state_based::{StateBasedAction, check_state_based_actions};
    use ironsmith::{CardId, CardType, CounterType, Phase, Step, Subtype, Zone};
    for (players, attacked) in [(3usize, true), (3, false), (2, false)] {
        let names: Vec<String> = ["Alice", "Bob", "Carol"][..players]
            .iter()
            .map(|s| (*s).into())
            .collect();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(names, 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let def = CardDefinitionBuilder::new(CardId::new(), "Siege battle")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![Subtype::Siege])
            .defense(5)
            .build();
        let battle = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        if game.object(battle).unwrap().counters.get(&CounterType::Defense).copied().unwrap_or(0) == 0 {
            game.add_counters(battle, CounterType::Defense, 5);
        }
        assert!(game.set_battle_protector(battle, bob) || game.battle_protector(battle) == Some(bob));
        let attacker = q08_creature(&mut game, alice, "Battle attacker", 2, 2);
        if attacked {
            let mut combat = combat_state::new_combat();
            combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(attacker, AttackTarget::Battle(battle))],
            )
            .unwrap();
            game.combat = Some(combat);
        }
        // The protector leaves the game.
        game.player_mut(bob).unwrap().has_lost = true;
        let actions = check_state_based_actions(&game);
        let choice = actions.contains(&StateBasedAction::BattleProtectorChoice(battle));
        let dies = actions.contains(&StateBasedAction::BattleDies(battle));
        if attacked {
            assert!(
                !choice && !dies,
                "CR 704.5x: no protector is chosen while attacking creatures attack the battle; got {actions:?}"
            );
        } else if players == 3 {
            assert!(choice && !dies, "CR 704.5x: controller chooses a new opponent protector; got {actions:?}");
        } else {
            assert!(dies, "CR 704.5x: no eligible protector puts the battle into its owner's graveyard; got {actions:?}");
        }
    }
}

fn q08_copy(game: &mut GameState, recipient: ironsmith::ObjectId, donor: ironsmith::ObjectId, name_override: Option<&str>, legendary: bool) {
    use ironsmith::continuous::EffectTarget;
    use ironsmith::effects::{ApplyContinuousEffect, RuntimeModification};
    use ironsmith::{ChooseSpec, Supertype, Until};
    let controller = game.current_controller(recipient).unwrap();
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(recipient),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false,
            name_override: name_override.map(str::to_string),
            name_override_surface: None,
            add_supertypes: if legendary { vec![Supertype::Legendary] } else { vec![] },
            copy_exception_surface: None,
        },
        Until::Forever,
    )
    .execute(game, &mut EffectContext::new_default(recipient, controller))
    .unwrap();
    game.refresh_continuous_state().unwrap();
}

/// CR 707.3: an object copying a copy uses the copy's copiable values,
/// including the copy effect's exceptions, but not noncopy effects.
/// CR 707.2b: changing the original's copiable values afterward doesn't change the copy.
#[test]
fn cr_707_3_and_707_2b_copy_of_copy_uses_modified_copiable_values_and_locks_them() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Supertype, Until};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let donor = q08_creature(&mut game, bob, "Original donor", 3, 6);
    let first = q08_creature(&mut game, alice, "First copier", 1, 1);
    let second = q08_creature(&mut game, alice, "Second copier", 1, 1);
    let later = q08_creature(&mut game, bob, "Later template", 7, 7);
    q08_copy(&mut game, first, donor, Some("Exception name"), true);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(first),
        Modification::ModifyPowerToughness { power: 5, toughness: 5 },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(first, alice))
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_power(first), Some(8), "fixture: noncopy pump on first copier");
    q08_copy(&mut game, second, first, None, false);
    assert_eq!(
        game.current_name(second).as_deref(),
        Some("Exception name"),
        "CR 707.3: the copy's exception is part of its copiable values"
    );
    assert!(
        game.current_supertypes(second).unwrap().contains(&Supertype::Legendary),
        "CR 707.3: added supertype exception is copied"
    );
    assert_eq!(
        (game.current_power(second), game.current_toughness(second)),
        (Some(3), Some(6)),
        "CR 707.3/707.2: noncopy effects on the copied copy are not copied"
    );
    // CR 707.2b: the original donor now changes its copiable values.
    q08_copy(&mut game, donor, later, None, false);
    assert_eq!(game.current_name(donor).as_deref(), Some("Later template"));
    assert_eq!(
        (game.current_name(first).as_deref(), game.current_power(first)),
        (Some("Exception name"), Some(8)),
        "CR 707.2b: changing the original's copiable values doesn't change an existing copy"
    );
    assert_eq!(
        (game.current_name(second).as_deref(), game.current_power(second)),
        (Some("Exception name"), Some(3)),
        "CR 707.2b: the copy of the copy is unchanged too"
    );
}

/// CR 704.5p: an Equipment that becomes a creature, or stops being an
/// Equipment (becoming a plain artifact), while attached becomes unattached
/// and remains on the battlefield.
#[test]
fn cr_704_5p_equipment_that_becomes_creature_or_plain_artifact_unattaches() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, Subtype, Zone};
    for becomes_creature in [false, true] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = q08_creature(&mut game, alice, "Equipped host", 2, 2);
        let equipment_def = CardDefinitionBuilder::new(CardId::new(), "Plain Equipment")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        let equipment = game.create_object_from_definition(&equipment_def, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(host)));
        let template = if becomes_creature {
            q08_creature(&mut game, alice, "Creature template", 4, 4)
        } else {
            let def = CardDefinitionBuilder::new(CardId::new(), "Plain artifact template")
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_definition(&def, alice, Zone::Battlefield)
        };
        q08_copy(&mut game, equipment, template, None, false);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let o = game.object(equipment).expect("still exists");
        assert_eq!(o.zone, Zone::Battlefield, "CR 704.5p: it remains on the battlefield");
        assert!(
            o.attached_to.is_none(),
            "CR 704.5p: becomes_creature={becomes_creature}: no longer a legal attachment, so it becomes unattached"
        );
    }
}

/// CR 702.184b: a station card's station symbols are keyword abilities — the
/// threshold row works at its charge threshold and is lost with "loses all abilities".
#[test]
fn cr_702_184b_station_symbol_rows_are_abilities_of_the_station_card() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CounterType, Until, Zone};
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Exploration Broodship",
        "Mana cost: {G}\nType: Artifact — Spacecraft\nPower/Toughness: 4/4\nStation (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)\n3+ | You may play an additional land on each of your turns.\n8+ | Flying\nOnce during each of your turns, you may cast a permanent spell from your graveyard by sacrificing a land in addition to paying its other costs.",
        false,
    )
    .unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&def));
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.player(alice).unwrap().land_plays_per_turn, 1, "below threshold");
    game.add_counters(source, CounterType::Charge, 3);
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.player(alice).unwrap().land_plays_per_turn,
        2,
        "CR 702.184b/721: the station symbol row applies at 3+ charge counters"
    );
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::RemoveAllAbilities,
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.player(alice).unwrap().land_plays_per_turn,
        1,
        "CR 702.184b: station symbols are keyword abilities, so losing all abilities removes the row"
    );
}

/// CR 707.10c: when a copy may get new targets, the controller may leave a
/// now-illegal target unchanged; if they change targets, the new targets must be legal.
#[test]
fn cr_707_10c_copy_may_keep_an_illegal_target_but_new_targets_must_be_legal() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::decisions::context::{BooleanContext, TargetsContext};
    use ironsmith::effect::EffectId;
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{ChooseSpec, Effect, LegalAction, ObjectId, Phase, StackEntry, StaticAbility, Target, Until, Zone};
    struct Dm {
        first: ObjectId,
        second: ObjectId,
        change: bool,
        stage: u8,
        retarget_domain: Option<Vec<Target>>,
    }
    impl DecisionMaker for Dm {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            self.change
        }
        fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
            if self.stage == 0 {
                vec![Target::Object(self.first)]
            } else {
                self.retarget_domain = Some(c.requirements[0].legal_targets.clone());
                vec![Target::Object(self.second)]
            }
        }
    }
    let bolt = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Q08 copy bolt",
        "Mana cost: {0}\nType: Instant\nThis spell deals 2 damage to target creature.",
        false,
    )
    .unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&bolt));
    for change in [false, true] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let first = q08_creature(&mut game, bob, "First target", 3, 3);
        let second = q08_creature(&mut game, bob, "Second target", 3, 3);
        let third = q08_creature(&mut game, bob, "Always shrouded", 3, 3);
        let copier_source = q08_creature(&mut game, alice, "Copier source", 1, 1);
        let card = game.create_object_from_definition(&bolt, alice, Zone::Hand);
        let action = ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
            .unwrap();
        let mut dm = Dm { first, second, change, stage: 0, retarget_domain: None };
        let mut queue = TriggerQueue::new();
        q08_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        let original = game.stack[0].object_id;
        assert_eq!(game.stack[0].targets, vec![Target::Object(first)]);
        // The original's target becomes illegal; a third creature is also illegal.
        for id in [first, third] {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(id),
                Modification::AddAbility(StaticAbility::shroud()),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(id, bob))
            .unwrap();
        }
        game.refresh_continuous_state().unwrap();
        dm.stage = 1;
        game.stack.push(StackEntry::ability(
            copier_source,
            alice,
            vec![
                Effect::with_id(0, Effect::copy_spell(ChooseSpec::SpecificObject(original))),
                Effect::may_choose_new_targets(EffectId(0)),
            ],
        ));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 2, "fixture: copy put on the stack above the original");
        let copy_targets = game.stack.last().unwrap().targets.clone();
        if change {
            let domain = dm.retarget_domain.clone().expect("new targets were chosen");
            // Offering the unchanged (now illegal) original target is the "leave unchanged" option.
            assert!(
                !domain.contains(&Target::Object(third)),
                "CR 707.10c: changed targets must be legal; a shrouded creature cannot be chosen as a new target; domain={domain:?}"
            );
            assert!(domain.contains(&Target::Object(second)));
            assert_eq!(copy_targets, vec![Target::Object(second)]);
        } else {
            assert_eq!(
                copy_targets,
                vec![Target::Object(first)],
                "CR 707.10c: the controller may leave the target unchanged even though it is now illegal"
            );
        }
    }
}

/// CR 708.9: when a player leaves the game, their face-down permanents and
/// face-down spells are revealed to all players (the game continues). Accepts
/// any public channel the engine uses: a reveal receipt, a public-reveal mark,
/// or an explicit end-of-match opening request for the departed card.
#[test]
fn cr_708_9_departing_player_reveals_face_down_permanents_and_spells() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::game_state::HiddenCardInfo;
    use ironsmith::{CardId, CardType, Zone};
    let def = CardDefinitionBuilder::new(CardId::new(), "Hidden identity")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
    let mut failures = Vec::new();
    for tracked in [false, true] {
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let bob_perm = game.create_object_from_definition(&def, bob, Zone::Battlefield);
        let bob_spell = game.create_object_from_definition(&def, bob, Zone::Stack);
        for (slot, (id, zone)) in [(bob_perm, Zone::Battlefield), (bob_spell, Zone::Stack)].into_iter().enumerate() {
            assert!(game.set_face_down(id));
            if tracked {
                let commitment = format!("q08-depart-{slot}");
                game.set_hidden_card_info(
                    id,
                    HiddenCardInfo {
                        incarnation: Some(0),
                        owner: bob,
                        zone,
                        slot: slot as u16,
                        commitment: commitment.clone(),
                        origin_slot: Some(slot as u16),
                        origin_commitment: Some(commitment),
                        public_slot: None,
                        public_commitment: None,
                    },
                );
            }
        }
        let stables: Vec<_> = [bob_perm, bob_spell]
            .iter()
            .map(|id| game.object(*id).unwrap().stable_id)
            .collect();
        let before = game.ui_effect_events().map(|e| e.id).max();
        game.concede_game(bob).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let _ = ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue);
        assert!(!game.player(bob).unwrap().is_in_game(), "fixture: Bob left the game");
        let disclosure = game.end_of_match_disclosure_cards(bob);
        for (label, id, stable) in [("permanent", bob_perm, stables[0]), ("spell", bob_spell, stables[1])] {
            let known = game.ui_effect_events().any(|e| {
                before.is_none_or(|b| e.id > b) && e.kind == "reveal" && e.stable_ids.contains(&stable)
            }) || game.is_publicly_revealed_hidden_card(id)
                || disclosure.iter().any(|c| c.object_id == id && !c.anchor_only);
            if !known {
                failures.push(format!("tracked={tracked}: departed owner's face-down {label} was not revealed"));
            }
        }
    }
    assert!(failures.is_empty(), "CR 708.9: {failures:?}");
}

/// CR 702.160a: a prototyped cast uses the alternative power, toughness and
/// mana cost (and color derived from it); a normal cast uses the printed set.
#[test]
fn cr_702_160a_prototyped_cast_uses_the_alternative_characteristic_set() {
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{CastingMethod, Color, LegalAction, ManaSymbol, Phase, Zone};
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Blitz Automaton",
        "Mana cost: {7}\nType: Artifact Creature — Construct\nPower/Toughness: 6/4\nPrototype {2}{R} — 3/2 (You may cast this spell with different mana cost, color, and size. It keeps its abilities and types.)\nHaste",
        false,
    )
    .unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&def));
    for prototyped in [false, true] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let card = game.create_object_from_definition(&def, alice, Zone::Hand);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 8);
        let actions: Vec<_> = ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .into_iter()
            .filter(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
            .collect();
        let action = actions
            .iter()
            .find(|a| {
                let normal = matches!(a, LegalAction::CastSpell { casting_method: CastingMethod::Normal, .. });
                normal != prototyped
            })
            .cloned()
            .unwrap_or_else(|| panic!("CR 702.160a: prototyped={prototyped} cast offered; actions={actions:?}"));
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut queue = TriggerQueue::new();
        q08_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
        let spent = 8 - game.player(alice).unwrap().mana_pool.amount(ManaSymbol::Red);
        assert_eq!(spent, if prototyped { 3 } else { 7 }, "CR 702.160a: prototyped={prototyped} cost paid");
        let spell = game.stack.last().unwrap().object_id;
        let (pt, mv) = (
            (game.current_power(spell), game.current_toughness(spell)),
            ironsmith::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game).mana_value(),
        );
        let expected_pt = if prototyped { (Some(3), Some(2)) } else { (Some(6), Some(4)) };
        assert_eq!(pt, expected_pt, "CR 702.160a: spell P/T prototyped={prototyped}");
        assert_eq!(mv, if prototyped { 3 } else { 7 }, "CR 702.160a: spell mana value");
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let perm = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).is_some_and(|o| o.name == "Blitz Automaton"))
            .unwrap();
        assert_eq!(
            (game.current_power(perm), game.current_toughness(perm)),
            expected_pt,
            "CR 702.160a: permanent keeps the chosen characteristic set"
        );
        let red = game.current_colors(perm).unwrap().contains(Color::Red);
        assert_eq!(red, prototyped, "CR 702.160a/718.3b: prototyped permanent is red, normal one colorless");
    }
}
