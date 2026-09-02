use super::*;
use crate::{Cost, ManaSymbol};

fn generic_two() -> ManaCost {
    ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])
}

#[test]
fn try_map_preserves_ward_cost_alternatives() {
    let ward: StaticAbility<(), (), Cost<&'static str>, ()> =
        StaticAbility::ward(TotalCost::one_of(vec![
            TotalCost::from_cost(Cost::effect("discard")),
            TotalCost::mana(generic_two()),
        ]));

    let mapped: StaticAbility<(), (), Cost<usize>, ()> = ward
        .try_map(
            Ok::<_, ()>,
            Ok::<_, ()>,
            |cost| cost.try_map_effect(|effect| Ok::<_, ()>(effect.len())),
            Ok::<_, ()>,
        )
        .expect("ward alternatives should map recursively");

    let StaticAbilityPayload::Ward(cost) = mapped.payload else {
        panic!("expected mapped ward payload");
    };
    assert_eq!(
        cost,
        TotalCost::one_of(vec![
            TotalCost::from_cost(Cost::effect(7usize)),
            TotalCost::mana(generic_two()),
        ])
    );
}

#[test]
fn try_map_preserves_full_escalate_cost_and_surface() {
    let escalate: StaticAbility<(), (), Cost<&'static str>, ()> =
        StaticAbility::escalate_with_cost_surface(
            TotalCost::from_costs(vec![Cost::mana(generic_two()), Cost::effect("discard")]),
            Some("{2}, Discard a card".to_string()),
        );

    let mapped: StaticAbility<(), (), Cost<usize>, ()> = escalate
        .try_map(
            Ok::<_, ()>,
            Ok::<_, ()>,
            |cost| cost.try_map_effect(|effect| Ok::<_, ()>(effect.len())),
            Ok::<_, ()>,
        )
        .expect("Escalate costs should map recursively");

    let StaticAbilityPayload::Escalate(spec) = mapped.payload else {
        panic!("expected mapped Escalate payload");
    };
    assert_eq!(
        spec.cost,
        TotalCost::from_costs(vec![Cost::mana(generic_two()), Cost::effect(7usize)])
    );
    assert_eq!(spec.cost_surface.as_deref(), Some("{2}, Discard a card"));
}
