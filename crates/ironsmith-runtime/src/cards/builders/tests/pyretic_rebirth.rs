#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Return target artifact or creature card from your graveyard to your hand. Pyretic Rebirth deals damage equal to that card's mana value to up to one target creature or planeswalker.";

fn unwrap_with_id_and_tag(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_with_id_and_tag(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_with_id_and_tag(&tagged.effect);
    }
    effect
}

#[test]
fn pyretic_rebirth_keeps_optional_damage_and_returned_card_mana_value() {
    let definition = parse_oracle_card_definition("Pyretic Rebirth");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Pyretic Rebirth should have a spell program");
    let [return_segment, damage_segment] = program.segments.as_slice() else {
        panic!("expected linked return and damage segments: {program:#?}");
    };
    let [return_root] = return_segment.default_effects.as_slice() else {
        panic!("expected one returned-card producer: {return_segment:#?}");
    };
    let returned = return_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the returned card should retain its result tag");
    assert!(
        returned
            .effect
            .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
            .is_some(),
        "{returned:#?}"
    );

    let [damage_root] = damage_segment.default_effects.as_slice() else {
        panic!("expected one linked damage action: {damage_segment:#?}");
    };
    let damage = unwrap_with_id_and_tag(damage_root)
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("the follow-up should deal typed damage");
    assert_eq!(damage.target.count().min, 0, "{damage:#?}");
    assert_eq!(damage.target.count().max, Some(1), "{damage:#?}");
    assert!(matches!(
        damage.amount.unhinted(),
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &returned.tag)
    ));
}
