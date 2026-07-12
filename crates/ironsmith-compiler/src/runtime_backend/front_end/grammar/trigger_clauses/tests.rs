use super::*;
use crate::runtime_backend::util::tokenize_line;

#[test]
fn maps_semantic_atoms_across_punctuation() {
    let tokens = tokenize_line("a creature, attacks", 0);
    assert_eq!(
        parse_trigger_clause_atom_token(&tokens, TriggerClauseAtom::Attack),
        Some(3)
    );
    assert_eq!(
        parse_trigger_word_span_tokens(&tokens, 2),
        Some(TriggerClauseTokenSpan { first: 3, end: 4 })
    );
}

#[test]
fn parses_activation_tap_cost_qualifiers() {
    let tokens = tokenize_line("an ability without {T} in its activation cost", 0);
    let parsed = parse_activation_cost_tap_condition(&tokens).unwrap();
    assert!(!parsed.required);
    assert_eq!(parsed.condition_word, 2);
    assert_eq!(parsed.condition_token, 2);
}

#[test]
fn keeps_list_or_but_splits_clause_or() {
    let list = tokenize_line("one or more creatures", 0);
    assert_eq!(parse_trigger_or_split(&list), None);

    let clauses = tokenize_line("this attacks or this blocks", 0);
    assert_eq!(
        parse_trigger_or_split(&clauses),
        Some(TriggerOrSplit { separator: 2 })
    );
}

#[test]
fn parses_counter_spans_and_recipient() {
    let tokens = tokenize_line("one or more charge counters are put on a creature", 0);
    let descriptor = parse_counter_descriptor_spans(&tokens, 0, 4).unwrap();
    assert_eq!(descriptor.descriptor, 0..4);
    assert_eq!(
        parse_trigger_counter_type(&tokens[descriptor.with_counter]),
        Some(CounterType::Charge)
    );
    assert_eq!(
        parse_counter_recipient_span(&tokens, 8).unwrap().tokens,
        9..10
    );
}

#[test]
fn parses_ability_owner_tails() {
    let possessive = tokenize_line("a creatures boast ability", 0);
    let parsed = parse_possessive_ability_tail(&possessive).unwrap();
    assert_eq!(parsed.owner, 0..2);
    assert_eq!(parsed.marker.as_deref(), Some("boast"));

    let of_object = tokenize_line("an ability of a creature that isnt a mana ability", 0);
    let parsed = parse_ability_of_object_tail(&of_object).unwrap();
    assert_eq!(parsed.filter, 3..5);
    assert!(parsed.non_mana_only);
}

#[test]
fn parses_players_attacked_subject_as_typed_span() {
    let tokens = tokenize_line("one or more opponents are attacked", 0);
    let parsed = parse_players_attacked_clause(&tokens).unwrap();
    assert_eq!(parsed.player, 0..4);
}

#[test]
fn parses_fully_unlock_room_as_typed_keyword_action() {
    for text in ["you fully unlock a Room", "you fully unlocked a Room"] {
        let tokens = tokenize_line(text, 0);
        let parsed = parse_fully_unlock_room_trigger(&tokens).expect(text);
        assert_eq!(parsed.action, KeywordActionKind::UnlockDoor);
        assert_eq!(parsed.player, PlayerFilter::You);
        assert_eq!(parsed.source_filter.subtypes, [Subtype::Room]);
    }
}

#[test]
fn parses_trigger_subject_and_origin_surfaces_as_typed_facts() {
    assert_eq!(
        parse_not_during_turn_draw_suffix_words(
            &["a", "card", "if", "it", "isnt", "your", "turn",]
        ),
        Some(PlayerFilter::You)
    );
    assert_eq!(
        parse_enters_origin_clause_words(&["from", "your", "graveyard"]),
        Some(EntersOriginClause {
            zone: Zone::Graveyard,
            owner: Some(PlayerFilter::You),
        })
    );

    let source = parse_source_trigger_subject_words(&["this", "artifact", "creature"]);
    assert_eq!(source.filter.card_types, [CardType::Creature]);

    let compound = parse_you_or_controlled_object_subject_words(&[
        "you", "or", "a", "creature", "you", "control",
    ])
    .unwrap();
    assert_eq!(compound.player, PlayerFilter::You);
    assert_eq!(compound.filter.card_types, [CardType::Creature]);
    assert_eq!(compound.filter.controller, Some(PlayerFilter::You));

    assert_eq!(
        parse_opponents_each_lose_exact_life_words(&[
            "one",
            "or",
            "more",
            "opponents",
            "each",
            "lose",
            "exactly",
            "three",
            "life",
        ]),
        Some(3)
    );
    assert_eq!(
        parse_roll_result_words(&["a", "dies", "highest", "natural", "result"]),
        Some(RollResultShape::HighestNatural)
    );
    assert_eq!(
        parse_roll_result_words(&["six"]),
        Some(RollResultShape::Fixed(6))
    );
}

#[test]
fn grouped_opponent_life_loss_reaches_the_typed_trigger_ast() {
    let tokens = tokenize_line("one or more opponents lose life", 0);
    let parsed =
        crate::runtime_backend::families::activation_and_restrictions::parse_trigger_clause_lexed(
            &tokens,
        )
        .unwrap();
    assert_eq!(
        parsed,
        crate::runtime_backend::ast::TriggerSpec::PlayersLoseLifeOneOrMore(PlayerFilter::Opponent,)
    );
    assert_eq!(
        crate::runtime_backend::compile_support::compile_trigger_spec(parsed),
        crate::triggers::Trigger::players_lose_life_one_or_more(PlayerFilter::Opponent),
    );
}
