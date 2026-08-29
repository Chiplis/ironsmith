use super::*;
use crate::Subtype;
use crate::lexer::lex_line;

#[test]
fn comma_separated_negated_characteristics_are_conjunctive() {
    let tokens = lex_line("a noncreature, nonland card", 0).unwrap();
    let filter =
        parse_looked_card_reveal_filter_shape(&tokens).expect("negated characteristic filter");

    assert!(filter.any_of.is_empty(), "{filter:#?}");
    assert_eq!(
        filter.excluded_card_types,
        vec![CardType::Creature, CardType::Land]
    );
}

#[test]
fn repeated_complete_card_union_preserves_branch_scope_and_surface() {
    let filter = parse("a Doctor card, a card with doctor's companion, or a Vehicle card");

    assert!(filter.has_explicit_union_branch_articles(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert_eq!(filter.any_of[0].subtypes, [Subtype::Doctor]);
    assert!(filter.any_of[1].subtypes.is_empty(), "{filter:#?}");
    assert_eq!(
        filter.any_of[1].ability_markers,
        ["doctor's companion".to_string()]
    );
    assert_eq!(filter.any_of[2].subtypes, [Subtype::Vehicle]);
    assert_eq!(
        filter.description(),
        "a Doctor card, a card with doctor's companion, or a Vehicle card"
    );
}

fn parse(raw: &str) -> ObjectFilter {
    parse_looked_card_reveal_filter_shape(&lex_line(raw, 0).unwrap()).unwrap()
}

#[test]
fn parses_typed_special_looked_card_filters() {
    assert_eq!(parse("a permanent card").card_types.len(), 6);
    assert!(
        parse("a nonland permanent card")
            .excluded_card_types
            .contains(&CardType::Land)
    );
    let and_or = parse("a land and/or legendary permanent card");
    assert_eq!(and_or.any_of.len(), 2);
    assert_eq!(
        and_or.union_connective(),
        crate::filter::ObjectFilterUnionConnective::AndOr
    );
    assert_eq!(
        parse("artifact and/or land cards").union_connective(),
        crate::filter::ObjectFilterUnionConnective::AndOr
    );
    assert_eq!(
        parse("a card with the chosen name")
            .tagged_constraints
            .len(),
        1
    );

    let shared = parse("a permanent card that shares a card type with the sacrificed permanent");
    assert!(shared.tagged_constraints.iter().any(|constraint| {
        constraint.tag == crate::tag::CompilerReferenceTag::Sacrificed0.key()
            && constraint.relation == TaggedOpbjectRelation::SharesCardType
    }));
}
