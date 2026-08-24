use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_chain_carry_leaf_shapes() {
    let tokens = lex_line("Choose a land of each basic land type.", 0).unwrap();
    assert!(parse_choose_each_basic_land_type_tokens(&tokens));
    let tokens = lex_line("Two 1/1 white Soldier creature tokens", 0).unwrap();
    assert!(parse_create_fragment_tokens(&tokens));
    let tokens = lex_line("Tap those, then unattach all Equipment from them.", 0).unwrap();
    assert!(parse_tap_then_unattach_tokens(&tokens));

    let tokens = lex_line("Then sacrifice the rest.", 0).unwrap();
    assert_eq!(
        parse_rest_action_tokens(&tokens),
        Some(RestActionShape::Sacrifice)
    );

    let tokens = lex_line("Until your next untap step, it gains flying.", 0).unwrap();
    let duration = parse_carry_duration_prefix_tokens(&tokens).unwrap();
    assert_eq!(duration.duration, Until::ControllersNextUntapStep);
    assert!(
        duration
            .rest
            .first()
            .is_some_and(|token| token.is_word("it"))
    );

    let tokens = lex_line(
            "Until your next turn, whenever either of those creatures deals combat damage, you draw a card.",
            0,
        )
        .unwrap();
    assert!(
        parse_carry_duration_prefix_tokens(&tokens).is_none(),
        "a delayed-trigger lifetime must remain attached to its trigger clause"
    );

    let tokens = lex_line("And draw a card.", 0).unwrap();
    assert_eq!(
        parse_carry_clause_head_tokens(&tokens),
        CarryClauseHead::Draw
    );
}

#[test]
fn leading_duration_scaled_stat_and_pronoun_grant_is_a_coordinated_chain() {
    let tokens = lex_line(
        "Until end of turn, double target creature's power and it gains first strike.",
        0,
    )
    .unwrap();

    assert_eq!(
        coordinated_effect_chain_leading_duration(&tokens),
        Some(true)
    );
}

#[test]
fn leading_duration_gain_then_get_is_one_shared_subject_clause() {
    let tokens = lex_line(
            "Until end of turn, creatures you control gain trample and get +1/+1 for each basic land type among lands you control.",
            0,
        )
        .unwrap();

    assert_eq!(coordinated_effect_chain_leading_duration(&tokens), None);
}

#[test]
fn parses_owner_and_delay_facts() {
    let tokens = lex_line(
            "Exile all cards from your library face down, then shuffle all cards from your graveyard into your library.",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_exile_library_shuffle_tokens(&tokens).map(|spec| spec.owner),
        Some(ChainOwner::You)
    );
    let tokens = lex_line(
        "At the beginning of your next end step, exile the token.",
        0,
    )
    .unwrap();
    let facts = parse_delayed_copy_facts_tokens(&tokens);
    assert!(facts.has_exile && facts.has_token);
    assert_eq!(
        facts.timing,
        Some(DelayedCopyTiming::EndStep {
            player_is_you: true
        })
    );

    let tokens = lex_line(
        "At the beginning of your next upkeep, sacrifice the token.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_delayed_copy_facts_tokens(&tokens).timing,
        Some(DelayedCopyTiming::Upkeep {
            player_is_you: true
        })
    );
}

#[test]
fn action_splits_preserve_card_type_lists() {
    let tokens = lex_line(
        "Discard two cards or sacrifice a creature or planeswalker of your choice.",
        0,
    )
    .unwrap();
    let splits = parse_or_action_splits_tokens(&tokens);
    assert_eq!(splits.len(), 1);

    let tokens = lex_line("Destroy target artifact, creature, or enchantment.", 0).unwrap();
    assert!(parse_or_action_splits_tokens(&tokens).is_empty());
}

#[test]
fn destroy_split_requires_a_temporary_restriction_tail() {
    let tokens = lex_line(
        "Destroy target creature, and that creature can't attack or block this turn.",
        0,
    )
    .unwrap();
    assert_eq!(parse_destroy_restriction_splits_tokens(&tokens).len(), 1);

    let tokens = lex_line("Destroy target creature and draw a card.", 0).unwrap();
    assert!(parse_destroy_restriction_splits_tokens(&tokens).is_empty());
}
