use super::*;

pub fn parse_conjoined_untap_all_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConjoinedUntapAllShape<'_>> {
    let after_all = primitives::strip_lexed_prefix_phrase(tokens, &["all"])?;
    let (left_filter_tokens, right_filter_tokens) =
        primitives::split_lexed_once_on_separator(after_all, || {
            primitives::phrase(&["and", "all"]).void()
        })?;
    if left_filter_tokens.is_empty() || right_filter_tokens.is_empty() {
        return None;
    }
    Some(ConjoinedUntapAllShape {
        left_filter_tokens,
        right_filter_tokens,
    })
}

pub fn parse_untap_action_tokens(tokens: &[OwnedLexToken]) -> UntapActionShape<'_> {
    if let Some((_, filter_tokens)) = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("all"), primitives::kw("each"))).void(),
    ) && !filter_tokens.is_empty()
    {
        return UntapActionShape::All { filter_tokens };
    }
    if permission_shapes::exact_tokens(tokens, &["them"]) {
        UntapActionShape::Tagged {
            filter_tokens: None,
        }
    } else if let Some((_, filter_tokens)) =
        primitives::parse_prefix(tokens, primitives::kw("those").void())
        && !filter_tokens.is_empty()
    {
        UntapActionShape::Tagged {
            filter_tokens: Some(filter_tokens),
        }
    } else {
        UntapActionShape::Explicit {
            target_tokens: tokens,
        }
    }
}
