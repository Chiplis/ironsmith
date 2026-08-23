#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "{T}: Each 1/1 creature you control gets +1/+2 until end of turn.";

fn continuous_pump(effect: &Effect) -> Option<&crate::effects::ApplyContinuousEffect> {
    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        return Some(apply);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return continuous_pump(&tagged.effect);
    }
    None
}

#[test]
fn pendelhaven_elder_keeps_the_exact_one_one_recipient_filter() {
    let definition = parse_oracle_card_definition("Pendelhaven Elder");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Pendelhaven Elder should retain its activated ability");
    let pump = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| continuous_pump(effect))
        .unwrap_or_else(|| {
            panic!(
                "the activation should keep its temporary anthem: {:#?}",
                activated.effects
            )
        });
    let crate::continuous::EffectTarget::Filter(filter) = &pump.target else {
        panic!("the pump should retain its recipient filter: {pump:#?}")
    };
    assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(
        filter.power,
        Some(crate::filter::Comparison::Equal(1)),
        "{pump:#?}"
    );
    assert_eq!(
        filter.toughness,
        Some(crate::filter::Comparison::Equal(1)),
        "{pump:#?}"
    );
    assert_eq!(
        pump.runtime_modifications.as_slice(),
        [crate::effects::RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(1),
            toughness: Value::Fixed(2),
        }]
    );
    assert_eq!(pump.until, crate::Until::EndOfTurn);
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);
}
