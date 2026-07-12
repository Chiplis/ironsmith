pub(crate) use self::become_clause::parse_become_clause;
use self::helpers::{parse_controller_or_owner_of_target_subject, render_lower_words};
use self::next_turn_cant::parse_next_turn_cant_clause;
use super::super::activation_and_restrictions::{
    build_may_cast_tagged_effect, find_negation_span, parse_cant_restrictions,
    parse_choose_card_type_phrase_words, parse_choose_color_phrase_words,
    parse_choose_creature_type_phrase_words, parse_choose_player_phrase_words,
    parse_may_cast_it_sentence, parse_single_word_keyword_action,
    parse_target_player_choose_objects_clause, parse_you_choose_objects_clause_with_count_value,
    parse_you_choose_player_clause, starts_with_target_indicator,
};
use super::super::grammar::choices::parse_choice_land_type_phrase_words;
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::clause_dispatch_shapes as clause_grammar;
use super::super::grammar::effects::followup_shapes as followup_grammar;
use super::super::grammar::effects::parse_mana_replacement_clause_spec_lexed;
use super::super::grammar::primitives::TokenWordView;
use super::super::grammar::structure::split_trailing_if_clause_lexed;
use super::super::keyword_static::{parse_ability_line, parse_pt_modifier_values};
use super::super::lexer::{LexedClause, OwnedLexToken, contains_token_word};
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::parse_cast_or_play_tagged_clause;
use super::super::util::{
    parse_subject, parse_target_phrase, parser_trace, parser_trace_stack, span_from_tokens,
    trim_commas,
};
use super::clause_primitives::run_clause_primitives;
use super::dispatch_inner::{
    parse_additional_phase_sentence, parse_prevent_damage_sentence, parse_take_extra_turn_sentence,
    trim_edge_punctuation,
};
use super::for_each_helpers::{
    is_mana_replacement_clause_words, is_mana_trigger_additional_clause_words,
    is_target_player_dealt_damage_by_this_turn_subject, parse_for_each_object_subject,
    parse_get_for_each_count_value, parse_get_modifier_values_with_tail,
    parse_has_base_power_clause, parse_has_base_power_toughness_clause,
};
use super::search_library::parse_restriction_duration;
use super::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, find_unquoted_token_word,
    parse_sentence_delayed_next_step_unless_pays, try_build_unless,
};
use super::verb_dispatch::parse_effect_with_verb;
use super::verb_handlers::parse_control_duration;
use super::zone_counter_helpers::parse_put_counters;
use super::zone_handlers::{
    collapse_leading_signed_pt_modifier_tokens,
    parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control, parse_sacrifice,
};
use super::{
    Verb, bind_implicit_player_context, find_verb, parse_effect_chain_with_subject_verb_primitives,
    parse_simple_gain_ability_clause, parse_simple_lose_ability_clause,
};
use crate::TagKey;
use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, KeywordAction, PlayerAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::object::CounterType;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;

mod become_clause;
mod helpers;
mod next_turn_cant;

type ClauseDispatchCompatWords<'a> = TokenWordView<'a>;

const TARGET_WORD: &str = "target";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommonPlayerActionPattern {
    Amount,
    ObjectSelection,
    ZoneMovement,
    Choice,
    Payment,
    StateChange,
}

#[derive(Debug, Clone, Copy)]
struct PlayerAmountClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

fn parse_copular_base_pt_animation_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_copular_animation_shape(tokens) else {
        return Ok(None);
    };

    parse_become_clause(shape.subject_tokens, shape.animation_tokens).map(Some)
}

#[derive(Debug, Clone, Copy)]
struct PlayerObjectClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerZoneClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerChoiceClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerPaymentClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerStateClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
enum CommonPlayerActionClause<'a> {
    Amount(PlayerAmountClause<'a>),
    Object(PlayerObjectClause<'a>),
    Zone(PlayerZoneClause<'a>),
    Choice(PlayerChoiceClause<'a>),
    Payment(PlayerPaymentClause<'a>),
    State(PlayerStateClause<'a>),
}

impl<'a> PlayerAmountClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerObjectClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerZoneClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerChoiceClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerPaymentClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerStateClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

fn common_player_action_pattern_for(
    verb: Verb,
    action_tokens: &[OwnedLexToken],
) -> Option<CommonPlayerActionPattern> {
    let words = TokenWordView::new(action_tokens);
    if matches!(verb, Verb::Pay) {
        return Some(CommonPlayerActionPattern::Payment);
    }
    if matches!(verb, Verb::Scry | Verb::Surveil) {
        return Some(CommonPlayerActionPattern::Choice);
    }
    if matches!(
        verb,
        Verb::Sacrifice | Verb::Discard | Verb::Reveal | Verb::Look
    ) {
        return Some(CommonPlayerActionPattern::ObjectSelection);
    }
    if matches!(
        verb,
        Verb::Shuffle | Verb::Move | Verb::Put | Verb::Return | Verb::Exile
    ) || words.word_refs().iter().any(|word| {
        matches!(
            *word,
            "library" | "graveyard" | "hand" | "battlefield" | "exile"
        )
    }) {
        return Some(CommonPlayerActionPattern::ZoneMovement);
    }
    if matches!(
        verb,
        Verb::Draw | Verb::Lose | Verb::Gain | Verb::Mill | Verb::Get | Verb::Add
    ) {
        return Some(CommonPlayerActionPattern::Amount);
    }
    if matches!(verb, Verb::Skip | Verb::Take | Verb::Become | Verb::End) {
        return Some(CommonPlayerActionPattern::StateChange);
    }
    None
}

fn parse_control_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_control_player_shape(tokens) else {
        return Ok(None);
    };
    let TargetAst::Player(target_filter, _) = parse_target_phrase(shape.target_tokens)? else {
        return Ok(None);
    };
    let duration = parse_control_duration(shape.duration_tokens)?;
    Ok(Some(EffectAst::subject_verb_control_player(
        shape.player,
        PlayerFilter::Target(Box::new(target_filter)),
        duration,
    )))
}

fn is_pronoun_top_or_bottom_library_choice_put_tail(tokens: &[OwnedLexToken]) -> bool {
    clause_grammar::is_pronoun_library_choice_put_shape(tokens)
}

impl<'a> CommonPlayerActionClause<'a> {
    fn recognize(
        subject: SubjectAst,
        verb: Verb,
        action_tokens: &'a [OwnedLexToken],
    ) -> Option<Self> {
        if !matches!(subject, SubjectAst::Player(_)) {
            return None;
        }
        let pattern = common_player_action_pattern_for(verb, action_tokens)?;
        Some(match pattern {
            CommonPlayerActionPattern::Amount => Self::Amount(PlayerAmountClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::ObjectSelection => Self::Object(PlayerObjectClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::ZoneMovement => Self::Zone(PlayerZoneClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::Choice => Self::Choice(PlayerChoiceClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::Payment => Self::Payment(PlayerPaymentClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::StateChange => Self::State(PlayerStateClause {
                subject,
                verb,
                action_tokens,
            }),
        })
    }

    #[cfg(test)]
    fn pattern(&self) -> CommonPlayerActionPattern {
        match self {
            Self::Amount(_) => CommonPlayerActionPattern::Amount,
            Self::Object(_) => CommonPlayerActionPattern::ObjectSelection,
            Self::Zone(_) => CommonPlayerActionPattern::ZoneMovement,
            Self::Choice(_) => CommonPlayerActionPattern::Choice,
            Self::Payment(_) => CommonPlayerActionPattern::Payment,
            Self::State(_) => CommonPlayerActionPattern::StateChange,
        }
    }

    fn lower(self) -> Result<EffectAst, CardTextError> {
        match self {
            Self::Amount(clause) => clause.lower(),
            Self::Object(clause) => clause.lower(),
            Self::Zone(clause) => clause.lower(),
            Self::Choice(clause) => clause.lower(),
            Self::Payment(clause) => clause.lower(),
            Self::State(clause) => clause.lower(),
        }
    }
}

fn parse_play_exiled_cards_for_as_long_as_exiled_clause(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    (clause_grammar::parse_tagged_permission_shape(tokens)
        == Some(clause_grammar::TaggedPermissionShape::PlayExiledForAsLongAsExiled))
    .then(|| {
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            true,
            false,
            false,
            None,
        )
    })
}

fn parse_mana_any_type_cast_tagged_this_way_clause(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    (clause_grammar::parse_tagged_permission_shape(tokens)
        == Some(clause_grammar::TaggedPermissionShape::ManaAnyTypeCastsTaggedThisWay))
    .then(|| {
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            false,
            false,
            true,
            None,
        )
    })
}

pub(crate) fn parse_for_each_prevent_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_for_each_prevent_shape(tokens) else {
        return Ok(None);
    };
    let Some(filter) = parse_for_each_object_subject(shape.subject_tokens)? else {
        return Ok(None);
    };

    let Some(prevent_effect) = parse_prevent_damage_sentence(shape.prevent_tokens)? else {
        return Ok(None);
    };

    let effects = if let Some(idx) = shape.unless_token {
        if let Some(unless_effect) = try_build_unless(
            vec![prevent_effect.clone()],
            SubjectVerbPrimitiveClause::new(tokens),
            idx,
        )? {
            vec![unless_effect]
        } else {
            vec![prevent_effect]
        }
    } else {
        vec![prevent_effect]
    };
    Ok(Some(EffectAst::ForEachObject { filter, effects }))
}

pub(crate) fn parse_for_each_counter_group_removed_this_way_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_counter_group_removed_shape(tokens) else {
        return Ok(None);
    };
    if shape.group_size == 0 {
        return Err(CardTextError::ParseError(format!(
            "counter group size must be positive (clause: '{}')",
            render_lower_words(tokens)
        )));
    }
    if shape.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after counter group clause (clause: '{}')",
            render_lower_words(tokens)
        )));
    }

    let effects = parse_effect_chain_with_subject_verb_primitives(shape.effect_tokens)?;
    Ok(Some(EffectAst::RepeatEffects {
        count: Value::DividedRoundedDown(Box::new(Value::X), shape.group_size as i32)
            .with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay),
        effects,
    }))
}

fn parse_cast_any_number_from_among_tagged_clause(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let shape = clause_grammar::parse_cast_any_tagged_shape(tokens)?;

    let mut filter = ObjectFilter::nonland().in_zone(Zone::Exile).match_tagged(
        TagKey::from(IT_TAG),
        crate::target::TaggedOpbjectRelation::IsTaggedObject,
    );

    filter.mana_value = shape.mana_value;

    Some(EffectAst::ForEachObject {
        filter,
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                false,
                false,
                true,
                None,
            )],
        }],
    })
}

fn parse_cast_single_spell_from_among_hand_cards_clause(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if clause_grammar::parse_tagged_permission_shape(tokens)
        != Some(clause_grammar::TaggedPermissionShape::CastSingleFromAmongHandCards)
    {
        return None;
    }

    Some(
        EffectAst::may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
            PlayerAst::You,
            PlayerAst::That,
            ObjectFilter::nonland().in_zone(Zone::Hand),
            Zone::Hand,
        ),
    )
}

fn parse_passive_sacrifice_by_controller_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_passive_sacrifice_shape(tokens) else {
        return Ok(None);
    };

    let filter = parse_object_filter(shape.object_tokens, false)?;
    Ok(Some(EffectAst::ForEachObject {
        filter,
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::ItsController,
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            1,
            None,
        )],
    }))
}

fn parse_get_pump_clause(
    subject_tokens: &[OwnedLexToken],
    action_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(subject_shape) = clause_grammar::parse_pump_subject_shape(subject_tokens) else {
        parser_trace("parse_get_pump_clause:subject-shape-miss", subject_tokens);
        return Ok(None);
    };
    let collapsed_modifier_tail = collapse_leading_signed_pt_modifier_tokens(action_tokens);
    let modifier_tail = collapsed_modifier_tail.as_deref().unwrap_or(action_tokens);

    if let Some(modifier) = clause_grammar::parse_discarded_this_way_modifier_shape(modifier_tail) {
        let target = parse_target_phrase(subject_shape.subject_tokens)?;
        return Ok(Some(EffectAst::subject_verb_pump_for_each(
            modifier.power,
            modifier.toughness,
            target,
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay),
            subject_shape.duration.unwrap_or(Until::EndOfTurn),
        )));
    }

    let Some(mod_token) = modifier_tail.first().map(OwnedLexToken::parser_text) else {
        parser_trace("parse_get_pump_clause:missing-modifier", action_tokens);
        return Ok(None);
    };
    let Ok((power, toughness)) = parse_pt_modifier_values(mod_token) else {
        parser_trace("parse_get_pump_clause:modifier-shape-miss", modifier_tail);
        return Ok(None);
    };
    let mut count = parse_get_for_each_count_value(modifier_tail)?;
    if count.is_none()
        && let Some(for_each_tokens) =
            clause_grammar::parse_modifier_duration_for_each_tokens(modifier_tail)
    {
        count = parse_get_for_each_count_value(for_each_tokens)?;
    }
    if let Some(count) = count {
        let power_per = match power {
            Value::Fixed(value) => value,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each power modifier (clause: '{}')",
                    render_lower_words(full_tokens)
                )));
            }
        };
        let toughness_per = match toughness {
            Value::Fixed(value) => value,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each toughness modifier (clause: '{}')",
                    render_lower_words(full_tokens)
                )));
            }
        };
        let target = parse_target_phrase(subject_shape.subject_tokens)?;
        return Ok(Some(EffectAst::subject_verb_pump_for_each(
            power_per,
            toughness_per,
            target,
            count,
            subject_shape.duration.unwrap_or(Until::EndOfTurn),
        )));
    }

    let (power, toughness, parsed_duration, condition) =
        parse_get_modifier_values_with_tail(modifier_tail, power, toughness)?;
    let duration = subject_shape.duration.unwrap_or(parsed_duration);
    let effect = match subject_shape.kind {
        clause_grammar::PumpSubjectKind::Tagged => EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(subject_shape.subject_tokens),
            ),
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::DemonstrativeTarget => EffectAst::subject_verb_pump(
            power,
            toughness,
            parse_target_phrase(subject_shape.subject_tokens)?,
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::ControlledFilter {
            filter_tokens,
            controller,
        } => {
            let Ok(mut filter) = parse_object_filter(filter_tokens, false) else {
                return Ok(None);
            };
            if filter == ObjectFilter::default() {
                return Ok(None);
            }
            filter.controller = Some(controller);
            EffectAst::subject_verb_pump_all(filter, power, toughness, duration)
        }
        clause_grammar::PumpSubjectKind::DirectTarget(target_tokens) => {
            EffectAst::subject_verb_pump(
                power,
                toughness,
                parse_target_phrase(target_tokens)?,
                duration,
                condition,
            )
        }
        clause_grammar::PumpSubjectKind::Equipped => EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Tagged(
                TagKey::from("equipped"),
                span_from_tokens(subject_shape.subject_tokens),
            ),
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::Enchanted => EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Tagged(
                TagKey::from("enchanted"),
                span_from_tokens(subject_shape.subject_tokens),
            ),
            duration,
            condition,
        ),
        clause_grammar::PumpSubjectKind::FilterCandidate {
            filter_tokens,
            mentions_this,
            disallowed_pronoun,
            demonstrative_reference,
        } => {
            if demonstrative_reference {
                return Ok(None);
            }
            let Ok(filter) = parse_object_filter(filter_tokens, false) else {
                return Ok(None);
            };
            if filter == ObjectFilter::default()
                || (mentions_this && !filter.other)
                || (disallowed_pronoun && !filter.other)
            {
                return Ok(None);
            }
            EffectAst::subject_verb_pump_all(filter, power, toughness, duration)
        }
    };
    Ok(Some(effect))
}

fn lower_direct_clause_shape(
    shape: clause_grammar::DirectClauseShape,
    tokens: &[OwnedLexToken],
) -> EffectAst {
    match shape {
        clause_grammar::DirectClauseShape::RingTemptsYou => {
            EffectAst::subject_verb_ring_tempts_you(PlayerAst::You)
        }
        clause_grammar::DirectClauseShape::TakeInitiative => {
            EffectAst::subject_verb_take_initiative(PlayerAst::You)
        }
        clause_grammar::DirectClauseShape::ChooseOddOrEven => {
            EffectAst::subject_verb_choose_named_option(
                PlayerAst::Implicit,
                vec!["odd".to_string(), "even".to_string()],
            )
        }
        clause_grammar::DirectClauseShape::ChooseLeftOrRight => {
            EffectAst::subject_verb_choose_named_option(
                PlayerAst::You,
                vec!["left".to_string(), "right".to_string()],
            )
        }
        clause_grammar::DirectClauseShape::ClearSuspected => {
            EffectAst::subject_verb_clear_suspected(None)
        }
        clause_grammar::DirectClauseShape::CopySourceExiledCard => {
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::default().in_zone(Zone::Exile).match_tagged(
                    TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    crate::target::TaggedOpbjectRelation::IsTaggedObject,
                ),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                zones: vec![Zone::Exile],
                search_mode: None,
            }
        }
        clause_grammar::DirectClauseShape::PutTaggedPlusOneCounter => {
            EffectAst::subject_verb_put_counters(
                CounterType::PlusOnePlusOne,
                Value::Fixed(1),
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                None,
                false,
            )
        }
        clause_grammar::DirectClauseShape::DamagedPlayersCantGainLife => {
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::gain_life(PlayerFilter::DamagedPlayer),
                Until::EndOfTurn,
                None,
            )
        }
        clause_grammar::DirectClauseShape::DamageCantBePrevented => EffectAst::subject_verb_cant(
            crate::effect::Restriction::prevent_damage(),
            Until::EndOfTurn,
            None,
        ),
        clause_grammar::DirectClauseShape::TurnSourceExiledFaceUp => EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::TurnFaceUp {
                target: TargetAst::Tagged(
                    TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    span_from_tokens(tokens),
                ),
            },
        ),
        clause_grammar::DirectClauseShape::TurnTaggedFaceUp => EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::TurnFaceUp {
                target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
            },
        ),
        clause_grammar::DirectClauseShape::Planeswalk => {
            EffectAst::subject_verb_emit_keyword_action(
                crate::events::KeywordActionKind::Planeswalk,
                1,
            )
        }
        clause_grammar::DirectClauseShape::ChaosEnsues => {
            EffectAst::subject_verb_emit_keyword_action(
                crate::events::KeywordActionKind::ChaosEnsues,
                1,
            )
        }
        clause_grammar::DirectClauseShape::DoubleX => EffectAst::subject_verb_scale_x_value(
            TargetAst::Tagged(TagKey::from("triggering"), span_from_tokens(tokens)),
            2,
        ),
        clause_grammar::DirectClauseShape::OnlyChosenCanAttack => EffectAst::subject_verb_cant(
            crate::effect::Restriction::attack(
                ObjectFilter::creature().not_tagged(TagKey::from(IT_TAG)),
            ),
            Until::EndOfCombat,
            None,
        ),
        clause_grammar::DirectClauseShape::OnlyChosenCanBlock => EffectAst::subject_verb_cant(
            crate::effect::Restriction::block(
                ObjectFilter::creature().not_tagged(TagKey::from(IT_TAG)),
            ),
            Until::EndOfCombat,
            None,
        ),
        clause_grammar::DirectClauseShape::CastNonlandTaggedThisWay => {
            let filter = ObjectFilter::nonland().in_zone(Zone::Exile).match_tagged(
                TagKey::from(IT_TAG),
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            );
            EffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::May {
                    effects: vec![EffectAst::subject_verb_cast_tagged(
                        TagKey::from(IT_TAG),
                        PlayerAst::You,
                        false,
                        false,
                        true,
                        None,
                    )],
                }],
            }
        }
    }
}

pub(crate) fn parse_effect_clause(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    let stripped_instead = super::strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);

    if let Some(shape) = followup_grammar::parse_counter_linked_land_subtype_followup(tokens) {
        let _counter_type = shape.counter_type;
        return Ok(EffectAst::subject_verb_add_subtypes(
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
            vec![shape.subtype],
            Until::Forever,
        ));
    }

    if let Some(effect) = effect_grammar::parse_prevent_damage_sentence_lexed(tokens)? {
        return Ok(effect);
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens)
        && let Ok(base_effect) = parse_effect_clause(trailing_if.leading_tokens)
    {
        return Ok(EffectAst::Conditional {
            predicate: trailing_if.predicate,
            if_true: vec![base_effect],
            if_false: Vec::new(),
        });
    }

    if let Some(spec) = parse_may_cast_it_sentence(tokens) {
        return Ok(build_may_cast_tagged_effect(&spec));
    }

    if let Some(effect) = parse_play_exiled_cards_for_as_long_as_exiled_clause(tokens) {
        return Ok(effect);
    }

    if let Some(effect) = parse_cast_or_play_tagged_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_cast_any_number_from_among_tagged_clause(tokens) {
        return Ok(effect);
    }

    if let Some(effect) = parse_cast_single_spell_from_among_hand_cards_clause(tokens) {
        return Ok(effect);
    }

    if let Some(effect) = parse_mana_any_type_cast_tagged_this_way_clause(tokens) {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_leading_may_clause_shape(tokens) {
        let mut effects = parse_effect_chain_with_subject_verb_primitives(shape.effect_tokens)?;
        return Ok(match shape.actor {
            clause_grammar::LeadingMayActorShape::Player(player) => {
                for effect in &mut effects {
                    bind_implicit_player_context(effect, player);
                }
                EffectAst::MayByPlayer { player, effects }
            }
            clause_grammar::LeadingMayActorShape::Implicit => EffectAst::May { effects },
        });
    }

    if let Some(shape) = clause_grammar::parse_tagged_plural_pump_shape(tokens)
        && let Some(effect) =
            parse_get_pump_clause(shape.subject_tokens, shape.modifier_tokens, tokens)?
    {
        return Ok(effect);
    }

    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();

    if let Some(effect) = parse_for_each_prevent_damage_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_for_each_counter_group_removed_this_way_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_direct_clause_shape(tokens) {
        return Ok(lower_direct_clause_shape(shape, tokens));
    }

    if let Some(shape) = clause_grammar::parse_shared_ability_gain_shape(tokens) {
        return Ok(EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Tagged(
                TagKey::from(IT_TAG),
                Some(crate::cards::builders::TextSpan::synthetic()),
            ),
            shape
                .abilities
                .into_iter()
                .map(GrantedAbilityAst::KeywordAction)
                .collect(),
            Until::Forever,
        ));
    }
    if let Some(effect) = parse_take_extra_turn_sentence(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(effect);
    }
    if let Some(spec) = parse_mana_replacement_clause_spec_lexed(tokens) {
        return Ok(EffectAst::subject_verb_register_mana_replacement(
            ObjectFilter::land().you_control(),
            vec![spec.replacement_mana],
            crate::effects::ReplacementApplyMode::UntilEndOfTurn,
        ));
    }
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}') [rule=mana-replacement]",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}') [rule=mana-trigger-additional]",
            clause_words.join(" ")
        )));
    }

    if let Some(shape) = clause_grammar::parse_for_each_card_payment_shape(tokens) {
        let mut filter = ObjectFilter::default();
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(EffectAst::ForEachObject {
            filter,
            effects: vec![EffectAst::UnlessAction {
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                    crate::zone::Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                alternative: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LoseLife {
                        amount: Value::Fixed(shape.life_amount as i32),
                    },
                )],
                player: PlayerAst::You,
            }],
        });
    }

    if let Some(shape) = clause_grammar::parse_opponent_return_choice_shape(tokens) {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::UnlessAction {
                    effects: vec![EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        false,
                    )],
                    alternative: vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    )],
                    player: PlayerAst::ItsController,
                },
            ],
        });
    }

    if let Some(effects) =
        parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(match effects.as_slice() {
            [effect] => effect.clone(),
            _ => EffectAst::Sequence { effects },
        });
    }

    if let Some(effect) =
        parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(tokens)
    {
        return Ok(effect);
    }

    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(effect);
    }

    let clause = SubjectVerbPrimitiveClause::new(tokens);
    if let Some(unless_idx) = find_unquoted_token_word(clause, "unless") {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        if !main_tokens.is_empty()
            && let Ok(main_effect) = parse_effect_clause(&main_tokens)
            && let Some(unless_effect) = try_build_unless(vec![main_effect], clause, unless_idx)?
        {
            return Ok(unless_effect);
        }
    }

    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_passive_sacrifice_by_controller_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_copular_base_pt_animation_clause(tokens)? {
        return Ok(effect);
    }

    let choice_tokens = clause_grammar::strip_optional_you_choice_tokens(tokens);
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();

    if let Some((consumed, excluded_color)) = parse_choose_color_phrase_words(&choice_words)?
        && consumed == choice_words.len()
        && excluded_color.is_none()
    {
        return Ok(EffectAst::subject_verb_choose_color(
            crate::cards::builders::PlayerAst::Implicit,
        ));
    }

    if let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_creature_type(
            crate::cards::builders::PlayerAst::Implicit,
            excluded_subtypes,
        ));
    }

    if let Some(parsed) = parse_choice_land_type_phrase_words(&choice_words)
        && parsed.consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_land_type(
            crate::cards::builders::PlayerAst::Implicit,
            parsed.exclude_basic,
        ));
    }

    if let Some((consumed, options)) = parse_choose_card_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_card_type(
            crate::cards::builders::PlayerAst::Implicit,
            options,
        ));
    }

    if let Some(consumed) = parse_choose_player_phrase_words(&choice_words)
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_player(
            crate::cards::builders::PlayerAst::Implicit,
            PlayerFilter::Any,
            TagKey::from(IT_TAG),
            false,
            0,
        ));
    }

    if let Some(shape) = clause_grammar::parse_choose_target_shape(tokens)
        && let Ok(target) = parse_target_phrase(shape.target_tokens)
    {
        let player_target = match &target {
            TargetAst::Player(_, _) => true,
            TargetAst::WithCount(inner, _) => matches!(inner.as_ref(), TargetAst::Player(_, _)),
            _ => false,
        };
        if player_target
            || clause_grammar::parse_clause_subject_verb_shape(shape.target_tokens).is_none()
        {
            return Ok(EffectAst::subject_verb_target_only(target));
        }
    }

    if let Some((chooser, choose_filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(tokens)?
    {
        return Ok(EffectAst::subject_verb_choose_player(
            chooser,
            choose_filter,
            TagKey::from(IT_TAG),
            random,
            exclude_previous_choices,
        ));
    }

    if let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_you_choose_objects_clause_with_count_value(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some(shape) = clause_grammar::parse_assigns_no_combat_damage_shape(tokens) {
        let source = match shape {
            clause_grammar::AssignsNoCombatDamageShape::Unsupported => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported assigns-no-combat-damage clause tail (clause: '{}') [rule=assigns-no-combat-damage-tail]",
                    clause_words.join(" ")
                )));
            }
            clause_grammar::AssignsNoCombatDamageShape::Supported(
                clause_grammar::AssignDamageSourceShape::Source,
            ) => TargetAst::Source(None),
            clause_grammar::AssignsNoCombatDamageShape::Supported(
                clause_grammar::AssignDamageSourceShape::Tagged,
            ) => TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
            clause_grammar::AssignsNoCombatDamageShape::Supported(
                clause_grammar::AssignDamageSourceShape::Target(target_tokens),
            ) => parse_target_phrase(target_tokens)?,
        };
        return Ok(
            EffectAst::subject_verb_prevent_all_combat_damage_from_source(source, Until::EndOfTurn),
        );
    }

    if starts_with_target_indicator(tokens)
        && find_negation_span(tokens)
            .is_some_and(|(neg_start, _)| find_verb(&tokens[..neg_start]).is_none())
        && let (duration, clause_tokens) =
            parse_restriction_duration(tokens)?.unwrap_or((Until::Forever, tokens.to_vec()))
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && let Some(target) = parsed.target.clone()
    {
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_cant(parsed.restriction.clone(), duration, None),
            ],
        });
    }

    if let Some(shape) = clause_grammar::parse_target_only_shape(tokens) {
        if find_negation_span(tokens).is_some() || shape.restriction_like {
            return Err(CardTextError::ParseError(format!(
                "unsupported target-only restriction clause (clause: '{}') [rule=target-only-restriction]",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::subject_verb_target_only(target));
    }

    if let Some(shape) = clause_grammar::parse_embedded_choose_target_shape(tokens) {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::subject_verb_target_only(target));
    }

    if let Some(effect) = parse_next_turn_cant_clause(tokens)? {
        return Ok(effect);
    }

    if let Some((duration, clause_tokens)) = parse_restriction_duration(tokens)?
        && find_negation_span(&clause_tokens).is_some()
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && parsed.target.is_none()
    {
        return Ok(EffectAst::subject_verb_cant(
            parsed.restriction.clone(),
            duration,
            None,
        ));
    }

    if let Some(effect) = parse_hexproof_targeting_override_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_cast_target_without_paying_shape(tokens) {
        let _ = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::SubjectVerb(
            crate::runtime_backend::ast::SubjectVerbEffectAst {
                subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::Implicit,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: TagKey::from(IT_TAG),
                    player: PlayerAst::Implicit,
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                },
            },
        ));
    }

    if let Some(effect) = parse_passive_goad_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_control_player_clause(tokens)? {
        return Ok(effect);
    }

    // Generic "X if <predicate>" fallback: clauses like "play the exiled card
    // without paying its mana cost if you attacked with three or more
    // creatures this turn" have no known leading verb, but the head parses on
    // its own and the tail is a recognizable predicate. Only attempted where
    // the clause would otherwise be a hard no-verb error.
    if clause_grammar::parse_clause_subject_verb_shape(tokens).is_none()
        && let Some(shape) = clause_grammar::parse_trailing_if_fallback_shape(tokens)
        && let Ok(head_effects) = super::parse_effect_sentence_lexed(shape.head_tokens)
        && !head_effects.is_empty()
    {
        parser_trace("parse_effect_clause:trailing-if-fallback", tokens);
        return Ok(EffectAst::Conditional {
            predicate: shape.predicate,
            if_true: head_effects,
            if_false: Vec::new(),
        });
    }

    let (verb, _) = find_verb(tokens).ok_or_else(|| {
        let clause = render_lower_words(tokens);
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "convert",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "unattach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "detain",
            "goad",
            "suspect",
            "end",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    let verb_shape = clause_grammar::parse_clause_subject_verb_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "could not split subject and verb in effect clause (clause: '{}')",
            render_lower_words(tokens)
        ))
    })?;
    let subject_tokens_storage = trim_commas(verb_shape.subject_tokens);
    let subject_tokens = subject_tokens_storage.as_slice();
    let rest = verb_shape.action_tokens;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));

    if matches!(verb, Verb::Counter)
        && !subject_tokens.is_empty()
        && contains_token_word(tokens, "on")
        && let Ok(effect) = parse_put_counters(tokens)
    {
        parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
        return Ok(effect);
    }

    if matches!(verb, Verb::Get)
        && let Some(effect) = parse_get_pump_clause(subject_tokens, rest, tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Sacrifice)
        && let Some((subject, target)) = parse_controller_or_owner_of_target_subject(subject_tokens)
    {
        return parse_sacrifice(rest, Some(subject), Some(target));
    }
    if matches!(verb, Verb::Put)
        && let Some((SubjectAst::Player(PlayerAst::ItsOwner), target)) =
            parse_controller_or_owner_of_target_subject(subject_tokens)
    {
        if is_pronoun_top_or_bottom_library_choice_put_tail(rest) {
            return Ok(EffectAst::subject_verb_move_to_library_top_or_bottom_choice(target));
        }
    }
    let subject_word_view = ClauseDispatchCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}') [rule=combat-history-player-subject]",
            render_lower_words(tokens)
        )));
    }
    if matches!(verb, Verb::Gain) && !subject_tokens.is_empty() {
        if let Some(shape) = clause_grammar::parse_protection_choice_shape(rest) {
            let target = parse_target_phrase(subject_tokens)?;
            return Ok(EffectAst::subject_verb_grant_protection_choice(
                target,
                shape.includes_colorless,
                shape.includes_artifacts,
            ));
        }
    }
    if matches!(verb, Verb::Gain)
        && let Some(effects) =
            super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(EffectAst::Sequence { effects });
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Gain) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let parsed_actions = parse_ability_line(tail.ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(tail.ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !tail.ability_tokens.is_empty()
            && tail.trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_grant_abilities_to_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Lose) && clause_grammar::parse_shared_ability_gain_shape(rest).is_some()
    {
        let target = match clause_grammar::parse_reference_subject_shape(subject_tokens) {
            clause_grammar::ReferenceSubjectShape::Source => {
                TargetAst::Source(span_from_tokens(subject_tokens))
            }
            clause_grammar::ReferenceSubjectShape::Tagged => {
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
            }
            clause_grammar::ReferenceSubjectShape::Other => parse_target_phrase(subject_tokens)?,
        };
        return Ok(EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            Until::EndOfTurn,
        ));
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let ability_tokens = trim_edge_punctuation(tail.ability_tokens);
        let trailing_tokens = trim_edge_punctuation(tail.trailing_tokens);
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    if matches!(verb, Verb::Return)
        && clause_grammar::is_return_tagged_reference_shape(subject_tokens)
    {
        let mut return_tokens = subject_tokens.to_vec();
        return_tokens.extend(rest.iter().cloned());
        return parse_effect_with_verb(verb, Some(SubjectAst::This), &return_tokens);
    }
    if matches!(verb, Verb::Put)
        && clause_grammar::is_exiled_cards_to_hand_shape(subject_tokens, rest)
    {
        let filter = parse_object_filter(subject_tokens, false)?;
        return Ok(EffectAst::subject_verb_return_all_to_hand(filter));
    }
    let mut effect = if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = parse_subject(subject_tokens);
        if let Some(clause) = CommonPlayerActionClause::recognize(subject, verb, rest) {
            clause.lower()?
        } else {
            parse_effect_with_verb(verb, Some(subject), rest)?
        }
    };
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        };
    }
    Ok(effect)
}

fn parse_passive_goad_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_passive_goad_shape(tokens) else {
        return Ok(None);
    };
    let target = match shape.target {
        clause_grammar::GoadTargetShape::TaggedToken => {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens))
        }
        clause_grammar::GoadTargetShape::Target(target_tokens) => {
            parse_target_phrase(target_tokens)?
        }
    };
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(EffectAst::subject_verb_goad(target)))
}

fn parse_hexproof_targeting_override_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let (duration, clause_tokens) =
        parse_restriction_duration(tokens)?.unwrap_or((Until::Forever, tokens.to_vec()));
    let Some(shape) = clause_grammar::parse_hexproof_targeting_override_shape(&clause_tokens)
    else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.filter_tokens, false)?;
    Ok(Some(EffectAst::subject_verb_remove_abilities_all(
        filter,
        vec![GrantedAbilityAst::KeywordAction(KeywordAction::Hexproof)],
        duration,
    )))
}

pub(crate) fn parse_effect_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    fn lex_tail(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex test tail")
    }

    #[test]
    fn common_player_action_clause_classifies_core_shapes() {
        let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
        for (verb, tail, expected) in [
            (
                Verb::Draw,
                "X cards where X is their devotion to black",
                CommonPlayerActionPattern::Amount,
            ),
            (
                Verb::Sacrifice,
                "a creature they control",
                CommonPlayerActionPattern::ObjectSelection,
            ),
            (
                Verb::Shuffle,
                "their graveyard into their library",
                CommonPlayerActionPattern::ZoneMovement,
            ),
            (Verb::Pay, "{2}", CommonPlayerActionPattern::Payment),
            (Verb::Scry, "X", CommonPlayerActionPattern::Choice),
        ] {
            let tail = lex_tail(tail);
            let clause = CommonPlayerActionClause::recognize(subject.clone(), verb, &tail)
                .expect("common player clause should be recognized");
            assert_eq!(clause.pattern(), expected, "{verb:?} {tail:?}");
        }
    }

    #[test]
    fn common_player_action_clause_recognizes_typed_clause_variants() {
        let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
        for (verb, tail, assert_variant) in [
            (
                Verb::Draw,
                "X cards where X is their devotion to black",
                matches_amount as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Sacrifice,
                "a creature they control",
                matches_object as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Shuffle,
                "their graveyard into their library",
                matches_zone as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Scry,
                "X",
                matches_choice as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Pay,
                "{2}",
                matches_payment as fn(CommonPlayerActionClause<'_>),
            ),
        ] {
            let tail = lex_tail(tail);
            let clause = CommonPlayerActionClause::recognize(subject.clone(), verb, &tail)
                .expect("common player clause should be recognized");
            assert_variant(clause);
        }
    }

    fn matches_amount(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Amount(_)));
    }

    fn matches_object(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Object(_)));
    }

    fn matches_zone(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Zone(_)));
    }

    fn matches_choice(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Choice(_)));
    }

    fn matches_payment(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Payment(_)));
    }

    #[test]
    fn common_player_action_clause_delegates_to_effect_parser() {
        for text in [
            "Target opponent draws a card",
            "Target opponent sacrifices a creature they control",
            "Target opponent shuffles their library",
            "Target opponent pays {2}",
            "Each opponent scries 1",
        ] {
            let tokens = lex_line(text, 0).expect("lex clause");
            parse_effect_clause(&tokens)
                .unwrap_or_else(|err| panic!("common player clause should parse: {text}: {err:?}"));
        }
    }

    #[test]
    fn parses_control_target_player_during_next_turn_clause() {
        let tokens = lex_line(
            "You control target player during that player's next turn.",
            0,
        )
        .expect("lex clause");
        let effect = parse_effect_clause(&tokens)
            .expect("control target player during next turn should parse");
        let debug = format!("{effect:?}").to_ascii_lowercase();
        assert!(
            debug.contains("controlplayer") && debug.contains("nextturn"),
            "expected control-player-next-turn effect, got {debug}"
        );
    }

    #[test]
    fn counter_linked_land_subtype_followup_lowers_to_prior_tagged_land() {
        let tokens = lex_line(
            "That land is an Island in addition to its other types for as long as it has a flood counter on it.",
            0,
        )
        .unwrap();
        let effect = parse_effect_clause(&tokens).expect("typed land subtype followup");
        let debug = format!("{effect:#?}");
        assert!(debug.contains("AddSubtypes"), "{debug}");
        assert!(debug.contains("Island"), "{debug}");
        assert!(debug.contains(IT_TAG), "{debug}");
    }

    #[test]
    fn filtered_combat_damage_prevention_keeps_non_subtype_source_filter() {
        let tokens = lex_line(
            "Prevent all combat damage non-Soldier creatures would deal this turn.",
            0,
        )
        .unwrap();
        effect_grammar::parse_prevent_damage_sentence_lexed(&tokens)
            .expect("typed prevention grammar should not error")
            .expect("typed prevention grammar should recognize filtered source");
        let effect = parse_effect_clause(&tokens).expect("typed filtered prevention");
        let debug = format!("{effect:#?}");
        assert!(debug.contains("PreventAllCombatDamage"), "{debug}");
        assert!(debug.contains("Soldier"), "{debug}");
        assert!(debug.contains("excluded_subtypes"), "{debug}");
    }

    #[test]
    fn discarded_this_way_pump_split_keeps_typed_modifier_tail() {
        let tokens = lex_line(
            "target creature gets +2/+0 until end of turn for each card discarded this way",
            0,
        )
        .unwrap();
        let shape = clause_grammar::parse_clause_subject_verb_shape(&tokens).unwrap();
        assert!(
            clause_grammar::parse_discarded_this_way_modifier_shape(shape.action_tokens).is_some(),
            "{:?}",
            shape.action_tokens
        );
    }

    #[test]
    fn tagged_plural_pump_clause_lowers_directly() {
        let tokens = lex_line("they each get +2/+2 until end of turn", 0).unwrap();
        let shape = clause_grammar::parse_clause_subject_verb_shape(&tokens).unwrap();
        assert_eq!(
            ClauseDispatchCompatWords::new(shape.subject_tokens).word_refs(),
            ["they", "each"]
        );
        let effect = parse_get_pump_clause(shape.subject_tokens, shape.action_tokens, &tokens)
            .expect("tagged plural pump should not error")
            .expect("tagged plural pump should be recognized");
        assert!(
            matches!(effect, EffectAst::SubjectVerb(_)),
            "expected typed subject-verb pump, got {effect:?}"
        );
    }
}
