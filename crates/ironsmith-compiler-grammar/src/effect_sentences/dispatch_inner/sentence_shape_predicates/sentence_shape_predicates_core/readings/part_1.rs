//! Sentence readings 1–22, in rank order.

use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ZoneMoveActionAst;
use super::super::*;
use super::Sentence;

pub(super) fn read_win_the_game(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A terminal win-game action has an explicit player subject but no
    // ordinary controller verb. Claim it before the typed subject/verb head
    // registry commits on `you` and reports an unsupported action. Trigger
    // splitting commonly removes the sentence-end token, so this route must
    // accept both standalone and embedded effect surfaces.
    if let Some(effect) =
        super::super::super::super::clause_pattern_helpers::parse_win_the_game_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_source_and_blocked_creatures_top_library_shuffle(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This joint source-and-blocked-object move is one typed iteration over
    // two object filters, followed by an owner-relative shuffle. Claim the
    // complete grammar-proven sentence before broad `put` and coordination
    // routes can commit on only `this creature` and discard the second set.
    if let Some(effect) = parse_source_and_blocked_creatures_top_library_shuffle_sentence(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_deals_damage_word_view(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let words = crate::lexer::parser_token_word_refs(tokens);
    // The final `and planeswalker it has dealt damage to this game` is the
    // object half of one historical damage-recipient union, not a second
    // executable action. Prove that the damage body lowers to the typed
    // player/object history sequence before the generic action-chain splitter
    // sees the embedded `has dealt` verb.
    if let Some(verb_word_idx) =
        crate::slice_primitives::select_position(&words, |word| matches!(*word, "deal" | "deals"))
    {
        let word_view = TokenWordView::new(tokens);
        let body_start = word_view
            .map_word_or_end_to_token_boundary(verb_word_idx + 1)
            .unwrap_or(tokens.len());
        let body = trim_edge_punctuation(&tokens[body_start..]);
        let historical_union =
                super::super::super::super::verb_handlers::is_historical_player_object_damage_recipient_clause(&body);
        if historical_union {
            crate::parse_trace::event(
                "effect-route: subject-verb verb=Deal subject=source recognizer=historical-player-object-union",
            );
            return Ok(Some(vec![
                super::super::super::super::verb_handlers::parse_deal_damage(&body)?,
            ]));
        }
    }
    Ok(None)
}
pub(super) fn read_becomes_word_view(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let words = crate::lexer::parser_token_word_refs(tokens);
    // A copy exception is part of the `becomes a copy` action, even though
    // its comma-separated tail has an ordinary `has` verb. Parse the complete
    // typed animation before generic coordination can reinterpret
    // `except it has ...` as an independent battlefield anthem.
    if super::super::super::super::parse_leading_player_may_lexed(tokens).is_none()
        && let Some(become_word_idx) =
            crate::slice_primitives::select_last_position(&words, |word| {
                matches!(*word, "become" | "becomes")
            })
    {
        let view = TokenWordView::new(tokens);
        let subject_end = view
            .map_word_or_end_to_token_boundary(become_word_idx)
            .unwrap_or(tokens.len());
        let body_start = view
            .map_word_or_end_to_token_boundary(become_word_idx + 1)
            .unwrap_or(tokens.len());
        let subject = trim_edge_punctuation(&tokens[..subject_end]);
        let body = trim_edge_punctuation(&tokens[body_start..]);
        if !subject.is_empty()
            && !body.is_empty()
            && effect_grammar::become_shapes::parse_become_rest_shape(&body)
                .copy_exception
                .is_some()
        {
            return Ok(Some(vec![
                super::super::super::super::clause_dispatch::parse_become_clause(&subject, &body)?,
            ]));
        }
    }
    Ok(None)
}
pub(super) fn read_and_can_attack(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let words = crate::lexer::parser_token_word_refs(tokens);
    // A trailing subjectless permission belongs to the same subject as the
    // preceding action: "this creature gets ... and can attack ... as though
    // it didn't have defender."  The standalone permission recognizer cannot
    // treat the whole prefix as an object subject, while the broad effect
    // chain otherwise mistakes the comparison's `have defender` for an
    // ability grant. Split only the grammar-proven final `and can attack`
    // clause, parse the prefix normally, and reattach the typed permission to
    // the prefix's explicit subject.
    // The trailing clause is deliberately subjectless. Bind it to
    // the preceding effect-chain result (`it`) rather than
    // reparsing the original `target ...` phrase as an untargeted
    // all-objects filter.
    if let Some(and_word_idx) =
        crate::word_primitives::parse_last_sequence_start(&words, &["and", "can", "attack"])
    {
        let word_view = TokenWordView::new(tokens);
        let and_token_idx = word_view
            .map_word_or_end_to_token_boundary(and_word_idx)
            .unwrap_or(tokens.len());
        let can_token_idx = word_view
            .map_word_or_end_to_token_boundary(and_word_idx + 1)
            .unwrap_or(tokens.len());
        let prefix = trim_edge_punctuation(&tokens[..and_token_idx]);
        if !prefix.is_empty()
            && let Some((_, verb_word_idx)) =
                super::super::super::super::lex_chain_helpers::find_verb_lexed(&prefix)
        {
            let prefix_words = TokenWordView::new(&prefix);
            let subject_end = prefix_words
                .map_word_or_end_to_token_boundary(verb_word_idx)
                .unwrap_or(prefix.len());
            let subject = trim_edge_punctuation(&prefix[..subject_end]);
            if !subject.is_empty() {
                if let Some(permission) =
                    parse_can_attack_as_though_no_defender_clause(&tokens[can_token_idx..])?
                {
                    let parsed_prefix = parse_effect_sentence_lexed(&prefix)?;
                    if !parsed_prefix.is_empty() {
                        let mut coordinated = Vec::new();
                        for effect in parsed_prefix {
                            match effect {
                                EffectAst::Coordinated { effects, .. } => {
                                    coordinated.extend(effects);
                                }
                                effect => coordinated.push(effect),
                            }
                        }
                        coordinated.push(permission);
                        return Ok(Some(vec![EffectAst::Coordinated {
                            effects: coordinated,
                            leading_duration: false,
                            result_conjunction: false,
                        }]));
                    }
                }
            }
        }
    }
    Ok(None)
}
pub(super) fn read_can_attack_as_though_no_defender(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This permission contains the words `have defender`, but those words are
    // inside an `as though` comparison rather than an ability to grant. Claim
    // the complete typed combat-permission clause before the broad gain-
    // ability routes can reduce it to granting defender itself.
    if let Some(effect) = parse_can_attack_as_though_no_defender_clause(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_each_prior_affected_object_controller_mana_value_life(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Keep a demonstrative per-object reward ahead of the broad gain-life
    // sentence parser. The latter can otherwise reduce "the controller of
    // each of those artifacts" to the ability controller and discard both
    // the iteration and prior-result provenance.
    if let Some(effect) =
            super::super::super::super::chain_carry::parse_each_prior_affected_object_controller_mana_value_life(
                tokens,
            )?
        {
            return Ok(Some(vec![effect]));
        }
    Ok(None)
}
pub(super) fn read_destroy_attached_object_then_source_damage_to_controller(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_destroy_attached_object_then_source_damage_to_controller(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_as_you_cast_from_zone_this_turn_grant(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This grammar-proven cast-origin grant must own the complete sentence.
    // Later gain-ability routes permissively accept the leading `as you cast`
    // phrase as an object-filter subject and lose both the hand provenance
    // and authored duration surface.
    if let Some(effect) = parse_as_you_cast_from_zone_this_turn_grant(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_sentence_delayed_next_step_unless_pays(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Delayed payment clauses can name any supported next step. Route the
    // complete sentence before broad subject/verb parsing splits at `unless`;
    // otherwise an action-first draw-step clause is reduced to a life-loss
    // action with an unsupported timing tail. Parsing the action prefix
    // recurses with the timing marker removed, so this route terminates.
    if let Some(effects) =
        parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_attacking_doesnt_tap_if_source_untapped(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_attacking_doesnt_tap_if_source_untapped(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_trailing_if_clause(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A trailing condition on a mass-destruction instruction governs whether
    // that instruction happens. Route the grammar-proven condition before the
    // broad destroy subject/verb primitive can consume only its leading
    // action and silently discard the predicate.
    if crate::grammar::structure::split_trailing_if_clause_lexed(tokens).is_some()
        && let Ok(effect) =
            super::super::super::super::chain_carry::parse_effect_clause_with_trailing_if_lexed(
                tokens,
            )
        && matches!(
            &effect,
            EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                effects,
                ..
            }) if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { .. }),
                    ..
                })]
            )
        )
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_each_player_exile_sacrifice_return_exiled(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::super::super::player_subject_sequences::parse_each_player_exile_sacrifice_return_exiled(
                tokens,
            )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_may_have_any_number_tagged_phase_out(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) =
        super::super::super::super::chain_carry::parse_may_have_any_number_tagged_phase_out_lexed(
            tokens,
        )
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_if_you_dont(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::dispatch_entry::parse_if_you_dont_sentence(tokens)?
    {
        return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::ExplicitDidNot,
            effects,
        })]));
    }
    Ok(None)
}
pub(super) fn read_sentence_damage_unless_controller_has_source_deal_damage(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A causative `unless its controller has <source> deal ...` clause is one
    // action choice. The broad subject/verb recognizer can otherwise claim
    // only the embedded damage phrase and discard both the primary action
    // and the `unless` relationship.
    if let Some(effects) = super::super::super::super::subject_verb_primitives::
            parse_sentence_damage_unless_controller_has_source_deal_damage(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_shared_color_target_fanout(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Shared-characteristic fanouts are one linked target set. In particular,
    // a broad destroy parser must not reduce `target enchantment and each
    // other enchantment that shares a color with it` to two unrelated
    // targets before the typed relation is recorded.
    if let Some(effects) =
        super::super::super::super::fanout_family::parse_shared_color_target_fanout_sentence(
            tokens,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_keyword_bundle_pump(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A keyword-bundle pump contains an authored `and so on for ...` list,
    // not a conjunction of executable actions. Route that complete typed
    // shape before the broad leading-duration chain predicate can split it
    // into only the first two conditional pump clauses.
    if let Some(effects) =
            super::super::super::super::subject_verb_special_recognizers::parse_keyword_bundle_pump_sentence(tokens)?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_coordinated_leading_duration(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A genuine top-level conjunction with a leading duration needs chain
    // carry before broad gain/subject recognizers see an isolated arm. The
    // grammar predicate rejects quoted/list conjunctions and `then` chains.
    if effect_grammar::chain_carry::coordinated_effect_chain_leading_duration(tokens) == Some(true)
    {
        return super::super::super::super::parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
pub(super) fn read_explicit_assign_no_combat_damage_followup(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_explicit_assign_no_combat_damage_followup(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_source_gets_unblockable(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A source pump followed by `can't be blocked this turn` is one shared-
    // subject program. Preserve both typed effects before any generic chain
    // or prohibition route can reinterpret the leading source/pump words as
    // the blocked-object filter and silently retain only the restriction.
    if let Some(effects) = parse_source_gets_unblockable_subject_verb(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_target_gets_unblockable(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The target variant is the same atomic program: one explicit target,
    // one P/T modification, and a same-target blocking restriction. Give it
    // the same early ownership as the source form so the broad pump route
    // cannot accept only the leading `gets ...` clause and discard the
    // coordinated `can't be blocked this turn` tail.
    if let Some(effects) = parse_target_gets_unblockable_subject_verb(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
