//! The readings of one static ability line before it is split into sentences:
//! the compound shapes that span a sentence boundary or that the broad
//! keyword-line grammar would misread ("pay life or enter tapped", the
//! conditional anthem/grant lines with an "otherwise" sentence, the carried
//! attached-subject lines, protection's attachment exception, ...). Formerly a
//! first-match ladder in `keyword_static`; every reading runs, resolved by rank
//! while the overlaps are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct StaticLine<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl StaticLine<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<Vec<StaticAbilityAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<StaticAbilityAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("static-compound-line-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&StaticLine<'_>) -> bool,
    read: fn(&StaticLine<'_>) -> ParseOutcome<Vec<StaticAbilityAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("static-compound-line-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("once-each-turn-paid-die-reroll-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_once_each_turn_paid_die_reroll_line(input)),
    },
    Reading {
        id: RuleId::new("attached-anthem-reach-shadow-permission-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_attached_anthem_reach_shadow_permission_line(input)),
    },
    Reading {
        id: RuleId::new("pay-life-or-enter-tapped-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_pay_life_or_enter_tapped_line(input)),
    },
    Reading {
        id: RuleId::new("first-spell-cost-reduction-and-flash-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_first_spell_cost_reduction_and_flash_line(input)),
    },
    Reading {
        id: RuleId::new("source-graveyard-dynamic-surcharge-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_source_graveyard_dynamic_surcharge_line(input)),
    },
    Reading {
        id: RuleId::new("pregame-reveal-from-opening-hand-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_pregame_reveal_from_opening_hand_line(input)),
    },
    Reading {
        id: RuleId::new("enter-as-copy-as-enters-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_enter_as_copy_as_enters_line(input)),
    },
    Reading {
        id: RuleId::new("draw-replacement-reveal-top-matching-to-hand-rest-bottom-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| {
            input.outcome(read_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(input))
        },
    },
    Reading {
        id: RuleId::new("removed-draft-leading-conditional-static-sentence-chain"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| {
            input.outcome(read_removed_draft_leading_conditional_static_sentence_chain(input))
        },
    },
    Reading {
        id: RuleId::new("independent-leading-conditional-static-sentence-chain"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| {
            input.outcome(read_independent_leading_conditional_static_sentence_chain(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("attached-conditional-keyword-otherwise-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_attached_conditional_keyword_otherwise_line(input)),
    },
    Reading {
        id: RuleId::new("attached-conditional-anthem-otherwise-base-and-restriction-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| {
            input.outcome(
                read_attached_conditional_anthem_otherwise_base_and_restriction_line(input),
            )
        },
    },
    Reading {
        id: RuleId::new("attached-conditional-loses-all-abilities-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_attached_conditional_loses_all_abilities_line(input)),
    },
    Reading {
        id: RuleId::new("conditional-anthem-replacement-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_conditional_anthem_replacement_line(input)),
    },
    Reading {
        id: RuleId::new("conditional-anthem-otherwise-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_conditional_anthem_otherwise_line(input)),
    },
    Reading {
        id: RuleId::new("carried-conditional-anthem-grant-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_carried_conditional_anthem_grant_line(input)),
    },
    Reading {
        id: RuleId::new("carried-subject-type-addition-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_carried_subject_type_addition_line(input)),
    },
    Reading {
        id: RuleId::new("carried-attached-subject-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_carried_attached_subject_line(input)),
    },
    Reading {
        id: RuleId::new("players-cant-search-with-any-player-ignore-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_players_cant_search_with_any_player_ignore_line(input)),
    },
    Reading {
        id: RuleId::new("attached-restrictions-with-ignore-special-action-line"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| {
            input.outcome(read_attached_restrictions_with_ignore_special_action_line(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("protection-attachment-exception"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_protection_attachment_exception(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &StaticLine<'_>) -> ParseOutcome<RuleMatch<Vec<StaticAbilityAst>>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<Vec<StaticAbilityAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_once_each_turn_paid_die_reroll_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_once_each_turn_paid_die_reroll_line(tokens) {
        return Ok(Some(vec![ability]));
    }
    Ok(None)
}
fn read_attached_anthem_reach_shadow_permission_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_attached_anthem_reach_shadow_permission_line(tokens) {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_pay_life_or_enter_tapped_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // Pay-life ETB replacements span two sentences. Parse the complete
    // compound before the generic sentence splitter can reinterpret the
    // "if you don't, it enters tapped" suffix as a standalone static line.
    if let Some(ability) = parse_pay_life_or_enter_tapped_line(tokens)? {
        return Ok(Some(vec![StaticAbilityAst::Static(ability)]));
    }
    Ok(None)
}
fn read_first_spell_cost_reduction_and_flash_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_first_spell_cost_reduction_and_flash_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_source_graveyard_dynamic_surcharge_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_source_graveyard_dynamic_surcharge_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_pregame_reveal_from_opening_hand_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_pregame_reveal_from_opening_hand_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    Ok(None)
}
fn read_enter_as_copy_as_enters_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // Compound "as this enters ... if you do ..." replacement text is one
    // semantic unit even though it contains a sentence boundary.
    if let Some(ability) = parse_enter_as_copy_as_enters_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) =
        parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(tokens)?
    {
        return Ok(Some(vec![StaticAbilityAst::from(ability)]));
    }
    Ok(None)
}
fn read_removed_draft_leading_conditional_static_sentence_chain(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // Borrow preprocessing expands "The same is true" into one independent
    // leading `If` sentence per keyword. Keep that complete typed chain ahead
    // of broad compound anthem/grant families, which can otherwise consume
    // the condition words as part of an affected-object filter.
    if let Some(abilities) = parse_removed_draft_leading_conditional_static_sentence_chain(tokens)?
    {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_independent_leading_conditional_static_sentence_chain(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) =
            leading_conditional_sentence_chain::parse_independent_leading_conditional_static_sentence_chain(
                tokens,
            )
        {
            return Ok(Some(abilities));
        }
    Ok(None)
}
fn read_attached_conditional_keyword_otherwise_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_attached_conditional_keyword_otherwise_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_attached_conditional_anthem_otherwise_base_and_restriction_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) =
        parse_attached_conditional_anthem_otherwise_base_and_restriction_line(tokens)?
    {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_attached_conditional_loses_all_abilities_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // Keep attachment-relative conditions attached to their affected object.
    // The broad `red creatures lose all abilities` family otherwise drops the
    // Aura/Equipment relationship and turns the rule into a global effect.
    if let Some(abilities) = parse_attached_conditional_loses_all_abilities_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_conditional_anthem_replacement_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_conditional_anthem_replacement_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_conditional_anthem_otherwise_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_conditional_anthem_otherwise_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_carried_conditional_anthem_grant_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_carried_conditional_anthem_grant_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_carried_subject_type_addition_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_carried_subject_type_addition_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_carried_attached_subject_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_carried_attached_subject_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_players_cant_search_with_any_player_ignore_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // "Any player may pay ... to ignore this effect" grants a priority
    // special action; it is not a one-shot spell effect. Keep both sentences
    // together so the permission remains linked to the source restriction.
    if let Some(abilities) = parse_players_cant_search_with_any_player_ignore_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_attached_restrictions_with_ignore_special_action_line(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // The attached object's controller receives a turn-scoped special action,
    // not a one-shot choice as this Aura resolves. Keep the complete
    // two-sentence rule together before ordinary sentence splitting can lower
    // the restriction and sacrifice permission as spell effects.
    if let Some(abilities) = parse_attached_restrictions_with_ignore_special_action_line(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}
fn read_protection_attachment_exception(
    input: &StaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = split_lexed_sentences(tokens);
    let words = parser_token_word_refs(tokens);
    if sentences.len() == 2
        && crate::word_primitives::any_sequence_occurs(
            &words,
            &[
                &[
                    "this",
                    "effect",
                    "doesn't",
                    "remove",
                    "auras",
                    "and",
                    "equipment",
                    "you",
                    "control",
                    "that",
                    "are",
                    "already",
                    "attached",
                    "to",
                    "it",
                ],
                &[
                    "this",
                    "effect",
                    "doesnt",
                    "remove",
                    "auras",
                    "and",
                    "equipment",
                    "you",
                    "control",
                    "that",
                    "are",
                    "already",
                    "attached",
                    "to",
                    "it",
                ],
            ],
        )
        && let Some(mut abilities) = parse_enchanted_creature_has_line(sentences[0])?
    {
        for ability in &mut abilities {
            if let StaticAbilityAst::AttachedKeywordActionGrant {
                action: KeywordAction::ProtectionFromChosenColor,
                display,
                protection_does_not_remove_controlled_attachments,
                ..
            } = ability
            {
                *protection_does_not_remove_controlled_attachments = true;
                display.push_str(". This effect doesn't remove Auras and Equipment you control that are already attached to it");
            }
        }
        return Ok(Some(abilities));
    }
    Ok(None)
}
/// A decline the ladder made before the readings ranked after it.
fn declines_1(input: &StaticLine<'_>) -> bool {
    let tokens = input.tokens;
    let mut inside_quote = false;
    let starts_executable_keyword_clause = tokens.iter().enumerate().any(|(index, token)| {
        if token.kind == TokenKind::Quote {
            inside_quote = !inside_quote;
            return false;
        }
        let pieces = token.parser_word_pieces();
        !inside_quote
            && pieces.len() == 1
            && pieces
                .first()
                .is_some_and(|piece| matches!(piece.text.as_str(), "bolster" | "adapt"))
            && (index == 0
                || tokens.get(index - 1).is_some_and(|previous| {
                    matches!(
                        previous.kind,
                        TokenKind::Comma | TokenKind::Colon | TokenKind::Period
                    ) || previous.is_word("then")
                }))
    });
    starts_executable_keyword_clause
}

/// Whether any decline the ladder made before its fallback holds: the
/// fallback ran only when none did.
pub(super) fn declines(input: &StaticLine<'_>) -> bool {
    declines_1(input)
}
