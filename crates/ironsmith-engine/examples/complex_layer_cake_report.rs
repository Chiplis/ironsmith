use ironsmith::bench_support::{EffectMix, battlefield_scale, complex_layer_cake_stress_report};
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::game_loop::{
    advance_priority_with_dm, drain_pending_trigger_events, generate_and_queue_step_triggers,
    last_priority_advance_perf,
};
use ironsmith::rules::state_based::{
    apply_legend_rule_choice, check_state_based_actions, get_legend_rule_specs,
};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{
    AutoPassDecisionMaker, CardType, Color, ColorSet, GameState, ObjectId, Phase, Step, Subtype,
    Supertype, TriggerQueue, TurnAction, TurnRunner, Zone, execute_untap_step,
};
use std::time::Instant;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let verify = args.iter().any(|arg| arg == "--verify");
    let turn = args.iter().any(|arg| arg == "--turn");
    let legend = args.iter().any(|arg| arg == "--legend");
    let positional: Vec<_> = args
        .iter()
        .filter(|arg| !matches!(arg.as_str(), "--verify" | "--turn" | "--legend"))
        .collect();
    let creatures = positional
        .first()
        .map(|value| value.as_str())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(300);
    let effects = positional
        .get(1)
        .map(|value| value.as_str())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(12);

    if verify {
        verify_expected_characteristics(creatures, effects);
        return;
    }
    if turn {
        report_turn_transition(creatures, effects);
        return;
    }
    if legend {
        report_legend_choice(creatures, effects);
        return;
    }

    let report = complex_layer_cake_stress_report(creatures, effects);
    println!("{report:#?}");
}

fn report_legend_choice(creatures: usize, effects: usize) {
    let scenario = battlefield_scale(creatures, EffectMix::ComplexLayerCake(effects));
    let mut game = scenario.game;
    let controller = scenario.priority_player;
    let legend = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Scale Legend")
        .card_types(vec![CardType::Artifact])
        .supertypes(vec![Supertype::Legendary])
        .build();
    let mut legends = Vec::new();
    for _ in 0..6 {
        legends.push(game.create_object_from_definition(&legend, controller, Zone::Battlefield));
    }
    game.refresh_continuous_state();

    let mut trigger_queue = TriggerQueue::new();
    let find_started = Instant::now();
    let specs = get_legend_rule_specs(&game);
    let find_ms = find_started.elapsed().as_millis();

    let apply_started = Instant::now();
    let keep = specs
        .first()
        .and_then(|(_, spec)| spec.legends.first())
        .copied()
        .unwrap_or(legends[0]);
    apply_legend_rule_choice(&mut game, keep);
    let apply_ms = apply_started.elapsed().as_millis();
    let apply_counters = game.work_counters();

    let drain_started = Instant::now();
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    let drain_ms = drain_started.elapsed().as_millis();

    let refresh_started = Instant::now();
    game.refresh_continuous_state();
    let refresh_ms = refresh_started.elapsed().as_millis();

    let check_after_started = Instant::now();
    let actions2 = check_state_based_actions(&game);
    let check_after_ms = check_after_started.elapsed().as_millis();

    println!(
        "legend choice: creatures={creatures}, effects={effects}, battlefield={}, specs={}, find_ms={find_ms}, apply_ms={apply_ms}, drain_ms={drain_ms}, refresh_ms={refresh_ms}, check_after_ms={check_after_ms}, remaining_actions={}, counters={apply_counters:#?}",
        game.battlefield.len(),
        specs.len(),
        actions2.len()
    );
}

fn report_turn_transition(creatures: usize, effects: usize) {
    let direct_scenario = battlefield_scale(creatures, EffectMix::ComplexLayerCake(effects));
    let mut direct_game = direct_scenario.game;
    let mut direct_trigger_queue = TriggerQueue::new();
    direct_game.turn.phase = Phase::Beginning;
    direct_game.turn.step = Some(Step::Untap);
    let direct_untap_started = Instant::now();
    execute_untap_step(&mut direct_game);
    let direct_untap_ms = direct_untap_started.elapsed().as_millis();
    let direct_untap_counters = direct_game.work_counters();

    direct_game.turn.step = Some(Step::Upkeep);
    direct_game.turn.priority_player = Some(direct_game.turn.active_player);
    let upkeep_triggers_started = Instant::now();
    drain_pending_trigger_events(&mut direct_game, &mut direct_trigger_queue);
    generate_and_queue_step_triggers(&mut direct_game, &mut direct_trigger_queue);
    let upkeep_triggers_ms = upkeep_triggers_started.elapsed().as_millis();
    let upkeep_trigger_counters = direct_game.work_counters();

    let scenario = battlefield_scale(creatures, EffectMix::ComplexLayerCake(effects));
    let mut game = scenario.game;
    let mut trigger_queue = TriggerQueue::new();
    let mut runner = TurnRunner::new();
    let mut runner_iterations = 0usize;

    let runner_started = Instant::now();
    loop {
        runner_iterations += 1;
        match runner
            .advance(&mut game, &mut trigger_queue)
            .expect("runner advance should succeed")
        {
            TurnAction::Continue => continue,
            TurnAction::RunPriority => break,
            other => panic!("unexpected runner action before first priority: {other:?}"),
        }
    }
    let runner_ms = runner_started.elapsed().as_millis();
    let runner_counters = game.work_counters();

    let mut dm = AutoPassDecisionMaker;
    let priority_started = Instant::now();
    let progress = advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("priority advance should succeed");
    let priority_ms = priority_started.elapsed().as_millis();
    let priority_counters = game.work_counters();

    println!(
        "turn transition: creatures={creatures}, effects={effects}, battlefield={}, runner_iterations={runner_iterations}, runner_ms={runner_ms}, priority_ms={priority_ms}, progress={progress:?}",
        game.battlefield.len()
    );
    println!(
        "direct_untap_ms={direct_untap_ms}, upkeep_triggers_ms={upkeep_triggers_ms}, direct_untap_counters={direct_untap_counters:#?}, upkeep_trigger_counters={upkeep_trigger_counters:#?}"
    );
    println!("runner_counters={runner_counters:#?}");
    println!("priority_counters={priority_counters:#?}");
    if let Some(perf) = last_priority_advance_perf() {
        println!("priority_perf={perf:#?}");
    }
}

fn verify_expected_characteristics(creatures: usize, effects: usize) {
    assert!(
        creatures >= 4,
        "expected at least four creatures to sample each synthetic subtype"
    );
    assert_eq!(
        effects, 72,
        "expected-characteristic verification is calibrated for 72 effects"
    );
    let scenario = battlefield_scale(creatures, EffectMix::ComplexLayerCake(effects));
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[0],
        Subtype::Goblin,
        27,
        15,
        ColorSet::GREEN.union(ColorSet::WHITE).union(ColorSet::RED),
        &[StaticAbilityId::Flying],
    );
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[1],
        Subtype::Elf,
        15,
        3,
        ColorSet::GREEN,
        &[StaticAbilityId::Vigilance],
    );
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[2],
        Subtype::Soldier,
        9,
        15,
        ColorSet::GREEN,
        &[StaticAbilityId::Haste],
    );
    assert_token_characteristics(
        &scenario.game,
        scenario.battlefield[3],
        Subtype::Zombie,
        15,
        3,
        ColorSet::GREEN.union(ColorSet::from_color(Color::Black)),
        &[],
    );
    println!(
        "verified representative characteristics for {creatures} creatures + {effects} effects"
    );
}

fn assert_token_characteristics(
    game: &GameState,
    id: ObjectId,
    subtype: Subtype,
    power: i32,
    toughness: i32,
    colors: ColorSet,
    static_ability_ids: &[StaticAbilityId],
) {
    let chars = game
        .calculated_characteristics(id)
        .expect("token should have calculated characteristics");
    assert_eq!(chars.power, Some(power));
    assert_eq!(chars.toughness, Some(toughness));
    assert!(chars.card_types.contains(&CardType::Creature));
    assert!(chars.card_types.contains(&CardType::Artifact));
    assert!(chars.subtypes.contains(&subtype));
    assert_eq!(chars.colors, colors);
    for ability_id in static_ability_ids {
        assert!(
            chars
                .static_abilities
                .iter()
                .any(|ability| ability.id() == *ability_id),
            "expected token #{:?} to have static ability {:?}, got {:?}",
            id,
            ability_id,
            chars.static_abilities
        );
    }
}
