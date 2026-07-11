use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::{ObjectFilter, PlayerFilter};

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AnthemSubjectGrammarMatch {
    Filter(ObjectFilter),
    RejectFragment,
}

pub(crate) fn parse_exact_anthem_subject_grammar(
    tokens: &[OwnedLexToken],
) -> Option<AnthemSubjectGrammarMatch> {
    primitives::parse_all(
        trim_lexed_commas(tokens),
        alt((
            parse_commander_subject,
            parse_attacking_token_subject,
            parse_dangling_conjunction_fragment,
            parse_leading_condition_fragment,
        )),
        "anthem subject",
    )
    .ok()
}

fn parse_commander_subject(input: &mut LexStream<'_>) -> WResult<AnthemSubjectGrammarMatch> {
    alt((primitives::kw("commander"), primitives::kw("commanders")))
        .void()
        .parse_next(input)?;
    let controller = parse_controller_clause(input)?;
    Ok(AnthemSubjectGrammarMatch::Filter(
        ObjectFilter::permanent()
            .commander()
            .controlled_by(controller),
    ))
}

fn parse_attacking_token_subject(input: &mut LexStream<'_>) -> WResult<AnthemSubjectGrammarMatch> {
    primitives::kw("attacking").void().parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens")))
        .void()
        .parse_next(input)?;
    let controller = parse_controller_clause(input)?;
    let mut filter = ObjectFilter::permanent().token().controlled_by(controller);
    filter.attacking = true;
    Ok(AnthemSubjectGrammarMatch::Filter(filter))
}

fn parse_controller_clause(input: &mut LexStream<'_>) -> WResult<PlayerFilter> {
    alt((
        primitives::phrase(&["you", "control"]).value(PlayerFilter::You),
        primitives::phrase(&["opponents", "control"]).value(PlayerFilter::Opponent),
        primitives::phrase(&["an", "opponent", "controls"]).value(PlayerFilter::Opponent),
    ))
    .parse_next(input)
}

fn parse_dangling_conjunction_fragment(
    input: &mut LexStream<'_>,
) -> WResult<AnthemSubjectGrammarMatch> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek((primitives::kw("and"), eof)).void())
        .map(|((), ())| ())
        .parse_next(input)?;
    primitives::kw("and").void().parse_next(input)?;
    Ok(AnthemSubjectGrammarMatch::RejectFragment)
}

fn parse_leading_condition_fragment(
    input: &mut LexStream<'_>,
) -> WResult<AnthemSubjectGrammarMatch> {
    primitives::phrase(&["as", "long", "as"])
        .void()
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek((primitives::kw("it"), eof)).void())
        .map(|((), ())| ())
        .parse_next(input)?;
    primitives::kw("it").void().parse_next(input)?;
    Ok(AnthemSubjectGrammarMatch::RejectFragment)
}

#[cfg(test)]
#[path = "subject_shapes/tests.rs"]
mod tests;
