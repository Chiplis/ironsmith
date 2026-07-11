use super::super::super::lexer::{TokenWordView, lex_line};
use super::*;

#[test]
fn negation_span_and_or_split_are_typed() {
    let tokens = lex_line("creatures can't attack or activate abilities", 0).unwrap();
    let negation = parse_activation_negation_span_tokens(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(&tokens[negation.first..negation.end]).word_refs(),
        ["cant"]
    );
    let split = parse_cant_restriction_or_split_tokens(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(&split.first).word_refs(),
        ["creatures", "cant", "attack"]
    );
    assert_eq!(
        TokenWordView::new(&split.second).word_refs(),
        ["creatures", "cant", "activate", "abilities"]
    );
}

#[test]
fn attack_or_block_is_one_restriction_tail() {
    let tokens = lex_line("this creature can't attack or block", 0).unwrap();
    assert!(parse_cant_restriction_or_split_tokens(&tokens).is_none());
}

#[test]
fn unspent_mana_retention_surface_is_typed() {
    assert_eq!(
        parse_unspent_mana_retention_tail_words(&[
            "lose", "unspent", "red", "mana", "as", "steps", "and", "phases", "end",
        ]),
        Some(UnspentManaRetentionTail {
            color: Some(Color::Red),
        })
    );
    assert_eq!(
        parse_unspent_mana_retention_static_words(&[
            "each", "player", "dont", "lose", "unspent", "mana", "as", "steps",
        ]),
        Some(UnspentManaRetentionStatic {
            subject: ManaRetentionSubject::AnyPlayer,
            color: None,
        })
    );
}

#[test]
fn cast_qualifier_possessive_and_condition_envelopes_are_typed() {
    let qualifier =
        parse_activation_cast_limit_qualifier_words(&["noncreature", "spells"]).unwrap();
    assert_eq!(qualifier.consumed, 1);
    assert!(
        qualifier
            .filter
            .excluded_card_types
            .contains(&crate::types::CardType::Creature)
    );

    let possessive = lex_line("artifacts'", 0).unwrap();
    assert_eq!(
        TokenWordView::new(&parse_activation_possessive_owner_tokens(&possessive)).word_refs(),
        ["artifact"]
    );

    let prefixed = lex_line("during your turn, creatures can't block", 0).unwrap();
    assert!(matches!(
        parse_static_restriction_condition_shape_tokens(&prefixed),
        Some(StaticRestrictionConditionShape::Timing {
            timing: ActivationTiming::DuringYourTurn,
            ..
        })
    ));
    let conditional = lex_line("if you control a creature, players can't gain life", 0).unwrap();
    assert!(matches!(
        parse_static_restriction_condition_shape_tokens(&conditional),
        Some(StaticRestrictionConditionShape::Condition {
            kind: StaticRestrictionConditionKind::If,
            ..
        })
    ));
}
