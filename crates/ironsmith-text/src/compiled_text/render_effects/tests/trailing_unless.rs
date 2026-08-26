use super::*;

fn trailing_damage(condition: crate::effect::Condition, amount: i32) -> Effect {
    Effect::new(crate::effects::ConditionalEffect::trailing_unless(
        condition,
        vec![Effect::deal_damage(
            Value::Fixed(amount),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer),
        )],
    ))
}

#[test]
fn trailing_unless_renders_positive_resolution_conditions() {
    let mut commander = ObjectFilter::default();
    commander.is_commander = true;
    assert_eq!(
        describe_effect(&trailing_damage(
            crate::effect::Condition::PlayerControls {
                player: PlayerFilter::IteratedPlayer,
                filter: commander,
            },
            4,
        )),
        "Deal 4 damage to that player unless they control a commander"
    );

    let mut basic_land = ObjectFilter::land();
    basic_land.supertypes.push(crate::types::Supertype::Basic);
    assert_eq!(
        describe_effect(&trailing_damage(
            crate::effect::Condition::PlayerHasAtLeast {
                player: PlayerFilter::IteratedPlayer,
                filter: basic_land,
                count: 2,
            },
            2,
        )),
        "Deal 2 damage to that player unless they control two or more basic lands"
    );

    let hand_count = |count| crate::effect::Condition::ValueComparison {
        left: Value::CardsInHand(PlayerFilter::IteratedPlayer),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Fixed(count),
    };
    assert_eq!(
        describe_effect(&trailing_damage(
            crate::effect::Condition::Or(Box::new(hand_count(3)), Box::new(hand_count(4)),),
            2,
        )),
        "Deal 2 damage to that player unless they have exactly three or exactly four cards in hand"
    );

    assert_eq!(
        describe_effect(&trailing_damage(
            crate::effect::Condition::EnchantedPermanentAttackedThisTurn,
            2,
        )),
        "Deal 2 damage to that player unless that creature attacked this turn"
    );
}

#[test]
fn targeted_source_damage_keeps_characteristic_and_referential_sacrifice() {
    let text = "At the beginning of your end step, target enchantment deals damage equal to its mana value to its controller unless that player sacrifices it.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Source Damage Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(text)
            .expect("target-source damage-unless route should compile");
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("source-damage probe should compile as a triggered ability")
    };
    let [unless_effect] = triggered.effects.flattened_default_effects() else {
        panic!("source-damage probe should keep one unless wrapper")
    };
    let unless_pays = unless_effect
        .downcast_ref::<crate::effects::UnlessPaysEffect>()
        .expect("source-damage probe should keep its typed payment");
    assert_eq!(
        describe_target_source_damage_unless_referential_sacrifice(unless_pays).as_deref(),
        Some(
            "target enchantment deals damage equal to its mana value to its controller unless that player sacrifices it"
        )
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn linked_any_target_damage_unless_payment_keeps_the_paid_branch_recipient() {
    let text = "Rhystic Lightning deals 4 damage to any target unless that permanent's controller or that player pays {2}. If they do, Rhystic Lightning deals 2 damage to the permanent or player.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Rhystic Lightning")
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .expect("linked damage unless-payment procedure should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}
