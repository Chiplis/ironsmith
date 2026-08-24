use super::*;

#[test]
pub(super) fn parses_resource_chosen_name_target_shape() {
    let tokens = lex("target creature with a name chosen for this source this way");
    let shape = parse_resource_chosen_name_target_shape(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(shape.base_tokens).word_refs(),
        vec!["target", "creature"]
    );
    assert_eq!(
        shape.chosen_name_source,
        ironsmith_core::ChosenNameSourceSurface::Source
    );
}
