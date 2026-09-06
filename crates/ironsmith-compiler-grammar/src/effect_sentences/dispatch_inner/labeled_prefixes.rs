fn parse_player_villainous_choice_mode_program(
    program: crate::grammar::semantic_lowering::VillainousChoiceModeProgram<'_>,
) -> Result<Vec<EffectAst>, CardTextError> {
    match program {
        crate::grammar::semantic_lowering::VillainousChoiceModeProgram::Direct(tokens) => {
            if tokens.len() >= 2 && tokens[0].is_word("you") && tokens[1].is_word("create") {
                return crate::effect_sentences::parse_create(
                    &tokens[1..],
                    Some(
                        crate::grammar::shared_util::reference_shapes::SubjectAst::Player(
                            crate::cards::builders::PlayerAst::You,
                        ),
                    ),
                )
                .map(|effect| vec![effect]);
            }
            parse_effect_sentence_lexed(tokens)
        }
        crate::grammar::semantic_lowering::VillainousChoiceModeProgram::SharedSubjectPair(pair) => {
            let parse_action = |action_tokens: &[OwnedLexToken]| {
                let mut clause = Vec::with_capacity(
                    pair.subject_tokens
                        .len()
                        .saturating_add(action_tokens.len()),
                );
                clause.extend_from_slice(pair.subject_tokens);
                clause.extend_from_slice(action_tokens);
                parse_effect_sentence_lexed(&clause)
            };
            let mut effects = parse_action(pair.first_action_tokens)?;
            effects.extend(parse_action(pair.second_action_tokens)?);
            Ok(effects)
        }
    }
}

fn parse_player_villainous_choice_statement(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let shape =
        crate::grammar::semantic_lowering::parse_villainous_choice_player_statement_tokens(tokens);
    let Some(shape) = shape else {
        return Ok(None);
    };
    let first_mode_effects = parse_player_villainous_choice_mode_program(shape.first_mode_program)?;
    let second_mode_effects =
        parse_player_villainous_choice_mode_program(shape.second_mode_program)?;
    let (player, player_surface) = match shape.iteration {
        crate::grammar::semantic_lowering::VillainousChoicePlayerIteration::EachOpponent => {
            (PlayerFilter::IteratedPlayer, "that player")
        }
        crate::grammar::semantic_lowering::VillainousChoicePlayerIteration::TargetOpponent => {
            (PlayerFilter::target_opponent(), "target opponent")
        }
    };
    let choice = EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice {
        player,
        player_surface: Some(player_surface.to_string()),
        modes: vec![
            crate::cards::builders::ChooseOneModeAst {
                description: render_token_slice(shape.first_mode_tokens),
                effects: first_mode_effects,
            },
            crate::cards::builders::ChooseOneModeAst {
                description: render_token_slice(shape.second_mode_tokens),
                effects: second_mode_effects,
            },
        ],
    });
    Ok(Some(match shape.iteration {
        crate::grammar::semantic_lowering::VillainousChoicePlayerIteration::EachOpponent => {
            let body = if let Some(count) = shape.minimum_life_lost_this_turn {
                vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: PredicateAst::ValueComparison {
                        left: Value::LifeLostThisTurn(PlayerFilter::IteratedPlayer),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(count as i32),
                    },
                    if_true: vec![choice],
                    if_false: Vec::new(),
                })]
            } else {
                vec![choice]
            };
            vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects: body })]
        }
        crate::grammar::semantic_lowering::VillainousChoicePlayerIteration::TargetOpponent => {
            vec![
                EffectAst::subject_verb_target_only(TargetAst::Player(
                    PlayerFilter::target_opponent(),
                    Some(crate::TextSpan::synthetic()),
                )),
                choice,
            ]
        }
    }))
}

pub fn parse_effect_sentence_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_sentence_inner_lexed_unstacked(tokens)
}

#[path = "labeled_prefixes/sentence_prelude_readings.rs"]
mod sentence_prelude_readings;

#[path = "labeled_prefixes/sentence_fallback_readings.rs"]
mod sentence_fallback_readings;

fn parse_effect_sentence_inner_lexed_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let dispatch_shape = effect_grammar::labeled_dispatch::parse_labeled_dispatch_shape(tokens);
    let input = sentence_prelude_readings::SentencePrelude {
        tokens,
        dispatch_shape: &dispatch_shape,
        read_by_cache: Default::default(),
    };
    match sentence_prelude_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    if let Some(diag) = super::sentence_unsupported::diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }
    let input = sentence_fallback_readings::SentenceFallback {
        tokens,
        dispatch_shape: &dispatch_shape,
    };
    match sentence_fallback_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let (_, effects) = super::sentence_registry::run_sentence_parse_rules_lexed(tokens)?;
    Ok(effects)
}

fn lower_matching_spell_cost_reduction_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let shape =
        effect_grammar::labeled_dispatch::parse_matching_spell_cost_reduction_shape(tokens)?;
    let mut reduction = shape.reduction;
    if let Some(where_tokens) = shape.where_value_tokens
        && let Some(where_value) = parse_value_binding_clause(where_tokens)
    {
        reduction = where_value;
    }

    if let Some(mana_reduction) = shape.next_spell_mana_reduction {
        Some(EffectAst::subject_verb_reduce_next_spell_cost_this_turn(
            shape.player,
            shape.filter,
            mana_reduction,
        ))
    } else if shape.next_spell {
        (!matches!(reduction, Value::X)).then(|| {
            EffectAst::subject_verb_reduce_next_spell_generic_cost_this_turn(
                shape.player,
                shape.filter,
                reduction,
            )
        })
    } else if shape.duration == Until::EndOfTurn {
        Some(
            EffectAst::subject_verb_reduce_matching_spell_cost_this_turn(
                shape.player,
                shape.filter,
                reduction,
            ),
        )
    } else {
        Some(EffectAst::subject_verb_reduce_matching_spell_cost(
            shape.player,
            shape.filter,
            reduction,
            shape.duration,
        ))
    }
}

fn parse_exile_replacement_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(effect) = parse_zone_replacement_subject_verb(tokens)? else {
        return Ok(None);
    };
    crate::parse_trace::event(
        "effect-route: subject-verb verb=Exile subject=implicit recognizer=instead-replacement",
    );
    Ok(Some(vec![effect]))
}

#[path = "labeled_prefixes/followup_predicates.rs"]
mod followup_predicates;
pub use followup_predicates::*;

#[path = "labeled_prefixes/reference.rs"]
mod reference_programs;
pub use reference_programs::parse_subject_verb_extension_sentence;
use reference_programs::{
    parse_earthbend_subject_verb_sentence, parse_for_each_opponent_doesnt_subject_verb_sentence,
    parse_gain_ability_subject_verb_sentence, parse_gain_ability_to_source_subject_verb_sentence,
};
#[path = "labeled_prefixes/core.rs"]
mod labeled_prefixes_core_programs;
use labeled_prefixes_core_programs::parse_passive_color_type_addition_sentence;
