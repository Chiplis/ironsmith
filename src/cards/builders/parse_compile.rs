use super::effect_ast_traversal::{
    assert_effect_ast_variant_coverage, for_each_nested_effects, for_each_nested_effects_mut,
};
#[allow(unused_imports)]
use crate::ability;
#[allow(unused_imports)]
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, TriggeredAbility};
#[allow(unused_imports)]
use crate::card::PowerToughness;
#[allow(unused_imports)]
use crate::cards::CardDefinition;
#[allow(unused_imports)]
use crate::cards::builders::ability_lowering::lower_parsed_ability;
#[allow(unused_imports)]
use crate::cards::builders::effect_pipeline::{
    EffectPreludeTag, PreparedEffectsForLowering, PreparedPredicateForLowering,
    PreparedTriggeredEffectsForLowering, prepare_effects_for_lowering,
    prepare_effects_with_trigger_context_for_lowering, prepare_triggered_effects_for_lowering,
};
#[allow(unused_imports)]
use crate::cards::builders::parse_parsing::{
    EffectLoweringContext, IdGenContext, LoweringFrame, NormalizedLine, contains_until_end_of_turn,
    map_span_to_original, parse_card_type, parse_number_word_i32, parse_subtype_word,
};
#[allow(unused_imports)]
use crate::cards::builders::reference_resolution::{
    EffectReferenceResolutionConfig, annotate_effect_sequence,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    AnnotatedEffect, AnnotatedEffectSequence, CardDefinitionBuilder, CardTextError,
    ClashOpponentAst, ControlDurationAst, DamageBySpec, EffectAst, ExtraTurnAnchorAst,
    GrantedAbilityAst, IT_TAG, IfResultPredicate, LineAst, LoweredEffects, NewTargetRestrictionAst,
    ObjectRefAst, ParseAnnotations, PlayerAst, PredicateAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, ReferenceEnv, ReferenceExports, ReferenceImports,
    RetargetModeAst, ReturnControllerAst, SharedTypeConstraintAst, TagKey, TargetAst, TriggerSpec,
    choose_spec_targets_object, infer_player_filter_from_object_filter,
    object_filter_as_tagged_reference, resolve_attach_object_spec, resolve_choose_spec_it_tag,
    resolve_it_tag, resolve_it_tag_key, resolve_non_target_player_filter,
    resolve_restriction_it_tag, resolve_target_spec_with_choices, resolve_unless_player_filter,
    resolve_value_it_tag, watch_tag_from_filter,
};
#[allow(unused_imports)]
use crate::color::ColorSet;
#[allow(unused_imports)]
use crate::cost::TotalCost;
#[allow(unused_imports)]
use crate::effect::{
    ChoiceCount, Condition, Effect, EffectId, EffectMode, EffectPredicate, EventValueSpec, Until,
    Value,
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

pub(crate) fn compile_trigger_spec(trigger: TriggerSpec) -> Trigger {
    match trigger {
        TriggerSpec::ThisAttacks => Trigger::this_attacks(),
        TriggerSpec::ThisAttacksAndIsntBlocked => Trigger::this_attacks_and_isnt_blocked(),
        TriggerSpec::ThisAttacksWhileSaddled => Trigger::this_attacks_while_saddled(),
        TriggerSpec::ThisAttacksWithNOthers(other_count) => {
            Trigger::this_attacks_with_n_others(other_count as usize)
        }
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
        TriggerSpec::BecomesBlocked(filter) => Trigger::becomes_blocked(filter),
        TriggerSpec::BlocksOrBecomesBlocked(filter) => Trigger::blocks_or_becomes_blocked(filter),
        TriggerSpec::ThisBlocksOrBecomesBlocked => Trigger::this_blocks_or_becomes_blocked(),
        TriggerSpec::ThisDies => Trigger::this_dies(),
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
        TriggerSpec::PlayerTapsForMana { player, filter } => {
            Trigger::player_taps_for_mana(player, filter)
        }
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
        TriggerSpec::PlayerDrawsNthCardEachTurn {
            player,
            card_number,
        } => Trigger::player_draws_nth_card_each_turn(player, card_number),
        TriggerSpec::PlayerDiscardsCard { player, filter } => {
            Trigger::player_discards_card(player, filter)
        }
        TriggerSpec::PlayerSacrifices { player, filter } => {
            Trigger::player_sacrifices(player, filter)
        }
        TriggerSpec::Dies(filter) => Trigger::dies(filter),
        TriggerSpec::PutIntoGraveyard(filter) => Trigger::put_into_graveyard(filter),
        TriggerSpec::CardsLeaveYourGraveyard {
            filter,
            one_or_more,
            during_your_turn,
        } => Trigger::cards_leave_your_graveyard(filter, one_or_more, during_your_turn),
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
        TriggerSpec::EntersBattlefieldTapped(filter) => Trigger::enters_battlefield_tapped(filter),
        TriggerSpec::EntersBattlefieldUntapped(filter) => {
            Trigger::enters_battlefield_untapped(filter)
        }
        TriggerSpec::BeginningOfUpkeep(player) => Trigger::beginning_of_upkeep(player),
        TriggerSpec::BeginningOfDrawStep(player) => Trigger::beginning_of_draw_step(player),
        TriggerSpec::BeginningOfCombat(player) => Trigger::beginning_of_combat(player),
        TriggerSpec::EndOfCombat => Trigger::end_of_combat(),
        TriggerSpec::BeginningOfEndStep(player) => Trigger::beginning_of_end_step(player),
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerSpec::ThisEntersBattlefield => Trigger::this_enters_battlefield(),
        TriggerSpec::ThisDealsCombatDamageToPlayer => Trigger::this_deals_combat_damage_to_player(),
        TriggerSpec::DealsCombatDamageToPlayer { source, player } => {
            Trigger::deals_combat_damage_to_player(source, player)
        }
        TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player } => {
            Trigger::deals_combat_damage_to_player_one_or_more(source, player)
        }
        TriggerSpec::YouCastThisSpell => Trigger::you_cast_this_spell(),
        TriggerSpec::KeywordAction { action, player } => Trigger::keyword_action(action, player),
        TriggerSpec::KeywordActionFromSource { action, player } => {
            Trigger::keyword_action_from_source(action, player)
        }
        TriggerSpec::Expend { player, amount } => Trigger::expend(amount, player),
        TriggerSpec::Custom(description) => {
            debug_assert!(
                false,
                "unsupported custom trigger spec compiled: {description}"
            );
            Trigger::custom("unsupported_custom_trigger", description)
        }
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
        TriggerSpec::Custom(description) => Err(CardTextError::ParseError(format!(
            "unsupported trigger clause (clause: '{}')",
            description
        ))),
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
    Ok(compile_statement_effects_with_imports(effects, &ReferenceImports::default())?.effects)
}

pub(crate) fn compile_statement_effects_with_imports(
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared = prepare_effects_for_lowering(effects, imports.clone())?;
    materialize_prepared_statement_effects(&prepared)
}

pub(crate) fn materialize_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, _) = compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    Ok(LoweredEffects {
        effects: prepend_effect_prelude(compiled, compile_effect_prelude_tags(&prepared.prelude)),
        choices: Vec::new(),
        exports: ReferenceExports::from_env(&ctx.reference_env()),
    })
}

pub(crate) fn materialize_prepared_effects_with_trigger_context(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, choices) =
        compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    Ok(LoweredEffects {
        effects: prepend_effect_prelude(compiled, compile_effect_prelude_tags(&prepared.prelude)),
        choices,
        exports: ReferenceExports::from_env(&ctx.reference_env()),
    })
}

pub(crate) fn materialize_prepared_triggered_effects(
    prepared: &PreparedTriggeredEffectsForLowering,
) -> Result<(LoweredEffects, Option<Condition>), CardTextError> {
    let lowered = materialize_prepared_effects_with_trigger_context(&prepared.prepared)?;
    let intervening_if = prepared
        .intervening_if
        .as_ref()
        .map(compile_prepared_predicate_for_lowering)
        .transpose()?;
    Ok((lowered, intervening_if))
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
    ctx.apply_reference_env(refs);
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
        TriggerSpec::SpellCast { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::SpellCopied { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLife(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLifeDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCard(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsNthCardEachTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDiscardsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerTapsForMana { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerSacrifices { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::ThisDealsDamageToPlayer { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::KeywordAction { player, .. }
        | TriggerSpec::KeywordActionFromSource { player, .. } => {
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
        TriggerSpec::Custom(_) => None,
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
            | TriggerSpec::CounterPutOn { .. } => true,
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
    Ok((lowered.effects, lowered.choices))
}

pub(crate) fn compile_trigger_effects_with_imports(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared =
        prepare_effects_with_trigger_context_for_lowering(trigger, effects, imports.clone())?;
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
            let mut resolved = filter.clone();
            resolved.zone = None;
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::TaggedObjectMatches(resolved_tag, resolved)
        }
        PredicateAst::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::PlayerTaggedObjectMatches {
                player,
                tag: resolved_tag,
                filter: resolved,
            }
        }
        PredicateAst::PlayerControls { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControls {
                player,
                filter: resolved,
            }
        }
        PredicateAst::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
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
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
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
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
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
        PredicateAst::PlayerHasLessLifeThanYou { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasLessLifeThanYou { player }
        }
        PredicateAst::PlayerHasCitysBlessing { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasCitysBlessing { player }
        }
        PredicateAst::PlayerTappedLandForManaThisTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerTappedLandForManaThisTurn { player }
        }
        PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHadLandEnterBattlefieldThisTurn { player }
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
        PredicateAst::YouHaveNoCardsInHand => {
            Condition::Not(Box::new(Condition::CardsInHandOrMore(1)))
        }
        PredicateAst::YourTurn => Condition::YourTurn,
        PredicateAst::CreatureDiedThisTurn => Condition::CreatureDiedThisTurn,
        PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn => {
            Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        }
        PredicateAst::SourceIsTapped => Condition::SourceIsTapped,
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
        PredicateAst::YouAttackedThisTurn => Condition::AttackedThisTurn,
        PredicateAst::SourceWasCast => Condition::SourceWasCast,
        PredicateAst::NoSpellsWereCastLastTurn => Condition::NoSpellsWereCastLastTurn,
        PredicateAst::ThisSpellWasKicked => Condition::ThisSpellWasKicked,
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
        PredicateAst::Unmodeled(text) => Condition::Unmodeled(text.clone()),
        PredicateAst::And(left, right) => {
            let left = compile_condition_from_predicate_ast(left, ctx, saved_last_tag)?;
            let right = compile_condition_from_predicate_ast(right, ctx, saved_last_tag)?;
            Condition::And(Box::new(left), Box::new(right))
        }
    })
}

pub(crate) fn compile_trigger_effects_with_intervening_if(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
) -> Result<(Vec<Effect>, Vec<ChooseSpec>, Option<Condition>), CardTextError> {
    compile_trigger_effects_with_intervening_if_imports(
        trigger,
        effects,
        &ReferenceImports::default(),
    )
}

pub(crate) fn compile_trigger_effects_with_intervening_if_imports(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>, Option<Condition>), CardTextError> {
    let prepared = if let Some(trigger) = trigger {
        prepare_triggered_effects_for_lowering(trigger, effects, imports.clone())?
    } else {
        PreparedTriggeredEffectsForLowering {
            prepared: prepare_effects_with_trigger_context_for_lowering(
                None,
                effects,
                imports.clone(),
            )?,
            intervening_if: None,
        }
    };
    let (lowered, intervening_if) = materialize_prepared_triggered_effects(&prepared)?;
    Ok((lowered.effects, lowered.choices, intervening_if))
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
            | EffectAst::MoveToLibrarySecondFromTop { target: $target }
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
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::RemoveAbilitiesAll { filter, .. }
        | EffectAst::GrantAbilitiesChoiceAll { filter, .. }
        | EffectAst::Enchant { filter }
        | EffectAst::SearchLibrary { filter, .. }
        | EffectAst::DestroyAllAttachedTo { filter, .. } => Some(filter),
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
        } => {
            matches!(predicate, PredicateAst::TaggedMatches(t, _) if t.as_str() == tag)
                || matches!(predicate, PredicateAst::PlayerTaggedObjectMatches { tag: t, .. } if t.as_str() == tag)
                || effects_reference_tag(if_true, tag)
                || effects_reference_tag(if_false, tag)
        }
        EffectAst::RetargetStackObject {
            new_target_restriction,
            ..
        } => {
            if let Some(NewTargetRestrictionAst::Object(filter)) = new_target_restriction {
                filter_references_tag(filter, tag)
            } else {
                false
            }
        }
        EffectAst::PutIntoHand { object, .. } => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::CreateTokenCopy { object, .. } => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::CreateToken { count, .. } | EffectAst::CreateTokenWithMods { count, .. } => {
            value_references_tag(count, tag)
        }
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
        Value::Count(filter) | Value::CountScaled(filter, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestManaValue(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::PowerOf(spec) | Value::ToughnessOf(spec) => choose_spec_references_tag(spec, tag),
        Value::ManaValueOf(spec) => choose_spec_references_tag(spec, tag),
        Value::CountersOn(spec, _) => choose_spec_references_tag(spec, tag),
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
    match spec {
        ChooseSpec::Tagged(tag) => tag.as_str().starts_with("exiled_"),
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_exiled_tag(inner)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                    && constraint.tag.as_str().starts_with("exiled_")
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
        PlayerFilter::ControllerOf(reference) | PlayerFilter::OwnerOf(reference) => {
            object_ref_references_tag(reference, tag)
        }
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
        TargetAst::Source(_) | TargetAst::AnyTarget(_) | TargetAst::Spell(_) => false,
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
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::LookAtTopCards { count: amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::CreateToken { count: amount, .. }
        | EffectAst::Investigate { count: amount }
        | EffectAst::CreateTokenCopy { count: amount, .. }
        | EffectAst::CreateTokenCopyFromSource { count: amount, .. }
        | EffectAst::CreateTokenWithMods { count: amount, .. }
        | EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            value_references_event_derived_amount(amount)
        }
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
        EffectAst::SetBasePower { power, .. } => value_references_event_derived_amount(power),
        EffectAst::PumpForEach { count, .. } => value_references_event_derived_amount(count),
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
        | EffectAst::Surveil { player, .. }
        | EffectAst::PlayFromGraveyardUntilEot { player }
        | EffectAst::GrantPlayTaggedUntilEndOfTurn { player, .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            player,
            ..
        }
        | EffectAst::ExileInsteadOfGraveyardThisTurn { player }
        | EffectAst::ExtraTurnAfterTurn { player, .. }
        | EffectAst::RevealTop { player }
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
        | EffectAst::SetLifeTotal { player, .. }
        | EffectAst::SkipTurn { player }
        | EffectAst::SkipCombatPhases { player }
        | EffectAst::SkipNextCombatPhaseThisTurn { player }
        | EffectAst::SkipDrawStep { player }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::CreateToken { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. }
        | EffectAst::SearchLibrary { player, .. }
        | EffectAst::ShuffleGraveyardIntoLibrary { player }
        | EffectAst::ShuffleLibrary { player }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::SacrificeAll { player, .. }
        | EffectAst::ChooseObjects { player, .. } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
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
        EffectAst::LoseLife { amount, .. } | EffectAst::GainLife { amount, .. } => {
            value_references_tag(amount, IT_TAG)
        }
        EffectAst::PreventDamage { amount, .. } => value_references_tag(amount, IT_TAG),
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
        } => {
            matches!(
                predicate,
                PredicateAst::ItIsLandCard | PredicateAst::ItMatches(_)
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
        EffectAst::PutRestOnBottomOfLibrary => true,
        EffectAst::RetargetStackObject {
            new_target_restriction,
            ..
        } => {
            if let Some(NewTargetRestrictionAst::Object(filter)) = new_target_restriction {
                filter_references_tag(filter, IT_TAG)
            } else {
                false
            }
        }
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
        EffectAst::CreateToken { count, .. } | EffectAst::CreateTokenWithMods { count, .. } => {
            value_references_tag(count, IT_TAG)
        }
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
        ctx.apply_reference_env(&current.in_env);
        ctx.auto_tag_object_targets =
            ctx.force_auto_tag_object_targets || current.auto_tag_object_targets;

        if let Some((effect_sequence, effect_choices, consumed)) =
            compile_vote_sequence(&annotated.effects[idx..], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            ctx.apply_reference_env(&annotated.effects[idx + consumed - 1].out_env);
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
            ctx.apply_reference_env(&annotated.effects[idx + 1].out_env);
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
            ctx.apply_reference_env(&annotated.effects[idx + 1].out_env);
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
            ctx.apply_reference_env(&annotated.effects[idx + 1].out_env);
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
            ctx.apply_reference_env(&annotated.effects[idx + 1].out_env);
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

pub(crate) fn collect_tag_spans_from_line(
    line: &LineAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    match line {
        LineAst::Triggered { effects, .. }
        | LineAst::Statement { effects }
        | LineAst::AdditionalCost { effects } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
        }
        LineAst::AdditionalCostChoice { options } => {
            for option in options {
                collect_tag_spans_from_effects_with_context(&option.effects, annotations, ctx);
            }
        }
        LineAst::AlternativeCastingMethod(_)
        | LineAst::OptionalCost(_)
        | LineAst::StaticAbility(_)
        | LineAst::StaticAbilities(_)
        | LineAst::Ability(_)
        | LineAst::Abilities(_) => {}
    }
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

pub(crate) fn compile_if_do_with_opponent_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDoesNot {
        effects: second_effects,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachOpponent {
        effects: opponent_effects,
    } = first
    {
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
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
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

pub(crate) fn compile_effects_in_iterated_player_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
    tagged_object: Option<String>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut iterated_frame = saved_frame.clone();
    iterated_frame.iterated_player = true;
    iterated_frame.last_effect_id = None;
    if let Some(tag) = tagged_object.clone() {
        iterated_frame.last_object_tag = Some(tag);
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
            EffectAst::CreateToken { player, .. }
            | EffectAst::CreateTokenWithMods { player, .. }
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

pub(crate) fn compile_vote_sequence(
    effects: &[AnnotatedEffect],
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>, usize)>, CardTextError> {
    let Some(first) = effects.first() else {
        return Ok(None);
    };
    let EffectAst::VoteStart { options } = &first.effect else {
        return Ok(None);
    };

    let mut option_effects: HashMap<String, Vec<EffectAst>> = HashMap::new();
    let mut extra_mandatory: u32 = 0;
    let mut extra_optional: u32 = 0;
    let mut consumed = 1;

    while consumed < effects.len() {
        match &effects[consumed].effect {
            EffectAst::VoteOption { option, effects } => {
                if option_effects
                    .insert(option.clone(), effects.clone())
                    .is_some()
                {
                    return Err(CardTextError::ParseError(format!(
                        "duplicate vote option clause for '{option}'"
                    )));
                }
                consumed += 1;
            }
            EffectAst::VoteExtra { count, optional } => {
                if *optional {
                    extra_optional = extra_optional.saturating_add(*count);
                } else {
                    extra_mandatory = extra_mandatory.saturating_add(*count);
                }
                consumed += 1;
            }
            _ => break,
        }
    }

    let (vote_options, choices) = with_preserved_lowering_context(
        ctx,
        |ctx| {
            ctx.iterated_player = true;
        },
        |ctx| {
            let mut vote_options = Vec::new();
            let mut choices = Vec::new();
            for option in options {
                let mut option_effects_ast =
                    option_effects.get(option).cloned().ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "missing effects for vote option '{option}'"
                        ))
                    })?;
                force_implicit_vote_token_controller_you(&mut option_effects_ast);
                ctx.last_effect_id = None;
                ctx.last_object_tag = None;
                ctx.last_player_filter = None;
                let (compiled, option_choices) = compile_effects(&option_effects_ast, ctx)?;
                for choice in option_choices {
                    push_choice(&mut choices, choice);
                }
                vote_options.push(VoteOption::new(option.clone(), compiled));
            }
            Ok((vote_options, choices))
        },
    )?;

    let effect = if extra_optional > 0 {
        Effect::vote_with_optional_extra(vote_options, extra_mandatory, extra_optional)
    } else {
        Effect::vote(vote_options, extra_mandatory)
    };

    Ok(Some((vec![effect], choices, consumed)))
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
    let choice_zone = resolved_filter.zone.unwrap_or(Zone::Battlefield);
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
    let choice_zone = resolved_filter.zone.unwrap_or(Zone::Battlefield);
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

fn lower_granted_ability_ast(ability: &GrantedAbilityAst) -> Result<StaticAbility, CardTextError> {
    match ability {
        GrantedAbilityAst::Static(ability) => Ok(ability.clone()),
        GrantedAbilityAst::ParsedObjectAbility { ability, display } => {
            let mut lowered = lower_parsed_ability(ability.clone())?.ability;
            if lowered.text.is_none() {
                lowered.text = Some(display.clone());
            }
            Ok(StaticAbility::grant_object_ability_for_filter(
                ObjectFilter::source(),
                lowered,
                display.clone(),
            ))
        }
    }
}

fn lower_granted_abilities_ast(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<StaticAbility>, CardTextError> {
    abilities.iter().map(lower_granted_ability_ast).collect()
}

fn compile_effect_inner(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
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

fn current_reference_env(ctx: &EffectLoweringContext) -> ReferenceEnv {
    ctx.reference_env()
}

fn try_compile_combat_and_damage_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::DealDamage { amount, target } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let (effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    Effect::deal_damage(resolved_amount.clone(), spec)
                })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            (effects, choices)
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
            let mut amount_source_spec = source_spec.clone();

            if source_spec.is_target() {
                let source_tag = ctx.next_tag("damage_source");
                effects.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(source_spec.clone()))
                        .tag(source_tag.clone()),
                );
                amount_source_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                if source == target {
                    damage_target_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                }
            }

            let damage_effect = tag_object_target_effect(
                Effect::deal_damage(
                    Value::PowerOf(Box::new(amount_source_spec)),
                    damage_target_spec.clone(),
                ),
                &damage_target_spec,
                ctx,
                "damaged",
            );
            effects.push(damage_effect);

            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
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
        EffectAst::Proliferate => (vec![Effect::proliferate()], Vec::new()),
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
        EffectAst::Explore { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect =
                tag_object_target_effect(Effect::explore(spec.clone()), &spec, ctx, "explored");
            (vec![effect], choices)
        }
        EffectAst::OpenAttraction => (vec![Effect::open_attraction()], Vec::new()),
        EffectAst::ManifestDread => (vec![Effect::manifest_dread()], Vec::new()),
        EffectAst::Bolster { amount } => (vec![Effect::bolster(*amount)], Vec::new()),
        EffectAst::Support { amount } => (vec![Effect::support(*amount)], Vec::new()),
        EffectAst::Adapt { amount } => (vec![Effect::adapt(*amount)], Vec::new()),
        EffectAst::CounterActivatedOrTriggeredAbility => (
            vec![Effect::counter_activated_or_triggered_ability()],
            Vec::new(),
        ),
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
            false,
            || Effect::add_mana(mana.clone()),
            |filter| Effect::add_mana_player(mana.clone(), filter),
        )?,
        EffectAst::AddManaScaled {
            mana,
            amount,
            player,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
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
                false,
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
                false,
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
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
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
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
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
                false,
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
        EffectAst::Discover { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::discover(count.clone()),
            |filter| Effect::discover_player(count.clone(), filter),
        )?,
        EffectAst::BecomeBasicLandTypeChoice { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "become_basic_land_type", |spec| {
                Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::new(
                    spec,
                    duration.clone(),
                ))
            })?
        }
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
        } => {
            let restriction = resolve_restriction_it_tag(restriction, &current_reference_env(ctx))?;
            (
                vec![Effect::cant_until(restriction, duration.clone())],
                Vec::new(),
            )
        }
        EffectAst::PlayFromGraveyardUntilEot { player } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let effect = Effect::grant_play_from_graveyard_until_eot(player_filter);
            (vec![effect], Vec::new())
        }
        EffectAst::GrantPlayTaggedUntilEndOfTurn { tag, player } => {
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
                    crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                ))],
                Vec::new(),
            )
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
        EffectAst::GrantPlayTaggedUntilYourNextTurn { tag, player } => {
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
                    crate::effects::GrantPlayTaggedEffect::until_your_next_turn(
                        resolved_tag,
                        player_filter,
                    ),
                )],
                Vec::new(),
            )
        }
        EffectAst::CastTagged {
            tag,
            allow_land,
            as_copy,
            without_paying_mana_cost,
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
            );
            (vec![effect], Vec::new())
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
            if let TriggerSpec::IsDealtDamage(filter) = trigger {
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
            } else {
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
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-by-player effect branch is unsupported".to_string(),
                ));
            }
            let (player_filter, mut player_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = Effect::may_player(player_filter, inner_effects);
            let mut choices = inner_choices;
            choices.append(&mut player_choices);
            (vec![effect], choices)
        }
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
        EffectAst::MayByTaggedController { tag, effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty may-by-tagged-controller effect branch is unsupported".to_string(),
                ));
            }
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-by-tagged-controller effect branch is unsupported"
                        .to_string(),
                ));
            }
            let effect = Effect::for_each_controller_of_tagged(
                tag.clone(),
                vec![Effect::may(inner_effects)],
            );
            (vec![effect], inner_choices)
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
            let predicate = match predicate {
                IfResultPredicate::Did => EffectPredicate::Happened,
                IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
                IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
            };
            let effect = Effect::if_then(condition, predicate, inner_effects);
            (vec![effect], inner_choices)
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
            let effect = Effect::for_players(filter.clone(), inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachPlayer { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_players(PlayerFilter::Any, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTargetPlayers { count, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let target_spec =
                ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)).with_count(*count);
            let choose_targets =
                Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()));
            let effect = Effect::for_players(PlayerFilter::target_player(), inner_effects);
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
                ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "cannot resolve 'this way/it' tag without prior tagged object".to_string(),
                    )
                })?
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
            ctx.last_object_tag = Some(tag.as_str().to_string());
            let effect = Effect::look_at_top_cards(player_filter, count.clone(), tag.clone());
            (vec![effect], choices)
        }
        EffectAst::RevealHand { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let spec = if choices.is_empty() {
                ChooseSpec::Player(player_filter)
            } else {
                ChooseSpec::target_player()
            };
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
            let in_chosen = Condition::PlayerTaggedObjectMatches {
                player: PlayerFilter::IteratedPlayer,
                tag: chosen_tag_key,
                filter: membership_filter,
            };
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
            let in_chosen = Condition::PlayerTaggedObjectMatches {
                player: PlayerFilter::IteratedPlayer,
                tag: chosen_tag_key,
                filter: membership_filter,
            };
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
            let in_it = Condition::PlayerTaggedObjectMatches {
                player: PlayerFilter::IteratedPlayer,
                tag: TagKey::from(IT_TAG),
                filter: membership_filter,
            };
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
            let predicate = match predicate {
                IfResultPredicate::Did => EffectPredicate::Happened,
                IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
                IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
            };
            let effect = Effect::if_then(*condition, predicate, inner_effects);
            (vec![effect], inner_choices)
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
            new_target_restriction,
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

            if let Some(restriction) = new_target_restriction {
                let restriction = match restriction {
                    NewTargetRestrictionAst::Player(player) => {
                        let (filter, _) =
                            resolve_effect_player_filter(*player, ctx, false, false, false)?;
                        crate::effects::NewTargetRestriction::Player(filter)
                    }
                    NewTargetRestrictionAst::Object(filter) => {
                        let resolved = resolve_it_tag(filter, &current_reference_env(ctx))?;
                        crate::effects::NewTargetRestriction::Object(resolved)
                    }
                };
                effect = effect.with_restriction(restriction);
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
                PredicateAst::ItIsLandCard | PredicateAst::ItMatches(_)
            ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == IT_TAG)
                || matches!(
                    predicate,
                    PredicateAst::PlayerTaggedObjectMatches { tag, .. } if tag.as_str() == IT_TAG
                );

            let antecedent_choice = if saved_last_tag.is_none() && predicate_references_it {
                true_choices
                    .iter()
                    .chain(false_choices.iter())
                    .find(|choice| choice.is_target() && choose_spec_targets_object(choice))
                    .cloned()
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
            let spec = ChooseSpec::target(ChooseSpec::Object(filter.clone()));
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
        EffectAst::Investigate { count } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            (vec![Effect::investigate(count)], Vec::new())
        }
        EffectAst::Amass { subtype, amount } => {
            (vec![Effect::amass(*subtype, *amount)], Vec::new())
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
            let needs_created_tag = ctx.auto_tag_object_targets || attached_to.is_some();
            let mut created_tag: Option<String> = None;
            if needs_created_tag {
                let tag = ctx.next_tag("created");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag.clone());
                created_tag = Some(tag);
            }

            let mut compiled = vec![effect];
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
        EffectAst::CreateToken {
            name,
            count,
            player,
        } => {
            let token = token_definition_for(name.as_str())
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = if matches!(player_filter, PlayerFilter::You) {
                Effect::create_tokens(token, count.clone())
            } else {
                Effect::create_tokens_player(token, count, player_filter)
            };
            let mut effect = effect;
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
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
                && last_tag.starts_with("exile_cost_")
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
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "animated_creature", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
                    duration.clone(),
                )
                .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
                    power: power.clone(),
                    toughness: toughness.clone(),
                    sublayer: crate::continuous::PtSublayer::Setting,
                })
                .resolve_set_pt_values_at_resolution(),
            )
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
            let abilities = lower_granted_abilities_ast(abilities)?;
            if abilities.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesAll with no abilities"
                        .to_string(),
                ));
            }

            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                crate::continuous::Modification::AddAbility(abilities[0].clone()),
                duration.clone(),
            )
            .lock_filter_at_resolution();

            for ability in abilities.iter().skip(1) {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::AddAbility(ability.clone()),
                );
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
            let abilities = lower_granted_abilities_ast(abilities)?;
            if abilities.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesChoiceAll with no abilities"
                        .to_string(),
                ));
            }
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let modes = abilities
                .iter()
                .map(|ability| EffectMode {
                    description: String::new(),
                    effects: vec![Effect::grant_abilities_all(
                        resolved_filter.clone(),
                        vec![ability.clone()],
                        duration.clone(),
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
                    crate::continuous::Modification::AddAbility(first_ability.clone()),
                    duration.clone(),
                );

                for ability in abilities.iter().skip(1) {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::AddAbility(ability.clone()),
                    );
                }

                Effect::new(apply)
            })?
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
            let abilities = lower_granted_abilities_ast(abilities)?;
            if abilities.is_empty() {
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
                    .map(|ability| EffectMode {
                        description: if matches!(spec, ChooseSpec::Source) {
                            format!(
                                "This creature gains {} until end of turn.",
                                ability.display()
                            )
                        } else {
                            String::new()
                        },
                        effects: vec![Effect::new(
                            crate::effects::GrantAbilitiesTargetEffect::new(
                                spec.clone(),
                                vec![ability.clone()],
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
            player,
            reveal,
            shuffle,
            count,
            tapped,
        } => {
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
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
            let owner_matches_chooser = filter
                .owner
                .as_ref()
                .is_none_or(|owner| owner == &player_filter);
            let use_search_effect = *shuffle
                && count.min == 0
                && count.max == Some(1)
                && *destination != Zone::Battlefield
                && owner_matches_chooser;
            if use_search_effect {
                let mut effect =
                    Effect::search_library(filter, *destination, player_filter.clone(), *reveal);
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
                    player_filter.clone(),
                    tag.clone(),
                )
                .in_zone(Zone::Library)
                .with_description(choose_description)
                .as_search();
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
            compile_player_effect_from_filter(*player, ctx, true, Effect::shuffle_library_player)?
        }
        EffectAst::VoteStart { .. }
        | EffectAst::VoteOption { .. }
        | EffectAst::VoteExtra { .. } => {
            return Err(CardTextError::ParseError(
                "vote clauses must appear together".to_string(),
            ));
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
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
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            let choice_zone = resolved_filter.zone.unwrap_or(Zone::Battlefield);
            if choice_zone == Zone::Battlefield
                && resolved_filter.controller.is_none()
                && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
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
        } => {
            let (resolved_player, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_filter = if let Some(filter) = filter {
                let mut resolved = resolve_it_tag(filter, &current_reference_env(ctx))?;
                if resolved.zone.is_none() {
                    resolved.zone = Some(Zone::Hand);
                }
                if resolved.owner.is_none() {
                    resolved.owner = Some(resolved_player.clone());
                }
                Some(resolved)
            } else {
                None
            };
            let effect = Effect::discard_player_filtered(
                count.clone(),
                resolved_player,
                *random,
                resolved_filter,
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
            (vec![effect], choices)
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            controller,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let from_exile_tag = choose_spec_references_exiled_tag(&spec);
            let use_move_to_zone =
                from_exile_tag || !matches!(controller, ReturnControllerAst::Preserve);
            let mut effect = tag_object_target_effect(
                if use_move_to_zone {
                    // Blink-style "exile ... then return it" should move the tagged
                    // exiled object back to the battlefield from exile. We also use
                    // MoveToZone for explicit controller overrides so "under your control"
                    // semantics are preserved for tagged references like "that card".
                    let move_back = crate::effects::MoveToZoneEffect::new(
                        spec.clone(),
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
                    Effect::return_from_graveyard_to_battlefield(spec.clone(), *tapped)
                },
                &spec,
                ctx,
                "returned",
            );
            if ctx.auto_tag_object_targets && !spec.is_target() && choose_spec_targets_object(&spec)
            {
                let tag = ctx.next_tag("returned");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::MoveToLibrarySecondFromTop { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = Effect::new(crate::effects::MoveToLibrarySecondFromTopEffect::new(
                spec.clone(),
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
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn try_compile_player_turn_and_counter_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
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
        EffectAst::Mill { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
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
        kind: AbilityKind::Activated(ActivatedAbility::mana_with_cost_effects(
            TotalCost::free(),
            vec![Effect::sacrifice_source()],
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
    for window in words.windows(3) {
        if (window[0] == "deals" || window[0] == "deal") && window[2] == "damage" {
            return parse_number_word(window[1]);
        }
    }
    None
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
    let has_noncreature_cast_trigger = words
        .windows(6)
        .any(|window| window == ["whenever", "you", "cast", "a", "noncreature", "spell"])
        || words
            .windows(5)
            .any(|window| window == ["whenever", "you", "cast", "noncreature", "spell"]);
    if !has_noncreature_cast_trigger {
        return None;
    }
    let has_damage_subject = words.windows(3).any(|window| {
        window == ["this", "token", "deals"]
            || window == ["this", "creature", "deals"]
            || window == ["this", "token", "deal"]
            || window == ["this", "creature", "deal"]
    }) || words
        .windows(2)
        .any(|window| window == ["it", "deals"] || window == ["it", "deal"]);
    if !has_damage_subject {
        return None;
    }
    if !words
        .windows(3)
        .any(|window| window == ["to", "each", "opponent"])
    {
        return None;
    }
    parse_deals_damage_amount(&words)
}

pub(crate) fn parse_crew_amount(words: &[&str]) -> Option<u32> {
    let crew_idx = words.iter().position(|word| *word == "crew")?;
    let amount_word = words.get(crew_idx + 1)?;
    let amount = parse_number_word(amount_word)?;
    u32::try_from(amount).ok()
}

pub(crate) fn parse_equip_amount(words: &[&str]) -> Option<u32> {
    let equip_idx = words.iter().position(|word| *word == "equip")?;
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
        .windows(2)
        .any(|window| window == ["equipped", "creature"]);
    if !has_equipped_subject {
        return None;
    }

    let mut lines = Vec::new();
    let has_plus_one = words.windows(2).any(|window| window == ["gets", "+1/+1"]);
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
        if words.contains(&keyword) {
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
            effects: vec![Effect::deal_damage(Value::Fixed(amount), target.clone())],
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
            effects: vec![Effect::deal_damage(Value::Fixed(amount), target.clone())],
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
            effects: vec![Effect::deal_damage(Value::Fixed(amount), target.clone())],
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
            effects: vec![Effect::pump(-1, -1, target.clone(), Until::EndOfTurn)],
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
            mana_cost: ability::merge_cost_effects(
                TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Red]])),
                Vec::new(),
            ),
            effects: vec![Effect::pump(1, 0, ChooseSpec::Source, Until::EndOfTurn)],
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{R}: This creature gets +1/+0 until end of turn.".to_string()),
    }
}

pub(crate) fn token_white_tap_target_creature_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: ability::merge_cost_effects(
                TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::White]])),
                vec![Effect::tap_source()],
            ),
            effects: vec![Effect::tap(target.clone())],
            choices: vec![target],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{W}, {T}: Tap target creature.".to_string()),
    }
}

pub(crate) fn token_tap_add_single_mana_ability(symbol: ManaSymbol) -> Ability {
    let mana_text = ManaCost::from_pips(vec![vec![symbol]]).to_oracle();
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: crate::ability::merge_cost_effects(
                TotalCost::free(),
                vec![Effect::tap_source()],
            ),
            effects: vec![Effect::add_mana(vec![symbol])],
            choices: Vec::new(),
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!("{{T}}: Add {mana_text}.")),
    }
}

pub(crate) fn parse_token_tap_add_single_mana_symbol(words: &[&str]) -> Option<ManaSymbol> {
    let add_idx = words.iter().position(|word| *word == "add")?;
    if !words[..add_idx].contains(&"t") {
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
            effects: vec![Effect::poison_counters_player(
                1,
                PlayerFilter::DamagedPlayer,
            )],
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
            effects: vec![Effect::for_each_opponent(vec![Effect::deal_damage(
                Value::Fixed(amount),
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            )])],
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
            effects: vec![Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    target.clone(),
                    crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
                    Until::Forever,
                ),
            )],
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
            effects: vec![Effect::return_from_graveyard_to_hand(target.clone())],
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
            if idx > 0 && lowercase_words.contains(word) {
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
            let keep_lowercase = idx > 0 && lowercase_words.contains(&letters_only.as_str());
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
    let named_idx = words.iter().position(|word| *word == "named")?;
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
    while end < words.len() && !stop_words.contains(&words[end]) {
        end += 1;
    }
    if end <= named_idx + 1 {
        return None;
    }
    let name_word_count = end - (named_idx + 1);

    if let Some(named_pos) = source_text.find("named") {
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
        if stop_words.contains(word)
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
    let mut cost_effects = Vec::new();
    if tap_cost {
        cost_effects.push(Effect::tap_source());
    }
    cost_effects.push(Effect::sacrifice_source());
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
            mana_cost: ability::merge_cost_effects(TotalCost::mana(mana_cost), cost_effects),
            effects: vec![Effect::return_from_graveyard_to_battlefield(
                target.clone(),
                false,
            )],
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
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
            effects,
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
            effects: vec![Effect::create_tokens(dragon, Value::Fixed(1))],
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
        .filter(|word| !word.is_empty())
        .collect();
    let has_word = |needle: &str| words.iter().any(|word| *word == needle);
    let has_explicit_pt = words.iter().any(|word| parse_token_pt(word).is_some());
    let has_equipment_rules_subject = has_word("equipment")
        && words
            .windows(2)
            .any(|window| window == ["equipped", "creature"]);

    if has_word("treasure") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::treasure_token_definition());
    }
    if has_word("clue") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::clue_token_definition());
    }
    if has_word("map") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::map_token_definition());
    }
    if has_word("lander") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::lander_token_definition());
    }
    if has_word("junk") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::junk_token_definition());
    }
    if has_word("gold") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::gold_token_definition());
    }
    if has_word("shard") && !words.contains(&"creature") {
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
    if has_word("food") && !words.contains(&"creature") {
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
    if has_word("blood") && !words.contains(&"creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Blood")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("powerstone") && !words.contains(&"creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Powerstone")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("vehicle") && has_word("artifact") && !words.contains(&"creature") {
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
        if words.contains(&"flying") {
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
        && (!words.contains(&"creature") || has_equipment_rules_subject)
    {
        let mut subtypes = Vec::new();
        for word in &words {
            if let Some(subtype) = parse_subtype_word(word)
                && !subtype.is_creature_type()
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }
        let token_name = words
            .iter()
            .find(|word| {
                !matches!(
                    **word,
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
            .map(|word| {
                let mut chars = word.chars();
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
        if words.contains(&"legendary") {
            builder = builder.supertypes(vec![crate::types::Supertype::Legendary]);
        }
        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }
        if words.contains(&"colorless") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::make_colorless(
                ObjectFilter::source(),
            )));
        }
        if let Some(rules_text) = parse_equipment_rules_text(&words)
            && let Ok(def) = builder.clone().parse_text(&rules_text)
        {
            return Some(def);
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"leaves")
            && words.contains(&"battlefield")
            && words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"target")
            && let Some(amount) = parse_deals_damage_amount(&words)
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
    if has_word("wall")
        && lower.contains("0/4")
        && lower.contains("artifact")
        && lower.contains("creature")
    {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Wall")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Wall])
            .power_toughness(PowerToughness::fixed(0, 4))
            .defender();
        return Some(builder.build());
    }
    if has_word("squirrel") && lower.contains("1/1") && lower.contains("green") {
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
        && lower.contains("0/2")
        && words.contains(&"when")
        && words.contains(&"token")
        && words.contains(&"dies")
        && words.contains(&"create")
        && words.contains(&"2/2")
        && words.contains(&"flying")
        && words.contains(&"r")
        && words.contains(&"+1/+0");
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
    if has_word("elephant") && lower.contains("3/3") && lower.contains("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Elephant")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elephant])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(3, 3));
        return Some(builder.build());
    }
    let has_construct_cda_words = words.contains(&"power")
        && words.contains(&"toughness")
        && words.contains(&"equal")
        && words.contains(&"number")
        && words.contains(&"artifacts")
        && words.contains(&"you")
        && words.contains(&"control");
    let has_construct_plus_words = words.contains(&"gets")
        && words.contains(&"+1/+1")
        && words.contains(&"for")
        && words.contains(&"each")
        && words.contains(&"artifact")
        && words.contains(&"you")
        && words.contains(&"control");
    let is_zero_zero_construct = has_word("construct") && lower.contains("0/0");
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
    if has_word("shapeshifter") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Shapeshifter")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Shapeshifter])
            .power_toughness(PowerToughness::fixed(3, 2));
        if lower.contains("changeling") || lower == "shapeshifter" {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }
        return Some(builder.build());
    }
    if has_word("astartes")
        && has_word("warrior")
        && lower.contains("2/2")
        && lower.contains("white")
    {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Astartes Warrior")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Astartes, Subtype::Warrior])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(2, 2));
        if lower.contains("vigilance") {
            builder = builder.vigilance();
        }
        return Some(builder.build());
    }
    if words.contains(&"creature") {
        let mut card_types = vec![CardType::Creature];
        let first_creature_idx = words.iter().position(|word| *word == "creature");
        let artifact_before_creature =
            first_creature_idx.is_some_and(|idx| words[..idx].contains(&"artifact"));
        let enchantment_before_creature =
            first_creature_idx.is_some_and(|idx| words[..idx].contains(&"enchantment"));
        if artifact_before_creature {
            card_types.insert(0, CardType::Artifact);
        }
        if enchantment_before_creature {
            card_types.insert(0, CardType::Enchantment);
        }

        let (power, toughness) = words.iter().find_map(|word| parse_token_pt(word))?;

        let mut subtypes = Vec::new();
        let subtype_scan_end = words
            .iter()
            .position(|word| parse_card_type(word).is_some())
            .unwrap_or(words.len());
        for word in &words[..subtype_scan_end] {
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
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
                .unwrap_or_else(|| "Token".to_string())
        });

        let mut builder = CardDefinitionBuilder::new(CardId::new(), token_name)
            .token()
            .card_types(card_types)
            .power_toughness(PowerToughness::fixed(power, toughness));
        if words.contains(&"legendary") {
            builder = builder.supertypes(vec![crate::types::Supertype::Legendary]);
        }

        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }

        let mut colors = ColorSet::new();
        if words.contains(&"white") {
            colors = colors.union(ColorSet::WHITE);
        }
        if words.contains(&"blue") {
            colors = colors.union(ColorSet::BLUE);
        }
        if words.contains(&"black") {
            colors = colors.union(ColorSet::BLACK);
        }
        if words.contains(&"red") {
            colors = colors.union(ColorSet::RED);
        }
        if words.contains(&"green") {
            colors = colors.union(ColorSet::GREEN);
        }
        if !colors.is_empty() {
            builder = builder.color_indicator(colors);
        }

        if words.contains(&"flying") {
            builder = builder.flying();
        }
        if words.contains(&"defender") {
            builder = builder.defender();
        }
        if words.contains(&"prowess") {
            builder = builder.prowess();
        }
        if words.contains(&"vigilance") {
            builder = builder.vigilance();
        }
        if words.contains(&"trample") {
            builder = builder.trample();
        }
        if words.contains(&"lifelink") {
            builder = builder.lifelink();
        }
        if words.contains(&"deathtouch") {
            builder = builder.deathtouch();
        }
        if words.contains(&"haste") {
            builder = builder.haste();
        }
        if words.contains(&"menace") {
            builder = builder.menace();
        }
        if words.contains(&"reach") {
            builder = builder.reach();
        }
        if let Some(upkeep_idx) = words
            .windows(2)
            .position(|window| window == ["cumulative", "upkeep"])
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
        if words.contains(&"crews")
            && words.contains(&"vehicles")
            && words.contains(&"power")
            && words.contains(&"greater")
            && words.contains(&"2")
        {
            return None;
        }
        if words.contains(&"banding") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
                "banding",
            )));
        }
        if words.contains(&"hexproof") {
            builder = builder.hexproof();
        }
        if words.contains(&"indestructible") {
            builder = builder.indestructible();
        }
        if let Some(amount) = words.windows(2).find_map(|window| {
            if window[0] == "toxic" {
                window[1].parse::<u32>().ok()
            } else {
                None
            }
        }) {
            builder = builder.toxic(amount);
        }
        if words.contains(&"sacrifice")
            && words.contains(&"this")
            && words.contains(&"token")
            && words.contains(&"return")
            && words.contains(&"named")
            && words.contains(&"graveyard")
            && words.contains(&"battlefield")
            && !words.contains(&"beginning")
            && let Some(card_name) = extract_named_card_name(&words, lower.as_str())
            && let Some(sacrifice_idx) = words.iter().position(|word| *word == "sacrifice")
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
        if words
            .windows(5)
            .any(|window| window == ["at", "the", "beginning", "of", "your"])
            && words.contains(&"upkeep")
            && words.contains(&"sacrifice")
            && words.contains(&"this")
            && words.contains(&"token")
            && words.contains(&"return")
            && words.contains(&"named")
            && words.contains(&"graveyard")
            && words.contains(&"battlefield")
            && let Some(card_name) = extract_named_card_name(&words, lower.as_str())
        {
            builder =
                builder.with_ability(token_upkeep_sacrifice_return_named_from_graveyard_ability(
                    &card_name,
                    words.contains(&"haste"),
                ));
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"create")
            && words.contains(&"2/2")
            && words.contains(&"red")
            && words.contains(&"dragon")
            && words.contains(&"flying")
            && words.contains(&"r")
            && words.contains(&"+1/+0")
        {
            builder = builder.with_ability(token_dies_create_dragon_with_firebreathing_ability());
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"target")
            && let Some(amount) = parse_deals_damage_amount(&words)
        {
            builder = builder.with_ability(token_dies_deals_damage_any_target_ability(amount));
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"target")
            && words.contains(&"creature")
            && words.contains(&"gets")
            && words.contains(&"-1/-1")
        {
            builder =
                builder.with_ability(token_dies_target_creature_gets_minus_one_minus_one_ability());
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"leaves")
            && words.contains(&"battlefield")
            && words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"you")
            && words.contains(&"each")
            && words.contains(&"creature")
            && words.contains(&"control")
            && let Some(amount) = parse_deals_damage_amount(&words)
        {
            let ability = Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::this_leaves_battlefield(),
                    effects: vec![
                        Effect::deal_damage(amount, ChooseSpec::SourceController),
                        Effect::for_each(
                            ObjectFilter::creature().you_control(),
                            vec![Effect::deal_damage(amount, ChooseSpec::Iterated)],
                        ),
                    ],
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
        if words.contains(&"bands")
            && words.contains(&"other")
            && words.contains(&"creatures")
            && words.contains(&"named")
            && words.contains(&"wolves")
        {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
                "bands with other creatures named Wolves of the Hunt",
            )));
        }
        if words.contains(&"r")
            && words.contains(&"this")
            && words.contains(&"creature")
            && words.contains(&"gets")
            && words.contains(&"+1/+0")
            && !(words.contains(&"when")
                && words.contains(&"token")
                && words.contains(&"dies")
                && words.contains(&"create"))
        {
            builder = builder.with_ability(token_red_pump_ability());
        }
        if words.contains(&"w")
            && words.contains(&"t")
            && words.contains(&"tap")
            && words.contains(&"target")
            && words.contains(&"creature")
        {
            builder = builder.with_ability(token_white_tap_target_creature_ability());
        }
        if words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"player")
            && words.contains(&"poison")
            && words.contains(&"counter")
        {
            builder = builder.with_ability(token_damage_to_player_poison_counter_ability());
        }
        if let Some(amount) =
            token_inline_noncreature_spell_each_opponent_damage_amount(lower.as_str())
        {
            builder =
                builder.with_ability(token_noncreature_spell_each_opponent_damage_ability(amount));
        }
        if words.contains(&"whenever")
            && words.contains(&"token")
            && words.contains(&"becomes")
            && words.contains(&"tapped")
            && words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"target")
            && words.contains(&"player")
            && let Some(amount) = parse_deals_damage_amount(&words)
        {
            builder = builder.with_ability(
                token_becomes_tapped_deals_damage_target_player_ability(amount),
            );
        }
        if words.contains(&"whenever")
            && words.contains(&"token")
            && words.contains(&"deals")
            && words.contains(&"combat")
            && words.contains(&"damage")
            && words.contains(&"player")
            && words.contains(&"gain")
            && words.contains(&"control")
            && words.contains(&"artifact")
        {
            builder =
                builder.with_ability(token_combat_damage_gain_control_target_artifact_ability());
        }
        if words.contains(&"when")
            && words.contains(&"leaves")
            && words.contains(&"battlefield")
            && words.contains(&"return")
            && words.contains(&"named")
            && words.contains(&"graveyard")
            && words.contains(&"hand")
            && let Some(card_name) = extract_named_card_name(&words, lower.as_str())
        {
            builder = builder.with_ability(
                token_leaves_return_named_from_graveyard_to_hand_ability(&card_name),
            );
        }
        if has_word("pest")
            && words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"gain")
            && words.contains(&"1")
            && words.contains(&"life")
        {
            let ability = Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::this_dies(),
                    effects: vec![Effect::gain_life(1)],
                    choices: Vec::new(),
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("When this token dies, you gain 1 life.".to_string()),
            };
            builder = builder.with_ability(ability);
        }
        if words.contains(&"first") && words.contains(&"strike") {
            builder = builder.first_strike();
        }
        if words.contains(&"double") && words.contains(&"strike") {
            builder = builder.double_strike();
        }
        if has_word("mercenary")
            && words.contains(&"creature")
            && words.contains(&"1/1")
            && words.contains(&"red")
        {
            let target =
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
            let ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(
                        TotalCost::free(),
                        vec![Effect::tap_source()],
                    ),
                    effects: vec![Effect::pump(1, 0, target.clone(), Until::EndOfTurn)],
                    choices: vec![target],
                    timing: crate::ability::ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec!["activate only as a sorcery".to_string()],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(
                    "{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery."
                        .to_string(),
                ),
            };
            builder = builder.with_ability(ability);
        }
        let has_cant_attack_or_block = words.contains(&"cant")
            && words.contains(&"attack")
            && words.contains(&"or")
            && words.contains(&"block");
        if has_cant_attack_or_block && words.contains(&"alone") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
                "this token can't attack or block alone".to_string(),
            )));
        } else if has_cant_attack_or_block {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                "this token can't attack or block".to_string(),
            )));
        } else if words.contains(&"cant") && words.contains(&"block") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::cant_block()));
        }
        if words.contains(&"can")
            && words.contains(&"block")
            && words.contains(&"only")
            && words.contains(&"creatures")
            && words.contains(&"flying")
        {
            builder = builder.with_ability(Ability::static_ability(
                StaticAbility::can_block_only_flying(),
            ));
        }
        if words.contains(&"counter")
            && words.contains(&"noncreature")
            && words.contains(&"spell")
            && words.contains(&"sacrifice")
            && words.contains(&"token")
            && words.contains(&"unless")
            && words.contains(&"controller")
            && words.contains(&"pays")
            && words.contains(&"1")
        {
            let target = ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::spell().without_type(CardType::Creature),
            ));
            let counter_ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(
                        TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])),
                        vec![Effect::sacrifice_source()],
                    ),
                    effects: vec![Effect::counter_unless_pays(
                        target.clone(),
                        vec![ManaSymbol::Generic(1)],
                    )],
                    choices: vec![target],
                    timing: crate::ability::ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(
                    "{1}, Sacrifice this token: Counter target noncreature spell unless its controller pays {1}."
                        .to_string(),
                ),
            };
            builder = builder.with_ability(counter_ability);
        }
        if words.contains(&"changeling") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }
        if words.contains(&"this")
            && words.contains(&"token")
            && words.contains(&"gets")
            && words.contains(&"+1/+1")
            && words.contains(&"for")
            && words.contains(&"each")
            && words.contains(&"card")
            && words.contains(&"named")
            && (words.contains(&"graveyard") || words.contains(&"graveyards"))
        {
            let card_name = words
                .windows(2)
                .position(|window| window == ["card", "named"])
                .and_then(|named_card_idx| {
                    let start = named_card_idx + 2;
                    let end = (start..words.len())
                        .find(|idx| {
                            matches!(
                                words[*idx],
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
        let is_land_you_control_enters_pump_token = words.contains(&"whenever")
            && words.contains(&"land")
            && words.contains(&"control")
            && words.contains(&"enters")
            && words.contains(&"this")
            && words.contains(&"token")
            && words.contains(&"gets")
            && words.contains(&"+1/+0")
            && contains_until_end_of_turn(&words);
        if is_land_you_control_enters_pump_token {
            let ability = Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::enters_battlefield(ObjectFilter::land().you_control()),
                    effects: vec![Effect::pump(1, 0, ChooseSpec::Source, Until::EndOfTurn)],
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
    let (left, right) = word.split_once('/')?;
    if left.starts_with('+')
        || right.starts_with('+')
        || left.starts_with('-')
        || right.starts_with('-')
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
    use crate::effect::Value;
    use crate::effects::{GrantPlayTaggedEffect, InvestigateEffect};
    use crate::ids::CardId;
    use crate::types::CardType;

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
            RefState::Known(TagKey::from("destroyed_0"))
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
}
