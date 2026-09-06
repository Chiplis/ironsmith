use crate::cards::builders::PermissionEffectAst;
use crate::cards::builders::ForEachEffectAst;
use super::*;

pub(super) fn bind_adjacent_life_stat_pronouns(
    effects: &mut [EffectAst],
    recognized_reference: bool,
) {
    if !recognized_reference {
        return;
    }

    fn tagged_stat_reference(value: &Value) -> Option<crate::target::ChooseSpec> {
        let spec = match value.unhinted() {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) => spec.as_ref(),
            _ => return None,
        };
        matches!(spec.unhinted(), crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
            .then(|| spec.clone())
    }

    fn life_amount(effect: &EffectAst) -> Option<&Value> {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return None;
        };
        match action {
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount }) => Some(amount),
            _ => None,
        }
    }

    fn retarget_source_stat(value: &mut Value, antecedent: &crate::target::ChooseSpec) {
        match value {
            Value::SurfaceHinted { value, .. } => retarget_source_stat(value, antecedent),
            Value::SourcePower => {
                *value = Value::PowerOf(Box::new(antecedent.clone()));
            }
            Value::SourceToughness => {
                *value = Value::ToughnessOf(Box::new(antecedent.clone()));
            }
            Value::PowerOf(spec)
                if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
            {
                *spec = Box::new(antecedent.clone());
            }
            Value::ToughnessOf(spec)
                if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
            {
                *spec = Box::new(antecedent.clone());
            }
            _ => {}
        }
    }

    for index in 0..effects.len().saturating_sub(1) {
        let Some(antecedent) = life_amount(&effects[index]).and_then(tagged_stat_reference) else {
            continue;
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = &mut effects[index + 1]
        else {
            continue;
        };
        let amount = match action {
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount }) => amount,
            _ => continue,
        };
        retarget_source_stat(amount, &antecedent);
    }
}

/// Repair an X-valued life follow-up that the isolated clause parser lowered
/// to its historical source-stat fallback before the sentence-wide where-X
/// binder ran. The authored X uses and typed tagged stat value together prove
/// that both adjacent life actions share one value; copying the complete value
/// preserves the same LKI object identity and presentation hints.
pub fn bind_adjacent_shared_x_life_stat_values(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) {
    let words = token_word_refs(tokens);
    let Some(where_x_index) =
        crate::word_primitives::parse_sequence_start(&words, &["where", "x", "is"])
    else {
        return;
    };
    if words[..where_x_index]
        .iter()
        .filter(|word| **word == "x")
        .count()
        < 2
        || !words.get(where_x_index + 3..).is_some_and(|tail| {
            crate::word_primitives::parse_any_sequence_prefix(
                tail,
                &[&["its", "power"], &["its", "toughness"]],
            )
        })
    {
        return;
    }

    fn life_amount(effect: &EffectAst) -> Option<&Value> {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return None;
        };
        match action {
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount }) => Some(amount),
            _ => None,
        }
    }

    fn life_amount_mut(effect: &mut EffectAst) -> Option<&mut Value> {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return None;
        };
        match action {
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount }) => Some(amount),
            _ => None,
        }
    }

    fn tagged_where_x_stat(value: &Value) -> bool {
        if !value.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs) {
            return false;
        }
        let spec = match value.unhinted() {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) => spec,
            _ => return false,
        };
        matches!(spec.unhinted(), ChooseSpec::Tagged(_))
    }

    fn bind_in_list(effects: &mut [EffectAst]) {
        for index in 0..effects.len().saturating_sub(1) {
            let (leading, trailing) = effects.split_at_mut(index + 1);
            let Some(shared_value) = life_amount(&leading[index])
                .filter(|value| tagged_where_x_stat(value))
                .cloned()
            else {
                continue;
            };
            let Some(follow_up) = life_amount_mut(&mut trailing[0]) else {
                continue;
            };
            if !follow_up.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
                && matches!(
                    follow_up.unhinted(),
                    Value::SourcePower | Value::SourceToughness
                )
            {
                *follow_up = shared_value;
            }
        }
    }

    bind_in_list(effects);
    for effect in effects {
        for_each_nested_effects_mut(effect, true, bind_in_list);
    }
}

pub(super) fn effect_uses_half_life_total_value(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                    dynamic_power_toughness,
                    ..
                }),
            ..
        }) => dynamic_power_toughness
            .as_ref()
            .is_some_and(|(power, toughness)| {
                value_is_half_life_total(power) || value_is_half_life_total(toughness)
            }),
        EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })
        | EffectAst::ManaRestricted { effects, .. } => {
            effects.iter().any(effect_uses_half_life_total_value)
        }
        _ => false,
    }
}

pub(super) fn value_is_half_life_total(value: &Value) -> bool {
    matches!(value.unhinted(), Value::HalfLifeTotalRoundedUp(_))
}

pub fn collapse_token_copy_next_end_step_sacrifice_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    if !facts.has_sacrifice || !facts.has_token {
        return;
    }
    let (is_next_upkeep, upkeep_player_is_you, next_end_step_player) = match facts.timing {
        Some(chain_grammar::DelayedCopyTiming::EndStep { player_is_you }) => (
            false,
            false,
            if player_is_you {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            },
        ),
        Some(chain_grammar::DelayedCopyTiming::Upkeep { player_is_you }) => {
            (true, player_is_you, PlayerFilter::Any)
        }
        _ => return,
    };

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_sacrifice = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { .. })
                        | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { .. }),
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { filter, count, .. }),
                    ..
                }),
            ) => *count == 1 && filter.token,
            _ => false,
        };

        if !mark_next_end_step_sacrifice {
            idx += 1;
            continue;
        }

        if is_next_upkeep {
            let sacrifice = effects.remove(idx + 1);
            effects.insert(
                idx + 1,
                EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep {
                    player: if upkeep_player_is_you {
                        PlayerAst::You
                    } else {
                        PlayerAst::Any
                    },
                    effects: vec![sacrifice],
                }),
            );
            idx += 2;
            continue;
        }

        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy {
                    sacrifice_at_next_end_step,
                    sacrifice_at_next_end_step_reference_surface,
                    next_end_step_player: effect_next_end_step_player,
                    ..
                })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                    sacrifice_at_next_end_step,
                    sacrifice_at_next_end_step_reference_surface,
                    next_end_step_player: effect_next_end_step_player,
                    ..
                }),
            ..
        }) = &mut effects[idx]
        {
            *sacrifice_at_next_end_step = true;
            *sacrifice_at_next_end_step_reference_surface =
                token_copy_action_reference_surface(tokens, "sacrifice");
            *effect_next_end_step_player = next_end_step_player.clone();
        }
        effects.remove(idx + 1);
    }
}
