use super::super::super::super::lexer::lex_line;
use super::*;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn target_player_choice_head_returns_typed_actor_count_and_filter() {
    let tokens = lex("Target opponent chooses up to two creatures from a graveyard or hand.");
    let parsed = parse_target_player_choice_tokens(&tokens).unwrap().unwrap();

    assert_eq!(parsed.actor, TargetPlayerChoiceActor::TargetOpponent);
    assert_eq!(parsed.count, ChoiceCount::up_to(2));
    assert_eq!(
        TokenWordView::new(parsed.filter_tokens).word_refs()[0],
        "creatures"
    );
    assert!(parsed.filter_facts.graveyard_and_hand);

    let opponent = lex("An opponent chooses up to two nonland permanents they control.");
    let opponent = parse_target_player_choice_tokens(&opponent)
        .unwrap()
        .expect("indefinite opponent choice should use the same typed grammar");
    assert_eq!(opponent.actor, TargetPlayerChoiceActor::Opponent);
    assert_eq!(opponent.count, ChoiceCount::up_to(2));

    let article_tokens = lex("Target opponent chooses a card.");
    let article = parse_target_player_choice_tokens(&article_tokens)
        .unwrap()
        .unwrap();
    assert_eq!(article.count, ChoiceCount::exactly(1));
    assert_eq!(
        TokenWordView::new(article.filter_tokens).word_refs(),
        vec!["card"]
    );
}

#[test]
fn object_filter_facts_cover_tagged_disjunction_and_bare_card() {
    let facts = parse_choice_object_filter_facts_words(&[
        "card",
        "from",
        "it",
        "or",
        "a",
        "card",
        "from",
        "a",
        "graveyard",
    ]);
    assert!(facts.tagged_graveyard_disjunction);
    assert!(facts.graveyard_arm_is_plain_card);
    assert!(!facts.bare_card);

    assert!(parse_choice_object_filter_facts_words(&["cards"]).bare_card);
}

#[test]
fn possessive_choice_keeps_zone_tail_and_choice_owner() {
    let tokens = lex("a creature card of their choice from their graveyard");
    let parsed = parse_possessive_object_choice_tokens(&tokens).unwrap();

    assert_eq!(parsed.actor, PossessiveObjectChoiceActor::SubjectPlayer);
    assert_eq!(
        TokenWordView::new(&parsed.object_tokens).word_refs(),
        vec!["a", "creature", "card", "from", "their", "graveyard"]
    );
}
