#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn triggered_ability(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected a triggered ability")
}

#[test]
fn gilt_leaf_winnower_keeps_power_toughness_inequality() {
    let definition = parse_oracle_card_definition("Gilt-Leaf Winnower");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Gilt-Leaf Winnower"]
    );

    let destroy = triggered_ability(&definition)
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DestroyEffect>())
        .expect("the enter trigger should retain a typed destroy effect");
    let ChooseSpec::Object(filter) = destroy.spec.base() else {
        panic!("the destroy effect should target an object: {destroy:#?}");
    };
    assert_eq!(
        filter.power_toughness_relation,
        Some(crate::filter::PowerToughnessRelation::NotEqual)
    );
    assert!(filter.excluded_subtypes.contains(&Subtype::Elf));
}

#[test]
fn olorin_keeps_each_opponents_exile_partition_and_power_metric() {
    let definition = parse_oracle_card_definition("Olórin's Searing Light");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Olórin's Searing Light"]
    );

    fn has_iterated_exiled_first_power(effect: &crate::effect::Effect) -> bool {
        if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
            && let crate::effect::Value::PriorEffectMetric { query, .. } = damage.amount.unhinted()
            && query.metric == ironsmith_core::EffectMetric::FirstPower
            && query.player == Some(PlayerFilter::IteratedPlayer)
            && query.action == Some(ironsmith_core::PriorEffectAction::Exiled)
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            found |= has_iterated_exiled_first_power(child);
        });
        found
    }

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Olórin should retain a spell program");
    assert!(
        program.segments.iter().any(|segment| {
            segment
                .default_effects
                .iter()
                .any(has_iterated_exiled_first_power)
        }),
        "the damage amount must query each iterated opponent's own exiled creature: {program:#?}"
    );
}

#[test]
fn nashi_uses_card_copy_cast_semantics_without_a_stack_spell_copy() {
    let definition = parse_oracle_card_definition("Nashi, Moon's Legacy");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Nashi, Moon's Legacy"]
    );

    let debug = format!("{:#?}", triggered_ability(&definition).effects);
    assert!(
        debug.contains("CastTaggedEffect") && debug.contains("as_copy: true"),
        "Nashi should cast the copy of the selected graveyard card: {debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "the selected graveyard card is not a spell on the stack: {debug}"
    );
}

#[test]
fn yidris_keeps_hand_origin_cascade_grant_until_end_of_turn() {
    let definition = parse_oracle_card_definition("Yidris, Maelstrom Wielder");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Yidris, Maelstrom Wielder"]
    );

    let apply = triggered_ability(&definition)
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .expect("Yidris should retain a typed temporary cascade grant");
    let crate::continuous::EffectTarget::Filter(filter) = &apply.target else {
        panic!("Yidris should apply the grant to a spell filter: {apply:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(filter.cast_by, Some(PlayerFilter::You));
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert!(filter.has_as_you_cast_this_turn_surface());
    assert_eq!(apply.until, crate::effect::Until::EndOfTurn);
    assert!(matches!(
        &apply.modification,
        Some(crate::continuous::Modification::AddAbility(ability))
            if ability.id() == StaticAbilityId::Cascade
    ));
}
