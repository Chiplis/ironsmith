#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn flying_creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .flying()
        .build()
}

fn flying_artifact_creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .flying()
        .build()
}

fn flying_artifact(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .flying()
        .build()
}

fn colored_flying_creature(name: &str, color: crate::color::ColorSet) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(2, 2))
        .flying()
        .build()
}

#[test]
fn ray_of_frost_keeps_attachment_relative_color_condition_and_exact_surface() {
    let definition = parse_oracle_card_definition("Ray of Frost");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Flash",
            "Enchant creature",
            "When this Aura enters, if enchanted creature is red, tap it.",
            "As long as enchanted creature is red, it loses all abilities.",
            "Enchanted creature doesn't untap during its controller's untap step."
        ]
    );

    let conditional_loss = definition.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        let Some(model) = static_ability.compiled_model() else {
            return false;
        };
        let ironsmith_core::StaticAbilityPayload::AttachedAbilityGrant(grant) = &model.payload
        else {
            return false;
        };
        matches!(
            &grant.condition,
            Some(crate::ConditionExpr::AttachedToSourceMatches(filter))
                if filter.colors == Some(crate::color::ColorSet::RED)
        ) && matches!(
            &grant.ability.kind,
            ironsmith_core::AbilityKind::Static(granted)
                if granted.id == Some(StaticAbilityId::RemoveAllAbilitiesForFilter)
        )
    });
    assert!(
        conditional_loss,
        "Ray of Frost must carry a typed red-host condition on the attached-object ability loss: {:#?}",
        definition.abilities
    );
}

#[test]
fn ray_of_frost_removes_abilities_only_from_an_attached_red_creature() {
    let definition = parse_oracle_card_definition("Ray of Frost");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let red_host = game.create_object_from_definition(
        &colored_flying_creature("Red Host", crate::color::ColorSet::RED),
        alice,
        Zone::Battlefield,
    );
    let blue_host = game.create_object_from_definition(
        &colored_flying_creature("Blue Host", crate::color::ColorSet::BLUE),
        alice,
        Zone::Battlefield,
    );
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    assert!(game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(red_host),));
    assert!(
        !game.object_has_static_ability_id(red_host, StaticAbilityId::Flying),
        "the attached red creature should lose all abilities"
    );
    assert!(
        game.object_has_static_ability_id(blue_host, StaticAbilityId::Flying),
        "an unrelated creature must keep its abilities"
    );

    assert!(game.detach_object_from_current_target(aura));
    assert!(
        game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(blue_host),)
    );
    assert!(
        game.object_has_static_ability_id(red_host, StaticAbilityId::Flying),
        "the former host should regain its abilities after detachment"
    );
    assert!(
        game.object_has_static_ability_id(blue_host, StaticAbilityId::Flying),
        "a nonred attached creature is the semantic near miss and must keep its abilities"
    );
}

#[test]
fn named_attached_anthems_render_keyword_loss_as_a_direct_continuous_effect() {
    let hammer = parse_oracle_card_definition("Colossus Hammer");
    assert_eq!(
        canonical_compiled_lines(&hammer).join("\n"),
        "Equipped creature gets +10/+10 and loses flying.\nEquip {8}"
    );
    let coils = parse_oracle_card_definition("Tightening Coils");
    assert_eq!(
        canonical_compiled_lines(&coils).join("\n"),
        "Enchant creature\nEnchanted creature gets -6/-0 and loses flying."
    );
    let short_circuit = parse_oracle_card_definition("Short Circuit");
    assert_eq!(
        canonical_compiled_lines(&short_circuit).join("\n"),
        "Flash\nEnchant artifact or creature\nAs long as enchanted permanent is a creature, it gets -3/-0 and loses flying."
    );

    for definition in [&hammer, &coils, &short_circuit] {
        let debug = format!("{:#?}", definition.abilities);
        assert!(debug.contains("RemoveAbilityForFilter"), "{debug}");
        assert!(debug.contains("Flying"), "{debug}");
        assert!(
            !debug.contains("GrantAbility(GrantAbility"),
            "keyword loss must not be represented as a quoted nested ability grant: {debug}"
        );
    }
}

#[test]
fn sky_tether_keeps_the_compound_grant_and_loss_surface() {
    let definition = parse_oracle_card_definition("Sky Tether");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Enchant creature",
            "Enchanted creature has defender and loses flying."
        ]
    );

    let mut grant_filter = None;
    let mut loss_filter = None;
    for ability in &definition.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(model) = static_ability.compiled_model() else {
            continue;
        };
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant)
                if matches!(
                    &grant.ability.kind,
                    ironsmith_core::AbilityKind::Static(granted)
                        if granted.id == Some(StaticAbilityId::Defender)
                ) =>
            {
                grant_filter = Some(grant.filter.clone());
            }
            ironsmith_core::StaticAbilityPayload::RemoveAbilityForFilter {
                filter,
                ability,
                mode: ironsmith_core::AbilityLossMode::Lose,
            } if ability.id == Some(StaticAbilityId::Flying) => {
                loss_filter = Some(filter.clone());
            }
            _ => {}
        }
    }
    let grant_filter = grant_filter.expect("defender should be a typed filtered grant");
    let loss_filter = loss_filter.expect("flying should be a typed filtered loss");
    assert_eq!(grant_filter, loss_filter);
    assert!(
        grant_filter.static_abilities.is_empty(),
        "defender must be granted, not embedded in the affected-object filter: {grant_filter:#?}"
    );
}

#[test]
fn sky_tether_grants_defender_and_removes_flying_only_while_attached() {
    let definition = parse_oracle_card_definition("Sky Tether");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let host = game.create_object_from_definition(
        &flying_creature("Tethered Flier", 2, 2),
        alice,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &flying_creature("Untethered Flier", 3, 3),
        alice,
        Zone::Battlefield,
    );
    let tether = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.remove_summoning_sickness(host);
    game.remove_summoning_sickness(bystander);

    assert!(game.can_attack(host));
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Defender));

    assert!(game.attach_object_to_target(tether, crate::object::AttachmentTarget::Object(host)));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Defender));
    assert!(!game.can_attack(host));
    assert!(game.object_has_static_ability_id(bystander, StaticAbilityId::Flying));
    assert!(!game.object_has_static_ability_id(bystander, StaticAbilityId::Defender));
    assert!(game.can_attack(bystander));

    assert!(game.detach_object_from_current_target(tether));
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Defender));
    assert!(game.can_attack(host));
}

#[test]
fn colossus_hammer_removes_flying_only_while_attached() {
    let definition = parse_oracle_card_definition("Colossus Hammer");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let host = game.create_object_from_definition(
        &flying_creature("Hammer Host", 2, 2),
        alice,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &flying_creature("Flying Bystander", 3, 3),
        alice,
        Zone::Battlefield,
    );
    let hammer = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(host), Some(2));
    assert!(game.attach_object_to_target(hammer, crate::object::AttachmentTarget::Object(host),));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(host), Some(12));
    assert_eq!(game.calculated_toughness(host), Some(12));
    assert!(
        game.object_has_static_ability_id(bystander, StaticAbilityId::Flying),
        "an attached Hammer must not remove flying from unrelated creatures"
    );

    assert!(game.detach_object_from_current_target(hammer));
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(host), Some(2));
    assert_eq!(game.calculated_toughness(host), Some(2));
}

#[test]
fn tightening_coils_removes_flying_and_reduces_only_the_enchanted_creature() {
    let definition = parse_oracle_card_definition("Tightening Coils");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let host = game.create_object_from_definition(
        &flying_creature("Coils Host", 7, 7),
        alice,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &flying_creature("Other Flier", 4, 4),
        alice,
        Zone::Battlefield,
    );
    let coils = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(coils, crate::object::AttachmentTarget::Object(host),));

    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(host), Some(1));
    assert_eq!(game.calculated_toughness(host), Some(7));
    assert!(game.object_has_static_ability_id(bystander, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(bystander), Some(4));

    assert!(game.detach_object_from_current_target(coils));
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(host), Some(7));
}

#[test]
fn high_score_equipment_lines_remove_flying_only_while_attached() {
    for (card_name, power_bonus, toughness_bonus) in
        [("Magebane Armor", 2, 4), ("Starforged Sword", 3, 3)]
    {
        let definition = parse_oracle_card_definition(card_name);
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let host = game.create_object_from_definition(
            &flying_creature("Equipment Host", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let bystander = game.create_object_from_definition(
            &flying_creature("Equipment Bystander", 3, 3),
            alice,
            Zone::Battlefield,
        );
        let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

        assert!(
            game.attach_object_to_target(equipment, crate::object::AttachmentTarget::Object(host),)
        );
        assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Flying));
        assert_eq!(game.calculated_power(host), Some(2 + power_bonus));
        assert_eq!(game.calculated_toughness(host), Some(2 + toughness_bonus));
        assert!(game.object_has_static_ability_id(bystander, StaticAbilityId::Flying));

        assert!(game.detach_object_from_current_target(equipment));
        assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
        assert_eq!(game.calculated_power(host), Some(2));
        assert_eq!(game.calculated_toughness(host), Some(2));
    }
}

#[test]
fn short_circuit_applies_both_effects_only_while_its_host_is_a_creature() {
    let definition = parse_oracle_card_definition("Short Circuit");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let creature_host = game.create_object_from_definition(
        &flying_artifact_creature("Circuit Creature", 5, 5),
        alice,
        Zone::Battlefield,
    );
    let noncreature_host = game.create_object_from_definition(
        &flying_artifact("Circuit Artifact"),
        alice,
        Zone::Battlefield,
    );
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    assert!(
        game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(creature_host),)
    );
    assert!(!game.object_has_static_ability_id(creature_host, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(creature_host), Some(2));

    assert!(game.detach_object_from_current_target(aura));
    assert!(game.object_has_static_ability_id(creature_host, StaticAbilityId::Flying));
    assert_eq!(game.calculated_power(creature_host), Some(5));

    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(noncreature_host),
    ));
    assert!(
        game.object_has_static_ability_id(noncreature_host, StaticAbilityId::Flying),
        "the conditional loss must stay inactive while the enchanted permanent is not a creature"
    );
}
