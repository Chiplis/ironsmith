use super::*;

#[test]
fn implicit_choice_actor_is_preserved_for_enclosing_sentence_binding() {
    let tokens =
        crate::lexer::lex_line("Choose a nonland card exiled this way.", 0).expect("lex choice");
    let (chooser, filter, count, _) = parse_you_choose_objects_clause_with_count_value(&tokens)
        .expect("parse choice")
        .expect("match choice");
    assert_eq!(chooser, PlayerAst::Implicit);
    assert!(count.is_single());
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn up_to_prior_amount_choice_preserves_count_and_value_source() {
    let tokens = crate::lexer::lex_line("Choose up to that many target creatures you control.", 0)
        .expect("lex choice");
    let (_, _, count, count_value) = parse_you_choose_objects_clause_with_count_value(&tokens)
        .expect("parse choice")
        .expect("match choice");

    assert!(count.is_up_to_dynamic_x());
    assert_eq!(
        count_value,
        Some(Value::EventValue(crate::effect::EventValueSpec::Amount))
    );
}

#[test]
fn for_each_choice_count_lowers_to_a_typed_dynamic_value() {
    let tokens = crate::lexer::lex_line("Choose a permanent for each card in their graveyard.", 0)
        .expect("lex choice");
    let (_, _, count, count_value) = parse_you_choose_objects_clause_with_count_value(&tokens)
        .expect("parse choice")
        .expect("match choice");

    assert!(count.is_dynamic_x());
    let value = count_value.expect("for-each count value");
    assert!(value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
    let Value::Count(filter) = value.unhinted() else {
        panic!("expected an object-count value: {value:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
}

#[test]
fn that_player_for_each_choice_keeps_its_dynamic_count_basis() {
    let tokens = crate::lexer::lex_line(
        "That player chooses a permanent for each card in their graveyard.",
        0,
    )
    .expect("lex participant choice");
    let (chooser, filter, count, count_value) =
        parse_target_player_choose_objects_clause_with_count_value(&tokens)
            .expect("parse participant choice")
            .expect("match participant choice");

    assert_eq!(chooser, PlayerAst::That);
    assert!(count.is_dynamic_x());
    assert_eq!(filter.controller, None);
    let value = count_value.expect("for-each count value");
    assert!(value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
    let Value::Count(filter) = value.unhinted() else {
        panic!("expected an object-count value: {value:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
}

#[test]
fn participant_choice_only_restricts_control_when_oracle_says_so() {
    let unrestricted = crate::lexer::lex_line("That player chooses up to two Plains.", 0)
        .expect("lex unrestricted choice");
    let (_, unrestricted_filter, _, _) =
        parse_target_player_choose_objects_clause_with_count_value(&unrestricted)
            .expect("parse unrestricted choice")
            .expect("match unrestricted choice");
    assert_eq!(unrestricted_filter.controller, None);

    let controlled =
        crate::lexer::lex_line("That player chooses up to two Plains they control.", 0)
            .expect("lex controlled choice");
    let (_, controlled_filter, _, _) =
        parse_target_player_choose_objects_clause_with_count_value(&controlled)
            .expect("parse controlled choice")
            .expect("match controlled choice");
    assert_eq!(
        controlled_filter.controller,
        Some(PlayerFilter::IteratedPlayer)
    );

    let negative_tag_only = crate::lexer::lex_line(
        "That player chooses a permanent that hasn't been chosen this way.",
        0,
    )
    .expect("lex complement-only choice");
    let (_, negative_filter, _, _) =
        parse_target_player_choose_objects_clause_with_count_value(&negative_tag_only)
            .expect("parse complement-only choice")
            .expect("match complement-only choice");
    assert_eq!(negative_filter.controller, None);
    assert!(
        negative_filter
            .tagged_constraints
            .iter()
            .any(|constraint| { constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject })
    );
}
