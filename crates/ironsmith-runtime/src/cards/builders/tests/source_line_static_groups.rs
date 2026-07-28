#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn source_line_static_group_counts(definition: &CardDefinition) -> Vec<usize> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let model = static_ability.compiled_model()?;
            let ironsmith_core::StaticAbilityPayload::SourceLineStaticGroup { member_count } =
                &model.payload
            else {
                return None;
            };
            Some(*member_count)
        })
        .collect()
}

#[test]
fn same_line_static_losses_recombine_from_typed_group_provenance() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Static Loss Group Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("All creatures lose flying and islandwalk.")
        .expect("same-line static loss list should parse");

    assert_eq!(source_line_static_group_counts(&definition), vec![2]);
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["All creatures lose flying and islandwalk.".to_string()]
    );
}

#[test]
fn separately_authored_static_losses_keep_their_line_boundary() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Separate Static Loss Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("All creatures lose flying.\nAll creatures lose islandwalk.")
        .expect("separate static loss lines should parse");

    assert!(source_line_static_group_counts(&definition).is_empty());
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "All creatures lose flying.".to_string(),
            "All creatures lose islandwalk.".to_string(),
        ]
    );
}

#[test]
fn hand_to_hand_recombines_its_coordinated_player_restrictions_exactly() {
    let definition = parse_oracle_card_definition("Hand to Hand");

    assert_eq!(source_line_static_group_counts(&definition), vec![2]);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "During combat, players can't cast instant spells or activate abilities that aren't mana abilities."
    );
}

#[test]
fn mystic_decree_recombines_its_keyword_loss_list_exactly() {
    let definition = parse_oracle_card_definition("Mystic Decree");

    assert_eq!(source_line_static_group_counts(&definition), vec![2]);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "All creatures lose flying and islandwalk."
    );
}

#[test]
fn stasis_field_recombines_layered_static_models_in_authored_surface_order() {
    let definition = parse_oracle_card_definition("Stasis Field");

    assert_eq!(source_line_static_group_counts(&definition), vec![3]);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Enchant creature\n\
         Enchanted creature has base power and toughness 0/2, has defender, and loses all other abilities."
    );
}

#[test]
fn gemcutter_buccaneer_recombines_type_addition_and_mixed_ability_grants() {
    let definition = parse_oracle_card_definition("Gemcutter Buccaneer");

    assert_eq!(source_line_static_group_counts(&definition), vec![4]);
    let structural_equip_grants = definition
        .abilities
        .iter()
        .filter(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return false;
            };
            static_ability.id()
                == crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
                && static_ability.compiled_model().is_some_and(|model| {
                    let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
                        &model.payload
                    else {
                        return false;
                    };
                    matches!(
                        crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(
                            &grant.ability,
                        )
                        .kind,
                        AbilityKind::Activated(_)
                    )
                })
        })
        .count();
    assert_eq!(structural_equip_grants, 2);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Whenever this creature or another Pirate you control enters, create a tapped Treasure token.\n\
         Treasures you control are Equipment in addition to their other types and have \"Equipped creature gets +2/+0,\" equip Pirate {1}, and equip {3}."
    );
}

#[test]
fn brave_the_sands_keeps_its_blocking_rule_on_its_authored_line() {
    let definition = parse_oracle_card_definition("Brave the Sands");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Creatures you control have vigilance.\n\
         Each creature you control can block an additional creature each combat."
    );
}

#[test]
fn brave_the_sands_retains_two_independent_typed_grants() {
    let definition = parse_oracle_card_definition("Brave the Sands");
    let [vigilance_ability, blocking_ability] = definition.abilities.as_slice() else {
        panic!(
            "Brave the Sands should lower to exactly two static grants: {:#?}",
            definition.abilities
        );
    };
    assert!(
        source_line_static_group_counts(&definition).is_empty(),
        "separately authored lines must not gain same-line grouping provenance"
    );

    let AbilityKind::Static(vigilance_static) = &vigilance_ability.kind else {
        panic!("vigilance grant should be static");
    };
    let AbilityKind::Static(blocking_static) = &blocking_ability.kind else {
        panic!("blocking-capacity grant should be static");
    };
    let vigilance_model = vigilance_static
        .compiled_model()
        .expect("vigilance grant should retain its typed model");
    let blocking_model = blocking_static
        .compiled_model()
        .expect("blocking grant should retain its typed model");
    let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(vigilance_grant) =
        &vigilance_model.payload
    else {
        panic!("first ability should be a filtered object-ability grant");
    };
    let ironsmith_core::StaticAbilityPayload::GrantAbility(blocking_grant) =
        &blocking_model.payload
    else {
        panic!("second ability should be a filtered static-ability grant");
    };

    assert_eq!(vigilance_grant.filter, blocking_grant.filter);
    assert_eq!(
        blocking_grant.set_quantifier_surface,
        Some(ironsmith_core::SetQuantifierSurface::Each)
    );
    assert!(matches!(
        &vigilance_grant.ability.kind,
        ironsmith_core::AbilityKind::Static(granted)
            if granted.id == Some(crate::static_abilities::StaticAbilityId::Vigilance)
    ));
    assert!(matches!(
        &blocking_grant.ability.kind,
        ironsmith_core::AbilityKind::Static(granted)
            if matches!(
                granted.payload,
                ironsmith_core::StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(1)
            )
    ));
}
