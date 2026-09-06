use super::*;

#[test]
fn chooses_a_color_from_the_filtered_objects_instead_of_an_object() {
    let tokens = crate::lexer::lex_line(
        "Choose a color of a permanent you control. Add one mana of that color.",
        0,
    )
    .expect("dynamic color-choice sentence should lex");
    let effects = parse_activated_effects_lexed("", &tokens, 0)
        .expect("dynamic color-choice sentence should parse");
    let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        panic!("expected one typed mana effect, got {effects:#?}");
    };
    let SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong {
        filter,
        choose_color_of_object_surface,
    }) = &subject_verb.action
    else {
        panic!("expected a restricted color-choice effect, got {effects:#?}");
    };
    assert!(*choose_color_of_object_surface);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert!(filter.card_types.is_empty(), "{filter:#?}");
}

#[test]
fn chooses_a_color_of_a_typed_permanent_without_erasing_that_type() {
    let tokens = crate::lexer::lex_line(
        "Choose a color of an artifact you control. Add one mana of that color.",
        0,
    )
    .expect("typed color-choice sentence should lex");
    let effects = parse_activated_effects_lexed("", &tokens, 0)
        .expect("typed color-choice sentence should parse");
    let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        panic!("expected one typed mana effect, got {effects:#?}");
    };
    let SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong { filter, .. }) = &subject_verb.action else {
        panic!("expected a restricted color-choice effect, got {effects:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Artifact]);
}

#[test]
fn unrelated_choose_object_then_chosen_color_is_not_reinterpreted() {
    let tokens = crate::lexer::lex_line(
        "Choose a permanent you control. Add one mana of the chosen color.",
        0,
    )
    .expect("near-miss sentence should lex");
    assert!(!is_choose_color_of_matching_object_mana_shape(&tokens));
}
