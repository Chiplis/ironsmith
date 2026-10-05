//! Complete persistent damage-redirection clauses. Timed/shared-budget shields
//! have separate instruction grammars and cannot be accepted by this reader.
use winnow::combinator::{opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;
use crate::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone)]
pub struct StaticDamageRedirectionShape<'a> {
    pub untapped_source: Option<&'a [OwnedLexToken]>,
    pub combat_only: bool,
    pub recipient: &'a [OwnedLexToken],
    pub source: Option<&'a [OwnedLexToken]>,
    pub destination: &'a [OwnedLexToken],
}
fn parse_untapped_source<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["as", "long", "as"]).parse_next(input)?;
    let source = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::phrase(&["is", "untapped"])))
        .map(|((), _)| ()).take().parse_next(input)?;
    primitives::phrase(&["is", "untapped"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    Ok(source)
}
fn parse_lexed<'a>(input: &mut LexStream<'a>) -> WResult<StaticDamageRedirectionShape<'a>> {
    let untapped_source = opt(parse_untapped_source).parse_next(input)?;
    primitives::kw("all").parse_next(input)?;
    let combat_only = opt(primitives::kw("combat")).parse_next(input)?.is_some();
    primitives::phrase(&["damage", "that", "would", "be", "dealt", "to"]).parse_next(input)?;
    let recipients_and_source = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::phrase(&["is", "dealt", "to"])))
        .map(|((), _)| ()).take().parse_next(input)?;
    primitives::phrase(&["is", "dealt", "to"]).parse_next(input)?;
    let destination = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("instead")))
        .map(|((), _)| ()).take().parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let (recipient, source) = match recipients_and_source.iter().position(|token| token.is_word("by")) {
        Some(index) => (&recipients_and_source[..index], Some(&recipients_and_source[index + 1..])),
        None => (recipients_and_source, None),
    };
    Ok(StaticDamageRedirectionShape { untapped_source, combat_only, recipient, source, destination })
}
pub fn parse_static_damage_redirection(tokens: &[OwnedLexToken]) -> Option<StaticDamageRedirectionShape<'_>> {
    // Prevent duration/trailing instructions from being mistaken for filters.
    if tokens.iter().any(|token| token.is_word("turn") || token.is_word("until") || token.is_word("may")) { return None; }
    primitives::probe_all(tokens, parse_lexed, "persistent damage redirection")
}
