use super::*;

pub(super) fn parse_cards_discarded_this_turn_count_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = TokenWordView::new(tokens).to_word_refs();
    value_helper_shapes::parse_cards_discarded_this_turn_player(&words)
        .map(Value::CardsDiscardedThisTurn)
}

pub fn parse_players_with_cards_in_hand_at_least(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerFilter, u32)> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let with_idx = crate::word_primitives::parse_sequence_start(&words, &["with"])?;
    let players = match &words[..with_idx] {
        ["your", "opponents"] | ["opponents"] => PlayerFilter::Opponent,
        ["players"] | ["each", "player"] => PlayerFilter::Any,
        ["other", "players"] => PlayerFilter::NotYou,
        ["you"] => PlayerFilter::You,
        _ => return None,
    };
    let threshold_range = word_view.token_span_for_words(with_idx + 1, word_view.len())?;
    let threshold_tokens = trim_edge_punctuation(&tokens[threshold_range]);
    let (minimum, used) =
        crate::grammar::primitives::probe_shape(parse_greater_than_or_equal_quantity_prefix(
            &threshold_tokens,
            false,
            false,
            "player hand-size count",
        ))
        .flatten()?;
    let remainder = TokenWordView::new(&threshold_tokens[used..]).to_word_refs();
    matches!(
        remainder.as_slice(),
        ["card" | "cards", "in", "hand"] | ["card" | "cards", "in", "their", "hand"]
    )
    .then_some((players, minimum))
}
