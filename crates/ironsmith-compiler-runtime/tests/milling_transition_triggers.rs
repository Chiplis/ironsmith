//! Actual mill actions, replacement destinations, batch/actor scopes, and full
//! Oracle bodies through direct and restored artifacts. Authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::effects::{EffectContext, EffectExecutor, ForPlayersEffect, MillEffect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ChooseSpec, ObjectFilter, PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/milling_transition_triggers.json.fixture"
    ))
    .unwrap()
}
fn definitions_from(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    definitions_from(name, row["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn resource(game: &mut GameState, owner: PlayerId, zone: Zone, kind: &str) -> ObjectId {
    let text = if kind.contains("Creature") {
        format!("Type: {kind}\nPower/Toughness: 1/1")
    } else {
        format!("Type: {kind}")
    };
    let definition = compile_to_runtime_definition("Milling resource", &text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn stack(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    let count = stack(game, dm);
    for _ in 0..30 {
        if game.stack_is_empty() {
            return count;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        stack(game, dm);
    }
    panic!("milling triggers did not settle");
}
fn execute(
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
fn mill(game: &mut GameState, source: ObjectId, player: PlayerId, count: i32) {
    execute(
        game,
        source,
        A,
        &MillEffect::new(count, PlayerFilter::Specific(player)),
    );
}
fn counters(game: &GameState, object: ObjectId) -> u32 {
    game.object(object)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}
fn tokens(game: &GameState) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.kind == ironsmith::object::ObjectKind::Token)
        })
        .count()
}
fn has_milling(kind: &TriggerKind) -> bool {
    match kind {
        TriggerKind::CardsMilled { .. } => true,
        TriggerKind::Either { left, right } => has_milling(&left.kind) || has_milling(&right.kind),
        _ => false,
    }
}
struct Choices {
    target: Option<Target>,
    accept: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.accept
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target
            .iter()
            .filter(|target| {
                ctx.requirements
                    .iter()
                    .any(|req| req.legal_targets.contains(target))
            })
            .cloned()
            .collect()
    }
}
fn activate(game: &mut GameState, source: ObjectId, dm: &mut impl DecisionMaker) {
    game.turn.priority_player = Some(A);
    let action = ironsmith::decision::compute_legal_actions(game, A).unwrap().into_iter()
        .find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
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
        if state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("activation pending without choice");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    settle(game, dm);
}

#[test]
fn seven_exact_cards_retain_typed_milling_events_and_all_oracle_bodies() {
    assert_eq!(fixtures().len(), 7);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
                AbilityKind::Triggered(triggered) if triggered.trigger.compiled_model().is_some_and(|model| has_milling(&model.kind)))));
        }
    }
}

#[test]
fn actual_mill_is_distinct_from_arbitrary_library_moves_and_keeps_public_replacement_cards() {
    for definition in definitions("Glowing One") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ordinary = resource(&mut game, B, Zone::Library, "Creature");
        game.move_object(
            ordinary,
            Zone::Graveyard,
            ironsmith::events::EventCause::from_effect(source, A),
        )
        .unwrap();
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        for kind in ["Creature", "Land", "Artifact"] {
            resource(&mut game, B, Zone::Library, kind);
        }
        mill(&mut game, source, B, 3);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 2);
        assert_eq!(game.player(A).unwrap().life, 22);
        execute(
            &mut game,
            source,
            B,
            &ironsmith::effects::ExileInsteadOfGraveyardEffect::you(),
        );
        let original = resource(&mut game, B, Zone::Library, "Creature");
        mill(&mut game, source, B, 1);
        let events = game.take_pending_trigger_events();
        let actual = events
            .iter()
            .find_map(|event| event.downcast::<ironsmith::events::CardMilledEvent>())
            .unwrap();
        assert_eq!(actual.original_card, original);
        assert_ne!(actual.card, original);
        assert_eq!(actual.snapshot.as_ref().unwrap().object_id, actual.card);
        assert_eq!(actual.snapshot.as_ref().unwrap().zone, Zone::Exile);
        for event in events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(game.player(A).unwrap().life, 23);
        mill(&mut game, source, B, 0);
        mill(&mut game, source, B, 3);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
    }
}

#[test]
fn passive_batches_span_players_while_explicit_player_subjects_keep_separate_groups() {
    for (event, expected) in [
        ("one or more nonland cards are milled", 1),
        ("a player mills one or more nonland cards", 3),
        ("an opponent mills one or more nonland cards", 2),
        ("a player mills a nonland card", 6),
    ] {
        for definition in definitions_from(
            "Milling group fixture",
            &format!("Type: Enchantment\nWhenever {event}, you gain 1 life."),
        ) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for player in [A, B, C] {
                for kind in ["Creature", "Land", "Artifact"] {
                    resource(&mut game, player, Zone::Library, kind);
                }
            }
            execute(
                &mut game,
                source,
                A,
                &ForPlayersEffect::new(
                    PlayerFilter::Any,
                    vec![Effect::new(MillEffect::new(
                        3,
                        PlayerFilter::IteratedPlayer,
                    ))],
                ),
            );
            assert_eq!(
                settle(&mut game, &mut SelectFirstDecisionMaker),
                expected,
                "{event}"
            );
            assert_eq!(game.player(A).unwrap().life, 20 + expected as i32);
            for _ in 0..2 {
                resource(&mut game, B, Zone::Library, "Creature");
                mill(&mut game, source, B, 1);
            }
            assert_eq!(
                settle(&mut game, &mut SelectFirstDecisionMaker),
                2,
                "separate instructions stay distinct"
            );
        }
    }
}

#[test]
fn lo_and_li_share_opponent_subject_and_count_discard_and_mill_with_different_cardinalities() {
    for definition in definitions("Lo and Li, Royal Advisors") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let advisor = resource(&mut game, A, Zone::Battlefield, "Creature — Advisor");
        let other = resource(&mut game, A, Zone::Battlefield, "Creature — Human");
        let opponent = resource(&mut game, B, Zone::Battlefield, "Creature — Advisor");
        for player in [A, B] {
            for _ in 0..3 {
                resource(&mut game, player, Zone::Library, "Land");
            }
        }
        mill(&mut game, source, A, 3);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        mill(&mut game, source, B, 3);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        for _ in 0..2 {
            resource(&mut game, B, Zone::Hand, "Artifact");
        }
        execute(
            &mut game,
            source,
            B,
            &ironsmith::effects::DiscardEffect::you(2),
        );
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 2);
        assert_eq!(counters(&game, source), 3);
        assert_eq!(counters(&game, advisor), 3);
        assert_eq!(counters(&game, other), 0);
        assert_eq!(counters(&game, opponent), 0);
        for _ in 0..4 {
            resource(&mut game, C, Zone::Library, "Artifact");
        }
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        activate(
            &mut game,
            source,
            &mut Choices {
                target: Some(Target::Player(C)),
                accept: true,
            },
        );
        assert!(game.player(C).unwrap().library.is_empty());
        assert_eq!(counters(&game, advisor), 4);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn mirelurk_trigger_limit_is_once_per_turn_and_scorchbeast_limit_counts_accepted_choices() {
    for definition in definitions("Mirelurk Queen") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..3 {
            resource(&mut game, A, Zone::Library, "Artifact");
        }
        for _ in 0..2 {
            resource(&mut game, B, Zone::Library, "Artifact");
        }
        mill(&mut game, source, B, 1);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(counters(&game, source), 1);
        mill(&mut game, source, B, 1);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        game.next_turn();
        resource(&mut game, B, Zone::Library, "Artifact");
        mill(&mut game, source, B, 1);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(counters(&game, source), 2);
    }
    for definition in definitions("Screeching Scorchbeast") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        resource(&mut game, B, Zone::Library, "Artifact");
        mill(&mut game, source, B, 1);
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: false
                }
            ),
            1
        );
        assert_eq!(tokens(&game), 0);
        for kind in ["Artifact", "Land", "Creature"] {
            resource(&mut game, B, Zone::Library, kind);
        }
        mill(&mut game, source, B, 3);
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: true
                }
            ),
            1
        );
        assert_eq!(
            tokens(&game),
            2,
            "only nonlands supply the filtered group amount"
        );
        resource(&mut game, B, Zone::Library, "Artifact");
        mill(&mut game, source, B, 1);
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: true
                }
            ),
            0
        );
        game.next_turn();
        resource(&mut game, B, Zone::Library, "Artifact");
        mill(&mut game, source, B, 1);
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: true
                }
            ),
            1
        );
        assert_eq!(tokens(&game), 3);
    }
}

#[test]
fn radroach_uses_graveyard_intervening_condition_and_never_returns_a_new_incarnation() {
    for definition in definitions("Infesting Radroach") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        resource(&mut game, A, Zone::Library, "Artifact");
        mill(&mut game, source, A, 1);
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: true
                }
            ),
            0
        );
        resource(&mut game, B, Zone::Library, "Land");
        mill(&mut game, source, B, 1);
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: true
                }
            ),
            0
        );
        resource(&mut game, B, Zone::Library, "Artifact");
        mill(&mut game, source, B, 1);
        assert_eq!(stack(&mut game, &mut SelectFirstDecisionMaker), 1);
        let exiled = game
            .move_object(
                source,
                Zone::Exile,
                ironsmith::events::EventCause::from_effect(source, A),
            )
            .unwrap();
        let returned = game
            .move_object(
                exiled,
                Zone::Graveyard,
                ironsmith::events::EventCause::from_effect(exiled, A),
            )
            .unwrap();
        settle(
            &mut game,
            &mut Choices {
                target: None,
                accept: true,
            },
        );
        assert_eq!(game.object(returned).unwrap().zone, Zone::Graveyard);
        assert!(game.player(A).unwrap().hand.is_empty());
        resource(&mut game, B, Zone::Library, "Artifact");
        mill(&mut game, returned, B, 1);
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: true
                }
            ),
            1
        );
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}

#[test]
fn zellix_keeps_ability_label_creature_filter_and_real_paid_tap_activation() {
    for definition in definitions("Zellix, Sanity Flayer") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        for player in [A, B, C] {
            for kind in ["Creature", "Creature", "Land"] {
                resource(&mut game, player, Zone::Library, kind);
            }
        }
        execute(
            &mut game,
            source,
            A,
            &ForPlayersEffect::new(
                PlayerFilter::Any,
                vec![Effect::new(MillEffect::new(
                    3,
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
        );
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: None,
                    accept: true
                }
            ),
            3
        );
        assert_eq!(tokens(&game), 3);
        for kind in ["Artifact", "Creature", "Land"] {
            resource(&mut game, B, Zone::Library, kind);
        }
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        activate(
            &mut game,
            source,
            &mut Choices {
                target: Some(Target::Player(B)),
                accept: true,
            },
        );
        assert!(game.is_tapped(source));
        assert!(game.player(B).unwrap().library.is_empty());
        assert_eq!(tokens(&game), 4);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(
            ironsmith_text::compiled_text_lines(&definition)
                .join(" ")
                .contains("Hive Mind")
        );
    }
}

#[test]
fn radiation_and_secondary_combat_damage_bodies_use_the_same_actual_mill_producer() {
    for name in ["Glowing One", "Infesting Radroach"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            execute(
                &mut game,
                source,
                A,
                &ironsmith::effects::DealDamageEffect::new(
                    2,
                    ChooseSpec::Player(PlayerFilter::Specific(B)),
                )
                .with_combat(true),
            );
            settle(&mut game, &mut SelectFirstDecisionMaker);
            let expected = if name == "Glowing One" { 4 } else { 2 };
            assert_eq!(
                game.player(B).unwrap().counter_count(CounterType::Rad),
                expected
            );
        }
    }
    for definition in definitions("Glowing One") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for kind in ["Artifact", "Land", "Creature"] {
            resource(&mut game, B, Zone::Library, kind);
        }
        game.add_player_counters_with_source(B, CounterType::Rad, 3, Some(source), Some(A)).unwrap();
        execute(
            &mut game,
            source,
            B,
            &ironsmith::effects::RadiationEffect::new(),
        );
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 2);
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(game.player(B).unwrap().life, 18);
        assert_eq!(game.player(B).unwrap().counter_count(CounterType::Rad), 1);
    }
}

#[test]
fn each_player_mill_freezes_counts_before_the_first_players_cards_enter_graveyards() {
    let mut game = game();
    let source = resource(&mut game, A, Zone::Battlefield, "Artifact");
    resource(&mut game, A, Zone::Graveyard, "Creature");
    for player in [A, B, C] {
        for _ in 0..4 {
            resource(&mut game, player, Zone::Library, "Creature");
        }
    }
    let count = ironsmith_core::Value::Count(ObjectFilter::creature().in_zone(Zone::Graveyard));
    execute(
        &mut game,
        source,
        A,
        &ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(MillEffect::new(
                count,
                PlayerFilter::IteratedPlayer,
            ))],
        ),
    );
    for player in [A, B, C] {
        assert_eq!(
            game.player(player).unwrap().library.len(),
            3,
            "all three counts use the same pre-instruction state"
        );
    }
}

#[test]
fn hidden_destination_exposes_no_characteristics_and_prevention_emits_no_mill() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for destination in [Zone::Hand, Zone::Library] {
        for definition in definitions("Glowing One") {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let original = resource(&mut game, B, Zone::Library, "Artifact");
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    A,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(original),
                        Some(Zone::Library),
                        Some(Zone::Graveyard),
                    ),
                    ReplacementAction::ChangeDestination(destination),
                ),
            );
            mill(&mut game, source, B, 1);
            let events = game.take_pending_trigger_events();
            for event in &events {
                if let Some(milled) = event.downcast::<ironsmith::events::CardMilledEvent>() {
                    assert!(
                        milled.snapshot.is_none(),
                        "no private card characteristics in a mill notification"
                    );
                }
            }
            for event in events {
                game.queue_trigger_event(event.provenance(), event);
            }
            assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        }
    }
    for definition in definitions("Glowing One") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(&mut game, B, Zone::Library, "Artifact");
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(original),
                    Some(Zone::Library),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::Prevent,
            ));
        mill(&mut game, source, B, 1);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        assert_eq!(game.object(original).unwrap().zone, Zone::Library);
    }
}

#[test]
fn mirelurk_entry_and_scorchbeast_attack_put_rad_counters_on_the_authored_players() {
    for definition in definitions("Mirelurk Queen") {
        let mut game = game();
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        game.move_object(
            hand,
            Zone::Battlefield,
            ironsmith::events::EventCause::from_effect(hand, A),
        )
        .unwrap();
        assert_eq!(
            settle(
                &mut game,
                &mut Choices {
                    target: Some(Target::Player(B)),
                    accept: true
                }
            ),
            1
        );
        assert_eq!(game.player(B).unwrap().counter_count(CounterType::Rad), 2);
        assert_eq!(game.player(A).unwrap().counter_count(CounterType::Rad), 0);
    }
    for definition in definitions("Screeching Scorchbeast") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat = ironsmith::combat_state::CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[ironsmith::decision::AttackerDeclaration {
                creature: source,
                target: ironsmith::combat_state::AttackTarget::Player(B),
            }],
        )
        .unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game, &mut SelectFirstDecisionMaker);
        for player in [A, B, C] {
            assert_eq!(
                game.player(player).unwrap().counter_count(CounterType::Rad),
                2
            );
        }
    }
}

struct MothmanTargets {
    targets: Vec<Target>,
    expected_max: usize,
}
impl DecisionMaker for MothmanTargets {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert_eq!(ctx.requirements.len(), 1);
        assert_eq!(ctx.requirements[0].min_targets, 0);
        assert_eq!(
            ctx.requirements[0].max_targets,
            Some(self.expected_max),
            "target cap is the filtered event count before resolution"
        );
        for target in &self.targets {
            assert!(ctx.requirements[0].legal_targets.contains(target));
        }
        self.targets.clone()
    }
}
#[test]
fn mothman_target_cap_is_the_exact_simultaneous_nonland_count_and_survives_card_departure() {
    for definition in definitions("The Wise Mothman") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = resource(&mut game, A, Zone::Battlefield, "Creature");
        let enemy = resource(&mut game, B, Zone::Battlefield, "Creature");
        for player in [A, B, C] {
            resource(&mut game, player, Zone::Library, "Land");
            resource(
                &mut game,
                player,
                Zone::Library,
                if player == C { "Land" } else { "Artifact" },
            );
        }
        execute(
            &mut game,
            source,
            A,
            &ForPlayersEffect::new(
                PlayerFilter::Any,
                vec![Effect::new(MillEffect::new(
                    2,
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
        );
        let milled = *game
            .player(A)
            .unwrap()
            .graveyard
            .iter()
            .find(|id| {
                game.object(**id)
                    .unwrap()
                    .card_types
                    .contains(&ironsmith::CardType::Artifact)
            })
            .unwrap();
        game.move_object(
            milled,
            Zone::Exile,
            ironsmith::events::EventCause::from_effect(source, A),
        )
        .unwrap();
        let mut dm = MothmanTargets {
            targets: vec![Target::Object(own), Target::Object(enemy)],
            expected_max: 2,
        };
        assert_eq!(settle(&mut game, &mut dm), 1);
        assert_eq!(counters(&game, own), 1);
        assert_eq!(counters(&game, enemy), 1);
        assert_eq!(counters(&game, source), 0);
        resource(&mut game, B, Zone::Library, "Creature");
        mill(&mut game, source, B, 1);
        let mut dm = MothmanTargets {
            targets: Vec::new(),
            expected_max: 1,
        };
        assert_eq!(settle(&mut game, &mut dm), 1, "up to X allows zero targets");
    }
}
#[test]
fn typed_milling_count_does_not_widen_an_unrelated_or_differently_filtered_trigger() {
    for text in [
        "Type: Enchantment\nAt the beginning of your upkeep, draw X cards, where X is the number of nonland cards milled this way.",
        "Type: Enchantment\nWhenever one or more nonland cards are milled, draw X cards, where X is the number of creature cards milled this way.",
    ] {
        assert!(compile_to_runtime_definition("Mismatched milling count", text, false).is_err());
    }
    for definition in definitions_from(
        "Local mill count",
        "Type: Enchantment\nWhenever one or more nonland cards are milled, mill two cards, then draw X cards, where X is the number of creature cards milled this way.",
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        // The triggering action is on Bob; Alice's local mill contains no
        // nonlands, so it cannot recursively trigger itself.
        for kind in ["Artifact", "Land", "Land"] {
            resource(&mut game, A, Zone::Library, kind);
        }
        resource(&mut game, B, Zone::Library, "Artifact");
        mill(&mut game, source, B, 1);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert!(
            game.player(A).unwrap().hand.is_empty(),
            "the local creature count is zero, not the triggering nonland count of one"
        );
        assert_eq!(game.player(A).unwrap().library.len(), 1);
    }
}
#[test]
fn mothman_entry_and_attack_each_give_every_player_one_rad_counter() {
    for definition in definitions("The Wise Mothman") {
        let mut game = game();
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source = game
            .move_object(
                hand,
                Zone::Battlefield,
                ironsmith::events::EventCause::from_effect(hand, A),
            )
            .unwrap();
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        for player in [A, B, C] {
            assert_eq!(
                game.player(player).unwrap().counter_count(CounterType::Rad),
                1
            );
        }
        game.remove_summoning_sickness(source);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat = ironsmith::combat_state::CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[ironsmith::decision::AttackerDeclaration {
                creature: source,
                target: ironsmith::combat_state::AttackTarget::Player(B),
            }],
        )
        .unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game, &mut SelectFirstDecisionMaker);
        for player in [A, B, C] {
            assert_eq!(
                game.player(player).unwrap().counter_count(CounterType::Rad),
                2
            );
        }
    }
}

#[test]
fn simultaneous_mill_commits_all_originals_before_replacement_added_mill_instructions() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions_from(
        "Mill replacement order",
        "Type: Enchantment\nWhenever one or more nonland cards are milled, you gain that much life.",
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let alice_original = resource(&mut game, A, Zone::Library, "Artifact");
        resource(&mut game, B, Zone::Library, "Artifact");
        resource(&mut game, B, Zone::Library, "Artifact");
        resource(&mut game, C, Zone::Library, "Artifact");
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(alice_original),
                    Some(Zone::Library),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::Additionally(vec![Effect::new(MillEffect::new(
                    1,
                    PlayerFilter::Specific(B),
                ))]),
            ));
        execute(
            &mut game,
            source,
            A,
            &ForPlayersEffect::new(
                PlayerFilter::Any,
                vec![Effect::new(MillEffect::new(
                    1,
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
        );
        assert!(
            game.player(B).unwrap().library.is_empty(),
            "Bob's original top card moves before the replacement-added instruction mills his next card"
        );
        assert_eq!(
            settle(&mut game, &mut SelectFirstDecisionMaker),
            2,
            "one three-player original batch and one later instruction"
        );
        assert_eq!(
            game.player(A).unwrap().life,
            24,
            "three original nonlands plus the additional nonland all count"
        );
    }
}

#[test]
fn deferred_mill_additions_preserve_participant_and_replacement_controller_context() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    let mut game = game();
    let source = resource(&mut game, A, Zone::Battlefield, "Artifact");
    let replacement_source = resource(&mut game, B, Zone::Battlefield, "Enchantment");
    let mut originals = Vec::new();
    for player in [A, B, C] {
        resource(&mut game, player, Zone::Library, "Artifact");
        originals.push(resource(&mut game, player, Zone::Library, "Artifact"));
    }
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement_source,
            B,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(originals[0]),
                Some(Zone::Library),
                Some(Zone::Graveyard),
            ),
            ReplacementAction::Additionally(vec![
                Effect::gain_life(3),
                Effect::new(MillEffect::new(1, PlayerFilter::IteratedPlayer)),
            ]),
        ));
    execute(
        &mut game,
        source,
        A,
        &ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(MillEffect::new(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        ),
    );
    assert_eq!(
        game.player(A).unwrap().graveyard.len(),
        2,
        "the added mill retains the original affected/iterated player"
    );
    assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
    assert_eq!(game.player(C).unwrap().graveyard.len(), 1);
    assert_eq!(game.player(A).unwrap().life, 20);
    assert_eq!(
        game.player(B).unwrap().life,
        23,
        "the added program is controlled by its captured replacement source, not the outer spell"
    );
}

struct PauseMillAddition {
    pause: bool,
    pending: bool,
    prompts: usize,
}
impl DecisionMaker for PauseMillAddition {
    fn decide_boolean(&mut self, game: &GameState, _: &BooleanContext) -> bool {
        self.prompts += 1;
        for player in [A, B, C] {
            assert!(
                game.player(player).unwrap().library.is_empty(),
                "every original participant commits before an addition asks a question"
            );
            assert_eq!(game.player(player).unwrap().graveyard.len(), 1);
        }
        self.pending = self.pause;
        !self.pause
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}
#[test]
fn a_pending_or_failed_mill_addition_rolls_back_the_whole_simultaneous_instruction() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for fails in [false, true] {
        let mut game = game();
        let source = resource(&mut game, A, Zone::Battlefield, "Artifact");
        let replacement_source = resource(&mut game, B, Zone::Battlefield, "Enchantment");
        let originals: Vec<_> = [A, B, C]
            .into_iter()
            .map(|player| resource(&mut game, player, Zone::Library, "Artifact"))
            .collect();
        let additions = if fails {
            vec![
                Effect::gain_life(3),
                Effect::lose_life(ironsmith_core::Value::X),
            ]
        } else {
            vec![Effect::may(vec![Effect::gain_life(3)])]
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                replacement_source,
                B,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(originals[0]),
                    Some(Zone::Library),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::Additionally(additions),
            ),
        );
        game.take_pending_trigger_events();
        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(MillEffect::new(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        );
        let mut dm = PauseMillAddition {
            pause: true,
            pending: false,
            prompts: 0,
        };
        {
            let mut ctx = EffectContext::new(source, A, &mut dm);
            let result = effect.execute(&mut game, &mut ctx);
            assert_eq!(result.is_err(), fails);
        }
        for (index, player) in [A, B, C].into_iter().enumerate() {
            assert_eq!(
                game.player(player).unwrap().library.as_slice(),
                &[originals[index]]
            );
            assert!(game.player(player).unwrap().graveyard.is_empty());
            assert_eq!(game.player(player).unwrap().life, 20);
        }
        assert!(
            game.take_pending_trigger_events().is_empty(),
            "no phantom mill or zone notification survives rollback"
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        if !fails {
            assert!(dm.pending);
            dm.pending = false;
            dm.pause = false;
            let mut ctx = EffectContext::new(source, A, &mut dm);
            effect.execute(&mut game, &mut ctx).unwrap();
            drop(ctx);
            assert_eq!(dm.prompts, 2);
            assert_eq!(game.player(B).unwrap().life, 23);
            for player in [A, B, C] {
                assert_eq!(game.player(player).unwrap().graveyard.len(), 1);
            }
        }
    }
}

#[test]
fn every_mill_receipt_freezes_before_an_earlier_addition_moves_a_later_result() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    let mut game = game();
    let source = resource(&mut game, A, Zone::Battlefield, "Artifact");
    let replacement_source = resource(&mut game, C, Zone::Battlefield, "Enchantment");
    let alice_original = resource(&mut game, A, Zone::Library, "Artifact");
    let costly = compile_to_runtime_definition(
        "Five mana milled card",
        "Mana cost: {5}\nType: Artifact",
        false,
    )
    .unwrap();
    let bob_original = game.create_object_from_definition(&costly, B, Zone::Library);
    resource(&mut game, C, Zone::Library, "Artifact");
    let mut bob_graveyard = ObjectFilter::default().in_zone(Zone::Graveyard);
    bob_graveyard.owner = Some(PlayerFilter::Specific(B));
    for (original, additions) in [
        (
            alice_original,
            vec![Effect::new(ironsmith::effects::ExileEffect::with_spec(
                ChooseSpec::All(bob_graveyard),
            ))],
        ),
        (
            bob_original,
            vec![Effect::gain_life(ironsmith_core::Value::ManaValueOf(
                Box::new(ChooseSpec::Tagged("it".into())),
            ))],
        ),
    ] {
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                replacement_source,
                C,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(original),
                    Some(Zone::Library),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::Additionally(additions),
            ));
    }
    execute(
        &mut game,
        source,
        A,
        &ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(MillEffect::new(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        ),
    );
    assert!(game.player(B).unwrap().graveyard.is_empty());
    assert_eq!(
        game.player(C).unwrap().life,
        25,
        "the second receipt retains its exact five-mana result snapshot after the first addition exiles it"
    );
}
