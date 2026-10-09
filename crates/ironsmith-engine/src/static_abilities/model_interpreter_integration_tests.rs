use super::*;
use crate::ability::Ability;
use crate::card::CardBuilder;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::mana_payment::*;
use crate::types::CardType;
use crate::zone::Zone;

fn permanent(game: &mut GameState, model: CompiledStaticAbility) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Compiled ability probe")
        .card_types(vec![CardType::Creature])
        .build();
    let id = game.create_object_from_card(&card, PlayerId(0), Zone::Battlefield);
    game.object_mut(id)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::from_model(model)));
    id
}

#[test]
fn compiled_krrik_plans_and_commits_life_without_changing_black_cost() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    permanent(
        &mut game,
        CompiledStaticAbility::krrik_black_mana_may_be_paid_with_life(),
    );
    let alice = PlayerId(0);
    let source = game.new_object_id();
    let mut request = ManaPaymentRequest::new(
        alice,
        source,
        crate::costs::PaymentReason::CastSpell,
        ManaCost::from_symbols(vec![ManaSymbol::Black, ManaSymbol::Black]),
    );
    request.allow_black_life = game.player_can_pay_black_with_life(alice, Some(source));
    assert!(request.allow_black_life);
    assert!(!game.player_can_pay_black_with_life(PlayerId(1), Some(source)));
    assert_eq!(
        mana_payment_life_options(&game, &request),
        vec![(ManaPipId(0), 2), (ManaPipId(1), 2)]
    );
    let plan = plan_first_mana_payment(&game, &request).unwrap();
    assert_eq!(plan.life_to_pay, 4);
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "preview must not pay life"
    );
    assert_eq!(request.cost.to_oracle(), "{B}{B}");
    assert_eq!(
        execute_mana_payment_plan(
            &mut game,
            &request,
            &plan,
            &mut crate::decision::SelectFirstDecisionMaker
        ),
        Ok(ManaPaymentExecution::Paid)
    );
    assert_eq!(game.player(alice).unwrap().life, 16);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    game.player_mut(alice).unwrap().life = 3;
    assert!(plan_first_mana_payment(&game, &request).is_err());
}

#[test]
fn compiled_krrik_respects_life_restrictions_and_explicit_pip_selection() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    permanent(
        &mut game,
        CompiledStaticAbility::krrik_black_mana_may_be_paid_with_life(),
    );
    let alice = PlayerId(0);
    let source = game.new_object_id();
    let mut request = ManaPaymentRequest::new(
        alice,
        source,
        crate::costs::PaymentReason::CastSpell,
        ManaCost::from_symbols(vec![ManaSymbol::Black]),
    );
    request.allow_black_life = true;
    game.player_mut(alice).unwrap().mana_pool.black = 1;
    assert_eq!(
        plan_first_mana_payment(&game, &request)
            .unwrap()
            .life_to_pay,
        0
    );
    request.preferences.required_life_pips.push(ManaPipId(0));
    let plan = plan_first_mana_payment(&game, &request).unwrap();
    assert_eq!(plan.life_to_pay, 2);
    assert_eq!(plan.expected_pool_after_payment.black, 1);
    permanent(
        &mut game,
        CompiledStaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
    );
    assert!(mana_payment_life_options(&game, &request).is_empty());
    assert!(plan_first_mana_payment(&game, &request).is_err());
}

#[test]
fn compiled_combat_caps_and_evasion_keep_runtime_values() {
    assert_eq!(
        StaticAbility::from_model(CompiledStaticAbility::max_attackers_each_combat(2))
            .max_creatures_can_attack_each_combat(),
        Some(2)
    );
    assert_eq!(
        StaticAbility::from_model(
            CompiledStaticAbility::max_attackers_can_attack_you_each_combat(1)
        )
        .max_creatures_can_attack_you_each_combat(),
        Some(1)
    );
    assert_eq!(
        StaticAbility::from_model(CompiledStaticAbility::max_blockers_each_combat(3))
            .max_creatures_can_block_each_combat(),
        Some(3)
    );
    assert!(
        StaticAbility::from_model(CompiledStaticAbility::cant_be_countered_ability())
            .cant_be_countered()
    );
    let filter = crate::target::ObjectFilter::creature();
    assert_eq!(
        StaticAbility::from_model(CompiledStaticAbility::bands_with_other(
            filter.clone(),
            "Bands with other creatures"
        ))
        .bands_with_other_filter(),
        Some(&filter)
    );
    let types = vec![CardType::Artifact, CardType::Enchantment];
    assert_eq!(
        StaticAbility::from_model(
            CompiledStaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_types(
                types.clone()
            )
        )
        .required_defending_player_card_types_for_unblockable(),
        Some(types)
    );
    assert_eq!(
        StaticAbility::from_model(
            CompiledStaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(
                CardType::Artifact
            )
        )
        .required_defending_player_card_type_for_unblockable(),
        Some(CardType::Artifact)
    );
}

#[test]
fn compiled_group_restrictions_and_attack_payments_reach_legality_hooks() {
    use ironsmith_core::{
        AttackCostCondition, AttackingGroupAttackCondition as Group,
        CantAttackUnlessConditionSpec as Condition,
    };
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = permanent(&mut game, CompiledStaticAbility::lifelink());
    let other = permanent(&mut game, CompiledStaticAbility::lifelink());
    let attacker = StaticAbility::from_model(CompiledStaticAbility::cant_attack_unless_condition(
        Condition::AttackingGroupCondition(Group::AtLeastNOtherCreaturesAttack(1)),
        "Attack together",
    ));
    assert_eq!(
        attacker.can_attack_with_attacking_group(&game, source, PlayerId(0), &[source]),
        Some(false)
    );
    assert_eq!(
        attacker.can_attack_with_attacking_group(&game, source, PlayerId(0), &[source, other]),
        Some(true)
    );
    let blocker = StaticAbility::from_model(CompiledStaticAbility::cant_attack_unless_condition(
        Condition::AttackingGroupCondition(Group::AtLeastNOtherCreaturesBlock(1)),
        "Block together",
    ));
    assert_eq!(
        blocker.can_block_with_blocking_group(&game, source, &[source]),
        Some(false)
    );
    assert_eq!(
        blocker.can_block_with_blocking_group(&game, source, &[source, other]),
        Some(true)
    );
    game.object_mut(source)
        .unwrap()
        .add_counters(crate::object::CounterType::PlusOnePlusOne, 3);
    let tax = StaticAbility::from_model(CompiledStaticAbility::cant_attack_unless_condition(
        Condition::AttackCost(AttackCostCondition::PayGenericPerSourceCounter {
            counter_type: crate::object::CounterType::PlusOnePlusOne,
            amount_per_counter: 1,
        }),
        "Pay per counter",
    ));
    assert_eq!(
        tax.generic_attack_mana_cost_for_source(&game, source, PlayerId(0)),
        Some(3)
    );
    assert_eq!(
        tax.can_pay_attack_cost(&game, source, PlayerId(0)),
        Some(true),
        "the caller checks the separately reported generic mana requirement"
    );
    let sacrifice = StaticAbility::from_model(CompiledStaticAbility::cant_attack_unless_condition(
        Condition::AttackCost(AttackCostCondition::SacrificePermanents {
            filter: crate::target::ObjectFilter::artifact(),
            count: 1,
        }),
        "Sacrifice an artifact to attack",
    ));
    assert_eq!(
        sacrifice.can_pay_attack_cost(&game, source, PlayerId(0)),
        Some(false)
    );
    let artifact = CardBuilder::new(CardId::new(), "Attack payment resource")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&artifact, PlayerId(0), Zone::Battlefield);
    assert_eq!(
        sacrifice.can_pay_attack_cost(&game, source, PlayerId(0)),
        Some(true)
    );
}
