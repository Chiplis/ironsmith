#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn visit_effects(effect: &Effect, visit: &mut impl FnMut(&Effect)) {
    visit(effect);
    effect.visit_child_effects(&mut |child| visit_effects(child, visit));
}

#[test]
fn mysterious_limousine_returns_only_older_cards_exiled_with_the_source() {
    let definition = parse_oracle_card_definition("Mysterious Limousine");
    let compiled = canonical_compiled_lines(&definition);
    assert_eq!(compiled.last().map(String::as_str), Some("Crew 2"));
    assert!(
        compiled.first().is_some_and(|line| line.ends_with(
            "return each other card exiled with this Vehicle to the battlefield under its owner's control."
        )),
        "the public renderer must keep the exact source-linked return surface: {compiled:#?}"
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::OrTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Mysterious Limousine should retain its enters-or-attacks trigger");

    let mut current_exile_tag = None;
    let mut source_linked_return = None;
    for root in triggered.effects.flattened_default_effects() {
        visit_effects(root, &mut |effect| {
            if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
                && tagged
                    .effect
                    .downcast_ref::<crate::effects::ExileUntilEffect>()
                    .is_some()
            {
                current_exile_tag = Some(tagged.tag.clone());
            }
            if let Some(moved) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
                && moved.zone == Zone::Battlefield
                && moved.exiled_with_source_surface.is_some()
            {
                source_linked_return = Some(moved.clone());
            }
        });
    }

    let current_exile_tag = current_exile_tag.expect("the current exile result should be tagged");
    let returned = source_linked_return.expect("the source-linked return should stay typed");
    let ChooseSpec::All(filter) = returned.target.base() else {
        panic!("the return should exhaust the linked collection: {returned:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Exile));
    assert!(!filter.other, "`other` must not become source-relative");
    assert!(filter.card_types.is_empty(), "{filter:#?}");
    assert!(filter.subtypes.is_empty(), "{filter:#?}");
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == current_exile_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    }));
    let surface = returned
        .exiled_with_source_surface
        .as_ref()
        .expect("the exact source-linked return surface should survive lowering");
    assert_eq!(
        surface.subject,
        ironsmith_core::ExiledWithSourceSubjectSurface::Custom("each other card".to_string())
    );
}
