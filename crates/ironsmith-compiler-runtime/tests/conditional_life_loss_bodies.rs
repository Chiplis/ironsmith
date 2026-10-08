//! Exact frozen complete bodies. Authored source evidence only: UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::{Color, ColorSet};
use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::events::EventKind;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_definition, materialize_artifact, materialize_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
const SIMULACRUM: &str = "Tezzeret's Simulacrum";
const SANCTUARY: &str = "Necra Sanctuary";

fn definitions(name: &str) -> [CardDefinition; 3] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/conditional_life_loss_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = result.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let native = encode_runtime_definition(direct.clone()).unwrap();
    let native = materialize_definition(serde_json::from_slice(&serde_json::to_vec(&native).unwrap()).unwrap()).unwrap();
    let definitions = [direct, materialize_artifact(&decoded).unwrap(), native];
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    definitions
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain; game.turn.step = None;
    game
}
struct Choices { target: PlayerId, opponent_only: bool, pause: bool, pending: bool, prompts: usize }
impl Choices {
    fn new(target: PlayerId, opponent_only: bool) -> Self {
        Self { target, opponent_only, pause: false, pending: false, prompts: 0 }
    }
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.prompts += 1;
        assert_eq!(ctx.player, A);
        assert_eq!(ctx.requirements.len(), 1, "one declaration shared by both amounts");
        let legal = &ctx.requirements[0].legal_targets;
        assert_eq!(legal.contains(&Target::Player(A)), !self.opponent_only);
        assert!(legal.contains(&Target::Player(B)) && legal.contains(&Target::Player(C)));
        assert!(legal.contains(&Target::Player(self.target)));
        vec![Target::Player(self.target)]
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        if self.pause { self.pending = true; return false; }
        true
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn activate(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let index = game.current_abilities(source).unwrap().iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
    let action = LegalAction::ActivateAbility { source, ability_index: index };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(3); let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        // Preserve the native announcement state before accepting each pending choice.
        state = state.clone();
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert!(game.is_tapped(source), "the printed tap cost was actually paid");
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, A);
    assert_eq!(game.stack[0].ability_index, Some(index));
    assert!(game.stack[0].source_stable_id.is_some());
    assert!(game.stack[0].source_snapshot.is_some());
    assert!(game.turn_store.turn_history.activated_abilities_this_turn.contains(&(source, index)));
    assert_eq!(game.stack[0].targets, vec![Target::Player(dm.target)]);
    assert_eq!(game.stack[0].target_assignments.len(), 1);
    assert!(!compute_legal_actions(game, A).unwrap().iter().any(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)));
}
fn upkeep(game: &mut GameState, player: PlayerId, dm: &mut Choices) {
    game.turn.active_player = player;
    // Restore the genuine boundary after untap. Upkeep owns the step transition,
    // while its caller must already have entered the beginning phase.
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Untap);
    let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::Upkeep);
    let mut queue = TriggerQueue::new();
    runner.advance(game, &mut queue).unwrap();
    assert_eq!(game.turn.phase, Phase::Beginning);
    assert_eq!(game.turn.step, Some(Step::Upkeep));
    assert!(matches!(runner.state(), ironsmith::turn_runner::TurnState::UpkeepPriority));
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn colored(game: &mut GameState, controller: PlayerId, colors: ColorSet) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), "Colored witness")
        .card_types(vec![CardType::Artifact]).color_indicator(colors).build(), controller, Zone::Battlefield)
}
fn tezzeret(game: &mut GameState, controller: PlayerId, zone: Zone, planeswalker: bool) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), "Subtype witness")
        .card_types(vec![if planeswalker { CardType::Planeswalker } else { CardType::Artifact }])
        .subtypes(vec![Subtype::Tezzeret]).loyalty(5).build(), controller, zone)
}
fn check_loss(game: &GameState, target: PlayerId, amount: u32) {
    for player in [A, B, C] { assert_eq!(game.player(player).unwrap().life,
        20 - if player == target { amount as i32 } else { 0 }); }
    assert_eq!(game.turn_store.turn_history.total_life_lost_for_players(&[target]), amount);
    assert_eq!(game.turn_store.turn_history.event_kind_count(EventKind::LifeLoss), u32::from(amount > 0));
    assert_eq!(game.turn_store.turn_history.event_kind_count(EventKind::Damage), 0, "loss is not damage");
}
#[test]
fn complete_frozen_metadata_and_executable_bodies_survive_all_three_routes() {
    for name in [SIMULACRUM, SANCTUARY] { for definition in definitions(name) {
        let simulacrum = name == SIMULACRUM;
        assert_eq!(definition.card.name, name);
        assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), if simulacrum { "{3}" } else { "{2}{B}" });
        assert_eq!(definition.card.card_types, if simulacrum { vec![CardType::Artifact, CardType::Creature] } else { vec![CardType::Enchantment] });
        assert_eq!(definition.card.subtypes, if simulacrum { vec![Subtype::Golem] } else { vec![] });
        assert_eq!(definition.card.power_toughness, if simulacrum { Some(PowerToughness::fixed(2, 3)) } else { None });
        assert_eq!(definition.card.colors(), if simulacrum { ColorSet::COLORLESS } else { ColorSet::BLACK });
        assert_eq!(definition.card.color_identity(), if simulacrum { ColorSet::COLORLESS } else { ColorSet::BLACK });
        assert!(definition.card.supertypes.is_empty());
        let expected = if simulacrum {
            "{T}: Target opponent loses 1 life. If you control a Tezzeret planeswalker, that player loses 3 life instead."
        } else {
            "At the beginning of your upkeep, if you control a green or white permanent, target player loses 1 life. If you control a green permanent and a white permanent, that player loses 3 life instead."
        };
        assert_eq!(ironsmith_text::canonical_compiled_lines(&definition).join("\n").trim_end_matches('.'), expected.trim_end_matches('.'), "independently authored complete executable body");
        assert_eq!(definition.abilities.len(), 1);
        let program = match &definition.abilities[0].kind {
            AbilityKind::Activated(ability) if simulacrum => &ability.effects,
            AbilityKind::Triggered(ability) if !simulacrum => &ability.effects,
            other => panic!("wrong full-body ability: {other:?}"),
        };
        assert_eq!(program.segments.iter().map(|segment| segment.self_replacements.len()).sum::<usize>(), 1);
    }}
}
#[test]
fn simulacrum_paid_activation_rechecks_tezzeret_type_zone_and_controller_before_one_loss() {
    for definition in definitions(SIMULACRUM) { for scenario in 0..10 {
        let mut game = game(); let mut dm = Choices::new(C, true);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let witness = match scenario {
            1 | 4 | 6 | 7 => Some(tezzeret(&mut game, A, Zone::Battlefield, true)),
            2 => Some(tezzeret(&mut game, B, Zone::Battlefield, true)),
            3 => Some(tezzeret(&mut game, A, Zone::Graveyard, true)),
            8 => Some(tezzeret(&mut game, A, Zone::Battlefield, false)),
            _ => None,
        };
        if scenario == 7 { game.phase_out(witness.unwrap()); }
        if scenario == 9 {
            game.create_object_from_card(&CardBuilder::new(CardId::new(), "Other planeswalker")
                .card_types(vec![CardType::Planeswalker]).subtypes(vec![Subtype::Jace]).loyalty(5).build(), A, Zone::Battlefield);
        }
        activate(&mut game, source, &mut dm);
        if scenario == 4 { game.move_object_by_effect(witness.unwrap(), Zone::Graveyard).unwrap(); }
        if scenario == 5 { tezzeret(&mut game, A, Zone::Battlefield, true); }
        if scenario == 6 { game.set_current_controller(witness.unwrap(), B).unwrap(); }
        // Native state cloning keeps the paid activation; later source control/departure
        // must not turn "you" into the new controller or make the tap payable again.
        game = game.clone();
        game.set_current_controller(source, B).unwrap();
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        check_loss(&game, C, if scenario == 1 || scenario == 5 { 3 } else { 1 });
        assert_eq!(dm.prompts, 1); assert!(game.stack_is_empty());
    }}
}
#[test]
fn sanctuary_outer_or_and_inner_and_are_distinct_and_controller_relative() {
    for definition in definitions(SANCTUARY) { for scenario in 0..8 {
        let mut game = game(); let mut dm = Choices::new(A, false);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        match scenario {
            1 => { colored(&mut game, A, ColorSet::GREEN); }
            2 => { colored(&mut game, A, ColorSet::WHITE); }
            3 => { colored(&mut game, A, ColorSet::GREEN.with(Color::White)); }
            4 => { colored(&mut game, A, ColorSet::GREEN); colored(&mut game, A, ColorSet::WHITE); }
            5 => { colored(&mut game, B, ColorSet::GREEN.with(Color::White)); }
            6 => { colored(&mut game, A, ColorSet::GREEN); colored(&mut game, B, ColorSet::WHITE); }
            7 => { colored(&mut game, A, ColorSet::GREEN.with(Color::White)); }
            _ => {}
        }
        upkeep(&mut game, if scenario == 7 { B } else { A }, &mut dm);
        let triggered = [1, 2, 3, 4, 6].contains(&scenario);
        assert_eq!(game.stack.len(), usize::from(triggered));
        if triggered { assert!(game.stack[0].intervening_if.is_some()); resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        check_loss(&game, A, if [3, 4].contains(&scenario) { 3 } else if triggered { 1 } else { 0 });
    }}
}
#[test]
fn sanctuary_live_gates_recheck_after_real_upkeep_and_source_departure() {
    for definition in definitions(SANCTUARY) { for scenario in 0..5 {
        let mut game = game(); let mut dm = Choices::new(B, false);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let green = colored(&mut game, A, ColorSet::GREEN);
        let white = if scenario != 3 { Some(colored(&mut game, A, ColorSet::WHITE)) } else { None };
        upkeep(&mut game, A, &mut dm); assert_eq!(game.stack.len(), 1);
        match scenario {
            0 => { game.move_object_by_effect(white.unwrap(), Zone::Graveyard).unwrap(); }
            1 => { game.move_object_by_effect(white.unwrap(), Zone::Graveyard).unwrap(); game.move_object_by_effect(green, Zone::Graveyard).unwrap(); }
            2 => { game.set_current_controller(white.unwrap(), B).unwrap(); }
            4 => { game.object_mut(white.unwrap()).unwrap().color_override = Some(ColorSet::RED); game.refresh_continuous_state().unwrap(); }
            _ => { colored(&mut game, A, ColorSet::WHITE); }
        }
        game.set_current_controller(source, B).unwrap(); game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        check_loss(&game, B, match scenario { 1 => 0, 3 => 3, _ => 1 });
    }}
}
#[test]
fn illegal_sole_player_target_does_not_refund_paid_tap_or_retarget() {
    for name in [SIMULACRUM, SANCTUARY] { for definition in definitions(name) {
        let mut game = game(); let mut dm = Choices::new(B, name == SIMULACRUM);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        if name == SIMULACRUM { game.remove_summoning_sickness(source); activate(&mut game, source, &mut dm); }
        else { colored(&mut game, A, ColorSet::GREEN); upkeep(&mut game, A, &mut dm); }
        game.player_mut(B).unwrap().has_lost = true;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        check_loss(&game, B, 0); assert!(game.stack_is_empty());
        if name == SIMULACRUM { assert!(game.is_tapped(source)); }
    }}
}
#[test]
fn chosen_loss_external_replacement_pending_and_failure_restore_native_resolution_then_retry_once() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for name in [SIMULACRUM, SANCTUARY] { for definition in definitions(name) { for resource_failure in [false, true] {
        let mut game = game(); let mut dm = Choices::new(B, name == SIMULACRUM);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        if name == SIMULACRUM { game.remove_summoning_sickness(source); tezzeret(&mut game, A, Zone::Battlefield, true); activate(&mut game, source, &mut dm); }
        else { colored(&mut game, A, ColorSet::GREEN.with(Color::White)); upkeep(&mut game, A, &mut dm); }
        let addition = if resource_failure {
            Effect::new(ironsmith::effects::CreateTokenEffect::you(ironsmith::cards::tokens::treasure_token_definition(), 1))
        } else { Effect::may(vec![Effect::gain_life(2)]) };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::life::matchers::WouldLoseLifeMatcher::any_player(),
            ReplacementAction::Instead(vec![Effect::gain_life(1), addition])));
        if resource_failure { game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 0, ..Default::default() }); }
        dm.pause = !resource_failure;
        let ids = game.next_object_id_counter();
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert_eq!(result.is_err(), resource_failure);
        if !resource_failure { assert!(dm.pending); }
        check_loss(&game, B, 0); assert_eq!(game.stack.len(), 1);
        assert_eq!(game.next_object_id_counter(), ids);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        if name == SIMULACRUM { assert!(game.is_tapped(source), "resolution rollback cannot undo completed payment"); }
        game = game.clone(); game.set_token_creation_limits(Default::default()); dm.pause = false; dm.pending = false;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack_is_empty()); assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(game.player(A).unwrap().life, if resource_failure { 21 } else { 23 });
        assert_eq!(game.turn_store.turn_history.total_life_lost_for_players(&[A, B, C]), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }}}
}
#[test]
fn simulacrum_unpaid_sick_or_opponents_source_cannot_publish_an_activation() {
    for definition in definitions(SIMULACRUM) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.set_summoning_sick(source);
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)));
        assert!(!game.is_tapped(source)); assert!(game.stack_is_empty());
        game.remove_summoning_sickness(source);
        game.set_current_controller(source, B).unwrap();
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)));
        assert!(!game.is_tapped(source)); assert!(game.stack_is_empty());
    }
}
