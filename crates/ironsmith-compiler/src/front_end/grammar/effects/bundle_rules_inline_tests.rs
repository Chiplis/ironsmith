use super::*;
use crate::lexer::lex_line;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("bundle grammar fixture should lex")
}

#[test]
fn alternative_cost_bundle_returns_typed_kind() {
    let first = lex("You may cast a spell with flashback from your hand.");
    let second = lex("If you do, pay its flashback cost rather than its mana cost.");
    assert_eq!(
        parse_alternative_cost_bundle_shape(&first, &second),
        Some(AlternativeCostBundleShape {
            kind: AlternativeCastKind::Flashback,
        })
    );
}

#[test]
fn outside_game_choice_carries_filter_boundaries() {
    let first = lex(
        "You may reveal an Eldrazi card you own from outside the game or choose a face-up Eldrazi card you own in exile.",
    );
    let second = lex("Put that card into your hand.");
    let shape = parse_outside_game_choice_shape(&first, &second)
        .expect("shape parse should not be malformed")
        .expect("shape should match");
    assert!(has_atom(
        &parser_token_word_refs(shape.reveal_filter),
        "eldrazi"
    ));
    assert!(has_atom(
        &parser_token_word_refs(shape.choose_filter),
        "eldrazi"
    ));
}

#[test]
fn chosen_counter_shape_preserves_target_and_action() {
    let first = lex("Choose a counter on target permanent.");
    let second = lex("Put an additional counter of that kind on it.");
    let shape = parse_chosen_counter_bundle_shape(&first, &second).expect("shape");
    assert_eq!(shape.action, ChosenCounterAction::PutAdditional);
    assert!(matches!(shape.target, ChosenCounterTarget::Clause(_)));
}

#[test]
fn search_slot_shape_splits_article_led_filters() {
    let tokens = lex(
        "Search your library for a Plains card, an Island card, and a Swamp card, reveal those cards, put them into your hand, then shuffle.",
    );
    let shape = parse_search_library_slots_shape(&tokens).expect("shape");
    assert!(!shape.multi_zone);
    assert_eq!(shape.filters.len(), 3);
}

#[test]
fn explicit_slot_name_preserves_internal_punctuation() {
    let tokens = lex("a card named Nissa, Genesis Mage");
    assert_eq!(
        parse_explicit_card_name_surface_tokens(&tokens).as_deref(),
        Some("Nissa, Genesis Mage")
    );
}

#[test]
fn selected_hand_double_choice_keeps_both_filter_spans() {
    let first = lex("Target opponent reveals their hand.");
    let second = lex(
        "You choose from it a nonland card with mana value 3 or less and a card with mana value 4 or greater.",
    );
    let third = lex("That player discards those cards.");
    let shape = parse_selected_hand_double_choice_shape(&first, &second, &third)
        .expect("selected-hand double choice shape");

    assert_eq!(shape.revealed_player, RevealedHandPlayer::TargetOpponent);
    assert_eq!(
        parser_token_word_refs(shape.first_choice),
        [
            "a", "nonland", "card", "with", "mana", "value", "3", "or", "less"
        ]
    );
    assert_eq!(
        parser_token_word_refs(shape.second_choice),
        ["a", "card", "with", "mana", "value", "4", "or", "greater"]
    );
}
