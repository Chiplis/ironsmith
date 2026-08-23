#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_compiled(definition: &CardDefinition, expected: &[&str]) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        expected.join("\n"),
        "{definition:#?}"
    );
}

fn find_nested<T: Clone + 'static>(effect: &Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested::<T>(child);
        }
    });
    found
}

#[test]
fn lazotep_convert_keeps_the_additive_black_copy_characteristic() {
    let definition = parse_oracle_card_definition("Lazotep Convert");
    assert_compiled(
        &definition,
        &[
            "You may have this creature enter as a copy of any creature card in a graveyard except it's a 4/4 black zombie in addition to its other colors and types.",
        ],
    );

    let copy = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.compiled_model(),
            _ => None,
        })
        .find_map(|model| match &model.payload {
            ironsmith_core::StaticAbilityPayload::EnterAsCopyAsEnters { spec, .. } => Some(spec),
            _ => None,
        })
        .expect("Lazotep Convert should retain a typed enter-as-copy replacement");

    assert_eq!(copy.added_colors, crate::color::ColorSet::BLACK);
    assert_eq!(copy.set_base_power_toughness, Some((4, 4)));
    assert_eq!(copy.added_subtypes, vec![Subtype::Zombie]);
}

#[test]
fn sorin_ravenous_neonate_uses_life_gained_this_turn_as_damage_amount() {
    let definition = parse_oracle_card_definition("Sorin, Ravenous Neonate");
    assert_compiled(
        &definition,
        &[
            "Extort",
            "+2: Create a Food token.",
            "−1: Sorin deals damage equal to the amount of life you gained this turn to any target.",
            "−6: Gain control of target creature. It becomes a Vampire in addition to its other types. Put a lifelink counter on it if you control a white noncreature, nonplaneswalker permanent.",
        ],
    );

    let damage = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .flat_map(|program| program.flattened_default_effects())
        .find_map(find_nested::<crate::effects::DealDamageEffect>)
        .expect("Sorin's -1 should retain executable dynamic damage");
    assert_eq!(
        damage.amount.unhinted(),
        &crate::effect::Value::LifeGainedThisTurn(PlayerFilter::You)
    );
    assert!(damage.target.is_target(), "{damage:#?}");
}
