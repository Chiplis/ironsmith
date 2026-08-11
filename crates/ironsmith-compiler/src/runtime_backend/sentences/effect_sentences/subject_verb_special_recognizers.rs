use super::super::front_end::grammar::effects::special_sentence_shapes as shapes;
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleIndex, RULE_SHAPE_STARTS_IF};
use super::sentence_helpers::target_ast_to_object_filter;
use super::{parse_object_filter, parse_target_phrase as parse_target_phrase_lexed};
use crate::cards::builders::{CardTextError, ChoiceCount, EffectAst};
use crate::cards::builders::{IT_TAG, PlayerAst, TagKey, Value};
use crate::effect::{EventValueSpec, Until};
use crate::runtime_backend::model::ast::{SubjectVerbActionAst, SubjectVerbRoleAst};
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::CardType;

pub(crate) fn parse_keyword_bundle_pump_sentence(
    tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let shape = shapes::parse_keyword_bundle_pump_shape(tokens).map_err(|error| {
        let clause = crate::runtime_backend::token_word_refs(tokens).join(" ");
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

pub(crate) fn parse_scaled_target_power_sentence(
    tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
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
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::clause_pattern_helpers::parse_redirect_next_damage_sentence(view.tokens)
}

pub(super) fn parse_prevent_next_time_damage_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::clause_pattern_helpers::parse_prevent_next_time_damage_sentence(view.tokens)
}

pub(super) fn parse_scaled_target_power_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_scaled_target_power_sentence(view.tokens)
}

pub(super) fn parse_keyword_bundle_pump_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_keyword_bundle_pump_sentence(view.tokens)
}

pub(super) fn parse_spell_this_way_pay_life_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if shapes::parses_spell_this_way_pay_life(view.tokens) {
        return Ok(Some(vec![
            EffectAst::subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(TagKey::from(IT_TAG), PlayerAst::You),
        ]));
    }
    Ok(None)
}

pub(super) fn parse_sacrifice_any_number_then_draw_that_many_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = shapes::parse_sacrifice_then_draw_shape(view.tokens) else {
        return Ok(None);
    };
    if shape.filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after 'any number of' (clause: '{}')",
            view.display_text()
        )));
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
        parse_object_filter(shape.filter_tokens, false)?
    };
    let tag = TagKey::from("sacrificed_0");

    Ok(Some(vec![
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
    ]))
}

pub(super) const SUBJECT_VERB_PRE_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 6] = [
    LexRuleDef {
        id: "redirect-next-damage",
        priority: 100,
        heads: &["the", "all"],
        shape_mask: 0,
        run: parse_redirect_next_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "prevent-next-time-damage",
        priority: 110,
        heads: &["the"],
        shape_mask: 0,
        run: parse_prevent_next_time_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "scale-target-power",
        priority: 120,
        heads: &["double", "triple", "until"],
        shape_mask: 0,
        run: parse_scaled_target_power_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "keyword-bundle-pump",
        priority: 125,
        heads: &["until"],
        shape_mask: 0,
        run: parse_keyword_bundle_pump_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "spell-this-way-pay-life",
        priority: 130,
        heads: &["if"],
        shape_mask: RULE_SHAPE_STARTS_IF,
        run: parse_spell_this_way_pay_life_rule_lexed,
    },
    LexRuleDef {
        id: "sacrifice-any-number-then-draw-that-many",
        priority: 140,
        heads: &["sacrifice"],
        shape_mask: 0,
        run: parse_sacrifice_any_number_then_draw_that_many_rule_lexed,
    },
];

pub(super) const SUBJECT_VERB_PRE_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SUBJECT_VERB_PRE_DIAGNOSTIC_RULES_LEXED);
