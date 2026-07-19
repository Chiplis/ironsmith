use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, lex_line, render_token_slice};
use super::super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VoteCountRewriteSurface {
    DrawForEachVote {
        vote: String,
    },
    SharedSubjectPair {
        subject: String,
        first_action: String,
        first_vote: String,
        second_action: String,
        second_vote: String,
    },
    TrailingForEach {
        head: String,
        vote: String,
    },
}

pub(crate) fn parse_vote_count_rewrite_surface(sentence: &str) -> Option<VoteCountRewriteSurface> {
    let tokens = lex_line(sentence.trim(), 0).ok()?;
    primitives::parse_all(
        &tokens,
        alt((
            parse_shared_subject_vote_pair_lexed,
            parse_draw_for_each_vote_lexed,
            parse_trailing_for_each_vote_lexed,
        )),
        "vote-count rewrite",
    )
    .ok()
}

fn parse_draw_for_each_vote_lexed(input: &mut LexStream<'_>) -> WResult<VoteCountRewriteSurface> {
    primitives::phrase(&["you", "draw", "cards", "equal", "to", "the", "number", "of"])
        .parse_next(input)?;
    let vote_tokens = parse_vote_label_lexed(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(VoteCountRewriteSurface::DrawForEachVote {
        vote: render(vote_tokens),
    })
}

fn parse_shared_subject_vote_pair_lexed(
    input: &mut LexStream<'_>,
) -> WResult<VoteCountRewriteSurface> {
    let subject_tokens = alt((
        primitives::phrase(&["each", "opponent"]),
        primitives::phrase(&["each", "opponents"]),
        primitives::phrase(&["each", "player"]),
        primitives::kw("you").void(),
    ))
    .take()
    .parse_next(input)?;
    let first_action_tokens = take_until_for_each(input)?;
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    let first_vote_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            parse_vote_word,
            opt(primitives::comma()),
            primitives::kw("and"),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    parse_vote_word.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let second_action_tokens = take_until_for_each(input)?;
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    let second_vote_tokens = parse_vote_label_lexed(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(VoteCountRewriteSurface::SharedSubjectPair {
        subject: render(subject_tokens),
        first_action: render(first_action_tokens),
        first_vote: render(first_vote_tokens),
        second_action: render(second_action_tokens),
        second_vote: render(second_vote_tokens),
    })
}

fn parse_trailing_for_each_vote_lexed(
    input: &mut LexStream<'_>,
) -> WResult<VoteCountRewriteSurface> {
    let head_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(parse_complete_trailing_vote_tail_lexed),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let vote_tokens = parse_complete_trailing_vote_tail_lexed(input)?;
    Ok(VoteCountRewriteSurface::TrailingForEach {
        head: render(head_tokens),
        vote: render(vote_tokens),
    })
}

fn parse_complete_trailing_vote_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    let vote_tokens = parse_vote_label_lexed(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(vote_tokens)
}

fn take_until_for_each<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::phrase(&["for", "each"])))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn parse_vote_label_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let vote_tokens = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_vote_word))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    parse_vote_word.parse_next(input)?;
    Ok(vote_tokens)
}

fn parse_vote_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("vote"), primitives::kw("votes")))
        .void()
        .parse_next(input)
}

fn render(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_vote_rewrites_without_card_names() {
        assert_eq!(
            parse_vote_count_rewrite_surface("You draw cards equal to the number of truth votes"),
            Some(VoteCountRewriteSurface::DrawForEachVote {
                vote: "truth".to_string(),
            })
        );
        assert_eq!(
            parse_vote_count_rewrite_surface(
                "A source deals 3 damage to that player for each consequences vote"
            ),
            Some(VoteCountRewriteSurface::TrailingForEach {
                head: "A source deals 3 damage to that player".to_string(),
                vote: "consequences".to_string(),
            })
        );
    }

    #[test]
    fn parses_shared_subject_vote_pair() {
        assert_eq!(
            parse_vote_count_rewrite_surface(
                "Each opponent sacrifices a creature for each death vote and discards a card for each taxes vote."
            ),
            Some(VoteCountRewriteSurface::SharedSubjectPair {
                subject: "Each opponent".to_string(),
                first_action: "sacrifices a creature".to_string(),
                first_vote: "death".to_string(),
                second_action: "discards a card".to_string(),
                second_vote: "taxes".to_string(),
            })
        );
    }

}
