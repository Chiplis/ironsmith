//! Explicit representation failures roll back damage, never cap gameplay.
//! Source-authored regressions; no execution in the implementation campaign.
use ironsmith::effects::{
    DealDamageEffect, EffectContext as ExecutionContext, EffectExecutor, ExecutionError,
};
use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn setup(keyword: &str) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 30);
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Damage source",
        &format!("Type: Creature — Beast\nPower/Toughness: 3/3\n{keyword}"),
        false,
    )
    .unwrap();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    game.take_pending_trigger_events();
    (game, source)
}
fn add(game: &mut GameState, source: ObjectId, action: ReplacementAction) {
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            source,
            A,
            ironsmith::events::DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
            action,
        ));
}
fn damage(
    game: &mut GameState,
    source: ObjectId,
    amount: i32,
    target: ChooseSpec,
) -> Result<ironsmith::effect::EffectOutcome, ExecutionError> {
    DealDamageEffect::new(amount, target)
        .execute(game, &mut ExecutionContext::new_default(source, A))
}
fn incomplete(error: ExecutionError) {
    assert!(error.is_incomplete_execution(), "{error:?}");
    assert!(matches!(
        error,
        ExecutionError::ResourceLimitExceeded { .. }
    ));
}
#[test]
fn multiply_and_double_overflow_restore_consumed_replacements_and_history() {
    for action in [
        ReplacementAction::Double,
        ReplacementAction::Modify(EventModification::Multiply(3)),
    ] {
        let (mut game, source) = setup("");
        add(&mut game, source, action.clone());
        incomplete(damage(&mut game, source, i32::MAX, ChooseSpec::SpecificPlayer(B)).unwrap_err());
        assert_eq!(game.player(B).unwrap().life, 30);
        assert_eq!(
            game.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::Damage),
            0
        );
        assert_eq!(
            game.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::LifeLoss),
            0
        );
        let factor = if matches!(action, ReplacementAction::Double) {
            2
        } else {
            3
        };
        assert_eq!(
            damage(&mut game, source, 1, ChooseSpec::SpecificPlayer(B))
                .unwrap()
                .as_count(),
            Some(factor)
        );
        assert_eq!(
            damage(&mut game, source, 1, ChooseSpec::SpecificPlayer(B))
                .unwrap()
                .as_count(),
            Some(1)
        );
    }
}
#[test]
fn additive_u32_overflow_is_not_a_successful_smaller_event() {
    let (mut game, source) = setup("");
    add(
        &mut game,
        source,
        ReplacementAction::Modify(EventModification::SetTo(u32::MAX)),
    );
    add(
        &mut game,
        source,
        ReplacementAction::Modify(EventModification::Add(1)),
    );
    incomplete(damage(&mut game, source, 1, ChooseSpec::SpecificPlayer(B)).unwrap_err());
    assert_eq!(game.player(B).unwrap().life, 30);
    assert!(game.stack.is_empty());
    assert_eq!(
        game.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::Damage),
        0
    );
}
#[test]
fn exact_supported_damage_boundary_and_one_larger_result() {
    let (mut game, source) = setup("");
    assert_eq!(
        damage(&mut game, source, i32::MAX, ChooseSpec::SpecificPlayer(B))
            .unwrap()
            .as_count(),
        Some(i64::from(i32::MAX))
    );
    assert_eq!(game.player(B).unwrap().life, 30 - i32::MAX);
    let (mut game, source) = setup("");
    add(&mut game, source, ReplacementAction::Double);
    assert_eq!(
        damage(&mut game, source, 1 << 30, ChooseSpec::SpecificPlayer(B))
            .unwrap()
            .as_count(),
        Some(1_i64 << 31)
    );
    assert_eq!(i64::from(game.player(B).unwrap().life), 30 - (1_i64 << 31));

    // Damage events and marked damage use unsigned 32-bit values; outcomes
    // remain wide. Exceeding that actual boundary must still roll back.
    let (mut game, source) = setup("");
    add(
        &mut game,
        source,
        ReplacementAction::Modify(EventModification::SetTo(u32::MAX)),
    );
    assert_eq!(
        damage(&mut game, source, 1, ChooseSpec::SpecificObject(source))
            .unwrap()
            .as_count(),
        Some(i64::from(u32::MAX))
    );
    assert_eq!(game.damage_on(source), u32::MAX);
    game.set_damage_marked(source, 0);
    add(
        &mut game,
        source,
        ReplacementAction::Modify(EventModification::SetTo(u32::MAX)),
    );
    add(
        &mut game,
        source,
        ReplacementAction::Modify(EventModification::Add(1)),
    );
    incomplete(damage(&mut game, source, 1, ChooseSpec::SpecificObject(source)).unwrap_err());
    assert_eq!(game.damage_on(source), 0);
}
#[test]
fn signed_life_lifelink_counter_and_marked_damage_state_bounds_roll_back() {
    let (mut game, source) = setup("");
    game.player_mut(B).unwrap().life = i32::MIN + 1;
    incomplete(damage(&mut game, source, 2, ChooseSpec::SpecificPlayer(B)).unwrap_err());
    assert_eq!(game.player(B).unwrap().life, i32::MIN + 1);
    let (mut game, source) = setup("Lifelink");
    game.player_mut(A).unwrap().life = i32::MAX;
    incomplete(damage(&mut game, source, 1, ChooseSpec::SpecificPlayer(B)).unwrap_err());
    assert_eq!(game.player(A).unwrap().life, i32::MAX);
    assert_eq!(game.player(B).unwrap().life, 30);
    let (mut game, source) = setup("Infect");
    game.add_player_counters_with_source(
        B,
        ironsmith::CounterType::Poison,
        u32::MAX - 1,
        None,
        None,
    )
    .unwrap();
    incomplete(damage(&mut game, source, 2, ChooseSpec::SpecificPlayer(B)).unwrap_err());
    assert_eq!(
        game.player(B)
            .unwrap()
            .counter_count(ironsmith::CounterType::Poison),
        u32::MAX - 1
    );
    let (mut game, source) = setup("");
    game.set_damage_marked(source, u32::MAX - 1);
    incomplete(damage(&mut game, source, 2, ChooseSpec::SpecificObject(source)).unwrap_err());
    assert_eq!(game.damage_on(source), u32::MAX - 1);
}
#[test]
fn combat_representation_error_restores_whole_step_and_replacement() {
    let (mut game, source) = setup("");
    add(
        &mut game,
        source,
        ReplacementAction::Modify(EventModification::Multiply(u32::MAX)),
    );
    let combat = ironsmith::combat_state::CombatState {
        attackers: vec![ironsmith::combat_state::AttackerInfo {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
        ..Default::default()
    };
    let error = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
        &mut game,
        &combat,
        false,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap_err();
    let ironsmith::game_loop::CombatDamageAssignmentErrorKind::Execution(error) = error.kind else {
        panic!("expected execution failure");
    };
    incomplete(error);
    assert_eq!(game.player(B).unwrap().life, 30);
    assert_eq!(
        game.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::Damage),
        0
    );
    let repeated = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
        &mut game,
        &combat,
        false,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap_err();
    assert!(matches!(
        repeated.kind,
        ironsmith::game_loop::CombatDamageAssignmentErrorKind::Execution(
            ExecutionError::ResourceLimitExceeded { .. }
        )
    ));
}

#[test]
fn another_action_preserves_wide_current_turn_damage_history() {
    let (mut game, source) = setup("");
    damage(
        &mut game,
        source,
        i32::MAX,
        ChooseSpec::SpecificObject(source),
    )
    .unwrap();
    // Marked damage is reset separately here; the completed turn history remains.
    game.set_damage_marked(source, 0);
    assert_eq!(
        damage(&mut game, source, 1, ChooseSpec::SpecificObject(source))
            .unwrap()
            .as_count(),
        Some(1)
    );
    assert_eq!(game.damage_on(source), 1);
    assert_eq!(
        game.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::Damage),
        2
    );
    let history = &game.turn_store.turn_history;
    let total: u64 = history
        .event_records
        .iter()
        .chain(history.staged_event_records.iter())
        .filter_map(|record| record.event.downcast::<ironsmith::events::DamageEvent>())
        .map(|event| u64::from(event.amount))
        .sum();
    assert_eq!(total, i32::MAX as u64 + 1);
}
