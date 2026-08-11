#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Flying\nWhen this creature enters, you may pay {X}. When you do, you may cast target instant or sorcery card with mana value X from a graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead.";

#[test]
fn halo_forager_keeps_the_reflexive_targeted_graveyard_cast_and_replacement() {
    let definition = parse_oracle_card_definition("Halo Forager");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Halo Forager should have its enters trigger");
    let reflexive = trigger
        .effects
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>())
        .expect("the paid X result should create a reflexive trigger");
    let [choice] = reflexive.choices.as_slice() else {
        panic!("expected one targeted graveyard spell choice: {reflexive:#?}");
    };
    assert!(
        choice.is_target(),
        "the reflexive choice must remain targeted"
    );
    let ChooseSpec::Object(filter) = choice.base() else {
        panic!("expected one targeted graveyard spell choice: {choice:#?}");
    };
    assert_eq!(
        filter.card_types,
        [CardType::Instant, CardType::Sorcery],
        "{filter:#?}"
    );
    assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
    assert!(filter.owner.is_none(), "{filter:#?}");
    assert!(matches!(
        filter.mana_value.as_ref(),
        Some(crate::filter::Comparison::EqualExpr(value))
            if matches!(value.unhinted(), Value::X)
    ));

    let debug = format!("{reflexive:#?}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    assert!(
        debug.contains("RegisterFutureZoneReplacementEffect"),
        "{debug}"
    );

    let [_, may_root, replacement_root] = reflexive.effects.as_slice() else {
        panic!("expected target, optional cast, and linked replacement: {reflexive:#?}");
    };
    let may = may_root
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("optional cast should retain its result id");
    let replacement = replacement_root
        .downcast_ref::<crate::effects::IfEffect>()
        .expect("replacement registration should remain result-gated");
    assert_eq!(
        replacement.condition, may.id,
        "the replacement must be registered only when the optional cast happened"
    );
    assert_eq!(
        replacement.predicate,
        crate::effect::EffectPredicate::Happened
    );
}
