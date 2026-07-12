use super::*;
use crate::runtime_backend::lexer::lex_line;
use crate::{CardType, Subtype, Zone};

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

    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    let artifact = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types.contains(&CardType::Artifact))
        .expect("artifact branch");
    assert!(artifact.excluded_subtypes.contains(&Subtype::Equipment));
    assert!(!artifact.excluded_subtypes.contains(&Subtype::Aura));
    assert_eq!(artifact.controller, Some(PlayerFilter::You));
    assert!(artifact.mana_value.is_some());

    let enchantment = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types.contains(&CardType::Enchantment))
        .expect("enchantment branch");
    assert!(enchantment.excluded_subtypes.contains(&Subtype::Aura));
    assert!(!enchantment.excluded_subtypes.contains(&Subtype::Equipment));
    assert_eq!(enchantment.controller, Some(PlayerFilter::You));
    assert!(enchantment.mana_value.is_some());
}
