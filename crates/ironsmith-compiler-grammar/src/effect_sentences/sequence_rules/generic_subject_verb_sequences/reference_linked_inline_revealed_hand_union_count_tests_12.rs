use super::*;
use crate::lexer::lex_line;

#[test]
fn revealed_target_hand_scopes_shared_terminal_union_count() {
    let first = lex_line("Target opponent reveals their hand.", 0).unwrap();
    let second = lex_line("You draw a card for each Forest and green card in it.", 1).unwrap();
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("pair parser")
        .expect("revealed-hand union pair");

    let [
        _,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected reveal plus draw, got {effects:#?}");
    };
    let Value::Count(filter) = count.unhinted() else {
        panic!("expected an object count, got {count:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent
        )))
    );
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(filter.any_of[0].subtypes, [crate::Subtype::Forest]);
    assert_eq!(filter.any_of[1].colors, Some(crate::ColorSet::GREEN));
}
