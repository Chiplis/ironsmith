//! Exact-card source proposals; all scenarios are authored but unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::effects::{
    DestroyEffect, DrawCardsEffect, EffectContext, EffectExecutor, GainLifeEffect,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Step, Target};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::PlayerFilter;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const D: PlayerId = PlayerId(3);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/qualified_player_draws.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn resource(game: &mut GameState, player: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Draw resource", text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn library(game: &mut GameState, player: PlayerId, n: usize) {
    for _ in 0..n {
        resource(
            game,
            player,
            Zone::Library,
            "Mana cost: {9}\nType: Artifact",
        );
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: &dyn EffectExecutor) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: i32) {
    apply(
        game,
        source,
        &DrawCardsEffect::new(count, PlayerFilter::Specific(player)),
    );
}
fn pending(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) {
    pending(game, dm);
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        pending(game, dm);
    }
    panic!("draw program did not settle");
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    decline: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        !self.decline && ctx.can_accept
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.targets
            .iter()
            .copied()
            .filter(|target| {
                ctx.requirements
                    .iter()
                    .any(|req| req.legal_targets.contains(target))
            })
            .collect()
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut impl DecisionMaker) {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    for mana in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(mana, 20);
    }
    game.turn.priority_player = Some(A);
    let action = ironsmith::decision::compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).expect("real cast is legal");
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
    for _ in 0..30 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("missing cast choice");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    settle(game, dm);
}
fn battlefield(game: &GameState, name: &str, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == name && game.controller_of(object) == controller
            })
        })
        .collect()
}
#[test]
fn four_exact_cards_have_lossless_direct_and_artifact_programs() {
    assert_eq!(rows().len(), 4);
    for row in rows() {
        definitions(row["name"].as_str().unwrap());
    }
}
#[test]
fn possession_tracks_the_enchanted_opponent_and_optional_draw_but_skips_its_own_draw_step() {
    for definition in definitions("Psychic Possession") {
        let mut game = game();
        for player in [A, B, C] {
            library(&mut game, player, 12);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "Psychic Possession", A)[0];
        draw(&mut game, source, C, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        draw(&mut game, source, B, 2);
        assert_eq!(pending(&mut game, &mut dm), 2);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        dm.decline = true;
        draw(&mut game, source, B, 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        assert!(ironsmith::turn::execute_draw_step_with(&mut game, &mut dm).unwrap().is_empty());
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
    }
}
#[test]
fn watcher_enters_with_stun_and_qualifies_opponent_turns_then_resolves_both_death_targets() {
    for definition in definitions("The Watcher in the Water") {
        let mut game = game();
        game.set_teams(vec![vec![A, B], vec![C, D]]).unwrap();
        library(&mut game, A, 12);
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "The Watcher in the Water", A)[0];
        assert!(game.is_tapped(source));
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&CounterType::Stun),
            Some(&9)
        );
        for active in [A, B] {
            game.turn.active_player = active;
            draw(&mut game, source, A, 1);
            assert_eq!(pending(&mut game, &mut dm), 0);
        }
        game.turn.active_player = C;
        draw(&mut game, source, A, 2);
        assert_eq!(pending(&mut game, &mut dm), 2);
        settle(&mut game, &mut dm);
        let tentacles = battlefield(&game, "Tentacle", A);
        assert_eq!(tentacles.len(), 2);
        let kraken = resource(
            &mut game,
            A,
            Zone::Battlefield,
            "Type: Creature — Kraken\nPower/Toughness: 4/4",
        );
        game.tap(kraken);
        let artifact = resource(&mut game, C, Zone::Battlefield, "Type: Artifact");
        dm.targets = vec![Target::Object(kraken), Target::Object(artifact)];
        apply(
            &mut game,
            source,
            &DestroyEffect::with_spec(ChooseSpec::SpecificObject(tentacles[0])),
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert!(!game.is_tapped(kraken));
        assert_eq!(
            game.object(artifact)
                .unwrap()
                .counters
                .get(&CounterType::Stun),
            Some(&1)
        );
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&CounterType::Stun),
            Some(&9)
        );
    }
}
#[test]
fn wiretapping_hideaway_free_play_and_first_card_are_scoped_to_each_actual_draw_step() {
    for definition in definitions("Wiretapping") {
        let mut game = game();
        library(&mut game, A, 24);
        let mut dm = SelectFirstDecisionMaker;
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "Wiretapping", A)[0];
        assert_eq!(game.get_exiled_with_source_links(source).len(), 1);
        game.empty_mana_pools();
        for _ in 0..8 {
            resource(&mut game, A, Zone::Hand, "Type: Land");
        }
        // An earlier draw in upkeep must not consume the first draw of the draw step.
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        draw(&mut game, source, A, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.turn.step = Some(Step::Draw);
        for event in ironsmith::turn::execute_draw_step_with(&mut game, &mut dm).unwrap() {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(
            battlefield(&game, "Draw resource", A).len(),
            1,
            "the hidden nine-mana artifact was played with no mana available"
        );
        assert_eq!(game.draw_step_context_for_player(A), (true, 2));
        draw(&mut game, source, A, 2);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.add_step_after(Step::Draw, Step::Draw);
        ironsmith::turn::advance_step(&mut game).unwrap();
        for event in ironsmith::turn::execute_draw_step_with(&mut game, &mut dm).unwrap() {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.draw_step_context_for_player(A), (true, 2));
    }
}
#[test]
fn wedding_ring_copy_and_qualified_events_capture_participant_and_amount_before_resolution() {
    for definition in definitions("Wedding Ring") {
        let mut game = game();
        for player in [A, B, C] {
            library(&mut game, player, 12);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "Wedding Ring", A)[0];
        let partner = battlefield(&game, "Wedding Ring", B)[0];
        assert!(matches!(
            game.object(partner).unwrap().kind,
            ironsmith::object::ObjectKind::Token
        ));
        game.turn.active_player = C;
        draw(&mut game, source, C, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.turn.active_player = A;
        draw(&mut game, source, B, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.turn.active_player = B;
        apply(
            &mut game,
            source,
            &GainLifeEffect::with_filter(4, PlayerFilter::Specific(B)),
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 24);
        draw(&mut game, source, B, 2);
        assert_eq!(pending(&mut game, &mut dm), 2);
        apply(
            &mut game,
            source,
            &DestroyEffect::with_spec(ChooseSpec::SpecificObject(partner)),
        );
        settle(&mut game, &mut dm);
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            2,
            "who-controls is an event qualification, not an intervening-if"
        );
    }
}

fn ring_pair(definition: &CardDefinition) -> (GameState, ObjectId, ObjectId, ObjectId) {
    let mut game = game();
    game.turn.active_player = B;
    library(&mut game, A, 12);
    library(&mut game, B, 12);
    let ring = game.create_object_from_definition(definition, A, Zone::Battlefield);
    let partner = game.create_object_from_definition(definition, B, Zone::Battlefield);
    let replacer = resource(&mut game, B, Zone::Battlefield, "Type: Artifact");
    (game, ring, partner, replacer)
}
fn gain_addition(game: &mut GameState, source: ObjectId, effects: Vec<ironsmith::Effect>) {
    game.effect_store.replacement_effects.add_resolution_effect(
        ironsmith::replacement::ReplacementEffect::with_matcher(
            source,
            B,
            ironsmith::events::WouldGainLifeMatcher::you(),
            ironsmith::replacement::ReplacementAction::Additionally(effects),
        ),
    );
}
fn remove_partner(partner: ObjectId) -> ironsmith::Effect {
    ironsmith::Effect::new(DestroyEffect::with_spec(ChooseSpec::SpecificObject(
        partner,
    )))
}
#[test]
fn original_draw_and_life_receipts_capture_qualification_before_replacement_removes_partner() {
    for definition in definitions("Wedding Ring") {
        for life in [false, true] {
            let (mut game, source, partner, replacer) = ring_pair(&definition);
            if life {
                gain_addition(&mut game, replacer, vec![remove_partner(partner)]);
            } else {
                game.effect_store.replacement_effects.add_resolution_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(
                        replacer,
                        B,
                        ironsmith::events::WouldDrawCardMatcher::you(),
                        ironsmith::replacement::ReplacementAction::Additionally(vec![
                            remove_partner(partner),
                        ]),
                    ),
                );
            }
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = EffectContext::new(replacer, B, &mut dm);
            let outcome = if life {
                GainLifeEffect::you(3).execute(&mut game, &mut ctx)
            } else {
                DrawCardsEffect::you(1).execute(&mut game, &mut ctx)
            }
            .unwrap();
            assert!(game.object(partner).is_none());
            assert_eq!(
                outcome
                    .events
                    .iter()
                    .filter(|event| if life {
                        event
                            .downcast::<ironsmith::events::LifeGainEvent>()
                            .is_some()
                    } else {
                        event
                            .downcast::<ironsmith::events::CardsDrawnEvent>()
                            .is_some()
                    })
                    .count(),
                1,
                "capture preserves original physical event evidence"
            );
            for event in outcome.events {
                game.queue_trigger_event(event.provenance(), event);
            }
            assert_eq!(
                pending(&mut game, &mut dm),
                1,
                "later publication must not erase or duplicate the captured trigger"
            );
            settle(&mut game, &mut dm);
            if life {
                assert_eq!(game.player(A).unwrap().life, 23);
            } else {
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
            }
            assert!(game.object(source).is_some());
        }
    }
}
#[test]
fn replacement_nested_program_captures_its_draw_before_a_later_added_instruction() {
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, replacer) = ring_pair(&definition);
        gain_addition(
            &mut game,
            replacer,
            vec![ironsmith::Effect::new(
                ironsmith::effects::SequenceEffect::new(vec![
                    ironsmith::Effect::new(DrawCardsEffect::you(1)),
                    remove_partner(partner),
                ]),
            )],
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(replacer, B, &mut dm);
        let outcome = GainLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(pending(&mut game, &mut dm), 2);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 23);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}
#[test]
fn qualification_absent_at_original_event_is_not_created_retroactively_by_addition() {
    for definition in definitions("Wedding Ring") {
        let mut game = game();
        game.turn.active_player = B;
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let later_partner = game.create_object_from_definition(&definition, B, Zone::Hand);
        let replacer = resource(&mut game, B, Zone::Battlefield, "Type: Artifact");
        gain_addition(
            &mut game,
            replacer,
            vec![ironsmith::Effect::new(
                ironsmith::effects::PutOntoBattlefieldEffect::new(
                    ChooseSpec::SpecificObject(later_partner),
                    false,
                    PlayerFilter::You,
                ),
            )],
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(replacer, B, &mut dm);
        let outcome = GainLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(battlefield(&game, "Wedding Ring", B).len(), 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}
#[test]
fn effect_backed_cost_captures_original_gain_before_added_program_and_retains_payment_evidence() {
    use ironsmith::costs::CostPayer;
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, replacer) = ring_pair(&definition);
        gain_addition(&mut game, replacer, vec![remove_partner(partner)]);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ironsmith::costs::CostContext::new(replacer, B, &mut dm);
        ironsmith::costs::CostEffect::new(GainLifeEffect::you(3))
            .pay(&mut game, &mut ctx)
            .unwrap();
        assert!(game.object(partner).is_none());
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 23);
    }
}
#[test]
fn simultaneous_combat_lifelink_captures_both_original_gains_before_first_addition() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, replacer) = ring_pair(&definition);
        gain_addition(&mut game, replacer, vec![remove_partner(partner)]);
        let attackers: Vec<_> = (0..2)
            .map(|_| {
                resource(
                    &mut game,
                    B,
                    Zone::Battlefield,
                    "Type: Creature\nPower/Toughness: 2/2\nLifelink",
                )
            })
            .collect();
        let combat = CombatState {
            attackers: attackers
                .into_iter()
                .map(|creature| AttackerInfo {
                    creature,
                    target: AttackTarget::Player(C),
                })
                .collect(),
            ..Default::default()
        };
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::CombatDamage);
        let mut dm = SelectFirstDecisionMaker;
        let events = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
            &mut game, &combat, false, &mut dm,
        )
        .unwrap();
        assert_eq!(game.player(B).unwrap().life, 24);
        assert_eq!(game.player(C).unwrap().life, 16);
        assert_eq!(
            events
                .iter()
                .flat_map(|event| event.lifelink_outcome.iter())
                .flat_map(|outcome| &outcome.events)
                .filter(|event| event
                    .downcast::<ironsmith::events::LifeGainEvent>()
                    .is_some())
                .count(),
            2
        );
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::queue_combat_damage_triggers(&mut game, &events, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 2);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 24);
    }
}

#[test]
fn receipt_capture_matches_delayed_listeners_once_and_keeps_them_for_later_occurrences() {
    for definition in definitions("Wedding Ring") {
        let (mut game, source, partner, replacer) = ring_pair(&definition);
        apply(
            &mut game,
            source,
            &ironsmith::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith::triggers::Trigger::from_model(
                    ironsmith_core::trigger_model::Trigger::player_gains_life(
                        PlayerFilter::Specific(B),
                        None,
                    ),
                )
                .unwrap(),
                vec![ironsmith::Effect::new(GainLifeEffect::you(1))],
                false,
                vec![],
                PlayerFilter::You,
            )
            .until_end_of_turn(),
        );
        gain_addition(&mut game, replacer, vec![remove_partner(partner)]);
        apply(
            &mut game,
            replacer,
            &GainLifeEffect::with_filter(3, PlayerFilter::Specific(B)),
        );
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(pending(&mut game, &mut dm), 2);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 24);
        apply(
            &mut game,
            replacer,
            &GainLifeEffect::with_filter(2, PlayerFilter::Specific(B)),
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 25);
    }
}
#[test]
fn pending_added_program_rolls_back_original_gain_and_captured_triggers_before_replay() {
    struct Pause {
        waiting: bool,
    }
    impl DecisionMaker for Pause {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            self.waiting = true;
            false
        }
        fn awaiting_choice(&self) -> bool {
            self.waiting
        }
    }
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, replacer) = ring_pair(&definition);
        gain_addition(
            &mut game,
            replacer,
            vec![
                ironsmith::Effect::new(ironsmith::effects::MayEffect::new(vec![
                    ironsmith::Effect::new(DrawCardsEffect::you(1)),
                ])),
                remove_partner(partner),
            ],
        );
        let mut pause = Pause { waiting: false };
        let mut ctx = EffectContext::new(replacer, B, &mut pause);
        let outcome = GainLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(outcome.events.is_empty());
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(game.player(B).unwrap().hand.len(), 0);
        assert!(game.object(partner).is_some());
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(pending(&mut game, &mut dm), 0);
        let mut ctx = EffectContext::new(replacer, B, &mut dm);
        let outcome = GainLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(pending(&mut game, &mut dm), 2);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 23);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}

#[test]
fn shared_draw_step_preserves_player_sequence_and_captures_before_the_first_players_addition() {
    // CR 121.2d/121.6b/805.6a: these draws are sequential, unlike simultaneous life changes.
    for definition in definitions("Wedding Ring") {
        let mut game = game();
        game.restore_two_headed_giant(vec![vec![B, A], vec![C, D]], 0, A)
            .unwrap();
        for player in [A, B, C] {
            library(&mut game, player, 12);
        }
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let partner = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.create_object_from_definition(&definition, C, Zone::Battlefield);
        let replacer = resource(&mut game, A, Zone::Battlefield, "Type: Artifact");
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                replacer,
                A,
                ironsmith::events::WouldDrawCardMatcher::you(),
                ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                    partner,
                )]),
            ),
        );
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let mut runner =
            ironsmith::TurnRunner::from_state_for_sync(ironsmith::TurnRunnerState::Draw);
        let mut queue = TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        loop {
            match runner.advance(&mut game, &mut queue).unwrap() {
                ironsmith::TurnAction::RunPriority => break,
                ironsmith::TurnAction::Continue => {}
                ironsmith::TurnAction::Decision(
                    ironsmith::decisions::context::DecisionContext::SelectOptions(context),
                ) => runner.respond_options(dm.decide_options(&game, &context)),
                action => panic!("unexpected shared draw action: {action:?}"),
            }
        }
        assert_eq!(game.draw_step_context_for_player(A), (true, 1));
        assert_eq!(game.draw_step_context_for_player(B), (true, 1));
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(
            game.stack.len(),
            1,
            "B's later draw does not qualify after its artifact was removed by A's replacement"
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.player(C).unwrap().hand.len(), 1);
    }
}

#[test]
fn replacement_created_draw_waits_for_other_original_life_changes_before_removing_qualification() {
    // CR 121.7: the first gain's Instead(draw) payload waits for B's original gain.
    for definition in definitions("Wedding Ring") {
        let mut game = game();
        game.restore_two_headed_giant(vec![vec![B, A], vec![C, D]], 0, A)
            .unwrap();
        library(&mut game, A, 4);
        let observer = game.create_object_from_definition(&definition, C, Zone::Battlefield);
        let partner = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let source = resource(&mut game, C, Zone::Battlefield, "Type: Artifact");
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldGainLifeMatcher::new(PlayerFilter::Specific(A)),
                ironsmith::replacement::ReplacementAction::Instead(vec![ironsmith::Effect::new(
                    DrawCardsEffect::new(1, PlayerFilter::Specific(A)),
                )]),
            ),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldDrawCardMatcher::new(PlayerFilter::Specific(A)),
                ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                    partner,
                )]),
            ),
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, C, &mut dm);
        let outcome = ironsmith::effects::ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![ironsmith::Effect::new(GainLifeEffect::with_filter(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(
            pending(&mut game, &mut dm),
            1,
            "B's unreplaced original gain precedes A's replacement-created draw and its artifact removal"
        );
        assert_eq!(game.stack.last().unwrap().object_id, observer);
    }
}

fn simultaneous_replacement_fixture(
    definition: &CardDefinition,
) -> (GameState, ObjectId, ObjectId, ObjectId) {
    let mut game = game();
    game.restore_two_headed_giant(vec![vec![B, A], vec![C, D]], 0, A)
        .unwrap();
    library(&mut game, A, 4);
    let observer = game.create_object_from_definition(definition, C, Zone::Battlefield);
    let partner = game.create_object_from_definition(definition, B, Zone::Battlefield);
    let source = resource(&mut game, C, Zone::Battlefield, "Type: Artifact");
    (game, observer, partner, source)
}
fn replace_first_gain(game: &mut GameState, source: ObjectId, effects: Vec<ironsmith::Effect>) {
    game.effect_store.replacement_effects.add_resolution_effect(
        ironsmith::replacement::ReplacementEffect::with_matcher(
            source,
            C,
            ironsmith::events::WouldGainLifeMatcher::new(PlayerFilter::Specific(A)),
            ironsmith::replacement::ReplacementAction::Instead(effects),
        ),
    );
}
fn shared_gain(
    game: &mut GameState,
    source: ObjectId,
    dm: &mut impl DecisionMaker,
) -> ironsmith::effect::EffectOutcome {
    let mut ctx = EffectContext::new(source, C, dm);
    ironsmith::effects::ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![ironsmith::Effect::new(GainLifeEffect::with_filter(
            1,
            PlayerFilter::IteratedPlayer,
        ))],
    )
    .execute(game, &mut ctx)
    .unwrap()
}
#[test]
fn non_draw_prefix_stays_before_remaining_originals_instead_of_deferring_the_whole_payload() {
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        replace_first_gain(
            &mut game,
            source,
            vec![
                remove_partner(partner),
                ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
            ],
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert!(game.object(partner).is_none());
        assert_eq!(
            pending(&mut game, &mut dm),
            0,
            "the non-draw prefix already removed B's qualification before B's gain"
        );
    }
}
#[test]
fn draw_continuation_keeps_local_tags_result_ids_controller_and_departed_source_snapshot() {
    use ironsmith::effect::{EffectId, Value};
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        replace_first_gain(
            &mut game,
            source,
            vec![
                ironsmith::Effect::new(ironsmith::effects::TagMatchingObjectsEffect::new(
                    ironsmith_core::ObjectFilter::specific(partner),
                    "saved_partner",
                )),
                ironsmith::Effect::new(ironsmith::effects::WithIdEffect::new(
                    EffectId(77),
                    ironsmith::Effect::new(GainLifeEffect::you(2)),
                )),
                remove_partner(source),
                ironsmith::Effect::new(ironsmith::effects::WithIdEffect::new(
                    EffectId(78),
                    ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
                )),
                ironsmith::Effect::new(GainLifeEffect::with_filter(
                    Value::Add(
                        Box::new(Value::EffectValue(EffectId(77))),
                        Box::new(Value::EffectValue(EffectId(78))),
                    ),
                    PlayerFilter::Specific(D),
                )),
                ironsmith::Effect::new(DestroyEffect::with_spec(ChooseSpec::tagged(
                    "saved_partner",
                ))),
            ],
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert!(game.object(source).is_none());
        assert!(game.object(partner).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(
            game.player(C).unwrap().life,
            27,
            "two prefix life, two other originals, and three suffix life use the retained replacement controller and result frame"
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(C).unwrap().life, 28);
    }
}
#[test]
fn suspended_draw_suffix_rolls_back_prefix_and_every_other_original_before_replay() {
    struct Pause(bool);
    impl DecisionMaker for Pause {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            self.0 = true;
            false
        }
        fn awaiting_choice(&self) -> bool {
            self.0
        }
    }
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        replace_first_gain(
            &mut game,
            source,
            vec![
                ironsmith::Effect::new(GainLifeEffect::you(2)),
                ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
                ironsmith::Effect::new(ironsmith::effects::MayEffect::new(vec![remove_partner(
                    partner,
                )])),
            ],
        );
        let mut pause = Pause(false);
        let outcome = shared_gain(&mut game, source, &mut pause);
        assert!(pause.0);
        assert!(outcome.events.is_empty());
        for player in [A, B, C, D] {
            assert_eq!(game.player(player).unwrap().life, 20);
        }
        assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.object(partner).is_some());
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(pending(&mut game, &mut dm), 0);
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert!(game.object(partner).is_none());
        assert_eq!(pending(&mut game, &mut dm), 1);
    }
}
#[test]
fn conditional_draw_inside_original_replacement_waits_for_remaining_life_originals() {
    // The optional branch is selected before the other originals, while its
    // draw and removal resume afterward.
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        replace_first_gain(
            &mut game,
            source,
            vec![ironsmith::Effect::new(ironsmith::effects::MayEffect::new(
                vec![
                    ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
                    remove_partner(partner),
                ],
            ))],
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(
            pending(&mut game, &mut dm),
            1,
            "the accepted conditional draw must not run before B's original gain"
        );
    }
}

#[test]
fn conditional_optional_scope_is_selected_once_before_other_originals_and_resumed_without_reasking()
{
    struct ObserveChoice {
        life: Vec<i32>,
        accept: bool,
    }
    impl DecisionMaker for ObserveChoice {
        fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
            self.life.push(game.player(C).unwrap().life);
            self.accept && ctx.can_accept
        }
    }
    for definition in definitions("Wedding Ring") {
        for accept in [false, true] {
            let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
            let branch = ironsmith::effects::ConditionalEffect::new(
                ironsmith::ConditionExpr::LifeTotalOrLess(20),
                vec![ironsmith::Effect::new(ironsmith::effects::MayEffect::new(
                    vec![
                        ironsmith::Effect::new(GainLifeEffect::you(2)),
                        ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
                        remove_partner(partner),
                    ],
                ))],
                vec![ironsmith::Effect::new(GainLifeEffect::you(9))],
            );
            let id = ironsmith::effect::EffectId(85);
            replace_first_gain(
                &mut game,
                source,
                vec![
                    ironsmith::Effect::new(ironsmith::effects::WithIdEffect::new(
                        id,
                        ironsmith::Effect::new(branch),
                    )),
                    ironsmith::Effect::new(ironsmith::effects::IfEffect::if_then(
                        id,
                        ironsmith::effect::EffectPredicate::Happened,
                        vec![ironsmith::Effect::new(GainLifeEffect::with_filter(
                            1,
                            PlayerFilter::Specific(D),
                        ))],
                    )),
                ],
            );
            let mut dm = ObserveChoice {
                life: vec![],
                accept,
            };
            let outcome = shared_gain(&mut game, source, &mut dm);
            for event in outcome.events {
                game.queue_trigger_event(event.provenance(), event);
            }
            assert_eq!(
                dm.life,
                vec![20],
                "the selected condition and May choice are not rerun after the other originals"
            );
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(accept));
            assert_eq!(game.object(partner).is_none(), accept);
            assert_eq!(game.player(C).unwrap().life, if accept { 25 } else { 22 });
            assert_eq!(pending(&mut game, &mut dm), 1);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(C).unwrap().life, if accept { 26 } else { 23 });
        }
    }
}

#[test]
fn whole_program_tag_scope_finishes_after_deferred_draw_and_keeps_the_drawn_incarnation() {
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        let selected = ironsmith::effects::ConditionalEffect::new(
            ironsmith::ConditionExpr::LifeTotalOrLess(20),
            vec![
                ironsmith::Effect::new(GainLifeEffect::you(2)),
                ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
                remove_partner(partner),
            ],
            vec![],
        );
        replace_first_gain(
            &mut game,
            source,
            vec![
                ironsmith::Effect::new(ironsmith::effects::TaggedEffect::new(
                    "draw_receipt",
                    ironsmith::Effect::new(selected),
                )),
                ironsmith::Effect::new(ironsmith::effects::ExileEffect::with_spec(
                    ChooseSpec::tagged("draw_receipt"),
                )),
            ],
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(
            game.exile
                .iter()
                .filter(|id| game
                    .object(**id)
                    .is_some_and(|object| object.owner == A && object.name == "Draw resource"))
                .count(),
            1
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
    }
}

#[test]
fn zero_draw_has_no_continuation_boundary_and_keeps_its_suffix_in_the_original() {
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        replace_first_gain(
            &mut game,
            source,
            vec![ironsmith::Effect::new(ironsmith::effects::MayEffect::new(
                vec![
                    ironsmith::Effect::new(DrawCardsEffect::new(0, PlayerFilter::Specific(A))),
                    remove_partner(partner),
                ],
            ))],
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.object(partner).is_none());
        assert_eq!(pending(&mut game, &mut dm), 0);
    }
}

#[test]
fn quantified_replacement_draw_prefix_keeps_remaining_original_life_changes_ahead_of_draws() {
    // A nested ForPlayers retains its native per-player program state rather
    // than running the entire iteration before the other originals.
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        replace_first_gain(
            &mut game,
            source,
            vec![ironsmith::Effect::new(
                ironsmith::effects::ForPlayersEffect::new(
                    PlayerFilter::Specific(A),
                    vec![
                        ironsmith::Effect::new(GainLifeEffect::with_filter(
                            2,
                            PlayerFilter::Specific(C),
                        )),
                        ironsmith::Effect::new(DrawCardsEffect::new(
                            1,
                            PlayerFilter::IteratedPlayer,
                        )),
                        remove_partner(partner),
                    ],
                ),
            )],
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(
            pending(&mut game, &mut dm),
            1,
            "the nested player program's draw does not precede B's original life gain"
        );
    }
}

#[test]
fn continued_player_program_keeps_each_participants_prior_result_and_action_order() {
    use ironsmith::effect::{EffectId, Value};
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        library(&mut game, B, 6);
        resource(&mut game, A, Zone::Hand, "Type: Land");
        for _ in 0..2 {
            resource(&mut game, B, Zone::Hand, "Type: Land");
        }
        let amount = EffectId(101);
        replace_first_gain(
            &mut game,
            source,
            vec![ironsmith::Effect::new(
                ironsmith::effects::ForPlayersEffect::new(
                    PlayerFilter::Opponent,
                    vec![
                        ironsmith::Effect::new(ironsmith::effects::WithIdEffect::new(
                            amount,
                            ironsmith::Effect::new(GainLifeEffect::with_filter(
                                Value::CardsInHand(PlayerFilter::IteratedPlayer),
                                PlayerFilter::IteratedPlayer,
                            )),
                        )),
                        ironsmith::Effect::new(DrawCardsEffect::new(
                            Value::EffectValue(amount),
                            PlayerFilter::IteratedPlayer,
                        )),
                    ],
                ),
            )],
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldDrawCardMatcher::new(PlayerFilter::Specific(A)),
                ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                    partner,
                )]),
            ),
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            2,
            "A draws its own one-card prefix result"
        );
        assert_eq!(
            game.player(B).unwrap().hand.len(),
            4,
            "B draws its own two-card prefix result"
        );
        assert_eq!(
            pending(&mut game, &mut dm),
            2,
            "B's nested original gain and outer original gain precede A's draw-added removal"
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.player(C).unwrap().life, 25);
    }
}

#[test]
fn optional_player_draw_cursor_retains_first_choice_and_asks_later_players_in_sequence() {
    struct PerPlayerChoice {
        first_accepts: bool,
        observed: Vec<(PlayerId, i32)>,
    }
    impl DecisionMaker for PerPlayerChoice {
        fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
            self.observed
                .push((ctx.player, game.player(C).unwrap().life));
            (ctx.player != A || self.first_accepts) && ctx.can_accept
        }
    }
    for definition in definitions("Wedding Ring") {
        for first_accepts in [false, true] {
            let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
            library(&mut game, B, 4);
            library(&mut game, C, 4);
            replace_first_gain(
                &mut game,
                source,
                vec![ironsmith::Effect::new(
                    ironsmith::effects::ForPlayersEffect::new(
                        PlayerFilter::Opponent,
                        vec![ironsmith::Effect::new(ironsmith::effects::MayEffect::new(
                            vec![
                                ironsmith::Effect::new(DrawCardsEffect::new(
                                    1,
                                    PlayerFilter::IteratedPlayer,
                                )),
                                remove_partner(partner),
                            ],
                        ))],
                    ),
                )],
            );
            game.effect_store.replacement_effects.add_resolution_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    source,
                    C,
                    ironsmith::events::WouldDrawCardMatcher::new(PlayerFilter::Specific(A)),
                    ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                        partner,
                    )]),
                ),
            );
            let mut dm = PerPlayerChoice {
                first_accepts,
                observed: vec![],
            };
            let outcome = shared_gain(&mut game, source, &mut dm);
            for event in outcome.events {
                game.queue_trigger_event(event.provenance(), event);
            }
            assert_eq!(
                dm.observed,
                vec![(A, 20), (B, if first_accepts { 22 } else { 20 })],
                "pause cannot repeat the first choice or preask later sequential draw choices"
            );
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                usize::from(first_accepts)
            );
            assert_eq!(game.player(B).unwrap().hand.len(), 1);
            assert_eq!(
                pending(&mut game, &mut dm),
                if first_accepts { 1 } else { 2 }
            );
            settle(&mut game, &mut dm);
            assert_eq!(
                game.player(C).unwrap().hand.len(),
                usize::from(!first_accepts),
                "B's draw is captured before its following optional removal unit"
            );
        }
    }
}

#[test]
fn player_major_replacement_keeps_later_players_prefix_after_the_first_deferred_draw() {
    use ironsmith::effect::{EffectId, Value};
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        library(&mut game, B, 6);
        resource(&mut game, A, Zone::Hand, "Type: Land");
        for _ in 0..2 {
            resource(&mut game, B, Zone::Hand, "Type: Land");
        }
        let amount = EffectId(102);
        let mut iteration = ironsmith::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![
                ironsmith::Effect::new(ironsmith::effects::WithIdEffect::new(
                    amount,
                    ironsmith::Effect::new(GainLifeEffect::with_filter(
                        Value::CardsInHand(PlayerFilter::IteratedPlayer),
                        PlayerFilter::IteratedPlayer,
                    )),
                )),
                ironsmith::Effect::new(DrawCardsEffect::new(
                    Value::EffectValue(amount),
                    PlayerFilter::IteratedPlayer,
                )),
            ],
        );
        iteration.sequential = true;
        replace_first_gain(&mut game, source, vec![ironsmith::Effect::new(iteration)]);
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldDrawCardMatcher::new(PlayerFilter::Specific(A)),
                ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                    partner,
                )]),
            ),
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(B).unwrap().hand.len(), 4);
        assert_eq!(
            pending(&mut game, &mut dm),
            1,
            "B's outer original gain precedes the draw; its player-major nested prefix follows the removal"
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.player(C).unwrap().life, 23);
    }
}

#[test]
fn continued_offer_keeps_starting_player_and_stops_after_the_completed_accepted_body() {
    struct Offer {
        players: Vec<PlayerId>,
    }
    impl DecisionMaker for Offer {
        fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
            self.players.push(ctx.player);
            ctx.player == D && ctx.can_accept
        }
    }
    for definition in definitions("Wedding Ring") {
        let (mut game, _, _, source) = simultaneous_replacement_fixture(&definition);
        library(&mut game, C, 3);
        library(&mut game, D, 3);
        let iteration = ironsmith::effects::ForPlayersEffect::new_starting_with_controller(
            PlayerFilter::Any,
            vec![ironsmith::Effect::new(ironsmith::effects::MayEffect::new(
                vec![ironsmith::Effect::new(DrawCardsEffect::new(
                    1,
                    PlayerFilter::IteratedPlayer,
                ))],
            ))],
        )
        .stop_after_first_happened();
        replace_first_gain(&mut game, source, vec![ironsmith::Effect::new(iteration)]);
        let mut dm = Offer { players: vec![] };
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(dm.players, vec![C, D]);
        assert_eq!(game.player(D).unwrap().hand.len(), 1);
        for player in [A, B, C] {
            assert!(game.player(player).unwrap().hand.is_empty());
        }
        assert_eq!(pending(&mut game, &mut dm), 1);
    }
}

#[test]
fn replacement_repetition_freezes_count_and_resumes_prefix_and_suffix_once() {
    use ironsmith::effect::Value;
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        for _ in 0..2 {
            resource(&mut game, A, Zone::Hand, "Type: Land");
        }
        replace_first_gain(
            &mut game,
            source,
            vec![ironsmith::Effect::new(
                ironsmith::effects::RepeatEffectsEffect::new(
                    Value::CardsInHand(PlayerFilter::Specific(A)),
                    vec![
                        ironsmith::Effect::new(GainLifeEffect::you(1)),
                        ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
                    ],
                ),
            )],
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldDrawCardMatcher::new(PlayerFilter::Specific(A)),
                ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                    partner,
                )]),
            ),
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            4,
            "the original count was two, before either draw"
        );
        assert_eq!(
            game.player(C).unwrap().life,
            24,
            "one prefix gain per repetition plus two outer originals"
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(C).unwrap().life, 25);
    }
}

#[test]
fn paused_iteration_suffix_restores_every_original_and_replays_without_duplicate_prefixes() {
    struct Pause(bool);
    impl DecisionMaker for Pause {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            self.0 = true;
            false
        }
        fn awaiting_choice(&self) -> bool {
            self.0
        }
    }
    for definition in definitions("Wedding Ring") {
        for owner in 0..3 {
            let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
            let body = vec![
                ironsmith::Effect::new(GainLifeEffect::you(2)),
                ironsmith::Effect::new(DrawCardsEffect::new(1, PlayerFilter::Specific(A))),
                ironsmith::Effect::new(ironsmith::effects::MayEffect::new(vec![remove_partner(
                    partner,
                )])),
            ];
            let iteration = if owner == 2 {
                ironsmith::Effect::new(ironsmith::effects::RepeatEffectsEffect::new(1, body))
            } else {
                let mut players =
                    ironsmith::effects::ForPlayersEffect::new(PlayerFilter::Specific(A), body);
                players.sequential = owner == 1;
                ironsmith::Effect::new(players)
            };
            replace_first_gain(&mut game, source, vec![iteration]);
            let mut pause = Pause(false);
            let outcome = shared_gain(&mut game, source, &mut pause);
            assert!(pause.0);
            assert!(outcome.events.is_empty());
            for player in [A, B, C, D] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert!(game.player(A).unwrap().hand.is_empty());
            assert!(game.object(partner).is_some());
            let mut dm = SelectFirstDecisionMaker;
            assert_eq!(pending(&mut game, &mut dm), 0);
            let outcome = shared_gain(&mut game, source, &mut dm);
            for event in outcome.events {
                game.queue_trigger_event(event.provenance(), event);
            }
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(game.player(C).unwrap().life, 24);
            assert!(game.object(partner).is_none());
            assert_eq!(pending(&mut game, &mut dm), 1);
        }
    }
}

#[test]
fn each_players_draw_receipt_precedes_the_next_players_replacement_program() {
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        game.turn.active_player = B;
        library(&mut game, B, 3);
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldDrawCardMatcher::new(PlayerFilter::Specific(A)),
                ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                    partner,
                )]),
            ),
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, C, &mut dm);
        let outcome = ironsmith::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![ironsmith::Effect::new(DrawCardsEffect::new(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert!(game.object(partner).is_none());
        assert_eq!(
            pending(&mut game, &mut dm),
            1,
            "B drew while qualified, before A's later draw replacement removed its Ring"
        );
    }
}

#[test]
fn earlier_direct_draw_segment_is_captured_before_a_later_replacement_changes_qualification() {
    // A later draw replacement cannot
    // erase a trigger for a card already drawn in this instruction.
    for definition in definitions("Wedding Ring") {
        let mut game = game();
        game.turn.active_player = B;
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let partner = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let source = resource(&mut game, B, Zone::Battlefield, "Type: Artifact");
        library(&mut game, B, 1);
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                B,
                ironsmith::events::WouldDrawCardWhileLibraryEmptyMatcher::new(
                    PlayerFilter::Specific(B),
                ),
                ironsmith::replacement::ReplacementAction::Instead(vec![remove_partner(partner)]),
            ),
        );
        draw(&mut game, source, B, 2);
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert!(game.object(partner).is_none());
        assert_eq!(pending(&mut game, &mut dm), 1);
    }
}

#[test]
fn draw_introduced_by_nested_life_replacement_waits_for_the_enclosing_originals() {
    // The outer payload contains no literal DrawCards node: its nested life
    // event introduces the draw through a replacement at runtime.
    for definition in definitions("Wedding Ring") {
        let (mut game, _, partner, source) = simultaneous_replacement_fixture(&definition);
        replace_first_gain(
            &mut game,
            source,
            vec![ironsmith::Effect::new(GainLifeEffect::with_filter(
                1,
                PlayerFilter::Specific(D),
            ))],
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldGainLifeMatcher::new(PlayerFilter::Specific(D)),
                ironsmith::replacement::ReplacementAction::Instead(vec![ironsmith::Effect::new(
                    DrawCardsEffect::new(1, PlayerFilter::Specific(A)),
                )]),
            ),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                C,
                ironsmith::events::WouldDrawCardMatcher::new(PlayerFilter::Specific(A)),
                ironsmith::replacement::ReplacementAction::Additionally(vec![remove_partner(
                    partner,
                )]),
            ),
        );
        let mut dm = SelectFirstDecisionMaker;
        let outcome = shared_gain(&mut game, source, &mut dm);
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(
            pending(&mut game, &mut dm),
            1,
            "the dynamically introduced draw still waits for B's outer original gain"
        );
    }
}
