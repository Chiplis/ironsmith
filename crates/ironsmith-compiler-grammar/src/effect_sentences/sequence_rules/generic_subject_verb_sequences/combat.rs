use super::*;

pub fn parse_damage_prevention_reflect_to_any_target_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PreventDamage {
                amount,
                target,
                duration,
                source_of_your_choice,
                protect_you_and_permanents_you_control,
                ..
            },
        ..
    }) = first_effect
    else {
        return Ok(None);
    };

    if !sequence_grammar::parse_prevention_reflect_followup_shape(
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    let follow_up = EffectAst::subject_verb_damage(
        Value::EventValue(EventValueSpec::Amount),
        TargetAst::AnyTarget(None),
    );
    Ok(Some(vec![
        EffectAst::subject_verb_prevent_damage_with_options(
            amount.clone(),
            target.clone(),
            duration.clone(),
            *source_of_your_choice,
            *protect_you_and_permanents_you_control,
            vec![follow_up],
        ),
    ]))
}

pub fn parse_next_damage_prevention_gain_life_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_mut_slice() else {
        return Ok(None);
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = first_effect else {
        return Ok(None);
    };
    let follow_up_effects = match action {
        SubjectVerbActionAst::PreventNextTimeDamage {
            follow_up_effects, ..
        }
        | SubjectVerbActionAst::PreventDamage {
            follow_up_effects, ..
        } => follow_up_effects,
        _ => return Ok(None),
    };
    if !follow_up_effects.is_empty() {
        return Ok(None);
    }

    if !sequence_grammar::parse_prevention_gain_life_followup_shape(
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    // The exact sequence shape above establishes both the affected player and
    // the event-relative amount.  Construct that typed follow-up directly:
    // parsing the sentence in isolation loses the prevention-event context.
    follow_up_effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::GainLife {
            amount: Value::EventValue(EventValueSpec::Amount),
        },
    ));
    Ok(Some(first_effects))
}
