use winnow::prelude::*;

use crate::cards::builders::CardTextError;

use super::super::super::lexer::{OwnedLexToken, render_token_slice};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HideawayKeywordShape {
    pub(crate) count: i32,
}

pub(crate) fn parse_hideaway_keyword_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<HideawayKeywordShape>, CardTextError> {
    let Some(((), count_tokens)) =
        primitives::parse_prefix(tokens, primitives::kw("hideaway").void())
    else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);
    let count_result = primitives::parse_all(
        count_tokens,
        (
            leaf::parse_leaf_number_prefix_lexed,
            winnow::combinator::opt(primitives::period()),
        )
            .map(|(count, _)| count),
        "hideaway keyword count",
    )
    .map_err(|_| {
        CardTextError::ParseError(format!(
            "hideaway keyword expected numeric count in '{display}'"
        ))
    });
    let count = match count_result {
        Ok(count) => count,
        Err(error) => {
            let is_single_malformed_count = primitives::parse_all(
                count_tokens,
                (
                    primitives::word_parser_text,
                    winnow::combinator::opt(primitives::period()),
                )
                    .void(),
                "hideaway keyword malformed count",
            )
            .is_ok();
            if is_single_malformed_count {
                return Err(error);
            }
            return Ok(None);
        }
    };
    if count == 0 {
        return Err(CardTextError::ParseError(format!(
            "hideaway keyword expected positive count in '{display}'"
        )));
    }
    Ok(Some(HideawayKeywordShape {
        count: count as i32,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn parses_typed_hideaway_count_and_rejects_malformed_counts() {
        let hideaway = lex_line("Hideaway 5.", 0).unwrap();
        assert_eq!(
            parse_hideaway_keyword_tokens(&hideaway).unwrap(),
            Some(HideawayKeywordShape { count: 5 })
        );
        let malformed = lex_line("Hideaway X.", 0).unwrap();
        assert!(parse_hideaway_keyword_tokens(&malformed).is_err());
        let unrelated = lex_line("Flying", 0).unwrap();
        assert_eq!(parse_hideaway_keyword_tokens(&unrelated).unwrap(), None);
    }
}
