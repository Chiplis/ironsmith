use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedKeywordGrantAndLossSpec<'a> {
    pub grant_tokens: &'a [OwnedLexToken],
}

pub fn parse_attached_keyword_grant_and_loss_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedKeywordGrantAndLossSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_attached_keyword_grant_and_loss_lexed,
        "attached keyword grant and loss",
    )
}

fn parse_attached_keyword_grant_and_loss_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedKeywordGrantAndLossSpec<'a>> {
    alt((primitives::kw("has"), primitives::kw("have"))).parse_next(input)?;
    let grant_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            primitives::kw("and"),
            alt((primitives::kw("lose"), primitives::kw("loses"))),
            primitives::phrase(&["all", "other", "abilities"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("lose"), primitives::kw("loses"))).parse_next(input)?;
    primitives::phrase(&["all", "other", "abilities"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Some(AttachedKeywordGrantAndLossSpec {
        grant_tokens: trim_lexed_commas(grant_tokens),
    })
    .ok_or_else(|| primitives::backtrack_err("attached keyword grant", "keyword tokens"))
}
