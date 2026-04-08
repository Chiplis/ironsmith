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

pub(crate) fn compile_trigger_spec(trigger: TriggerSpec) -> Trigger {
    match trigger {
        TriggerSpec::StateBased { display, .. } => Trigger::state_based(display),
        TriggerSpec::ThisAttacks => Trigger::this_attacks(),
        TriggerSpec::ThisAttacksWithExactlyNOthers(other_count) => {
            Trigger::this_attacks_with_exact_n_others(other_count as usize)
        }
        TriggerSpec::ThisAttacksAndIsntBlocked => Trigger::this_attacks_and_isnt_blocked(),
        TriggerSpec::ThisAttacksWhileSaddled => Trigger::this_attacks_while_saddled(),
        TriggerSpec::Attacks(filter) => Trigger::attacks(filter),
        TriggerSpec::AttacksAndIsntBlocked(filter) => Trigger::attacks_and_isnt_blocked(filter),
        TriggerSpec::AttacksWhileSaddled(filter) => Trigger::attacks_while_saddled(filter),
        TriggerSpec::AttacksOneOrMore(filter) => Trigger::attacks_one_or_more(filter),
        TriggerSpec::AttacksOneOrMoreWithMinTotal {
            filter,
            min_total_attackers,
        } => Trigger::attacks_one_or_more_with_min_total(filter, min_total_attackers as usize),
        TriggerSpec::AttacksAlone(filter) => Trigger::attacks_alone(filter),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(filter) => Trigger::attacks_you(filter),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(filter) => {
            Trigger::attacks_you_one_or_more(filter)
        }
        TriggerSpec::ThisBlocks => Trigger::this_blocks(),
        TriggerSpec::ThisBlocksObject(filter) => Trigger::this_blocks_object(filter),
        TriggerSpec::Blocks(filter) => Trigger::blocks(filter),
        TriggerSpec::ThisBecomesBlocked => Trigger::this_becomes_blocked(),
        TriggerSpec::ThisDies => Trigger::this_dies(),
        TriggerSpec::ThisDiesOrIsExiled => Trigger::this_dies_or_is_exiled(),
        TriggerSpec::ThisLeavesBattlefield => Trigger::this_leaves_battlefield(),
        TriggerSpec::ThisBecomesMonstrous => Trigger::this_becomes_monstrous(),
        TriggerSpec::ThisBecomesTapped => Trigger::becomes_tapped(),
        TriggerSpec::PermanentBecomesTapped(filter) => Trigger::permanent_becomes_tapped(filter),
        TriggerSpec::ThisBecomesUntapped => Trigger::becomes_untapped(),
        TriggerSpec::ThisTurnedFaceUp => Trigger::this_is_turned_face_up(),
        TriggerSpec::TurnedFaceUp(filter) => Trigger::turned_face_up(filter),
        TriggerSpec::ThisBecomesTargeted => Trigger::becomes_targeted(),
        TriggerSpec::BecomesTargeted(filter) => Trigger::becomes_targeted_object(filter),
        TriggerSpec::ThisBecomesTargetedBySpell(filter) => {
            Trigger::becomes_targeted_by_spell(filter)
        }
        TriggerSpec::BecomesTargetedBySourceController {
            target,
            source_controller,
        } => Trigger::becomes_targeted_by_source_controller(target, source_controller),
        TriggerSpec::ThisDealsDamage => Trigger::this_deals_damage(),
        TriggerSpec::ThisDealsDamageToPlayer { player, amount } => {
            Trigger::this_deals_damage_to_player(player, amount)
        }
        TriggerSpec::ThisDealsDamageTo(filter) => Trigger::this_deals_damage_to(filter),
        TriggerSpec::ThisDealsCombatDamage => Trigger::this_deals_combat_damage(),
        TriggerSpec::ThisDealsCombatDamageTo(filter) => {
            Trigger::this_deals_combat_damage_to(filter)
        }
        TriggerSpec::DealsDamage(filter) => Trigger::deals_damage(filter),
        TriggerSpec::DealsCombatDamage(filter) => Trigger::deals_combat_damage(filter),
        TriggerSpec::DealsCombatDamageTo { source, target } => {
            Trigger::deals_combat_damage_to(source, target)
        }
        TriggerSpec::PlayerPlaysLand { player, filter } => {
            Trigger::player_plays_land(player, filter)
        }
        TriggerSpec::PlayerGivesGift(player) => Trigger::player_gives_gift(player),
        TriggerSpec::PlayerSearchesLibrary(player) => Trigger::player_searches_library(player),
        TriggerSpec::PlayerShufflesLibrary {
            player,
            caused_by_effect,
            source_controller_shuffles,
        } => Trigger::player_shuffles_library(player, caused_by_effect, source_controller_shuffles),
        TriggerSpec::PlayerTapsForMana { player, filter } => {
            Trigger::player_taps_for_mana(player, filter)
        }
        TriggerSpec::AbilityActivated {
            activator,
            filter,
            non_mana_only,
        } => Trigger::ability_activated_qualified(activator, filter, non_mana_only),
        TriggerSpec::ThisIsDealtDamage => Trigger::is_dealt_damage(ChooseSpec::Source),
        TriggerSpec::IsDealtDamage(filter) => Trigger::is_dealt_damage(ChooseSpec::Object(filter)),
        TriggerSpec::YouGainLife => Trigger::you_gain_life(),
        TriggerSpec::YouGainLifeDuringTurn(during_turn) => {
            Trigger::you_gain_life_during_turn(during_turn)
        }
        TriggerSpec::PlayerLosesLife(player) => Trigger::player_loses_life(player),
        TriggerSpec::PlayerLosesLifeDuringTurn {
            player,
            during_turn,
        } => Trigger::player_loses_life_during_turn(player, during_turn),
        TriggerSpec::YouDrawCard => Trigger::you_draw_card(),
        TriggerSpec::PlayerDrawsCard(player) => Trigger::player_draws_card(player),
        TriggerSpec::PlayerDrawsCardNotDuringTurn {
            player,
            during_turn,
        } => Trigger::player_draws_card_not_during_turn(player, during_turn),
        TriggerSpec::PlayerDrawsNthCardEachTurn {
            player,
            card_number,
        } => Trigger::player_draws_nth_card_each_turn(player, card_number),
        TriggerSpec::PlayerDiscardsCard {
            player,
            filter,
            cause_controller,
            effect_like_only,
        } => {
            if let Some(cause_controller) = cause_controller {
                Trigger::player_discards_card_caused_by_controller(
                    player,
                    filter,
                    cause_controller,
                    effect_like_only,
                )
            } else {
                Trigger::player_discards_card(player, filter)
            }
        }
        TriggerSpec::PlayerRevealsCard {
            player,
            filter,
            from_source,
        } => Trigger::player_reveals_card(player, filter, from_source),
        TriggerSpec::PlayerSacrifices { player, filter } => {
            Trigger::player_sacrifices(player, filter)
        }
        TriggerSpec::Dies(filter) => Trigger::dies(filter),
        TriggerSpec::PutIntoGraveyard(filter) => Trigger::put_into_graveyard(filter),
        TriggerSpec::PutIntoGraveyardFromZone { filter, from } => Trigger::new(
            crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .from(from)
                .to(crate::zone::Zone::Graveyard)
                .filter(filter),
        ),
        TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller,
            one_or_more,
        } => {
            let mut trigger = crate::triggers::CounterPutOnTrigger::new(filter);
            if let Some(counter_type) = counter_type {
                trigger = trigger.counter_type(counter_type);
            }
            if let Some(source_controller) = source_controller {
                trigger = trigger.source_controller(source_controller);
            }
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::DiesCreatureDealtDamageByThisTurn { victim, damager } => match damager {
            DamageBySpec::ThisCreature => {
                Trigger::creature_dealt_damage_by_this_creature_this_turn_dies(victim)
            }
            DamageBySpec::EquippedCreature => {
                Trigger::creature_dealt_damage_by_equipped_creature_this_turn_dies(victim)
            }
            DamageBySpec::EnchantedCreature => {
                Trigger::creature_dealt_damage_by_enchanted_creature_this_turn_dies(victim)
            }
        },
        TriggerSpec::SpellCast {
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => Trigger::spell_cast_qualified(
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        ),
        TriggerSpec::SpellCopied { filter, copier } => Trigger::spell_copied(filter, copier),
        TriggerSpec::EntersBattlefield(filter) => Trigger::enters_battlefield(filter),
        TriggerSpec::EntersBattlefieldOneOrMore(filter) => {
            Trigger::enters_battlefield_one_or_more(filter)
        }
        TriggerSpec::EntersBattlefieldFromZone {
            mut filter,
            from,
            owner,
            one_or_more,
        } => {
            if let Some(owner) = owner {
                filter.owner = Some(owner);
            }
            let trigger = crate::triggers::ZoneChangeTrigger::new()
                .from(from)
                .to(crate::zone::Zone::Battlefield)
                .filter(filter);
            if one_or_more {
                Trigger::new(trigger.count(crate::triggers::CountMode::OneOrMore))
            } else {
                Trigger::new(trigger)
            }
        }
        TriggerSpec::EntersBattlefieldTapped(filter) => Trigger::enters_battlefield_tapped(filter),
        TriggerSpec::EntersBattlefieldUntapped(filter) => {
            Trigger::enters_battlefield_untapped(filter)
        }
        TriggerSpec::BeginningOfUpkeep(player) => Trigger::beginning_of_upkeep(player),
        TriggerSpec::BeginningOfDrawStep(player) => Trigger::beginning_of_draw_step(player),
        TriggerSpec::BeginningOfCombat(player) => Trigger::beginning_of_combat(player),
        TriggerSpec::BeginningOfEndStep(player) => Trigger::beginning_of_end_step(player),
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerSpec::BeginningOfPostcombatMain(player) => {
            Trigger::beginning_of_postcombat_main_phase(player)
        }
        TriggerSpec::ThisEntersBattlefield => Trigger::this_enters_battlefield(),
        TriggerSpec::ThisEntersBattlefieldFromZone {
            mut subject_filter,
            from,
            owner,
        } => {
            if let Some(owner) = owner {
                subject_filter.owner = Some(owner);
            }
            Trigger::new(
                crate::triggers::ZoneChangeTrigger::new()
                    .from(from)
                    .to(crate::zone::Zone::Battlefield)
                    .filter(subject_filter)
                    .this(),
            )
        }
        TriggerSpec::ThisDealsCombatDamageToPlayer => Trigger::this_deals_combat_damage_to_player(),
        TriggerSpec::DealsCombatDamageToPlayer { source, player } => {
            Trigger::deals_combat_damage_to_player(source, player)
        }
        TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player } => {
            Trigger::deals_combat_damage_to_player_one_or_more(source, player)
        }
        TriggerSpec::YouCastThisSpell => Trigger::you_cast_this_spell(),
        TriggerSpec::KeywordAction {
            action,
            player,
            source_filter,
        } => match source_filter {
            Some(filter) => Trigger::keyword_action_matching_object(action, player, filter),
            None => Trigger::keyword_action(action, player),
        },
        TriggerSpec::KeywordActionFromSource { action, player } => {
            Trigger::keyword_action_from_source(action, player)
        }
        TriggerSpec::WinsClash { player } => Trigger::wins_clash(player),
        TriggerSpec::Expend { player, amount } => Trigger::expend(amount, player),
        TriggerSpec::SagaChapter(chapters) => Trigger::saga_chapter(chapters),
        TriggerSpec::HauntedCreatureDies => Trigger::custom(
            "haunted_creature_dies",
            "When the creature it haunts dies".to_string(),
        ),
        TriggerSpec::Either(left, right) => {
            Trigger::either(compile_trigger_spec(*left), compile_trigger_spec(*right))
        }
    }
}

pub(crate) fn ensure_concrete_trigger_spec(trigger: &TriggerSpec) -> Result<(), CardTextError> {
    match trigger {
        TriggerSpec::Either(left, right) => {
            ensure_concrete_trigger_spec(left)?;
            ensure_concrete_trigger_spec(right)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(crate) fn compile_statement_effects(
    effects: &[EffectAst],
) -> Result<Vec<Effect>, CardTextError> {
    Ok(
        compile_statement_effects_with_imports(effects, &ReferenceImports::default())?
            .effects
            .to_vec(),
    )
}

pub(crate) fn compile_statement_effects_with_imports(
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared = rewrite_prepare_effects_for_lowering(effects, imports.clone())?;
    materialize_prepared_statement_effects(&prepared)
}

pub(crate) fn materialize_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    if let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        },
    ] = prepared.effects.as_slice()
    {
        let default_effects =
            compile_statement_effects_with_imports(if_false, &prepared.imports)?.effects;
        let replacement_effects =
            compile_statement_effects_with_imports(if_true, &prepared.imports)?.effects;
        let condition = compile_condition_from_predicate_ast_with_env(
            predicate,
            &prepared.initial_env,
            prepared.imports.last_object_tag.as_ref(),
        )?;
        return Ok(LoweredEffects {
            effects: crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment {
                    default_effects: default_effects.flattened_default_effects().to_vec(),
                    self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                        condition,
                        replacement_effects.flattened_default_effects().to_vec(),
                    )],
                },
            ]),
            choices: Vec::new(),
            exports: prepared.exports.clone(),
        });
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, _) = compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = fold_local_zone_rewrite_self_replacements(compiled);
    let final_env = ctx.reference_env();
    Ok(LoweredEffects {
        effects: crate::resolution::ResolutionProgram::from_effects(prepend_effect_prelude(
            compiled,
            compile_effect_prelude_tags(&prepared.prelude),
        )),
        choices: Vec::new(),
        exports: ReferenceExports::from_env(&final_env),
    })
}

pub(crate) fn materialize_prepared_effects_with_trigger_context(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    if let Some((
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        },
        prefix_effects,
    )) = prepared.effects.split_last()
        && prefix_effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::SelfReplacement { .. }))
    {
        let prefix_lowered =
            compile_statement_effects_with_imports(prefix_effects, &prepared.imports)?;
        let default_lowered = compile_statement_effects_with_imports(if_false, &prepared.imports)?;
        let replacement_lowered =
            compile_statement_effects_with_imports(if_true, &prepared.imports)?;
        let condition = compile_condition_from_predicate_ast_with_env(
            predicate,
            &prepared.initial_env,
            prepared.imports.last_object_tag.as_ref(),
        )?;
        let mut default_effects = prefix_lowered.effects.flattened_default_effects().to_vec();
        default_effects.extend(default_lowered.effects.flattened_default_effects().to_vec());

        let mut choices = prefix_lowered.choices;
        choices.extend(default_lowered.choices);
        choices.extend(replacement_lowered.choices);
        return Ok(LoweredEffects {
            effects: crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment {
                    default_effects,
                    self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                        condition,
                        replacement_lowered
                            .effects
                            .flattened_default_effects()
                            .to_vec(),
                    )],
                },
            ]),
            choices,
            exports: prepared.exports.clone(),
        });
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, choices) =
        compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = fold_local_zone_rewrite_self_replacements(compiled);
    let final_env = ctx.reference_env();
    Ok(LoweredEffects {
        effects: crate::resolution::ResolutionProgram::from_effects(prepend_effect_prelude(
            compiled,
            compile_effect_prelude_tags(&prepared.prelude),
        )),
        choices,
        exports: ReferenceExports::from_env(&final_env),
    })
}

pub(crate) fn materialize_prepared_triggered_effects(
    prepared: &PreparedTriggeredEffectsForLowering,
) -> Result<(LoweredEffects, Option<Condition>), CardTextError> {
    let mut lowered = materialize_prepared_effects_with_trigger_context(&prepared.prepared)?;
    strip_erroneous_meld_player_exile_effect(&mut lowered);
    let intervening_if = prepared
        .intervening_if
        .as_ref()
        .map(compile_prepared_predicate_for_lowering)
        .transpose()?;
    Ok((lowered, intervening_if))
}

fn strip_erroneous_meld_player_exile_effect(lowered: &mut LoweredEffects) {
    let flattened = lowered.effects.flattened_default_effects();
    if flattened.len() < 2 {
        return;
    }

    let mut rewritten = Vec::with_capacity(flattened.len());
    let mut idx = 0usize;
    while idx < flattened.len() {
        let skip_erroneous_exile = idx + 1 < flattened.len()
            && flattened[idx]
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .is_some_and(|effect| {
                    effect.zone == crate::zone::Zone::Exile
                        && effect.target
                            == crate::target::ChooseSpec::Player(
                                crate::target::PlayerFilter::IteratedPlayer,
                            )
                })
            && flattened[idx + 1]
                .downcast_ref::<crate::effects::MeldEffect>()
                .is_some();
        if skip_erroneous_exile {
            idx += 1;
            continue;
        }

        rewritten.push(flattened[idx].clone());
        idx += 1;
    }

    if rewritten.len() != flattened.len() {
        lowered.effects = crate::resolution::ResolutionProgram::from_effects(rewritten);
    }
}

fn fold_local_zone_rewrite_self_replacements(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::new();
    let mut idx = 0usize;

    while idx < effects.len() {
        if idx + 1 < effects.len()
            && let Some(with_id) = effects[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = effects[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition == with_id.id
            && if_effect.predicate == EffectPredicate::Happened
            && if_effect.else_.is_empty()
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&if_effect.then, &with_id.effect)
        {
            rewritten.push(Effect::with_id(
                with_id.id.0,
                Effect::new(crate::effects::LocalRewriteEffect::new(
                    (*with_id.effect).clone(),
                    zone_replacements,
                )),
            ));
            idx += 2;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }

    rewritten
}

fn extract_local_zone_replacement_followups(
    effects: &[Effect],
    antecedent: &Effect,
) -> Option<Vec<crate::effects::RegisterZoneReplacementEffect>> {
    let mut replacements = Vec::new();
    let antecedent_target = antecedent.0.get_target_spec().cloned();
    for effect in effects {
        let mut register = effect
            .downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?
            .clone();
        if register.mode != crate::effects::ReplacementApplyMode::OneShot {
            return None;
        }
        if choose_spec_contains_it_tag(&register.target)
            && let Some(target_spec) = &antecedent_target
        {
            register.target = target_spec.clone();
        }
        replacements.push(register);
    }
    Some(replacements)
}

fn choose_spec_contains_it_tag(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(tag) => tag.as_str() == IT_TAG,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_contains_it_tag(inner)
        }
        _ => false,
    }
}

fn validate_unbound_iterated_player(
    debug_repr: String,
    context: &str,
) -> Result<(), CardTextError> {
    if str_contains(debug_repr.as_str(), "IteratedPlayer") {
        return Err(CardTextError::InvariantViolation(format!(
            "{context} references PlayerFilter::IteratedPlayer without a trigger or loop that binds \"that player\": {debug_repr}"
        )));
    }
    Ok(())
}

fn validate_choose_specs_for_iterated_player(
    choices: &[ChooseSpec],
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if iterated_player_bound {
        return Ok(());
    }
    for choice in choices {
        validate_unbound_iterated_player(format!("{choice:?}"), context)?;
    }
    Ok(())
}

fn validate_condition_for_iterated_player(
    condition: &Condition,
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if iterated_player_bound {
        return Ok(());
    }
    validate_unbound_iterated_player(format!("{condition:?}"), context)
}

fn validate_effects_for_iterated_player(
    effects: &[Effect],
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        validate_effect_for_iterated_player(effect, iterated_player_bound, context)?;
    }
    Ok(())
}

fn validate_effect_for_iterated_player(
    effect: &Effect,
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return validate_effects_for_iterated_player(
            &sequence.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        if !iterated_player_bound && let Some(decider) = &may.decider {
            validate_unbound_iterated_player(format!("{decider:?}"), context)?;
        }
        return validate_effects_for_iterated_player(&may.effects, iterated_player_bound, context);
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", unless_pays.player), context)?;
        }
        return validate_effects_for_iterated_player(
            &unless_pays.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", unless_action.player), context)?;
        }
        validate_effects_for_iterated_player(
            &unless_action.effects,
            iterated_player_bound,
            context,
        )?;
        return validate_effects_for_iterated_player(
            &unless_action.alternative,
            iterated_player_bound,
            context,
        );
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", for_players.filter), context)?;
        }
        return validate_effects_for_iterated_player(&for_players.effects, true, context);
    }
    if let Some(for_each_object) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", for_each_object.filter), context)?;
        }
        return validate_effects_for_iterated_player(&for_each_object.effects, true, context);
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return validate_effects_for_iterated_player(&for_each_tagged.effects, true, context);
    }
    if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return validate_effects_for_iterated_player(&for_each_controller.effects, true, context);
    }
    if let Some(for_each_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        return validate_effects_for_iterated_player(&for_each_player.effects, true, context);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        validate_condition_for_iterated_player(
            &conditional.condition,
            iterated_player_bound,
            context,
        )?;
        validate_effects_for_iterated_player(&conditional.if_true, iterated_player_bound, context)?;
        return validate_effects_for_iterated_player(
            &conditional.if_false,
            iterated_player_bound,
            context,
        );
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        validate_effects_for_iterated_player(&if_effect.then, iterated_player_bound, context)?;
        return validate_effects_for_iterated_player(
            &if_effect.else_,
            iterated_player_bound,
            context,
        );
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return validate_effect_for_iterated_player(&tagged.effect, iterated_player_bound, context);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return validate_effect_for_iterated_player(
            &with_id.effect,
            iterated_player_bound,
            context,
        );
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        for mode in &choose_mode.modes {
            validate_effects_for_iterated_player(&mode.effects, iterated_player_bound, context)?;
        }
        return Ok(());
    }
    if let Some(vote) = effect.downcast_ref::<crate::effects::VoteEffect>() {
        if let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice {
            for option in options {
                validate_effects_for_iterated_player(
                    &option.effects_per_vote,
                    iterated_player_bound,
                    context,
                )?;
            }
        }
        return Ok(());
    }
    if let Some(reflexive) = effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
        validate_choose_specs_for_iterated_player(&reflexive.choices, false, context)?;
        return validate_effects_for_iterated_player(&reflexive.effects, false, context);
    }
    if let Some(schedule_delayed) =
        effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
    {
        if !iterated_player_bound {
            validate_unbound_iterated_player(
                format!("{:?}", schedule_delayed.controller),
                context,
            )?;
            if let Some(filter) = &schedule_delayed.target_filter {
                validate_unbound_iterated_player(format!("{filter:?}"), context)?;
            }
        }
        return validate_effects_for_iterated_player(&schedule_delayed.effects, false, context);
    }
    if let Some(schedule_when_leaves) =
        effect.downcast_ref::<crate::effects::ScheduleEffectsWhenTaggedLeavesEffect>()
    {
        if !iterated_player_bound {
            validate_unbound_iterated_player(
                format!("{:?}", schedule_when_leaves.controller),
                context,
            )?;
        }
        return validate_effects_for_iterated_player(&schedule_when_leaves.effects, false, context);
    }
    if let Some(haunt) = effect.downcast_ref::<crate::effects::HauntExileEffect>() {
        validate_choose_specs_for_iterated_player(&haunt.haunt_choices, false, context)?;
        return validate_effects_for_iterated_player(&haunt.haunt_effects, false, context);
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        if !iterated_player_bound && matches!(choose.chooser, PlayerFilter::Target(_)) {
            return Ok(());
        }
    }

    if !iterated_player_bound {
        validate_unbound_iterated_player(format!("{effect:?}"), context)?;
    }
    Ok(())
}

pub(crate) fn validate_iterated_player_bindings_in_lowered_effects(
    lowered: &LoweredEffects,
    initial_iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    let iterated_player_bound = initial_iterated_player_bound || lowered.exports.iterated_player;
    validate_effects_for_iterated_player(&lowered.effects, iterated_player_bound, context)?;
    validate_choose_specs_for_iterated_player(&lowered.choices, iterated_player_bound, context)
}

pub(crate) fn trigger_binds_iterated_player(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::SpellCast { .. }
        | TriggerSpec::SpellCopied { .. }
        | TriggerSpec::PlayerLosesLife(_)
        | TriggerSpec::PlayerLosesLifeDuringTurn { .. }
        | TriggerSpec::PlayerDrawsCard(_)
        | TriggerSpec::PlayerDrawsCardNotDuringTurn { .. }
        | TriggerSpec::PlayerDrawsNthCardEachTurn { .. }
        | TriggerSpec::PlayerDiscardsCard { .. }
        | TriggerSpec::PlayerRevealsCard { .. }
        | TriggerSpec::PlayerPlaysLand { .. }
        | TriggerSpec::PlayerGivesGift(_)
        | TriggerSpec::PlayerSearchesLibrary(_)
        | TriggerSpec::PlayerShufflesLibrary { .. }
        | TriggerSpec::PlayerTapsForMana { .. }
        | TriggerSpec::PlayerSacrifices { .. }
        | TriggerSpec::BeginningOfUpkeep(_)
        | TriggerSpec::BeginningOfDrawStep(_)
        | TriggerSpec::BeginningOfCombat(_)
        | TriggerSpec::BeginningOfEndStep(_)
        | TriggerSpec::BeginningOfPrecombatMain(_)
        | TriggerSpec::BeginningOfPostcombatMain(_)
        | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_)
        | TriggerSpec::KeywordAction { .. }
        | TriggerSpec::KeywordActionFromSource { .. }
        | TriggerSpec::WinsClash { .. }
        | TriggerSpec::Expend { .. } => true,
        TriggerSpec::StateBased { .. } => false,
        TriggerSpec::BecomesTargetedBySourceController {
            source_controller, ..
        } => *source_controller != PlayerFilter::Any,
        TriggerSpec::Either(left, right) => {
            trigger_binds_iterated_player(left) && trigger_binds_iterated_player(right)
        }
        _ => false,
    }
}

pub(crate) fn compile_effect_prelude_tags(prelude: &[EffectPreludeTag]) -> Vec<Effect> {
    prelude
        .iter()
        .map(|tag| match tag {
            EffectPreludeTag::AttachedSource(tag) => Effect::tag_attached_to_source(tag.as_str()),
            EffectPreludeTag::TriggeringObject(tag) => Effect::tag_triggering_object(tag.as_str()),
            EffectPreludeTag::TriggeringDamageTarget(tag) => {
                Effect::tag_triggering_damage_target(tag.as_str())
            }
        })
        .collect()
}

pub(crate) fn compile_condition_from_predicate_ast_with_env(
    predicate: &PredicateAst,
    refs: &ReferenceEnv,
    saved_last_object_tag: Option<&TagKey>,
) -> Result<Condition, CardTextError> {
    let mut ctx = EffectLoweringContext::new();
    let reference_env: crate::cards::builders::ReferenceEnv = refs.clone().into();
    ctx.apply_reference_env(&reference_env);
    let saved_last_tag = saved_last_object_tag.map(|tag| tag.as_str().to_string());
    compile_condition_from_predicate_ast(predicate, &mut ctx, &saved_last_tag)
}

pub(crate) fn compile_prepared_predicate_for_lowering(
    prepared: &PreparedPredicateForLowering,
) -> Result<Condition, CardTextError> {
    compile_condition_from_predicate_ast_with_env(
        &prepared.predicate,
        &prepared.reference_env,
        prepared.saved_last_object_tag.as_ref(),
    )
}

fn prepend_effect_prelude(mut compiled: Vec<Effect>, mut prelude: Vec<Effect>) -> Vec<Effect> {
    if prelude.is_empty() {
        return compiled;
    }
    prelude.append(&mut compiled);
    prelude
}

pub(crate) fn inferred_trigger_player_filter(trigger: &TriggerSpec) -> Option<PlayerFilter> {
    match trigger {
        TriggerSpec::StateBased { .. } => None,
        TriggerSpec::EntersBattlefield(_)
        | TriggerSpec::EntersBattlefieldOneOrMore(_)
        | TriggerSpec::EntersBattlefieldFromZone { .. }
        | TriggerSpec::EntersBattlefieldTapped(_)
        | TriggerSpec::EntersBattlefieldUntapped(_) => Some(PlayerFilter::ControllerOf(
            ObjectRef::tagged(TagKey::from("triggering")),
        )),
        TriggerSpec::SpellCast { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::SpellCopied { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLife(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLifeDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCard(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCardNotDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsNthCardEachTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDiscardsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerRevealsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerPlaysLand { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerGivesGift(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerSearchesLibrary(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerShufflesLibrary { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerTapsForMana { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::AbilityActivated { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerSacrifices { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::ThisDealsDamageToPlayer { .. }
        | TriggerSpec::ThisDealsCombatDamageToPlayer
        | TriggerSpec::DealsCombatDamageToPlayer { .. } => Some(PlayerFilter::DamagedPlayer),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_) => {
            Some(PlayerFilter::IteratedPlayer)
        }
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::BeginningOfPostcombatMain(player)
        | TriggerSpec::KeywordAction { player, .. }
        | TriggerSpec::KeywordActionFromSource { player, .. }
        | TriggerSpec::WinsClash { player } => {
            if *player == PlayerFilter::Any {
                Some(PlayerFilter::Active)
            } else {
                Some(PlayerFilter::IteratedPlayer)
            }
        }
        TriggerSpec::BecomesTargetedBySourceController {
            source_controller, ..
        } => {
            if *source_controller == PlayerFilter::Any {
                Some(PlayerFilter::Active)
            } else {
                Some(PlayerFilter::IteratedPlayer)
            }
        }
        TriggerSpec::Either(left, right) => {
            let left_filter = inferred_trigger_player_filter(left);
            let right_filter = inferred_trigger_player_filter(right);
            if left_filter == right_filter {
                left_filter
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn trigger_binds_player_reference_context(trigger: &TriggerSpec) -> bool {
    trigger_binds_iterated_player(trigger)
        || inferred_trigger_player_filter(trigger)
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
}

pub(crate) fn trigger_supports_event_value(trigger: &TriggerSpec, spec: &EventValueSpec) -> bool {
    match spec {
        EventValueSpec::Amount | EventValueSpec::LifeAmount => match trigger {
            TriggerSpec::YouGainLife
            | TriggerSpec::YouGainLifeDuringTurn(_)
            | TriggerSpec::PlayerLosesLife(_)
            | TriggerSpec::PlayerLosesLifeDuringTurn { .. }
            | TriggerSpec::ThisIsDealtDamage
            | TriggerSpec::IsDealtDamage(_)
            | TriggerSpec::ThisDealsDamage
            | TriggerSpec::ThisDealsDamageTo(_)
            | TriggerSpec::DealsDamage(_)
            | TriggerSpec::ThisDealsCombatDamage
            | TriggerSpec::ThisDealsCombatDamageTo(_)
            | TriggerSpec::DealsCombatDamage(_)
            | TriggerSpec::DealsCombatDamageTo { .. }
            | TriggerSpec::ThisDealsCombatDamageToPlayer
            | TriggerSpec::DealsCombatDamageToPlayer { .. }
            | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
            | TriggerSpec::KeywordAction { .. }
            | TriggerSpec::KeywordActionFromSource { .. }
            | TriggerSpec::CounterPutOn { .. } => true,
            TriggerSpec::StateBased { .. } => false,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec)
                    && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
        EventValueSpec::BlockersBeyondFirst { .. } => match trigger {
            TriggerSpec::ThisBecomesBlocked => true,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec)
                    && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
    }
}

pub(crate) fn compile_trigger_effects(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let lowered =
        compile_trigger_effects_with_imports(trigger, effects, &ReferenceImports::default())?;
    Ok((lowered.effects.to_vec(), lowered.choices))
}

pub(crate) fn compile_trigger_effects_with_imports(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared = rewrite_prepare_effects_with_trigger_context_for_lowering(
        trigger,
        effects,
        imports.clone(),
    )?;
    materialize_prepared_effects_with_trigger_context(&prepared)
}

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

pub(crate) fn effects_reference_tag(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag(effect, tag))
}

// Keep direct target-bearing variants in one place to prevent drift across
// tag-reference checks and tag-span collection.
macro_rules! direct_target_effect_variants {
    ($target:ident) => {
        EffectAst::DealDamage {
            target: $target,
            ..
        } | EffectAst::Counter { target: $target }
            | EffectAst::CounterUnlessPays {
                target: $target,
                ..
            }
            | EffectAst::Explore { target: $target }
            | EffectAst::Connive { target: $target }
            | EffectAst::Goad { target: $target }
            | EffectAst::PutCounters {
                target: $target,
                ..
            }
            | EffectAst::PutOrRemoveCounters {
                target: $target,
                ..
            }
            | EffectAst::ForEachCounterKindPutOrRemove { target: $target }
            | EffectAst::Tap { target: $target }
            | EffectAst::Untap { target: $target }
            | EffectAst::PhaseOut { target: $target }
            | EffectAst::RemoveFromCombat { target: $target }
            | EffectAst::TapOrUntap { target: $target }
            | EffectAst::Destroy { target: $target }
            | EffectAst::DestroyNoRegeneration { target: $target }
            | EffectAst::Exile {
                target: $target,
                ..
            }
            | EffectAst::ExileWhenSourceLeaves { target: $target }
            | EffectAst::SacrificeSourceWhenLeaves { target: $target }
            | EffectAst::ExileUntilSourceLeaves {
                target: $target,
                ..
            }
            | EffectAst::LookAtHand { target: $target }
            | EffectAst::Transform { target: $target }
            | EffectAst::Convert { target: $target }
            | EffectAst::Flip { target: $target }
            | EffectAst::Regenerate { target: $target }
            | EffectAst::TargetOnly { target: $target }
            | EffectAst::RemoveUpToAnyCounters {
                target: $target,
                ..
            }
            | EffectAst::ReturnToHand {
                target: $target,
                ..
            }
            | EffectAst::ReturnToBattlefield {
                target: $target,
                ..
            }
            | EffectAst::Pump {
                target: $target,
                ..
            }
            | EffectAst::BecomeBasicLandType {
                target: $target,
                ..
            }
            | EffectAst::BecomeBasicLandTypeChoice {
                target: $target,
                ..
            }
            | EffectAst::BecomeCreatureTypeChoice {
                target: $target,
                ..
            }
            | EffectAst::BecomeColorChoice {
                target: $target,
                ..
            }
            | EffectAst::SetBasePower {
                target: $target,
                ..
            }
            | EffectAst::SetBasePowerToughness {
                target: $target,
                ..
            }
            | EffectAst::BecomeBasePtCreature {
                target: $target,
                ..
            }
            | EffectAst::AddCardTypes {
                target: $target,
                ..
            }
            | EffectAst::RemoveCardTypes {
                target: $target,
                ..
            }
            | EffectAst::AddSubtypes {
                target: $target,
                ..
            }
            | EffectAst::SetColors {
                target: $target,
                ..
            }
            | EffectAst::MakeColorless {
                target: $target,
                ..
            }
            | EffectAst::PumpForEach {
                target: $target,
                ..
            }
            | EffectAst::PumpByLastEffect {
                target: $target,
                ..
            }
            | EffectAst::GrantAbilitiesToTarget {
                target: $target,
                ..
            }
            | EffectAst::GrantToTarget {
                target: $target,
                ..
            }
            | EffectAst::RemoveAbilitiesFromTarget {
                target: $target,
                ..
            }
            | EffectAst::GrantAbilitiesChoiceToTarget {
                target: $target,
                ..
            }
            | EffectAst::GrantProtectionChoice {
                target: $target,
                ..
            }
            | EffectAst::PreventDamage {
                target: $target,
                ..
            }
            | EffectAst::PreventAllDamageToTarget {
                target: $target,
                ..
            }
            | EffectAst::PreventDamageToTargetPutCounters {
                target: $target,
                ..
            }
            | EffectAst::RedirectNextDamageFromSourceToTarget {
                target: $target,
                ..
            }
            | EffectAst::RedirectNextTimeDamageToSource {
                target: $target,
                ..
            }
            | EffectAst::GainControl {
                target: $target,
                ..
            }
            | EffectAst::CopySpell {
                target: $target,
                ..
            }
            | EffectAst::MoveToLibraryNthFromTop {
                target: $target,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                source: $target,
                ..
            }
            | EffectAst::PreventAllCombatDamageFromSource {
                source: $target,
                ..
            }
    };
}

fn with_direct_effect_targets(effect: &EffectAst, mut visit: impl FnMut(&TargetAst)) {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            visit(creature1);
            visit(creature2);
        }
        EffectAst::FightIterated { creature2 } => {
            visit(creature2);
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            visit(source);
            visit(target);
        }
        EffectAst::BecomeCopy { target, source, .. } => {
            visit(target);
            visit(source);
        }
        direct_target_effect_variants!(target) => {
            visit(target);
        }
        EffectAst::MoveToZone {
            target,
            attached_to,
            ..
        } => {
            visit(target);
            if let Some(attach_target) = attached_to {
                visit(attach_target);
            }
        }
        EffectAst::MoveAllCounters { from, to } => {
            visit(from);
            visit(to);
        }
        EffectAst::DestroyAllAttachedTo { target, .. } => {
            visit(target);
        }
        EffectAst::Attach { object, target } => {
            visit(object);
            visit(target);
        }
        EffectAst::RetargetStackObject { target, mode, .. } => {
            visit(target);
            if let RetargetModeAst::OneToFixed { target: fixed } = mode {
                visit(fixed);
            }
        }
        _ => {}
    }
}

fn direct_effect_targets_reference_tag(effect: &EffectAst, tag: &str) -> bool {
    let mut references = false;
    with_direct_effect_targets(effect, |target| {
        if !references {
            references = target_references_tag(target, tag);
        }
    });
    references
}

fn filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
}

fn effect_tagged_filter(effect: &EffectAst) -> Option<&ObjectFilter> {
    match effect {
        EffectAst::DealDamageEach { filter, .. }
        | EffectAst::PutCountersAll { filter, .. }
        | EffectAst::RemoveCountersAll { filter, .. }
        | EffectAst::DoubleCountersOnEach { filter, .. }
        | EffectAst::TapAll { filter }
        | EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::RegenerateAll { filter }
        | EffectAst::DestroyAll { filter }
        | EffectAst::DestroyAllOfChosenColor { filter }
        | EffectAst::ExileAll { filter, .. }
        | EffectAst::PreventDamageEach { filter, .. }
        | EffectAst::ReturnAllToHand { filter }
        | EffectAst::ReturnAllToHandOfChosenColor { filter }
        | EffectAst::ReturnAllToBattlefield { filter, .. }
        | EffectAst::ExchangeControl { filter, .. }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::ScalePowerToughnessAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::RemoveAbilitiesAll { filter, .. }
        | EffectAst::GrantAbilitiesChoiceAll { filter, .. }
        | EffectAst::GrantBySpec {
            spec: crate::grant::GrantSpec { filter, .. },
            ..
        }
        | EffectAst::SearchLibrary { filter, .. }
        | EffectAst::DestroyAllAttachedTo { filter, .. } => Some(filter),
        EffectAst::Enchant {
            filter: crate::object::AuraAttachmentFilter::Object(filter),
        } => Some(filter),
        _ => None,
    }
}

pub(crate) fn effect_references_tag(effect: &EffectAst, tag: &str) -> bool {
    assert_effect_ast_variant_coverage(effect);
    if direct_effect_targets_reference_tag(effect, tag) {
        return true;
    }
    if let Some(filter) = effect_tagged_filter(effect) {
        return filter_references_tag(filter, tag);
    }

    match effect {
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        } => {
            matches!(predicate, PredicateAst::TaggedMatches(t, _) if t.as_str() == tag)
                || matches!(predicate, PredicateAst::PlayerTaggedObjectMatches { tag: t, .. } if t.as_str() == tag)
                || effects_reference_tag(if_true, tag)
                || effects_reference_tag(if_false, tag)
        }
        EffectAst::DrawForEachTaggedMatching {
            tag: effect_tag,
            filter,
            ..
        } => effect_tag.as_str() == tag || filter_references_tag(filter, tag),
        EffectAst::RetargetStackObject { .. } => false,
        EffectAst::PutIntoHand { object, .. } => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::CreateTokenCopy { object, .. } => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::CreateTokenWithMods { count, .. } => value_references_tag(count, tag),
        EffectAst::ForEachObject { filter, effects } => {
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == tag)
                || effects_reference_tag(effects, tag)
        }
        EffectAst::Cant { restriction, .. } => restriction_references_tag(restriction, tag),
        _ => {
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested
                        .iter()
                        .any(|nested_effect| effect_references_tag(nested_effect, tag));
                }
            });
            references
        }
    }
}

pub(crate) fn value_references_tag(value: &Value, tag: &str) -> bool {
    match value {
        Value::Add(left, right) => {
            value_references_tag(left, tag) || value_references_tag(right, tag)
        }
        Value::Scaled(value, _) => value_references_tag(value, tag),
        Value::HalfRoundedDown(value) => value_references_tag(value, tag),
        Value::Count(filter) | Value::CountScaled(filter, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::PowerOf(spec) | Value::ToughnessOf(spec) => choose_spec_references_tag(spec, tag),
        Value::ManaValueOf(spec) => choose_spec_references_tag(spec, tag),
        Value::CountersOn(spec, _) => choose_spec_references_tag(spec, tag),
        Value::DamageDealtThisTurnByTaggedSpellCast(t) => t.as_str() == tag,
        _ => false,
    }
}

pub(crate) fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(t) => t.as_str() == tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_tag(inner, tag)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        _ => false,
    }
}

pub(crate) fn choose_spec_references_exiled_tag(spec: &ChooseSpec) -> bool {
    fn is_exiled_tag(tag: &TagKey) -> bool {
        let tag = tag.as_str();
        str_starts_with(tag, "exiled_") || str_starts_with(tag, "__sentence_helper_exiled")
    }

    match spec {
        ChooseSpec::Tagged(tag) => is_exiled_tag(tag),
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_exiled_tag(inner)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                    && is_exiled_tag(&constraint.tag)
            })
        }
        _ => false,
    }
}

pub(crate) fn object_ref_references_tag(reference: &ObjectRef, tag: &str) -> bool {
    matches!(reference, ObjectRef::Tagged(found) if found.as_str() == tag)
}

pub(crate) fn player_filter_references_tag(filter: &PlayerFilter, tag: &str) -> bool {
    match filter {
        PlayerFilter::Target(inner) => player_filter_references_tag(inner, tag),
        PlayerFilter::ControllerOf(reference)
        | PlayerFilter::OwnerOf(reference)
        | PlayerFilter::AliasedOwnerOf(reference)
        | PlayerFilter::AliasedControllerOf(reference) => object_ref_references_tag(reference, tag),
        _ => false,
    }
}

pub(crate) fn target_references_tag(target: &TargetAst, tag: &str) -> bool {
    match target {
        TargetAst::Tagged(found, _) => found.as_str() == tag,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            player_filter_references_tag(filter, tag)
        }
        TargetAst::WithCount(inner, _) => target_references_tag(inner, tag),
        TargetAst::AttackedPlayerOrPlaneswalker(_) => false,
        TargetAst::Source(_)
        | TargetAst::AnyTarget(_)
        | TargetAst::AnyOtherTarget(_)
        | TargetAst::Spell(_) => false,
    }
}

pub(crate) fn effects_reference_it_tag(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_it_tag)
}

pub(crate) fn effects_reference_its_controller(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_its_controller)
}

pub(crate) fn value_references_event_derived_amount(value: &Value) -> bool {
    matches!(
        value,
        Value::EventValue(EventValueSpec::Amount)
            | Value::EventValue(EventValueSpec::LifeAmount)
            | Value::EventValueOffset(EventValueSpec::Amount, _)
            | Value::EventValueOffset(EventValueSpec::LifeAmount, _)
    )
}

pub(crate) fn effect_references_event_derived_amount(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::PoisonCounters { count: amount, .. }
        | EffectAst::EnergyCounters { count: amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Fateseal { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::LookAtTopCards { count: amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::Investigate { count: amount }
        | EffectAst::CreateTokenCopy { count: amount, .. }
        | EffectAst::CreateTokenCopyFromSource { count: amount, .. }
        | EffectAst::CreateTokenWithMods { count: amount, .. }
        | EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            value_references_event_derived_amount(amount)
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => value_references_event_derived_amount(amount),
        EffectAst::PutCounters { count, .. } | EffectAst::PutCountersAll { count, .. } => {
            value_references_event_derived_amount(count)
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            value_references_event_derived_amount(put_count)
                || value_references_event_derived_amount(remove_count)
        }
        EffectAst::RemoveCountersAll { amount, .. } => {
            value_references_event_derived_amount(amount)
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            life.as_ref()
                .is_some_and(value_references_event_derived_amount)
                || additional_generic
                    .as_ref()
                    .is_some_and(value_references_event_derived_amount)
        }
        EffectAst::Discard { count, .. } => value_references_event_derived_amount(count),
        EffectAst::Pump {
            power, toughness, ..
        }
        | EffectAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | EffectAst::BecomeBasePtCreature {
            power, toughness, ..
        }
        | EffectAst::PumpAll {
            power, toughness, ..
        } => {
            value_references_event_derived_amount(power)
                || value_references_event_derived_amount(toughness)
        }
        EffectAst::ScalePowerToughnessAll { .. } => false,
        EffectAst::SetBasePower { power, .. } => value_references_event_derived_amount(power),
        EffectAst::PumpForEach { count, .. } => value_references_event_derived_amount(count),
        EffectAst::ConsultTopOfLibrary { stop_rule, .. } => {
            matches!(
                stop_rule,
                crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value)
                    if value_references_event_derived_amount(value)
            )
        }
        _ => {
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested.iter().any(effect_references_event_derived_amount);
                }
            });
            references
        }
    }
}

pub(crate) fn effect_references_its_controller(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::DrawForEachTaggedMatching { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::GainControl { player, .. }
        | EffectAst::LoseGame { player }
        | EffectAst::AddMana { player, .. }
        | EffectAst::AddManaScaled { player, .. }
        | EffectAst::AddManaAnyColor { player, .. }
        | EffectAst::AddManaAnyOneColor { player, .. }
        | EffectAst::AddManaChosenColor { player, .. }
        | EffectAst::AddManaFromLandCouldProduce { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Fateseal { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::PlayFromGraveyardUntilEot { player }
        | EffectAst::AdditionalLandPlays { player, .. }
        | EffectAst::ReduceNextSpellCostThisTurn { player, .. }
        | EffectAst::GrantNextSpellAbilityThisTurn { player, .. }
        | EffectAst::GrantPlayTaggedUntilEndOfTurn { player, .. }
        | EffectAst::GrantBySpec { player, .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            player,
            ..
        }
        | EffectAst::ExileInsteadOfGraveyardThisTurn { player }
        | EffectAst::ExtraTurnAfterTurn { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard { player, .. }
        | EffectAst::LookAtTopCards { player, .. }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::CopySpell { player, .. }
        | EffectAst::RetargetStackObject {
            chooser: player, ..
        }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::DoubleManaPool { player }
        | EffectAst::SetLifeTotal { player, .. }
        | EffectAst::SkipTurn { player }
        | EffectAst::SkipCombatPhases { player }
        | EffectAst::SkipNextCombatPhaseThisTurn { player }
        | EffectAst::SkipDrawStep { player }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. }
        | EffectAst::SearchLibrary { player, .. }
        | EffectAst::ShuffleGraveyardIntoLibrary { player }
        | EffectAst::ShuffleLibrary { player }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::SacrificeAll { player, .. }
        | EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsAcrossZones { player, .. }
        | EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player,
            ..
        } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::ExchangeLifeTotals { player1, player2 } => {
            matches!(player1, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || matches!(player2, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::MayByPlayer { player, effects } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
        }
        EffectAst::UnlessPays {
            effects, player, ..
        } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            player,
            ..
        } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
                || effects_reference_its_controller(alternative)
        }
        _ => {
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested.iter().any(effect_references_its_controller);
                }
            });
            references
        }
    }
}

pub(crate) fn effect_references_it_tag(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    if direct_effect_targets_reference_tag(effect, IT_TAG) {
        return true;
    }

    match effect {
        EffectAst::DealDamage { amount, .. } => value_references_tag(amount, IT_TAG),
        EffectAst::DealDamageEach { amount, filter } => {
            value_references_tag(amount, IT_TAG) || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::Draw { count, .. } => value_references_tag(count, IT_TAG),
        EffectAst::DrawForEachTaggedMatching { tag, filter, .. } => {
            tag.as_str() == IT_TAG || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::LoseLife { amount, .. } | EffectAst::GainLife { amount, .. } => {
            value_references_tag(amount, IT_TAG)
        }
        EffectAst::PreventDamage { amount, .. } => value_references_tag(amount, IT_TAG),
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => value_references_tag(amount, IT_TAG),
        EffectAst::PreventDamageEach { amount, filter, .. } => {
            value_references_tag(amount, IT_TAG) || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::PutCounters { count, .. } => value_references_tag(count, IT_TAG),
        EffectAst::PutCountersAll { count, filter, .. } => {
            value_references_tag(count, IT_TAG) || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            life.as_ref()
                .is_some_and(|value| value_references_tag(value, IT_TAG))
                || additional_generic
                    .as_ref()
                    .is_some_and(|value| value_references_tag(value, IT_TAG))
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        } => {
            matches!(
                predicate,
                PredicateAst::ItIsLandCard
                    | PredicateAst::ItIsSoulbondPaired
                    | PredicateAst::ItMatches(_)
            ) || matches!(predicate, PredicateAst::TaggedMatches(t, _) if t.as_str() == IT_TAG)
                || matches!(
                    predicate,
                    PredicateAst::PlayerTaggedObjectMatches { tag: t, .. } if t.as_str() == IT_TAG
                )
                || effects_reference_it_tag(if_true)
                || effects_reference_it_tag(if_false)
        }
        EffectAst::PutIntoHand { object, .. } => {
            matches!(object, ObjectRefAst::Tagged(tag) if tag.as_str() == IT_TAG)
        }
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard { .. }
        | EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary { .. } => true,
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            battlefield_filter: _,
            ..
        } => true,
        EffectAst::PutRestOnBottomOfLibrary => true,
        EffectAst::RetargetStackObject { .. } => false,
        EffectAst::CreateTokenCopy { object, .. } => {
            matches!(object, ObjectRefAst::Tagged(tag) if tag.as_str() == IT_TAG)
        }
        EffectAst::GrantPlayTaggedUntilEndOfTurn { tag, .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            tag, ..
        }
        | EffectAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }
        | EffectAst::CastTagged { tag, .. }
        | EffectAst::ReorderTopOfLibrary { tag } => tag.as_str() == IT_TAG,
        EffectAst::CreateTokenWithMods { count, .. } => value_references_tag(count, IT_TAG),
        EffectAst::ForEachTagged { tag, effects } => {
            tag.as_str() == IT_TAG || effects_reference_it_tag(effects)
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { .. } => true,
        EffectAst::ForEachObject { filter, effects } => {
            filter_references_tag(filter, IT_TAG) || effects_reference_it_tag(effects)
        }
        EffectAst::Cant { restriction, .. } => restriction_references_tag(restriction, IT_TAG),
        _ => {
            if let Some(filter) = effect_tagged_filter(effect) {
                return filter_references_tag(filter, IT_TAG);
            }
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested.iter().any(effect_references_it_tag);
                }
            });
            references
        }
    }
}

pub(crate) fn restriction_references_tag(
    restriction: &crate::effect::Restriction,
    tag: &str,
) -> bool {
    use crate::effect::Restriction;

    let maybe_filter = match restriction {
        Restriction::Attack(filter)
        | Restriction::Block(filter)
        | Restriction::Untap(filter)
        | Restriction::BeBlocked(filter)
        | Restriction::BeDestroyed(filter)
        | Restriction::BeRegenerated(filter)
        | Restriction::BeSacrificed(filter)
        | Restriction::HaveCountersPlaced(filter)
        | Restriction::BeTargeted(filter)
        | Restriction::BeCountered(filter)
        | Restriction::Transform(filter)
        | Restriction::AttackOrBlock(filter)
        | Restriction::ActivateAbilitiesOf(filter)
        | Restriction::ActivateTapAbilitiesOf(filter)
        | Restriction::ActivateNonManaAbilitiesOf(filter) => Some(filter),
        _ => None,
    };
    if let Some(filter) = maybe_filter {
        return filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
    }

    if let Restriction::BlockSpecificAttacker { blockers, attacker } = restriction {
        let blockers_reference = blockers
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        let attacker_reference = attacker
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        return blockers_reference || attacker_reference;
    }
    if let Restriction::MustBlockSpecificAttacker { blockers, attacker } = restriction {
        let blockers_reference = blockers
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        let attacker_reference = attacker
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        return blockers_reference || attacker_reference;
    }

    false
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
            force_auto_tag_object_targets: ctx.force_auto_tag_object_targets,
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

    while idx < annotated.effects.len() {
        let current = &annotated.effects[idx];
        apply_local_reference_env(ctx, &current.in_env);
        ctx.auto_tag_object_targets =
            ctx.force_auto_tag_object_targets || current.auto_tag_object_targets;

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

pub(crate) fn collect_tag_spans_from_effects_with_context(
    effects: &[EffectAst],
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    for effect in effects {
        collect_tag_spans_from_effect(effect, annotations, ctx);
    }
}

fn collect_direct_effect_target_spans(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) -> bool {
    let mut collected = false;
    with_direct_effect_targets(effect, |target| {
        collect_tag_spans_from_target(target, annotations, ctx);
        collected = true;
    });
    collected
}

pub(crate) fn collect_tag_spans_from_effect(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    assert_effect_ast_variant_coverage(effect);
    if collect_direct_effect_target_spans(effect, annotations, ctx) {
        return;
    }

    match effect {
        EffectAst::RemoveCountersAll { .. } => {}
        _ => for_each_nested_effects(effect, true, |nested| {
            collect_tag_spans_from_effects_with_context(nested, annotations, ctx);
        }),
    }
}

pub(crate) fn collect_tag_spans_from_target(
    target: &TargetAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    if let TargetAst::WithCount(inner, _) = target {
        collect_tag_spans_from_target(inner, annotations, ctx);
        return;
    }
    if let TargetAst::Tagged(tag, Some(span)) = target {
        let mapped = map_span_to_original(*span, &ctx.normalized, &ctx.original, &ctx.char_map);
        annotations.record_tag_span(tag, mapped);
    }
    if let TargetAst::Object(filter, _, Some(it_span)) = target
        && filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        let mapped = map_span_to_original(*it_span, &ctx.normalized, &ctx.original, &ctx.char_map);
        annotations.record_tag_span(&TagKey::from(IT_TAG), mapped);
    }
}

fn tag_last_discard_in_effects(effects: &mut [EffectAst], tag: &TagKey) -> bool {
    for effect in effects.iter_mut().rev() {
        if let EffectAst::Discard {
            tag: discard_tag, ..
        } = effect
        {
            *discard_tag = Some(tag.clone());
            return true;
        }
    }
    false
}

fn bind_explicit_tag_to_player_tagged_predicate(
    predicate: &PredicateAst,
    tag: &TagKey,
) -> PredicateAst {
    let mut bound = predicate.clone();
    if let PredicateAst::PlayerTaggedObjectMatches {
        tag: predicate_tag, ..
    } = &mut bound
        && predicate_tag.as_str() == IT_TAG
    {
        *predicate_tag = tag.clone();
    }
    bound
}

pub(crate) fn compile_if_do_with_opponent_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDoesNot {
        effects: second_effects,
        predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachOpponent {
        effects: opponent_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
            let mut tagged_opponent_effects = opponent_effects.clone();
            if !tag_last_discard_in_effects(&mut tagged_opponent_effects, &explicit_tag) {
                return Err(CardTextError::ParseError(
                    "missing discard antecedent for tagged opponent follow-up".to_string(),
                ));
            }
            let first_ast = EffectAst::ForEachOpponent {
                effects: tagged_opponent_effects,
            };
            let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: bind_explicit_tag_to_player_tagged_predicate(
                        predicate,
                        &explicit_tag,
                    ),
                    if_true: Vec::new(),
                    if_false: second_effects.clone(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let mut merged_opponent_effects = opponent_effects.clone();
        merged_opponent_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }
    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
            let mut tagged_player_effects = player_effects.clone();
            if !tag_last_discard_in_effects(&mut tagged_player_effects, &explicit_tag) {
                return Err(CardTextError::ParseError(
                    "missing discard antecedent for tagged player follow-up".to_string(),
                ));
            }
            let first_ast = EffectAst::ForEachPlayer {
                effects: tagged_player_effects,
            };
            let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: bind_explicit_tag_to_player_tagged_predicate(
                        predicate,
                        &explicit_tag,
                    ),
                    if_true: Vec::new(),
                    if_false: second_effects.clone(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let first_ast = EffectAst::ForEachPlayer {
            effects: player_effects.clone(),
        };
        let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
        let id = if let Some(last) = first_effects.pop() {
            let id = ctx.next_effect_id();
            first_effects.push(Effect::with_id(id.0, last));
            id
        } else {
            return Err(CardTextError::ParseError(
                "missing per-player antecedent effect for if-you-don't follow-up".to_string(),
            ));
        };

        let (inner_effects, inner_choices) =
            compile_effects_in_iterated_player_context(second_effects, ctx, None)?;
        for choice in inner_choices {
            push_choice(&mut choices, choice);
        }
        let conditional = Effect::if_then(id, EffectPredicate::DidNotHappen, inner_effects);
        first_effects.push(Effect::for_each_opponent(vec![conditional]));
        return Ok(Some((first_effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    if let Some(predicate) = predicate {
        let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
        let mut tagged_first_effects = first_effects.clone();
        let Some(EffectAst::ForEachOpponent {
            effects: tagged_opponent_effects,
        }) = tagged_first_effects.first_mut()
        else {
            return Ok(None);
        };
        if !tag_last_discard_in_effects(tagged_opponent_effects, &explicit_tag) {
            return Err(CardTextError::ParseError(
                "missing discard antecedent for tagged opponent follow-up".to_string(),
            ));
        }
        let tagged_first = if let Some(condition) = condition {
            EffectAst::ResolvedIfResult {
                condition,
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        } else {
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        };
        let (mut first_compiled, mut choices) = compile_effect(&tagged_first, ctx)?;
        let followup = EffectAst::ForEachOpponent {
            effects: vec![EffectAst::Conditional {
                predicate: bind_explicit_tag_to_player_tagged_predicate(predicate, &explicit_tag),
                if_true: Vec::new(),
                if_false: second_effects.clone(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    let Some(EffectAst::ForEachOpponent {
        effects: opponent_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_opponent_effects = opponent_effects.clone();
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachOpponent {
        effects: merged_opponent_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

pub(crate) fn compile_if_do_with_player_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachPlayerDoesNot {
        effects: second_effects,
        predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
            let mut tagged_player_effects = player_effects.clone();
            if !tag_last_discard_in_effects(&mut tagged_player_effects, &explicit_tag) {
                return Err(CardTextError::ParseError(
                    "missing discard antecedent for tagged player follow-up".to_string(),
                ));
            }
            let first_ast = EffectAst::ForEachPlayer {
                effects: tagged_player_effects,
            };
            let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
            let followup = EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: bind_explicit_tag_to_player_tagged_predicate(
                        predicate,
                        &explicit_tag,
                    ),
                    if_true: Vec::new(),
                    if_false: second_effects.clone(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let mut merged_player_effects = player_effects.clone();
        merged_player_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    if let Some(predicate) = predicate {
        let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
        let mut tagged_first_effects = first_effects.clone();
        let Some(EffectAst::ForEachPlayer {
            effects: tagged_player_effects,
        }) = tagged_first_effects.first_mut()
        else {
            return Ok(None);
        };
        if !tag_last_discard_in_effects(tagged_player_effects, &explicit_tag) {
            return Err(CardTextError::ParseError(
                "missing discard antecedent for tagged player follow-up".to_string(),
            ));
        }
        let tagged_first = if let Some(condition) = condition {
            EffectAst::ResolvedIfResult {
                condition,
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        } else {
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        };
        let (mut first_compiled, mut choices) = compile_effect(&tagged_first, ctx)?;
        let followup = EffectAst::ForEachPlayer {
            effects: vec![EffectAst::Conditional {
                predicate: bind_explicit_tag_to_player_tagged_predicate(predicate, &explicit_tag),
                if_true: Vec::new(),
                if_false: second_effects.clone(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    let Some(EffectAst::ForEachPlayer {
        effects: player_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_player_effects = player_effects.clone();
    merged_player_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachPlayer {
        effects: merged_player_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

pub(crate) fn compile_if_do_with_opponent_did(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDid {
        effects: second_effects,
        predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachOpponent {
        effects: opponent_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: predicate.clone(),
                    if_true: second_effects.clone(),
                    if_false: Vec::new(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let mut merged_opponent_effects = opponent_effects.clone();
        merged_opponent_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }
    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: predicate.clone(),
                    if_true: second_effects.clone(),
                    if_false: Vec::new(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let first_ast = EffectAst::ForEachPlayer {
            effects: player_effects.clone(),
        };
        let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
        let id = if let Some(last) = first_effects.pop() {
            let id = ctx.next_effect_id();
            first_effects.push(Effect::with_id(id.0, last));
            id
        } else {
            return Err(CardTextError::ParseError(
                "missing per-player antecedent effect for if-you-do follow-up".to_string(),
            ));
        };

        let (inner_effects, inner_choices) =
            compile_effects_in_iterated_player_context(second_effects, ctx, None)?;
        for choice in inner_choices {
            push_choice(&mut choices, choice);
        }
        let conditional = Effect::if_then(id, EffectPredicate::Happened, inner_effects);
        first_effects.push(Effect::for_each_opponent(vec![conditional]));
        return Ok(Some((first_effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    if let Some(predicate) = predicate {
        let (mut first_compiled, mut choices) = compile_effect(first, ctx)?;
        let followup = EffectAst::ForEachOpponent {
            effects: vec![EffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: second_effects.clone(),
                if_false: Vec::new(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    let Some(EffectAst::ForEachOpponent {
        effects: opponent_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_opponent_effects = opponent_effects.clone();
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachOpponent {
        effects: merged_opponent_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

pub(crate) fn compile_if_do_with_player_did(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachPlayerDid {
        effects: second_effects,
        predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
            let followup = EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: predicate.clone(),
                    if_true: second_effects.clone(),
                    if_false: Vec::new(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let mut merged_player_effects = player_effects.clone();
        merged_player_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    if let Some(predicate) = predicate {
        let (mut first_compiled, mut choices) = compile_effect(first, ctx)?;
        let followup = EffectAst::ForEachPlayer {
            effects: vec![EffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: second_effects.clone(),
                if_false: Vec::new(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    let Some(EffectAst::ForEachPlayer {
        effects: player_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_player_effects = player_effects.clone();
    merged_player_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachPlayer {
        effects: merged_player_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

#[derive(Debug, Clone)]
pub(crate) struct EffectLoweringContextState {
    frame: LoweringFrame,
}

impl EffectLoweringContextState {
    fn capture(ctx: &EffectLoweringContext) -> Self {
        Self {
            frame: ctx.lowering_frame(),
        }
    }

    fn restore(self, ctx: &mut EffectLoweringContext) {
        ctx.apply_lowering_frame(self.frame);
    }
}

pub(crate) fn with_preserved_lowering_context<T, Configure, Run>(
    ctx: &mut EffectLoweringContext,
    configure: Configure,
    run: Run,
) -> Result<T, CardTextError>
where
    Configure: FnOnce(&mut EffectLoweringContext),
    Run: FnOnce(&mut EffectLoweringContext) -> Result<T, CardTextError>,
{
    let saved = EffectLoweringContextState::capture(ctx);
    configure(ctx);
    let result = run(ctx);
    saved.restore(ctx);
    result
}

pub(crate) fn compile_effects_preserving_last_effect(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut id_gen = ctx.id_gen_context();
    let (compiled, choices, mut frame_out) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, saved_frame.clone())?;
    frame_out.last_effect_id = saved_frame.last_effect_id;
    ctx.apply_id_gen_context(id_gen);
    ctx.apply_lowering_frame(frame_out);
    Ok((compiled, choices))
}

fn effect_predicate_from_if_result(predicate: IfResultPredicate) -> EffectPredicate {
    match predicate {
        IfResultPredicate::Did => EffectPredicate::Happened,
        IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
        IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
        IfResultPredicate::WasDeclined => EffectPredicate::WasDeclined,
    }
}

fn compile_repeat_process_body(
    effects: &[EffectAst],
    continue_effect_index: usize,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>, EffectId), CardTextError> {
    let mut compiled = Vec::new();
    let mut choices = Vec::new();
    let mut condition: Option<EffectId> = None;

    for (idx, effect) in effects.iter().enumerate() {
        let (mut effect_list, effect_choices) = compile_effect(effect, ctx)?;
        if idx == continue_effect_index {
            if effect_list.is_empty() {
                return Err(CardTextError::ParseError(
                    "repeat process condition compiled to no effects".to_string(),
                ));
            }
            let id = ctx.next_effect_id();
            assign_effect_result_id(
                &mut effect_list,
                id,
                "repeat process condition is missing a final effect",
            )?;
            ctx.last_effect_id = Some(id);
            condition = Some(id);
        }
        compiled.extend(effect_list);
        for choice in effect_choices {
            push_choice(&mut choices, choice);
        }
    }

    let condition = condition.ok_or_else(|| {
        CardTextError::ParseError("repeat process is missing a condition effect".to_string())
    })?;
    Ok((compiled, choices, condition))
}

pub(crate) fn compile_effects_in_iterated_player_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
    tagged_object: Option<String>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut iterated_frame = saved_frame.clone();
    iterated_frame.iterated_player = true;
    iterated_frame.last_effect_id = None;
    // Seed per-player loops with the loop slot itself so "that player" resolves
    // to the current iterated player unless an inner effect establishes a more
    // specific player reference.
    iterated_frame.last_player_filter = Some(PlayerFilter::IteratedPlayer);
    if tagged_object.is_some() {
        // Inside per-object iteration, implicit "it" should bind to the current
        // iterated object, not to the outer tag collection being traversed.
        iterated_frame.last_object_tag = Some(IT_TAG.to_string());
    }

    let mut id_gen = ctx.id_gen_context();
    let (compiled, choices, frame_out) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, iterated_frame)?;
    ctx.apply_id_gen_context(id_gen);
    let produced_last_tag = if tagged_object.is_none() {
        frame_out.last_object_tag.clone()
    } else {
        None
    };
    ctx.apply_lowering_frame(saved_frame);
    if let Some(tag) = produced_last_tag {
        ctx.last_object_tag = Some(tag);
    }
    Ok((compiled, choices))
}

pub(crate) fn force_implicit_vote_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::CreateTokenWithMods { player, .. }
            | EffectAst::CreateTokenCopy { player, .. }
            | EffectAst::CreateTokenCopyFromSource { player, .. } => {
                if matches!(player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                force_implicit_vote_token_controller_you(nested);
            }),
        }
    }
}

fn is_vote_related_predicate(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::VoteOptionGetsMoreVotes { .. }
            | PredicateAst::VoteOptionGetsMoreVotesOrTied { .. }
            | PredicateAst::NoVoteObjectsMatched { .. }
    )
}

fn compiled_vote_option_uses_iterated_player(effects: &[Effect], choices: &[ChooseSpec]) -> bool {
    str_contains(format!("{effects:?}{choices:?}").as_str(), "IteratedPlayer")
}

pub(crate) fn compile_vote_sequence(
    effects: &[AnnotatedEffect],
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>, usize)>, CardTextError> {
    let Some(first) = effects.first() else {
        return Ok(None);
    };
    let vote_start = match &first.effect {
        EffectAst::VoteStart { options } => Some((Some(options.clone()), None)),
        EffectAst::VoteStartObjects { filter, count } => {
            Some((None, Some((filter.clone(), *count))))
        }
        _ => None,
    };
    let Some((named_options, object_vote)) = vote_start else {
        return Ok(None);
    };

    let mut extra_mandatory: u32 = 0;
    let mut extra_optional: u32 = 0;
    let consumed = effects
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(idx, annotated)| match &annotated.effect {
            EffectAst::VoteOption { .. } | EffectAst::VoteExtra { .. } => Some(idx + 1),
            EffectAst::Conditional { predicate, .. } if is_vote_related_predicate(predicate) => {
                Some(idx + 1)
            }
            _ => None,
        })
        .last()
        .unwrap_or(1);

    for annotated in effects.iter().take(consumed).skip(1) {
        if let EffectAst::VoteExtra { count, optional } = &annotated.effect {
            if *optional {
                extra_optional = extra_optional.saturating_add(*count);
            } else {
                extra_mandatory = extra_mandatory.saturating_add(*count);
            }
        }
    }

    if let Some((filter, count)) = object_vote {
        let resolved = resolve_it_tag(&filter, &current_reference_env(ctx))?;
        let effect = if extra_optional > 0 {
            Effect::vote_objects_with_optional_extra(
                resolved,
                count,
                extra_mandatory,
                extra_optional,
            )
        } else {
            Effect::vote_objects(resolved, count, extra_mandatory)
        };
        let mut compiled = vec![effect];
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            match &annotated.effect {
                EffectAst::VoteExtra { .. } => {}
                _ => {
                    let (followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
                    compiled.extend(followups);
                    for choice in followup_choices {
                        push_choice(&mut choices, choice);
                    }
                }
            }
            apply_local_reference_env(ctx, &annotated.out_env);
        }
        return Ok(Some((compiled, choices, consumed)));
    }

    let mut vote_options = named_options
        .as_ref()
        .expect("named vote start should exist")
        .iter()
        .map(|option| VoteOption::new(option.clone(), Vec::new()))
        .collect::<Vec<_>>();
    let mut choices = Vec::new();
    let mut post_vote_effects = Vec::new();
    for annotated in effects.iter().take(consumed).skip(1) {
        apply_local_reference_env(ctx, &annotated.in_env);
        ctx.auto_tag_object_targets =
            ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
        match &annotated.effect {
            EffectAst::VoteExtra { .. } => {}
            EffectAst::VoteOption { option, effects } => {
                let mut option_effects_ast = effects.clone();
                force_implicit_vote_token_controller_you(&mut option_effects_ast);
                let (repeat_effects, repeat_choices) = compile_effects(&option_effects_ast, ctx)?;
                if compiled_vote_option_uses_iterated_player(&repeat_effects, &repeat_choices) {
                    let (per_vote_effects, per_vote_choices) =
                        compile_effects_in_iterated_player_context(&option_effects_ast, ctx, None)?;
                    if let Some(vote_option_idx) =
                        find_index(vote_options.as_slice(), |vote_option| {
                            vote_option.name.eq_ignore_ascii_case(option)
                        })
                    {
                        vote_options[vote_option_idx]
                            .effects_per_vote
                            .extend(per_vote_effects);
                    }
                    for choice in per_vote_choices {
                        push_choice(&mut choices, choice);
                    }
                } else {
                    post_vote_effects.push(Effect::repeat_effects(
                        Value::VoteCount(option.clone()),
                        repeat_effects,
                    ));
                    for choice in repeat_choices {
                        push_choice(&mut choices, choice);
                    }
                }
            }
            _ => {
                let (followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
                post_vote_effects.extend(followups);
                for choice in followup_choices {
                    push_choice(&mut choices, choice);
                }
            }
        }
        apply_local_reference_env(ctx, &annotated.out_env);
    }

    let effect = if extra_optional > 0 {
        Effect::vote_with_optional_extra(vote_options, extra_mandatory, extra_optional)
    } else {
        Effect::vote(vote_options, extra_mandatory)
    };
    let mut compiled = vec![effect];
    compiled.extend(post_vote_effects);

    Ok(Some((compiled, choices, consumed)))
}

pub(crate) fn choose_spec_for_targeted_player_filter(filter: &PlayerFilter) -> Option<ChooseSpec> {
    if let PlayerFilter::Target(inner) = filter {
        return Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())));
    }
    None
}

pub(crate) fn collect_targeted_player_specs_from_filter(
    filter: &ObjectFilter,
    specs: &mut Vec<ChooseSpec>,
) {
    if let Some(controller) = &filter.controller
        && let Some(spec) = choose_spec_for_targeted_player_filter(controller)
    {
        push_choice(specs, spec);
    }

    if let Some(owner) = &filter.owner
        && let Some(spec) = choose_spec_for_targeted_player_filter(owner)
    {
        push_choice(specs, spec);
    }

    if let Some(targets_player) = &filter.targets_player
        && let Some(spec) = choose_spec_for_targeted_player_filter(targets_player)
    {
        push_choice(specs, spec);
    }

    if let Some(targets_object) = &filter.targets_object {
        collect_targeted_player_specs_from_filter(targets_object, specs);
    }
}

pub(crate) fn target_context_prelude_for_filter(
    filter: &ObjectFilter,
) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let mut choices = Vec::new();
    collect_targeted_player_specs_from_filter(filter, &mut choices);
    let effects = choices
        .iter()
        .cloned()
        .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
        .collect();
    (effects, choices)
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

pub(crate) fn compile_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    // Keep dynamic stack growth for deeply nested recursive lowering paths.
    stacker::maybe_grow(1024 * 1024, 2 * 1024 * 1024, || {
        compile_effect_inner(effect, ctx)
    })
}

fn compile_effect_inner(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    if matches!(
        effect,
        EffectAst::RepeatThisProcess | EffectAst::RepeatThisProcessOnce
    ) {
        return Err(CardTextError::ParseError(
            "unsupported repeat this process effect tail".to_string(),
        ));
    }
    if let Some(compiled) = try_compile_effect_via_handlers(effect, ctx)? {
        return Ok(compiled);
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing compile-effect dispatch route for effect variant: {effect:?}"
    )))
}

type EffectCompileOutcome = (Vec<Effect>, Vec<ChooseSpec>);
type EffectCompileHandler = fn(
    &EffectAst,
    &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError>;

#[derive(Clone, Copy)]
struct EffectCompileHandlerDef {
    run: EffectCompileHandler,
}

const EFFECT_COMPILE_HANDLERS: [EffectCompileHandlerDef; 14] = [
    EffectCompileHandlerDef {
        run: try_compile_combat_and_damage_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_board_state_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_player_resource_and_choice_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_timing_and_control_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_flow_and_iteration_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_destroy_and_exile_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_visibility_and_card_selection_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_stack_and_condition_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_attachment_and_setup_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_token_generation_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_continuous_and_modifier_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_search_and_reorder_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_object_zone_and_exchange_effect,
    },
    EffectCompileHandlerDef {
        run: try_compile_player_turn_and_counter_effect,
    },
];

fn try_compile_effect_via_handlers(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    for EffectCompileHandlerDef { run, .. } in EFFECT_COMPILE_HANDLERS {
        if let Some(compiled) = run(effect, ctx)? {
            return Ok(Some(compiled));
        }
    }
    Ok(None)
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

fn try_compile_combat_and_damage_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::DealDamage { amount, target } => {
            let mut resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
                && !ctx.iterated_player
            {
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut resolved_amount,
                    &PlayerFilter::Target(Box::new(filter.clone())),
                );
            }
            let (effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    Effect::deal_damage(resolved_amount.clone(), spec)
                })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if matches!(
                target,
                TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_)
            ) {
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }
            (effects, choices)
        }
        EffectAst::DealDistributedDamage { amount, target } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                Effect::new(crate::effects::DealDistributedDamageEffect::new(
                    resolved_amount.clone(),
                    spec,
                ))
            })?
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            let (source_spec, mut choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            let mut damage_target_spec = if source == target {
                source_spec.clone()
            } else {
                let (target_spec, target_choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                target_spec
            };

            let mut effects = Vec::new();
            let mut damage_source_spec = source_spec.clone();
            let per_target_source_spec = if source == target {
                ChooseSpec::Iterated
            } else {
                source_spec.clone()
            };

            if source_spec.is_target() {
                let source_tag = ctx.next_tag("damage_source");
                effects.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(source_spec.clone()))
                        .tag(source_tag.clone()),
                );
                damage_source_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                if source == target {
                    damage_target_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                }
            }

            if !damage_target_spec.is_target()
                && let ChooseSpec::Object(filter) | ChooseSpec::All(filter) =
                    damage_target_spec.base()
            {
                let mut per_target_damage =
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        per_target_source_spec.clone(),
                        Effect::deal_damage(
                            Value::PowerOf(Box::new(per_target_source_spec.clone())),
                            ChooseSpec::Iterated,
                        ),
                    ));
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("damaged");
                    ctx.last_object_tag = Some(tag.clone());
                    per_target_damage = per_target_damage.tag(tag);
                }
                effects.push(Effect::for_each(filter.clone(), vec![per_target_damage]));
            } else {
                let damage_effect = tag_object_target_effect(
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        damage_source_spec.clone(),
                        Effect::deal_damage(
                            Value::PowerOf(Box::new(damage_source_spec.clone())),
                            damage_target_spec.clone(),
                        ),
                    )),
                    &damage_target_spec,
                    ctx,
                    "damaged",
                );
                effects.push(damage_effect);
            }

            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if matches!(
                target,
                TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_)
            ) {
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }

            (effects, choices)
        }
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            let (spec1, mut choices) =
                resolve_target_spec_with_choices(creature1, &current_reference_env(ctx))?;
            let (spec2, other_choices) =
                resolve_target_spec_with_choices(creature2, &current_reference_env(ctx))?;
            for choice in other_choices {
                push_choice(&mut choices, choice);
            }
            let effect = Effect::fight(spec1.clone(), spec2.clone());
            (vec![effect], choices)
        }
        EffectAst::FightIterated { creature2 } => {
            let (spec2, choices) =
                resolve_target_spec_with_choices(creature2, &current_reference_env(ctx))?;
            let effect = Effect::fight(ChooseSpec::Iterated, spec2);
            (vec![effect], choices)
        }
        EffectAst::Clash { opponent } => match opponent {
            ClashOpponentAst::Opponent => (
                vec![Effect::new(
                    crate::effects::ClashEffect::against_any_opponent(),
                )],
                Vec::new(),
            ),
            ClashOpponentAst::TargetOpponent => {
                let choice = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Opponent));
                (
                    vec![Effect::new(
                        crate::effects::ClashEffect::against_target_opponent(),
                    )],
                    vec![choice],
                )
            }
            ClashOpponentAst::DefendingPlayer => (
                vec![Effect::new(
                    crate::effects::ClashEffect::against_defending_player(),
                )],
                Vec::new(),
            ),
        },
        EffectAst::DealDamageEach { amount, filter } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let tag = ctx.next_tag("damaged");
            ctx.last_object_tag = Some(tag.clone());
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::deal_damage(resolved_amount, ChooseSpec::Iterated).tag(tag)],
            );
            (vec![effect], Vec::new())
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_board_state_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    use crate::effect::EffectMode;

    let compiled = match effect {
        EffectAst::PutCounters {
            counter_type,
            count,
            target,
            target_count,
            distributed,
        } => {
            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }
            let mut put_counters =
                crate::effects::PutCountersEffect::new(*counter_type, count.clone(), spec.clone());
            if let Some(target_count) = target_count {
                put_counters = put_counters.with_target_count(*target_count);
            }
            if *distributed {
                put_counters = put_counters.with_distributed(true);
            }
            let effect =
                tag_object_target_effect(Effect::new(put_counters), &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            (vec![effect], choices)
        }
        EffectAst::PutOrRemoveCounters {
            put_counter_type,
            put_count,
            remove_counter_type,
            remove_count,
            put_mode_text,
            remove_mode_text,
            target,
            target_count,
        } => {
            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }

            let put_effect =
                Effect::put_counters(*put_counter_type, put_count.clone(), spec.clone());
            let remove_effect =
                Effect::remove_counters(*remove_counter_type, remove_count.clone(), spec.clone());

            let effect = Effect::choose_one(vec![
                EffectMode {
                    description: put_mode_text.clone(),
                    effects: vec![put_effect],
                },
                EffectMode {
                    description: remove_mode_text.clone(),
                    effects: vec![remove_effect],
                },
            ]);

            let effect = tag_object_target_effect(effect, &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            (vec![effect], choices)
        }
        EffectAst::ForEachCounterKindPutOrRemove { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            (
                vec![Effect::new(
                    crate::effects::ForEachCounterKindPutOrRemoveEffect::new(spec),
                )],
                choices,
            )
        }
        EffectAst::PutCountersAll {
            counter_type,
            count,
            filter,
        } => {
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::put_counters(
                    *counter_type,
                    count.clone(),
                    ChooseSpec::Iterated,
                )],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::RemoveCountersAll {
            amount,
            filter,
            counter_type,
            up_to,
        } => {
            let iterated = ChooseSpec::Iterated;
            let inner = if let Some(counter_type) = counter_type {
                if *up_to {
                    Effect::remove_up_to_counters(*counter_type, amount.clone(), iterated.clone())
                } else {
                    Effect::remove_counters(*counter_type, amount.clone(), iterated.clone())
                }
            } else {
                Effect::remove_up_to_any_counters(amount.clone(), iterated.clone())
            };
            let effect = Effect::for_each(filter.clone(), vec![inner]);
            (vec![effect], Vec::new())
        }
        EffectAst::DoubleCountersOnEach {
            counter_type,
            filter,
        } => {
            let iterated = ChooseSpec::Iterated;
            let count = Value::CountersOn(Box::new(iterated.clone()), Some(*counter_type));
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::put_counters(*counter_type, count, iterated)],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::Proliferate { count } => (vec![Effect::proliferate(count.clone())], Vec::new()),
        EffectAst::Tap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::tap(spec.clone())
            } else {
                Effect::new(crate::effects::TapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "tapped");
            (vec![effect], choices)
        }
        EffectAst::TapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::tap_all(resolved_filter));
            (prelude, choices)
        }
        EffectAst::Untap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::untap(spec.clone())
            } else {
                Effect::new(crate::effects::UntapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "untapped");
            (vec![effect], choices)
        }
        EffectAst::PhaseOut { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::phase_out(spec.clone())
            } else {
                Effect::new(crate::effects::PhaseOutEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "phased_out");
            (vec![effect], choices)
        }
        EffectAst::RemoveFromCombat { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(
                    spec.clone(),
                )),
                &spec,
                ctx,
                "removed_from_combat",
            );
            (vec![effect], choices)
        }
        EffectAst::TapOrUntap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let tap_effect = Effect::tap(spec.clone());
            let untap_effect = Effect::untap(spec.clone());
            let modes = vec![
                EffectMode {
                    description: "Tap".to_string(),
                    effects: vec![tap_effect],
                },
                EffectMode {
                    description: "Untap".to_string(),
                    effects: vec![untap_effect],
                },
            ];
            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "tap_or_untap");
            (vec![effect], choices)
        }
        EffectAst::UntapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::untap_all(resolved_filter));
            (prelude, choices)
        }
        EffectAst::GrantProtectionChoice {
            target,
            allow_colorless,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut modes = Vec::new();
            if *allow_colorless {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Colorless);
                modes.push(EffectMode {
                    description: "Colorless".to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];

            for (name, color) in colors {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Color(
                    ColorSet::from(color),
                ));
                modes.push(EffectMode {
                    description: name.to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "protected");
            (vec![effect], choices)
        }
        EffectAst::Earthbend { counters } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::EarthbendEffect::new(
                    spec.clone(),
                    *counters,
                )),
                &spec,
                ctx,
                "earthbend",
            );
            (vec![effect], vec![spec])
        }
        EffectAst::Behold { subtype, count } => {
            (vec![Effect::behold(*subtype, *count)], Vec::new())
        }
        EffectAst::Explore { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect =
                tag_object_target_effect(Effect::explore(spec.clone()), &spec, ctx, "explored");
            (vec![effect], choices)
        }
        EffectAst::OpenAttraction => (vec![Effect::open_attraction()], Vec::new()),
        EffectAst::ManifestTopCardOfLibrary { player } => (
            vec![Effect::manifest_top_card_of_library(
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?,
            )],
            Vec::new(),
        ),
        EffectAst::ManifestDread => (vec![Effect::manifest_dread()], Vec::new()),
        EffectAst::Populate {
            count,
            enters_tapped,
            enters_attacking,
            has_haste,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
        } => {
            let mut effect = Effect::new(
                crate::effects::PopulateEffect::new(count.clone())
                    .enters_tapped(*enters_tapped)
                    .attacking(*enters_attacking)
                    .haste(*has_haste)
                    .sacrifice_at_next_end_step(*sacrifice_at_next_end_step)
                    .exile_at_next_end_step(*exile_at_next_end_step)
                    .exile_at_end_of_combat(*exile_at_end_of_combat)
                    .sacrifice_at_end_of_combat(*sacrifice_at_end_of_combat),
            );
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], Vec::new())
        }
        EffectAst::Bolster { amount } => (vec![Effect::bolster(*amount)], Vec::new()),
        EffectAst::Support { amount } => (vec![Effect::support(*amount)], Vec::new()),
        EffectAst::Adapt { amount } => (vec![Effect::adapt(*amount)], Vec::new()),
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_player_resource_and_choice_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Draw { count, player } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::draw(count.clone()),
                |filter| Effect::target_draws(count.clone(), filter),
            )?
        }
        EffectAst::DrawForEachTaggedMatching {
            player,
            tag,
            filter,
        } => {
            let resolved_player =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            (
                vec![Effect::new(
                    crate::effects::DrawForEachTaggedMatchingEffect::new(
                        resolved_player,
                        resolved_tag,
                        resolved_filter,
                    ),
                )],
                Vec::new(),
            )
        }
        EffectAst::Counter { target } => {
            compile_tagged_effect_for_target(target, ctx, "countered", Effect::counter)?
        }
        EffectAst::CounterUnlessPays {
            target,
            mana,
            life,
            additional_generic,
        } => {
            let additional_generic = additional_generic
                .as_ref()
                .map(|value| resolve_value_it_tag(value, &current_reference_env(ctx)))
                .transpose()?;
            compile_tagged_effect_for_target(target, ctx, "countered", |spec| {
                Effect::counter_unless_pays_with_life_and_additional(
                    spec,
                    mana.clone(),
                    life.clone(),
                    additional_generic.clone(),
                )
            })?
        }
        EffectAst::LoseLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::lose_life(amount.clone()),
                |filter| Effect::lose_life_player(amount.clone(), filter),
            )?
        }
        EffectAst::GainLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::gain_life(amount.clone()),
                |filter| Effect::gain_life_player(amount.clone(), ChooseSpec::Player(filter)),
            )?
        }
        EffectAst::CreateEmblem { player, text } => {
            let emblem = compile_emblem_description_from_text(text)?;
            let (filter, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = if matches!(filter, PlayerFilter::You) {
                Effect::create_emblem(emblem)
            } else {
                Effect::for_players(filter, vec![Effect::create_emblem(emblem)])
            };
            (vec![effect], choices)
        }
        EffectAst::LoseGame { player } => compile_player_effect(
            *player,
            ctx,
            true,
            Effect::lose_the_game,
            Effect::lose_the_game_player,
        )?,
        EffectAst::WinGame { player } => compile_player_effect(
            *player,
            ctx,
            true,
            Effect::win_the_game,
            Effect::win_the_game_player,
        )?,
        EffectAst::ExileTopOfLibrary {
            count,
            player,
            tags,
            accumulated_tags,
        } => {
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut effect =
                crate::effects::ExileTopOfLibraryEffect::new(resolved_count, player_filter.clone());
            for tag in tags {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                effect = effect.tag_moved(resolved_tag);
            }
            for tag in accumulated_tags {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                effect = effect.append_tagged(resolved_tag);
            }
            if let Some(tag) = tags.first() {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            }
            ctx.last_player_filter = Some(player_filter);
            (vec![Effect::new(effect)], choices)
        }
        EffectAst::RearrangeLookedCardsInLibrary { tag, player, count } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            (
                vec![Effect::rearrange_looked_cards_in_library(
                    resolved_tag,
                    player_filter,
                    *count,
                )],
                choices,
            )
        }
        EffectAst::SearchLibrarySlotsToHand {
            slots,
            player,
            reveal,
            progress_tag,
        } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let refs = current_reference_env(ctx);
            let resolved_slots = slots
                .iter()
                .map(|slot| {
                    let resolved_filter = resolve_it_tag(&slot.filter, &refs)?;
                    Ok(if slot.optional {
                        crate::effects::SearchLibrarySlot::optional(resolved_filter)
                    } else {
                        crate::effects::SearchLibrarySlot::required(resolved_filter)
                    })
                })
                .collect::<Result<Vec<_>, CardTextError>>()?;
            let resolved_tag = resolve_it_tag_key(progress_tag, &refs)?;
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter.clone());
            (
                vec![Effect::search_library_slots_to_hand(
                    resolved_slots,
                    player_filter,
                    *reveal,
                    resolved_tag,
                )],
                choices,
            )
        }
        EffectAst::MayMoveToZone {
            target,
            zone,
            player,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (decider, player_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            for choice in player_choices {
                push_choice(&mut choices, choice);
            }
            (
                vec![Effect::may_move_to_zone(spec, *zone, decider)],
                choices,
            )
        }
        EffectAst::PreventAllCombatDamage { duration } => (
            vec![Effect::prevent_all_combat_damage(duration.clone())],
            Vec::new(),
        ),
        EffectAst::PreventAllCombatDamageFromSource { duration, source } => {
            compile_effect_for_target(source, ctx, |spec| {
                Effect::prevent_all_combat_damage_from(spec, duration.clone())
            })?
        }
        EffectAst::PreventAllCombatDamageToPlayers { duration } => (
            vec![Effect::prevent_all_combat_damage_to_players(
                duration.clone(),
            )],
            Vec::new(),
        ),
        EffectAst::PreventAllCombatDamageToYou { duration } => (
            vec![Effect::prevent_all_combat_damage_to_you(duration.clone())],
            Vec::new(),
        ),
        EffectAst::PreventDamage {
            amount,
            target,
            duration,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_effect_for_target(target, ctx, |spec| {
                Effect::prevent_damage(amount.clone(), spec, duration.clone())
            })?
        }
        EffectAst::PreventAllDamageToTarget { target, duration } => {
            if let TargetAst::Object(filter, explicit_target_span, _) = target
                && explicit_target_span.is_none()
            {
                let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                (
                    vec![Effect::prevent_all_damage_to(
                        resolved_filter,
                        duration.clone(),
                    )],
                    Vec::new(),
                )
            } else {
                compile_effect_for_target(target, ctx, |spec| {
                    Effect::prevent_all_damage_to_target(spec, duration.clone())
                })?
            }
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount,
            target,
            duration,
            counter_type,
        } => {
            let follow_up = vec![Effect::put_counters(
                *counter_type,
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::AnyTarget,
            )];
            match amount {
                Some(amount) => {
                    let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
                    compile_effect_for_target(target, ctx, |spec| {
                        Effect::new(
                            crate::effects::PreventDamageEffect::new(
                                amount.clone(),
                                spec,
                                duration.clone(),
                            )
                            .with_follow_up_effects(follow_up.clone()),
                        )
                    })?
                }
                None => compile_effect_for_target(target, ctx, |spec| {
                    Effect::new(
                        crate::effects::PreventAllDamageToTargetEffect::new(spec, duration.clone())
                            .with_follow_up_effects(follow_up.clone()),
                    )
                })?,
            }
        }
        EffectAst::PreventNextTimeDamage { source, target } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::PreventNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::PreventNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            let target_spec = match target {
                PreventNextTimeDamageTargetAst::AnyTarget => {
                    crate::effects::PreventNextTimeDamageTarget::AnyTarget
                }
                PreventNextTimeDamageTargetAst::You => {
                    crate::effects::PreventNextTimeDamageTarget::You
                }
            };
            (
                vec![Effect::new(
                    crate::effects::PreventNextTimeDamageEffect::new(source_spec, target_spec),
                )],
                Vec::new(),
            )
        }
        EffectAst::RedirectNextDamageFromSourceToTarget { amount, target } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::RedirectNextDamageToTargetEffect::new(
                    amount.clone(),
                    spec,
                ))
            })?
        }
        EffectAst::RedirectNextTimeDamageToSource { source, target } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::RedirectNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::RedirectNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::RedirectNextTimeDamageToSourceEffect::new(
                    source_spec.clone(),
                    spec,
                ))
            })?
        }
        EffectAst::PreventDamageEach {
            amount,
            filter,
            duration,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let effect = Effect::for_each(
                filter,
                vec![Effect::prevent_damage(
                    amount,
                    ChooseSpec::Iterated,
                    duration.clone(),
                )],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::AddMana { mana, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::add_mana(mana.clone()),
            |filter| Effect::add_mana_player(mana.clone(), filter),
        )?,
        EffectAst::AddManaScaled {
            mana,
            amount,
            player,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                    mana.clone(),
                    amount.clone(),
                    filter,
                ))
            })?
        }
        EffectAst::AddManaAnyColor {
            amount,
            player,
            available_colors,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || {
                    if let Some(colors) = available_colors.clone() {
                        Effect::add_mana_of_any_color_restricted(amount.clone(), colors)
                    } else {
                        Effect::add_mana_of_any_color(amount.clone())
                    }
                },
                |filter| {
                    if let Some(colors) = available_colors.clone() {
                        Effect::add_mana_of_any_color_restricted_player(
                            amount.clone(),
                            filter,
                            colors,
                        )
                    } else {
                        Effect::add_mana_of_any_color_player(amount.clone(), filter)
                    }
                },
            )?
        }
        EffectAst::AddManaAnyOneColor { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::add_mana_of_any_one_color(amount.clone()),
                |filter| Effect::add_mana_of_any_one_color_player(amount.clone(), filter),
            )?
        }
        EffectAst::AddManaChosenColor {
            amount,
            player,
            fixed_option,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                if let Some(fixed) = fixed_option {
                    Effect::new(
                        crate::effects::mana::AddManaOfChosenColorEffect::with_fixed_option(
                            amount.clone(),
                            filter,
                            *fixed,
                        ),
                    )
                } else {
                    Effect::new(crate::effects::mana::AddManaOfChosenColorEffect::new(
                        amount.clone(),
                        filter,
                    ))
                }
            })?
        }
        EffectAst::AddManaFromLandCouldProduce {
            amount,
            player,
            land_filter,
            allow_colorless,
            same_type,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                Effect::add_mana_of_land_produced_types_player(
                    amount.clone(),
                    filter,
                    land_filter.clone(),
                    *allow_colorless,
                    *same_type,
                )
            })?
        }
        EffectAst::AddManaCommanderIdentity { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::add_mana_from_commander_color_identity(amount.clone()),
                |filter| {
                    Effect::add_mana_from_commander_color_identity_player(amount.clone(), filter)
                },
            )?
        }
        EffectAst::AddManaImprintedColors => (
            vec![Effect::new(
                crate::effects::mana::AddManaOfImprintedColorsEffect::new(),
            )],
            Vec::new(),
        ),
        EffectAst::Scry { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::scry(count.clone()),
            |filter| Effect::scry_player(count.clone(), filter),
        )?,
        EffectAst::Fateseal { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::fateseal(count.clone()),
            |filter| Effect::fateseal_player(count.clone(), filter),
        )?,
        EffectAst::Discover { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::discover(count.clone()),
            |filter| Effect::discover_player(count.clone(), filter),
        )?,
        EffectAst::ConsultTopOfLibrary {
            player,
            mode,
            filter,
            stop_rule,
            all_tag,
            match_tag,
        } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_all_tag = resolve_it_tag_key(all_tag, &current_reference_env(ctx))?;
            let resolved_match_tag = resolve_it_tag_key(match_tag, &current_reference_env(ctx))?;
            let resolved_stop_rule = match stop_rule {
                crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch => {
                    crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                }
                crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value) => {
                    crate::effects::ConsultTopOfLibraryStopRule::MatchCount(resolve_value_it_tag(
                        value,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            let resolved_mode = match mode {
                crate::cards::builders::LibraryConsultModeAst::Reveal => {
                    crate::effects::consult_helpers::LibraryConsultMode::Reveal
                }
                crate::cards::builders::LibraryConsultModeAst::Exile => {
                    crate::effects::consult_helpers::LibraryConsultMode::Exile
                }
            };
            ctx.last_object_tag = Some(resolved_match_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter.clone());
            (
                vec![Effect::consult_top_of_library(
                    player_filter,
                    resolved_mode,
                    resolved_filter,
                    resolved_stop_rule,
                    resolved_all_tag,
                    resolved_match_tag,
                )],
                choices,
            )
        }
        EffectAst::PutTaggedRemainderOnBottomOfLibrary {
            tag,
            keep_tagged,
            order,
            player,
        } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let resolved_keep_tagged = keep_tagged
                .as_ref()
                .map(|tag| resolve_it_tag_key(tag, &current_reference_env(ctx)))
                .transpose()?;
            let resolved_order = match order {
                crate::cards::builders::LibraryBottomOrderAst::Random => {
                    crate::effects::consult_helpers::LibraryBottomOrder::Random
                }
                crate::cards::builders::LibraryBottomOrderAst::ChooserChooses => {
                    crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
                }
            };
            (
                vec![Effect::put_tagged_remainder_on_library_bottom(
                    resolved_tag,
                    resolved_keep_tagged,
                    resolved_order,
                    player_filter,
                )],
                choices,
            )
        }
        EffectAst::BecomeBasicLandTypeChoice { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "become_basic_land_type", |spec| {
                Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::new(
                    spec,
                    duration.clone(),
                ))
            })?
        }
        EffectAst::BecomeBasicLandType {
            target,
            subtype,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "become_basic_land_type", |spec| {
            Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::fixed(
                spec,
                *subtype,
                duration.clone(),
            ))
        })?,
        EffectAst::BecomeCreatureTypeChoice {
            target,
            duration,
            excluded_subtypes,
        } => {
            compile_tagged_effect_for_target(target, ctx, "become_creature_type_choice", |spec| {
                Effect::new(crate::effects::BecomeCreatureTypeChoiceEffect::new(
                    spec,
                    duration.clone(),
                    excluded_subtypes.clone(),
                ))
            })?
        }
        EffectAst::BecomeColorChoice { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "become_color_choice", |spec| {
                Effect::new(crate::effects::BecomeColorChoiceEffect::new(
                    spec,
                    duration.clone(),
                ))
            })?
        }
        EffectAst::BecomeCopy {
            target,
            source,
            duration,
        } => {
            let refs = current_reference_env(ctx);
            let (target_spec, mut choices) = resolve_target_spec_with_choices(target, &refs)?;
            let (source_spec, source_choices) = resolve_target_spec_with_choices(source, &refs)?;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }

            let effect = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                target_spec.clone(),
                crate::effects::continuous::RuntimeModification::CopyOf(source_spec),
                duration.clone(),
            ));
            let effect = tag_object_target_effect(effect, &target_spec, ctx, "copied");
            (vec![effect], choices)
        }
        EffectAst::Surveil { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::surveil(count.clone()),
            |filter| Effect::surveil_player(count.clone(), filter),
        )?,
        EffectAst::PayMana { cost, player } => {
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
                Effect::new(crate::effects::PayManaEffect::new(
                    cost.clone(),
                    ChooseSpec::Player(filter),
                ))
            })?
        }
        EffectAst::PayEnergy { amount, player } => {
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
                Effect::new(crate::effects::PayEnergyEffect::new(
                    amount.clone(),
                    ChooseSpec::Player(filter),
                ))
            })?
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_timing_and_control_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Cant {
            restriction,
            duration,
            condition,
        } => {
            let restriction = resolve_restriction_it_tag(restriction, &current_reference_env(ctx))?;
            if let Some(condition) = condition {
                match &restriction {
                    crate::effect::Restriction::Untap(filter) => {
                        let apply = crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Filter(filter.clone()),
                            crate::continuous::Modification::DoesntUntap,
                            duration.clone(),
                        )
                        .with_condition(condition.clone())
                        .lock_filter_at_resolution();
                        (vec![Effect::new(apply)], Vec::new())
                    }
                    other => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported conditioned restriction: {other:?}"
                        )));
                    }
                }
            } else {
                (
                    vec![Effect::cant_until(restriction, duration.clone())],
                    Vec::new(),
                )
            }
        }
        EffectAst::PlayFromGraveyardUntilEot { player } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let effect = Effect::grant_play_from_graveyard_until_eot(player_filter);
            (vec![effect], Vec::new())
        }
        EffectAst::AdditionalLandPlays {
            count,
            player,
            duration,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let effect = Effect::additional_land_plays(
                resolve_value_it_tag(count, &current_reference_env(ctx))?,
                player_filter,
                duration.clone(),
            );
            (vec![effect], Vec::new())
        }
        EffectAst::ReduceNextSpellCostThisTurn {
            player,
            filter,
            reduction,
        } => {
            let mut player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            (
                vec![Effect::new(
                    crate::effects::GrantNextSpellCostReductionEffect::new(
                        player_filter,
                        resolved_filter,
                        reduction.clone(),
                    ),
                )],
                Vec::new(),
            )
        }
        EffectAst::GrantNextSpellAbilityThisTurn {
            player,
            filter,
            ability,
        } => {
            let mut player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            let mut lowered = lower_granted_abilities_ast(std::slice::from_ref(ability))?;
            let Some(ability) = lowered.pop() else {
                return Err(CardTextError::ParseError(
                    "temporary next-spell grant did not lower to a static ability".to_string(),
                ));
            };
            (
                vec![Effect::grant_next_spell_ability_this_turn(
                    player_filter,
                    resolved_filter,
                    ability,
                )],
                Vec::new(),
            )
        }
        EffectAst::GrantPlayTaggedUntilEndOfTurn {
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            let mut effects = vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                resolved_tag.clone(),
                player_filter.clone(),
                crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                *allow_land,
                *allow_any_color_for_cast,
            ))];
            if *without_paying_mana_cost {
                effects.push(Effect::new(
                    crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
                        resolved_tag,
                        player_filter,
                    ),
                ));
            }
            (effects, Vec::new())
        }
        EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            tag,
            player,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            (
                vec![Effect::new(
                    crate::effects::GrantTaggedSpellLifeCostByManaValueEffect::new(
                        resolved_tag,
                        player_filter,
                    ),
                )],
                Vec::new(),
            )
        }
        EffectAst::GrantPlayTaggedUntilYourNextTurn {
            tag,
            player,
            allow_land,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            (
                vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    resolved_tag,
                    player_filter,
                    crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd,
                    *allow_land,
                    false,
                ))],
                Vec::new(),
            )
        }
        EffectAst::CastTagged {
            tag,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            cost_reduction,
        } => {
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            let effect = Effect::cast_tagged(
                resolved_tag,
                *allow_land,
                *as_copy,
                *without_paying_mana_cost,
                cost_reduction.clone(),
            );
            (vec![effect], Vec::new())
        }
        EffectAst::RegisterZoneReplacement {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            duration,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::OneShot
                }
            };
            let effect = Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
                spec,
                *from_zone,
                *to_zone,
                *replacement_zone,
                mode,
            ));
            (vec![effect], choices)
        }
        EffectAst::ExileInsteadOfGraveyardThisTurn { player } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let effect = Effect::exile_instead_of_graveyard_this_turn(player_filter);
            (vec![effect], Vec::new())
        }
        EffectAst::GainControl {
            target,
            player,
            duration,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (controller, mut controller_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            choices.append(&mut controller_choices);
            let runtime_modification = if matches!(controller, PlayerFilter::You) {
                crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController
            } else {
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    controller,
                )
            };
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec.clone(),
                    runtime_modification,
                    duration.clone(),
                )),
                &spec,
                ctx,
                "controlled",
            );
            (vec![effect], choices)
        }
        EffectAst::ControlPlayer { player, duration } => {
            let (start, duration) = match duration {
                ControlDurationAst::UntilEndOfTurn => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::DuringNextTurn => (
                    crate::game_state::PlayerControlStart::NextTurn,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::Forever => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::Forever,
                ),
                ControlDurationAst::AsLongAsYouControlSource => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilSourceLeaves,
                ),
            };

            let mut choices = Vec::new();
            if let PlayerFilter::Target(inner) = player {
                let spec = ChooseSpec::target(ChooseSpec::Player((**inner).clone()));
                choices.push(spec);
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else {
                ctx.last_player_filter = Some(player.clone());
            }

            let effect = Effect::control_player(player.clone(), start, duration);
            (vec![effect], choices)
        }
        EffectAst::ExtraTurnAfterTurn { player, anchor } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = match anchor {
                ExtraTurnAnchorAst::CurrentTurn => Effect::extra_turn_player(player_filter),
                ExtraTurnAnchorAst::ReferencedTurn => {
                    Effect::extra_turn_after_next_turn_player(player_filter)
                }
            };
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilNextEndStep { player, effects } => {
            let (delayed_effects, choices) = compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                Trigger::beginning_of_end_step(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilNextUpkeep { player, effects } => {
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_upkeep(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilNextDrawStep { player, effects } => {
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_draw_step(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects } => {
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilEndOfCombat { effects } => {
            let (delayed_effects, choices) = compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                Trigger::end_of_combat(),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::DelayedTriggerThisTurn { trigger, effects } => {
            let (delayed_effects, choices) = compile_trigger_effects(Some(trigger), effects)?;
            match trigger {
                TriggerSpec::IsDealtDamage(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            Trigger::is_dealt_damage(ChooseSpec::Source),
                            delayed_effects,
                            false,
                            watched_tag,
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_filter)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                compile_trigger_spec(TriggerSpec::IsDealtDamage(resolved_filter)),
                                delayed_effects,
                                false,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyard(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            Trigger::this_dies(),
                            delayed_effects,
                            false,
                            watched_tag,
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_filter)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                compile_trigger_spec(TriggerSpec::PutIntoGraveyard(
                                    resolved_filter,
                                )),
                                delayed_effects,
                                false,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyardFromZone { filter, from } => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            compile_trigger_spec(TriggerSpec::PutIntoGraveyardFromZone {
                                filter: resolved_filter,
                                from: *from,
                            }),
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn(),
                    );
                    (vec![effect], choices)
                }
                _ => {
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            compile_trigger_spec(trigger.clone()),
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn(),
                    );
                    (vec![effect], choices)
                }
            }
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { filter, effects } => {
            let target_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot schedule 'dies this turn' trigger without prior object context"
                        .to_string(),
                )
            })?;
            let previous_last = ctx.last_object_tag.clone();
            ctx.last_object_tag = Some("triggering".to_string());
            let compiled = compile_effects_preserving_last_effect(effects, ctx);
            ctx.last_object_tag = previous_last;
            let (delayed_effects, choices) = compiled?;
            let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                Trigger::this_dies(),
                delayed_effects,
                true,
                target_tag,
                PlayerFilter::You,
            );
            if let Some(filter) = filter {
                delayed = delayed
                    .with_target_filter(resolve_it_tag(filter, &current_reference_env(ctx))?);
            }
            let effect = Effect::new(delayed);
            (vec![effect], choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_flow_and_iteration_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::May { effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty may-effect branch is unsupported".to_string(),
                ));
            }
            if let Some(compiled) = lower_may_imprint_from_hand_effect(effects, ctx)? {
                return Ok(Some(compiled));
            }
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-effect branch is unsupported".to_string(),
                ));
            }
            let effect = Effect::may(inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::MayByPlayer { player, effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty may-by-player effect branch is unsupported".to_string(),
                ));
            }
            if matches!(player, PlayerAst::You | PlayerAst::Implicit)
                && let Some(compiled) = lower_may_imprint_from_hand_effect(effects, ctx)?
            {
                return Ok(Some(compiled));
            }
            let (player_filter, mut player_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-by-player effect branch is unsupported".to_string(),
                ));
            }
            let effect = Effect::may_player(player_filter, inner_effects);
            let mut choices = inner_choices;
            choices.append(&mut player_choices);
            (vec![effect], choices)
        }
        EffectAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn => (
            vec![Effect::new(
                crate::effects::RetainManaUntilEndOfTurnEffect::you(),
            )],
            Vec::new(),
        ),
        EffectAst::RepeatThisProcessMay => (
            vec![Effect::new(crate::effects::RepeatProcessPromptEffect::new(
                "You may repeat this process any number of times",
            ))],
            Vec::new(),
        ),
        EffectAst::UnlessPays {
            effects,
            player,
            mana,
        } => {
            if effects.len() == 1
                && let EffectAst::ForEachObject {
                    filter,
                    effects: per_object_effects,
                } = &effects[0]
            {
                let rewritten = EffectAst::ForEachObject {
                    filter: filter.clone(),
                    effects: vec![EffectAst::UnlessPays {
                        effects: per_object_effects.clone(),
                        player: *player,
                        mana: mana.clone(),
                    }],
                };
                return Ok(Some(compile_effect(&rewritten, ctx)?));
            }

            let previous_last_player_filter = ctx.last_player_filter.clone();
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let player_filter = resolve_unless_player_filter(
                *player,
                &current_reference_env(ctx),
                previous_last_player_filter,
            )?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::unless_pays(inner_effects, player_filter, mana.clone());
            (vec![effect], inner_choices)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        } => {
            if effects.len() == 1
                && let EffectAst::ForEachObject {
                    filter,
                    effects: per_object_effects,
                } = &effects[0]
            {
                let rewritten = EffectAst::ForEachObject {
                    filter: filter.clone(),
                    effects: vec![EffectAst::UnlessAction {
                        effects: per_object_effects.clone(),
                        alternative: alternative.clone(),
                        player: *player,
                    }],
                };
                return Ok(Some(compile_effect(&rewritten, ctx)?));
            }

            let previous_last_player_filter = ctx.last_player_filter.clone();
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let (alt_effects, alt_choices) = compile_effects(alternative, ctx)?;
            let player_filter = resolve_unless_player_filter(
                *player,
                &current_reference_env(ctx),
                previous_last_player_filter,
            )?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::unless_action(inner_effects, alt_effects, player_filter);
            let mut choices = inner_choices;
            choices.extend(alt_choices);
            (vec![effect], choices)
        }
        EffectAst::IfResult { predicate, effects } => {
            let condition = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for if clause".to_string())
            })?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(condition);
                    ctx.bind_unbound_x_to_last_effect = true;
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect = Effect::if_then(condition, predicate, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::WhenResult { predicate, effects } => {
            let condition = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for when clause".to_string())
            })?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(condition);
                    ctx.bind_unbound_x_to_last_effect = true;
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect =
                Effect::reflexive_trigger(condition, predicate, inner_effects, inner_choices);
            (vec![effect], Vec::new())
        }
        EffectAst::ForEachOpponent { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_opponent(inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachPlayersFiltered { filter, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = try_compile_simultaneous_each_player_scry(filter.clone(), &inner_effects)
                .unwrap_or_else(|| Effect::for_players(filter.clone(), inner_effects));
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachPlayer { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect =
                try_compile_simultaneous_each_player_scry(PlayerFilter::Any, &inner_effects)
                    .unwrap_or_else(|| Effect::for_players(PlayerFilter::Any, inner_effects));
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTargetPlayers { count, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let target_spec =
                ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)).with_count(*count);
            let choose_targets =
                Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()));
            let effect = try_compile_simultaneous_each_player_scry(
                PlayerFilter::target_player(),
                &inner_effects,
            )
            .unwrap_or_else(|| Effect::for_players(PlayerFilter::target_player(), inner_effects));
            let mut choices = vec![target_spec];
            for choice in inner_choices {
                push_choice(&mut choices, choice);
            }
            (vec![choose_targets, effect], choices)
        }
        EffectAst::ForEachObject { filter, effects } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = None;
                    ctx.last_object_tag = Some(IT_TAG.to_string());
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let effect = Effect::for_each(resolved_filter, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTagged { tag, effects } => {
            let effective_tag = if tag.as_str() == IT_TAG {
                ctx.last_object_tag
                    .clone()
                    .unwrap_or_else(|| IT_TAG.to_string())
            } else {
                tag.as_str().to_string()
            };

            let (inner_effects, inner_choices) = compile_effects_in_iterated_player_context(
                effects,
                ctx,
                Some(effective_tag.clone()),
            )?;
            let effect = Effect::for_each_tagged(effective_tag, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTaggedPlayer { tag, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_tagged_player(tag.clone(), inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::RepeatProcess {
            effects,
            continue_effect_index,
            continue_predicate,
        } => {
            let (body_effects, choices, condition) = with_preserved_lowering_context(
                ctx,
                |_| {},
                |ctx| compile_repeat_process_body(effects, *continue_effect_index, ctx),
            )?;
            let effect = Effect::repeat_process(
                body_effects,
                condition,
                effect_predicate_from_if_result(*continue_predicate),
            );
            (vec![effect], choices)
        }
        EffectAst::ForEachOpponentDoesNot { .. } => {
            return Err(CardTextError::ParseError(
                "for each opponent who doesn't must follow an opponent clause".to_string(),
            ));
        }
        EffectAst::ForEachPlayerDoesNot { .. } => {
            return Err(CardTextError::ParseError(
                "for each player who doesn't must follow a player clause".to_string(),
            ));
        }
        EffectAst::ForEachOpponentDid { .. } => {
            return Err(CardTextError::ParseError(
                "for each opponent who ... this way must follow an opponent clause".to_string(),
            ));
        }
        EffectAst::ForEachPlayerDid { .. } => {
            return Err(CardTextError::ParseError(
                "for each player who ... this way must follow a player clause".to_string(),
            ));
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_destroy_and_exile_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Destroy { target } => {
            compile_tagged_effect_for_target(target, ctx, "destroyed", |spec| {
                Effect::new(crate::effects::DestroyEffect::with_spec(spec))
            })?
        }
        EffectAst::DestroyNoRegeneration { target } => {
            compile_tagged_effect_for_target(target, ctx, "destroyed", |spec| {
                Effect::new(crate::effects::DestroyNoRegenerationEffect::with_spec(spec))
            })?
        }
        EffectAst::DestroyAllAttachedTo { filter, target } => {
            let (target_spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut prelude = Vec::new();
            let mut choices = choices;
            let target_tag = if let ChooseSpec::Tagged(tag) = &target_spec {
                tag.as_str().to_string()
            } else {
                if !choose_spec_targets_object(&target_spec) || !target_spec.is_target() {
                    return Err(CardTextError::ParseError(
                        "destroy-attached target must be an object target or tagged object"
                            .to_string(),
                    ));
                }
                let tag = ctx.next_tag("attachment_target");
                prelude.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
                        .tag(tag.clone()),
                );
                tag
            };
            ctx.last_object_tag = Some(target_tag.clone());

            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            resolved_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(target_tag.as_str()),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                });

            let (mut filter_prelude, filter_choices) =
                target_context_prelude_for_filter(&resolved_filter);
            for choice in filter_choices {
                push_choice(&mut choices, choice);
            }

            let mut effect = Effect::destroy_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.append(&mut filter_prelude);
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::DestroyAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = Effect::destroy_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::DestroyAllNoRegeneration { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = Effect::new(crate::effects::DestroyNoRegenerationEffect::all(
                resolved_filter,
            ));
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::DestroyAllOfChosenColor { filter } => {
            use crate::effect::EffectMode;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];
            let auto_tag = if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                ctx.last_object_tag = Some(tag.clone());
                Some(tag)
            } else {
                None
            };
            for (_name, color) in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = format!("Destroy all {}.", filter.description());
                let mut effect = Effect::destroy_all(filter);
                if let Some(tag) = &auto_tag {
                    effect = effect.tag(tag.clone());
                }
                modes.push(EffectMode {
                    description,
                    effects: vec![effect],
                });
            }
            prelude.push(Effect::choose_one(modes));
            (prelude, choices)
        }
        EffectAst::DestroyAllOfChosenColorNoRegeneration { filter } => {
            use crate::effect::EffectMode;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];
            let auto_tag = if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                ctx.last_object_tag = Some(tag.clone());
                Some(tag)
            } else {
                None
            };
            for (_name, color) in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = format!(
                    "Destroy all {}. They can't be regenerated.",
                    filter.description()
                );
                let mut effect =
                    Effect::new(crate::effects::DestroyNoRegenerationEffect::all(filter));
                if let Some(tag) = &auto_tag {
                    effect = effect.tag(tag.clone());
                }
                modes.push(EffectMode {
                    description,
                    effects: vec![effect],
                });
            }
            prelude.push(Effect::choose_one(modes));
            (prelude, choices)
        }
        EffectAst::ExileAll { filter, face_down } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if let Some(player_filter) = infer_player_filter_from_object_filter(&resolved_filter) {
                ctx.last_player_filter = Some(player_filter);
            }
            let keep_last_object_tag =
                resolved_filter.tagged_constraints.iter().any(|constraint| {
                    matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                    )
                });
            let mut effect = Effect::new(
                crate::effects::ExileEffect::all(resolved_filter).with_face_down(*face_down),
            );
            if ctx.auto_tag_object_targets && !keep_last_object_tag {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::Exile { target, face_down } => {
            if let Some(compiled) = lower_hand_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_counted_non_target_exile_target(target, *face_down, ctx)?
            {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_single_non_target_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = if spec.count().is_single() && !*face_down {
                Effect::move_to_zone(spec.clone(), Zone::Exile, true)
            } else {
                Effect::new(
                    crate::effects::ExileEffect::with_spec(spec.clone()).with_face_down(*face_down),
                )
            };
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::ExileWhenSourceLeaves { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let ChooseSpec::Tagged(tag) = spec.base() else {
                return Err(CardTextError::ParseError(
                    "cannot compile 'exile ... when this source leaves' without tagged context"
                        .to_string(),
                ));
            };
            let effect = Effect::new(crate::effects::ExileTaggedWhenSourceLeavesEffect::new(
                tag.clone(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::SacrificeSourceWhenLeaves { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let ChooseSpec::Tagged(tag) = spec.base() else {
                return Err(CardTextError::ParseError(
                    "cannot compile 'sacrifice this source when ... leaves' without tagged context"
                        .to_string(),
                ));
            };
            let effect = Effect::new(
                crate::effects::ScheduleEffectsWhenTaggedLeavesEffect::new(
                    tag.clone(),
                    vec![Effect::sacrifice_source()],
                    PlayerFilter::You,
                )
                .with_current_source_as_ability_source(),
            );
            (vec![effect], choices)
        }
        EffectAst::ExileUntilSourceLeaves { target, face_down } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = Effect::new(
                crate::effects::ExileUntilEffect::source_leaves(spec.clone())
                    .with_face_down(*face_down),
            );
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            (vec![effect], choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_visibility_and_card_selection_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::LookAtHand { target } => {
            let (effects, choices) = compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::LookAtHandEffect::new(spec))
            })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            (effects, choices)
        }
        EffectAst::TargetOnly { target } => {
            compile_tagged_effect_for_target(target, ctx, "targeted", |spec| {
                Effect::new(crate::effects::TargetOnlyEffect::new(spec))
            })?
        }
        EffectAst::RevealTop { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let tag = ctx.next_tag("revealed");
            ctx.last_object_tag = Some(tag.clone());
            let effect = Effect::reveal_top(player_filter, tag);
            (vec![effect], choices)
        }
        EffectAst::RevealTopChooseCardTypePutToHandRestBottom { player, count } => {
            use crate::effect::{Condition, EffectMode, Value};

            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut modes = Vec::new();
            let card_type_modes = [
                ("Artifact", CardType::Artifact),
                ("Battle", CardType::Battle),
                ("Creature", CardType::Creature),
                ("Enchantment", CardType::Enchantment),
                ("Instant", CardType::Instant),
                ("Kindred", CardType::Kindred),
                ("Land", CardType::Land),
                ("Planeswalker", CardType::Planeswalker),
                ("Sorcery", CardType::Sorcery),
            ];

            for (label, card_type) in card_type_modes {
                let looked_tag = ctx.next_tag("revealed");
                let mut card_type_filter = ObjectFilter::default();
                card_type_filter.card_types.push(card_type);

                let reveal = Effect::look_at_top_cards(
                    player_filter.clone(),
                    Value::Fixed(*count as i32),
                    TagKey::from(looked_tag.as_str()),
                );
                let reveal_tagged =
                    Effect::new(crate::effects::RevealTaggedEffect::new(looked_tag.clone()));
                let move_by_type = Effect::for_each_tagged(
                    looked_tag,
                    vec![Effect::conditional(
                        Condition::TaggedObjectMatches(TagKey::from("__it__"), card_type_filter),
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Hand,
                            false,
                        )],
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Library,
                            false,
                        )],
                    )],
                );

                modes.push(EffectMode {
                    description: label.to_string(),
                    effects: vec![reveal, reveal_tagged, move_by_type],
                });
            }

            (vec![Effect::choose_one(modes)], choices)
        }
        EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
            player,
            count,
            filter,
        } => {
            use crate::effect::{Condition, Value};

            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let looked_tag = ctx.next_tag("revealed");
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            resolved_filter.zone = None;

            let reveal = Effect::look_at_top_cards(
                player_filter,
                Value::Fixed(*count as i32),
                TagKey::from(looked_tag.as_str()),
            );
            let reveal_tagged =
                Effect::new(crate::effects::RevealTaggedEffect::new(looked_tag.clone()));
            let distribute = Effect::for_each_tagged(
                looked_tag.clone(),
                vec![Effect::conditional(
                    Condition::TaggedObjectMatches(TagKey::from("__it__"), resolved_filter),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Hand,
                        false,
                    )],
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Graveyard,
                        false,
                    )],
                )],
            );

            ctx.last_object_tag = Some(looked_tag);
            (vec![reveal, reveal_tagged, distribute], choices)
        }
        EffectAst::RevealTagged { tag } => {
            let resolved_tag = if tag.as_str() == IT_TAG {
                if let Some(existing) = ctx.last_object_tag.clone() {
                    existing
                } else {
                    let generated = ctx.next_tag("revealed");
                    ctx.last_object_tag = Some(generated.clone());
                    generated
                }
            } else {
                let explicit = tag.as_str().to_string();
                ctx.last_object_tag = Some(explicit.clone());
                explicit
            };
            (
                vec![Effect::new(crate::effects::RevealTaggedEffect::new(
                    resolved_tag,
                ))],
                Vec::new(),
            )
        }
        EffectAst::LookAtTopCards { player, count, tag } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.next_tag("revealed").as_str())
            } else {
                tag.clone()
            };
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            let effect = Effect::look_at_top_cards(player_filter, count.clone(), resolved_tag);
            (vec![effect], choices)
        }
        EffectAst::RevealHand { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let spec = choices
                .first()
                .cloned()
                .unwrap_or_else(|| ChooseSpec::Player(player_filter.clone()));
            ctx.last_player_filter = Some(match *player {
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => player_filter.clone(),
            });
            let effect = Effect::new(crate::effects::LookAtHandEffect::reveal(spec));
            (vec![effect], choices)
        }
        EffectAst::PutIntoHand { player, object } => {
            let ObjectRefAst::Tagged(tag) = object;
            let tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let (_, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = Effect::move_to_zone(ChooseSpec::Tagged(tag), Zone::Hand, false);
            (vec![effect], choices)
        }
        EffectAst::PutSomeIntoHandRestIntoGraveyard { player, count } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'them' without prior reference".to_string(),
                )
            })?;

            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;

            // Choose N from the looked-at cards (which are typically tagged by a prior LookAtTopCardsEffect).
            let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
            choose_filter.zone = Some(Zone::Library);
            let chosen_tag = ctx.next_tag("chosen");
            let chosen_tag_key: TagKey = chosen_tag.as_str().into();
            let choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    choose_filter,
                    ChoiceCount::exactly(*count as usize),
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            // Move the chosen cards to hand.
            let move_chosen = Effect::for_each_tagged(
                chosen_tag.clone(),
                vec![Effect::move_to_zone(
                    ChooseSpec::Iterated,
                    Zone::Hand,
                    false,
                )],
            );

            // Then move the rest to graveyard.
            let mut membership_filter = ObjectFilter::default();
            membership_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key, membership_filter);
            let move_rest = Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_chosen,
                    Vec::new(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Graveyard,
                        false,
                    )],
                )],
            );

            (vec![choose, move_chosen, move_rest], choices)
        }
        EffectAst::PutSomeIntoHandRestOnBottomOfLibrary { player, count } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'them' without prior reference".to_string(),
                )
            })?;

            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;

            // Choose N from the looked-at cards (which are typically tagged by a prior LookAtTopCardsEffect).
            let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
            choose_filter.zone = Some(Zone::Library);
            let chosen_tag = ctx.next_tag("chosen");
            let chosen_tag_key: TagKey = chosen_tag.as_str().into();
            let choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    choose_filter,
                    ChoiceCount::exactly(*count as usize),
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            // Move the chosen cards to hand.
            let move_chosen = Effect::for_each_tagged(
                chosen_tag.clone(),
                vec![Effect::move_to_zone(
                    ChooseSpec::Iterated,
                    Zone::Hand,
                    false,
                )],
            );

            // Then move the rest to the bottom of the library.
            let mut membership_filter = ObjectFilter::default();
            membership_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key, membership_filter);
            let move_rest = Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_chosen,
                    Vec::new(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Library,
                        false,
                    )],
                )],
            );

            (vec![choose, move_chosen, move_rest], choices)
        }
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
            player,
            filter,
            reveal,
            if_not_chosen,
        } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve looked-at cards without prior reference".to_string(),
                )
            })?;

            let (chooser, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut choose_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let source_zone = choose_filter.zone.unwrap_or(Zone::Library);
            choose_filter.zone = Some(source_zone);
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });

            let chosen_tag = ctx.next_tag("chosen");
            let chosen_tag_key: TagKey = chosen_tag.as_str().into();
            let choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    choose_filter,
                    ChoiceCount::up_to(1),
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(source_zone),
            );

            let mut compiled = vec![choose];
            if *reveal {
                compiled.push(Effect::for_each_tagged(
                    chosen_tag.clone(),
                    vec![Effect::new(crate::effects::RevealTaggedEffect::new(
                        chosen_tag.clone(),
                    ))],
                ));
            }
            let move_to_hand_id = ctx.next_effect_id();
            compiled.push(Effect::with_id(
                move_to_hand_id.0,
                Effect::for_each_tagged(
                    chosen_tag.clone(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Hand,
                        false,
                    )],
                ),
            ));

            if source_zone == Zone::Library {
                let mut membership_filter = ObjectFilter::default();
                membership_filter
                    .tagged_constraints
                    .push(TaggedObjectConstraint {
                        tag: TagKey::from("__it__"),
                        relation: TaggedOpbjectRelation::SameStableId,
                    });
                let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key, membership_filter);
                compiled.push(Effect::for_each_tagged(
                    looked_tag,
                    vec![Effect::conditional(
                        in_chosen,
                        Vec::new(),
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Graveyard,
                            false,
                        )],
                    )],
                ));
            }

            if !if_not_chosen.is_empty() {
                let (if_not_effects, if_not_choices) = with_preserved_lowering_context(
                    ctx,
                    |_| {},
                    |ctx| compile_effects(if_not_chosen, ctx),
                )?;
                compiled.push(Effect::if_then(
                    move_to_hand_id,
                    EffectPredicate::DidNotHappen,
                    if_not_effects,
                ));
                choices.extend(if_not_choices);
            }

            ctx.last_object_tag = Some(chosen_tag);
            ctx.last_effect_id = Some(move_to_hand_id);
            (compiled, choices)
        }
        EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
            player,
            filter,
            reveal,
            if_not_chosen,
        } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve looked-at cards without prior reference".to_string(),
                )
            })?;

            let (chooser, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut choose_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            choose_filter.zone = Some(Zone::Library);
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });

            let chosen_tag = ctx.next_tag("chosen");
            let chosen_tag_key: TagKey = chosen_tag.as_str().into();
            let choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    choose_filter,
                    ChoiceCount::up_to(1),
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            let mut compiled = vec![choose];
            if *reveal {
                compiled.push(Effect::for_each_tagged(
                    chosen_tag.clone(),
                    vec![Effect::new(crate::effects::RevealTaggedEffect::new(
                        chosen_tag.clone(),
                    ))],
                ));
            }
            let move_to_hand_id = ctx.next_effect_id();
            compiled.push(Effect::with_id(
                move_to_hand_id.0,
                Effect::for_each_tagged(
                    chosen_tag.clone(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Hand,
                        false,
                    )],
                ),
            ));

            let mut membership_filter = ObjectFilter::default();
            membership_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key, membership_filter);
            compiled.push(Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_chosen,
                    Vec::new(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Library,
                        false,
                    )],
                )],
            ));

            if !if_not_chosen.is_empty() {
                let (if_not_effects, if_not_choices) = with_preserved_lowering_context(
                    ctx,
                    |_| {},
                    |ctx| compile_effects(if_not_chosen, ctx),
                )?;
                compiled.push(Effect::if_then(
                    move_to_hand_id,
                    EffectPredicate::DidNotHappen,
                    if_not_effects,
                ));
                choices.extend(if_not_choices);
            }

            ctx.last_object_tag = Some(chosen_tag);
            ctx.last_effect_id = Some(move_to_hand_id);
            (compiled, choices)
        }
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player,
            battlefield_filter,
            tapped,
        } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve looked-at cards without prior reference".to_string(),
                )
            })?;

            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut primary_filter =
                resolve_it_tag(battlefield_filter, &current_reference_env(ctx))?;
            primary_filter.zone = Some(Zone::Library);
            primary_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });

            let battlefield_tag = ctx.next_tag("chosen");
            let battlefield_tag_key: TagKey = battlefield_tag.as_str().into();
            let choose_primary = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    primary_filter,
                    ChoiceCount::up_to(1),
                    chooser.clone(),
                    battlefield_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            let move_primary_id = ctx.next_effect_id();
            let move_primary = Effect::with_id(
                move_primary_id.0,
                Effect::for_each_tagged(
                    battlefield_tag.clone(),
                    vec![Effect::put_onto_battlefield(
                        ChooseSpec::Iterated,
                        *tapped,
                        chooser.clone(),
                    )],
                ),
            );

            let hand_tag = ctx.next_tag("chosen");
            let hand_tag_key: TagKey = hand_tag.as_str().into();
            let mut fallback_filter = ObjectFilter::tagged(looked_tag.clone());
            fallback_filter.zone = Some(Zone::Library);
            let fallback_choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    fallback_filter,
                    ChoiceCount::exactly(1),
                    chooser.clone(),
                    hand_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );
            let move_fallback = Effect::for_each_tagged(
                hand_tag.clone(),
                vec![Effect::move_to_zone(
                    ChooseSpec::Iterated,
                    Zone::Hand,
                    false,
                )],
            );
            let fallback = Effect::if_then(
                move_primary_id,
                EffectPredicate::DidNotHappen,
                vec![fallback_choose, move_fallback],
            );

            let mut in_battlefield_choice_filter = ObjectFilter::default();
            in_battlefield_choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_battlefield_choice =
                Condition::TaggedObjectMatches(battlefield_tag_key, in_battlefield_choice_filter);

            let mut in_hand_choice_filter = ObjectFilter::default();
            in_hand_choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_hand_choice =
                Condition::TaggedObjectMatches(hand_tag_key, in_hand_choice_filter);

            let move_rest = Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_battlefield_choice,
                    Vec::new(),
                    vec![Effect::conditional(
                        in_hand_choice,
                        Vec::new(),
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Library,
                            false,
                        )],
                    )],
                )],
            );

            ctx.last_object_tag = Some(hand_tag);
            ctx.last_effect_id = Some(move_primary_id);
            (
                vec![choose_primary, move_primary, fallback, move_rest],
                choices,
            )
        }
        EffectAst::PutRestOnBottomOfLibrary => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'rest' without prior reference".to_string(),
                )
            })?;

            let mut membership_filter = ObjectFilter::default();
            membership_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_it = Condition::TaggedObjectMatches(TagKey::from(IT_TAG), membership_filter);
            let move_rest = Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_it,
                    Vec::new(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Library,
                        false,
                    )],
                )],
            );

            (vec![move_rest], Vec::new())
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_stack_and_condition_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::ResolvedIfResult {
            condition,
            predicate,
            effects,
        } => {
            let (inner_effects, inner_choices) =
                with_preserved_lowering_context(ctx, |_| {}, |ctx| compile_effects(effects, ctx))?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect = Effect::if_then(*condition, predicate, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ResolvedWhenResult {
            condition,
            predicate,
            effects,
        } => {
            let (inner_effects, inner_choices) =
                with_preserved_lowering_context(ctx, |_| {}, |ctx| compile_effects(effects, ctx))?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect =
                Effect::reflexive_trigger(*condition, predicate, inner_effects, inner_choices);
            (vec![effect], Vec::new())
        }
        EffectAst::CopySpell {
            target,
            count,
            player,
            may_choose_new_targets,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let copy_effect = Effect::with_id(
                id.0,
                Effect::new(crate::effects::CopySpellEffect::new_for_player(
                    spec.clone(),
                    count.clone(),
                    player_filter.clone(),
                )),
            );
            let retarget_effect = if *may_choose_new_targets {
                Some(Effect::may_choose_new_targets_player(
                    id,
                    player_filter.clone(),
                ))
            } else {
                None
            };
            let mut compiled = vec![copy_effect];
            if let Some(retarget) = retarget_effect {
                compiled.push(retarget);
            }
            (compiled, choices)
        }
        EffectAst::RetargetStackObject {
            target,
            mode,
            chooser,
            require_change,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (chooser_filter, chooser_choices) =
                resolve_effect_player_filter(*chooser, ctx, true, true, true)?;
            for choice in chooser_choices {
                push_choice(&mut choices, choice);
            }

            let mut effect = crate::effects::RetargetStackObjectEffect::new(spec.clone())
                .with_chooser(chooser_filter);

            if *require_change {
                effect = effect.require_change();
            }

            let compiled_mode = match mode {
                RetargetModeAst::All => crate::effects::RetargetMode::All,
                RetargetModeAst::OneToFixed { target: fixed } => {
                    let (fixed_spec, fixed_choices) =
                        resolve_target_spec_with_choices(fixed, &current_reference_env(ctx))?;
                    for choice in fixed_choices {
                        push_choice(&mut choices, choice);
                    }
                    crate::effects::RetargetMode::OneToFixed(fixed_spec)
                }
            };
            effect = effect.with_mode(compiled_mode);

            let effect = tag_object_target_effect(Effect::new(effect), &spec, ctx, "retargeted");
            (vec![effect], choices)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            let saved_last_tag = ctx.last_object_tag.clone();
            let (true_effects, true_choices) = compile_effects(if_true, ctx)?;
            let true_last_tag = ctx.last_object_tag.clone();
            ctx.last_object_tag = saved_last_tag.clone();
            let (false_effects, false_choices) = compile_effects(if_false, ctx)?;
            let predicate_references_it = matches!(
                predicate,
                PredicateAst::ItIsLandCard
                    | PredicateAst::ItIsSoulbondPaired
                    | PredicateAst::ItMatches(_)
            ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == IT_TAG)
                || matches!(
                    predicate,
                    PredicateAst::PlayerTaggedObjectMatches { tag, .. } if tag.as_str() == IT_TAG
                );

            let antecedent_choice = if saved_last_tag.is_none() && predicate_references_it {
                let mut antecedent_choice = None;
                for choice in true_choices.iter().chain(false_choices.iter()) {
                    if choice.is_target() && choose_spec_targets_object(choice) {
                        antecedent_choice = Some(choice.clone());
                        break;
                    }
                }
                antecedent_choice
            } else {
                None
            };

            let mut condition_reference_tag = saved_last_tag.clone();
            let mut prelude = Vec::new();
            if condition_reference_tag.is_none()
                && let Some(choice) = antecedent_choice.clone()
            {
                let tag = if let Some(existing) = tagged_alias_for_choice(&true_effects, &choice) {
                    existing
                } else {
                    ctx.next_tag("targeted")
                };
                // Seed the tag from already-resolved spell targets without introducing an extra
                // target declaration requirement in UI/cast-time target collection.
                prelude.push(
                    Effect::new(crate::effects::SequenceEffect::new(Vec::new())).tag(tag.clone()),
                );
                condition_reference_tag = Some(tag);
            }

            let original_last_tag = ctx.last_object_tag.clone();
            ctx.last_object_tag = condition_reference_tag.clone().or(saved_last_tag.clone());
            let condition =
                compile_condition_from_predicate_ast(predicate, ctx, &condition_reference_tag)?;
            ctx.last_object_tag = original_last_tag;

            let conditional = if false_effects.is_empty() {
                Effect::conditional_only(condition, true_effects)
            } else {
                Effect::conditional(condition, true_effects, false_effects)
            };
            prelude.push(conditional);

            if let Some(reference_tag) = condition_reference_tag {
                ctx.last_object_tag = Some(reference_tag);
            } else if if_false.is_empty() {
                ctx.last_object_tag = true_last_tag.clone().or(saved_last_tag.clone());
            } else {
                ctx.last_object_tag = saved_last_tag.clone();
            }

            let mut choices = true_choices;
            for choice in false_choices {
                push_choice(&mut choices, choice);
            }
            if let Some(choice) = antecedent_choice {
                push_choice(&mut choices, choice);
            }
            (prelude, choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_attachment_and_setup_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Enchant { filter } => {
            let spec = filter.target_spec();
            let effect = Effect::attach_to(spec.clone());
            (vec![effect], vec![spec])
        }
        EffectAst::Attach { object, target } => {
            let (objects, object_choices) =
                resolve_attach_object_spec(object, &current_reference_env(ctx))?;
            let (target, target_choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut choices = Vec::new();
            for choice in object_choices {
                push_choice(&mut choices, choice);
            }
            for choice in target_choices {
                push_choice(&mut choices, choice);
            }
            (vec![Effect::attach_objects(objects, target)], choices)
        }
        EffectAst::PutSticker { target, action } => match target {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() =>
            {
                let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
                let tag = ctx.next_tag("stickered");
                let tag_key = TagKey::from(tag.as_str());
                let choose_effect = crate::effects::ChooseObjectsEffect::new(
                    resolved_filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag_key.clone(),
                )
                .in_zone(choice_zone);
                ctx.last_object_tag = Some(tag.as_str().to_string());
                (
                    vec![
                        Effect::new(choose_effect),
                        Effect::put_sticker(ChooseSpec::Tagged(tag_key), *action),
                    ],
                    Vec::new(),
                )
            }
            _ => compile_effect_for_target(target, ctx, |spec| Effect::put_sticker(spec, *action))?,
        },
        EffectAst::Investigate { count } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            (vec![Effect::investigate(count)], Vec::new())
        }
        EffectAst::Amass { subtype, amount } => {
            let mut effect = Effect::amass(*subtype, *amount);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("amassed");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], Vec::new())
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_token_generation_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::CreateTokenWithMods {
            name,
            count,
            dynamic_power_toughness,
            player,
            attached_to,
            tapped,
            attacking,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
        } => {
            let token = token_definition_for(name.as_str())
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut effect = if matches!(player_filter, PlayerFilter::You) {
                crate::effects::CreateTokenEffect::you(token, count.clone())
            } else {
                crate::effects::CreateTokenEffect::new(token, count.clone(), player_filter.clone())
            };
            if *tapped {
                effect = effect.tapped();
            }
            if *attacking {
                effect = effect.attacking();
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_end_of_combat();
            }
            if *sacrifice_at_end_of_combat {
                effect = effect.sacrifice_at_end_of_combat();
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step();
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step();
            }
            let mut effect = Effect::new(effect);
            let resolved_dynamic_pt = dynamic_power_toughness
                .as_ref()
                .map(|(power, toughness)| {
                    Ok::<_, CardTextError>((
                        resolve_value_it_tag(power, &current_reference_env(ctx))?,
                        resolve_value_it_tag(toughness, &current_reference_env(ctx))?,
                    ))
                })
                .transpose()?;
            let needs_created_tag = ctx.auto_tag_object_targets
                || attached_to.is_some()
                || resolved_dynamic_pt.is_some();
            let mut created_tag: Option<String> = None;
            if needs_created_tag {
                let tag = ctx.next_tag("created");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag.clone());
                created_tag = Some(tag);
            }

            let mut compiled = vec![effect];
            if let Some((power, toughness)) = resolved_dynamic_pt {
                let Some(created_tag) = created_tag.clone() else {
                    return Err(CardTextError::InvariantViolation(
                        "dynamic token pt requires created token tag to be present".to_string(),
                    ));
                };
                compiled.extend(
                    compile_effect_for_target(
                        &TargetAst::Tagged(TagKey::from(created_tag.as_str()), None),
                        ctx,
                        |spec| {
                            Effect::set_base_power_toughness(
                                power.clone(),
                                toughness.clone(),
                                spec,
                                Until::Forever,
                            )
                        },
                    )?
                    .0,
                );
            }
            if let Some(target) = attached_to {
                let (target_spec, target_choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                let Some(created_tag) = created_tag else {
                    return Err(CardTextError::InvariantViolation(
                        "attached token creation requires created token tag to be present"
                            .to_string(),
                    ));
                };
                let objects = ChooseSpec::All(ObjectFilter::tagged(created_tag));
                compiled.push(Effect::attach_objects(objects, target_spec));
            }
            (compiled, choices)
        }
        EffectAst::CreateTokenCopy {
            object,
            count,
            player,
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
            half_power_toughness_round_up,
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        } => {
            let ObjectRefAst::Tagged(tag) = object;
            let tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut effect = crate::effects::CreateTokenCopyEffect::new(
                ChooseSpec::Tagged(tag),
                count,
                player_filter,
            );
            if *enters_tapped {
                effect = effect.enters_tapped(true);
            }
            if *enters_attacking {
                effect = effect.attacking(true);
            }
            if let Some(attack_player) = attack_target_player_or_planeswalker_controlled_by {
                let attack_player_filter =
                    resolve_non_target_player_filter(*attack_player, &current_reference_env(ctx))?;
                effect =
                    effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter);
            }
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            if *has_haste {
                effect = effect.haste(true);
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_eoc(true);
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step(true);
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
            if let Some(colors) = set_colors {
                effect = effect.set_colors(*colors);
            }
            if let Some(card_types) = set_card_types {
                effect = effect.set_card_types(card_types.clone());
            }
            if let Some(subtypes) = set_subtypes {
                effect = effect.set_subtypes(subtypes.clone());
            }
            for card_type in added_card_types {
                effect = effect.added_card_type(*card_type);
            }
            for subtype in added_subtypes {
                effect = effect.added_subtype(*subtype);
            }
            for supertype in removed_supertypes {
                effect = effect.removed_supertype(*supertype);
            }
            if let Some((power, toughness)) = set_base_power_toughness {
                effect = effect.set_base_power_toughness(*power, *toughness);
            }
            for ability in granted_abilities {
                effect = effect.grant_static_ability(ability.clone());
            }
            let mut effect = Effect::new(effect);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::CreateTokenCopyFromSource {
            source,
            count,
            player,
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
            half_power_toughness_round_up,
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (mut source_spec, source_choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }
            if let Some(last_tag) = ctx.last_object_tag.as_deref()
                && str_starts_with(last_tag, "exile_cost_")
                && let ChooseSpec::Object(filter) = &source_spec
                && filter.zone == Some(Zone::Exile)
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                        && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                })
            {
                source_spec = ChooseSpec::Tagged(TagKey::from(last_tag));
            }
            let mut effect =
                crate::effects::CreateTokenCopyEffect::new(source_spec, count, player_filter);
            if *enters_tapped {
                effect = effect.enters_tapped(true);
            }
            if *enters_attacking {
                effect = effect.attacking(true);
            }
            if let Some(attack_player) = attack_target_player_or_planeswalker_controlled_by {
                let attack_player_filter =
                    resolve_non_target_player_filter(*attack_player, &current_reference_env(ctx))?;
                effect =
                    effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter);
            }
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            if *has_haste {
                effect = effect.haste(true);
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_eoc(true);
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step(true);
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
            if let Some(colors) = set_colors {
                effect = effect.set_colors(*colors);
            }
            if let Some(card_types) = set_card_types {
                effect = effect.set_card_types(card_types.clone());
            }
            if let Some(subtypes) = set_subtypes {
                effect = effect.set_subtypes(subtypes.clone());
            }
            for card_type in added_card_types {
                effect = effect.added_card_type(*card_type);
            }
            for subtype in added_subtypes {
                effect = effect.added_subtype(*subtype);
            }
            for supertype in removed_supertypes {
                effect = effect.removed_supertype(*supertype);
            }
            if let Some((power, toughness)) = set_base_power_toughness {
                effect = effect.set_base_power_toughness(*power, *toughness);
            }
            for ability in granted_abilities {
                effect = effect.grant_static_ability(ability.clone());
            }

            let mut effect = Effect::new(effect);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_continuous_and_modifier_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Monstrosity { amount } => {
            (vec![Effect::monstrosity(amount.clone())], Vec::new())
        }
        EffectAst::RemoveUpToAnyCounters {
            amount,
            target,
            counter_type,
            up_to,
        } => {
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            compile_tagged_effect_for_target(target, ctx, "counters", |spec| {
                let effect = if let Some(counter_type) = counter_type {
                    if *up_to {
                        Effect::remove_up_to_counters(*counter_type, amount.clone(), spec)
                    } else {
                        Effect::remove_counters(*counter_type, amount.clone(), spec)
                    }
                } else {
                    Effect::remove_up_to_any_counters(amount.clone(), spec)
                };
                Effect::with_id(id.0, effect)
            })?
        }
        EffectAst::MoveAllCounters { from, to } => {
            let (from_spec, mut choices) =
                resolve_target_spec_with_choices(from, &current_reference_env(ctx))?;
            let (to_spec, to_choices) =
                resolve_target_spec_with_choices(to, &current_reference_env(ctx))?;
            for choice in to_choices {
                push_choice(&mut choices, choice);
            }
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_all_counters(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            (vec![effect], choices)
        }
        EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
            condition,
        } => {
            let resolved_power = resolve_value_it_tag(power, &current_reference_env(ctx))?;
            let resolved_toughness = resolve_value_it_tag(toughness, &current_reference_env(ctx))?;
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec,
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: resolved_power.clone(),
                        toughness: resolved_toughness.clone(),
                    },
                    duration.clone(),
                )
                .require_creature_target();
                if let Some(condition) = condition {
                    apply = apply.with_condition(condition.clone());
                }
                Effect::new(apply)
            })?
        }
        EffectAst::SwitchPowerToughness { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "switched_pt", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec(
                        spec,
                        crate::continuous::Modification::SwitchPowerToughness,
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })?
        }
        EffectAst::SetBasePowerToughness {
            power,
            toughness,
            target,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_base_pt", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::SetPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        sublayer: crate::continuous::PtSublayer::Setting,
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .resolve_set_pt_values_at_resolution(),
            )
        })?,
        EffectAst::BecomeBasePtCreature {
            power,
            toughness,
            target,
            card_types,
            subtypes,
            colors,
            abilities,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "animated_creature", |spec| {
            let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddCardTypes(card_types.clone()),
                duration.clone(),
            )
            .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
                power: power.clone(),
                toughness: toughness.clone(),
                sublayer: crate::continuous::PtSublayer::Setting,
            })
            .resolve_set_pt_values_at_resolution();
            if let Some(colors) = colors {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::SetColors(*colors),
                );
            }
            if !subtypes.is_empty() {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::AddSubtypes(subtypes.clone()),
                );
            }
            for ability in abilities {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::AddAbility(ability.clone()),
                );
            }
            Effect::new(apply)
        })?,
        EffectAst::AddCardTypes {
            target,
            card_types,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddCardTypes(card_types.clone()),
                duration.clone(),
            ))
        })?,
        EffectAst::RemoveCardTypes {
            target,
            card_types,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::RemoveCardTypes(card_types.clone()),
                duration.clone(),
            ))
        })?,
        EffectAst::AddSubtypes {
            target,
            subtypes,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "subtyped", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddSubtypes(subtypes.clone()),
                duration.clone(),
            ))
        })?,
        EffectAst::SetBasePower {
            power,
            target,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_base_power", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::SetPower {
                        value: power.clone(),
                        sublayer: crate::continuous::PtSublayer::Setting,
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .resolve_set_pt_values_at_resolution(),
            )
        })?,
        EffectAst::SetColors {
            target,
            colors,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_colors", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::SetColors(*colors),
                duration.clone(),
            ))
        })?,
        EffectAst::MakeColorless { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "set_colorless", |spec| {
                Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::MakeColorless,
                    duration.clone(),
                ))
            })?
        }
        EffectAst::PumpForEach {
            power_per,
            toughness_per,
            target,
            count,
            duration,
        } => {
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::pump_for_each(
                    spec,
                    *power_per,
                    *toughness_per,
                    resolved_count.clone(),
                    duration.clone(),
                )
            })?
        }
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let tag = ctx.next_tag("pumped");
            let effect = Effect::new(
                crate::effects::ApplyContinuousEffect::new_runtime(
                    crate::continuous::EffectTarget::Filter(resolved_filter.clone()),
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                    },
                    duration.clone(),
                )
                .lock_filter_at_resolution(),
            )
            .tag_all(tag.clone());
            ctx.last_object_tag = Some(tag);
            (vec![effect], Vec::new())
        }
        EffectAst::ScalePowerToughnessAll {
            filter,
            power,
            toughness,
            multiplier,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let scaled_stat = |value: Value| {
                if *multiplier == 1 {
                    value
                } else {
                    Value::Scaled(Box::new(value), *multiplier)
                }
            };
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        ChooseSpec::Iterated,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: if *power {
                                scaled_stat(Value::PowerOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                            toughness: if *toughness {
                                scaled_stat(Value::ToughnessOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
        } => {
            let id = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for pump clause".to_string())
            })?;
            let power_value = if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            };
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        spec,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: power_value.clone(),
                            toughness: Value::Fixed(*toughness),
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })?
        }
        EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesAll with no abilities"
                        .to_string(),
                ));
            }

            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                modifications[0].clone(),
                duration.clone(),
            )
            .lock_filter_at_resolution();

            for modification in modifications.iter().skip(1) {
                apply = apply.with_additional_modification(modification.clone());
            }

            (vec![Effect::new(apply)], Vec::new())
        }
        EffectAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
        } => {
            let abilities = lower_granted_abilities_ast(abilities)?;
            if abilities.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove RemoveAbilitiesAll with no abilities"
                        .to_string(),
                ));
            }

            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                crate::continuous::Modification::RemoveAbility(abilities[0].clone()),
                duration.clone(),
            )
            .lock_filter_at_resolution();

            for ability in abilities.iter().skip(1) {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::RemoveAbility(ability.clone()),
                );
            }

            (vec![Effect::new(apply)], Vec::new())
        }
        EffectAst::GrantAbilitiesChoiceAll {
            filter,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesChoiceAll with no abilities"
                        .to_string(),
                ));
            }
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let modes = modifications
                .iter()
                .map(|modification| EffectMode {
                    description: String::new(),
                    effects: vec![Effect::new(
                        crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Filter(resolved_filter.clone()),
                            modification.clone(),
                            duration.clone(),
                        )
                        .lock_filter_at_resolution(),
                    )],
                })
                .collect::<Vec<_>>();
            (vec![Effect::choose_one(modes)], Vec::new())
        }
        EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            let Some(first_modification) = modifications.first() else {
                return Ok(Some(compile_tagged_effect_for_target(
                    target,
                    ctx,
                    "granted",
                    |spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
                )?));
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    first_modification.clone(),
                    duration.clone(),
                );

                for modification in modifications.iter().skip(1) {
                    apply = apply.with_additional_modification(modification.clone());
                }

                Effect::new(apply)
            })?
        }
        EffectAst::GrantToTarget {
            target,
            grantable,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
            Effect::grant(grantable.clone(), spec, *duration)
        })?,
        EffectAst::GrantBySpec {
            spec,
            player,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(&spec.filter, &current_reference_env(ctx))?;
            let player =
                resolve_non_target_player_filter(player.clone(), &current_reference_env(ctx))?;
            let mut resolved_spec = spec.clone();
            resolved_spec.filter = resolved_filter;
            (
                vec![Effect::grant_by_spec(resolved_spec, player, *duration)],
                Vec::new(),
            )
        }
        EffectAst::RemoveAbilitiesFromTarget {
            target,
            abilities,
            duration,
        } => {
            let abilities = lower_granted_abilities_ast(abilities)?;
            let Some(first_ability) = abilities.first() else {
                return Ok(Some(compile_tagged_effect_for_target(
                    target,
                    ctx,
                    "granted",
                    |spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
                )?));
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::RemoveAbility(first_ability.clone()),
                    duration.clone(),
                );

                for ability in abilities.iter().skip(1) {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::RemoveAbility(ability.clone()),
                    );
                }

                Effect::new(apply)
            })?
        }
        EffectAst::GrantAbilitiesChoiceToTarget {
            target,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Ok(Some(compile_tagged_effect_for_target(
                    target,
                    ctx,
                    "granted",
                    |spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
                )?));
            }

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let modes = abilities
                    .iter()
                    .zip(modifications.iter())
                    .map(|(ability, modification)| EffectMode {
                        description: granted_ability_mode_description(ability, &spec)
                            .unwrap_or_default(),
                        effects: vec![Effect::new(
                            crate::effects::ApplyContinuousEffect::with_spec(
                                spec.clone(),
                                modification.clone(),
                                duration.clone(),
                            ),
                        )],
                    })
                    .collect::<Vec<_>>();
                Effect::choose_one(modes)
            })?
        }
        EffectAst::Transform { target } => {
            compile_tagged_effect_for_target(target, ctx, "transformed", Effect::transform)?
        }
        EffectAst::Meld {
            result_name,
            enters_tapped,
            enters_attacking,
        } => (
            vec![Effect::new(
                crate::effects::MeldEffect::new(result_name.clone())
                    .enters_tapped(*enters_tapped)
                    .enters_attacking(*enters_attacking),
            )],
            Vec::new(),
        ),
        EffectAst::Convert { target } => {
            compile_tagged_effect_for_target(target, ctx, "converted", Effect::convert)?
        }
        EffectAst::Flip { target } => {
            compile_tagged_effect_for_target(target, ctx, "flipped", Effect::flip)?
        }
        EffectAst::GrantAbilityToSource { ability } => {
            let lowered = lower_parsed_ability(ability.clone())?;
            (
                vec![Effect::grant_object_ability_to_source(lowered.ability)],
                Vec::new(),
            )
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_search_and_reorder_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::SearchLibrary {
            filter,
            destination,
            chooser,
            player,
            search_mode,
            reveal,
            shuffle,
            count,
            count_value,
            tapped,
        } => {
            let (chooser_filter, chooser_choices) = if matches!(*chooser, PlayerAst::Implicit)
                && matches!(*player, PlayerAst::That)
                && filter.owner.is_some()
                && ctx.last_player_filter.as_ref().is_some_and(|filter| {
                    !matches!(
                        filter,
                        PlayerFilter::IteratedPlayer | PlayerFilter::TaggedPlayer(_)
                    )
                }) {
                (PlayerFilter::You, Vec::new())
            } else {
                resolve_effect_player_filter(*chooser, ctx, true, true, true)?
            };
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            for choice in chooser_choices {
                push_choice(&mut choices, choice);
            }
            let count = *count;
            let mut filter = filter.clone();
            if filter.owner.is_none() && !matches!(player_filter, PlayerFilter::You) {
                filter.owner = Some(player_filter.clone());
            }
            ctx.last_player_filter = Some(
                filter
                    .owner
                    .clone()
                    .unwrap_or_else(|| player_filter.clone()),
            );
            let use_search_effect = *shuffle
                && count.min == 0
                && count.max == Some(1)
                && count_value.is_none()
                && *destination != Zone::Battlefield;
            if use_search_effect {
                let mut effect = Effect::new(
                    crate::effects::SearchLibraryEffect::new(
                        filter,
                        *destination,
                        chooser_filter.clone(),
                        player_filter.clone(),
                        *reveal,
                    )
                    .with_search_mode(*search_mode),
                );
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("searched");
                    ctx.last_object_tag = Some(tag.clone());
                    effect = effect.tag(tag);
                }
                let effects = vec![effect];
                (effects, choices)
            } else {
                let tag = ctx.next_tag("searched");
                if ctx.auto_tag_object_targets {
                    ctx.last_object_tag = Some(tag.clone());
                }
                let mut generic_search_filter = ObjectFilter::default();
                generic_search_filter.owner = filter.owner.clone();
                let choose_description = if filter == generic_search_filter {
                    if count.max == Some(1) {
                        "card"
                    } else {
                        "cards"
                    }
                } else {
                    "objects"
                };
                let choose = crate::effects::ChooseObjectsEffect::new(
                    filter,
                    count,
                    chooser_filter.clone(),
                    tag.clone(),
                )
                .with_count_value_opt(count_value.clone())
                .in_zone(Zone::Library)
                .with_description(choose_description);
                let choose = match search_mode {
                    crate::effect::SearchSelectionMode::Exact => choose.as_search(),
                    crate::effect::SearchSelectionMode::Optional => choose.as_optional_search(),
                    crate::effect::SearchSelectionMode::AllMatching => {
                        choose.as_all_matching_search()
                    }
                };
                let choose = if *reveal { choose.reveal() } else { choose };

                let to_top = matches!(destination, Zone::Library);
                let move_effect = if *destination == Zone::Battlefield {
                    Effect::put_onto_battlefield(
                        ChooseSpec::Iterated,
                        *tapped,
                        player_filter.clone(),
                    )
                } else {
                    Effect::move_to_zone(ChooseSpec::Iterated, *destination, to_top)
                };
                let mut sequence_effects = vec![Effect::new(choose)];
                if *shuffle && *destination == Zone::Library {
                    sequence_effects.push(Effect::shuffle_library_player(player_filter.clone()));
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                } else {
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                    if *shuffle {
                        sequence_effects.push(Effect::shuffle_library_player(player_filter));
                    }
                }
                let sequence = crate::effects::SequenceEffect::new(sequence_effects);
                (vec![Effect::new(sequence)], std::mem::take(&mut choices))
            }
        }
        EffectAst::ShuffleGraveyardIntoLibrary { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::shuffle_graveyard_into_library_player,
        )?,
        EffectAst::ReorderGraveyard { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::reorder_graveyard_player)?
        }
        EffectAst::ReorderTopOfLibrary { tag } => {
            let effective_tag = if tag.as_str() == IT_TAG {
                ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "cannot resolve 'them' without prior tagged object".to_string(),
                    )
                })?
            } else {
                tag.as_str().to_string()
            };
            (
                vec![Effect::new(crate::effects::ReorderLibraryTopEffect::new(
                    effective_tag,
                ))],
                Vec::new(),
            )
        }
        EffectAst::ShuffleLibrary { player } => {
            if ctx
                .last_object_tag
                .as_ref()
                .is_some_and(|tag| tag.starts_with("searched"))
                && ctx
                    .last_player_filter
                    .as_ref()
                    .is_some_and(|filter| *filter != PlayerFilter::You)
            {
                (
                    vec![Effect::shuffle_library_player(
                        ctx.last_player_filter.clone().expect("checked above"),
                    )],
                    Vec::new(),
                )
            } else {
                compile_player_effect_from_filter(
                    *player,
                    ctx,
                    true,
                    Effect::shuffle_library_player,
                )?
            }
        }
        EffectAst::VoteOption { option, effects } => {
            let mut option_effects_ast = effects.clone();
            force_implicit_vote_token_controller_you(&mut option_effects_ast);
            let (repeat_effects, repeat_choices) = compile_effects(&option_effects_ast, ctx)?;
            (
                vec![Effect::repeat_effects(
                    Value::VoteCount(option.clone()),
                    repeat_effects,
                )],
                repeat_choices,
            )
        }
        EffectAst::VoteStart { .. }
        | EffectAst::VoteStartObjects { .. }
        | EffectAst::VoteExtra { .. } => {
            return Err(CardTextError::ParseError(
                "vote clauses must appear together".to_string(),
            ));
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
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

fn try_compile_object_zone_and_exchange_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            tag,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let references_revealed_hand = filter.zone == Some(Zone::Hand)
                && filter.owner.is_none()
                && filter.controller.is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if references_revealed_hand && ctx.last_player_filter.is_some() {
                resolved_filter.tagged_constraints.retain(|constraint| {
                    !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
                resolved_filter.owner = ctx.last_player_filter.clone();
            }
            preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
            if choice_zone == Zone::Battlefield
                && resolved_filter.controller.is_none()
                && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
            let followup_player = choose_followup_player_filter(&resolved_filter, &chooser)
                .unwrap_or_else(|| chooser.clone());
            let choose_effect = crate::effects::ChooseObjectsEffect::new(
                resolved_filter,
                *count,
                chooser,
                tag.clone(),
            )
            .in_zone(choice_zone);
            let effect = Effect::new(choose_effect);
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(followup_player);
            (effects, choices)
        }
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            player,
            tag,
            zones,
            search_mode,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let references_revealed_hand = filter.zone == Some(Zone::Hand)
                && filter.owner.is_none()
                && filter.controller.is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if references_revealed_hand && ctx.last_player_filter.is_some() {
                resolved_filter.tagged_constraints.retain(|constraint| {
                    !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
                resolved_filter.owner = ctx.last_player_filter.clone();
            }
            preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            if slice_contains(zones.as_slice(), &Zone::Battlefield)
                && resolved_filter.controller.is_none()
                && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
            let followup_player = choose_followup_player_filter(&resolved_filter, &chooser)
                .unwrap_or_else(|| chooser.clone());
            let mut choose_effect = crate::effects::ChooseObjectsEffect::new(
                resolved_filter,
                *count,
                chooser,
                tag.clone(),
            )
            .in_zones(zones.clone());
            if let Some(search_mode) = search_mode {
                choose_effect = match search_mode {
                    crate::effect::SearchSelectionMode::Exact => choose_effect.as_search(),
                    crate::effect::SearchSelectionMode::Optional => {
                        choose_effect.as_optional_search()
                    }
                    crate::effect::SearchSelectionMode::AllMatching => {
                        choose_effect.as_all_matching_search()
                    }
                };
            } else if slice_contains(zones.as_slice(), &Zone::Library) {
                choose_effect = choose_effect.as_search();
            }
            let effect = Effect::new(choose_effect);
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(followup_player);
            (effects, choices)
        }
        EffectAst::ChoosePlayer {
            chooser,
            filter,
            tag,
            random,
            exclude_previous_choices,
        } => {
            let (chooser_filter, choices) =
                resolve_effect_player_filter(*chooser, ctx, true, true, false)?;
            let resolved_filter = filter.clone();
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.next_tag("chosen_player").as_str())
            } else {
                tag.clone()
            };
            let excluded_tags = if *exclude_previous_choices == 0 {
                Vec::new()
            } else {
                let len = ctx.recent_player_choice_tags.len();
                ctx.recent_player_choice_tags[len.saturating_sub(*exclude_previous_choices)..]
                    .iter()
                    .cloned()
                    .map(TagKey::from)
                    .collect::<Vec<_>>()
            };
            let mut choose_effect = crate::effects::ChoosePlayerEffect::new(
                chooser_filter,
                resolved_filter,
                resolved_tag.clone(),
            )
            .excluding_tags(excluded_tags);
            if *random {
                choose_effect = choose_effect.at_random();
            }
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::new(choose_effect));
            ctx.last_player_filter = Some(PlayerFilter::TaggedPlayer(resolved_tag.clone()));
            ctx.recent_player_choice_tags
                .push(resolved_tag.as_str().to_string());
            (effects, choices)
        }
        EffectAst::TagMatchingObjects { filter, zones, tag } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut effect =
                crate::effects::TagMatchingObjectsEffect::new(resolved_filter, tag.clone());
            if !zones.is_empty() {
                effect = effect.in_zones(zones.clone());
            }
            ctx.last_object_tag = Some(tag.as_str().to_string());
            (vec![Effect::new(effect)], Vec::new())
        }
        EffectAst::ChooseSpellCastHistory {
            chooser,
            cast_by,
            filter,
            tag,
        } => {
            let (chooser_filter, choices) =
                resolve_effect_player_filter(*chooser, ctx, true, true, false)?;
            let cast_by_filter =
                resolve_non_target_player_filter(*cast_by, &current_reference_env(ctx))?;
            let effect = Effect::new(
                crate::effects::ChooseSpellCastHistoryEffect::new(
                    chooser_filter,
                    cast_by_filter,
                    filter.clone(),
                    tag.clone(),
                )
                .with_description("Choose one of those sorcery spells"),
            );
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            ctx.last_object_tag = Some(tag.as_str().to_string());
            (effects, choices)
        }
        EffectAst::ChooseCardName {
            player,
            filter,
            tag,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_card_name(
                chooser.clone(),
                filter.clone(),
                tag.clone(),
            ));
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::ChooseColor { player } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_color(chooser.clone()));
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::ChooseNamedOption { player, options } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_named_option(
                chooser.clone(),
                options.clone(),
            ));
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::ChooseCreatureType {
            player,
            excluded_subtypes,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_creature_type(
                chooser.clone(),
                excluded_subtypes.clone(),
            ));
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::Sacrifice {
            filter,
            player,
            count,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let target_prelude: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            let mut resolved_filter = match resolve_it_tag(filter, &current_reference_env(ctx)) {
                Ok(resolved) => resolved,
                Err(_)
                    if filter.tagged_constraints.len() == 1
                        && filter.tagged_constraints[0].tag.as_str() == IT_TAG =>
                {
                    // "Sacrifice it" can legally refer to the source itself in standalone
                    // clauses like "If there are no counters on this land, sacrifice it."
                    ObjectFilter::source()
                }
                Err(err) => return Err(err),
            };
            if resolved_filter.controller.is_none() && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
            if resolved_filter.source {
                if *count != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "source sacrifice only supports count 1 (count: {})",
                        count
                    )));
                }
                if !matches!(chooser, PlayerFilter::You) {
                    return Err(CardTextError::ParseError(
                        "source sacrifice requires source controller chooser".to_string(),
                    ));
                }
                let mut effects = target_prelude;
                effects.push(Effect::sacrifice_source());
                return Ok(Some((effects, choices)));
            }
            if *count == 1
                && let Some(tag) = object_filter_as_tagged_reference(&resolved_filter)
            {
                let mut effects = target_prelude;
                effects.push(Effect::new(crate::effects::SacrificeTargetEffect::new(
                    ChooseSpec::tagged(tag),
                )));
                return Ok(Some((effects, choices)));
            }

            let tag = ctx.next_tag("sacrificed");
            ctx.last_object_tag = Some(tag.clone());
            let choose = Effect::choose_objects(
                resolved_filter,
                *count as usize,
                chooser.clone(),
                tag.clone(),
            );
            let sacrifice =
                Effect::sacrifice_player(ObjectFilter::tagged(tag), *count, chooser.clone());
            let mut effects = target_prelude;
            effects.push(choose);
            effects.push(sacrifice);
            (effects, choices)
        }
        EffectAst::SacrificeAll { filter, player } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let count = Value::Count(resolved_filter.clone());
            let effect = Effect::sacrifice_player(resolved_filter, count, chooser.clone());
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            (effects, choices)
        }
        EffectAst::DiscardHand { player } => compile_player_effect(
            *player,
            ctx,
            true,
            Effect::discard_hand,
            Effect::discard_hand_player,
        )?,
        EffectAst::Discard {
            count,
            player,
            random,
            filter,
            tag,
        } => {
            let resolved_filter = if let Some(filter) = filter {
                let mut resolved = resolve_it_tag(filter, &current_reference_env(ctx))?;
                if resolved.zone.is_none() {
                    resolved.zone = Some(Zone::Hand);
                }
                Some(resolved)
            } else {
                None
            };
            let (resolved_player, choices) = if matches!(*player, PlayerAst::Implicit) {
                if let Some(inferred_player) = resolved_filter
                    .as_ref()
                    .and_then(infer_player_filter_from_object_filter)
                    .or_else(|| ctx.last_player_filter.clone())
                {
                    (inferred_player, Vec::new())
                } else {
                    resolve_effect_player_filter(*player, ctx, true, true, true)?
                }
            } else {
                resolve_effect_player_filter(*player, ctx, true, true, true)?
            };
            let resolved_filter = resolved_filter.map(|mut resolved| {
                if resolved.owner.is_none() {
                    resolved.owner = Some(resolved_player.clone());
                }
                resolved
            });
            let tag = tag
                .clone()
                .unwrap_or_else(|| TagKey::from(ctx.next_tag("discarded").as_str()));
            ctx.last_object_tag = Some(tag.as_str().to_string());
            let effect = Effect::new(
                crate::effects::DiscardEffect::new_with_filter(
                    count.clone(),
                    resolved_player,
                    *random,
                    resolved_filter,
                )
                .with_tag(tag),
            );
            (vec![effect], choices)
        }
        EffectAst::Connive { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect =
                tag_object_target_effect(Effect::connive(spec.clone()), &spec, ctx, "connived");
            (vec![effect], choices)
        }
        EffectAst::ConniveIterated => (vec![Effect::connive(ChooseSpec::Iterated)], Vec::new()),
        EffectAst::Detain { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect =
                tag_object_target_effect(Effect::detain(spec.clone()), &spec, ctx, "detained");
            (vec![effect], choices)
        }
        EffectAst::Goad { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect = tag_object_target_effect(Effect::goad(spec.clone()), &spec, ctx, "goaded");
            (vec![effect], choices)
        }
        EffectAst::ReturnToHand { target, random } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let from_graveyard = target_mentions_graveyard(target);
            let effect = tag_object_target_effect(
                if from_graveyard {
                    Effect::return_from_graveyard_to_hand_with_random(spec.clone(), *random)
                } else {
                    Effect::new(crate::effects::ReturnToHandEffect::with_spec(spec.clone()))
                },
                &spec,
                ctx,
                "returned",
            );
            ctx.last_player_filter = Some(if spec.is_target() {
                PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
            } else if let Some(tag) = ctx.last_object_tag.clone() {
                PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(TagKey::from(tag.as_str())))
            } else {
                PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
            });
            (vec![effect], choices)
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            transformed,
            converted,
            controller,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let from_exile_tag = choose_spec_references_exiled_tag(&spec);
            let use_move_to_zone =
                from_exile_tag || !matches!(controller, ReturnControllerAst::Preserve);
            let mut effects = Vec::new();
            let resolved_spec = if !spec.is_target() {
                match &spec {
                    ChooseSpec::Object(filter)
                        if filter.tagged_constraints.is_empty()
                            && filter.zone == Some(Zone::Graveyard) =>
                    {
                        let tag = ctx.next_tag("chosen_return");
                        ctx.last_object_tag = Some(tag.clone());
                        effects.push(Effect::choose_objects(
                            filter.clone(),
                            1usize,
                            PlayerFilter::You,
                            tag.clone(),
                        ));
                        ChooseSpec::tagged(tag)
                    }
                    ChooseSpec::WithCount(inner, count)
                        if count.is_single()
                            && matches!(inner.base(), ChooseSpec::Object(filter) if filter.tagged_constraints.is_empty() && filter.zone == Some(Zone::Graveyard)) =>
                    {
                        let ChooseSpec::Object(filter) = inner.base() else {
                            unreachable!("guard ensures graveyard object base")
                        };
                        let tag = ctx.next_tag("chosen_return");
                        ctx.last_object_tag = Some(tag.clone());
                        effects.push(Effect::choose_objects(
                            filter.clone(),
                            count.clone(),
                            PlayerFilter::You,
                            tag.clone(),
                        ));
                        ChooseSpec::tagged(tag)
                    }
                    _ => spec.clone(),
                }
            } else {
                spec.clone()
            };

            let mut effect = tag_object_target_effect(
                if use_move_to_zone {
                    // Blink-style "exile ... then return it" should move the tagged
                    // exiled object back to the battlefield from exile. We also use
                    // MoveToZone for explicit controller overrides so "under your control"
                    // semantics are preserved for tagged references like "that card".
                    let move_back = crate::effects::MoveToZoneEffect::new(
                        resolved_spec.clone(),
                        Zone::Battlefield,
                        false,
                    );
                    let move_back = if *tapped {
                        move_back.tapped()
                    } else {
                        move_back
                    };
                    let move_back = match controller {
                        ReturnControllerAst::Preserve => move_back,
                        ReturnControllerAst::Owner => move_back.under_owner_control(),
                        ReturnControllerAst::You => move_back.under_you_control(),
                    };
                    Effect::new(move_back)
                } else {
                    Effect::return_from_graveyard_to_battlefield(resolved_spec.clone(), *tapped)
                },
                &resolved_spec,
                ctx,
                "returned",
            );
            if ctx.auto_tag_object_targets
                && !resolved_spec.is_target()
                && choose_spec_targets_object(&resolved_spec)
            {
                let tag = ctx.next_tag("returned");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            effects.push(effect);
            if *transformed {
                let transform_spec = if let Some(tag) = ctx.last_object_tag.clone() {
                    ChooseSpec::tagged(tag)
                } else {
                    resolved_spec.clone()
                };
                effects.push(Effect::transform(transform_spec));
            }
            if *converted {
                let convert_spec = if let Some(tag) = ctx.last_object_tag.clone() {
                    ChooseSpec::tagged(tag)
                } else {
                    resolved_spec.clone()
                };
                effects.push(Effect::convert(convert_spec));
            }
            (effects, choices)
        }
        EffectAst::MoveToLibraryNthFromTop { target, position } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = Effect::new(crate::effects::MoveToLibraryNthFromTopEffect::new(
                spec.clone(),
                position.clone(),
            ));
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::MoveToZone {
            target,
            zone,
            to_top,
            battlefield_controller,
            battlefield_tapped,
            attached_to,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let resolved_attach_spec = if let Some(attach_target) = attached_to {
                if *zone != Zone::Battlefield {
                    return Err(CardTextError::ParseError(
                        "attached battlefield destination requires zone battlefield".to_string(),
                    ));
                }
                let (attach_spec, attach_choices) =
                    resolve_target_spec_with_choices(attach_target, &current_reference_env(ctx))?;
                for choice in attach_choices {
                    push_choice(&mut choices, choice);
                }
                Some(attach_spec)
            } else {
                None
            };
            if resolved_attach_spec.is_none()
                && *zone == Zone::Battlefield
                && let ChooseSpec::WithCount(inner, count) = &spec
                && !inner.is_target()
                && let ChooseSpec::Object(filter) = inner.base()
                && filter.zone == Some(Zone::Hand)
            {
                let chooser = filter
                    .owner
                    .clone()
                    .or_else(|| filter.controller.clone())
                    .unwrap_or(PlayerFilter::You);
                let chosen_tag = ctx.next_tag("chosen");
                let choose = Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        filter.clone(),
                        count.clone(),
                        chooser,
                        chosen_tag.clone(),
                    )
                    .in_zone(Zone::Hand)
                    .replace_tagged_objects(),
                );
                let spec = ChooseSpec::tagged(chosen_tag);
                let move_effect =
                    crate::effects::MoveToZoneEffect::new(spec.clone(), *zone, *to_top);
                let move_effect = if *zone == Zone::Battlefield && *battlefield_tapped {
                    move_effect.tapped()
                } else {
                    move_effect
                };
                let move_effect = match battlefield_controller {
                    ReturnControllerAst::Preserve => move_effect,
                    ReturnControllerAst::Owner => move_effect.under_owner_control(),
                    ReturnControllerAst::You => move_effect.under_you_control(),
                };
                let mut effect = Effect::new(move_effect);
                if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("moved");
                    ctx.last_object_tag = Some(tag.clone());
                    effect = effect.tag(tag);
                }
                return Ok(Some((vec![choose, effect], choices)));
            }
            let move_effect = crate::effects::MoveToZoneEffect::new(spec.clone(), *zone, *to_top);
            let move_effect = if *zone == Zone::Battlefield && *battlefield_tapped {
                move_effect.tapped()
            } else {
                move_effect
            };
            let move_effect = match battlefield_controller {
                ReturnControllerAst::Preserve => move_effect,
                ReturnControllerAst::Owner => move_effect.under_owner_control(),
                ReturnControllerAst::You => move_effect.under_you_control(),
            };
            let mut effect = Effect::new(move_effect);
            let mut moved_tag: Option<String> = None;
            let should_tag = choose_spec_targets_object(&spec)
                && (ctx.auto_tag_object_targets || attached_to.is_some());
            if should_tag {
                let tag = ctx.next_tag("moved");
                moved_tag = Some(tag.clone());
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }

            if let Some(attach_spec) = resolved_attach_spec {
                let moved_tag = moved_tag.ok_or_else(|| {
                    CardTextError::ParseError(
                        "attached battlefield destination requires object-tagged move source"
                            .to_string(),
                    )
                })?;
                let moved_objects =
                    ChooseSpec::All(ObjectFilter::tagged(TagKey::from(moved_tag.as_str())));
                return Ok(Some((
                    vec![effect, Effect::attach_objects(moved_objects, attach_spec)],
                    choices,
                )));
            }

            (vec![effect], choices)
        }
        EffectAst::ShuffleObjectsIntoLibrary { target, player } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (player_filter, player_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            for choice in player_choices {
                push_choice(&mut choices, choice);
            }
            let mut effect = Effect::shuffle_objects_into_library(spec.clone(), player_filter);
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::ReturnAllToHand { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            (
                vec![Effect::return_all_to_hand(resolved_filter)],
                Vec::new(),
            )
        }
        EffectAst::ReturnAllToHandOfChosenColor { filter } => {
            use crate::effect::EffectMode;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];
            for (_name, color) in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = format!(
                    "Return all {} to their owners' hands.",
                    filter.description()
                );
                modes.push(EffectMode {
                    description,
                    effects: vec![Effect::return_all_to_hand(filter)],
                });
            }
            prelude.push(Effect::choose_one(modes));
            (prelude, choices)
        }
        EffectAst::ReturnAllToBattlefield { filter, tapped } => {
            let mut effect = Effect::new(crate::effects::ReturnAllToBattlefieldEffect::new(
                resolve_it_tag(filter, &current_reference_env(ctx))?,
                *tapped,
            ));
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("returned");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            (vec![effect], Vec::new())
        }
        EffectAst::ExchangeControl {
            filter,
            count,
            shared_type,
        } => {
            let targets = ChooseSpec::target(ChooseSpec::Object(filter.clone()))
                .with_count(ChoiceCount::exactly(*count as usize));
            let exchange = crate::effects::ExchangeControlEffect::new(targets.clone(), targets);
            let exchange = if let Some(shared_type) = shared_type {
                let constraint = match shared_type {
                    SharedTypeConstraintAst::CardType => {
                        crate::effects::SharedTypeConstraint::CardType
                    }
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
            (vec![effect], Vec::new())
        }
        EffectAst::ExchangeControlHeterogeneous {
            permanent1,
            permanent2,
            shared_type,
        } => compile_exchange_control_heterogeneous_effect(
            permanent1,
            permanent2,
            *shared_type,
            ctx,
        )?,
        EffectAst::ExchangeTextBoxes { target } => compile_exchange_text_boxes_effect(target, ctx)?,
        EffectAst::ExchangeZones {
            player,
            zone1,
            zone2,
        } => compile_exchange_zones_effect(*player, *zone1, *zone2, ctx)?,
        EffectAst::ExchangeValues {
            left,
            right,
            duration,
        } => compile_exchange_values_effect(left, right, duration.clone(), ctx)?,
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_player_turn_and_counter_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::RingTemptsYou { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::ring_tempts_player)?
        }
        EffectAst::VentureIntoDungeon {
            player,
            undercity_if_no_active,
        } => compile_player_effect_from_filter(*player, ctx, true, |filter| {
            if *undercity_if_no_active {
                Effect::venture_into_undercity_player(filter)
            } else {
                Effect::venture_into_dungeon_player(filter)
            }
        })?,
        EffectAst::BecomeMonarch { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::become_monarch_player)?
        }
        EffectAst::TakeInitiative { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::take_initiative_player)?
        }
        EffectAst::DoubleManaPool { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::double_mana_pool_player)?
        }
        EffectAst::ExchangeLifeTotals { player1, player2 } => {
            compile_exchange_life_totals_effect(*player1, *player2, ctx)?
        }
        EffectAst::SetLifeTotal { amount, player } => {
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                Effect::set_life_total_player(amount.clone(), filter)
            })?
        }
        EffectAst::SkipTurn { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::skip_turn_player)?
        }
        EffectAst::SkipCombatPhases { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_combat_phases_player,
        )?,
        EffectAst::SkipNextCombatPhaseThisTurn { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_next_combat_phase_this_turn_player,
        )?,
        EffectAst::SkipDrawStep { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::skip_draw_step_player)?
        }
        EffectAst::Regenerate { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::regenerate(spec.clone(), crate::effect::Until::EndOfTurn),
                &spec,
                ctx,
                "regenerated",
            );
            (vec![effect], choices)
        }
        EffectAst::RegenerateAll { filter } => {
            let (mut prelude, choices) = target_context_prelude_for_filter(filter);
            prelude.push(Effect::regenerate(
                ChooseSpec::all(filter.clone()),
                crate::effect::Until::EndOfTurn,
            ));
            (prelude, choices)
        }
        EffectAst::Mill { count, player } => compile_player_effect_with_generated_object_tag(
            *player,
            ctx,
            true,
            "milled",
            || Effect::mill(count.clone()),
            |filter| Effect::mill_player(count.clone(), filter),
        )?,
        EffectAst::PoisonCounters { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::poison_counters(count.clone()),
            |filter| Effect::poison_counters_player(count.clone(), filter),
        )?,
        EffectAst::EnergyCounters { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::energy_counters(count.clone()),
            |filter| Effect::energy_counters_player(count.clone(), filter),
        )?,
        _ => return Ok(None),
    };

    Ok(Some(compiled))
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

pub(crate) fn parse_equipment_rules_text(words: &[&str]) -> Option<String> {
    let has_equipped_subject = words
        .iter()
        .enumerate()
        .any(|(idx, _)| idx + 2 <= words.len() && words[idx..idx + 2] == ["equipped", "creature"]);
    if !has_equipped_subject {
        return None;
    }

    let mut lines = Vec::new();
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

    if let Some(equip_amount) = parse_equip_amount(words) {
        lines.push(format!("Equip {{{equip_amount}}}"));
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
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
        let token_name = find_index(words.as_slice(), |word| {
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
        if let Some(rules_text) = parse_equipment_rules_text(&words)
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
                    trigger: Trigger::enters_battlefield(ObjectFilter::land().you_control()),
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
            crate::cards::builders::parser::token_primitives::iter_contains(
                &for_each.filter.card_types,
                &CardType::Creature,
            )
        );
        assert!(
            crate::cards::builders::parser::token_primitives::iter_contains(
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
