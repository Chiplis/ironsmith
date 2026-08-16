#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "{T}: Add {C}.\n{2}: This land becomes a 2/2 Assembly-Worker artifact creature until end of turn. It's still a land.\n{1}, {T}: Target attacking Assembly-Worker gets +2/+2 until end of turn.";

fn modifications(
    effect: &crate::effects::ApplyContinuousEffect,
) -> impl Iterator<Item = &crate::continuous::Modification> {
    effect
        .modification
        .iter()
        .chain(effect.additional_modifications.iter())
}

#[test]
fn mishras_foundry_keeps_assembly_worker_on_both_activated_abilities() {
    let definition = parse_oracle_card_definition("Mishra's Foundry");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.mana_output.is_none() => Some(activated),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [animation, pump] = activated.as_slice() else {
        panic!("expected animation and pump activations: {activated:#?}");
    };

    let animation_effect = animation
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .expect("animation should be one typed continuous effect");
    assert!(modifications(animation_effect).any(|modification| matches!(
        modification,
        crate::continuous::Modification::AddCardTypes(types)
            if types.contains(&CardType::Artifact) && types.contains(&CardType::Creature)
    )));
    assert!(modifications(animation_effect).any(|modification| matches!(
        modification,
        crate::continuous::Modification::AddSubtypes(subtypes)
            if subtypes == &[Subtype::AssemblyWorker]
    )));

    let pump_effect = pump
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .expect("pump should be one typed continuous effect");
    let Some(ChooseSpec::Target(target)) = &pump_effect.target_spec else {
        panic!("pump must target an Assembly-Worker: {pump_effect:#?}");
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("pump target must be an object filter: {target:#?}");
    };
    assert!(filter.attacking, "{filter:#?}");
    assert_eq!(filter.subtypes, [Subtype::AssemblyWorker], "{filter:#?}");
}
