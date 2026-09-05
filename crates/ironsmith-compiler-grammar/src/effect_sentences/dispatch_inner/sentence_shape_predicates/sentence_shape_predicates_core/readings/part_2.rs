//! Sentence readings 23–44, in rank order.

use super::super::*;
use super::Sentence;

pub(super) fn read_may_cast_it(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if super::super::super::super::parse_leading_player_may_lexed(tokens).is_some()
        && let Some(spec) = parse_may_cast_it_sentence(tokens)
    {
        return Ok(Some(vec![build_may_cast_tagged_effect(&spec)]));
    }
    Ok(None)
}
pub(super) fn read_generic_top_cards_cloak_counted_rest_bottom(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A looked-card cloak partition owns the apparent `and put` action: the
    // cloaked subset and the library-bottom complement are two dispositions
    // of one looked collection. Claim the fully consumed typed shape before
    // generic coordination splits off the remainder and drops the cloak.
    if let Some(effects) = parse_generic_top_cards_cloak_counted_rest_bottom_subject_verb(tokens) {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Look subject=implicit recognizer=cloak-looked-partition",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_explicit_action_segments(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let explicit_action_segments =
        super::super::super::super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens);
    if explicit_action_segments.len() >= 2
        && !matches!(
            sentence_shapes::parse_leading_if_sentence_tokens(tokens),
            Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
        )
        && explicit_action_segments.iter().all(|segment| {
            super::super::super::super::lex_chain_helpers::segment_has_effect_head_lexed(segment)
        })
    {
        if sentence_shapes::parse_where_x_sentence_tokens(tokens).is_some() {
            return parse_effect_sentence_with_where_x_lexed(tokens).map(Some);
        }
        return super::super::super::super::parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
pub(super) fn read_coordinated_cant_restrictions(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let cant_segments = split_lexed_slices_on_and(tokens);
    if !cant_segments.is_empty()
        && cant_segments.iter().all(|segment| {
            super::super::super::super::super::activation_and_restrictions::find_negation_span(
                segment,
            )
            .is_some()
        })
        && let Some(effects) = parse_cant_effect_sentence_lexed(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_roll_dice_choose_one_result(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // These shapes must be recognized before the broad sentence-shape
    // predicates below. Otherwise a result-prefixed sentence can be claimed
    // by generic target parsing, and a leading roll clause can be reduced to
    // the unsupported `two d6` fragment.
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
pub(super) fn read_sentence_delayed_timing_suffix(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A result gate can govern an action that is scheduled for a later step,
    // as in "If you do, unattach it at the beginning of the next end step."
    // Preserve that timing before the broad result-prefix route strips the
    // suffix and parses only the immediate action. The delayed parser keeps
    // the result gate outside the scheduled payload, so it can still bind to
    // the preceding optional effect.
    if let Some(effects) =
        parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_leading_result_prefix(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let mut trailing_effects =
            super::super::super::super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?;
        if matches!(
            &prefix.predicate,
            crate::cards::builders::IfResultPredicate::Value(_)
        ) {
            bind_numeric_result_counter_amounts(&mut trailing_effects);
        }
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
        super::super::super::super::preserve_leading_result_coordination_lexed(tokens, &mut result);
        return Ok(Some(result));
    }
    Ok(None)
}
pub(super) fn read_leading_player_may(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Explicit player offers must retain both their actor and optionality
    // before broad subject/verb parsing claims the action. This is especially
    // important for split actors such as "that player or that permanent's
    // controller may ...", whose second branch is otherwise discarded.
    // A singular immediate "you may cast it" instruction is a choice
    // made during resolution, not a persistent cast permission. Keep the
    // explicit May wrapper before the broader tagged-permission parser
    // below gets a chance to lower only the cast action.
    // A tagged play/cast permission may itself contain a second authored
    // "you may" in its mana-spending rider. Preserve that complete typed
    // permission before generic chain splitting treats the rider as an
    // unrelated `spend` action.
    if super::super::super::super::parse_leading_player_may_lexed(tokens).is_some() {
        if let Some(spec) = parse_may_cast_it_sentence(tokens) {
            return Ok(Some(vec![build_may_cast_tagged_effect(&spec)]));
        }
        if let Some(effect) = crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)? {
            return Ok(Some(vec![effect]));
        }
        return super::super::super::super::parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
pub(super) fn read_until_duration_triggered(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) =
        super::super::super::super::clause_primitives::parse_until_duration_triggered_clause(
            tokens,
        )?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_sentence_delayed_trigger_this_turn(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A delayed trigger may contain a compound damage fanout as its payload.
    // Preserve the outer `whenever ... this turn` scope before the broad
    // fanout recognizer examines the whole sentence as a direct action.
    if let Some(effects) = parse_sentence_delayed_trigger_this_turn(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_compound_damage_fanout(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::fanout_family::parse_compound_damage_fanout_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_player_villainous_choice(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let conditional_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    if let Some(effects) = parse_player_villainous_choice_statement(conditional_tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_consult_disposition_bundle(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::bundle_rules::parse_consult_disposition_bundle(tokens)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_future_zone_replacement(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) =
        super::super::super::super::dispatch_entry::future_zone_replacement_from_sentence_tokens(
            tokens,
        )
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_delayed_schedule_sentence(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(schedule) =
        effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens)
    {
        let effects = parse_effect_sentence_lexed_inner(schedule.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "delayed schedule sentence missing effect payload".to_string(),
            ))
            .map(Some);
        }
        let delayed = match schedule.step {
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::UntapStep => {
                EffectAst::DelayedUntilNextUntapStep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::Upkeep => {
                EffectAst::DelayedUntilNextUpkeep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::DrawStep => {
                EffectAst::DelayedUntilNextDrawStep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::MainPhase => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextMainPhase { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::FirstMainPhase => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextFirstMainPhase { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::EndStep
                if schedule.start_next_turn =>
            {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerAst::You,
                    PlayerAst::That => PlayerAst::That,
                    PlayerAst::Target => PlayerAst::Target,
                    PlayerAst::TargetOpponent => PlayerAst::TargetOpponent,
                    _ => PlayerAst::Any,
                };
                EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::EndStep => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextEndStep { player, effects }
            }
        };
        return Ok(Some(vec![delayed]));
    }
    Ok(None)
}
pub(super) fn read_sentence_you_and_attacking_player_each_draw_and_lose(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::super::super::subject_verb_primitives::parse_sentence_you_and_attacking_player_each_draw_and_lose(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_if_any_tagged(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let conditional_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    if conditional_tokens.first().is_some_and(|token| token.is_word("if"))
            && let Some(effects) = super::super::super::super::subject_verb_primitives::
                parse_if_any_tagged_cards_share_card_type_with_triggering_spell(
                    SubjectVerbPrimitiveClause::new(conditional_tokens),
                )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_if_enters(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let conditional_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    if conditional_tokens
            .first()
            .is_some_and(|token| token.is_word("if"))
            && let Some(effects) =
                super::super::super::super::subject_verb_primitives::parse_if_enters_with_additional_counter_sentence(
                    SubjectVerbPrimitiveClause::new(conditional_tokens),
                )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_generic_damage_replacement_counters(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The damage-replacement counter form begins with `If`, but its leading
    // clause describes an event rather than a state predicate. Route the
    // typed subject/verb recognizer before the generic conditional parser
    // attempts to interpret that event as a predicate.
    if let Some(effect) = parse_generic_damage_replacement_counters_subject_verb(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_conditional_sentence_family(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let conditional_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    if conditional_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && let Some(effects) =
            parse_conditional_sentence_family_lexed(conditional_tokens, parse_effect_chain_lexed)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_redirect_next_damage(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Redirect clauses begin with an affected-object phrase rather than a
    // normal subject/verb pair (`All damage ... is dealt ...`). Dispatch the
    // typed redirect grammar before the generic extension parser reports a
    // missing verb.
    if let Some(effects) =
        super::super::super::super::clause_pattern_helpers::parse_redirect_next_damage_sentence(
            tokens,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_prevent_next_time_damage(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::clause_pattern_helpers::parse_prevent_next_time_damage_sentence(
            tokens,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
