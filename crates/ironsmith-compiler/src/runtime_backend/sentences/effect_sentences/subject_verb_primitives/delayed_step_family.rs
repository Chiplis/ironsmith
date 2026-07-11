use super::*;
use crate::runtime_backend::front_end::grammar::effects::delayed_step_shapes as delayed_grammar;

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
    }
}

pub(crate) fn find_unquoted_token_word(
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

fn rewrite_cost_source_values_to_it_tag(cost: &mut crate::cost::TotalCost) {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(_) => {
            let mut components = cost.costs().to_vec();
            for component in &mut components {
                match component {
                    crate::costs::Cost::DynamicMana(dynamic) => {
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
                    crate::costs::Cost::Energy(value)
                    | crate::costs::Cost::Mill(value)
                    | crate::costs::Cost::Life(value) => rewrite_value_source_to_it_tag(value),
                    _ => {}
                }
            }
            *cost = crate::cost::TotalCost::from_costs(components);
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut branches = branches.to_vec();
            for branch in &mut branches {
                rewrite_cost_source_values_to_it_tag(branch);
            }
            *cost = crate::cost::TotalCost::one_of(branches);
        }
    }
}

pub(crate) fn rewrite_unless_cost_source_values_to_it_tag(effect: &mut EffectAst) {
    if let EffectAst::UnlessPays { cost, .. } = effect {
        rewrite_cost_source_values_to_it_tag(cost);
    }
}

pub(crate) fn parse_sentence_delayed_next_step_unless_pays(
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

pub(crate) fn parse_sentence_delayed_next_upkeep_unless_pays_lose_game(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_period_segments();
    if segments.len() != 2 && segments.len() != 3 {
        return Ok(None);
    }

    let (mut effects, upkeep_clause, lose_clause) = if segments.len() == 3 {
        let first_effects = parse_effect_chain(segments[0].tokens())?;
        if first_effects.is_empty() {
            return Ok(None);
        }
        (first_effects, segments[1], segments[2])
    } else {
        (Vec::new(), segments[0], segments[1])
    };
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

    if !delayed_lose_game_unless_paid_matches(lose_clause) {
        return Ok(None);
    }

    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: crate::cost::TotalCost::mana(crate::mana::ManaCost::from_symbols(mana)),
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
        _ => return None,
    };

    if first != normalized_first {
        payment_clause.replace_leading_word(normalized_first);
    }

    Some(payment_clause)
}

fn parse_unless_payment_clause_as_cost(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<crate::cost::TotalCost>, CardTextError> {
    let Some(payment_tokens) = normalize_unless_payment_clause_tokens(clause) else {
        return Ok(None);
    };
    crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(
        payment_tokens.tokens(),
    )
}

fn parse_unless_sacrifice_clause_as_cost(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<crate::cost::TotalCost>, CardTextError> {
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
    Ok(Some(crate::cost::TotalCost::from_cost(
        crate::costs::Cost::sacrifice(filter),
    )))
}

fn parse_unless_sacrifice_or_pay_cost(
    after_clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(PlayerAst, crate::cost::TotalCost)>, CardTextError> {
    let after_words = after_clause.words().to_word_refs();
    let Some((player, action_word_start)) = parse_delayed_player_prefix(&after_words) else {
        return Ok(None);
    };
    let Some(action_clause) = after_clause.after_words(action_word_start) else {
        return Ok(None);
    };
    let action_clause = action_clause.trimmed();
    let Some(or_idx) =
        crate::runtime_backend::families::activation_and_restrictions::find_payment_alternative_or(
            action_clause.tokens(),
        )
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
        crate::cost::TotalCost::one_of(vec![sacrifice_cost, payment_cost]),
    )))
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
pub(crate) fn try_build_unless(
    effects: Vec<EffectAst>,
    clause: SubjectVerbPrimitiveClause<'_>,
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_clause = clause.from(unless_idx + 1).trimmed();
    let after_words = after_clause.words().to_word_refs();
    let payment_shape = delayed_grammar::split_delayed_payment_action_shape(after_clause.tokens());

    if let Some((player, cost)) = parse_unless_sacrifice_or_pay_cost(after_clause)? {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
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
        }));
    }

    // Prefer the action-only slice for explicit-player clauses like
    // "unless that player discards ... or sacrifices ...". Parsing the full
    // clause first can flatten the trailing "or" branch into the first action.
    if let Ok(mut alternative) = parse_effect_chain(action_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    // Fall back to the full clause when the action-only parse needs the
    // explicit player prefix to succeed.
    if let Ok(mut alternative) = parse_effect_chain(after_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(after_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(action_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) =
        parse_effect_clause(action_clause.tokens()).map(|effect| vec![effect])
    {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

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
}

pub(crate) fn parse_sentence_fallback_mechanic_marker(
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

pub(crate) fn parse_sentence_implicit_become_clause(
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
            return Ok(Some(vec![EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                card_types,
                subtypes,
                Vec::new(),
                None,
                Vec::new(),
                Vec::new(),
                duration,
            )]));
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
        return Ok(Some(vec![EffectAst::subject_verb_add_card_types(
            target, card_types, duration,
        )]));
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

pub(crate) fn parse_sentence_gains_or_loses_all_creature_types(
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

pub(crate) fn parse_sentence_lose_draw_clash_repeat_process(
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
        continue_predicate: IfResultPredicate::Value(crate::effect::Comparison::GreaterThan(0)),
    }]))
}
