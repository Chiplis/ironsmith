use super::*;

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

/// Render a die-result backreference and a subsequent random attachment from
/// the effect graph that proves both relationships.
pub(super) fn describe_roll_result_damage_then_random_source_attachment(
    effects: &[Effect],
) -> Option<String> {
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

            let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
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
            let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
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
