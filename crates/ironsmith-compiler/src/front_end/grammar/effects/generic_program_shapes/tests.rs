use super::*;
use crate::runtime_backend::lexer::lex_line;

#[test]
fn parses_damage_and_choice_program_shapes() {
    let damage = lex_line(
        "Any opponent may have this creature deal 3 damage to them",
        0,
    )
    .expect("lex fixture");
    let parsed = parse_any_player_source_damage(&damage).expect("damage shape");
    assert_eq!(parsed.player, PlayerAst::Opponent);
    assert!(!parsed.damage_tokens.is_empty());

    let choice = lex_line("Each player chooses a creature then sacrifices the rest", 0)
        .expect("lex fixture");
    assert_eq!(
        parse_choice_complement_clause(&choice)
            .expect("choice shape")
            .word_refs(),
        ["a", "creature"]
    );

    let target_choice = lex_line("up to two target creatures", 0).unwrap();
    assert!(vote_options_tokens_look_like_target_choice(&target_choice));
    let named_choice = lex_line("peace", 0).unwrap();
    assert!(!vote_options_tokens_look_like_target_choice(&named_choice));

    let named_option = lex_line("Then for each peace vote, draw a card.", 0).unwrap();
    let named_option = parse_named_vote_option_effects_shape(&named_option).unwrap();
    assert_eq!(
        LexedClause::new(named_option.option_tokens).word_refs(),
        ["peace"]
    );
    assert_eq!(
        LexedClause::new(named_option.effect_tokens).word_refs(),
        ["draw", "a", "card"]
    );
}

#[test]
fn parses_and_coordinated_counted_choice_complement_clause() {
    let tokens = lex_line(
        "Each player chooses five lands they control and sacrifices the rest.",
        0,
    )
    .expect("counted choice complement");
    let choice = parse_choice_complement_clause(&tokens).expect("choice complement shape");

    assert_eq!(choice.word_refs(), ["five", "lands", "they", "control"]);
}
