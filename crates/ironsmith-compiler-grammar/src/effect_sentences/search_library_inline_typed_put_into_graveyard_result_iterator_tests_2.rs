use super::*;

#[test]
fn creature_card_put_into_graveyard_this_way_keeps_a_typed_lki_gate() {
    let tokens = crate::lexer::lex_line(
            "For each creature card put into a graveyard this way, you create a tapped 2/2 black Zombie creature token",
            0,
        )
        .expect("typed graveyard-result iterator should lex");
    let effects = parse_for_each_put_into_graveyard_this_way_sentence(&tokens)
        .expect("typed iterator should parse")
        .expect("typed iterator should claim the sentence");
    let [EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })] = effects.as_slice() else {
        panic!("expected a tagged result iterator: {effects:#?}");
    };
    let [
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(filter),
            if_true,
            if_false,
        }),
    ] = effects.as_slice()
    else {
        panic!("expected a typed last-known-information gate: {effects:#?}");
    };
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
    assert!(filter.has_explicit_card_noun());
    assert!(filter.has_put_into_graveyard_this_way_surface());
    assert!(!if_true.is_empty());
    assert!(if_false.is_empty());
}

#[test]
fn unqualified_card_iterator_keeps_only_an_equality_transparent_surface_gate() {
    let tokens = crate::lexer::lex_line(
        "For each card put into a graveyard this way, you gain 1 life",
        0,
    )
    .expect("ordinary result iterator should lex");
    let effects = parse_for_each_put_into_graveyard_this_way_sentence(&tokens)
        .expect("ordinary iterator should parse")
        .expect("ordinary iterator should claim the sentence");
    let [EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })] = effects.as_slice() else {
        panic!("expected a tagged result iterator: {effects:#?}");
    };
    let [
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(filter),
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected the authored action surface on a typed LKI gate: {effects:#?}");
    };
    assert_eq!(filter, &ObjectFilter::default());
    assert!(filter.has_explicit_card_noun());
    assert!(filter.has_put_into_graveyard_this_way_surface());
}
