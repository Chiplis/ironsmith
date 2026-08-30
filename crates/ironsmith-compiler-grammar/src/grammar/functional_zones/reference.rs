use super::*;

pub(super) fn contains_named_source_command_zone_move(tokens: &[OwnedLexToken]) -> bool {
    let from_command = crate::slice_primitives::find_window_by(tokens, 3, |window| {
        window[0].is_word("from") && window[1].is_word("command") && window[2].is_word("zone")
    });
    let from_the_command = crate::slice_primitives::find_window_by(tokens, 4, |window| {
        window[0].is_word("from")
            && window[1].is_word("the")
            && window[2].is_word("command")
            && window[3].is_word("zone")
    });
    let origin = from_command.or(from_the_command);
    let Some(origin) = origin else {
        return false;
    };

    if tokens.first().is_some_and(|token| token.is_word("return")) {
        return is_named_or_normalized_source_surface(trim_named_source_surface(
            &tokens[1..origin],
        ));
    }
    if !tokens.first().is_some_and(|token| token.is_word("put")) {
        return false;
    }
    let Some(onto) =
        crate::slice_primitives::select_position(&tokens[1..origin], |token| token.is_word("onto"))
            .map(|index| index + 1)
    else {
        return false;
    };
    is_named_or_normalized_source_surface(trim_named_source_surface(&tokens[1..onto]))
}

pub(super) fn is_named_or_normalized_source_surface(tokens: &[OwnedLexToken]) -> bool {
    is_authored_proper_name_phrase(tokens)
        || matches!(
            normalized_activated_zone_words(tokens).as_slice(),
            ["this"] | ["this", "card"] | ["this", "creature"] | ["this", "permanent"]
        )
}

pub(super) fn trim_named_source_surface(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let start = usize::from(
        tokens
            .first()
            .is_some_and(|token| matches!(token.as_word(), Some("the" | "a" | "an"))),
    );
    &tokens[start..]
}
