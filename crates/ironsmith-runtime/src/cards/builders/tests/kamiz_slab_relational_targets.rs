#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn kamiz_reuses_the_first_target_as_the_lesser_power_comparison_source() {
    let definition = parse_oracle_card_definition("Kamiz, Obscura Oculus");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Whenever you attack, target attacking creature can't be blocked this turn. It connives. Then choose another attacking creature with lesser power. That creature gains double strike until end of turn."
        ],
        "{definition:#?}"
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Kamiz must retain the attack trigger");
    let [_, _, choice_segment, grant_segment] = triggered.effects.segments.as_slice() else {
        panic!("expected target, connive, choice, and grant segments: {triggered:#?}");
    };
    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        panic!("expected one relational choice: {choice_segment:#?}");
    };
    let with_source = choice_effect
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        .expect("the second choice must compare against the first tagged target");
    assert!(matches!(
        with_source.source.unhinted(),
        ChooseSpec::Tagged(tag) if tag.as_str() == "targeted_0"
    ));
    let sequence = with_source
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the authored leading-Then sequence stays intact");
    let [choose_effect] = sequence.effects.as_slice() else {
        panic!("expected one choice inside the comparison context: {sequence:#?}");
    };
    let choose = choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("typed creature choice");
    assert!(choose.filter.other);
    assert_eq!(
        choose.filter.power_relative_to_source,
        Some(ironsmith_core::SourcePowerRelation::LessThanSource)
    );
    assert!(choose.filter.attacking);

    let [grant_effect] = grant_segment.default_effects.as_slice() else {
        panic!("expected one linked double-strike grant: {grant_segment:#?}");
    };
    let grant = grant_effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("typed continuous grant");
    assert!(grant.target_spec.as_ref().is_some_and(|spec| {
        matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == "__it__")
    }));
}

#[test]
fn slab_hammer_pumps_only_the_equipped_attacker_after_the_optional_return() {
    let definition = parse_oracle_card_definition("Slab Hammer");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Whenever equipped creature attacks, you may return a land you control to its owner's hand. If you do, the creature gets +2/+2 until end of turn.",
            "Equip {2}",
        ],
        "{definition:#?}"
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Slab Hammer must retain the equipped-creature attack trigger");
    let [_, result_segment] = triggered.effects.segments.as_slice() else {
        panic!("expected optional return and result segments: {triggered:#?}");
    };
    let [result_effect] = result_segment.default_effects.as_slice() else {
        panic!("expected one prior-result gate: {result_segment:#?}");
    };
    let result_if = result_effect
        .downcast_ref::<crate::effects::IfEffect>()
        .expect("typed if-you-do gate");
    let [pumped_effect] = result_if.then.as_slice() else {
        panic!("expected one pump: {result_if:#?}");
    };
    let tagged = pumped_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("pump result provenance");
    let pump = tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("typed continuous pump");
    assert!(pump.target_spec.as_ref().is_some_and(|spec| {
        matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == "equipped")
    }));
}
