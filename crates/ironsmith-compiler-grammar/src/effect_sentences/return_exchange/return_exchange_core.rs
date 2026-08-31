use super::*;

pub fn parse_exchange(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    use crate::grammar::effects::{
        ExchangeClauseShape, ExchangeSharedTypeShape, ExchangeValueKindShape,
        ExchangeValueOperandShape,
    };

    fn shared_type(shape: Option<ExchangeSharedTypeShape>) -> Option<SharedTypeConstraintAst> {
        shape.map(|shape| match shape {
            ExchangeSharedTypeShape::PermanentType => SharedTypeConstraintAst::PermanentType,
            ExchangeSharedTypeShape::CardType => SharedTypeConstraintAst::CardType,
        })
    }

    fn value_operand(
        shape: ExchangeValueOperandShape<'_>,
    ) -> Result<ExchangeValueAst, CardTextError> {
        match shape {
            ExchangeValueOperandShape::LifeTotal(player) => Ok(ExchangeValueAst::LifeTotal(player)),
            ExchangeValueOperandShape::SourceStat {
                source_tokens,
                kind,
            } => Ok(ExchangeValueAst::Stat {
                target: TargetAst::Source(span_from_tokens(source_tokens)),
                kind: match kind {
                    ExchangeValueKindShape::Power => ExchangeValueKindAst::Power,
                    ExchangeValueKindShape::Toughness => ExchangeValueKindAst::Toughness,
                },
            }),
            ExchangeValueOperandShape::TargetStat {
                target_tokens,
                kind,
            } => Ok(ExchangeValueAst::Stat {
                target: parse_target_phrase(target_tokens)?,
                kind: match kind {
                    ExchangeValueKindShape::Power => ExchangeValueKindAst::Power,
                    ExchangeValueKindShape::Toughness => ExchangeValueKindAst::Toughness,
                },
            }),
        }
    }

    let clause_text = crate::lexer::token_word_refs(tokens).join(" ");
    let shape = crate::grammar::effects::parse_exchange_clause_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{clause_text}')"
        ))
    })?;
    match shape {
        ExchangeClauseShape::LifeTotalsOnly => match subject {
            Some(SubjectAst::Player(PlayerAst::Target)) => Ok(
                EffectAst::subject_verb_exchange_life_totals(PlayerAst::Target, PlayerAst::Target),
            ),
            _ => Err(CardTextError::ParseError(format!(
                "unsupported life-total exchange clause (clause: '{clause_text}')"
            ))),
        },
        ExchangeClauseShape::LifeTotalsWith(player2) => {
            let player1 = match subject {
                Some(SubjectAst::Player(player)) => player,
                _ => PlayerAst::You,
            };
            Ok(EffectAst::subject_verb_exchange_life_totals(
                player1, player2,
            ))
        }
        ExchangeClauseShape::TextBoxes { target_tokens } => {
            let target = parse_target_phrase(target_tokens).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported text-box exchange target (clause: '{clause_text}')"
                ))
            })?;
            Ok(EffectAst::subject_verb_exchange_text_boxes(target))
        }
        ExchangeClauseShape::Zones {
            player,
            zone1,
            zone2,
        } => Ok(EffectAst::subject_verb_exchange_zones(player, zone1, zone2)),
        ExchangeClauseShape::Values { tokens } => {
            let (duration, remainder) =
                if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
                    (duration, remainder)
                } else {
                    (Until::Forever, trim_commas(tokens).to_vec())
                };
            let (left, right) = crate::grammar::effects::parse_exchange_value_operands(&remainder)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported exchange value operands (clause: '{clause_text}')"
                    ))
                })?;
            Ok(EffectAst::subject_verb_exchange_values(
                value_operand(left)?,
                value_operand(right)?,
                duration,
            ))
        }
        ExchangeClauseShape::Control(control) => {
            if control.invalid_shared_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported exchange share-type clause (clause: '{clause_text}')"
                )));
            }
            let constraint = shared_type(control.shared_type);
            if let Some((left_tokens, right_tokens)) = control.heterogeneous {
                let left_target =
                    crate::grammar::primitives::probe_shape(parse_target_phrase(left_tokens));
                let right_target =
                    crate::grammar::primitives::probe_shape(parse_target_phrase(right_tokens));
                if let (Some(permanent1), Some(permanent2)) = (left_target, right_target) {
                    return Ok(EffectAst::subject_verb_exchange_control_heterogeneous(
                        permanent1, permanent2, constraint,
                    ));
                }
            }
            if control.filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(
                    "missing exchange target filter".to_string(),
                ));
            }
            let controller_set =
                crate::grammar::targets::parse_target_controller_set_suffix(control.filter_tokens);
            let mut filter = parse_object_filter(&controller_set.core_tokens, false)?;
            match controller_set.constraint {
                crate::grammar::targets::TargetControllerSetConstraint::None => {}
                crate::grammar::targets::TargetControllerSetConstraint::SameController => {
                    filter.target_set_same_controller = true;
                }
                crate::grammar::targets::TargetControllerSetConstraint::DifferentControllers => {
                    filter.target_set_different_controllers = true;
                }
            }
            Ok(EffectAst::subject_verb_exchange_control(
                filter,
                control.count,
                constraint,
            ))
        }
    }
}
