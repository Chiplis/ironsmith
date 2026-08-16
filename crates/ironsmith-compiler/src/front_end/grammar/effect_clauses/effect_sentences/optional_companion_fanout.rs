use super::super::grammar::effects::optional_companion_shapes::{
    LeadingOptionalCompanionVerb, parse_leading_optional_companion_shape,
    parse_shared_subject_optional_companion_shape,
};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed,
};
use super::super::lexer::OwnedLexToken;
use super::parse_effect_sentence_lexed;
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, EffectAst, GrantedAbilityAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, TargetAst, TextSpan,
};
use crate::effect::Until;

fn synthetic_word(word: &str) -> OwnedLexToken {
    OwnedLexToken::word(word, TextSpan::synthetic())
}

fn parse_rewritten_effects(tokens: Vec<OwnedLexToken>) -> Result<Vec<EffectAst>, CardTextError> {
    let effects = parse_effect_sentence_lexed(&tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "optional-companion branch produced no semantic effects (clause: '{}')",
            crate::lexer::token_word_refs(&tokens).join(" ")
        )));
    }
    Ok(effects)
}

fn subject_action_clause(
    subject: &[OwnedLexToken],
    action: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let mut clause = Vec::with_capacity(subject.len() + action.len());
    clause.extend_from_slice(subject);
    clause.extend_from_slice(action);
    clause
}

fn leading_action_clause(
    verb: LeadingOptionalCompanionVerb,
    target: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let verb = match verb {
        LeadingOptionalCompanionVerb::Destroy => "destroy",
        LeadingOptionalCompanionVerb::Tap => "tap",
    };
    let mut clause = Vec::with_capacity(target.len() + 1);
    clause.push(synthetic_word(verb));
    clause.extend_from_slice(target);
    clause
}

fn choice_grant_parts(effect: EffectAst) -> Option<(TargetAst, Vec<GrantedAbilityAst>, Until)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            },
        ..
    }) = effect
    else {
        return None;
    };
    Some((target, abilities, duration))
}

/// Convert two per-subject keyword-choice parses into one resolution-time
/// choice whose selected keyword is applied to both subjects. The target
/// phrases remain inside coordinated child effects, so lowering exposes the
/// optional target slot at cast/trigger targeting time rather than turning it
/// into a resolution-time object choice.
fn combine_shared_keyword_choice(
    first: &[EffectAst],
    second: &[EffectAst],
) -> Result<Option<EffectAst>, CardTextError> {
    let [first] = first else {
        return Ok(None);
    };
    let [second] = second else {
        return Ok(None);
    };
    let Some((first_target, first_abilities, first_duration)) = choice_grant_parts(first.clone())
    else {
        return Ok(None);
    };
    let Some((second_target, second_abilities, second_duration)) =
        choice_grant_parts(second.clone())
    else {
        return Ok(None);
    };
    if first_abilities != second_abilities || first_duration != second_duration {
        return Err(CardTextError::ParseError(
            "shared optional-companion keyword choice produced mismatched branches".to_string(),
        ));
    }
    if first_abilities.len() < 2 {
        return Ok(None);
    }

    let modes = first_abilities
        .into_iter()
        .map(|ability| ChooseOneModeAst {
            description: String::new(),
            effects: vec![EffectAst::Coordinated {
                effects: vec![
                    EffectAst::subject_verb_grant_abilities_to_target(
                        first_target.clone(),
                        vec![ability.clone()],
                        first_duration.clone(),
                    ),
                    EffectAst::subject_verb_grant_abilities_to_target(
                        second_target.clone(),
                        vec![ability],
                        first_duration.clone(),
                    ),
                ],
                leading_duration: false,
                result_conjunction: false,
            }],
        })
        .collect();
    Ok(Some(EffectAst::ChooseOneOf { modes }))
}

fn parse_optional_companion_fanout_body(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(shape) = parse_shared_subject_optional_companion_shape(tokens) {
        let mut first = parse_rewritten_effects(subject_action_clause(
            shape.first_subject_tokens,
            shape.action_tokens,
        ))?;
        let second = parse_rewritten_effects(subject_action_clause(
            shape.companion_tokens,
            shape.action_tokens,
        ))?;
        if let Some(choice) = combine_shared_keyword_choice(&first, &second)? {
            return Ok(Some(vec![choice]));
        }
        first.extend(second);
        return Ok(Some(vec![EffectAst::Coordinated {
            effects: first,
            leading_duration: false,
            result_conjunction: false,
        }]));
    }

    if let Some(shape) = parse_leading_optional_companion_shape(tokens) {
        let mut first =
            parse_rewritten_effects(leading_action_clause(shape.verb, shape.first_target_tokens))?;
        let second =
            parse_rewritten_effects(leading_action_clause(shape.verb, shape.companion_tokens))?;
        first.extend(second);
        return Ok(Some(vec![EffectAst::Coordinated {
            effects: first,
            leading_duration: false,
            result_conjunction: false,
        }]));
    }

    Ok(None)
}

pub(crate) fn parse_optional_companion_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(prefix) = split_leading_result_prefix_lexed(tokens) else {
        return parse_optional_companion_fanout_body(tokens);
    };
    let Some(mut effects) = parse_optional_companion_fanout_body(prefix.trailing_tokens)? else {
        return Ok(None);
    };

    super::preserve_result_conjunction_body_lexed(prefix.trailing_tokens, &mut effects);
    Ok(Some(vec![match prefix.kind {
        LeadingResultPrefixKind::If => EffectAst::IfResult {
            predicate: prefix.predicate,
            effects,
        },
        LeadingResultPrefixKind::When => EffectAst::WhenResult {
            predicate: prefix.predicate,
            effects,
        },
    }]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::ChoiceCount;
    use crate::lexer::lex_line;

    fn optional_target(target: &TargetAst) -> Option<(&TargetAst, ChoiceCount)> {
        let TargetAst::WithCount(inner, count) = target else {
            return None;
        };
        Some((inner, *count))
    }

    #[test]
    fn razorgrass_keeps_source_and_independently_optional_other_target() {
        let tokens = lex_line(
            "This creature and up to one other target creature each get +3/+3 until end of turn.",
            0,
        )
        .unwrap();
        let parsed = parse_optional_companion_fanout_sentence(&tokens)
            .unwrap()
            .unwrap();
        let [EffectAst::Coordinated { effects, .. }] = parsed.as_slice() else {
            panic!("expected coordinated source/companion pump: {parsed:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Pump { target: first, .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Pump { target: second, .. },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected two pump actions: {effects:#?}");
        };
        assert!(matches!(
            first,
            TargetAst::Source(_) | TargetAst::Object(_, _, _)
        ));
        let (second, count) = optional_target(second).expect("optional companion target");
        assert_eq!(count.min, 0);
        assert_eq!(count.max, Some(1));
        let TargetAst::Object(filter, _, _) = second else {
            panic!("expected object companion target: {second:#?}");
        };
        assert!(filter.other);
    }

    #[test]
    fn ballroom_uses_one_keyword_choice_for_both_subjects() {
        let tokens = lex_line(
            "This creature and up to one other target creature you control both gain your choice of first strike or lifelink until end of turn.",
            0,
        )
        .unwrap();
        let parsed = parse_optional_companion_fanout_sentence(&tokens)
            .unwrap()
            .unwrap();
        let [EffectAst::ChooseOneOf { modes }] = parsed.as_slice() else {
            panic!("expected one shared keyword choice: {parsed:#?}");
        };
        assert_eq!(modes.len(), 2);
        for mode in modes {
            let [EffectAst::Coordinated { effects, .. }] = mode.effects.as_slice() else {
                panic!("expected coordinated grants in each mode: {mode:#?}");
            };
            assert_eq!(effects.len(), 2);
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. },
                ..
            }) = &effects[1]
            else {
                panic!("expected companion grant: {effects:#?}");
            };
            let (_, count) = optional_target(target).expect("optional companion target");
            assert_eq!((count.min, count.max), (0, Some(1)));
        }
    }

    #[test]
    fn dragon_turtle_keeps_source_and_optional_opposing_creature_target() {
        let tokens = lex_line(
            "Tap it and up to one target creature an opponent controls.",
            0,
        )
        .unwrap();
        let parsed = parse_optional_companion_fanout_sentence(&tokens)
            .unwrap()
            .unwrap();
        let [EffectAst::Coordinated { effects, .. }] = parsed.as_slice() else {
            panic!("expected coordinated taps: {parsed:#?}");
        };
        assert_eq!(effects.len(), 2);
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tap { target, .. },
            ..
        }) = &effects[1]
        else {
            panic!("expected optional tap target: {effects:#?}");
        };
        let (_, count) = optional_target(target).expect("optional tap target");
        assert_eq!((count.min, count.max), (0, Some(1)));
    }

    #[test]
    fn lockjaw_keeps_each_target_declaration_with_its_restriction() {
        let tokens = lex_line(
            "This creature and up to one other target creature can't be blocked this turn.",
            0,
        )
        .unwrap();
        let parsed = parse_optional_companion_fanout_sentence(&tokens)
            .unwrap()
            .unwrap();
        let effects = match parsed.as_slice() {
            [EffectAst::Coordinated { effects, .. }] => effects.as_slice(),
            effects => effects,
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Cant { .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TargetOnly { target, .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Cant { .. },
                ..
            }),
        ] = effects
        else {
            panic!("expected source restriction and optional target pair: {effects:#?}");
        };
        let (_, count) = optional_target(target).expect("optional unblockable companion");
        assert_eq!((count.min, count.max), (0, Some(1)));
    }

    #[test]
    fn multi_slot_shared_action_uses_one_coordinated_fanout() {
        let tokens = lex_line(
            "Destroy up to one target artifact, up to one target creature, and up to one target enchantment.",
            0,
        )
        .unwrap();
        assert!(
            parse_optional_companion_fanout_sentence(&tokens)
                .unwrap()
                .is_none(),
            "the two-subject companion parser must yield to multi-target fanout"
        );

        let parsed = parse_effect_sentence_lexed(&tokens).unwrap();
        let [EffectAst::Coordinated { effects, .. }] = parsed.as_slice() else {
            panic!("expected one coordinated multi-target action: {parsed:#?}");
        };
        assert_eq!(effects.len(), 3);
        for effect in effects {
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Destroy { target, .. },
                ..
            }) = effect
            else {
                panic!("expected an independently targeted destroy action: {effect:#?}");
            };
            let (_, count) = optional_target(target).expect("independent optional target slot");
            assert_eq!((count.min, count.max), (0, Some(1)));
        }
    }
}
