//! The document-level readings tried before the sentence loop: whole-document
//! shapes ("Choose one or more —", the fight programs, the flat statement
//! split, ...) and the single-sentence statements read complete. They were a
//! first-match ladder in `dispatch_entry`; they are a registry whose rules all
//! run on every document and must agree: the order they are written in no
//! longer defines the language.

use crate::word_primitives::WordWindows as _;
use super::SubjectVerbPrimitiveClause;
use super::dispatch_entry::{
    SentenceInput, parse_complete_become_statement, parse_complete_composable_fight_program,
    parse_complete_compound_gain_statement, parse_complete_create_statement,
    parse_complete_delegated_partition_program, parse_complete_get_pump_statement,
    parse_complete_investigate_statement, parse_complete_simple_subject_verb_sentence,
    parse_delegated_partition_program_prefix, parse_each_player_coin_face_sequence,
    parse_resolving_card_countered_exile_replacement, parse_single_put_counters_effect_chain,
    parse_temporary_per_blocker_tax,
};
use super::dispatch_inner::trim_edge_punctuation;
use super::divvy::try_parse_divvy_sentence_sequence;
use super::sentence_helpers::*;
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, ForEachEffectAst, IfResultPredicate,
    PermissionEffectAst,
};
use crate::grammar::effects as effect_grammar;
use crate::lexer::{OwnedLexToken, TokenKind, split_lexed_sentences};
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// A document handed to the readings: its tokens and its sentences.
pub(super) struct Document<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) sentences: Vec<&'a [OwnedLexToken]>,
}

impl<'a> Document<'a> {
    pub(super) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self {
            tokens,
            sentences: split_lexed_sentences(tokens),
        }
    }

    /// A reading's outcome: its error is a committed diagnostic on the document.
    fn outcome(
        &self,
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        match read {
            Ok(Some(effects)) => {
                ParseOutcome::matched(effects, crate::util::span_from_tokens(self.tokens))
            }
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("document-reading"),
                crate::util::span_from_tokens(self.tokens),
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    read: fn(&Document<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const DOCUMENT_REGISTRY: RuleId = RuleId::new("document-reading-registry");

/// The readings, in the order they were ranked.
const DOCUMENT_READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("chosen-color-or-type-protection-program"), head: HeadDiscriminator::Words(&["choose"]),
        read: |document| document.outcome(read_chosen_color_or_type_protection(document)),
    },
    Reading {
        id: RuleId::new("bounded-instruction-program"), head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_bounded_instruction_program(document)),
    },
    Reading {
        id: RuleId::new("complete-comma-filtered-play-permission"), head: HeadDiscriminator::Words(&["you", "may", "cast"]),
        read: |document| document.outcome(read_comma_filtered_play_permission(document)),
    },
    Reading {
        id: RuleId::new("explicit-resolution-instruction-menu"), head: HeadDiscriminator::Words(&["choose", "one", "instruction"]),
        read: |document| document.outcome(crate::instruction_menu::parse(document.tokens)),
    },
    Reading {
        id: RuleId::new("explicit-status-instruction-program"), head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_status_instruction_program(document)),
    },
    Reading {
        id: RuleId::new("imperative-participant-program"), head: HeadDiscriminator::Words(&["for", "each"]),
        read: |document| document.outcome(read_imperative_participant_program(document)),
    },
    Reading {
        id: RuleId::new("value-bound-following-program"), head: HeadDiscriminator::Words(&["with", "x"]),
        read: |document| document.outcome(read_value_bound_following_program(document)),
    },
    Reading {
        id: RuleId::new("leading-sentences-value-bound-program"), head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_leading_sentences_value_bound_program(document)),
    },
    Reading {
        id: RuleId::new("selected-card-copy-free-cast"), head: HeadDiscriminator::Words(&["copy"]),
        read: |document| document.outcome(read_selected_card_copy_free_cast(document)),
    },
    Reading {
        id: RuleId::new("explicit-optional-cost-program"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_explicit_optional_cost_program(document)),
    },
    Reading {
        id: RuleId::new("bounded-inline-modal-program"),
        // "choose ...", "you may choose ...", "its owner chooses ...".
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_bounded_inline_modal(document)),
    },
    Reading {
        id: RuleId::new("declared-any-target-program"),
        head: HeadDiscriminator::Words(&["choose"]),
        read: |document| document.outcome(super::declared_any_target::parse(document.tokens)),
    },
    Reading {
        id: RuleId::new("resolving-card-countered-exile-replacement"),
        head: HeadDiscriminator::Any,
        read: |document| {
            document.outcome(read_resolving_card_countered_exile_replacement(document))
        },
    },
    Reading {
        id: RuleId::new("temporary-per-blocker-tax"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_temporary_per_blocker_tax(document)),
    },
    Reading {
        id: RuleId::new("choice-complement"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_choice_complement(document)),
    },
    Reading {
        id: RuleId::new("exile-then-return-same-object"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_exile_then_return_same_object(document)),
    },
    Reading {
        id: RuleId::new("reveal-source-exiled-permanents"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_reveal_source_exiled_permanents(document)),
    },
    Reading {
        id: RuleId::new("each-player-may-discard-hand-and-draw"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_each_player_may_discard_hand_and_draw(document)),
    },
    Reading {
        id: RuleId::new("conditional-put-counters"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_conditional_put_counters(document)),
    },
    Reading {
        id: RuleId::new("foreach-following-program"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_foreach_following_program(document)),
    },
    Reading {
        id: RuleId::new("result-following-program"), head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_result_following_program(document)),
    },
    Reading {
        id: RuleId::new("optional-following-program"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_optional_following_program(document)),
    },
    Reading {
        id: RuleId::new("first-shared-resolution-program"), head: HeadDiscriminator::Words(&["resolve"]),
        read: |document| document.outcome(read_first_shared_resolution_program(document)),
    },
    Reading {
        id: RuleId::new("named-action-program"), head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_named_action_program(document)),
    },
    Reading {
        id: RuleId::new("repeat-following-process"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_repeat_following_process(document)),
    },
    Reading {
        id: RuleId::new("emblem-payload"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_emblem_payload(document)),
    },
    Reading {
        id: RuleId::new("complete-create"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_create(document)),
    },
    Reading {
        id: RuleId::new("complete-investigate"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_investigate(document)),
    },
    Reading {
        id: RuleId::new("complete-kicked-search-replacement-bundle"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_kicked_search_replacement_bundle(document)),
    },
    Reading {
        id: RuleId::new("complete-delegated-partition"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_delegated_partition(document)),
    },
    Reading {
        id: RuleId::new("each-player-coin-face"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_each_player_coin_face(document)),
    },
    Reading {
        id: RuleId::new("divvy"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_divvy(document)),
    },
    Reading {
        id: RuleId::new("delegated-partition-program-prefix"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_delegated_partition_program_prefix(document)),
    },
    Reading {
        id: RuleId::new("complete-composable-fight"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_composable_fight(document)),
    },
    Reading {
        id: RuleId::new("cant-effect"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_cant_effect(document)),
    },
    Reading {
        id: RuleId::new("serial-target-pt-modifiers"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_serial_target_pt_modifiers(document)),
    },
    Reading {
        id: RuleId::new("compound-damage-fanout"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_compound_damage_fanout(document)),
    },
    Reading {
        id: RuleId::new("complete-compound-gain"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_compound_gain(document)),
    },
    Reading {
        id: RuleId::new("keyword-bundle-pump"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_keyword_bundle_pump(document)),
    },
    Reading {
        id: RuleId::new("effect-clause-with-trailing-if"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_effect_clause_with_trailing_if(document)),
    },
    Reading {
        // "<effect> unless a player has this creature deal N damage to
        // them" (Barbarian Bully): an any-player punisher choice.
        id: RuleId::new("unless-any-player-has-source-deal-damage"),
        head: HeadDiscriminator::Any,
        read: |document| {
            document.outcome(match document.sentences.as_slice() {
                [sentence] if crate::lexer::parser_token_word_refs(sentence)
                    .word_windows(3)
                    .any(|words| words == ["unless", "a", "player"] || words == ["unless", "any", "player"]) =>
                {
                    super::subject_verb_primitives::parse_sentence_damage_unless_controller_has_source_deal_damage(
                        super::subject_verb_primitives::SubjectVerbPrimitiveClause::new(sentence),
                    )
                }
                _ => Ok(None),
            })
        },
    },
    Reading {
        id: RuleId::new("complete-get-pump"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_get_pump(document)),
    },
    Reading {
        id: RuleId::new("unblock-sole-blocked-attackers"),
        head: HeadDiscriminator::Any,
        read: |document| {
            document.outcome(Ok(match document.sentences.as_slice() {
                [sentence] => super::statement_readings::parse_unblock_sole_blocked_attackers_sentence(
                    sentence,
                ),
                _ => None,
            }))
        },
    },
    Reading {
        id: RuleId::new("complete-become"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_become(document)),
    },
    Reading {
        id: RuleId::new("sentence-you-and-attacking-player-each-draw-and-lose"),
        head: HeadDiscriminator::Any,
        read: |document| {
            document.outcome(read_sentence_you_and_attacking_player_each_draw_and_lose(
                document,
            ))
        },
    },
    Reading {
        id: RuleId::new("sentence-each-opponent-draws-then-you-draw-per-opponent"),
        head: HeadDiscriminator::Any,
        read: |document| {
            document.outcome(read_sentence_each_opponent_draws_then_you_draw_per_opponent(document))
        },
    },
    Reading {
        id: RuleId::new("each-opponent-attacking-that-player-does-the-same"),
        head: HeadDiscriminator::Any,
        read: |document| {
            document.outcome(read_each_opponent_attacking_that_player_does_the_same(
                document,
            ))
        },
    },
    Reading {
        id: RuleId::new("next-end-step-followups"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_next_end_step_followups(document)),
    },
    Reading {
        id: RuleId::new("for-each-target-players"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_for_each_target_players(document)),
    },
    Reading {
        id: RuleId::new("for-each-participant"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_for_each_participant(document)),
    },
    Reading {
        id: RuleId::new("complete-simple-subject-verb"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_simple_subject_verb(document)),
    },
    Reading {
        id: RuleId::new("leading-player-may"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_leading_player_may(document)),
    },
];

/// The document's reading, if a rule has one. Every reading whose head admits
/// the document runs; two readings that disagree are an ambiguity.
pub(super) fn read_document(document: &Document<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    let head = crate::lexer::parser_token_word_refs(document.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in DOCUMENT_READINGS {
        if !reading.head.accepts(head) {
            continue;
        }
        let (registry_outcome, registry_claims) =
            crate::word_claims::scoped(|| (reading.read)(document).within(reading.id));
        match registry_outcome {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            ).with_claims(registry_claims)),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading, as the registry deduplicated.
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    resolve_registry_candidates(DOCUMENT_REGISTRY, distinct, diagnostics)
}

fn read_resolving_card_countered_exile_replacement(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effect) = parse_resolving_card_countered_exile_replacement(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_temporary_per_blocker_tax(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    // This complete turn-scoped blocking tax contains both `can't` and an
    // `unless` payment tail. Claim its grammar-proven effect shape before
    // broad restriction-chain parsing can split that tail as a second
    // restriction subject.
    // This complete turn-scoped blocking tax contains both `can't` and an
    // `unless` payment tail. Claim its grammar-proven effect shape before
    // broad restriction-chain parsing can split that tail as a second
    // restriction subject.
    if let Some(effect) = parse_temporary_per_blocker_tax(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_choice_complement(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    // A choice-complement program owns its comma-separated keep slots and
    // trailing `then sacrifices the rest` action. Preserve that complete
    // typed shape before generic statement coordination splits either part.
    // A choice-complement program owns its comma-separated keep slots and
    // trailing `then sacrifices the rest` action. Preserve that complete
    // typed shape before generic statement coordination splits either part.
    if split_lexed_sentences(tokens).len() == 1
        && let Some(effect) = super::dispatch_inner::parse_choice_complement_subject_verb(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_exile_then_return_same_object(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    // A same-object blink owns the complete comma-then sentence, including
    // inline battlefield-entry modifiers. Claim that grammar-proven program
    // before the broad single subject/verb route accepts only exile+return
    // and silently discards `with ... counter on it`.
    // A same-object blink owns the complete comma-then sentence, including
    // inline battlefield-entry modifiers. Claim that grammar-proven program
    // before the broad single subject/verb route accepts only exile+return
    // and silently discards `with ... counter on it`.
    if let Some(effects) =
        super::dispatch_inner::parse_exile_then_return_same_object_sentence(tokens)?
    {
        return Ok(effects).map(Some);
    }
    Ok(None)
}
fn read_reveal_source_exiled_permanents(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    // A source-linked exile reveal owns both the face-up action and the
    // permanent subset move inside one quantified player loop. Claim the
    // complete typed sequence before broad participant-clause parsing can
    // accept the `Each player` head and reject its compound action tail.
    // A source-linked exile reveal owns both the face-up action and the
    // permanent subset move inside one quantified player loop. Claim the
    // complete typed sequence before broad participant-clause parsing can
    // accept the `Each player` head and reject its compound action tail.
    if let Some(effects) =
        super::chain_carry::parse_reveal_source_exiled_permanents_sentence_lexed(tokens)
    {
        return Ok(Some(
            super::chain_carry::preserve_coordinated_effect_chain_surface(tokens, effects),
        ));
    }
    Ok(None)
}
fn read_each_player_may_discard_hand_and_draw(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    // This complete optional wheel has one shared `may` scope around both
    // actions. Claim the grammar-proven program before generic coordination
    // sends the draw suffix into the discard-count parser.
    // This complete optional wheel has one shared `may` scope around both
    // actions. Claim the grammar-proven program before generic coordination
    // sends the draw suffix into the discard-count parser.
    if effect_grammar::sacrifice_discard_shapes::parse_each_player_may_discard_hand_and_draw_tokens(
        tokens,
    )
    .is_some()
    {
        return super::parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
/// A frontend-authored bounded program uses unquoted semicolons for its
/// internal sentence boundaries so the enclosing period still closes its scope.
/// Restore those boundaries only inside the already bounded token slice.
pub(crate) fn parse_bounded_program(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut body = tokens.to_vec();
    let mut quoted = false;
    let mut depth = 0usize;
    for token in &mut body {
        if token.kind == TokenKind::Quote { quoted = !quoted; }
        else if !quoted {
            match token.kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => depth = depth.saturating_sub(1),
                TokenKind::Semicolon if depth == 0 => token.kind = TokenKind::Period,
                _ => {},
            }
        }
    }
    let mut effects = crate::clause_support::parse_effect_sentences_lexed(&body)?;
    bind_just_created_named_token_sources(&mut effects);
    Ok(effects)
}

/// "Create a ... token named Memory; Allow the controllers of tokens named
/// Memory just created to ...": the later instruction names the exact token
/// the earlier creation in this program produced, never every object with
/// that name. The creation keeps its result under a declared tag.
fn bind_just_created_named_token_sources(effects: &mut [EffectAst]) {
    use crate::cards::builders::{KeywordActionAst, SubjectVerbActionAst, TargetAst, TokenActionAst};
    for index in 0..effects.len() {
        let name = match &effects[index] {
            EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
                SubjectVerbActionAst::KeywordActions(KeywordActionAst::GrantExileControllerInspection {
                    sources: TargetAst::Object(filter, _, _), ..
                }) => filter.name.clone(),
                _ => None,
            },
            _ => None,
        };
        let Some(name) = name else { continue };
        // The nearest earlier token creation in this program is the one
        // "just created".
        let creation = (0..index).rev().find(|candidate| matches!(&effects[*candidate],
            EffectAst::SubjectVerb(subject_verb) if matches!(&subject_verb.action,
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. }))));
        let Some(creation) = creation else { continue };
        let tag = ironsmith_compiler_semantic::tag::declared_key(format!("just_created_{}", name.to_ascii_lowercase()));
        let created = std::mem::replace(&mut effects[creation], EffectAst::Sequence { effects: Vec::new() });
        effects[creation] = EffectAst::TagAffected { tag: tag.clone(), effect: Box::new(created) };
        if let EffectAst::SubjectVerb(subject_verb) = &mut effects[index]
            && let SubjectVerbActionAst::KeywordActions(KeywordActionAst::GrantExileControllerInspection { sources, .. }) = &mut subject_verb.action
        {
            *sources = TargetAst::Tagged(tag, None);
        }
    }
}

fn bounded_program_colon(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut quoted = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote { quoted = !quoted; }
        else if !quoted && token.kind == TokenKind::Colon { return Some(index); }
        else if !quoted && token.kind == TokenKind::Period { return None; }
    }
    None
}

/// Brackets delimit a nestable complete program. Legacy unbracketed programs
/// retain their first-sentence scope. Quoted punctuation never closes a body.
fn bounded_program_payload<'a>(document: &Document<'a>, colon: usize)
    -> Result<(&'a [OwnedLexToken], &'a [OwnedLexToken]), CardTextError>
{
    let tokens = document.tokens;
    if tokens.get(colon + 1).is_some_and(|token| token.kind == TokenKind::LBracket) {
        let start = colon + 2;
        let mut depth = 1usize;
        let mut quoted = false;
        for (index, token) in tokens.iter().enumerate().skip(start) {
            if token.kind == TokenKind::Quote { quoted = !quoted; }
            else if !quoted {
                if token.kind == TokenKind::LBracket { depth += 1; }
                else if token.kind == TokenKind::RBracket {
                    depth -= 1;
                    if depth == 0 {
                        let tail = &tokens[index + 1..];
                        // A conjunction after the closing delimiter coordinates a sibling
                        // instruction. Conjunctions inside the body remain locally owned.
                        let tail = if tail.first().is_some_and(|token| token.is_word("and")) {
                            crate::word_claims::claim_tokens(&tail[..1]);
                            &tail[1..]
                        } else {
                            if tail.first().is_some_and(|token| !matches!(token.kind, TokenKind::Period | TokenKind::Semicolon)) {
                                return Err(CardTextError::ParseError("bounded program needs a separator after its closing bracket".into()));
                            }
                            tail
                        };
                        return Ok((&tokens[start..index], crate::util::trim_edge_punctuation_tokens(tail)));
                    }
                }
            }
        }
        return Err(CardTextError::ParseError("bounded program has no closing bracket".into()));
    }
    let Some(&first) = document.sentences.first() else { return Ok((&[], &[])); };
    Ok((&first[colon + 1..], crate::util::trim_edge_punctuation_tokens(&tokens[first.len()..])))
}

fn bounded_program_followups(output: &mut Vec<EffectAst>, tail: &[OwnedLexToken]) -> Result<(), CardTextError> {
    if !tail.is_empty() { output.extend(crate::clause_support::parse_effect_sentences_lexed(tail)?); }
    Ok(())
}

/// One non-targeted card is copied and its copy may be cast immediately.
/// Existing CastTagged(as_copy) owns CR707.12's copy/cast continuation, exactly
/// as the plural linked-exile copy/cast reader does for each selected card.
fn read_imperative_participant_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = document.sentences.first() else { return Ok(None); };
    let words = crate::lexer::parser_token_word_refs(first);
    if !words.starts_with(&["for", "each"]) || !words.windows(2).any(|words| words == ["you", "may"]) { return Ok(None); }
    let Some(shape) = crate::grammar::effects::for_each_shapes::parse_participant_clause_shape(first) else { return Ok(None); };
    if shape.participant_is_actor || !crate::lexer::parser_token_word_refs(shape.inner_tokens).starts_with(&["you", "may"]) { return Ok(None); }
    let effect = super::for_each_helpers::parse_imperative_optional_participant(first)?.ok_or_else(|| CardTextError::ParseError(format!("complete imperative participant program has no participant reader: {}", crate::lexer::render_token_slice(first))))?;
    crate::word_claims::claim_tokens(&first[..first.len()-shape.inner_tokens.len()]);
    let mut output = vec![effect];
    for sentence in document.sentences.iter().skip(1) { output.extend(crate::clause_support::parse_effect_sentences_lexed(sentence)?); }
    Ok(Some(output))
}

fn read_value_bound_following_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(colon) = bounded_program_colon(document.tokens) else { return Ok(None); };
    let header = &document.tokens[..colon];
    let words = crate::lexer::parser_token_word_refs(header);
    if !words.starts_with(&["with", "x", "equal", "to"]) || !words.ends_with(&["do", "the", "following"]) { return Ok(None); }
    if words.as_slice() == ["with", "x", "equal", "to", "the", "number", "of", "objects", "chosen", "this", "way", "do", "the", "following"] { return Ok(None); }
    let Some(comma) = header.iter().position(|token|token.kind == TokenKind::Comma) else { return Ok(None); };
    let Some((value, used)) = crate::grammar::shared_util::value_expr::parse_value_expr_tokens(&header[4..comma]) else { return Ok(None); };
    if used != comma - 4 { return Ok(None); }
    let (body, tail) = bounded_program_payload(document, colon)?;
    let effects = parse_bounded_program(body)?;
    if effects.is_empty() { return Ok(None); }
    crate::word_claims::claim_tokens(header);
    let mut output = vec![EffectAst::BindX { value, effects }];
    bounded_program_followups(&mut output, tail)?;
    Ok(Some(output))
}

/// "Return target creature card ... . With X equal to that creature's power,
/// do the following: [...]" (It's Alive!): ordinary leading sentences, then a
/// value-bound program whose value names their result. The leading sentences
/// keep their own readings; the bound program is read by its owner above.
fn read_leading_sentences_value_bound_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    let (mut quoted, mut depth) = (false, 0usize);
    let mut split = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Quote => quoted = !quoted,
            TokenKind::LBracket if !quoted => depth += 1,
            TokenKind::RBracket if !quoted => depth = depth.saturating_sub(1),
            TokenKind::Colon if !quoted && depth == 0 => break,
            TokenKind::Period if !quoted && depth == 0
                && crate::lexer::parser_token_word_refs(&tokens[index + 1..]).starts_with(&["with", "x", "equal", "to"]) =>
            {
                split = Some(index);
                break;
            }
            _ => {}
        }
    }
    let Some(period) = split else { return Ok(None); };
    let Some(bound) = read_value_bound_following_program(&Document::new(&tokens[period + 1..]))? else { return Ok(None); };
    let mut output = crate::clause_support::parse_effect_sentences_lexed(&tokens[..=period])?;
    if output.is_empty() { return Ok(None); }
    output.extend(bound);
    Ok(Some(output))
}

fn read_selected_card_copy_free_cast(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [first, second, rest @ ..] = document.sentences.as_slice() else { return Ok(None); };
    let first = crate::util::trim_edge_punctuation_tokens(first);
    if first.len() < 3 || !first[0].is_word("copy") || !first[1].is_any_word(&["a", "an"]) { return Ok(None); }
    if crate::lexer::parser_token_word_refs(second).as_slice() != ["you", "may", "cast", "the", "copy", "without", "paying", "its", "mana", "cost"] { return Ok(None); }
    if !first.iter().any(|token| token.is_word("card")) || first.iter().any(|token| token.is_word("target")) { return Ok(None); }
    let filter = crate::object_filters::parse_object_filter(&first[2..], false)?;
    if !filter.zone.is_some_and(|zone| zone != crate::zone::Zone::Stack) { return Ok(None); }
    let tag = crate::util::helper_tag_for_tokens(first, "copied_card");
    crate::word_claims::claim_tokens(&first[..2]);
    crate::word_claims::claim_tokens(second);
    let mut output = vec![EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
        filter, count: crate::effect::ChoiceCount::exactly(1), count_value: None,
        player: crate::cards::builders::PlayerAst::You, tag: tag.clone(),
    }), EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::MayByPlayer {
        player: crate::cards::builders::PlayerAst::You,
        effects: vec![EffectAst::subject_verb_cast_tagged(tag, crate::cards::builders::PlayerAst::You, false, true, true, None)],
    })];
    for sentence in rest { output.extend(crate::clause_support::parse_effect_sentences_lexed(sentence)?); }
    Ok(Some(output))
}

/// Reuse the bounded owner from nested clause/chain entry points before
/// comma coordination can separate its participant selector from its body.
pub(super) fn parse_bounded_foreach_program(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError> {
    read_foreach_following_program(&Document::new(tokens))
}

fn read_foreach_following_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(colon) = bounded_program_colon(document.tokens) else {
        // `Choose ... . For each card chosen this way, do the following:
        // [...]`: earlier complete sentences precede the program's header.
        // Several complete sentences may precede it ("You choose one of
        // them. Choose that card. For each card chosen this way, ...").
        let mut quoted = false;
        let mut depth = 0usize;
        let header_period = document.tokens.iter().enumerate().position(|(index, token)| {
            match token.kind {
                TokenKind::Quote => quoted = !quoted,
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => depth = depth.saturating_sub(1),
                _ => {}
            }
            !quoted && depth == 0 && token.kind == TokenKind::Period && {
                let rest = &document.tokens[index + 1..];
                bounded_program_colon(rest).is_some()
                    && crate::lexer::parser_token_word_refs(rest).starts_with(&["for", "each"])
            }
        });
        let Some(period) = header_period else { return Ok(None); };
        let rest_tokens = &document.tokens[period + 1..];
        let rest = Document::new(rest_tokens);
        let Some(mut program) = read_foreach_following_program(&rest)? else { return Ok(None); };
        let mut output = crate::clause_support::parse_effect_sentences_lexed(&document.tokens[..=period])?;
        output.append(&mut program);
        return Ok(Some(output));
    };
    let header = &document.tokens[..colon];
    let Some(comma) = header.iter().rposition(|token|token.kind==TokenKind::Comma) else{return Ok(None)};
    let suffix = &header[comma + 1..];
    let exact_words = |words: &[&str]| suffix.len() == words.len()
        && suffix.iter().zip(words).all(|(token, word)| token.is_word(word));
    let optional = if exact_words(&["do", "the", "following"]) { false }
        else if exact_words(&["you", "may", "do", "the", "following"]) { true }
        else { return Ok(None); };
    let subject = &header[..comma];
    if !crate::lexer::parser_token_word_refs(subject).starts_with(&["for","each"]) { return Ok(None); }
    // Counter iteration is owned by the independent player-counter program
    // reader. A counter on a player is not an object-target noun phrase.
    let subject_words=crate::lexer::parser_token_word_refs(subject);
    if let Some(counter_subject)=subject_words.strip_prefix(&["for","each"])
        .and_then(|words|words.strip_suffix(&["on","you"]))
        && counter_subject.last().is_some_and(|word|matches!(*word,"counter"|"counters"))
        && crate::grammar::filters::parse_counter_type_words(counter_subject).is_some()
    { return Ok(None); }
    // "For each player/opponent, do the following: [...]": every such
    // player in turn is "that player" in the bounded body.
    if matches!(subject_words.as_slice(), ["for","each","player"] | ["for","each","opponent"]) {
        let (body, tail) = bounded_program_payload(document, colon)?;
        let effects = parse_bounded_program(body)?;
        if effects.is_empty() { return Err(CardTextError::ParseError("bounded foreach program has an empty body".into())); }
        crate::word_claims::claim_tokens(header);
        let effects = if optional {
            vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: crate::cards::builders::PlayerAst::That, effects,
            })]
        } else { effects };
        let mut output = vec![EffectAst::ForEach(if subject_words[2] == "player" {
            ForEachEffectAst::ForEachPlayer { effects }
        } else {
            ForEachEffectAst::ForEachOpponent { effects }
        })];
        bounded_program_followups(&mut output, tail)?;
        return Ok(Some(output));
    }
    let player_target=super::for_each_helpers::parse_for_each_targeted_player_subject(subject)?;
    let target = if player_target.is_none() {super::for_each_helpers::parse_for_each_targeted_object_subject(subject)?}else{None};
    // `For each object chosen this way`: the generic noun names the chosen
    // set exactly as `card` would.
    let generic_object_subject;
    let object_subject = if matches!(subject_words.as_slice(), ["for", "each", "object" | "objects", "chosen", "this", "way"]) {
        generic_object_subject = subject.iter().map(|token| if token.is_any_word(&["object", "objects"]) {
            crate::lexer::synthetic_word_tokens(["card"]).remove(0)
        } else { token.clone() }).collect::<Vec<_>>();
        crate::word_claims::claim_tokens(&subject[2..3]);
        &generic_object_subject[..]
    } else { subject };
    let filter = if player_target.is_none() && target.is_none() { super::for_each_helpers::parse_for_each_object_subject(object_subject)? } else { None };
    if player_target.is_none() && target.is_none() && filter.is_none() { return Ok(None); }
    let (body, tail) = bounded_program_payload(document, colon)?;
    let mut effects = parse_bounded_program(body)?;
    if effects.is_empty() { return Err(CardTextError::ParseError("bounded foreach program has an empty body".into())); }
    for effect in &mut effects { super::chain_carry::bind_implicit_player_context(effect, crate::cards::builders::PlayerAst::You); }
    crate::word_claims::claim_tokens(&header[comma..]);
    // The choice is repeated for each captured subject. Putting May outside
    // this loop would turn independent optional actions into one all-or-none
    // decision and would also mis-scope the body's result-dependent steps.
    let effects = if optional {
        vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: crate::cards::builders::PlayerAst::You, effects,
        })]
    } else { effects };
    let mut output = if let Some((count,filter))=player_target {
        vec![EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers {count,filter,effects})]
    } else if let Some(target) = target {
        vec![EffectAst::TagAffected { tag: crate::tag::CompilerReferenceTag::It.bind(), effect: Box::new(EffectAst::subject_verb_target_only(target)) },
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag: crate::tag::CompilerReferenceTag::It.bind(), effects })]
    } else {
        let filter = filter.expect("object iterator");
        // `For each card chosen this way`: iterate the exact chosen objects
        // wherever they are (a chosen library or hand card is not on the
        // battlefield), not a battlefield query restricted to them.
        let chosen_tag = match filter.tagged_constraints.as_slice() {
            [constraint] if filter.zone.is_none()
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject => Some(constraint.tag.clone()),
            _ => None,
        };
        match chosen_tag {
            Some(tag) => vec![EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag: crate::tag::TagRef::of(tag), effects })],
            None => vec![EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, effects })],
        }
    };
    bounded_program_followups(&mut output, tail)?;
    Ok(Some(output))
}

/// The colon bounds one complete optional program, including its internal
/// decisions and result-dependent fallbacks. Later sentences remain outside it.
fn read_first_shared_resolution_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [first, rest @ ..] = document.sentences.as_slice() else { return Ok(None); };
    let Some(colon) = first.iter().position(|token| token.kind == TokenKind::Colon) else { return Ok(None); };
    let header = &first[..colon];
    let words = crate::lexer::parser_token_word_refs(header);
    let tail = ["ability", "you", "control", "this", "turn", "as", "follows"];
    if !words.starts_with(&["resolve", "the", "first"]) || !words.ends_with(&tail) || words.len() <= 3 + tail.len() { return Ok(None); }
    let label = words[3..words.len()-tail.len()].join(" ");
    let program = parse_bounded_program(&first[colon+1..])?;
    if program.is_empty() { return Err(CardTextError::ParseError("first shared resolution program is empty".into())); }
    crate::word_claims::claim_tokens(header);
    let mut effects = vec![EffectAst::subject_verb(crate::model::ast::SubjectVerbRoleAst::Actor,
        crate::cards::builders::PlayerAst::Implicit, crate::model::ast::SubjectVerbActionAst::KeywordActions(
            crate::cards::builders::KeywordActionAst::FirstResolutionEachTurn { label, program }))];
    for sentence in rest { effects.extend(crate::clause_support::parse_effect_sentences_lexed(sentence)?); }
    Ok(Some(effects))
}

fn read_named_action_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(colon) = bounded_program_colon(document.tokens) else { return Ok(None); };
    let header = &document.tokens[..colon];
    // "If you do, perform the <label> action as follows: ..." as its own
    // document (the tail an optional-cost reader hands on).
    if document.tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(comma) = header.iter().position(|token| token.is_comma())
        && crate::lexer::parser_token_word_refs(&document.tokens[1..comma]) == ["you", "do"]
        && crate::lexer::parser_token_word_refs(&document.tokens[comma + 1..]).starts_with(&["perform", "the"])
    {
        let Some(program) = read_named_action_program(&Document::new(&document.tokens[comma + 1..]))? else { return Ok(None); };
        crate::word_claims::claim_tokens(&document.tokens[..=comma]);
        return Ok(Some(vec![EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did, effects: program,
        })]));
    }
    // A sequenced "Then perform the <name> action as follows: ..." sentence.
    if document.tokens.first().is_some_and(|token| token.is_word("then"))
        && crate::lexer::parser_token_word_refs(&document.tokens[1..]).starts_with(&["perform", "the"])
    {
        let Some(program) = read_named_action_program(&Document::new(&document.tokens[1..]))? else { return Ok(None); };
        crate::word_claims::claim_tokens(&document.tokens[..1]);
        return Ok(Some(program));
    }
    // "<earlier instructions>. Then perform the <name> action as follows: ..."
    // (Ontarachie Envoy): the earlier sentences resolve first, then the
    // named program; each keeps its own reader.
    if let Some(period) = header.iter().rposition(|token| token.is_period())
        && period + 1 < header.len()
    {
        let rest = &document.tokens[period + 1..];
        let leading_then = rest.first().is_some_and(|token| token.is_word("then"));
        let program_tokens = if leading_then { &rest[1..] } else { rest };
        let program_words = crate::lexer::parser_token_word_refs(program_tokens);
        // "... . If you controlled that spell, perform the catalyze action
        // as follows: ...": the named program is the conditional's body.
        if program_words.first() == Some(&"if")
            && !header[..period].iter().any(|token| token.kind == TokenKind::Quote)
            && let Some(comma) = program_tokens.iter().position(|token| token.is_comma())
            && crate::lexer::parser_token_word_refs(&program_tokens[comma + 1..]).starts_with(&["perform", "the"])
            && comma > 1
        {
            // "If you do, perform the <label> action as follows: ..." (after
            // an optional payment): the program runs only if it was paid.
            if crate::lexer::parser_token_word_refs(&program_tokens[1..comma]) == ["you", "do"] {
                let Some(program) = read_named_action_program(&Document::new(&program_tokens[comma + 1..]))? else { return Ok(None); };
                let mut effects = crate::clause_support::parse_effect_sentences_lexed(&document.tokens[..=period])?;
                crate::word_claims::claim_tokens(&program_tokens[..=comma]);
                if leading_then { crate::word_claims::claim_tokens(&rest[..1]); }
                effects.push(EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::IfResult {
                    predicate: crate::cards::builders::IfResultPredicate::Did, effects: program,
                }));
                return Ok(Some(effects));
            }
            let probe = crate::word_claims::probe_only(|| {
                crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&program_tokens[1..comma]).is_ok()
            });
            if probe {
                let Some(program) = read_named_action_program(&Document::new(&program_tokens[comma + 1..]))? else { return Ok(None); };
                let predicate = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&program_tokens[1..comma])?;
                let mut effects = crate::clause_support::parse_effect_sentences_lexed(&document.tokens[..=period])?;
                crate::word_claims::claim_tokens(&program_tokens[..1]);
                crate::word_claims::claim_tokens(&program_tokens[comma..comma + 1]);
                if leading_then { crate::word_claims::claim_tokens(&rest[..1]); }
                effects.push(EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::Conditional {
                    predicate, if_true: program, if_false: Vec::new(),
                }));
                return Ok(Some(effects));
            }
        }
        if program_words.starts_with(&["perform", "the"])
            && !header[..period].iter().any(|token| token.kind == TokenKind::Quote)
        {
            let Some(program) = read_named_action_program(&Document::new(program_tokens))? else { return Ok(None); };
            let mut effects = crate::clause_support::parse_effect_sentences_lexed(&document.tokens[..=period])?;
            if leading_then { crate::word_claims::claim_tokens(&rest[..1]); }
            effects.extend(program);
            return Ok(Some(effects));
        }
    }
    let all_words = crate::lexer::parser_token_word_refs(header);
    let (words, performer_player) = if all_words.starts_with(&["perform", "the"]) {
        (all_words.as_slice(), crate::cards::builders::PlayerAst::Implicit)
    } else if all_words.starts_with(&["you", "perform", "the"]) {
        (&all_words[1..], crate::cards::builders::PlayerAst::You)
    } else if all_words.starts_with(&["that", "player", "performs", "the"]) {
        (&all_words[2..], crate::cards::builders::PlayerAst::That)
    } else if all_words.starts_with(&["its", "controller", "performs", "the"]) {
        (&all_words[2..], crate::cards::builders::PlayerAst::ItsController)
    } else if all_words.starts_with(&["its", "owner", "performs", "the"]) {
        (&all_words[2..], crate::cards::builders::PlayerAst::ItsOwner)
    } else { return Ok(None); };
    let actor_words = all_words.len() - words.len();
    let mut input_effects=Vec::new();
    let choice_suffix = ["as", "follows", "completing", "the", "action", "only", "if", "all", "choices", "were", "available"];
    let choice_required = words.ends_with(&choice_suffix);
    let proposed_action=words.windows(2).position(|pair|pair==["action","on"]);
    let (suffix_len, program_completion, objects) = if let Some(action)=proposed_action.filter(|_|choice_required || words.ends_with(&["as","follows"])) {
        let view=crate::lexer::TokenWordView::new(header);
        let Some(range)=view.token_span_for_words(actor_words+action+2,actor_words+words.len()-if choice_required { choice_suffix.len() } else { 2 })else{return Ok(None)};
        let inputs=crate::util::helper_tag_for_tokens(&header[range.clone()],"proposed_action_inputs");
        input_effects=super::named_action_inputs::capture(&header[range],inputs.clone())?;
        let completion = if choice_required { ironsmith_core::effect::NamedActionProgramCompletion::ProposedObjectsWithAvailableChoices }
            else { ironsmith_core::effect::NamedActionProgramCompletion::ProposedObjects };
        (words.len()-action,completion,Some(crate::model::TargetAst::Tagged(inputs,None)))
    }else if words.ends_with(&["action", "as", "follows"]) {
        (3, ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted, None)
    } else if words.ends_with(&["action", "with", "tokens", "created", "this", "way", "if", "any", "are", "created"]) {
        (10, ironsmith_core::effect::NamedActionProgramCompletion::NonemptyObjects,
            Some(crate::cards::builders::TargetAst::Tagged(crate::tag::TagRef::of(crate::tag::TagKey::from("__named_action_created_objects")), None)))
    } else if words.ends_with(&["action", "with", "tokens", "created", "this", "way", "as", "follows"]) {
        (8, ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
            Some(crate::cards::builders::TargetAst::Tagged(crate::tag::TagRef::of(crate::tag::TagKey::from("__named_action_created_objects")), None)))
    } else if words.ends_with(&["action", "with", "objects", "chosen", "this", "way", "if", "the", "following", "cost", "is", "paid"]) {
        (12, ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment,
            Some(crate::model::TargetAst::Tagged(crate::tag::CompilerReferenceTag::ChosenObjects.bind(), None)))
    } else { return Ok(None); };
    if words.len() <= suffix_len + 2 { return Ok(None); }
    let label = words[2..words.len() - suffix_len].join(" ");
    let (body, mut tail) = bounded_program_payload(document, colon)?;
    let mut program = parse_bounded_program(body)?;
    if objects.as_ref().is_some_and(|target| matches!(target, crate::cards::builders::TargetAst::Tagged(tag, _) if tag.as_str() == "__named_action_created_objects")) {
        let following = Document::new(tail);
        if let Some(first) = following.sentences.first()
            && matches!(crate::lexer::parser_token_word_refs(first).as_slice(), ["the", "token", "enters", "tapped"] | ["the", "tokens", "enter", "tapped"])
            && named_program_creation_count(&program) == 1 {
            named_program_creation_tapped(&mut program);
            crate::word_claims::claim_tokens(first);
            tail = crate::util::trim_edge_punctuation_tokens(&tail[first.len()..]);
        }
    }
    if program.is_empty() { return Ok(None); }
    crate::word_claims::claim_tokens(header);
    let mut effects = input_effects;
    effects.push(EffectAst::subject_verb(crate::model::ast::SubjectVerbRoleAst::Actor,
        performer_player, crate::model::ast::SubjectVerbActionAst::KeywordActions(
            crate::cards::builders::KeywordActionAst::PerformNamedAction { program_completion, label, performer: None, objects, prior_objects: None, program: Some(program) })));
    bounded_program_followups(&mut effects,tail)?;
    Ok(Some(effects))
}

fn named_program_creation_count(effects: &[EffectAst]) -> usize {
    let mut count = 0;
    for effect in effects {
        if matches!(effect, EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst { action: crate::cards::builders::SubjectVerbActionAst::Tokens(crate::cards::builders::TokenActionAst::CreateTokenWithMods { .. } | crate::cards::builders::TokenActionAst::CreateTokenCopy { .. } | crate::cards::builders::TokenActionAst::CreateTokenCopyFromSource { .. }), .. })) { count += 1; }
        crate::effect_ast_traversal::for_each_nested_effects(effect, true, |effects| count += named_program_creation_count(effects));
    }
    count
}
fn named_program_creation_tapped(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst { action: crate::cards::builders::SubjectVerbActionAst::Tokens(action), .. }) = effect {
            match action {
                crate::cards::builders::TokenActionAst::CreateTokenWithMods { tapped, .. } => *tapped = true,
                crate::cards::builders::TokenActionAst::CreateTokenCopy { enters_tapped, .. } | crate::cards::builders::TokenActionAst::CreateTokenCopyFromSource { enters_tapped, .. } => *enters_tapped = true,
                _ => {},
            }
        }
        crate::effect_ast_traversal::for_each_nested_effects_mut(effect, true, named_program_creation_tapped);
    }
}

fn read_result_following_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(colon) = bounded_program_colon(document.tokens) else { return Ok(None); };
    let header = &document.tokens[..colon];
    let Some(prefix) = crate::grammar::structure::split_leading_result_prefix_lexed(header) else { return Ok(None); };
    if crate::lexer::parser_token_word_refs(prefix.trailing_tokens).as_slice() != ["do", "the", "following"] { return Ok(None); }
    let (body, tail) = bounded_program_payload(document, colon)?;
    let effects = parse_bounded_program(body)?;
    if effects.is_empty() { return Ok(None); }
    crate::word_claims::claim_tokens(header);
    let conditional = match prefix.kind {
        crate::grammar::structure::LeadingResultPrefixKind::If => crate::cards::builders::ConditionalEffectAst::IfResult { predicate: prefix.predicate, effects },
        crate::grammar::structure::LeadingResultPrefixKind::When => crate::cards::builders::ConditionalEffectAst::WhenResult { predicate: prefix.predicate, effects },
    };
    let mut output = vec![EffectAst::Conditionals(conditional)];
    bounded_program_followups(&mut output, tail)?;
    Ok(Some(output))
}

fn read_optional_following_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(colon) = bounded_program_colon(document.tokens) else { return Ok(None); };
    let header = &document.tokens[..colon];
    let suffix=["may","do","the","following"];
    let Some(may_start)=header.windows(suffix.len()).rposition(|tokens|tokens.iter().zip(suffix).all(|(token,word)|token.is_word(word))) else{return Ok(None)};
    if may_start+suffix.len()!=header.len(){return Ok(None)}
    use crate::cards::builders::PlayerAst;
    let actors:[(&[&str],PlayerAst);7]=[
        (&["you"],PlayerAst::You),(&["they"],PlayerAst::That),
        (&["that","player"],PlayerAst::That),(&["its","controller"],PlayerAst::ItsController),
        (&["its","owner"],PlayerAst::ItsOwner),(&["target","player"],PlayerAst::Target),
        (&["target","opponent"],PlayerAst::TargetOpponent),
    ];
    let Some((scope_start,player))=actors.iter().find_map(|(words,player)| {
        let start=may_start.checked_sub(words.len())?;
        header[start..may_start].iter().zip(*words).all(|(token,word)|token.is_word(word))
            .then_some((start,*player))
    }) else{return Ok(None)};
    let mut prefix_end=scope_start;
    if prefix_end>0 {
        // A coordination separator bounds the earlier ordinary program. It
        // does not belong to the optional scope introduced by this header.
        if !header[prefix_end-1].is_word("then"){return Ok(None)}
        prefix_end-=1;
        if prefix_end>0 && header[prefix_end-1].kind==TokenKind::Comma {prefix_end-=1;}
        if prefix_end==0{return Ok(None)}
    }
    let prefix=if prefix_end>0 {crate::clause_support::parse_effect_sentences_lexed(&header[..prefix_end])?}else{Vec::new()};
    let (body, tail) = bounded_program_payload(document, colon)?;
    let effects = parse_bounded_program(body)?;
    if effects.is_empty() { return Ok(None); }
    crate::word_claims::claim_tokens(&header[prefix_end..]);
    let mut output=prefix;
    output.push(EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects }));
    bounded_program_followups(&mut output, tail)?;
    Ok(Some(output))
}

/// "Repeat the following process X times. <process sentences>" (Torment of
/// Hailfire): the remaining sentences run the given number of times.
fn read_repeat_following_process(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [first, rest @ ..] = document.sentences.as_slice() else {
        return Ok(None);
    };
    // A colon scopes repetition to the first sentence. Later sentences are
    // independent effects, outside the complete repeated action body.
    if let Some(colon) = bounded_program_colon(document.tokens) {
        let header = &document.tokens[..colon];
        let words = crate::lexer::parser_token_word_refs(header);
        if !words.starts_with(&["repeat", "the", "following", "process"])
            || words.last() != Some(&"times") || words.len() < 6 { return Ok(None); }
        let count_words = &words[4..words.len()-1];
        let Some((count, used)) = crate::util::parse_value_expr_words(count_words) else { return Ok(None); };
        if used != count_words.len() { return Ok(None); }
        let (body, tail) = bounded_program_payload(document, colon)?;
        let effects = parse_bounded_program(body)?;
        if effects.is_empty() { return Ok(None); }
        crate::word_claims::claim_tokens(header);
        let mut output = vec![EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, effects })];
        bounded_program_followups(&mut output, tail)?;
        return Ok(Some(output));
    }
    if rest.is_empty() {
        return Ok(None);
    }
    if first.iter().any(|token| !matches!(token.kind, TokenKind::Word | TokenKind::Comma | TokenKind::Period)) {
        return Ok(None);
    }
    let binding_start = first.iter().position(|token| token.is_word("where"));
    let header = binding_start.map_or(*first, |index| &first[..index]);
    let words = crate::lexer::parser_token_word_refs(header);
    let count_word = match words.as_slice() {
        ["repeat", "the", "following", "process", count, "times"] => *count,
        _ => return Ok(None),
    };
    let count = if count_word == "x" {
        if let Some(index) = binding_start {
            let Some(value) = super::dispatch_entry::parse_exact_where_x_value_expression(&first[index..]) else {
                return Ok(None);
            };
            value
        } else {
            crate::effect::Value::X
        }
    } else if let Some(count) = crate::util::parse_number_word_u32(count_word) {
        if binding_start.is_some() { return Ok(None); }
        crate::effect::Value::Fixed(count as i32)
    } else {
        return Ok(None);
    };
    let first_end = first.as_ptr() as usize - document.tokens.as_ptr() as usize;
    let first_end = first_end / std::mem::size_of::<OwnedLexToken>() + first.len();
    let Some(process_tokens) = document.tokens.get(first_end..) else {
        return Ok(None);
    };
    let effects = crate::clause_support::parse_effect_sentences_lexed(process_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }
    // Only the independently validated repeat header belongs to this reader.
    // The expression and body retain their own readers and token ownership.
    crate::word_claims::claim_for_line(header);
    Ok(Some(vec![EffectAst::ForEach(
        ForEachEffectAst::RepeatEffects { count, effects },
    )]))
}

fn read_conditional_put_counters(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // This reader owns one conditional counter statement. Later sentences
    // (including Otherwise) belong to the document's control-flow composer.
    if document.sentences.len() != 1 {
        return Ok(None);
    }
    let tokens = document.tokens;
    if effect_grammar::split_conditional_sentence_family_head_lexed(tokens).is_some() {
        let ParseOutcome::Match(plan) = effect_grammar::control_flow::recognize_control_flow(tokens) else {
            return Ok(None);
        };
        // Ownership follows the consequence, not words in the condition:
        // "If you put ... counters this way, draw a card" is a draw body.
        let words = crate::lexer::parser_token_word_refs(plan.value.body_tokens);
        if words.first() == Some(&"put")
            && words
                .iter()
                .any(|word| matches!(*word, "counter" | "counters"))
            && let Some(effects) = effect_grammar::parse_conditional_sentence_family_lexed(
                tokens,
                parse_single_put_counters_effect_chain,
            )?
        {
            return Ok(Some(effects));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod conditional_counter_ownership_tests {
    use super::*;

    #[test]
    fn colon_repeat_owns_only_its_process_sentence() {
        let tokens = crate::lexer::lex_line(
            "Repeat the following process X times: Draw a card; then you gain 1 life. You lose X life.", 0).unwrap();
        let effects = read_repeat_following_process(&Document::new(&tokens)).unwrap().unwrap();
        assert_eq!(effects.len(), 2, "the independent loss stays outside repetition");
        let EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, effects: body }) = &effects[0] else { panic!("missing scoped repeat"); };
        assert!(matches!(count, crate::effect::Value::X));
        assert_eq!(body.len(), 2);
        for text in [
            "Repeat the following process many times: Draw a card.",
            "Repeat the following process X times unless you pay: Draw a card.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(read_repeat_following_process(&Document::new(&tokens)).unwrap().is_none());
        }
    }

    #[test]
    fn repeat_header_composes_an_exact_value_binding_with_independent_actions() {
        let tokens = crate::lexer::lex_line(
            "Repeat the following process X times, where X is this creature's power. Draw a card. You gain 1 life.", 0).unwrap();
        let effects = read_repeat_following_process(&Document::new(&tokens)).unwrap().unwrap();
        let [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, effects })] = effects.as_slice() else {
            panic!("missing complete repeat program: {effects:?}");
        };
        assert!(matches!(count.unhinted(), crate::effect::Value::PowerOf(source)
            if matches!(source.unhinted(), crate::target::ChooseSpec::Source)));
        assert_eq!(effects.len(), 2);
        for header in [
            "Repeat the following process X times, where Y is this creature's power",
            "Repeat the following process 3 times, where X is this creature's power",
            "Repeat the following process X times, where X is this creature's power unless you pay",
            "Repeat the following process X times {2}",
        ] {
            let tokens = crate::lexer::lex_line(&format!("{header}. Draw a card."), 0).unwrap();
            assert!(read_repeat_following_process(&Document::new(&tokens)).unwrap().is_none(), "{header}");
        }
    }

    #[test]
    fn conditional_counter_reader_owns_only_counter_consequences() {
        let draw = crate::lexer::lex_line(
            "if you put two or more +1/+1 counters on creatures this way, draw a card.", 0).unwrap();
        assert!(read_conditional_put_counters(&Document::new(&draw)).unwrap().is_none());
        let put = crate::lexer::lex_line(
            "if you control an artifact, put a +1/+1 counter on target creature.", 0).unwrap();
        assert!(read_conditional_put_counters(&Document::new(&put)).unwrap().is_some());
    }
}

fn read_emblem_payload(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if effect_grammar::emblem_shapes::parse_damaged_player_emblem_payload_tokens(tokens)
        .or_else(|| effect_grammar::emblem_shapes::parse_emblem_payload_tokens(tokens))
        .is_some_and(|shape| shape.requires_whole_sentence_dispatch)
        && let Some(effect) = super::zone_handlers::parse_emblem_action(tokens, None)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_create(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) = parse_complete_create_statement(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_investigate(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) = parse_complete_investigate_statement(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_kicked_search_replacement_bundle(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) =
        super::bundle_rules::parse_complete_kicked_search_replacement_bundle(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_delegated_partition(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) = parse_complete_delegated_partition_program(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_each_player_coin_face(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    // A physical heads/tails follow-up is correlated with the result of the
    // preceding per-player flip. Claim the grammar-proven pair before broad
    // independent-statement parsing turns both sentences into unrelated
    // `ForEachPlayer` programs.
    // A physical heads/tails follow-up is correlated with the result of the
    // preceding per-player flip. Claim the grammar-proven pair before broad
    // independent-statement parsing turns both sentences into unrelated
    // `ForEachPlayer` programs.
    if let Some(effects) = parse_each_player_coin_face_sequence(&sentences)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_divvy(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    let divvy_sentences = sentences
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    if let Some(effects) = try_parse_divvy_sentence_sequence(&divvy_sentences)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_delegated_partition_program_prefix(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let Some(effects) = parse_delegated_partition_program_prefix(&sentences)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_composable_fight(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) = parse_complete_composable_fight_program(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_cant_effect(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        // The outer conditional owns its predicate and consequence. A nested
        // restriction's duration cannot commit this whole-document probe to
        // parsing the predicate as a restriction subject. Keep the same
        // ownership boundary as the statement registry; the low-level cant
        // grammar still supports its specialized conditional restriction forms.
        && !sentence
            .first()
            .is_some_and(|token| token.is_any_word(&["if", "unless"]))
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && super::lex_chain_helpers::split_effect_chain_on_and_lexed(sentence).len() == 1
        && let Some(effects) = super::parse_cant_effect_sentence_lexed(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_serial_target_pt_modifiers(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(effects) =
            super::fanout_family::parse_serial_target_pt_modifiers_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_compound_damage_fanout(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    // A paired damage sentence with one spent-to-cast condition per arm is
    // one grammar-proven fanout. The broad standalone subject/verb leaf below
    // can otherwise accept only the final damage arm after preprocessing has
    // normalized the source text, silently dropping the first condition.
    // A paired damage sentence with one spent-to-cast condition per arm is
    // one grammar-proven fanout. The broad standalone subject/verb leaf below
    // can otherwise accept only the final damage arm after preprocessing has
    // normalized the source text, silently dropping the first condition.
    if let [sentence] = sentences.as_slice()
        && let Some(effects) =
            super::fanout_family::parse_compound_damage_fanout_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_compound_gain(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(effects) = parse_complete_compound_gain_statement(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_keyword_bundle_pump(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    // Every `if it has ...` fragment in a keyword ladder is a condition on
    // one pump arm, not a trailing condition on the complete sentence. Give
    // the grammar-proven ladder priority over the broad trailing-if route.
    // Every `if it has ...` fragment in a keyword ladder is a condition on
    // one pump arm, not a trailing condition on the complete sentence. Give
    // the grammar-proven ladder priority over the broad trailing-if route.
    if let [sentence] = sentences.as_slice()
        && let Some(effects) =
            super::subject_verb_special_recognizers::parse_keyword_bundle_pump_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_effect_clause_with_trailing_if(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    let [sentence] = sentences.as_slice() else {
        return Ok(None);
    };
    // Two conditions joined by "and" are the conditional damage pair's; a run of them is the keyword bundle's.
    if sentence.iter().filter(|token| token.is_word("if")).count() >= 2 {
        return Ok(None);
    }
    // "..., except it has X if it doesn't have X": the condition belongs to
    // the copy exception, not to the whole effect (the become reading owns it).
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence)
        .is_some_and(|trailing| trailing.leading_tokens.iter().any(|token| token.is_word("except")))
    {
        return Ok(None);
    }
    // A pump-and-grant with a trailing condition is the compound gain statement's, which keeps the condition.
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some_and(|trailing| {
        effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(trailing.leading_tokens)
            .is_some()
            || effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(
                trailing.leading_tokens,
            )
            .is_some()
    }) {
        return Ok(None);
    }
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && effect_grammar::subject_verb_registry_shapes::parse_registry_next_end_step_shape(
            sentence,
        )
        .is_none()
        && crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some()
        && let Ok(effect) = super::chain_carry::parse_effect_clause_with_trailing_if_lexed(sentence)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_get_pump(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(effect) = parse_complete_get_pump_statement(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_become(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(effect) = parse_complete_become_statement(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
/// "Create a Gold token. Each opponent attacking that player does the same."
/// (Curse of Opulence): the first sentence's actions are repeated for every
/// opponent attacking the enchanted player.
fn read_each_opponent_attacking_that_player_does_the_same(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    let [first, second] = sentences.as_slice() else {
        return Ok(None);
    };
    let second_words = crate::lexer::token_word_refs(second);
    let Some(tail) = second_words.strip_prefix(&["each", "opponent", "attacking", "that", "player"]) else {
        return Ok(None);
    };
    let effects = crate::clause_support::parse_effect_sentences_lexed(first)?;
    let repeated_basis = if tail == ["does", "the", "same"] {
        effects.clone()
    } else {
        // The same relational player subject may carry its own explicit
        // action ("... untaps all nonland permanents they control").
        let start = crate::lexer::parser_token_word_positions(second)
            .get(5)
            .map(|(index, _)| *index)
            .unwrap_or(second.len());
        if start == second.len() {
            return Ok(None);
        }
        crate::clause_support::parse_effect_sentences_lexed(&second[start..])?
    };
    let mut repeated = Vec::with_capacity(repeated_basis.len());
    for effect in repeated_basis {
        let EffectAst::SubjectVerb(mut subject_verb) = effect else {
            return Ok(None);
        };
        subject_verb.subject.player = crate::cards::builders::PlayerAst::That;
        repeated.push(EffectAst::SubjectVerb(subject_verb));
    }
    let mut out = effects;
    out.push(EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
        sequential: false,
        filter: crate::target::PlayerFilter::opponents_attacking_event_defender(),
        effects: repeated,
    }));
    Ok(Some(out))
}

fn read_sentence_each_opponent_draws_then_you_draw_per_opponent(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && let Some(effects) = super::subject_verb_primitives::
            parse_sentence_each_opponent_draws_then_you_draw_per_opponent(
                SubjectVerbPrimitiveClause::new(sentence),
            )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_sentence_you_and_attacking_player_each_draw_and_lose(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
            && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
            && let Some(effects) = super::subject_verb_primitives::
                parse_sentence_you_and_attacking_player_each_draw_and_lose(
                    SubjectVerbPrimitiveClause::new(sentence),
                )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_next_end_step_followups(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && effect_grammar::subject_verb_registry_shapes::parse_registry_next_end_step_shape(
            sentence,
        )
        .is_some()
        // "exile this permanent and return it ... at the beginning of the
        // next end step" (Frenetic Sliver) coordinates an immediate action
        // with the delayed one; the single delayed-action shape would read
        // the second action as part of the exiled object's noun phrase.
        && !crate::lexer::token_word_refs(sentence)
            .word_windows(2)
            .any(|window| window == ["and", "return"])
    {
        let clause = SubjectVerbPrimitiveClause::new(sentence);
        if let Some(effects) =
            super::subject_verb_primitives::parse_sentence_sacrifice_it_next_end_step(clause)?
        {
            return Ok(Some(effects));
        }
        if let Some(effects) =
            super::subject_verb_primitives::parse_sentence_exile_it_next_end_step(
                SubjectVerbPrimitiveClause::new(sentence),
            )?
        {
            return Ok(Some(effects));
        }
    }
    Ok(None)
}
fn read_for_each_target_players(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(effect) =
            super::for_each_helpers::parse_for_each_target_players_clause(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_for_each_participant(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    let [sentence] = sentences.as_slice() else {
        return Ok(None);
    };
    if let Some(effects) = super::subject_verb_primitives::parse_sentence_each_player_may_reveal_selected_cards_in_their_hand(
        SubjectVerbPrimitiveClause::new(sentence),
    )? {
        return Ok(Some(effects));
    }
    // An explicitly named actor after a coordination boundary starts a new
    // participant scope. Let chain composition keep it outside this loop.
    if !(sentence.first().is_some_and(|token| token.is_word("for"))
        && sentence.iter().any(|token| token.is_word("who")))
        && let crate::recognition::ParseOutcome::Match(plan) =
            effect_grammar::coordination::recognize_coordination(sentence)
        && plan.value.members.iter().skip(1).any(|member| {
            member.head.is_some_and(|head| {
                matches!(
                    head.actor,
                    effect_grammar::typed_clause_heads::ClauseActorHeadAst::Controller
                        | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Player
                        | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Iterated
                )
            })
        })
    {
        return Ok(None);
    }
    // A choice complement ("each player chooses ... then sacrifices the rest") is the complement statement's.
    if super::dispatch_inner::is_choice_complement_shape(sentence) {
        return Ok(None);
    }
    // "Each player may discard their hand and draw seven cards" is its own statement.
    if effect_grammar::sacrifice_discard_shapes::parse_each_player_may_discard_hand_and_draw_tokens(
        sentence,
    )
    .is_some()
    {
        return Ok(None);
    }
    // A loop with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some() {
        return Ok(None);
    }
    // A loop that opens with a creation ("each player creates ...") is the create statement's.
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence).is_some_and(
        |shape| {
            shape.participant_is_actor
                && shape
                    .inner_tokens
                    .first()
                    .is_some_and(|token| token.is_any_word(&["create", "creates"]))
        },
    ) {
        return Ok(None);
    }
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence).is_some()
        && let Some(effect) = super::for_each_helpers::parse_for_each_player_clause(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_simple_subject_verb(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
        && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(effect) = parse_complete_simple_subject_verb_sentence(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
/// "You may cast X ... as though ..." grants a permission; its `may` is part of
/// the permission, never an optional wrapper over the sentence's other effects.
fn leading_may_is_cast_as_though_permission(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    crate::word_primitives::parse_sequence_prefix(&words, &["you", "may", "cast"])
        && crate::word_primitives::sequence_occurs(&words, &["as", "though"])
}
fn read_leading_player_may(
    document: &Document<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    // The explicit quoted-price owner builds one atomic MayPay. A generic
    // optional-action wrapper must not strip its modal/price envelope and
    // offer a competing ordinary mana-payment reading.
    if sentences.first().is_some_and(|first|
        super::bounded_source_programs::has_optional_cost_program_head(first))
    { return Ok(None); }
    // A quoted restriction grant is a complete typed effect shape, but its
    // whole-sentence recognizer must not absorb a leading player choice.
    // Preserve that choice before independent-statement dispatch flattens a
    // following `If you do` clause away from its antecedent.
    // A quoted restriction grant is a complete typed effect shape, but its
    // whole-sentence recognizer must not absorb a leading player choice.
    // Preserve that choice before independent-statement dispatch flattens a
    // following `If you do` clause away from its antecedent.
    if let Some(first_sentence) = sentences.first().copied()
        && first_sentence
            .iter()
            .any(|token| token.kind == TokenKind::Quote)
        && !leading_may_is_cast_as_though_permission(first_sentence)
        && let Some(player) = super::parse_leading_player_may_lexed(first_sentence)
    {
        let mut stripped = super::chain_carry::remove_through_first_word(first_sentence);
        if let Some(rest) =
            super::super::front_end::grammar::effects::chain_carry::strip_leading_have_tokens(
                &stripped,
            )
        {
            stripped = rest.to_vec();
        }
        let mut optional_effects =
            super::dispatch_entry::parse_effect_sentences_lexed_inner(&stripped)?;
        for effect in &mut optional_effects {
            super::chain_carry::bind_implicit_player_context(effect, player);
        }
        let mut effects = vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player,
            effects: optional_effects,
        })];
        for sentence in sentences.iter().skip(1) {
            if let Some(followup) =
                super::super::grammar::sentence_markers::parse_conditional_followup_tokens(sentence)
            {
                let continuation = trim_edge_punctuation(followup.tail_tokens);
                effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    predicate: IfResultPredicate::Did,
                    effects: super::dispatch_entry::parse_effect_sentences_lexed_inner(
                        &continuation,
                    )?,
                }));
            } else {
                effects.extend(super::dispatch_entry::parse_effect_sentences_lexed_inner(
                    sentence,
                )?);
            }
        }
        return Ok(Some(effects));
    }
    Ok(None)
}

#[cfg(test)]
#[test]
fn bounded_object_program_keeps_later_sentence_outside_iteration() {
    for subject in ["For each creature you control", "For each of creatures you control"] {
        let text = format!("{subject}, do the following: [Put a +1/+1 counter on that creature; You gain 1 life]. You gain 3 life.");
        let tokens = crate::lexer::lex_line(&text, 0).unwrap();
        let effects = read_foreach_following_program(&Document::new(&tokens)).unwrap().unwrap();
        assert_eq!(effects.len(), 2);
        let EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects: body, .. }) = &effects[0] else { panic!("object iteration"); };
        assert_eq!(body.len(), 2);
    }
}

#[cfg(test)]
#[test]
fn independent_program_headers_do_not_become_creator_subjects() {
    for text in [
        "Perform the assess action as follows: Create zero 1/1 green Saproling creature tokens.",
        "Perform the tally action as follows: Create a 1/1 white Soldier creature token.",
        "You may do the following: Create zero 1/1 green Saproling creature tokens.",
        "For each creature you control, do the following: [Create a 1/1 white Soldier creature token].",
    ] {
        let tokens=crate::lexer::lex_line(text,0).unwrap();
        assert!(parse_complete_create_statement(&tokens).unwrap().is_none(),"the create leaf cannot absorb the outer program header: {text}");
        crate::clause_support::parse_effect_sentences_lexed(&tokens).unwrap_or_else(|error|panic!("the independent envelope and create child must compose: {text}: {error:?}"));
    }
    for text in ["Create zero 1/1 green Saproling creature tokens.","You create a 1/1 white Soldier creature token."] {
        let tokens=crate::lexer::lex_line(text,0).unwrap();
        assert!(parse_complete_create_statement(&tokens).unwrap().is_some(),"ordinary creator clauses remain independent: {text}");
    }
}

/// A quoted cost is an explicit optional payment instruction, not a modal
/// quote or a loose optional action. Keep following result clauses as siblings.
fn read_explicit_optional_cost_program(document:&Document<'_>)->Result<Option<Vec<EffectAst>>,CardTextError>{
    let Some(first)=document.sentences.first().copied() else{return Ok(None);};
    if !super::bounded_source_programs::has_optional_cost_program_head(first){return Ok(None);}
    let mut effects=super::parse_effect_sentence_lexed(first)?;
    if !matches!(effects.as_slice(),[EffectAst::Permissions(PermissionEffectAst::MayPay{..})]) {return Ok(None);}
    // The continuation can contain a nested bounded If-you-do program.
    // Give its owner the complete tail; splitting its interior sentences
    // here would erase the bracket/result scope before that owner runs.
    let tail=crate::util::trim_edge_punctuation_tokens(&document.tokens[first.len()..]);
    if !tail.is_empty(){effects.extend(crate::clause_support::parse_effect_sentences_lexed(tail)?);}
    Ok(Some(effects))
}

/// A resolution-time modal choice with each complete mode explicitly bounded.
/// Brackets allow nested optional/repeat/foreach programs and keep a following
/// sibling outside the selected mode. No source keyword participates here.
fn read_bounded_inline_modal(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    let Some(colon) = bounded_program_colon(tokens) else { return Ok(None); };
    let header = crate::lexer::parser_token_word_refs(&tokens[..colon]);
    // "you may choose one of the following: [A] or [B]": the same choice,
    // which the player may decline.
    let optional = header.starts_with(&["you", "may", "choose"]);
    let header = if optional { header[2..].to_vec() } else { header };
    let chooser = match header.as_slice() {
        ["choose", "one", "of", "the", "following"]
        | ["you", "choose", "one", "of", "the", "following"] => crate::target::PlayerFilter::You,
        ["its", "controller", "chooses", "one", "of", "the", "following"] =>
            crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(
                crate::tag::CompilerReferenceTag::It.bind().into())),
        ["its", "owner", "chooses", "one", "of", "the", "following"] =>
            crate::target::PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(
                crate::tag::CompilerReferenceTag::It.bind().into())),
        _ => return Ok(None),
    };
    crate::word_claims::claim_tokens(&tokens[..=colon]);
    let mut cursor = colon + 1;
    let mut modes = Vec::new();
    loop {
        if !tokens.get(cursor).is_some_and(|token| token.kind == TokenKind::LBracket) {
            return Err(CardTextError::ParseError("a bounded modal choice requires a bracketed mode".into()));
        }
        let start = cursor + 1;
        let mut depth = 1usize;
        let mut quoted = false;
        cursor = start;
        while cursor < tokens.len() {
            let token = &tokens[cursor];
            if token.kind == TokenKind::Quote { quoted = !quoted; }
            else if !quoted {
                if token.kind == TokenKind::LBracket { depth += 1; }
                else if token.kind == TokenKind::RBracket { depth -= 1; if depth == 0 { break; } }
            }
            cursor += 1;
        }
        if depth != 0 { return Err(CardTextError::ParseError("bounded modal mode has no closing bracket".into())); }
        let body = &tokens[start..cursor];
        let effects = parse_bounded_program(body)?;
        if effects.is_empty() { return Err(CardTextError::ParseError("bounded modal mode must contain an effect".into())); }
        modes.push(crate::cards::builders::ChooseOneModeAst {
            description: crate::lexer::render_token_slice(body),
            effects,
        });
        cursor += 1;
        if tokens.get(cursor).is_some_and(|token| token.is_word("or")) {
            crate::word_claims::claim_tokens(&tokens[cursor..cursor+1]);
            cursor += 1;
            continue;
        }
        break;
    }
    if modes.len() < 2 { return Err(CardTextError::ParseError("bounded modal choice requires at least two modes".into())); }
    if tokens.get(cursor).is_some_and(|token| !matches!(token.kind, TokenKind::Period | TokenKind::Semicolon)) {
        return Err(CardTextError::ParseError("bounded modal choice needs a separator before its following program".into()));
    }
    let choice = EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf {
        chooser,
        modes,
    });
    let mut effects = vec![if optional {
        EffectAst::Permissions(PermissionEffectAst::May { effects: vec![choice] })
    } else { choice }];
    bounded_program_followups(&mut effects, crate::util::trim_edge_punctuation_tokens(&tokens[cursor..]))?;
    Ok(Some(effects))
}

// Explicit status operations own their full sequential instruction, including
// `, then` siblings. Quoted labels never supply structural commas.
fn read_status_instruction_program(document:&Document<'_>)->Result<Option<Vec<EffectAst>>,CardTextError>{
    let Some(first)=document.sentences.first().copied() else{return Ok(None)};
    let mut quoted=false;let mut boundaries=Vec::new();
    for (index,token) in first.iter().enumerate(){
        if token.kind==TokenKind::Quote{quoted=!quoted;}
        else if !quoted && token.kind==TokenKind::Comma{boundaries.push(index);}
    }
    let mut body=first;let mut predicate=None;
    if first.first().is_some_and(|token|token.is_word("if")){
        let Some(comma)=boundaries.iter().copied().find(|index|crate::custom_status::source_condition(&first[..*index]).is_some()) else{return Ok(None)};
        predicate=crate::custom_status::source_condition(&first[..comma]);
        body=&first[comma+1..];crate::word_claims::claim_tokens(&first[comma..comma+1]);
    }
    let mut quoted=false;let mut cuts=Vec::new();
    for (index,token) in body.iter().enumerate(){
        if token.kind==TokenKind::Quote{quoted=!quoted;}
        else if !quoted && token.kind==TokenKind::Comma && body.get(index+1).is_some_and(|token|token.is_word("then")){cuts.push(index);}
    }
    if cuts.is_empty(){return Ok(None);}
    let Some(initial)=crate::custom_status::source_effect(&body[..cuts[0]]) else{return Ok(None)};
    let mut program=vec![initial];let mut start=cuts[0]+2;
    crate::word_claims::claim_tokens(&body[cuts[0]..start]);
    for end in cuts.iter().copied().skip(1).chain(std::iter::once(body.len())){
        if crate::lexer::token_word_refs(&body[start..end]).is_empty(){
            return Err(CardTextError::ParseError("empty instruction after status-program then separator".into()));
        }
        program.extend(crate::clause_support::parse_effect_sentences_lexed(&body[start..end])?);
        if end<body.len(){crate::word_claims::claim_tokens(&body[end..end+2]);start=end+2;}
    }
    let mut result=if let Some(predicate)=predicate{vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional{predicate,if_true:program,if_false:Vec::new()})]}else{program};
    let consumed=first.len();
    if consumed<document.tokens.len(){result.extend(crate::clause_support::parse_effect_sentences_lexed(&document.tokens[consumed..])?);}
    Ok(Some(result))
}

fn read_comma_filtered_play_permission(document:&Document<'_>)->Result<Option<Vec<EffectAst>>,CardTextError>{
 let Some(first)=document.sentences.first().copied() else{return Ok(None)};
 if !first.iter().any(|token|token.kind==TokenKind::Comma){return Ok(None)}
 let Some(permission)=crate::permission_helpers::parse_cast_or_play_tagged_clause(first)? else{return Ok(None)};
 let mut result=vec![permission];
 if first.len()<document.tokens.len(){result.extend(crate::clause_support::parse_effect_sentences_lexed(&document.tokens[first.len()..])?);}
 Ok(Some(result))
}

#[cfg(test)]
mod explicit_status_program_boundary_tests {
 use super::*;
 #[test]
 fn trailing_then_is_not_an_empty_successful_instruction(){
  let tokens=crate::util::tokenize_line("Remove the locked status from this card in the command zone, then.",0);
  assert!(read_status_instruction_program(&Document::new(&tokens)).is_err());
 }
 #[test]
 fn quoted_condition_comma_and_following_sibling_keep_their_scopes(){
  let tokens=crate::util::tokenize_line("If this card has the \"ready, now\" status, remove the locked status from this card in the command zone, then give this card the unlocked status in the command zone. Draw a card.",0);
  let program=read_status_instruction_program(&Document::new(&tokens)).unwrap().unwrap();
  assert_eq!(program.len(),2,"later draw stays outside conditional status program");
  let EffectAst::Conditionals(ConditionalEffectAst::Conditional{if_true,..})=&program[0] else{panic!("status condition lost")};
  assert_eq!(if_true.len(),2);
 }
}

/// Nested sentence dispatch must use the same complete bounded-mode owner as
/// a document starting with the menu. A preceding object-choice statement must
/// not divert its header into a different, partial legacy choice reading.
pub(crate) fn parse_bounded_inline_modal_tokens(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
    read_bounded_inline_modal(&Document::new(tokens))
}

/// A sentence after an earlier statement ("..., you get a research counter.
/// If you have eight or more, do the following: [A; B]") reaches nested
/// sentence dispatch; it keeps the same complete bracketed-program owner as a
/// document that starts with it.
/// A named-action program that is not its document's first sentence
/// ("Choose target creature. Its owner performs the X action as follows:
/// [...]") is the same complete bracketed owner at sentence scope.
/// A later sentence "Perform the <name> action as follows: ..." read with the
/// complete named-program grammar (its body's own sentences included).
pub(crate) fn parse_named_action_program_sentence(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
    read_named_action_program(&Document::new(tokens))
}

pub(crate) fn parse_named_action_program_tokens(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
    if !tokens.iter().any(|token| token.kind == TokenKind::LBracket) { return Ok(None); }
    Ok(crate::word_claims::speculative_keep_if(
        || read_named_action_program(&Document::new(tokens)),
        |result| matches!(result, Ok(Some(_))),
    ).ok().flatten())
}

pub(crate) fn parse_bounded_instruction_program_tokens(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
    if !tokens.iter().any(|token| token.kind == TokenKind::LBracket) { return Ok(None); }
    // Only a complete bracketed program is owned here; any other bracketed
    // sentence keeps its ordinary nested readers.
    Ok(crate::word_claims::speculative_keep_if(
        || read_bounded_instruction_program(&Document::new(tokens)),
        |result| matches!(result, Ok(Some(_))),
    ).ok().flatten())
}

/// Explicit lexical boundaries compose existing resolution-time instructions.
/// A local conditional gates only its bracketed body; following instructions
/// remain independent. This reader never infers a source keyword or card name.
fn read_bounded_instruction_program(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(colon) = bounded_program_colon(document.tokens) else { return Ok(None); };
    let header = &document.tokens[..colon];
    let words = crate::lexer::parser_token_word_refs(header);
    let plain = words == ["do", "the", "following"];
    let result_followup = crate::grammar::sentence_markers::parse_conditional_followup_tokens(header)
        .is_some_and(|followup| crate::lexer::parser_token_word_refs(
            crate::util::trim_edge_punctuation_tokens(followup.tail_tokens)) == ["do", "the", "following"]);
    let predicate = if plain || result_followup { None } else {
        if !header.first().is_some_and(|token| token.is_word("if")) { return Ok(None); }
        // This reader owns only a guard directly followed by its bounded
        // instruction head. An outer condition followed by another complete
        // player/object program belongs to the ordinary conditional reader.
        let Some(comma) = header.iter().position(|token| token.kind == TokenKind::Comma) else { return Ok(None); };
        if crate::lexer::parser_token_word_refs(&header[comma+1..]) != ["do", "the", "following"] { return Ok(None); }
        Some(crate::grammar::filters::parse_condition_predicate_lexed(&header[1..comma])?)
    };
    if !document.tokens.get(colon+1).is_some_and(|token| token.kind == TokenKind::LBracket) { return Ok(None); }
    let (body, mut tail) = bounded_program_payload(document, colon)?;
    let effects = parse_bounded_program(body)?;
    let mut if_false = Vec::new();
    if predicate.is_some() && tail.first().is_some_and(|token| token.is_word("otherwise")) {
        if let Some(otherwise_colon) = bounded_program_colon(tail)
            && crate::lexer::parser_token_word_refs(&tail[..otherwise_colon]) == ["otherwise", "do", "the", "following"]
            && tail.get(otherwise_colon + 1).is_some_and(|token| token.kind == TokenKind::LBracket)
        {
            let (otherwise_body, remainder) = bounded_program_payload(&Document::new(tail), otherwise_colon)?;
            if_false = parse_bounded_program(otherwise_body)?;
            if if_false.is_empty() { return Err(CardTextError::ParseError("bounded otherwise program cannot be empty".into())); }
            crate::word_claims::claim_tokens(&tail[..=otherwise_colon]);
            tail = remainder;
        }
    }
    if effects.is_empty() { return Err(CardTextError::ParseError("bounded instruction program cannot be empty".into())); }
    if plain || result_followup { crate::word_claims::claim_tokens(&document.tokens[..=colon]); }
    else {
        crate::word_claims::claim_tokens(&header[..1]);
        crate::word_claims::claim_tokens(&document.tokens[header.len()-4..=colon]);
    }
    let mut output = if result_followup {
        vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did, effects,
        })]
    } else { match predicate {
        Some(predicate) => vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate, if_true: effects, if_false,
        })],
        None => effects,
    }};
    bounded_program_followups(&mut output, tail)?;
    Ok(Some(output))
}

#[cfg(test)]
mod bounded_instruction_scope_tests {
    use super::*;
    #[test]
    fn conditional_program_keeps_complete_body_and_independent_tail() {
        let tokens=crate::lexer::lex_line("If you control an artifact, do the following: [You gain 2 life; Draw a card]. You lose 1 life.",0).unwrap();
        let output=read_bounded_instruction_program(&Document::new(&tokens)).unwrap().unwrap();
        assert_eq!(output.len(),2);
        let EffectAst::Conditionals(ConditionalEffectAst::Conditional{if_true,if_false,..})=&output[0] else{panic!("{output:?}")};
        assert_eq!(if_true.len(),2);assert!(if_false.is_empty());
    }
    #[test]
    fn malformed_or_unbounded_headers_cannot_claim_other_programs(){
        for text in ["Do the following: You gain 1 life.","If you control an artifact, draw a card: [You gain 1 life]."]{
            let tokens=crate::lexer::lex_line(text,0).unwrap();assert!(read_bounded_instruction_program(&Document::new(&tokens)).unwrap().is_none());
        }
        let tokens=crate::lexer::lex_line("Do the following: [You gain 1 life",0).unwrap();
        assert!(read_bounded_instruction_program(&Document::new(&tokens)).is_err());
    }
}

#[cfg(test)]mod bounded_otherwise_program_tests{use super::*;
 #[test]fn both_branches_are_bounded_and_independent_followup_stays_outside(){let tokens=crate::lexer::lex_line("If you control an artifact, do the following: [You gain 2 life; Draw a card]. Otherwise, do the following: [You lose 2 life; Draw a card]. You gain 3 life.",0).unwrap();let output=read_bounded_instruction_program(&Document::new(&tokens)).unwrap().unwrap();assert_eq!(output.len(),2);let EffectAst::Conditionals(ConditionalEffectAst::Conditional{if_true,if_false,..})=&output[0] else{panic!("missing conditional")};assert_eq!(if_true.len(),2);assert_eq!(if_false.len(),2);}
}

#[cfg(test)]
#[test]
fn named_library_program_keeps_closed_body_and_conjoined_followup_separate() {
    for operand in ["the top two cards of your library", "those cards and the top card of each opponent's library"] {
        let text = format!("Perform the archive action on {operand} as follows: [Exile the proposed objects; You gain 1 life] and each opponent loses 1 life.");
        let tokens = crate::lexer::lex_line(&text, 0).unwrap();
        let effects = read_named_action_program(&Document::new(&tokens)).unwrap().unwrap();
        // Capture effects precede the one declared program, followed by the
        // independent loss instruction. The gain belongs only to the body.
        let program = effects.iter().position(|effect| matches!(effect,
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::KeywordActions(
                    crate::cards::builders::KeywordActionAst::PerformNamedAction { program: Some(_), .. }), .. }))).unwrap();
        assert!(program > 0, "the pure operand capture must precede the program");
        assert_eq!(effects.len(), program + 2, "exactly one independent sibling follows");
        let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: crate::cards::builders::SubjectVerbActionAst::KeywordActions(
                crate::cards::builders::KeywordActionAst::PerformNamedAction { program: Some(body), .. }), .. }) = &effects[program] else { unreachable!() };
        assert_eq!(body.len(), 2, "the two semicolon-separated instructions stay inside the program");
    }
    let tokens = crate::lexer::lex_line("Perform the archive action as follows: [You gain 1 life] you gain 2 life.", 0).unwrap();
    assert!(read_named_action_program(&Document::new(&tokens)).is_err(), "a missing separator remains invalid");
}

#[cfg(test)]
#[test]
fn player_counter_program_is_not_an_object_target_iterator() {
 for kind in ["corruption","restlessness","study"] {
  let text=format!("For each {kind} counter on you, do the following: You gain 1 life.");
  let tokens=crate::lexer::lex_line(&text,0).unwrap();
  assert!(read_foreach_following_program(&Document::new(&tokens)).unwrap().is_none());
  let effects=crate::clause_support::parse_effect_sentences_lexed(&tokens).unwrap();
  let [EffectAst::ForEach(ForEachEffectAst::RepeatEffects{count:crate::effect::Value::PlayerCounters(crate::target::PlayerFilter::You,_),effects})]=effects.as_slice() else{panic!("snapshot player-counter iteration: {effects:?}")};
  assert_eq!(effects.len(),1);
 }
}

#[cfg(test)]
mod quoted_optional_price_owner_tests {
    use super::*;
    #[test]
    fn quoted_composite_price_is_one_maypay_before_a_complete_bounded_result_branch() {
        let text="You may pay the cost \"Exile a creature card from your graveyard; Exile an instant or sorcery card from your graveyard\". If you do, do the following: [You gain 1 life. You gain 2 life.]";
        let tokens=crate::lexer::lex_line(text,0).unwrap();let document=Document::new(&tokens);
        assert_eq!(document.sentences.len(),2,"inner bounded statements cannot split the document");
        assert!(read_leading_player_may(&document).unwrap().is_none(),"the broad optional-action reader cannot shadow a typed whole-price owner");
        let effects=read_explicit_optional_cost_program(&document).unwrap().unwrap();
        let Some(EffectAst::Permissions(PermissionEffectAst::MayPay{effects:cost,..}))=effects.first() else{panic!("quoted price must stay an atomic optional payment")};
        assert_eq!(cost.len(),2,"both distinct required exile actions remain in the price");
        assert!(effects.iter().skip(1).any(|effect|matches!(effect,
            EffectAst::Conditionals(ConditionalEffectAst::IfResult{effects,..}) if effects.len()==2)),"the full paid-result branch retains both authored statements");
    }
    #[test]
    fn a_nested_paid_creation_program_keeps_its_trailing_entry_rider() {
        let text="Perform the example action with tokens created this way if any are created: [You may pay the cost \"Exile a creature card from your graveyard; Exile an instant or sorcery card from your graveyard\". If you do, do the following: [Create a X/X blue Zombie creature token.]]. The token enters tapped.";
        let tokens=crate::lexer::lex_line(text,0).unwrap();let document=Document::new(&tokens);
        assert_eq!(document.sentences.len(),2);
        let effects=read_named_action_program(&document).unwrap().unwrap();
        assert_eq!(effects.len(),1,"the creation entry rider belongs inside the complete named program");
        assert_eq!(named_program_creation_count(&effects),1);
        fn tapped_creation_count(effects: &[EffectAst]) -> usize {
            let mut count = 0;
            for effect in effects {
                if matches!(effect, EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::Tokens(
                        crate::cards::builders::TokenActionAst::CreateTokenWithMods { tapped: true, .. }
                        | crate::cards::builders::TokenActionAst::CreateTokenCopy { enters_tapped: true, .. }
                        | crate::cards::builders::TokenActionAst::CreateTokenCopyFromSource { enters_tapped: true, .. }), .. })) { count += 1; }
                crate::effect_ast_traversal::for_each_nested_effects(effect, true, |body| count += tapped_creation_count(body));
            }
            count
        }
        assert_eq!(tapped_creation_count(&effects),1,"the exact sole creation has its entry rider folded before entry");
    }
    #[test]
    fn ordinary_quoted_optional_actions_keep_their_original_owner() {
        let tokens=crate::lexer::lex_line("You may have target creature gain \"Whenever this creature attacks, draw a card.\" until end of turn.",0).unwrap();let document=Document::new(&tokens);
        assert!(read_explicit_optional_cost_program(&document).unwrap().is_none());
        assert!(!super::super::bounded_source_programs::has_optional_cost_program_head(&tokens));
    }
}

#[cfg(test)]
mod optional_foreach_bounded_header_tests {
 use super::*;
 #[test]
 fn optional_bounded_program_belongs_inside_each_iteration_and_tail_stays_outside(){
  let tokens=crate::lexer::lex_line("For each creature you control, you may do the following: [You gain 1 life; You gain 2 life]. You gain 4 life.",0).unwrap();
  let effects=read_foreach_following_program(&Document::new(&tokens)).unwrap().unwrap();assert_eq!(effects.len(),2);
  let EffectAst::ForEach(ForEachEffectAst::ForEachObject{effects:body,..})=&effects[0] else{panic!("iteration owns optionality")};
  let [EffectAst::Permissions(PermissionEffectAst::MayByPlayer{player,effects:optional_body})]=body.as_slice() else{panic!("one complete optional body per subject")};
  assert_eq!(player,&crate::cards::builders::PlayerAst::You);assert_eq!(optional_body.len(),2);
 }
 #[test]
 fn optionality_does_not_change_the_completed_exile_receipt_subject(){
  let read=|suffix:&str|{let tokens=crate::lexer::lex_line(&format!("For each card exiled this way, {suffix}: [You gain 1 life]"),0).unwrap();read_foreach_following_program(&Document::new(&tokens)).unwrap().unwrap()};
  let mandatory=read("do the following");let optional=read("you may do the following");
  let [EffectAst::ForEach(ForEachEffectAst::ForEachObject{filter:mandatory_filter,..})]=mandatory.as_slice() else{panic!("existing captured subject")};
  let [EffectAst::ForEach(ForEachEffectAst::ForEachObject{filter:optional_filter,..})]=optional.as_slice() else{panic!("same captured subject")};
  assert_eq!(mandatory_filter,optional_filter);
 }
 #[test]
 fn incomplete_optional_headers_and_empty_programs_do_not_gain_a_foreach_reading(){
  for text in ["For each creature you control, you may {1} do the following: [You gain 1 life]", "For each creature you control, you may 2 do the following: [You gain 1 life]", "For each creature you control, you may \"do\" the following: [You gain 1 life]", "For each creature you control, you may do following: [You gain 1 life]","For each creature you control, you might do the following: [You gain 1 life]","For each creature you control, you may do the following tomorrow: [You gain 1 life]"]{
   let tokens=crate::lexer::lex_line(text,0).unwrap();assert!(read_foreach_following_program(&Document::new(&tokens)).unwrap().is_none(),"{text}");
  }
  let tokens=crate::lexer::lex_line("For each creature you control, you may do the following: []",0).unwrap();assert!(read_foreach_following_program(&Document::new(&tokens)).is_err());
 }
}

// A complete local choice/consumer pair can lower to ordinary literal grant
// alternatives. No persistent map can leak a stale choice from another domain.
fn read_chosen_color_or_type_protection(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>,CardTextError> {
    let [choice,grant]=document.sentences.as_slice() else {return Ok(None)};
    if crate::lexer::parser_token_word_refs(choice)!=["choose","a","color","or","card","type"] {return Ok(None)}
    let grant=crate::util::trim_edge_punctuation_tokens(grant);
    let Some(verb)=grant.iter().position(|token|token.is_any_word(&["gains","gain"])) else {return Ok(None)};
    if crate::lexer::parser_token_word_refs(&grant[verb..])!=["gains","protection","from","the","chosen","color","or","type","until","end","of","turn"] {return Ok(None)}
    let target=crate::util::parse_target_phrase(&grant[..verb])?;
    let mut protections=vec![
        crate::ability::ProtectionFrom::Color(crate::color::ColorSet::WHITE),
        crate::ability::ProtectionFrom::Color(crate::color::ColorSet::BLUE),
        crate::ability::ProtectionFrom::Color(crate::color::ColorSet::BLACK),
        crate::ability::ProtectionFrom::Color(crate::color::ColorSet::RED),
        crate::ability::ProtectionFrom::Color(crate::color::ColorSet::GREEN),
    ];
    protections.extend(ironsmith_core::ChooseCardTypeEffect::all_card_types().iter().copied().map(crate::ability::ProtectionFrom::CardType));
    let abilities=protections.into_iter().map(|protection|crate::cards::builders::GrantedAbilityAst::StaticAbility(Box::new(
        crate::cards::builders::StaticAbilityAst::Static(crate::model::CompilerStaticAbilityCore::protection(protection))
    ))).collect();
    crate::word_claims::claim_tokens(choice);
    crate::word_claims::claim_tokens(&grant[verb..]);
    Ok(Some(vec![EffectAst::subject_verb_grant_abilities_choice_to_target(target,abilities,crate::effect::Until::EndOfTurn)]))
}

#[cfg(test)]
mod color_or_type_protection_program_tests {
    use super::*;
    #[test]
    fn union_choice_uses_one_existing_resolution_grant_with_one_target() {
        let tokens=crate::lexer::lex_line("Choose a color or card type. Target permanent gains protection from the chosen color or type until end of turn.",0).unwrap();
        let effects=read_chosen_color_or_type_protection(&Document::new(&tokens)).unwrap().unwrap();
        let [EffectAst::SubjectVerb(subject)]=effects.as_slice() else {panic!("one common target owner")};
        let crate::cards::builders::SubjectVerbActionAst::Grants(crate::cards::builders::GrantActionAst::GrantAbilitiesChoiceToTarget{abilities,duration,..})=&subject.action else {panic!("existing resolution-time choice")};
        assert_eq!(abilities.len(),5+ironsmith_core::ChooseCardTypeEffect::all_card_types().len());
        assert_eq!(*duration,crate::effect::Until::EndOfTurn);
        let extra=crate::lexer::lex_line("Choose a color or card type. Target permanent gains protection from the chosen color or type until end of turn. Draw a card.",0).unwrap();
        assert!(read_chosen_color_or_type_protection(&Document::new(&extra)).unwrap().is_none());
    }
}
