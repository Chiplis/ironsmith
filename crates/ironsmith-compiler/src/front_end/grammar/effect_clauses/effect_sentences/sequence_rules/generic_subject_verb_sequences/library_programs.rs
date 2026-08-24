use super::*;

pub fn parse_next_damage_prevention_exile_top_sequence(
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

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PreventNextTimeDamage {
                follow_up_effects, ..
            },
        ..
    }) = first_effect
    else {
        return Ok(None);
    };
    if !follow_up_effects.is_empty() {
        return Ok(None);
    }

    if !sequence_grammar::parse_prevention_exile_top_followup_shape(
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    follow_up_effects.push(EffectAst::subject_verb_exile_top_of_library(
        PlayerAst::You,
        Value::EventValue(EventValueSpec::Amount),
        Vec::new(),
        Vec::new(),
    ));
    Ok(Some(first_effects))
}

pub fn parse_search_delayed_upkeep_unless_pays_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = effect_sentences::parse_effect_chain(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if first_effects.is_empty() {
        return Ok(None);
    }

    let Some(shape) = sequence_grammar::parse_delayed_upkeep_payment_shape(
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };

    let mut effects = first_effects;
    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: ironsmith_core::TotalCost::mana(shape.mana),
            before_delayed_step: false,
        }],
    });
    Ok(Some(effects))
}
