//! The document-level readings tried before the sentence loop: whole-document
//! shapes ("Choose one or more —", the fight programs, the flat statement
//! split, ...) and the single-sentence statements read complete. They were a
//! first-match ladder in `dispatch_entry`; they are a registry whose rules all
//! run on every document and must agree: the order they are written in no
//! longer defines the language.

use super::dispatch_entry::{
    parse_complete_become_statement, parse_complete_composable_fight_program,
    parse_complete_compound_gain_statement, parse_complete_create_statement,
    parse_complete_delegated_partition_program, parse_complete_get_pump_statement,
    parse_complete_investigate_statement, parse_complete_simple_subject_verb_sentence,
    parse_delegated_partition_program_prefix, parse_each_player_coin_face_sequence,
    parse_resolving_card_countered_exile_replacement,
    parse_single_put_counters_effect_chain, parse_temporary_per_blocker_tax, SentenceInput,
};
use super::dispatch_inner::trim_edge_punctuation;
use super::divvy::try_parse_divvy_sentence_sequence;
use super::sentence_helpers::*;
use super::SubjectVerbPrimitiveClause;
use crate::cards::builders::{CardTextError, EffectAst, IfResultPredicate, ConditionalEffectAst, PermissionEffectAst};
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
    fn outcome(&self, read: Result<Option<Vec<EffectAst>>, CardTextError>) -> ParseOutcome<Vec<EffectAst>> {
        match read {
            Ok(Some(effects)) => ParseOutcome::matched(effects, crate::util::span_from_tokens(self.tokens)),
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
        id: RuleId::new("resolving-card-countered-exile-replacement"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_resolving_card_countered_exile_replacement(document)),
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
        id: RuleId::new("complete-get-pump"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_get_pump(document)),
    },
    Reading {
        id: RuleId::new("complete-become"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_complete_become(document)),
    },
    Reading {
        id: RuleId::new("sentence-you-and-attacking-player-each-draw-and-lose"),
        head: HeadDiscriminator::Any,
        read: |document| document.outcome(read_sentence_you_and_attacking_player_each_draw_and_lose(document)),
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
        match (reading.read)(document).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
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

fn read_resolving_card_countered_exile_replacement(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effect) = parse_resolving_card_countered_exile_replacement(tokens) {
            return Ok(Some(vec![effect]));
        }
    Ok(None)
}
fn read_temporary_per_blocker_tax(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_choice_complement(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_exile_then_return_same_object(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_reveal_source_exiled_permanents(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
            return Ok(Some(super::chain_carry::preserve_coordinated_effect_chain_surface(tokens, effects)));
        }
    Ok(None)
}
fn read_each_player_may_discard_hand_and_draw(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_conditional_put_counters(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // This reader owns one conditional counter statement. Later sentences
    // (including Otherwise) belong to the document's control-flow composer.
    if document.sentences.len() != 1 {
        return Ok(None);
    }
    let tokens = document.tokens;
    if effect_grammar::split_conditional_sentence_family_head_lexed(tokens).is_some() {
            let words = crate::lexer::parser_token_word_refs(tokens);
            if crate::word_primitives::sequence_occurs(&words, &["put"])
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
fn read_emblem_payload(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if effect_grammar::emblem_shapes::parse_emblem_payload_tokens(tokens)
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
fn read_complete_investigate(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) = parse_complete_investigate_statement(tokens)? {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_complete_kicked_search_replacement_bundle(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) =
            super::bundle_rules::parse_complete_kicked_search_replacement_bundle(tokens)?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_complete_delegated_partition(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) = parse_complete_delegated_partition_program(tokens) {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_each_player_coin_face(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_delegated_partition_program_prefix(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let Some(effects) = parse_delegated_partition_program_prefix(&sentences)? {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_complete_composable_fight(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = document.tokens;
    if let Some(effects) = parse_complete_composable_fight_program(tokens)? {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_cant_effect(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
            && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
            && super::lex_chain_helpers::split_effect_chain_on_and_lexed(sentence).len() == 1
            && let Some(effects) = super::parse_cant_effect_sentence_lexed(sentence)?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_serial_target_pt_modifiers(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_compound_damage_fanout(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_complete_compound_gain(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
            && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
            && let Some(effects) = parse_complete_compound_gain_statement(sentence)?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_keyword_bundle_pump(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_effect_clause_with_trailing_if(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    let [sentence] = sentences.as_slice() else {
        return Ok(None);
    };
    // Two conditions joined by "and" are the conditional damage pair's; a run of them is the keyword bundle's.
    if sentence.iter().filter(|token| token.is_word("if")).count() >= 2 {
        return Ok(None);
    }
    // A pump-and-grant with a trailing condition is the compound gain statement's, which keeps the condition.
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some_and(|trailing| effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(trailing.leading_tokens).is_some() || effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(trailing.leading_tokens).is_some()) {
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
fn read_complete_get_pump(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_sentence_you_and_attacking_player_each_draw_and_lose(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_next_end_step_followups(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
            && effect_grammar::subject_verb_registry_shapes::parse_registry_next_end_step_shape(
                sentence,
            )
            .is_some()
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
fn read_for_each_target_players(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
fn read_for_each_participant(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
            member.head.is_some_and(|head| matches!(
                head.actor,
                effect_grammar::typed_clause_heads::ClauseActorHeadAst::Controller
                    | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Player
                    | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Iterated
            ))
        })
    {
        return Ok(None);
    }
    // A choice complement ("each player chooses ... then sacrifices the rest") is the complement statement's.
    if super::dispatch_inner::is_choice_complement_shape(sentence) {
        return Ok(None);
    }
    // "Each player may discard their hand and draw seven cards" is its own statement.
    if effect_grammar::sacrifice_discard_shapes::parse_each_player_may_discard_hand_and_draw_tokens(sentence).is_some() {
        return Ok(None);
    }
    // A loop with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some() {
        return Ok(None);
    }
    // A loop that opens with a creation ("each player creates ...") is the create statement's.
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence).is_some_and(|shape| {
        shape.participant_is_actor
            && shape
                .inner_tokens
                .first()
                .is_some_and(|token| token.is_any_word(&["create", "creates"]))
    }) {
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
fn read_complete_simple_subject_verb(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
    if let [sentence] = sentences.as_slice()
            && !sentence.iter().any(|token| token.kind == TokenKind::Quote)
            && let Some(effect) = parse_complete_simple_subject_verb_sentence(sentence)?
        {
            return Ok(Some(vec![effect]));
        }
    Ok(None)
}
fn read_leading_player_may(document: &Document<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &document.sentences;
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
            let mut optional_effects = super::dispatch_entry::parse_effect_sentences_lexed_inner(&stripped)?;
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
                        effects: super::dispatch_entry::parse_effect_sentences_lexed_inner(&continuation)?,
                    }));
                } else {
                    effects.extend(super::dispatch_entry::parse_effect_sentences_lexed_inner(sentence)?);
                }
            }
            return Ok(Some(effects));
        }
    Ok(None)
}
