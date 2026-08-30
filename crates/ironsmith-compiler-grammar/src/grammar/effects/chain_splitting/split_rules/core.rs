use super::*;

pub(super) fn find_then_split(
    segment: &[OwnedLexToken],
    is_ability_head: &mut impl FnMut(&[OwnedLexToken]) -> bool,
) -> Option<ThenSplit> {
    let starts_with_for_each = starts_with_each_player_or_opponent(segment);
    let mut input = LexStream::new(segment);
    let mut inside_quotes = false;
    while !input.is_empty() {
        let idx = segment.len().saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            break;
        };
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        let (then_idx, explicit_comma_then) = if token.kind == TokenKind::Comma
            && segment
                .get(idx + 1)
                .is_some_and(|next| is_word(next, "then"))
        {
            (idx + 1, true)
        } else if is_word(token, "then") {
            (idx, false)
        } else {
            continue;
        };
        let before = trim_lexed_commas(segment.get(..idx).unwrap_or_default());
        let after = trim_lexed_commas(segment.get(then_idx + 1..).unwrap_or_default());
        let facts = then_followup_facts(before, after, starts_with_for_each);
        if facts.should_split(is_ability_head(after)) {
            return Some(ThenSplit {
                separator_idx: idx,
                then_idx,
                explicit_comma_then,
            });
        }
    }
    None
}

pub(super) fn is_word(token: &OwnedLexToken, expected: &'static str) -> bool {
    let mut input = LexStream::new(std::slice::from_ref(token));
    (
        super::super::super::super::primitives::kw(expected),
        super::super::super::super::primitives::end_of_block(),
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}
