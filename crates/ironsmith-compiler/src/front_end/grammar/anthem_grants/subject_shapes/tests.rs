use super::*;
use crate::filter::StackObjectKind;
use crate::runtime_backend::lexer::lex_line;
use crate::{CardType, Color, Subtype, Supertype, Zone};

#[test]
fn distributive_literal_pt_subject_keeps_both_exact_characteristics() {
    let tokens = lex_line("Each 1/1 creature you control", 0)
        .expect("literal P/T anthem subject should lex");
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("literal P/T anthem subject should parse");
    };

    assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(filter.power, Some(crate::filter::Comparison::Equal(1)));
    assert_eq!(filter.toughness, Some(crate::filter::Comparison::Equal(1)));
}

#[test]
fn distributive_compound_subtype_subject_preserves_every_subtype() {
    let tokens = lex_line("Each Eldrazi Spawn creature you control", 0)
        .expect("compound subtype anthem subject should lex");
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("compound subtype anthem subject should parse");
    };

    assert!(filter.subtypes.is_empty(), "{filter:#?}");
    assert_eq!(filter.all_subtypes, vec![Subtype::Eldrazi, Subtype::Spawn]);
}

#[test]
fn parses_colored_instant_and_sorcery_spell_subject_with_shared_controller() {
    let tokens = lex_line("Red instant and sorcery spells you control", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed colored instant-and-sorcery subject");
    };

    assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
    assert_eq!(
        filter.stack_kind,
        Some(StackObjectKind::Spell),
        "{filter:#?}"
    );
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert!(
        filter
            .colors
            .is_some_and(|colors| colors.contains(Color::Red)),
        "{filter:#?}"
    );
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(
        filter
            .any_of
            .iter()
            .flat_map(|branch| branch.card_types.iter().copied())
            .collect::<Vec<_>>(),
        vec![CardType::Instant, CardType::Sorcery]
    );
    assert_eq!(
        filter.description(),
        "a red instant and sorcery spell you control"
    );
}

#[test]
fn parses_instant_and_sorcery_spell_subject_with_shared_caster_and_origin() {
    for (text, expected_zone) in [
        ("Instant and sorcery spells you cast", Zone::Stack),
        (
            "Instant and sorcery spells you cast from your hand",
            Zone::Hand,
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
            parse_exact_anthem_subject_grammar(&tokens)
        else {
            panic!("expected a typed instant-and-sorcery spell subject for {text}");
        };

        assert_eq!(filter.zone, Some(expected_zone), "{filter:#?}");
        assert_eq!(filter.cast_by, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.controller, None, "{filter:#?}");
        assert!(filter.has_mana_cost, "{filter:#?}");
        assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
        assert_eq!(
            filter
                .any_of
                .iter()
                .flat_map(|branch| branch.card_types.iter().copied())
                .collect::<Vec<_>>(),
            vec![CardType::Instant, CardType::Sorcery]
        );
        assert!(
            filter
                .any_of
                .iter()
                .all(|branch| branch.cast_by.is_none() && branch.zone.is_none()),
            "{filter:#?}"
        );
    }
}

#[test]
fn parses_commander_controller_subject_to_typed_filter() {
    let tokens = lex_line("Commanders you control", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected typed commander subject");
    };

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.is_commander);
}

#[test]
fn parses_attacking_token_controller_subject_to_typed_filter() {
    let tokens = lex_line("Attacking tokens you control", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected typed attacking-token subject");
    };

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.token);
    assert!(filter.attacking);
}

#[test]
fn parses_relative_attachment_state_as_an_intrinsic_filter_constraint() {
    for (text, expected_tag) in [
        ("Creatures you control that are enchanted", "enchanted"),
        ("Artifact you control that is equipped", "equipped"),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
            parse_exact_anthem_subject_grammar(&tokens)
        else {
            panic!("expected an attachment-qualified filter for {text}");
        };

        assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
        assert_eq!(
            filter.tagged_constraints[0].tag.as_str(),
            expected_tag,
            "{filter:#?}"
        );
        assert_eq!(
            filter.tagged_constraints[0].relation,
            TaggedOpbjectRelation::IsTaggedObject,
            "{filter:#?}"
        );
    }
}

#[test]
fn parses_relative_enchanted_by_filter_as_a_matching_attachment() {
    let tokens = lex_line(
        "Other creatures you control that are enchanted by Auras you control",
        0,
    )
    .unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected an attachment-qualified filter");
    };

    assert!(filter.other, "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    let attachment = filter
        .with_attached_object
        .as_deref()
        .expect("enchanted-by clause should produce an intrinsic attachment filter");
    assert!(
        attachment.subtypes.contains(&Subtype::Aura),
        "{attachment:#?}"
    );
    assert_eq!(
        attachment.controller,
        Some(PlayerFilter::You),
        "{attachment:#?}"
    );
}

#[test]
fn parses_disjunctive_relative_attachment_states_as_typed_union() {
    let tokens = lex_line("Creatures you control that are enchanted or equipped", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed attachment-state union");
    };

    assert!(
        filter.has_relative_attachment_state_surface(),
        "{filter:#?}"
    );
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(
        filter
            .any_of
            .iter()
            .flat_map(|branch| branch.tagged_constraints.iter())
            .map(|constraint| constraint.tag.as_str())
            .collect::<Vec<_>>(),
        vec!["enchanted", "equipped"]
    );
}

#[test]
fn classifies_speculative_fragments_without_suffix_recovery() {
    for fragment in [
        "all abilities and",
        "you draw two cards lose 2 life and",
        "as long as enchanted permanent is an equipment it",
        "three or more poison counters enchanted creature",
    ] {
        let tokens = lex_line(fragment, 0).unwrap();
        assert_eq!(
            parse_exact_anthem_subject_grammar(&tokens),
            Some(AnthemSubjectGrammarMatch::RejectFragment),
            "{fragment}"
        );
    }
}

#[test]
fn leaves_unrelated_subjects_for_existing_typed_grammar() {
    let tokens = lex_line("Other creatures you control", 0).unwrap();
    assert_eq!(parse_exact_anthem_subject_grammar(&tokens), None);
}

#[test]
fn parses_distributive_chosen_land_type_subject_without_suffix_recovery() {
    let tokens = lex_line("Each land of the chosen type", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed distributive filter");
    };
    assert!(filter.chosen_land_type, "{filter:#?}");
    assert_eq!(filter.zone, Some(Zone::Battlefield));
}

#[test]
fn parses_relative_subtype_list_without_widening_to_all_creatures() {
    let tokens = lex_line(
        "Each other creature you control that's a Skeleton or Pirate",
        0,
    )
    .unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed relative subtype-list filter");
    };

    assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
    assert_eq!(
        filter.subtypes,
        [Subtype::Skeleton, Subtype::Pirate],
        "{filter:#?}"
    );
    assert!(!filter.type_or_subtype_union, "{filter:#?}");
}

#[test]
fn parses_differently_qualified_type_branches_as_typed_disjunction() {
    let tokens = lex_line(
        "Each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater",
        0,
    )
    .unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed shared-suffix disjunction");
    };

    assert_eq!(filter.zone, Some(Zone::Battlefield), "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert!(filter.mana_value.is_some(), "{filter:#?}");
    assert!(filter.card_types.is_empty(), "{filter:#?}");
    assert!(filter.subtypes.is_empty(), "{filter:#?}");
    assert!(filter.excluded_card_types.is_empty(), "{filter:#?}");
    assert!(filter.excluded_subtypes.is_empty(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    let artifact = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types.contains(&CardType::Artifact))
        .expect("artifact branch");
    assert!(artifact.excluded_subtypes.contains(&Subtype::Equipment));
    assert!(!artifact.subtypes.contains(&Subtype::Equipment));
    assert!(!artifact.excluded_subtypes.contains(&Subtype::Aura));
    assert!(artifact.zone.is_none(), "{artifact:#?}");
    assert!(artifact.controller.is_none(), "{artifact:#?}");
    assert!(artifact.mana_value.is_none(), "{artifact:#?}");

    let enchantment = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types.contains(&CardType::Enchantment))
        .expect("enchantment branch");
    assert!(enchantment.excluded_subtypes.contains(&Subtype::Aura));
    assert!(!enchantment.subtypes.contains(&Subtype::Aura));
    assert!(!enchantment.excluded_subtypes.contains(&Subtype::Equipment));
    assert!(enchantment.zone.is_none(), "{enchantment:#?}");
    assert!(enchantment.controller.is_none(), "{enchantment:#?}");
    assert!(enchantment.mana_value.is_none(), "{enchantment:#?}");
    assert_eq!(
        filter.description(),
        "a non-equipment artifact and non-aura enchantment you control with mana value 4 or greater"
    );
}

#[test]
fn factors_shared_controller_and_leading_other_from_conjunctive_subject() {
    let tokens = lex_line("Other Plants and Treefolk you control", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed conjunctive anthem subject");
    };

    assert_eq!(filter.zone, Some(Zone::Battlefield), "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert!(filter.other, "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter
            .any_of
            .iter()
            .all(|branch| branch.zone.is_none() && branch.controller.is_none() && !branch.other),
        "{filter:#?}"
    );
    assert_eq!(
        filter.description(),
        "another Plant and Treefolk you control"
    );
}

#[test]
fn factors_leading_other_across_mixed_type_and_subtype_subjects() {
    let tokens = lex_line(
        "Other nontoken artifact creatures and Vehicles you control",
        0,
    )
    .unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed mixed type-and-subtype anthem subject");
    };

    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert!(filter.other, "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter.any_of.iter().all(|branch| !branch.other),
        "{filter:#?}"
    );
}

#[test]
fn distributes_shared_creature_head_across_supertype_and_subtype_branches() {
    let tokens = lex_line("Other snow and Zombie creatures you control", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected a typed shared-head anthem subject");
    };

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
            .any(|branch| branch.supertypes == [Supertype::Snow]),
        "{filter:#?}"
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [Subtype::Zombie]),
        "{filter:#?}"
    );
}
