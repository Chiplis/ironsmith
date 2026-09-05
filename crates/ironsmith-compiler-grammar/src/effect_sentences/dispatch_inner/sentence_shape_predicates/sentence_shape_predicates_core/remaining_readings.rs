//! The readings of one effect sentence between the sentence registries and
//! the chain materializer: the grammar-proven whole-sentence shapes (repeated
//! counter placements, joint draws and creates, target-player resource
//! coordination, delayed "this turn" sentences, trailing "unless", "where X",
//! target gets-clauses, discards, conditionals, tap-or-untap choices, token
//! copy exceptions, choose-target shapes, ...). Formerly a first-match ladder
//! in `sentence_shape_predicates_core`; every reading runs, resolved by rank
//! while the overlaps are measured. The chain materializer is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct RemainingSentence<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl RemainingSentence<'_> {
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
                RuleId::new("sentence-remaining-registry-reading"),
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
    admits: fn(&RemainingSentence<'_>) -> bool,
    read: fn(&RemainingSentence<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("sentence-remaining-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("repeated-counter-placement-coordination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_repeated_counter_placement_coordination(input)),
    },
    Reading {
        id: RuleId::new("atomic-put-counter-for-each"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_atomic_put_counter_for_each(input)),
    },
    Reading {
        id: RuleId::new("joint-draw"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_joint_draw(input)),
    },
    Reading {
        id: RuleId::new("target-player-resource-coordination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_player_resource_coordination(input)),
    },
    Reading {
        id: RuleId::new("joint-create"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_joint_create(input)),
    },
    Reading {
        id: RuleId::new("delayed-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_delayed_this_turn(input)),
    },
    Reading {
        id: RuleId::new("sentence-delayed-next-step-unless-pays"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sentence_delayed_next_step_unless_pays(input)),
    },
    Reading {
        id: RuleId::new("single-sentence-unless-action"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_single_sentence_unless_action(input)),
    },
    Reading {
        id: RuleId::new("sentence-damage-to-that-player-unless-enchanted-attacked"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_sentence_damage_to_that_player_unless_enchanted_attacked(input))
        },
    },
    Reading {
        id: RuleId::new("unless-control-flow"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("single-sentence-unless-action")
        },
        read: |input| input.outcome(read_unless_control_flow(input)),
    },
    Reading {
        id: RuleId::new("where-x-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_sentence(input)),
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
        id: RuleId::new("target-gets-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_gets_clause(input)),
    },
    Reading {
        id: RuleId::new("each-player-return"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("sentence-each-player-return-with-additional-counter")
        },
        read: |input| input.outcome(read_each_player_return(input)),
    },
    Reading {
        id: RuleId::new("leading-discard"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_discard(input)),
    },
    Reading {
        id: RuleId::new("conditional-inline-looked-card-partition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conditional_inline_looked_card_partition(input)),
    },
    Reading {
        id: RuleId::new("leading-if-comma-conditional"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_if_comma_conditional(input)),
    },
    Reading {
        id: RuleId::new("tap-or-untap-all-choice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tap_or_untap_all_choice(input)),
    },
    Reading {
        id: RuleId::new("choose-target-prelude"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_target_prelude(input)),
    },
    Reading {
        id: RuleId::new("compound-damage-fanout"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_compound_damage_fanout(input)),
    },
    Reading {
        id: RuleId::new("create-token-copy-exception"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_create_token_copy_exception(input)),
    },
    Reading {
        id: RuleId::new("choose-target"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("choose-target-prelude")
        },
        read: |input| input.outcome(read_choose_target(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &RemainingSentence<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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

fn read_repeated_counter_placement_coordination(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::chain_carry::parse_repeated_counter_placement_coordination(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_atomic_put_counter_for_each(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if super::super::super::chain_carry::is_atomic_put_counter_for_each_sentence(tokens) {
        return Ok(Some(vec![
            super::super::super::zone_counter_helpers::parse_put_counters(tokens)?,
        ]));
    }
    Ok(None)
}
fn read_joint_draw(input: &RemainingSentence<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A joint draw has two independent player subjects but one shared verb
    // phrase. Claim the complete grammar shape before broad subject/verb
    // parsing retains only the leading `you` actor.
    if effect_grammar::subject_verb_registry_shapes::parse_joint_draw_shape(tokens).is_some()
            && let Some(effects) =
                super::super::super::subject_verb_primitives::parse_sentence_you_and_target_player_each_draw(
                    SubjectVerbPrimitiveClause::new(tokens),
                )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_target_player_resource_coordination(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The shared target-player subject belongs to the coordinated resource
    // program. Claim this grammar-proven shape before any whole-sentence
    // target or modifier probe can commit to the leading `target` token.
    if super::super::super::chain_carry::has_target_player_resource_coordination(tokens) {
        return super::super::super::parse_effect_chain_inner_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_joint_create(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A joint create has two independent actors but one shared verb phrase.
    // Claim the complete grammar shape before the broad imperative-create
    // route can retain only the leading `you` actor.
    if effect_grammar::subject_verb_registry_shapes::parse_joint_create_shape(tokens).is_some()
        && let Some(effects) =
            super::super::super::subject_verb_primitives::parse_sentence_you_and_player_each_create(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_delayed_this_turn(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The complete delayed sentence owns both its trigger and its payload.
    // Claim it before broad payload recognizers (notably quantified player
    // fanout) inspect the outer sentence and try to parse only the trigger
    // header as a recurring ability.
    if effect_grammar::delayed_sentence_shapes::parse_delayed_this_turn_shape(tokens).is_some()
        && let Some(effects) = parse_sentence_delayed_trigger_this_turn(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_sentence_delayed_next_step_unless_pays(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::subject_verb_primitives::parse_sentence_delayed_next_step_unless_pays(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_single_sentence_unless_action(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let contains_quantified_opponent =
        effect_grammar::for_each_shapes::parse_quantified_opponent_presence(tokens);
    if crate::lexer::split_lexed_sentences(tokens).len() == 1
        && !contains_quantified_opponent
        && !tokens.first().is_some_and(|token| token.is_word("if"))
        && !effect_grammar::chain_splitting::has_authored_comma_then_surface_tokens(tokens)
        && effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(tokens).is_some()
        && let Some(effects) = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_sentence_damage_to_that_player_unless_enchanted_attacked(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = super::super::super::subject_verb_primitives::
            parse_sentence_damage_to_that_player_unless_enchanted_attacked(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_unless_control_flow(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Predicate-form trailing `unless` clauses (for example, "unless that
    // creature attacked this turn") are control flow rather than a payment
    // alternative. Route a grammar-proven control-flow plan before the broad
    // damage and target primitives can accept only the leading action and
    // silently discard its postcondition.
    if tokens.iter().any(|token| token.is_word("unless"))
        && !effect_grammar::chain_splitting::has_authored_comma_then_surface_tokens(tokens)
        && matches!(
            effect_grammar::control_flow::recognize_control_flow(tokens),
            crate::recognition::ParseOutcome::Match(_)
        )
    {
        return super::super::super::parse_effect_chain_inner_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_where_x_sentence(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This grammar-proven boundary must precede the broad target-gets
    // fast path below; otherwise that path consumes the semicolon tail as
    // part of the where-X value and never reaches inner dispatch.
    if effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(tokens).is_some_and(
        |shape| shape.has_trailing_segment() && tokens.iter().any(OwnedLexToken::is_semicolon),
    ) {
        return parse_effect_sentence_with_where_x_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_sentence_each_player_return_with_additional_counter(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_sentence_each_player_return_with_additional_counter(
        SubjectVerbPrimitiveClause::new(tokens),
    )? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Return subject=each-player recognizer=return-with-additional-counter",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_target_gets_clause(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let head_words = crate::lexer::parser_token_word_refs(tokens);
    // Commas between parallel target-state adjectives belong to the object
    // filter (`target nonattacking, nonblocking creature`), not to effect
    // coordination. Prove the complete gets-clause and lower it before a
    // comma segment can reach typed-head diagnostics as a verbless object.
    if head_words.first() == Some(&"target")
        && head_words
            .iter()
            .any(|word| matches!(*word, "get" | "gets"))
        && tokens.iter().any(OwnedLexToken::is_comma)
        && let Some(shape) =
            effect_grammar::clause_dispatch_shapes::parse_clause_subject_verb_shape(tokens)
        && let Some(effect) = super::super::super::clause_dispatch::parse_get_pump_clause(
            shape.subject_tokens,
            shape.action_tokens,
            tokens,
        )?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_each_player_return(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let head_words = crate::lexer::parser_token_word_refs(tokens);
    // Destination-first returns often carry comma-separated card-type
    // unions. Preserve the quantified actor and complete return operand
    // before generic coordination sees those type-list commas.
    if crate::word_primitives::parse_any_sequence_prefix(
        &head_words,
        &[
            &["each", "player", "return"],
            &["each", "player", "returns"],
        ],
    ) {
        let view = TokenWordView::new(tokens);
        let return_start = view.token_index_after_words(3).unwrap_or(tokens.len());
        let return_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[return_start..]);
        let mut effect = super::super::super::zone_handlers::parse_return(return_tokens)?;
        super::super::super::chain_carry::bind_implicit_player_context(
            &mut effect,
            PlayerAst::That,
        );
        return Ok(Some(vec![EffectAst::ForEachPlayer {
            effects: vec![effect],
        }]));
    }
    Ok(None)
}
fn read_leading_discard(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A sentence-leading bare discard is an imperative controlled by the
    // ability controller. Do not inherit an event participant (such as a
    // damaged player) merely because the command follows a damage trigger.
    if tokens.first().is_some_and(|token| token.is_word("discard"))
        && !tokens
            .iter()
            .any(|token| token.is_word("if") || token.is_word("unless"))
        && !super::super::super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(tokens)
    {
        let discard_body = crate::util::trim_edge_punctuation_tokens(&tokens[1..]);
        let mut effect = super::super::super::zone_handlers::parse_discard(discard_body, None)?;
        super::super::super::chain_carry::bind_implicit_player_context(&mut effect, PlayerAst::You);
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_conditional_inline_looked_card_partition(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The consequence owns one typed look/selection/remainder program.
    // Prove that program before the general conditional registry asks a
    // broad `look` verb handler to consume the internal `, then` tail.
    if let Some(effects) =
        super::super::super::chain_carry::parse_conditional_inline_looked_card_partition(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_leading_if_comma_conditional(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The legacy conditional dispatcher probes subject/verb primitives
    // before its consequence callback. A repeated damage head must reach
    // that callback intact or its second amount becomes an orphaned
    // verbless clause. Prove the typed body first, then use the ordinary
    // conditional predicate grammar with the dedicated fanout callback.
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(comma) =
            crate::slice_primitives::select_position(tokens, OwnedLexToken::is_comma)
        && super::super::super::fanout_family::parse_compound_damage_fanout_sentence(
            crate::util::trim_edge_punctuation_tokens(&tokens[comma + 1..]),
        )?
        .is_some()
    {
        return effect_grammar::parse_conditional_sentence_with_grammar_entrypoint_lexed(
            tokens,
            parse_required_damage_fanout,
        )
        .map(Some);
    }
    Ok(None)
}
fn read_tap_or_untap_all_choice(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if effect_grammar::chain_carry::parse_tap_or_untap_all_choice_tokens(tokens) {
        let action_tokens = crate::lexer::trim_lexed_commas(&tokens[1..]);
        return Ok(Some(vec![super::super::super::zone_handlers::parse_tap(
            action_tokens,
        )?]));
    }
    Ok(None)
}
fn read_choose_target_prelude(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::clause_pattern_helpers::parse_choose_target_prelude_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_compound_damage_fanout(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::fanout_family::parse_compound_damage_fanout_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_create_token_copy_exception(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let head_words = crate::lexer::parser_token_word_refs(tokens);
    // A token-copy exception is one creation action. Its comma-separated
    // characteristic and ability clauses (`except they're 3/3 ... , and they
    // have flying`) are copiable-value modifiers, not independent effects.
    // Let the complete typed creation grammar prove the sentence before the
    // generic chain splitter can expose the verbless `except` tail.
    if head_words.first() == Some(&"create")
        && crate::word_primitives::sequence_occurs(&head_words, &["except"])
        && head_words
            .iter()
            .any(|word| matches!(*word, "token" | "tokens"))
        && head_words
            .iter()
            .any(|word| matches!(*word, "copy" | "copies"))
        && let Ok(
            effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy { .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                ..
            }),
        ) = super::super::super::parse_create(tokens, None)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_choose_target(
    input: &RemainingSentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Triggered, activated, and modal bodies can enter this single-sentence
    // dispatcher without passing through the document-level sentence loop.
    // A complete target declaration containing a relative history clause
    // ("cards ... that were put there") must remain one typed target effect;
    // otherwise the subject/verb planner can reinterpret the relative `put`
    // as a zone-change action.
    if let Some(shape) = effect_grammar::clause_dispatch_shapes::parse_choose_target_shape(tokens)
        && parse_target_phrase(shape.target_tokens).is_ok()
    {
        return Ok(Some(vec![super::super::super::parse_effect_clause_lexed(
            tokens,
        )?]));
    }
    Ok(None)
}
