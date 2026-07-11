use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

fn tokens(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

#[test]
fn parses_player_and_delayed_sentence_shapes() {
    assert_eq!(
        parse_delayed_player_prefix_words(&["target", "opponent", "pays"], false),
        Some((PlayerAst::TargetOpponent, 2))
    );
    assert_eq!(
        parse_lose_draw_clash_shape(&tokens(
            "You lose 2 life and draw 2 cards, then clash with an opponent. If you win, repeat this process."
        )),
        Some(LoseDrawClashShape {
            life_count: 2,
            draw_count: 2,
            repeat_if_win: true,
        })
    );
}

#[test]
fn parses_registry_payment_and_become_shapes() {
    let upkeep = tokens("At the beginning of your next upkeep, pay {2}{U}.");
    let shape = parse_delayed_upkeep_payment_shape(&upkeep).unwrap();
    assert!(!shape.mana_tokens.is_empty());

    let unless = tokens("Target opponent pays {3}.");
    let split = split_delayed_payment_action_shape(&unless).unwrap();
    assert_eq!(
        LexedClause::new(split.player_tokens).word_refs(),
        vec!["target", "opponent"]
    );

    let become_tokens = tokens("This creature becomes a Dragon.");
    let shape = parse_implicit_become_subject_shape(&become_tokens).unwrap();
    assert_eq!(shape.kind, ImplicitBecomeSubjectKind::Source);
    assert!(!shape.remainder_tokens.is_empty());
}

#[test]
fn parses_delayed_timing_marker_and_known_fallback() {
    let timing = tokens("Sacrifice it at the beginning of their next upkeep.");
    let shape = parse_delayed_timing_marker_shape(&timing).unwrap();
    assert_eq!(shape.step, DelayedTimingStepShape::Upkeep);
    assert_eq!(shape.player, PlayerAst::That);
    assert_eq!(shape.start_word, 2);

    assert!(is_known_fallback_marker_shape(&tokens(
        "Put that pile into your hand."
    )));
}

#[test]
fn delayed_timing_marker_normalizes_apostrophized_player() {
    let timing = tokens(
        "Exile that creature at the beginning of that player's next upkeep unless they pay {2}.",
    );
    let shape = parse_delayed_timing_marker_shape(&timing).unwrap();
    assert_eq!(shape.start_word, 3);
    assert_eq!(shape.end_word, 11);
    assert_eq!(shape.step, DelayedTimingStepShape::Upkeep);
    assert_eq!(shape.player, PlayerAst::That);
}
