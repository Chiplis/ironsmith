use crate::cards::builders::effect_ast_traversal::{
    for_each_nested_effects_mut, try_for_each_nested_effects_mut,
};
use crate::cards::builders::{
    AnnotatedEffect, AnnotatedEffectSequence, CardTextError, EffectAst, IT_TAG, IdGenContext,
    NewTargetRestrictionAst, ObjectRefAst, PlayerAst, PredicateAst, PreventNextTimeDamageSourceAst,
    RefState, ReferenceEnv, ReferenceFrame, ReferenceImports, RetargetModeAst, TargetAst,
    choose_spec_targets_object, effect_references_event_derived_amount, effects_reference_it_tag,
    effects_reference_its_controller, infer_player_filter_from_object_filter, resolve_it_tag,
    resolve_non_target_player_filter, resolve_target_spec_with_choices,
};
use crate::effect::{EffectId, EventValueSpec};
use crate::target::ObjectRef;
use crate::{ChooseSpec, ObjectFilter, PlayerFilter, TagKey, Value};

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct BoundEffectsAst {
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) imports: ReferenceImports,
    pub(crate) unresolved_it_before: usize,
    pub(crate) unresolved_it_after: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EffectReferenceResolutionConfig {
    pub(crate) allow_life_event_value: bool,
    pub(crate) bind_unbound_x_to_last_effect: bool,
    pub(crate) initial_last_effect_id: Option<EffectId>,
    pub(crate) initial_iterated_player: bool,
    pub(crate) force_auto_tag_object_targets: bool,
}

#[derive(Debug, Clone, Copy)]
struct EffectReferenceResolutionState {
    last_effect_id: Option<EffectId>,
    allow_life_event_value: bool,
    bind_unbound_x_to_last_effect: bool,
}

fn trigger_supports_event_amount(trigger: &crate::cards::builders::TriggerSpec) -> bool {
    matches!(
        trigger,
        crate::cards::builders::TriggerSpec::YouGainLife
            | crate::cards::builders::TriggerSpec::YouGainLifeDuringTurn(_)
            | crate::cards::builders::TriggerSpec::PlayerLosesLife(_)
            | crate::cards::builders::TriggerSpec::PlayerLosesLifeDuringTurn { .. }
            | crate::cards::builders::TriggerSpec::ThisIsDealtDamage
            | crate::cards::builders::TriggerSpec::IsDealtDamage(_)
            | crate::cards::builders::TriggerSpec::ThisDealsDamage
            | crate::cards::builders::TriggerSpec::ThisDealsDamageTo(_)
            | crate::cards::builders::TriggerSpec::DealsDamage(_)
            | crate::cards::builders::TriggerSpec::ThisDealsCombatDamage
            | crate::cards::builders::TriggerSpec::ThisDealsCombatDamageTo(_)
            | crate::cards::builders::TriggerSpec::DealsCombatDamage(_)
            | crate::cards::builders::TriggerSpec::DealsCombatDamageTo { .. }
            | crate::cards::builders::TriggerSpec::ThisDealsCombatDamageToPlayer
            | crate::cards::builders::TriggerSpec::DealsCombatDamageToPlayer { .. }
            | crate::cards::builders::TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
            | crate::cards::builders::TriggerSpec::CounterPutOn { .. }
    ) || matches!(
        trigger,
        crate::cards::builders::TriggerSpec::Either(left, right)
            if trigger_supports_event_amount(left) && trigger_supports_event_amount(right)
    )
}

pub(crate) fn annotate_effect_sequence(
    effects: &[EffectAst],
    imports: &ReferenceImports,
    config: EffectReferenceResolutionConfig,
    id_gen: IdGenContext,
) -> Result<AnnotatedEffectSequence, CardTextError> {
    let env = ReferenceEnv::from_imports(
        imports,
        config.initial_iterated_player,
        config.allow_life_event_value,
        config.bind_unbound_x_to_last_effect,
        config.initial_last_effect_id,
    );
    let mut id_gen = id_gen;
    annotate_effect_sequence_with_env_internal(effects, env, config, &mut id_gen)
}

pub(crate) fn annotate_effect_sequence_from_env(
    effects: &[EffectAst],
    env: ReferenceEnv,
    config: EffectReferenceResolutionConfig,
    id_gen: IdGenContext,
) -> Result<AnnotatedEffectSequence, CardTextError> {
    let mut id_gen = id_gen;
    annotate_effect_sequence_with_env_internal(effects, env, config, &mut id_gen)
}

fn lowering_reference_frame(frame: &ReferenceFrame) -> ReferenceEnv {
    ReferenceEnv::from_frame(frame)
}

fn next_reference_tag(id_gen: &mut IdGenContext, prefix: &str) -> String {
    let tag = format!("{prefix}_{}", id_gen.next_tag_id);
    id_gen.next_tag_id += 1;
    tag
}

fn track_effect_player(
    player: PlayerAst,
    frame: &mut ReferenceFrame,
    allow_target: bool,
    allow_target_opponent: bool,
) -> Result<(), CardTextError> {
    if matches!(player, PlayerAst::Implicit) {
        return Ok(());
    }

    let refs = lowering_reference_frame(frame);
    let filter = match player {
        PlayerAst::Target if allow_target => PlayerFilter::target_player(),
        PlayerAst::TargetOpponent if allow_target_opponent => {
            PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
        }
        _ => resolve_non_target_player_filter(player, &refs)?,
    };
    frame.last_player_filter = Some(filter);
    Ok(())
}

fn track_target_player(target: &TargetAst, frame: &mut ReferenceFrame) {
    match target {
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            frame.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
        }
        TargetAst::Object(filter, _, _) => track_player_from_object_filter(filter, frame),
        _ => {}
    }
}

fn track_player_from_object_filter(filter: &ObjectFilter, frame: &mut ReferenceFrame) {
    if let Some(tag) = frame.last_object_tag.as_deref() {
        if filter.owner.is_some() {
            frame.last_player_filter = Some(PlayerFilter::OwnerOf(ObjectRef::tagged(tag)));
            return;
        }
        if filter.controller.is_some()
            || filter.tagged_constraints.iter().any(|constraint| {
                matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameControllerAsTagged
                )
            })
        {
            frame.last_player_filter = Some(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)));
            return;
        }
    }
    if let Some(player_filter) = infer_player_filter_from_object_filter(filter) {
        frame.last_player_filter = Some(player_filter);
    }
}

fn maybe_tag_target(
    target: &TargetAst,
    frame: &mut ReferenceFrame,
    id_gen: &mut IdGenContext,
    prefix: &str,
) -> Result<(), CardTextError> {
    let refs = lowering_reference_frame(frame);
    let (spec, _) = resolve_target_spec_with_choices(target, &refs)?;
    if frame.auto_tag_object_targets && choose_spec_targets_object(&spec) {
        frame.last_object_tag = Some(next_reference_tag(id_gen, prefix));
    }
    track_target_player(target, frame);
    Ok(())
}

fn advance_effects_preserving_last_effect(
    effects: &[EffectAst],
    id_gen: &mut IdGenContext,
    frame: &mut ReferenceFrame,
) -> Result<(), CardTextError> {
    let saved_last_effect = frame.last_effect_id;
    advance_reference_frames(effects, id_gen, frame)?;
    frame.last_effect_id = saved_last_effect;
    Ok(())
}

fn advance_effects_in_iterated_player_context(
    effects: &[EffectAst],
    id_gen: &mut IdGenContext,
    frame: &mut ReferenceFrame,
    tagged_object: Option<String>,
) -> Result<(), CardTextError> {
    let saved = frame.clone();
    let mut nested = saved.clone();
    nested.iterated_player = true;
    nested.last_effect_id = None;
    if let Some(tag) = tagged_object {
        nested.last_object_tag = Some(tag);
    }
    advance_reference_frames(effects, id_gen, &mut nested)?;
    if saved.last_object_tag != nested.last_object_tag {
        frame.last_object_tag = nested.last_object_tag;
    }
    Ok(())
}

fn advance_reference_frames(
    effects: &[EffectAst],
    id_gen: &mut IdGenContext,
    frame: &mut ReferenceFrame,
) -> Result<(), CardTextError> {
    for effect in effects {
        advance_reference_frame_for_effect(effect, id_gen, frame)?;
    }
    Ok(())
}

fn advance_reference_frame_for_effect(
    effect: &EffectAst,
    id_gen: &mut IdGenContext,
    frame: &mut ReferenceFrame,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::SetLifeTotal { player, .. }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Discover { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::PayMana { player, .. }
        | EffectAst::PayEnergy { player, .. }
        | EffectAst::AddMana { player, .. }
        | EffectAst::AddManaScaled { player, .. }
        | EffectAst::AddManaAnyColor { player, .. }
        | EffectAst::AddManaAnyOneColor { player, .. }
        | EffectAst::AddManaChosenColor { player, .. }
        | EffectAst::AddManaFromLandCouldProduce { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::ExtraTurnAfterTurn { player, .. }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::PutSomeIntoHandRestIntoGraveyard { player, .. }
        | EffectAst::PutSomeIntoHandRestOnBottomOfLibrary { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::SkipTurn { player }
        | EffectAst::SkipCombatPhases { player }
        | EffectAst::SkipNextCombatPhaseThisTurn { player }
        | EffectAst::SkipDrawStep { player }
        | EffectAst::ShuffleGraveyardIntoLibrary { player }
        | EffectAst::ReorderGraveyard { player }
        | EffectAst::ShuffleLibrary { player } => {
            track_effect_player(player.clone(), frame, true, true)?;
        }
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard { player, .. }
        | EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary { player, .. } => {
            track_effect_player(player.clone(), frame, true, true)?;
            frame.last_object_tag = Some(next_reference_tag(id_gen, "chosen"));
        }
        EffectAst::RevealTop { player } => {
            track_effect_player(player.clone(), frame, true, true)?;
            frame.last_object_tag = Some(next_reference_tag(id_gen, "revealed"));
        }
        EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard { player, .. } => {
            track_effect_player(player.clone(), frame, true, true)?;
            frame.last_object_tag = Some(next_reference_tag(id_gen, "revealed"));
        }
        EffectAst::ControlPlayer { player, .. } => {
            frame.last_player_filter = Some(player.clone());
        }
        EffectAst::LookAtHand { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::Counter { target }
        | EffectAst::CounterUnlessPays { target, .. }
        | EffectAst::CopySpell { target, .. }
        | EffectAst::PreventDamage { target, .. }
        | EffectAst::PreventAllDamageToTarget { target, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { target, .. }
        | EffectAst::RedirectNextTimeDamageToSource { target, .. }
        | EffectAst::Transform { target }
        | EffectAst::Flip { target } => {
            maybe_tag_target(&target, frame, id_gen, "targeted")?;
        }
        EffectAst::DealDamage { target, .. } | EffectAst::DealDamageEqualToPower { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "damaged")?;
        }
        EffectAst::PutCounters { target, .. }
        | EffectAst::PutOrRemoveCounters { target, .. }
        | EffectAst::RemoveUpToAnyCounters { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "counters")?;
        }
        EffectAst::Tap { target } => {
            maybe_tag_target(&target, frame, id_gen, "tapped")?;
        }
        EffectAst::Untap { target } => {
            maybe_tag_target(&target, frame, id_gen, "untapped")?;
        }
        EffectAst::PhaseOut { target } => {
            maybe_tag_target(&target, frame, id_gen, "phased_out")?;
        }
        EffectAst::RemoveFromCombat { target } => {
            maybe_tag_target(&target, frame, id_gen, "removed_from_combat")?;
        }
        EffectAst::TapOrUntap { target } => {
            maybe_tag_target(&target, frame, id_gen, "tap_or_untap")?;
        }
        EffectAst::GrantProtectionChoice { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "protected")?;
        }
        EffectAst::Explore { target } => {
            maybe_tag_target(&target, frame, id_gen, "explored")?;
        }
        EffectAst::GainControl { target, player, .. } => {
            track_effect_player(player.clone(), frame, true, true)?;
            maybe_tag_target(&target, frame, id_gen, "controlled")?;
        }
        EffectAst::RetargetStackObject { .. } => {
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "retargeted"));
            }
        }
        EffectAst::Connive { target } => {
            maybe_tag_target(&target, frame, id_gen, "connived")?;
        }
        EffectAst::Goad { target } => {
            maybe_tag_target(&target, frame, id_gen, "goaded")?;
        }
        EffectAst::Destroy { target } | EffectAst::DestroyNoRegeneration { target } => {
            maybe_tag_target(&target, frame, id_gen, "destroyed")?;
        }
        EffectAst::Exile { target, .. } | EffectAst::ExileUntilSourceLeaves { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "exiled")?;
        }
        EffectAst::ReturnToHand { target, .. } | EffectAst::Regenerate { target } => {
            maybe_tag_target(&target, frame, id_gen, "returned")?;
        }
        EffectAst::ReturnToBattlefield { target, .. } => {
            let refs = lowering_reference_frame(frame);
            let (spec, _) = resolve_target_spec_with_choices(&target, &refs)?;
            if frame.auto_tag_object_targets && choose_spec_targets_object(&spec) {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "returned"));
            }
        }
        EffectAst::MoveToLibrarySecondFromTop { target } | EffectAst::MoveToZone { target, .. } => {
            let refs = lowering_reference_frame(frame);
            let (spec, _) = resolve_target_spec_with_choices(&target, &refs)?;
            if frame.auto_tag_object_targets && choose_spec_targets_object(&spec) {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "moved"));
            }
        }
        EffectAst::RevealTagged { tag } => {
            frame.last_object_tag = Some(if tag.as_str() == IT_TAG {
                frame
                    .last_object_tag
                    .clone()
                    .unwrap_or_else(|| next_reference_tag(id_gen, "revealed"))
            } else {
                tag.as_str().to_string()
            });
        }
        EffectAst::LookAtTopCards { tag, .. } => {
            frame.last_object_tag = Some(if tag.as_str() == IT_TAG {
                next_reference_tag(id_gen, "revealed")
            } else {
                tag.as_str().to_string()
            });
        }
        EffectAst::ChooseObjects {
            filter,
            tag,
            player,
            ..
        } => {
            let references_revealed_hand = filter.zone == Some(crate::zone::Zone::Hand)
                && filter.owner.is_none()
                && filter.controller.is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                });
            let refs = lowering_reference_frame(frame);
            let chooser_filter = if matches!(player, PlayerAst::Implicit) {
                None
            } else {
                Some(match player {
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => {
                        PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
                    }
                    other => resolve_non_target_player_filter(*other, &refs)?,
                })
            };
            if let Some(player_filter) = if references_revealed_hand {
                frame.last_player_filter.clone()
            } else {
                None
            }
            .or_else(|| {
                resolve_it_tag(filter, &refs)
                .ok()
                .and_then(|resolved| infer_player_filter_from_object_filter(&resolved))
            })
            .or_else(|| infer_player_filter_from_object_filter(filter))
            .or(chooser_filter)
            {
                frame.last_player_filter = Some(player_filter);
            }
            frame.last_object_tag = Some(tag.as_str().to_string());
        }
        EffectAst::SearchLibrary { player, .. } => {
            track_effect_player(player.clone(), frame, true, true)?;
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "searched"));
            }
        }
        EffectAst::Sacrifice { player, .. } => {
            track_effect_player(player.clone(), frame, true, true)?;
            frame.last_object_tag = Some(next_reference_tag(id_gen, "sacrificed"));
        }
        EffectAst::SacrificeAll { player, .. } => {
            track_effect_player(player.clone(), frame, true, true)?;
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "sacrificed"));
            }
        }
        EffectAst::CreateToken { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. } => {
            track_effect_player(player.clone(), frame, true, true)?;
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "created"));
            }
        }
        EffectAst::CreateTokenWithMods {
            player,
            attached_to,
            ..
        } => {
            track_effect_player(player.clone(), frame, true, true)?;
            if frame.auto_tag_object_targets || attached_to.is_some() {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "created"));
            }
        }
        EffectAst::MoveAllCounters { .. } => {
            if frame.auto_tag_object_targets {
                let _ = next_reference_tag(id_gen, "from");
                frame.last_object_tag = Some(next_reference_tag(id_gen, "to"));
            }
        }
        EffectAst::Pump { target, .. } | EffectAst::PumpForEach { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "pumped")?;
        }
        EffectAst::SetBasePowerToughness { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "set_base_pt")?;
        }
        EffectAst::BecomeBasePtCreature { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "animated_creature")?;
        }
        EffectAst::AddCardTypes { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "typed")?;
        }
        EffectAst::AddSubtypes { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "subtyped")?;
        }
        EffectAst::SetBasePower { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "set_base_power")?;
        }
        EffectAst::SetColors { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "set_colors")?;
        }
        EffectAst::MakeColorless { target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "set_colorless")?;
        }
        EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::RemoveAbilitiesFromTarget { target, .. }
        | EffectAst::PreventAllCombatDamageFromSource { source: target, .. } => {
            maybe_tag_target(&target, frame, id_gen, "targeted")?;
        }
        EffectAst::PumpAll { .. } => {
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "pumped"));
            }
        }
        EffectAst::ReturnAllToBattlefield { filter, .. } => {
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "returned"));
            }
            track_player_from_object_filter(filter, frame);
        }
        EffectAst::DestroyAll { filter }
        | EffectAst::DestroyAllNoRegeneration { filter }
        | EffectAst::DestroyAllOfChosenColor { filter }
        | EffectAst::DestroyAllOfChosenColorNoRegeneration { filter }
        | EffectAst::DestroyAllAttachedTo { filter, .. } => {
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "destroyed"));
            }
            track_player_from_object_filter(filter, frame);
        }
        EffectAst::ExileAll { filter, .. } => {
            if frame.auto_tag_object_targets {
                frame.last_object_tag = Some(next_reference_tag(id_gen, "exiled"));
            }
            track_player_from_object_filter(filter, frame);
        }
        EffectAst::ExileWhenSourceLeaves { target, .. }
        | EffectAst::SacrificeSourceWhenLeaves { target, .. } => {
            track_target_player(target, frame);
        }
        EffectAst::ExchangeControl { .. } => {
            frame.last_object_tag = Some(next_reference_tag(id_gen, "exchanged"));
        }
        EffectAst::May { effects }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. } => {
            advance_effects_preserving_last_effect(&effects, id_gen, frame)?;
        }
        EffectAst::MayByPlayer { player, effects } => {
            advance_effects_preserving_last_effect(&effects, id_gen, frame)?;
            track_effect_player(player.clone(), frame, true, true)?;
        }
        EffectAst::MayByTaggedController { effects, .. } => {
            advance_effects_preserving_last_effect(&effects, id_gen, frame)?;
        }
        EffectAst::DelayedUntilNextUpkeep { player, effects }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects } => {
            advance_effects_preserving_last_effect(&effects, id_gen, frame)?;
            track_effect_player(player.clone(), frame, true, true)?;
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            let saved = frame.clone();
            let mut true_frame = saved.clone();
            advance_reference_frames(&if_true, id_gen, &mut true_frame)?;
            if if_false.is_empty() {
                *frame = true_frame;
            } else {
                let mut false_frame = saved.clone();
                advance_reference_frames(&if_false, id_gen, &mut false_frame)?;
                frame.last_object_tag = saved.last_object_tag;
                frame.last_player_filter = saved.last_player_filter;
                frame.iterated_player = saved.iterated_player;
            }
        }
        EffectAst::ResolvedIfResult {
            condition, effects, ..
        } => {
            let saved_last_effect = frame.last_effect_id;
            let saved_bind = frame.bind_unbound_x_to_last_effect;
            frame.last_effect_id = Some(*condition);
            frame.bind_unbound_x_to_last_effect = true;
            advance_reference_frames(&effects, id_gen, frame)?;
            frame.last_effect_id = saved_last_effect;
            frame.bind_unbound_x_to_last_effect = saved_bind;
        }
        EffectAst::ResolvedWhenResult {
            condition, effects, ..
        } => {
            let saved_last_effect = frame.last_effect_id;
            let saved_bind = frame.bind_unbound_x_to_last_effect;
            frame.last_effect_id = Some(*condition);
            frame.bind_unbound_x_to_last_effect = true;
            advance_reference_frames(&effects, id_gen, frame)?;
            frame.last_effect_id = saved_last_effect;
            frame.bind_unbound_x_to_last_effect = saved_bind;
        }
        EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. } => {
            advance_effects_in_iterated_player_context(&effects, id_gen, frame, None)?;
        }
        EffectAst::ForEachObject { effects, .. } => {
            let saved = frame.clone();
            let mut nested = saved.clone();
            nested.last_effect_id = None;
            nested.last_object_tag = Some(IT_TAG.to_string());
            advance_reference_frames(&effects, id_gen, &mut nested)?;
        }
        EffectAst::ForEachTagged { tag, effects } => {
            let tagged_object = if tag.as_str() == IT_TAG {
                frame.last_object_tag.clone()
            } else {
                Some(tag.as_str().to_string())
            };
            advance_effects_in_iterated_player_context(&effects, id_gen, frame, tagged_object)?;
        }
        EffectAst::Fight { .. }
        | EffectAst::FightIterated { .. }
        | EffectAst::Clash { .. }
        | EffectAst::DealDamageEach { .. }
        | EffectAst::ForEachCounterKindPutOrRemove { .. }
        | EffectAst::PutCountersAll { .. }
        | EffectAst::DoubleCountersOnEach { .. }
        | EffectAst::Proliferate
        | EffectAst::TapAll { .. }
        | EffectAst::UntapAll { .. }
        | EffectAst::LoseGame { .. }
        | EffectAst::WinGame { .. }
        | EffectAst::PreventAllCombatDamage { .. }
        | EffectAst::PreventAllCombatDamageToPlayers { .. }
        | EffectAst::PreventAllCombatDamageToYou { .. }
        | EffectAst::PreventNextTimeDamage { .. }
        | EffectAst::PreventDamageEach { .. }
        | EffectAst::Earthbend { .. }
        | EffectAst::OpenAttraction
        | EffectAst::ManifestDread
        | EffectAst::Bolster { .. }
        | EffectAst::Support { .. }
        | EffectAst::Adapt { .. }
        | EffectAst::CounterActivatedOrTriggeredAbility
        | EffectAst::AddManaImprintedColors
        | EffectAst::BecomeBasicLandTypeChoice { .. }
        | EffectAst::BecomeCreatureTypeChoice { .. }
        | EffectAst::BecomeColorChoice { .. }
        | EffectAst::ExileUntilMatchGrantPlayUntilEndOfTurn { .. }
        | EffectAst::ExileUntilMatchCast { .. }
        | EffectAst::Cant { .. }
        | EffectAst::PlayFromGraveyardUntilEot { .. }
        | EffectAst::GrantPlayTaggedUntilEndOfTurn { .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. }
        | EffectAst::GrantPlayTaggedUntilYourNextTurn { .. }
        | EffectAst::CastTagged { .. }
        | EffectAst::ExileInsteadOfGraveyardThisTurn { .. }
        | EffectAst::RevealTopChooseCardTypePutToHandRestBottom { .. }
        | EffectAst::PutRestOnBottomOfLibrary
        | EffectAst::UnlessPays { .. }
        | EffectAst::UnlessAction { .. }
        | EffectAst::IfResult { .. }
        | EffectAst::WhenResult { .. }
        | EffectAst::ForEachOpponentDoesNot { .. }
        | EffectAst::ForEachPlayerDoesNot { .. }
        | EffectAst::ForEachOpponentDid { .. }
        | EffectAst::ForEachPlayerDid { .. }
        | EffectAst::Enchant { .. }
        | EffectAst::Attach { .. }
        | EffectAst::Investigate { .. }
        | EffectAst::Amass { .. }
        | EffectAst::Monstrosity { .. }
        | EffectAst::ConniveIterated
        | EffectAst::RemoveCountersAll { .. }
        | EffectAst::RegenerateAll { .. }
        | EffectAst::SwitchPowerToughness { .. }
        | EffectAst::PumpByLastEffect { .. }
        | EffectAst::GrantAbilitiesAll { .. }
        | EffectAst::RemoveAbilitiesAll { .. }
        | EffectAst::GrantAbilitiesChoiceAll { .. }
        | EffectAst::GrantAbilityToSource { .. }
        | EffectAst::ReorderTopOfLibrary { .. }
        | EffectAst::VoteStart { .. }
        | EffectAst::VoteOption { .. }
        | EffectAst::VoteExtra { .. }
        | EffectAst::ReturnAllToHand { .. }
        | EffectAst::ReturnAllToHandOfChosenColor { .. } => {}
    }

    Ok(())
}

fn effect_reference_resolution_state(env: &ReferenceEnv) -> EffectReferenceResolutionState {
    EffectReferenceResolutionState {
        last_effect_id: env.last_effect_id.clone().into_option(),
        allow_life_event_value: env.allow_life_event_value,
        bind_unbound_x_to_last_effect: env.bind_unbound_x_to_last_effect,
    }
}

fn annotate_effect_sequence_with_env_internal(
    effects: &[EffectAst],
    mut current_env: ReferenceEnv,
    config: EffectReferenceResolutionConfig,
    id_gen: &mut IdGenContext,
) -> Result<AnnotatedEffectSequence, CardTextError> {
    let mut annotated = Vec::with_capacity(effects.len());

    for (idx, effect) in effects.iter().enumerate() {
        let in_env = current_env.clone();
        let effect = resolve_effect_references_in_effect(
            effect.clone(),
            id_gen,
            effect_reference_resolution_state(&in_env),
        )?;
        let remaining = if idx + 1 < effects.len() {
            &effects[idx + 1..]
        } else {
            &[]
        };
        let auto_tag_object_targets =
            effects_reference_it_tag(remaining) || effects_reference_its_controller(remaining);
        let assigned_effect_id =
            maybe_assign_effect_result_id(effects, idx, id_gen, in_env.allow_life_event_value);

        let mut out_env = advance_reference_env_for_effect(
            &effect,
            &in_env,
            config,
            id_gen,
            auto_tag_object_targets,
        )?;
        if let Some(id) = assigned_effect_id {
            out_env.last_effect_id = RefState::Known(id);
        }

        current_env = out_env.clone();
        annotated.push(AnnotatedEffect {
            effect,
            in_env,
            out_env,
            assigned_effect_id,
            auto_tag_object_targets,
        });
    }

    Ok(AnnotatedEffectSequence {
        effects: annotated,
        final_env: current_env,
    })
}

fn maybe_assign_effect_result_id(
    effects: &[EffectAst],
    idx: usize,
    id_gen: &mut IdGenContext,
    allow_life_event_value: bool,
) -> Option<EffectId> {
    let next_is_result_gate = idx + 1 < effects.len()
        && matches!(
            effects[idx + 1],
            EffectAst::IfResult { .. } | EffectAst::WhenResult { .. }
        );
    let next_is_if_result_with_opponent_doesnt = next_is_result_gate
        && idx + 2 < effects.len()
        && matches!(effects[idx + 2], EffectAst::ForEachOpponentDoesNot { .. });
    let next_is_if_result_with_player_doesnt = next_is_result_gate
        && idx + 2 < effects.len()
        && matches!(effects[idx + 2], EffectAst::ForEachPlayerDoesNot { .. });
    let next_is_if_result_with_opponent_did = next_is_result_gate
        && idx + 2 < effects.len()
        && matches!(effects[idx + 2], EffectAst::ForEachOpponentDid { .. });
    let next_is_if_result_with_player_did = next_is_result_gate
        && idx + 2 < effects.len()
        && matches!(effects[idx + 2], EffectAst::ForEachPlayerDid { .. });
    let next_needs_event_derived_amount = !allow_life_event_value
        && idx + 1 < effects.len()
        && effect_references_event_derived_amount(&effects[idx + 1]);
    let next_needs_prior_effect_value =
        idx + 1 < effects.len() && matches!(effects[idx + 1], EffectAst::PumpByLastEffect { .. });

    if !(next_is_if_result_with_opponent_doesnt
        || next_is_if_result_with_player_doesnt
        || next_is_if_result_with_opponent_did
        || next_is_if_result_with_player_did
        || next_is_result_gate
        || next_needs_event_derived_amount
        || next_needs_prior_effect_value)
    {
        return None;
    }

    let id = EffectId(id_gen.next_effect_id);
    id_gen.next_effect_id += 1;
    Some(id)
}

fn resolve_effect_references_in_effect(
    mut effect: EffectAst,
    id_gen: &mut IdGenContext,
    state: EffectReferenceResolutionState,
) -> Result<EffectAst, CardTextError> {
    if let EffectAst::IfResult { predicate, effects } = effect {
        let condition = state.last_effect_id.ok_or_else(|| {
            CardTextError::ParseError("missing prior effect for if clause".to_string())
        })?;
        let effects = resolve_effect_sequence_references_with_state(
            &effects,
            id_gen,
            EffectReferenceResolutionState {
                last_effect_id: Some(condition),
                allow_life_event_value: state.allow_life_event_value,
                bind_unbound_x_to_last_effect: true,
            },
        )?;
        return Ok(EffectAst::ResolvedIfResult {
            condition,
            predicate,
            effects,
        });
    }

    if let EffectAst::WhenResult { predicate, effects } = effect {
        let condition = state.last_effect_id.ok_or_else(|| {
            CardTextError::ParseError("missing prior effect for when clause".to_string())
        })?;
        let effects = resolve_effect_sequence_references_with_state(
            &effects,
            id_gen,
            EffectReferenceResolutionState {
                last_effect_id: Some(condition),
                allow_life_event_value: state.allow_life_event_value,
                bind_unbound_x_to_last_effect: true,
            },
        )?;
        return Ok(EffectAst::ResolvedWhenResult {
            condition,
            predicate,
            effects,
        });
    }

    if let EffectAst::PumpByLastEffect {
        power,
        toughness,
        target,
        duration,
    } = &effect
        && let Some(id) = state.last_effect_id
    {
        return Ok(EffectAst::Pump {
            power: if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            },
            toughness: Value::Fixed(*toughness),
            target: target.clone(),
            duration: duration.clone(),
            condition: None,
        });
    }

    if let EffectAst::DelayedTriggerThisTurn { trigger, effects } = &mut effect {
        let nested_state = EffectReferenceResolutionState {
            last_effect_id: state.last_effect_id,
            allow_life_event_value: trigger_supports_event_amount(trigger),
            bind_unbound_x_to_last_effect: state.bind_unbound_x_to_last_effect,
        };
        *effects = resolve_effect_sequence_references_with_state(effects, id_gen, nested_state)?;
        return Ok(effect);
    }

    resolve_effect_result_values_in_fields(&mut effect, state)?;
    try_for_each_nested_effects_mut(&mut effect, true, |nested| {
        let resolved = resolve_effect_sequence_references_with_state(nested, id_gen, state)?;
        nested.clone_from_slice(&resolved);
        Ok::<_, CardTextError>(())
    })?;
    Ok(effect)
}

fn resolve_effect_sequence_references_with_state(
    effects: &[EffectAst],
    id_gen: &mut IdGenContext,
    mut state: EffectReferenceResolutionState,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut resolved = Vec::with_capacity(effects.len());

    for (idx, effect) in effects.iter().enumerate() {
        let effect = resolve_effect_references_in_effect(effect.clone(), id_gen, state)?;
        let remaining = if idx + 1 < effects.len() {
            &effects[idx + 1..]
        } else {
            &[]
        };
        let _ = effects_reference_it_tag(remaining) || effects_reference_its_controller(remaining);
        let assigned_effect_id =
            maybe_assign_effect_result_id(effects, idx, id_gen, state.allow_life_event_value);
        state.last_effect_id = assigned_effect_id;
        resolved.push(effect);
    }

    Ok(resolved)
}

fn advance_reference_env_for_effect(
    effect: &EffectAst,
    env: &ReferenceEnv,
    config: EffectReferenceResolutionConfig,
    id_gen: &mut IdGenContext,
    auto_tag_object_targets: bool,
) -> Result<ReferenceEnv, CardTextError> {
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            let true_sequence =
                annotate_effect_sequence_with_env_internal(if_true, env.clone(), config, id_gen)?;
            if if_false.is_empty() {
                return Ok(true_sequence.final_env);
            }

            let false_sequence =
                annotate_effect_sequence_with_env_internal(if_false, env.clone(), config, id_gen)?;
            Ok(ReferenceEnv {
                last_object_tag: RefState::join(
                    &true_sequence.final_env.last_object_tag,
                    &false_sequence.final_env.last_object_tag,
                ),
                last_player_filter: RefState::join(
                    &true_sequence.final_env.last_player_filter,
                    &false_sequence.final_env.last_player_filter,
                ),
                last_effect_id: env.last_effect_id.clone(),
                iterated_player: env.iterated_player,
                allow_life_event_value: env.allow_life_event_value,
                bind_unbound_x_to_last_effect: env.bind_unbound_x_to_last_effect,
            })
        }
        EffectAst::ResolvedIfResult {
            condition, effects, ..
        } => {
            let mut nested_env = env.clone();
            nested_env.last_effect_id = RefState::Known(*condition);
            nested_env.bind_unbound_x_to_last_effect = true;
            let nested =
                annotate_effect_sequence_with_env_internal(effects, nested_env, config, id_gen)?;
            let mut out_env = nested.final_env;
            out_env.last_effect_id = env.last_effect_id.clone();
            out_env.bind_unbound_x_to_last_effect = env.bind_unbound_x_to_last_effect;
            Ok(out_env)
        }
        EffectAst::ResolvedWhenResult {
            condition, effects, ..
        } => {
            let mut nested_env = env.clone();
            nested_env.last_effect_id = RefState::Known(*condition);
            nested_env.bind_unbound_x_to_last_effect = true;
            let nested =
                annotate_effect_sequence_with_env_internal(effects, nested_env, config, id_gen)?;
            let mut out_env = nested.final_env;
            out_env.last_effect_id = env.last_effect_id.clone();
            out_env.bind_unbound_x_to_last_effect = env.bind_unbound_x_to_last_effect;
            Ok(out_env)
        }
        _ => {
            let mut frame = env.to_frame(
                auto_tag_object_targets,
                config.force_auto_tag_object_targets,
            );
            advance_reference_frame_for_effect(effect, id_gen, &mut frame)?;
            Ok(ReferenceEnv::from_frame(&frame))
        }
    }
}

fn resolve_effect_result_values_in_fields(
    effect: &mut EffectAst,
    state: EffectReferenceResolutionState,
) -> Result<(), CardTextError> {
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
        | EffectAst::Monstrosity { amount }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
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
        | EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            resolve_effect_result_value(amount, state)?;
        }
        EffectAst::PutCounters { count, .. } | EffectAst::PutCountersAll { count, .. } => {
            resolve_effect_result_value(count, state)?;
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            resolve_effect_result_value(put_count, state)?;
            resolve_effect_result_value(remove_count, state)?;
        }
        EffectAst::RemoveCountersAll { amount, .. } => {
            resolve_effect_result_value(amount, state)?;
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            if let Some(value) = life.as_mut() {
                resolve_effect_result_value(value, state)?;
            }
            if let Some(value) = additional_generic.as_mut() {
                resolve_effect_result_value(value, state)?;
            }
        }
        EffectAst::AddManaFromLandCouldProduce { amount, .. } => {
            resolve_effect_result_value(amount, state)?;
        }
        EffectAst::Discard { count, .. } => {
            resolve_effect_result_value(count, state)?;
        }
        EffectAst::CreateTokenWithMods { count, .. } => {
            resolve_effect_result_value(count, state)?;
        }
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
            resolve_effect_result_value(power, state)?;
            resolve_effect_result_value(toughness, state)?;
        }
        EffectAst::SetBasePower { power, .. } => {
            resolve_effect_result_value(power, state)?;
        }
        EffectAst::PumpForEach { count, .. } => {
            resolve_effect_result_value(count, state)?;
        }
        _ => {}
    }
    Ok(())
}

fn resolve_effect_result_value(
    value: &mut Value,
    state: EffectReferenceResolutionState,
) -> Result<(), CardTextError> {
    match value {
        Value::X if state.bind_unbound_x_to_last_effect => {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for X binding".to_string())
            })?;
            *value = Value::EffectValue(id);
        }
        Value::Add(left, right) => {
            resolve_effect_result_value(left, state)?;
            resolve_effect_result_value(right, state)?;
        }
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount)
            if !state.allow_life_event_value =>
        {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger or prior effect"
                        .to_string(),
                )
            })?;
            *value = Value::EffectValue(id);
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset)
            if !state.allow_life_event_value =>
        {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger or prior effect"
                        .to_string(),
                )
            })?;
            *value = Value::EffectValueOffset(id, *offset);
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn bind_unresolved_it_references_with_imports(
    effects: &[EffectAst],
    seed_last_object_tag: Option<&str>,
) -> BoundEffectsAst {
    let seed_tag = seed_last_object_tag
        .map(TagKey::from)
        .unwrap_or_else(|| TagKey::from(IT_TAG));
    let unresolved_it_before = count_unresolved_it_occurrences(effects);
    let mut resolved = effects.to_vec();
    for effect in &mut resolved {
        let _ = bind_unresolved_it_in_effect(effect, &seed_tag);
    }
    let unresolved_it_after = count_unresolved_it_occurrences(&resolved);
    BoundEffectsAst {
        effects: resolved,
        imports: ReferenceImports {
            last_object_tag: Some(seed_tag),
            ..Default::default()
        },
        unresolved_it_before,
        unresolved_it_after,
    }
}

#[cfg(test)]
fn count_unresolved_it_occurrences(effects: &[EffectAst]) -> usize {
    let mut cloned = effects.to_vec();
    let sentinel = TagKey::from("__count_unresolved_it__");
    cloned
        .iter_mut()
        .map(|effect| bind_unresolved_it_in_effect(effect, &sentinel))
        .sum()
}

#[cfg(test)]
fn bind_unresolved_it_in_effect(effect: &mut EffectAst, seed_tag: &TagKey) -> usize {
    let mut replacements = bind_unresolved_it_in_effect_fields(effect, seed_tag);
    for_each_nested_effects_mut(effect, true, |nested| {
        for inner in nested {
            replacements += bind_unresolved_it_in_effect(inner, seed_tag);
        }
    });
    replacements
}

#[cfg(test)]
fn bind_unresolved_it_in_effect_fields(effect: &mut EffectAst, seed_tag: &TagKey) -> usize {
    match effect {
        EffectAst::DealDamage { amount, target } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            bind_unresolved_it_in_target(source, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            bind_unresolved_it_in_target(creature1, seed_tag)
                + bind_unresolved_it_in_target(creature2, seed_tag)
        }
        EffectAst::FightIterated { creature2 } => bind_unresolved_it_in_target(creature2, seed_tag),
        EffectAst::DealDamageEach { amount, filter } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::Draw { count, .. } => bind_unresolved_it_in_value(count, seed_tag),
        EffectAst::Counter { target } => bind_unresolved_it_in_target(target, seed_tag),
        EffectAst::CounterUnlessPays { target, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PutCounters { count, target, .. } => {
            bind_unresolved_it_in_value(count, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            target,
            ..
        } => {
            bind_unresolved_it_in_value(put_count, seed_tag)
                + bind_unresolved_it_in_value(remove_count, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PutCountersAll { count, filter, .. } => {
            bind_unresolved_it_in_value(count, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::RemoveCountersAll { amount, filter, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::DoubleCountersOnEach { filter, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::PhaseOut { target }
        | EffectAst::RemoveFromCombat { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Connive { target }
        | EffectAst::Goad { target }
        | EffectAst::Transform { target }
        | EffectAst::Flip { target }
        | EffectAst::Regenerate { target }
        | EffectAst::ReturnToHand { target, .. }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToLibrarySecondFromTop { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::Destroy { target }
        | EffectAst::DestroyNoRegeneration { target }
        | EffectAst::Exile { target, .. }
        | EffectAst::ExileWhenSourceLeaves { target }
        | EffectAst::SacrificeSourceWhenLeaves { target }
        | EffectAst::ExileUntilSourceLeaves { target, .. }
        | EffectAst::GainControl { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::RemoveAbilitiesFromTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::SwitchPowerToughness { target, .. }
        | EffectAst::AddCardTypes { target, .. }
        | EffectAst::AddSubtypes { target, .. }
        | EffectAst::SetColors { target, .. }
        | EffectAst::MakeColorless { target, .. }
        | EffectAst::PumpByLastEffect { target, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::TapAll { filter }
        | EffectAst::UntapAll { filter }
        | EffectAst::DestroyAll { filter }
        | EffectAst::DestroyAllNoRegeneration { filter }
        | EffectAst::DestroyAllOfChosenColor { filter }
        | EffectAst::DestroyAllOfChosenColorNoRegeneration { filter }
        | EffectAst::ExileAll { filter, .. }
        | EffectAst::RegenerateAll { filter }
        | EffectAst::ReturnAllToHand { filter }
        | EffectAst::ReturnAllToHandOfChosenColor { filter }
        | EffectAst::ReturnAllToBattlefield { filter, .. }
        | EffectAst::Enchant { filter }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::RemoveAbilitiesAll { filter, .. }
        | EffectAst::GrantAbilitiesChoiceAll { filter, .. }
        | EffectAst::ForEachObject { filter, .. } => bind_unresolved_it_in_filter(filter, seed_tag),
        EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::PoisonCounters { count: amount, .. }
        | EffectAst::EnergyCounters { count: amount, .. }
        | EffectAst::Monstrosity { amount } => bind_unresolved_it_in_value(amount, seed_tag),
        EffectAst::PreventDamage { amount, target, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PreventNextTimeDamage { source, .. } => {
            bind_unresolved_it_in_prevent_next_source(source, seed_tag)
        }
        EffectAst::RedirectNextDamageFromSourceToTarget { amount, target } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::RedirectNextTimeDamageToSource { source, target } => {
            bind_unresolved_it_in_prevent_next_source(source, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PreventAllDamageToTarget { target, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PreventDamageEach { amount, filter, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. } => bind_unresolved_it_in_value(amount, seed_tag),
        EffectAst::AddManaFromLandCouldProduce {
            amount,
            land_filter,
            ..
        } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(land_filter, seed_tag)
        }
        EffectAst::ExileUntilMatchGrantPlayUntilEndOfTurn { filter, .. }
        | EffectAst::ExileUntilMatchCast { filter, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::Cant { restriction, .. } => {
            bind_unresolved_it_in_restriction(restriction, seed_tag)
        }
        EffectAst::GrantPlayTaggedUntilEndOfTurn { tag, .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            tag, ..
        }
        | EffectAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }
        | EffectAst::CastTagged { tag, .. }
        | EffectAst::RevealTagged { tag }
        | EffectAst::ReorderTopOfLibrary { tag }
        | EffectAst::MayByTaggedController { tag, .. }
        | EffectAst::ForEachTagged { tag, .. }
        | EffectAst::ForEachTaggedPlayer { tag, .. } => bind_unresolved_it_in_tag(tag, seed_tag),
        EffectAst::ControlPlayer { player, .. }
        | EffectAst::ForEachPlayersFiltered { filter: player, .. } => {
            bind_unresolved_it_in_player_filter(player, seed_tag)
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { filter, .. } => {
            if let Some(filter) = filter.as_mut() {
                bind_unresolved_it_in_filter(filter, seed_tag)
            } else {
                0
            }
        }
        EffectAst::LookAtTopCards { count, tag, .. } => {
            bind_unresolved_it_in_value(count, seed_tag) + bind_unresolved_it_in_tag(tag, seed_tag)
        }
        EffectAst::PutIntoHand { object, .. } => {
            bind_unresolved_it_in_object_ref_ast(object, seed_tag)
        }
        EffectAst::CopySpell { target, count, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::RetargetStackObject {
            target,
            mode,
            new_target_restriction,
            ..
        } => {
            let mut replacements = bind_unresolved_it_in_target(target, seed_tag);
            if let RetargetModeAst::OneToFixed { target } = mode {
                replacements += bind_unresolved_it_in_target(target, seed_tag);
            }
            if let Some(NewTargetRestrictionAst::Object(filter)) = new_target_restriction.as_mut() {
                replacements += bind_unresolved_it_in_filter(filter, seed_tag);
            }
            replacements
        }
        EffectAst::Conditional { predicate, .. } => {
            bind_unresolved_it_in_predicate(predicate, seed_tag)
        }
        EffectAst::ChooseObjects { filter, tag, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
                + bind_unresolved_it_in_tag(tag, seed_tag)
        }
        EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::ExchangeControl { filter, .. }
        | EffectAst::DestroyAllAttachedTo { filter, .. }
        | EffectAst::SearchLibrary { filter, .. } => bind_unresolved_it_in_filter(filter, seed_tag),
        EffectAst::Discard { count, filter, .. } => {
            let mut replacements = bind_unresolved_it_in_value(count, seed_tag);
            if let Some(filter) = filter.as_mut() {
                replacements += bind_unresolved_it_in_filter(filter, seed_tag);
            }
            replacements
        }
        EffectAst::MoveToZone {
            target,
            attached_to,
            ..
        } => {
            let mut replacements = bind_unresolved_it_in_target(target, seed_tag);
            if let Some(attach) = attached_to.as_mut() {
                replacements += bind_unresolved_it_in_target(attach, seed_tag);
            }
            replacements
        }
        EffectAst::CreateToken { count, .. } | EffectAst::Investigate { count } => {
            bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::CreateTokenCopy { object, count, .. } => {
            bind_unresolved_it_in_object_ref_ast(object, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::CreateTokenCopyFromSource { source, count, .. } => {
            bind_unresolved_it_in_target(source, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::CreateTokenWithMods {
            count, attached_to, ..
        } => {
            let mut replacements = bind_unresolved_it_in_value(count, seed_tag);
            if let Some(target) = attached_to.as_mut() {
                replacements += bind_unresolved_it_in_target(target, seed_tag);
            }
            replacements
        }
        EffectAst::RemoveUpToAnyCounters { amount, target, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::MoveAllCounters { from, to } => {
            bind_unresolved_it_in_target(from, seed_tag)
                + bind_unresolved_it_in_target(to, seed_tag)
        }
        EffectAst::Pump {
            power,
            toughness,
            target,
            ..
        }
        | EffectAst::SetBasePowerToughness {
            power,
            toughness,
            target,
            ..
        }
        | EffectAst::BecomeBasePtCreature {
            power,
            toughness,
            target,
            ..
        } => {
            bind_unresolved_it_in_value(power, seed_tag)
                + bind_unresolved_it_in_value(toughness, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::SetBasePower { power, target, .. } => {
            bind_unresolved_it_in_value(power, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PumpForEach { target, count, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::ForEachOpponentDid {
            predicate: Some(predicate),
            ..
        }
        | EffectAst::ForEachPlayerDid {
            predicate: Some(predicate),
            ..
        } => bind_unresolved_it_in_predicate(predicate, seed_tag),
        EffectAst::Attach { object, target } => {
            bind_unresolved_it_in_target(object, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_object_ref_ast(reference: &mut ObjectRefAst, seed_tag: &TagKey) -> usize {
    let ObjectRefAst::Tagged(tag) = reference;
    bind_unresolved_it_in_tag(tag, seed_tag)
}

#[cfg(test)]
fn bind_unresolved_it_in_tag(tag: &mut TagKey, seed_tag: &TagKey) -> usize {
    if tag.as_str() == IT_TAG {
        *tag = seed_tag.clone();
        1
    } else {
        0
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_runtime_object_ref(
    reference: &mut crate::filter::ObjectRef,
    seed_tag: &TagKey,
) -> usize {
    if let crate::filter::ObjectRef::Tagged(tag) = reference {
        bind_unresolved_it_in_tag(tag, seed_tag)
    } else {
        0
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_player_filter(filter: &mut PlayerFilter, seed_tag: &TagKey) -> usize {
    match filter {
        PlayerFilter::Target(inner) => bind_unresolved_it_in_player_filter(inner, seed_tag),
        PlayerFilter::Excluding { base, excluded } => {
            bind_unresolved_it_in_player_filter(base, seed_tag)
                + bind_unresolved_it_in_player_filter(excluded, seed_tag)
        }
        PlayerFilter::ControllerOf(reference) | PlayerFilter::OwnerOf(reference) => {
            bind_unresolved_it_in_runtime_object_ref(reference, seed_tag)
        }
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_filter(filter: &mut ObjectFilter, seed_tag: &TagKey) -> usize {
    let mut replacements = 0;
    for constraint in &mut filter.tagged_constraints {
        replacements += bind_unresolved_it_in_tag(&mut constraint.tag, seed_tag);
    }
    if let Some(owner) = filter.owner.as_mut() {
        replacements += bind_unresolved_it_in_player_filter(owner, seed_tag);
    }
    if let Some(controller) = filter.controller.as_mut() {
        replacements += bind_unresolved_it_in_player_filter(controller, seed_tag);
    }
    replacements
}

#[cfg(test)]
fn bind_unresolved_it_in_target(target: &mut TargetAst, seed_tag: &TagKey) -> usize {
    match target {
        TargetAst::Tagged(tag, _) => bind_unresolved_it_in_tag(tag, seed_tag),
        TargetAst::Object(filter, _, _) => bind_unresolved_it_in_filter(filter, seed_tag),
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            bind_unresolved_it_in_player_filter(filter, seed_tag)
        }
        TargetAst::WithCount(inner, _) => bind_unresolved_it_in_target(inner, seed_tag),
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_prevent_next_source(
    source: &mut PreventNextTimeDamageSourceAst,
    seed_tag: &TagKey,
) -> usize {
    if let PreventNextTimeDamageSourceAst::Filter(filter) = source {
        bind_unresolved_it_in_filter(filter, seed_tag)
    } else {
        0
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_choose_spec(spec: &mut ChooseSpec, seed_tag: &TagKey) -> usize {
    match spec {
        ChooseSpec::Tagged(tag) => bind_unresolved_it_in_tag(tag, seed_tag),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            bind_unresolved_it_in_choose_spec(inner, seed_tag)
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            bind_unresolved_it_in_player_filter(filter, seed_tag)
        }
        ChooseSpec::EachPlayer(filter) => bind_unresolved_it_in_player_filter(filter, seed_tag),
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_value(value: &mut Value, seed_tag: &TagKey) -> usize {
    match value {
        Value::Add(left, right) => {
            bind_unresolved_it_in_value(left, seed_tag)
                + bind_unresolved_it_in_value(right, seed_tag)
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::ColorsAmong(filter) => bind_unresolved_it_in_filter(filter, seed_tag),
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::CountersOn(spec, _) => bind_unresolved_it_in_choose_spec(spec, seed_tag),
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_predicate(predicate: &mut PredicateAst, seed_tag: &TagKey) -> usize {
    match predicate {
        PredicateAst::ItMatches(filter) | PredicateAst::TaggedMatches(_, filter) => {
            let mut replacements = bind_unresolved_it_in_filter(filter, seed_tag);
            if let PredicateAst::TaggedMatches(tag, _) = predicate {
                replacements += bind_unresolved_it_in_tag(tag, seed_tag);
            }
            replacements
        }
        PredicateAst::PlayerTaggedObjectMatches { tag, filter, .. } => {
            bind_unresolved_it_in_tag(tag, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        PredicateAst::PlayerControls { filter, .. }
        | PredicateAst::PlayerControlsAtLeast { filter, .. }
        | PredicateAst::PlayerControlsExactly { filter, .. }
        | PredicateAst::PlayerControlsAtLeastWithDifferentPowers { filter, .. }
        | PredicateAst::PlayerControlsNo { filter, .. }
        | PredicateAst::PlayerControlsMost { filter, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        PredicateAst::PlayerControlsOrHasCardInGraveyard {
            control_filter,
            graveyard_filter,
            ..
        } => {
            bind_unresolved_it_in_filter(control_filter, seed_tag)
                + bind_unresolved_it_in_filter(graveyard_filter, seed_tag)
        }
        PredicateAst::And(left, right) => {
            bind_unresolved_it_in_predicate(left, seed_tag)
                + bind_unresolved_it_in_predicate(right, seed_tag)
        }
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_restriction(
    restriction: &mut crate::effect::Restriction,
    seed_tag: &TagKey,
) -> usize {
    use crate::effect::Restriction;

    match restriction {
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
        | Restriction::ActivateNonManaAbilitiesOf(filter) => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        Restriction::BlockSpecificAttacker { blockers, attacker }
        | Restriction::MustBlockSpecificAttacker { blockers, attacker } => {
            bind_unresolved_it_in_filter(blockers, seed_tag)
                + bind_unresolved_it_in_filter(attacker, seed_tag)
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::TextSpan;
    use crate::cards::builders::IfResultPredicate;
    use crate::*;

    #[test]
    fn binding_reports_typed_unresolved_it_counts() {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        let effects = vec![EffectAst::DealDamage {
            amount: Value::Count(filter),
            target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
        }];

        let bound = bind_unresolved_it_references_with_imports(&effects, Some("bound_target"));
        assert_eq!(bound.unresolved_it_before, 2);
        assert_eq!(bound.unresolved_it_after, 0);
        assert_eq!(
            bound.imports.last_object_tag.as_ref().map(TagKey::as_str),
            Some("bound_target")
        );
        assert!(format!("{:?}", bound.effects).contains("bound_target"));
    }

    #[test]
    fn resolves_if_result_to_explicit_condition_and_binds_x() {
        let effects = vec![
            EffectAst::Investigate {
                count: Value::Fixed(1),
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![EffectAst::Investigate { count: Value::X }],
            },
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate if-result references");

        assert_eq!(annotated.effects[0].assigned_effect_id, Some(EffectId(0)));

        match &annotated.effects[1].effect {
            EffectAst::ResolvedIfResult {
                condition,
                predicate,
                effects,
            } => {
                assert_eq!(*condition, EffectId(0));
                assert_eq!(*predicate, IfResultPredicate::Did);
                assert_eq!(effects.len(), 1);
                match &effects[0] {
                    EffectAst::Investigate { count } => {
                        assert_eq!(count, &Value::EffectValue(EffectId(0)));
                    }
                    other => panic!("expected investigate follow-up, got {other:?}"),
                }
            }
            other => panic!("expected resolved if-result, got {other:?}"),
        }
    }

    #[test]
    fn annotate_effect_sequence_tracks_player_from_same_controller_filter() {
        let mut filter = ObjectFilter::creature();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameControllerAsTagged,
        });

        let effects = vec![
            EffectAst::ExileAll {
                filter,
                face_down: false,
            },
            EffectAst::RevealHand {
                player: PlayerAst::That,
            },
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports::with_last_object_tag("seeded"),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate same-controller follow-up");

        assert_eq!(
            annotated.effects[0].out_env.last_player_filter,
            RefState::Known(PlayerFilter::ControllerOf(ObjectRef::tagged("seeded")))
        );
        assert_eq!(
            annotated.effects[1].in_env.last_player_filter,
            RefState::Known(PlayerFilter::ControllerOf(ObjectRef::tagged("seeded")))
        );
    }

    #[test]
    fn resolves_event_amount_to_prior_effect_value_when_trigger_context_disallows_it() {
        let effects = vec![
            EffectAst::Investigate {
                count: Value::Fixed(1),
            },
            EffectAst::Draw {
                count: Value::EventValue(EventValueSpec::Amount),
                player: PlayerAst::You,
            },
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports::default(),
            EffectReferenceResolutionConfig {
                allow_life_event_value: false,
                ..Default::default()
            },
            IdGenContext::default(),
        )
        .expect("annotate event-derived amount");

        assert_eq!(annotated.effects[0].assigned_effect_id, Some(EffectId(0)));

        match &annotated.effects[1].effect {
            EffectAst::Draw { count, .. } => {
                assert_eq!(count, &Value::EffectValue(EffectId(0)));
            }
            other => panic!("expected draw effect, got {other:?}"),
        }
    }

    #[test]
    fn annotates_followup_effect_with_explicit_object_reference_frame() {
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
            },
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence metadata");

        assert_eq!(
            annotated.effects[1].in_env.last_object_tag,
            RefState::Known(TagKey::from("destroyed_0"))
        );
    }

    #[test]
    fn annotate_effect_sequence_sets_followup_in_env_from_destroyed_tag() {
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
            },
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert_eq!(
            annotated.effects[1].in_env.last_object_tag,
            RefState::Known(TagKey::from("destroyed_0"))
        );
    }

    #[test]
    fn annotate_effect_sequence_assigns_prior_effect_id_for_event_amount_followup() {
        let effects = vec![
            EffectAst::Investigate {
                count: Value::Fixed(1),
            },
            EffectAst::Draw {
                count: Value::EventValue(EventValueSpec::Amount),
                player: PlayerAst::You,
            },
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert_eq!(annotated.effects[0].assigned_effect_id, Some(EffectId(0)));
        match &annotated.effects[1].effect {
            EffectAst::Draw { count, .. } => {
                assert_eq!(count, &Value::EffectValue(EffectId(0)));
            }
            other => panic!("expected draw effect, got {other:?}"),
        }
    }

    #[test]
    fn annotate_effect_sequence_joins_conditional_last_object_tag_when_branches_agree() {
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::Unmodeled("test".to_string()),
            if_true: Vec::new(),
            if_false: Vec::new(),
        }];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports {
                last_object_tag: Some(TagKey::from("seeded")),
                ..Default::default()
            },
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert_eq!(
            annotated.final_env.last_object_tag,
            RefState::Known(TagKey::from("seeded"))
        );
    }

    #[test]
    fn annotate_effect_sequence_marks_conditional_last_object_tag_ambiguous_when_branches_diverge()
    {
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::Unmodeled("test".to_string()),
            if_true: vec![
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
                },
            ],
            if_false: vec![
                EffectAst::Exile {
                    target: TargetAst::Object(
                        ObjectFilter::creature(),
                        Some(TextSpan::synthetic()),
                        None,
                    ),
                    face_down: false,
                },
                EffectAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: TagKey::from(IT_TAG),
                    player: PlayerAst::You,
                    allow_land: false,
                },
            ],
        }];

        let annotated = annotate_effect_sequence(
            &effects,
            &ReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert!(matches!(
            annotated.final_env.last_object_tag,
            RefState::Ambiguous
        ));
    }
}
