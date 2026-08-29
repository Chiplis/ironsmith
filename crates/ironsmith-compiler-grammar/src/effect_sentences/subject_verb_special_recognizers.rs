use super::super::front_end::grammar::effects::special_sentence_shapes as shapes;
use super::super::rule_engine::{
    LexClauseView, LexRuleDef, LexRuleHandler, LexRuleIndex, RULE_SHAPE_STARTS_IF, lex_clause_span,
};
use super::sentence_helpers::target_ast_to_object_filter;
use super::{parse_object_filter, parse_target_phrase as parse_target_phrase_lexed};
use crate::cards::builders::{CardTextError, ChoiceCount, EffectAst};
use crate::cards::builders::{IT_TAG, PlayerAst, TagKey, Value};
use crate::effect::{EventValueSpec, Until};
use crate::model::ast::{SubjectVerbActionAst, SubjectVerbRoleAst};
use crate::recognition::{ParseOutcome, RuleId};
use crate::registry::{HeadDiscriminator, RegistryRuleMetadata};
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::CardType;

pub fn parse_keyword_bundle_pump_sentence(
    tokens: &[crate::lexer::OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let shape = shapes::parse_keyword_bundle_pump_shape(tokens).map_err(|error| {
        let clause = crate::lexer::token_word_refs(tokens).join(" ");
        match error {
            shapes::KeywordBundleShapeError::UnsupportedAbility => CardTextError::ParseError(
                format!("unsupported keyword-bundle ability in gets clause: '{clause}'"),
            ),
            shapes::KeywordBundleShapeError::ModifierChanged => CardTextError::ParseError(format!(
                "keyword-bundle gets clause changes modifier mid-sequence: '{clause}'"
            )),
            shapes::KeywordBundleShapeError::UnsupportedTrailingList => CardTextError::ParseError(
                format!("unsupported trailing keyword-bundle list in gets clause: '{clause}'"),
            ),
        }
    })?;
    let Some(shape) = shape else {
        return Ok(None);
    };
    let base_filter = parse_object_filter(shape.filter_tokens, false)?;
    Ok(Some(
        shape
            .abilities
            .into_iter()
            .map(|ability_id| {
                EffectAst::subject_verb_pump_all(
                    base_filter.clone().with_static_ability(ability_id),
                    shape.power.clone(),
                    shape.toughness.clone(),
                    shape.duration.clone(),
                )
                .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each))
            })
            .collect(),
    ))
}

pub fn parse_scaled_target_power_sentence(
    tokens: &[crate::lexer::OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = shapes::parse_scaled_power_shape(tokens) else {
        return Ok(None);
    };
    let effect = match shape {
        shapes::ScaledPowerShape::SetLifeTotal {
            player,
            player_filter,
        } => EffectAst::subject_verb_set_life_total(
            player,
            Value::Scaled(Box::new(Value::LifeTotal(player_filter)), 2),
        ),
        shapes::ScaledPowerShape::DoubleManaPool { player } => {
            EffectAst::subject_verb_double_mana_pool(player)
        }
        shapes::ScaledPowerShape::ScaleAll {
            filter_tokens,
            axes,
            multiplier,
        } => EffectAst::subject_verb_scale_power_toughness_all(
            parse_object_filter(filter_tokens, false)?,
            axes.power,
            axes.toughness,
            multiplier,
            Until::EndOfTurn,
        ),
        shapes::ScaledPowerShape::ScaleTarget {
            target_tokens,
            axes,
            multiplier,
        } => {
            let target = parse_target_phrase_lexed(target_tokens)?;
            let amount_source_filter =
                target_ast_to_object_filter(target.clone()).unwrap_or_else(|| {
                    let mut fallback = ObjectFilter::default();
                    fallback.card_types.push(CardType::Creature);
                    fallback
                });
            let value_spec = Box::new(ChooseSpec::target(ChooseSpec::Object(amount_source_filter)));
            let scaled_stat = |value: Value| {
                if multiplier == 1 {
                    value
                } else {
                    Value::Scaled(Box::new(value), multiplier)
                }
            };
            EffectAst::subject_verb_pump(
                if axes.power {
                    scaled_stat(Value::PowerOf(value_spec.clone()))
                } else {
                    Value::Fixed(0)
                },
                if axes.toughness {
                    scaled_stat(Value::ToughnessOf(value_spec))
                } else {
                    Value::Fixed(0)
                },
                target,
                Until::EndOfTurn,
                None,
            )
        }
    };
    Ok(Some(vec![effect]))
}

pub(super) fn parse_redirect_next_damage_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    let rule = RuleId::new("redirect-next-damage");
    match super::clause_pattern_helpers::parse_redirect_next_damage_sentence(view.tokens) {
        Ok(Some(value)) => ParseOutcome::matched(value, lex_clause_span(view)),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            ParseOutcome::Error(crate::recognition::ParseDiagnostic::from_card_text_error(
                rule,
                lex_clause_span(view),
                error,
            ))
        }
    }
}

pub(super) fn parse_prevent_next_time_damage_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    let rule = RuleId::new("prevent-next-time-damage");
    match super::clause_pattern_helpers::parse_prevent_next_time_damage_sentence(view.tokens) {
        Ok(Some(value)) => ParseOutcome::matched(value, lex_clause_span(view)),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            ParseOutcome::Error(crate::recognition::ParseDiagnostic::from_card_text_error(
                rule,
                lex_clause_span(view),
                error,
            ))
        }
    }
}

pub(super) fn parse_scaled_target_power_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    let rule = RuleId::new("scaled-target-power");
    match parse_scaled_target_power_sentence(view.tokens) {
        Ok(Some(value)) => ParseOutcome::matched(value, lex_clause_span(view)),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            ParseOutcome::Error(crate::recognition::ParseDiagnostic::from_card_text_error(
                rule,
                lex_clause_span(view),
                error,
            ))
        }
    }
}

pub(super) fn parse_keyword_bundle_pump_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    let rule = RuleId::new("keyword-bundle-pump");
    match parse_keyword_bundle_pump_sentence(view.tokens) {
        Ok(Some(value)) => ParseOutcome::matched(value, lex_clause_span(view)),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            ParseOutcome::Error(crate::recognition::ParseDiagnostic::from_card_text_error(
                rule,
                lex_clause_span(view),
                error,
            ))
        }
    }
}

pub(super) fn parse_spell_this_way_pay_life_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    if shapes::parses_spell_this_way_pay_life(view.tokens) {
        return ParseOutcome::matched(vec![
            EffectAst::subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(TagKey::from(IT_TAG), PlayerAst::You),
        ], lex_clause_span(view));
    }
    ParseOutcome::NoMatch
}

pub(super) fn parse_sacrifice_any_number_then_draw_that_many_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    let Some(shape) = shapes::parse_sacrifice_then_draw_shape(view.tokens) else {
        return ParseOutcome::NoMatch;
    };
    if shape.filter_tokens.is_empty() {
        return ParseOutcome::Error(crate::recognition::ParseDiagnostic::malformed(
            RuleId::new("sacrifice-any-number-draw"),
            lex_clause_span(view),
            [crate::recognition::ParseExpectation::new(
                "sacrifice object",
            )],
            format!(
                "missing sacrifice object after 'any number of' (clause: '{}')",
                view.display_text()
            ),
        ));
    }
    let filter = if shape.artifact_enchantment_or_token {
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![
            ObjectFilter::artifact().you_control(),
            ObjectFilter::enchantment().you_control(),
            ObjectFilter::default().token().you_control(),
        ];
        filter
    } else {
        match parse_object_filter(shape.filter_tokens, false) {
            Ok(filter) => filter,
            Err(error) => {
                return ParseOutcome::Error(
                    crate::recognition::ParseDiagnostic::from_card_text_error(
                        RuleId::new("sacrifice-any-number-draw"),
                        lex_clause_span(view),
                        error,
                    ),
                );
            }
        }
    };
    let tag = crate::tag::CompilerReferenceTag::Sacrificed0.key();

    ParseOutcome::matched(
        vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::subject_verb_sacrifice_all(PlayerAst::You, ObjectFilter::tagged(tag)),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::Draw {
                    count: Value::EventValue(EventValueSpec::Amount)
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ThatManyCards),
                },
            ),
        ],
        lex_clause_span(view),
    )
}

pub(super) fn parse_additional_land_play_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    let rule = RuleId::new("additional-land-play");
    match crate::permission_helpers::parse_additional_land_plays_clause_lexed(view.tokens) {
        Ok(Some(effect)) => ParseOutcome::matched(vec![effect], lex_clause_span(view)),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            ParseOutcome::Error(crate::recognition::ParseDiagnostic::from_card_text_error(
                rule,
                lex_clause_span(view),
                error,
            ))
        }
    }
}

pub(super) fn parse_cross_zone_where_x_fanout_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    let words = crate::lexer::parser_token_word_refs(view.tokens);
    let proves_cross_zone_fanout =
        crate::word_primitives::sequence_occurs(&words, &["each", "player", "exiles"])
            && crate::word_primitives::sequence_occurs(&words, &["where", "x", "is"])
            && crate::word_primitives::contains_all_words(
                &words,
                &["permanents", "cards", "hand", "then"],
            );
    if !proves_cross_zone_fanout {
        return ParseOutcome::NoMatch;
    }
    super::chain_carry::parse_effect_chain_rule_lexed(view)
}

pub(super) const SUBJECT_VERB_PRE_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 8] = [
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("redirect-next-damage"),
            HeadDiscriminator::words(&["the", "all"]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_redirect_next_damage_sentence_rule_lexed),
    },
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("prevent-next-time-damage"),
            HeadDiscriminator::words(&["the"]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_prevent_next_time_damage_sentence_rule_lexed),
    },
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("scale-target-power"),
            HeadDiscriminator::words(&["double", "triple", "until"]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_scaled_target_power_sentence_rule_lexed),
    },
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("keyword-bundle-pump"),
            HeadDiscriminator::words(&["until"]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_keyword_bundle_pump_sentence_rule_lexed),
    },
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("spell-this-way-pay-life"),
            HeadDiscriminator::words(&["if"]),
        ),
        shape_mask: RULE_SHAPE_STARTS_IF,
        run: LexRuleHandler::Structured(parse_spell_this_way_pay_life_rule_lexed),
    },
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("sacrifice-any-number-then-draw-that-many"),
            HeadDiscriminator::words(&["sacrifice"]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_sacrifice_any_number_then_draw_that_many_rule_lexed),
    },
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("additional-land-play"),
            HeadDiscriminator::words(&["you"]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_additional_land_play_rule_lexed),
    },
    LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("cross-zone-where-x-fanout"),
            HeadDiscriminator::words(&["put"]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_cross_zone_where_x_fanout_rule_lexed),
    },
];

pub(super) const SUBJECT_VERB_PRE_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SUBJECT_VERB_PRE_DIAGNOSTIC_RULES_LEXED);
