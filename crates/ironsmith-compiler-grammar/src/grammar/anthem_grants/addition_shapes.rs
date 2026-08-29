use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhereXYBindingsShape<'a> {
    pub x_tokens: &'a [OwnedLexToken],
    pub y_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeColorScope<'a> {
    Colors,
    Types {
        qualifier_tokens: &'a [OwnedLexToken],
    },
    Unsupported {
        tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeColorAdditionShape<'a> {
    pub descriptor_tokens: &'a [OwnedLexToken],
    pub scopes: Vec<TypeColorScope<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemAndAdditionShape<'a> {
    pub get_token: usize,
    pub and_token: usize,
    pub addition_tokens: &'a [OwnedLexToken],
    pub temporary: bool,
}

pub fn parse_where_x_y_bindings_shape(
    tokens: &[OwnedLexToken],
) -> Option<WhereXYBindingsShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&["where", "x", "is"]))?;
    let (x_tokens, y_tokens) = primitives::split_lexed_once_on_separator(rest, || {
        primitives::phrase(&["and", "y", "is"])
    })?;
    let x_tokens = trim_lexed_commas(x_tokens);
    let y_tokens = trim_lexed_commas(y_tokens);
    (!x_tokens.is_empty() && !y_tokens.is_empty())
        .then_some(WhereXYBindingsShape { x_tokens, y_tokens })
}

pub fn parse_type_color_addition_shape(
    tokens: &[OwnedLexToken],
) -> Option<TypeColorAdditionShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, after_be) = primitives::parse_prefix(tokens, parse_be_word)?;
    let (descriptor_tokens, scope_tokens) =
        primitives::split_lexed_once_on_separator(after_be, || {
            alt((
                primitives::phrase(&["in", "addition", "to", "its", "other"]),
                primitives::phrase(&["in", "addition", "to", "their", "other"]),
            ))
        })
        .unwrap_or((after_be, &[]));
    let descriptor_tokens = trim_lexed_commas(descriptor_tokens);
    if descriptor_tokens.is_empty() {
        return None;
    }
    if scope_tokens.is_empty() {
        return Some(TypeColorAdditionShape {
            descriptor_tokens,
            scopes: Vec::new(),
        });
    }
    let scopes = primitives::split_lexed_slices_on_and(scope_tokens)
        .into_iter()
        .map(trim_lexed_commas)
        .map(parse_scope)
        .collect::<Vec<_>>();
    Some(TypeColorAdditionShape {
        descriptor_tokens,
        scopes,
    })
}

pub fn parse_anthem_and_addition_shape(
    tokens: &[OwnedLexToken],
) -> Option<AnthemAndAdditionShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let get_token = find_word(tokens, 0, parse_get_word)?;
    let and_token = find_word(tokens, get_token + 1, parse_and_word)?;
    let addition_tokens = trim_lexed_commas(tokens.get(and_token + 1..)?);
    (!addition_tokens.is_empty()).then_some(AnthemAndAdditionShape {
        get_token,
        and_token,
        addition_tokens,
        temporary: primitives::find_prefix(tokens, || {
            primitives::phrase(&["until", "end", "of", "turn"])
        })
        .is_some(),
    })
}

fn parse_scope(tokens: &[OwnedLexToken]) -> TypeColorScope<'_> {
    if primitives::parse_all(tokens, parse_color_scope, "type/color scope").is_ok() {
        return TypeColorScope::Colors;
    }
    if let Some((qualifier_tokens, _)) =
        primitives::split_lexed_once_before_suffix(tokens, 0, || (parse_type_word, eof).void())
    {
        return TypeColorScope::Types {
            qualifier_tokens: trim_lexed_commas(qualifier_tokens),
        };
    }
    TypeColorScope::Unsupported { tokens }
}

fn find_word<'a>(
    tokens: &'a [OwnedLexToken],
    start: usize,
    mut parser: impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
) -> Option<usize> {
    let search = tokens.get(start..)?;
    let mut input = LexStream::new(search);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser.parse_next(&mut candidate).is_ok() {
            return Some(start + offset);
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

fn parse_be_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("is"),
        primitives::kw("are"),
        primitives::kw("it's"),
        primitives::kw("its"),
    ))
    .void()
    .parse_next(input)
}

fn parse_get_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("get"), primitives::kw("gets")))
        .void()
        .parse_next(input)
}

fn parse_and_word(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("and").void().parse_next(input)
}

fn parse_color_scope(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("color"), primitives::kw("colors")))
        .void()
        .parse_next(input)
}

fn parse_type_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("type"), primitives::kw("types")))
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn splits_where_x_and_y_bindings() {
        let tokens = lex("where X is your life total and Y is cards in your hand");
        let shape = parse_where_x_y_bindings_shape(&tokens).expect("bindings");
        assert!(!shape.x_tokens.is_empty());
        assert!(!shape.y_tokens.is_empty());
    }

    #[test]
    fn parses_type_and_color_scopes() {
        let tokens = lex("is black Zombie in addition to its other colors and creature types");
        let shape = parse_type_color_addition_shape(&tokens).expect("addition");
        assert_eq!(shape.scopes.len(), 2);

        let tokens = lex("are Equipment in addition to their other types");
        let shape = parse_type_color_addition_shape(&tokens).expect("plural addition");
        assert_eq!(shape.scopes.len(), 1);
    }

    #[test]
    fn parses_unscoped_color_setting_tail() {
        let tokens = lex("is black");
        let shape = parse_type_color_addition_shape(&tokens).expect("unscoped color tail");
        assert!(shape.scopes.is_empty());
        assert_eq!(
            shape
                .descriptor_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect::<Vec<_>>(),
            ["black"]
        );
    }
}
