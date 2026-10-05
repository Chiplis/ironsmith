//! Exact frozen cards and future schedule semantics. Authored; execution deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effects::{EffectContext, EffectExecutor, SkipScheduledEffect};
use ironsmith::events::{DamageTarget, EventCause, TurnedFaceUpEvent};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{StackEntry, Step};
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
use ironsmith::{CardId, CardType, GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_core::ScheduledSkipKind as K;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/scheduled_turn_skips.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        text.push_str(&format!("Loyalty: {loyalty}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20)
}
fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Schedule witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let id = game.create_object_from_card(&card, player, Zone::Battlefield);
    game.set_summoning_sick(id);
    id
}
struct ChooseBob;
impl DecisionMaker for ChooseBob {
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        ctx.requirements
            .iter()
            .flat_map(|requirement| {
                if requirement.legal_targets.contains(&Target::Player(B)) {
                    return vec![Target::Player(B)];
                }
                let objects: Vec<_> = requirement.legal_targets.iter().copied().filter(|target| {
                matches!(target, Target::Object(id) if game.current_controller(*id) == Some(B))
            }).take(requirement.max_targets.unwrap_or(usize::MAX)).collect();
                if objects.len() >= requirement.min_targets {
                    objects
                } else {
                    requirement
                        .legal_targets
                        .iter()
                        .copied()
                        .take(requirement.min_targets)
                        .collect()
                }
            })
            .collect()
    }
}
fn event(game: &mut GameState, event: TriggerEvent) -> usize {
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(game, &mut queue, &mut ChooseBob).unwrap();
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, &mut ChooseBob).unwrap();
    }
    count
}
fn skip(game: &mut GameState, player: PlayerId, kind: K, count: u32) {
    let source = game.new_object_id();
    SkipScheduledEffect {
        player: PlayerFilter::Specific(player),
        kind,
        count,
    }
    .execute(game, &mut EffectContext::new_default(source, player))
    .unwrap();
}
fn advance_combat(game: &mut GameState) -> TurnAction {
    TurnRunner::from_state_for_sync(TurnState::BeginCombat)
        .advance(game, &mut TriggerQueue::new())
        .unwrap()
}

#[test]
fn nine_frozen_cards_retain_lossless_direct_and_restored_definitions() {
    assert_eq!(fixtures().len(), 9);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn repeated_combat_skips_survive_turn_changes_and_consume_extra_combats_individually() {
    for definition in definitions("Stonehorn Dignitary") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(
            event(
                &mut game,
                TriggerEvent::new(
                    ironsmith::events::ZoneChangeEvent::with_cause(
                        source,
                        Zone::Hand,
                        Zone::Battlefield,
                        ironsmith::events::cause::EventCause::effect(),
                        None
                    ),
                    Default::default()
                )
            ),
            1
        );
        assert_eq!(
            event(
                &mut game,
                TriggerEvent::new(
                    ironsmith::events::ZoneChangeEvent::with_cause(
                        source,
                        Zone::Hand,
                        Zone::Battlefield,
                        ironsmith::events::cause::EventCause::effect(),
                        None
                    ),
                    Default::default()
                )
            ),
            1
        );
        assert_eq!(game.turn_store.pending_combat_phase_skips.pending(B), 2);
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        assert!(matches!(advance_combat(&mut game), TurnAction::Continue));
        assert_eq!(game.turn_store.pending_combat_phase_skips.pending(B), 1);
        assert_eq!(game.turn_store.combat_phases_started_this_turn, 0);
        assert!(matches!(advance_combat(&mut game), TurnAction::Continue));
        assert_eq!(game.turn_store.pending_combat_phase_skips.pending(B), 0);
        assert!(matches!(advance_combat(&mut game), TurnAction::RunPriority));
        assert_eq!(game.turn_store.combat_phases_started_this_turn, 1);
    }
}
#[test]
fn eater_schedules_two_distinct_skips_and_skipped_extra_turn_does_not_cure_control() {
    for definition in definitions("Eater of Days") {
        let mut game = game();
        game.establish_turn_start_continuous_control();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.set_summoning_sick(source);
        assert_eq!(
            event(
                &mut game,
                TriggerEvent::new(
                    ironsmith::events::ZoneChangeEvent::with_cause(
                        source,
                        Zone::Hand,
                        Zone::Battlefield,
                        ironsmith::events::cause::EventCause::effect(),
                        None
                    ),
                    Default::default()
                )
            ),
            1
        );
        assert_eq!(game.turn_store.skip_next_turn.pending(A), 2);
        game.turn_store.extra_turns.push(A);
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        assert_eq!(game.turn_store.skip_next_turn.pending(A), 1);
        assert!(game.is_summoning_sick(source));
        game.next_turn();
        assert_eq!(game.turn.active_player, C);
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        assert_eq!(game.turn_store.skip_next_turn.pending(A), 0);
        assert!(game.is_summoning_sick(source));
    }
}
#[test]
fn skipped_untap_omits_untapping_phasing_and_events_but_begins_continuous_control() {
    let mut game = game();
    let bob = creature(&mut game, B);
    game.tap(bob);
    let phased = creature(&mut game, B);
    game.phase_out(phased);
    skip(&mut game, B, K::UntapStep, 1);
    game.take_pending_trigger_events();
    game.next_turn();
    assert_eq!(game.turn.active_player, B);
    assert!(!game.is_summoning_sick(bob));
    assert!(!game.is_summoning_sick(phased));
    let mut runner = TurnRunner::new();
    assert!(matches!(
        runner.advance(&mut game, &mut TriggerQueue::new()).unwrap(),
        TurnAction::Continue
    ));
    assert!(game.is_tapped(bob));
    assert!(game.is_phased_out(phased));
    assert_eq!(game.pending_step_skips(B, Step::Untap), 0);
    assert!(game.take_pending_trigger_events().iter().all(|event| {
        event
            .downcast::<ironsmith::events::PermanentUntappedEvent>()
            .is_none()
    }));
    let late = creature(&mut game, B);
    ironsmith::turn::execute_untap_step(&mut game);
    assert!(
        game.is_summoning_sick(late),
        "an added untap is not a new continuous-control boundary"
    );
}
#[test]
fn brine_is_all_opponents_while_combat_damage_skips_only_the_damaged_player() {
    for definition in definitions("Brine Elemental") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(
            event(
                &mut game,
                TriggerEvent::new(TurnedFaceUpEvent::new(source, A), Default::default())
            ),
            1
        );
        assert_eq!(game.pending_step_skips(A, Step::Untap), 0);
        assert_eq!(game.pending_step_skips(B, Step::Untap), 1);
        assert_eq!(game.pending_step_skips(C, Step::Untap), 1);
    }
    for name in ["Blinding Angel", "Shisato, Whispering Hunter"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert_eq!(
                event(
                    &mut game,
                    TriggerEvent::new(
                        ironsmith::events::DamageEvent::with_cause(
                            source,
                            DamageTarget::Player(B),
                            1,
                            false,
                            EventCause::from_effect(source, A)
                        ),
                        Default::default()
                    )
                ),
                0
            );
            assert_eq!(
                event(
                    &mut game,
                    TriggerEvent::new(
                        ironsmith::events::DamageEvent::with_cause(
                            source,
                            DamageTarget::Player(B),
                            1,
                            true,
                            EventCause::from_effect(source, A)
                        ),
                        Default::default()
                    )
                ),
                1
            );
            if name == "Blinding Angel" {
                assert_eq!(game.turn_store.pending_combat_phase_skips.pending(B), 1);
                assert_eq!(game.turn_store.pending_combat_phase_skips.pending(C), 0);
            } else {
                assert_eq!(game.pending_step_skips(B, Step::Untap), 1);
                assert_eq!(game.pending_step_skips(C, Step::Untap), 0);
            }
        }
    }
}
#[test]
fn avizoa_full_activated_body_retains_pump_and_next_untap_skip() {
    for definition in definitions("Avizoa") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let power = game.current_power(source).unwrap();
        let ability = definition
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            ability.timing,
            ironsmith::ability::ActivationTiming::OncePerTurn
        );
        game.stack
            .push(StackEntry::ability(source, A, ability.effects.clone()));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.current_power(source), Some(power + 2));
        assert_eq!(game.pending_step_skips(A, Step::Untap), 1);
    }
}
#[test]
fn legacy_phase_scheduler_consumes_the_same_persistent_combat_counters() {
    let mut game = game();
    skip(&mut game, B, K::CombatPhase, 2);
    game.next_turn();
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    ironsmith::turn::advance_phase(&mut game).unwrap();
    assert_eq!(game.turn.phase, Phase::NextMain);
    assert_eq!(game.turn_store.pending_combat_phase_skips.pending(B), 1);
}
#[test]
fn current_turn_only_skip_expires_without_spending_a_future_combat_skip() {
    let mut game = game();
    game.turn_store.skip_next_combat_phases.insert(A);
    skip(&mut game, A, K::CombatPhase, 1);
    game.next_turn();
    assert!(!game.turn_store.skip_next_combat_phases.contains(&A));
    assert_eq!(game.turn_store.pending_combat_phase_skips.pending(A), 1);
}

#[test]
fn dovin_taps_only_the_selected_opponents_permanents_and_binds_followup_skip() {
    for definition in definitions("Dovin, Architect of Law") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = creature(&mut game, A);
        let bob = creature(&mut game, B);
        let carol = creature(&mut game, C);
        let ability = definition
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .last()
            .unwrap();
        game.stack.push(
            StackEntry::ability(source, A, ability.effects.clone())
                .with_targets(vec![Target::Player(B)]),
        );
        resolve_stack_entry_with(&mut game, &mut ChooseBob).unwrap();
        assert!(game.is_tapped(bob));
        assert!(!game.is_tapped(own));
        assert!(!game.is_tapped(carol));
        assert_eq!(game.pending_step_skips(B, Step::Untap), 1);
        assert_eq!(game.pending_step_skips(C, Step::Untap), 0);
    }
}
#[test]
fn yosei_target_player_controls_the_separate_optional_permanent_target_group() {
    for definition in definitions("Yosei, the Morning Star") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bob = creature(&mut game, B);
        let carol = creature(&mut game, C);
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
        assert_eq!(
            event(
                &mut game,
                TriggerEvent::new(
                    ironsmith::events::ZoneChangeEvent::with_cause(
                        source,
                        Zone::Battlefield,
                        Zone::Graveyard,
                        ironsmith::events::cause::EventCause::from_effect(source, A),
                        Some(snapshot)
                    ),
                    Default::default()
                )
            ),
            1
        );
        assert!(game.is_tapped(bob));
        assert!(!game.is_tapped(carol));
        assert_eq!(game.pending_step_skips(B, Step::Untap), 1);
        assert_eq!(game.pending_step_skips(C, Step::Untap), 0);
    }
}
#[test]
fn revenant_requires_actual_white_payment_not_just_printed_mana_cost() {
    for definition in definitions("Revenant Patriarch") {
        for paid_white in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            if paid_white {
                game.object_mut(source)
                    .unwrap()
                    .mana_spent_to_cast
                    .add(ironsmith::mana::ManaSymbol::White, 1);
            }
            assert_eq!(
                event(
                    &mut game,
                    TriggerEvent::new(
                        ironsmith::events::ZoneChangeEvent::with_cause(
                            source,
                            Zone::Stack,
                            Zone::Battlefield,
                            ironsmith::events::cause::EventCause::effect(),
                            None
                        ),
                        Default::default()
                    )
                ),
                usize::from(paid_white)
            );
            assert_eq!(
                game.turn_store.pending_combat_phase_skips.pending(B),
                u32::from(paid_white)
            );
        }
    }
}

#[test]
fn future_skips_follow_the_player_across_grand_melee_lanes_without_resurrection() {
    let mut game = GameState::new((0..10).map(|n| format!("Player {n}")).collect(), 20);
    game.restore_grand_melee((0..10).map(PlayerId::from_index).collect())
        .unwrap();
    let views = game.grand_melee_marker_views();
    assert_eq!(views.len(), 2);
    let first = views[0].number;
    let second = views[1].number;
    game.select_grand_melee_turn_marker(first).unwrap();
    let player = views[1].holder;
    skip(&mut game, player, K::CombatPhase, 2);
    skip(&mut game, player, K::UntapStep, 1);
    game.select_grand_melee_turn_marker(second).unwrap();
    assert_eq!(
        game.turn_store.pending_combat_phase_skips.pending(player),
        2
    );
    assert!(game.consume_step_skip(player, Step::Untap));
    assert!(game.turn_store.pending_combat_phase_skips.remove(&player));
    game.select_grand_melee_turn_marker(first).unwrap();
    assert_eq!(
        game.turn_store.pending_combat_phase_skips.pending(player),
        1
    );
    assert_eq!(game.pending_step_skips(player, Step::Untap), 0);
    let snapshot = game.grand_melee_restore_snapshot().unwrap();
    for marker in snapshot.markers {
        assert_eq!(
            marker.turn_store.pending_combat_phase_skips.pending(player),
            1
        );
    }
}

#[test]
fn new_grand_melee_lanes_do_not_inherit_another_holders_control_boundary() {
    let mut game = GameState::new((0..10).map(|n| format!("Player {n}")).collect(), 20);
    game.establish_turn_start_continuous_control();
    game.restore_grand_melee((0..10).map(PlayerId::from_index).collect())
        .unwrap();
    for marker in game.grand_melee_restore_snapshot().unwrap().markers {
        assert_eq!(marker.turn_store.continuous_control_turn_started, None);
    }
}
