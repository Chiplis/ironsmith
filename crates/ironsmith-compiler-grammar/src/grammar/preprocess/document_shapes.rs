use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, lex_line, render_token_slice};
use super::super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreprocessSentenceList {
    pub sentences: Vec<String>,
    pub terminal_period: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedPreprocessSentence {
    text: String,
    terminated: bool,
}

fn sentence_boundary<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::period().void(), eof.value(()))).parse_next(input)
}

fn parse_preprocess_sentence_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ParsedPreprocessSentence> {
    let body = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(sentence_boundary))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    let terminated = opt(primitives::period()).parse_next(input)?.is_some();
    Ok(ParsedPreprocessSentence {
        text: render_token_slice(body).trim().to_string(),
        terminated,
    })
}

fn parse_preprocess_sentence_list_lexed(
    input: &mut LexStream<'_>,
) -> WResult<PreprocessSentenceList> {
    let parsed: Vec<ParsedPreprocessSentence> =
        repeat(1.., parse_preprocess_sentence_lexed).parse_next(input)?;
    eof.parse_next(input)?;
    let terminal_period = parsed.last().is_some_and(|sentence| sentence.terminated);
    let sentences = parsed
        .into_iter()
        .map(|sentence| sentence.text)
        .filter(|sentence| !sentence.is_empty())
        .collect();
    Ok(PreprocessSentenceList {
        sentences,
        terminal_period,
    })
}

pub fn parse_preprocess_sentence_list(text: &str) -> Option<PreprocessSentenceList> {
    let tokens = crate::util::lex_fragment(text.trim(), 0)?;
    crate::grammar::primitives::probe_all(
        &tokens,
        parse_preprocess_sentence_list_lexed,
        "preprocess sentence list",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_sentence_lists_and_terminal_period() {
        assert_eq!(
            parse_preprocess_sentence_list("Draw a card. You gain 2 life."),
            Some(PreprocessSentenceList {
                sentences: vec!["Draw a card".to_string(), "You gain 2 life".to_string()],
                terminal_period: true,
            })
        );
        assert!(
            !parse_preprocess_sentence_list("Draw a card")
                .unwrap()
                .terminal_period
        );
    }
}
