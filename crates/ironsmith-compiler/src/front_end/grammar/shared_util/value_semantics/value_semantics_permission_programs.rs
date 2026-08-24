use super::*;

pub(super) fn parse_spells_cast_this_turn_matching_count_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let filter_words = word_view.to_word_refs();
    let surface = value_helper_shapes::parse_spell_cast_this_turn_surface(&filter_words)?;
    let filter_token_range = word_view.token_span_for_words(0, surface.filter_end)?;
    let filter_tokens = trim_commas(&tokens[filter_token_range]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::SpellsCastThisTurnMatching {
        player: surface.player,
        filter,
        exclude_source: surface.exclude_source,
    })
}

pub fn parse_spells_cast_this_turn_matching_count_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_words = TokenWordView::new(tokens);
    let word_refs = filter_words.to_word_refs();
    let surface = value_helper_shapes::parse_spell_cast_this_turn_surface(&word_refs)?;
    let filter_token_range = filter_words.token_span_for_words(0, surface.filter_end)?;
    let filter_tokens = trim_lexed_commas(&tokens[filter_token_range]);
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(Value::SpellsCastThisTurnMatching {
        player: surface.player,
        filter,
        exclude_source: surface.exclude_source,
    })
}
