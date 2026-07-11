use super::super::lexer::{OwnedLexToken, trim_lexed_commas};
use super::super::token_primitives::parse_leading_may_action_lexed;
use super::dispatch_entry::leading_may_actor_to_player;
use super::search_library::normalize_search_library_filter;
use crate::cards::builders::{CardTextError, ObjectFilter, PlayerAst};
use crate::runtime_backend::grammar::effects::looked_card_shapes;

pub(crate) fn parse_top_cards_view_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerAst, crate::effect::Value, bool)> {
    let shape = looked_card_shapes::parse_top_cards_view_shape(tokens)?;
    Some((PlayerAst::You, shape.count, shape.revealed))
}

pub(crate) fn parse_looked_card_choice_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter_tokens = looked_card_shapes::strip_up_to_one_looked_card_choice_tokens(tokens);
    if filter_tokens.is_empty() {
        return None;
    }
    let mut filter = parse_looked_card_reveal_filter(&filter_tokens)?;
    normalize_search_library_filter(&mut filter);
    filter.zone = None;
    Some(filter)
}

pub(crate) fn parse_counted_looked_cards_into_your_hand_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    looked_card_shapes::parse_counted_looked_cards_into_hand_shape(tokens).map(|shape| shape.count)
}

pub(crate) fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    looked_card_shapes::parse_kicked_counted_looked_cards_into_hand_shape(tokens)
        .map(|shape| shape.count)
}

pub(crate) fn parse_may_put_filtered_looked_card_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    let Some(action_match) =
        parse_leading_may_action_lexed(trim_lexed_commas(tokens), &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, PlayerAst::You);
    let Some(shape) =
        looked_card_shapes::parse_looked_card_battlefield_shape(action_match.tail_tokens)
    else {
        return Ok(None);
    };
    let filter = if let Some(filter) = parse_looked_card_choice_filter(shape.filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    Ok(Some((chooser, filter, shape.tapped)))
}

pub(crate) fn parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool, ObjectFilter)>, CardTextError> {
    let Some(action_match) =
        parse_leading_may_action_lexed(trim_lexed_commas(tokens), &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, PlayerAst::You);
    let Some(shape) =
        looked_card_shapes::parse_looked_card_battlefield_and_hand_shape(action_match.tail_tokens)
    else {
        return Ok(None);
    };
    let battlefield_filter = parse_looked_card_choice_filter(shape.battlefield_filter_tokens)
        .ok_or_else(|| {
            CardTextError::ParseError("unable to parse first looked-card choice filter".to_string())
        })?;
    let Some(hand_filter) = parse_looked_card_choice_filter(shape.hand_filter_tokens) else {
        return Ok(None);
    };
    Ok(Some((
        chooser,
        battlefield_filter,
        shape.tapped,
        hand_filter,
    )))
}

pub(crate) fn parse_if_you_dont_put_card_from_among_them_into_your_hand(
    tokens: &[OwnedLexToken],
) -> bool {
    looked_card_shapes::is_if_you_dont_put_looked_card_into_hand(tokens)
}

pub(crate) fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    looked_card_shapes::is_put_rest_on_library_bottom(tokens)
}

pub(crate) fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    looked_card_shapes::parse_looked_card_reveal_filter_shape(tokens)
}
