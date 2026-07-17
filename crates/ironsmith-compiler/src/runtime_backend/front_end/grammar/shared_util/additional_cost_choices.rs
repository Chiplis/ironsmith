use crate::cards::builders::{AdditionalCostChoiceOptionAst, CardTextError};
use crate::runtime_backend::ast::EffectAst;
use crate::runtime_backend::clause_support::parse_effect_sentences_lexed;
use crate::runtime_backend::effect_sentences::find_verb;
use crate::runtime_backend::grammar::{permission_shapes, primitives};
use crate::runtime_backend::lexer::{OwnedLexToken, TokenKind, TokenWordView};

pub(crate) fn parse_additional_cost_choices(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst<EffectAst>>>, CardTextError> {
    let words = TokenWordView::new(tokens).word_refs();
    if permission_shapes::find_words(&words, &["one", "or", "more"]).is_some()
        || permission_shapes::find_words(&words, &["or"]).is_none()
    {
        return Ok(None);
    }
    let mut normalized = Vec::new();
    for part in primitives::split_lexed_slices_on_or(tokens) {
        let mut option = part.to_vec();
        while option.first().is_some_and(is_and_or) {
            option.remove(0);
        }
        trim_commas(&mut option);
        if !option.is_empty() {
            normalized.push(option);
        }
    }
    if normalized.len() < 2
        || normalized
            .iter()
            .any(|option| find_verb(option).is_none() && !is_verbless_keyword_cost_option(option))
    {
        return Ok(None);
    }

    let mut choices = Vec::with_capacity(normalized.len());
    for option in normalized {
        let effects = parse_effect_sentences_lexed(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                render_option_text(&option)
            )));
        }
        choices.push(AdditionalCostChoiceOptionAst {
            description: render_option_text(&option),
            effects,
        });
    }
    Ok((choices.len() >= 2).then_some(choices))
}

fn is_verbless_keyword_cost_option(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    permission_shapes::prefix_words(&words, &["behold"])
        || permission_shapes::prefix_words(&words, &["blight"])
}

fn is_and_or(token: &OwnedLexToken) -> bool {
    token.as_word().is_some_and(|word| {
        permission_shapes::exact_words(&[word], &["and"])
            || permission_shapes::exact_words(&[word], &["or"])
    })
}

fn trim_commas(tokens: &mut Vec<OwnedLexToken>) {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens.remove(0);
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens.pop();
    }
}

fn render_option_text(tokens: &[OwnedLexToken]) -> String {
    tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Comma | TokenKind::Period))
        .map(OwnedLexToken::parser_text)
        .collect::<Vec<_>>()
        .join(" ")
}
