//! The readings of one uncoordinated effect chain: the typed chain shapes the
//! carry parser knew before composing subject/verb primitives ("you may cast
//! it", the consult traversals, "any player may ...", the coordinated "and"
//! segments, ...). Formerly a first-match ladder in `chain_carry`; every
//! reading runs, resolved by rank while the overlaps are measured.

use crate::cards::builders::PermissionEffectAst;
use crate::cards::builders::ForEachEffectAst;
use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
    resolve_registry_candidates,
};

/// One effect chain, read as a whole, with the composition readings that claim
/// it computed once on demand.
pub(super) struct Chain<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) claims: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl Chain<'_> {
    /// Whether the composition reading `id` reads this chain; a specific
    /// reading the ladder ranked after that rung admits the chain only when it
    /// does not.
    fn claimed_by(&self, id: &'static str) -> bool {
        if let Some(claimed) = self.claims.borrow().get(id) {
            return *claimed;
        }
        let claimed = CHAIN_COMPOSITION
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.claims.borrow_mut().insert(id, claimed);
        claimed
    }

    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = CHAIN_READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the chain.
    fn outcome(
        &self,
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(effects)) => ParseOutcome::matched(effects, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("chain-reading"),
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
    admits: fn(&Chain<'_>) -> bool,
    read: fn(&Chain<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const CHAIN_REGISTRY: RuleId = RuleId::new("chain-reading-registry");
pub(super) const CHAIN_COMPOSITION_REGISTRY: RuleId = RuleId::new("chain-composition-registry");

/// The readings, in the order they were ranked.
const CHAIN_READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("sentence-each-player-may-reveal-selected-cards-in-their-hand"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_sentence_each_player_may_reveal_selected_cards_in_their_hand(input))
        },
    },
    Reading {
        id: RuleId::new("named-token-appositive"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_named_token_appositive(input)),
    },
    Reading {
        id: RuleId::new("cast-or-play-tagged-permission"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cast_or_play_tagged_permission(input)),
    },
    Reading {
        id: RuleId::new("inline-looked-card-partition-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_inline_looked_card_partition_chain(input)),
    },
    Reading {
        id: RuleId::new("shuffle-graveyard-into-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_shuffle_graveyard_into_library(input)),
    },
    Reading {
        id: RuleId::new("mill-then-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_mill_then_followup(input)),
    },
    Reading {
        id: RuleId::new("leading-then-shuffle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_then_shuffle(input)),
    },
    Reading {
        id: RuleId::new("reveal-source-exiled-permanents-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_reveal_source_exiled_permanents_sentence(input)),
    },
    Reading {
        id: RuleId::new("may-cast-it"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            // A cast permission the tagged-permission reading claims in full is that reading's.
            true
                && !(clause_may_contain_cast_or_play_permission_lexed(tokens) && matches!(parse_cast_or_play_tagged_clause(tokens), Ok(Some(_))) && matches!(immediate_tagged_permission_spec(tokens), Ok(true)))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("cast-or-play-tagged-permission")
                // Composition readings the ladder ranked above this one read the chain first.
                && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_may_cast_it(input)),
    },
    Reading {
        id: RuleId::new("for-each-exiled-this-way"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_for_each_exiled_this_way(input)),
    },
    Reading {
        id: RuleId::new("for-each-object-effect-chain"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_for_each_object_effect_chain(input)),
    },
    Reading {
        id: RuleId::new("attacking-doesnt-tap-if-source-untapped"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_attacking_doesnt_tap_if_source_untapped(input)),
    },
    Reading {
        id: RuleId::new("additional-phases"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_additional_phases(input)),
    },
    Reading {
        id: RuleId::new("tap-object-union-then"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_tap_object_union_then(input)),
    },
    Reading {
        id: RuleId::new("may-have-any-number-tagged-phase-out"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_may_have_any_number_tagged_phase_out(input)),
    },
    Reading {
        id: RuleId::new("destroy-then-temporary-cant-attack-block-chain"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_destroy_then_temporary_cant_attack_block_chain(input)),
    },
    Reading {
        id: RuleId::new("exile-library-then-shuffle-graveyard-chain"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_exile_library_then_shuffle_graveyard_chain(input)),
    },
    Reading {
        id: RuleId::new("each-player-may-discard-hand-and-draw"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_each_player_may_discard_hand_and_draw(input)),
    },
    Reading {
        id: RuleId::new("generic-top-cards-put-counted-into-hand-rest-graveyard"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            // A looked-card partition chain is the partition reading's.
            true
                && parse_inline_looked_card_partition_chain(tokens).is_none()
                // Composition readings the ladder ranked above this one read the chain first.
                && !input.claimed_by("comma-then-chain")
        },
        read: |input| {
            input.outcome(read_generic_top_cards_put_counted_into_hand_rest_graveyard(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("meld-them-into"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_meld_them_into(input)),
    },
    Reading {
        id: RuleId::new("any-player-or-opponent-may"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_any_player_or_opponent_may(input)),
    },
    Reading {
        id: RuleId::new("any-player-may-sacrifice"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_any_player_may_sacrifice(input)),
    },
    Reading {
        id: RuleId::new("any-player-may-have-source-deal-damage"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
        },
        read: |input| input.outcome(read_any_player_may_have_source_deal_damage(input)),
    },
    Reading {
        id: RuleId::new("generic-consult-reveal-until-battlefield-bottom"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
                && !input.claimed_by("trailing-if-player-may")
                && !input.claimed_by("player-may")
                && !input.claimed_by("leading-may")
        },
        read: |input| input.outcome(read_generic_consult_reveal_until_battlefield_bottom(input)),
    },
    Reading {
        id: RuleId::new("consult-traversal-with-inline-followup"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            // A consult that reveals until a match and puts the rest on the bottom is the generic consult reading's.
            true
                && !(split_leading_result_prefix_lexed(tokens).is_none() && matches!(super::super::dispatch_inner::parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(tokens), Ok(Some(_))))
                // Composition readings the ladder ranked above this one read the chain first.
                && !input.claimed_by("comma-then-chain")
                && !input.claimed_by("trailing-if-player-may")
                && !input.claimed_by("player-may")
                && !input.claimed_by("leading-may")
        },
        read: |input| input.outcome(read_consult_traversal_with_inline_followup(input)),
    },
    Reading {
        id: RuleId::new("consult-traversal"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            // A consult with an inline followup is the followup reading's.
            true
                && !matches!(super::super::consult_family::parse_consult_traversal_with_inline_followup(tokens), Ok(Some(_)))
                // A consult that reveals until a match and puts the rest on the bottom is the generic consult reading's.
                && !(split_leading_result_prefix_lexed(tokens).is_none() && matches!(super::super::dispatch_inner::parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(tokens), Ok(Some(_))))
                // Composition readings the ladder ranked above this one read the chain first.
                && !input.claimed_by("comma-then-chain")
                && !input.claimed_by("trailing-if-player-may")
                && !input.claimed_by("player-may")
                && !input.claimed_by("leading-may")
        },
        read: |input| input.outcome(read_consult_traversal(input)),
    },
    Reading {
        id: RuleId::new("tap-or-untap-all-choice"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
                && !input.claimed_by("trailing-if-player-may")
                && !input.claimed_by("player-may")
                && !input.claimed_by("leading-may")
        },
        read: |input| input.outcome(read_tap_or_untap_all_choice(input)),
    },
    Reading {
        id: RuleId::new("unless-payment-choice"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
                && !input.claimed_by("trailing-if-player-may")
                && !input.claimed_by("player-may")
                && !input.claimed_by("leading-may")
        },
        read: |input| input.outcome(read_unless_payment_choice(input)),
    },
    Reading {
        id: RuleId::new("or-action-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
                && !input.claimed_by("trailing-if-player-may")
                && !input.claimed_by("player-may")
                && !input.claimed_by("leading-may")
        },
        read: |input| input.outcome(read_or_action_clause(input)),
    },
    Reading {
        id: RuleId::new("cast-or-play-tagged-permission-late"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Composition readings the ladder ranked above this one read the chain first.
            true && !input.claimed_by("comma-then-chain")
                && !input.claimed_by("trailing-if-player-may")
                && !input.claimed_by("player-may")
                && !input.claimed_by("leading-may")
        },
        read: |input| input.outcome(read_cast_or_play_tagged_permission_late(input)),
    },
];

/// The composition readers: the general chain grammar (", then" chains, "<player>
/// may ...", coordinated "and" segments). They read what no specific reading
/// claims, in this order; the overlaps among them are measured.
const CHAIN_COMPOSITION: &[Reading] = &[
    Reading {
        id: RuleId::new("comma-then-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_comma_then_chain(input)),
    },
    Reading {
        id: RuleId::new("trailing-if-player-may"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_trailing_if_player_may(input)),
    },
    Reading {
        id: RuleId::new("player-may"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_may(input)),
    },
    Reading {
        id: RuleId::new("leading-may"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_may(input)),
    },
    Reading {
        id: RuleId::new("explicit-comma-then-boundary"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_explicit_comma_then_boundary(input)),
    },
    Reading {
        id: RuleId::new("coordinated-and-segments"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_coordinated_and_segments(input)),
    },
];

/// The chain's reading: a specific reading if one claims it, else the first
/// composition reader that does. A specific reading's committed error stands
/// only when composition has no reading either.
pub(super) fn read_chain(input: &Chain<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    match collect(CHAIN_REGISTRY, CHAIN_READINGS, input) {
        ParseOutcome::Match(matched) => ParseOutcome::Match(matched),
        ParseOutcome::NoMatch => collect(CHAIN_COMPOSITION_REGISTRY, CHAIN_COMPOSITION, input),
        ParseOutcome::Error(specific) => {
            match collect(CHAIN_COMPOSITION_REGISTRY, CHAIN_COMPOSITION, input) {
                ParseOutcome::NoMatch => ParseOutcome::Error(specific),
                outcome => outcome,
            }
        }
    }
}

fn collect(
    registry: RuleId,
    readings: &[Reading],
    input: &Chain<'_>,
) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in readings {
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
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{registry}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    // The specific readings are strict: two different readings of one chain
    // are an ambiguity error. The composition tier is ranked while its
    // overlaps are measured.
    let outcome = if registry == CHAIN_REGISTRY {
        resolve_registry_candidates(registry, distinct, diagnostics)
    } else {
        resolve_ranked_candidates(registry, distinct, diagnostics, || {
            crate::lexer::parser_token_word_refs(input.tokens).join(" ")
        })
    };
    match &outcome {
        ParseOutcome::Match(matched) => {
            crate::parse_trace::event(format!("{registry}: {} read the input", matched.value.rule));
        }
        ParseOutcome::Error(diagnostic) => {
            crate::parse_trace::event(format!("{registry}: error: {}", diagnostic.message));
        }
        ParseOutcome::NoMatch => {}
    }
    outcome
}

fn immediate_tagged_permission_spec(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    Ok(matches!(
        parse_permission_clause_spec_lexed(tokens)?,
        Some(PermissionClauseSpec::Tagged {
            lifetime: PermissionLifetime::Immediate,
            ..
        })
    ))
}

fn read_sentence_each_player_may_reveal_selected_cards_in_their_hand(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_sentence_each_player_may_reveal_selected_cards_in_their_hand(
        SubjectVerbPrimitiveClause::new(tokens),
    )? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_named_token_appositive(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let named_token_appositive = tokens.first().is_some_and(|token| token.is_word("create"))
        && crate::slice_primitives::select_position(tokens, OwnedLexToken::is_comma).is_some_and(
            |comma| {
                !tokens[..comma].iter().any(|token| token.is_word("token"))
                    && tokens[comma + 1..]
                        .iter()
                        .any(|token| token.is_word("token"))
            },
        );
    if named_token_appositive {
        return Ok(Some(vec![super::super::creation_handlers::parse_create(
            tokens, None,
        )?]));
    }
    Ok(None)
}
fn read_cast_or_play_tagged_permission(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A complete tagged permission may include its own coordinated
    // any-color mana rider. Preserve that atomic grammar before the
    // generic leading-`may` path removes the first modal subject and
    // leaves the rider as a verb-less `spend mana ...` clause.
    if clause_may_contain_cast_or_play_permission_lexed(tokens)
        && let Some(effect) = parse_cast_or_play_tagged_clause(tokens)?
    {
        if immediate_tagged_permission_spec(tokens)?
            && let Some(player) = parse_leading_player_may_lexed(tokens)
        {
            return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player,
                effects: vec![effect],
            })]));
        }
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_inline_looked_card_partition_chain(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_inline_looked_card_partition_chain(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_shuffle_graveyard_into_library(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A hand/graveyard-into-library shuffle owns its complete clause even
    // when a participant loop already stripped the subject; coordination
    // would otherwise sever the zone list from the shuffle verb. An optional
    // shuffle keeps its `may` scope through the may-aware routes.
    if parse_leading_player_may_lexed(tokens).is_none()
        && !chain_grammar::starts_with_may_tokens(tokens)
        && super::super::super::grammar::effects::parse_shuffle_graveyard_shape_lexed(tokens)
            .is_some_and(|shape| shape.has_hand_clause)
        && let Some(effects) =
            super::super::search_library::parse_shuffle_graveyard_into_library_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_mill_then_followup(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
    if let [mill_tokens, followup_tokens] = comma_then_segments.as_slice() {
        let inline_sentences = [
            SentenceInput::from_lexed(mill_tokens),
            SentenceInput::from_lexed(followup_tokens),
        ];
        if let Some(effects) =
                super::super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_mill_then_may_put_from_among_into_hand(
                    &inline_sentences,
                    0,
                )?
            {
                return Ok(Some(effects));
            }
    }
    Ok(None)
}
fn read_leading_then_shuffle(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
    if let [leading_tokens, shuffle_tokens] = comma_then_segments.as_slice() {
        let leading_segments = split_segments_on_comma_effect_head_lexed(vec![leading_tokens]);
        if let [look_tokens, deployment_tokens] = leading_segments.as_slice() {
            let inline_sentences = [
                SentenceInput::from_lexed(look_tokens),
                SentenceInput::from_lexed(deployment_tokens),
                SentenceInput::from_lexed(shuffle_tokens),
            ];
            if let Some(effects) =
                super::super::sequence_rules::try_parse_document_program(&inline_sentences, 0)?
                    .filter(|matched| matched.consumed_sentences == inline_sentences.len())
                    .map(|matched| matched.effects)
            {
                return Ok(Some(effects));
            }
        }
    }
    Ok(None)
}
fn read_reveal_source_exiled_permanents_sentence(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_reveal_source_exiled_permanents_sentence_lexed(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_comma_then_chain(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let comma_then_segments = split_segments_on_comma_then_lexed(vec![tokens]);
    // Once the conservative typed splitter has proved a real `, then`
    // boundary, route the complete chain before any prefix-tolerant
    // specialist can claim only its final verb. The inner chain pass retains
    // carried players, result values, tags, and authored ordering across the
    // separately parsed arms.
    if comma_then_segments.len() > 1 {
        return parse_effect_chain_inner_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_may_cast_it(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // An immediate "you may cast/play" instruction is an optional action,
    // not a persistent permission. Claim it before the generic leading-may
    // path strips `may` while probing broader cast-permission surfaces.
    if let Some(spec) = parse_may_cast_it_sentence(tokens) {
        return Ok(Some(vec![build_may_cast_tagged_effect(&spec)]));
    }
    Ok(None)
}
fn read_for_each_exiled_this_way(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_for_each_exiled_this_way_sentence(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_for_each_object_effect_chain(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_for_each_object_effect_chain_shape(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_attacking_doesnt_tap_if_source_untapped(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) = super::super::super::grammar::effects::sentence_predicate_shapes::
            parse_attacking_doesnt_tap_if_source_untapped_tokens(tokens)
        {
            let filter = parse_object_filter(shape.affected_tokens, false)?;
            return Ok(Some(vec![
                EffectAst::subject_verb_grant_abilities_all_dynamically_with_condition(
                    filter,
                    vec![crate::cards::builders::GrantedAbilityAst::KeywordAction(
                        Box::new(crate::payload::KeywordAction::Vigilance),
                    )],
                    Until::EndOfCombat,
                    PredicateAst::Source(SourcePredicateAst::SourceIsUntapped),
                ),
            ]));
        }
    Ok(None)
}
fn read_additional_phases(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A phase-insertion clause has no ordinary subject/verb head ("there is
    // an additional combat phase").  Conditional and labeled effect bodies
    // enter through the chain parser, so route the already-typed phase shape
    // before generic verb discovery as well as at the sentence entrypoint.
    if let Some(shape) = parse_additional_phases_shape(tokens) {
        return Ok(Some(vec![EffectAst::subject_verb_additional_phases(
            shape.phases,
        )]));
    }
    Ok(None)
}
fn read_tap_object_union_then(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Preserve coordinated object operands as one tap result so a subsequent
    // "them" refers to the entire affected set, not only the first operand.
    if let Some(shape) = parse_tap_object_union_then_tokens(tokens) {
        let first = parse_target_phrase(shape.first_target_tokens)?;
        let first_filter = match first {
            TargetAst::Source(_) => ObjectFilter::source(),
            TargetAst::Object(filter, None, _) => filter,
            TargetAst::Tagged(tag, _) => ObjectFilter::tagged(tag),
            _ => {
                return Err(CardTextError::ParseError(
                    "coordinated tap operand must be a non-target object reference".to_string(),
                ))
                .map(Some);
            }
        };
        let all_filter = parse_object_filter(shape.all_filter_tokens, false)?;
        let mut union = ObjectFilter::default();
        union.any_of = vec![first_filter, all_filter];
        let mut effects = vec![EffectAst::subject_verb_tap_all(union)];
        effects.extend(parse_effect_chain_lexed(shape.followup_tokens)?);
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_may_have_any_number_tagged_phase_out(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_may_have_any_number_tagged_phase_out_lexed(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_destroy_then_temporary_cant_attack_block_chain(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_destroy_then_temporary_cant_attack_block_chain_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_library_then_shuffle_graveyard_chain(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_exile_library_then_shuffle_graveyard_chain_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_each_player_may_discard_hand_and_draw(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) =
        sacrifice_discard_grammar::parse_each_player_may_discard_hand_and_draw_tokens(tokens)
    {
        let optional_effects = vec![
            EffectAst::subject_verb_discard_hand(PlayerAst::That),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::That,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                    count: shape.draw_count,
                }),
            ),
        ];
        return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::Permissions(PermissionEffectAst::May {
                effects: optional_effects,
            })],
        })]));
    }
    Ok(None)
}
fn read_generic_top_cards_put_counted_into_hand_rest_graveyard(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::dispatch_inner::parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(tokens)
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_meld_them_into(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(result_tokens) = chain_grammar::parse_meld_them_into_tokens(tokens) {
        let result_words = token_word_refs(result_tokens);
        if result_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing meld result name (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )))
            .map(Some);
        }
        return Ok(Some(vec![EffectAst::subject_verb_meld(
            result_words.join(" "),
            false,
            false,
        )]));
    }
    Ok(None)
}
fn read_any_player_or_opponent_may(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // "Any player may pay ..." is a turn-order offer that ends when one
    // player accepts, rather than a single optional action performed by an
    // arbitrary player. Keep the payer and payer-relative dynamic values
    // inside the existing sequential AnyPlayerMay scope.
    if let Some(player) = parse_leading_player_may_lexed(tokens)
        && matches!(player, PlayerAst::Any | PlayerAst::Opponent)
    {
        let stripped = remove_through_first_word(tokens);
        let stripped = crate::util::trim_edge_punctuation_tokens(&stripped);
        if stripped.first().is_some_and(|token| token.is_word("pay")) {
            let payment = super::super::zone_handlers::parse_pay(
                crate::util::trim_edge_punctuation_tokens(&stripped[1..]),
                Some(crate::cards::builders::SubjectAst::Player(PlayerAst::That)),
            )?;
            return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay {
                players: if player == PlayerAst::Opponent {
                    PlayerFilter::Opponent
                } else {
                    PlayerFilter::Any
                },
                effects: vec![payment],
            })]));
        }
    }
    Ok(None)
}
fn read_any_player_may_sacrifice(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) = parse_any_player_may_sacrifice_shape(tokens) {
        let sacrifice = super::super::zone_handlers::parse_sacrifice(
            shape.action_tokens,
            Some(crate::cards::builders::SubjectAst::Player(PlayerAst::That)),
            None,
        )?;
        return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay {
            players: shape.players,
            effects: vec![sacrifice],
        })]));
    }
    Ok(None)
}
fn read_any_player_may_have_source_deal_damage(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Claim the complete causative damage offer before the broad leading-may
    // handler strips its participant and lowers only the inner damage. The
    // specialist distinguishes sequential "any player/opponent" offers from
    // a single targeted player's choice.
    if let Some(effects) =
        super::super::dispatch_inner::parse_any_player_may_have_source_deal_damage(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_trailing_if_player_may(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let leading_scope = chain_grammar::parse_leading_chain_scope_tokens(tokens);
    let starts_with_each_opponent =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachOpponent);
    let starts_with_each_player =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachPlayer);
    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) {
        if let Some(player) = parse_leading_player_may_lexed(trailing_if.leading_tokens) {
            let mut stripped = remove_through_first_word(trailing_if.leading_tokens);
            if let Some(rest) = chain_grammar::strip_leading_choose_to_tokens(&stripped) {
                stripped = rest.to_vec();
            }
            if let Some(rest) = chain_grammar::strip_leading_have_tokens(&stripped) {
                stripped = rest.to_vec();
            }
            let mut effects = parse_effect_chain_lexed(&stripped)?;
            for effect in &mut effects {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                predicate: trailing_if.predicate,
                effects: vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects })],
            })]));
        }

        if chain_grammar::starts_with_may_tokens(trailing_if.leading_tokens)
            && !starts_with_each_opponent
            && !starts_with_each_player
        {
            let stripped = remove_first_word(trailing_if.leading_tokens);
            let effects = parse_effect_chain_lexed(&stripped)?;
            return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                predicate: trailing_if.predicate,
                effects: vec![EffectAst::Permissions(PermissionEffectAst::May { effects })],
            })]));
        }
    }
    Ok(None)
}
fn read_player_may(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(player) = parse_leading_player_may_lexed(tokens) {
        let mut stripped = remove_through_first_word(tokens);
        if let Some(rest) = chain_grammar::strip_leading_choose_to_tokens(&stripped) {
            stripped = rest.to_vec();
        }
        if let Some(rest) = chain_grammar::strip_leading_have_tokens(&stripped) {
            stripped = rest.to_vec();
        }
        if let Some(mut permission) = parse_additional_land_plays_clause_lexed(&stripped)? {
            bind_implicit_player_context(&mut permission, player);
            return Ok(Some(vec![permission]));
        }
        let stripped_words = crate::lexer::parser_token_word_refs(&stripped);
        let has_copy_exception =
            crate::slice_primitives::select_last_position(&stripped_words, |word| {
                matches!(*word, "become" | "becomes")
            })
            .is_some_and(|become_word_idx| {
                let view = TokenWordView::new(&stripped);
                let body_start = view
                    .map_word_or_end_to_token_boundary(become_word_idx + 1)
                    .unwrap_or(stripped.len());
                super::super::super::grammar::effects::become_shapes::parse_become_rest_shape(
                    &stripped[body_start..],
                )
                .copy_exception
                .is_some()
            });
        let mut effects = if has_copy_exception {
            super::super::parse_effect_sentence_lexed(&stripped)?
        } else {
            parse_effect_chain_lexed(&stripped)?
        };
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects })]));
            }
            return Ok(Some(effects));
        }
        if has_any_number_of_times_suffix(&stripped) && is_repeatable_optional_payment(&effects) {
            return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                effects: vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects })],
                continue_effect_index: 0,
                continue_predicate: crate::cards::builders::IfResultPredicate::Did,
            })]));
        }
        return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects })]));
    }
    Ok(None)
}
fn read_leading_may(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let leading_scope = chain_grammar::parse_leading_chain_scope_tokens(tokens);
    let starts_with_each_opponent =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachOpponent);
    let starts_with_each_player =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachPlayer);
    if chain_grammar::starts_with_may_tokens(tokens)
        && !starts_with_each_opponent
        && !starts_with_each_player
    {
        let stripped = remove_first_word(tokens);
        if let Some(permission) = parse_additional_land_plays_clause_lexed(&stripped)? {
            return Ok(Some(vec![permission]));
        }
        let effects = parse_effect_chain_lexed(&stripped)?;
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::May { effects })]));
            }
            return Ok(Some(effects));
        }
        if has_any_number_of_times_suffix(&stripped) && is_repeatable_optional_payment(&effects) {
            return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                effects: vec![EffectAst::Permissions(PermissionEffectAst::May { effects })],
                continue_effect_index: 0,
                continue_predicate: crate::cards::builders::IfResultPredicate::Did,
            })]));
        }
        return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::May { effects })]));
    }
    Ok(None)
}
fn read_generic_consult_reveal_until_battlefield_bottom(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The broad consult recognizer intentionally accepts a traversal prefix.
    // Claim the complete inline consult/disposition program first so a result-
    // prefixed clause does not silently lose its battlefield move and library
    // remainder after the traversal.
    if split_leading_result_prefix_lexed(tokens).is_none()
            && let Some(effects) =
            super::super::dispatch_inner::parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(
                tokens,
            )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_consult_traversal_with_inline_followup(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A consult traversal can continue after its stop condition in the same
    // sentence. Preserve that complete procedure before the bare traversal
    // fallback intentionally returns only the consult action.
    if let Some(effects) =
        super::super::consult_family::parse_consult_traversal_with_inline_followup(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_consult_traversal(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Consult traversal has a `reveal` verb, but its `until` stop rule is
    // what gives the sentence its semantics. Claim the complete traversal
    // before the ordinary subject/verb registry lowers only the leading
    // reveal as a plain top-of-library effect.
    if let Some(parts) = super::super::consult_family::parse_consult_traversal_sentence(tokens)? {
        return Ok(Some(parts.effects));
    }
    Ok(None)
}
fn read_tap_or_untap_all_choice(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if chain_grammar::parse_tap_or_untap_all_choice_tokens(tokens) {
        let action_tokens = remove_first_word(tokens);
        return Ok(Some(vec![super::super::zone_handlers::parse_tap(
            &action_tokens,
        )?]));
    }
    Ok(None)
}
fn read_unless_payment_choice(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // An `or` inside a grammar-proven trailing payment is cost structure,
    // not effect coordination. Route the complete clause through the typed
    // trailing-unless builder before the generic chain splitter sees either
    // alternative as a sibling action.
    if has_unless_payment_choice(tokens)? {
        return Ok(Some(vec![parse_effect_clause_lexed(tokens)?]));
    }
    Ok(None)
}
fn read_or_action_clause(input: &Chain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(unless_action) = parse_or_action_clause_lexed(tokens)? {
        return Ok(Some(vec![unless_action]));
    }
    Ok(None)
}
fn read_cast_or_play_tagged_permission_late(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if clause_may_contain_cast_or_play_permission_lexed(tokens)
        && let Some(effect) = parse_cast_or_play_tagged_clause(tokens)?
    {
        if immediate_tagged_permission_spec(tokens)?
            && let Some(player) = parse_leading_player_may_lexed(tokens)
        {
            return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player,
                effects: vec![effect],
            })]));
        }
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_explicit_comma_then_boundary(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Some specialized subject/verb parsers accept a valid leading clause
    // without requiring end-of-input. Split a genuine top-level conjunction
    // before entering that registry, otherwise a first arm such as `copy that
    // spell` or `deals damage` can silently consume the whole sentence and
    // drop the following action.
    if has_explicit_comma_then_boundary_lexed(tokens) {
        return parse_effect_chain_inner_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_coordinated_and_segments(
    input: &Chain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let split_segments = split_effect_chain_on_and_lexed(tokens);
    let executable_heads = split_segments
        .iter()
        .filter(|segment| super::super::lex_chain_helpers::segment_has_effect_head_lexed(segment))
        .count();
    let has_expandable_shared_verb_operand = split_segments
        .iter()
        .zip(split_segments.iter().skip(1))
        .any(|(left, right)| expand_missing_verb_segment_lexed(left, right).is_some());
    if split_leading_result_prefix_lexed(tokens).is_none()
        && split_segments.len() > 1
        && (executable_heads > 1 || has_expandable_shared_verb_operand)
    {
        return parse_effect_chain_inner_lexed(tokens).map(Some);
    }
    Ok(None)
}
