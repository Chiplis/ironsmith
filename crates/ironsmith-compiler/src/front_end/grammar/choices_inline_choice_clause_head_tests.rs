use super::*;
use crate::lexer::lex_line;

#[test]
fn opponent_choice_head_retains_the_choice_verb_for_type_parsers() {
    let tokens = lex_line("An opponent chooses a creature type.", 0).unwrap();
    let head = parse_choice_clause_head_tokens(&tokens).expect("choice head");

    assert_eq!(head.actor, ChoiceClauseActor::Opponent);
    assert_eq!(
        TokenWordView::new(head.choice_tokens).word_refs(),
        ["chooses", "a", "creature", "type"]
    );
}
