use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::primitives;
use crate::ir::ChosenOptionContext;
use crate::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaseLabelKind {
    ToSolve,
    Solved,
}

pub(crate) fn parse_case_label_tokens(tokens: &[OwnedLexToken]) -> Option<CaseLabelKind> {
    primitives::parse_all(tokens, case_label, "case-label").ok()
}

pub(crate) fn parse_chosen_option_context_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChosenOptionContext> {
    primitives::parse_all(tokens, chosen_option_context, "chosen-option-label").ok()
}

fn case_label(input: &mut LexStream<'_>) -> WResult<CaseLabelKind> {
    winnow::combinator::alt((
        primitives::phrase(&["to", "solve"]).value(CaseLabelKind::ToSolve),
        primitives::kw("solved").value(CaseLabelKind::Solved),
    ))
    .parse_next(input)
}

fn chosen_option_context(input: &mut LexStream<'_>) -> WResult<ChosenOptionContext> {
    let words = winnow::combinator::repeat(1.., primitives::word_parser_text)
        .fold(String::new, |mut label, word| {
            if !label.is_empty() {
                label.push(' ');
            }
            label.push_str(word);
            label
        })
        .parse_next(input)?;
    Ok(ChosenOptionContext::source_option(words))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn parses_case_labels_and_chosen_options_as_typed_variants() {
        let to_solve = lex_line("To solve", 0).unwrap();
        assert_eq!(
            parse_case_label_tokens(&to_solve),
            Some(CaseLabelKind::ToSolve)
        );
        let solved = lex_line("Solved", 0).unwrap();
        assert_eq!(
            parse_case_label_tokens(&solved),
            Some(CaseLabelKind::Solved)
        );

        let choice = lex_line("Khans", 0).unwrap();
        assert_eq!(
            parse_chosen_option_context_tokens(&choice),
            Some(ChosenOptionContext::SourceOption("khans".to_string()))
        );
    }
}
