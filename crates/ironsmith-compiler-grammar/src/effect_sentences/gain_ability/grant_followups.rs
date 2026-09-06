use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::StatChangeActionAst;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::GrantActionAst;
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
                SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                    duration: effect_duration,
                    ..
                }),
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
        return Ok(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: trailing_if.predicate,
            effects: vec![pump],
        }));
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
        return Ok(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true: effects,
            if_false: Vec::new(),
        })]);
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
