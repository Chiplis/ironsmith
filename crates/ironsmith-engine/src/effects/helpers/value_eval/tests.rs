use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::continuous::{
    CalculationContext, ContinuousEffect, ContinuousEffectManager, EffectTarget, Modification,
    resolve_value_direct,
};
use crate::ids::CardId;
use crate::target::ObjectFilter;
use crate::types::CardType;

fn fixture() -> (GameState, ObjectId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let card = CardBuilder::new(CardId::from_raw(99102), "Value source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(-3, 5))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    (game, source, alice)
}
fn continuous(value: &Value, game: &GameState, source: ObjectId, controller: PlayerId) -> i32 {
    resolve_value_direct(
        value,
        game.objects_map(),
        &[],
        &game.battlefield,
        &HashSet::new(),
        source,
        controller,
        game,
    )
}

#[test]
fn nested_arithmetic_preserves_floor_and_contextual_x() {
    let (game, source, alice) = fixture();
    let mut exec = ExecutionContext::new_default(source, alice);
    exec.x_value = Some(7);
    let value = Value::Add(
        Box::new(Value::XTimes(2)),
        Box::new(Value::HalfRoundedDown(Box::new(Value::Fixed(-5)))),
    );
    assert_eq!(
        resolve(&value, &EvaluationContext::execution_context(&game, &exec)).unwrap(),
        11
    );
    assert_eq!(continuous(&value, &game, source, alice), -3);
    let value = Value::DividedRoundedDown(Box::new(Value::Fixed(-5)), 3);
    assert_eq!(continuous(&value, &game, source, alice), -2);
}

#[test]
fn division_by_zero_keeps_resolution_error() {
    let (game, source, alice) = fixture();
    let exec = ExecutionContext::new_default(source, alice);
    let value = Value::DividedRoundedDown(Box::new(Value::Fixed(1)), 0);
    assert!(
        matches!(resolve(&value, &EvaluationContext::execution_context(&game, &exec)), Err(ExecutionError::UnresolvableValue(reason)) if reason == "division by zero in dynamic value")
    );
}

#[test]
#[should_panic(expected = "unsupported continuous-effect value Fixed(1): division by zero")]
fn division_by_zero_keeps_layer_error() {
    let (game, source, alice) = fixture();
    continuous(
        &Value::DividedRoundedDown(Box::new(Value::Fixed(1)), 0),
        &game,
        source,
        alice,
    );
}

#[test]
fn nested_layer_values_read_supplied_effects() {
    let (game, source, alice) = fixture();
    let mut effects = ContinuousEffectManager::new();
    effects.add_effect(ContinuousEffect::new(
        source,
        alice,
        EffectTarget::Specific(source),
        Modification::ModifyPowerToughness {
            power: 8,
            toughness: 2,
        },
    ));
    let calculation = CalculationContext {
        objects: game.objects_map(),
        effects: &effects,
        battlefield: &game.battlefield,
        game: &game,
        current_object: source,
    };
    let value = Value::Add(
        Box::new(Value::SourcePower),
        Box::new(Value::TotalToughness(ObjectFilter::creature())),
    );
    assert_eq!(
        resolve_continuous(&value, LayerValueContext::new(&calculation, source, alice)),
        12
    );
    assert_eq!(
        resolve_value_direct(
            &value,
            game.objects_map(),
            effects.effects(),
            &game.battlefield,
            &HashSet::new(),
            source,
            alice,
            &game
        ),
        12
    );
    let mut filter = ObjectFilter::creature();
    filter.power = Some(crate::filter::Comparison::GreaterThan(0));
    let count = Value::Scaled(Box::new(Value::Count(filter)), 3);
    assert_eq!(
        resolve_value_direct(
            &count,
            game.objects_map(),
            effects.effects(),
            &game.battlefield,
            &HashSet::new(),
            source,
            alice,
            &game
        ),
        3
    );
    assert_eq!(continuous(&count, &game, source, alice), 0);
}

#[test]
fn domain_uses_granted_types_in_execution_and_continuous_values() {
    let (mut game, source, alice) = fixture();
    let land_card = CardBuilder::new(CardId::new(), "Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let land = game.create_object_from_card(&land_card, alice, Zone::Battlefield);
    let effect = ContinuousEffect::new(
        source,
        alice,
        EffectTarget::Specific(land),
        Modification::AddSubtypes(vec![Subtype::Island, Subtype::Swamp]),
    );
    game.effect_store
        .continuous_effects
        .add_effect(effect.clone());
    let value = Value::BasicLandTypesAmong(ObjectFilter::land().you_control());
    let exec = ExecutionContext::new_default(source, alice);
    assert_eq!(
        resolve(&value, &EvaluationContext::execution_context(&game, &exec)).unwrap(),
        3
    );
    let calculation = CalculationContext {
        objects: game.objects_map(),
        effects: &game.effect_store.continuous_effects,
        battlefield: &game.battlefield,
        game: &game,
        current_object: source,
    };
    assert_eq!(
        resolve_continuous(&value, LayerValueContext::new(&calculation, source, alice)),
        3
    );
    assert_eq!(
        resolve_value_direct(
            &value,
            game.objects_map(),
            &[effect],
            &game.battlefield,
            &HashSet::new(),
            source,
            alice,
            &game,
        ),
        3
    );
    // Retained objects keep the types they had at snapshot time.
    let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(land).unwrap(),
        &game,
    );
    let mut exec = exec;
    exec.set_tagged_objects("land", vec![snapshot]);
    game.remove_object(land);
    assert_eq!(
        resolve(
            &Value::BasicLandTypesAmong(ObjectFilter::tagged("land")),
            &EvaluationContext::execution_context(&game, &exec),
        )
        .unwrap(),
        3
    );
}

#[test]
fn tagged_aggregates_retain_snapshot_numbers() {
    let (mut game, source, alice) = fixture();
    let mut snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    snapshot.power = Some(9);
    let mut exec = ExecutionContext::new_default(source, alice);
    exec.set_tagged_objects("test", vec![snapshot]);
    game.remove_object(source);
    let context = EvaluationContext::execution_context(&game, &exec);
    assert_eq!(
        resolve(&Value::TotalPower(ObjectFilter::tagged("test")), &context).unwrap(),
        9
    );
    // Numeric information uses the departed object's LKI (CR 608.2h).
    assert_eq!(resolve(
        &Value::PowerOf(Box::new(ChooseSpec::Tagged("test".into()))), &context,
    ).unwrap(), 9);
    // Selecting that removed object for a new operation still fails.
    assert!(crate::effects::helpers::resolve_objects_from_spec(
        &game, &ChooseSpec::Tagged("test".into()), &exec,
    ).unwrap().is_empty(), "physical selection cannot return a removed object");
}

#[test]
fn event_amount_and_life_amount_offsets_share_numeric_lookup() {
    let (game, source, alice) = fixture();
    let mut exec = ExecutionContext::new_default(source, alice);
    exec.event_value_amount = Some(6);
    for spec in [EventValueSpec::Amount, EventValueSpec::LifeAmount] {
        let context = EvaluationContext::execution_context(&game, &exec);
        assert_eq!(
            resolve(&Value::EventValue(spec.clone()), &context).unwrap(),
            6
        );
        assert_eq!(
            resolve(&Value::EventValueOffset(spec, -2), &context).unwrap(),
            4
        );
    }
}

#[test]
fn absent_numeric_stats_keep_context_specific_outcomes() {
    let (mut game, source, alice) = fixture();
    let card = CardBuilder::new(CardId::from_raw(99103), "Plain artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let exec = ExecutionContext::new_default(artifact, alice);
    assert!(
        resolve(
            &Value::SourcePower,
            &EvaluationContext::execution_context(&game, &exec)
        )
        .is_err()
    );
    assert_eq!(continuous(&Value::SourcePower, &game, artifact, alice), 0);
    assert_eq!(
        continuous(
            &Value::LeastPower(ObjectFilter::creature()),
            &game,
            source,
            alice
        ),
        -3
    );
}

#[test]
fn triggering_die_result_uses_its_event_and_rejects_missing_or_planar_rolls() {
    use crate::events::other::DieRolledEvent;
    use crate::triggers::TriggerEvent;
    let (mut game, source, alice) = fixture();
    let mut exec = ExecutionContext::new_default(source, alice);
    let value = Value::EventValue(EventValueSpec::DieResult);
    exec.event_value_amount = Some(99);
    assert!(resolve(&value, &EvaluationContext::execution_context(&game, &exec)).is_err());
    exec.triggering_event = Some(TriggerEvent::new_with_provenance(
        DieRolledEvent::new_with_natural_result(alice, source, 5, 6, 6).for_attraction_visit(),
        Default::default(),
    ));
    game.turn_store.turn_history.record_die_roll(alice, 2);
    assert_eq!(
        resolve(&value, &EvaluationContext::execution_context(&game, &exec)).unwrap(),
        6
    );
    assert_eq!(
        resolve(
            &Value::EventValueOffset(EventValueSpec::DieResult, -1),
            &EvaluationContext::execution_context(&game, &exec)
        )
        .unwrap(),
        5
    );
    exec.triggering_event = Some(TriggerEvent::new_with_provenance(
        DieRolledEvent::new_planar(alice, source, 6),
        Default::default(),
    ));
    assert!(resolve(&value, &EvaluationContext::execution_context(&game, &exec)).is_err());
}
