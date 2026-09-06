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
use crate::grammar::structure::{LeadingResultPrefixKind, split_leading_result_prefix_lexed};
use crate::cards::builders::{CardTextError, EffectAst, ConditionalEffectAst};
use crate::grammar::effects as effect_grammar;
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
                && !(sentence
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless"])))
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
            !(effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(sentence).is_some())
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
fn read_deal_damage_equal_to_power(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
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
    if split_leading_result_prefix_lexed(sentence).is_some() { return Ok(None); }
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
        && !shape.subject_tokens.first().is_some_and(|token| token.is_any_word(
            &["if", "unless", "when", "whenever", "at", "as", "then", "instead"]
        ))
        && !shape.subject_tokens.iter().any(|token| token.is_any_word(&["has", "have", "get", "gets"]))
        && super::lex_chain_helpers::split_effect_chain_on_and_lexed(sentence).len() == 1
        && super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![sentence]).len() == 1
        && let Some(effects) = super::gain_ability::parse_gain_ability_sentence(sentence)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_trailing_if_clause(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if split_leading_result_prefix_lexed(input.sentence).is_some() { return Ok(None); }
    // Two conditions joined by "and" are the conditional damage pair's; a run of them is the keyword bundle's.
    if input.sentence.iter().filter(|token| token.is_word("if")).count() >= 2 {
        return Ok(None);
    }
    // A pump-and-grant with a trailing condition is the compound gain statement's, which keeps the condition.
    if crate::grammar::structure::split_trailing_if_clause_lexed(input.sentence).is_some_and(|trailing| effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(trailing.leading_tokens).is_some() || effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(trailing.leading_tokens).is_some()) {
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
    if let Some(effect) = parse_complete_simple_subject_verb_sentence(sentence)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_for_each_player(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
    if effect_grammar::sacrifice_discard_shapes::parse_each_player_may_discard_hand_and_draw_tokens(input.sentence).is_some() {
        return Ok(None);
    }
    // A loop with a trailing condition is the conditional statement's.
    if crate::grammar::structure::split_trailing_if_clause_lexed(input.sentence).is_some() {
        return Ok(None);
    }
    // A loop that opens with a creation ("each player creates ...") is the create statement's.
    if effect_grammar::for_each_shapes::parse_participant_clause_shape(input.sentence).is_some_and(|shape| {
        shape.participant_is_actor
            && shape
                .inner_tokens
                .first()
                .is_some_and(|token| token.is_any_word(&["create", "creates"]))
    }) {
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
fn read_owner_subject_shuffle(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_shuffle_object_shape_lexed(input.sentence) else {
        return Ok(None);
    };
    if shape.owner_subject_target_tokens.is_none()
        && !matches!(crate::util::parse_subject(shape.subject_tokens),
            crate::cards::builders::SubjectAst::Player(crate::cards::builders::PlayerAst::ItsOwner))
    { return Ok(None); }
    super::search_library::parse_shuffle_object_into_library_sentence(input.sentence)
}
fn read_return_clause(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !input.sentence.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }

    if let Some(effects) = super::parse_same_name_target_fanout_sentence(input.sentence)? {
        return Ok(Some(effects));
    }
    if super::lex_chain_helpers::split_effect_chain_on_and_lexed(input.sentence).len() > 1 {
        return Ok(None);
    }

    if let Some(effects) = super::dispatch_inner::parse_target_relative_combat_set_sentence(input.sentence)? {
        return Ok(Some(effects));
    }
    if let Some(effects) = crate::effect_sentences::zone_handlers::parse_return_with_event_timing(input.sentence)? {
        return Ok(Some(effects));
    }
    if let crate::recognition::ParseOutcome::Match(plan) = effect_grammar::coordination::recognize_coordination(input.sentence)
        && plan.value.boundaries.iter().zip(plan.value.members.iter().skip(1)).any(|(boundary, member)|
            matches!(boundary.operator, crate::model::CoordinationOperatorAst::Or | crate::model::CoordinationOperatorAst::And)
                && (super::find_verb(member.tokens).is_some_and(|(_, index)| index == 0)
                    || member.head.is_some_and(|head| matches!(head.actor,
                        effect_grammar::typed_clause_heads::ClauseActorHeadAst::Controller
                        | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Player
                        | effect_grammar::typed_clause_heads::ClauseActorHeadAst::Reference)))) {
        return Ok(None);
    }
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(input.sentence).is_some() {
        return Ok(None);
    }
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(input.sentence) {
        return Ok(None);
    }
    // A statement with a delayed timing suffix is the delayed statement's.
    if crate::grammar::effects::delayed_step_shapes::parse_delayed_timing_marker_shape(input.sentence)
        .is_some_and(|marker| marker.start_word != 0) {
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
fn read_exile_single_segment(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !input.sentence.first().is_some_and(|token| token.is_word("exile")) {
        return Ok(None);
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
    if crate::grammar::effects::delayed_step_shapes::parse_delayed_timing_marker_shape(input.sentence)
        .is_some_and(|marker| marker.start_word != 0) {
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
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(input.sentence).is_some() {
        return Ok(None);
    }
    // A tap coordinated with another action is not a bare tap statement.
    if super::lex_chain_helpers::has_authored_comma_then_surface_lexed(input.sentence)
        || super::lex_chain_helpers::split_effect_chain_on_and_lexed(input.sentence).len() > 1 {
        return Ok(None);
    }
    let sentence = input.sentence;
    if sentence.first().is_some_and(|token| token.is_word("tap")) {
        return super::zone_handlers::parse_tap(&sentence[1..]).map(|effect| Some(vec![effect]));
    }
    Ok(None)
}
fn read_cant_effect(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence = input.sentence;
    let is_document = input.is_document;
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
fn read_leading_result_prefix(
    input: &Statement<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(prefix) = split_leading_result_prefix_lexed(input.sentence) else { return Ok(None); };
    if crate::grammar::structure::split_leading_numeric_result_prefix_lexed(input.sentence).is_some() {
        let label = crate::grammar::document_shapes::parse_statement_label_split_tokens(prefix.trailing_tokens);
        let body = label.map_or(prefix.trailing_tokens, |label| label.body_tokens);
        let mut effects = super::parse_effect_sentence_lexed(body)?;
        if let Some(label) = label {
            effects = vec![EffectAst::ResultBranchLabel {
                label: crate::lexer::render_token_slice(label.label_tokens).trim().to_string(),
                effects,
            }];
        }
        return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: prefix.predicate, effects,
        })]));
    }
    // A nested condition belongs to the result's consequence. Searching for
    // a later verb would discard that condition and the result envelope.
    if prefix.trailing_tokens.first().is_some_and(|token| token.is_any_word(&["if", "unless"])) {
        return super::dispatch_inner::parse_effect_sentence_inner_lexed(input.sentence).map(Some);
    }
    if let Some(effect) = super::clause_pattern_helpers::parse_verb_first_clause(prefix.trailing_tokens)? {
        return Ok(Some(vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate: prefix.predicate, effects: vec![effect] }),
            LeadingResultPrefixKind::When => EffectAst::Conditionals(ConditionalEffectAst::WhenResult { predicate: prefix.predicate, effects: vec![effect] }),
        }]));
    }
    Ok(None)
}

fn read_destroy_single_segment(input: &Statement<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
    if effect_grammar::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(input.sentence).is_some() {
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
