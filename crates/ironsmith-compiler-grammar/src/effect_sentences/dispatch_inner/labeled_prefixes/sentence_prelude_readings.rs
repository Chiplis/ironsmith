//! The readings of one effect sentence before the sentence registries: the
//! labeled prefixes ("if you don't", result prefixes, labeled effect
//! prefixes, "then" tails), the classified heads (conditional families,
//! votes, participants, casts, searches, unless, gains and losses, ...) and
//! the restriction sentences that carry no effects. Formerly a first-match
//! ladder in `labeled_prefixes`; every reading runs, resolved by rank while
//! the overlaps are measured. The unsupported-sentence diagnosis and the
//! sentence registries follow in the wrapper.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct SentencePrelude<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) dispatch_shape: &'a effect_grammar::labeled_dispatch::LabeledDispatchShape<'a>,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl SentencePrelude<'_> {
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
                RuleId::new("sentence-prelude-registry-reading"),
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
    admits: fn(&SentencePrelude<'_>) -> bool,
    read: fn(&SentencePrelude<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("sentence-prelude-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("if-you-dont"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_if_you_dont(input)),
    },
    Reading {
        id: RuleId::new("roll-dice-choose-one-result"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_roll_dice_choose_one_result(input)),
    },
    Reading {
        id: RuleId::new("leading-result-prefix"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_result_prefix(input)),
    },
    Reading {
        id: RuleId::new("player-villainous-choice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_villainous_choice(input)),
    },
    Reading {
        id: RuleId::new("activate-only-restriction-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activate_only_restriction_sentence(input)),
    },
    Reading {
        id: RuleId::new("trigger-only-restriction-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_trigger_only_restriction_sentence(input)),
    },
    Reading {
        id: RuleId::new("scaled-target-power"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_scaled_target_power(input)),
    },
    Reading {
        id: RuleId::new("round-up-each-time"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_round_up_each_time(input)),
    },
    Reading {
        id: RuleId::new("vote-affinity"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_vote_affinity(input)),
    },
    Reading {
        id: RuleId::new("labeled-effect-prefix"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_labeled_effect_prefix(input)),
    },
    Reading {
        id: RuleId::new("conditional-source-exiled-owner-library-bottom"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conditional_source_exiled_owner_library_bottom(input)),
    },
    Reading {
        id: RuleId::new("conditional-exile-replacement"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conditional_exile_replacement(input)),
    },
    Reading {
        id: RuleId::new("conditional-pre-primitives"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conditional_pre_primitives(input)),
    },
    Reading {
        id: RuleId::new("next-spell-grant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_next_spell_grant(input)),
    },
    Reading {
        id: RuleId::new("matching-spell-cost-reduction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_matching_spell_cost_reduction(input)),
    },
    Reading {
        id: RuleId::new("source-exiled-owner-library-bottom"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_exiled_owner_library_bottom(input)),
    },
    Reading {
        id: RuleId::new("explicit-assign-no-combat-damage-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_explicit_assign_no_combat_damage_followup(input)),
    },
    Reading {
        id: RuleId::new("pre-extension-head"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_pre_extension_head(input)),
    },
    Reading {
        id: RuleId::new("conditional-sentence-family"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("leading-result-prefix")
        },
        read: |input| input.outcome(read_conditional_sentence_family(input)),
    },
    Reading {
        id: RuleId::new("exile-then"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_then(input)),
    },
    Reading {
        id: RuleId::new("then-tail"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_then_tail(input)),
    },
    Reading {
        id: RuleId::new("late-leading-result-prefix"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("leading-result-prefix")
        },
        read: |input| input.outcome(read_late_leading_result_prefix(input)),
    },
    Reading {
        id: RuleId::new("each-player-choose"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_each_player_choose(input)),
    },
    Reading {
        id: RuleId::new("for-each-opponent"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_for_each_opponent(input)),
    },
    Reading {
        id: RuleId::new("for-each-player"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_for_each_player(input)),
    },
    Reading {
        id: RuleId::new("cast-from-among-free"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cast_from_among_free(input)),
    },
    Reading {
        id: RuleId::new("cast-hand-free"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cast_hand_free(input)),
    },
    Reading {
        id: RuleId::new("unquoted-search-library"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("for-each-player")
        },
        read: |input| input.outcome(read_unquoted_search_library(input)),
    },
    Reading {
        id: RuleId::new("exile-all-cards-from-hand-graveyard"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_all_cards_from_hand_graveyard(input)),
    },
    Reading {
        id: RuleId::new("starts-enchant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_starts_enchant(input)),
    },
    Reading {
        id: RuleId::new("starts-earthbend"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_starts_earthbend(input)),
    },
    Reading {
        id: RuleId::new("unless-with-comma-then-boundary"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_unless_with_comma_then_boundary(input)),
    },
    Reading {
        id: RuleId::new("unless-pays"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_unless_pays(input)),
    },
    Reading {
        id: RuleId::new("unless-post-primitives"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_unless_post_primitives(input)),
    },
    Reading {
        id: RuleId::new("gain-or-lose"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("leading-result-prefix")
        },
        read: |input| input.outcome(read_gain_or_lose(input)),
    },
    Reading {
        id: RuleId::new("vote-extension"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_vote_extension(input)),
    },
    Reading {
        id: RuleId::new("return-rounded-up"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_return_rounded_up(input)),
    },
    Reading {
        id: RuleId::new("choose-do-same-for"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_do_same_for(input)),
    },
    Reading {
        id: RuleId::new("cast-any-number-graveyard-free"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cast_any_number_graveyard_free(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &SentencePrelude<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_if_you_dont(input: &SentencePrelude<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = super::super::dispatch_entry::parse_if_you_dont_sentence(tokens)? {
        return Ok(Some(vec![EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::ExplicitDidNot,
            effects,
        }]));
    }
    Ok(None)
}
fn read_roll_dice_choose_one_result(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
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
fn read_leading_result_prefix(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The copy specialist can own a complete `copy ..., then copy
    // ...` program. Keep its typed coordination boundary inside
    // the result wrapper instead of flattening its actions
    // through the generic chain splitter.
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let trailing_effects =
            if let Some(copy_effect) = parse_copy_spell_clause(prefix.trailing_tokens)? {
                vec![copy_effect]
            } else {
                super::super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?
            };
        let mut result = vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
        }];
        super::super::preserve_leading_result_coordination_lexed(tokens, &mut result);
        return Ok(Some(result));
    }
    Ok(None)
}
fn read_player_villainous_choice(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_player_villainous_choice_statement(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_activate_only_restriction_sentence(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if is_activate_only_restriction_sentence_lexed(tokens) {
        return Ok(Some(Vec::new()));
    }
    Ok(None)
}
fn read_trigger_only_restriction_sentence(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if is_trigger_only_restriction_sentence_lexed(tokens) {
        return Ok(Some(Vec::new()));
    }
    Ok(None)
}
fn read_scaled_target_power(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::subject_verb_special_recognizers::parse_scaled_target_power_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_round_up_each_time(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.round_up_each_time {
        return Ok(Some(Vec::new()));
    }
    Ok(None)
}
fn read_vote_affinity(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_vote_affinity_subject_verb(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_labeled_effect_prefix(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(stripped) = split_labeled_effect_prefix_lexed(tokens) {
        return parse_effect_sentence_lexed(stripped).map(Some);
    }
    Ok(None)
}
fn read_conditional_source_exiled_owner_library_bottom(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.starts_if
            && effect_grammar::control_copy_attach_shapes::contains_source_exiled_owner_library_bottom_shape(tokens)
            && let Some(effects) = parse_conditional_sentence_family_lexed(
                tokens,
                parse_effect_chain_preserving_source_exiled_owner_library_bottom,
            )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_conditional_exile_replacement(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.starts_if
        && let Some(mut effects) = parse_exile_replacement_subject_verb_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_conditional_pre_primitives(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.starts_if
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_next_spell_grant(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_matching_spell_cost_reduction(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = lower_matching_spell_cost_reduction_sentence(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_source_exiled_owner_library_bottom(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_source_exiled_owner_library_bottom_subject_verb(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_explicit_assign_no_combat_damage_followup(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A complete trailing no-combat-damage action must be separated before
    // the prefix-tolerant subject/verb extension can absorb it into a broad
    // destroy target. The helper independently grammar-proves and lowers both
    // arms, so ordinary `and` lists remain on the normal path.
    if let Some(effects) = parse_explicit_assign_no_combat_damage_followup(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_pre_extension_head(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.pre_extension_head
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_conditional_sentence_family(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(mut effects) = parse_conditional_sentence_family_lexed(
        tokens,
        parse_effect_chain_preserving_source_exiled_owner_library_bottom,
    )? {
        super::super::preserve_leading_result_coordination_lexed(tokens, &mut effects);
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_then(input: &SentencePrelude<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.exile_then {
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(Some(effects));
        }
        if let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(Some(effects));
        }
    }
    Ok(None)
}
fn read_then_tail(input: &SentencePrelude<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let dispatch_shape = input.dispatch_shape;
    if let Some(then_tail) = dispatch_shape.then_tail {
        return parse_effect_sentence_lexed(then_tail).map(Some);
    }
    Ok(None)
}
fn read_late_leading_result_prefix(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This late result-prefix route intentionally parses the trailing
    // actions through the inner chain parser to avoid re-entering result
    // dispatch. Restore the authored conjunction after that parse, using
    // the same grammar-confirmed boundary as the ordinary conditional
    // route rather than inferring it from adjacent effects.
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let trailing_effects =
            super::super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?;
        let mut result = vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
        }];
        super::super::preserve_leading_result_coordination_lexed(tokens, &mut result);
        return Ok(Some(result));
    }
    Ok(None)
}
fn read_each_player_choose(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    // The complete choice/complement grammar and the generic participant
    // action grammar are separate subdomains of this classified head.
    // A proved complement owns its `then ... the rest` relation; only a
    // non-complement participant action reaches the generic extension.
    if dispatch_shape.each_player_choose {
        if let Some(effect) = parse_choice_complement_subject_verb(tokens)? {
            return Ok(Some(vec![effect]));
        }
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(Some(effects));
        }
    }
    Ok(None)
}
fn read_for_each_opponent(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_for_each_opponent_clause(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_for_each_player(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_for_each_player_clause(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_cast_from_among_free(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let dispatch_shape = input.dispatch_shape;
    if let Some(cast_from_among) = dispatch_shape.cast_from_among_free {
        let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
        filter.card_types.push(CardType::Instant);
        filter.card_types.push(CardType::Sorcery);
        filter.card_types.push(CardType::Artifact);
        filter.card_types.push(CardType::Creature);
        filter.card_types.push(CardType::Enchantment);
        filter.card_types.push(CardType::Planeswalker);
        filter.card_types.push(CardType::Battle);
        filter.type_or_subtype_union = true;
        if let Some(bound) = cast_from_among.mana_value_or_less {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(bound as i32));
        }
        let chosen = crate::tag::CompilerReferenceTag::ChosenCastFromAmong.bind();
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::model::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    copy_cast_reminder_surface: false,
                    copy_instruction_surface: None,
                    without_paying_mana_cost: true,
                    additional_mana_cost: None,
                    cost_reduction: None,
                    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                },
            }),
        ]));
    }
    Ok(None)
}
fn read_cast_hand_free(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.cast_hand_free {
        let chosen = crate::tag::CompilerReferenceTag::ChosenHandSpellToCast.bind();
        let filter = ObjectFilter::nonland()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You);
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::model::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    copy_cast_reminder_surface: false,
                    copy_instruction_surface: None,
                    without_paying_mana_cost: true,
                    additional_mana_cost: None,
                    cost_reduction: None,
                    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                },
            }),
        ]));
    }
    Ok(None)
}
fn read_unquoted_search_library(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.has_unquoted_search
        && let Some(mut effects) = parse_search_library_sentence_lexed(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_all_cards_from_hand_graveyard(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.exile_all_cards_from_hand_graveyard
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_starts_enchant(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.starts_enchant
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_starts_earthbend(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.starts_earthbend
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_unless_with_comma_then_boundary(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    // The unless clause belongs only to the ordered tail. Split the
    // grammar-proven `, then` boundary before the whole-sentence unless
    // primitive can wrap the earlier action and drop the tail action.
    if dispatch_shape.has_unless
        && super::super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(tokens)
    {
        return parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_unless_pays(input: &SentencePrelude<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.has_unless
        && let Some(mut effects) = super::super::parse_sentence_unless_pays(
            super::super::SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_unless_post_primitives(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.has_unless
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_gain_or_lose(input: &SentencePrelude<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    // Voter-relative player sets contain an ordinary action verb (for
    // example, "loses"), so route the typed voting subject before the generic
    // gain/lose primitive can erase the vote-affinity predicate.
    // An independent action followed by an explicit gain/lose action is
    // an action choice, not one unusually long gain-ability subject. The
    // broad grant parser accepts object-filter prefixes and can otherwise
    // consume the leading action while retaining only the grant branch.
    if dispatch_shape.has_gain_or_lose {
        if let Some(unless_action) = super::super::parse_or_action_clause_lexed(tokens)? {
            return Ok(Some(vec![unless_action]));
        }
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(Some(effects));
        }
        if let Ok(mut effects) = parse_effect_chain_lexed(tokens) {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(Some(effects));
        }
        if let Ok(mut effect) = super::super::parse_effect_clause_with_trailing_if(tokens) {
            apply_where_x_to_damage_amounts(tokens, std::slice::from_mut(&mut effect))?;
            return Ok(Some(vec![effect]));
        }
        if let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(Some(effects));
        }
    }
    Ok(None)
}
fn read_vote_extension(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.has_vote
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_return_rounded_up(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.return_rounded_up
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_do_same_for(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.choose_do_same_for
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_cast_any_number_graveyard_free(
    input: &SentencePrelude<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.cast_any_number_graveyard_free {
        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Instant);
        filter.card_types.push(CardType::Sorcery);
        filter.type_or_subtype_union = true;
        filter.colors = Some(crate::color::ColorSet::from(crate::color::Color::Red));
        let tag = crate::tag::CompilerReferenceTag::ChosenCastFromGraveyard.bind();
        return Ok(Some(vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
                zones: vec![Zone::Graveyard],
                search_mode: Some(crate::effect::SearchSelectionMode::Optional),
            },
            EffectAst::subject_verb_cast_tagged(tag, PlayerAst::You, false, false, true, None),
        ]));
    }
    Ok(None)
}
