use super::*;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;

const LIFECRAFT_AWAKENING: &str = "Put X +1/+1 counters on target artifact you control. If it isn't a creature or Vehicle, it becomes a 0/0 Construct artifact creature.";

fn count_nested<T: 'static>(effect: &crate::effect::Effect) -> usize {
    let mut count = usize::from(effect.downcast_ref::<T>().is_some());
    effect.visit_child_effects(&mut |child| count += count_nested::<T>(child));
    count
}

#[test]
fn target_bound_conditional_animation_lowers_inside_the_spell_program() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Lifecraft Awakening")
        .card_types(vec![CardType::Sorcery])
        .parse_text(LIFECRAFT_AWAKENING)
        .expect("Lifecraft Awakening should compile");

    assert!(
        definition.abilities.is_empty(),
        "a resolution-scoped animation must not become a global static ability: {:#?}",
        definition.abilities
    );
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Lifecraft Awakening should retain its spell program");

    let mut counters = 0;
    let mut conditionals = 0;
    let mut continuous = 0;
    for effect in program.flattened_default_effects() {
        counters += count_nested::<crate::effects::PutCountersEffect>(effect);
        conditionals += count_nested::<crate::effects::ConditionalEffect>(effect);
        continuous += count_nested::<crate::effects::ApplyContinuousEffect>(effect);
    }

    assert_eq!(counters, 1, "expected one counter effect: {program:#?}");
    assert_eq!(
        conditionals, 1,
        "expected one conditional animation: {program:#?}"
    );
    assert_eq!(
        continuous, 1,
        "the fused animation must stay in the conditional branch: {program:#?}"
    );

    let conditional = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::ConditionalEffect>(effect))
        .expect("expected a conditional effect");
    let crate::ConditionExpr::Not(disjunction) = &conditional.condition else {
        panic!(
            "the single negated copula must scope over both descriptors: {:#?}",
            conditional.condition
        );
    };
    let crate::ConditionExpr::Or(creature, vehicle) = disjunction.as_ref() else {
        panic!("the negated descriptor must contain creature-or-Vehicle: {disjunction:#?}");
    };
    let crate::ConditionExpr::TaggedObjectMatches(creature_tag, creature_filter) =
        creature.as_ref()
    else {
        panic!("expected a tagged creature check: {creature:#?}");
    };
    let crate::ConditionExpr::TaggedObjectMatches(vehicle_tag, vehicle_filter) = vehicle.as_ref()
    else {
        panic!("expected a tagged Vehicle check: {vehicle:#?}");
    };
    assert_eq!(creature_tag.as_str(), "counters_0");
    assert_eq!(vehicle_tag, creature_tag);
    assert_eq!(creature_filter.card_types, [CardType::Creature]);
    assert!(creature_filter.subtypes.is_empty());
    assert!(vehicle_filter.card_types.is_empty());
    assert_eq!(vehicle_filter.subtypes, [Subtype::Vehicle]);

    let animation = conditional
        .if_true
        .iter()
        .find_map(super::find_nested_effect::<crate::effects::ApplyContinuousEffect>)
        .expect("expected the conditional animation effect");
    assert_eq!(
        animation.target_spec,
        Some(crate::target::ChooseSpec::Tagged(creature_tag.clone())),
        "the animation must consume the same tagged object checked by the condition"
    );
    assert_eq!(
        usize::from(animation.modification.is_some()) + animation.additional_modifications.len(),
        4,
        "card types, subtype replacement, and base P/T must be fused into the animation"
    );
}
