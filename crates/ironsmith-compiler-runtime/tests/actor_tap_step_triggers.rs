//! Active tap/untap subjects retain actor, pre-transition recipient and step
//! provenance. Source-only campaign: these authored regressions are unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, EffectExecutor, ResolvedTarget, TapEffect, UntapEffect};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions_from(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/actor_tap_step_triggers.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    definitions_from(name, row["text"].as_str().unwrap())
}
fn card(game: &mut GameState, player: PlayerId, zone: Zone, creature: bool) -> ObjectId {
    let definition = compile_to_runtime_definition(
        "Actor tap resource",
        if creature {
            "Type: Creature\nPower/Toughness: 2/2"
        } else {
            "Type: Artifact"
        },
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn stack(game: &mut GameState) -> usize {
    put_triggers_on_stack_with_dm(
        game,
        &mut TriggerQueue::new(),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState) -> usize {
    let count = stack(game);
    for _ in 0..20 {
        if game.stack_is_empty() {
            return count;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        stack(game);
    }
    panic!("actor tap triggers did not settle");
}
fn execute(game: &mut GameState, source: ObjectId, player: PlayerId, effect: &dyn EffectExecutor) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, player, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
}
fn time(game: &GameState, source: ObjectId) -> u32 {
    game.object(source)
        .unwrap()
        .counters
        .get(&ironsmith::object::CounterType::Time)
        .copied()
        .unwrap_or(0)
}

#[test]
fn all_four_exact_actor_cards_round_trip_with_explicit_actor_and_step_models() {
    assert_eq!(fixtures().len(), 4);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
                if triggered.trigger.compiled_model().is_some_and(|model| matches!(&model.kind, TriggerKind::PlayerChangesTapState { player: PlayerFilter::You, .. })))));
        }
    }
}

#[test]
fn sentry_requires_the_actor_to_be_you_and_the_pre_tap_recipient_to_be_an_opponents_creature() {
    for definition in definitions("Icewrought Sentry") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let power = game.current_power(source).unwrap();
        let creature = card(&mut game, B, Zone::Battlefield, true);
        execute(
            &mut game,
            source,
            B,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(creature)),
        );
        assert_eq!(
            settle(&mut game),
            0,
            "recipient control is not the acting player"
        );
        game.untap(creature);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(creature)),
        );
        assert_eq!(settle(&mut game), 1);
        assert_eq!(game.current_power(source), Some(power + 2));
        execute(
            &mut game,
            source,
            A,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(creature)),
        );
        assert_eq!(
            settle(&mut game),
            0,
            "tapping an already tapped creature is not a transition"
        );
        let mine = card(&mut game, A, Zone::Battlefield, true);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(mine)),
        );
        assert_eq!(settle(&mut game), 0);
        let artifact = card(&mut game, B, Zone::Battlefield, false);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(artifact)),
        );
        assert_eq!(settle(&mut game), 0);
    }
}

#[test]
fn solitary_sanctuary_qualifies_before_tap_and_targets_its_controllers_creature() {
    for definition in definitions("Solitary Sanctuary") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mine = card(&mut game, A, Zone::Battlefield, true);
        let theirs = card(&mut game, B, Zone::Battlefield, true);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(theirs)),
        );
        assert_eq!(stack(&mut game), 1);
        game.set_current_controller(theirs, A).unwrap();
        settle(&mut game);
        assert_eq!(
            game.object(mine)
                .unwrap()
                .counters
                .get(&ironsmith::object::CounterType::PlusOnePlusOne),
            Some(&1)
        );
        assert!(game.object(theirs).unwrap().counters.is_empty());
    }
}

#[test]
fn sharae_groups_all_qualifying_recipients_and_limits_the_ability_once_per_turn() {
    for definition in definitions("Sharae of Numbing Depths") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        card(&mut game, A, Zone::Library, false);
        let one = card(&mut game, B, Zone::Battlefield, true);
        let two = card(&mut game, B, Zone::Battlefield, true);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::all(ObjectFilter::creature().opponent_controls()),
        );
        assert_eq!(settle(&mut game), 1);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        game.untap(one);
        game.untap(two);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::all(ObjectFilter::creature().opponent_controls()),
        );
        assert_eq!(settle(&mut game), 0);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}

#[test]
fn authored_target_player_and_iterated_actor_do_not_inherit_the_effect_controller() {
    let sentry = definitions("Icewrought Sentry");
    let actor_spell = definitions_from(
        "Directed tap",
        "Type: Instant\nTarget opponent taps target creature.",
    );
    let iterated_spell = definitions_from(
        "Iterated tap",
        "Type: Instant\nFor each opponent, that player taps target creature.",
    );
    for path in 0..2 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&sentry[path], A, Zone::Battlefield);
        let subject = card(&mut game, B, Zone::Battlefield, true);
        let spell = game.create_object_from_definition(&actor_spell[path], B, Zone::Stack);
        let mut ctx = EffectContext::new_default(spell, B).with_targets(vec![
            ResolvedTarget::Player(A),
            ResolvedTarget::Object(subject),
        ]);
        for effect in actor_spell[path]
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            let outcome = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            for event in outcome.events {
                game.queue_trigger_event(Default::default(), event);
            }
        }
        assert_eq!(
            settle(&mut game),
            1,
            "the target player A performed B's instruction"
        );
        game.untap(subject);
        let loop_source = game.create_object_from_definition(&iterated_spell[path], B, Zone::Stack);
        let mut ctx = EffectContext::new_default(loop_source, B)
            .with_targets(vec![ResolvedTarget::Object(subject)]);
        for effect in iterated_spell[path]
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            let outcome = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            for event in outcome.events {
                game.queue_trigger_event(Default::default(), event);
            }
        }
        assert_eq!(
            settle(&mut game),
            1,
            "the authored player-loop actor survives without rebinding the controller"
        );
        assert_eq!(ctx.controller, B);
        assert!(game.object(source).is_some());
    }
}

#[test]
fn calendar_counts_only_completed_own_step_untaps_after_events_wait_until_upkeep() {
    for definition in definitions("The Millennium Calendar") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = card(&mut game, A, Zone::Battlefield, false);
        let two = card(&mut game, A, Zone::Battlefield, false);
        let stunned = card(&mut game, A, Zone::Battlefield, false);
        let phased = card(&mut game, A, Zone::Battlefield, false);
        let other = card(&mut game, B, Zone::Battlefield, false);
        for id in [one, two, stunned, phased, other] {
            game.tap(id);
        }
        game.add_counters(stunned, ironsmith::object::CounterType::Stun, 1)
            .unwrap();
        game.phase_out(phased);
        game.next_turn();
        game.next_turn();
        game.turn.active_player = A;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Untap);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.is_tapped(stunned) && game.is_tapped(other));
        // The phased permanent returns at this untap step and also untaps.
        assert!(!game.is_tapped(one) && !game.is_tapped(two));
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        assert_eq!(
            settle(&mut game),
            1,
            "one turn-based untap event, not one per permanent"
        );
        assert_eq!(time(&game, source), 3);
        game.tap(one);
        execute(
            &mut game,
            source,
            A,
            &UntapEffect::with_spec(ChooseSpec::SpecificObject(one)),
        );
        assert_eq!(
            settle(&mut game),
            0,
            "an effect during upkeep isn't during the untap step"
        );
        assert_eq!(time(&game, source), 3);
        let mut seedborn =
            compile_to_runtime_definition("Other-step untapper", "Type: Enchantment", false)
                .unwrap();
        seedborn.abilities.push(ironsmith::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::untap_during_each_other_players_untap_step(
                ObjectFilter::permanent().you_control(),
                "Untap all permanents you control during each other player's untap step.".into(),
            ),
        ));
        game.create_object_from_definition(&seedborn, A, Zone::Battlefield);
        game.turn.active_player = B;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Untap);
        game.tap(one);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(
            !game.is_tapped(one),
            "the actual other-player untap rule performed the action"
        );
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        assert_eq!(
            settle(&mut game),
            0,
            "your action during somebody else's step does not qualify"
        );
    }
}

#[test]
fn calendar_state_threshold_sacrifices_and_each_opponent_loses_the_full_thousand() {
    use ironsmith::game_loop::check_and_apply_sbas_with;
    use ironsmith::object::CounterType;
    for definition in definitions("The Millennium Calendar") {
        for source_leaves_before_resolution in [false, true] {
            let mut game =
                GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 2000);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let stable = game.object(source).unwrap().stable_id;
            let mut queue = TriggerQueue::new();
            let mut dm = SelectFirstDecisionMaker;
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
                if triggered.trigger.compiled_model().is_some_and(|model| matches!(&model.kind, TriggerKind::StateBased { .. })))));
            game.add_counters(source, CounterType::Time, 999).unwrap();
            check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
            assert!(
                queue.entries.is_empty(),
                "999 is below the authored threshold"
            );
            game.add_counters(source, CounterType::Time, 1).unwrap();
            check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(queue.entries.len(), 1);
            check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(
                queue.entries.len(),
                1,
                "a pending state trigger cannot duplicate"
            );
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1);
            check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
            assert!(
                queue.entries.is_empty(),
                "an on-stack state trigger cannot duplicate"
            );
            if source_leaves_before_resolution {
                game.move_object(
                    source,
                    Zone::Exile,
                    ironsmith::events::cause::EventCause::effect(),
                )
                .unwrap();
            } else {
                game.remove_counters(source, CounterType::Time, 1, None, None)
                    .unwrap();
                assert_eq!(time(&game, source), 999);
            }
            // The threshold is a state trigger, not an intervening-if gate.
            // Nor is the loss conditional on a successful sacrifice: the
            // authored body has ordinary "and", not "when you do".
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(A).unwrap().life, 2000);
            assert_eq!(game.player(B).unwrap().life, 1000);
            assert_eq!(game.player(PlayerId(2)).unwrap().life, 1000);
            assert!(!game.battlefield.contains(&source));
            let destination = if source_leaves_before_resolution {
                Zone::Exile
            } else {
                Zone::Graveyard
            };
            assert_eq!(
                game.find_object_by_stable_id(stable)
                    .and_then(|id| game.object(id))
                    .map(|object| object.zone),
                Some(destination)
            );
            check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
            assert!(queue.entries.is_empty());
        }
    }
}

#[test]
fn calendar_doubling_is_a_paid_tap_activation() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    for definition in definitions("The Millennium Calendar") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(A);
        game.add_counters(source, ironsmith::object::CounterType::Time, 2)
            .unwrap();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 2);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        let mut dm = SelectFirstDecisionMaker;
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..30 {
            if state.pending_activation.is_none() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("activation did not finish payment")
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert!(state.pending_activation.is_none());
        assert_eq!(game.stack.len(), 1);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(time(&game, source), 4);
    }
}

#[test]
fn both_convoke_payment_routes_publish_one_group_with_the_actual_payer() {
    // Official MTGO rules correction: multiple convoke taps produce one
    // Deeproot Pilgrimage trigger, not one per pip.
    // https://www.mtgo.com/news/mtgo-blog-12192023
    let row: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/passive_tap_state_triggers.json.fixture"
    ))
    .unwrap();
    let pilgrimage = row
        .iter()
        .find(|row| row["name"] == "Deeproot Pilgrimage")
        .unwrap();
    let observers = definitions_from("Deeproot Pilgrimage", pilgrimage["text"].as_str().unwrap());
    let spells = definitions_from(
        "Convoke payment probe",
        "Mana cost: {2}\nType: Sorcery\nConvoke\nYou gain 1 life.",
    );
    for path in 0..2 {
        for priority_path in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            game.create_object_from_definition(&observers[path], A, Zone::Battlefield);
            let merfolk = compile_to_runtime_definition(
                "Payment Merfolk",
                "Type: Creature — Merfolk\nPower/Toughness: 1/1",
                false,
            )
            .unwrap();
            let one = game.create_object_from_definition(&merfolk, A, Zone::Battlefield);
            let two = game.create_object_from_definition(&merfolk, A, Zone::Battlefield);
            if priority_path {
                use ironsmith::game_loop::{
                    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
                    apply_priority_response_with_dm,
                };
                let hand = game.create_object_from_definition(&spells[path], A, Zone::Hand);
                let action = ironsmith::decision::LegalAction::CastSpell {
                    spell_id: hand,
                    from_zone: Zone::Hand,
                    casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
                };
                assert!(
                    ironsmith::decision::compute_legal_actions(&game, A)
                        .unwrap()
                        .contains(&action)
                );
                let mut state = PriorityLoopState::new(2);
                let mut queue = TriggerQueue::new();
                let mut dm = SelectFirstDecisionMaker;
                let mut progress = apply_priority_response_with_dm(
                    &mut game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::PriorityAction(action),
                    &mut dm,
                )
                .unwrap();
                for _ in 0..30 {
                    if state.pending_cast.is_none() && !game.stack_is_empty() {
                        break;
                    }
                    if let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress {
                        progress = apply_decision_context_with_dm(
                            &mut game, &mut queue, &mut state, &ctx, &mut dm,
                        )
                        .unwrap();
                    } else {
                        break;
                    }
                }
                assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                settle(&mut game);
                assert_eq!(
                    game.player(A).unwrap().life,
                    21,
                    "the real spell also resolved"
                );
            } else {
                use ironsmith::mana_payment::{
                    ManaPaymentExecution, ManaPaymentRequest, execute_mana_payment_plan,
                    plan_first_mana_payment,
                };
                let spell = game.create_object_from_definition(&spells[path], A, Zone::Stack);
                let request = ManaPaymentRequest::new(
                    A,
                    spell,
                    ironsmith::costs::PaymentReason::CastSpell,
                    ironsmith::mana::ManaCost::new().add_generic(2),
                )
                .with_spend_policy(game.mana_spend_policy(A, Some(spell)));
                let plan = plan_first_mana_payment(&game, &request).unwrap();
                assert_eq!(
                    execute_mana_payment_plan(
                        &mut game,
                        &request,
                        &plan,
                        &mut SelectFirstDecisionMaker
                    ),
                    Ok(ManaPaymentExecution::Paid)
                );
                let events = game.take_pending_trigger_events();
                let taps = events
                    .iter()
                    .filter_map(|event| event.downcast::<ironsmith::events::PermanentTappedEvent>())
                    .collect::<Vec<_>>();
                assert_eq!(taps.len(), 2);
                assert!(taps.iter().all(|tap| {
                    tap.actor == Some(A)
                        && tap
                            .before_snapshot
                            .as_ref()
                            .is_some_and(|before| !before.tapped)
                }));
                for event in events {
                    game.queue_trigger_event(event.provenance(), event);
                }
                assert_eq!(settle(&mut game), 1);
            }
            assert!(game.is_tapped(one) && game.is_tapped(two));
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter(|id| game.object(**id).unwrap().kind
                        == ironsmith::object::ObjectKind::Token)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn distinct_tap_instructions_inside_one_cost_container_remain_distinct_events() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/passive_tap_state_triggers.json.fixture"
    ))
    .unwrap();
    let row = rows
        .iter()
        .find(|row| row["name"] == "Deeproot Pilgrimage")
        .unwrap();
    for definition in definitions_from("Deeproot Pilgrimage", row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let merfolk = compile_to_runtime_definition(
            "Separate cost Merfolk",
            "Type: Creature — Merfolk\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        let one = game.create_object_from_definition(&merfolk, A, Zone::Battlefield);
        let two = game.create_object_from_definition(&merfolk, A, Zone::Battlefield);
        let sequence = ironsmith::effects::SequenceEffect::new(vec![
            ironsmith::Effect::new(TapEffect::with_spec(ChooseSpec::SpecificObject(one))),
            ironsmith::Effect::new(TapEffect::with_spec(ChooseSpec::SpecificObject(two))),
        ]);
        let cost = ironsmith::costs::Cost::try_effect(ironsmith::Effect::new(sequence)).unwrap();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ironsmith::costs::CostContext::new(source, A, &mut dm);
        assert_eq!(
            cost.pay(&mut game, &mut ctx).unwrap(),
            ironsmith::costs::CostPaymentResult::Paid
        );
        assert_eq!(
            settle(&mut game),
            2,
            "a cost wrapper doesn't collapse its sequential instructions"
        );
    }
}

#[test]
fn simultaneous_tap_keeps_the_common_before_state_and_completed_after_state() {
    for definition in definitions("Icewrought Sentry") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let observer = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = card(&mut game, A, Zone::Battlefield, false);
        let vehicle = compile_to_runtime_definition(
            "Conditional recipient",
            "Type: Artifact — Vehicle\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        let recipient = game.create_object_from_definition(&vehicle, B, Zone::Battlefield);
        let animation = ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(recipient),
            ironsmith::continuous::Modification::AddCardTypes(vec![ironsmith::CardType::Creature]),
            ironsmith::effect::Until::ThisLeavesTheBattlefield,
        )
        .with_condition(ironsmith::ConditionExpr::SourceIsTapped);
        execute(&mut game, source, A, &animation);
        assert!(
            !game
                .current_card_types(recipient)
                .unwrap()
                .contains(&ironsmith::CardType::Creature)
        );
        let mut ctx = EffectContext::new_default(source, A);
        let outcome = TapEffect::all(ObjectFilter::artifact())
            .execute(&mut game, &mut ctx)
            .unwrap();
        let event = outcome
            .events
            .iter()
            .find_map(|event| {
                event
                    .downcast::<ironsmith::events::PermanentTappedEvent>()
                    .filter(|event| event.permanent == recipient)
            })
            .unwrap();
        assert!(
            !event
                .before_snapshot
                .as_ref()
                .unwrap()
                .card_types
                .contains(&ironsmith::CardType::Creature)
        );
        assert!(
            event
                .snapshot
                .as_ref()
                .unwrap()
                .card_types
                .contains(&ironsmith::CardType::Creature)
        );
        let power = game.current_power(observer);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(
            settle(&mut game),
            0,
            "the earlier member's type change cannot rewrite the shared pre-action filter"
        );
        assert_eq!(game.current_power(observer), power);
    }
}
