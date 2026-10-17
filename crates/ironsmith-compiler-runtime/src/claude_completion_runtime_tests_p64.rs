//! Claude package 64 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::static_abilities::StaticAbilityId;

const P64_LIQUID_FIRE: &str = "Mana cost: {4}{R}{R}\nType: Sorcery\nAs an additional cost to cast this spell, choose a number between 0 and 5.\nLiquid Fire deals X damage to target creature and 5 minus X damage to that creature's controller, where X is the chosen number.";

#[test]
fn liquid_fire_announces_the_chosen_number_as_a_bounded_x() {
    let definition = compile_to_runtime_definition("Liquid Fire", P64_LIQUID_FIRE, false)
        .expect("Liquid Fire should compile");
    let statics = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(statics.contains(&StaticAbilityId::ThisSpellXMaximum), "{statics:?}");
    assert!(statics.contains(&StaticAbilityId::ThisSpellXMinimum), "{statics:?}");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Fixed(5)"), "X is capped at 5: {debug}");
    let spell = format!("{:?}", definition.spell_effect.as_ref().expect("spell effect"));
    assert!(spell.contains("DealDamageEffect"), "{spell}");
    assert!(
        spell.contains("amount: X") || spell.contains("value: X"),
        "the creature takes X: {spell}"
    );
    assert!(
        !spell.contains("ChosenNumber") && !spell.contains("PriorEffectMetric"),
        "the chosen number is the announced X, not a missing local choice: {spell}"
    );
}
