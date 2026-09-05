//! The readings of an effect document after the direct, after-direct and
//! legacy readings decline: the grammar-proven whole-document shapes
//! (delegated library choices, hexproof targeting overrides, leading-duration
//! flash bundles, permission-then-trigger grants, create/exile/sacrifice and
//! choose/return/draw triples, target declarations, keyword-bundle pumps,
//! play permissions, replacements, emblems, ...). Formerly a first-match
//! ladder in `dispatch_entry`; every reading runs, resolved by rank while the
//! overlaps are measured. Per-sentence parsing is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct RemainingDocument<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl RemainingDocument<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
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
                RuleId::new("document-remaining-registry-reading"),
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
    admits: fn(&RemainingDocument<'_>) -> bool,
    read: fn(&RemainingDocument<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("document-remaining-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("delegated-categorical-library-choice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_delegated_categorical_library_choice(input)),
    },
    Reading {
        id: RuleId::new("hexproof-targeting-override"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_hexproof_targeting_override(input)),
    },
    Reading {
        id: RuleId::new("until-next-turn-cast-permission-then-trigger"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_until_next_turn_cast_permission_then_trigger(input)),
    },
    Reading {
        id: RuleId::new("permission-then-whenever-grant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_permission_then_whenever_grant(input)),
    },
    Reading {
        id: RuleId::new("token-lifecycle-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_token_lifecycle_sentence(input)),
    },
    Reading {
        id: RuleId::new("choose-return-draw-triple"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_return_draw_triple(input)),
    },
    Reading {
        id: RuleId::new("historical-graveyard-target-declaration"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_historical_graveyard_target_declaration(input)),
    },
    Reading {
        id: RuleId::new("choose-target-prelude"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_target_prelude(input)),
    },
    Reading {
        id: RuleId::new("choose-target"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_target(input)),
    },
    Reading {
        id: RuleId::new("next-batch-enter-with-counters"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_next_batch_enter_with_counters(input)),
    },
    Reading {
        id: RuleId::new("keyword-bundle-pump"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_keyword_bundle_pump(input)),
    },
    Reading {
        id: RuleId::new("play-permission"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_play_permission(input)),
    },
    Reading {
        id: RuleId::new("counter-linked-land-subtype-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counter_linked_land_subtype_followup(input)),
    },
    Reading {
        id: RuleId::new("turn-scoped-enter-tapped-replacement"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_turn_scoped_enter_tapped_replacement(input)),
    },
    Reading {
        id: RuleId::new("tapped-land-mana-replacement"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tapped_land_mana_replacement(input)),
    },
    Reading {
        id: RuleId::new("reflected-prevent-next-damage"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_reflected_prevent_next_damage(input)),
    },
    Reading {
        id: RuleId::new("quoted-emblem-then-action"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quoted_emblem_then_action(input)),
    },
    Reading {
        id: RuleId::new("emblem-payload"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_emblem_payload(input)),
    },
    Reading {
        id: RuleId::new("coordinated-leading-duration-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_coordinated_leading_duration_chain(input)),
    },
    Reading {
        id: RuleId::new("gain-then-get"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_gain_then_get(input)),
    },
    Reading {
        id: RuleId::new("leading-gain-duration"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("coordinated-leading-duration-chain")
        },
        read: |input| input.outcome(read_leading_gain_duration(input)),
    },
    Reading {
        id: RuleId::new("can-block-additional-creature-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_can_block_additional_creature_this_turn(input)),
    },
    Reading {
        id: RuleId::new("sentence-prelude-shape"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sentence_prelude_shape(input)),
    },
    Reading {
        id: RuleId::new("leading-player-may"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_player_may(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &RemainingDocument<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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
    let outcome = resolve_ranked_candidates(REGISTRY, distinct, diagnostics, || {
        crate::lexer::parser_token_word_refs(input.tokens).join(" ")
    });
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_delegated_categorical_library_choice(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_delegated_categorical_library_choice(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_hexproof_targeting_override(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The targeting-relation classifier owns this complete as-though
    // permission. Gain-ability grammar explicitly rejects this domain, so
    // the ignored ability cannot be reinterpreted as an ability grant.
    if let Some(effect) =
        super::super::clause_dispatch::parse_hexproof_targeting_override_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_until_next_turn_cast_permission_then_trigger(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let words = crate::lexer::parser_token_word_refs(tokens);
    if crate::word_primitives::parse_sequence_prefix(
        &words,
        &["until", "your", "next", "turn", "you", "may", "cast"],
    ) && let Some(split) = comma_and_word_boundary(tokens, "each")
        && let Some(prefix_comma) =
            crate::slice_primitives::select_position(tokens, OwnedLexToken::is_comma)
    {
        let permission_tokens = &tokens[..split];
        let replacement_tokens = &tokens[split + 2..];
        let replacement_words = crate::lexer::parser_token_word_refs(replacement_tokens);
        if crate::word_primitives::parse_sequence_prefix(
                &replacement_words,
                &["each", "creature", "you", "control", "enters"],
            ) && let Some(permission) =
                super::super::super::permission_helpers::parse_cast_spells_as_though_they_had_flash_clause(
                    permission_tokens,
                )?
            {
                let mut duration_replacement = tokens[..=prefix_comma].to_vec();
                duration_replacement.extend_from_slice(replacement_tokens);
                let mut effects = vec![permission];
                let mut replacement = parse_effect_sentences_lexed_inner(&duration_replacement)?;
                if !replacement.is_empty() {
                    effects.append(&mut replacement);
                    return Ok(Some(effects));
                }
            }
    }
    Ok(None)
}
fn read_permission_then_whenever_grant(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A temporary flash permission followed by an authored cast-trigger grant
    // is two coordinated effects. Parse the permission independently so the
    // leading `may` cannot incorrectly wrap (or replace) the delayed grant.
    if let Some(split) = comma_and_word_boundary(tokens, "whenever") {
        let permission_tokens = &tokens[..split];
        let grant_tokens = &tokens[split + 2..];
        let grant_words = crate::lexer::parser_token_word_refs(grant_tokens);
        let strict_cast_grant = crate::word_primitives::parse_sequence_prefix(
            &grant_words,
            &["whenever", "you", "cast"],
        ) && crate::word_primitives::sequence_occurs(
            &grant_words,
            &["this", "turn", "it", "gains"],
        );
        if strict_cast_grant
                && let Some(permission) =
                    super::super::super::permission_helpers::parse_cast_spells_as_though_they_had_flash_clause(
                        permission_tokens,
                    )?
            {
                let mut effects = vec![permission];
                let mut grant = parse_effect_sentences_lexed_inner(grant_tokens)?;
                if !grant.is_empty() {
                    effects.append(&mut grant);
                    return Ok(Some(effects));
                }
            }
    }
    Ok(None)
}
fn read_token_lifecycle_sentence(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    // These two follow-ups form one reciprocal lifecycle around the token
    // produced by the first sentence. Parse the producer independently,
    // then bind both grammar-proven references before the broad sequence
    // registry can consume the exile sentence as an unrelated suffix.
    if let [create, exile_created, sacrifice_source] = sentence_parts.as_slice()
            && crate::grammar::trigger_subjects::parse_token_lifecycle_sentence_tokens(exile_created)
                == Some(
                    crate::grammar::trigger_subjects::TokenLifecycleSentenceKind::ExileCreatedTokenWhenSourceLeaves,
                )
            && crate::grammar::trigger_subjects::parse_token_lifecycle_sentence_tokens(
                sacrifice_source,
            ) == Some(
                crate::grammar::trigger_subjects::TokenLifecycleSentenceKind::SacrificeSourceWhenCreatedTokenLeaves,
            )
        {
            let create_effect = super::super::parse_create(create, None)?;
            if !matches!(
                &create_effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::CreateTokenWithMods { .. },
                    ..
                })
            ) {
                return Err(CardTextError::ParseError(
                    "created-token lifecycle producer was not a token creation".to_string(),
                )).map(Some);
            }
            let mut effects = vec![create_effect];
            let exile = parse_sentence_exile_that_token_when_source_leaves(
                exile_created,
                effects.as_slice(),
            )
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "created-token lifecycle lost its token producer before exile".to_string(),
                )
            })?;
            effects.push(exile);
            let sacrifice = parse_sentence_sacrifice_source_when_that_token_leaves(
                sacrifice_source,
                effects.as_slice(),
            )
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "created-token lifecycle lost its token producer before sacrifice".to_string(),
                )
            })?;
            effects.push(sacrifice);
            return Ok(Some(effects
                .into_iter()
                .map(|effect| EffectAst::SourceSentence {
                    effects: vec![effect],
                    leading_then: false,
                    starting_with_controller: false,
                })
                .collect()));
        }
    Ok(None)
}
fn read_choose_return_draw_triple(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    if let [choose, return_them, draw] = sentence_parts.as_slice()
        && crate::word_primitives::parse_sequence_prefix(
            &crate::lexer::parser_token_word_refs(choose),
            &[
                "choose",
                "up",
                "to",
                "three",
                "target",
                "permanent",
                "cards",
                "in",
                "graveyards",
                "that",
                "were",
                "put",
                "there",
                "from",
                "the",
                "battlefield",
                "this",
                "turn",
            ],
        )
        && crate::word_primitives::parse_sequence_prefix(
            &crate::lexer::parser_token_word_refs(return_them),
            &["return", "them", "to", "the", "battlefield"],
        )
        && crate::word_primitives::parse_sequence_prefix(
            &crate::lexer::parser_token_word_refs(draw),
            &[
                "you",
                "draw",
                "a",
                "card",
                "for",
                "each",
                "opponent",
                "who",
                "controls",
                "one",
                "or",
                "more",
                "of",
                "those",
                "permanents",
            ],
        )
    {
        let Some(target) = exact_historical_graveyard_target_declaration(choose) else {
            return Err(CardTextError::ParseError(
                "historical graveyard target declaration lost its typed envelope".to_string(),
            ))
            .map(Some);
        };
        let mut effects = vec![target];
        for sentence in [return_them, draw] {
            effects.extend(parse_effect_sentences_lexed_inner(sentence)?);
        }
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_historical_graveyard_target_declaration(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    // A complete authored target declaration is already a fully typed effect
    // clause. Route it before subject/verb planning: a relative filter such
    // as "cards ... that were put there" otherwise exposes the embedded
    // `put` verb and the planner can mistake the filter tail for a separate
    // zone-change action.
    if sentence_parts.len() == 1
        && let Some(effect) = exact_historical_graveyard_target_declaration(tokens)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_choose_target_prelude(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    if sentence_parts.len() == 1
        && let Some(effects) =
            super::super::clause_pattern_helpers::parse_choose_target_prelude_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_target(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    if sentence_parts.len() == 1
            && let Some(shape) =
                super::super::super::grammar::effects::clause_dispatch_shapes::parse_choose_target_shape(
                    tokens,
                )
            && !super::super::super::grammar::effects::chain_splitting::has_authored_comma_then_surface_tokens(
                tokens,
            )
            && !crate::word_primitives::sequence_occurs(
                &crate::lexer::parser_token_word_refs(tokens),
                &["then"],
            )
            && super::super::super::util::parse_target_phrase(shape.target_tokens).is_ok()
        {
            return Ok(Some(vec![super::super::parse_effect_clause_lexed(tokens)?]));
        }
    Ok(None)
}
fn read_next_batch_enter_with_counters(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    if sentence_parts.len() == 1
        && let Some(effect) = parse_next_batch_enter_with_counters(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_keyword_bundle_pump(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    // The keyword-bundle pump is one semantic sentence even though its
    // `+1/+1 if ...` arms and `and so on for ...` tail contain many commas.
    // Trigger CST probing enters through this multi-sentence entrypoint; if
    // the whole typed shape is not claimed here, a later comma can appear to
    // be a valid trigger/effect boundary and discard most of the bundle.
    if sentence_parts.len() == 1
        && let Some(effects) =
            super::super::subject_verb_special_recognizers::parse_keyword_bundle_pump_sentence(
                tokens,
            )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_play_permission(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    // A turn-scoped play permission contains an authored `play ... and cast
    // ...` conjunction which must be consumed as one typed effect. When an
    // independent sentence follows it, the generic multi-sentence chain
    // planner can otherwise revisit the first sentence as an action chain and
    // split it into the unsupported fragment `play lands`. Claim the complete
    // first sentence before parsing the remaining independent statements.
    if sentence_parts.len() > 1
        && let Some(first) = sentence_parts.first()
        && let Some(permission) = super::super::parse_play_permission_subject_verb(first)?
    {
        let mut effects = vec![permission];
        for sentence in sentence_parts.iter().skip(1) {
            if !sentence.is_empty() {
                effects.extend(parse_effect_sentences_lexed_inner(sentence)?);
            }
        }
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_counter_linked_land_subtype_followup(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Counter-linked land subtype text is an effect continuation even though
    // its surface starts like a static ability.  The clause dispatcher owns
    // the typed AddSubtypes/ForAsLongAs lowering; route it before sentence
    // verb splitting turns the `in addition` scope into an unsupported tail.
    if super::super::super::front_end::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(tokens)
            .is_some()
        {
            return Ok(Some(vec![super::super::parse_effect_clause_lexed(tokens)?]));
        }
    Ok(None)
}
fn read_turn_scoped_enter_tapped_replacement(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_turn_scoped_enter_tapped_replacement(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_tapped_land_mana_replacement(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_tapped_land_mana_replacement(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_reflected_prevent_next_damage(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = reflected_prevent_next_damage_from_tokens(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_quoted_emblem_then_action(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = super::super::zone_handlers::parse_quoted_emblem_then_action(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_emblem_payload(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Quoted emblem abilities may contain their own sentence boundaries and
    // activated-ability colons. Consume the typed whole-sentence shape before
    // generic sentence and subject/verb splitting sees those nested tokens.
    if effect_grammar::emblem_shapes::parse_emblem_payload_tokens(tokens)
        .is_some_and(|shape| shape.requires_whole_sentence_dispatch)
        && let Some(effect) = super::super::zone_handlers::parse_emblem_action(tokens, None)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_coordinated_leading_duration_chain(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    // A genuine coordinated clause with one leading duration must reach the
    // chain parser before the duration-gain fast path below. That fast path is
    // intentionally tolerant of surrounding text and can otherwise retain
    // only a later `it gains ...` arm while dropping an earlier action.
    if sentence_parts.len() == 1
        && effect_grammar::chain_carry::coordinated_effect_chain_leading_duration(tokens)
            == Some(true)
    {
        return parse_effect_sentence_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_gain_then_get(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    // A gain/get compound has one authored target and one trailing duration.
    // The direct gain parser already proves and preserves both facts, but the
    // broad whole-body bundle/chain routes below may independently lower the
    // `gains` and `gets` arms.  That fallback loses the shared target before
    // the second arm is compiled and can therefore retarget the pump to the
    // resolving spell's source.  Give the exact compound grammar first
    // refusal at the complete-effect-body boundary.
    if sentence_parts.len() == 1
        && (effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(tokens).is_some()
            || effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(tokens).is_some())
        && let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_leading_gain_duration(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentence_parts = split_lexed_sentences(tokens);
    let sentence_words = crate::grammar::primitives::TokenWordView::new(tokens).to_word_refs();
    // Activated-line preprocessing can remove quote delimiters around a
    // nested granted rule. Preserve the outer leading-duration gain
    // before a `can't` inside the rule is claimed as the top-level effect.
    if sentence_parts.len() == 1
        && effect_grammar::gain_ability_shapes::parse_leading_gain_duration_shape(&sentence_words)
            .is_some()
        && let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_can_block_additional_creature_this_turn(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This clause is also a valid static ability sentence.  In an activated
    // ability, however, it is a temporary grant and must retain its explicit
    // turn duration instead of going through the generic granted-object
    // ability parser, which defaults to Forever.
    if let Some(effect) = parse_can_block_additional_creature_this_turn_clause(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_sentence_prelude_shape(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The two-dice choice sentence is a complete effect on its own.  Route it
    // before generic verb parsing, which otherwise reduces it to the partial
    // clause `two d6` and reports a misleading unsupported-roll error.
    if let Some(effect_grammar::SentencePreludeShape::RollDiceChooseOneResult {
        count,
        sides,
        surface,
    }) = effect_grammar::parse_sentence_prelude_shape_tokens(tokens)
    {
        return Ok(Some(vec![
            EffectAst::subject_verb_roll_dice_choose_result_with_surface(
                PlayerAst::Implicit,
                count,
                sides,
                Some(surface),
            ),
        ]));
    }
    Ok(None)
}
fn read_leading_player_may(
    input: &RemainingDocument<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Keep the hand/graveyard/permanents-to-library bundle intact.  Generic
    // comma splitting can otherwise hand the resource verb only `your hand,
    // your graveyard`, losing the destination and the owned-permanents part.
    // An optional shuffle keeps its `may` scope through the may-aware routes.
    if super::super::chain_carry::parse_leading_player_may_lexed(tokens).is_none()
        && let Some(effects) =
            super::super::search_library::parse_shuffle_graveyard_into_library_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}

fn comma_and_word_boundary(tokens: &[OwnedLexToken], word: &str) -> Option<usize> {
    let view = crate::lexer::TokenWordView::new(tokens);
    let and_word = view.parse_phrase_start(&["and", word])?;
    let and_token = view.map_word_to_token_start(and_word)?;
    let comma = and_token.checked_sub(1)?;
    tokens
        .get(comma)
        .is_some_and(OwnedLexToken::is_comma)
        .then_some(comma)
}

fn exact_historical_graveyard_target_declaration(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let exact_shape = words.len() == 18
        && crate::word_primitives::parse_sequence_prefix(&words, &["choose", "up", "to"])
        && matches!(words.get(3), Some(&"three" | &"3"))
        && words.get(4..).is_some_and(|tail| {
            crate::word_primitives::parse_sequence_complete(
                tail,
                &[
                    "target",
                    "permanent",
                    "cards",
                    "in",
                    "graveyards",
                    "that",
                    "were",
                    "put",
                    "there",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ],
            )
        });
    if !exact_shape {
        return None;
    }
    let mut filter = ObjectFilter::permanent_card().in_zone(Zone::Graveyard);
    filter.entered_graveyard_this_turn = true;
    filter.entered_graveyard_from_battlefield_this_turn = true;
    filter.set_graveyard_entry_history_surface(Some(
        ironsmith_core::GraveyardEntryHistorySurface::PutThereFromBattlefieldThisTurn,
    ));
    Some(EffectAst::subject_verb_explicit_target_only(
        TargetAst::WithCount(
            Box::new(TargetAst::Object(filter, span_from_tokens(tokens), None)),
            crate::effect::ChoiceCount::up_to(3),
        ),
    ))
}
