#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn tawnos_toymaker_keeps_the_typed_copy_exception() {
    let definition = parse_oracle_card_definition("Tawnos, the Toymaker");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Tawnos should have a triggered ability");
    let may = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::MayEffect>())
        .expect("Tawnos's copy instruction should remain optional");
    let [copy_effect, type_effect] = may.effects.as_slice() else {
        panic!("expected copy plus typed exception, got {may:#?}");
    };

    let copy = copy_effect
        .downcast_ref::<TaggedEffect>()
        .and_then(|tagged| tagged.effect.downcast_ref::<WithIdEffect>())
        .and_then(|with_id| {
            with_id
                .effect
                .downcast_ref::<crate::effects::CopySpellEffect>()
        })
        .expect("Tawnos should retain a typed copy-spell action");
    assert!(matches!(
        copy.target.base(),
        ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
    ));
    assert_eq!(
        copy.target_reference_kind,
        Some(crate::filter::StackObjectKind::Spell),
        "Tawnos's triggering `it` must retain the spell-cast trigger domain"
    );

    let add_type = type_effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("Tawnos should modify the copied stack object");
    assert!(matches!(
        add_type.target_spec.as_ref().map(ChooseSpec::base),
        Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "__copied_stack_object__"
    ));
    assert_eq!(
        add_type.modification,
        Some(crate::continuous::Modification::AddCardTypes(vec![
            CardType::Artifact
        ]))
    );
    assert_eq!(
        add_type.type_retention_surface,
        Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes)
    );

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Whenever you cast a Beast or Bird creature spell, you may copy that spell, except the copy is an artifact in addition to its other types."
    );
}
