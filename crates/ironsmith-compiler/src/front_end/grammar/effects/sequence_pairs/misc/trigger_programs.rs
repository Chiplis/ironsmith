use super::*;

pub fn is_delayed_dies_exile_play_shape(first: &[OwnedLexToken], second: &[OwnedLexToken]) -> bool {
    if !starts_sequence(first, DELAYED_DIES) {
        return false;
    }
    let mut input = LexStream::new(first);
    let initial_len = input.len();
    let mut action_start = None;
    while !input.is_empty() {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = match parsed {
            Ok(token) => token,
            Err(_) => return false,
        };
        if token.kind == TokenKind::Comma {
            action_start = Some(initial_len.saturating_sub(input.len()));
            break;
        }
    }
    let Some(action_start) = action_start else {
        return false;
    };
    let action = &first[action_start..];
    starts_content_sequence(action, EXILE_TOP_POWER)
        && ends_content_sequence(action, CHOOSE_EXILED)
        && matches_complete_content_sequence(second, PLAY_NEXT_TURN)
}
