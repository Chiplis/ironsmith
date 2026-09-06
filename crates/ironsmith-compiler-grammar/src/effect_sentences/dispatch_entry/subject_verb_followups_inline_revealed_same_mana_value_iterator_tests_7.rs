use super::*;

#[test]
fn revealed_cards_compare_against_a_different_card_in_the_revealed_set() {
    let tokens = crate::lexer::lex_line(
            "Reveal up to five nonland cards from your hand. For each of those cards that has the same mana value as another card revealed this way, create a Treasure token.",
            0,
        )
        .expect("correlated revealed-card sentence should lex");
    let effects = parse_effect_sentences_lexed(&tokens)
        .expect("correlated revealed-card sentence should parse");

    let reveal_tag = effects.iter().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { tag }),
            ..
        }) => Some(tag),
        _ => None,
    });
    let Some(reveal_tag) = reveal_tag else {
        panic!("expected an explicitly tagged reveal: {effects:#?}");
    };
    let Some(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag,
        effects: iterator_effects,
    })) = effects.last()
    else {
        panic!("expected a tagged revealed-card iterator: {effects:#?}");
    };
    assert_eq!(tag, reveal_tag);
    let [
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: PredicateAst::ItMatches(filter),
            effects: create_effects,
        }),
    ] = iterator_effects.as_slice()
    else {
        panic!("expected one typed per-card condition: {iterator_effects:#?}");
    };
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **reveal_tag
            && constraint.relation
                == crate::filter::TaggedOpbjectRelation::SameManaValueAsAnotherTagged
    }));
    assert!(!create_effects.is_empty());
}

#[test]
fn ordinary_for_each_revealed_card_does_not_gain_a_mana_value_condition() {
    let tokens = crate::lexer::lex_line(
            "Reveal up to five nonland cards from your hand. For each card revealed this way, create a Treasure token.",
            0,
        )
        .expect("ordinary revealed-card sentence should lex");
    let effects = parse_effect_sentences_lexed(&tokens)
        .expect("ordinary revealed-card sentence should parse");
    assert!(
        !format!("{effects:#?}").contains("SameManaValueAsAnotherTagged"),
        "the qualifier must not be inferred: {effects:#?}"
    );
}
