use super::super::clause_support::parse_triggered_line_lexed;
use super::super::grammar::effects::{
    clause_primitive_shapes as clause_shapes, parse_unless_pays_shape_tokens,
    split_change_target_clause_lexed, split_change_target_unless_clause_lexed,
    split_choose_new_targets_clause_lexed,
};
use super::super::grammar::primitives as grammar;
use super::super::grammar::trigger_surface;
use super::super::lexer::LexedClause;
use super::super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    parse_additional_land_plays_clause, parse_cast_or_play_tagged_clause,
    parse_cast_spells_as_though_they_had_flash_clause,
    parse_unsupported_play_cast_permission_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::util::{
    is_article, parse_subject, parse_target_phrase, parse_value_expr_words, span_from_tokens,
};
use super::parse_restriction_duration;
use super::sentence_helpers::*;
use super::subject_verb_primitives::SubjectVerbPrimitiveClause;
#[allow(unused_imports)]
use crate::cards::builders::{
    COPIED_STACK_OBJECT_TAG, CardTextError, ClashOpponentAst, EffectAst, GrantedAbilityAst, IT_TAG,
    LineAst, OwnedLexToken, PlayerAst, PredicateAst, ReferenceImports, RetargetModeAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::effect::{ChoiceCount, Value};
use crate::mana::ManaSymbol;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

pub(crate) type ClausePrimitiveParser =
    fn(&[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError>;

pub(crate) struct ClausePrimitive {
    pub(crate) parser: ClausePrimitiveParser,
}

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

pub(crate) fn parse_retarget_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_choose_new_targets_clause(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_change_target_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}

pub(crate) fn parse_copy_targets_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_copy_targets_shape(tokens) else {
        return Ok(None);
    };
    if shape.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target after copy-target clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }
    let fixed_filter = parse_object_filter(shape.target_tokens, false)?;
    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        TargetAst::Tagged(
            TagKey::from(COPIED_STACK_OBJECT_TAG),
            LexedClause::new(tokens).span(),
        ),
        RetargetModeAst::OneToFixed {
            target: TargetAst::Object(fixed_filter, None, None),
        },
        false,
    )))
}

pub(crate) fn parse_choose_new_targets_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(split) = split_choose_new_targets_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.reference_target {
        let reference_tag = match clause_shapes::parse_retarget_reference_shape(split.target_tokens)
        {
            Some(clause_shapes::RetargetReferenceShape::Copy) => COPIED_STACK_OBJECT_TAG,
            Some(clause_shapes::RetargetReferenceShape::Other) => IT_TAG,
            None => {
                return Err(CardTextError::ParseError(format!(
                    "missing typed retarget reference shape (clause: '{}')",
                    LexedClause::new(tokens).text()
                )));
            }
        };
        let target = TargetAst::Tagged(
            TagKey::from(reference_tag),
            span_from_tokens(split.target_tokens),
        );
        return Ok(Some(EffectAst::subject_verb_retarget_stack_object(
            PlayerAst::Implicit,
            target,
            RetargetModeAst::All,
            false,
        )));
    }
    let tail_tokens = split.target_tokens;
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing choose-new-targets target".to_string(),
        ));
    }

    let filter = parse_stack_retarget_filter(tail_tokens)?;

    let mut target = TargetAst::Object(
        filter,
        if split.explicit_target {
            span_from_tokens(tail_tokens)
        } else {
            None
        },
        None,
    );
    if let Some(count) = split.count {
        target = TargetAst::WithCount(Box::new(target), count);
    }

    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        target,
        RetargetModeAst::All,
        false,
    )))
}

pub(crate) fn parse_change_target_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.first_word() != Some("change") {
        return Ok(None);
    }

    if let Some((main_tokens, unless_tokens)) = split_change_target_unless_clause_lexed(tokens) {
        let Some(inner) = parse_change_target_clause_inner(&main_tokens)? else {
            return Ok(None);
        };
        let (player, cost) = parse_unless_pays_clause(&unless_tokens)?;
        return Ok(Some(EffectAst::UnlessPays {
            effects: vec![inner],
            player,
            cost,
        }));
    }

    parse_change_target_clause_inner(tokens)
}

pub(crate) fn parse_change_target_clause_inner(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(split) = split_change_target_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target after change-the-target clause".to_string(),
        ));
    }

    let tail_tokens = split.target_tokens;
    let mut filter = parse_stack_retarget_filter(&tail_tokens)?;

    for constraint in clause_shapes::parse_retarget_constraint_shapes(&tail_tokens) {
        filter = apply_retarget_constraint(filter, constraint);
    }

    let target = TargetAst::Object(filter, span_from_tokens(tokens), None);

    let mode = if split.fixed_to_source {
        RetargetModeAst::OneToFixed {
            target: TargetAst::Source(span_from_tokens(tokens)),
        }
    } else {
        RetargetModeAst::All
    };

    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        target,
        mode,
        true,
    )))
}

fn apply_retarget_constraint(
    filter: ObjectFilter,
    constraint: clause_shapes::RetargetConstraintShape,
) -> ObjectFilter {
    match constraint {
        clause_shapes::RetargetConstraintShape::SingleTarget => filter.target_count_exact(1),
        clause_shapes::RetargetConstraintShape::SingleCreatureTarget => filter
            .targeting_only_object(ObjectFilter::creature())
            .target_count_exact(1),
        clause_shapes::RetargetConstraintShape::SourceOnlyTarget => filter
            .targeting_only_object(ObjectFilter::source())
            .target_count_exact(1),
        clause_shapes::RetargetConstraintShape::YouOnlyTarget => filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1),
        clause_shapes::RetargetConstraintShape::AnyPlayerTarget => filter
            .targeting_only_player(PlayerFilter::Any)
            .target_count_exact(1),
    }
}

pub(crate) fn parse_unless_pays_clause(
    tokens: &[OwnedLexToken],
) -> Result<(PlayerAst, crate::cost::TotalCost), CardTextError> {
    let shape = parse_unless_pays_shape_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing typed unless-payment shape (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    let player = match parse_subject(shape.player_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };
    let cost = crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(shape.payment_tokens)?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported unless-payment clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;

    Ok((player, cost))
}

pub(crate) fn parse_stack_retarget_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let Some(shape) = clause_shapes::parse_stack_retarget_filter_shape(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported retarget target clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    };
    let mut filter = match shape.kind {
        clause_shapes::StackRetargetFilterKind::ActivatedAbility => {
            ObjectFilter::activated_ability()
        }
        clause_shapes::StackRetargetFilterKind::SpellOrAbility => ObjectFilter::spell_or_ability(),
        clause_shapes::StackRetargetFilterKind::Ability => ObjectFilter::ability(),
        clause_shapes::StackRetargetFilterKind::InstantOrSorcery => {
            ObjectFilter::instant_or_sorcery()
        }
        clause_shapes::StackRetargetFilterKind::Spell => ObjectFilter::spell(),
    };
    filter.other = shape.other;

    Ok(filter)
}

pub(crate) fn run_clause_primitives(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    const PRIMITIVES: &[ClausePrimitive] = &[
        ClausePrimitive {
            parser: parse_choose_card_name_clause,
        },
        ClausePrimitive {
            parser: parse_repeat_this_process_clause,
        },
        ClausePrimitive {
            parser: parse_dont_lose_this_mana_as_steps_and_phases_end_clause,
        },
        ClausePrimitive {
            parser: parse_retarget_clause,
        },
        ClausePrimitive {
            parser: parse_copy_targets_clause,
        },
        ClausePrimitive {
            parser: parse_copy_spell_clause,
        },
        ClausePrimitive {
            parser: parse_win_the_game_clause,
        },
        ClausePrimitive {
            parser: parse_deal_damage_equal_to_power_clause,
        },
        ClausePrimitive {
            parser: parse_fight_clause,
        },
        ClausePrimitive {
            parser: parse_clash_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_target_players_clause,
        },
        ClausePrimitive {
            parser: parse_each_player_exiles_hand_face_down_and_draws_clause,
        },
        ClausePrimitive {
            parser: parse_each_player_return_with_additional_counter_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_opponent_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_player_clause,
        },
        ClausePrimitive {
            parser: parse_double_counters_clause,
        },
        ClausePrimitive {
            parser: parse_distribute_counters_clause,
        },
        ClausePrimitive {
            parser: parse_until_end_of_turn_may_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_until_your_next_turn_may_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_additional_land_plays_clause,
        },
        ClausePrimitive {
            parser: parse_cast_spells_as_though_they_had_flash_clause,
        },
        ClausePrimitive {
            parser: parse_unsupported_play_cast_permission_clause,
        },
        ClausePrimitive {
            parser: parse_cast_or_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_next_damage_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_all_damage_clause,
        },
        ClausePrimitive {
            parser: parse_can_attack_as_though_no_defender_clause,
        },
        ClausePrimitive {
            parser: parse_can_block_additional_creature_this_turn_clause,
        },
        ClausePrimitive {
            parser: parse_attack_or_block_this_turn_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_attack_this_turn_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_must_be_blocked_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_must_block_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_until_duration_triggered_clause,
        },
        ClausePrimitive {
            parser: parse_keyword_mechanic_clause,
        },
        ClausePrimitive {
            parser: parse_connive_clause,
        },
        ClausePrimitive {
            parser: parse_choose_target_and_verb_clause,
        },
        ClausePrimitive {
            parser: parse_verb_first_clause,
        },
    ];

    for primitive in PRIMITIVES {
        if let Some(effect) = (primitive.parser)(tokens)? {
            return Ok(Some(effect));
        }
    }
    Ok(None)
}

pub(crate) fn parse_choose_card_name_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_choose_card_name_shape(tokens) else {
        return Ok(None);
    };
    let filter = shape
        .filter_tokens
        .map(|filter_tokens| {
            parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported choose-card-name filter (clause: '{}')",
                    LexedClause::new(tokens).text()
                ))
            })
        })
        .transpose()?;

    Ok(Some(EffectAst::subject_verb_choose_card_name(
        shape.player,
        filter,
        TagKey::from(CHOSEN_NAME_TAG),
    )))
}

pub(crate) fn parse_repeat_this_process_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    Ok(
        clause_shapes::parse_repeat_process_shape(tokens).map(|shape| match shape {
            clause_shapes::RepeatProcessShape::Required => EffectAst::RepeatThisProcess,
            clause_shapes::RepeatProcessShape::Once => EffectAst::RepeatThisProcessOnce,
            clause_shapes::RepeatProcessShape::May => EffectAst::RepeatThisProcessMay,
        }),
    )
}

pub(crate) fn parse_dont_lose_this_mana_as_steps_and_phases_end_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if clause_shapes::is_dont_lose_mana_between_steps_shape(tokens) {
        return Ok(Some(
            EffectAst::subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_each_player_exiles_hand_face_down_and_draws_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if !clause_shapes::is_each_player_exiles_hand_face_down_and_draws_shape(tokens) {
        return Ok(None);
    }

    let mut hand_cards = ObjectFilter::default();
    hand_cards.zone = Some(Zone::Hand);
    hand_cards.owner = Some(PlayerFilter::IteratedPlayer);

    Ok(Some(EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_exile(TargetAst::Object(hand_cards, None, None), true),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::That,
                SubjectVerbActionAst::Draw {
                    count: Value::Fixed(7),
                },
            ),
        ],
    }))
}

pub(crate) fn parse_each_player_return_with_additional_counter_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(mut effects) = parse_sentence_each_player_return_with_additional_counter(
        SubjectVerbPrimitiveClause::new(tokens),
    )?
    else {
        return Ok(None);
    };

    Ok(Some(if effects.len() == 1 {
        effects.remove(0)
    } else {
        EffectAst::Sequence { effects }
    }))
}

pub(crate) fn parse_attack_or_block_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some(shape) = clause_shapes::parse_combat_requirement_shape(tokens) else {
        return Ok(None);
    };
    if shape.kind != clause_shapes::CombatRequirementKind::AttackOrBlock {
        return Ok(None);
    }
    let subject_clause = LexedClause::new(shape.subject_tokens).trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span())
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };
    let abilities = vec![GrantedAbilityAst::MustAttack, GrantedAbilityAst::MustBlock];

    if subject_clause.is_empty() || starts_with_target_indicator(subject_clause.tokens()) {
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target,
            abilities,
            Until::EndOfTurn,
        )));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker/blocker subject in attacks-or-blocks-if-able clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter,
        abilities,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_attack_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some(shape) = clause_shapes::parse_combat_requirement_shape(tokens) else {
        return Ok(None);
    };
    if shape.kind != clause_shapes::CombatRequirementKind::Attack {
        return Ok(None);
    }
    let subject_clause = LexedClause::new(shape.subject_tokens).trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span())
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };
    let ability = GrantedAbilityAst::MustAttack;

    if subject_clause.is_empty() || starts_with_target_indicator(subject_clause.tokens()) {
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target,
            vec![ability],
            Until::EndOfTurn,
        )));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in attacks-if-able clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter,
        vec![ability],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_must_be_blocked_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some(shape) = clause_shapes::parse_combat_requirement_shape(tokens) else {
        return Ok(None);
    };
    if shape.kind != clause_shapes::CombatRequirementKind::MustBeBlocked {
        return Ok(None);
    }
    let subject_clause = LexedClause::new(shape.subject_tokens).trimmed();
    if subject_clause.is_empty() {
        return Ok(None);
    }
    if starts_with_target_indicator(subject_clause.tokens()) {
        let attacker_target = parse_target_phrase(subject_clause.tokens())?;
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(attacker_target),
                EffectAst::subject_verb_cant(
                    crate::effect::Restriction::must_be_blocked(ObjectFilter::tagged(IT_TAG)),
                    Until::EndOfTurn,
                    None,
                ),
            ],
        }));
    }

    let attacker_target = parse_target_phrase(subject_clause.tokens())?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in must-be-blocked clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_cant(
        crate::effect::Restriction::must_be_blocked(attacker_filter),
        Until::EndOfTurn,
        None,
    )))
}

pub(crate) fn parse_must_block_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    let Some(shape) = clause_shapes::parse_must_block_shape(tokens) else {
        return Ok(None);
    };
    match shape {
        clause_shapes::MustBlockShape::SubjectThisTurn { subject_tokens } => {
            let subject_clause = LexedClause::new(subject_tokens).trimmed();
            let target = parse_target_phrase(subject_clause.tokens())?;
            let ability = GrantedAbilityAst::MustBlock;
            if starts_with_target_indicator(subject_clause.tokens()) {
                return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
                    target,
                    vec![ability],
                    Until::EndOfTurn,
                )));
            }
            let filter = target_ast_to_object_filter(target).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported blocker subject in blocks-if-able clause (clause: '{}')",
                    clause_text
                ))
            })?;
            Ok(Some(EffectAst::subject_verb_grant_abilities_all(
                filter,
                vec![ability],
                Until::EndOfTurn,
            )))
        }
        clause_shapes::MustBlockShape::AllCreatures {
            attacker_and_duration_tokens,
        } => {
            let (duration, attacker_tokens) = if let Some((duration, remainder)) =
                parse_restriction_duration(attacker_and_duration_tokens)?
            {
                (duration, remainder)
            } else {
                (Until::EndOfTurn, attacker_and_duration_tokens.to_vec())
            };
            let attacker_clause = LexedClause::new(&attacker_tokens).trimmed();
            if attacker_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attacker in must-block clause (clause: '{}')",
                    clause_text
                )));
            }
            let attacker_target = parse_target_phrase(attacker_clause.tokens())?;
            if starts_with_target_indicator(attacker_clause.tokens()) {
                return Ok(Some(EffectAst::Sequence {
                    effects: vec![
                        EffectAst::subject_verb_target_only(attacker_target),
                        EffectAst::subject_verb_cant(
                            crate::effect::Restriction::must_block_specific_attacker(
                                ObjectFilter::creature(),
                                ObjectFilter::tagged(IT_TAG),
                            ),
                            duration,
                            None,
                        ),
                    ],
                }));
            }
            let attacker_filter =
                target_ast_to_object_filter(attacker_target).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported attacker target in must-block clause (clause: '{}')",
                        clause_text
                    ))
                })?;
            Ok(Some(EffectAst::subject_verb_cant(
                crate::effect::Restriction::must_block_specific_attacker(
                    ObjectFilter::creature(),
                    attacker_filter,
                ),
                duration,
                None,
            )))
        }
        clause_shapes::MustBlockShape::SubjectAgainstAttacker {
            subject_tokens,
            attacker_and_duration_tokens,
        } => {
            let subject_clause = LexedClause::new(subject_tokens).trimmed();
            let blockers_filter = parse_subject_object_filter(subject_clause.tokens())?
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported blocker subject in must-block clause (clause: '{}')",
                        clause_text
                    ))
                })?;
            let (duration, attacker_tokens) = if let Some((duration, remainder)) =
                parse_restriction_duration(attacker_and_duration_tokens)?
            {
                (duration, remainder)
            } else {
                (Until::EndOfTurn, attacker_and_duration_tokens.to_vec())
            };
            let attacker_clause = LexedClause::new(&attacker_tokens).trimmed();
            if attacker_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attacker in must-block clause (clause: '{}')",
                    clause_text
                )));
            }
            let attacker_filter = if clause_shapes::is_it_reference_shape(attacker_clause.tokens())
            {
                ObjectFilter::tagged("triggering")
            } else {
                let attacker_target = parse_target_phrase(attacker_clause.tokens())?;
                target_ast_to_object_filter(attacker_target).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported attacker target in must-block clause (clause: '{}')",
                        clause_text
                    ))
                })?
            };
            if starts_with_target_indicator(subject_clause.tokens()) {
                let blocker_target = parse_target_phrase(subject_clause.tokens())?;
                return Ok(Some(EffectAst::Sequence {
                    effects: vec![
                        EffectAst::subject_verb_target_only(blocker_target),
                        EffectAst::subject_verb_cant(
                            crate::effect::Restriction::must_block_specific_attacker(
                                ObjectFilter::tagged(IT_TAG),
                                attacker_filter,
                            ),
                            duration,
                            None,
                        ),
                    ],
                }));
            }
            Ok(Some(EffectAst::subject_verb_cant(
                crate::effect::Restriction::must_block_specific_attacker(
                    blockers_filter,
                    attacker_filter,
                ),
                duration,
                None,
            )))
        }
    }
}

pub(crate) fn parse_until_duration_triggered_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause_shapes::parse_duration_trigger_prefix_shape(tokens).is_none() {
        return Ok(None);
    }

    let Some((duration, trigger_tokens)) = parse_restriction_duration(tokens)? else {
        return Ok(None);
    };
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing trigger after duration clause (clause: '{}')",
            clause.text()
        )));
    }

    let trigger_clause = LexedClause::new(&trigger_tokens);
    let trigger_words = trigger_clause.word_refs();
    if clause_shapes::parse_trigger_clause_intro_shape(trigger_clause.tokens()).is_none() {
        return Ok(None);
    }

    let (trigger, effects, max_triggers_per_turn) =
        match parse_triggered_line_lexed(&trigger_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => (trigger, effects, max_triggers_per_turn),
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported duration-triggered clause (clause: '{}')",
                    clause.text()
                )));
            }
        };

    let trigger_text = trigger_words.join(" ");
    let granted = GrantedAbilityAst::ParsedObjectAbility {
        ability: parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(trigger_text.clone()),
            trigger_surface::parse_trigger_frequency_condition_tokens(
                &trigger_tokens,
                max_triggers_per_turn,
            ),
            None,
            ReferenceImports::default(),
        ),
        display: trigger_text,
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        TargetAst::Source(span_from_tokens(tokens)),
        vec![granted],
        duration,
    )))
}

pub(crate) fn is_damage_source_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _)
    )
}

pub(crate) fn parse_deal_damage_equal_to_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_power_damage_shape(tokens)? else {
        return Ok(None);
    };
    let source = if shape.source_is_tagged {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(shape.source_tokens))
    } else {
        parse_target_phrase(shape.source_tokens)?
    };
    if !is_damage_source_target(&source) {
        return Err(CardTextError::ParseError(format!(
            "unsupported damage source target phrase (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }
    match shape.target {
        clause_shapes::PowerDamageTargetShape::EachPlayer => Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_damage_with_source(
                source,
                shape.amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        })),
        clause_shapes::PowerDamageTargetShape::EachOpponent => {
            Ok(Some(EffectAst::ForEachOpponent {
                effects: vec![EffectAst::subject_verb_damage_with_source(
                    source,
                    shape.amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }))
        }
        clause_shapes::PowerDamageTargetShape::Source => Ok(Some(
            EffectAst::subject_verb_damage_with_source(source.clone(), shape.amount, source),
        )),
        clause_shapes::PowerDamageTargetShape::Tokens(target_tokens) => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(Some(EffectAst::subject_verb_damage_with_source(
                source,
                shape.amount,
                target,
            )))
        }
    }
}

pub(crate) fn parse_fight_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_fight_shape(tokens) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    if shape.right_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_text
        )));
    }

    let creature1 = if let Some(left_tokens) = shape.left_tokens {
        if let Some(filter) = parse_for_each_object_subject(left_tokens)? {
            let creature2 = parse_target_phrase(shape.right_tokens)?;
            if matches!(
                creature2,
                TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
            ) {
                return Err(CardTextError::ParseError(format!(
                    "fight target must be a creature (clause: '{}')",
                    clause_text
                )));
            }
            return Ok(Some(EffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::subject_verb_fight_iterated(creature2)],
            }));
        }
        parse_target_phrase(left_tokens)?
    } else {
        TargetAst::Source(None)
    };
    let creature2 = if shape.right_is_tagged_other {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(shape.right_tokens))
    } else {
        parse_target_phrase(shape.right_tokens)?
    };

    for target in [&creature1, &creature2] {
        if matches!(
            target,
            TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
        ) {
            return Err(CardTextError::ParseError(format!(
                "fight target must be a creature (clause: '{}')",
                clause_text
            )));
        }
    }

    Ok(Some(EffectAst::subject_verb_fight(creature1, creature2)))
}

pub(crate) fn parse_clash_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    Ok(clause_shapes::parse_clash_shape(tokens).map(EffectAst::subject_verb_clash))
}
