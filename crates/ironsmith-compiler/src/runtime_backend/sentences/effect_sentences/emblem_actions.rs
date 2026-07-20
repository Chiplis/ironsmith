use super::super::clause_dispatch;
use super::*;

use crate::runtime_backend::activation_and_restrictions::parse_activated_line;
use crate::runtime_backend::ast::{EmblemAbilityAst, EmblemDescriptionAst};
use crate::runtime_backend::clause_support::{
    parse_static_ability_ast_line_lexed, parse_triggered_line_lexed,
};
use crate::runtime_backend::grammar::effects::emblem_shapes;
use crate::runtime_backend::grammar::{
    activated_lines, clause_support as clause_grammar, trigger_surface,
};
use crate::runtime_backend::lexer::{render_token_slice, split_lexed_sentences};
use crate::runtime_backend::semantic::LineAst;

fn parse_emblem_ability_tokens(tokens: &[OwnedLexToken]) -> Option<EmblemAbilityAst> {
    let tokens = tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Quote)
        .then(|| &tokens[1..])
        .unwrap_or(tokens);
    let tokens = tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Quote)
        .then(|| &tokens[..tokens.len().saturating_sub(1)])
        .unwrap_or(tokens);
    if clause_grammar::parse_trigger_intro_tokens(tokens).body_first > 0
        && let Ok(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        }) = parse_triggered_line_lexed(tokens)
    {
        return Some(EmblemAbilityAst::Triggered {
            trigger,
            effects,
            trigger_limit_condition: trigger_surface::parse_trigger_frequency_condition_tokens(
                tokens,
                max_triggers_per_turn,
            ),
        });
    }

    if activated_lines::parse_activated_line_split_tokens(tokens).is_some()
        && let Ok(Some(ability)) = parse_activated_line(tokens)
    {
        return Some(EmblemAbilityAst::Activated(ability));
    }

    parse_static_ability_ast_line_lexed(tokens)
        .ok()
        .flatten()
        .filter(|abilities| !abilities.is_empty())
        .map(EmblemAbilityAst::Static)
}

fn parse_emblem_ability_group(tokens: &[OwnedLexToken]) -> Vec<EmblemAbilityAst> {
    let sentences = split_lexed_sentences(tokens);
    let first_is_triggered_or_activated = sentences.first().is_some_and(|sentence| {
        clause_grammar::parse_trigger_intro_tokens(sentence).body_first > 0
            || activated_lines::parse_activated_line_split_tokens(sentence).is_some()
    });
    let later_starts_nested_ability = sentences.iter().skip(1).any(|sentence| {
        clause_grammar::parse_trigger_intro_tokens(sentence).body_first > 0
            || activated_lines::parse_activated_line_split_tokens(sentence).is_some()
    });

    if (sentences.len() <= 1 || first_is_triggered_or_activated)
        && !later_starts_nested_ability
        && let Some(ability) = parse_emblem_ability_tokens(tokens)
    {
        return vec![ability];
    }

    sentences
        .into_iter()
        .filter_map(parse_emblem_ability_tokens)
        .collect()
}

fn emblem_group_presentation(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens).trim().to_string();
    if tokens.last().is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Period | TokenKind::Bang | TokenKind::Question
        )
    }) {
        rendered
    } else {
        format!("{rendered}.")
    }
}

fn parse_emblem_description_ast(
    shape: emblem_shapes::EmblemPayloadShape<'_>,
) -> EmblemDescriptionAst {
    let text = shape
        .ability_groups
        .iter()
        .map(|tokens| emblem_group_presentation(tokens))
        .collect::<Vec<_>>()
        .join("\n");
    let abilities = shape
        .ability_groups
        .into_iter()
        .flat_map(parse_emblem_ability_group)
        .collect();
    EmblemDescriptionAst { text, abilities }
}

pub(crate) fn parse_emblem_action(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let shape = emblem_shapes::parse_emblem_payload_tokens(tokens)?;
    let subject = subject.or_else(|| {
        shape
            .explicit_you
            .then_some(SubjectAst::Player(PlayerAst::You))
    });
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    Some(EffectAst::subject_verb_create_emblem(
        player,
        parse_emblem_description_ast(shape),
    ))
}

/// Parse a quoted emblem followed by an ordinary effect in the same Oracle
/// sentence, such as `You get an emblem with "...", then create ...`.
/// Keeping this boundary at the whole-sentence level prevents commas and
/// sentence punctuation inside the quoted ability from being treated as
/// outer effect separators.
pub(crate) fn parse_quoted_emblem_then_action(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let open_quote = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Quote)?;
    let close_quote = tokens
        .iter()
        .enumerate()
        .skip(open_quote + 1)
        .find_map(|(index, token)| (token.kind == TokenKind::Quote).then_some(index))?;
    let after_quote = tokens.get(close_quote + 1..).unwrap_or_default();
    let then_offset = after_quote.iter().position(|token| token.is_word("then"))?;
    if after_quote[..then_offset]
        .iter()
        .any(|token| !matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        return None;
    }
    let emblem = parse_emblem_action(&tokens[..=close_quote], None)?;
    let trailing = crate::runtime_backend::lexer::trim_lexed_commas(
        after_quote.get(then_offset + 1..).unwrap_or_default(),
    );
    if trailing.is_empty() {
        return None;
    }
    let mut effects = vec![emblem];
    effects.extend(
        crate::runtime_backend::sentences::effect_sentences::parse_effect_chain_lexed(&trailing)
            .ok()?,
    );
    (effects.len() > 1).then_some(EffectAst::Sequence { effects })
}

/// Subject/verb dispatch normally passes only the action tail to `parse_get`.
/// That path has already consumed the quote tokens which delimit an emblem's
/// ability text, so retain a narrow fallback for the resulting `an emblem
/// with ...` shape. The whole-sentence parser remains authoritative whenever
/// the quotes are still present.
pub(crate) fn parse_unquoted_emblem_action(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words.starts_with(&["an", "emblem", "with"]) || tokens.len() <= 3 {
        return None;
    }
    let payload_tokens = tokens.get(3..)?;
    let trailing_then = payload_tokens
        .iter()
        .enumerate()
        .find_map(|(index, token)| {
            if !token.is_word("then") {
                return None;
            }
            let previous = payload_tokens.get(index.saturating_sub(1))?;
            matches!(previous.kind, TokenKind::Comma | TokenKind::Period).then_some(index)
        });
    let (ability_tokens, trailing_tokens) = trailing_then
        .map(|index| {
            (
                &payload_tokens[..index.saturating_sub(1)],
                Some(crate::runtime_backend::lexer::trim_lexed_commas(
                    payload_tokens.get(index + 1..).unwrap_or_default(),
                )),
            )
        })
        .unwrap_or((payload_tokens, None));
    let shape = emblem_shapes::EmblemPayloadShape {
        explicit_you: false,
        ability_groups: vec![ability_tokens],
        requires_whole_sentence_dispatch: false,
    };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let emblem = EffectAst::subject_verb_create_emblem(player, parse_emblem_description_ast(shape));
    let Some(trailing_tokens) = trailing_tokens else {
        return Some(emblem);
    };
    let trailing = clause_dispatch::parse_effect_clause_lexed(trailing_tokens).ok()?;
    Some(EffectAst::Sequence {
        effects: vec![emblem, trailing],
    })
}
