#![allow(dead_code)]

#[allow(unused_imports)]
use crate::ability;
#[allow(unused_imports)]
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, TriggeredAbility};
#[allow(unused_imports)]
use crate::card::PowerToughness;
#[allow(unused_imports)]
use crate::cards::CardDefinition;
#[allow(unused_imports)]
#[allow(unused_imports)]
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, ClashOpponentAst, ControlDurationAst, DamageBySpec,
    EffectAst, EffectLoweringContext, ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst,
    GrantedAbilityAst, IT_TAG, IdGenContext, IfResultPredicate, LineAst, LoweringFrame,
    NormalizedLine, ObjectRefAst, ParseAnnotations, PlayerAst, PredicateAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, RetargetModeAst,
    ReturnControllerAst, SharedTypeConstraintAst, TagKey, TargetAst, TriggerSpec,
};
#[allow(unused_imports)]
use crate::color::ColorSet;
#[allow(unused_imports)]
use crate::cost::TotalCost;
#[allow(unused_imports)]
use crate::effect::{
    ChoiceCount, Condition, Effect, EffectId, EffectMode, EffectPredicate, EmblemDescription,
    EventValueSpec, Until, Value,
};
#[allow(unused_imports)]
use crate::effects::composition::VoteOption;
#[allow(unused_imports)]
use crate::events::cause::CauseFilter;
#[allow(unused_imports)]
use crate::filter::{
    ObjectFilter, ObjectRef, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
#[allow(unused_imports)]
use crate::ids::CardId;
#[allow(unused_imports)]
use crate::mana::{ManaCost, ManaSymbol};
#[allow(unused_imports)]
use crate::static_abilities::StaticAbility;
#[allow(unused_imports)]
use crate::target::ChooseSpec;
#[allow(unused_imports)]
use crate::triggers::Trigger;
#[allow(unused_imports)]
use crate::types::{CardType, Subtype};
#[allow(unused_imports)]
use crate::zone::Zone;
#[allow(unused_imports)]
use std::collections::HashMap;

use super::activation_and_restrictions::{contains_word_sequence, find_word_sequence_start};
use super::token_primitives::{
    find_index, find_window_by, slice_contains, str_contains, str_find, str_split_once,
    str_split_once_char, str_starts_with, str_strip_suffix,
};

use super::effect_ast_traversal::{
    assert_effect_ast_variant_coverage, for_each_nested_effects, for_each_nested_effects_mut,
};
use super::effect_pipeline::{
    EffectPreludeTag, PreparedEffectsForLowering, PreparedPredicateForLowering,
    PreparedTriggeredEffectsForLowering,
};
use super::effect_sentences::parse_subtype_word;
use super::lowering_support::{
    rewrite_lower_parsed_ability as lower_parsed_ability, rewrite_prepare_effects_for_lowering,
    rewrite_prepare_effects_with_trigger_context_for_lowering,
};
use super::reference_helpers::{
    choose_spec_targets_object, infer_player_filter_from_object_filter,
    object_filter_as_tagged_reference, resolve_attach_object_spec, resolve_it_tag,
    resolve_it_tag_key, resolve_non_target_player_filter, resolve_restriction_it_tag,
    resolve_target_spec_with_choices, resolve_unless_player_filter, resolve_value_it_tag,
    watch_tag_from_filter,
};
use super::reference_model::{
    AnnotatedEffect, AnnotatedEffectSequence, LoweredEffects, ReferenceEnv, ReferenceExports,
    ReferenceImports,
};
use super::reference_resolution::{EffectReferenceResolutionConfig, annotate_effect_sequence};
use super::static_ability_helpers::lower_granted_abilities_ast;
use super::util::{
    contains_until_end_of_turn, map_span_to_original, parse_card_type, parse_number_word_i32,
};

#[path = "compile_support/control_flow_handlers.rs"]
mod control_flow_handlers;
#[path = "compile_support/effect_combat_resource_handlers.rs"]
mod effect_combat_resource_handlers;
#[path = "compile_support/effect_continuous_turn_handlers.rs"]
mod effect_continuous_turn_handlers;
#[path = "compile_support/effect_dispatch.rs"]
mod effect_dispatch;
#[path = "compile_support/effect_flow_search_handlers.rs"]
mod effect_flow_search_handlers;
#[path = "compile_support/effect_handlers.rs"]
mod effect_handlers;
#[path = "compile_support/effect_visibility_object_handlers.rs"]
mod effect_visibility_object_handlers;
#[path = "compile_support/iterated_player_validation.rs"]
mod iterated_player_validation;
#[path = "compile_support/prepared_effects.rs"]
mod prepared_effects;
#[path = "compile_support/tag_support.rs"]
mod tag_support;
#[path = "compile_support/trigger_support.rs"]
mod trigger_support;

pub(crate) use control_flow_handlers::{
    choose_spec_for_targeted_player_filter, collect_targeted_player_specs_from_filter,
    compile_effects_in_iterated_player_context, compile_effects_preserving_last_effect,
    compile_if_do_with_opponent_did, compile_if_do_with_opponent_doesnt,
    compile_if_do_with_player_did, compile_if_do_with_player_doesnt, compile_repeat_process_body,
    compile_vote_sequence, effect_predicate_from_if_result,
    force_implicit_vote_token_controller_you, target_context_prelude_for_filter,
    with_preserved_lowering_context,
};
pub(crate) use effect_dispatch::compile_effect;
pub(crate) use iterated_player_validation::validate_iterated_player_bindings_in_lowered_effects;
pub(crate) use prepared_effects::{
    compile_condition_from_predicate_ast_with_env, compile_effect_prelude_tags,
    compile_prepared_predicate_for_lowering, compile_statement_effects,
    compile_statement_effects_with_imports, materialize_prepared_effects_with_trigger_context,
    materialize_prepared_statement_effects, materialize_prepared_triggered_effects,
};
pub(crate) use tag_support::{
    choose_spec_references_exiled_tag, choose_spec_references_tag, collect_tag_spans_from_effect,
    collect_tag_spans_from_effects_with_context, collect_tag_spans_from_target,
    effect_references_event_derived_amount, effect_references_it_tag,
    effect_references_its_controller, effect_references_tag, effects_reference_it_tag,
    effects_reference_its_controller, effects_reference_tag, object_ref_references_tag,
    player_filter_references_tag, restriction_references_tag, target_references_tag,
    value_references_event_derived_amount, value_references_tag,
};
pub(crate) use trigger_support::{
    compile_trigger_effects, compile_trigger_effects_with_imports, compile_trigger_spec,
    ensure_concrete_trigger_spec, inferred_trigger_player_filter,
    trigger_binds_player_reference_context, trigger_supports_event_value,
};

pub(crate) fn compile_condition_from_predicate_ast(
    predicate: &PredicateAst,
    ctx: &mut EffectLoweringContext,
    saved_last_tag: &Option<String>,
) -> Result<Condition, CardTextError> {
    let refs = current_reference_env(ctx);
    Ok(match predicate {
        PredicateAst::ItIsLandCard => {
            let mut filter = ObjectFilter {
                zone: None,
                card_types: vec![CardType::Land],
                ..Default::default()
            };
            filter.zone = None;
            if let Some(tag) = saved_last_tag.clone() {
                Condition::TaggedObjectMatches(tag.into(), filter)
            } else {
                Condition::TargetMatches(filter)
            }
        }
        PredicateAst::ItIsSoulbondPaired => {
            if let Some(tag) = saved_last_tag.clone() {
                Condition::TaggedObjectIsSoulbondPaired(tag.into())
            } else {
                Condition::TargetIsSoulbondPaired
            }
        }
        PredicateAst::ItMatches(filter) => {
            let mut resolved = filter.clone();
            resolved.zone = None;
            if let Some(tag) = saved_last_tag.clone() {
                Condition::TaggedObjectMatches(tag.into(), resolved)
            } else {
                Condition::TargetMatches(resolved)
            }
        }
        PredicateAst::TaggedMatches(tag, filter) => {
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::TaggedObjectMatches(resolved_tag, resolve_it_tag(filter, &refs)?)
        }
        PredicateAst::EnchantedPermanentAttackedThisTurn => {
            Condition::EnchantedPermanentAttackedThisTurn
        }
        PredicateAst::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::PlayerTaggedObjectMatches {
                player,
                tag: resolved_tag,
                filter: resolve_it_tag(filter, &refs)?,
            }
        }
        PredicateAst::PlayerControls { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerControls {
                player,
                filter: resolved,
            }
        }
        PredicateAst::VoteOptionGetsMoreVotes { option } => {
            Condition::VoteOptionGetsMoreVotes(option.clone())
        }
        PredicateAst::VoteOptionGetsMoreVotesOrTied { option } => {
            Condition::VoteOptionGetsMoreVotesOrTied(option.clone())
        }
        PredicateAst::NoVoteObjectsMatched { filter } => {
            Condition::Not(Box::new(Condition::TaggedObjectMatches(
                crate::effects::VOTED_OBJECTS_TAG.into(),
                resolve_it_tag(filter, &refs)?,
            )))
        }
        PredicateAst::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerControlsAtLeast {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::PlayerControlsExactly {
            player,
            filter,
            count,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerControlsExactly {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerControlsAtLeastWithDifferentPowers {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::PlayerControlsOrHasCardInGraveyard {
            player,
            control_filter,
            graveyard_filter,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved_control = resolve_it_tag(control_filter, &refs)?;
            resolved_control.zone = None;
            let resolved_graveyard = resolve_it_tag(graveyard_filter, &refs)?;
            Condition::Or(
                Box::new(Condition::PlayerControls {
                    player: player.clone(),
                    filter: resolved_control,
                }),
                Box::new(Condition::PlayerControls {
                    player,
                    filter: resolved_graveyard,
                }),
            )
        }
        PredicateAst::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerOwnsCardNamedInZones {
                player,
                name: name.clone(),
                zones: zones.clone(),
            }
        }
        PredicateAst::PlayerControlsNo { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::Not(Box::new(Condition::PlayerControls {
                player,
                filter: resolved,
            }))
        }
        PredicateAst::PlayerControlsMost { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControlsMost {
                player,
                filter: resolved,
            }
        }
        PredicateAst::PlayerControlsMoreThanYou { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControlsMoreThanYou {
                player,
                filter: resolved,
            }
        }
        PredicateAst::AnOpponentControlsMoreThanPlayer { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::AnOpponentControlsMoreThanPlayer {
                player,
                filter: resolved,
            }
        }
        PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerLifeAtMostHalfStartingLifeTotal { player }
        }
        PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerLifeLessThanHalfStartingLifeTotal { player }
        }
        PredicateAst::PlayerHasLessLifeThanYou { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasLessLifeThanYou { player }
        }
        PredicateAst::PlayerHasMoreLifeThanYou { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreLifeThanYou { player }
        }
        PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasNoOpponentWithMoreLifeThan { player }
        }
        PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreLifeThanEachOtherPlayer { player }
        }
        PredicateAst::PlayerIsMonarch { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerIsMonarch { player }
        }
        PredicateAst::PlayerHasInitiative { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasInitiative { player }
        }
        PredicateAst::PlayerHasCitysBlessing { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasCitysBlessing { player }
        }
        PredicateAst::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCompletedDungeon {
                player,
                dungeon_name: dungeon_name.clone(),
            }
        }
        PredicateAst::PlayerTappedLandForManaThisTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerTappedLandForManaThisTurn { player }
        }
        PredicateAst::PlayerGainedLifeThisTurnOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerGainedLifeThisTurnOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::CreatureDiedThisTurnOrMore(count) => {
            Condition::CreatureDiedThisTurnOrMore(*count)
        }
        PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHadLandEnterBattlefieldThisTurn { player }
        }
        PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn {
                player,
                tag: tag.clone(),
            }
        }
        PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerControlsBasicLandTypesAmongLandsOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasCardTypesInGraveyardOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::PlayerCardsInHandOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandOrMore {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::PlayerCardsInHandOrFewer { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandOrFewer {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::PlayerHasMoreCardsInHandThanYou { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreCardsInHandThanYou { player }
        }
        PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player }
        }
        PredicateAst::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCastSpellsThisTurnOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::YouHaveNoCardsInHand => {
            Condition::Not(Box::new(Condition::CardsInHandOrMore(1)))
        }
        PredicateAst::YourTurn => Condition::YourTurn,
        PredicateAst::CreatureDiedThisTurn => Condition::CreatureDiedThisTurn,
        PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn => {
            Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        }
        PredicateAst::SourceIsTapped => Condition::SourceIsTapped,
        PredicateAst::SourceIsSaddled => Condition::SourceIsSaddled,
        PredicateAst::SourceHasNoCounter(counter_type) => {
            Condition::SourceHasNoCounter(*counter_type)
        }
        PredicateAst::TriggeringObjectHadNoCounter(counter_type) => {
            Condition::Not(Box::new(Condition::TriggeringObjectHadCounters {
                counter_type: *counter_type,
                min_count: 1,
            }))
        }
        PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        } => Condition::SourceHasCounterAtLeast {
            counter_type: *counter_type,
            count: *count,
        },
        PredicateAst::SourcePowerAtLeast(count) => Condition::SourcePowerAtLeast(*count),
        PredicateAst::SourceAttackedOrBlockedThisTurn => Condition::SourceAttackedOrBlockedThisTurn,
        PredicateAst::SourceIsInZone(zone) => Condition::SourceIsInZone(*zone),
        PredicateAst::YouAttackedThisTurn => Condition::AttackedThisTurn,
        PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count) => {
            return Err(CardTextError::ParseError(format!(
                "attack-count combat predicate should have been lowered into an exact attack trigger before condition compilation (count: {count})"
            )));
        }
        PredicateAst::SourceWasCast => Condition::SourceWasCast,
        PredicateAst::NoSpellsWereCastLastTurn => Condition::NoSpellsWereCastLastTurn,
        PredicateAst::YouHaveFullParty => Condition::YouHaveFullParty,
        PredicateAst::ThisSpellWasKicked => Condition::ThisSpellWasKicked,
        PredicateAst::ThisSpellPaidLabel(label) => Condition::ThisSpellPaidLabel(label.clone()),
        PredicateAst::TargetWasKicked => Condition::TargetWasKicked,
        PredicateAst::TargetSpellCastOrderThisTurn(order) => {
            Condition::TargetSpellCastOrderThisTurn(*order)
        }
        PredicateAst::TargetSpellControllerIsPoisoned => Condition::TargetSpellControllerIsPoisoned,
        PredicateAst::TargetSpellNoManaSpentToCast => {
            Condition::Not(Box::new(Condition::TargetSpellManaSpentToCastAtLeast {
                amount: 1,
                symbol: None,
            }))
        }
        PredicateAst::YouControlMoreCreaturesThanTargetSpellController => {
            Condition::YouControlMoreCreaturesThanTargetSpellController
        }
        PredicateAst::TargetIsBlocked => Condition::TargetIsBlocked,
        PredicateAst::TargetHasGreatestPowerAmongCreatures => {
            Condition::TargetHasGreatestPowerAmongCreatures
        }
        PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell => {
            Condition::TargetManaValueLteColorsSpentToCastThisSpell
        }
        PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            Condition::ManaSpentToCastThisSpellAtLeast {
                amount: *amount,
                symbol: *symbol,
            }
        }
        PredicateAst::ThisSpellWasCastFromZone(zone) => Condition::ThisSpellWasCastFromZone(*zone),
        PredicateAst::ValueComparison {
            left,
            operator,
            right,
        } => {
            if let (
                Value::X,
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(amount),
            ) = (left, operator, right)
                && *amount >= 0
            {
                Condition::XValueAtLeast(*amount as u32)
            } else {
                Condition::ValueComparison {
                    left: resolve_value_it_tag(left, &refs)?,
                    operator: *operator,
                    right: resolve_value_it_tag(right, &refs)?,
                }
            }
        }
        PredicateAst::Unmodeled(text) => Condition::Unmodeled(text.clone()),
        PredicateAst::Not(inner) => {
            let inner = compile_condition_from_predicate_ast(inner, ctx, saved_last_tag)?;
            Condition::Not(Box::new(inner))
        }
        PredicateAst::And(left, right) => {
            let left = compile_condition_from_predicate_ast(left, ctx, saved_last_tag)?;
            let right = compile_condition_from_predicate_ast(right, ctx, saved_last_tag)?;
            Condition::And(Box::new(left), Box::new(right))
        }
    })
}

pub(crate) fn compile_effects(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let annotated = annotate_effect_sequence(
        effects,
        &ReferenceImports::from_lowering_frame(&ctx.lowering_frame()),
        EffectReferenceResolutionConfig {
            allow_life_event_value: ctx.allow_life_event_value,
            bind_unbound_x_to_last_effect: ctx.bind_unbound_x_to_last_effect,
            initial_last_effect_id: ctx.last_effect_id,
            initial_iterated_player: ctx.iterated_player,
            force_auto_tag_object_targets: ctx.force_auto_tag_object_targets
                || ctx.auto_tag_object_targets,
        },
        ctx.id_gen_context(),
    )?;
    compile_annotated_effects_with_context(&annotated, ctx)
}

pub(crate) fn compile_annotated_effects_with_context(
    annotated: &AnnotatedEffectSequence,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut compiled = Vec::new();
    let mut choices = Vec::new();
    let mut idx = 0;
    let effective_force_auto_tag_object_targets =
        ctx.force_auto_tag_object_targets || ctx.auto_tag_object_targets;

    while idx < annotated.effects.len() {
        let current = &annotated.effects[idx];
        apply_local_reference_env(ctx, &current.in_env);
        ctx.auto_tag_object_targets =
            effective_force_auto_tag_object_targets || current.auto_tag_object_targets;

        if let Some((effect_sequence, effect_choices, consumed)) =
            compile_vote_sequence(&annotated.effects[idx..], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            apply_local_reference_env(ctx, &annotated.effects[idx + consumed - 1].out_env);
            idx += consumed;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_opponent_doesnt(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_player_doesnt(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_opponent_did(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_player_did(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        let (mut effect_list, effect_choices) = compile_effect(&current.effect, ctx)?;
        if let Some(id) = current.assigned_effect_id {
            if !effect_list.is_empty() {
                assign_effect_result_id(
                    &mut effect_list,
                    id,
                    "missing final effect while assigning event id (annotated effect)",
                )?;
            }
        }
        let effect_list_is_empty = effect_list.is_empty();
        compiled.extend(effect_list);
        for choice in effect_choices {
            push_choice(&mut choices, choice);
        }
        let mut frame_out = current.out_env.to_lowering_frame(false, false);
        if current.assigned_effect_id.is_some() && effect_list_is_empty {
            frame_out.last_effect_id = None;
        }
        ctx.apply_reference_frame(frame_out);
        idx += 1;
    }

    let compiled = prepend_missing_target_choice_prelude(compiled, &choices);
    Ok((compiled, choices))
}

fn assign_effect_result_id(
    effects: &mut Vec<Effect>,
    id: EffectId,
    error_message: &str,
) -> Result<(), CardTextError> {
    let Some(last) = effects.pop() else {
        return Err(CardTextError::InvariantViolation(error_message.to_string()));
    };
    effects.push(Effect::with_id(id.0, last));
    Ok(())
}

pub(crate) fn compile_effects_with_explicit_frame(
    effects: &[EffectAst],
    id_gen: &mut IdGenContext,
    frame: LoweringFrame,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>, LoweringFrame), CardTextError> {
    let mut ctx = EffectLoweringContext::from_parts(id_gen.clone(), frame);
    let (compiled, choices) = compile_effects(effects, &mut ctx)?;
    *id_gen = ctx.id_gen_context();
    let frame_out = ctx.lowering_frame();
    Ok((compiled, choices, frame_out))
}

fn prepend_missing_target_choice_prelude(
    mut compiled: Vec<Effect>,
    choices: &[ChooseSpec],
) -> Vec<Effect> {
    let mut prelude = Vec::new();
    for choice in choices {
        if !choice.is_target() {
            continue;
        }
        let already_exposed = compiled.iter().any(|effect| {
            effect
                .0
                .get_target_spec()
                .is_some_and(|spec| spec == choice)
        });
        if !already_exposed {
            prelude.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    if prelude.is_empty() {
        return compiled;
    }
    prelude.append(&mut compiled);
    prelude
}

fn preserve_chooser_relative_player_filters(
    original: &ObjectFilter,
    resolved: &mut ObjectFilter,
    chooser: &PlayerFilter,
) {
    if !matches!(
        chooser,
        PlayerFilter::Target(_) | PlayerFilter::IteratedPlayer
    ) {
        return;
    }

    if matches!(original.owner, Some(PlayerFilter::IteratedPlayer)) {
        resolved.owner = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(original.controller, Some(PlayerFilter::IteratedPlayer)) {
        resolved.controller = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(original.cast_by, Some(PlayerFilter::IteratedPlayer)) {
        resolved.cast_by = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(original.targets_player, Some(PlayerFilter::IteratedPlayer)) {
        resolved.targets_player = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(
        original.targets_only_player,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        resolved.targets_only_player = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(
        original.attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        resolved.attacking_player_or_planeswalker_controlled_by =
            Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(
        original.entered_battlefield_controller,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        resolved.entered_battlefield_controller = Some(PlayerFilter::IteratedPlayer);
    }
    if let (Some(original_targets), Some(resolved_targets)) = (
        original.targets_object.as_deref(),
        resolved.targets_object.as_deref_mut(),
    ) {
        preserve_chooser_relative_player_filters(original_targets, resolved_targets, chooser);
    }
    if let (Some(original_targets), Some(resolved_targets)) = (
        original.targets_only_object.as_deref(),
        resolved.targets_only_object.as_deref_mut(),
    ) {
        preserve_chooser_relative_player_filters(original_targets, resolved_targets, chooser);
    }
    for (original_any_of, resolved_any_of) in original.any_of.iter().zip(resolved.any_of.iter_mut())
    {
        preserve_chooser_relative_player_filters(original_any_of, resolved_any_of, chooser);
    }
}

fn bind_relative_iterated_player_filters_to_chooser(
    filter: &mut ObjectFilter,
    chooser: &PlayerFilter,
) {
    if matches!(chooser, PlayerFilter::IteratedPlayer) {
        return;
    }

    if matches!(filter.owner, Some(PlayerFilter::IteratedPlayer)) {
        filter.owner = Some(chooser.clone());
    }
    if matches!(filter.controller, Some(PlayerFilter::IteratedPlayer)) {
        filter.controller = Some(chooser.clone());
    }
    if matches!(filter.cast_by, Some(PlayerFilter::IteratedPlayer)) {
        filter.cast_by = Some(chooser.clone());
    }
    if matches!(filter.targets_player, Some(PlayerFilter::IteratedPlayer)) {
        filter.targets_player = Some(chooser.clone());
    }
    if matches!(
        filter.targets_only_player,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        filter.targets_only_player = Some(chooser.clone());
    }
    if matches!(
        filter.attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        filter.attacking_player_or_planeswalker_controlled_by = Some(chooser.clone());
    }
    if matches!(
        filter.entered_battlefield_controller,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        filter.entered_battlefield_controller = Some(chooser.clone());
    }
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        bind_relative_iterated_player_filters_to_chooser(targets, chooser);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        bind_relative_iterated_player_filters_to_chooser(targets, chooser);
    }
    for any_of in &mut filter.any_of {
        bind_relative_iterated_player_filters_to_chooser(any_of, chooser);
    }
}

fn bind_relative_iterated_player_to_last_player_filter(
    player_filter: &mut PlayerFilter,
    filter: &mut ObjectFilter,
    last_player_filter: &PlayerFilter,
) {
    if last_player_filter.mentions_iterated_player() {
        return;
    }

    if matches!(player_filter, PlayerFilter::IteratedPlayer) {
        *player_filter = last_player_filter.clone();
    }
    bind_relative_iterated_player_filters_to_chooser(filter, last_player_filter);
}

fn bind_relative_iterated_player_in_value_to_player_filter(
    value: &mut Value,
    player_filter: &PlayerFilter,
) {
    match value {
        Value::Add(left, right) => {
            bind_relative_iterated_player_in_value_to_player_filter(left, player_filter);
            bind_relative_iterated_player_in_value_to_player_filter(right, player_filter);
        }
        Value::Scaled(inner, _) => {
            bind_relative_iterated_player_in_value_to_player_filter(inner, player_filter);
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter) => {
            bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
        }
        Value::CreaturesDiedThisTurnControlledBy(filter) => {
            if matches!(filter, PlayerFilter::IteratedPlayer)
                && !matches!(player_filter, PlayerFilter::IteratedPlayer)
            {
                *filter = player_filter.clone();
            }
        }
        _ => {}
    }
}

fn choose_followup_player_filter(
    filter: &ObjectFilter,
    chooser: &PlayerFilter,
) -> Option<PlayerFilter> {
    let inferred = infer_player_filter_from_object_filter(filter);
    if inferred
        .as_ref()
        .is_some_and(PlayerFilter::mentions_iterated_player)
        && matches!(
            chooser,
            PlayerFilter::Target(_) | PlayerFilter::Opponent | PlayerFilter::Specific(_)
        )
    {
        Some(chooser.clone())
    } else {
        inferred.or_else(|| Some(chooser.clone()))
    }
}

pub(crate) fn hand_exile_filter_and_count(
    target: &TargetAst,
    ctx: &EffectLoweringContext,
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let (filter, count) = match target {
        TargetAst::Object(filter, _, _) => (filter, ChoiceCount::exactly(1)),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => (filter, *count),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
    if resolved_filter.zone != Some(Zone::Hand) {
        return Ok(None);
    }
    Ok(Some((resolved_filter, count)))
}

pub(crate) fn lower_hand_exile_target(
    target: &TargetAst,
    face_down: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let Some((mut filter, count)) = hand_exile_filter_and_count(target, ctx)? else {
        return Ok(None);
    };

    let mut chooser = filter
        .owner
        .clone()
        .or_else(|| filter.controller.clone())
        .unwrap_or(PlayerFilter::You);

    if ctx.iterated_player && matches!(chooser, PlayerFilter::Target(_)) {
        chooser = PlayerFilter::IteratedPlayer;
        if matches!(filter.owner, Some(PlayerFilter::Target(_))) {
            filter.owner = Some(PlayerFilter::IteratedPlayer);
        }
        if matches!(filter.controller, Some(PlayerFilter::Target(_))) {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
    } else {
        bind_relative_iterated_player_filters_to_chooser(&mut filter, &chooser);
    }

    let (mut prelude, choices) = target_context_prelude_for_filter(&filter);
    let tag = ctx.next_tag("exiled");
    let tag_key: TagKey = tag.as_str().into();
    ctx.last_object_tag = Some(tag.clone());
    ctx.last_player_filter = Some(chooser.clone());

    prelude.push(Effect::new(
        crate::effects::ChooseObjectsEffect::new(filter, count, chooser, tag_key.clone())
            .in_zone(Zone::Hand),
    ));
    prelude.push(Effect::new(
        crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag_key))
            .with_face_down(face_down),
    ));
    Ok(Some((prelude, choices)))
}

pub(crate) fn lower_counted_non_target_exile_target(
    target: &TargetAst,
    face_down: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let (filter, count) = match target {
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() && !count.is_single() =>
            {
                (filter, *count)
            }
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
    let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
    if choice_zone != Zone::Library {
        return Ok(None);
    }

    let mut chooser = resolved_filter
        .owner
        .clone()
        .or_else(|| resolved_filter.controller.clone())
        .unwrap_or(PlayerFilter::You);

    if ctx.iterated_player && matches!(chooser, PlayerFilter::Target(_)) {
        chooser = PlayerFilter::IteratedPlayer;
        if matches!(resolved_filter.owner, Some(PlayerFilter::Target(_))) {
            resolved_filter.owner = Some(PlayerFilter::IteratedPlayer);
        }
        if matches!(resolved_filter.controller, Some(PlayerFilter::Target(_))) {
            resolved_filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
    } else {
        bind_relative_iterated_player_filters_to_chooser(&mut resolved_filter, &chooser);
    }

    if choice_zone == Zone::Battlefield
        && resolved_filter.controller.is_none()
        && resolved_filter.tagged_constraints.is_empty()
    {
        resolved_filter.controller = Some(chooser.clone());
    }

    let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
    let tag = ctx.next_tag("exiled");
    let tag_key: TagKey = tag.as_str().into();
    ctx.last_object_tag = Some(tag.clone());
    ctx.last_player_filter = Some(chooser.clone());

    prelude.push(Effect::new(
        crate::effects::ChooseObjectsEffect::new(resolved_filter, count, chooser, tag_key.clone())
            .in_zone(choice_zone)
            .top_only(),
    ));
    prelude.push(Effect::new(
        crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag_key))
            .with_face_down(face_down),
    ));
    Ok(Some((prelude, choices)))
}

pub(crate) fn lower_single_non_target_exile_target(
    target: &TargetAst,
    face_down: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let (filter, count) = match target {
        TargetAst::Object(filter, explicit_target_span, _) if explicit_target_span.is_none() => {
            (filter, ChoiceCount::exactly(1))
        }
        TargetAst::WithCount(inner, count) if count.is_single() => match inner.as_ref() {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() =>
            {
                (filter, *count)
            }
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
    let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
    if choice_zone != Zone::Library {
        return Ok(None);
    }

    let mut chooser = resolved_filter
        .owner
        .clone()
        .or_else(|| resolved_filter.controller.clone())
        .unwrap_or(PlayerFilter::You);

    if ctx.iterated_player && matches!(chooser, PlayerFilter::Target(_)) {
        chooser = PlayerFilter::IteratedPlayer;
        if matches!(resolved_filter.owner, Some(PlayerFilter::Target(_))) {
            resolved_filter.owner = Some(PlayerFilter::IteratedPlayer);
        }
        if matches!(resolved_filter.controller, Some(PlayerFilter::Target(_))) {
            resolved_filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
    } else {
        bind_relative_iterated_player_filters_to_chooser(&mut resolved_filter, &chooser);
    }

    let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
    let tag = ctx.next_tag("exiled");
    let tag_key: TagKey = tag.as_str().into();
    ctx.last_object_tag = Some(tag.clone());
    ctx.last_player_filter = Some(chooser.clone());

    let choose =
        crate::effects::ChooseObjectsEffect::new(resolved_filter, count, chooser, tag_key.clone())
            .in_zone(choice_zone)
            .top_only();

    prelude.push(Effect::new(choose));
    prelude.push(Effect::new(
        crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag_key))
            .with_face_down(face_down),
    ));
    Ok(Some((prelude, choices)))
}

pub(crate) fn lower_may_imprint_from_hand_effect(
    effects: &[EffectAst],
    ctx: &EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    if effects.len() != 1 {
        return Ok(None);
    }

    let EffectAst::Exile { target, face_down } = &effects[0] else {
        return Ok(None);
    };
    if *face_down {
        return Ok(None);
    }

    let Some((filter, count)) = hand_exile_filter_and_count(target, ctx)? else {
        return Ok(None);
    };
    if !count.is_single() {
        return Ok(None);
    }

    Ok(Some((
        vec![Effect::new(
            crate::effects::cards::ImprintFromHandEffect::new(filter),
        )],
        Vec::new(),
    )))
}

pub(crate) fn resolve_effect_player_filter(
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
) -> Result<(PlayerFilter, Vec<ChooseSpec>), CardTextError> {
    let refs = current_reference_env(ctx);
    let (filter, choices) = match player {
        PlayerAst::Target if allow_target => (
            PlayerFilter::target_player(),
            vec![ChooseSpec::target_player()],
        ),
        PlayerAst::TargetOpponent if allow_target_opponent => (
            PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
            vec![ChooseSpec::target(ChooseSpec::Player(
                PlayerFilter::Opponent,
            ))],
        ),
        _ => (resolve_non_target_player_filter(player, &refs)?, Vec::new()),
    };

    if track_last_player_filter && !matches!(player, PlayerAst::Implicit) {
        ctx.last_player_filter = Some(filter.clone());
    }
    Ok((filter, choices))
}

pub(crate) fn compile_player_effect<YouBuilder, OtherBuilder>(
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce() -> Effect,
    OtherBuilder: FnOnce(PlayerFilter) -> Effect,
{
    let (filter, choices) =
        resolve_effect_player_filter(player, ctx, allow_target, allow_target, true)?;
    let effect = if matches!(&filter, PlayerFilter::You) {
        build_you()
    } else {
        build_other(filter)
    };
    let mut effects = Vec::new();
    // Only inject explicit target-context effects when the payload effect itself
    // does not expose target metadata via get_target_spec().
    if effect.0.get_target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

fn try_compile_simultaneous_each_player_scry(
    player_filter: PlayerFilter,
    inner_effects: &[Effect],
) -> Option<Effect> {
    if inner_effects.len() != 1 {
        return None;
    }
    let scry = inner_effects[0].downcast_ref::<crate::effects::ScryEffect>()?;
    if scry.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    Some(Effect::new(crate::effects::EachPlayerScryEffect::new(
        scry.count.clone(),
        player_filter,
    )))
}

fn compile_emblem_description_from_text(text: &str) -> Result<EmblemDescription, CardTextError> {
    let definition =
        CardDefinitionBuilder::new(CardId::new(), "Emblem").parse_text(text.to_string())?;
    Ok(EmblemDescription {
        name: "Emblem".to_string(),
        text: text.to_string(),
        abilities: definition.abilities,
    })
}

pub(crate) fn compile_player_effect_with_generated_object_tag<YouBuilder, OtherBuilder>(
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    tag_prefix: &str,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce() -> Effect,
    OtherBuilder: FnOnce(PlayerFilter) -> Effect,
{
    let (filter, choices) =
        resolve_effect_player_filter(player, ctx, allow_target, allow_target, true)?;
    let effect = if matches!(&filter, PlayerFilter::You) {
        build_you()
    } else {
        build_other(filter)
    };
    let mut effects = Vec::new();
    if effect.0.get_target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    if ctx.auto_tag_object_targets {
        let tag = ctx.next_tag(tag_prefix);
        effects.push(effect.tag(tag.clone()));
        ctx.last_object_tag = Some(tag);
    } else {
        effects.push(effect);
    }
    Ok((effects, choices))
}

pub(crate) fn compile_player_effect_from_filter<Builder>(
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(PlayerFilter) -> Effect,
{
    let (filter, choices) =
        resolve_effect_player_filter(player, ctx, allow_target, allow_target, true)?;
    let mut effects = Vec::new();
    let effect = build(filter);
    // Only inject explicit target-context effects when the payload effect itself
    // does not expose target metadata via get_target_spec().
    if effect.0.get_target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

fn compile_exchange_life_totals_effect(
    player1: PlayerAst,
    player2: PlayerAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (filter1, choices1) = resolve_effect_player_filter(player1, ctx, true, true, true)?;
    let (filter2, choices2) = resolve_effect_player_filter(player2, ctx, true, true, true)?;

    let effect = Effect::exchange_life_totals(filter1, filter2);
    let mut choices = Vec::new();

    if choices1.len() == 1
        && choices2.len() == 1
        && choices1[0].base() == choices2[0].base()
        && choices1[0].is_target()
    {
        push_choice(
            &mut choices,
            choices1[0].clone().with_count(ChoiceCount::exactly(2)),
        );
    } else {
        for choice in choices1.into_iter().chain(choices2) {
            push_choice(&mut choices, choice);
        }
    }

    let mut effects = Vec::new();
    if effect.0.get_target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

fn compile_exchange_control_heterogeneous_effect(
    permanent1: &TargetAst,
    permanent2: &TargetAst,
    shared_type: Option<SharedTypeConstraintAst>,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (spec1, mut choices) =
        resolve_target_spec_with_choices(permanent1, &current_reference_env(ctx))?;
    let reference_tag = ctx.next_tag("exchange_first");
    let original_last_object_tag = ctx.last_object_tag.clone();
    ctx.last_object_tag = Some(reference_tag.clone());
    let (spec2, other_choices) =
        resolve_target_spec_with_choices(permanent2, &current_reference_env(ctx))?;
    ctx.last_object_tag = original_last_object_tag;
    for choice in other_choices {
        push_choice(&mut choices, choice);
    }

    let exchange = crate::effects::ExchangeControlEffect::new(spec1, spec2)
        .with_permanent1_reference_tag(reference_tag);
    let exchange = if let Some(shared_type) = shared_type {
        let constraint = match shared_type {
            SharedTypeConstraintAst::CardType => crate::effects::SharedTypeConstraint::CardType,
            SharedTypeConstraintAst::PermanentType => {
                crate::effects::SharedTypeConstraint::PermanentType
            }
        };
        exchange.with_shared_type(constraint)
    } else {
        exchange
    };

    let mut effect = Effect::new(exchange);
    let tag = ctx.next_tag("exchanged");
    effect = effect.tag(tag.clone());
    ctx.last_object_tag = Some(tag);
    Ok((vec![effect], choices))
}

fn compile_exchange_zones_effect(
    player: PlayerAst,
    zone1: Zone,
    zone2: Zone,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (player_filter, choices) = resolve_effect_player_filter(player, ctx, true, true, true)?;
    let effect = Effect::exchange_zones(player_filter, zone1, zone2);
    let mut effects = Vec::new();
    if effect.0.get_target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

fn compile_exchange_text_boxes_effect(
    target: &TargetAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (spec, choices) = resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
    let effect = Effect::exchange_text_boxes(spec);
    let tag = ctx.next_tag("exchanged");
    ctx.last_object_tag = Some(tag.clone());
    Ok((vec![effect.tag(tag)], choices))
}

fn compile_exchange_value_operand(
    operand: &ExchangeValueAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(crate::effects::ExchangeValueOperand, Vec<ChooseSpec>), CardTextError> {
    match operand {
        ExchangeValueAst::LifeTotal(player) => {
            let (filter, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            Ok((
                crate::effects::ExchangeValueOperand::LifeTotal(filter),
                choices,
            ))
        }
        ExchangeValueAst::Stat { target, kind } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let operand = match kind {
                ExchangeValueKindAst::Power => crate::effects::ExchangeValueOperand::Power(spec),
                ExchangeValueKindAst::Toughness => {
                    crate::effects::ExchangeValueOperand::Toughness(spec)
                }
            };
            Ok((operand, choices))
        }
    }
}

fn compile_exchange_values_effect(
    left: &ExchangeValueAst,
    right: &ExchangeValueAst,
    duration: Until,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (left, mut choices) = compile_exchange_value_operand(left, ctx)?;
    let (right, other_choices) = compile_exchange_value_operand(right, ctx)?;
    for choice in other_choices {
        push_choice(&mut choices, choice);
    }
    let effect = Effect::exchange_values(left, right, duration);
    let mut effects = Vec::new();
    if effect.0.get_target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

fn current_reference_env(ctx: &EffectLoweringContext) -> ReferenceEnv {
    ctx.reference_env().into()
}

fn apply_local_reference_env(ctx: &mut EffectLoweringContext, env: &ReferenceEnv) {
    let reference_env: crate::cards::builders::ReferenceEnv = env.clone().into();
    ctx.apply_reference_env(&reference_env);
}

fn lower_granted_ability_grant_modifications(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<crate::continuous::Modification>, CardTextError> {
    let mut modifications = Vec::with_capacity(abilities.len());
    for ability in abilities {
        match ability {
            GrantedAbilityAst::ParsedObjectAbility { ability, display } => {
                let mut lowered = lower_parsed_ability(ability.clone())?;
                lowered.ability.text = Some(display.clone());
                modifications.push(crate::continuous::Modification::AddAbilityGeneric(
                    lowered.ability,
                ));
            }
            _ => {
                let mut lowered = lower_granted_abilities_ast(std::slice::from_ref(ability))?;
                if let Some(static_ability) = lowered.pop() {
                    modifications.push(crate::continuous::Modification::AddAbility(static_ability));
                }
            }
        }
    }
    Ok(modifications)
}

fn granted_ability_mode_description(
    ability: &GrantedAbilityAst,
    spec: &ChooseSpec,
) -> Result<String, CardTextError> {
    if !matches!(spec, ChooseSpec::Source) {
        return Ok(String::new());
    }

    let display = match ability {
        GrantedAbilityAst::ParsedObjectAbility { display, .. } => display.clone(),
        _ => lower_granted_abilities_ast(std::slice::from_ref(ability))?
            .into_iter()
            .next()
            .map(|ability| ability.display())
            .unwrap_or_default(),
    };

    Ok(format!("This creature gains {display} until end of turn."))
}

pub(crate) fn tagged_alias_for_choice(effects: &[Effect], choice: &ChooseSpec) -> Option<String> {
    for effect in effects {
        let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
            continue;
        };
        if let Some(target_spec) = tagged.effect.0.get_target_spec()
            && target_spec == choice
        {
            return Some(tagged.tag.as_str().to_string());
        }
    }
    None
}

pub(crate) fn tag_object_target_effect(
    effect: Effect,
    spec: &ChooseSpec,
    ctx: &mut EffectLoweringContext,
    prefix: &str,
) -> Effect {
    if ctx.auto_tag_object_targets && choose_spec_targets_object(spec) {
        let tag = ctx.next_tag(prefix);
        ctx.last_object_tag = Some(tag.clone());
        effect.tag(tag)
    } else {
        effect
    }
}

pub(crate) fn eldrazi_spawn_or_scion_mana_ability() -> Ability {
    Ability {
        kind: AbilityKind::Activated(ActivatedAbility::mana_with_costs(
            TotalCost::free(),
            vec![crate::costs::Cost::sacrifice_self()],
            vec![ManaSymbol::Colorless],
        )),
        functional_zones: vec![Zone::Battlefield],
        text: Some("Sacrifice this creature: Add {C}.".to_string()),
    }
}

pub(crate) fn eldrazi_spawn_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Eldrazi Spawn")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Spawn])
        .power_toughness(PowerToughness::fixed(0, 1))
        .with_ability(eldrazi_spawn_or_scion_mana_ability())
        .build()
}

pub(crate) fn eldrazi_scion_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Eldrazi Scion")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Scion])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(eldrazi_spawn_or_scion_mana_ability())
        .build()
}

pub(crate) fn parse_number_word(word: &str) -> Option<i32> {
    parse_number_word_i32(word)
}

pub(crate) fn parse_deals_damage_amount(words: &[&str]) -> Option<i32> {
    let match_idx = find_window_by(words, 3, |window| {
        if (window[0] == "deals" || window[0] == "deal") && window[2] == "damage" {
            return true;
        }
        false
    })?;
    parse_number_word(words[match_idx + 1])
}

pub(crate) fn token_inline_noncreature_spell_each_opponent_damage_amount(
    name: &str,
) -> Option<i32> {
    let lower_name = name.to_ascii_lowercase();
    let words: Vec<&str> = lower_name
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-'
            })
        })
        .filter(|word| !word.is_empty())
        .collect();
    let has_noncreature_cast_trigger = find_window_by(&words, 6, |window| {
        window == ["whenever", "you", "cast", "a", "noncreature", "spell"]
    })
    .is_some()
        || find_window_by(&words, 5, |window| {
            window == ["whenever", "you", "cast", "noncreature", "spell"]
        })
        .is_some();
    if !has_noncreature_cast_trigger {
        return None;
    }
    let has_damage_subject = find_window_by(&words, 3, |window| {
        window == ["this", "token", "deals"]
            || window == ["this", "creature", "deals"]
            || window == ["this", "token", "deal"]
            || window == ["this", "creature", "deal"]
    })
    .is_some()
        || find_window_by(&words, 2, |window| {
            window == ["it", "deals"] || window == ["it", "deal"]
        })
        .is_some();
    if !has_damage_subject {
        return None;
    }
    if find_window_by(&words, 3, |window| window == ["to", "each", "opponent"]).is_none() {
        return None;
    }
    parse_deals_damage_amount(&words)
}

pub(crate) fn parse_crew_amount(words: &[&str]) -> Option<u32> {
    let crew_idx = find_index(words, |word| *word == "crew")?;
    let amount_word = words.get(crew_idx + 1)?;
    let amount = parse_number_word(amount_word)?;
    u32::try_from(amount).ok()
}

pub(crate) fn parse_equip_amount(words: &[&str]) -> Option<u32> {
    let equip_idx = find_index(words, |word| *word == "equip")?;
    let amount_word = words.get(equip_idx + 1)?;
    let amount = parse_number_word(amount_word)?;
    u32::try_from(amount).ok()
}

pub(crate) fn join_simple_and_list(parts: &[&str]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let mut out = parts[..parts.len() - 1].join(", ");
            out.push_str(", and ");
            out.push_str(parts.last().copied().unwrap_or_default());
            out
        }
    }
}

pub(crate) fn parse_equipment_rules_text(words: &[&str], source_text: &str) -> Option<String> {
    let has_equipped_subject = words
        .iter()
        .enumerate()
        .any(|(idx, _)| idx + 2 <= words.len() && words[idx..idx + 2] == ["equipped", "creature"]);
    if !has_equipped_subject {
        return None;
    }

    let mut lines = Vec::new();
    let lower_source = source_text.to_ascii_lowercase();
    if let Some(has_idx) = lower_source.find("equipped creature has ") {
        let ability_start = has_idx + "equipped creature has ".len();
        let ability_tail = &source_text[ability_start..];
        let lower_ability_tail = &lower_source[ability_start..];
        let ability_end = [
            " and equip ",
            "\" and equip ",
            "\"and equip ",
            "' and equip ",
            "'and equip ",
        ]
        .iter()
        .filter_map(|pattern| lower_ability_tail.find(pattern))
        .min()
        .or_else(|| lower_ability_tail.rfind(" equip "))
        .unwrap_or(ability_tail.len());
        let ability_clause = ability_tail[..ability_end].trim();
        if ability_clause.contains(':') {
            let normalized_clause = ability_clause.trim_matches(|ch| ch == '\'' || ch == '"');
            let mut granted_text = normalized_clause.trim_end_matches('.').to_string();
            if !granted_text.ends_with(['.', '!', '?']) {
                granted_text.push('.');
            }
            lines.push(format!("Equipped creature has \"{granted_text}\""));
        }
    }

    if lines.is_empty() {
        let has_plus_one = find_window_by(words, 2, |window| window == ["gets", "+1/+1"]).is_some();
        let mut granted_keywords: Vec<&str> = Vec::new();
        for keyword in [
            "vigilance",
            "trample",
            "haste",
            "flying",
            "lifelink",
            "deathtouch",
            "menace",
            "reach",
            "hexproof",
            "indestructible",
        ] {
            if words.iter().any(|word| *word == keyword) {
                granted_keywords.push(keyword);
            }
        }
        if has_plus_one {
            if granted_keywords.is_empty() {
                lines.push("Equipped creature gets +1/+1.".to_string());
            } else {
                lines.push(format!(
                    "Equipped creature gets +1/+1 and has {}.",
                    join_simple_and_list(&granted_keywords)
                ));
            }
        } else if !granted_keywords.is_empty() {
            lines.push(format!(
                "Equipped creature has {}.",
                join_simple_and_list(&granted_keywords)
            ));
        }
    }

    if let Some(equip_amount) = parse_equip_amount(words) {
        lines.push(format!("Equip {{{equip_amount}}}"));
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn extract_double_quoted_token_rules_text(source_text: &str) -> Option<String> {
    let start = source_text.find('"')?;
    let rest = &source_text[start + 1..];
    let end = rest.find('"')?;
    let quoted = rest[..end].trim();
    if quoted.is_empty() {
        None
    } else {
        Some(quoted.to_string())
    }
}

fn extract_inline_token_rules_text(source_text: &str) -> Option<String> {
    let lower = source_text.to_ascii_lowercase();
    let with_idx = lower.find(" with ")?;
    let tail = source_text.get(with_idx + " with ".len()..)?.trim();
    let tail_lower = tail.to_ascii_lowercase();

    let mut starts = Vec::new();
    for needle in ["whenever ", "when ", "at "] {
        if let Some(idx) = tail_lower.find(needle) {
            starts.push(idx);
        }
    }
    let start = starts.into_iter().min()?;
    let rules_text = tail.get(start..)?.trim();
    if rules_text.is_empty() {
        None
    } else {
        Some(rules_text.to_string())
    }
}

fn normalize_token_self_reference_for_parser(rules_text: &str, self_reference: &str) -> String {
    rules_text
        .replace("This token", self_reference)
        .replace("this token", &self_reference.to_ascii_lowercase())
}

fn try_parse_quoted_token_rules_text(
    builder: &CardDefinitionBuilder,
    source_text: &str,
    self_reference: &str,
) -> Option<CardDefinition> {
    let quoted = extract_double_quoted_token_rules_text(source_text)
        .or_else(|| extract_inline_token_rules_text(source_text))?;
    let quoted_lower = quoted.to_ascii_lowercase();
    let looks_triggered = quoted_lower.starts_with("when ")
        || quoted_lower.starts_with("whenever ")
        || quoted_lower.starts_with("at ");
    let looks_activated = quoted.contains(':');
    if !looks_triggered && !looks_activated {
        return None;
    }

    let normalized = normalize_token_self_reference_for_parser(&quoted, self_reference);
    let base_ability_count = builder.clone().build().abilities.len();
    let mut parsed = builder.clone().parse_text(&normalized).ok()?;
    if parsed.abilities.len() == base_ability_count + 1 {
        if let Some(last) = parsed.abilities.last_mut() {
            let mut original = quoted.trim().to_string();
            if !original.ends_with(['.', '!', '?']) {
                original.push('.');
            }
            last.text = Some(original);
        }
    }
    Some(parsed)
}

pub(crate) fn token_dies_deals_damage_any_target_ability(amount: i32) -> Ability {
    let target = ChooseSpec::AnyTarget;
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::Fixed(amount),
                target.clone(),
            )]),
            choices: vec![target],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "When this token dies, it deals {amount} damage to any target."
        )),
    }
}

pub(crate) fn token_leaves_deals_damage_any_target_ability(amount: i32) -> Ability {
    let target = ChooseSpec::AnyTarget;
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_leaves_battlefield(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::Fixed(amount),
                target.clone(),
            )]),
            choices: vec![target],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "When this token leaves the battlefield, it deals {amount} damage to any target."
        )),
    }
}

pub(crate) fn token_becomes_tapped_deals_damage_target_player_ability(amount: i32) -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::becomes_tapped(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::Fixed(amount),
                target.clone(),
            )]),
            choices: vec![target],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "Whenever this token becomes tapped, it deals {amount} damage to target player."
        )),
    }
}

pub(crate) fn token_dies_target_creature_gets_minus_one_minus_one_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::pump(
                -1,
                -1,
                target.clone(),
                Until::EndOfTurn,
            )]),
            choices: vec![target],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "When this token dies, target creature gets -1/-1 until end of turn.".to_string(),
        ),
    }
}

pub(crate) fn token_red_pump_ability() -> Ability {
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Red]])),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::pump(
                1,
                0,
                ChooseSpec::Source,
                Until::EndOfTurn,
            )]),
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{R}: This creature gets +1/+0 until end of turn.".to_string()),
    }
}

pub(crate) fn token_white_tap_target_creature_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![
                crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::White]])),
                crate::costs::Cost::tap(),
            ]),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::tap(
                target.clone(),
            )]),
            choices: vec![target],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{W}, {T}: Tap target creature.".to_string()),
    }
}

pub(crate) fn token_tap_add_single_mana_ability(symbol: ManaSymbol) -> Ability {
    let mana_text = ManaCost::from_pips(vec![vec![symbol]]).to_oracle();
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::add_mana(
                vec![symbol],
            )]),
            choices: Vec::new(),
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!("{{T}}: Add {mana_text}.")),
    }
}

pub(crate) fn parse_token_tap_add_single_mana_symbol(words: &[&str]) -> Option<ManaSymbol> {
    let add_idx = find_index(words, |word| *word == "add")?;
    if !words[..add_idx].iter().any(|word| *word == "t") {
        return None;
    }
    let symbol = parse_token_mana_symbol(words.get(add_idx + 1).copied()?)?;
    if matches!(symbol, ManaSymbol::Generic(_) | ManaSymbol::X) {
        return None;
    }
    Some(symbol)
}

pub(crate) fn token_damage_to_player_poison_counter_ability() -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_deals_combat_damage_to_player(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::poison_counters_player(1, PlayerFilter::DamagedPlayer),
            ]),
            choices: Vec::new(),
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "Whenever this creature deals damage to a player, that player gets a poison counter."
                .to_string(),
        ),
    }
}

pub(crate) fn token_noncreature_spell_each_opponent_damage_ability(amount: i32) -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::spell_cast(
                Some(ObjectFilter::spell().without_type(CardType::Creature)),
                PlayerFilter::You,
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::for_each_opponent(vec![Effect::deal_damage(
                    Value::Fixed(amount),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                )]),
            ]),
            choices: Vec::new(),
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "Whenever you cast a noncreature spell, this token deals {amount} damage to each opponent."
        )),
    }
}

pub(crate) fn token_combat_damage_gain_control_target_artifact_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::artifact().controlled_by(PlayerFilter::DamagedPlayer),
    ));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_deals_combat_damage_to_player(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    target.clone(),
                    crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
                    Until::Forever,
                ),
            )]),
            choices: vec![target],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "Whenever this token deals combat damage to a player, gain control of target artifact that player controls."
                .to_string(),
        ),
    }
}

pub(crate) fn token_leaves_return_named_from_graveyard_to_hand_ability(card_name: &str) -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    ));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_leaves_battlefield(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::return_from_graveyard_to_hand(target.clone()),
            ]),
            choices: vec![target],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "When this token leaves the battlefield, return target card named {card_name} from your graveyard to your hand."
        )),
    }
}

pub(crate) fn parse_token_mana_symbol(word: &str) -> Option<ManaSymbol> {
    match word {
        "w" => Some(ManaSymbol::White),
        "u" => Some(ManaSymbol::Blue),
        "b" => Some(ManaSymbol::Black),
        "r" => Some(ManaSymbol::Red),
        "g" => Some(ManaSymbol::Green),
        "c" => Some(ManaSymbol::Colorless),
        "x" => Some(ManaSymbol::X),
        _ => word.parse::<u8>().ok().map(ManaSymbol::Generic),
    }
}

pub(crate) fn title_case_words(words: &[&str]) -> String {
    let lowercase_words = [
        "a", "an", "the", "and", "or", "but", "nor", "for", "so", "yet", "of", "in", "on", "at",
        "to", "from", "with", "without", "by", "as", "into", "onto", "over", "under",
    ];
    words
        .iter()
        .filter(|word| !word.is_empty())
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0 && lowercase_words.iter().any(|candidate| candidate == word) {
                return (*word).to_string();
            }
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                let mut out = first.to_uppercase().to_string();
                out.push_str(chars.as_str());
                out
            } else {
                String::new()
            }
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn title_case_phrase_preserving_punctuation(phrase: &str) -> String {
    let lowercase_words = [
        "a", "an", "the", "and", "or", "but", "nor", "for", "so", "yet", "of", "in", "on", "at",
        "to", "from", "with", "without", "by", "as", "into", "onto", "over", "under",
    ];
    phrase
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .enumerate()
        .map(|(idx, word)| {
            let letters_only: String = word
                .chars()
                .filter(|ch| ch.is_ascii_alphabetic())
                .map(|ch| ch.to_ascii_lowercase())
                .collect();
            let keep_lowercase = idx > 0
                && lowercase_words
                    .iter()
                    .any(|candidate| *candidate == letters_only.as_str());
            if keep_lowercase {
                return word.to_string();
            }
            let mut out = String::with_capacity(word.len());
            let mut uppercased = false;
            for ch in word.chars() {
                if !uppercased && ch.is_ascii_alphabetic() {
                    out.extend(ch.to_uppercase());
                    uppercased = true;
                } else {
                    out.push(ch);
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn extract_named_card_name(words: &[&str], source_text: &str) -> Option<String> {
    let named_idx = find_index(words, |word| *word == "named")?;
    if named_idx > 0 && matches!(words[named_idx - 1], "card" | "cards") {
        return None;
    }
    let stop_words = [
        "from",
        "to",
        "and",
        "with",
        "that",
        "thats",
        "it",
        "at",
        "until",
        "if",
        "where",
        "when",
        "whenever",
        "this",
        "token",
        "tokens",
        "tapped",
        "attacking",
        "add",
        "sacrifice",
        "draw",
        "deals",
        "deal",
        "damage",
        "gets",
        "gains",
        "gain",
        "cant",
        "can",
        "attack",
        "block",
        "flying",
        "trample",
        "haste",
        "vigilance",
        "menace",
        "deathtouch",
        "lifelink",
        "reach",
        "hexproof",
        "indestructible",
        "first",
        "double",
        "strike",
        "t",
        "w",
        "u",
        "b",
        "r",
        "g",
        "c",
    ];
    let mut end = named_idx + 1;
    while end < words.len() && !stop_words.iter().any(|candidate| *candidate == words[end]) {
        end += 1;
    }
    if end <= named_idx + 1 {
        return None;
    }
    let name_word_count = end - (named_idx + 1);

    if let Some(named_pos) = str_find(source_text, "named") {
        let after_named = &source_text[named_pos + "named".len()..];
        let raw_words: Vec<&str> = after_named
            .split_whitespace()
            .take(name_word_count)
            .collect();
        if raw_words.len() == name_word_count {
            let raw_name = raw_words.join(" ");
            let titled = title_case_phrase_preserving_punctuation(raw_name.as_str());
            if !titled.is_empty() {
                return Some(titled);
            }
        }
    }

    Some(title_case_words(&words[named_idx + 1..end]))
}

pub(crate) fn extract_leading_explicit_token_name(words: &[&str]) -> Option<String> {
    let is_simple_name_word = |word: &str| {
        word.chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
    };
    let is_descriptor = |word: &str| {
        matches!(
            word,
            "legendary"
                | "snow"
                | "basic"
                | "artifact"
                | "enchantment"
                | "creature"
                | "land"
                | "instant"
                | "sorcery"
                | "battle"
                | "planeswalker"
                | "token"
                | "tokens"
                | "white"
                | "blue"
                | "black"
                | "red"
                | "green"
                | "colorless"
                | "named"
                | "with"
                | "that"
                | "which"
                | "and"
                | "or"
                | "a"
                | "an"
                | "flying"
                | "haste"
                | "deathtouch"
                | "trample"
                | "vigilance"
                | "lifelink"
                | "menace"
                | "reach"
                | "hexproof"
                | "indestructible"
                | "prowess"
                | "first"
                | "double"
                | "strike"
                | "when"
                | "whenever"
                | "if"
                | "this"
                | "it"
                | "those"
                | "cant"
                | "can"
                | "attack"
                | "block"
                | "dies"
                | "deals"
                | "deal"
                | "damage"
                | "draw"
                | "add"
                | "sacrifice"
                | "counter"
                | "gets"
                | "gains"
                | "gain"
        )
    };
    let first = *words.first()?;
    if !is_simple_name_word(first)
        || is_descriptor(first)
        || parse_token_pt(first).is_some()
        || parse_card_type(first).is_some()
        || parse_subtype_word(first).is_some()
    {
        return None;
    }

    let mut name_words = vec![first];
    for word in words.iter().skip(1) {
        if !is_simple_name_word(word)
            || is_descriptor(word)
            || parse_token_pt(word).is_some()
            || parse_card_type(word).is_some()
            || parse_subtype_word(word).is_some()
        {
            break;
        }
        name_words.push(*word);
    }

    if name_words.len() < 2 {
        None
    } else {
        Some(title_case_words(&name_words))
    }
}

pub(crate) fn extract_leading_token_name_phrase(words: &[&str]) -> Option<String> {
    let is_simple_name_word = |word: &str| {
        word.chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
    };
    let stop_words = [
        "a",
        "an",
        "the",
        "legendary",
        "snow",
        "basic",
        "named",
        "with",
        "that",
        "which",
        "when",
        "whenever",
        "if",
        "at",
        "until",
        "this",
        "it",
        "those",
        "token",
        "tokens",
        "and",
        "or",
        "to",
        "from",
        "add",
        "sacrifice",
        "draw",
        "deals",
        "deal",
        "damage",
        "dies",
        "gets",
        "gains",
        "gain",
        "cant",
        "can",
        "attack",
        "block",
        "flying",
        "haste",
        "deathtouch",
        "trample",
        "vigilance",
        "lifelink",
        "menace",
        "reach",
        "hexproof",
        "indestructible",
        "prowess",
        "first",
        "double",
        "strike",
        "white",
        "blue",
        "black",
        "red",
        "green",
        "colorless",
        "w",
        "u",
        "b",
        "r",
        "g",
        "c",
        "t",
    ];

    let mut name_words = Vec::new();
    for word in words {
        if stop_words.iter().any(|candidate| *candidate == *word)
            || parse_token_pt(word).is_some()
            || parse_card_type(word).is_some()
        {
            break;
        }
        if !is_simple_name_word(word) {
            break;
        }
        name_words.push(*word);
    }

    if name_words.len() < 2 {
        None
    } else {
        Some(title_case_words(&name_words))
    }
}

pub(crate) fn token_sacrifice_return_named_from_graveyard_ability(
    card_name: &str,
    mana_symbols: Vec<ManaSymbol>,
    tap_cost: bool,
) -> Ability {
    let mut costs = Vec::new();
    if tap_cost {
        costs.push(crate::costs::Cost::tap());
    }
    costs.push(crate::costs::Cost::validated_effect(Effect::new(
        crate::effects::SacrificeTargetEffect::source(),
    )));
    let mana_cost = if mana_symbols.is_empty() {
        ManaCost::new()
    } else {
        ManaCost::from_pips(
            mana_symbols
                .into_iter()
                .map(|symbol| vec![symbol])
                .collect(),
        )
    };
    let mut cost_parts = Vec::new();
    if !mana_cost.is_empty() {
        cost_parts.push(mana_cost.to_oracle());
    }
    if tap_cost {
        cost_parts.push("{T}".to_string());
    }
    cost_parts.push("Sacrifice this token".to_string());
    let cost_text = cost_parts.join(", ");
    let target = ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    );
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs({
                let mut total_costs = vec![crate::costs::Cost::mana(mana_cost)];
                total_costs.extend(costs);
                total_costs
            }),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::return_from_graveyard_to_battlefield(target.clone(), false),
            ]),
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "{cost_text}: Return a card named {card_name} from your graveyard to the battlefield."
        )),
    }
}

pub(crate) fn token_upkeep_sacrifice_return_named_from_graveyard_ability(
    card_name: &str,
    grants_haste: bool,
) -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    ));
    let mut effects = vec![
        Effect::sacrifice_source(),
        Effect::return_from_graveyard_to_battlefield(target.clone(), false),
    ];
    if grants_haste {
        effects.push(Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                target.clone(),
                crate::continuous::Modification::AddAbility(StaticAbility::haste()),
                Until::EndOfTurn,
            ),
        ));
    }
    let mut text = format!(
        "At the beginning of your upkeep, sacrifice this token and return target card named {card_name} from your graveyard to the battlefield."
    );
    if grants_haste {
        text.push_str(" It gains haste until end of turn.");
    }
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
            effects: effects.into(),
            choices: vec![target],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(text),
    }
}

pub(crate) fn token_dies_create_dragon_with_firebreathing_ability() -> Ability {
    let dragon = CardDefinitionBuilder::new(CardId::new(), "Dragon")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .color_indicator(ColorSet::RED)
        .power_toughness(PowerToughness::fixed(2, 2))
        .flying()
        .with_ability(token_red_pump_ability())
        .build();
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::create_tokens(dragon, Value::Fixed(1)),
            ]),
            choices: Vec::new(),
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'".to_string(),
        ),
    }
}

pub(crate) fn token_definition_for(name: &str) -> Option<CardDefinition> {
    let lower = name.trim().to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-'
            })
        })
        .map(|word| match word {
            "can't" | "cannot" => "cant",
            "aren't" => "arent",
            "isn't" => "isnt",
            "they're" => "theyre",
            "it's" => "its",
            "you're" => "youre",
            _ => word,
        })
        .filter(|word| !word.is_empty())
        .collect();
    let has_word = |needle: &str| slice_contains(words.as_slice(), &needle);
    let has_words = |needles: &[&str]| needles.iter().all(|needle| has_word(needle));
    let has_any_word = |needles: &[&str]| needles.iter().any(|needle| has_word(needle));
    let has_phrase = |phrase: &[&str]| contains_word_sequence(words.as_slice(), phrase);
    let has_text = |needle: &str| str_contains(lower.as_str(), needle);
    let has_explicit_pt = words.iter().any(|word| parse_token_pt(word).is_some());
    let has_equipment_rules_subject =
        has_word("equipment") && has_phrase(&["equipped", "creature"]);

    if has_word("treasure") && !has_word("creature") {
        return Some(crate::cards::tokens::treasure_token_definition());
    }
    if has_word("clue") && !has_word("creature") {
        return Some(crate::cards::tokens::clue_token_definition());
    }
    if has_word("map") && !has_word("creature") {
        return Some(crate::cards::tokens::map_token_definition());
    }
    if has_word("lander") && !has_word("creature") {
        return Some(crate::cards::tokens::lander_token_definition());
    }
    if has_word("junk") && !has_word("creature") {
        return Some(crate::cards::tokens::junk_token_definition());
    }
    if has_word("gold") && !has_word("creature") {
        return Some(crate::cards::tokens::gold_token_definition());
    }
    if has_word("shard") && !has_word("creature") {
        return Some(crate::cards::tokens::shard_token_definition());
    }
    if has_word("walker") && !has_word("planeswalker") {
        return Some(crate::cards::tokens::walker_token_definition());
    }
    if has_word("eldrazi") && has_word("spawn") {
        return Some(eldrazi_spawn_token_definition());
    }
    if has_word("eldrazi") && has_word("scion") {
        return Some(eldrazi_scion_token_definition());
    }
    if has_word("food") && !has_word("creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Food")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food]);
        return Some(builder.build());
    }
    if has_word("wicked") && has_word("role") {
        return Some(crate::cards::tokens::wicked_role_token_definition());
    }
    if has_word("young") && has_word("hero") && has_word("role") {
        return Some(crate::cards::tokens::young_hero_role_token_definition());
    }
    if has_word("monster") && has_word("role") {
        return Some(crate::cards::tokens::monster_role_token_definition());
    }
    if has_word("sorcerer") && has_word("role") {
        return Some(crate::cards::tokens::sorcerer_role_token_definition());
    }
    if has_word("royal") && has_word("role") {
        return Some(crate::cards::tokens::royal_role_token_definition());
    }
    if has_word("cursed") && has_word("role") {
        return Some(crate::cards::tokens::cursed_role_token_definition());
    }
    if has_word("blood") && !has_word("creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Blood")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("powerstone") && !has_word("creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Powerstone")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("vehicle") && has_word("artifact") && !has_word("creature") {
        let explicit_name_from_words = words.iter().find_map(|word| {
            if parse_token_pt(word).is_some() {
                return None;
            }
            if !word
                .chars()
                .all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
            {
                return None;
            }
            if matches!(
                *word,
                "artifact"
                    | "token"
                    | "tokens"
                    | "vehicle"
                    | "colorless"
                    | "named"
                    | "with"
                    | "and"
                    | "crew"
                    | "flying"
                    | "white"
                    | "blue"
                    | "black"
                    | "red"
                    | "green"
            ) {
                return None;
            }
            if parse_card_type(word).is_some() || parse_subtype_word(word).is_some() {
                return None;
            }
            Some(title_case_words(&[*word]))
        });
        let token_name = extract_named_card_name(&words, lower.as_str())
            .or(explicit_name_from_words)
            .unwrap_or_else(|| "Vehicle".to_string());
        let mut builder = CardDefinitionBuilder::new(CardId::new(), token_name)
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle]);
        if let Some((power, toughness)) = words.iter().find_map(|word| parse_token_pt(word)) {
            builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
        }
        if has_word("flying") {
            builder = builder.flying();
        }
        if let Some(crew_amount) = parse_crew_amount(&words) {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
                format!("crew {crew_amount}"),
            )));
        }
        return Some(builder.build());
    }
    if has_word("artifact")
        && !has_explicit_pt
        && (!has_word("creature") || has_equipment_rules_subject)
    {
        let mut subtypes = Vec::new();
        for word in &words {
            if let Some(subtype) = parse_subtype_word(word)
                && !subtype.is_creature_type()
                && !subtypes.iter().any(|candidate| *candidate == subtype)
            {
                subtypes.push(subtype);
            }
        }
        let token_name = extract_named_card_name(&words, lower.as_str())
            .or_else(|| {
                find_index(words.as_slice(), |word| {
                    !matches!(
                        *word,
                        "artifact"
                            | "token"
                            | "tokens"
                            | "named"
                            | "colorless"
                            | "white"
                            | "blue"
                            | "black"
                            | "red"
                            | "green"
                    )
                })
                .map(|idx| {
                    let mut chars = words[idx].chars();
                    match chars.next() {
                        Some(first) => {
                            let mut name = first.to_uppercase().to_string();
                            name.push_str(chars.as_str());
                            name
                        }
                        None => "Artifact".to_string(),
                    }
                })
            })
            .unwrap_or_else(|| "Artifact".to_string());
        let mut builder = CardDefinitionBuilder::new(CardId::new(), token_name)
            .token()
            .card_types(vec![CardType::Artifact]);
        if has_word("legendary") {
            builder = builder.supertypes(vec![crate::types::Supertype::Legendary]);
        }
        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }
        if has_word("colorless") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::make_colorless(
                ObjectFilter::source(),
            )));
        }
        if let Some(rules_text) = parse_equipment_rules_text(&words, name)
            && let Ok(def) = builder.clone().parse_text(&rules_text)
        {
            return Some(def);
        }
        if has_words(&[
            "when",
            "token",
            "leaves",
            "battlefield",
            "deals",
            "damage",
            "target",
        ]) && let Some(amount) = parse_deals_damage_amount(&words)
        {
            builder = builder.with_ability(token_leaves_deals_damage_any_target_ability(amount));
        }
        return Some(builder.build());
    }
    if has_word("angel") && !has_explicit_pt {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Angel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(4, 4))
            .flying();
        return Some(builder.build());
    }
    if has_word("wall") && has_text("0/4") && has_text("artifact") && has_text("creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Wall")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Wall])
            .power_toughness(PowerToughness::fixed(0, 4))
            .defender();
        return Some(builder.build());
    }
    if has_word("squirrel") && has_text("1/1") && has_text("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Squirrel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Squirrel])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(1, 1));
        return Some(builder.build());
    }
    let is_dragon_egg_death_spawn_pattern = has_word("dragon")
        && has_word("egg")
        && has_text("0/2")
        && has_words(&[
            "when", "token", "dies", "create", "2/2", "flying", "r", "+1/+0",
        ]);
    if is_dragon_egg_death_spawn_pattern {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Dragon Egg")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Dragon])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(0, 2))
            .defender()
            .with_ability(token_dies_create_dragon_with_firebreathing_ability());
        return Some(builder.build());
    }
    if has_word("elephant") && has_text("3/3") && has_text("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Elephant")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elephant])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(3, 3));
        return Some(builder.build());
    }
    let has_construct_cda_words = has_words(&[
        "power",
        "toughness",
        "equal",
        "number",
        "artifacts",
        "you",
        "control",
    ]);
    let has_construct_plus_words =
        has_words(&["gets", "+1/+1", "for", "each", "artifact", "you", "control"]);
    let is_zero_zero_construct = has_word("construct") && has_text("0/0");
    if has_word("construct")
        && (!has_explicit_pt
            || has_construct_cda_words
            || has_construct_plus_words
            || is_zero_zero_construct)
    {
        let construct_scaling_text = "This token gets +1/+1 for each artifact you control.";
        let scaling_ability = Ability::static_ability(StaticAbility::characteristic_defining_pt(
            Value::Count(ObjectFilter::artifact().you_control()),
            Value::Count(ObjectFilter::artifact().you_control()),
        ))
        .with_text(construct_scaling_text);
        let builder = CardDefinitionBuilder::new(CardId::new(), "Construct")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Construct])
            .power_toughness(PowerToughness::fixed(0, 0))
            .with_ability(scaling_ability);
        return Some(builder.build());
    }
    if has_word("shapeshifter") && !has_word("creature") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Shapeshifter")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Shapeshifter])
            .power_toughness(PowerToughness::fixed(3, 2));
        if has_text("changeling") || lower == "shapeshifter" {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }
        return Some(builder.build());
    }
    if has_word("astartes") && has_word("warrior") && has_text("2/2") && has_text("white") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Astartes Warrior")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Astartes, Subtype::Warrior])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(2, 2));
        if has_text("vigilance") {
            builder = builder.vigilance();
        }
        return Some(builder.build());
    }
    if has_word("creature") {
        let mut card_types = vec![CardType::Creature];
        let first_creature_idx = find_index(words.as_slice(), |word| *word == "creature");
        let artifact_before_creature =
            first_creature_idx.is_some_and(|idx| slice_contains(&words[..idx], &"artifact"));
        let enchantment_before_creature =
            first_creature_idx.is_some_and(|idx| slice_contains(&words[..idx], &"enchantment"));
        if artifact_before_creature {
            card_types.insert(0, CardType::Artifact);
        }
        if enchantment_before_creature {
            card_types.insert(0, CardType::Enchantment);
        }
        let is_creature_token = card_types.contains(&CardType::Creature);

        let (power, toughness) = words.iter().find_map(|word| parse_token_pt(word))?;

        let mut subtypes = Vec::new();
        let subtype_scan_end = find_index(words.as_slice(), |word| parse_card_type(word).is_some())
            .unwrap_or(words.len());
        for word in &words[..subtype_scan_end] {
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
                && !subtypes.iter().any(|candidate| *candidate == subtype)
            {
                subtypes.push(subtype);
            }
        }

        let explicit_name = extract_named_card_name(&words, lower.as_str())
            .or_else(|| extract_leading_token_name_phrase(&words))
            .or_else(|| extract_leading_explicit_token_name(&words));
        let token_name = explicit_name.unwrap_or_else(|| {
            subtypes
                .first()
                .map(|subtype| format!("{subtype:?}"))
                .unwrap_or_else(|| "OwnedLexToken".to_string())
        });

        let mut builder = CardDefinitionBuilder::new(CardId::new(), token_name)
            .token()
            .card_types(card_types)
            .power_toughness(PowerToughness::fixed(power, toughness));
        if has_word("legendary") {
            builder = builder.supertypes(vec![crate::types::Supertype::Legendary]);
        }

        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }

        let mut colors = ColorSet::new();
        if has_word("white") {
            colors = colors.union(ColorSet::WHITE);
        }
        if has_word("blue") {
            colors = colors.union(ColorSet::BLUE);
        }
        if has_word("black") {
            colors = colors.union(ColorSet::BLACK);
        }
        if has_word("red") {
            colors = colors.union(ColorSet::RED);
        }
        if has_word("green") {
            colors = colors.union(ColorSet::GREEN);
        }
        if !colors.is_empty() {
            builder = builder.color_indicator(colors);
        }

        if has_word("flying") {
            builder = builder.flying();
        }
        if has_word("defender") {
            builder = builder.defender();
        }
        if has_word("prowess") {
            builder = builder.prowess();
        }
        if has_word("vigilance") {
            builder = builder.vigilance();
        }
        if has_word("trample") {
            builder = builder.trample();
        }
        if has_word("lifelink") {
            builder = builder.lifelink();
        }
        if has_word("deathtouch") {
            builder = builder.deathtouch();
        }
        if has_word("haste") {
            builder = builder.haste();
        }
        if has_word("menace") {
            builder = builder.menace();
        }
        if has_word("reach") {
            builder = builder.reach();
        }
        if let Some(upkeep_idx) =
            find_word_sequence_start(words.as_slice(), &["cumulative", "upkeep"])
        {
            let mut cost_symbols = Vec::new();
            for word in &words[upkeep_idx + 2..] {
                if matches!(*word, "when" | "whenever" | "at") {
                    break;
                }
                let Some(symbol) = parse_token_mana_symbol(word) else {
                    break;
                };
                cost_symbols.push(symbol);
            }
            let text = if cost_symbols.is_empty() {
                "Cumulative upkeep".to_string()
            } else {
                let cost = crate::mana::ManaCost::from_symbols(cost_symbols).to_oracle();
                format!("Cumulative upkeep {cost}")
            };
            builder =
                builder.with_ability(Ability::static_ability(StaticAbility::keyword_marker(text)));
        }
        if let Some(symbol) = parse_token_tap_add_single_mana_symbol(&words) {
            builder = builder.with_ability(token_tap_add_single_mana_ability(symbol));
        }
        if has_words(&["crews", "vehicles", "power", "greater", "2"]) {
            return None;
        }
        if has_word("banding") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
                "banding",
            )));
        }
        if has_word("hexproof") {
            builder = builder.hexproof();
        }
        if has_word("indestructible") {
            builder = builder.indestructible();
        }
        if let Some(toxic_idx) = find_window_by(words.as_slice(), 2, |window| window[0] == "toxic")
        {
            if let Ok(amount) = words[toxic_idx + 1].parse::<u32>() {
                builder = builder.toxic(amount);
            }
        }
        if has_words(&[
            "sacrifice",
            "this",
            "token",
            "return",
            "named",
            "graveyard",
            "battlefield",
        ]) && !has_word("beginning")
            && let Some(card_name) = extract_named_card_name(&words, lower.as_str())
            && let Some(sacrifice_idx) = find_index(words.as_slice(), |word| *word == "sacrifice")
        {
            let mut mana_symbols = Vec::new();
            let mut tap_cost = false;
            for word in &words[..sacrifice_idx] {
                if *word == "t" {
                    tap_cost = true;
                    continue;
                }
                if let Some(symbol) = parse_token_mana_symbol(word) {
                    mana_symbols.push(symbol);
                }
            }
            builder = builder.with_ability(token_sacrifice_return_named_from_graveyard_ability(
                &card_name,
                mana_symbols,
                tap_cost,
            ));
        }
        if has_phrase(&["at", "the", "beginning", "of", "your"])
            && has_words(&[
                "upkeep",
                "sacrifice",
                "this",
                "token",
                "return",
                "named",
                "graveyard",
                "battlefield",
            ])
            && let Some(card_name) = extract_named_card_name(&words, lower.as_str())
        {
            builder =
                builder.with_ability(token_upkeep_sacrifice_return_named_from_graveyard_ability(
                    &card_name,
                    has_word("haste"),
                ));
        }
        if has_words(&[
            "when", "token", "dies", "create", "2/2", "red", "dragon", "flying", "r", "+1/+0",
        ]) {
            builder = builder.with_ability(token_dies_create_dragon_with_firebreathing_ability());
        }
        if has_words(&["when", "token", "dies", "deals", "damage", "target"])
            && let Some(amount) = parse_deals_damage_amount(&words)
        {
            builder = builder.with_ability(token_dies_deals_damage_any_target_ability(amount));
        }
        if has_words(&[
            "when", "token", "dies", "target", "creature", "gets", "-1/-1",
        ]) {
            builder =
                builder.with_ability(token_dies_target_creature_gets_minus_one_minus_one_ability());
        }
        if has_words(&[
            "when",
            "token",
            "leaves",
            "battlefield",
            "deals",
            "damage",
            "you",
            "each",
            "creature",
            "control",
        ]) && let Some(amount) = parse_deals_damage_amount(&words)
        {
            let ability = Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::this_leaves_battlefield(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::deal_damage(amount, ChooseSpec::SourceController),
                        Effect::for_each(
                            ObjectFilter::creature().you_control(),
                            vec![Effect::deal_damage(amount, ChooseSpec::Iterated)],
                        ),
                    ]),
                    choices: Vec::new(),
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(format!(
                    "When this token leaves the battlefield, it deals {amount} damage to you and each creature you control."
                )),
            };
            builder = builder.with_ability(ability);
        }
        if has_words(&["bands", "other", "creatures", "named", "wolves"]) {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
                "bands with other creatures named Wolves of the Hunt",
            )));
        }
        if has_words(&["r", "this", "creature", "gets", "+1/+0"])
            && !has_words(&["when", "token", "dies", "create"])
        {
            builder = builder.with_ability(token_red_pump_ability());
        }
        if has_words(&["w", "t", "tap", "target", "creature"]) {
            builder = builder.with_ability(token_white_tap_target_creature_ability());
        }
        if has_words(&["deals", "damage", "player", "poison", "counter"]) {
            builder = builder.with_ability(token_damage_to_player_poison_counter_ability());
        }
        if let Some(amount) =
            token_inline_noncreature_spell_each_opponent_damage_amount(lower.as_str())
        {
            builder =
                builder.with_ability(token_noncreature_spell_each_opponent_damage_ability(amount));
        }
        if has_words(&[
            "whenever", "token", "becomes", "tapped", "deals", "damage", "target", "player",
        ]) && let Some(amount) = parse_deals_damage_amount(&words)
        {
            builder = builder.with_ability(
                token_becomes_tapped_deals_damage_target_player_ability(amount),
            );
        }
        if has_words(&[
            "whenever", "token", "deals", "combat", "damage", "player", "gain", "control",
            "artifact",
        ]) {
            builder =
                builder.with_ability(token_combat_damage_gain_control_target_artifact_ability());
        }
        if has_words(&[
            "when",
            "leaves",
            "battlefield",
            "return",
            "named",
            "graveyard",
            "hand",
        ]) && let Some(card_name) = extract_named_card_name(&words, lower.as_str())
        {
            builder = builder.with_ability(
                token_leaves_return_named_from_graveyard_to_hand_ability(&card_name),
            );
        }
        if has_word("pest") && has_words(&["when", "token", "dies", "gain", "1", "life"]) {
            let ability = Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::this_dies(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::gain_life(1),
                    ]),
                    choices: Vec::new(),
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("When this token dies, you gain 1 life.".to_string()),
            };
            builder = builder.with_ability(ability);
        }
        if has_words(&["first", "strike"]) {
            builder = builder.first_strike();
        }
        if has_words(&["double", "strike"]) {
            builder = builder.double_strike();
        }
        if let Some(parsed) = try_parse_quoted_token_rules_text(
            &builder,
            name,
            if is_creature_token {
                "This creature"
            } else {
                "This permanent"
            },
        ) {
            return Some(parsed);
        }
        if has_word("mercenary") && has_words(&["creature", "1/1", "red"]) {
            let target =
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
            let ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: TotalCost::from_cost(crate::costs::Cost::tap()),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::pump(1, 0, target.clone(), Until::EndOfTurn),
                    ]),
                    choices: vec![target],
                    timing: crate::ability::ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec!["activate only as a sorcery".to_string()],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(
                    "{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery."
                        .to_string(),
                ),
            };
            builder = builder.with_ability(ability);
        }
        let has_cant_attack_or_block = has_words(&["cant", "attack", "or", "block"]);
        if has_cant_attack_or_block && has_word("alone") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
                "this token can't attack or block alone".to_string(),
            )));
        } else if has_cant_attack_or_block {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                "this token can't attack or block".to_string(),
            )));
        } else if has_words(&["cant", "block"]) {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::cant_block()));
        }
        if has_words(&["can", "block", "only", "creatures", "flying"]) {
            builder = builder.with_ability(Ability::static_ability(
                StaticAbility::can_block_only_flying(),
            ));
        }
        if has_words(&[
            "counter",
            "noncreature",
            "spell",
            "sacrifice",
            "token",
            "unless",
            "controller",
            "pays",
            "1",
        ]) {
            let target = ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::spell().without_type(CardType::Creature),
            ));
            let counter_ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: TotalCost::from_costs(vec![
                        crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![
                            ManaSymbol::Generic(1),
                        ]])),
                        crate::costs::Cost::sacrifice_self(),
                    ]),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::counter_unless_pays(target.clone(), vec![ManaSymbol::Generic(1)]),
                    ]),
                    choices: vec![target],
                    timing: crate::ability::ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(
                    "{1}, Sacrifice this token: Counter target noncreature spell unless its controller pays {1}."
                        .to_string(),
                ),
            };
            builder = builder.with_ability(counter_ability);
        }
        if has_word("changeling") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }
        if has_words(&[
            "this", "token", "gets", "+1/+1", "for", "each", "card", "named",
        ]) && has_any_word(&["graveyard", "graveyards"])
        {
            let card_name = find_word_sequence_start(words.as_slice(), &["card", "named"])
                .and_then(|named_card_idx| {
                    let start = named_card_idx + 2;
                    let end = find_index(&words[start..], |word| {
                        matches!(
                            *word,
                            "in" | "from"
                                | "and"
                                | "or"
                                | "with"
                                | "that"
                                | "where"
                                | "when"
                                | "whenever"
                        )
                    })
                    .map(|offset| start + offset)
                    .unwrap_or(words.len());
                    (end > start).then(|| title_case_words(&words[start..end]))
                })
                .or_else(|| extract_named_card_name(&words, lower.as_str()));
            if let Some(card_name) = card_name {
                let mut named_filter = ObjectFilter::default();
                named_filter.zone = Some(Zone::Graveyard);
                named_filter.name = Some(card_name.clone());
                let count =
                    crate::static_abilities::AnthemCountExpression::MatchingFilter(named_filter);
                let anthem = crate::static_abilities::Anthem::for_source(0, 0).with_values(
                    crate::static_abilities::AnthemValue::scaled(1, count.clone()),
                    crate::static_abilities::AnthemValue::scaled(1, count),
                );
                let reminder_text = format!(
                    "This token gets +1/+1 for each card named {card_name} in each graveyard."
                );
                builder = builder.with_ability(
                    Ability::static_ability(StaticAbility::new(anthem))
                        .with_text(reminder_text.as_str()),
                );
            }
        }

        // Final Fantasy "Chocobo" token text: a Bird token with a quoted landfall-ish pump ability.
        // Example: Create a 2/2 green Bird creature token with
        // "Whenever a land you control enters, this token gets +1/+0 until end of turn."
        let is_land_you_control_enters_pump_token = has_words(&[
            "whenever", "land", "control", "enters", "this", "token", "gets", "+1/+0",
        ]) && contains_until_end_of_turn(&words);
        if is_land_you_control_enters_pump_token {
            let ability = Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::enters_battlefield(ObjectFilter::land().you_control(), None),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::pump(1, 0, ChooseSpec::Source, Until::EndOfTurn),
                    ]),
                    choices: Vec::new(),
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(
                    "Whenever a land you control enters, this token gets +1/+0 until end of turn."
                        .to_string(),
                ),
            };
            builder = builder.with_ability(ability);
        }

        return Some(builder.build());
    }
    None
}

pub(crate) fn parse_token_pt(word: &str) -> Option<(i32, i32)> {
    let (left, right) = str_split_once_char(word, '/')?;
    if str_starts_with(left, "+")
        || str_starts_with(right, "+")
        || str_starts_with(left, "-")
        || str_starts_with(right, "-")
    {
        return None;
    }
    let power = left.parse::<i32>().ok()?;
    let toughness = right.parse::<i32>().ok()?;
    Some((power, toughness))
}

pub(crate) fn target_mentions_graveyard(target: &TargetAst) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => filter.zone == Some(Zone::Graveyard),
        TargetAst::WithCount(inner, _) => target_mentions_graveyard(inner),
        _ => false,
    }
}

pub(crate) fn compile_effect_for_target<Builder>(
    target: &TargetAst,
    ctx: &mut EffectLoweringContext,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(ChooseSpec) -> Effect,
{
    let refs = current_reference_env(ctx);
    let (spec, choices) = resolve_target_spec_with_choices(target, &refs)?;
    let effect = tag_object_target_effect(build(spec.clone()), &spec, ctx, "targeted");
    Ok((vec![effect], choices))
}

pub(crate) fn compile_tagged_effect_for_target<Builder>(
    target: &TargetAst,
    ctx: &mut EffectLoweringContext,
    tag_prefix: &str,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(ChooseSpec) -> Effect,
{
    let refs = current_reference_env(ctx);
    let (spec, choices) = resolve_target_spec_with_choices(target, &refs)?;
    let effect = tag_object_target_effect(build(spec.clone()), &spec, ctx, tag_prefix);
    Ok((vec![effect], choices))
}

pub(crate) fn push_choice(choices: &mut Vec<ChooseSpec>, choice: ChooseSpec) {
    if !choices.iter().any(|existing| existing == &choice) {
        choices.push(choice);
    }
}

#[cfg(test)]
mod parse_compile_tests {
    use super::*;
    use crate::cards::TextSpan;
    use crate::cards::builders::RefState;
    use crate::effect::{Condition, Value};
    use crate::effects::{
        AmassEffect, ConditionalEffect, ExecuteWithSourceEffect, ForEachObject,
        ForEachTaggedEffect, GrantPlayTaggedEffect, InvestigateEffect, MoveToZoneEffect,
        TaggedEffect,
    };
    use crate::ids::CardId;
    use crate::target::ChooseSpec;
    use crate::types::{CardType, Subtype};

    #[test]
    fn compile_investigate_uses_ast_count() {
        let mut ctx = EffectLoweringContext::new();
        let (effects, choices) = compile_effect(
            &EffectAst::Investigate {
                count: Value::Fixed(2),
            },
            &mut ctx,
        )
        .expect("compile investigate");

        assert!(choices.is_empty());
        assert_eq!(effects.len(), 1);
        let investigate = effects[0]
            .downcast_ref::<InvestigateEffect>()
            .expect("investigate effect");
        assert_eq!(investigate.count, Value::Fixed(2));
    }

    #[test]
    fn parse_text_investigate_twice_compiles_to_count_two() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Investigate Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Investigate twice.")
            .expect("parse investigate twice");

        let effects = def.spell_effect.as_ref().expect("spell effects");
        assert_eq!(effects.len(), 1);
        let investigate = effects[0]
            .downcast_ref::<InvestigateEffect>()
            .expect("investigate effect");
        assert_eq!(investigate.count, Value::Fixed(2));
    }

    #[test]
    fn compile_amass_tags_output_when_followup_references_it() {
        let mut ctx = EffectLoweringContext::new();
        ctx.auto_tag_object_targets = true;

        let (effects, choices) = compile_effect(
            &EffectAst::Amass {
                subtype: Some(Subtype::Orc),
                amount: 2,
            },
            &mut ctx,
        )
        .expect("compile amass");

        assert!(choices.is_empty());
        assert_eq!(effects.len(), 1);

        let tagged = effects[0]
            .downcast_ref::<TaggedEffect>()
            .expect("amass should lower through TaggedEffect when auto-tagging is active");
        assert_eq!(tagged.tag.as_str(), "amassed_0");

        let amass = tagged
            .effect
            .downcast_ref::<AmassEffect>()
            .expect("inner effect should still be AmassEffect");
        assert_eq!(amass.subtype, Some(Subtype::Orc));
        assert_eq!(amass.amount, 2);
        assert_eq!(ctx.last_object_tag.as_deref(), Some("amassed_0"));
    }

    #[test]
    fn compile_damage_equal_to_power_over_each_object_fans_out_per_object() {
        let (effects, choices) = compile_effect(
            &EffectAst::DealDamageEqualToPower {
                source: TargetAst::Tagged(TagKey::from("amassed_0"), None),
                target: TargetAst::Object(
                    ObjectFilter::creature().without_subtype(Subtype::Army),
                    None,
                    None,
                ),
            },
            &mut EffectLoweringContext::new(),
        )
        .expect("compile power-based fanout damage");

        assert!(choices.is_empty());
        assert_eq!(effects.len(), 1);

        let for_each = effects[0]
            .downcast_ref::<ForEachObject>()
            .expect("non-target object damage should lower through ForEachObject");
        assert!(
            crate::cards::builders::compiler::token_primitives::iter_contains(
                &for_each.filter.card_types,
                &CardType::Creature,
            )
        );
        assert!(
            crate::cards::builders::compiler::token_primitives::iter_contains(
                &for_each.filter.excluded_subtypes,
                &Subtype::Army,
            )
        );
        assert_eq!(for_each.effects.len(), 1);

        let with_source = for_each.effects[0]
            .downcast_ref::<ExecuteWithSourceEffect>()
            .expect("fan-out damage should preserve the chosen source");
        assert_eq!(
            with_source.source,
            ChooseSpec::Tagged(TagKey::from("amassed_0"))
        );

        let deal_damage = with_source
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .expect("wrapped effect should still be DealDamageEffect");
        assert_eq!(
            deal_damage.amount,
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from("amassed_0"))))
        );
        assert_eq!(deal_damage.target, ChooseSpec::Iterated);
    }

    #[test]
    fn parse_text_gargoyle_sentinel_keeps_the_activation_on_self() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Gargoyle Sentinel")
            .parse_text(
                "Mana cost: {3}\n\
                 Type: Artifact Creature — Gargoyle\n\
                 Power/Toughness: 3/3\n\
                 Defender (This creature can't attack.)\n\
                 {3}: Until end of turn, this creature loses defender and gains flying.",
            )
            .expect("Gargoyle Sentinel text should parse");

        let rendered = crate::compiled_text::compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains("this creature loses defender and gains flying"),
            "expected a self-targeted temporary activation, got {rendered}"
        );
        assert!(
            !rendered.contains("creatures lose defender"),
            "expected the activation to stay on the sentinel itself, got {rendered}"
        );

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected Gargoyle Sentinel to have an activated ability");
        let apply_effects = activated
            .effects
            .segments
            .iter()
            .flat_map(|segment| segment.default_effects.iter())
            .filter_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
            .collect::<Vec<_>>();
        assert_eq!(
            apply_effects.len(),
            2,
            "expected the lowered activation to produce exactly two source-scoped continuous effects"
        );
        assert!(
            apply_effects.iter().all(|apply| {
                matches!(apply.target_spec, Some(crate::target::ChooseSpec::Source))
                    && matches!(apply.until, crate::effect::Until::EndOfTurn)
            }),
            "expected the lowered activated ability to stay source-targeted until end of turn, got {apply_effects:#?}"
        );

        let debug = format!("{def:?}");
        assert!(
            !debug.contains("GrantAbilitiesAll") && !debug.contains("RemoveAbilitiesAll"),
            "expected no broad battlefield-wide ability changes in the lowered definition, got {debug}"
        );
    }

    #[test]
    fn parse_equipment_rules_text_keeps_single_quoted_activated_grant() {
        let words = [
            "colorless",
            "equipment",
            "artifact",
            "token",
            "named",
            "rock",
            "with",
            "equipped",
            "creature",
            "has",
            "sacrifice",
            "rock",
            "this",
            "creature",
            "deals",
            "2",
            "damage",
            "to",
            "any",
            "target",
            "and",
            "equip",
            "1",
        ];
        let source_text = "Colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.";

        let rules_text =
            parse_equipment_rules_text(&words, source_text).expect("equipment rules text");

        assert!(
            rules_text.contains("Equipped creature has \"{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target.\"")
                && rules_text.contains("Equip {1}"),
            "expected quoted activated ability plus equip line, got {rules_text}"
        );
    }

    #[test]
    fn equipment_token_rules_text_reparses_into_grant_and_equip() {
        let words = [
            "colorless",
            "equipment",
            "artifact",
            "token",
            "named",
            "rock",
            "with",
            "equipped",
            "creature",
            "has",
            "sacrifice",
            "rock",
            "this",
            "creature",
            "deals",
            "2",
            "damage",
            "to",
            "any",
            "target",
            "and",
            "equip",
            "1",
        ];
        let source_text = "colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.";
        let rules_text =
            parse_equipment_rules_text(&words, source_text).expect("equipment rules text");

        let def = CardDefinitionBuilder::new(CardId::new(), "Rock")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .with_ability(Ability::static_ability(StaticAbility::make_colorless(
                ObjectFilter::source(),
            )))
            .parse_text(&rules_text)
            .expect("equipment token rules text should parse");

        let activated_texts = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(_) => ability.text.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            activated_texts.iter().any(|text| *text == "Equip {1}"),
            "expected reparsed equipment token to keep equip, got {activated_texts:?}"
        );
        assert!(
            format!("{def:?}").contains("Equipped creature has"),
            "expected reparsed equipment token to keep the granted ability, got {def:#?}"
        );
    }

    #[test]
    fn token_definition_reparses_quoted_triggered_rules_text() {
        let source_text = "2/2 black Alien Angel artifact creature token with first strike, vigilance, and \"Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.\"";

        let def =
            token_definition_for(source_text).expect("quoted token trigger should build a token");
        let debug = format!("{def:#?}");

        assert!(
            debug.contains("Whenever an opponent casts a creature spell, this token isn't a creature until end of turn."),
            "expected token text to preserve the original quoted trigger, got {debug}"
        );
        assert!(
            debug.contains("RemoveCardTypes"),
            "expected quoted trigger to compile into a real remove-card-types effect, got {debug}"
        );
    }

    #[test]
    fn token_definition_reparses_unquoted_triggered_rules_tail() {
        let source_text = "2/2 black Alien Angel artifact creature token with first strike, vigilance, and Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.";

        let def = token_definition_for(source_text)
            .expect("unquoted preserved trigger tail should still build a token");
        let debug = format!("{def:#?}");

        assert!(
            debug.contains("Whenever an opponent casts a creature spell, this token isn't a creature until end of turn."),
            "expected token text to preserve the inline trigger tail, got {debug}"
        );
        assert!(
            debug.contains("RemoveCardTypes"),
            "expected inline trigger tail to compile into a real remove-card-types effect, got {debug}"
        );
    }

    #[test]
    fn try_parse_quoted_token_rules_text_parses_blink_trigger() {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Alien Angel")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Alien, Subtype::Angel])
            .color_indicator(ColorSet::BLACK)
            .power_toughness(PowerToughness::fixed(2, 2))
            .first_strike()
            .vigilance();
        let source_text = "2/2 black Alien Angel artifact creature token with first strike, vigilance, and \"Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.\"";
        let parsed = try_parse_quoted_token_rules_text(&builder, source_text, "This creature")
            .expect("quoted trigger should parse generically");
        let debug = format!("{parsed:#?}");

        assert!(
            debug.contains("RemoveCardTypes"),
            "expected generic quoted-token parse to compile remove-card-types, got {debug}"
        );
        assert!(
            debug.contains("until: EndOfTurn"),
            "expected generic quoted-token parse to keep until-end-of-turn duration, got {debug}"
        );
    }

    #[test]
    fn resolve_target_spec_treats_source_object_filters_as_source() {
        let target = TargetAst::Object(ObjectFilter::source(), None, None);
        let (spec, choices) = resolve_target_spec_with_choices(&target, &ReferenceEnv::default())
            .expect("source object target should resolve cleanly");

        assert_eq!(
            spec,
            ChooseSpec::Source,
            "source object filters should resolve to the source choose spec"
        );
        assert!(
            choices.is_empty(),
            "self-targeted object filters should not create extra target choices"
        );
    }

    fn test_ctx(line: &str) -> NormalizedLine {
        NormalizedLine {
            original: line.to_string(),
            normalized: line.to_string(),
            char_map: (0..line.len()).collect(),
        }
    }

    #[test]
    fn collect_tag_spans_tracks_connive_and_destroy_no_regeneration_targets() {
        let mut annotations = ParseAnnotations::default();
        let ctx = test_ctx("alpha beta");
        let alpha = TagKey::from("alpha");
        let beta = TagKey::from("beta");

        collect_tag_spans_from_effect(
            &EffectAst::Connive {
                target: TargetAst::Tagged(
                    alpha.clone(),
                    Some(TextSpan {
                        line: 0,
                        start: 0,
                        end: 5,
                    }),
                ),
            },
            &mut annotations,
            &ctx,
        );
        collect_tag_spans_from_effect(
            &EffectAst::DestroyNoRegeneration {
                target: TargetAst::Tagged(
                    beta.clone(),
                    Some(TextSpan {
                        line: 0,
                        start: 6,
                        end: 10,
                    }),
                ),
            },
            &mut annotations,
            &ctx,
        );

        assert!(
            annotations
                .tag_spans
                .get(&alpha)
                .is_some_and(|spans| spans.len() == 1),
            "expected span recorded for connive target tag"
        );
        assert!(
            annotations
                .tag_spans
                .get(&beta)
                .is_some_and(|spans| spans.len() == 1),
            "expected span recorded for destroy-no-regeneration target tag"
        );
    }

    #[test]
    fn collect_tag_spans_tracks_counter_unless_pays_target() {
        let mut annotations = ParseAnnotations::default();
        let ctx = test_ctx("gamma");
        let gamma = TagKey::from("gamma");
        let effect = EffectAst::CounterUnlessPays {
            target: TargetAst::Tagged(
                gamma.clone(),
                Some(TextSpan {
                    line: 0,
                    start: 0,
                    end: 5,
                }),
            ),
            mana: vec![],
            life: None,
            additional_generic: None,
        };

        collect_tag_spans_from_effect(&effect, &mut annotations, &ctx);
        assert!(
            annotations
                .tag_spans
                .get(&gamma)
                .is_some_and(|spans| spans.len() == 1),
            "expected span recorded for counter-unless-pays target tag"
        );
        assert!(
            effect_references_tag(&effect, "gamma"),
            "counter-unless-pays tagged target should be detected by tag reference checks"
        );
    }

    #[test]
    fn this_attacks_triggers_bind_the_defending_player() {
        assert_eq!(
            inferred_trigger_player_filter(&TriggerSpec::ThisAttacks),
            Some(PlayerFilter::Defending)
        );
    }

    #[test]
    fn compile_statement_effects_drops_empty_global_ability_grants() {
        let effects = vec![EffectAst::GrantAbilitiesAll {
            filter: ObjectFilter::default(),
            abilities: Vec::new(),
            duration: Until::EndOfTurn,
        }];

        let compiled =
            compile_statement_effects(&effects).expect("normalization should remove empty grants");
        assert!(compiled.is_empty());
    }

    #[test]
    fn compile_statement_effects_with_imports_returns_reference_exports() {
        let effects = vec![EffectAst::Destroy {
            target: TargetAst::Object(ObjectFilter::creature(), Some(TextSpan::synthetic()), None),
        }];

        let lowered =
            compile_statement_effects_with_imports(&effects, &ReferenceImports::default())
                .expect("compile statement with imports");

        assert_eq!(lowered.effects.len(), 1);
        assert_eq!(
            lowered.exports.last_object_tag,
            RefState::Known(TagKey::from("destroyed_0")).into()
        );
    }

    #[test]
    fn compile_effects_with_explicit_frame_uses_annotated_reference_frames() {
        let effects = vec![
            EffectAst::Destroy {
                target: TargetAst::Object(
                    ObjectFilter::creature(),
                    Some(TextSpan::synthetic()),
                    None,
                ),
            },
            EffectAst::GrantPlayTaggedUntilEndOfTurn {
                tag: TagKey::from(IT_TAG),
                player: PlayerAst::You,
                allow_land: false,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: false,
            },
        ];

        let (compiled, _, frame_out) = compile_effects_with_explicit_frame(
            &effects,
            &mut IdGenContext::default(),
            LoweringFrame::default(),
        )
        .expect("compile with explicit frame");

        let grant = compiled
            .iter()
            .find_map(|effect| effect.downcast_ref::<GrantPlayTaggedEffect>())
            .expect("grant-play-tagged effect");
        assert_eq!(grant.tag.as_str(), "destroyed_0");
        assert_eq!(frame_out.last_object_tag.as_deref(), Some("destroyed_0"));
    }

    #[test]
    fn compile_may_branch_preserves_auto_tagged_destroy_followup() {
        let effects = vec![
            EffectAst::May {
                effects: vec![EffectAst::Destroy {
                    target: TargetAst::WithCount(
                        Box::new(TargetAst::Object(
                            ObjectFilter::creature(),
                            Some(TextSpan::synthetic()),
                            None,
                        )),
                        ChoiceCount::up_to(3),
                    ),
                }],
            },
            EffectAst::GrantPlayTaggedUntilEndOfTurn {
                tag: TagKey::from(IT_TAG),
                player: PlayerAst::You,
                allow_land: false,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: false,
            },
        ];

        let (compiled, _, frame_out) = compile_effects_with_explicit_frame(
            &effects,
            &mut IdGenContext::default(),
            LoweringFrame::default(),
        )
        .expect("compile may branch with tagged follow-up");

        let may = compiled[0]
            .downcast_ref::<crate::effects::MayEffect>()
            .expect("expected may effect");
        let tagged = may.effects[0]
            .downcast_ref::<TaggedEffect>()
            .expect("destroy inside may should stay tagged for follow-up linkage");
        let destroy = tagged
            .effect
            .downcast_ref::<crate::effects::DestroyEffect>()
            .expect("expected tagged destroy effect");
        assert_eq!(tagged.tag.as_str(), "destroyed_0");
        assert_eq!(
            destroy.spec,
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
                .with_count(ChoiceCount::up_to(3))
        );

        let grant = compiled[1]
            .downcast_ref::<GrantPlayTaggedEffect>()
            .expect("grant-play-tagged follow-up");
        assert_eq!(grant.tag.as_str(), "destroyed_0");
        assert_eq!(frame_out.last_object_tag.as_deref(), Some("destroyed_0"));
    }

    #[test]
    fn compile_for_each_tagged_rewrites_it_targets_to_iterated_object() {
        let effects = vec![EffectAst::ForEachTagged {
            tag: TagKey::from("revealed_0"),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::ItMatches(ObjectFilter::permanent()),
                if_true: vec![EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    zone: Zone::Battlefield,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Owner,
                    battlefield_tapped: false,
                    attached_to: None,
                }],
                if_false: vec![EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    zone: Zone::Graveyard,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                }],
            }],
        }];

        let (compiled, _, _) = compile_effects_with_explicit_frame(
            &effects,
            &mut IdGenContext::default(),
            LoweringFrame::default(),
        )
        .expect("compile for-each-tagged");

        let for_each = compiled[0]
            .downcast_ref::<ForEachTaggedEffect>()
            .expect("for-each-tagged effect");
        let conditional = for_each.effects[0]
            .downcast_ref::<ConditionalEffect>()
            .expect("conditional effect");
        let move_true = conditional.if_true[0]
            .downcast_ref::<MoveToZoneEffect>()
            .expect("true branch move");
        let move_false = conditional.if_false[0]
            .downcast_ref::<MoveToZoneEffect>()
            .expect("false branch move");

        assert!(matches!(
            conditional.condition,
            Condition::TaggedObjectMatches(ref tag, _)
                if tag.as_str() == IT_TAG
        ));
        assert!(matches!(move_true.target, ChooseSpec::Iterated));
        assert!(matches!(move_false.target, ChooseSpec::Iterated));
    }

    #[test]
    fn compile_next_spell_grant_after_targeted_player_effect_binds_that_player() {
        let effects = vec![
            EffectAst::AddManaAnyOneColor {
                amount: Value::Fixed(2),
                player: PlayerAst::Target,
            },
            EffectAst::GrantNextSpellAbilityThisTurn {
                player: PlayerAst::That,
                filter: ObjectFilter::spell().cast_by(PlayerFilter::IteratedPlayer),
                ability: GrantedAbilityAst::KeywordAction(
                    crate::cards::builders::KeywordAction::Cascade,
                ),
            },
        ];

        let (compiled, _, _) = compile_effects_with_explicit_frame(
            &effects,
            &mut IdGenContext::default(),
            LoweringFrame::default(),
        )
        .expect("targeted player followup should compile");

        let grant = compiled
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::GrantNextSpellAbilityEffect>())
            .expect("expected next-spell grant effect");
        assert!(
            !matches!(grant.player, PlayerFilter::IteratedPlayer),
            "grant player should bind to the targeted player, got {grant:?}"
        );
        assert!(
            !matches!(grant.filter.cast_by, Some(PlayerFilter::IteratedPlayer)),
            "grant filter should bind caster to the targeted player, got {grant:?}"
        );
    }

    #[test]
    fn compile_next_spell_grant_with_imported_target_player_binds_that_player() {
        let effects = vec![EffectAst::GrantNextSpellAbilityThisTurn {
            player: PlayerAst::That,
            filter: ObjectFilter::spell().cast_by(PlayerFilter::IteratedPlayer),
            ability: GrantedAbilityAst::KeywordAction(
                crate::cards::builders::KeywordAction::Cascade,
            ),
        }];

        let frame = LoweringFrame {
            last_player_filter: Some(PlayerFilter::target_player()),
            ..Default::default()
        };
        let (compiled, _, _) =
            compile_effects_with_explicit_frame(&effects, &mut IdGenContext::default(), frame)
                .expect("imported target-player followup should compile");

        let grant = compiled
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::GrantNextSpellAbilityEffect>())
            .expect("expected next-spell grant effect");
        assert!(
            !matches!(grant.player, PlayerFilter::IteratedPlayer),
            "grant player should bind to the imported targeted player, got {grant:?}"
        );
        assert!(
            !matches!(grant.filter.cast_by, Some(PlayerFilter::IteratedPlayer)),
            "grant filter should bind caster to the imported targeted player, got {grant:?}"
        );
    }
}
