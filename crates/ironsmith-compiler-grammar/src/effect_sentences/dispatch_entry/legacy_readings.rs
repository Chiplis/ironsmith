//! The document readings the legacy sentence path tried before composing the
//! sentences: token-creation followups, coin-face sequences, the typed effect
//! bundles, the single-sentence subject/verb programs. Formerly a first-match
//! ladder in `dispatch_entry`; every reading runs and the readings must agree.
use super::SubjectVerbPrimitiveClause;
use super::*;
use crate::cards::builders::{CardTextError, EffectAst};
use crate::grammar::effects as effect_grammar;
use crate::lexer::OwnedLexToken;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};
/// A document and its sentences.
pub(super) struct LegacyDocument<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) sentences: &'a [&'a [OwnedLexToken]],
}

impl LegacyDocument<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("legacy-document-registry-reading"),
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
    admits: fn(&LegacyDocument<'_>) -> bool,
    read: fn(&LegacyDocument<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("legacy-document-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("choose-then-each-other-becomes-copy"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_then_each_other_becomes_copy(input)),
    },
    Reading {
        id: RuleId::new("created-token-counter-kind-distribution-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_created_token_counter_kind_distribution_followup(input)),
    },
    Reading {
        id: RuleId::new("created-token-mill-counter-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_created_token_mill_counter_followup(input)),
    },
    Reading {
        id: RuleId::new("each-player-coin-face"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_each_player_coin_face(input)),
    },
    Reading {
        id: RuleId::new("exile-return-tagged-entry-counters"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_return_tagged_entry_counters(input)),
    },
    Reading {
        id: RuleId::new("counter-linked-land-subtype-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counter_linked_land_subtype_followup(input)),
    },
    Reading {
        id: RuleId::new("typed-effect-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_typed_effect_bundle(input)),
    },
    Reading {
        id: RuleId::new("repeated-counter-placement-coordination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_repeated_counter_placement_coordination(input)),
    },
    Reading {
        id: RuleId::new("sentence-each-player-return-with-additional-counter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_sentence_each_player_return_with_additional_counter(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("exile-top-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_top_sentence(input)),
    },
    Reading {
        id: RuleId::new("become-rest"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_become_rest(input)),
    },
    Reading {
        id: RuleId::new("leading-duration-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_duration_sentence(input)),
    },
    Reading {
        id: RuleId::new("search-library-slots-to-hand-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_search_library_slots_to_hand_bundle(input)),
    },
    Reading {
        id: RuleId::new("delayed-schedule-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_delayed_schedule_sentence(input)),
    },
    Reading {
        id: RuleId::new("generic-consult-reveal-until-battlefield-bottom"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_consult_reveal_until_battlefield_bottom(input)),
    },
    Reading {
        id: RuleId::new("trigger-line-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            // A delayed schedule or a duration-scoped "this turn" trigger is the delayed schedule's, not a recurring trigger.
            effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens)
                .is_none()
                && effect_grammar::delayed_sentence_shapes::parse_delayed_this_turn_shape(tokens)
                    .is_none()
        },
        read: |input| input.outcome(read_trigger_line_sentence(input)),
    },
    Reading {
        id: RuleId::new("zone-replacement"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_zone_replacement(input)),
    },
    Reading {
        id: RuleId::new("conditional-sentence-family"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let words = crate::lexer::parser_token_word_refs(input.tokens);
            // A consult traversal ("reveal cards from the top of your library until ...") is the consult reader's.
            let consult =
                crate::word_primitives::sequence_occurs(&words, &["cards", "from", "the", "top"])
                    && words.contains(&"until");
            // A replacement ("if ... would ..., ... instead") is the zone-replacement reading's.
            let replacement = words.contains(&"would") && words.contains(&"instead");
            !consult && !replacement
        },
        read: |input| input.outcome(read_conditional_sentence_family(input)),
    },
    Reading {
        id: RuleId::new("create-chosen-characteristics"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_create_chosen_characteristics(input)),
    },
    Reading {
        id: RuleId::new("direct-token-creation-alternative"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_direct_token_creation_alternative(input)),
    },
    Reading {
        id: RuleId::new("quoted-token-rule-then-coin-flip-outcomes"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quoted_token_rule_then_coin_flip_outcomes(input)),
    },
    Reading {
        id: RuleId::new("quoted-token-rule-then-conditional-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quoted_token_rule_then_conditional_followup(input)),
    },
    Reading {
        id: RuleId::new("quoted-token-rule-then-linked-counter-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quoted_token_rule_then_linked_counter_followup(input)),
    },
    Reading {
        id: RuleId::new("reveal-hand-then-put-same-name-as-permanent"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_reveal_hand_then_put_same_name_as_permanent(input)),
    },
    Reading {
        id: RuleId::new("exile-cast-permission"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_cast_permission(input)),
    },
    Reading {
        id: RuleId::new("delegated-categorical-library-choice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_delegated_categorical_library_choice(input)),
    },
    Reading {
        id: RuleId::new("complete-delegated-search-partition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_complete_delegated_search_partition(input)),
    },
    Reading {
        id: RuleId::new("create-with-abilities-from-among"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_create_with_abilities_from_among(input)),
    },
    Reading {
        id: RuleId::new("quantified-token-creation-with-embedded-rules"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quantified_token_creation_with_embedded_rules(input)),
    },
    Reading {
        id: RuleId::new(
            "sentence-each-player-reveals-top-count-put-permanents-onto-battlefield-rest-graveyard",
        ),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(input))
        },
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs; two
/// readings that disagree are an ambiguity.
pub(super) fn read(input: &LegacyDocument<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    match &outcome {
        ParseOutcome::Match(matched) => {
            crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
        }
        ParseOutcome::Error(diagnostic) => {
            crate::parse_trace::event(format!("{REGISTRY}: error: {}", diagnostic.message));
        }
        ParseOutcome::NoMatch => {}
    }
    outcome
}

fn read_choose_then_each_other_becomes_copy(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = input.sentences;
    if let Some(effects) = parse_choose_then_each_other_becomes_copy(sentences)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_created_token_counter_kind_distribution_followup(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if let Some(effects) =
        parse_created_token_counter_kind_distribution_followup(sentences, tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_created_token_mill_counter_followup(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if let Some(effects) = parse_created_token_mill_counter_followup(sentences, tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_each_player_coin_face(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = input.sentences;
    if let Some(effects) = parse_each_player_coin_face_sequence(sentences)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_return_tagged_entry_counters(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = input.sentences;
    // A delayed return followed by `each of them enters with ... if it's ...`
    // is one producer and a coordinated list of entry-counter qualifiers.
    // Parse the final sentence atomically before the broad document/bundle
    // compatibility routes can split its `and an additional` arm and nest the
    // planeswalker condition beneath the creature arm.
    if let [exile_sentence, return_sentence, counter_sentence] = sentences
        && let Some(counter_effects) =
            super::super::subject_verb_primitives::parse_tagged_conditional_entry_counters_sentence(
                SubjectVerbPrimitiveClause::new(counter_sentence),
            )?
    {
        let mut exile_effects = parse_effect_sentences_lexed_inner(exile_sentence)?;
        let mut return_effects = parse_effect_sentences_lexed_inner(return_sentence)?;
        let has_exile_producer = exile_effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Exile { .. },
                    ..
                })
            )
        });
        let has_delayed_return = return_effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::DelayedUntilNextEndStep { .. }));
        if has_exile_producer && has_delayed_return {
            exile_effects.append(&mut return_effects);
            exile_effects.push(EffectAst::Coordinated {
                effects: counter_effects,
                leading_duration: false,
                result_conjunction: false,
            });
            return Ok(Some(exile_effects));
        }
    }
    Ok(None)
}
fn read_counter_linked_land_subtype_followup(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = input.sentences;
    // A counter-linked land subtype sentence is a typed effect continuation,
    // not a static grant. Claim that grammar-proven sentence boundary before
    // the broad multi-sentence bundle compatibility path can absorb `has a
    // ... counter` as a static-ability verb and truncate the duration.
    if sentences.len() > 1
            && sentences.iter().any(|part| {
                super::super::super::front_end::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(part)
                    .is_some()
            })
        {
            let mut effects = Vec::new();
            for part in sentences {
                if part.is_empty() {
                    continue;
                }
                if super::super::super::front_end::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(part)
                    .is_some()
                {
                    effects.push(super::super::parse_effect_clause_lexed(part)?);
                } else {
                    effects.extend(parse_effect_sentences_lexed_inner(part)?);
                }
            }
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_typed_effect_bundle(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    let has_later_delayed_this_turn = sentences.iter().skip(1).any(|sentence| {
        effect_grammar::delayed_sentence_shapes::parse_delayed_this_turn_shape(sentence).is_some()
    });
    let has_later_self_replacement = sentences.iter().skip(1).any(|sentence| {
        matches!(
            classify_instead_followup_tokens(sentence),
            InsteadSemantics::SelfReplacement
        )
    });
    if sentences.len() > 1
        && !has_later_delayed_this_turn
        && !has_later_self_replacement
        && let Some(effects) = super::super::bundle_rules::parse_typed_effect_bundle_lexed(tokens)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_repeated_counter_placement_coordination(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    // Two peer counter placements may share the leading `put` verb.  Claim
    // that complete coordination at the public sentence boundary as well as
    // in chain parsing: triggered and activated line lowering can enter this
    // dispatcher directly, before the ordinary chain route has a chance to
    // separate the implicit second action from the first target filter.
    if sentences.len() == 1
        && let Some(effects) =
            super::super::chain_carry::parse_repeated_counter_placement_coordination(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_sentence_each_player_return_with_additional_counter(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    // The full each-player return owns its additional-entry-counter suffix.
    // Complete effect-body parsing otherwise reaches the tolerant return
    // route first, which accepts the movement prefix and silently drops the
    // counter producer/consumer relationship.
    if sentences.len() == 1
        && let Some(effects) = parse_sentence_each_player_return_with_additional_counter(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_top_sentence(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    // An inline exile-top collection and its from-among deployment are one
    // typed producer/selection/consumer program. The compatibility subject-
    // verb route can parse both actions independently, but then exposes its
    // internal choose/iteration and loses the explicit battlefield controller.
    if sentences.len() == 1
        && source_words.first() == Some(&"exile")
        && crate::word_primitives::sequence_occurs(&source_words, &["top"])
        && crate::word_primitives::sequence_occurs(&source_words, &["from", "among"])
        && crate::word_primitives::sequence_occurs(&source_words, &["onto", "the", "battlefield"])
        && let Some(effects) = super::super::bundle_rules::parse_typed_effect_bundle_lexed(tokens)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_become_rest(input: &LegacyDocument<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    // A copy exception belongs to the complete `becomes a copy` action. The
    // single-sentence dispatcher already proves and lowers that bundle before
    // generic coordination can reinterpret the exception's trailing `has`
    // clause. This sequence entrypoint normally enters its own inner parser,
    // so explicitly preserve the same ownership for one-sentence bodies.
    if sentences.len() == 1
        && source_words
            .iter()
            .any(|word| matches!(*word, "become" | "becomes"))
        && effect_grammar::become_shapes::parse_become_rest_shape(tokens)
            .copy_exception
            .is_some()
    {
        return super::super::parse_effect_sentence_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_leading_duration_sentence(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    // A leading duration and its dynamic base-P/T clause are one typed
    // control-flow program. The broad sentence dispatcher can independently
    // recognize the subject and the where-X tail, but that compatibility
    // route flattens a mixed type/subtype count into an intersection. The
    // effect-chain grammar already proves the complete duration and retains
    // the inclusive count filter, so let that narrower route own this family.
    if sentences.len() == 1
        && (crate::word_primitives::parse_sequence_prefix(
            &source_words,
            &["until", "end", "of", "turn"],
        ) || crate::word_primitives::parse_sequence_prefix(
            &source_words,
            &["until", "your", "next", "turn"],
        ))
        && crate::word_primitives::sequence_occurs(
            &source_words,
            &["base", "power", "and", "toughness"],
        )
        && crate::word_primitives::sequence_occurs(&source_words, &["where", "x", "is"])
        && let Ok(effects) = super::super::parse_effect_chain_lexed(tokens)
        && effects.iter().any(dynamic_base_pt_where_x_effect)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_search_library_slots_to_hand_bundle(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    // Heterogeneous search slots are one complete source sentence. The broad
    // bundle registry is normally reserved for multi-sentence programs, so
    // claim this typed single-sentence family before generic search parsing
    // merges the independently selectable filters into one conjunction.
    if source_words.first() == Some(&"search")
        && let Some(effects) =
            super::super::bundle_rules::parse_search_library_slots_to_hand_bundle(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_delayed_schedule_sentence(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    // A next-step header or a duration-scoped `... this turn` trigger in a
    // resolving effect is a delayed schedule, not a recurring triggered
    // ability. Route the complete sentence through effect dispatch before
    // the public trigger-text convenience path strips the timing header and
    // returns only its payload.
    if sentences.len() == 1
        && (effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens)
            .is_some()
            || effect_grammar::delayed_sentence_shapes::parse_delayed_this_turn_shape(tokens)
                .is_some())
    {
        return super::super::parse_effect_sentence_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_generic_consult_reveal_until_battlefield_bottom(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    // This complete inline consult procedure has a dedicated typed
    // parser. Let it own the one-sentence form before generic
    // coordination turns the procedure into an opaque carry wrapper;
    // callers need the consult action itself at the public boundary to
    // inspect and transport its stop filter and collection tags.
    if sentences.len() == 1
                && let Some(effects) = super::super::dispatch_inner::
                    parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(tokens)?
            {
                return Ok(Some(effects));
            }
    Ok(None)
}
fn read_trigger_line_sentence(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    let has_later_trigger_sentence = sentences.iter().skip(1).any(|sentence| {
        crate::lexer::parser_token_word_refs(sentence)
            .first()
            .is_some_and(|word| matches!(*word, "when" | "whenever" | "at"))
    });
    // The public effect-sequence entrypoint is also used to inspect
    // complete triggered rules text. Let trigger grammar own the
    // header, then return its already-typed body rather than sending
    // the header through ordinary effect-chain dispatch.
    if source_words
        .first()
        .is_some_and(|word| matches!(*word, "when" | "whenever" | "at"))
        && !has_later_trigger_sentence
        && let Ok(crate::cards::builders::LineAst::Triggered { effects, .. }) =
            super::super::super::clause_support::parse_triggered_line_lexed(tokens)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_zone_replacement(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    // Labels such as `Adamant —` are presentation around a complete
    // conditional sentence. Parse that family from the untouched token
    // stream before SentenceInput normalization removes the comma that
    // separates the predicate from its consequence.
    // A graveyard-to-exile replacement also begins with `if`, but the
    // condition describes the replaced event rather than a runtime gate.
    // Preserve that typed replacement before generic conditional parsing
    // tries to lower `would be put` as an ordinary predicate.
    if sentences.len() == 1
        && let Some(effect) =
            super::super::dispatch_inner::parse_zone_replacement_subject_verb(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_conditional_sentence_family(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if sentences.len() == 1
        && let Some(effects) = effect_grammar::parse_conditional_sentence_family_lexed(
            tokens,
            super::super::parse_effect_chain_lexed,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_create_chosen_characteristics(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    // A chosen-color-and-type token is a single create instruction. The
    // inner `and type` belongs to the token definition, so it must reach
    // the typed creation grammar before generic effect-chain splitting
    // can mistake `type` for a coordinated action.
    // A top-level `or` between two complete token blueprints is a
    // resolution choice.  The dedicated creation grammar proves both
    // branches before constructing `ChooseOneOf`; give it the intact
    // sentence before generic coordination turns the alternatives into
    // two effects that both execute.
    if sentences.len() == 1 && source_words.first() == Some(&"create") && {
        let (chosen_color, chosen_type) =
            super::super::super::grammar::token_definitions::source_chosen_token_characteristics(
                &source_words,
            );
        chosen_color || chosen_type
    } {
        return super::super::parse_create(tokens, None)
            .map(|effect| vec![effect])
            .map(Some);
    }
    Ok(None)
}
fn read_direct_token_creation_alternative(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    if sentences.len() == 1
        && source_words.first() == Some(&"create")
        && super::super::creation_handlers::is_direct_token_creation_alternative_candidate(tokens)
        && let Ok(effect @ EffectAst::ChooseOneOf { .. }) = super::super::parse_create(tokens, None)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_quoted_token_rule_then_coin_flip_outcomes(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A period can terminate both an embedded quoted token rule and the
    // outer create-token sentence. The general sentence splitter quite
    // correctly ignores periods inside quotes, so make that outer
    // boundary explicit only when a new `put` instruction begins after a
    // balanced quoted rule. Parsing the reconstructed two-sentence stream
    // lets the ordinary carry machinery bind `it` to the created token.
    if let Some(effects) = parse_quoted_token_rule_then_coin_flip_outcomes(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_quoted_token_rule_then_conditional_followup(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_quoted_token_rule_then_conditional_followup(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_quoted_token_rule_then_linked_counter_followup(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_quoted_token_rule_then_linked_counter_followup(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_reveal_hand_then_put_same_name_as_permanent(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_reveal_hand_then_put_same_name_as_permanent(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_cast_permission(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    const EXILE_CAST_PREFIX: &[&str] = &[
        "you", "may", "cast", "a", "spell", "from", "among", "cards", "you", "own", "in", "exile",
    ];
    let tokens = input.tokens;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    let ordinary_exile_cast = source_words
        .get(EXILE_CAST_PREFIX.len()..)
        .is_some_and(|tail| {
            crate::word_primitives::parse_sequence_complete(
                tail,
                &["without", "paying", "its", "mana", "cost"],
            )
        });
    let dream_exile_cast = source_words
        .get(EXILE_CAST_PREFIX.len()..)
        .is_some_and(|tail| {
            crate::word_primitives::parse_sequence_complete(
                tail,
                &[
                    "with", "dream", "counters", "on", "them", "without", "paying", "its", "mana",
                    "cost",
                ],
            )
        });
    if crate::word_primitives::parse_sequence_prefix(&source_words, EXILE_CAST_PREFIX)
        && (ordinary_exile_cast || dream_exile_cast)
    {
        let tag = crate::tag::CompilerReferenceTag::ChosenCounteredExileSpell.key();
        let mut filter = ObjectFilter::default()
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::Exile);
        if dream_exile_cast {
            filter = filter.with_counter_type(crate::object::CounterType::Dream);
        }
        return Ok(Some(vec![EffectAst::May {
            effects: vec![
                EffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: tag.clone(),
                },
                EffectAst::subject_verb_cast_tagged(tag, PlayerAst::You, false, false, true, None),
            ],
        }]));
    }
    Ok(None)
}
fn read_delegated_categorical_library_choice(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_delegated_categorical_library_choice(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_delegated_search_partition(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_complete_delegated_search_partition(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_create_with_abilities_from_among(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let source_words = crate::lexer::parser_token_word_refs(tokens);
    if crate::word_primitives::sequence_occurs(&source_words, &["create"])
        && crate::word_primitives::any_sequence_occurs(
            &source_words,
            &[
                &["abilities", "from", "among"],
                &["ability", "from", "among"],
            ],
        )
        && crate::word_primitives::sequence_occurs(&source_words, &["found", "among"])
        && let Ok(effect) = super::super::parse_create(tokens, None)
        && matches!(
            &effect,
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenWithMods { count, .. },
                ..
            }) if matches!(count.unhinted(), Value::StaticAbilitiesAmong { .. })
        )
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_quantified_token_creation_with_embedded_rules(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_quantified_token_creation_with_embedded_rules(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    input: &LegacyDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This is one comma-coordinated instruction, not three independent
    // effect sentences. Preserve the dynamic reveal count and the shared
    // revealed-card set before document sentence normalization can reduce
    // the leading clause to a single-card RevealTop action.
    if let Some(effects) = super::super::
                parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
                    SubjectVerbPrimitiveClause::new(tokens),
                )?
            {
                return Ok(Some(effects));
            }
    Ok(None)
}
