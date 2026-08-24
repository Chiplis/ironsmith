use super::*;
use crate::lexer::lex_line;

#[test]
fn identifies_destroy_fanout_and_repeated_target_starts() {
    let tokens = lex_line(
        "Destroy up to one target artifact and up to one target enchantment.",
        0,
    )
    .unwrap();
    let shape = parse_destroy_multi_target_shape(&tokens).unwrap();
    assert!(shape.repeated_target_words);
    assert_eq!(
        up_to_one_target_word_starts(&TokenWordView::new(&tokens).to_word_refs()),
        [1, 7]
    );
}

#[test]
fn leaves_target_and_attached_object_sets_for_the_linked_destroy_parser() {
    let tokens = lex_line(
        "Destroy target creature with flying and all Equipment attached to that creature.",
        0,
    )
    .unwrap();

    assert!(parse_destroy_multi_target_shape(&tokens).is_none());
}
