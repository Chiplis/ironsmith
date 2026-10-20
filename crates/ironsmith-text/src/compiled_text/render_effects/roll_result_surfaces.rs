use super::*;

/// A retained roll result metric counts qualifying dice, not repetitions of
/// the token/counter instruction. Render each outcome as one quantified clause
/// only when its action's count names this exact roll and its exact metric.
pub(in crate::compiled_text) fn describe_roll_metric_count_actions_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.iter().any(|segment| {
        !segment.self_replacements.is_empty() || segment.starts_new_source_line
    }) {
        return None;
    }
    let effects = program.flattened_default_effects();
    let rolls = effects.iter().enumerate().filter_map(|(index, effect)| {
        let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        let roll = with_id.effect.downcast_ref::<crate::effects::RollDiceChooseResultEffect>()?;
        Some((index, with_id.id, roll))
    }).collect::<Vec<_>>();
    let [(roll_index, roll_id, roll)] = rolls.as_slice() else {
        return None;
    };
    if roll.player != PlayerFilter::You
        || !roll.total
        || roll.ignore_lower
        || roll.in_order
        || roll.sides < 2
    {
        return None;
    }

    fn counted_result(value: &Value, roll_id: crate::effect::EffectId)
        -> Option<(&'static str, i32)>
    {
        let (reference, each_count) = match value.unhinted() {
            Value::Scaled(reference, count) if *count > 0 => (reference.as_ref(), *count),
            reference => (reference, 1),
        };
        let Value::PriorEffectMetric { effect_id, query } = reference.unhinted() else {
            return None;
        };
        let label = match query.metric {
            crate::effect::EffectMetric::DistinctDieResults => "different",
            crate::effect::EffectMetric::OddDieResults => "odd",
            crate::effect::EffectMetric::EvenDieResults => "even",
            _ => return None,
        };
        let expected = crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::Outcome,
            query.metric,
        ).with_action(crate::effect::PriorEffectAction::Rolled);
        (*effect_id == roll_id && *query == expected).then_some((label, each_count))
    }

    fn counted_action(effect: &Effect, roll_id: crate::effect::EffectId)
        -> Option<(String, String)>
    {
        let action = unwrap_basic_tag_wrappers(effect);
        if let Some(create) = action.downcast_ref::<crate::effects::CreateTokenEffect>() {
            let (label, each_count) = counted_result(&create.count, roll_id)?;
            let mut single = create.clone();
            single.count = Value::Fixed(each_count);
            return Some((label.to_string(), describe_effect(&Effect::new(single))));
        }
        if let Some(counters) = action.downcast_ref::<crate::effects::PutCountersEffect>() {
            let (label, each_count) = counted_result(&counters.amount, roll_id)?;
            let mut single = counters.clone();
            single.amount = Value::Fixed(each_count);
            return Some((label.to_string(), describe_effect(&Effect::new(single))));
        }
        None
    }

    let mut parts = Vec::new();
    if *roll_index > 0 {
        let prefix = describe_effect_list(&effects[..*roll_index]);
        if prefix.trim().is_empty() {
            return None;
        }
        parts.push(prefix.trim_end_matches('.').to_string());
    }
    parts.push(describe_effect(&effects[*roll_index]).trim_end_matches('.').to_string());
    let mut tail_index = *roll_index + 1;
    while let Some(effect) = effects.get(tail_index) {
        let Some((label, action)) = counted_action(effect, *roll_id) else { break };
        parts.push(format!(
            "For each {label} result, {}",
            lowercase_first(action.trim_end_matches('.')),
        ));
        tail_index += 1;
    }
    if tail_index == *roll_index + 1 {
        return None;
    }
    if tail_index < effects.len() {
        let tail = describe_effect_list(&effects[tail_index..]);
        if tail.trim().is_empty() {
            return None;
        }
        parts.push(tail.trim_end_matches('.').to_string());
    }
    Some(parts.join(". "))
}

/// An ordered two-die roll with one exhaustive, identical result table for
/// each die and an optional doubles reward is one printed procedure. The
/// branch pair check ignores only static-ability instance identities: each
/// roll installs a distinct instance of the same ability.
pub(in crate::compiled_text) fn describe_per_die_result_ability_table_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.iter().any(|segment| {
        !segment.self_replacements.is_empty() || segment.starts_new_source_line
    }) {
        return None;
    }
    let effects = program.flattened_default_effects();
    let (roll_effect, branches) = effects.split_first()?;
    let with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let roll = with_id
        .effect
        .downcast_ref::<crate::effects::RollDiceChooseResultEffect>()?;
    if roll.player != PlayerFilter::You
        || roll.count != 2
        || !(2..=20).contains(&roll.sides)
        || roll.die_text.is_some()
        || roll.ignore_lower
        || roll.total
        || roll.count_value.is_some()
        || !roll.in_order
        || branches.len() != usize::try_from(roll.sides).ok()?.checked_mul(2)?.checked_add(1)?
    {
        return None;
    }

    fn result_branch<'a>(
        effect: &'a Effect,
        roll_id: crate::effect::EffectId,
        metric: crate::effect::EffectMetric,
        result: i32,
    ) -> Option<&'a Effect> {
        let branch = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
        let Condition::ValueComparison {
            left: Value::EffectMetric { effect_id, source, metric: actual_metric },
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: Value::Fixed(actual_result),
        } = &branch.condition else { return None };
        if *effect_id != roll_id
            || *source != crate::effect::EffectMetricSource::Outcome
            || *actual_metric != metric
            || *actual_result != result
            || !branch.if_false.is_empty()
            || branch.surface != ironsmith_core::ConditionalSurface::LeadingIf
            || branch.capture_condition_result
        {
            return None;
        }
        let [reward] = branch.if_true.as_slice() else { return None };
        Some(reward)
    }

    fn equivalent_reward(first: &Effect, second: &Effect) -> bool {
        let Some(first) = first.downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
            return false;
        };
        let Some(second) = second.downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
            return false;
        };
        let mut first = first.clone();
        let mut second = second.clone();
        // The first die's authored "this creature" hint is presentation only;
        // the second die uses an unhinted source target. Require both to be
        // the same executable source before comparing their rewards.
        for reward in [&mut first, &mut second] {
            if reward.target_spec.as_ref().is_some_and(|spec| spec.unhinted() == &ChooseSpec::Source) {
                reward.target_spec = Some(ChooseSpec::Source);
            }
        }
        if first == second {
            return true;
        }
        let (Some(crate::continuous::Modification::AddAbility(first_ability)),
            Some(crate::continuous::Modification::AddAbility(second_ability))) =
            (&first.modification, &second.modification)
        else { return false };
        if first_ability.compiled_model().is_none()
            || first_ability.compiled_model() != second_ability.compiled_model()
        {
            return false;
        }
        first.modification = None;
        second.modification = None;
        first == second
    }

    let sides = usize::try_from(roll.sides).ok()?;
    let mut rows = Vec::with_capacity(sides);
    for index in 0..sides {
        let result = i32::try_from(index + 1).ok()?;
        let first = result_branch(&branches[index], with_id.id, crate::effect::EffectMetric::Count, result)?;
        let second = result_branch(&branches[index + sides], with_id.id, crate::effect::EffectMetric::OtherNumber, result)?;
        if !equivalent_reward(first, second) {
            return None;
        }
        let apply = first.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        if apply.target != crate::continuous::EffectTarget::Source
            || apply.until != crate::effect::Until::EndOfTurn
            || apply.condition.is_some()
            || !apply.additional_modifications.is_empty()
            || apply.source_type.is_some()
            || apply.set_quantifier_surface.is_some()
            || apply.type_retention_surface.is_some()
            || apply.animation_pt_surface.is_some()
            || apply.animation_duration_surface.is_some()
            || apply.lock_filter_at_resolution
            || apply.resolve_set_pt_values_at_resolution
            || apply.grant_only_if_absent
        {
            return None;
        }
        if index == 0 {
            if apply.modification.is_some()
                || !matches!(apply.runtime_modifications.as_slice(),
                    [crate::effects::continuous::RuntimeModification::ModifyPowerToughness { .. }])
            {
                return None;
            }
        } else if !apply.runtime_modifications.is_empty()
            || !matches!(&apply.modification, Some(crate::continuous::Modification::AddAbility(_)))
        {
            return None;
        }
        // Source-reference surface can render an individual reward as
        // lowercase "this creature ...". Sentence case is presentation only;
        // the structural checks above already established the same reward
        // for each die and the exact ordered result table.
        rows.push(capitalize_first(&describe_effect(first)));
    }
    let first_row = rows.first()?;
    if !first_row.starts_with("This creature gets ") {
        return None;
    }
    let mut ability_rows = Vec::with_capacity(sides - 1);
    for row in rows.iter().skip(1) {
        ability_rows.push(row.strip_prefix("This creature gains ")?
            .strip_suffix(" until end of turn")?);
    }

    let doubles = branches.last()?.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let die_count = Value::EffectMetric {
        effect_id: with_id.id,
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::Count,
    };
    let other_result = Value::EffectMetric {
        effect_id: with_id.id,
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::OtherNumber,
    };
    if doubles.condition != (Condition::ValueComparison {
        left: die_count,
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: other_result,
    })
        || !doubles.if_false.is_empty()
        || doubles.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || doubles.capture_condition_result
    {
        return None;
    }
    let [doubles_reward] = doubles.if_true.as_slice() else { return None };
    let doubles_apply = doubles_reward.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if doubles_apply.target != crate::continuous::EffectTarget::Source
        || doubles_apply.until != crate::effect::Until::EndOfTurn
        || doubles_apply.condition.is_some()
        || !doubles_apply.additional_modifications.is_empty()
        || !doubles_apply.runtime_modifications.is_empty()
        || !matches!(&doubles_apply.modification, Some(crate::continuous::Modification::AddAbility(_)))
    {
        return None;
    }
    let doubles_ability = capitalize_first(&describe_effect(doubles_reward))
        .strip_prefix("This creature gains ")?
        .strip_suffix(" until end of turn")?
        .to_string();
    let sides_word = small_number_word(roll.sides).unwrap_or_else(|| roll.sides.to_string());
    let mut rendered = format!(
        "Roll two {sides_word}-sided dice. For each result of 1, {}. For each other result, it gains the indicated ability until end of turn. If you rolled doubles, it also gains {doubles_ability} until end of turn",
        lowercase_first(first_row.trim_end_matches('.'))
    );
    for (index, ability) in ability_rows.iter().enumerate() {
        rendered.push_str(&format!("\n• {} — {ability}", index + 2));
    }
    Some(rendered)
}

fn complete_die_parity_label(
    predicate: &crate::effect::EffectPredicate,
    sides: u32,
) -> Option<&'static str> {
    if sides < 2 {
        return None;
    }
    let sides = i32::try_from(sides).ok()?;
    let crate::effect::EffectPredicate::Value(Comparison::OneOf(values)) = predicate else {
        return None;
    };

    let mut actual = values.to_vec();
    actual.sort_unstable();
    actual.dedup();
    let odds = (1..=sides)
        .filter(|value| value % 2 != 0)
        .collect::<Vec<_>>();
    let evens = (1..=sides)
        .filter(|value| value % 2 == 0)
        .collect::<Vec<_>>();
    if actual == odds {
        Some("odd")
    } else if actual == evens {
        Some("even")
    } else {
        None
    }
}

/// Render a repeated die roll followed by exhaustive odd/even result
/// branches. The runtime graph already records every roll as an execution
/// fact, and each `IfEffect` repeats its branch for every matching fact. This
/// renderer recovers the authored aggregate surface only when the graph proves
/// both complete parity partitions for the die.
pub(in crate::compiled_text) fn describe_repeated_die_parity_result_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let effects = program.flattened_default_effects();
    let repeated_rolls = effects
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| {
            let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
            let repeat = with_id
                .effect
                .downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
            let [roll_effect] = repeat.effects.as_slice() else {
                return None;
            };
            let roll = roll_effect.downcast_ref::<crate::effects::RollDieEffect>()?;
            Some((index, with_id, repeat, roll))
        })
        .collect::<Vec<_>>();
    let [(roll_index, roll_with_id, repeat, roll)] = repeated_rolls.as_slice() else {
        return None;
    };
    if roll.player != PlayerFilter::You || roll.die_text.is_some() {
        return None;
    }

    let branches = effects.get(*roll_index + 1..)?;
    if branches.len() != 2 {
        return None;
    }
    let mut saw_odd = false;
    let mut saw_even = false;
    let mut branch_surfaces = Vec::with_capacity(2);
    for branch in branches {
        let conditional = unwrap_if_effect(branch)?;
        if conditional.condition != roll_with_id.id
            || !conditional.else_.is_empty()
            || conditional.then.is_empty()
        {
            return None;
        }
        let parity = complete_die_parity_label(&conditional.predicate, roll.sides)?;
        let already_seen = match parity {
            "odd" => std::mem::replace(&mut saw_odd, true),
            "even" => std::mem::replace(&mut saw_even, true),
            _ => return None,
        };
        if already_seen {
            return None;
        }
        let result = describe_result_branch_effect_list(&conditional.then);
        let result = result.trim().trim_end_matches('.');
        if result.is_empty() {
            return None;
        }
        branch_surfaces.push(format!(
            "For each {parity} result, {}",
            lowercase_first(result)
        ));
    }
    if !saw_odd || !saw_even {
        return None;
    }

    let count = describe_value(&repeat.count);
    let sides = small_number_word(roll.sides).unwrap_or_else(|| roll.sides.to_string());
    let die_noun = if matches!(repeat.count.unhinted(), Value::Fixed(1)) {
        "die"
    } else {
        "dice"
    };
    let mut surfaces = Vec::new();
    if *roll_index > 0 {
        let prefix = describe_effect_list(&effects[..*roll_index]);
        let prefix = prefix.trim().trim_end_matches('.');
        if prefix.is_empty() {
            return None;
        }
        surfaces.push(prefix.to_string());
    }
    surfaces.push(format!("Roll {count} {sides}-sided {die_noun}"));
    surfaces.extend(branch_surfaces);
    Some(surfaces.join(". "))
}

fn exhaustive_d20_result_branches(branches: &[Effect], roll_id: crate::effect::EffectId) -> bool {
    if branches.len() < 2 {
        return false;
    }
    let mut covered = [false; 21];
    for branch in branches {
        let Some(if_effect) = unwrap_if_effect(branch) else {
            return false;
        };
        if if_effect.condition != roll_id
            || !if_effect.else_.is_empty()
            || if_effect.then.is_empty()
        {
            return false;
        }
        let crate::effect::EffectPredicate::Value(comparison) = &if_effect.predicate else {
            return false;
        };
        let (min, max) = match comparison {
            Comparison::Equal(value) => (*value, *value),
            Comparison::BetweenInclusive(min, max) => (*min, *max),
            _ => return false,
        };
        if min < 1 || max > 20 || min > max {
            return false;
        }
        for result in min..=max {
            let Ok(result) = usize::try_from(result) else {
                return false;
            };
            if std::mem::replace(&mut covered[result], true) {
                return false;
            }
        }
    }
    covered[1..].iter().all(|covered| *covered)
}

/// Render a draw whose typed numeric input is the exact preceding die-roll
/// result. The explicit `PriorEffectResult` hint distinguishes authored "the
/// result" from an ambient trigger event amount, which is especially
/// important inside combat-damage triggers.
pub(in crate::compiled_text) fn describe_roll_die_then_draw_equal_result_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    let [roll_segment, draw_segment, trailing_segments @ ..] = program.segments.as_slice() else {
        return None;
    };
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return None;
    }
    let [roll_effect] = roll_segment.default_effects.as_slice() else {
        return None;
    };
    let roll_with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let roll = roll_with_id
        .effect
        .downcast_ref::<crate::effects::RollDieEffect>()?;
    if roll.player != PlayerFilter::You || roll.die_text.is_some() {
        return None;
    }
    let [draw_effect] = draw_segment.default_effects.as_slice() else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !draw
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::PriorEffectResult)
        || !draw
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
        || !matches!(
            draw.count.unhinted(),
            Value::EffectValue(id) if *id == roll_with_id.id
        )
    {
        return None;
    }

    let mut rendered = format!(
        "{}. Draw cards equal to the result",
        describe_effect(roll_effect).trim().trim_end_matches('.')
    );
    if !trailing_segments.is_empty() {
        let trailing_effects = trailing_segments
            .iter()
            .flat_map(|segment| segment.default_effects.iter().cloned())
            .collect::<Vec<_>>();
        if trailing_effects.is_empty() {
            return None;
        }
        let trailing = describe_effect_list(&trailing_effects);
        let trailing = trailing.trim().trim_end_matches('.');
        if trailing.is_empty() {
            return None;
        }
        rendered.push_str(". ");
        rendered.push_str(&capitalize_first(trailing));
    }
    Some(rendered)
}

/// Rejoin a comma-then setup whose final typed result is a d20 with exhaustive
/// numeric branches lowered alongside it or into later resolution segments.
///
/// The enclosing sequence's result ID is executable provenance: because the
/// roll is its final member, the branch predicates consume that exact roll.
/// Splitting the sequence only for rendering lets the established table
/// compactor recover the authored prefix and rows without guessing from text.
pub(in crate::compiled_text) fn describe_sequenced_d20_numeric_result_table_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return None;
    }
    let effects = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter().cloned())
        .collect::<Vec<_>>();
    let [setup_effect, branches @ ..] = effects.as_slice() else {
        return None;
    };
    if branches.len() < 2 {
        return None;
    }
    let setup_with_id = setup_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sequence = setup_with_id
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let (roll_effect, prefix_effects) = sequence.effects.split_last()?;
    if prefix_effects.is_empty()
        || prefix_effects.iter().any(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::RollDieEffect>()
                .is_some()
        })
    {
        return None;
    }
    let roll = structural_unwrap_render_wrappers(roll_effect)
        .downcast_ref::<crate::effects::RollDieEffect>()?;
    if roll.player != PlayerFilter::You || roll.sides != 20 || roll.die_text.is_some() {
        return None;
    }

    if !exhaustive_d20_result_branches(branches, setup_with_id.id) {
        return None;
    }

    let mut normalized = prefix_effects.to_vec();
    normalized.push(Effect::with_id(setup_with_id.id.0, roll_effect.clone()));
    normalized.extend(branches.iter().cloned());
    describe_roll_die_with_numeric_result_table(&normalized)
}

/// Result rows of a d20 table cover every possible result exactly once. A
/// modified roll ("roll a d20 and add ...") can exceed 20, so its top row is
/// open ("15+"), and its closed rows may extend past 20.
fn d20_result_branches_cover_results(
    branches: &[Effect],
    roll_id: crate::effect::EffectId,
    modified: bool,
) -> bool {
    if branches.len() < 2 {
        return false;
    }
    let mut covered = [false; 101];
    let mut open_top: Option<usize> = None;
    let mut max_closed = 0usize;
    for branch in branches {
        let Some(if_effect) = unwrap_if_effect(branch) else {
            return false;
        };
        if if_effect.condition != roll_id
            || !if_effect.else_.is_empty()
            || if_effect.then.is_empty()
        {
            return false;
        }
        let crate::effect::EffectPredicate::Value(comparison) = &if_effect.predicate else {
            return false;
        };
        let (min, max) = match comparison {
            Comparison::Equal(value) => (*value as i64, *value as i64),
            Comparison::BetweenInclusive(min, max) => (*min as i64, *max as i64),
            Comparison::GreaterThanOrEqual(min) if modified && open_top.is_none() => {
                let Ok(min) = usize::try_from(*min as i64) else {
                    return false;
                };
                if min < 2 || min > 100 {
                    return false;
                }
                open_top = Some(min);
                continue;
            }
            _ => return false,
        };
        let limit = if modified { 100 } else { 20 };
        if min < 1 || max > limit || min > max {
            return false;
        }
        for result in min..=max {
            let Ok(result) = usize::try_from(result) else {
                return false;
            };
            if std::mem::replace(&mut covered[result], true) {
                return false;
            }
            max_closed = max_closed.max(result);
        }
    }
    let top = match open_top {
        Some(open) => {
            if max_closed >= open {
                return false;
            }
            open - 1
        }
        None if modified => max_closed,
        None => 20,
    };
    top >= 1 && covered[1..=top].iter().all(|covered| *covered)
}

/// "roll a d20 for each player being attacked and ignore all but the
/// highest roll" (Iron Mastiff): one retained d20 result feeds the table.
pub(in crate::compiled_text) fn is_highest_single_d20_result_roll(
    roll: &crate::effects::RollDiceChooseResultEffect,
) -> bool {
    roll.sides == 20
        && roll.ignore_lower
        && !roll.total
        && !roll.in_order
        // "Roll two d20 and ignore the lower roll" (Berserker's Frenzy)
        // retains its authored "d20" die name.
        && matches!(roll.die_text.as_deref(), None | Some("d20"))
        && roll.player == PlayerFilter::You
}

/// Rejoin a d20 roll and its numeric result rows when the lowering put the
/// roll and each row into separate resolution segments ("Roll a d20 and add
/// the number of creature cards in your graveyard ... 1—14 | ... 15+ | ...",
/// Revivify, Diviner's Portent, Danse Macabre). The rows' predicates consume
/// the roll's exact effect id, so the established table compactor applies.
pub(in crate::compiled_text) fn describe_segmented_d20_numeric_result_table_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.is_empty()
        || program
            .segments
            .iter()
            .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return None;
    }
    let effects = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter().cloned())
        .collect::<Vec<_>>();
    let roll_idx = effects.iter().position(|effect| {
        effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .is_some_and(|with_id| {
                with_id
                    .effect
                    .downcast_ref::<crate::effects::RollDieEffect>()
                    .is_some()
                    || with_id
                        .effect
                        .downcast_ref::<crate::effects::RollDiceChooseResultEffect>()
                        .is_some_and(is_highest_single_d20_result_roll)
            })
    })?;
    let with_id = effects[roll_idx].downcast_ref::<crate::effects::WithIdEffect>()?;
    let modified = if let Some(roll) = with_id
        .effect
        .downcast_ref::<crate::effects::RollDieEffect>()
    {
        if roll.sides != 20 || roll.die_text.is_some() {
            return None;
        }
        roll.result_modifier.is_some()
    } else {
        false
    };
    let branches = &effects[roll_idx + 1..];
    if !d20_result_branches_cover_results(branches, with_id.id, modified) {
        return None;
    }
    describe_roll_die_with_numeric_result_table(&effects)
}

/// Render a die-result backreference and a subsequent random attachment from
/// the effect graph that proves both relationships.
pub(super) fn describe_roll_result_damage_then_random_source_attachment(
    effects: &[Effect],
) -> Option<String> {
    fn transparent_single(effect: &Effect) -> &Effect {
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
            && let [inner] = sequence.effects.as_slice()
        {
            return transparent_single(inner);
        }
        effect
    }

    let mut damage_source_surface = None;
    let (roll_effect, damage_effect, attachment_target) = match effects {
        [
            roll_effect,
            damage_effect,
            choose_player_effect,
            attach_effect,
        ] => {
            let choose =
                choose_player_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
            if choose.chooser != PlayerFilter::You
                || !choose.random
                || !choose.excluded_tags.is_empty()
                || choose.remember_as_chosen_player
            {
                return None;
            }

            let attach = transparent_single(attach_effect)
                .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
            if !matches!(attach.objects.base(), ChooseSpec::Source)
                || !matches!(
                    attach.target.base(),
                    ChooseSpec::Player(PlayerFilter::TaggedPlayer(tag)) if tag == &choose.tag
                )
            {
                return None;
            }
            (
                roll_effect,
                damage_effect,
                describe_player_filter(&choose.filter),
            )
        }
        [roll_effect, damage_effect, attach_effect] => {
            let attach = transparent_single(attach_effect)
                .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
            let count = attach.target.count();
            let ChooseSpec::Player(filter) = attach.target.base() else {
                return None;
            };
            if !matches!(attach.objects.base(), ChooseSpec::Source)
                || !count.is_single()
                || !count.random
            {
                return None;
            }
            (roll_effect, damage_effect, describe_player_filter(filter))
        }
        _ => return None,
    };

    let roll_with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    roll_with_id
        .effect
        .downcast_ref::<crate::effects::RollDieEffect>()?;

    let damage_effect = if let Some(execute) =
        damage_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
    {
        if !matches!(execute.source.base(), ChooseSpec::Source) {
            return None;
        }
        damage_source_surface = Some(describe_choose_spec(&execute.source));
        execute.effect.as_ref()
    } else {
        damage_effect
    };
    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || damage.unpreventable
        || !matches!(
            damage.amount.unhinted(),
            Value::EffectValue(id) if *id == roll_with_id.id
        )
    {
        return None;
    }

    let roll = describe_effect(roll_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let damage_target = describe_choose_spec(&damage.target);
    if damage_source_surface.as_deref() == Some("this Aura")
        && matches!(
            damage.target.base(),
            ChooseSpec::Player(PlayerFilter::AliasedControllerOf(crate::target::ObjectRef::Tagged(tag)))
                if tag.as_str() == "triggering"
        )
    {
        return Some(format!(
            "{roll}. This Aura deals that much damage to that player. Then attach this Aura to {attachment_target} chosen at random"
        ));
    }
    Some(format!(
        "{roll}. Deal damage to {damage_target} equal to the result. Then attach this source to {attachment_target} chosen at random"
    ))
}

/// The authored roll, damage, and attachment sentences lower into separate
/// resolution segments. Flatten only an otherwise plain program so the same
/// relationship proof also governs cross-segment rendering.
pub(in crate::compiled_text) fn describe_roll_result_damage_then_random_source_attachment_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let effects = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter().cloned())
        .collect::<Vec<_>>();
    describe_roll_result_damage_then_random_source_attachment(&effects)
}
