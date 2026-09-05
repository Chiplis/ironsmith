use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::TagKey;
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::target::PlayerFilter;

use super::super::primitives;

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCounteredTriggerSpec<'a> {
    pub filter_tokens: Option<&'a [OwnedLexToken]>,
    pub controller: PlayerFilter,
}

fn parse_cast_verb<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("cast"), primitives::kw("casts")))
        .void()
        .parse_next(input)
}

fn parse_linking_be<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::kw("is"),
        primitives::kw("are"),
        primitives::kw("was"),
        primitives::kw("were"),
        primitives::kw("be"),
        primitives::kw("been"),
    ))
    .void()
    .parse_next(input)
}

fn parse_countered_suffix<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    (
        parse_linking_be,
        primitives::kw("countered"),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn parse_spell_noun<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("spell"), primitives::kw("spells")))
        .void()
        .parse_next(input)
}

fn parse_you_cast_controller<'a>(
    input: &mut LexStream<'a>,
) -> Result<PlayerFilter, ErrMode<ContextError>> {
    alt((
        primitives::kw("you"),
        primitives::kw("youve"),
        primitives::kw("you've"),
    ))
    .value(PlayerFilter::You)
    .parse_next(input)
}

fn parse_opponent_controller_word<'a>(
    input: &mut LexStream<'a>,
) -> Result<PlayerFilter, ErrMode<ContextError>> {
    alt((primitives::kw("opponent"), primitives::kw("opponents")))
        .value(PlayerFilter::Opponent)
        .parse_next(input)
}

fn take_subject_before_countered_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    repeat_till(0.., any.void(), peek(parse_countered_suffix))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn take_before_first_cast(
    tokens: &[OwnedLexToken],
) -> Result<&[OwnedLexToken], ErrMode<ContextError>> {
    let mut input = LexStream::new(tokens);
    let before_cast = repeat_till(0.., any.void(), peek(parse_cast_verb))
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)?;
    parse_cast_verb.parse_next(&mut input)?;
    Ok(before_cast)
}

fn strip_explicit_you_controller(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::split_lexed_once_before_suffix(tokens, 0, || parse_you_cast_controller)
        .map(|(filter_tokens, _)| filter_tokens)
}

fn strip_opponent_controller(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (opponent_idx, _, _) = primitives::find_prefix(tokens, || parse_opponent_controller_word)?;
    let controller_start = opponent_idx.saturating_sub(1);
    Some(&tokens[..controller_start])
}

fn has_parser<'a, O, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    primitives::find_prefix(tokens, make_parser).is_some()
}

fn infer_subject_controller(tokens: &[OwnedLexToken]) -> PlayerFilter {
    if has_parser(tokens, || {
        alt((
            primitives::phrase(&["on", "your", "team"]),
            primitives::phrase(&["your", "team"]),
            primitives::kw("you").void(),
        ))
    }) {
        PlayerFilter::You
    } else if has_parser(tokens, || {
        alt((
            primitives::phrase(&["enchanted", "player"]),
            primitives::phrase(&["enchanted", "players"]),
        ))
    }) {
        PlayerFilter::TaggedPlayer(crate::tag::CompilerReferenceTag::Enchanted.bind())
    } else if has_parser(tokens, || {
        alt((
            primitives::phrase(&["chosen", "player"]),
            primitives::phrase(&["chosen", "players"]),
        ))
    }) {
        PlayerFilter::ChosenPlayer
    } else if has_parser(tokens, || parse_opponent_controller_word) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

fn parse_unqualified_spell_filter<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        alt((
            primitives::phrase(&["a", "spell"]),
            primitives::kw("spell").void(),
            primitives::kw("spells").void(),
        )),
        eof,
    )
        .void()
        .parse_next(input)
}

fn is_unqualified_spell_filter(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_unqualified_spell_filter).is_some()
}

fn parse_spell_countered_trigger_spec<'a>(
    input: &mut LexStream<'a>,
) -> Result<SpellCounteredTriggerSpec<'a>, ErrMode<ContextError>> {
    let subject_tokens = take_subject_before_countered_suffix(input)?;
    parse_countered_suffix.parse_next(input)?;

    let subject_tokens = trim_lexed_commas(subject_tokens);
    if subject_tokens.is_empty() || !has_parser(subject_tokens, || parse_spell_noun) {
        return Err(primitives::backtrack_err(
            "spell-countered trigger subject",
            "spell subject",
        ));
    }

    let before_cast = take_before_first_cast(subject_tokens)?;
    let (filter_tokens, controller) =
        if let Some(filter_tokens) = strip_explicit_you_controller(before_cast) {
            (filter_tokens, PlayerFilter::You)
        } else if let Some(filter_tokens) = strip_opponent_controller(before_cast) {
            (filter_tokens, PlayerFilter::Opponent)
        } else {
            (before_cast, infer_subject_controller(subject_tokens))
        };

    let filter_tokens = (!filter_tokens.is_empty() && !is_unqualified_spell_filter(filter_tokens))
        .then_some(filter_tokens);

    Ok(SpellCounteredTriggerSpec {
        filter_tokens,
        controller,
    })
}

pub fn parse_spell_countered_trigger_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SpellCounteredTriggerSpec<'_>> {
    primitives::parse_prefix(tokens, parse_spell_countered_trigger_spec).map(|(spec, _)| spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_you_cast_controller_and_unqualified_spell() {
        let tokens = lex_line("A spell you've cast is countered.", 0).unwrap();
        let spec = parse_spell_countered_trigger_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.controller, PlayerFilter::You);
        assert!(spec.filter_tokens.is_none());
    }

    #[test]
    fn parses_opponent_controller_and_preserves_qualified_filter_tokens() {
        let tokens = lex_line("An instant spell an opponent cast was countered.", 0).unwrap();
        let spec = parse_spell_countered_trigger_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.controller, PlayerFilter::Opponent);
        assert_eq!(
            TokenWordView::new(spec.filter_tokens.unwrap()).word_refs(),
            ["an", "instant", "spell"]
        );
    }

    #[test]
    fn infers_opponent_from_the_post_cast_subject_tail() {
        let tokens = lex_line("A spell cast by an opponent is countered.", 0).unwrap();
        let spec = parse_spell_countered_trigger_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.controller, PlayerFilter::Opponent);
        assert!(spec.filter_tokens.is_none());
    }

    #[test]
    fn rejects_countered_clauses_without_a_cast_relation() {
        let tokens = lex_line("A spell is countered.", 0).unwrap();
        assert!(parse_spell_countered_trigger_spec_lexed(&tokens).is_none());
    }
}
