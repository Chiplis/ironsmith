use super::*;
use crate::lexer::lex_line;

fn parse_three(third: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line("Each player mills a card.", 0).expect("first sentence");
    let second = lex_line(
        "If a land card was milled this way, create a Treasure token.",
        1,
    )
    .expect("second sentence");
    let third = lex_line(third, 2).expect("third sentence");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
        SentenceInput::from_lexed(&third),
    ];
    parse_each_player_mill_then_land_result_then_cast_one_milled_spell(&sentences, 0)
        .expect("triple parser")
}

#[test]
fn exact_triple_shares_mill_tag_and_defers_one_cast_choice() {
    let effects = parse_three("Until end of turn, you may cast a spell from among those cards.")
        .expect("exact result-linked permission");
    let [
        EffectAst::TagAffected { tag: mill_tag, .. },
        conditional,
        permission,
    ] = effects.as_slice()
    else {
        panic!("expected tagged mill, condition, and permission: {effects:#?}");
    };
    assert!(matches!(
        conditional,
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(tag, filter),
            ..
        }) if tag == mill_tag && filter.card_types == [CardType::Land]
            && filter.prior_effect_action_surface()
                == Some(ironsmith_core::PriorEffectAction::Milled)
    ));
    assert!(matches!(
        permission,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                max_plays: Some(1),
                allow_land: false,
                ..
            }),
            ..
        }) if tag == mill_tag
    ));
}

#[test]
fn plural_permission_is_not_claimed_as_the_one_spell_rule() {
    assert!(
        parse_three("Until end of turn, you may cast spells from among those cards.").is_none()
    );
}
