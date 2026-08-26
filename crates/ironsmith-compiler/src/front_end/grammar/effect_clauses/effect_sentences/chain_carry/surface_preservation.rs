use super::*;

pub fn parse_each_prior_affected_object_controller_mana_value_life(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    const PREFIX: &[&str] = &["the", "controller", "of", "each", "of", "those"];
    const SUFFIX: &[&str] = &["gains", "life", "equal", "to", "its", "mana", "value"];
    if !crate::word_primitives::parse_sequence_prefix(&words, PREFIX)
        || !crate::word_primitives::parse_sequence_suffix(&words, SUFFIX)
        || words.len() <= PREFIX.len() + SUFFIX.len()
    {
        return Ok(None);
    }
    let noun_words = &words[PREFIX.len()..words.len() - SUFFIX.len()];
    let noun_tokens = crate::lexer::synthetic_word_tokens(noun_words);
    let noun_filter = parse_object_filter(&noun_tokens, false)?;
    if noun_filter.card_types.is_empty()
        && noun_filter.subtypes.is_empty()
        && noun_filter.any_of.is_empty()
    {
        return Ok(None);
    }

    let it = TagKey::from(IT_TAG);
    Ok(Some(EffectAst::ForEachTagged {
        tag: it.clone(),
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::ItsController,
            SubjectVerbActionAst::GainLife {
                amount: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(it)))
                    .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
            },
        )],
    }))
}

fn nested_comma_then_candidate_count(effect: &EffectAst) -> usize {
    if matches!(effect, EffectAst::CommaThen { .. }) {
        return 0;
    }

    let mut count = 0usize;
    for_each_nested_effects(effect, true, |nested| {
        if nested.len() > 1 {
            count += 1;
        } else if let [child] = nested {
            count += nested_comma_then_candidate_count(child);
        }
    });
    count
}

fn wrap_first_nested_comma_then_candidate(effect: &mut EffectAst) -> bool {
    if matches!(effect, EffectAst::CommaThen { .. }) {
        return false;
    }

    let mut wrapped = false;
    for_each_nested_effect_vec_mut(effect, true, |nested| {
        if wrapped {
            return;
        }
        if nested.len() > 1 {
            let effects = std::mem::take(nested);
            nested.push(EffectAst::CommaThen { effects });
            wrapped = true;
        } else if let [child] = nested.as_mut_slice() {
            wrapped = wrap_first_nested_comma_then_candidate(child);
        }
    });
    wrapped
}

fn preserve_unique_nested_comma_then_surface(effects: &mut [EffectAst]) {
    let [effect] = effects else {
        return;
    };
    if nested_comma_then_candidate_count(effect) == 1 {
        let _ = wrap_first_nested_comma_then_candidate(effect);
    }
}

pub fn preserve_coordinated_effect_chain_surface(
    tokens: &[OwnedLexToken],
    mut effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    // Whole-line parsing can reach this surface-preservation pass with the
    // semantic actions already flattened, without going through
    // `parse_effect_chain_lexed`. Preserve an authored same-sentence
    // comma-then boundary here as well so every parser entrypoint produces
    // the same typed sequence.
    if effects.len() > 1 && has_authored_comma_then_surface_lexed(tokens) {
        let coordination = crate::grammar::effects::coordination::coordination_from_effects(
            crate::model::CoordinationKindAst::Carry,
            crate::model::CoordinationOperatorAst::CommaThen,
            crate::model::EffectOrderingAst::Ordered,
            effects,
        )
        .expect("comma-then surface contains at least two effects");
        return vec![EffectAst::Coordination(coordination)];
    }
    if has_authored_comma_then_surface_lexed(tokens) {
        preserve_unique_nested_comma_then_surface(&mut effects);
    }

    let Some(leading_duration) = chain_grammar::coordinated_effect_chain_leading_duration(tokens)
    else {
        return effects;
    };

    // A shared-subject tail can already be coordinated by its specialist
    // parser even though the top-level grammar proves that an earlier action
    // belongs to the same authored conjunction. Flatten only an ordinary
    // nested conjunction so the complete source clause keeps one typed
    // boundary. Result conjunctions and duration-leading conjunctions carry
    // additional semantics and must remain nested.
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::Coordinated {
                leading_duration: true,
                ..
            } | EffectAst::Coordinated {
                result_conjunction: true,
                ..
            }
        )
    }) {
        return effects;
    }
    if effects.len() > 1
        && effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::Coordinated { .. }))
    {
        effects = effects
            .into_iter()
            .flat_map(|effect| match effect {
                EffectAst::Coordinated {
                    effects,
                    leading_duration: false,
                    result_conjunction: false,
                } => effects,
                effect => vec![effect],
            })
            .collect();
    }

    // The grammar above proves this was a top-level source conjunction and
    // rejects card-type lists, quoted text, shared subjects, and every clause
    // containing an explicit "then". Keep that authored relationship as
    // typed surface metadata for every semantic action family. The sequence
    // still executes its children in order, so reference flow between arms is
    // preserved without asking the renderer to infer coordination later.
    if effects.len() < 2 {
        return effects;
    }

    // In `gains flying and loses trample until end of turn`, the trailing
    // duration scopes both coordinated continuous-effect arms. Only carry it
    // backward across an exact two-arm pair with the same semantic target,
    // and only when the first arm has no authored duration of its own.
    if !leading_duration
        && let Some(duration) = shared_trailing_continuous_effect_duration(&effects)
    {
        apply_carried_effect_duration(&mut effects[0], &duration);
    }

    let coordination = crate::grammar::effects::coordination::coordination_from_effects(
        crate::model::CoordinationKindAst::Carry,
        crate::model::CoordinationOperatorAst::And,
        crate::model::EffectOrderingAst::Unordered,
        effects,
    )
    .expect("coordinated effect-chain surface contains at least two effects");
    let coordinated = EffectAst::Coordination(coordination);
    if leading_duration {
        return crate::grammar::effects::control_flow::wrap_leading_duration_program(
            tokens,
            vec![coordinated.clone()],
        )
        .map_or_else(|| vec![coordinated], |wrapped| vec![wrapped]);
    }
    vec![coordinated]
}

enum ContinuousEffectScope<'a> {
    Target(&'a TargetAst),
    Filter(&'a ObjectFilter),
}

fn target_is_source(target: &TargetAst) -> bool {
    matches!(target, TargetAst::Source(_))
        || matches!(target, TargetAst::Object(filter, _, _) if filter.source)
}

fn same_target_ignoring_surface_spans(left: &TargetAst, right: &TargetAst) -> bool {
    if target_is_source(left) && target_is_source(right) {
        return true;
    }
    if matches!(right, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG)
        || matches!(left, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG)
    {
        // Within an exact two-arm continuous-effect coordination, `it` is
        // the typed anaphor for the other arm's declared target. Treating the
        // span-bearing target nodes as unequal would drop a trailing duration
        // from the first arm of `loses first strike or swampwalk until end of
        // turn`.
        return true;
    }
    match (left, right) {
        (TargetAst::AnyTarget(_), TargetAst::AnyTarget(_))
        | (TargetAst::AnyOtherTarget(_), TargetAst::AnyOtherTarget(_))
        | (
            TargetAst::AttackedPlayerOrPlaneswalker(_),
            TargetAst::AttackedPlayerOrPlaneswalker(_),
        )
        | (TargetAst::Spell(_), TargetAst::Spell(_)) => true,
        (
            TargetAst::ObjectOrPlayer(left_object, left_player, _),
            TargetAst::ObjectOrPlayer(right_object, right_player, _),
        ) => left_object == right_object && left_player == right_player,
        (TargetAst::PlayerOrPlaneswalker(left, _), TargetAst::PlayerOrPlaneswalker(right, _))
        | (TargetAst::Player(left, _), TargetAst::Player(right, _)) => left == right,
        (TargetAst::Object(left, _, _), TargetAst::Object(right, _, _)) => left == right,
        (TargetAst::Tagged(left, _), TargetAst::Tagged(right, _)) => left == right,
        (
            TargetAst::WithCount(left_target, left_count),
            TargetAst::WithCount(right_target, right_count),
        ) => {
            left_count == right_count
                && same_target_ignoring_surface_spans(left_target, right_target)
        }
        (
            TargetAst::WithCountValue(left_target, left_count, left_value),
            TargetAst::WithCountValue(right_target, right_count, right_value),
        ) => {
            left_count == right_count
                && left_value == right_value
                && same_target_ignoring_surface_spans(left_target, right_target)
        }
        _ => false,
    }
}

fn continuous_effect_scope_and_duration(
    effect: &EffectAst,
) -> Option<(ContinuousEffectScope<'_>, &Until)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return None;
    };
    match action {
        SubjectVerbActionAst::GainControl {
            target, duration, ..
        }
        | SubjectVerbActionAst::Pump {
            target, duration, ..
        }
        | SubjectVerbActionAst::PumpForEach {
            target, duration, ..
        }
        | SubjectVerbActionAst::PumpByLastEffect {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetBasePowerToughness {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetBasePower {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeBasePtCreature {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddCardTypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetCardTypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveCardTypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddSubtypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveSubtypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetCreatureSubtypes {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddColors {
            target, duration, ..
        }
        | SubjectVerbActionAst::AddAllSubtypesOfFamily {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeAuraEnchantment {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeBasicLandType {
            target, duration, ..
        }
        | SubjectVerbActionAst::SetColors {
            target, duration, ..
        }
        | SubjectVerbActionAst::MakeColorless {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeCreatureTypeChoice {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeColorChoice {
            target, duration, ..
        }
        | SubjectVerbActionAst::BecomeCopy {
            target, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesToTarget {
            target, duration, ..
        }
        | SubjectVerbActionAst::RemoveAbilitiesFromTarget {
            target, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
            target, duration, ..
        } => Some((ContinuousEffectScope::Target(target), duration)),
        SubjectVerbActionAst::PumpAll {
            filter, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesAll {
            filter, duration, ..
        }
        | SubjectVerbActionAst::RemoveAbilitiesAll {
            filter, duration, ..
        }
        | SubjectVerbActionAst::GrantAbilitiesChoiceAll {
            filter, duration, ..
        } => Some((ContinuousEffectScope::Filter(filter), duration)),
        _ => None,
    }
}

fn same_continuous_effect_scope(
    left: ContinuousEffectScope<'_>,
    right: ContinuousEffectScope<'_>,
) -> bool {
    match (left, right) {
        (ContinuousEffectScope::Target(left), ContinuousEffectScope::Target(right)) => {
            same_target_ignoring_surface_spans(left, right)
        }
        (ContinuousEffectScope::Filter(left), ContinuousEffectScope::Filter(right)) => {
            left == right
        }
        _ => false,
    }
}

pub(super) fn shared_trailing_continuous_effect_duration(effects: &[EffectAst]) -> Option<Until> {
    let [first, second] = effects else {
        return None;
    };
    let (first_scope, first_duration) = continuous_effect_scope_and_duration(first)?;
    let (second_scope, second_duration) = continuous_effect_scope_and_duration(second)?;
    (matches!(first_duration, Until::Forever)
        && !matches!(second_duration, Until::Forever)
        && same_continuous_effect_scope(first_scope, second_scope))
    .then(|| second_duration.clone())
}
