//! Source-authored native scenarios, intentionally UNRUN.
//! RuntimeSavepoint owns the runner and inactive host continuations. A
//! ReplayCheckpoint owns native game/queue/priority state, not the runner.
use super::*;
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::game_state::{Phase, Step};
use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
use ironsmith::types::Subtype;

fn definition(name: &str) -> CardDefinition {
    // The fourth fixture row remains held; these scenarios admit only the
    // three exact Oracle bodies assigned to this duration reconstruction.
    assert!(matches!(name, "Fatigue" | "Misstep" | "Orcish Farmer"));
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/next_step_durations.json.fixture"
    )).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    ironsmith_registry_test::compile_to_runtime_definition(name, &text, false).unwrap()
}

fn resolve_body(game: &mut GameState, name: &str, caster: PlayerId, target: ResolvedTarget) {
    let definition = definition(name);
    let zone = if name == "Orcish Farmer" { Zone::Battlefield } else { Zone::Stack };
    let source = game.create_object_from_definition(&definition, caster, zone);
    let program = if name == "Orcish Farmer" {
        definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(&ability.effects),
            _ => None,
        }).unwrap()
    } else {
        definition.spell_effect.as_ref().unwrap()
    };
    let mut context = EffectContext::new_default(source, caster).with_targets(vec![target]);
    for effect in program {
        execute_effect(game, effect, &mut context).unwrap();
    }
    game.refresh_continuous_state().unwrap();
}

fn permanent(game: &mut GameState, owner: PlayerId, name: &str, kind: CardType) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), name)
        .card_types(vec![kind]).build(), owner, Zone::Battlefield)
}

fn optional_artifact(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Untap choice")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            ),
        )).build();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn advance(wasm: &mut WasmGame) -> Result<TurnAction, ironsmith::game_loop::GameLoopError> {
    let action = wasm.runner.as_mut().unwrap().advance(&mut wasm.game, &mut wasm.trigger_queue)?;
    wasm.pending_decision = match &action {
        TurnAction::Decision(context) => Some(context.clone()),
        _ => None,
    };
    wasm.runner_pending_decision = matches!(&action, TurnAction::Decision(_));
    wasm.runner_awaiting_priority = matches!(&action, TurnAction::RunPriority);
    Ok(action)
}

fn assert_misstep_pending(game: &GameState, player: PlayerId) {
    assert_eq!(game.effect_store.restriction_effects.iter().filter(|effect| {
        matches!(&effect.duration, ironsmith::effect::Until::PlayersNextUntapStep {
            player: ironsmith::target::PlayerFilter::Specific(bound),
        } if *bound == player) && !effect.consumed_next_untap
    }).count(), 1);
}

fn assert_misstep_consumed(game: &GameState) {
    assert!(!game.effect_store.restriction_effects.iter().any(|effect|
        matches!(&effect.duration, ironsmith::effect::Until::PlayersNextUntapStep { .. })));
}

fn assert_land(game: &GameState, land: ObjectId, subtype: Subtype) {
    assert!(game.current_has_subtype(land, subtype));
    let other = if subtype == Subtype::Swamp { Subtype::Forest } else { Subtype::Swamp };
    assert!(!game.current_has_subtype(land, other));
}

fn armed_runtime(optional: bool) -> (WasmGame, PlayerId, ObjectId, ObjectId, ObjectId) {
    let mut wasm = WasmGame::new();
    wasm.game = GameState::new(vec!["Caster".into(), "Affected".into()], 20);
    let caster = PlayerId::from_index(0);
    let affected = PlayerId::from_index(1);
    wasm.game.turn.active_player = affected;
    wasm.game.turn.turn_number = 2;
    wasm.game.turn.phase = Phase::Beginning;
    wasm.game.turn.step = Some(Step::Untap);
    let creature = permanent(&mut wasm.game, affected, "Affected creature", CardType::Creature);
    let land = wasm.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::basic_forest(), affected, Zone::Battlefield);
    let artifact = if optional {
        optional_artifact(&mut wasm.game, affected)
    } else {
        permanent(&mut wasm.game, affected, "Replacement artifact", CardType::Artifact)
    };
    for object in [creature, land, artifact] { wasm.game.tap(object); }
    for _ in 0..2 {
        resolve_body(&mut wasm.game, "Fatigue", caster, ResolvedTarget::Player(affected));
    }
    resolve_body(&mut wasm.game, "Misstep", caster, ResolvedTarget::Player(affected));
    resolve_body(&mut wasm.game, "Orcish Farmer", caster, ResolvedTarget::Object(land));
    wasm.game.player_mut(affected).unwrap().mana_pool.blue = 2;
    wasm.game.take_pending_trigger_events();
    wasm.runner = Some(TurnRunner::new());
    (wasm, affected, creature, land, artifact)
}

fn assert_armed(game: &GameState, affected: PlayerId, creature: ObjectId, land: ObjectId) {
    assert_misstep_pending(game, affected);
    assert_land(game, land, Subtype::Swamp);
    assert!(game.is_tapped(creature));
    assert!(game.is_tapped(land));
    assert_eq!(game.pending_step_skips(affected, Step::Draw), 2);
    assert_eq!(game.turn_store.untap_step_started_at, None);
    assert_eq!(game.player(affected).unwrap().mana_pool.blue, 2);
}

#[test]
fn pending_untap_branches_restore_real_choices_and_replay_restores_only_native_game_state() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, affected, creature, land, artifact) = armed_runtime(true);
    let TurnAction::Decision(DecisionContext::Boolean(prompt)) = advance(&mut wasm).unwrap() else {
        panic!("the untap step must pause at the artifact's real choice");
    };
    assert_eq!(prompt.source, Some(artifact));
    assert_armed(&wasm.game, affected, creature, land);
    assert!(wasm.game.take_pending_trigger_events().is_empty());
    wasm.pending_decision_game = Some(Box::new(wasm.game.clone()));
    let replay = wasm.capture_replay_checkpoint();
    wasm.pending_action_checkpoint = Some(replay.clone());
    let mut pending = RuntimeSavepoint::capture(&wasm);
    let copied_pending = pending.clone();

    wasm.runner.as_mut().unwrap().respond_boolean(true);
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::UntapEndMana));
    assert_misstep_consumed(&wasm.game);
    assert_land(&wasm.game, land, Subtype::Forest);
    assert!(wasm.game.is_tapped(creature));
    assert!(!wasm.game.is_tapped(artifact));
    assert!(!wasm.game.is_tapped(land));
    assert!(wasm.game.turn_store.untap_step_started_at.is_some());
    assert_eq!(wasm.game.player(affected).unwrap().mana_pool.blue, 2,
        "the rule has expired after untapping, before the separate mana boundary");

    // Native replay rewinds the occurrence and effects, but deliberately
    // leaves the completed runner alone. Only the full savepoint resumes it.
    wasm.restore_replay_checkpoint(&replay);
    assert_armed(&wasm.game, affected, creature, land);
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::UntapEndMana));
    copied_pending.restore(&mut wasm);
    assert!(wasm.runner_pending_decision);
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Untap));
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Decision(DecisionContext::Boolean(_))));
    assert_armed(wasm.pending_decision_game.as_ref().unwrap(), affected, creature, land);
    assert_armed(&wasm.pending_action_checkpoint.as_ref().unwrap().game, affected, creature, land);

    let late_creature = permanent(&mut wasm.game, affected, "Creature added to branch", CardType::Creature);
    wasm.game.tap(late_creature);
    wasm.runner.as_mut().unwrap().respond_boolean(false);
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert!(wasm.game.is_tapped(artifact));
    assert!(wasm.game.is_tapped(late_creature),
        "the restored player rule evaluates a live creature filter when untapping resumes");
    assert_misstep_consumed(&wasm.game);
    let completed_receipt = wasm.game.turn_store.untap_step_started_at;
    pending.exchange(&mut wasm);
    assert_armed(&wasm.game, affected, creature, land);
    assert!(wasm.game.object(late_creature).is_none());
    assert!(wasm.runner_pending_decision);
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Untap));
    pending.exchange(&mut wasm);
    assert_eq!(wasm.game.turn_store.untap_step_started_at, completed_receipt);
    assert!(wasm.game.is_tapped(artifact));
    assert!(wasm.game.is_tapped(late_creature));
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Upkeep));
    assert_eq!(wasm.game.player(affected).unwrap().mana_pool.blue, 0);
    assert_eq!(wasm.game.pending_step_skips(affected, Step::Draw), 2);
}

#[test]
fn errored_untap_keeps_duration_owners_occurrence_receipt_and_replacement_for_native_retry() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, affected, creature, land, artifact) = armed_runtime(false);
    let replacement = wasm.game.effect_store.replacement_effects.add_one_shot_effect(
        ironsmith::replacement::ReplacementEffect::with_matcher(artifact, affected,
            ironsmith::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                ironsmith::target::ObjectFilter::specific(artifact)),
            ironsmith::replacement::ReplacementAction::Instead(vec![
                ironsmith::effect::Effect::gain_life(2),
                ironsmith::effect::Effect::lose_life(ironsmith::effect::Value::X),
            ]),
        ),
    );
    let saved = RuntimeSavepoint::capture(&wasm);
    for _ in 0..2 {
        assert!(matches!(advance(&mut wasm),
            Err(ironsmith::game_loop::GameLoopError::ResolutionFailed(_))));
        assert_armed(&wasm.game, affected, creature, land);
        assert_eq!(wasm.game.player(affected).unwrap().life, 20);
        assert!(wasm.game.is_tapped(artifact));
        assert!(wasm.game.effect_store.replacement_effects.get_effect(replacement).is_some());
        assert!(wasm.game.take_pending_trigger_events().is_empty());
        assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::BeginTurn));
    }
    // A divergent successful branch must not destroy the failed branch's
    // pending exact land owner, live player rule, or counted draw skips.
    wasm.game.effect_store.replacement_effects.remove_effect(replacement);
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert_land(&wasm.game, land, Subtype::Forest);
    assert_misstep_consumed(&wasm.game);
    saved.restore(&mut wasm);
    assert_armed(&wasm.game, affected, creature, land);
    assert!(wasm.game.effect_store.replacement_effects.get_effect(replacement).is_some());
    wasm.game.effect_store.replacement_effects.remove_effect(replacement);
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert!(wasm.game.is_tapped(creature));
    assert!(!wasm.game.is_tapped(land));
    assert!(!wasm.game.is_tapped(artifact));
    assert_misstep_consumed(&wasm.game);
}

#[test]
fn counted_fatigue_skips_and_added_draw_continuation_survive_exchange_and_replay() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.game = GameState::new(vec!["A".into(), "B".into()], 20);
    let player = PlayerId::from_index(0);
    wasm.game.turn.active_player = player;
    wasm.game.turn.turn_number = 2;
    wasm.game.turn.phase = Phase::Beginning;
    wasm.game.turn.step = Some(Step::Draw);
    let card = CardBuilder::new(CardId::new(), "Unskipped draw").build();
    wasm.game.create_object_from_card(&card, player, Zone::Library);
    for _ in 0..2 { resolve_body(&mut wasm.game, "Fatigue", player, ResolvedTarget::Player(player)); }
    wasm.game.add_step_after(Step::Draw, Step::Draw);
    wasm.runner = Some(TurnRunner::from_state_for_sync(TurnState::Draw));
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert_eq!(wasm.game.pending_step_skips(player, Step::Draw), 1);
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::SkippedPhaseEndMana));
    let replay = wasm.capture_replay_checkpoint();
    let mut boundary = RuntimeSavepoint::capture(&wasm);
    let copy = boundary.clone();
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Draw));
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert_eq!(wasm.game.pending_step_skips(player, Step::Draw), 0);
    assert!(wasm.game.player(player).unwrap().hand.is_empty());
    assert_eq!(wasm.game.player(player).unwrap().library.len(), 1);
    boundary.exchange(&mut wasm);
    assert_eq!(wasm.game.pending_step_skips(player, Step::Draw), 1);
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::SkippedPhaseEndMana));
    boundary.exchange(&mut wasm);
    assert_eq!(wasm.game.pending_step_skips(player, Step::Draw), 0);
    wasm.restore_replay_checkpoint(&replay);
    assert_eq!(wasm.game.pending_step_skips(player, Step::Draw), 1);
    // Pairing the correct runner with the pending added-step schedule is a
    // full-savepoint operation, not a property of the replay checkpoint.
    copy.restore(&mut wasm);
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Draw));
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert_eq!(wasm.game.pending_step_skips(player, Step::Draw), 0);
    assert!(wasm.game.player(player).unwrap().hand.is_empty());
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::FirstMain));
    wasm.game.turn.phase = Phase::Beginning;
    wasm.game.turn.step = Some(Step::Draw);
    wasm.runner = Some(TurnRunner::from_state_for_sync(TurnState::Draw));
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::RunPriority));
    assert_eq!(wasm.game.player(player).unwrap().hand.len(), 1);
}

#[test]
fn inactive_grand_melee_host_lane_retains_its_runner_and_bound_players_actual_untap() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.game = GameState::new((0..8).map(|index| format!("P{index}")).collect(), 20);
    wasm.game.restore_grand_melee((0..8).map(PlayerId::from_index).collect()).unwrap();
    let markers = wasm.game.grand_melee_marker_views();
    let root = markers[0].number;
    let other = markers[1].number;
    let a = markers[0].holder;
    let b = markers[1].holder;
    wasm.runner = Some(TurnRunner::new());
    wasm.select_grand_melee_stack_lane(b.0, other).unwrap();
    let creature = permanent(&mut wasm.game, b, "Other lane creature", CardType::Creature);
    let land = wasm.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::basic_forest(), b, Zone::Battlefield);
    for object in [creature, land] { wasm.game.tap(object); }
    for _ in 0..2 { resolve_body(&mut wasm.game, "Fatigue", b, ResolvedTarget::Player(b)); }
    resolve_body(&mut wasm.game, "Misstep", b, ResolvedTarget::Player(b));
    resolve_body(&mut wasm.game, "Orcish Farmer", b, ResolvedTarget::Object(land));
    wasm.select_grand_melee_stack_lane(a.0, root).unwrap();
    let root_creature = permanent(&mut wasm.game, a, "Root lane creature", CardType::Creature);
    wasm.game.tap(root_creature);
    assert!(wasm.game.is_active_player(b), "both marker holders are active in Grand Melee");
    assert!(!wasm.game.turn_players().contains(&b));
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    let root_receipt = wasm.game.turn_store.untap_step_started_at;
    assert!(root_receipt.is_some());
    assert!(!wasm.game.is_tapped(root_creature));
    assert!(wasm.game.is_tapped(creature));
    assert_land(&wasm.game, land, Subtype::Swamp);
    assert_misstep_pending(&wasm.game, b);
    assert!(wasm.game.effect_store.restriction_effects.iter().all(|effect|
        !effect.is_active(&wasm.game, wasm.game.turn.turn_number)));
    assert!(matches!(wasm.grand_melee_host_lanes[&other].runner.as_ref().unwrap().state(),
        TurnState::BeginTurn));
    let mut saved = RuntimeSavepoint::capture(&wasm);
    let copy = saved.clone();

    wasm.select_grand_melee_stack_lane(b.0, other).unwrap();
    assert_eq!(wasm.game.turn_store.untap_step_started_at, None);
    assert_eq!(wasm.game.pending_step_skips(b, Step::Draw), 2);
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert_misstep_consumed(&wasm.game);
    assert_land(&wasm.game, land, Subtype::Forest);
    assert!(wasm.game.is_tapped(creature));
    assert!(!wasm.game.is_tapped(land));
    assert!(wasm.game.turn_store.untap_step_started_at.is_some());
    wasm.select_grand_melee_stack_lane(a.0, root).unwrap();
    assert_eq!(wasm.game.turn_store.untap_step_started_at, root_receipt);
    assert!(matches!(wasm.grand_melee_host_lanes[&other].runner.as_ref().unwrap().state(),
        TurnState::UntapEndMana));
    saved.exchange(&mut wasm);
    assert_land(&wasm.game, land, Subtype::Swamp);
    assert_misstep_pending(&wasm.game, b);
    assert_eq!(wasm.game.turn_store.untap_step_started_at, root_receipt);
    assert!(matches!(wasm.grand_melee_host_lanes[&other].runner.as_ref().unwrap().state(),
        TurnState::BeginTurn));
    saved.exchange(&mut wasm);
    assert_land(&wasm.game, land, Subtype::Forest);
    assert_misstep_consumed(&wasm.game);
    copy.restore(&mut wasm);
    wasm.select_grand_melee_stack_lane(b.0, other).unwrap();
    assert_eq!(wasm.game.turn_store.untap_step_started_at, None);
    assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
    assert_land(&wasm.game, land, Subtype::Forest);
    assert_misstep_consumed(&wasm.game);
    assert_eq!(wasm.game.pending_step_skips(b, Step::Draw), 2);
}

// Authored rules inference / UNRUN: the dynamic-controller expectations below
// are proposed semantics, not a validated rules result. The selected beginning
// expirations belong to the original occurrence, even if control changes while
// its native runner is suspended at an optional choice.
#[test]
fn restored_suspended_untap_retains_original_boundary_before_late_rules_and_skip() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.game = GameState::new(vec!["Caster".into(), "Affected".into(), "Third".into()], 20);
    let caster = PlayerId::from_index(0);
    let affected = PlayerId::from_index(1);
    let third = PlayerId::from_index(2);
    wasm.game.turn.active_player = affected;
    wasm.game.turn.turn_number = 2;
    wasm.game.turn.phase = Phase::Beginning;
    wasm.game.turn.step = Some(Step::Untap);
    let creature = permanent(&mut wasm.game, affected, "Late rule creature", CardType::Creature);
    let land = wasm.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::basic_forest(), affected, Zone::Battlefield);
    let selected_land = wasm.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::basic_forest(), affected, Zone::Battlefield);
    let unselected_land = wasm.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::basic_forest(), third, Zone::Battlefield);
    let artifact = optional_artifact(&mut wasm.game, affected);
    for object in [creature, land, selected_land, unselected_land, artifact] { wasm.game.tap(object); }
    for old_land in [selected_land, unselected_land] {
        resolve_body(&mut wasm.game, "Orcish Farmer", caster, ResolvedTarget::Object(old_land));
        assert_land(&wasm.game, old_land, Subtype::Swamp);
    }
    let original_boundary = (
        wasm.game.turn.turn_number,
        wasm.game.effect_store.continuous_effects.current_timestamp(),
    );
    wasm.runner = Some(TurnRunner::new());
    let TurnAction::Decision(DecisionContext::Boolean(prompt)) = advance(&mut wasm).unwrap() else {
        panic!("expected the original untap to suspend before late registrations");
    };
    assert_eq!(prompt.source, Some(artifact));
    assert_eq!(wasm.game.turn_store.untap_step_started_at, None);
    assert_land(&wasm.game, selected_land, Subtype::Swamp);
    assert_land(&wasm.game, unselected_land, Subtype::Swamp);

    // B's originally selected expiry stays selected after moving to C.
    // C's originally unselected expiry does not join this occurrence after
    // moving to B; it awaits B's later actual untap under the inference above.
    wasm.game.set_current_controller(selected_land, third).unwrap();
    wasm.game.set_current_controller(unselected_land, affected).unwrap();

    // These native registrations model work performed while the owning
    // runner is suspended. They must not retroactively alter its occurrence.
    resolve_body(&mut wasm.game, "Misstep", caster, ResolvedTarget::Player(affected));
    resolve_body(&mut wasm.game, "Orcish Farmer", caster, ResolvedTarget::Object(land));
    wasm.game.skip_next_step(affected, Step::Untap);
    wasm.game.add_step_after(Step::Untap, Step::Untap);
    wasm.game.add_step_after(Step::Untap, Step::Untap);
    assert!(wasm.game.effect_store.continuous_effects.current_timestamp() > original_boundary.1);
    let suspended = RuntimeSavepoint::capture(&wasm);

    for restored in [false, true] {
        if restored { suspended.clone().restore(&mut wasm); }
        assert!(wasm.runner_pending_decision);
        assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Untap));
        assert_eq!(wasm.game.turn_store.untap_step_started_at, None);
        assert_eq!(wasm.game.current_controller(selected_land), Some(third));
        assert_eq!(wasm.game.current_controller(unselected_land), Some(affected));
        assert_land(&wasm.game, selected_land, Subtype::Swamp);
        assert_land(&wasm.game, unselected_land, Subtype::Swamp);
        assert!(matches!(advance(&mut wasm).unwrap(),
            TurnAction::Decision(DecisionContext::Boolean(_))));
        wasm.runner.as_mut().unwrap().respond_boolean(true);
        assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
        assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::UntapEndMana));
        assert_eq!(wasm.game.turn_store.untap_step_started_at, Some(original_boundary));
        assert!(!wasm.game.is_tapped(creature), "the late Misstep misses the resumed occurrence");
        assert!(!wasm.game.is_tapped(land));
        assert!(!wasm.game.is_tapped(artifact));
        assert_land(&wasm.game, land, Subtype::Swamp);
        assert_land(&wasm.game, selected_land, Subtype::Forest);
        assert!(wasm.game.is_tapped(selected_land), "C's land does not untap with B");
        assert_land(&wasm.game, unselected_land, Subtype::Swamp);
        assert!(!wasm.game.is_tapped(unselected_land));
        assert_misstep_pending(&wasm.game, affected);
        assert!(wasm.game.effect_store.restriction_effects.iter().all(|effect|
            !effect.is_active(&wasm.game, wasm.game.turn.turn_number)));
        assert_eq!(wasm.game.pending_step_skips(affected, Step::Untap), 1,
            "a resumed step cannot consume a skip registered after it began");

        wasm.game.tap(creature);
        wasm.game.tap(land);
        wasm.game.tap(unselected_land);
        assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
        assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Untap));
        assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
        assert_eq!(wasm.game.pending_step_skips(affected, Step::Untap), 0);
        assert_eq!(wasm.game.turn_store.untap_step_started_at, Some(original_boundary),
            "the skipped added untap must not manufacture an occurrence receipt");
        assert!(wasm.game.is_tapped(creature));
        assert!(wasm.game.is_tapped(land));
        assert_land(&wasm.game, land, Subtype::Swamp);
        assert_land(&wasm.game, selected_land, Subtype::Forest);
        assert_land(&wasm.game, unselected_land, Subtype::Swamp);
        assert!(wasm.game.is_tapped(unselected_land));
        assert_misstep_pending(&wasm.game, affected);

        assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Untap));
        assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue));
        assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::UntapEndMana));
        let actual_boundary = wasm.game.turn_store.untap_step_started_at.unwrap();
        assert_eq!(actual_boundary.0, original_boundary.0);
        assert!(actual_boundary.1 > original_boundary.1);
        assert!(wasm.game.is_tapped(creature), "the later actual occurrence applies Misstep");
        assert!(!wasm.game.is_tapped(land));
        assert_land(&wasm.game, land, Subtype::Forest);
        assert_land(&wasm.game, selected_land, Subtype::Forest);
        assert!(wasm.game.is_tapped(selected_land));
        assert_land(&wasm.game, unselected_land, Subtype::Forest);
        assert!(!wasm.game.is_tapped(unselected_land));
        assert_misstep_consumed(&wasm.game);
        assert_eq!(wasm.game.pending_step_skips(affected, Step::Untap), 0);
    }
}

fn failing_optional_untap() -> (WasmGame, PlayerId, ObjectId, ObjectId, ObjectId, ObjectId, ironsmith::replacement::ReplacementEffectId) {
    let (mut wasm, affected, creature, land, first) = armed_runtime(true);
    let second = optional_artifact(&mut wasm.game, affected);
    wasm.game.tap(second);
    let replacement = wasm.game.effect_store.replacement_effects.add_one_shot_effect(
        ironsmith::replacement::ReplacementEffect::with_matcher(second, affected,
            ironsmith::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                ironsmith::target::ObjectFilter::specific(second)),
            ironsmith::replacement::ReplacementAction::Instead(vec![
                ironsmith::effect::Effect::gain_life(2),
                ironsmith::effect::Effect::lose_life(ironsmith::effect::Value::X),
            ]),
        ),
    );
    (wasm, affected, creature, land, first, second, replacement)
}

#[test]
fn optional_untap_answer_then_replacement_error_keeps_prefix_response_and_boundary_for_native_retry() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, affected, creature, land, first, second, replacement) = failing_optional_untap();
    let TurnAction::Decision(DecisionContext::Boolean(prompt)) = advance(&mut wasm).unwrap() else {
        panic!("first optional prompt");
    };
    assert_eq!(prompt.source, Some(first));
    wasm.runner.as_mut().unwrap().respond_boolean(false);
    let TurnAction::Decision(DecisionContext::Boolean(prompt)) = advance(&mut wasm).unwrap() else {
        panic!("second optional prompt");
    };
    assert_eq!(prompt.source, Some(second));
    wasm.runner.as_mut().unwrap().respond_boolean(true);
    let answered = RuntimeSavepoint::capture(&wasm);
    for _ in 0..2 {
        assert!(matches!(advance(&mut wasm), Err(ironsmith::game_loop::GameLoopError::ResolutionFailed(_))),
            "a direct retry must reuse the same submitted true answer, not re-prompt");
        assert!(wasm.runner.as_ref().unwrap().has_pending_untap_continuation());
        assert!(wasm.runner.as_ref().unwrap().has_pending_replay_choice());
        assert!(matches!(wasm.runner.as_ref().unwrap().state(), TurnState::Untap));
        assert_armed(&wasm.game, affected, creature, land);
        assert!(wasm.game.is_tapped(first));
        assert!(wasm.game.is_tapped(second));
        assert_eq!(wasm.game.player(affected).unwrap().life, 20);
        assert!(wasm.game.effect_store.replacement_effects.get_effect(replacement).is_some());
        assert!(wasm.game.take_pending_trigger_events().is_empty());
    }
    for restored in [false, true] {
        if restored { answered.clone().restore(&mut wasm); }
        wasm.game.effect_store.replacement_effects.remove_effect(replacement);
        assert!(matches!(advance(&mut wasm).unwrap(), TurnAction::Continue),
            "retry completion must not ask again for either retained optional answer");
        assert!(wasm.game.is_tapped(first), "the earlier false answer remains part of the prefix");
        assert!(!wasm.game.is_tapped(second), "the submitted true response survives the failed attempt");
        assert!(wasm.game.is_tapped(creature));
        assert!(!wasm.game.is_tapped(land));
        assert_land(&wasm.game, land, Subtype::Forest);
        assert_misstep_consumed(&wasm.game);
        assert!(!wasm.runner.as_ref().unwrap().has_pending_untap_continuation());
    }
}

#[test]
fn ordinary_boolean_host_error_restores_the_last_prompt_without_losing_earlier_untap_answers() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, affected, creature, land, first, second, replacement) = failing_optional_untap();
    let TurnAction::Decision(first_prompt @ DecisionContext::Boolean(_)) = advance(&mut wasm).unwrap() else {
        panic!("first optional prompt");
    };
    // Use the actual runner-command owner, excluding only JS snapshot encoding.
    // Model dispatch taking the prompt before routing the ordinary Boolean.
    wasm.pending_decision = None;
    wasm.runner_pending_decision = false;
    wasm.apply_runner_decision(first_prompt, UiCommand::SelectOptions { option_indices: vec![0] }).unwrap();
    let second_prompt = wasm.pending_decision.clone().expect("second prompt");
    let DecisionContext::Boolean(prompt) = &second_prompt else {
        panic!("the accepted first answer must lead to the second prompt");
    };
    assert_eq!(prompt.source, Some(second));
    assert!(wasm.payment_disclosure.is_none(), "this ordinary Boolean has no disclosure transaction");
    let pending = RuntimeSavepoint::capture(&wasm);
    for restored in [false, true] {
        if restored { pending.clone().restore(&mut wasm); }
        for _ in 0..2 {
            wasm.pending_decision = None;
            wasm.runner_pending_decision = false;
            assert!(wasm.apply_runner_decision(second_prompt.clone(), UiCommand::SelectOptions { option_indices: vec![1] }).is_err());
            assert!(wasm.runner_pending_decision);
            assert!(!wasm.runner_awaiting_priority);
            let Some(DecisionContext::Boolean(ref prompt)) = wasm.pending_decision else { panic!("retry prompt was lost"); };
            assert_eq!(prompt.source, Some(second), "do not rewind to the already answered first prompt");
            assert!(wasm.runner.as_ref().unwrap().has_pending_untap_continuation());
            assert!(wasm.runner.as_ref().unwrap().has_pending_replay_choice());
            assert_armed(&wasm.game, affected, creature, land);
            assert_eq!(wasm.game.player(affected).unwrap().life, 20);
            assert!(wasm.game.is_tapped(first));
            assert!(wasm.game.is_tapped(second));
            assert!(wasm.game.effect_store.replacement_effects.get_effect(replacement).is_some());
        }
        wasm.game.effect_store.replacement_effects.remove_effect(replacement);
        wasm.pending_decision = None;
        wasm.runner_pending_decision = false;
        wasm.apply_runner_decision(second_prompt.clone(), UiCommand::SelectOptions { option_indices: vec![1] }).unwrap();
        assert!(wasm.game.is_tapped(first), "host recovery retains the earlier false answer");
        assert!(!wasm.game.is_tapped(second));
        assert_land(&wasm.game, land, Subtype::Forest);
        assert_misstep_consumed(&wasm.game);
        assert!(!wasm.runner.as_ref().unwrap().has_pending_untap_continuation());
    }
}
