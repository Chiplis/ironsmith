use super::*;

#[test]
pub(super) fn parses_not_chosen_this_way_as_the_complement_set() {
    let tokens = lex_line("each creature not chosen this way", 0).unwrap();
    let DestroyClauseKind::All(DestroyAllShape::ChosenThisWay {
        filter_tokens,
        relation,
    }) = parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected tagged destroy-all complement");
    };

    assert_eq!(words(filter_tokens), vec!["creature"]);
    assert_eq!(relation, TaggedDestroyRelation::ExceptMatching);
}

#[test]
pub(super) fn parses_not_chosen_by_any_player_as_the_complement_set() {
    let tokens = lex_line("all Plains that weren't chosen this way by any player", 0).unwrap();
    let DestroyClauseKind::All(DestroyAllShape::ChosenThisWay {
        filter_tokens,
        relation,
    }) = parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected tagged destroy-all complement");
    };

    assert_eq!(words(filter_tokens), vec!["plains"]);
    assert_eq!(relation, TaggedDestroyRelation::ExceptMatching);
}

#[test]
pub(super) fn chosen_this_way_type_qualifier_remains_an_object_filter() {
    let tokens = lex_line("all creatures that aren't of a type chosen this way", 0).unwrap();
    let DestroyClauseKind::All(DestroyAllShape::Plain { filter_tokens }) =
        parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("chosen creature-type qualifier must not become an object-result tag");
    };

    assert_eq!(
        words(filter_tokens),
        vec![
            "creatures",
            "that",
            "arent",
            "of",
            "a",
            "type",
            "chosen",
            "this",
            "way"
        ]
    );
}
