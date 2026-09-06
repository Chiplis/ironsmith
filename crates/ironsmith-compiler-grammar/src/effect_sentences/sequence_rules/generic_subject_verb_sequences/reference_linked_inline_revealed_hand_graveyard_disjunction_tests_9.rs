use super::*;
use crate::lexer::lex_line;

fn parse(second: &str) -> Option<Vec<EffectAst>> {
    let lexed = [
        lex_line("Target opponent reveals their hand.", 0).unwrap(),
        lex_line(second, 1).unwrap(),
    ];
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();
    crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects)).unwrap()
}

#[test]
fn exact_choice_keeps_branch_specific_nonland_and_opponent_constraints() {
    let effects = parse("You choose a nonland card from it or a card from their graveyard.")
        .expect("revealed-hand/graveyard choice");
    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, .. }) = &effects[1] else {
        panic!("expected cross-zone choice: {effects:#?}");
    };
    let [hand, graveyard] = filter.any_of.as_slice() else {
        panic!("expected exact disjunction: {filter:#?}");
    };
    assert_eq!(hand.zone, Some(Zone::Hand));
    assert_eq!(hand.excluded_card_types, [CardType::Land]);
    assert!(hand.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert_eq!(graveyard.zone, Some(Zone::Graveyard));
    assert_eq!(
        graveyard.owner,
        Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent
        )))
    );
    assert!(graveyard.excluded_card_types.is_empty());
}

#[test]
fn different_graveyard_owner_is_not_rebound() {
    assert!(parse("You choose a nonland card from it or a card from your graveyard.").is_none());
}
