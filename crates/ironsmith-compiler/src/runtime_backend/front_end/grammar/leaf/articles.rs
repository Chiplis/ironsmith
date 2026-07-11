use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::common::{finish_text_parse, phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafArticle {
    A,
    An,
    The,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LeafLeadingTokenWords<'a> {
    pub(crate) rest: &'a [OwnedLexToken],
    pub(crate) consumed_words: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LeafLeadingWordRefs<'slice, 'word> {
    pub(crate) rest: &'slice [&'word str],
    pub(crate) consumed_words: usize,
}

pub(crate) fn parse_leaf_article(input: &mut &str) -> WResult<LeafArticle> {
    alt((
        phrase("an").value(LeafArticle::An),
        phrase("a").value(LeafArticle::A),
        phrase("the").value(LeafArticle::The),
    ))
    .parse_next(input)
}

pub(crate) fn parse_leaf_article_complete(raw: &str) -> Result<LeafArticle, CardTextError> {
    finish_text_parse(raw, parse_leaf_article, "leaf-article")
}

pub(crate) fn parse_leaf_leading_articles_tokens(
    tokens: &[OwnedLexToken],
) -> LeafLeadingTokenWords<'_> {
    let (consumed_words, rest) =
        primitives::parse_prefix(tokens, parse_leading_articles_lexed).unwrap_or((0, tokens));
    LeafLeadingTokenWords {
        rest,
        consumed_words,
    }
}

pub(crate) fn parse_leaf_leading_indefinite_article_tokens(
    tokens: &[OwnedLexToken],
) -> LeafLeadingTokenWords<'_> {
    let (_, rest) = primitives::parse_prefix(tokens, parse_indefinite_article_lexed)
        .unwrap_or((LeafArticle::A, tokens));
    LeafLeadingTokenWords {
        rest,
        consumed_words: usize::from(rest.len() != tokens.len()),
    }
}

pub(crate) fn parse_leaf_leading_selected_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    accepted: &[&str],
) -> LeafLeadingTokenWords<'a> {
    let parser = |input: &mut LexStream<'a>| -> WResult<usize> {
        let mut consumed = 0;
        loop {
            let mut probe = input.clone();
            let Ok(word) = primitives::word_parser_text.parse_next(&mut probe) else {
                break;
            };
            if !accepted.iter().any(|accepted_word| word == *accepted_word) {
                break;
            }
            *input = probe;
            consumed += 1;
        }
        Ok(consumed)
    };
    let (consumed_words, rest) = primitives::parse_prefix(tokens, parser).unwrap_or((0, tokens));
    LeafLeadingTokenWords {
        rest,
        consumed_words,
    }
}

pub(crate) fn parse_leaf_leading_articles_words<'slice, 'word>(
    words: &'slice [&'word str],
) -> LeafLeadingWordRefs<'slice, 'word> {
    let mut input = words;
    let original_len = input.len();
    loop {
        let mut probe = input;
        if parse_article_word_slice.parse_next(&mut probe).is_err() {
            break;
        }
        input = probe;
    }
    LeafLeadingWordRefs {
        rest: input,
        consumed_words: original_len.saturating_sub(input.len()),
    }
}

fn parse_leading_articles_lexed<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let mut consumed = 0;
    loop {
        let mut probe = input.clone();
        if parse_article_lexed.parse_next(&mut probe).is_err() {
            break;
        }
        *input = probe;
        consumed += 1;
    }
    Ok(consumed)
}

fn parse_article_lexed<'a>(input: &mut LexStream<'a>) -> WResult<LeafArticle> {
    alt((
        primitives::kw("an").value(LeafArticle::An),
        primitives::kw("a").value(LeafArticle::A),
        primitives::kw("the").value(LeafArticle::The),
    ))
    .parse_next(input)
}

fn parse_indefinite_article_lexed<'a>(input: &mut LexStream<'a>) -> WResult<LeafArticle> {
    alt((
        primitives::kw("an").value(LeafArticle::An),
        primitives::kw("a").value(LeafArticle::A),
    ))
    .parse_next(input)
}

fn parse_article_word_slice<'slice, 'word>(
    input: &mut &'slice [&'word str],
) -> WResult<LeafArticle> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err("article", "a, an, or the"));
    };
    let article = match *word {
        "a" => LeafArticle::A,
        "an" => LeafArticle::An,
        "the" => LeafArticle::The,
        _ => return Err(primitives::backtrack_err("article", "a, an, or the")),
    };
    *input = rest;
    Ok(article)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{TokenWordView, lex_line};
    use super::*;

    #[test]
    fn article_parser_is_exact() {
        assert_eq!(parse_leaf_article_complete("an").unwrap(), LeafArticle::An);
        assert!(parse_leaf_article_complete("another").is_err());
    }

    #[test]
    fn leading_article_and_selected_prefixes_preserve_remainder() {
        let tokens = lex_line("the a creature", 0).unwrap();
        let parsed = parse_leaf_leading_articles_tokens(&tokens);
        assert_eq!(parsed.consumed_words, 2);
        assert_eq!(TokenWordView::new(parsed.rest).to_word_refs(), ["creature"]);

        let tokens = lex_line("then and draw", 0).unwrap();
        let parsed = parse_leaf_leading_selected_tokens(&tokens, &["then", "and"]);
        assert_eq!(parsed.consumed_words, 2);
        assert_eq!(TokenWordView::new(parsed.rest).to_word_refs(), ["draw"]);
    }
}
