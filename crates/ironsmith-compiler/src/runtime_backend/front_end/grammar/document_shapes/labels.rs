use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::primitives;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreservedKeywordLabelKind {
    CostOrCasting,
    Activated,
    Triggered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabelPrefixKind {
    PreservedKeyword(PreservedKeywordLabelKind),
    CouncilChoice,
}

pub(crate) fn parse_label_prefix_kind_tokens(tokens: &[OwnedLexToken]) -> Option<LabelPrefixKind> {
    primitives::parse_prefix(tokens, council_choice_label)
        .map(|((), _)| LabelPrefixKind::CouncilChoice)
        .or_else(|| {
            primitives::parse_prefix(tokens, preserved_keyword_label)
                .map(|(kind, _)| LabelPrefixKind::PreservedKeyword(kind))
        })
}

pub(crate) fn parse_preserved_keyword_label_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PreservedKeywordLabelKind> {
    match parse_label_prefix_kind_tokens(tokens)? {
        LabelPrefixKind::PreservedKeyword(kind) => Some(kind),
        LabelPrefixKind::CouncilChoice => None,
    }
}

fn council_choice_label(input: &mut LexStream<'_>) -> WResult<()> {
    winnow::combinator::alt((
        primitives::phrase(&["will", "of", "the", "council"]),
        primitives::phrase(&["council's", "dilemma"]),
        primitives::phrase(&["secret", "council"]),
    ))
    .parse_next(input)
}

fn preserved_keyword_label(input: &mut LexStream<'_>) -> WResult<PreservedKeywordLabelKind> {
    let head = primitives::word_parser_text.parse_next(input)?;
    match head {
        "buyback" | "blitz" | "bestow" | "cumulative" | "cycling" | "echo" | "equip" | "epic"
        | "escape" | "escalate" | "eternalize" | "evoke" | "flashback" | "kicker"
        | "multikicker" | "modular" | "morph" | "megamorph" | "prototype" | "replicate"
        | "reinforce" | "squad" | "spectacle" | "strive" | "surge" | "suspend" | "ward" => {
            Ok(PreservedKeywordLabelKind::CostOrCasting)
        }
        "boast" | "renew" => Ok(PreservedKeywordLabelKind::Activated),
        _ => Err(primitives::backtrack_err(
            "keyword label",
            "known keyword label head",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn classifies_keyword_and_council_labels() {
        let kicker = lex_line("Kicker", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&kicker),
            Some(LabelPrefixKind::PreservedKeyword(
                PreservedKeywordLabelKind::CostOrCasting
            ))
        );
        let prototype = lex_line("Prototype {2}{R}", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&prototype),
            Some(LabelPrefixKind::PreservedKeyword(
                PreservedKeywordLabelKind::CostOrCasting
            ))
        );
        let council = lex_line("Council's dilemma", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&council),
            Some(LabelPrefixKind::CouncilChoice)
        );

        let council_with_dash = lex_line("Council's dilemma —", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&council_with_dash),
            Some(LabelPrefixKind::CouncilChoice)
        );
    }
}
