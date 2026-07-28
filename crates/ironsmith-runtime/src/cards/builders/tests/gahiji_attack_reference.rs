#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::TaggedOpbjectRelation;

fn choose_spec_references_tag(spec: &ChooseSpec, expected: &TagKey) -> bool {
    match spec {
        ChooseSpec::Tagged(tag) => tag == expected,
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => choose_spec_references_tag(spec, expected),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *expected
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            })
        }
        _ => false,
    }
}

#[test]
fn gahiji_keeps_the_attacked_opponent_scope_and_triggering_attacker_reference() {
    let oracle = "Whenever a creature attacks one of your opponents or a planeswalker an opponent controls, that creature gets +2/+0 until end of turn.";
    let definition = parse_oracle_card_definition("Gahiji, Honored One");

    assert_eq!(unprocessed_compiled_lines(&definition).join("\n"), oracle);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Gahiji should have one triggered ability");
    let attacks = triggered
        .trigger
        .downcast_ref::<crate::triggers::AttacksTrigger>()
        .expect("Gahiji should use the typed per-creature attacks trigger");
    assert_eq!(
        attacks
            .filter
            .attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::Opponent)
    );
    assert!(
        attacks.filter.targets_only_player.is_none(),
        "the trigger must also match planeswalkers controlled by opponents"
    );

    let [tag_effect, pump_effect] = triggered.effects.flattened_default_effects() else {
        panic!("the trigger should tag the attacker and pump that exact object: {triggered:#?}");
    };
    let tag = tag_effect
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        .expect("the triggering attacker must receive a stable tag");
    let pump = pump_effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("the consequence should be a typed continuous change");
    assert!(
        pump.target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_tag(spec, &tag.tag)),
        "the pump must remain bound to the triggering attacker: {pump:#?}"
    );
    assert_eq!(pump.until, crate::effect::Until::EndOfTurn);
    assert!(
        matches!(
            pump.runtime_modifications.as_slice(),
            [crate::effects::RuntimeModification::ModifyPowerToughness {
                power: crate::effect::Value::Fixed(2),
                toughness: crate::effect::Value::Fixed(0),
            }]
        ),
        "the tagged attacker should get exactly +2/+0: {pump:#?}"
    );
}
