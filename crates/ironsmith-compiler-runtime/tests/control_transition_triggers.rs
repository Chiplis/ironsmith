//! Real control transitions, including static and expiring effects. These
//! campaign regressions are authored but unrun under the source-only workflow.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, EffectExecutor, GainControlEffect};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/control_transition_triggers.json.fixture"
    ))
    .unwrap()
}
fn compile(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture["name"] == name)
        .unwrap();
    compile(name, fixture["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn resource(game: &mut GameState, owner: PlayerId, text: &str) -> ObjectId {
    game.create_object_from_definition(
        &compile_to_runtime_definition("Control resource", text, false).unwrap(),
        owner,
        Zone::Battlefield,
    )
}
fn apply(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    effect: &dyn EffectExecutor,
) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, controller, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn steal(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    target: ObjectId,
    duration: ironsmith::effect::Until,
) {
    apply(
        game,
        source,
        player,
        &GainControlEffect::new(ChooseSpec::SpecificObject(target), duration),
    );
}
fn pending(game: &mut GameState) -> usize {
    put_triggers_on_stack_with_dm(
        game,
        &mut TriggerQueue::new(),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState) {
    pending(game);
    for _ in 0..20 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        pending(game);
    }
    panic!("control trigger did not settle");
}
fn treasure_count(game: &GameState, controller: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.name == "Treasure")
                && game.current_controller(**id) == Some(controller)
        })
        .count()
}
#[test]
fn exact_six_full_card_fixtures_round_trip_without_omitting_secondary_bodies() {
    assert_eq!(fixtures().len(), 6);
    for fixture in fixtures() {
        for definition in compile(
            fixture["name"].as_str().unwrap(),
            fixture["text"].as_str().unwrap(),
        ) {
            assert!(!definition.abilities.is_empty());
        }
    }
}
#[test]
fn actual_transition_uses_old_controller_for_loss_and_new_controller_for_gain() {
    let text = "Type: Enchantment\nWhen you lose control of this enchantment, you gain 1 life.\nWhen you gain control of this enchantment from another player, you gain 2 life.";
    for definition in compile("Control directions", text) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let actor = resource(&mut game, B, "Type: Artifact");
        game.refresh_continuous_state().unwrap();
        assert_eq!(pending(&mut game), 0, "entry is not a change of control");
        steal(
            &mut game,
            actor,
            B,
            source,
            ironsmith::effect::Until::Forever,
        );
        assert_eq!(pending(&mut game), 2);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.player(B).unwrap().life, 22);
        assert_eq!(game.player(C).unwrap().life, 20);
        steal(
            &mut game,
            actor,
            B,
            source,
            ironsmith::effect::Until::Forever,
        );
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            pending(&mut game),
            0,
            "the same controller and repeated refreshes are not events"
        );
    }
}
#[test]
fn zidane_observes_its_own_theft_once_from_the_old_controllers_lookback() {
    for definition in definitions("Zidane, Tantalus Thief") {
        let mut game = game();
        let zidane = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let actor = resource(&mut game, B, "Type: Artifact");
        game.refresh_continuous_state().unwrap();
        steal(
            &mut game,
            actor,
            B,
            zidane,
            ironsmith::effect::Until::Forever,
        );
        assert_eq!(pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(treasure_count(&game, A), 1);
        assert_eq!(treasure_count(&game, B), 0);
    }
}
#[test]
fn zidane_sees_cleanup_expiration_and_static_control_source_departure() {
    for definition in definitions("Zidane, Tantalus Thief") {
        let mut game = game();
        let zidane = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = resource(&mut game, B, "Type: Creature\nPower/Toughness: 2/2");
        steal(
            &mut game,
            zidane,
            A,
            target,
            ironsmith::effect::Until::EndOfTurn,
        );
        assert_eq!(pending(&mut game), 0);
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_controller(target), Some(B));
        assert_eq!(pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(treasure_count(&game, A), 1);
        let aura = resource(
            &mut game,
            A,
            "Type: Enchantment — Aura\nEnchant creature\nYou control enchanted creature.",
        );
        apply(
            &mut game,
            aura,
            A,
            &ironsmith::effects::AttachToEffect::new(ChooseSpec::SpecificObject(target)),
        );
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_controller(target), Some(A));
        assert_eq!(pending(&mut game), 0);
        apply(
            &mut game,
            zidane,
            A,
            &ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(aura)),
        );
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_controller(target), Some(B));
        assert_eq!(pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(treasure_count(&game, A), 2);
    }
}
#[test]
fn losing_the_source_to_another_zone_fires_but_phasing_and_blink_identity_do_not_reuse_it() {
    for definition in compile(
        "Control departure",
        "Type: Artifact\nWhen you lose control of this artifact, you gain 1 life.",
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let actor = resource(&mut game, A, "Type: Artifact");
        game.refresh_continuous_state().unwrap();
        game.phase_out(source);
        game.refresh_continuous_state().unwrap();
        game.phase_in(source);
        game.refresh_continuous_state().unwrap();
        assert_eq!(pending(&mut game), 0);
        apply(
            &mut game,
            actor,
            A,
            &ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(source)),
        );
        assert_eq!(pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        let other = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        assert_ne!(source, other);
        game.refresh_continuous_state().unwrap();
        assert_eq!(pending(&mut game), 0);
    }
}

#[test]
fn control_loss_uses_previous_abilities_but_does_not_invent_a_newly_granted_listener() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let text = "Type: Enchantment\nWhen you lose control of this enchantment, you gain 1 life.\nWhen you gain control of this enchantment from another player, you gain 2 life.";
    for definition in compile("Control ability lookback", text) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let actor = resource(&mut game, B, "Type: Artifact");
        let mut change = ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::ChangeController(B),
            ironsmith::effect::Until::Forever,
        );
        change
            .additional_modifications
            .push(Modification::RemoveAllAbilities);
        apply(&mut game, actor, B, &change);
        assert_eq!(
            pending(&mut game),
            1,
            "loss looks back; the post-event source has no gain ability"
        );
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.player(B).unwrap().life, 20);

        let blank = resource(&mut game, A, "Type: Enchantment");
        let loss = definition
            .abilities
            .iter()
            .find(|ability| {
                matches!(&ability.kind,
            AbilityKind::Triggered(trigger) if trigger.trigger.display().contains("lose control"))
            })
            .unwrap()
            .clone();
        let mut gain_and_grant = ApplyContinuousEffect::new(
            EffectTarget::Specific(blank),
            Modification::ChangeController(B),
            ironsmith::effect::Until::Forever,
        );
        gain_and_grant
            .additional_modifications
            .push(Modification::AddAbilityGeneric(loss));
        apply(&mut game, actor, B, &gain_and_grant);
        assert_eq!(
            pending(&mut game),
            0,
            "an ability first granted after the transition did not exist before it"
        );
    }
}

#[test]
fn a_false_control_duration_changes_neither_controller_nor_soulbond_or_sickness() {
    for definition in definitions("Zidane, Tantalus Thief") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = resource(&mut game, B, "Type: Creature\nPower/Toughness: 2/2");
        let partner = resource(&mut game, B, "Type: Creature\nPower/Toughness: 2/2");
        game.refresh_continuous_state().unwrap();
        game.set_soulbond_pair(target, partner);
        game.remove_summoning_sickness(target);
        steal(
            &mut game,
            source,
            A,
            target,
            ironsmith::effect::Until::ForAsLongAs(
                ironsmith_core::ContinuousDurationPredicate::ObjectTapped(
                    ironsmith_core::ContinuousDurationObject::Source,
                ),
            ),
        );
        assert_eq!(game.current_controller(target), Some(B));
        assert_eq!(game.soulbond_partner(target), Some(partner));
        assert!(!game.is_summoning_sick(target));
        assert_eq!(pending(&mut game), 0);
    }
}

fn announce(game: &mut GameState, action: ironsmith::decision::LegalAction) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("action did not finish");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
}
fn activate(game: &mut GameState, source: ObjectId, ability_index: usize) {
    let action = ironsmith::decision::LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(
        ironsmith::decision::compute_legal_actions(game, A)
            .unwrap()
            .contains(&action)
    );
    announce(game, action);
    settle(game);
}
#[test]
fn scepters_paid_exile_and_return_keep_links_and_loss_moves_each_card_to_its_owner() {
    for definition in definitions("Gustha's Scepter") {
        for leaves in [false, true] {
            let mut game = game();
            let scepter = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let actor = resource(&mut game, B, "Type: Artifact");
            let card =
                compile_to_runtime_definition("Scepter held card", "Type: Land", false).unwrap();
            game.create_object_from_definition(&card, A, Zone::Hand);
            activate(&mut game, scepter, 0);
            assert!(game.is_tapped(scepter));
            assert!(game.player(A).unwrap().hand.is_empty());
            assert_eq!(game.get_exiled_with_source_links(scepter).len(), 1);
            apply(
                &mut game,
                actor,
                A,
                &ironsmith::effects::UntapEffect::with_spec(ChooseSpec::SpecificObject(scepter)),
            );
            activate(&mut game, scepter, 1);
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert!(game.get_exiled_with_source_links(scepter).is_empty());
            apply(
                &mut game,
                actor,
                A,
                &ironsmith::effects::UntapEffect::with_spec(ChooseSpec::SpecificObject(scepter)),
            );
            activate(&mut game, scepter, 0);
            if leaves {
                apply(
                    &mut game,
                    actor,
                    B,
                    &ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(
                        scepter,
                    )),
                );
            } else {
                steal(
                    &mut game,
                    actor,
                    B,
                    scepter,
                    ironsmith::effect::Until::Forever,
                );
            }
            assert_eq!(pending(&mut game), 1);
            settle(&mut game);
            assert!(game.exile.is_empty());
            assert_eq!(
                game.player(A)
                    .unwrap()
                    .graveyard
                    .iter()
                    .filter(|id| game
                        .object(**id)
                        .is_some_and(|object| object.name == "Scepter held card"))
                    .count(),
                1
            );
            assert!(game.player(B).unwrap().graveyard.is_empty());
        }
    }
}

#[test]
fn coffin_queen_reanimates_with_paid_cost_and_watches_only_its_exact_source() {
    for definition in definitions("Coffin Queen") {
        for ending in 0..4 {
            let mut game = game();
            let queen = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(queen);
            let actor = resource(&mut game, B, "Type: Artifact");
            let corpse = compile_to_runtime_definition(
                "Queen returned creature",
                "Type: Creature\nPower/Toughness: 2/2",
                false,
            )
            .unwrap();
            game.create_object_from_definition(&corpse, B, Zone::Graveyard);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::ManaSymbol::Colorless, 2);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::ManaSymbol::Black, 1);
            let index = definition
                .abilities
                .iter()
                .position(|ability| matches!(&ability.kind, AbilityKind::Activated(_)))
                .unwrap();
            activate(&mut game, queen, index);
            assert!(game.is_tapped(queen));
            let returned = game
                .battlefield
                .iter()
                .copied()
                .find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "Queen returned creature")
                })
                .unwrap();
            assert_eq!(game.current_controller(returned), Some(A));
            if ending == 3 {
                game.phase_out(queen);
                game.refresh_continuous_state().unwrap();
                game.phase_in(queen);
                game.refresh_continuous_state().unwrap();
                assert_eq!(pending(&mut game), 0);
            }
            match ending {
                0 => apply(
                    &mut game,
                    actor,
                    A,
                    &ironsmith::effects::UntapEffect::with_spec(ChooseSpec::SpecificObject(queen)),
                ),
                1 | 3 => steal(
                    &mut game,
                    actor,
                    B,
                    queen,
                    ironsmith::effect::Until::Forever,
                ),
                _ => apply(
                    &mut game,
                    actor,
                    B,
                    &ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(
                        queen,
                    )),
                ),
            }
            assert_eq!(pending(&mut game), 1);
            settle(&mut game);
            assert!(game.object(returned).is_none());
            assert_eq!(
                game.exile
                    .iter()
                    .filter(|id| game.object(**id).is_some_and(|object| object.name
                        == "Queen returned creature"
                        && object.owner == B))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn coffin_queen_leaving_before_resolution_does_not_watch_a_blinked_new_queen() {
    for definition in definitions("Coffin Queen") {
        let mut game = game();
        let queen = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(queen);
        let actor = resource(&mut game, A, "Type: Artifact");
        let corpse = compile_to_runtime_definition(
            "Queen retained creature",
            "Type: Creature\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&corpse, B, Zone::Graveyard);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Colorless, 2);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Black, 1);
        let index = definition
            .abilities
            .iter()
            .position(|ability| matches!(&ability.kind, AbilityKind::Activated(_)))
            .unwrap();
        announce(
            &mut game,
            ironsmith::decision::LegalAction::ActivateAbility {
                source: queen,
                ability_index: index,
            },
        );
        let stable = game.object(queen).unwrap().stable_id;
        apply(
            &mut game,
            actor,
            A,
            &ironsmith::effects::ExileEffect::with_spec(ChooseSpec::SpecificObject(queen)),
        );
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        apply(
            &mut game,
            actor,
            A,
            &ironsmith::effects::PutOntoBattlefieldEffect::new(
                ChooseSpec::SpecificObject(exiled),
                false,
                ironsmith::target::PlayerFilter::You,
            ),
        );
        let new_queen = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(queen, new_queen);
        settle(&mut game);
        game.tap(new_queen);
        apply(
            &mut game,
            actor,
            A,
            &ironsmith::effects::UntapEffect::with_spec(ChooseSpec::SpecificObject(new_queen)),
        );
        assert_eq!(pending(&mut game), 0);
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Queen retained creature")
        }));
    }
}

#[test]
fn ogre_geargrabber_watches_the_chosen_equipment_until_its_control_actually_returns() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    use ironsmith::object::AttachmentTarget;
    for definition in definitions("Ogre Geargrabber") {
        let mut game = game();
        let ogre = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(ogre);
        let first = resource(&mut game, B, "Type: Artifact — Equipment");
        let second = resource(&mut game, C, "Type: Artifact — Equipment");
        let other_creature = resource(&mut game, C, "Type: Creature\nPower/Toughness: 2/2");
        assert!(game.attach_object_to_target(second, AttachmentTarget::Object(other_creature)));
        game.turn.phase = ironsmith::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat = CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[AttackerDeclaration {
                creature: ogre,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        game.combat = Some(combat);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game);
        assert_eq!(game.current_controller(first), Some(A));
        assert_eq!(
            game.object(first).unwrap().attached_to,
            Some(AttachmentTarget::Object(ogre))
        );
        assert_eq!(game.current_controller(second), Some(C));
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_controller(first), Some(B));
        assert_eq!(pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(game.object(first).unwrap().attached_to, None);
        assert_eq!(
            game.object(second).unwrap().attached_to,
            Some(AttachmentTarget::Object(other_creature))
        );
    }
}

#[test]
fn duplicity_swaps_only_the_older_linked_cards_then_discards_and_cleans_up_on_control_loss() {
    for definition in definitions("Duplicity") {
        let mut game = game();
        let actor = resource(&mut game, A, "Type: Artifact");
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        let library_card =
            compile_to_runtime_definition("Duplicity library card", "Type: Land", false).unwrap();
        let hand_card =
            compile_to_runtime_definition("Duplicity original hand", "Type: Artifact", false)
                .unwrap();
        for _ in 0..6 {
            game.create_object_from_definition(&library_card, A, Zone::Library);
        }
        for _ in 0..2 {
            game.create_object_from_definition(&hand_card, A, Zone::Hand);
        }
        apply(
            &mut game,
            actor,
            A,
            &ironsmith::effects::PutOntoBattlefieldEffect::new(
                ChooseSpec::SpecificObject(source),
                false,
                ironsmith::target::PlayerFilter::You,
            ),
        );
        let source = game.find_object_by_stable_id(stable).unwrap();
        settle(&mut game);
        assert_eq!(game.get_exiled_with_source_links(source).len(), 5);
        assert_eq!(game.player(A).unwrap().library.len(), 1);
        let mut queue = TriggerQueue::new();
        game.turn.phase = ironsmith::Phase::Beginning;
        let mut upkeep = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::Upkeep,
        );
        upkeep.advance(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().hand.len(), 5);
        assert!(
            game.player(A)
                .unwrap()
                .hand
                .iter()
                .all(|id| game.object(*id).unwrap().name == "Duplicity library card")
        );
        assert_eq!(game.get_exiled_with_source_links(source).len(), 2);
        assert!(
            game.get_exiled_with_source_links(source)
                .iter()
                .all(|id| game.object(*id).unwrap().name == "Duplicity original hand")
        );
        game.turn.phase = ironsmith::Phase::Ending;
        let mut end = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::EndStep,
        );
        end.advance(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().hand.len(), 4);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        steal(
            &mut game,
            actor,
            C,
            source,
            ironsmith::effect::Until::Forever,
        );
        assert_eq!(pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
        assert!(game.exile.is_empty());
        assert!(game.player(C).unwrap().graveyard.is_empty());
    }
}

#[test]
fn a_simultaneously_sacrificed_zidane_sees_control_revert_but_an_earlier_departure_does_not() {
    for definition in definitions("Zidane, Tantalus Thief") {
        for simultaneous in [false, true] {
            let mut game = game();
            let zidane = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let actor = resource(&mut game, A, "Type: Artifact");
            let target = resource(&mut game, B, "Type: Creature\nPower/Toughness: 2/2");
            let aura = resource(
                &mut game,
                A,
                "Type: Enchantment — Aura\nEnchant creature\nYou control enchanted creature.",
            );
            apply(
                &mut game,
                aura,
                A,
                &ironsmith::effects::AttachToEffect::new(ChooseSpec::SpecificObject(target)),
            );
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_controller(target), Some(A));
            assert_eq!(pending(&mut game), 0);
            let sacrifice = |game: &mut GameState, objects: &[ObjectId]| {
                let snapshots = objects.iter().map(|id| ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(*id).unwrap(), game)).collect();
                let mut dm = SelectFirstDecisionMaker;
                let mut ctx = EffectContext::new(actor, A, &mut dm);
                ctx.set_tagged_objects("sacrifice_pair", snapshots);
                let effect = ironsmith::effects::SacrificeEffect::new(
                    ironsmith::target::ObjectFilter::tagged("sacrifice_pair"),
                    objects.len() as i32,
                    ironsmith::target::PlayerFilter::You,
                );
                let outcome = effect.execute(game, &mut ctx).unwrap();
                for event in outcome.events {
                    game.queue_trigger_event(event.provenance(), event);
                }
            };
            if simultaneous {
                sacrifice(&mut game, &[zidane, aura]);
            } else {
                sacrifice(&mut game, &[zidane]);
                settle(&mut game);
                sacrifice(&mut game, &[aura]);
            }
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_controller(target), Some(B));
            assert_eq!(pending(&mut game), usize::from(simultaneous));
            settle(&mut game);
            assert_eq!(treasure_count(&game, A), usize::from(simultaneous));
        }
    }
}

#[test]
fn risky_moves_upkeep_recipient_makes_both_choices_and_only_a_lost_coin_transfers_the_chosen_creature()
 {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext};
    struct Choices {
        creature: ObjectId,
        object_choosers: Vec<PlayerId>,
        player_choosers: Vec<PlayerId>,
    }
    impl DecisionMaker for Choices {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
            self.object_choosers.push(ctx.player);
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal && candidate.id == self.creature)
                .map(|candidate| candidate.id)
                .collect()
        }
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|option| option.legal && option.description == "Charlie")
            {
                self.player_choosers.push(ctx.player);
                vec![option.index]
            } else {
                ctx.options
                    .iter()
                    .filter(|option| option.legal)
                    .take(ctx.min)
                    .map(|option| option.index)
                    .collect()
            }
        }
    }
    for definition in definitions("Risky Move") {
        let mut saw_win = false;
        let mut saw_loss = false;
        for seed in 0..16 {
            let mut game = game();
            game.set_random_seed(seed);
            let risky = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let selected = resource(&mut game, B, "Type: Creature\nPower/Toughness: 2/2");
            let unchosen = resource(&mut game, B, "Type: Creature\nPower/Toughness: 3/3");
            let wrong_controller = resource(&mut game, A, "Type: Creature\nPower/Toughness: 4/4");
            game.refresh_continuous_state().unwrap();
            game.turn.active_player = B;
            game.turn.phase = ironsmith::Phase::Beginning;
            let mut queue = TriggerQueue::new();
            let mut upkeep = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
                ironsmith::turn_runner::TurnState::Upkeep,
            );
            upkeep.advance(&mut game, &mut queue).unwrap();
            let mut dm = Choices {
                creature: selected,
                object_choosers: vec![],
                player_choosers: vec![],
            };
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            for _ in 0..20 {
                if game.stack_is_empty() {
                    break;
                }
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            }
            assert!(game.stack_is_empty());
            assert_eq!(game.current_controller(risky), Some(B));
            assert_eq!(dm.object_choosers, vec![B]);
            assert_eq!(dm.player_choosers, vec![B]);
            let flip = game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .filter_map(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::CoinFlippedEvent>()
                })
                .last()
                .expect("actual coin flip event");
            assert_eq!(flip.player, B);
            let lost = flip.flipper_lost();
            saw_loss |= lost;
            saw_win |= flip.flipper_won();
            assert_eq!(
                game.current_controller(selected),
                Some(if lost { C } else { B })
            );
            assert_eq!(game.current_controller(unchosen), Some(B));
            assert_eq!(game.current_controller(wrong_controller), Some(A));
        }
        assert!(
            saw_win && saw_loss,
            "fixed seed fixtures exercise both genuine coin outcomes"
        );
    }
}
