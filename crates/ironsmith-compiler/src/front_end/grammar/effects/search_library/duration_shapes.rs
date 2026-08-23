use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;
use crate::effect::Until;
use crate::grammar::{leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};

#[derive(Debug, Clone, PartialEq)]
pub struct SearchRestrictionDurationShape {
    pub duration: Until,
    pub remainder: Vec<OwnedLexToken>,
    pub placement: SearchRestrictionDurationPlacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchRestrictionDurationPlacement {
    Prefix,
    Suffix,
}

fn as_long_as_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

fn marker_present(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(expected)).is_some()
}

fn source_reference_present(tokens: &[OwnedLexToken]) -> bool {
    [
        "this",
        "thiss",
        "source",
        "artifact",
        "creature",
        "permanent",
    ]
    .into_iter()
    .any(|word| marker_present(tokens, word))
}

fn as_long_as_you_control_source(tokens: &[OwnedLexToken]) -> bool {
    marker_present(tokens, "you")
        && marker_present(tokens, "control")
        && source_reference_present(tokens)
}

fn as_long_as_source_remains_tapped(tokens: &[OwnedLexToken]) -> bool {
    marker_present(tokens, "remains")
        && marker_present(tokens, "tapped")
        && source_reference_present(tokens)
}

fn as_long_as_source_remains_on_battlefield(tokens: &[OwnedLexToken]) -> bool {
    marker_present(tokens, "remains")
        && marker_present(tokens, "battlefield")
        && source_reference_present(tokens)
}

fn comma_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Comma {
            return tokens.get(idx + 1..);
        }
    }
    None
}

fn until_from_leaf(duration: leaf::LeafDurationPhrase) -> Until {
    match duration {
        leaf::LeafDurationPhrase::ThisTurn | leaf::LeafDurationPhrase::UntilEndOfTurn => {
            Until::EndOfTurn
        }
        leaf::LeafDurationPhrase::UntilEndOfCombat => Until::EndOfCombat,
        leaf::LeafDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        leaf::LeafDurationPhrase::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
        leaf::LeafDurationPhrase::UntilYourNextUpkeep => Until::YourNextUpkeep,
        leaf::LeafDurationPhrase::ControllersNextUntapStep => Until::ControllersNextUntapStep,
        leaf::LeafDurationPhrase::Forever => Until::Forever,
    }
}

pub fn parse_search_restriction_duration_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<SearchRestrictionDurationShape>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some(parsed) = leaf::parse_leaf_restriction_duration_prefix_tokens(tokens) {
        return Ok(Some(SearchRestrictionDurationShape {
            duration: until_from_leaf(parsed.duration),
            remainder: trim_lexed_commas(parsed.rest).to_vec(),
            placement: SearchRestrictionDurationPlacement::Prefix,
        }));
    }

    if primitives::parse_prefix(tokens, as_long_as_marker).is_some() {
        if !as_long_as_you_control_source(tokens) {
            return Ok(None);
        }
        let Some(after) = comma_tail(tokens) else {
            return Err(CardTextError::ParseError(
                "missing comma after duration prefix".to_string(),
            ));
        };
        return Ok(Some(SearchRestrictionDurationShape {
            duration: Until::YouStopControllingThis,
            remainder: trim_lexed_commas(after).to_vec(),
            placement: SearchRestrictionDurationPlacement::Prefix,
        }));
    }

    if let Some(parsed) = leaf::parse_leaf_restriction_duration_suffix_tokens(tokens) {
        let remainder = trim_lexed_commas(parsed.rest).to_vec();
        if !remainder.is_empty() {
            return Ok(Some(SearchRestrictionDurationShape {
                duration: until_from_leaf(parsed.duration),
                remainder,
                placement: SearchRestrictionDurationPlacement::Suffix,
            }));
        }
    }

    if let Some((start, (), _)) = primitives::find_prefix(tokens, || as_long_as_marker) {
        let suffix = &tokens[start..];
        let duration = if as_long_as_source_remains_tapped(suffix) {
            Some(Until::SourceUntaps)
        } else if as_long_as_source_remains_on_battlefield(suffix) {
            Some(Until::ThisLeavesTheBattlefield)
        } else if as_long_as_you_control_source(suffix) {
            Some(Until::YouStopControllingThis)
        } else {
            None
        };
        if let Some(duration) = duration {
            return Ok(Some(SearchRestrictionDurationShape {
                duration,
                remainder: trim_lexed_commas(&tokens[..start]).to_vec(),
                placement: SearchRestrictionDurationPlacement::Suffix,
            }));
        }
    }

    if primitives::find_prefix(tokens, || primitives::phrase(&["this", "turn"])).is_some() {
        let cleaned = leaf::strip_leaf_this_turn_tokens(tokens);
        let remainder = trim_lexed_commas(&cleaned).to_vec();
        if !remainder.is_empty() {
            return Ok(Some(SearchRestrictionDurationShape {
                duration: Until::EndOfTurn,
                remainder,
                placement: SearchRestrictionDurationPlacement::Suffix,
            }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex_line;

    use super::*;

    #[test]
    fn parses_conditional_search_restriction_durations() {
        let tokens = lex_line(
            "for as long as you control this artifact, you may cast that card",
            0,
        )
        .unwrap();
        let parsed = parse_search_restriction_duration_shape_lexed(&tokens)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.duration, Until::YouStopControllingThis);
        assert_eq!(parsed.placement, SearchRestrictionDurationPlacement::Prefix);
        assert!(!parsed.remainder.is_empty());

        let tokens = lex_line("you may play it this turn", 0).unwrap();
        let parsed = parse_search_restriction_duration_shape_lexed(&tokens)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.duration, Until::EndOfTurn);
        assert_eq!(parsed.placement, SearchRestrictionDurationPlacement::Suffix);
    }

    #[test]
    fn distinguishes_leading_and_trailing_animation_durations() {
        let leading = lex_line("Until end of turn, target land becomes a 4/4 creature", 0).unwrap();
        let trailing = lex_line(
            "target artifact becomes an artifact creature for as long as this creature remains on the battlefield",
            0,
        )
        .unwrap();

        assert_eq!(
            parse_search_restriction_duration_shape_lexed(&leading)
                .unwrap()
                .unwrap()
                .placement,
            SearchRestrictionDurationPlacement::Prefix
        );
        assert_eq!(
            parse_search_restriction_duration_shape_lexed(&trailing)
                .unwrap()
                .unwrap()
                .placement,
            SearchRestrictionDurationPlacement::Suffix
        );
    }
}
