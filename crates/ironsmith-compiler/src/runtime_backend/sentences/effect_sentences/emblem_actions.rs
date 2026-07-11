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
