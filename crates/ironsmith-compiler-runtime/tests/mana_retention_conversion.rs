//! Source-authored only. All compilation and execution are deferred.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{DecisionContext, SelectOptionsContext};
use ironsmith::effects::EffectContext;
use ironsmith::effects::{AddManaEffect, EffectExecutor};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::{ManaPaymentRequest, check_mana_payment, plan_first_mana_payment, execute_mana_payment_plan};
use ironsmith::target::PlayerFilter;
use ironsmith::turn_runner::{TurnRunner, TurnState, TurnAction};
use ironsmith::{GameState, ObjectId, PlayerId, Zone, Phase, Step};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0); const B: PlayerId = PlayerId(1);
const COMPLETE: [&str; 5] = ["Horizon Stone", "Kruphix, God of Horizons", "Ozai, the Phoenix King", "Omnath, Locus of Mana", "Omnath, Locus of the Void"];
fn rows() -> Vec<serde_json::Value> { serde_json::from_str(include_str!("../../../fixtures/mana_retention_conversion.json.fixture")).unwrap() }
fn definitions(name: &str) -> Vec<CardDefinition> {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text()); artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(); assert_eq!(artifact, restored);
    vec![direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None; game
}
fn card(game: &mut GameState, owner: PlayerId, text: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition("Mana fixture", text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn land(game: &mut GameState, owner: PlayerId, types: &str) -> ObjectId {
    card(game, owner, &format!("Type: {types}\n{{T}}: Add {{G}}{{G}}."), Zone::Battlefield)
}
fn produce(game: &mut GameState, source: ObjectId, player: PlayerId, mana: Vec<ManaSymbol>) {
    AddManaEffect::new(mana, PlayerFilter::Specific(player))
        .execute(game, &mut EffectContext::new_default(source, player)).unwrap();
}
fn pt(game: &GameState, source: ObjectId) -> (i32, i32) {
    let chars = game.current_characteristics(source).unwrap(); (chars.power.unwrap(), chars.toughness.unwrap())
}
fn has(game: &GameState, source: ObjectId, id: ironsmith::static_abilities::StaticAbilityId) -> bool {
    game.current_has_static_ability_id(source, id)
}
fn settle(game: &mut GameState, queue: &mut ironsmith::triggers::TriggerQueue) {
    for _ in 0..30 {
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, &mut SelectFirstDecisionMaker).unwrap();
        if game.stack_is_empty() { return; }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("bounded retention scenario failed to settle");
}
#[test]
fn five_complete_bodies_are_strict_direct_and_artifact_programs() {
    for name in COMPLETE { assert_eq!(definitions(name).len(), 2); }
    // Omnath, Locus of All is intentionally not a complete-card proposal here:
    // its optional filtered reveal and mana-from-revealed-colors body is open.
    assert_eq!(rows().len(), 6);
}
#[test]
fn live_conversion_preserves_existing_units_without_production_and_obeys_source_presence() {
    for (name, output) in [("Horizon Stone", ManaSymbol::Colorless), ("Kruphix, God of Horizons", ManaSymbol::Colorless),
        ("Ozai, the Phoenix King", ManaSymbol::Red), ("Omnath, Locus of the Void", ManaSymbol::Colorless)] {
        for definition in definitions(name) {
            let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            produce(&mut game, source, A, vec![ManaSymbol::Blue, ManaSymbol::Green, ManaSymbol::Colorless]);
            produce(&mut game, source, B, vec![ManaSymbol::White]);
            let before = game.turn_store.turn_history.cards_drawn_by_player(A);
            game.empty_mana_pools().unwrap();
            assert_eq!(game.player(A).unwrap().mana_pool.amount(output), 3);
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(A), before);
            assert!(game.take_pending_trigger_events().iter().all(|event| event.downcast::<ironsmith::events::ManaAddedEvent>().is_none()));
            game.phase_out(source);
            game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            game.phase_in(source); game.set_current_controller(source, B).unwrap();
            produce(&mut game, source, A, vec![ManaSymbol::Green]); produce(&mut game, source, B, vec![ManaSymbol::Blue]);
            game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(B).unwrap().mana_pool.amount(output), 1);
            game.move_object_by_effect(source, Zone::Graveyard); game.empty_mana_pools().unwrap();
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        }
    }
}
#[test]
fn overlapping_converters_pause_runner_without_emptying_any_players_pool() {
    for first in definitions("Horizon Stone") { for second in definitions("Ozai, the Phoenix King") {
        let mut game = game(); game.create_object_from_definition(&first, B, Zone::Battlefield);
        let red = game.create_object_from_definition(&second, B, Zone::Battlefield);
        produce(&mut game, red, A, vec![ManaSymbol::Green]); produce(&mut game, red, B, vec![ManaSymbol::Blue; 2]);
        game.turn.phase = Phase::Beginning; game.turn.step = Some(Step::Upkeep);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::UpkeepPriority);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let TurnAction::Decision(DecisionContext::SelectOptions(context)) = runner.advance(&mut game, &mut queue).unwrap() else {panic!("affected-player choice")};
        assert_eq!(context.player, B); assert!(runner.has_pending_replay_choice());
        assert_eq!(game.player(A).unwrap().mana_pool.green, 1); assert_eq!(game.player(B).unwrap().mana_pool.blue, 2);
        let choice = context.options.iter().find(|option| option.object_id == Some(red)).unwrap().index;
        runner.respond_options(vec![choice]); assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0); assert_eq!(game.player(B).unwrap().mana_pool.red, 2);
        assert!(!runner.has_pending_replay_choice());
    } }
}
#[test]
fn same_color_retention_and_conversion_are_distinct_and_forced_loss_uses_the_real_event() {
    for definition in definitions("Omnath, Locus of Mana") {
        let mut game = game(); let omnath = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        produce(&mut game, omnath, A, vec![ManaSymbol::Green; 3]); produce(&mut game, omnath, A, vec![ManaSymbol::Blue; 2]);
        assert_eq!(pt(&game, omnath), (4, 4));
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.green, 3);
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 0); assert_eq!(pt(&game, omnath), (4, 4));
        let stone = game.create_object_from_definition(&definitions("Horizon Stone")[0], A, Zone::Battlefield);
        produce(&mut game, omnath, A, vec![ManaSymbol::Blue]); game.empty_mana_pools().unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.green, 3); assert_eq!(game.player(A).unwrap().mana_pool.colorless, 1);
        ironsmith::effects::EmptyManaPoolEffect::new(PlayerFilter::You)
            .execute(&mut game, &mut EffectContext::new_default(stone, A)).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.colorless, 4); assert_eq!(pt(&game, omnath), (1, 1));
        game.move_object_by_effect(stone, Zone::Graveyard);
        ironsmith::effects::EmptyManaPoolEffect::new(PlayerFilter::You)
            .execute(&mut game, &mut EffectContext::new_default(omnath, A)).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn total_unspent_anthem_and_ozai_threshold_refresh_after_real_production_and_payment() {
    for definition in definitions("Omnath, Locus of the Void") {
        let mut game = game(); let omnath = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(pt(&game, omnath), (6, 6));
        produce(&mut game, omnath, A, vec![ManaSymbol::White, ManaSymbol::Black, ManaSymbol::Colorless]);
        assert_eq!(pt(&game, omnath), (9, 9)); game.empty_mana_pools().unwrap(); assert_eq!(pt(&game, omnath), (9, 9));
        let request = ManaPaymentRequest::new(A, omnath, ironsmith::costs::PaymentReason::Other,
            ManaCost::from_pips(vec![vec![ManaSymbol::Colorless], vec![ManaSymbol::Generic(1)]]));
        assert!(check_mana_payment(&game, &request).is_ok());
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(pt(&game, omnath), (7, 7));
    }
    for definition in definitions("Ozai, the Phoenix King") {
        let mut game = game(); let ozai = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        use ironsmith::static_abilities::StaticAbilityId as Id;
        assert!(!has(&game, ozai, Id::Flying)); assert!(!has(&game, ozai, Id::Indestructible));
        assert!(has(&game, ozai, Id::Trample)); assert!(has(&game, ozai, Id::Haste));
        produce(&mut game, ozai, A, vec![ManaSymbol::Green; 6]);
        assert!(has(&game, ozai, Id::Flying)); assert!(has(&game, ozai, Id::Indestructible));
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.red, 6);
        let request = ManaPaymentRequest::new(A, ozai, ironsmith::costs::PaymentReason::Other, ManaCost::from_pips(vec![vec![ManaSymbol::Red]]));
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!has(&game, ozai, Id::Flying)); assert!(!has(&game, ozai, Id::Indestructible));
    }
}
#[test]
fn kruphix_keeps_its_devotion_type_rule_indestructible_and_no_maximum_hand_size() {
    for definition in definitions("Kruphix, God of Horizons") {
        let mut game = game(); let kruphix = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_is_creature(kruphix));
        assert!(has(&game, kruphix, ironsmith::static_abilities::StaticAbilityId::Indestructible));
        assert_eq!(game.player(A).unwrap().max_hand_size, i32::MAX);
        let devotion = card(&mut game, A, "Mana cost: {G}{G}{G}{U}{U}\nType: Enchantment", Zone::Battlefield);
        game.refresh_continuous_state().unwrap(); assert!(game.current_is_creature(kruphix));
        game.phase_out(devotion); game.refresh_continuous_state().unwrap();
        assert!(!game.current_is_creature(kruphix));
        game.phase_out(kruphix); game.refresh_continuous_state().unwrap();
        assert_eq!(game.player(A).unwrap().max_hand_size, 7);
    }
}
#[test]
fn void_landfall_and_ozai_firebending_execute_their_real_secondary_bodies() {
    for definition in definitions("Omnath, Locus of the Void") {
        let mut game = game(); let omnath = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let land = card(&mut game, A, "Type: Basic Land — Wastes", Zone::Hand);
        let receipt = game.move_object_with_etb_processing_with_dm(land, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!receipt.pending); assert!(receipt.programs.is_empty()); assert!(receipt.original.into_result().is_some());
        let mut queue = ironsmith::triggers::TriggerQueue::new(); settle(&mut game, &mut queue);
        assert_eq!(game.player(A).unwrap().mana_pool.colorless, 2); assert_eq!(pt(&game, omnath), (8, 8));
        let foreign = card(&mut game, B, "Type: Basic Land — Wastes", Zone::Hand);
        let receipt = game.move_object_with_etb_processing_with_dm(foreign, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!receipt.pending); assert!(receipt.original.into_result().is_some()); settle(&mut game, &mut queue);
        assert_eq!(game.player(A).unwrap().mana_pool.colorless, 2);
    }
    for definition in definitions("Ozai, the Phoenix King") {
        let mut game = game(); let ozai = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(ozai); game.mark_combat_phase_started();
        game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareAttackers);
        let mut queue = ironsmith::triggers::TriggerQueue::new(); let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::game_loop::apply_attacker_declarations(&mut game, &mut combat, &mut queue,
            &[ironsmith::decision::AttackerDeclaration {creature: ozai, target: ironsmith::combat_state::AttackTarget::Player(B)}]).unwrap();
        game.combat = Some(combat); settle(&mut game, &mut queue);
        assert_eq!(game.player(A).unwrap().mana_pool.red, 4);
        // Ordinary red mana converts, whereas the firebending units keep
        // their exact until-end-of-combat duration until that boundary.
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.red, 4);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::EndCombatPriority);
        game.turn.step = Some(Step::EndCombat);
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
        assert_eq!(game.player(A).unwrap().mana_pool.red, 4);
        game.move_object_by_effect(ozai, Zone::Graveyard);
        game.turn.phase = Phase::NextMain; game.turn.step = None;
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn skipped_end_combat_and_end_combat_procedure_do_not_leave_firebending_units_indefinite() {
    for procedure in [false, true] { for definition in definitions("Ozai, the Phoenix King") {
        let mut game = game(); let ozai = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.turn.phase = Phase::Combat; game.turn.step = Some(Step::CombatDamage);
        ironsmith::effects::ManaRetainedEffect::until_end_of_combat(vec![
            ironsmith::effect::Effect::new(AddManaEffect::you(vec![ManaSymbol::Red; 4]))
        ]).execute(&mut game, &mut EffectContext::new_default(ozai, A)).unwrap();
        if !procedure { game.skip_next_step(A, Step::EndCombat); }
        let state = if procedure { TurnState::EndCombatPhaseSbas } else { TurnState::EndCombat };
        let mut runner = TurnRunner::from_state_for_sync(state); let mut queue = ironsmith::triggers::TriggerQueue::new();
        for _ in 0..3 {
            assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue | TurnAction::RunPriority));
            if matches!(game.turn.phase, Phase::NextMain) { break; }
        }
        assert_eq!(game.player(A).unwrap().mana_pool.red, 4);
        game.move_object_by_effect(ozai, Zone::Graveyard);
        game.turn.phase = Phase::NextMain; game.turn.step = None;
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    } }
}
#[test]
fn cleanup_expires_turn_retention_before_loss_and_keeps_mana_until_the_step_ends() {
    for definition in definitions("Horizon Stone") {
        let mut game = game(); let stone = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        ironsmith::effects::RetainManaUntilEndOfTurnEffect {
            player: PlayerFilter::You,
            color: None,
        }
            .execute(&mut game, &mut EffectContext::new_default(stone, A)).unwrap();
        produce(&mut game, stone, A, vec![ManaSymbol::Blue; 2]);
        game.turn.phase = Phase::Ending; game.turn.step = Some(Step::Cleanup);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::CleanupApply);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 2, "514.2 does not itself empty pools");
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
        assert_eq!(game.player(A).unwrap().mana_pool.colorless, 2); assert_eq!(game.player(A).unwrap().mana_pool.blue, 0);
        // Retention from the preceding turn cannot revive after the converter leaves.
        game.move_object_by_effect(stone, Zone::Graveyard); game.turn.phase = Phase::Beginning; game.turn.step = Some(Step::Untap);
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn large_unspent_scalars_are_explicit_errors_in_queries_and_execution() {
    use ironsmith::effects::ExecutionError;
    use ironsmith::static_ability_processor::StaticEffectDiscoveryError;
    for name in ["Ozai, the Phoenix King", "Omnath, Locus of Mana", "Omnath, Locus of the Void"] {
        for definition in definitions(name) {
            let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            game.player_mut(A).unwrap().mana_pool.green = i32::MAX as u32 + 1;
            assert!(matches!(game.refresh_continuous_state(), Err(StaticEffectDiscoveryError::ScalarRange { .. })));
            assert!(matches!(game.try_current_characteristics(source), Err(StaticEffectDiscoveryError::ScalarRange { .. })));
            assert!(ironsmith::decision::compute_legal_actions(&game, A).is_err(), "unknown is not a completed action list");
            let ctx = EffectContext::new_default(source, A);
            assert!(matches!(ironsmith::effects::helpers::resolve_value(&game,
                &ironsmith::effect::Value::UnspentMana(PlayerFilter::You), &ctx), Err(ExecutionError::ResourceLimitExceeded { .. })));
            game.player_mut(A).unwrap().mana_pool.green = 501;
            game.refresh_continuous_state().unwrap();
            assert!(game.try_current_characteristics(source).unwrap().is_some(), "a prior error does not poison a later complete query");
        }
    }
}
#[test]
fn representable_mana_count_cannot_wrap_base_power_or_publish_a_successful_credit() {
    use ironsmith::effects::ExecutionError;
    use ironsmith::static_ability_processor::StaticEffectDiscoveryError;
    for (name, base) in [("Omnath, Locus of Mana", 1i32), ("Omnath, Locus of the Void", 6i32)] {
        for definition in definitions(name) {
            let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let safe = (i32::MAX - base) as u32;
            game.player_mut(A).unwrap().mana_pool.green = safe;
            game.refresh_continuous_state().unwrap(); assert_eq!(pt(&game, source), (i32::MAX, i32::MAX));
            let result = AddManaEffect::you(vec![ManaSymbol::Green])
                .execute(&mut game, &mut EffectContext::new_default(source, A));
            assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(StaticEffectDiscoveryError::ScalarRange { .. }))));
            assert_eq!(game.player(A).unwrap().mana_pool.green, safe, "invalid production is rolled back");
            assert_eq!(pt(&game, source), (i32::MAX, i32::MAX));
            game.object_mut(source).unwrap().add_counters(ironsmith::CounterType::PlusOnePlusOne, 1);
            assert!(matches!(game.refresh_continuous_state(), Err(StaticEffectDiscoveryError::ScalarRange { .. })),
                "counter addition cannot wrap a mana-derived P/T result");
        }
    }
}
#[test]
fn wide_pool_totals_reject_cross_color_and_cross_player_overflow() {
    use ironsmith::static_ability_processor::StaticEffectDiscoveryError;
    let mut game = game();
    game.player_mut(A).unwrap().mana_pool.white = u32::MAX;
    game.player_mut(A).unwrap().mana_pool.blue = u32::MAX;
    assert_eq!(game.player(A).unwrap().mana_pool.total_wide(), u64::from(u32::MAX) * 2);
    assert!(matches!(game.refresh_continuous_state(), Err(StaticEffectDiscoveryError::ScalarRange { .. })));
    game.player_mut(A).unwrap().mana_pool = Default::default();
    game.player_mut(A).unwrap().mana_pool.green = i32::MAX as u32;
    game.player_mut(B).unwrap().mana_pool.colorless = 1;
    assert!(matches!(game.refresh_continuous_state(), Err(StaticEffectDiscoveryError::ScalarRange { .. })));
}

#[test]
fn ordinary_pt_range_markers_cannot_escape_a_successful_checked_frame_without_a_mana_anthem() {
    use ironsmith::effects::{ExecutionError, execute_effect};
    use ironsmith::static_ability_processor::StaticEffectDiscoveryError;
    for counters in [false, true] {
        let mut game = game();
        let source = card(&mut game, A, &format!("Type: Creature\nPower/Toughness: {0}/{0}", i32::MAX), Zone::Battlefield);
        let other = card(&mut game, B, "Type: Creature\nPower/Toughness: 1/1", Zone::Battlefield);
        if counters { game.object_mut(source).unwrap().add_counters(ironsmith::CounterType::PlusOnePlusOne, 1); }
        else {
            game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::new(
                source, A, ironsmith::continuous::EffectTarget::Specific(source),
                ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 }));
        }
        assert!(matches!(game.refresh_continuous_state(), Err(StaticEffectDiscoveryError::ScalarRange { .. })));
        assert!(game.continuous_query_snapshot().is_err());
        let fight = ironsmith::effect::Effect::fight(ironsmith::target::ChooseSpec::SpecificObject(source),
            ironsmith::target::ChooseSpec::SpecificObject(other));
        assert!(matches!(execute_effect(&mut game, &fight, &mut EffectContext::new_default(source, A)),
            Err(ExecutionError::ContinuousDiscovery(StaticEffectDiscoveryError::ScalarRange { .. }))));
        assert_eq!(game.damage_on(source), 0);
        assert_eq!(game.damage_on(other), 0);
    }
}
