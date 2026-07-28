#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::types::{CardType, Subtype, Supertype};

fn anthem_filters(definition: &CardDefinition) -> Vec<&crate::target::ObjectFilter> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let model = static_ability.compiled_model()?;
            let ironsmith_core::StaticAbilityPayload::Anthem(anthem) = &model.payload else {
                return None;
            };
            anthem.filter.as_ref()
        })
        .collect()
}

fn assert_shared_creature_head_filter(
    filter: &crate::target::ObjectFilter,
    supertype: Supertype,
    subtype: Subtype,
) {
    assert_eq!(filter.zone, Some(Zone::Battlefield), "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
    assert!(filter.other, "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().all(|branch| {
        branch.zone.is_none()
            && branch.controller.is_none()
            && branch.card_types.is_empty()
            && !branch.other
    }));
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.supertypes == [supertype]),
        "{filter:#?}"
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [subtype]),
        "{filter:#?}"
    );
}

#[test]
fn synthetic_supertype_subtype_subject_is_one_semantic_anthem() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Shared Head Anthem Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Other legendary and Elf creatures you control get +2/+2.")
        .expect("shared-head synthetic anthem should parse");
    let filters = anthem_filters(&definition);

    assert_eq!(filters.len(), 1, "{:#?}", definition.abilities);
    assert_shared_creature_head_filter(filters[0], Supertype::Legendary, Subtype::Elf);
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["Other legendary and Elf creatures you control get +2/+2.".to_string()]
    );
}

#[test]
fn subtype_only_shared_head_remains_one_anthem() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Subtype Shared Head Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Other Ninja and Rogue creatures you control get +1/+1.")
        .expect("subtype shared-head anthem should parse");

    assert_eq!(anthem_filters(&definition).len(), 1);
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["Other Ninja and Rogue creatures you control get +1/+1.".to_string()]
    );
}

#[test]
fn narfi_preserves_the_shared_creature_and_controller_scope_exactly() {
    let definition = parse_oracle_card_definition("Narfi, Betrayer King");
    let filters = anthem_filters(&definition);

    assert_eq!(filters.len(), 1, "{:#?}", definition.abilities);
    assert_shared_creature_head_filter(filters[0], Supertype::Snow, Subtype::Zombie);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Other snow and Zombie creatures you control get +1/+1.\n\
         {S}{S}{S}: Return this card from your graveyard to the battlefield tapped."
    );
}
