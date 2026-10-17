use crate::cards::builders::{AdditionalCostChoiceOptionAst, CardTextError};
use crate::clause_support::parse_effect_sentences_lexed;
use crate::effect_sentences::find_verb;
use crate::grammar::{permission_shapes, primitives};
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView};
use crate::model::ast::EffectAst;

pub fn parse_additional_cost_choices(
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
    // "choose a creature you control or a warped creature card you own in
    // exile" (Close Encounter): one "choose" governs both object phrases, so
    // a verbless later option inherits the first option's choosing verb.
    if normalized.len() >= 2
        && normalized[0].first().is_some_and(|token| token.is_word("choose"))
    {
        let verb = normalized[0][0].clone();
        for option in normalized.iter_mut().skip(1) {
            // A bare noun phrase ("a warped creature card you own in exile")
            // opens with its article; "exile" inside it is not its verb.
            if option.first().is_some_and(|token| token.is_word("a") || token.is_word("an")) {
                option.insert(0, verb.clone());
            }
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
        // "forage or pay {B}" (Feed the Cycle)
        || permission_shapes::exact_words(&words, &["forage"])
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
