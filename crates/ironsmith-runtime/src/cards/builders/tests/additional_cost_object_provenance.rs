use super::shard_16::parse_oracle_card_definition;
use super::*;

const CASES: &[(&str, &str)] = &[
    (
        "Endemic Plague",
        "As an additional cost to cast this spell, sacrifice a creature.\nDestroy all creatures that share a creature type with the sacrificed creature. They can't be regenerated.",
    ),
    (
        "Foundry Helix",
        "As an additional cost to cast this spell, sacrifice a permanent.\nFoundry Helix deals 4 damage to any target. If the sacrificed permanent was an artifact, you gain 4 life.",
    ),
    (
        "Hellish Sideswipe",
        "As an additional cost to cast this spell, sacrifice an artifact or creature.\nDestroy target creature or Vehicle. If the sacrificed permanent was a Vehicle, draw a card.",
    ),
    (
        "Mind Extraction",
        "As an additional cost to cast this spell, sacrifice a creature.\nTarget player reveals their hand and discards all cards of each of the sacrificed creature's colors.",
    ),
    (
        "Soul Exchange",
        "As an additional cost to cast this spell, exile a creature you control.\nReturn target creature card from your graveyard to the battlefield. Put a +2/+2 counter on that creature if the exiled creature was a Thrull.",
    ),
    (
        "Splitting the Powerstone",
        "As an additional cost to cast this spell, sacrifice an artifact.\nCreate two tapped Powerstone tokens. If the sacrificed artifact was legendary, draw a card.",
    ),
];

fn contains_sacrifice_cost_tag(debug: &str) -> bool {
    // Spell additional costs currently have two generic lowering shapes: a
    // direct cost tag and a choose-then-sacrifice tag. Both name the paid cost
    // object; neither is ordinary spell-body antecedent memory.
    debug.contains("sacrifice_cost_0") || debug.contains("sacrificed_0")
}

fn find_discard_effect(effect: &crate::effect::Effect) -> Option<crate::effects::DiscardEffect> {
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        return Some(discard.clone());
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_discard_effect(child);
        }
    });
    found
}

fn additional_cost_object_condition(
    definition: &crate::CardDefinition,
) -> (
    &crate::TagKey,
    &crate::target::ObjectFilter,
    ironsmith_core::ConditionalSurface,
) {
    let conditional = definition
        .spell_effect
        .as_ref()
        .into_iter()
        .flat_map(|program| &program.segments)
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
        .expect("card should contain an additional-cost object conditional");
    let crate::effect::Condition::TaggedObjectMatches(tag, filter) = &conditional.condition else {
        panic!(
            "additional-cost object conditional should retain a tagged predicate: {conditional:#?}"
        );
    };
    (tag, filter, conditional.surface)
}

#[test]
fn additional_cost_object_cluster_compiles_to_canonical_surface() {
    for (name, oracle) in CASES {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            compiled_text_lines(&definition).join("\n"),
            *oracle,
            "{name} must preserve its additional-cost antecedent"
        );
    }
}

#[test]
fn additional_cost_object_cluster_keeps_typed_cost_relations() {
    for (name, kind) in [
        (
            "Foundry Helix",
            crate::target::SacrificedObjectKind::Permanent,
        ),
        (
            "Hellish Sideswipe",
            crate::target::SacrificedObjectKind::Permanent,
        ),
        (
            "Splitting the Powerstone",
            crate::target::SacrificedObjectKind::Artifact,
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let (tag, filter, _) = additional_cost_object_condition(&definition);
        assert!(
            matches!(tag.as_str(), "sacrificed_0" | "sacrifice_cost_0")
                && matches!(
                    filter.additional_cost_object_surface(),
                    Some(ironsmith_core::AdditionalCostObjectSurface {
                        action: ironsmith_core::AdditionalCostObjectAction::Sacrificed,
                        kind: actual_kind,
                    }) if actual_kind == kind
                ),
            "{name} must test the sacrificed cost object, not the preceding spell effect: {definition:#?}"
        );
    }

    let endemic = format!("{:#?}", parse_oracle_card_definition("Endemic Plague"));
    assert!(
        endemic.contains("SharesSubtypeWithTagged") && contains_sacrifice_cost_tag(&endemic),
        "Endemic Plague must destroy only creatures linked to its cost object: {endemic}"
    );

    let mind_definition = parse_oracle_card_definition("Mind Extraction");
    let mind_discard = mind_definition
        .spell_effect
        .as_ref()
        .into_iter()
        .flat_map(|program| program.flattened_default_effects())
        .find_map(find_discard_effect)
        .expect("Mind Extraction should contain a discard effect");
    let mind_filter = mind_discard
        .card_filter
        .as_ref()
        .expect("Mind Extraction must restrict which hand cards are discarded");
    let crate::effect::Value::Count(mind_count_filter) = mind_discard.count.unhinted() else {
        panic!("Mind Extraction must discard the full matching set: {mind_definition:#?}");
    };
    assert_eq!(
        mind_count_filter, mind_filter,
        "Mind Extraction's count and eligible-card filter must identify the same hand-card set"
    );
    let mind = format!("{mind_definition:#?}");
    assert!(
        mind.contains("SharesColorWithTagged")
            && contains_sacrifice_cost_tag(&mind)
            && mind_filter.zone == Some(crate::Zone::Hand)
            && mind_filter.owner.as_ref() == Some(&mind_discard.player)
            && matches!(
                mind_filter.additional_cost_object_surface(),
                Some(ironsmith_core::AdditionalCostObjectSurface {
                    action: ironsmith_core::AdditionalCostObjectAction::Sacrificed,
                    kind: crate::target::SacrificedObjectKind::Creature,
                })
            ),
        "Mind Extraction must discard the complete color-matched set: {mind}"
    );

    let soul_definition = parse_oracle_card_definition("Soul Exchange");
    let (soul_tag, soul_filter, soul_surface) = additional_cost_object_condition(&soul_definition);
    assert!(
        (soul_tag.as_str().starts_with("exile_cost_")
            || soul_tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
            && matches!(
                soul_filter.additional_cost_object_surface(),
                Some(ironsmith_core::AdditionalCostObjectSurface {
                    action: ironsmith_core::AdditionalCostObjectAction::Exiled,
                    kind: crate::target::SacrificedObjectKind::Creature,
                })
            ),
        "Soul Exchange must test the exiled cost creature, not the returned creature: {soul_definition:#?}"
    );
    assert_eq!(
        soul_surface,
        ironsmith_core::ConditionalSurface::TrailingIf,
        "Soul Exchange must retain the authored trailing-if condition surface"
    );
}
