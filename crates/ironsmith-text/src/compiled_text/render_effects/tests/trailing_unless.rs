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
