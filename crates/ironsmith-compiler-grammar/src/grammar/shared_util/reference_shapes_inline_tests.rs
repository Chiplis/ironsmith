use super::*;

#[test]
fn parses_subject_and_reference_surfaces() {
    assert_eq!(
        parse_subject_words(&["the", "active", "player"]),
        SubjectAst::Player(PlayerAst::Active)
    );
    assert_eq!(
        parse_subject_words(&["the", "player", "with", "the", "most", "life"]),
        SubjectAst::Player(PlayerAst::MostLifeTied)
    );
    assert_eq!(
        parse_subject_words(&["enchanted", "opponent", "creates"]),
        SubjectAst::Player(PlayerAst::Enchanted)
    );
    assert_eq!(
        parse_subject_words(&[
            "that",
            "player",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "discards",
            "two",
            "cards",
        ]),
        SubjectAst::Player(PlayerAst::ThatPlayerOrTargetController)
    );
    assert_eq!(
        parse_subject_words(&["that", "source's", "controller", "gains", "control",]),
        SubjectAst::TriggeringSourceController
    );
    assert_eq!(
        parse_subject_words(&[
            "that",
            "spell",
            "or",
            "ability's",
            "controller",
            "sacrifices",
            "a",
            "land",
        ]),
        SubjectAst::TriggeringSourceController
    );
    let lexed = crate::lexer::lex_line("that spell or ability's controller", 0)
        .expect("triggering spell-or-ability controller subject should lex");
    let lexed_words = crate::lexer::token_word_refs(&lexed);
    assert_eq!(
        parse_subject_tokens(&lexed),
        SubjectAst::TriggeringSourceController,
        "lexed words: {lexed_words:?}"
    );
    assert!(contains_source_from_your_hand(&[
        "discard", "this", "card", "from", "your", "hand"
    ]));
    assert!(is_source_from_exile(&["this", "creature", "from", "exile"]));
}

#[test]
fn parses_filter_keyword_and_player_advantage_surfaces() {
    assert_eq!(
        parse_filter_keyword_constraint_words(&["basic", "landcycling"]),
        Some((FilterKeywordConstraint::Marker("cycling"), 2))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["cascade"]),
        Some((FilterKeywordConstraint::Static(StaticAbilityId::Cascade), 1))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["toxic"]),
        Some((FilterKeywordConstraint::Marker("toxic"), 1))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["fading"]),
        Some((FilterKeywordConstraint::Marker("fading"), 1))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["unearth"]),
        Some((FilterKeywordConstraint::Marker("unearth"), 1))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["freerunning"]),
        Some((FilterKeywordConstraint::Marker("freerunning"), 1))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["doctor's", "companion"]),
        Some((FilterKeywordConstraint::Marker("doctor's companion"), 2))
    );
    let (constraints, connective, consumed) = parse_filter_keyword_constraint_list_words(&[
        "first",
        "strike",
        "double",
        "strike",
        "vigilance",
        "and",
        "or",
        "haste",
    ])
    .expect("split and/or should remain a keyword-list connective");
    assert_eq!(constraints.len(), 4);
    assert_eq!(connective, FilterKeywordListConnective::AndOr);
    assert_eq!(consumed, 8);
    assert_eq!(
        parse_life_advantage_player(&["opponent", "who", "has", "more", "life", "than", "you"]),
        Some(PlayerFilter::HasMoreLifeThanYou {
            base: Box::new(PlayerFilter::Opponent),
        })
    );
    assert_eq!(
        parse_life_advantage_player(&[
            "player", "with", "most", "life", "or", "tied", "for", "most", "life",
        ]),
        Some(PlayerFilter::MostLifeTied)
    );
}
