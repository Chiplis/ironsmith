use super::*;
use crate::lexer::lex_line;

fn parse(third: &str) -> Option<Vec<EffectAst>> {
    let lexed = [
        lex_line("Look at the top three cards of your library.", 0).unwrap(),
        lex_line(
            "Exile one face down and put the rest on the bottom of your library in any order.",
            1,
        )
        .unwrap(),
        lex_line(third, 2).unwrap(),
    ];
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();
    parse_look_at_top_partition_face_down_then_filtered_permission(&sentences, 0).unwrap()
}

#[test]
fn exact_three_sentence_shape_shares_selected_tag_and_creature_filter() {
    let effects =
        parse("For as long as it remains exiled, you may cast it if it's a creature spell.")
            .expect("three-sentence hidden-card permission");
    let selected_tag = match &effects[1] {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { tag, .. }) | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }) => {
            tag
        }
        _ => panic!("expected selected-card tag: {effects:#?}"),
    };
    assert!(matches!(
        effects.as_slice(),
        [.., EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                filter: Some(filter),
                ..
            }),
            ..
        })] if tag == selected_tag
            && filter.card_types == [CardType::Creature]
    ));
}

#[test]
fn ordinary_until_end_of_turn_permission_is_not_claimed() {
    assert!(parse("Until end of turn, you may cast it if it's a creature spell.").is_none());
}
