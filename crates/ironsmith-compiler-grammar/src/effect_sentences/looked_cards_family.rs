use super::super::lexer::{OwnedLexToken, trim_lexed_commas};
use super::super::token_primitives::parse_leading_may_action_lexed;
use super::dispatch_entry::leading_may_actor_to_player;
use super::search_library::normalize_search_library_filter;
use crate::cards::builders::{CardTextError, ObjectFilter, PlayerAst};
use crate::grammar::effects::looked_card_shapes;

fn parse_imperative_possessive_library_view(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerAst, crate::effect::Value, bool)> {
    let owner_start = crate::slice_primitives::find_window_by(tokens, 4, |window| {
        if !window[0].is_word("of") || !window[3].is_word("library") {
            return false;
        }
        window[1].is_word("target")
            && (window[2].is_word("player's") || window[2].is_word("opponent's"))
    })?;
    let player = if tokens[owner_start + 2].is_word("player's") {
        PlayerAst::Target
    } else {
        PlayerAst::TargetOpponent
    };

    // Reuse the ordinary, fully typed top-card count grammar after replacing
    // only the possessive library owner. The returned PlayerAst retains the
    // target declaration; this rewrite changes no effect semantics.
    let mut normalized = tokens.to_vec();
    normalized.remove(owner_start + 1);
    normalized.get_mut(owner_start + 1)?.replace_word("your");
    let shape = looked_card_shapes::parse_top_cards_view_shape(&normalized)?;
    Some((player, shape.count, shape.revealed))
}

pub fn parse_top_cards_view_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerAst, crate::effect::Value, bool)> {
    let tokens = crate::util::trim_edge_punctuation_tokens(trim_lexed_commas(tokens));
    if let Some(view) = parse_imperative_possessive_library_view(tokens) {
        return Some(view);
    }
    let explicit_subject = match tokens {
        [first, second, ..]
            if first.parser_text() == "target" && second.parser_text() == "opponent" =>
        {
            Some((PlayerAst::TargetOpponent, 2))
        }
        [first, second, ..]
            if first.parser_text() == "target" && second.parser_text() == "player" =>
        {
            Some((PlayerAst::Target, 2))
        }
        [first, second, ..]
            if first.parser_text() == "that" && second.parser_text() == "player" =>
        {
            Some((PlayerAst::That, 2))
        }
        [first, second, ..]
            if first.parser_text() == "an" && second.parser_text() == "opponent" =>
        {
            Some((PlayerAst::Opponent, 2))
        }
        [first, ..] if first.parser_text() == "they" => Some((PlayerAst::That, 1)),
        [first, ..] if first.parser_text() == "you" => Some((PlayerAst::You, 1)),
        _ => None,
    };

    // The generic view-shape grammar intentionally tolerates surrounding
    // words, so trying it first can silently consume an explicit subject and
    // default the library owner to You.  Preserve explicit player subjects
    // before falling back to the ordinary imperative form.
    let Some((player, subject_len)) = explicit_subject else {
        let shape = looked_card_shapes::parse_top_cards_view_shape(tokens)?;
        return Some((PlayerAst::You, shape.count, shape.revealed));
    };

    let mut action = tokens.get(subject_len..)?.to_vec();
    let first_action = action.first_mut()?;
    let first_action_word = first_action.parser_text().to_string();
    match first_action_word.as_str() {
        "reveals" => {
            first_action.replace_word("reveal");
        }
        "looks" => {
            first_action.replace_word("look");
        }
        "reveal" | "look" => {}
        _ => return None,
    }
    for token in &mut action {
        if token.parser_text() == "their" {
            token.replace_word("your");
        }
    }

    let shape = looked_card_shapes::parse_top_cards_view_shape(&action)?;
    Some((player, shape.count, shape.revealed))
}

pub fn parse_looked_card_choice_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter_tokens = looked_card_shapes::strip_up_to_one_looked_card_choice_tokens(tokens);
    if filter_tokens.is_empty() {
        return None;
    }
    let mut filter = parse_looked_card_reveal_filter(&filter_tokens)?;
    normalize_search_library_filter(&mut filter);
    filter.zone = None;
    Some(filter)
}

pub fn parse_counted_looked_cards_into_your_hand_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    looked_card_shapes::parse_counted_looked_cards_into_hand_shape(tokens).map(|shape| shape.count)
}

pub fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    looked_card_shapes::parse_kicked_counted_looked_cards_into_hand_shape(tokens)
        .map(|shape| shape.count)
}

pub fn parse_may_put_filtered_looked_card_onto_battlefield(
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

pub fn parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
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

pub fn parse_if_you_dont_put_card_from_among_them_into_your_hand(tokens: &[OwnedLexToken]) -> bool {
    looked_card_shapes::is_if_you_dont_put_looked_card_into_hand(tokens)
}

pub fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    looked_card_shapes::is_put_rest_on_library_bottom(tokens)
}

pub fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    looked_card_shapes::parse_looked_card_reveal_filter_shape(tokens)
}
