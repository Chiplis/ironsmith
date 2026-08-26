use super::*;
use crate::grammar::effects as effect_grammar;
use crate::grammar::effects::delayed_step_shapes as delayed_grammar;

const DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_WORDS: &[&str] = &["you", "choose", "one", "of", "them"];
const DELAYED_VENTURE_DUNGEON_WORDS: &[&str] = &["venture", "into", "the", "dungeon"];
const DELAYED_STILL_LAND_WORDS: &[&[&str]] = &[
    &["its", "still", "a", "land"],
    &["it", "still", "a", "land"],
];
fn parse_delayed_player_prefix(words: &[&str]) -> Option<(PlayerAst, usize)> {
    delayed_grammar::parse_delayed_player_prefix_words(words, false)
}

fn parse_delayed_player_before_pay(words: &[&str]) -> Option<(PlayerAst, usize)> {
    delayed_grammar::parse_delayed_player_prefix_words(words, true)
}

fn delayed_lose_game_unless_paid_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    delayed_grammar::is_delayed_lose_game_unless_paid_shape(clause.tokens())
}

fn delayed_clause_mentions_cast_or_play_action(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    delayed_grammar::delayed_action_shape(
        clause.tokens(),
        delayed_grammar::DelayedActionShape::CastOrPlay,
        false,
    )
}

fn delayed_clause_starts_with_mechanic_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
    marker_prefixes: &'static [&'static [&'static str]],
) -> bool {
    delayed_grammar::delayed_starts_any_shape(clause.tokens(), marker_prefixes)
}

fn delayed_clause_mentions_remains_tapped(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    delayed_grammar::delayed_mentions_remains_tapped_shape(clause.tokens())
}

fn delayed_clause_starts_with_action(
    clause: SubjectVerbPrimitiveClause<'_>,
    action: delayed_grammar::DelayedActionShape,
) -> bool {
    delayed_grammar::delayed_action_shape(clause.tokens(), action, true)
}

fn delayed_clause_mentions_mana_cost(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    delayed_grammar::delayed_mentions_mana_cost_shape(clause.tokens())
}

pub(super) fn wrap_delayed_next_step_unless_pays(
    step: DelayedNextStepKind,
    player: PlayerAst,
    effects: Vec<EffectAst>,
) -> EffectAst {
    match step {
        DelayedNextStepKind::Upkeep => EffectAst::DelayedUntilNextUpkeep { player, effects },
        DelayedNextStepKind::DrawStep => EffectAst::DelayedUntilNextDrawStep { player, effects },
        DelayedNextStepKind::EndStep => EffectAst::DelayedUntilNextEndStep {
            player: delayed_end_step_player_filter(player).unwrap_or(PlayerFilter::Any),
            effects,
        },
        DelayedNextStepKind::CleanupStep => EffectAst::DelayedUntilNextCleanupStep {
            player: delayed_end_step_player_filter(player).unwrap_or(PlayerFilter::Any),
            effects,
        },
        DelayedNextStepKind::EndOfCombat => EffectAst::DelayedUntilEndOfCombat { effects },
    }
}

/// Wrap an effect that a verb handler has already parsed when its remaining
/// clause is a delayed-step timing marker followed by an unless payment. This
/// is the local counterpart of the whole-sentence parser below: it keeps
/// subject-aware verb handlers from rejecting the timing tail after they have
/// consumed the resource amount and noun.
pub fn wrap_parsed_effect_in_delayed_next_step_unless_pays(
    trailing: &[OwnedLexToken],
    effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    let timing_clause = SubjectVerbPrimitiveClause::new(trailing).trimmed();
    let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(timing_clause)
    else {
        return Ok(None);
    };
    if timing_start_word != 0 {
        return Ok(None);
    }
    let Some(unless_idx) = timing_clause.find_token_word("unless") else {
        return Ok(None);
    };
    let Some(unless_effect) = try_build_unless(vec![effect], timing_clause, unless_idx)? else {
        return Ok(None);
    };
    Ok(Some(wrap_delayed_next_step_unless_pays(
        step,
        player,
        vec![unless_effect],
    )))
}

fn delayed_end_step_player_filter(player: PlayerAst) -> Option<PlayerFilter> {
    Some(match player {
        PlayerAst::You => PlayerFilter::You,
        PlayerAst::Any => PlayerFilter::Any,
        PlayerAst::That => PlayerFilter::IteratedPlayer,
        _ => return None,
    })
}

fn wrap_delayed_timing_effects(
    marker: delayed_grammar::DelayedTimingMarkerShape,
    effects: Vec<EffectAst>,
) -> Option<EffectAst> {
    Some(match marker.step {
        delayed_grammar::DelayedTimingStepShape::EndStep => EffectAst::DelayedUntilNextEndStep {
            player: delayed_end_step_player_filter(marker.player)?,
            effects,
        },
        delayed_grammar::DelayedTimingStepShape::Upkeep => EffectAst::DelayedUntilNextUpkeep {
            player: marker.player,
            effects,
        },
        delayed_grammar::DelayedTimingStepShape::DrawStep => EffectAst::DelayedUntilNextDrawStep {
            player: marker.player,
            effects,
        },
        delayed_grammar::DelayedTimingStepShape::CleanupStep => {
            EffectAst::DelayedUntilNextCleanupStep {
                player: delayed_end_step_player_filter(marker.player)?,
                effects,
            }
        }
        delayed_grammar::DelayedTimingStepShape::EndOfCombat => {
            EffectAst::DelayedUntilEndOfCombat { effects }
        }
    })
}

fn wrap_delayed_timing_inside_leading_condition(
    marker: delayed_grammar::DelayedTimingMarkerShape,
    effects: Vec<EffectAst>,
) -> Option<EffectAst> {
    let [
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = effects.as_slice()
    else {
        return wrap_delayed_timing_effects(marker, effects);
    };
    if !if_false.is_empty() {
        return wrap_delayed_timing_effects(marker, effects);
    }

    Some(EffectAst::Conditional {
        predicate: predicate.clone(),
        if_true: vec![wrap_delayed_timing_effects(marker, if_true.clone())?],
        if_false: Vec::new(),
    })
}

/// Parse an action whose timing marker is written as a suffix, such as
/// "sacrifice it at the beginning of the next end step". The action is parsed
/// normally after removing only the timing marker, then wrapped in the same
/// delayed queue AST used by prefix-timed sentences.
pub fn parse_sentence_delayed_timing_suffix(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = clause.trimmed();
    let Some(mut marker) = delayed_grammar::parse_delayed_timing_marker_shape(clause.tokens())
    else {
        return Ok(None);
    };
    if marker.start_word == 0 {
        return Ok(None);
    }

    let Some(before_timing) = clause.before_word(marker.start_word) else {
        return Ok(None);
    };
    // In “return it ... under its owner's control at the beginning of their
    // next upkeep”, “their” names that object's owner. Preserve that typed
    // relation instead of treating the pronoun as an unrelated iterated
    // player, which could make the delayed trigger fire on the wrong upkeep.
    if marker.player == PlayerAst::That
        && crate::word_primitives::sequence_occurs(
            &before_timing.trimmed_word_refs(),
            &["under", "its", "owners", "control"],
        )
    {
        marker.player = PlayerAst::ItsOwner;
    }
    let Some(after_timing) = clause.from_word(marker.end_word) else {
        return Ok(None);
    };
    let after_words = after_timing.trimmed_word_refs();
    if !after_words.is_empty() && !matches!(after_words.first(), Some(&"where")) {
        return Ok(None);
    }

    // A trailing where-clause remains part of the delayed action. In
    // particular, this keeps dynamic values such as "where X is the number of
    // lands you control at that time" evaluated when the delayed trigger
    // resolves rather than when it is created.
    let mut action_tokens = before_timing.tokens().to_vec();
    action_tokens.extend_from_slice(after_timing.tokens());
    let action_tokens = SubjectVerbPrimitiveClause::new(&action_tokens).trimmed();
    if action_tokens.is_empty() {
        return Ok(None);
    }
    // Result gates describe whether the immediate preceding effect happened,
    // while the timing suffix schedules only this sentence's action. Keep the
    // gate outside the delayed wrapper so reference resolution can bind it to
    // the preceding sibling rather than looking for a result inside a fresh
    // delayed-effect sequence.
    let leading_result =
        crate::grammar::structure::split_leading_result_prefix_lexed(action_tokens.tokens());
    let action_body = leading_result
        .as_ref()
        .map(|prefix| prefix.trailing_tokens)
        .unwrap_or_else(|| action_tokens.tokens());
    let segments = super::super::lex_chain_helpers::split_effect_chain_on_and_lexed(action_body);
    let split_last_coordinated_action = segments.len() > 1
        && segments
            .iter()
            .all(|segment| super::super::lex_chain_helpers::segment_has_effect_head_lexed(segment));
    let (mut immediate_effects, delayed_effects) = if split_last_coordinated_action {
        let mut immediate = Vec::new();
        for segment in &segments[..segments.len() - 1] {
            immediate.extend(parse_effect_chain(segment)?);
        }
        let delayed = parse_effect_chain(
            segments
                .last()
                .expect("multi-segment delayed suffix has a final action"),
        )?;
        (immediate, delayed)
    } else {
        (Vec::new(), parse_effect_chain(action_body)?)
    };
    if delayed_effects.is_empty() {
        return Ok(None);
    }

    let Some(delayed) = wrap_delayed_timing_inside_leading_condition(marker, delayed_effects)
    else {
        return Ok(None);
    };
    immediate_effects.push(delayed);
    let effect = match leading_result {
        Some(prefix) => match prefix.kind {
            crate::grammar::structure::LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: immediate_effects,
            },
            crate::grammar::structure::LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: immediate_effects,
            },
        },
        None => return Ok(Some(immediate_effects)),
    };
    Ok(Some(vec![effect]))
}

pub fn find_unquoted_token_word(
    clause: SubjectVerbPrimitiveClause<'_>,
    word: &str,
) -> Option<usize> {
    clause.find_unquoted_token_word(word)
}

fn bind_unless_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::UnlessPays {
            player: unless_player,
            effects,
            ..
        } => {
            if matches!(*unless_player, PlayerAst::Implicit) {
                *unless_player = player;
            }
            for nested in effects {
                bind_unless_player_context(nested, player);
            }
        }
        EffectAst::UnlessAction {
            player: unless_player,
            effects,
            alternative,
        } => {
            if matches!(*unless_player, PlayerAst::Implicit) {
                *unless_player = player;
            }
            for nested in effects {
                bind_unless_player_context(nested, player);
            }
            for nested in alternative {
                bind_unless_player_context(nested, player);
            }
        }
        _ => bind_implicit_player_context(effect, player),
    }
}

fn causative_recipient_filter(player: PlayerAst) -> Option<PlayerFilter> {
    Some(match player {
        PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
        PlayerAst::Any => PlayerFilter::Any,
        PlayerAst::Opponent => PlayerFilter::Opponent,
        PlayerAst::Target => PlayerFilter::target_player(),
        PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
        PlayerAst::That => PlayerFilter::IteratedPlayer,
        PlayerAst::ItsController => PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
        PlayerAst::ItsOwner => PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target),
        _ => return None,
    })
}

fn parse_causative_source_damage_to_player(
    clause: SubjectVerbPrimitiveClause<'_>,
    player: PlayerAst,
) -> Option<EffectAst> {
    let shape = effect_grammar::parse_source_damage_to_decider(clause.tokens())?;
    let (amount, used) = parse_value(shape.damage_tokens)?;
    let tail = SubjectVerbPrimitiveClause::new(&shape.damage_tokens[used..]).trimmed();
    let words = tail.word_refs();
    if !crate::word_primitives::first_is(&words, "damage")
        || !crate::word_primitives::parse_any_sequence_suffix(
            &words,
            &[&["to", "them"], &["to", "that", "player"]],
        )
    {
        return None;
    }
    Some(EffectAst::subject_verb_damage(
        amount,
        TargetAst::Player(causative_recipient_filter(player)?, None),
    ))
}

fn rewrite_value_source_to_it_tag(value: &mut Value) {
    match value {
        Value::SurfaceHinted { value, .. } => rewrite_value_source_to_it_tag(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            rewrite_value_source_to_it_tag(left);
            rewrite_value_source_to_it_tag(right);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => rewrite_value_source_to_it_tag(inner),
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec)
            if matches!(spec.as_ref(), crate::target::ChooseSpec::Source) =>
        {
            *spec = Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG)));
        }
        _ => {}
    }
}

fn rewrite_cost_source_values_to_it_tag(
    cost: &mut ironsmith_core::TotalCost<crate::model::CompilerCost>,
) {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(_) => {
            let mut components = cost.costs().to_vec();
            for component in &mut components {
                match component {
                    crate::model::CompilerCost::DynamicMana(dynamic) => {
                        if let Some(value) = dynamic.x_value.as_mut() {
                            rewrite_value_source_to_it_tag(value);
                        }
                        if let Some(value) = dynamic.additional_generic.as_mut() {
                            rewrite_value_source_to_it_tag(value);
                        }
                        if let Some(value) = dynamic.multiplier.as_mut() {
                            rewrite_value_source_to_it_tag(value);
                        }
                    }
                    crate::model::CompilerCost::Life(value) => {
                        rewrite_value_source_to_it_tag(value)
                    }
                    crate::model::CompilerCost::SacrificeSelf { .. } => {
                        *component = crate::model::CompilerCost::Sacrifice {
                            count: crate::effect::ChoiceCount::exactly(1),
                            filter: crate::target::ObjectFilter::tagged(TagKey::from(IT_TAG)),
                            all: false,
                            binding: None,
                        };
                    }
                    crate::model::CompilerCost::Sacrifice { filter, .. } if filter.source => {
                        *filter = crate::target::ObjectFilter::tagged(TagKey::from(IT_TAG));
                    }
                    _ => {}
                }
            }
            *cost = ironsmith_core::TotalCost::from_costs(components);
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut branches = branches.to_vec();
            for branch in &mut branches {
                rewrite_cost_source_values_to_it_tag(branch);
            }
            *cost = ironsmith_core::TotalCost::one_of(branches);
        }
    }
}

pub fn rewrite_unless_cost_source_values_to_it_tag(effect: &mut EffectAst) {
    if let EffectAst::UnlessPays { cost, .. } = effect {
        rewrite_cost_source_values_to_it_tag(cost);
    }
}

pub fn parse_sentence_delayed_next_step_unless_pays(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_period_segments();
    if segments.is_empty() {
        return Ok(None);
    }

    let (leading_segments, final_segment) = segments.split_at(segments.len() - 1);
    if let Some(after_timing) =
        delayed_grammar::parse_next_end_step_prefix_remainder(final_segment[0].tokens())
    {
        let timing_clause = SubjectVerbPrimitiveClause::new(after_timing).trimmed();
        if timing_clause.is_empty() {
            return Ok(None);
        }
        let Some(unless_idx) = timing_clause.find_token_word("unless") else {
            return Ok(None);
        };
        let delayed_effect_clause = timing_clause.before(unless_idx).trimmed();
        if delayed_effect_clause.is_empty() {
            return Ok(None);
        }
        let delayed_effects = parse_effect_chain(delayed_effect_clause.tokens())?;
        if delayed_effects.is_empty() {
            return Ok(None);
        }
        let delayed_refs_it =
            delayed_grammar::delayed_referential_sacrifice_shape(delayed_effect_clause.tokens());
        let Some(mut unless_effect) = try_build_unless(delayed_effects, timing_clause, unless_idx)?
        else {
            return Ok(None);
        };
        if delayed_refs_it {
            rewrite_unless_cost_source_values_to_it_tag(&mut unless_effect);
        }

        let mut effects = Vec::new();
        for segment in leading_segments {
            let parsed = parse_effect_chain(segment.tokens())?;
            if parsed.is_empty() {
                return Ok(None);
            }
            effects.extend(parsed);
        }
        effects.push(EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![unless_effect],
        });
        return Ok(Some(effects));
    }
    let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(final_segment[0])
    else {
        return Ok(None);
    };

    let Some(delayed_effect_clause) = final_segment[0]
        .before_word(timing_start_word)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if delayed_effect_clause.is_empty() {
        return Ok(None);
    }

    let delayed_effects = parse_effect_chain(delayed_effect_clause.tokens())?;
    if delayed_effects.is_empty() {
        return Ok(None);
    }

    let Some(timing_clause) = final_segment[0]
        .from_word(timing_start_word)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let Some(unless_idx) = timing_clause.find_token_word("unless") else {
        return Ok(None);
    };
    let Some(unless_effect) = try_build_unless(delayed_effects, timing_clause, unless_idx)? else {
        return Ok(None);
    };

    let mut effects = Vec::new();
    for segment in leading_segments {
        let parsed = parse_effect_chain(segment.tokens())?;
        if parsed.is_empty() {
            return Ok(None);
        }
        effects.extend(parsed);
    }
    effects.push(wrap_delayed_next_step_unless_pays(
        step,
        player,
        vec![unless_effect],
    ));
    Ok(Some(effects))
}

pub fn parse_sentence_delayed_next_upkeep_unless_pays_lose_game(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_period_segments();
    if segments.len() < 2 {
        return Ok(None);
    }

    let split = segments.len() - 2;
    let (leading_segments, delayed_segments) = segments.split_at(split);
    let [upkeep_clause, lose_clause] = delayed_segments else {
        unreachable!("the final two delayed-pact segments were selected above")
    };
    let mut effects = Vec::new();
    for segment in leading_segments {
        let leading_effects = parse_effect_chain(segment.tokens())?;
        if leading_effects.is_empty() {
            return Ok(None);
        }
        effects.extend(leading_effects);
    }
    let Some(payment_shape) =
        delayed_grammar::parse_delayed_upkeep_payment_shape(upkeep_clause.tokens())
    else {
        return Ok(None);
    };

    let mana = {
        use super::super::super::grammar::primitives as grammar;
        use super::super::super::lexer::LexStream;
        use winnow::prelude::*;

        let mut stream = LexStream::new(payment_shape.mana_tokens);
        grammar::collect_mana_symbols
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing mana payment in delayed next-upkeep clause (clause: '{}')",
                    upkeep_clause.text()
                ))
            })?
    };

    if !delayed_lose_game_unless_paid_matches(*lose_clause) {
        return Ok(None);
    }

    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: ironsmith_core::TotalCost::mana(crate::mana::ManaCost::from_symbols(mana)),
            before_delayed_step: false,
        }],
    });
    Ok(Some(effects))
}

fn normalize_unless_payment_clause_tokens(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<SubjectVerbPrimitiveOwnedClause> {
    let payment_clause = clause
        .split_once_on_word_trimmed("before")
        .map(|(payment_clause, _)| payment_clause.trimmed())
        .unwrap_or_else(|| clause.trimmed());
    let mut payment_clause =
        SubjectVerbPrimitiveOwnedClause::from_comma_trimmed_clause(payment_clause);
    let first = payment_clause.first_word()?;
    let normalized_first = match first {
        "pay" | "pays" => "pay",
        "sacrifice" | "sacrifices" => "sacrifice",
        "put" | "puts" => "put",
        _ => return None,
    };

    if first != normalized_first {
        payment_clause.replace_leading_word(normalized_first);
    }

    Some(payment_clause)
}

fn parse_unless_put_counters_clause_as_cost(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let payment_clause = clause
        .split_once_on_word_trimmed("before")
        .map(|(payment_clause, _)| payment_clause.trimmed())
        .unwrap_or_else(|| clause.trimmed());
    let Ok(effects) = parse_effect_chain(payment_clause.tokens()) else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutCounters {
                    counter_type,
                    count: Value::Fixed(count),
                    target,
                    target_count,
                    distributed: false,
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        return Ok(None);
    };
    if *count <= 0 {
        return Ok(None);
    }
    let TargetAst::Object(filter, _, _) = target else {
        return Ok(None);
    };
    let _ = (filter, target_count, counter_type, count);
    Ok(Some(ironsmith_core::TotalCost::from_cost(
        crate::model::CompilerCost::ValidatedEffect(Box::new(effects[0].clone())),
    )))
}

fn parse_unless_payment_clause_as_cost(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    if let Some(cost) = parse_unless_put_counters_clause_as_cost(clause)? {
        return Ok(Some(cost));
    }
    let Some(payment_tokens) = normalize_unless_payment_clause_tokens(clause) else {
        return Ok(None);
    };
    let referential_sacrifice =
        delayed_grammar::delayed_referential_sacrifice_shape(payment_tokens.tokens());
    let Some(mut cost) = crate::activation_and_restrictions::parse_payment_clause_as_total_cost(
        payment_tokens.tokens(),
    )?
    else {
        return Ok(None);
    };
    if referential_sacrifice {
        rewrite_cost_source_values_to_it_tag(&mut cost);
    }
    Ok(Some(cost))
}

fn parse_unless_sacrifice_clause_as_cost(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let words = clause.word_refs();
    if !matches!(words.first().copied(), Some("sacrifice" | "sacrifices")) {
        return Ok(None);
    }
    let effect = super::super::zone_handlers::parse_sacrifice(clause.tokens(), None, None)?;
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Sacrifice {
            filter, count: 1, ..
        },
        ..
    }) = effect
    else {
        return Ok(None);
    };
    Ok(Some(ironsmith_core::TotalCost::from_cost(
        crate::model::CompilerCost::Sacrifice {
            count: crate::effect::ChoiceCount::exactly(1),
            filter,
            all: false,
            binding: None,
        },
    )))
}

fn parse_unless_sacrifice_or_pay_cost(
    after_clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<
    Option<(
        PlayerAst,
        ironsmith_core::TotalCost<crate::model::CompilerCost>,
    )>,
    CardTextError,
> {
    let after_words = after_clause.words().to_word_refs();
    let Some((player, action_word_start)) = parse_delayed_player_prefix(&after_words) else {
        return Ok(None);
    };
    let Some(action_clause) = after_clause.after_words(action_word_start) else {
        return Ok(None);
    };
    let action_clause = action_clause.trimmed();
    let Some(or_idx) =
        crate::activation_and_restrictions::find_payment_alternative_or(action_clause.tokens())
    else {
        return Ok(None);
    };
    let left_clause = SubjectVerbPrimitiveClause::new(&action_clause.tokens()[..or_idx]).trimmed();
    let right_clause =
        SubjectVerbPrimitiveClause::new(&action_clause.tokens()[or_idx + 1..]).trimmed();
    if !delayed_clause_starts_with_action(
        left_clause,
        delayed_grammar::DelayedActionShape::Sacrifice,
    ) || !delayed_clause_starts_with_action(
        right_clause,
        delayed_grammar::DelayedActionShape::Pay,
    ) {
        return Ok(None);
    }
    let Some(sacrifice_cost) = parse_unless_sacrifice_clause_as_cost(left_clause)? else {
        return Ok(None);
    };
    let Some(payment_cost) = parse_unless_payment_clause_as_cost(right_clause)? else {
        return Ok(None);
    };
    Ok(Some((
        player,
        ironsmith_core::TotalCost::one_of(vec![sacrifice_cost, payment_cost]),
    )))
}

/// Return whether an `or` inside an `unless` tail joins two ways for the same
/// player to pay the cost. The outer action-choice splitter runs before the
/// ordinary unless primitive, so it must leave fully typed alternative costs
/// such as `sacrifices ... or pays 3 life` and `sacrifices ... or discards a
/// card` under the ownership of `UnlessPays`.
pub fn has_unless_payment_choice(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    let Some(unless_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("unless"))
    else {
        return Ok(false);
    };
    let after_clause = SubjectVerbPrimitiveClause::new(&tokens[unless_idx + 1..]).trimmed();
    let words = after_clause.words().to_word_refs();
    let Some((_, action_word_start)) = parse_delayed_player_prefix(&words) else {
        return Ok(false);
    };
    let Some(action_clause) = after_clause.after_words(action_word_start) else {
        return Ok(false);
    };
    let Some(cost) = parse_unless_payment_clause_as_cost(action_clause.trimmed())? else {
        return Ok(false);
    };
    Ok(matches!(
        cost.kind(),
        ironsmith_core::TotalCostKind::OneOf(branches) if branches.len() >= 2
    ))
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
pub fn try_build_unless(
    effects: Vec<EffectAst>,
    clause: SubjectVerbPrimitiveClause<'_>,
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_clause = clause.from(unless_idx + 1).trimmed();
    let after_words = after_clause.words().to_word_refs();
    let before_delayed_step = crate::word_primitives::parse_sequence_start(
        &after_words,
        &["before"],
    )
    .is_some_and(|before| {
        crate::slice_primitives::contains_any(&after_words[before + 1..], &["step", "upkeep"])
    });
    let payment_shape = delayed_grammar::split_delayed_payment_action_shape(after_clause.tokens());

    if let Some((player, cost)) = parse_unless_sacrifice_or_pay_cost(after_clause)? {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
            before_delayed_step,
        }));
    }

    // Determine the player from the "unless" clause
    let Some((player, action_word_start)) = (if let Some(shape) = payment_shape {
        let player_words = LexedClause::new(shape.player_tokens).word_refs();
        parse_delayed_player_before_pay(&player_words).map(|(player, _)| (player, 0))
    } else {
        parse_delayed_player_prefix(&after_words)
    }) else {
        return Ok(None);
    };

    let action_clause = if let Some(shape) = payment_shape {
        Some(SubjectVerbPrimitiveClause::new(shape.action_tokens))
    } else {
        after_clause.after_words(action_word_start)
    }
    .unwrap_or_else(|| after_clause.from(0))
    .trimmed();
    let action_word_storage = action_clause.words();
    let action_words = action_word_storage.to_word_refs();

    if let Some(alternative) = parse_causative_source_damage_to_player(action_clause, player) {
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative: vec![alternative],
            player,
        }));
    }

    if delayed_clause_starts_with_action(action_clause, delayed_grammar::DelayedActionShape::Pay) {
        if delayed_clause_mentions_mana_cost(action_clause) {
            return Err(CardTextError::ParseError(format!(
                "unsupported unless-payment mana-cost clause (clause: '{}')",
                clause.text()
            )));
        }
    } else if delayed_clause_starts_with_action(
        action_clause,
        delayed_grammar::DelayedActionShape::Draw,
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported non-cost unless action (clause: '{}')",
            clause.text()
        )));
    }

    if matches!(
        action_words.first().copied(),
        Some("sacrifice" | "sacrifices")
    ) && let Some(cost) = parse_unless_payment_clause_as_cost(action_clause)?
    {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
            before_delayed_step,
        }));
    }

    if matches!(
        action_words.first().copied(),
        Some("sacrifice" | "sacrifices")
    ) && let Ok(mut alternative) = super::super::zone_handlers::parse_sacrifice(
        action_clause.tokens(),
        Some(SubjectAst::Player(player)),
        None,
    )
    .map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    if let Some(cost) = parse_unless_payment_clause_as_cost(action_clause)? {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
            before_delayed_step,
        }));
    }

    // Prefer the action-only slice for explicit-player clauses like
    // "unless that player discards ... or sacrifices ...". Parsing the full
    // clause first can flatten the trailing "or" branch into the first action.
    if let Ok(mut alternative) = parse_effect_chain(action_clause.tokens())
        && !alternative.is_empty()
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    // Fall back to the full clause when the action-only parse needs the
    // explicit player prefix to succeed.
    if let Ok(mut alternative) = parse_effect_chain(after_clause.tokens())
        && !alternative.is_empty()
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(after_clause.tokens())
        && !alternative.is_empty()
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(action_clause.tokens())
        && !alternative.is_empty()
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    if let Ok(mut alternative) =
        parse_effect_clause(action_clause.tokens()).map(|effect| vec![effect])
        && !alternative.is_empty()
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    if delayed_clause_starts_with_action(
        action_clause,
        delayed_grammar::DelayedActionShape::Discard,
    ) && let Ok(mut alternative) =
        super::super::zone_handlers::parse_discard(action_clause.tokens(), None)
            .map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    Ok(None)
}

pub fn parse_sentence_fallback_mechanic_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if delayed_clause_mentions_cast_or_play_action(clause)
        && clause
            .parse_value_with_lexed(parse_cast_or_play_tagged_clause)?
            .is_some()
    {
        return Ok(None);
    }

    if delayed_grammar::delayed_exact_shape(
        clause.tokens(),
        DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_WORDS,
    ) {
        return Ok(None);
    }
    if delayed_grammar::delayed_exact_shape(clause.tokens(), DELAYED_VENTURE_DUNGEON_WORDS) {
        return Ok(Some(vec![EffectAst::subject_verb_venture_into_dungeon(
            crate::cards::builders::PlayerAst::You,
            false,
        )]));
    }

    let is_match = DELAYED_STILL_LAND_WORDS
        .iter()
        .any(|phrase| delayed_grammar::delayed_exact_shape(clause.tokens(), phrase))
        || delayed_clause_starts_with_mechanic_marker(clause, &MECHANIC_MARKER_PREFIXES[..3])
        || delayed_grammar::is_known_fallback_marker_shape(clause.tokens())
        || (delayed_clause_starts_with_mechanic_marker(clause, &MECHANIC_MARKER_PREFIXES[3..])
            && delayed_clause_mentions_remains_tapped(clause));
    if !is_match {
        return Ok(None);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported mechanic marker clause (clause: '{}')",
        clause.text()
    )))
}

pub fn parse_sentence_implicit_become_clause(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(subject_shape) = delayed_grammar::parse_implicit_become_subject_shape(clause.tokens())
    else {
        return Ok(None);
    };
    let target = match subject_shape.kind {
        delayed_grammar::ImplicitBecomeSubjectKind::Source => TargetAst::Source(None),
        delayed_grammar::ImplicitBecomeSubjectKind::Tagged => {
            TargetAst::Tagged(TagKey::from(IT_TAG), None)
        }
    };
    let rest_clause = SubjectVerbPrimitiveClause::new(subject_shape.remainder_tokens).trimmed();
    let (mut duration, duration_remainder_clause) =
        if let Some((duration, remainder)) = parse_restriction_duration(rest_clause.tokens())? {
            (duration, SubjectVerbPrimitiveOwnedClause::new(remainder))
        } else {
            (
                Until::Forever,
                SubjectVerbPrimitiveOwnedClause::from_clause(rest_clause),
            )
        };
    let mut rest_words = duration_remainder_clause.as_clause().trimmed_word_refs();
    let prefix_shape = delayed_grammar::parse_implicit_become_prefix_words(&rest_words);
    rest_words.drain(..prefix_shape.consumed);
    if rest_words.is_empty() {
        return Ok(None);
    }
    let negated = prefix_shape.negated;
    if let Some(suffix_len) = delayed_grammar::delayed_until_eot_suffix_len(&rest_words) {
        duration = Until::EndOfTurn;
        let new_len = rest_words.len().saturating_sub(suffix_len);
        rest_words.truncate(new_len);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negative_type_words =
        delayed_grammar::delayed_negative_type_prefix_len(&rest_words, negated)
            .filter(|prefix_len| rest_words.len() > *prefix_len || negated)
            .map(|prefix_len| &rest_words[prefix_len..]);
    if let Some(type_words) = negative_type_words {
        let mut card_types = Vec::new();
        let mut all_card_types = true;
        for word in type_words {
            if let Some(card_type) = parse_card_type(word) {
                if !iter_contains(card_types.iter(), &card_type) {
                    card_types.push(card_type);
                }
            } else {
                all_card_types = false;
                break;
            }
        }
        if all_card_types && !card_types.is_empty() {
            return Ok(Some(vec![EffectAst::subject_verb_remove_card_types(
                target, card_types, duration,
            )]));
        }

        let mut subtypes = Vec::new();
        let mut all_subtypes = true;
        for word in type_words {
            if let Some(subtype) = parse_subtype_flexible(word) {
                if !iter_contains(subtypes.iter(), &subtype) {
                    subtypes.push(subtype);
                }
            } else {
                all_subtypes = false;
                break;
            }
        }
        if all_subtypes && !subtypes.is_empty() {
            return Ok(Some(vec![EffectAst::subject_verb_remove_subtypes(
                target, subtypes, duration,
            )]));
        }
    }

    let addition_tail_len = delayed_grammar::delayed_addition_other_types_suffix_len(&rest_words);

    let body_words = if rest_words
        .first()
        .copied()
        .is_some_and(delayed_grammar::delayed_article_shape)
    {
        &rest_words[1..]
    } else {
        &rest_words[..]
    };
    if body_words.is_empty() {
        return Ok(None);
    }

    if let Ok((power, toughness)) = parse_pt_modifier_values(body_words[0])
        && body_words.len() > 1
    {
        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        let mut parsed_all_descriptor_words = true;
        let mut saw_subtype = false;
        for word in &body_words[1..] {
            if matches!(*word, "and" | "or") {
                continue;
            }
            if let Some(card_type) = parse_card_type(word) {
                if !iter_contains(card_types.iter(), &card_type) {
                    card_types.push(card_type);
                }
            } else if let Some(subtype) = parse_pluralized_subtype_word(word) {
                if !iter_contains(subtypes.iter(), &subtype) {
                    subtypes.push(subtype);
                }
                saw_subtype = true;
            } else {
                parsed_all_descriptor_words = false;
                break;
            }
        }
        if parsed_all_descriptor_words && (!card_types.is_empty() || saw_subtype) {
            if saw_subtype && !iter_contains(card_types.iter(), &CardType::Creature) {
                card_types.insert(0, CardType::Creature);
            }
            return Ok(Some(vec![
                EffectAst::subject_verb_become_base_pt_creature(
                    power,
                    toughness,
                    target,
                    card_types,
                    subtypes,
                    Vec::new(),
                    None,
                    Vec::new(),
                    Vec::new(),
                    false,
                    None,
                    Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness),
                    None,
                    duration,
                )
                .with_set_quantifier_surface(subject_shape.set_quantifier_surface),
            ]));
        }
    }

    if let Ok((power, toughness)) = parse_pt_modifier_values(body_words[0])
        && let Some(tail_len) = addition_tail_len
        && body_words.len() > 1 + tail_len
    {
        let subtype_words = &body_words[1..body_words.len().saturating_sub(tail_len)];
        let mut subtypes = Vec::new();
        for word in subtype_words {
            let Some(subtype) = parse_pluralized_subtype_word(word) else {
                return Ok(None);
            };
            if !iter_contains(subtypes.iter(), &subtype) {
                subtypes.push(subtype);
            }
        }
        if subtypes.is_empty() {
            return Ok(None);
        }
        return Ok(Some(vec![
            EffectAst::subject_verb_set_base_power_toughness(
                power,
                toughness,
                target.clone(),
                duration.clone(),
            ),
            EffectAst::subject_verb_add_subtypes(target, subtypes, duration),
        ]));
    }

    let type_words = if let Some(tail_len) = addition_tail_len {
        &body_words[..body_words.len().saturating_sub(tail_len)]
    } else {
        body_words
    };
    if type_words.is_empty() {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut all_card_types = true;
    for word in type_words {
        if let Some(card_type) = parse_card_type(word) {
            if !iter_contains(card_types.iter(), &card_type) {
                card_types.push(card_type);
            }
        } else {
            all_card_types = false;
            break;
        }
    }
    if all_card_types && !card_types.is_empty() {
        let effect = if addition_tail_len.is_some() {
            EffectAst::subject_verb_add_card_types(target, card_types, duration)
        } else {
            EffectAst::subject_verb_set_card_types(target, card_types, duration)
        };
        return Ok(Some(vec![effect]));
    }

    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        if !iter_contains(subtypes.iter(), &subtype) {
            subtypes.push(subtype);
        }
    }
    if subtypes.is_empty() {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::subject_verb_add_subtypes(
        target, subtypes, duration,
    )]))
}

pub fn parse_sentence_gains_or_loses_all_creature_types(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = delayed_grammar::parse_delayed_creature_types_shape(clause.tokens()) else {
        return Ok(None);
    };
    if !shape.gain
        && let Some(pump) = delayed_grammar::parse_delayed_losing_pump_shape(shape.subject_tokens)
    {
        let Ok((power, toughness)) = parse_pt_modifier_values(pump.modifier) else {
            return Ok(None);
        };
        let target = parse_target_phrase(pump.target_tokens)?;
        return Ok(Some(vec![
            EffectAst::subject_verb_pump(power, toughness, target.clone(), Until::EndOfTurn, None),
            EffectAst::subject_verb_remove_all_subtypes_of_family(
                target,
                crate::types::SubtypeFamily::Creature,
                Until::EndOfTurn,
            ),
        ]));
    }

    let target = if delayed_grammar::delayed_tagged_creature_reference_shape(shape.subject_tokens) {
        TargetAst::Tagged(TagKey::from(IT_TAG), None)
    } else {
        parse_target_phrase(shape.subject_tokens)?
    };
    let effect = if shape.gain {
        EffectAst::subject_verb_add_all_subtypes_of_family(
            target,
            crate::types::SubtypeFamily::Creature,
            Until::EndOfTurn,
        )
    } else {
        EffectAst::subject_verb_remove_all_subtypes_of_family(
            target,
            crate::types::SubtypeFamily::Creature,
            Until::EndOfTurn,
        )
    };
    Ok(Some(vec![effect]))
}

pub fn parse_sentence_lose_draw_clash_repeat_process(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = delayed_grammar::parse_lose_draw_clash_shape(clause.tokens()) else {
        return Ok(None);
    };

    let effects = vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife {
                amount: Value::Fixed(shape.life_count),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(shape.draw_count),
            },
        ),
        EffectAst::subject_verb_clash(ClashOpponentAst::Opponent),
    ];
    if !shape.repeat_if_win {
        return Ok(Some(effects));
    }

    Ok(Some(vec![EffectAst::RepeatProcess {
        effects,
        continue_effect_index: 2,
        continue_predicate: IfResultPredicate::WonClash,
    }]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn effect_sentence_routes_action_first_draw_step_unless_before_broad_verb_parsing() {
        let tokens = lex_line(
            "That player loses 1 life at the beginning of their next draw step unless they pay {1} before that draw step.",
            0,
        )
        .expect("delayed draw-step life loss should lex");
        let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
            .expect("complete delayed sentence should parse");

        assert!(matches!(
            effects.as_slice(),
            [EffectAst::DelayedUntilNextDrawStep {
                player: PlayerAst::That,
                effects,
            }] if matches!(
                effects.as_slice(),
                [EffectAst::UnlessPays {
                    player: PlayerAst::That,
                    effects,
                    ..
                }] if matches!(
                    effects.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::LoseLife {
                            amount: Value::Fixed(1),
                        },
                        ..
                    })]
                )
            )
        ));
    }

    #[test]
    fn referential_unless_sacrifice_cost_keeps_the_it_antecedent() {
        let tokens = lex_line("sacrifices it.", 0).expect("sacrifice cost should lex");
        let cost = parse_unless_payment_clause_as_cost(SubjectVerbPrimitiveClause::new(&tokens))
            .expect("sacrifice cost should parse")
            .expect("sacrifice should be a total cost");
        let [component] = cost.costs() else {
            panic!("expected one sacrifice component: {cost:#?}");
        };
        match component {
            crate::model::CompilerCost::Sacrifice { filter, .. } => {
                assert!(
                    !filter.source
                        && filter.tagged_constraints.iter().any(|constraint| {
                            constraint.relation
                                == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                                && constraint.tag.as_str() == IT_TAG
                        }),
                    "the pronoun must survive until lowering can bind it: {filter:#?}"
                );
            }
            other => panic!("expected a referential sacrifice cost: {other:#?}"),
        }
    }

    #[test]
    fn end_of_combat_suffix_wraps_generic_actions_in_delayed_effects() {
        for text in [
            "Remove a +1/+1 counter from it at end of combat.",
            "Put a -1/-1 counter on it at end of combat.",
            "Put it on top of its owner's library at end of combat.",
        ] {
            let tokens = lex_line(text, 0).expect("end-of-combat action should lex");
            let effects =
                parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(&tokens))
                    .expect("end-of-combat action should parse")
                    .expect("end-of-combat suffix should match");
            let [EffectAst::DelayedUntilEndOfCombat { effects: delayed }] = effects.as_slice()
            else {
                panic!("expected only a delayed end-of-combat action for {text}: {effects:#?}");
            };
            assert_eq!(
                delayed.len(),
                1,
                "expected one delayed payload for {text}: {delayed:#?}"
            );
        }
    }

    #[test]
    fn cleanup_step_suffix_wraps_sacrifice_in_delayed_effect() {
        let tokens = lex_line(
            "Sacrifice this Aura at the beginning of the next cleanup step.",
            0,
        )
        .expect("cleanup-step action should lex");
        let effects =
            parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("cleanup-step action should parse")
                .expect("cleanup-step suffix should match");
        let [
            EffectAst::DelayedUntilNextCleanupStep {
                player: PlayerFilter::Any,
                effects: delayed,
            },
        ] = effects.as_slice()
        else {
            panic!("expected one delayed cleanup-step action: {effects:#?}");
        };
        assert_eq!(delayed.len(), 1);
    }

    #[test]
    fn delayed_suffix_keeps_result_gate_outside_scheduled_action() {
        let tokens = lex_line(
            "If you do, unattach it at the beginning of the next end step.",
            0,
        )
        .expect("conditional delayed action should lex");

        let effects =
            parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("conditional delayed action should parse")
                .expect("delayed timing suffix should match");
        let [
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: gated,
            },
        ] = effects.as_slice()
        else {
            panic!("expected the result gate to remain outermost: {effects:#?}");
        };
        let [
            EffectAst::DelayedUntilNextEndStep {
                effects: delayed, ..
            },
        ] = gated.as_slice()
        else {
            panic!("expected the gated action to be delayed: {gated:#?}");
        };
        assert!(
            matches!(
                delayed.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Unattach { .. },
                    ..
                })]
            ),
            "expected only the unattach action inside the delayed queue: {delayed:#?}"
        );
    }

    #[test]
    fn delayed_suffix_schedules_only_the_final_coordinated_action() {
        let tokens = lex_line(
            "You gain 2 life, and you return this card from your graveyard to your hand at the beginning of the next end step.",
            0,
        )
        .expect("coordinated delayed action should lex");

        let effects =
            parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("coordinated delayed action should parse")
                .expect("delayed timing suffix should match");
        let [
            immediate,
            EffectAst::DelayedUntilNextEndStep {
                effects: delayed, ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected an immediate action followed by one delayed action: {effects:#?}");
        };
        assert!(
            !matches!(immediate, EffectAst::DelayedUntilNextEndStep { .. }),
            "the leading life gain must resolve immediately: {immediate:#?}"
        );
        assert_eq!(delayed.len(), 1, "only the return should be delayed");
    }

    #[test]
    fn delayed_suffix_keeps_a_leading_game_condition_outside_the_schedule() {
        let tokens = lex_line(
            "If the gift wasn't promised, return that card to the battlefield under its owner's control with a +1/+1 counter on it at the beginning of the next end step.",
            0,
        )
        .expect("conditional delayed action should lex");

        let effects =
            parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("conditional delayed action should parse")
                .expect("delayed timing suffix should match");
        let [
            EffectAst::Conditional {
                if_true, if_false, ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected the gift condition to remain outermost: {effects:#?}");
        };
        assert!(if_false.is_empty(), "unexpected gift-condition else branch");
        assert!(
            matches!(
                if_true.as_slice(),
                [EffectAst::DelayedUntilNextEndStep { .. }]
            ),
            "only the conditioned return should be scheduled: {if_true:#?}"
        );
    }

    #[test]
    fn try_build_unless_prefers_action_only_parse_for_explicit_player_or_choice() {
        let tokens = lex_line(
            "Target opponent loses 5 life unless that player discards two cards or sacrifices a creature or planeswalker of their choice.",
            0,
        )
        .expect("rewrite lexer should classify explicit-player unless choice");
        let clause = SubjectVerbPrimitiveClause::new(&tokens);
        let unless_idx = clause.find_token_word("unless").expect("unless token");
        let effects = parse_effect_chain(&tokens[..unless_idx])
            .expect("lead effect should parse before unless clause");

        let unless_effect = try_build_unless(effects, clause, unless_idx)
            .expect("unless choice should parse")
            .expect("unless choice should lower");
        let debug = format!("{unless_effect:?}");

        assert!(
            debug.contains("Discard"),
            "expected explicit-player unless choice to keep the discard branch, got {debug}"
        );
        assert!(
            debug.contains("Sacrifice"),
            "expected explicit-player unless choice to keep the sacrifice branch, got {debug}"
        );
        assert!(
            debug.contains("TargetOpponent"),
            "expected explicit-player unless choice to bind the target opponent context, got {debug}"
        );
    }

    #[test]
    fn try_build_unless_parses_sacrifice_or_pay_as_one_payment_choice() {
        let tokens = lex_line(
            "Draw a card unless target opponent sacrifices a creature of their choice or pays 3 life.",
            0,
        )
        .expect("unless sacrifice-or-pay text should lex");
        let clause = SubjectVerbPrimitiveClause::new(&tokens);
        let unless_idx = clause.find_token_word("unless").expect("unless token");
        let effects = parse_effect_chain(&tokens[..unless_idx])
            .expect("lead effect should parse before unless clause");

        let unless_effect = try_build_unless(effects, clause, unless_idx)
            .expect("unless sacrifice-or-pay should parse")
            .expect("unless sacrifice-or-pay should lower");
        let debug = format!("{unless_effect:?}");

        assert!(debug.contains("UnlessPays"), "{debug}");
        assert!(debug.contains("TargetOpponent"), "{debug}");
        assert!(debug.contains("OneOf"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Life"), "{debug}");
    }

    #[test]
    fn effect_chain_keeps_target_opponent_sacrifice_or_life_as_one_payment_choice() {
        let tokens = lex_line(
            "Draw a card unless target opponent sacrifices a creature of their choice or pays 3 life.",
            0,
        )
        .expect("unless sacrifice-or-pay text should lex");

        let effects = parse_effect_chain(&tokens).expect("full effect chain should parse");
        let [
            EffectAst::UnlessPays {
                player: PlayerAst::TargetOpponent,
                cost,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one target-opponent UnlessPays branch, got {effects:#?}");
        };
        assert_eq!(cost.as_one_of().map(<[_]>::len), Some(2), "{cost:#?}");
    }

    #[test]
    fn effect_chain_keeps_sacrifice_or_discard_inside_the_unless_payment() {
        let tokens = lex_line(
            "You lose 3 life unless you sacrifice a nonland permanent of your choice or discard a card.",
            0,
        )
        .expect("unless sacrifice-or-discard text should lex");

        assert!(
            has_unless_payment_choice(&tokens)
                .expect("typed alternative payment detection should succeed")
        );
        let effects = parse_effect_chain(&tokens).expect("full effect chain should parse");
        let [
            EffectAst::UnlessPays {
                player: PlayerAst::You,
                cost,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one UnlessPays branch, got {effects:#?}");
        };
        let Some(branches) = cost.as_one_of() else {
            panic!("expected an alternative total cost, got {cost:#?}");
        };
        assert_eq!(branches.len(), 2, "{cost:#?}");
        assert!(format!("{:#?}", branches[0]).contains("Sacrifice"));
        assert!(format!("{:#?}", branches[1]).contains("Discard"));
    }
}
