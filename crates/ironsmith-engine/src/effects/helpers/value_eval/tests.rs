use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::continuous::{
    CalculationContext, ContinuousEffect, ContinuousEffectManager, EffectTarget, Modification,
    resolve_value_direct,
};
use crate::ids::CardId;
use crate::target::ObjectFilter;
use crate::types::CardType;

fn linked_owner(source: ObjectId) -> crate::linked_exile::LinkedExileOwner {
    crate::linked_exile::LinkedExileOwner::capture(source,
        Some(ironsmith_core::LinkedExilePair {
            definition: ironsmith_core::LinkedExileDefinition([91; 32]), pair: 0,
        }), Some(&crate::continuous::AbilityOrigin::Printed(0))).unwrap()
}

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
fn source_linked_characteristics_use_the_live_incarnation_set_and_checked_wide_sum() {
    let (mut game, source, alice) = fixture();
    let value = Value::PowerOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EXILED_TAG.into())));
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.linked_exile_owner = Some(linked_owner(source));
    assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 0);
    let card = CardBuilder::new(CardId::new(), "Linked quantity witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(i32::MAX, 4)).build();
    let first = game.create_object_from_card(&card, alice, Zone::Exile);
    let second = game.create_object_from_card(&card, alice, Zone::Exile);
    game.add_exiled_with_source_link(source, first);
    game.add_linked_exile_pair_member(linked_owner(source), first);
    game.add_exiled_with_source_link(source, second);
    game.add_linked_exile_pair_member(linked_owner(source), second);
    assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 2 * i64::from(i32::MAX));
    ctx.tag_object(crate::tag::SOURCE_EXILED_TAG, ObjectSnapshot::from_object(game.object(first).unwrap(), &game));
    game.move_object_by_game_rule(first, Zone::Hand).unwrap();
    assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), i64::from(i32::MAX), "stale captured exile snapshot cannot restore a departed link");
    game.move_object_by_game_rule(second, Zone::Graveyard).unwrap();
    assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 0);
    let missing = ExecutionContext::new_default(ObjectId::from_raw(u64::MAX), alice);
    assert!(matches!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &missing)), Err(ExecutionError::IncompleteEvidence(_))));
}

#[test]
fn missing_original_sacrifice_is_an_error_while_completed_empty_is_zero_for_scalar_and_filter_values() {
    let (game, source, alice) = fixture();
    let tag = ironsmith_core::tag::SacrificeCostTag::OriginalResult(3).key();
    for value in [
        Value::PowerOf(Box::new(ChooseSpec::Tagged(tag.clone()))),
        Value::TotalPower(ObjectFilter::tagged(tag.clone())),
        Value::Count(ObjectFilter::tagged(tag.clone())),
    ] {
        let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.linked_exile_owner = Some(linked_owner(source));
        assert!(matches!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)), Err(ExecutionError::IncompleteEvidence(_))));
        ctx.set_tagged_objects(tag.clone(), Vec::new());
        assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 0);
    }
}

#[test]
fn source_linked_split_card_keeps_checked_combined_mana_value_and_face_down_zero() {
    let (mut game, source, alice) = fixture();
    let card = CardBuilder::new(CardId::new(), "First split half")
        .card_types(vec![CardType::Instant])
        .mana_cost(crate::ManaCost::from_pips(vec![vec![crate::ManaSymbol::Generic(2)]]))
        .build();
    let linked = game.create_object_from_card(&card, alice, Zone::Exile);
    let object = game.object_mut(linked).unwrap();
    object.linked_face_layout = crate::card::LinkedFaceLayout::Split;
    object.linked_face_mana_cost = Some(crate::ManaCost::from_pips(vec![vec![crate::ManaSymbol::Generic(3)]]).into());
    game.add_exiled_with_source_link(source, linked);
    game.add_linked_exile_pair_member(linked_owner(source), linked);
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EXILED_TAG.into())));
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.linked_exile_owner = Some(linked_owner(source));
    assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 5);
    assert!(game.set_face_down(linked));
    assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 0);
}

#[test]
fn direct_and_dispatched_linked_quantity_failures_leave_no_partial_stat_effect() {
    use crate::effects::EffectExecutor;
    for dispatched in [false, true] { for invalid in [false, true] {
        let (mut game, source, alice) = fixture();
        let card = CardBuilder::new(CardId::new(), "Checked linked card")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 4)).build();
        let linked = game.create_object_from_card(&card, alice, Zone::Exile);
        let link = if invalid { ObjectId::from_raw(u64::MAX) } else { linked };
        game.add_exiled_with_source_link(source, link);
        game.add_linked_exile_pair_member(linked_owner(source), link);
        if !invalid { game.object_mut(linked).unwrap().counters.insert(crate::CounterType::PlusOnePlusOne, u32::MAX); }
        let number = Value::PowerOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EXILED_TAG.into())));
        let pump = crate::effects::ModifyPowerToughnessEffect::new(ChooseSpec::SpecificObject(source), number.clone(), number, crate::effect::Until::EndOfTurn);
        let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.linked_exile_owner = Some(linked_owner(source));
        let ids = game.next_object_id_counter(); let history = game.turn_store.turn_history.event_records.len();
        let provenance = game.provenance_graph().node_count();
        let outcome = if dispatched { crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(pump.clone()), &mut ctx) }
            else { pump.execute(&mut game, &mut ctx) };
        assert!(matches!(outcome, Err(ExecutionError::IncompleteEvidence(_) | ExecutionError::ContinuousDiscovery(_))), "{outcome:?}");
        assert_eq!(game.next_object_id_counter(), ids); assert_eq!(game.turn_store.turn_history.event_records.len(), history);
        assert_eq!(game.provenance_graph().node_count(), provenance);
        assert!(ctx.effect_outcomes.is_empty() && ctx.tagged_objects.is_empty());
        assert!(!game.effect_store.has_pending_trigger_work());
        if invalid { game.remove_exiled_with_source_link(link); }
        else { game.object_mut(linked).unwrap().counters.clear(); }
        pump.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.current_power(source), Some(if invalid { -3 } else { 0 }));
    }}
}

#[test]
fn linked_quantity_propagates_incomplete_static_discovery_without_a_printed_fallback() {
    #[derive(Debug, Clone)]
    struct UnboundedLinkedSource(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl crate::static_abilities::StaticAbilityKind for UnboundedLinkedSource {
        fn id(&self) -> crate::static_abilities::StaticAbilityId { crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter }
        fn display(&self) -> String { "Unbounded linked quantity fixture".into() }
        fn generate_effects(&self, source: ObjectId, controller: PlayerId, game: &GameState) -> Vec<ContinuousEffect> {
            assert!(self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 32_768);
            let crate::ability::AbilityKind::Static(parent) = &game.object(source).unwrap().abilities[0].kind else { panic!("static fixture"); };
            vec![
                ContinuousEffect::new(source, controller, EffectTarget::Source, Modification::AddAbility(parent.clone())),
                ContinuousEffect::new(source, controller, EffectTarget::Source, Modification::ModifyPower(1)),
            ]
        }
    }
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let (mut game, source, alice) = fixture();
        let card = CardBuilder::new(CardId::new(), "Linked witness")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(7, 8)).build();
        let linked = game.create_object_from_card(&card, alice, Zone::Exile);
        game.add_exiled_with_source_link(source, linked);
    game.add_linked_exile_pair_member(linked_owner(source), linked);
        game.object_mut(source).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::new(UnboundedLinkedSource(Default::default()))));
        let value = Value::PowerOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EXILED_TAG.into())));
        let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.linked_exile_owner = Some(linked_owner(source));
        assert!(matches!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)),
            Err(ExecutionError::ContinuousDiscovery(crate::static_ability_processor::StaticEffectDiscoveryError::RoundLimit { .. }))));
    }).unwrap().join().unwrap();
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
    assert_eq!(
        resolve(
            &Value::PowerOf(Box::new(ChooseSpec::Tagged("test".into()))),
            &context,
        )
        .unwrap(),
        9
    );
    // Selecting that removed object for a new operation still fails.
    assert!(
        crate::effects::helpers::resolve_objects_from_spec(
            &game,
            &ChooseSpec::Tagged("test".into()),
            &exec,
        )
        .unwrap()
        .is_empty(),
        "physical selection cannot return a removed object"
    );
}

#[test]
fn distinct_names_separates_current_names_from_historical_tags_and_layer_frames() {
    let (mut game, source, alice) = fixture();
    let card = CardBuilder::new(CardId::new(), "Other name")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    let other = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let snapshots = [source, other].into_iter().map(|id|
        ObjectSnapshot::from_object_with_calculated_characteristics(game.object(id).unwrap(), &game)
    ).collect();
    let mut exec = ExecutionContext::new_default(source, alice);
    exec.set_tagged_objects("historical names", snapshots);
    let current = Value::DistinctNames(ObjectFilter::creature().you_control());
    let historical = Value::DistinctNames(ObjectFilter::tagged("historical names"));
    let name = game.object(source).unwrap().name.to_string();
    let effect = ContinuousEffect::new(source, alice, EffectTarget::Specific(other),
        Modification::SetName(name));
    let mut effects = ContinuousEffectManager::new();
    effects.add_effect(effect.clone());
    let calculation = CalculationContext {
        objects: game.objects_map(), effects: &effects, battlefield: &game.battlefield,
        game: &game, current_object: source,
    };
    assert_eq!(resolve_continuous(&current, LayerValueContext::new(&calculation, source, alice)), 1);
    game.effect_store.continuous_effects.add_effect(effect);
    assert_eq!(resolve(&current, &EvaluationContext::execution_context(&game, &exec)).unwrap(), 1);
    assert_eq!(resolve(&historical, &EvaluationContext::execution_context(&game, &exec)).unwrap(), 2);
    game.remove_object(other);
    assert_eq!(resolve(&historical, &EvaluationContext::execution_context(&game, &exec)).unwrap(), 2);
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
    assert_eq!(
        resolve(
            &Value::SourcePower,
            &EvaluationContext::execution_context(&game, &exec)
        )
        .unwrap(),
        0
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

#[test]
fn fractional_rounding_uses_a_wide_intermediate_for_representable_results() {
    let (mut game, source, alice) = fixture();
    for base in [i32::MIN, -1, 0, 1, i32::MAX] {
        for divisor in [2, 3, 4, i32::MAX] {
            let rounded = Value::DividedRoundedDown(
                Box::new(Value::Add(
                    Box::new(Value::Fixed(base)),
                    Box::new(Value::Fixed(divisor - 1)),
                )),
                divisor,
            );
            let expected =
                (i64::from(base) + i64::from(divisor) - 1).div_euclid(i64::from(divisor)) as i32;
            let exec = ExecutionContext::new_default(source, alice);
            assert_eq!(
                resolve(
                    &rounded,
                    &EvaluationContext::execution_context(&game, &exec)
                )
                .unwrap(),
                expected
            );
            assert_eq!(continuous(&rounded, &game, source, alice), expected);
        }
    }
    game.player_mut(alice).unwrap().life = i32::MAX;
    let exec = ExecutionContext::new_default(source, alice);
    assert_eq!(
        resolve(
            &Value::HalfLifeTotalRoundedUp(PlayerFilter::You),
            &EvaluationContext::execution_context(&game, &exec)
        )
        .unwrap(),
        1_073_741_824
    );
    let half = Value::HalfRoundedDown(Box::new(Value::Add(
        Box::new(Value::Fixed(i32::MAX)),
        Box::new(Value::Fixed(1)),
    )));
    assert_eq!(continuous(&half, &game, source, alice), 1_073_741_824);
    let overflow = Value::DividedRoundedDown(Box::new(Value::Fixed(i32::MIN)), -1);
    assert!(
        resolve(
            &overflow,
            &EvaluationContext::execution_context(&game, &exec)
        )
        .is_err()
    );
}

#[test]
fn canonical_maximum_evaluates_without_overflowing_its_algebraic_intermediates() {
    let (game, source, alice) = fixture();
    let exec = ExecutionContext::new_default(source, alice);
    for a in [i32::MIN, -3, 0, 2, i32::MAX] {
        for b in [i32::MIN, -5, 0, 7, i32::MAX] {
            let value = Value::Add(
                Box::new(Value::Add(
                    Box::new(Value::Fixed(a)),
                    Box::new(Value::Fixed(b)),
                )),
                Box::new(Value::Scaled(
                    Box::new(Value::Min(
                        Box::new(Value::Fixed(a)),
                        Box::new(Value::Fixed(b)),
                    )),
                    -1,
                )),
            );
            assert_eq!(
                resolve(&value, &EvaluationContext::execution_context(&game, &exec)).unwrap(),
                a.max(b)
            );
            assert_eq!(continuous(&value, &game, source, alice), a.max(b));
        }
    }
    // A mismatched minimum is ordinary arithmetic, even with that prose hint.
    let mismatched = Value::Add(
        Box::new(Value::Add(
            Box::new(Value::Fixed(4)),
            Box::new(Value::Fixed(9)),
        )),
        Box::new(Value::Scaled(
            Box::new(Value::Min(
                Box::new(Value::Fixed(3)),
                Box::new(Value::Fixed(8)),
            )),
            -1,
        )),
    )
    .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhicheverIsGreater);
    assert_eq!(
        resolve(
            &mismatched,
            &EvaluationContext::execution_context(&game, &exec)
        )
        .unwrap(),
        10
    );
}

#[test]
fn scoped_life_maxima_and_below_half_counts_preserve_sign_scope_ties_and_odd_thresholds() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 41);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let charlie = game.players[2].id;
    let card = CardBuilder::new(CardId::new(), "Life quantity source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let check = |game: &GameState, value: &Value, controller: PlayerId, expected| {
        let ctx = ExecutionContext::new_default(source, controller);
        assert_eq!(
            resolve(value, &EvaluationContext::execution_context(game, &ctx)).unwrap(),
            expected
        );
        assert_eq!(continuous(value, game, source, controller), expected);
    };
    assert!(game.write_life_total(bob, 20));
    assert!(game.write_life_total(charlie, 21));
    check(
        &game,
        &Value::MaximumLifeTotal(PlayerFilter::Opponent),
        alice,
        21,
    );
    check(
        &game,
        &Value::MaximumLifeTotal(PlayerFilter::Any),
        alice,
        41,
    );
    check(
        &game,
        &Value::CountPlayersBelowHalfStartingLifeTotal(PlayerFilter::Opponent),
        alice,
        1,
    );
    check(
        &game,
        &Value::MaximumLifeTotal(PlayerFilter::Opponent),
        bob,
        41,
    );
    assert!(game.write_life_total(charlie, 20));
    check(
        &game,
        &Value::MaximumLifeTotal(PlayerFilter::Opponent),
        alice,
        20,
    );
    check(
        &game,
        &Value::CountPlayersBelowHalfStartingLifeTotal(PlayerFilter::Opponent),
        alice,
        2,
    );
    assert!(game.write_life_total(bob, -3));
    assert!(game.write_life_total(charlie, -5));
    check(
        &game,
        &Value::MaximumLifeTotal(PlayerFilter::Opponent),
        alice,
        -3,
    );
    let ceiling = Value::HalfRoundedDown(Box::new(Value::Add(
        Box::new(Value::MaximumLifeTotal(PlayerFilter::Opponent)),
        Box::new(Value::Fixed(1)),
    )));
    check(&game, &ceiling, alice, -1);
    assert!(game.write_life_total(bob, i32::MAX));
    check(&game, &ceiling, alice, 1_073_741_824);
    let empty = PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::Any);
    check(&game, &Value::MaximumLifeTotal(empty.clone()), alice, 0);
    check(
        &game,
        &Value::CountPlayersBelowHalfStartingLifeTotal(empty),
        alice,
        0,
    );
}

#[test]
fn absolute_difference_widens_intermediates_without_inventing_an_out_of_range_value() {
    let (game, source, player) = fixture();
    let ctx = ExecutionContext::new_default(source, player);
    for (a, b, expected) in [
        (i32::MIN, i32::MIN, 0),
        (-5, 3, 8),
        (i32::MAX, i32::MAX - 5, 5),
    ] {
        let value = Value::absolute_difference(Value::Fixed(a), Value::Fixed(b));
        assert_eq!(
            resolve(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(),
            expected
        );
        assert_eq!(continuous(&value, &game, source, player), expected);
    }
    let unrepresentable =
        Value::absolute_difference(Value::Fixed(i32::MIN), Value::Fixed(i32::MAX));
    assert!(matches!(
        resolve(
            &unrepresentable,
            &EvaluationContext::execution_context(&game, &ctx)
        ),
        Err(ExecutionError::UnresolvableValue(_))
    ));
}

#[test]
fn noted_life_prefers_live_re_notes_and_exact_departure_receipts_without_blink_following() {
    let (mut game, source, alice) = fixture();
    game.note_life_total_for_source(source, alice).unwrap();
    let early = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    let ctx = ExecutionContext::new_default(source, alice).with_source_snapshot(early);
    game.write_life_total(alice, 18);
    let before_note =
        game.cached_object_snapshot_with_calculated_characteristics(game.object(source).unwrap());
    assert_eq!(before_note.noted_life_total, Some(20));
    game.note_life_total_for_source(source, alice).unwrap();
    let after_note =
        game.cached_object_snapshot_with_calculated_characteristics(game.object(source).unwrap());
    assert_eq!(
        after_note.noted_life_total,
        Some(18),
        "the note write itself invalidates cached LKI"
    );
    assert_eq!(
        resolve(
            &Value::LastNotedLifeTotal,
            &EvaluationContext::execution_context(&game, &ctx)
        )
        .unwrap(),
        18
    );
    let graveyard = game
        .move_object_by_game_rule(source, Zone::Graveyard)
        .unwrap();
    assert_eq!(
        game.noted_life_total_for_source(source),
        None,
        "departure still clears the live annotation"
    );
    assert_eq!(
        resolve(
            &Value::LastNotedLifeTotal,
            &EvaluationContext::execution_context(&game, &ctx)
        )
        .unwrap(),
        18,
        "the actual departure receipt supersedes an earlier 20-life source snapshot"
    );
    let returned = game
        .move_object_by_game_rule(graveyard, Zone::Battlefield)
        .unwrap();
    game.write_life_total(alice, 30);
    game.note_life_total_for_source(returned, alice).unwrap();
    assert_eq!(
        resolve(
            &Value::LastNotedLifeTotal,
            &EvaluationContext::execution_context(&game, &ctx)
        )
        .unwrap(),
        18
    );
    game.write_life_total(alice, 19);
    game.note_life_total_for_source(source, alice).unwrap();
    assert_eq!(
        resolve(
            &Value::LastNotedLifeTotal,
            &EvaluationContext::execution_context(&game, &ctx)
        )
        .unwrap(),
        19,
        "an explicit subsequent instruction may re-note for the same departed source identity"
    );
    assert_eq!(game.noted_life_total_for_source(returned), Some(30));
    let wrong = ExecutionContext::new_default(game.new_object_id(), alice)
        .with_source_snapshot(ctx.source_snapshot.clone().unwrap());
    assert!(
        resolve(
            &Value::LastNotedLifeTotal,
            &EvaluationContext::execution_context(&game, &wrong)
        )
        .is_err(),
        "a snapshot for another object is not a receipt"
    );
}

#[test]
fn numeric_damage_and_prevention_receipts_preserve_wide_amounts_before_narrowing() {
    let (game, source, player) = fixture();
    for amount in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
        let target = crate::events::DamageTarget::Player(player);
        let events = [
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::DamageEvent::with_cause(
                    source,
                    target,
                    amount,
                    false,
                    crate::events::EventCause::effect(),
                ),
                Default::default(),
            ),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::DamagePreventedEvent::new(
                    source, target, amount, source, player, false,
                ),
                Default::default(),
            ),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(player, amount),
                Default::default(),
            ),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeLossEvent::new(player, amount, false),
                Default::default(),
            ),
        ];
        for event in events {
            let context =
                ExecutionContext::new_default(source, player).with_triggering_event(event);
            assert_eq!(
                resolve_wide(
                    &Value::EventValue(EventValueSpec::Amount),
                    &EvaluationContext::execution_context(&game, &context),
                )
                .unwrap(),
                i64::from(amount)
            );
            let result = resolve(
                &Value::EventValue(EventValueSpec::Amount),
                &EvaluationContext::execution_context(&game, &context),
            );
            if amount == i32::MAX as u32 {
                assert_eq!(result.unwrap(), i32::MAX);
            } else {
                let error = result.unwrap_err();
                assert!(matches!(error, ExecutionError::ResourceLimitExceeded { requested, maximum, .. } if requested == u128::from(amount) && maximum == i32::MAX as u128));
            }
        }
    }
}

#[test]
fn an_authoritative_empty_object_quantity_is_zero_but_absent_evidence_still_errors() {
    let (game, source, player) = fixture();
    let tag = crate::tag::TagKey::from("empty_consult_match");
    let mut ctx = ExecutionContext::new_default(source, player);
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(tag.clone())));
    assert!(matches!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)),
        Err(ExecutionError::InvalidTarget)));
    ctx.set_tagged_objects(tag, Vec::new());
    assert_eq!(resolve_wide(&value, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 0);
}


#[test]
fn linked_scalar_unknown_native_and_source_only_import_require_recovery() {
    let (mut game, source, alice) = fixture();
    let card = CardBuilder::new(CardId::new(), "Unrelated native exile")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(9, 9)).build();
    let linked = game.create_object_from_card(&card, alice, Zone::Exile);
    game.add_exiled_with_source_link(source, linked);
    let number = Value::PowerOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EXILED_TAG.into())));
    let mut ctx = ExecutionContext::new_default(source, alice);
    assert!(matches!(resolve_wide(&number, &EvaluationContext::execution_context(&game, &ctx)),
        Err(ExecutionError::IncompleteEvidence(_))), "native metadata is required even when source-wide links exist");
    ctx.linked_exile_owner = Some(linked_owner(source));
    assert_eq!(resolve_wide(&number, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 0,
        "a proven pair with no members is known zero");
    game.add_linked_exile_pair_member(linked_owner(source), linked);
    let native_checkpoint = game.clone();
    assert_eq!(resolve_wide(&number, &EvaluationContext::execution_context(&native_checkpoint, &ctx)).unwrap(), 9);
    game.replace_exiled_with_source_links(HashMap::from([(source, vec![linked])]));
    assert!(matches!(resolve_wide(&number, &EvaluationContext::execution_context(&game, &ctx)),
        Err(ExecutionError::IncompleteEvidence(_))), "source-only import must never claim empty complete pair state");
    game = native_checkpoint;
    assert_eq!(resolve_wide(&number, &EvaluationContext::execution_context(&game, &ctx)).unwrap(), 9);
}
