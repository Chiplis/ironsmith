use super::*;

pub(super) fn apply_gain_clause_duration_to_leading_effect(
    effect: &mut EffectAst,
    duration: &Until,
) {
    match effect {
        EffectAst::Sequence { effects } => {
            for child in effects {
                apply_gain_clause_duration_to_leading_effect(child, duration);
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Pump {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpForEach {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpByLastEffect {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasePtCreature {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandType {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::MakeColorless {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCopy {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesChoiceAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                    duration: effect_duration,
                    ..
                },
            ..
        }) => {
            *effect_duration = duration.clone();
        }
        _ => {}
    }
}

pub(super) fn parse_single_effect_sentence_for_granted_otherwise(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &["you", "become", "the", "monarch"],
            &["you", "become", "monarch"],
        ],
    ) {
        return Ok(EffectAst::subject_verb_become_monarch(PlayerAst::You));
    }
    if let Some(trailing_if) = crate::grammar::structure::split_trailing_if_clause_lexed(tokens)
        && let Some(shape) =
            crate::grammar::effects::clause_dispatch_shapes::parse_clause_subject_verb_shape(
                trailing_if.leading_tokens,
            )
        && shape.kind == crate::grammar::effects::chain_splitting::ChainVerbKind::Get
        && let Some(pump) = super::super::parse_get_pump_clause(
            shape.subject_tokens,
            shape.action_tokens,
            trailing_if.leading_tokens,
        )?
    {
        return Ok(EffectAst::TrailingIf {
            predicate: trailing_if.predicate,
            effects: vec![pump],
        });
    }
    let mut effects = parse_effect_sentence_lexed(tokens)?;
    match effects.len() {
        0 => Err(CardTextError::ParseError(
            "empty otherwise branch in granted triggered ability".to_string(),
        )),
        1 => Ok(effects.remove(0)),
        _ => Ok(EffectAst::Sequence { effects }),
    }
}

pub fn append_gain_ability_trailing_effects(
    mut effects: Vec<EffectAst>,
    trailing_tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if trailing_tokens.is_empty() {
        return Ok(effects);
    }

    let trimmed = trim_commas(trailing_tokens);
    if let Some(predicate) = parse_trailing_if_predicate_lexed(&trimmed) {
        return Ok(vec![EffectAst::Conditional {
            predicate,
            if_true: effects,
            if_false: Vec::new(),
        }]);
    }

    if token_slice_first_is(&trimmed, "unless") {
        if let Some(unless_effect) =
            try_build_unless(effects, SubjectVerbPrimitiveClause::new(&trimmed), 0)?
        {
            return Ok(vec![unless_effect]);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing unless gain-ability clause (clause: '{}')",
            render_lower_words(&trimmed)
        )));
    }

    if let Ok(parsed_tail) = parse_effect_chain(&trimmed)
        && !parsed_tail.is_empty()
    {
        effects.extend(parsed_tail);
    }
    Ok(effects)
}
