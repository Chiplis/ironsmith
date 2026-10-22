//! The single statements a document's sentences are read as when they stand
//! independently: complete typed statements ("target creature gets +1/+1",
//! "create a token", "for each player, ...") that the flat split composes
//! without the sentence loop. They were a first-match ladder in
//! `dispatch_entry`; they are a registry whose readings all run and must agree.

use super::SubjectVerbPrimitiveClause;
use super::dispatch_entry::{
    parse_complete_become_statement, parse_complete_compound_gain_statement,
    parse_complete_create_statement, parse_complete_get_pump_statement,
    parse_complete_quantified_discard_statement, parse_complete_simple_mill_sentence,
    parse_complete_simple_subject_verb_sentence,
};
use crate::cards::builders::{CardTextError, ConditionalEffectAst, EffectAst};
use crate::grammar::effects as effect_grammar;
use crate::grammar::structure::{LeadingResultPrefixKind, split_leading_result_prefix_lexed};
use crate::lexer::OwnedLexToken;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// A sentence read as one independent statement, and whether it stands in a
/// document of several.
pub(super) struct Statement<'a> {
    pub(super) sentence: &'a [OwnedLexToken],
    pub(super) is_document: bool,
}

impl Statement<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the sentence.
    fn outcome(
        &self,
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        let span = crate::util::span_from_tokens(self.sentence);
        match read {
            Ok(Some(effects)) => ParseOutcome::matched(effects, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("statement-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test (the ladder's decline guards written before it), and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&Statement<'_>) -> bool,
    read: fn(&Statement<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const STATEMENT_REGISTRY: RuleId = RuleId::new("statement-reading-registry");

/// The readings, in the order they were ranked.
const STATEMENT_READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("transform-any-number"),
        head: HeadDiscriminator::Words(&["transform", "then"]),
        admits: |_| true,
        read: |input| input.outcome(read_transform_any_number(input)),
    },
    Reading {
        id: RuleId::new("counted-next-untap-steps"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counted_next_untap_steps(input)),
    },
    Reading {
        id: RuleId::new("other-chosen-player"),
        head: HeadDiscriminator::Words(&["the"]),
        admits: |_| true,
        read: |input| input.outcome(read_other_chosen_player(input)),
    },
    Reading {
        id: RuleId::new("control-votes-this-turn"),
        head: HeadDiscriminator::Words(&["you"]),
        admits: |_| true,
        read: |input| input.outcome(Ok(read_control_votes_this_turn(input))),
    },
    Reading {
        id: RuleId::new("unblock-sole-blocked-attackers"),
        head: HeadDiscriminator::Words(&["creatures"]),
        admits: |_| true,
        read: |input| input.outcome(Ok(read_unblock_sole_blocked_attackers(input))),
    },
    Reading {
        id: RuleId::new("block-attacker-of-choice"),
        head: HeadDiscriminator::Words(&["you", "have", "it", "that", "then"]),
        admits: |_| true,
        read: |input| input.outcome(read_block_attacker_of_choice(input)),
    },
    Reading {
        id: RuleId::new("optional-self-enter-tapped"),
        head: HeadDiscriminator::Words(&["you", "have"]),
        admits: |_| true,
        read: |input| input.outcome(Ok(read_optional_self_enter_tapped(input))),
    },
    Reading {
        id: RuleId::new("trailing-local-action-replacement"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(super::local_self_replacement::read(input.sentence)),
    },
    Reading {
        id: RuleId::new("bounded-number-choice"),
        head: HeadDiscriminator::Words(&["choose"]),
        admits: |_| true,
        read: |input| input.outcome(super::bounded_number_choice::parse_sentence(input.sentence)),
    },
    Reading {
        id: RuleId::new("player-chooses-number"),
        head: HeadDiscriminator::Words(&["target", "that"]),
        admits: |_| true,
        read: |input| {
            input.outcome(super::bounded_number_choice::parse_player_chooses_sentence(
                input.sentence,
            ))
        },
    },
    Reading {
        id: RuleId::new("owner-subject-shuffle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_owner_subject_shuffle(input)),
    },
    Reading {
        id: RuleId::new("conditional-inline-looked-card-partition"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
        },
        read: |input| input.outcome(read_conditional_inline_looked_card_partition(input)),
    },
    Reading {
        id: RuleId::new("damage-to-controller-of-last-damage-source"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_damage_to_controller_of_last_damage_source(input)),
    },
    Reading {
        id: RuleId::new("choose-target-prelude"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_choose_target_prelude(input)),
    },
    Reading {
        id: RuleId::new("deal-damage-equal-to-power"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_deal_damage_equal_to_power(input)),
    },
    Reading {
        id: RuleId::new("serial-target-pt-modifiers"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_serial_target_pt_modifiers(input)),
    },
    Reading {
        id: RuleId::new("for-as-long-as-source-remains-attached"),
        head: HeadDiscriminator::Any,
        admits: |input| input.sentence.first().is_some_and(|token| token.is_word("for")),
        read: |input| input.outcome(read_for_as_long_as_source_remains_attached(input)),
    },
    Reading {
        id: RuleId::new("sentence-delayed-timing-suffix"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_sentence_delayed_timing_suffix(input)),
    },
    Reading {
        id: RuleId::new("sentence-gets-then-fights"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_sentence_gets_then_fights(input)),
    },
    Reading {
        id: RuleId::new("complete-compound-gain"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_complete_compound_gain(input)),
    },
    Reading {
        id: RuleId::new("simple-gain-ability"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                // "Otherwise, it gains first strike" (Stolen Vitality) is the
                // else-arm of the preceding conditional; the sentence
                // followup planner owns it.
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless", "otherwise"])))
        },
        read: |input| input.outcome(read_simple_gain_ability(input)),
    },
    Reading {
        id: RuleId::new("trailing-if-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_trailing_if_clause(input)),
    },
    Reading {
        id: RuleId::new("complete-get-pump"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_complete_get_pump(input)),
    },
    Reading {
        id: RuleId::new("complete-become"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            // A quoted granted ability ("becomes a land and gains "{T}: Add
            // {U}."") is a coordinated become-plus-gain sentence, never a
            // single complete become statement (mirrors the document reading).
            !sentence
                .iter()
                .any(|token| token.kind == crate::lexer::TokenKind::Quote)
                && !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence)
                    .is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_complete_become(input)),
    },
    Reading {
        id: RuleId::new("sentence-you-and-attacking-player-each-draw-and-lose"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| {
            input.outcome(read_sentence_you_and_attacking_player_each_draw_and_lose(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("sentence-each-opponent-draws-then-you-draw-per-opponent"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            input
                .sentence
                .first()
                .is_some_and(|token| token.is_word("each"))
        },
        read: |input| {
            input.outcome(read_sentence_each_opponent_draws_then_you_draw_per_opponent(input))
        },
    },
    Reading {
        id: RuleId::new("for-each-target-players"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_for_each_target_players(input)),
    },
    Reading {
        id: RuleId::new("vote"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_vote(input)),
    },
    Reading {
        id: RuleId::new("complete-simple-subject-verb"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_complete_simple_subject_verb(input)),
    },
    Reading {
        id: RuleId::new("for-each-player"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_for_each_player(input)),
    },
    Reading {
        id: RuleId::new("complete-simple-mill"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_complete_simple_mill(input)),
    },
    Reading {
        id: RuleId::new("return-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_return_clause(input)),
    },
    Reading {
        id: RuleId::new("exile-single-segment"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_exile_single_segment(input)),
    },
    Reading {
        id: RuleId::new("complete-create"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_complete_create(input)),
    },
    Reading {
        id: RuleId::new("complete-quantified-discard"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_complete_quantified_discard(input)),
    },
    Reading {
        id: RuleId::new("tap"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_tap(input)),
    },
    Reading {
        id: RuleId::new("cant-effect"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_cant_effect(input)),
    },
    Reading {
        id: RuleId::new("leading-result-prefix"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_leading_result_prefix(input)),
    },
    Reading {
        id: RuleId::new("destroy-single-segment"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let sentence = input.sentence;
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
        },
        read: |input| input.outcome(read_destroy_single_segment(input)),
    },
];

/// The statement's reading, if a rule has one. Every admitted reading runs;
/// two readings that disagree are an ambiguity.
pub(super) fn read_statement(input: &Statement<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    let head = crate::lexer::parser_token_word_refs(input.sentence)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in STATEMENT_READINGS {
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
    // Equal readings from two rules are one reading, as the registry deduplicated.
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    resolve_registry_candidates(STATEMENT_REGISTRY, distinct, diagnostics)
}

fn read_conditional_inline_looked_card_partition(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effects) =
        super::chain_carry::parse_conditional_inline_looked_card_partition(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_target_prelude(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effects) =
        super::clause_pattern_helpers::parse_choose_target_prelude_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
/// "<this> deals 4 damage to the controller of the last red instant or
/// sorcery spell that dealt damage to you this turn." (Suffocation): capture
/// the most recent matching damage source as it was when it dealt the
/// damage, then damage its controller at that time.
fn read_damage_to_controller_of_last_damage_source(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = crate::util::trim_edge_punctuation_tokens(input.sentence);
    let words = crate::lexer::token_word_refs(sentence);
    if words.len() != sentence.len() {
        return Ok(None);
    }
    let Some(verb) = words
        .iter()
        .position(|word| matches!(*word, "deals" | "deal"))
    else {
        return Ok(None);
    };
    if verb == 0 || !crate::util::is_source_reference_words(&words[..verb]) {
        return Ok(None);
    }
    let [
        amount,
        "damage",
        "to",
        "the",
        "controller",
        "of",
        "the",
        "last",
        source @ ..,
        "that",
        "dealt",
        "damage",
        "to",
        "you",
        "this",
        "turn",
    ] = &words[verb + 1..]
    else {
        return Ok(None);
    };
    if source.is_empty() {
        return Ok(None);
    }
    let Some(amount) = amount
        .parse::<i32>()
        .ok()
        .or_else(|| crate::util::parse_number_word_i32(amount))
    else {
        return Ok(None);
    };
    let source_start = verb + 1 + 8;
    let source_tokens = &sentence[source_start..source_start + source.len()];
    let sources = crate::object_filters::parse_object_filter(source_tokens, false)?;
    let tag = crate::util::helper_tag_for_tokens(sentence, "last_damage_source");
    let capture = ironsmith_core::TagLastDamageSourceEffect::new(
        ironsmith_core::DamageHistoryQuery {
            sources: ironsmith_core::DamageHistorySources::Matching(sources),
            recipients: ironsmith_core::DamageHistoryRecipients::Players(
                crate::target::PlayerFilter::You,
            ),
            combat: None,
            reduction: ironsmith_core::DamageHistoryReduction::Total,
        },
        tag.key.clone(),
    );
    Ok(Some(vec![
        EffectAst::TagLastDamageSource(capture),
        EffectAst::subject_verb_damage(
            crate::effect::Value::Fixed(amount),
            crate::cards::builders::TargetAst::Player(
                crate::target::PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(
                    tag.key.clone(),
                )),
                None,
            ),
        ),
    ]))
}
fn read_deal_damage_equal_to_power(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(sentence).is_some()
        && effect_grammar::clause_primitive_shapes::parse_power_damage_shape(sentence)?.is_some()
    {
        return super::dispatch_inner::parse_effect_sentence_with_where_x_lexed(sentence).map(Some);
    }
    if effect_grammar::clause_dispatch_shapes::parse_leading_may_shape(sentence).is_none()
        && let Some(effect) =
            super::clause_primitives::parse_deal_damage_equal_to_power_clause(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_serial_target_pt_modifiers(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effects) =
        super::fanout_family::parse_serial_target_pt_modifiers_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
/// "for as long as this Equipment remains attached to it, that creature
/// becomes a copy of ..." (Assimilation Aegis, Blade of Shared Souls): the
/// copy effect lasts while the source stays attached to the affected object
/// (CR 611.2b), not for the rest of the game.
fn read_for_as_long_as_source_remains_attached(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::cards::builders::{CharacteristicActionAst, SubjectVerbActionAst};
    let sentence = crate::util::trim_edge_punctuation_tokens(input.sentence);
    let Some(comma) = sentence.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    let head = crate::lexer::token_word_refs(&sentence[..comma]);
    if !matches!(
        head.as_slice(),
        ["for", "as", "long", "as", "this", "equipment" | "aura" | "permanent", "remains", "attached", "to", "it"]
    ) {
        return Ok(None);
    }
    let body = &sentence[comma + 1..];
    if body.is_empty() {
        return Ok(None);
    }
    let mut effects = super::parse_effect_chain(body)?;
    let duration = crate::effect::Until::ForAsLongAs(
        ironsmith_core::ContinuousDurationPredicate::ObjectAttachedTo {
            attachment: ironsmith_core::ContinuousDurationObject::Source,
            attached_to: ironsmith_core::ContinuousDurationObject::AffectedObject,
        },
    );
    fn bind(effects: &mut [EffectAst], duration: &crate::effect::Until, bound: &mut usize) {
        for effect in effects.iter_mut() {
            if let EffectAst::SubjectVerb(subject_verb) = effect
                && let SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                    duration: copy_duration,
                    ..
                }) = &mut subject_verb.action
                && matches!(copy_duration, crate::effect::Until::Forever)
            {
                *copy_duration = duration.clone();
                *bound += 1;
                continue;
            }
            crate::model::visit::for_each_nested_effects_mut(effect, false, |nested| {
                bind(nested, duration, bound)
            });
        }
    }
    let mut bound = 0;
    bind(&mut effects, &duration, &mut bound);
    if bound != 1 {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn read_sentence_delayed_timing_suffix(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effects) = super::subject_verb_primitives::parse_sentence_delayed_timing_suffix(
        SubjectVerbPrimitiveClause::new(sentence),
    )? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_sentence_gets_then_fights(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    // The complete pump-and-fight grammar accepts an authored leading
    // `Then`.  Dispatch it before the broad subject/verb fallback, which can
    // otherwise mistake the entire pump clause for presentation text on the
    // first fight participant and discard the executable pump effect.
    if let Some(effects) = super::subject_verb_primitives::parse_sentence_gets_then_fights(
        SubjectVerbPrimitiveClause::new(sentence),
    )? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_compound_gain(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if split_leading_result_prefix_lexed(sentence).is_some() {
        return Ok(None);
    }
    if let Some(effects) = parse_complete_compound_gain_statement(sentence)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_simple_gain_ability(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    // A leading control-flow sentence is not an independent gain statement.
    // Feeding the whole sentence to the gain leaf makes the predicate clause
    // look like the granted object's filter (for example, `that creature is
    // an Ally`), discarding both the conditional node and its demonstrative
    // surface. Let the typed control-flow compositor own that envelope.
    if let Some(shape) =
        effect_grammar::gain_ability_shapes::parse_simple_gain_ability_shape(sentence)
        && shape.complete
        && !shape.subject_tokens.first().is_some_and(|token| {
            token.is_any_word(&[
                "if", "unless", "when", "whenever", "at", "as", "then", "instead", "for",
            ])
        })
        && !shape
            .subject_tokens
            .iter()
            .any(|token| token.is_any_word(&["has", "have", "get", "gets"]))
        && super::lex_chain_helpers::split_effect_chain_on_and_lexed(sentence).len() == 1
        && super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![sentence]).len() == 1
        && let Some(effects) = super::gain_ability::parse_gain_ability_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_trailing_if_clause(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if split_leading_result_prefix_lexed(input.sentence).is_some() {
        return Ok(None);
    }
    // Two conditions joined by "and" are the conditional damage pair's; a run of them is the keyword bundle's.
    if input
        .sentence
        .iter()
        .filter(|token| token.is_word("if"))
        .count()
        >= 2
    {
        return Ok(None);
    }
    // A pump-and-grant with a trailing condition is the compound gain statement's, which keeps the condition.
    if crate::grammar::structure::split_trailing_if_clause_lexed(input.sentence).is_some_and(
        |trailing| {
            effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(
                trailing.leading_tokens,
            )
            .is_some()
                || effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(
                    trailing.leading_tokens,
                )
                .is_some()
        },
    ) {
        return Ok(None);
    }
    let sentence = input.sentence;
    if crate::grammar::structure::split_trailing_if_clause_lexed(sentence).is_some()
        && let Ok(effect) = super::chain_carry::parse_effect_clause_with_trailing_if_lexed(sentence)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
/// "Return target creature you control and all Auras you control attached to
/// it to their owner's hand" (Orzhov Charm): the Auras must be read while
/// still attached, so this is "Return all Auras you control attached to
/// target creature you control to their owner's hand. Return that creature to
/// their owner's hand." — both instructions of one resolution.
fn split_return_with_attached_auras(sentence: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    if sentence.iter().any(|token| token.as_word().is_none()) {
        return None;
    }
    let words = crate::lexer::token_word_refs(sentence);
    // "Return another target permanent you control and all Auras ..."
    // (Essence Reliquary): the target may be qualified by "another".
    let noun_index = match (words.first(), words.get(1), words.get(2)) {
        (Some(&"return"), Some(&"target"), _) => 2,
        (Some(&"return"), Some(&"another"), Some(&"target")) => 3,
        _ => return None,
    };
    let and_all = words.windows(2).position(|pair| pair == ["and", "all"])?;
    let attached = words
        .windows(4)
        .position(|window| window == ["attached", "to", "it", "to"])?;
    if attached <= and_all + 2 {
        return None;
    }
    let target = &sentence[1..and_all];
    let collection = &sentence[and_all + 1..attached];
    let destination = &sentence[attached + 4..];
    let noun = target.get(noun_index - 1)?.as_word()?.to_string();
    if destination.is_empty() || target.len() < noun_index {
        return None;
    }
    let mut rewritten = vec![sentence[0].clone()];
    rewritten.extend_from_slice(collection);
    rewritten.extend(crate::lexer::synthetic_word_tokens(["attached", "to"]));
    if noun_index == 3 {
        // "attached to another target permanent" loses its target in the
        // attachment reading; "attached to target other permanent" keeps
        // both the declared target and the not-this-source exclusion.
        rewritten.push(target[1].clone());
        rewritten.push(crate::lexer::OwnedLexToken::word(
            "other",
            crate::cards::TextSpan::synthetic(),
        ));
        rewritten.extend_from_slice(&target[2..]);
    } else {
        rewritten.extend_from_slice(target);
    }
    rewritten.push(crate::lexer::OwnedLexToken::word("to", crate::cards::TextSpan::synthetic()));
    rewritten.extend_from_slice(destination);
    rewritten.push(crate::lexer::OwnedLexToken::period(crate::cards::TextSpan::synthetic()));
    rewritten.extend(crate::lexer::synthetic_word_tokens(["return", "that"]));
    rewritten.push(crate::lexer::OwnedLexToken::word(noun, crate::cards::TextSpan::synthetic()));
    rewritten.push(crate::lexer::OwnedLexToken::word("to", crate::cards::TextSpan::synthetic()));
    rewritten.extend_from_slice(destination);
    rewritten.push(crate::lexer::OwnedLexToken::period(crate::cards::TextSpan::synthetic()));
    Some(rewritten)
}

/// "return from <zone> to the battlefield <target>" → "return <target> from
/// <zone> to the battlefield".
fn reorder_fronted_return_zones(sentence: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let words = crate::lexer::token_word_refs(sentence);
    if words.len() < 8 || words[0] != "return" || words[1] != "from" {
        return None;
    }
    let to_word = words.iter().position(|word| *word == "to")?;
    if words.get(to_word + 1..to_word + 3)? != ["the", "battlefield"] {
        return None;
    }
    let view = crate::lexer::TokenWordView::new(sentence);
    let from_token = view.map_word_to_token_start(1)?;
    let target_token = view.map_word_to_token_start(to_word + 3)?;
    let zone_tokens = &sentence[from_token..target_token];
    let target_tokens = &sentence[target_token..];
    let (target_tokens, terminal) = match target_tokens.split_last() {
        Some((last, rest)) if last.is_period() => (rest, Some(last)),
        _ => (target_tokens, None),
    };
    if target_tokens.is_empty() {
        return None;
    }
    let mut reordered = Vec::with_capacity(sentence.len());
    reordered.push(sentence[0].clone());
    reordered.extend_from_slice(target_tokens);
    reordered.extend_from_slice(zone_tokens);
    if let Some(terminal) = terminal {
        reordered.push(terminal.clone());
    }
    Some(reordered)
}

fn read_complete_get_pump(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effect) = parse_complete_get_pump_statement(sentence)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_become(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effect) = parse_complete_become_statement(sentence)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_sentence_each_opponent_draws_then_you_draw_per_opponent(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::subject_verb_primitives::parse_sentence_each_opponent_draws_then_you_draw_per_opponent(
        SubjectVerbPrimitiveClause::new(input.sentence),
    )
}
fn read_sentence_you_and_attacking_player_each_draw_and_lose(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_you_and_attacking_player_each_draw_and_lose(
            SubjectVerbPrimitiveClause::new(sentence),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_for_each_target_players(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    // "Exile any number of target players' graveyards": the counted target
    // set is players, each losing their whole graveyard.
    if sentence.first().is_some_and(|token| token.is_word("exile"))
        && let Some(effect) =
            super::zone_handlers::parse_exile_counted_target_players_graveyards(&sentence[1..])
    {
        return Ok(Some(vec![effect]));
    }
    // A counted plural target set owns the trailing `each` action.  The
    // broad simple subject/verb parser can otherwise accept only the final
    // verb and collapse `any number of target players ... each draw` into one
    // unconstrained target player, discarding both the count and qualifier.
    if let Some(effect) = super::for_each_helpers::parse_for_each_target_players_clause(sentence)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_vote(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effect) = super::dispatch_inner::parse_vote_subject_verb(sentence)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_simple_subject_verb(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    // A turn-long lockout-and-permission bundle on the referenced player owns
    // its comma list before the generic restriction reader splits it.
    if let Some(effects) = super::referenced_player_turn_lockout::parse(sentence)? {
        return Ok(Some(effects));
    }
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_you_and_player_each_sacrifice(
            SubjectVerbPrimitiveClause::new(sentence),
        )?
    {
        return Ok(Some(effects));
    }

    if let Some(effects) =
        super::search_library::parse_for_each_revealed_this_way_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_reveal_selected_cards_in_your_hand(
            super::SubjectVerbPrimitiveClause::new(sentence),
        )?
    {
        return Ok(Some(effects));
    }
    if let Some(effects) = super::subject_verb_primitives::parse_sentence_transform_with_followup(
        super::SubjectVerbPrimitiveClause::new(sentence),
    )? {
        return Ok(Some(effects));
    }
    if let Some(effects) = super::subject_verb_primitives::parse_sentence_return_then_create(
        super::SubjectVerbPrimitiveClause::new(sentence),
    )? {
        return Ok(Some(effects));
    }
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_return_then_do_same_for_subtypes(
            super::SubjectVerbPrimitiveClause::new(sentence),
        )?
    {
        return Ok(Some(effects));
    }
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_put_fixed_and_counter_choice(
            super::SubjectVerbPrimitiveClause::new(sentence),
        )?
    {
        return Ok(Some(effects));
    }
    if (crate::grammar::effects::counter_marker_shapes::parse_shared_counter_target_tokens(
        sentence,
    )
    .is_some()
        || crate::grammar::effects::counter_marker_shapes::parse_put_counter_choice_tokens(
            sentence,
        )
        .is_some()
        || crate::grammar::effects::counter_marker_shapes::parse_counter_placement_sequence_tokens(
            sentence,
        )
        .is_some())
        && let Some(effects) = super::subject_verb_primitives::parse_sentence_put_counter_sequence(
            super::SubjectVerbPrimitiveClause::new(sentence),
        )?
    {
        return Ok(Some(effects));
    }
    if let Some(effects) =
        super::chain_carry::parse_repeated_counter_placement_coordination(sentence)?
    {
        return Ok(Some(effects));
    }
    if let Some(effects) =
        super::conditionals::parse_sentence_counter_target_spell_thats_second_cast_this_turn(
            sentence,
        )?
    {
        return Ok(Some(effects));
    }
    if let Some(effect) = parse_complete_simple_subject_verb_sentence(sentence)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_for_each_player(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // The vote reader owns both the collective vote and its received-vote
    // follow-ups, including their per-player tally bindings.
    if super::dispatch_inner::parse_vote_subject_verb(input.sentence)?.is_some() {
        return Ok(None);
    }
    if let Some(effects) = super::subject_verb_primitives::parse_sentence_each_player_may_reveal_selected_cards_in_their_hand(
        super::SubjectVerbPrimitiveClause::new(input.sentence),
    )? {
        return Ok(Some(effects));
    }
    // A choice complement ("each player chooses ... then sacrifices the rest") is the complement statement's.
    if super::dispatch_inner::is_choice_complement_shape(input.sentence) {
        return Ok(None);
    }
    // "Each player may discard their hand and draw seven cards" is its own statement.
    if effect_grammar::sacrifice_discard_shapes::parse_each_player_may_discard_hand_and_draw_tokens(
        input.sentence,
    )
    .is_some()
    {
        return Ok(None);
    }
    // A loop with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(input.sentence).is_some() {
        return Ok(None);
    }
    // A loop that opens with a creation ("each player creates ...") is the create statement's.
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(input.sentence).is_some_and(
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
    let sentence = input.sentence;
    let independently_owned_player_clause =
        effect_grammar::for_each_shapes::parse_participant_clause_shape(sentence).is_some();
    if independently_owned_player_clause
        && let Some(effect) = super::for_each_helpers::parse_for_each_player_clause(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_simple_mill(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effect) = parse_complete_simple_mill_sentence(sentence)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_owner_subject_shuffle(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_shuffle_object_shape_lexed(input.sentence) else {
        return Ok(None);
    };
    if shape.owner_subject_target_tokens.is_none()
        && !matches!(
            crate::util::parse_subject(shape.subject_tokens),
            crate::cards::builders::SubjectAst::Player(crate::cards::builders::PlayerAst::ItsOwner)
        )
    {
        return Ok(None);
    }
    super::search_library::parse_shuffle_object_into_library_sentence(input.sentence)
}
fn read_return_clause(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !input
        .sentence
        .first()
        .is_some_and(|token| token.is_word("return"))
    {
        return Ok(None);
    }
    // "Return from your graveyard to the battlefield any number of target
    // creature cards ..." (Agadeem's Awakening) fronts the origin and
    // destination; read it in the ordinary target-first order.
    if let Some(reordered) = reorder_fronted_return_zones(input.sentence) {
        return crate::clause_support::parse_effect_sentences_lexed(&reordered).map(Some);
    }
    if let Some(rewritten) = split_return_with_attached_auras(input.sentence) {
        return crate::clause_support::parse_effect_sentences_lexed(&rewritten).map(Some);
    }

    if let Some(effects) = super::parse_same_name_target_fanout_sentence(input.sentence)? {
        return Ok(Some(effects));
    }
    if super::lex_chain_helpers::split_effect_chain_on_and_lexed(input.sentence).len() > 1 {
        return Ok(None);
    }

    if let Some(effects) =
        super::dispatch_inner::parse_target_relative_combat_set_sentence(input.sentence)?
    {
        return Ok(Some(effects));
    }
    if let Some(effects) =
        crate::effect_sentences::zone_handlers::parse_return_with_event_timing(input.sentence)?
    {
        return Ok(Some(effects));
    }
    if let crate::recognition::ParseOutcome::Match(plan) =
        effect_grammar::coordination::recognize_coordination(input.sentence)
        && plan
            .value
            .boundaries
            .iter()
            .zip(plan.value.members.iter().skip(1))
            .any(|(boundary, member)| {
                matches!(
                    boundary.operator,
                    crate::model::CoordinationOperatorAst::Or
                        | crate::model::CoordinationOperatorAst::And
                ) && (super::find_verb(member.tokens).is_some_and(|(_, index)| index == 0)
                    || member.head.is_some_and(|head| {
                        matches!(
                            head.actor,
                            effect_grammar::typed_clause_heads::ClauseActorHeadAst::Controller
                                | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Player
                                | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Reference
                        )
                    }))
            })
    {
        return Ok(None);
    }
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(input.sentence)
        .is_some()
    {
        return Ok(None);
    }
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(input.sentence) {
        return Ok(None);
    }
    // A statement with a delayed timing suffix is the delayed statement's.
    if crate::grammar::effects::delayed_step_shapes::parse_delayed_timing_marker_shape(
        input.sentence,
    )
    .is_some_and(|marker| marker.start_word != 0)
    {
        return Ok(None);
    }
    // A statement with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(input.sentence).is_some() {
        return Ok(None);
    }
    let sentence = input.sentence;
    if sentence
        .first()
        .is_some_and(|token| token.is_word("return"))
        && effect_grammar::parse_return_clause_shape(sentence).is_some()
    {
        if let Some(effects) = super::parse_sentence_return_with_counters_on_it_lexed(sentence)? {
            return Ok(Some(effects));
        }
        if let Some(effects) =
            super::subject_verb_primitives::parse_sentence_return_multiple_targets(
                SubjectVerbPrimitiveClause::new(sentence),
            )?
        {
            return Ok(Some(effects));
        }
        return super::zone_handlers::parse_return(&sentence[1..]).map(|effect| Some(vec![effect]));
    }
    Ok(None)
}
fn read_exile_single_segment(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !input
        .sentence
        .first()
        .is_some_and(|token| token.is_word("exile"))
    {
        return Ok(None);
    }

    // The object-filter reader accepts a trailing where clause but cannot
    // bind the enclosing target count. Let the sentence binder own it.
    if effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(input.sentence)
        .is_some()
    {
        return super::dispatch_inner::parse_effect_sentence_with_where_x_lexed(input.sentence)
            .map(Some);
    }
    if let Some(effects) = super::parse_same_name_target_fanout_sentence(input.sentence)? {
        return Ok(Some(effects));
    }
    if super::lex_chain_helpers::split_effect_chain_on_and_lexed(input.sentence).len() > 1 {
        return Ok(None);
    }
    // Some continuations (such as meld) have their own grammar rather than
    // a generic chain verb. An authored boundary still rules out one exile.
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(input.sentence) {
        return Ok(None);
    }
    // A statement with a delayed timing suffix ("... at end of combat") is the delayed statement's.
    if crate::grammar::effects::delayed_step_shapes::parse_delayed_timing_marker_shape(
        input.sentence,
    )
    .is_some_and(|marker| marker.start_word != 0)
    {
        return Ok(None);
    }
    // A statement with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(input.sentence).is_some() {
        return Ok(None);
    }
    let sentence = input.sentence;
    if sentence.first().is_some_and(|token| token.is_word("exile"))
        && super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![sentence]).len() == 1
    {
        if let Some(effects) = super::parse_sentence_exile_source_with_counters_lexed(sentence)? {
            return Ok(Some(effects));
        }
        if effect_grammar::parse_exile_each_target_type_shape(sentence).is_some() {
            return super::parse_exile_up_to_one_each_target_type_sentence(sentence);
        }
        return super::zone_handlers::parse_exile(&sentence[1..], None)
            .map(|effect| Some(vec![effect]));
    }
    Ok(None)
}
fn read_complete_create(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effects) = parse_complete_create_statement(sentence)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_complete_quantified_discard(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    if let Some(effects) = parse_complete_quantified_discard_statement(sentence)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_tap(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(input.sentence)
        .is_some()
    {
        return Ok(None);
    }
    // A tap coordinated with another action is not a bare tap statement.
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(input.sentence)
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(input.sentence).len() > 1
    {
        return Ok(None);
    }
    let sentence = input.sentence;
    if sentence.first().is_some_and(|token| token.is_word("tap")) {
        if let Some(effects) =
            super::optional_companion_fanout::parse_optional_companion_fanout_sentence(sentence)?
        {
            return Ok(Some(effects));
        }
        return super::zone_handlers::parse_tap(&sentence[1..]).map(|effect| Some(vec![effect]));
    }
    Ok(None)
}
fn read_cant_effect(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    let is_document = input.is_document;
    if let Some(effects) =
        super::chain_carry::parse_duration_goaded_and_restricted_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    if is_document
        && !sentence
            .first()
            .is_some_and(|token| token.is_any_word(&["if", "unless"]))
        && let Some(effects) = super::parse_cant_effect_sentence_lexed(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn parse_result_consequence_get_then_gain(
    sentence: &[OwnedLexToken],
    prefix: &crate::grammar::structure::LeadingResultPrefixSpec<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let numeric = crate::grammar::structure::split_leading_numeric_result_prefix_lexed(sentence)
        .is_some();
    let label = if numeric {
        crate::grammar::document_shapes::parse_statement_label_split_tokens(prefix.trailing_tokens)
    } else {
        None
    };
    let mut body = label.map_or(prefix.trailing_tokens, |label| label.body_tokens);
    // The sentence's own terminal period is not part of the ability list.
    while let Some((last, rest)) = body.split_last()
        && matches!(last.kind, crate::lexer::TokenKind::Period)
    {
        body = rest;
    }
    if body
        .first()
        .is_some_and(|token| token.is_any_word(&["if", "unless", "then"]))
        || crate::grammar::structure::split_trailing_if_clause_lexed(body).is_some()
        || super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(body)
        || effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(body).is_some()
    {
        return Ok(None);
    }
    // Only the three-way "gets X and gains A and B" coordination needs the
    // shared owner; a definite description subject ("the creature you
    // control") must keep its reference binding from the generic reading.
    if body.iter().filter(|token| token.is_word("and")).count() < 2
        || body.first().is_some_and(|token| token.is_word("the"))
    {
        return Ok(None);
    }
    let Some(shape) = effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(body) else {
        return Ok(None);
    };
    if shape.ability_verb == effect_grammar::gain_ability_shapes::SharedAbilityVerb::Lose
        || shape
            .ability_tokens
            .iter()
            .any(|token| matches!(token.kind, crate::lexer::TokenKind::Period | crate::lexer::TokenKind::Comma))
    {
        return Ok(None);
    }
    let Some(mut effects) =
        super::gain_ability::parse_gain_ability_sentence_with_typed_subject(body, shape.subject_tokens)?
    else {
        return Ok(None);
    };
    if effects.is_empty() {
        return Ok(None);
    }
    if let Some(label) = label {
        effects = vec![EffectAst::ResultBranchLabel {
            label: crate::lexer::render_token_slice(label.label_tokens)
                .trim()
                .to_string(),
            effects,
        }];
    }
    let conditional = if numeric {
        ConditionalEffectAst::IfResult {
            predicate: prefix.predicate.clone(),
            effects,
        }
    } else {
        match prefix.kind {
            LeadingResultPrefixKind::If => ConditionalEffectAst::IfResult {
                predicate: prefix.predicate.clone(),
                effects,
            },
            LeadingResultPrefixKind::When => ConditionalEffectAst::WhenResult {
                predicate: prefix.predicate.clone(),
                effects,
            },
        }
    };
    Ok(Some(vec![EffectAst::Conditionals(conditional)]))
}

fn read_leading_result_prefix(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(prefix) = split_leading_result_prefix_lexed(input.sentence) else {
        return Ok(None);
    };
    // "When you sacrifice an Aura this way, for each other attacking
    // creature you control, create a token that's a copy of that Aura
    // attached to that creature": a looped consequence with an outer
    // antecedent.
    if let Some(effects) =
        super::for_each_helpers::parse_result_consequence_for_each_object(prefix.trailing_tokens)?
    {
        return Ok(Some(vec![EffectAst::Conditionals(match prefix.kind {
            LeadingResultPrefixKind::If => ConditionalEffectAst::IfResult {
                predicate: prefix.predicate,
                effects,
            },
            LeadingResultPrefixKind::When => ConditionalEffectAst::WhenResult {
                predicate: prefix.predicate,
                effects,
            },
        })]));
    }
    // "If you do, it gets +3/+0 and gains first strike and deathtouch until
    // end of turn": a pump coordinated with an ability list shares one
    // subject and one duration. Read it with the shared get-then-gain owner,
    // as the top-level sentence does, before a generic `and` split hands the
    // trailing duration to the last keyword only.
    if let Some(effects) = parse_result_consequence_get_then_gain(input.sentence, &prefix)? {
        return Ok(Some(effects));
    }
    // A value definition can sit between two actions in this consequence.
    // The single-verb shortcut below cannot consume that whole action chain.
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(
        prefix.trailing_tokens,
    )
    .is_some_and(|shape| shape.followup_tokens.is_some())
    {
        let effects = super::parse_effect_chain_lexed(prefix.trailing_tokens)?;
        return Ok(Some(vec![EffectAst::Conditionals(match prefix.kind {
            LeadingResultPrefixKind::If => ConditionalEffectAst::IfResult {
                predicate: prefix.predicate,
                effects,
            },
            LeadingResultPrefixKind::When => ConditionalEffectAst::WhenResult {
                predicate: prefix.predicate,
                effects,
            },
        })]));
    }
    // A terminal where-X definition ("... up to X target cards ..., where X
    // is ...") belongs to the consequence. The single-verb shortcut below
    // would parse the clause without it and leave X unbound, so route the
    // consequence through the where-X sentence owner.
    if crate::grammar::effects::sentence_predicate_shapes::parse_where_x_sentence_tokens(
        prefix.trailing_tokens,
    )
        .is_some()
        && crate::grammar::structure::split_leading_numeric_result_prefix_lexed(input.sentence)
            .is_none()
        && !prefix
            .trailing_tokens
            .first()
            .is_some_and(|token| token.is_any_word(&["if", "unless"]))
        && let Ok(effects) =
            super::dispatch_inner::parse_effect_sentence_with_where_x_lexed(prefix.trailing_tokens)
        && !effects.is_empty()
    {
        return Ok(Some(vec![EffectAst::Conditionals(match prefix.kind {
            LeadingResultPrefixKind::If => ConditionalEffectAst::IfResult {
                predicate: prefix.predicate,
                effects,
            },
            LeadingResultPrefixKind::When => ConditionalEffectAst::WhenResult {
                predicate: prefix.predicate,
                effects,
            },
        })]));
    }
    if crate::grammar::structure::split_leading_numeric_result_prefix_lexed(input.sentence)
        .is_some()
    {
        let label = crate::grammar::document_shapes::parse_statement_label_split_tokens(
            prefix.trailing_tokens,
        );
        let body = label.map_or(prefix.trailing_tokens, |label| label.body_tokens);
        let mut effects = super::parse_effect_sentence_lexed(body)?;
        if let Some(label) = label {
            effects = vec![EffectAst::ResultBranchLabel {
                label: crate::lexer::render_token_slice(label.label_tokens)
                    .trim()
                    .to_string(),
                effects,
            }];
        }
        return Ok(Some(vec![EffectAst::Conditionals(
            ConditionalEffectAst::IfResult {
                predicate: prefix.predicate,
                effects,
            },
        )]));
    }
    // A nested condition belongs to the result's consequence. Searching for
    // a later verb would discard that condition and the result envelope.
    if prefix
        .trailing_tokens
        .first()
        .is_some_and(|token| token.is_any_word(&["if", "unless"]))
    {
        return super::dispatch_inner::parse_effect_sentence_inner_lexed(input.sentence).map(Some);
    }
    // "When you do, return target Equipment card ... to the battlefield,
    // then attach it to that token" (Ratonhnhaké꞉ton): an ordered action
    // chain is the consequence. The single-verb shortcut below would read
    // only the first action and drop the rest.
    if super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(prefix.trailing_tokens) {
        let effects = super::parse_effect_chain_lexed(prefix.trailing_tokens)?;
        if !effects.is_empty() {
            return Ok(Some(vec![EffectAst::Conditionals(match prefix.kind {
                LeadingResultPrefixKind::If => ConditionalEffectAst::IfResult {
                    predicate: prefix.predicate,
                    effects,
                },
                LeadingResultPrefixKind::When => ConditionalEffectAst::WhenResult {
                    predicate: prefix.predicate,
                    effects,
                },
            })]));
        }
    }
    if let Some(effect) =
        super::clause_pattern_helpers::parse_verb_first_clause(prefix.trailing_tokens)?
    {
        return Ok(Some(vec![match prefix.kind {
            LeadingResultPrefixKind::If => {
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    predicate: prefix.predicate,
                    effects: vec![effect],
                })
            }
            LeadingResultPrefixKind::When => {
                EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                    predicate: prefix.predicate,
                    effects: vec![effect],
                })
            }
        }]));
    }
    Ok(None)
}

fn read_destroy_single_segment(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if effect_grammar::parse_split_all_shape(input.sentence)
        .is_some_and(|shape| shape.connective == effect_grammar::SplitAllConnectiveShape::Or)
    {
        return super::subject_verb_primitives::parse_sentence_destroy_or_exile_all_split(
            super::subject_verb_primitives::SubjectVerbPrimitiveClause::new(input.sentence),
        );
    }
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_destroy_creature_type_of_choice(
            super::subject_verb_primitives::SubjectVerbPrimitiveClause::new(input.sentence),
        )?
    {
        return Ok(Some(effects));
    }
    if let Some(effects) = super::parse_same_name_target_fanout_sentence(input.sentence)? {
        return Ok(Some(effects));
    }
    if let Some(effects) = super::parse_shared_color_target_fanout_sentence(input.sentence)? {
        return Ok(Some(effects));
    }
    if super::lex_chain_helpers::split_effect_chain_on_and_lexed(input.sentence).len() > 1 {
        return Ok(None);
    }
    // The sentence composer binds the local X definition into target filters
    // and target counts before lowering. The destroy leaf alone cannot do so.
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(input.sentence)
        .is_some()
    {
        return Ok(None);
    }
    // A statement with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(input.sentence).is_some() {
        return Ok(None);
    }
    let sentence = input.sentence;
    if sentence
        .first()
        .is_some_and(|token| token.is_word("destroy"))
        && super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![sentence]).len() == 1
        && let Some(effect) = super::clause_pattern_helpers::parse_verb_first_clause(sentence)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}

/// "Creatures it was blocking that had become blocked by only this creature
/// this combat become unblocked." (Ydwen Efreet).
fn read_unblock_sole_blocked_attackers(input: &Statement<'_>) -> Option<Vec<EffectAst>> {
    parse_unblock_sole_blocked_attackers_sentence(input.sentence)
}

pub(super) fn parse_unblock_sole_blocked_attackers_sentence(
    sentence: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = crate::lexer::token_word_refs(crate::util::trim_edge_punctuation_tokens(
        sentence,
    ))
    .into_iter()
    .filter(|word| !matches!(*word, "," | "."))
    .collect::<Vec<_>>();
    // "Creatures it was blocking that hadn't become blocked by another
    // creature this combat become unblocked[, then it blocks an attacking
    // creature of your choice]" (Balduvian Warlord): the same attackers as
    // "blocked by only that creature", named by negation.
    if let Some(rest) = words.strip_prefix(&[
        "creatures", "it", "was", "blocking", "that",
    ][..])
        && let Some(rest) = rest
            .strip_prefix(&["hadn't"][..])
            .or_else(|| rest.strip_prefix(&["hadnt"][..]))
            .or_else(|| rest.strip_prefix(&["had", "not"][..]))
        && let Some(rest) = rest.strip_prefix(&[
            "become", "blocked", "by", "another", "creature", "this", "combat", "become",
            "unblocked",
        ][..])
    {
        let mut effects = vec![EffectAst::UnblockSoleBlockedAttackersOf {
            blocker: crate::cards::builders::TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                None,
            ),
        }];
        if !rest.is_empty() {
            let tail = crate::lexer::synthetic_word_tokens(rest.iter().copied());
            let Ok(Some(block)) = parse_block_attacker_of_choice_sentence(&tail) else {
                return None;
            };
            effects.extend(block);
        }
        return Some(effects);
    }
    let rest = words.strip_prefix(&[
        "creatures", "it", "was", "blocking", "that", "had", "become", "blocked", "by", "only",
    ][..])?;
    // "only this creature" is the source (Ydwen Efreet); "only that
    // creature" is the creature the sentence's "it" names (False Orders).
    let (source, rest) = match rest {
        ["this", rest @ ..] => (true, rest),
        ["that", rest @ ..] => (false, rest),
        _ => return None,
    };
    let rest = match rest {
        ["creature" | "permanent", rest @ ..] => rest,
        rest if source => rest,
        _ => return None,
    };
    let rest = rest
        .strip_prefix(&["this", "combat"][..])
        .unwrap_or(rest);
    (rest == ["become", "unblocked"]).then(|| {
        if source {
            vec![EffectAst::UnblockSoleBlockedAttackers(
                ironsmith_core::UnblockSoleBlockedAttackersEffect::new(
                    crate::target::ChooseSpec::Source,
                ),
            )]
        } else {
            vec![EffectAst::UnblockSoleBlockedAttackersOf {
                blocker: crate::cards::builders::TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    None,
                ),
            }]
        }
    })
}

/// "You may have this land enter tapped." (Mariposa Military Base): the line
/// is an entry program (CR 614.1c), and tapping the entering source there
/// makes it enter tapped.
fn read_optional_self_enter_tapped(input: &Statement<'_>) -> Option<Vec<EffectAst>> {
    let tokens = crate::util::trim_edge_punctuation_tokens(input.sentence);
    let words = crate::lexer::token_word_refs(tokens);
    // The leading "you may" may already have been peeled off by the
    // player-may chain reading, leaving "have this land enter tapped".
    let (optional, rest) = match words.as_slice() {
        ["you", "may", "have", "this", rest @ ..] => (true, rest),
        ["have", "this", rest @ ..] => (false, rest),
        _ => return None,
    };
    let [noun @ .., "enter", "tapped"] = rest else {
        return None;
    };
    if noun.is_empty() || noun.len() > 2 {
        return None;
    }
    let tap = EffectAst::subject_verb_tap(crate::cards::builders::TargetAst::Source(None));
    Some(vec![if optional {
        EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May {
            effects: vec![tap],
        })
    } else {
        tap
    }])
}

/// "You may have it block an attacking creature of your choice." (False
/// Orders): a referenced creature starts blocking a chosen attacker.
fn read_block_attacker_of_choice(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_block_attacker_of_choice_sentence(input.sentence)
}

pub(super) fn parse_block_attacker_of_choice_sentence(
    sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::lexer::token_word_refs(crate::util::trim_edge_punctuation_tokens(
        sentence,
    ))
    .into_iter()
    .filter(|word| !matches!(*word, "," | "."))
    .collect::<Vec<_>>();
    // "then it blocks an attacking creature of your choice" (Balduvian
    // Warlord): the referenced creature itself is the actor, and the block is
    // mandatory.
    let (optional, direct, rest) = match words.strip_prefix(&["you", "may", "have"][..]) {
        Some(rest) => (true, false, rest),
        None => match words.strip_prefix(&["have"][..]) {
            Some(rest) => (false, false, rest),
            None => {
                let rest = words.strip_prefix(&["then"][..]).unwrap_or(words.as_slice());
                (false, true, rest)
            }
        },
    };
    let (blocker, rest) = match rest {
        ["it", rest @ ..] | ["that", "creature", rest @ ..] => (
            crate::cards::builders::TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                None,
            ),
            rest,
        ),
        _ => return Ok(None),
    };
    let block_verb: &[&str] = if direct { &["blocks"] } else { &["block"] };
    let Some(rest) = rest.strip_prefix(block_verb) else {
        return Ok(None);
    };
    let Some(filter_words) = rest.strip_suffix(&["of", "your", "choice"][..]) else {
        return Ok(None);
    };
    if !filter_words.contains(&"attacking") {
        return Ok(None);
    }
    let filter_tokens = crate::lexer::synthetic_word_tokens(filter_words.iter().copied());
    let Ok(mut attackers) = crate::object_filters::parse_object_filter(&filter_tokens, false) else {
        return Ok(None);
    };
    attackers.attacking = true;
    Ok(Some(vec![EffectAst::BlockAttackerOfChoice {
        blocker,
        attackers,
        optional,
    }]))
}

/// "You choose how each player votes this turn." (Illusion of Choice).
fn read_control_votes_this_turn(input: &Statement<'_>) -> Option<Vec<EffectAst>> {
    let tokens = crate::util::trim_edge_punctuation_tokens(input.sentence);
    let (_, rest) = crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&[
            "you", "choose", "how", "each", "player", "votes", "this", "turn",
        ]),
    )?;
    rest.is_empty()
        .then(|| vec![EffectAst::ControlVotesThisTurn])
}

/// "the other chosen player also loses that much life" (Sower of Discord):
/// of the players this source chose, the one other than the player the
/// triggering damage was dealt to.
fn read_other_chosen_player(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::grammar::primitives;
    let tokens = crate::util::trim_edge_punctuation_tokens(input.sentence);
    let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["the", "other", "chosen", "player"]),
    ) else {
        return Ok(None);
    };
    let body = primitives::parse_prefix(rest, primitives::kw("also"))
        .map(|(_, body)| body)
        .unwrap_or(rest);
    if body.is_empty() {
        return Ok(None);
    }
    let mut subject = vec![
        OwnedLexToken::word("that", crate::cards::builders::TextSpan::synthetic()),
        OwnedLexToken::word("player", crate::cards::builders::TextSpan::synthetic()),
    ];
    subject.extend_from_slice(body);
    let effects = super::parse_effect_chain_lexed(&subject)?;
    Ok(Some(vec![EffectAst::ForEach(
        crate::cards::builders::ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
            filter: crate::target::PlayerFilter::excluding(
                crate::target::PlayerFilter::TaggedPlayer(
                    ironsmith_core::tag::SOURCE_CHOSEN_PLAYERS_TAG.into(),
                ),
                crate::target::PlayerFilter::DamagedPlayer,
            ),
            effects,
        },
    )]))
}

/// "It doesn't untap during its controller's next two untap steps."
/// (Telekinesis): the ordinary next-untap-step restriction, covering that
/// many of the controller's untap steps.
fn read_counted_next_untap_steps(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    crate::grammar::effects::parse_counted_next_untap_steps(input.sentence)
}

/// "Then transform any number of Human Werewolves you control." (Tovolar,
/// Dire Overlord): choose any number of matching permanents, then transform
/// each chosen one (CR 701.27).
fn read_transform_any_number(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::grammar::primitives;
    let tokens = crate::util::trim_edge_punctuation_tokens(input.sentence);
    let tokens = primitives::parse_prefix(tokens, primitives::kw("then"))
        .map(|(_, rest)| crate::lexer::trim_lexed_commas(rest))
        .unwrap_or(tokens);
    let Some((_, filter_tokens)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["transform", "any", "number", "of"]),
    ) else {
        return Ok(None);
    };
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = crate::object_filters::parse_object_filter_lexed(filter_tokens, false)?;
    filter.zone.get_or_insert(crate::zone::Zone::Battlefield);
    let tag = crate::util::helper_tag_for_tokens(tokens, "transform_chosen");
    Ok(Some(vec![
        EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: crate::effect::ChoiceCount::any_number(),
            count_value: None,
            player: crate::cards::builders::PlayerAst::You,
            tag: tag.clone(),
        }),
        EffectAst::ForEach(crate::cards::builders::ForEachEffectAst::ForEachTagged {
            tag,
            effects: vec![EffectAst::subject_verb_transform(
                crate::cards::builders::TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    None,
                ),
            )],
        }),
    ]))
}
