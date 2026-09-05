//! Sentence readings 89–103, in rank order.

use super::super::*;
use super::Sentence;

pub(super) fn read_additional_phase(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_triggering_object_had_counters_create(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_triggering_object_had_counters_create_tokens(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_leading_if_conditional(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let quoted_ability_shape = sentence_shapes::parse_quoted_ability_sentence_tokens(tokens);
    let leading_if_shape = sentence_shapes::parse_leading_if_sentence_tokens(tokens);
    // A quoted ability can contain its own verbs. Parse the conditional
    // body as an outer gain grant first so a nested trigger such as
    // `"At the beginning of the end step, sacrifice this permanent."`
    // cannot steal dispatch from `the copy gains ...`.
    if matches!(
        leading_if_shape,
        Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
    ) {
        let conditional = if quoted_ability_shape.is_some() {
                parse_conditional_sentence_family_lexed(
                    tokens,
                    parse_gain_ability_before_effect_chain,
                )
            } else if effect_grammar::control_copy_attach_shapes::contains_source_exiled_owner_library_bottom_shape(tokens)
            {
                parse_conditional_sentence_family_lexed(
                    tokens,
                    parse_effect_chain_preserving_source_exiled_owner_library_bottom,
                )
            } else {
                parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)
            };
        let Some(mut effects) = conditional? else {
            return Err(CardTextError::InvariantViolation(
                "recognized leading-if shape was not claimed by conditional grammar".to_string(),
            ))
            .map(Some);
        };
        if matches!(effects.as_slice(), [EffectAst::Conditional { .. }]) {
            apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
            normalize_search_followup_shuffles(&mut effects);
            return Ok(Some(effects));
        }
        if matches!(effects.as_slice(), [EffectAst::IfResult { .. }]) {
            super::super::super::super::preserve_leading_result_coordination_lexed(
                tokens,
                &mut effects,
            );
            normalize_search_followup_shuffles(&mut effects);
            return Ok(Some(effects));
        }
        return Err(CardTextError::InvariantViolation(
            "leading-if grammar returned a non-conditional effect program".to_string(),
        ))
        .map(Some);
    }
    Ok(None)
}
pub(super) fn read_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Reveal subject=each-player recognizer=top-count-permanents-rest-graveyard",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_consult_traversal_with_inline_followup(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Preserve an inline continuation after a reveal-until traversal before
    // the broad subject/verb recognizer claims only the leading reveal.
    if let Some(effects) =
        super::super::super::super::consult_family::parse_consult_traversal_with_inline_followup(
            tokens,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_where_x_sentence(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A semicolon/comma after the where-X binding begins another effect
    // clause. Route the grammar-confirmed layout before broad gain and
    // subject/verb probes can absorb the trailing clause's subject into
    // the first `gets` modifier and report a malformed binding.
    if effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(tokens).is_some_and(
        |shape| {
            shape.comma_tail_has_effect_clause
                || (shape.has_trailing_segment() && tokens.iter().any(OwnedLexToken::is_semicolon))
        },
    ) {
        crate::parse_trace::event("effect-route: where-x binding with trailing effect clause");
        let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
        apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_gain_ability(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A three-arm continuous clause has one grammatical subject even though
    // its comma before `becomes` also looks like an ordinary effect-chain
    // boundary. Preserve the grammar-confirmed coordinated model before the
    // fallback chain splitter expands the middle arm and treats its subtype
    // payload as a new object-filter subject.
    if let Some(effects) =
        super::super::super::super::gain_ability::parse_gain_ability_sentence(tokens)?
        && is_loss_become_base_pt_coordinated_chain(&effects)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_exile_then_return_same_object(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Same-object exile/return programs own their complete `, then` clause.
    // In particular, a timing suffix on the exile action scopes both actions:
    // "exile it at end of combat, then return it ..." is one delayed program.
    // Route that typed shape before the general comma-then splitter turns it
    // into two immediate zone changes and loses the timing wrapper.
    if let Some(effects) = parse_exile_then_return_same_object_sentence(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Exile subject=explicit recognizer=exile-return-same-object",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_generic_top_cards_exile_counted_face_down_rest_bottom(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A looked-card partition owns its internal `, then` boundary.  Route the
    // grammar-proven full program before the generic chain splitter; otherwise
    // the leading look/exile actions can be mistaken for additional trigger
    // text and only the remainder move survives (for example, Clone Shell's
    // "look ..., exile one face down, then put the rest ..." trigger).
    if let Some(effects) =
        parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(tokens)
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Look subject=implicit recognizer=face-down-looked-partition",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_generic_each_player_exile_top_then_cast_any_number(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The comma-then boundary in the each-player exile-top/cast program is
    // internal to one collection-producing effect.  Its typed recognizer
    // accumulates every iterated player's exiled card under one tag before
    // granting the trailing cast permissions.  Generic chain splitting would
    // instead lower the leading library object as one unowned card and lose
    // both the player loop and the collection relationship.
    if let Some(effects) =
        parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(tokens)?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Exile subject=each-player recognizer=exile-top-cast",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_explicit_comma_then_boundary(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Once the specialist whole-sentence shapes above have had a chance to
    // claim the clause, an authored `, then` boundary must be parsed as an
    // executable chain before the broad subject/verb recognizer runs.  Broad
    // action parsers deliberately accept descriptive suffixes, so asking one
    // of them to parse the whole clause can otherwise keep only the leading
    // action (for example, `create a token, then copy that spell`) and silently
    // discard the follow-up.
    // A where-X binding scopes the complete ordered program. Strip and
    // parse that binding before handing the action body to the chain
    // parser; otherwise both actions survive but the later X remains
    // unbound because the generic chain route never sees the value tail.
    if super::super::super::super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(tokens)
    {
        if has_where_x_value_binding(tokens) {
            let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
            apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
            return Ok(Some(effects));
        }
        return super::super::super::super::parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
pub(super) fn read_put_verb_dispatch(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // `Put ... or remove ... counter` is a single typed counter operation,
    // not the generic action-choice form represented by `UnlessAction`.
    // Let the counter verb handler confirm the complete shape before the
    // broad top-level `or` splitter examines the sentence.
    if tokens.first().is_some_and(|token| token.is_word("put"))
        && let Ok(effect) = super::super::super::super::verb_dispatch::parse_effect_with_verb(
            super::super::super::super::Verb::Put,
            None,
            &tokens[1..],
        )
        && matches!(
            &effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutOrRemoveCounters { .. },
                ..
            })
        )
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_for_each_target_players(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A bounded target-player fanout has an explicit plural target set whose
    // members each perform the trailing action (`Two target players each
    // draw ...`). The broad subject/verb recognizer can parse its final verb
    // while collapsing the counted target phrase to one player, so give the
    // grammar-proven iterator ownership before that fallback.
    if let Some(effect) = super::super::super::super::parse_for_each_target_players_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_or_action_clause(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // An explicit top-level action choice must be split before the broad
    // subject/verb recognizer. Otherwise a later gain/lose verb can accept
    // the complete leading action as an object-filter subject and silently
    // retain only the final ability-grant branch.
    if let Some(unless_action) = super::super::super::super::parse_or_action_clause_lexed(tokens)? {
        return Ok(Some(vec![unless_action]));
    }
    Ok(None)
}
pub(super) fn read_top_level_subject_verb_recognition(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some((route, mut effects)) = parse_top_level_subject_verb_recognition(tokens)? {
        crate::parse_trace::event(format!("effect-route: {route}"));
        normalize_search_followup_shuffles(&mut effects);
        return Ok(Some(effects));
    }
    Ok(None)
}
