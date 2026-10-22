use super::super::clause_dispatch;
use super::*;

use crate::activation_and_restrictions::parse_activated_line;
use crate::clause_support::{parse_static_ability_ast_line_lexed, parse_triggered_line_lexed};
use crate::grammar::effects::emblem_shapes;
use crate::grammar::{activated_lines, clause_support as clause_grammar, trigger_surface};
use crate::lexer::{render_token_slice, split_lexed_sentences};
use crate::model::ast::{EmblemAbilityAst, EmblemDescriptionAst, StaticAbilityAst};
use crate::model::compiler_semantic::LineAst;

#[inline(never)]
fn parse_complete_typed_emblem_trigger(tokens: &[OwnedLexToken]) -> Option<EmblemAbilityAst> {
    let intro = clause_grammar::parse_trigger_intro_tokens(tokens);
    let split_idx = clause_grammar::parse_trigger_delimiters_tokens(tokens).first_comma?;
    if intro.body_first == 0 || split_idx <= intro.body_first || split_idx + 1 >= tokens.len() {
        return None;
    }
    let trigger = crate::grammar::primitives::probe_shape(
        crate::clause_support::parse_trigger_clause_lexed(&tokens[intro.body_first..split_idx]),
    )?;
    let effect_tokens = crate::lexer::trim_lexed_commas(&tokens[split_idx + 1..]);
    if effect_tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Period)
        .count()
        > 1
    {
        return None;
    }
    let effects = crate::grammar::primitives::probe_shape(
        crate::effect_sentences::parse_effect_sentences_lexed(effect_tokens),
    )?;
    if effects.is_empty() {
        return None;
    }
    Some(EmblemAbilityAst::Triggered {
        trigger,
        effects,
        trigger_limit_condition: trigger_surface::parse_trigger_frequency_condition_tokens(
            tokens, None,
        ),
    })
}

#[path = "emblem_actions/emblem_ability_readings.rs"]
mod emblem_ability_readings;

fn parse_emblem_ability_tokens(tokens: &[OwnedLexToken]) -> Option<EmblemAbilityAst> {
    let tokens = if tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Quote)
    {
        &tokens[1..]
    } else {
        tokens
    };
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Quote)
    {
        &tokens[..tokens.len().saturating_sub(1)]
    } else {
        tokens
    };
    // A rule nested inside the emblem's quoted text is delimited by
    // apostrophes ("... have haste and '{T}: This creature deals damage
    // ...'", Arlinn, Embraced by the Moon); read them as the quotes they are.
    let normalized;
    let tokens = if tokens.iter().any(|token| token.kind == TokenKind::Apostrophe) {
        normalized = tokens
            .iter()
            .map(|token| {
                if token.kind == TokenKind::Apostrophe {
                    OwnedLexToken::new(TokenKind::Quote, "\"", token.span())
                } else {
                    token.clone()
                }
            })
            .collect::<Vec<_>>();
        normalized.as_slice()
    } else {
        tokens
    };
    // "You draw a card during each opponent's draw step." (Teferi, Who
    // Slows the Sunset's emblem): an extra draw for you in each opponent's
    // draw step, read as the draw-step trigger with that effect.
    if matches!(
        crate::lexer::token_word_refs(tokens).as_slice(),
        ["you", "draw", "a", "card", "during", "each", "opponent's" | "opponents", "draw", "step"]
    ) && let Ok(trigger_tokens) =
        crate::lexer::lex_line("At the beginning of each opponent's draw step, you draw a card.", 0)
    {
        return parse_complete_typed_emblem_trigger(&trigger_tokens);
    }
    let input = emblem_ability_readings::EmblemAbility { tokens };
    match emblem_ability_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Some(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(_) => return None,
    }
    crate::grammar::primitives::probe_shape(parse_static_ability_ast_line_lexed(tokens))
        .flatten()
        .filter(|abilities| !abilities.is_empty())
        .or_else(|| {
            // Typed static lines the broad grammar leaves to the line-level
            // readings: "Artifact spells you cast cost {1} less to cast."
            // (Saheeli, Filigree Master's emblem), "Your opponents can't untap
            // more than two permanents during their untap steps." (Dovin Baan)
            crate::keyword_static::parse_spells_cost_modifier_line(tokens)
                .ok()
                .flatten()
                .or_else(|| {
                    crate::keyword_static::parse_untap_step_limit_line(tokens).ok().flatten()
                })
                .map(|ability| vec![StaticAbilityAst::Static(ability)])
        })
        .map(|mut abilities| {
            // An emblem has no exiling abilities of its own to pair with: its
            // "cards exiled with <creator>" pool is the creating object's
            // live links, bound when the emblem is created (CR 607.2a).
            for ability in &mut abilities {
                if let StaticAbilityAst::Static(ability) = ability
                    && let ironsmith_core::StaticAbilityPayload::Grants(spec) = &mut ability.payload
                {
                    spec.requires_linked_exile_pair = false;
                }
            }
            EmblemAbilityAst::Static(abilities)
        })
}

/// Every ability of one quoted group, or `None` when any part of the quoted
/// text yields no ability: an emblem must never silently lose its text.
fn parse_emblem_ability_group(tokens: &[OwnedLexToken]) -> Option<Vec<EmblemAbilityAst>> {
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
        return Some(vec![ability]);
    }

    let abilities = sentences
        .into_iter()
        .filter(|sentence| sentence.iter().any(|token| token.as_word().is_some()))
        .map(parse_emblem_ability_tokens)
        .collect::<Option<Vec<_>>>()?;
    (!abilities.is_empty()).then_some(abilities)
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
) -> Option<EmblemDescriptionAst> {
    let text = shape
        .ability_groups
        .iter()
        .map(|tokens| emblem_group_presentation(tokens))
        .collect::<Vec<_>>()
        .join("\n");
    let mut abilities = Vec::new();
    for group in shape.ability_groups {
        let Some(parsed) = parse_emblem_ability_group(group) else {
            crate::parse_loss::record(
                "emblem_ability_unparsed",
                format!("emblem ability text produced no ability: '{}'", render_token_slice(group).trim()),
            );
            return None;
        };
        abilities.extend(parsed);
    }
    Some(EmblemDescriptionAst { text, abilities })
}

#[path = "emblem_actions/core.rs"]
mod core_programs;
pub use core_programs::{
    parse_emblem_action, parse_quoted_emblem_then_action, parse_unquoted_emblem_action,
};
