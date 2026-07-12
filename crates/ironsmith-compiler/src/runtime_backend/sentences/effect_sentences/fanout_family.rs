use super::super::grammar::effects::fanout_shapes as fanout_grammar;
use super::super::grammar::effects::parse_serial_damage_fanout_tokens;
use super::super::keyword_static::parse_pt_modifier;
use super::super::lexer::{OwnedLexToken, find_token_word_sequence_span};
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    is_source_reference_words, non_article_token_word_refs, parse_target_phrase, span_from_tokens,
    trim_commas,
};
use super::zone_counter_helpers::{split_until_source_leaves_tail, target_object_filter_mut};
use super::zone_handlers::collapse_leading_signed_pt_modifier_tokens;
use super::{apply_where_x_to_damage_amounts, find_verb, parse_simple_gain_ability_clause};
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, SubjectVerbActionAst, SubjectVerbEffectAst, TagKey,
    TargetAst, Verb,
};
use crate::effect::{Until, Value};
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

const TARGET_WORD: &str = "target";

fn fanout_token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn fanout_words_contain_word(tokens: &[OwnedLexToken], expected: &str) -> bool {
    tokens
        .iter()
        .any(|token| fanout_token_is_word(token, expected))
}

pub(crate) fn parse_same_name_fanout_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let reference = fanout_grammar::parse_same_name_reference_span(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "missing 'that <object>' in same-name clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    let Some(reference) = reference else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..reference.start]);
    filter_tokens.extend_from_slice(&tokens[reference.end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in same-name fanout clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let controller_shape = fanout_grammar::strip_same_controller_shape(&filter_tokens);
    let cleaned_tokens = trim_commas(&controller_shape.cleaned_tokens);
    if cleaned_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing base object filter in same-name fanout clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&cleaned_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported same-name fanout filter (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    if controller_shape.same_controller {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameControllerAsTagged,
        });
    }
    Ok(Some(filter))
}

pub(crate) fn parse_same_name_target_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let Some(shape) = fanout_grammar::parse_same_name_fanout_shape(tokens) else {
        return Ok(None);
    };
    match shape {
        fanout_grammar::SameNameFanoutShape::Damage {
            amount,
            first_target_tokens,
            filter_tokens,
        } => {
            let Some(filter) = parse_same_name_fanout_filter(filter_tokens)? else {
                return Ok(None);
            };
            let first_target = parse_target_phrase(first_target_tokens)?;
            Ok(Some(vec![
                EffectAst::subject_verb_damage(amount.clone(), first_target),
                EffectAst::subject_verb_damage_each(amount, filter),
            ]))
        }
        fanout_grammar::SameNameFanoutShape::Action {
            verb,
            first_target_tokens,
            filter_tokens,
            mentions_graveyard,
            mentions_your_graveyard,
        } => {
            let Some(filter) = parse_same_name_fanout_filter(filter_tokens)? else {
                return Ok(None);
            };
            let mut first_target = parse_target_phrase(first_target_tokens)?;
            if verb == fanout_grammar::SameNameFanoutVerb::Return
                && let Some(first_filter) = target_object_filter_mut(&mut first_target)
            {
                if first_filter.zone.is_none() {
                    first_filter.zone = filter.zone;
                    if first_filter.zone.is_none() && mentions_graveyard {
                        first_filter.zone = Some(Zone::Graveyard);
                    }
                }
                if first_filter.owner.is_none() {
                    first_filter.owner = filter.owner.clone();
                    if first_filter.owner.is_none() && mentions_your_graveyard {
                        first_filter.owner = Some(PlayerFilter::You);
                    }
                }
            }
            let first_effect = match verb {
                fanout_grammar::SameNameFanoutVerb::Destroy => {
                    EffectAst::subject_verb_destroy(first_target)
                }
                fanout_grammar::SameNameFanoutVerb::Exile if until_source_leaves => {
                    EffectAst::subject_verb_exile_until_source_leaves(first_target, false)
                }
                fanout_grammar::SameNameFanoutVerb::Exile => {
                    EffectAst::subject_verb_exile(first_target, false)
                }
                fanout_grammar::SameNameFanoutVerb::Return => {
                    EffectAst::subject_verb_return_to_hand(first_target, false)
                }
            };
            let second_effect = match verb {
                fanout_grammar::SameNameFanoutVerb::Destroy => {
                    EffectAst::subject_verb_destroy_all(filter)
                }
                fanout_grammar::SameNameFanoutVerb::Exile if until_source_leaves => {
                    EffectAst::subject_verb_exile_all_until_source_leaves(
                        TargetAst::Object(filter, None, None),
                        false,
                    )
                }
                fanout_grammar::SameNameFanoutVerb::Exile => {
                    EffectAst::subject_verb_exile_all(filter, false)
                }
                fanout_grammar::SameNameFanoutVerb::Return => {
                    EffectAst::subject_verb_return_all_to_hand(filter)
                }
            };
            Ok(Some(vec![first_effect, second_effect]))
        }
    }
}

pub(crate) fn parse_shared_color_fanout_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let reference = fanout_grammar::parse_shares_color_reference_span(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "missing 'it' in shares-color clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    let Some(reference) = reference else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..reference.start]);
    filter_tokens.extend_from_slice(&tokens[reference.end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in shared-color fanout clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported shared-color fanout filter (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SharesColorWithTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    Ok(Some(filter))
}

fn split_full_shared_color_target(target: &TargetAst) -> Option<(TargetAst, ObjectFilter)> {
    let TargetAst::Object(filter, explicit_span, extra_span) = target else {
        return None;
    };
    let has_shared_color = filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.relation == TaggedOpbjectRelation::SharesColorWithTagged);
    if !filter.other || !has_shared_color {
        return None;
    }

    let mut first_filter = filter.clone();
    first_filter.other = false;
    first_filter.tagged_constraints.retain(|constraint| {
        !matches!(
            constraint.relation,
            TaggedOpbjectRelation::SharesColorWithTagged | TaggedOpbjectRelation::IsNotTaggedObject
        )
    });

    Some((
        TargetAst::Object(first_filter, *explicit_span, *extra_span),
        filter.clone(),
    ))
}

fn parse_explicit_shared_color_gets_or_gains(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    let Some(fanout_grammar::SharedColorFanoutShape::ExplicitGetOrGain {
        verb,
        duration_tokens,
        first_target_tokens,
        filter_tokens,
        action_tokens,
    }) = fanout_grammar::parse_shared_color_fanout_shape(tokens)
    else {
        return Ok(None);
    };
    let Some(filter) = parse_shared_color_fanout_filter(filter_tokens)? else {
        return Ok(None);
    };
    let first_target = parse_target_phrase(first_target_tokens)?;

    if verb == fanout_grammar::SharedColorVerb::Get {
        let modifier_tokens = &action_tokens[1..];
        let modifier_word = modifier_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing modifier in shared-color gets clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
        let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;

        return Ok(Some(vec![
            EffectAst::subject_verb_pump(
                Value::Fixed(power),
                Value::Fixed(toughness),
                first_target,
                Until::EndOfTurn,
                None,
            ),
            EffectAst::subject_verb_pump_all(
                filter,
                Value::Fixed(power),
                Value::Fixed(toughness),
                Until::EndOfTurn,
            ),
        ]));
    }

    let mut first_clause = Vec::new();
    if let Some(duration_tokens) = duration_tokens {
        first_clause.extend_from_slice(duration_tokens);
    }
    first_clause.extend_from_slice(first_target_tokens);
    first_clause.extend_from_slice(action_tokens);
    let Some(first_effect) = parse_simple_gain_ability_clause(&first_clause)? else {
        return Ok(None);
    };
    let (abilities, duration) = match first_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    abilities,
                    duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesAll {
                    abilities,
                    duration,
                    ..
                },
            ..
        }) => (abilities, duration),
        _ => return Ok(None),
    };

    Ok(Some(vec![
        EffectAst::subject_verb_grant_abilities_to_target(
            first_target,
            abilities.clone(),
            duration.clone(),
        ),
        EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration),
    ]))
}

pub(crate) fn parse_shared_color_target_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = fanout_grammar::strip_radiance_label(tokens);
    if let Some(effects) = parse_explicit_shared_color_gets_or_gains(tokens)? {
        return Ok(Some(effects));
    }
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = fanout_grammar::parse_shared_color_fanout_shape(tokens) else {
        return Ok(None);
    };
    match shape {
        fanout_grammar::SharedColorFanoutShape::Action {
            verb,
            first_target_tokens,
            filter_tokens,
        } => {
            let Some(filter) = parse_shared_color_fanout_filter(filter_tokens)? else {
                return Ok(None);
            };
            let first_target = parse_target_phrase(first_target_tokens)?;
            let effects = match verb {
                fanout_grammar::SharedColorVerb::Destroy => vec![
                    EffectAst::subject_verb_destroy(first_target),
                    EffectAst::subject_verb_destroy_all(filter),
                ],
                fanout_grammar::SharedColorVerb::Exile => vec![
                    EffectAst::subject_verb_exile(first_target, false),
                    EffectAst::subject_verb_exile_all(filter, false),
                ],
                fanout_grammar::SharedColorVerb::Untap => vec![
                    EffectAst::subject_verb_untap(first_target),
                    EffectAst::subject_verb_untap_all(filter),
                ],
                _ => return Ok(None),
            };
            Ok(Some(effects))
        }
        fanout_grammar::SharedColorFanoutShape::Damage {
            amount,
            first_target_tokens,
            filter_tokens,
        } => {
            let Some(filter) = parse_shared_color_fanout_filter(filter_tokens)? else {
                return Ok(None);
            };
            let first_target = parse_target_phrase(first_target_tokens)?;
            Ok(Some(vec![
                EffectAst::subject_verb_damage(amount.clone(), first_target),
                EffectAst::subject_verb_damage_each(amount, filter),
            ]))
        }
        fanout_grammar::SharedColorFanoutShape::Prevent {
            amount,
            first_target_tokens,
            filter_tokens,
        } => {
            let Some(filter) = parse_shared_color_fanout_filter(filter_tokens)? else {
                return Ok(None);
            };
            let first_target = parse_target_phrase(first_target_tokens)?;
            Ok(Some(vec![
                EffectAst::subject_verb_prevent_damage(
                    amount.clone(),
                    first_target,
                    Until::EndOfTurn,
                ),
                EffectAst::subject_verb_prevent_damage_each(amount, filter, Until::EndOfTurn),
            ]))
        }
        fanout_grammar::SharedColorFanoutShape::SubjectGetOrGain {
            verb,
            subject_tokens,
            split_targets,
            action_tokens,
        } => {
            let parsed_targets = if let Ok(full_target) = parse_target_phrase(subject_tokens)
                && let Some(parts) = split_full_shared_color_target(&full_target)
            {
                Some(parts)
            } else if let Some((first_tokens, filter_tokens)) = split_targets {
                let Some(filter) = parse_shared_color_fanout_filter(filter_tokens)? else {
                    return Ok(None);
                };
                Some((parse_target_phrase(first_tokens)?, filter))
            } else {
                None
            };
            let Some((first_target, filter)) = parsed_targets else {
                return Ok(None);
            };
            if verb == fanout_grammar::SharedColorVerb::Get {
                let modifier_word = action_tokens
                    .get(1)
                    .and_then(OwnedLexToken::as_word)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "missing modifier in shared-color gets clause (clause: '{}')",
                            words_all.join(" ")
                        ))
                    })?;
                let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                        words_all.join(" ")
                    ))
                })?;
                return Ok(Some(vec![
                    EffectAst::subject_verb_pump(
                        Value::Fixed(power),
                        Value::Fixed(toughness),
                        first_target,
                        Until::EndOfTurn,
                        None,
                    ),
                    EffectAst::subject_verb_pump_all(
                        filter,
                        Value::Fixed(power),
                        Value::Fixed(toughness),
                        Until::EndOfTurn,
                    ),
                ]));
            }
            let first_effect = if split_targets.is_some() {
                let mut first_clause = subject_tokens.to_vec();
                if let Some((first_tokens, _)) = split_targets {
                    first_clause = first_tokens.to_vec();
                }
                first_clause.extend_from_slice(action_tokens);
                parse_simple_gain_ability_clause(&first_clause)?
            } else {
                parse_simple_gain_ability_clause(tokens)?
            };
            let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        abilities,
                        duration,
                        ..
                    },
                ..
            })) = first_effect
            else {
                return Ok(None);
            };
            Ok(Some(vec![
                EffectAst::subject_verb_grant_abilities_to_target(
                    first_target,
                    abilities.clone(),
                    duration.clone(),
                ),
                EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration),
            ]))
        }
        fanout_grammar::SharedColorFanoutShape::ExplicitGetOrGain { .. } => {
            parse_explicit_shared_color_gets_or_gains(tokens)
        }
    }
}

#[derive(Debug, Clone)]
enum CompoundDamagePart {
    Target(TargetAst),
    EachObject(ObjectFilter),
    EachPlayer(PlayerFilter),
}

fn target_context_for_damage_part(part: &CompoundDamagePart) -> Option<PlayerFilter> {
    match part {
        CompoundDamagePart::Target(TargetAst::Player(filter, span))
        | CompoundDamagePart::Target(TargetAst::PlayerOrPlaneswalker(filter, span)) => {
            if span.is_some() {
                Some(PlayerFilter::Target(Box::new(filter.clone())))
            } else {
                Some(filter.clone())
            }
        }
        CompoundDamagePart::EachPlayer(_) => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

fn lower_damage_part_shape(
    shape: fanout_grammar::DamagePartShape,
    player_context: Option<PlayerFilter>,
) -> Result<Option<CompoundDamagePart>, CardTextError> {
    match shape {
        fanout_grammar::DamagePartShape::EachPlayer { opponent_only } => {
            let filter = if opponent_only {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(Some(CompoundDamagePart::EachPlayer(filter)))
        }
        fanout_grammar::DamagePartShape::EachObject {
            filter_tokens,
            controller,
        } => {
            let mut filter = match parse_object_filter(&filter_tokens, false) {
                Ok(filter) => filter,
                Err(_) => return Ok(None),
            };
            if filter.controller.is_none() {
                filter.controller = controller.map(|surface| match surface {
                    fanout_grammar::ControllerSurface::TargetPlayerOrControllerOfTarget => {
                        PlayerFilter::TargetPlayerOrControllerOfTarget
                    }
                    fanout_grammar::ControllerSurface::ContextualTargetPlayer => {
                        player_context.unwrap_or_else(PlayerFilter::target_player)
                    }
                    fanout_grammar::ControllerSurface::Opponent => PlayerFilter::Opponent,
                    fanout_grammar::ControllerSurface::You => PlayerFilter::You,
                });
            }
            Ok(Some(CompoundDamagePart::EachObject(filter)))
        }
        fanout_grammar::DamagePartShape::TargetYou(tokens) => Ok(Some(CompoundDamagePart::Target(
            TargetAst::Player(PlayerFilter::You, span_from_tokens(&tokens)),
        ))),
        fanout_grammar::DamagePartShape::TargetOpponent(tokens) => {
            Ok(Some(CompoundDamagePart::Target(TargetAst::Player(
                PlayerFilter::Opponent,
                span_from_tokens(&tokens),
            ))))
        }
        fanout_grammar::DamagePartShape::TargetTokens(tokens) => Ok(Some(
            CompoundDamagePart::Target(parse_target_phrase(&tokens)?),
        )),
    }
}

fn parse_each_damage_part(
    tokens: &[OwnedLexToken],
    player_context: Option<PlayerFilter>,
) -> Result<Option<CompoundDamagePart>, CardTextError> {
    let Some(shape) = fanout_grammar::parse_damage_part_shape(tokens, true) else {
        return Ok(None);
    };
    lower_damage_part_shape(shape, player_context)
}

fn parse_damage_part(
    tokens: &[OwnedLexToken],
    player_context: Option<PlayerFilter>,
) -> Result<Option<CompoundDamagePart>, CardTextError> {
    let Some(shape) = fanout_grammar::parse_damage_part_shape(tokens, false) else {
        return Ok(None);
    };
    lower_damage_part_shape(shape, player_context)
}

fn damage_player_iteration_effect(filter: PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    match filter {
        PlayerFilter::Opponent => EffectAst::ForEachOpponent { effects },
        PlayerFilter::Any => EffectAst::ForEachPlayer { effects },
        other => EffectAst::ForEachPlayersFiltered {
            filter: other,
            effects,
        },
    }
}

fn compound_damage_part_to_effect(part: CompoundDamagePart, amount: Value) -> EffectAst {
    match part {
        CompoundDamagePart::Target(target) => EffectAst::subject_verb_damage(amount, target),
        CompoundDamagePart::EachObject(filter) => {
            EffectAst::subject_verb_damage_each(amount, filter)
        }
        CompoundDamagePart::EachPlayer(filter) => damage_player_iteration_effect(
            filter,
            vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        ),
    }
}

fn compound_damage_effects(
    amount: Value,
    left: CompoundDamagePart,
    right: CompoundDamagePart,
) -> Vec<EffectAst> {
    match left {
        CompoundDamagePart::EachPlayer(filter) => {
            let mut nested = vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )];
            nested.push(compound_damage_part_to_effect(right, amount));
            vec![damage_player_iteration_effect(filter, nested)]
        }
        other => vec![
            compound_damage_part_to_effect(other, amount.clone()),
            compound_damage_part_to_effect(right, amount),
        ],
    }
}

pub(crate) fn parse_compound_damage_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(serial) = parse_serial_damage_fanout_tokens(tokens)? {
        let source_words = non_article_token_word_refs(&serial.source);
        if !serial.source.is_empty() && !is_source_reference_words(&source_words) {
            return Ok(None);
        }
        let mut effects = Vec::with_capacity(serial.parts.len());
        for part in serial.parts {
            let target_tokens = trim_commas(&part.target_tokens);
            if target_tokens.is_empty() {
                return Ok(None);
            }
            effects.push(EffectAst::subject_verb_damage(
                part.amount,
                parse_target_phrase(&target_tokens)?,
            ));
        }
        return Ok(Some(effects));
    }

    let Some(shape) = fanout_grammar::parse_compound_damage_shape(tokens) else {
        return Ok(None);
    };
    let Some(left) = parse_damage_part(&shape.left_tokens, None)? else {
        return Ok(None);
    };
    let right_context = target_context_for_damage_part(&left);
    let Some(right) = parse_each_damage_part(&shape.right_tokens, right_context)? else {
        return Ok(None);
    };

    let mut effects = compound_damage_effects(shape.amount, left, right);
    apply_where_x_to_damage_amounts(tokens, &mut effects)?;
    Ok(Some(effects))
}

pub(crate) fn parse_same_name_gets_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    if verb != Verb::Get || verb_idx == 0 || verb_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..verb_idx];
    let Some((and_idx, _and_end)) =
        find_token_word_sequence_span(subject_tokens, &["and", "all", "other"])
    else {
        return Ok(None);
    };
    if and_idx == 0 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
    if first_target_tokens.is_empty()
        || !fanout_words_contain_word(&first_target_tokens, TARGET_WORD)
    {
        return Ok(None);
    }
    let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
    if second_clause_tokens.is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let modifier_tokens = &tokens[verb_idx + 1..];
    let collapsed_modifier_tokens = collapse_leading_signed_pt_modifier_tokens(modifier_tokens)
        .unwrap_or_else(|| modifier_tokens.to_vec());
    let modifier_word = collapsed_modifier_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing modifier in same-name gets clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in same-name gets clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    let first_target = parse_target_phrase(&first_target_tokens)?;

    Ok(Some(vec![
        EffectAst::subject_verb_pump(
            Value::Fixed(power),
            Value::Fixed(toughness),
            first_target,
            Until::EndOfTurn,
            None,
        ),
        EffectAst::subject_verb_pump_all(
            filter,
            Value::Fixed(power),
            Value::Fixed(toughness),
            Until::EndOfTurn,
        ),
    ]))
}
