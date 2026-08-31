use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EtbEntryFilterSpec<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsEntersRevealFromHandSpec<'a> {
    pub source_kind_tokens: &'a [OwnedLexToken],
    pub reveal_filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealedThisWayOrControlSpec<'a> {
    pub reveal_filter_tokens: &'a [OwnedLexToken],
    pub control_condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastEntersAdditionalCounterSpec<'a> {
    pub spell_filter_tokens: &'a [OwnedLexToken],
    pub condition_tokens: &'a [OwnedLexToken],
    pub entry_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsEntersSpec<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub tail_tokens: &'a [OwnedLexToken],
}

pub fn parse_entry_filter_tokens(tokens: &[OwnedLexToken]) -> Option<EtbEntryFilterSpec<'_>> {
    crate::grammar::primitives::probe_all(tokens, parse_entry_filter_lexed, "ETB entry filter")
}

pub fn parse_reveal_from_hand_filter_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, filter_tokens, _) = primitives::find_prefix(tokens, || parse_reveal_from_hand_lexed)?;
    Some(filter_tokens)
}

pub fn parse_as_enters_reveal_from_hand_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AsEntersRevealFromHandSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_as_enters_reveal_from_hand_lexed,
        "as-enters reveal from hand",
    )
}

pub fn parse_revealed_this_way_or_control_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RevealedThisWayOrControlSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_revealed_this_way_or_control_lexed,
        "revealed-this-way-or-control condition",
    )
}

pub fn parse_enters_tapped_unless_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_enters_tapped_unless_condition_lexed,
        "enters-tapped-unless condition",
    )
}

pub fn parse_spell_cast_enters_additional_counter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpellCastEntersAdditionalCounterSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_spell_cast_enters_additional_counter_lexed,
        "spell-cast enters-with-additional-counter",
    )
}

pub fn parse_as_enters_tokens(tokens: &[OwnedLexToken]) -> Option<AsEntersSpec<'_>> {
    crate::grammar::primitives::probe_all(tokens, parse_as_enters_lexed, "as-enters clause")
}

fn parse_entry_filter_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EtbEntryFilterSpec<'a>> {
    let filter_tokens = take_until_entry_verb(input)?;
    parse_entry_verb(input)?;
    let tail_tokens = take_nonempty_sentence_body(input)?;
    Ok(EtbEntryFilterSpec {
        filter_tokens: trim_lexed_commas(filter_tokens),
        tail_tokens,
    })
}

fn parse_reveal_from_hand_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::kw("reveal").parse_next(input)?;
    let filter_tokens = take_until_phrase(input, &["from", "your", "hand"])?;
    primitives::phrase(&["from", "your", "hand"]).parse_next(input)?;
    Ok(trim_lexed_commas(filter_tokens))
}

fn parse_as_enters_reveal_from_hand_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AsEntersRevealFromHandSpec<'a>> {
    primitives::phrase(&["as", "this"]).parse_next(input)?;
    let source_kind_tokens = take_until_phrase(input, &["enters"])?;
    primitives::kw("enters").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may", "reveal"]).parse_next(input)?;
    let reveal_filter_tokens = take_until_phrase(input, &["from", "your", "hand"])?;
    primitives::phrase(&["from", "your", "hand"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AsEntersRevealFromHandSpec {
        source_kind_tokens: trim_lexed_commas(source_kind_tokens),
        reveal_filter_tokens: trim_lexed_commas(reveal_filter_tokens),
    })
}

fn parse_revealed_this_way_or_control_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RevealedThisWayOrControlSpec<'a>> {
    primitives::phrase(&["you", "revealed"]).parse_next(input)?;
    let reveal_filter_tokens = take_until_phrase(input, &["this", "way", "or"])?;
    primitives::phrase(&["this", "way", "or"]).parse_next(input)?;
    let control_condition_tokens = take_nonempty_sentence_body(input)?;
    Ok(RevealedThisWayOrControlSpec {
        reveal_filter_tokens: trim_lexed_commas(reveal_filter_tokens),
        control_condition_tokens,
    })
}

fn parse_enters_tapped_unless_condition_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    let entry_prefix = take_until_phrase(input, &["unless"])?;
    if !contains_enters_tapped_phrase(entry_prefix) {
        return Err(primitives::backtrack_err(
            "enters tapped unless",
            "an enters-tapped entry prefix",
        ));
    }
    primitives::kw("unless").parse_next(input)?;
    take_nonempty_sentence_body(input)
}

fn parse_spell_cast_enters_additional_counter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SpellCastEntersAdditionalCounterSpec<'a>> {
    primitives::phrase(&["whenever", "you", "cast"]).parse_next(input)?;
    let spell_filter_tokens = take_until_comma(input)?;
    primitives::comma().parse_next(input)?;
    let condition_tokens = take_until_comma(input)?;
    primitives::comma().parse_next(input)?;
    let entry_tokens = take_nonempty_sentence_body(input)?;
    if !contains_with_additional_counter(entry_tokens) {
        return Err(primitives::backtrack_err(
            "spell-cast ETB counter line",
            "entry text containing with additional counters",
        ));
    }
    Ok(SpellCastEntersAdditionalCounterSpec {
        spell_filter_tokens: trim_lexed_commas(spell_filter_tokens),
        condition_tokens: trim_lexed_commas(condition_tokens),
        entry_tokens,
    })
}

fn parse_as_enters_lexed<'a>(input: &mut LexStream<'a>) -> WResult<AsEntersSpec<'a>> {
    primitives::kw("as").parse_next(input)?;
    let subject_tokens = take_until_entry_verb(input)?;
    parse_entry_verb(input)?;
    let tail_tokens = take_nonempty_sentence_body(input)?;
    Ok(AsEntersSpec {
        subject_tokens: trim_lexed_commas(subject_tokens),
        tail_tokens,
    })
}

fn parse_entry_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("enter"), primitives::kw("enters")))
        .void()
        .parse_next(input)
}

fn take_until_entry_verb<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(parse_entry_verb))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn take_until_phrase<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::phrase(phrase)))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn take_until_comma<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::comma()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn take_nonempty_sentence_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let body = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let body = trim_lexed_commas(body);
    if body.is_empty() {
        return Err(primitives::backtrack_err(
            "ETB clause body",
            "non-empty body",
        ));
    }
    Ok(body)
}

fn contains_enters_tapped_phrase(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        (
            parse_entry_verb,
            alt((
                primitives::phrase(&["the", "battlefield", "tapped"]),
                primitives::kw("tapped").void(),
            )),
        )
            .void()
    })
    .is_some()
}

fn contains_with_additional_counter(tokens: &[OwnedLexToken]) -> bool {
    let mut parser = (
        repeat_till(0.., any.void(), peek(primitives::kw("with"))).map(|((), _)| ()),
        primitives::kw("with"),
        repeat_till(0.., any.void(), peek(primitives::kw("additional"))).map(|((), _)| ()),
        primitives::kw("additional"),
        repeat_till(
            0..,
            any.void(),
            peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
        )
        .map(|((), _)| ()),
        alt((primitives::kw("counter"), primitives::kw("counters"))),
    )
        .void();
    parser.parse_next(&mut LexStream::new(tokens)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_entry_and_as_enters_shapes() {
        let tokens = lex_line("Artifacts you control enter tapped.", 0).unwrap();
        let parsed = parse_entry_filter_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(parsed.filter_tokens),
            "Artifacts you control"
        );
        assert_eq!(render_token_slice(parsed.tail_tokens), "tapped");

        let tokens = lex_line("As this creature enters, it becomes a 3/3.", 0).unwrap();
        let parsed = parse_as_enters_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(parsed.subject_tokens), "this creature");
        assert_eq!(render_token_slice(parsed.tail_tokens), "it becomes a 3/3");
    }

    #[test]
    fn parses_reveal_shapes() {
        let tokens = lex_line(
            "As this land enters, you may reveal a Forest card from your hand.",
            0,
        )
        .unwrap();
        let parsed = parse_as_enters_reveal_from_hand_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(parsed.source_kind_tokens), "land");
        assert_eq!(
            render_token_slice(parsed.reveal_filter_tokens),
            "a Forest card"
        );

        let tokens = lex_line(
            "you revealed a Forest card this way or you control a Forest",
            0,
        )
        .unwrap();
        let parsed = parse_revealed_this_way_or_control_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(parsed.reveal_filter_tokens),
            "a Forest card"
        );
        assert_eq!(
            render_token_slice(parsed.control_condition_tokens),
            "you control a Forest"
        );
    }

    #[test]
    fn parses_unless_and_spell_cast_entry_shapes() {
        let tokens = lex_line("This land enters tapped unless you control a Forest.", 0).unwrap();
        let condition = parse_enters_tapped_unless_condition_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(condition), "you control a Forest");

        let tokens = lex_line(
            "Whenever you cast a snow spell, if snow mana was spent to cast it, it enters with an additional counter.",
            0,
        )
        .unwrap();
        let parsed = parse_spell_cast_enters_additional_counter_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(parsed.spell_filter_tokens),
            "a snow spell"
        );
        assert_eq!(
            render_token_slice(parsed.condition_tokens),
            "if snow mana was spent to cast it"
        );
    }
}
