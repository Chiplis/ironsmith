//! Authored source assertions. Execution is deliberately deferred.
use super::*;
use ironsmith_core::StaticAbilityPayload as P;
fn lex(text: &str) -> Vec<OwnedLexToken> { crate::lexer::lex_line(text, 0).unwrap() }

#[test]
fn land_descriptor_keeps_size_color_subtype_and_live_recipient() {
    let abilities = parse_lands_are_pt_creatures_still_lands_line(&lex(
        "Forests you control are 1/1 green Elf creatures that are still lands.")).unwrap().unwrap();
    assert_eq!(abilities.len(), 4);
    let P::AddCardTypes { filter, card_types } = &abilities[0].payload else { panic!("{abilities:?}"); };
    assert_eq!(card_types, &[CardType::Creature]);
    assert_eq!(filter.subtypes, [Subtype::Forest]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert!(matches!(&abilities[1].payload, P::AddSubtypes { subtypes, .. } if subtypes == &[Subtype::Elf]));
    assert!(matches!(&abilities[2].payload, P::SetColors { colors, .. } if *colors == ColorSet::GREEN));
    assert!(matches!(&abilities[3].payload, P::SetBasePowerToughness { power: 1, toughness: 1, .. }));
}

#[test]
fn chosen_type_possession_owns_explicit_graveyard_domain() {
    let abilities = parse_subject_are_card_types_in_addition_to_their_other_types_line(&lex(
        "Each creature card in your graveyard has the chosen creature type in addition to its other types.")).unwrap().unwrap();
    let P::AddChosenCreatureType { filter, .. } = &abilities[0].payload else { panic!("{abilities:?}"); };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert!(!filter.source);
    let vehicles = parse_subject_are_card_types_in_addition_to_their_other_types_line(&lex(
        "Vehicle creatures you control are the chosen creature type in addition to their other types.")).unwrap().unwrap();
    let P::AddChosenCreatureType { filter, .. } = &vehicles[0].payload else { panic!("{vehicles:?}"); };
    assert_eq!(filter.subtypes, [Subtype::Vehicle]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}

#[test]
fn bare_additive_types_do_not_become_replacement_types_or_card_types() {
    let ability = parse_subject_is_also_subtypes_line(&lex(
        "This creature is also a Cleric, Rogue, Warrior, and Wizard.")).unwrap().unwrap();
    let P::AddSubtypes { filter, subtypes } = &ability.payload else { panic!("{ability:?}"); };
    assert!(filter.is_source_only());
    assert_eq!(subtypes, &[Subtype::Cleric, Subtype::Rogue, Subtype::Warrior, Subtype::Wizard]);
}

#[test]
fn conditional_unsized_animation_keeps_attachment_antecedent_and_no_size() {
    let abilities = parse_conditional_copular_creature_line(&lex(
        "As long as enchanted permanent is a Vehicle, it's a creature in addition to its other types.")).unwrap().unwrap();
    assert_eq!(abilities.len(), 1);
    let StaticAbilityAst::Static(ability) = &abilities[0] else { panic!("{abilities:?}"); };
    let P::Conditional { ability, .. } = &ability.payload else { panic!("{ability:?}"); };
    let P::AddCardTypes { filter, card_types } = &ability.payload else { panic!("{ability:?}"); };
    assert_eq!(card_types, &[CardType::Creature]);
    assert!(!filter.source);
    assert!(filter.tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == "enchanted"));
}

#[test]
fn conditional_artifact_creatures_retain_original_types_and_optional_printed_size() {
    for (text, len) in [
        ("As long as this Vehicle has three or more fire counters on it, it's an artifact creature.", 1),
        ("Metalcraft — This artifact is a 5/5 Golem artifact creature as long as you control three or more artifacts.", 3),
    ] {
        let abilities = parse_conditional_copular_creature_line(&lex(text)).unwrap().unwrap();
        assert_eq!(abilities.len(), len, "{abilities:?}");
        for ability in &abilities { assert!(super::super::static_ability_ast_has_explicit_condition(ability)); }
        let debug = format!("{abilities:?}");
        assert!(debug.contains("AddCardTypes"));
        assert!(!debug.contains("SetCardTypes"));
        assert_eq!(debug.contains("SetBasePowerToughness"), len == 3);
    }
}

#[test]
fn complete_copular_readers_reject_hidden_tail_quote_and_subject_tokens() {
    for text in [
        "This creature is also a Cleric and.",
        "This creature is also a Cleric until end of turn.",
        "This creature is also a Cleric with flying.",
        "This creature nonsense is also a Cleric.",
        "This creature is also a \"Cleric\".",
    ] { assert!(parse_subject_is_also_subtypes_line(&lex(text)).unwrap().is_none(), "{text}"); }
    for text in [
        "All Swamps are 1/1 black creatures that are still lands and have flying.",
        "All Swamps nonsense are 1/1 black creatures that are still lands.",
        "All Swamps are 1/1 black mystery creatures that are still lands.",
        "All Swamps are \"1/1\" black creatures that are still lands.",
    ] { assert!(parse_lands_are_pt_creatures_still_lands_line(&lex(text)).unwrap().is_none(), "{text}"); }
    for text in [
        "All nonland permanents are the chosen color until end of turn.",
        "Enchanted land nonsense is the chosen color.",
        "Enchanted land is the chosen color and has flying.",
    ] { assert!(parse_subject_is_chosen_color_line(&lex(text)).unwrap().is_none(), "{text}"); }
    for text in [
        "As long as enchanted permanent is a Vehicle, it's a creature in addition to its other types and has flying.",
        "As long as this Vehicle has three or more fire counters on it, it's an artifact creature until end of turn.",
    ] { assert!(parse_conditional_copular_creature_line(&lex(text)).unwrap().is_none(), "{text}"); }
}

#[test]
fn real_subject_heads_reach_the_indexed_static_registry() {
    for (text, payload) in [
        ("Forests you control are 1/1 green Elf creatures that are still lands.", "SetBasePowerToughness"),
        ("All Swamps are 1/1 black creatures that are still lands.", "SetColors"),
        ("Enchanted land is every basic land type in addition to its other types.", "AddSubtypes"),
        ("All nonland permanents are legendary.", "AddSupertypes"),
        ("Permanents with ice counters on them are snow.", "AddSupertypes"),
        ("As long as enchanted permanent is a Vehicle, it's a creature in addition to its other types.", "AddCardTypes"),
        ("Metalcraft — This artifact is a 5/5 Golem artifact creature as long as you control three or more artifacts.", "AddCardTypes"),
    ] {
        let (parsed, loss) = crate::parse_loss::capture(|| super::super::parse_static_ability_ast_line_lexed(&lex(text)));
        assert!(!loss.is_lossy(), "{text}: {}", loss.reasons_text());
        let parsed = parsed.unwrap().expect(text);
        assert!(format!("{parsed:?}").contains(payload), "{text}: {parsed:?}");
    }
}

#[test]
fn new_copular_subjects_do_not_drop_replacement_or_dangling_connective_words() {
    for text in ["All Swamps instead are 1/1 black creatures that are still lands.",
        "All Swamps and are 1/1 black creatures that are still lands."] {
        assert!(parse_lands_are_pt_creatures_still_lands_line(&lex(text)).unwrap().is_none());
    }
    for text in [
        "As long as this isn't on the battlefield, it's a creature in addition to its other types.",
        "As long as this isn't on the battlefield, it is an artifact creature.",
        "As long as this isn't on the battlefield, it's an artifact creature.",
    ] {
        let tokens = lex(text);
        assert!(parse_conditional_copular_creature_line(&tokens).is_err());
        assert!(super::super::parse_static_ability_ast_line_lexed(&tokens).is_err());
    }
}

#[test]
fn compound_attached_pronouns_cannot_fall_back_to_the_aura_source() {
    let tokens = lex("As long as enchanted permanent is a Vehicle and you control three or more artifacts, it's a creature in addition to its other types.");
    assert!(parse_conditional_copular_creature_line(&tokens).is_err());
    assert!(super::super::parse_static_ability_ast_line_lexed(&tokens).is_err());
}

#[test]
fn existing_source_color_and_sized_animation_owners_keep_one_registry_reading() {
    let color = lex("This artifact is the chosen color.");
    assert!(parse_subject_is_chosen_color_line(&color).unwrap().is_none());
    let old_color = super::super::parse_source_is_chosen_color_line(&color).unwrap().unwrap();
    let routed = super::super::parse_static_ability_ast_line_lexed(&color).unwrap().unwrap();
    assert_eq!(routed.len(), 1);
    assert_eq!(format!("{:?}", routed[0]), format!("{:?}", StaticAbilityAst::Static(old_color)));

    let sized = lex("As long as you control three or more artifacts, it's a 1/1 Insect creature in addition to its other types.");
    let old = parse_filter_is_pt_creature_in_addition_line(&sized).unwrap().unwrap();
    assert!(parse_conditional_copular_creature_line(&sized).unwrap().is_none());
    let routed = super::super::parse_static_ability_ast_line_lexed(&sized).unwrap().unwrap();
    assert_eq!(format!("{routed:?}"), format!("{old:?}"));
}

#[test]
fn explicit_it_and_contracted_artifact_creatures_preserve_the_same_card_types() {
    for text in [
        "As long as this Vehicle has three or more fire counters on it, it is an artifact creature.",
        "As long as this Vehicle has three or more fire counters on it, it's an artifact creature.",
    ] {
        let routed = super::super::parse_static_ability_ast_line_lexed(&lex(text)).unwrap().unwrap();
        assert_eq!(routed.len(), 1);
        let debug = format!("{routed:?}");
        assert!(debug.contains("AddCardTypes"), "{debug}"); assert!(!debug.contains("SetCardTypes"), "{debug}");
    }
}

#[test]
fn explicit_chosen_creature_family_does_not_infer_a_basic_land_choice_from_recipients() {
    for text in [
        "Each land you control has the chosen creature type in addition to its other types.",
        "Each land creature you control has the chosen creature type in addition to its other types.",
        "Lands and creatures you control have the chosen creature type in addition to their other types.",
    ] {
        let abilities = parse_subject_are_card_types_in_addition_to_their_other_types_line(&lex(text)).unwrap().unwrap();
        assert!(matches!(&abilities[0].payload, P::AddChosenCreatureType { .. }), "{text}: {abilities:?}");
    }
    let bare = parse_subject_are_card_types_in_addition_to_their_other_types_line(&lex(
        "Lands you control are the chosen type in addition to their other types.")).unwrap().unwrap();
    assert!(matches!(&bare[0].payload, P::AddChosenBasicLandType { .. }));
}

#[test]
fn chosen_type_complete_subject_preserves_nonbattlefield_and_repeated_scopes() {
    let offboard = complete_characteristic_subject(&lex(
        "creature cards you own that aren't on the battlefield")).unwrap().unwrap();
    assert_eq!(offboard.zone, None, "do not clamp the outer union to battlefield");
    assert_eq!(offboard.any_of.len(), 7);
    for branch in &offboard.any_of {
        assert_eq!(branch.owner, Some(PlayerFilter::You));
        assert_eq!(branch.controller, None);
        assert_eq!(branch.card_types, [CardType::Creature]);
        assert!(!matches!(branch.zone, None | Some(Zone::Battlefield | Zone::OutsideGame)));
    }
    // Independently assert the outer set has no recipient restrictions and
    // each complete nominal retains only its own qualifiers.
    let tokens = lex("Slivers you control and nontoken creatures you control");
    let union = complete_characteristic_subject(&tokens).unwrap().unwrap();
    assert_eq!(union.any_of.len(), 2);
    assert_eq!(union.zone, None);
    assert_eq!(union.controller, None);
    assert!(!union.nontoken);
    assert!(!union.tapped);
    assert!(union.has_conjunctive_set_surface());
    for branch in &union.any_of {
        assert_eq!(branch.controller, Some(PlayerFilter::You));
        assert_eq!(branch.zone, Some(Zone::Battlefield));
    }
    assert!(union.any_of.iter().any(|branch| branch.subtypes.contains(&Subtype::Sliver)
        && !branch.nontoken));
    assert!(union.any_of.iter().any(|branch| branch.card_types.contains(&CardType::Creature)
        && branch.nontoken));
}

#[test]
fn chosen_type_extended_subjects_consume_all_qualifiers_or_decline() {
    for text in [
        "mystery creature cards you own that aren't on the battlefield",
        "creature cards you own that aren't on the battlefield mystery",
        "creature cards in your graveyard that aren't on the battlefield",
        "Slivers you control and mystery nontoken creatures you control",
        "Slivers you control and nontoken creatures you control until end of turn",
        "Slivers you control and",
        "\"Slivers\" you control and nontoken creatures you control",
    ] {
        assert!(complete_characteristic_subject(&lex(text)).unwrap().is_none(), "{text}");
    }
    for text in [
        "Creature cards you own that aren't on the battlefield are the chosen type in addition to their other types and have flying.",
        "Slivers you control and nontoken creatures you control are the chosen type in addition to their other creature types until end of turn.",
    ] {
        assert!(parse_subject_are_card_types_in_addition_to_their_other_types_line(&lex(text))
            .unwrap().is_none(), "{text}");
    }
}

#[test]
fn nonbattlefield_domain_does_not_change_chosen_type_family() {
    for (text, creature_family) in [
        ("Land cards you own that aren't on the battlefield are the chosen type in addition to their other types.", false),
        ("Land cards you own that aren't on the battlefield are the chosen creature type in addition to their other types.", true),
    ] {
        let abilities = parse_subject_are_card_types_in_addition_to_their_other_types_line(&lex(text))
            .unwrap().unwrap();
        assert_eq!(matches!(&abilities[0].payload, P::AddChosenCreatureType { .. }), creature_family);
        assert_eq!(matches!(&abilities[0].payload, P::AddChosenBasicLandType { .. }), !creature_family);
    }
}

#[test]
fn repeated_nominal_arms_do_not_share_leading_qualifiers_in_either_order() {
    for qualifier in ["nontoken", "tapped", "other"] {
        let restricted = format!("{qualifier} creatures you control");
        for text in [format!("{restricted} and Slivers you control"),
            format!("Slivers you control and {restricted}")] {
            let union = complete_characteristic_subject(&lex(&text)).unwrap().unwrap();
            assert_eq!(union.any_of.len(), 2);
            assert!(!union.nontoken && !union.tapped && !union.other);
            let sliver = union.any_of.iter().find(|branch| branch.subtypes == [Subtype::Sliver]).unwrap();
            assert!(!sliver.nontoken && !sliver.tapped && !sliver.other, "{text}: {union:?}");
            let creature = union.any_of.iter().find(|branch| branch.card_types == [CardType::Creature]).unwrap();
            assert_eq!(creature.nontoken, qualifier == "nontoken");
            assert_eq!(creature.tapped, qualifier == "tapped");
            assert_eq!(creature.other, qualifier == "other");
        }
    }
    for text in [
        "nontoken creatures you control and creature cards you own that aren't on the battlefield",
        "creature cards you own that aren't on the battlefield and nontoken creatures you control",
    ] {
        let union = complete_characteristic_subject(&lex(text)).unwrap().unwrap();
        assert_eq!(union.zone, None);
        assert_eq!(union.owner, None);
        assert_eq!(union.controller, None);
        assert!(!union.nontoken);
        let battlefield = union.any_of.iter().find(|branch| branch.zone == Some(Zone::Battlefield)).unwrap();
        assert_eq!(battlefield.controller, Some(PlayerFilter::You));
        assert_eq!(battlefield.owner, None);
        assert!(battlefield.nontoken);
        let cards = union.any_of.iter().find(|branch| branch.any_of.len() == 7).unwrap();
        for branch in &cards.any_of {
            assert_eq!(branch.owner, Some(PlayerFilter::You));
            assert_eq!(branch.controller, None);
            assert!(branch.has_explicit_card_noun());
            assert!(!branch.nontoken);
        }
    }
}

#[test]
fn both_nominal_arms_must_be_complete_and_unquoted() {
    for bad in ["mystery creatures you control", "creatures you control mystery",
        "creatures you control until end of turn", "creatures you control and",
        "\"creatures\" you control", "creatures you control \"mystery\""] {
        for text in [format!("{bad} and Slivers you control"),
            format!("Slivers you control and {bad}")] {
            assert!(complete_characteristic_subject(&lex(&text)).unwrap().is_none(), "{text}");
        }
    }
}
