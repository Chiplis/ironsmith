use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IdGenContext, PlayerAst, PredicateAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TargetAst, TriggerSpec,
};
use crate::effect::{EffectId, EventValueSpec};
use crate::filter::{Comparison, TaggedOpbjectRelation};
use crate::target::ChooseSpec;
use crate::target::ObjectRef;
use crate::{ObjectFilter, PlayerFilter, Value};
use ironsmith_core::{EffectMetric, EffectMetricSource};

#[cfg(test)]
use crate::TagKey;
#[cfg(test)]
use crate::cards::builders::{ObjectRefAst, PreventNextTimeDamageSourceAst, RetargetModeAst};
#[cfg(test)]
use crate::types::Subtype;

use super::compile_support::{
    effect_references_event_derived_amount, effects_reference_it_tag,
    effects_reference_its_controller, effects_reference_tag, value_references_event_derived_amount,
};
#[cfg(test)]
use super::effect_ast_traversal::for_each_nested_effects_mut;
use super::effect_ast_traversal::{for_each_nested_effects, try_for_each_nested_effects_mut};
use super::reference_helpers::{
    choose_spec_targets_object, infer_player_filter_from_object_filter, is_you_player_filter,
    object_filter_as_tagged_reference, resolve_it_tag, resolve_non_target_player_filter,
    resolve_target_spec_with_choices,
};
use super::reference_model::{
    AnnotatedEffect, AnnotatedEffectSequence, RefState, ReferenceEnv, ReferenceFrame,
    ReferenceImports,
};

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
    last_library_search_effect_id: Option<EffectId>,
    allow_life_event_value: bool,
    bind_unbound_x_to_last_effect: bool,
}

fn trigger_supports_event_amount(trigger: &TriggerSpec) -> bool {
    matches!(
        trigger,
        TriggerSpec::YouGainLife
            | TriggerSpec::YouGainLifeDuringTurn(_)
            | TriggerSpec::PlayerLosesLife(_)
            | TriggerSpec::PlayerLosesLifeDuringTurn { .. }
            | TriggerSpec::ThisIsDealtDamage
            | TriggerSpec::ThisIsDealtCombatDamage
            | TriggerSpec::IsDealtDamage(_)
            | TriggerSpec::IsDealtCombatDamage(_)
            | TriggerSpec::ThisDealsDamage
            | TriggerSpec::ThisDealsDamageTo(_)
            | TriggerSpec::DealsDamage(_)
            | TriggerSpec::ThisDealsDamageToPlayer { .. }
            | TriggerSpec::DealsDamageToPlayer { .. }
            | TriggerSpec::DealsNoncombatDamageToPlayer { .. }
            | TriggerSpec::ThisDealsCombatDamage
            | TriggerSpec::ThisDealsCombatDamageTo(_)
            | TriggerSpec::DealsCombatDamage(_)
            | TriggerSpec::DealsCombatDamageTo { .. }
            | TriggerSpec::ThisDealsCombatDamageToPlayer
            | TriggerSpec::DealsCombatDamageToPlayer { .. }
            | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
            | TriggerSpec::AttacksOneOrMore(_)
            | TriggerSpec::AttacksOneOrMoreWithMinTotal { .. }
            | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_)
            | TriggerSpec::CounterPutOn { .. }
            | TriggerSpec::EntersBattlefieldOneOrMore { .. }
    ) || matches!(
        trigger,
        TriggerSpec::Either(left, right)
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

fn lowering_reference_frame(frame: &ReferenceFrame) -> ReferenceEnv {
    ReferenceEnv::from_frame(frame)
}

fn next_reference_tag(id_gen: &mut IdGenContext, prefix: &str) -> String {
    let tag = if matches!(prefix, "exiled" | "looked" | "chosen" | "revealed") {
        format!("__sentence_helper_{prefix}_l0_s0_e{}", id_gen.next_tag_id)
    } else {
        format!("{prefix}_{}", id_gen.next_tag_id)
    };
    id_gen.next_tag_id += 1;
    tag
}

fn generated_object_result_tag_prefix(effect: &EffectAst) -> Option<&'static str> {
    match effect {
        EffectAst::SubjectVerb(subject_verb)
            if matches!(&subject_verb.action, SubjectVerbActionAst::Mill { .. }) =>
        {
            Some("milled")
        }
        EffectAst::SubjectVerb(subject_verb)
            if matches!(&subject_verb.action, SubjectVerbActionAst::Discover { .. }) =>
        {
            Some("discovered")
        }
        _ => None,
    }
}

fn target_is_any_damage_target(target: &TargetAst) -> bool {
    match target {
        TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_) => true,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_is_any_damage_target(inner)
        }
        _ => false,
    }
}

fn maybe_tag_generated_object_results(
    effect: &EffectAst,
    frame: &mut ReferenceFrame,
    id_gen: &mut IdGenContext,
) {
    if frame.auto_tag_object_targets
        && let Some(prefix) = generated_object_result_tag_prefix(effect)
    {
        frame.last_object_tag = Some(next_reference_tag(id_gen, prefix));
    }
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
    let preserve_existing_non_you = matches!(player, PlayerAst::You)
        && frame
            .last_player_filter
            .as_ref()
            .is_some_and(|existing| !is_you_player_filter(existing));
    if !preserve_existing_non_you {
        frame.last_player_filter = Some(filter);
    }
    Ok(())
}

fn predicate_bound_player_filter(predicate: &PredicateAst) -> Option<PlayerFilter> {
    match predicate {
        PredicateAst::PlayerWouldBeginExtraTurn { player } => match player {
            PlayerAst::Opponent => Some(PlayerFilter::Opponent),
            _ => None,
        },
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_bound_player_filter(left).or_else(|| predicate_bound_player_filter(right))
        }
        PredicateAst::Not(inner) => predicate_bound_player_filter(inner),
        _ => None,
    }
}

fn track_target_player(target: &TargetAst, frame: &mut ReferenceFrame) {
    match target {
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            frame.last_player_filter = Some(if matches!(filter, PlayerFilter::IteratedPlayer) {
                frame
                    .last_player_filter
                    .clone()
                    .unwrap_or(PlayerFilter::IteratedPlayer)
            } else {
                PlayerFilter::Target(Box::new(filter.clone()))
            });
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
        if filter.tagged_constraints.iter().any(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::SameControllerAsTagged
            )
        }) {
            frame.last_player_filter =
                Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(tag)));
            return;
        }
        if filter.controller.is_some() {
            frame.last_player_filter = Some(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)));
            return;
        }
    }
    if let Some(player_filter) = infer_player_filter_from_object_filter(filter) {
        frame.last_player_filter = Some(player_filter);
    }
}

fn chooser_bound_followup_player_filter(
    filter: &ObjectFilter,
    chooser: Option<&PlayerFilter>,
) -> Option<PlayerFilter> {
    let inferred = infer_player_filter_from_object_filter(filter);
    if inferred
        .as_ref()
        .is_some_and(PlayerFilter::mentions_iterated_player)
    {
        chooser.cloned().or(inferred)
    } else {
        inferred.or_else(|| chooser.cloned())
    }
}

fn should_alias_followup_player_to_chosen_owner(
    filter: &ObjectFilter,
    chooser: Option<&PlayerFilter>,
) -> bool {
    filter.zone == Some(crate::zone::Zone::Graveyard)
        && filter.owner == Some(PlayerFilter::Opponent)
        && chooser.map_or(true, is_you_player_filter)
}

fn maybe_tag_target(
    target: &TargetAst,
    frame: &mut ReferenceFrame,
    id_gen: &mut IdGenContext,
    prefix: &str,
) -> Result<(), CardTextError> {
    let refs = lowering_reference_frame(frame);
    let (spec, _) = resolve_target_spec_with_choices(target, &refs)?;
    if matches!(spec.base(), ChooseSpec::Source) {
        frame.source_object_antecedent = true;
    }
    if frame.auto_tag_object_targets
        && let Some(tag) = propagated_or_generated_object_tag(&spec, id_gen, prefix)
    {
        frame.last_object_tag = Some(tag);
    }
    track_target_player(target, frame);
    Ok(())
}

fn propagated_or_generated_object_tag(
    spec: &ChooseSpec,
    id_gen: &mut IdGenContext,
    prefix: &str,
) -> Option<String> {
    if !choose_spec_targets_object(spec) {
        return None;
    }

    match spec.base() {
        ChooseSpec::Tagged(tag) => Some(tag.as_str().to_string()),
        ChooseSpec::Object(_) | ChooseSpec::SpecificObject(_) | ChooseSpec::Source => {
            Some(next_reference_tag(id_gen, prefix))
        }
        _ => None,
    }
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
        EffectAst::Sequence { effects } => {
            advance_reference_frames(effects, id_gen, frame)?;
        }
        EffectAst::SubjectVerb(subject_verb) => {
            track_effect_player(subject_verb.subject.player, frame, true, true)?;
            match &subject_verb.action {
                SubjectVerbActionAst::Mill { .. } | SubjectVerbActionAst::Discover { .. } => {
                    maybe_tag_generated_object_results(effect, frame, id_gen);
                }
                SubjectVerbActionAst::Populate { .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "created"));
                    }
                }
                SubjectVerbActionAst::Amass { .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "amassed"));
                    }
                }
                SubjectVerbActionAst::Explore { target } => {
                    maybe_tag_target(target, frame, id_gen, "explored")?;
                }
                SubjectVerbActionAst::Endure { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "endured")?;
                }
                SubjectVerbActionAst::Connive { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "connived")?;
                }
                SubjectVerbActionAst::GrantProtectionChoice { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "protected")?;
                }
                SubjectVerbActionAst::PreventAllCombatDamageFromSource { source, .. } => {
                    maybe_tag_target(source, frame, id_gen, "targeted")?;
                }
                SubjectVerbActionAst::RetargetStackObject { .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "retargeted"));
                    }
                }
                SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestIntoGraveyard { .. }
                | SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestOnBottomOfLibrary {
                    ..
                } => {
                    track_effect_player(subject_verb.subject.player, frame, true, true)?;
                    frame.last_object_tag = Some(next_reference_tag(id_gen, "revealed"));
                }
                SubjectVerbActionAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard { .. }
                | SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary { .. }
                | SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary { .. }
                | SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary { .. }
                | SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary { .. } => {
                    track_effect_player(subject_verb.subject.player, frame, true, true)?;
                    frame.last_object_tag = Some(next_reference_tag(id_gen, "chosen"));
                }
                SubjectVerbActionAst::DealDamage { target, .. }
                | SubjectVerbActionAst::DealDistributedDamage { target, .. }
                | SubjectVerbActionAst::DealDamageEqualToPower { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "damaged")?;
                    if target_is_any_damage_target(target) {
                        if frame.auto_tag_object_targets {
                            frame.last_object_tag = Some(next_reference_tag(id_gen, "damaged"));
                        }
                        frame.last_player_filter = Some(PlayerFilter::DamagedPlayer);
                    }
                }
                SubjectVerbActionAst::Tap { target } => {
                    maybe_tag_target(target, frame, id_gen, "tapped")?;
                }
                SubjectVerbActionAst::Untap { target } => {
                    maybe_tag_target(target, frame, id_gen, "untapped")?;
                }
                SubjectVerbActionAst::TapAll { filter } => {
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::UntapAll { filter } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "untapped"));
                    }
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::TapOrUntapAll {
                    tap_filter,
                    untap_filter,
                } => {
                    track_player_from_object_filter(tap_filter, frame);
                    track_player_from_object_filter(untap_filter, frame);
                }
                SubjectVerbActionAst::TapOrUntap { target } => {
                    maybe_tag_target(target, frame, id_gen, "tap_or_untap")?;
                }
                SubjectVerbActionAst::PhaseOut { target } => {
                    maybe_tag_target(target, frame, id_gen, "phased_out")?;
                }
                SubjectVerbActionAst::PhaseOutAll { filter } => {
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::PhaseIn { target } => {
                    maybe_tag_target(target, frame, id_gen, "phased_in")?;
                }
                SubjectVerbActionAst::PhaseInAll { filter } => {
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::Transform { target } => {
                    maybe_tag_target(target, frame, id_gen, "transformed")?;
                }
                SubjectVerbActionAst::Convert { target } => {
                    maybe_tag_target(target, frame, id_gen, "converted")?;
                }
                SubjectVerbActionAst::Destroy { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "destroyed")?;
                }
                SubjectVerbActionAst::DestroyAll { filter, .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "destroyed"));
                    }
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::DestroyAllOfChosenColor { filter, .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "destroyed"));
                    }
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::Exile { target, .. } => {
                    let refs = lowering_reference_frame(frame);
                    let (spec, _) = resolve_target_spec_with_choices(target, &refs)?;
                    if matches!(spec.base(), ChooseSpec::Source) {
                        frame.source_object_antecedent = true;
                    }
                    if frame.auto_tag_object_targets {
                        if spec.is_target() {
                            if let Some(tag) =
                                propagated_or_generated_object_tag(&spec, id_gen, "exiled")
                            {
                                frame.last_object_tag = Some(tag);
                            }
                        } else if choose_spec_targets_object(&spec) {
                            frame.last_object_tag = Some(crate::tag::SOURCE_EXILED_TAG.to_string());
                        }
                    }
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::ExileAll { filter, .. } => {
                    let keep_last_object_tag =
                        filter.tagged_constraints.iter().any(|constraint| {
                            matches!(constraint.relation, TaggedOpbjectRelation::SameNameAsTagged)
                        });
                    if frame.auto_tag_object_targets && !keep_last_object_tag {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "exiled"));
                    }
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::LookAtHand { target } => {
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::LookAtTarget { target } => {
                    maybe_tag_target(target, frame, id_gen, "targeted")?;
                }
                SubjectVerbActionAst::Counter { target }
                | SubjectVerbActionAst::CounterUnlessPays { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "countered")?;
                }
                SubjectVerbActionAst::PutCounters { target, .. }
                | SubjectVerbActionAst::PutCounterChoice { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "counters")?;
                }
                SubjectVerbActionAst::RemoveUpToAnyCounters { target, .. }
                | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target } => {
                    maybe_tag_target(target, frame, id_gen, "counters")?;
                }
                SubjectVerbActionAst::MoveAllCounters { from, to }
                | SubjectVerbActionAst::MoveOneCounter { from, to } => {
                    if frame.auto_tag_object_targets {
                        let _ = next_reference_tag(id_gen, "from");
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "to"));
                    }
                    track_target_player(from, frame);
                    track_target_player(to, frame);
                }
                SubjectVerbActionAst::ReturnToHand { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "returned")?;
                    if let Some(tag) = frame.last_object_tag.as_deref() {
                        frame.last_player_filter =
                            Some(PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(tag)));
                    }
                }
                SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter } => {
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::MoveToLibraryNthFromTop { target, .. } => {
                    let refs = lowering_reference_frame(frame);
                    let (spec, _) = resolve_target_spec_with_choices(target, &refs)?;
                    if frame.auto_tag_object_targets
                        && let Some(tag) =
                            propagated_or_generated_object_tag(&spec, id_gen, "moved")
                    {
                        frame.last_object_tag = Some(tag);
                    }
                }
                SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target } => {
                    let refs = lowering_reference_frame(frame);
                    let (spec, _) = resolve_target_spec_with_choices(target, &refs)?;
                    if frame.auto_tag_object_targets
                        && let Some(tag) =
                            propagated_or_generated_object_tag(&spec, id_gen, "moved")
                    {
                        frame.last_object_tag = Some(tag);
                    }
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::ShuffleObjectsIntoLibrary { target } => {
                    maybe_tag_target(target, frame, id_gen, "moved")?;
                }
                SubjectVerbActionAst::PutSticker { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "stickered")?;
                }
                SubjectVerbActionAst::SwitchPowerToughness { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "switched_pt")?;
                }
                SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. } => {
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::Detain { target } => {
                    maybe_tag_target(target, frame, id_gen, "detained")?;
                }
                SubjectVerbActionAst::Goad { target } => {
                    maybe_tag_target(target, frame, id_gen, "goaded")?;
                }
                SubjectVerbActionAst::Suspect { target } => {
                    maybe_tag_target(target, frame, id_gen, "suspected")?;
                }
                SubjectVerbActionAst::ClearSuspected { target: Some(target) } => {
                    maybe_tag_target(target, frame, id_gen, "no_longer_suspected")?;
                }
                SubjectVerbActionAst::ClearSuspected { target: None } => {}
                SubjectVerbActionAst::RemoveFromCombat { target } => {
                    maybe_tag_target(target, frame, id_gen, "removed_from_combat")?;
                }
                SubjectVerbActionAst::Flip { target } => {
                    maybe_tag_target(target, frame, id_gen, "targeted")?;
                }
                SubjectVerbActionAst::Regenerate { target } => {
                    maybe_tag_target(target, frame, id_gen, "returned")?;
                }
                SubjectVerbActionAst::RegenerateAll { filter } => {
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::Discard { tag, .. } => {
                    frame.last_object_tag = Some(
                        tag.as_ref()
                            .map(|tag| tag.as_str().to_string())
                            .unwrap_or_else(|| next_reference_tag(id_gen, "discarded")),
                    );
                }
                SubjectVerbActionAst::Sacrifice {
                    filter,
                    count,
                    target,
                } => {
                    if target.is_some() {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "sacrificed"));
                    } else if filter.source {
                        // Source sacrifice lowers directly to SacrificeTargetEffect(Source) and
                        // does not materialize a new tagged object reference.
                    } else {
                        let refs = lowering_reference_frame(frame);
                        let resolved_filter = match resolve_it_tag(filter, &refs) {
                            Ok(resolved) => resolved,
                            Err(_)
                                if filter.tagged_constraints.len() == 1
                                    && filter.tagged_constraints[0].tag.as_str() == IT_TAG =>
                            {
                                ObjectFilter::source()
                            }
                            Err(err) => return Err(err),
                        };
                        if !(*count == 1
                            && object_filter_as_tagged_reference(&resolved_filter).is_some())
                        {
                            frame.last_object_tag =
                                Some(next_reference_tag(id_gen, "sacrificed"));
                        }
                    }
                }
                SubjectVerbActionAst::SacrificeAll { .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "sacrificed"));
                    }
                }
                SubjectVerbActionAst::ChoosePlayer { tag, .. } => {
                    frame.last_player_filter = Some(PlayerFilter::TaggedPlayer(tag.clone()));
                    frame
                        .recent_player_choice_tags
                        .push(tag.as_str().to_string());
                }
                SubjectVerbActionAst::ControlPlayer { player, .. } => {
                    frame.last_player_filter = Some(player.clone());
                }
                SubjectVerbActionAst::ChooseCardName { tag, .. } => {
                    frame.last_object_tag = Some(tag.as_str().to_string());
                }
                SubjectVerbActionAst::ChooseSpellCastHistory { filter, tag, .. } => {
                    track_player_from_object_filter(filter, frame);
                    frame.last_object_tag = Some(tag.as_str().to_string());
                }
                SubjectVerbActionAst::ExchangeLifeTotals { player2 } => {
                    track_effect_player(*player2, frame, true, true)?;
                }
                SubjectVerbActionAst::ExchangeTextBoxes { target } => {
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::ExchangeControl { filter, .. } => {
                    frame.last_object_tag = Some(next_reference_tag(id_gen, "exchanged"));
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::ExchangeControlHeterogeneous {
                    permanent1,
                    permanent2,
                    ..
                } => {
                    frame.last_object_tag = Some(next_reference_tag(id_gen, "exchanged"));
                    track_target_player(permanent1, frame);
                    track_target_player(permanent2, frame);
                }
                SubjectVerbActionAst::Attach { object, target } => {
                    track_target_player(object, frame);
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::ExileWhenSourceLeaves { target }
                | SubjectVerbActionAst::SacrificeSourceWhenLeaves { target } => {
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::MayMoveToZone { target, .. } => {
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::RegisterZoneReplacement { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "replaced")?;
                }
                SubjectVerbActionAst::DestroyAllAttachedTo { filter, .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "destroyed"));
                    }
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::ExchangeValues { left, right, .. } => {
                    match left {
                        crate::cards::builders::ExchangeValueAst::LifeTotal(player) => {
                            track_effect_player(*player, frame, true, true)?;
                        }
                        crate::cards::builders::ExchangeValueAst::Stat { target, .. } => {
                            track_target_player(target, frame);
                        }
                    }
                    match right {
                        crate::cards::builders::ExchangeValueAst::LifeTotal(player) => {
                            track_effect_player(*player, frame, true, true)?;
                        }
                        crate::cards::builders::ExchangeValueAst::Stat { target, .. } => {
                            track_target_player(target, frame);
                        }
                    }
                }
                SubjectVerbActionAst::GainControl { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "controlled")?;
                }
                SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { target, .. }
                | SubjectVerbActionAst::RedirectNextTimeDamageToSource { target, .. }
                | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                    source: target,
                }
                | SubjectVerbActionAst::PreventDamage { target, .. }
                | SubjectVerbActionAst::PreventAllDamageToTarget { target, .. }
                | SubjectVerbActionAst::PreventDamageToTargetPutCounters { target, .. }
                | SubjectVerbActionAst::PutOrRemoveCounters { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "targeted")?;
                }
                SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "exiled")?;
                }
                SubjectVerbActionAst::ReturnToBattlefield { target, .. } => {
                    let refs = lowering_reference_frame(frame);
                    let (spec, _) = resolve_target_spec_with_choices(target, &refs)?;
                    if frame.auto_tag_object_targets
                        && let Some(tag) =
                            propagated_or_generated_object_tag(&spec, id_gen, "returned")
                    {
                        frame.last_object_tag = Some(tag);
                    }
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::CopySpell { target, player, .. }
                | SubjectVerbActionAst::CopySpellForEachTarget { target, player, .. } => {
                    let _ = target;
                    track_effect_player(player.clone(), frame, true, true)?;
                }
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { player, .. } => {
                    track_effect_player(*player, frame, true, true)?;
                }
                SubjectVerbActionAst::CastTagged { player, .. } => {
                    track_effect_player(*player, frame, true, true)?;
                }
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { player, .. }
                | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { player, .. }
                | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { player, .. } => {
                    track_effect_player(*player, frame, true, true)?;
                }
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                    player,
                    ..
                } => {
                    track_effect_player(*player, frame, true, true)?;
                }
                SubjectVerbActionAst::RevealHand => {
                    frame.last_object_tag = None;
                }
                SubjectVerbActionAst::RevealTop => {
                    frame.last_object_tag = Some(next_reference_tag(id_gen, "revealed"));
                }
                SubjectVerbActionAst::ExileTopOfLibrary { tags, .. } => {
                    if let Some(tag) = tags.first() {
                        frame.last_object_tag = Some(if tag.as_str() == IT_TAG {
                            next_reference_tag(id_gen, "exiled")
                        } else {
                            tag.as_str().to_string()
                        });
                    }
                }
                SubjectVerbActionAst::RevealCardsFromHand { tag, .. } => {
                    frame.last_object_tag = Some(if tag.as_str() == IT_TAG {
                        next_reference_tag(id_gen, "revealed")
                    } else {
                        tag.as_str().to_string()
                    });
                }
                SubjectVerbActionAst::RevealTagged { tag } => {
                    frame.last_object_tag = Some(if tag.as_str() == IT_TAG {
                        frame
                            .last_object_tag
                            .clone()
                            .unwrap_or_else(|| next_reference_tag(id_gen, "revealed"))
                    } else {
                        tag.as_str().to_string()
                    });
                }
                SubjectVerbActionAst::LookAtTopCards { tag, .. } => {
                    frame.last_object_tag = Some(if tag.as_str() == IT_TAG {
                        next_reference_tag(id_gen, "revealed")
                    } else {
                        tag.as_str().to_string()
                    });
                }
                SubjectVerbActionAst::MoveToZone { target, .. } => {
                    let refs = lowering_reference_frame(frame);
                    let (spec, _) = resolve_target_spec_with_choices(target, &refs)?;
                    if frame.auto_tag_object_targets
                        && let Some(tag) =
                            propagated_or_generated_object_tag(&spec, id_gen, "moved")
                    {
                        frame.last_object_tag = Some(tag);
                    }
                    track_target_player(target, frame);
                }
                SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "returned"));
                    }
                    track_player_from_object_filter(filter, frame);
                }
                SubjectVerbActionAst::TargetOnly { target } => {
                    maybe_tag_target(target, frame, id_gen, "targeted")?;
                }
                SubjectVerbActionAst::TagMatchingObjects { filter, tag, .. } => {
                    track_player_from_object_filter(filter, frame);
                    frame.last_object_tag = Some(tag.as_str().to_string());
                }
                SubjectVerbActionAst::Pump { target, .. }
                | SubjectVerbActionAst::PumpForEach { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "pumped")?;
                }
                SubjectVerbActionAst::SetBasePowerToughness { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "set_base_pt")?;
                }
                SubjectVerbActionAst::BecomeBasePtCreature { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "animated_creature")?;
                }
                SubjectVerbActionAst::SetBasePower { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "set_base_power")?;
                }
                SubjectVerbActionAst::AddCardTypes { target, .. }
                | SubjectVerbActionAst::RemoveCardTypes { target, .. }
                | SubjectVerbActionAst::BecomeAuraEnchantment { target, .. }
                | SubjectVerbActionAst::BecomeBasicLandType { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "typed")?;
                }
                SubjectVerbActionAst::AddSubtypes { target, .. }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily { target, .. }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "subtyped")?;
                }
                SubjectVerbActionAst::AddColors { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "colored")?;
                }
                SubjectVerbActionAst::SetColors { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "set_colors")?;
                }
                SubjectVerbActionAst::MakeColorless { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "set_colorless")?;
                }
                SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "become_basic_land_type")?;
                }
                SubjectVerbActionAst::BecomeCreatureTypeChoice { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "become_creature_type_choice")?;
                }
                SubjectVerbActionAst::BecomeColorChoice { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "become_color_choice")?;
                }
                SubjectVerbActionAst::BecomeCopy { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "copied")?;
                }
                SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
                | SubjectVerbActionAst::GrantToTarget { target, .. }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. } => {
                    maybe_tag_target(target, frame, id_gen, "targeted")?;
                }
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    player, match_tag, ..
                } => {
                    track_effect_player(*player, frame, true, true)?;
                    frame.last_object_tag = Some(match_tag.as_str().to_string());
                }
                SubjectVerbActionAst::SearchLibrary { player, .. } => {
                    track_effect_player(*player, frame, true, true)?;
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "searched"));
                    }
                }
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. } => {
                    track_effect_player(*player, frame, true, true)?;
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "created"));
                    }
                }
                SubjectVerbActionAst::CreateTokenWithMods {
                    player,
                    attached_to,
                    dynamic_power_toughness,
                    ..
                } => {
                    track_effect_player(*player, frame, true, true)?;
                    if frame.auto_tag_object_targets
                        || attached_to.is_some()
                        || dynamic_power_toughness.is_some()
                    {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "created"));
                    }
                    if frame.auto_tag_object_targets && attached_to.is_some() {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "attachment_target"));
                    }
                }
                SubjectVerbActionAst::PumpAll { .. } => {
                    if frame.auto_tag_object_targets {
                        frame.last_object_tag = Some(next_reference_tag(id_gen, "pumped"));
                    }
                }
                _ => {}
            }
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
            let resolved_filter = resolve_it_tag(filter, &refs).ok();
            if let Some(player_filter) = if references_revealed_hand {
                frame.last_player_filter.clone()
            } else {
                None
            }
            .or_else(|| {
                resolved_filter.as_ref().and_then(|resolved| {
                    chooser_bound_followup_player_filter(resolved, chooser_filter.as_ref())
                })
            })
            .or_else(|| chooser_bound_followup_player_filter(filter, chooser_filter.as_ref()))
            {
                frame.last_player_filter = Some(player_filter);
            }
            let chosen_tag = tag.as_str().to_string();
            if resolved_filter.as_ref().is_some_and(|resolved_filter| {
                should_alias_followup_player_to_chosen_owner(
                    resolved_filter,
                    chooser_filter.as_ref(),
                )
            }) {
                frame.last_player_filter = Some(PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(
                    chosen_tag.as_str(),
                )));
            }
            frame.last_object_tag = Some(chosen_tag);
        }
        EffectAst::ChooseObjectsAcrossZones {
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
            let resolved_filter = resolve_it_tag(filter, &refs).ok();
            if let Some(player_filter) = if references_revealed_hand {
                frame.last_player_filter.clone()
            } else {
                None
            }
            .or_else(|| {
                resolved_filter.as_ref().and_then(|resolved| {
                    chooser_bound_followup_player_filter(resolved, chooser_filter.as_ref())
                })
            })
            .or_else(|| chooser_bound_followup_player_filter(filter, chooser_filter.as_ref()))
            {
                frame.last_player_filter = Some(player_filter);
            }
            let chosen_tag = tag.as_str().to_string();
            if resolved_filter.as_ref().is_some_and(|resolved_filter| {
                should_alias_followup_player_to_chosen_owner(
                    resolved_filter,
                    chooser_filter.as_ref(),
                )
            }) {
                frame.last_player_filter = Some(PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(
                    chosen_tag.as_str(),
                )));
            }
            frame.last_object_tag = Some(chosen_tag);
        }
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost { .. } => {}
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
        EffectAst::DelayedUntilNextUpkeep { player, effects }
        | EffectAst::DelayedUntilNextDrawStep { player, effects }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects } => {
            advance_effects_preserving_last_effect(&effects, id_gen, frame)?;
            track_effect_player(player.clone(), frame, true, true)?;
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
            ..
        } => {
            let saved = frame.clone();
            let mut true_frame = saved.clone();
            if let Some(player_filter) = predicate_bound_player_filter(predicate) {
                true_frame.last_player_filter = Some(player_filter);
            }
            advance_reference_frames(&if_true, id_gen, &mut true_frame)?;
            if if_false.is_empty() {
                *frame = true_frame;
            } else {
                let mut false_frame = saved.clone();
                if let Some(player_filter) = predicate_bound_player_filter(predicate) {
                    false_frame.last_player_filter = Some(player_filter);
                }
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
            if saved.last_object_tag != nested.last_object_tag {
                frame.last_object_tag = nested.last_object_tag;
            }
            if saved.last_player_filter != nested.last_player_filter {
                frame.last_player_filter = nested.last_player_filter;
            }
        }
        EffectAst::ForEachTagged { tag, effects } => {
            let tagged_object = if tag.as_str() == IT_TAG {
                frame.last_object_tag.clone()
            } else {
                Some(tag.as_str().to_string())
            };
            advance_effects_in_iterated_player_context(&effects, id_gen, frame, tagged_object)?;
        }
        EffectAst::RepeatProcess { effects, .. } => {
            advance_effects_preserving_last_effect(&effects, id_gen, frame)?;
        }
        EffectAst::RepeatEffects { effects, .. } => {
            advance_effects_preserving_last_effect(&effects, id_gen, frame)?;
        }
        EffectAst::BidLife { winner_effects, .. } => {
            advance_effects_preserving_last_effect(winner_effects, id_gen, frame)?;
        }
        EffectAst::ManaRestricted { effects, .. } => {
            advance_reference_frames(effects, id_gen, frame)?;
        }
        EffectAst::RepeatThisProcess
        | EffectAst::RepeatThisProcessMay
        | EffectAst::RepeatThisProcessOnce
        | EffectAst::UnlessPays { .. }
        | EffectAst::UnlessAction { .. }
        | EffectAst::IfResult { .. }
        | EffectAst::WhenResult { .. }
        | EffectAst::ForEachOpponentDoesNot { .. }
        | EffectAst::ForEachPlayerDoesNot { .. }
        | EffectAst::ForEachOpponentDid { .. }
        | EffectAst::ForEachPlayerDid { .. }
        | EffectAst::DirectionalAdjacentPlayerControl { .. }
        | EffectAst::VoteStart { .. }
        | EffectAst::VoteStartObjects { .. }
        | EffectAst::VoteOption { .. }
        | EffectAst::VoteExtra { .. } => {}
    }

    Ok(())
}

fn effect_reference_resolution_state(env: &ReferenceEnv) -> EffectReferenceResolutionState {
    EffectReferenceResolutionState {
        last_effect_id: env.last_effect_id.clone().into_option(),
        last_library_search_effect_id: env.last_library_search_effect_id.clone().into_option(),
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
        let suppress_for_power_self_damage =
            preserves_existing_it_for_power_self_damage_followup(&effect, remaining.first());
        let auto_tag_object_targets = if suppress_for_power_self_damage {
            false
        } else {
            effects_reference_it_tag(remaining)
                || effects_reference_its_controller(remaining)
                || effects_reference_tag(remaining, "damaged_0")
        };
        let assigned_effect_id =
            maybe_assign_effect_result_id(effects, idx, id_gen, in_env.allow_life_event_value);

        let mut out_env = advance_reference_env_for_effect(
            &effect,
            &in_env,
            config,
            id_gen,
            auto_tag_object_targets,
            suppress_for_power_self_damage,
        )?;
        if let Some(id) = assigned_effect_id
            && !matches!(
                effect,
                EffectAst::ResolvedIfResult { .. }
                    | EffectAst::ResolvedWhenResult { .. }
                    | EffectAst::IfResult { .. }
                    | EffectAst::WhenResult { .. }
            )
        {
            out_env.last_effect_id = RefState::Known(id);
        }
        if let Some(id) = assigned_effect_id
            && effect_is_library_search(&effect)
        {
            out_env.last_library_search_effect_id = RefState::Known(id);
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

pub(crate) fn preserves_existing_it_for_power_self_damage_followup(
    effect: &EffectAst,
    next_effect: Option<&EffectAst>,
) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::DealDamageEqualToPower {
                target: TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_),
                ..
            },
        ..
    }) = effect
    else {
        return false;
    };

    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::DealDamageEqualToPower {
                source: TargetAst::Tagged(source_tag, _),
                target: TargetAst::Tagged(target_tag, _),
            },
        ..
    })) = next_effect
    else {
        return false;
    };

    source_tag.as_str() == IT_TAG && target_tag.as_str() == IT_TAG
}

fn maybe_assign_effect_result_id(
    effects: &[EffectAst],
    idx: usize,
    id_gen: &mut IdGenContext,
    _allow_life_event_value: bool,
) -> Option<EffectId> {
    let next_is_result_gate = idx + 1 < effects.len()
        && matches!(
            effects[idx + 1],
            EffectAst::IfResult { .. }
                | EffectAst::WhenResult { .. }
                | EffectAst::ResolvedIfResult { .. }
                | EffectAst::ResolvedWhenResult { .. }
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
    let next_needs_event_derived_amount = idx + 1 < effects.len()
        && effect_can_supply_event_derived_amount_for(&effects[idx], &effects[idx + 1]);
    let later_needs_event_derived_amount = effect_can_supply_prior_effect_memory(&effects[idx])
        && idx + 1 < effects.len()
        && effects[idx + 1..]
            .iter()
            .any(|later| effect_can_supply_event_derived_amount_for(&effects[idx], later));
    let next_needs_prior_effect_value = idx + 1 < effects.len()
        && matches!(
            &effects[idx + 1],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    subject_verb.action,
                    SubjectVerbActionAst::PumpByLastEffect { .. }
                )
        );
    let later_needs_library_search_result = effect_is_library_search(&effects[idx])
        && idx + 1 < effects.len()
        && effects[idx + 1..]
            .iter()
            .any(effect_is_searched_library_gate);

    if !(next_is_if_result_with_opponent_doesnt
        || next_is_if_result_with_player_doesnt
        || next_is_if_result_with_opponent_did
        || next_is_if_result_with_player_did
        || next_is_result_gate
        || next_needs_event_derived_amount
        || later_needs_event_derived_amount
        || next_needs_prior_effect_value
        || later_needs_library_search_result)
    {
        return None;
    }

    let id = EffectId(id_gen.next_effect_id);
    id_gen.next_effect_id += 1;
    Some(id)
}

fn effect_is_searched_library_gate(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::SearchedLibrary,
            ..
        } | EffectAst::WhenResult {
            predicate: crate::cards::builders::IfResultPredicate::SearchedLibrary,
            ..
        }
    )
}

fn effect_is_library_search(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::ChooseObjectsAcrossZones {
            zones, search_mode, ..
        } => search_mode.is_some() && zones.contains(&crate::zone::Zone::Library),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SearchLibrary { shuffle, .. },
            ..
        }) => *shuffle,
        _ => false,
    }
}

fn effect_can_supply_prior_effect_memory(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => matches!(
            subject_verb.action,
            SubjectVerbActionAst::Destroy { .. }
                | SubjectVerbActionAst::DestroyAll { .. }
                | SubjectVerbActionAst::DestroyAllOfChosenColor { .. }
                | SubjectVerbActionAst::Exile { .. }
                | SubjectVerbActionAst::ExileAll { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::SacrificeAll { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::SearchLibrary { .. }
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealTagged { .. }
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::LookAtTopCards { .. }
                | SubjectVerbActionAst::Draw { .. }
        ),
        EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. } => {
            effects.iter().any(effect_can_supply_prior_effect_memory)
        }
        _ => false,
    }
}

fn effect_can_supply_event_derived_amount_for(effect: &EffectAst, consumer: &EffectAst) -> bool {
    if !effect_references_event_derived_amount(consumer) {
        return false;
    }
    if effect_references_only_other_number_metric(consumer) {
        return matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RollDiceChooseResult { .. },
                ..
            })
        );
    }
    true
}

fn effect_references_only_other_number_metric(effect: &EffectAst) -> bool {
    let mut saw_other_number = false;
    let mut saw_other_event_value = false;
    visit_effect_values(effect, &mut |value| {
        if value_references_only_other_number_metric(value) {
            saw_other_number = true;
        } else if value_references_event_derived_amount(value) {
            saw_other_event_value = true;
        }
    });
    saw_other_number && !saw_other_event_value
}

fn value_references_only_other_number_metric(value: &Value) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } => value_references_only_other_number_metric(value),
        Value::PendingEffectMetric {
            metric: EffectMetric::OtherNumber,
            ..
        }
        | Value::PendingEffectMetricOffset {
            metric: EffectMetric::OtherNumber,
            ..
        } => true,
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_only_other_number_metric(left)
                || value_references_only_other_number_metric(right)
        }
        Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => {
            value_references_only_other_number_metric(value)
        }
        _ => false,
    }
}

fn visit_effect_values(effect: &EffectAst, visit: &mut impl FnMut(&Value)) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            visit_subject_verb_action_values(&subject_verb.action, visit);
        }
        _ => {}
    }
    for_each_nested_effects(effect, true, |nested| {
        for nested_effect in nested {
            visit_effect_values(nested_effect, visit);
        }
    });
}

fn visit_filter_values(filter: &ObjectFilter, visit: &mut impl FnMut(&Value)) {
    for comparison in [
        filter.power.as_ref(),
        filter.toughness.as_ref(),
        filter.mana_value.as_ref(),
        filter.color_count.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        visit_comparison_values(comparison, visit);
    }
    for child in &filter.any_of {
        visit_filter_values(child, visit);
    }
}

fn visit_comparison_values(comparison: &crate::filter::Comparison, visit: &mut impl FnMut(&Value)) {
    match comparison {
        crate::filter::Comparison::EqualExpr(value)
        | crate::filter::Comparison::NotEqualExpr(value)
        | crate::filter::Comparison::LessThanExpr(value)
        | crate::filter::Comparison::LessThanOrEqualExpr(value)
        | crate::filter::Comparison::GreaterThanExpr(value)
        | crate::filter::Comparison::GreaterThanOrEqualExpr(value) => visit(value),
        _ => {}
    }
}

fn visit_subject_verb_action_values(action: &SubjectVerbActionAst, visit: &mut impl FnMut(&Value)) {
    match action {
        SubjectVerbActionAst::Draw { count }
        | SubjectVerbActionAst::Mill { count }
        | SubjectVerbActionAst::ExileTopOfLibrary { count, .. }
        | SubjectVerbActionAst::Scry { count }
        | SubjectVerbActionAst::Surveil { count }
        | SubjectVerbActionAst::Proliferate { count }
        | SubjectVerbActionAst::Investigate { count }
        | SubjectVerbActionAst::Discover { count }
        | SubjectVerbActionAst::Fateseal { count }
        | SubjectVerbActionAst::Populate { count, .. }
        | SubjectVerbActionAst::Connive { count, .. }
        | SubjectVerbActionAst::CreateTokenCopy { count, .. }
        | SubjectVerbActionAst::CreateTokenCopyFromSource { count, .. }
        | SubjectVerbActionAst::Monstrosity { amount: count }
        | SubjectVerbActionAst::LoseLife { amount: count }
        | SubjectVerbActionAst::GainLife { amount: count }
        | SubjectVerbActionAst::DealDamage { amount: count, .. }
        | SubjectVerbActionAst::DealDistributedDamage { amount: count, .. }
        | SubjectVerbActionAst::DealDamageEach { amount: count, .. }
        | SubjectVerbActionAst::PreventDamage { amount: count, .. }
        | SubjectVerbActionAst::PreventDamageEach { amount: count, .. }
        | SubjectVerbActionAst::CopySpell { count, .. }
        | SubjectVerbActionAst::PutCounters { count, .. }
        | SubjectVerbActionAst::PutCounterChoice { count, .. }
        | SubjectVerbActionAst::PutCountersAll { count, .. }
        | SubjectVerbActionAst::RemoveUpToAnyCounters { amount: count, .. }
        | SubjectVerbActionAst::RemoveCountersAll { amount: count, .. }
        | SubjectVerbActionAst::Discard { count, .. }
        | SubjectVerbActionAst::PoisonCounters { count }
        | SubjectVerbActionAst::EnergyCounters { count }
        | SubjectVerbActionAst::TicketCounters { count }
        | SubjectVerbActionAst::PayEnergy { amount: count }
        | SubjectVerbActionAst::SetLifeTotal { amount: count }
        | SubjectVerbActionAst::AddManaScaled { amount: count, .. }
        | SubjectVerbActionAst::AddManaAnyColor { amount: count, .. }
        | SubjectVerbActionAst::AddManaAnyOneColor { amount: count }
        | SubjectVerbActionAst::AddManaChosenColor { amount: count, .. }
        | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount: count, .. }
        | SubjectVerbActionAst::AddManaCommanderIdentity { amount: count }
        | SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount: count, .. }
        | SubjectVerbActionAst::LookAtTopCards { count, .. }
        | SubjectVerbActionAst::MoveToLibraryNthFromTop {
            position: count, ..
        }
        | SubjectVerbActionAst::AdditionalLandPlays { count, .. } => visit(count),
        SubjectVerbActionAst::Incubate { amount, count } => {
            visit(amount);
            visit(count);
        }
        SubjectVerbActionAst::CounterUnlessPays { .. } => {}
        SubjectVerbActionAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => {
            visit(amount);
        }
        SubjectVerbActionAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            visit(put_count);
            visit(remove_count);
        }
        SubjectVerbActionAst::Pump {
            power, toughness, ..
        }
        | SubjectVerbActionAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | SubjectVerbActionAst::BecomeBasePtCreature {
            power, toughness, ..
        }
        | SubjectVerbActionAst::PumpAll {
            power, toughness, ..
        } => {
            visit(power);
            visit(toughness);
        }
        SubjectVerbActionAst::SetBasePower { power, .. } => visit(power),
        SubjectVerbActionAst::PumpForEach { count, .. } => visit(count),
        SubjectVerbActionAst::ReturnToBattlefield {
            count_value: Some(count_value),
            ..
        } => visit(count_value),
        SubjectVerbActionAst::DestroyAll { filter, .. }
        | SubjectVerbActionAst::DestroyAllOfChosenColor { filter, .. }
        | SubjectVerbActionAst::ExileAll { filter, .. }
        | SubjectVerbActionAst::ReturnAllToHand { filter }
        | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter }
        | SubjectVerbActionAst::TapAll { filter }
        | SubjectVerbActionAst::UntapAll { filter }
        | SubjectVerbActionAst::PhaseOutAll { filter }
        | SubjectVerbActionAst::PhaseInAll { filter }
        | SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. }
        | SubjectVerbActionAst::SacrificeAll { filter }
        | SubjectVerbActionAst::RegenerateAll { filter }
        | SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. }
        | SubjectVerbActionAst::TagMatchingObjects { filter, .. }
        | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. }
        | SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. } => {
            visit_filter_values(filter, visit)
        }
        SubjectVerbActionAst::CreateTokenWithMods {
            count,
            dynamic_power_toughness,
            ..
        } => {
            visit(count);
            if let Some((power, toughness)) = dynamic_power_toughness {
                visit(power);
                visit(toughness);
            }
        }
        SubjectVerbActionAst::ConsultTopOfLibrary {
            stop_rule: crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value),
            ..
        } => visit(value),
        _ => {}
    }
}

fn resolve_effect_references_in_effect(
    mut effect: EffectAst,
    id_gen: &mut IdGenContext,
    state: EffectReferenceResolutionState,
) -> Result<EffectAst, CardTextError> {
    if let EffectAst::IfResult { predicate, effects } = effect {
        let condition = if matches!(
            predicate,
            crate::cards::builders::IfResultPredicate::SearchedLibrary
        ) {
            state.last_library_search_effect_id.or(state.last_effect_id)
        } else {
            state.last_effect_id
        }
        .ok_or_else(|| {
            CardTextError::ParseError("missing prior effect for if clause".to_string())
        })?;
        let effects = resolve_effect_sequence_references_with_state(
            &effects,
            id_gen,
            EffectReferenceResolutionState {
                last_effect_id: Some(condition),
                last_library_search_effect_id: state.last_library_search_effect_id,
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
                last_library_search_effect_id: state.last_library_search_effect_id,
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

    if let EffectAst::SubjectVerb(subject_verb) = &effect
        && let SubjectVerbActionAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
        } = &subject_verb.action
        && let Some(id) = state.last_effect_id
    {
        return Ok(EffectAst::subject_verb_pump(
            if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            },
            Value::Fixed(*toughness),
            target.clone(),
            duration.clone(),
            None,
        ));
    }

    if let EffectAst::DelayedTriggerThisTurn {
        trigger, effects, ..
    } = &mut effect
    {
        let nested_state = EffectReferenceResolutionState {
            last_effect_id: state.last_effect_id,
            last_library_search_effect_id: state.last_library_search_effect_id,
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
        let saved_last_effect_id = state.last_effect_id;
        let effect = resolve_effect_references_in_effect(effect.clone(), id_gen, state)?;
        let remaining = if idx + 1 < effects.len() {
            &effects[idx + 1..]
        } else {
            &[]
        };
        let _ = effects_reference_it_tag(remaining) || effects_reference_its_controller(remaining);
        let assigned_effect_id =
            maybe_assign_effect_result_id(effects, idx, id_gen, state.allow_life_event_value);
        state.last_effect_id = if matches!(
            effect,
            EffectAst::ResolvedIfResult { .. }
                | EffectAst::ResolvedWhenResult { .. }
                | EffectAst::IfResult { .. }
                | EffectAst::WhenResult { .. }
        ) {
            saved_last_effect_id
        } else {
            assigned_effect_id
        };
        if let Some(id) = assigned_effect_id
            && effect_is_library_search(&effect)
        {
            state.last_library_search_effect_id = Some(id);
        }
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
    suppress_force_auto_tag_object_targets: bool,
) -> Result<ReferenceEnv, CardTextError> {
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
            ..
        } => {
            let mut branch_env = env.clone();
            branch_env.source_object_antecedent |= predicate.establishes_source_object_antecedent();
            if let Some(player_filter) = predicate_bound_player_filter(predicate) {
                branch_env.last_player_filter = RefState::Known(player_filter);
            }
            let true_sequence = annotate_effect_sequence_with_env_internal(
                if_true,
                branch_env.clone(),
                config,
                id_gen,
            )?;
            if if_false.is_empty() {
                return Ok(true_sequence.final_env);
            }

            let false_sequence =
                annotate_effect_sequence_with_env_internal(if_false, branch_env, config, id_gen)?;
            Ok(ReferenceEnv {
                last_object_tag: RefState::join(
                    &true_sequence.final_env.last_object_tag,
                    &false_sequence.final_env.last_object_tag,
                ),
                last_it_choice_is_set: true_sequence.final_env.last_it_choice_is_set
                    && false_sequence.final_env.last_it_choice_is_set,
                last_player_filter: RefState::join(
                    &true_sequence.final_env.last_player_filter,
                    &false_sequence.final_env.last_player_filter,
                ),
                source_object_antecedent: true_sequence.final_env.source_object_antecedent
                    && false_sequence.final_env.source_object_antecedent,
                last_effect_id: env.last_effect_id.clone(),
                last_library_search_effect_id: env.last_library_search_effect_id.clone(),
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
                config.force_auto_tag_object_targets && !suppress_force_auto_tag_object_targets,
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
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Draw { count: amount }
            | SubjectVerbActionAst::ExileTopOfLibrary { count: amount, .. }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::Mill { count: amount }
            | SubjectVerbActionAst::Scry { count: amount }
            | SubjectVerbActionAst::Surveil { count: amount }
            | SubjectVerbActionAst::Proliferate { count: amount }
            | SubjectVerbActionAst::Investigate { count: amount }
            | SubjectVerbActionAst::Monstrosity { amount }
            | SubjectVerbActionAst::Discover { count: amount }
            | SubjectVerbActionAst::Fateseal { count: amount }
            | SubjectVerbActionAst::Populate { count: amount, .. }
            | SubjectVerbActionAst::Connive { count: amount, .. }
            | SubjectVerbActionAst::DealDamage { amount, .. }
            | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEach { amount, .. }
            | SubjectVerbActionAst::PreventDamage { amount, .. }
            | SubjectVerbActionAst::PreventDamageEach { amount, .. }
            | SubjectVerbActionAst::CopySpell { count: amount, .. }
            | SubjectVerbActionAst::PutCounters { count: amount, .. }
            | SubjectVerbActionAst::PutCounterChoice { count: amount, .. }
            | SubjectVerbActionAst::PutCountersAll { count: amount, .. }
            | SubjectVerbActionAst::RemoveUpToAnyCounters { amount, .. }
            | SubjectVerbActionAst::RemoveCountersAll { amount, .. }
            | SubjectVerbActionAst::Discard { count: amount, .. }
            | SubjectVerbActionAst::PoisonCounters { count: amount }
            | SubjectVerbActionAst::EnergyCounters { count: amount }
            | SubjectVerbActionAst::TicketCounters { count: amount }
            | SubjectVerbActionAst::PayEnergy { amount }
            | SubjectVerbActionAst::SetLifeTotal { amount }
            | SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount }
            | SubjectVerbActionAst::LookAtTopCards { count: amount, .. }
            | SubjectVerbActionAst::MoveToLibraryNthFromTop {
                position: amount, ..
            }
            | SubjectVerbActionAst::AdditionalLandPlays { count: amount, .. } => {
                resolve_effect_result_value(amount, state)?;
            }
            SubjectVerbActionAst::Incubate { amount, count } => {
                resolve_effect_result_value(amount, state)?;
                resolve_effect_result_value(count, state)?;
            }
            SubjectVerbActionAst::CounterUnlessPays { cost, .. } => {
                resolve_effect_result_values_in_total_cost(cost, state)?;
            }
            SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                amount: Some(amount),
                ..
            } => {
                resolve_effect_result_value(amount, state)?;
            }
            SubjectVerbActionAst::DealDamageEqualToPower { .. }
            | SubjectVerbActionAst::DrawForEachTaggedMatching { .. }
            | SubjectVerbActionAst::RevealHand
            | SubjectVerbActionAst::EmitKeywordAction { .. }
            | SubjectVerbActionAst::Amass { .. }
            | SubjectVerbActionAst::LookAtObjects { .. }
            | SubjectVerbActionAst::LookAtTarget { .. }
            | SubjectVerbActionAst::PutSomeIntoHandRestIntoGraveyard { .. }
            | SubjectVerbActionAst::PutSomeIntoHandRestOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::Bolster { .. }
            | SubjectVerbActionAst::Support { .. }
            | SubjectVerbActionAst::Adapt { .. }
            | SubjectVerbActionAst::Explore { .. }
            | SubjectVerbActionAst::Endure { .. }
            | SubjectVerbActionAst::Exploit
            | SubjectVerbActionAst::ConniveIterated
            | SubjectVerbActionAst::OpenAttraction
            | SubjectVerbActionAst::ManifestTopCardOfLibrary
            | SubjectVerbActionAst::ManifestCardFromHand
            | SubjectVerbActionAst::ManifestDread
            | SubjectVerbActionAst::Earthbend { .. }
            | SubjectVerbActionAst::Behold { .. }
            | SubjectVerbActionAst::Fight { .. }
            | SubjectVerbActionAst::FightIterated { .. }
            | SubjectVerbActionAst::Clash { .. }
            | SubjectVerbActionAst::FlipCoin
            | SubjectVerbActionAst::RollDie { .. }
            | SubjectVerbActionAst::RollDiceChooseResult { .. }
            | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
            | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
            | SubjectVerbActionAst::ReorderGraveyard
            | SubjectVerbActionAst::ChooseColor
            | SubjectVerbActionAst::ChooseCardType { .. }
            | SubjectVerbActionAst::ChooseNamedOption { .. }
            | SubjectVerbActionAst::ChooseCreatureType { .. }
            | SubjectVerbActionAst::ChooseCardName { .. }
            | SubjectVerbActionAst::ChoosePlayer { .. }
            | SubjectVerbActionAst::AddMana { .. }
            | SubjectVerbActionAst::ExchangeLifeTotals { .. }
            | SubjectVerbActionAst::ExchangeTextBoxes { .. }
            | SubjectVerbActionAst::ExchangeZones { .. }
            | SubjectVerbActionAst::PutRestOnBottomOfLibrary
            | SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn
            | SubjectVerbActionAst::ExchangeValues { .. }
            | SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn
            | SubjectVerbActionAst::ControlCombatChoicesThisTurn { .. }
            | SubjectVerbActionAst::GainControl { .. }
            | SubjectVerbActionAst::PutSticker { .. }
            | SubjectVerbActionAst::SwitchPowerToughness { .. }
            | SubjectVerbActionAst::RevealTop
            | SubjectVerbActionAst::RevealTagged { .. }
            | SubjectVerbActionAst::RevealCardsFromHand { .. }
            | SubjectVerbActionAst::AddManaColorsAmong { .. }
            | SubjectVerbActionAst::AddManaImprintedColors
            | SubjectVerbActionAst::DoubleManaPool
            | SubjectVerbActionAst::EmptyManaPool
            | SubjectVerbActionAst::EndTurn
            | SubjectVerbActionAst::SkipTurn
            | SubjectVerbActionAst::SkipCombatPhases
            | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
            | SubjectVerbActionAst::SkipMainPhasesThisTurn
            | SubjectVerbActionAst::SkipCombatPhasesThisTurn
            | SubjectVerbActionAst::SkipDrawStep
            | SubjectVerbActionAst::PlayFromGraveyardUntilEot
            | SubjectVerbActionAst::ControlPlayer { .. }
            | SubjectVerbActionAst::ReduceNextSpellCostThisTurn { .. }
            | SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn { .. }
            | SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { .. }
            | SubjectVerbActionAst::RingTemptsYou
            | SubjectVerbActionAst::VentureIntoDungeon { .. }
            | SubjectVerbActionAst::BecomeMonarch
            | SubjectVerbActionAst::TakeInitiative
            | SubjectVerbActionAst::CreateEmblem { .. }
            | SubjectVerbActionAst::LoseGame
            | SubjectVerbActionAst::WinGame
            | SubjectVerbActionAst::PayAnyEnergy { .. }
            | SubjectVerbActionAst::PayAnyLife { .. }
            | SubjectVerbActionAst::PayMana { .. }
            | SubjectVerbActionAst::DiscardHand
            | SubjectVerbActionAst::Detain { .. }
            | SubjectVerbActionAst::Goad { .. }
            | SubjectVerbActionAst::Suspect { .. }
            | SubjectVerbActionAst::ClearSuspected { .. }
            | SubjectVerbActionAst::RemoveFromCombat { .. }
            | SubjectVerbActionAst::Flip { .. }
            | SubjectVerbActionAst::Regenerate { .. }
            | SubjectVerbActionAst::RegenerateAll { .. }
            | SubjectVerbActionAst::TapAll { .. }
            | SubjectVerbActionAst::UntapAll { .. }
            | SubjectVerbActionAst::TapOrUntap { .. }
            | SubjectVerbActionAst::TapOrUntapAll { .. }
            | SubjectVerbActionAst::PhaseOut { .. }
            | SubjectVerbActionAst::PhaseOutAll { .. }
            | SubjectVerbActionAst::PhaseIn { .. }
            | SubjectVerbActionAst::PhaseInAll { .. }
            | SubjectVerbActionAst::Transform { .. }
            | SubjectVerbActionAst::Convert { .. }
            | SubjectVerbActionAst::Tap { .. }
            | SubjectVerbActionAst::Untap { .. }
            | SubjectVerbActionAst::Destroy { .. }
            | SubjectVerbActionAst::DestroyAll { .. }
            | SubjectVerbActionAst::DestroyAllOfChosenColor { .. }
            | SubjectVerbActionAst::Exile { .. }
            | SubjectVerbActionAst::ExileAll { .. }
            | SubjectVerbActionAst::LookAtHand { .. }
            | SubjectVerbActionAst::Counter { .. }
            | SubjectVerbActionAst::ReturnToHand { .. }
            | SubjectVerbActionAst::ReturnAllToHand { .. }
            | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { .. }
            | SubjectVerbActionAst::DoubleCountersOnEach { .. }
            | SubjectVerbActionAst::DoubleCountersOnTarget { .. }
            | SubjectVerbActionAst::MoveAllCounters { .. }
            | SubjectVerbActionAst::MoveOneCounter { .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { .. }
            | SubjectVerbActionAst::Sacrifice { .. }
            | SubjectVerbActionAst::SacrificeAll { .. }
            | SubjectVerbActionAst::PutIntoHand { .. }
            | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
            | SubjectVerbActionAst::RearrangeLookedCardsInLibrary { .. }
            | SubjectVerbActionAst::ReorderTopOfLibrary { .. }
            | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. }
            | SubjectVerbActionAst::ScalePowerToughnessAll { .. }
            | SubjectVerbActionAst::GrantProtectionChoice { .. }
            | SubjectVerbActionAst::PreventAllCombatDamage { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSource { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToPlayers { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToYou { .. }
            | SubjectVerbActionAst::PreventNextTimeDamage { .. }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget { .. }
            | SubjectVerbActionAst::PreventAllDamageToTarget { .. }
            | SubjectVerbActionAst::PreventAllDamageFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventDamageToTargetPutCounters { amount: None, .. }
            | SubjectVerbActionAst::Meld { .. }
            | SubjectVerbActionAst::SearchLibrarySlotsToHand { .. }
            | SubjectVerbActionAst::RevealTopChooseCardTypePutToHandRestBottom { .. }
            | SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestIntoGraveyard { .. }
            | SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard { .. }
            | SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::RetargetStackObject { .. }
            | SubjectVerbActionAst::GrantAbilityToSource { .. }
            | SubjectVerbActionAst::ExchangeControl { .. }
            | SubjectVerbActionAst::ExchangeControlHeterogeneous { .. }
            | SubjectVerbActionAst::DestroyAllAttachedTo { .. }
            | SubjectVerbActionAst::Attach { .. }
            | SubjectVerbActionAst::ExileWhenSourceLeaves { .. }
            | SubjectVerbActionAst::SacrificeSourceWhenLeaves { .. }
            | SubjectVerbActionAst::MayMoveToZone { .. }
            | SubjectVerbActionAst::RegisterZoneReplacement { .. }
            | SubjectVerbActionAst::RegisterFutureZoneReplacement { .. }
            | SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement { .. }
            | SubjectVerbActionAst::Enchant { .. }
            | SubjectVerbActionAst::ChooseSpellCastHistory { .. }
            | SubjectVerbActionAst::CopySpellForEachTarget { .. }
            | SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::CastTagged { .. }
            | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. }
            | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. }
            | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. }
            | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { .. }
            | SubjectVerbActionAst::MoveToZone { .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { .. }
            | SubjectVerbActionAst::TargetOnly { .. }
            | SubjectVerbActionAst::TagMatchingObjects { .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { .. }
            | SubjectVerbActionAst::PumpByLastEffect { .. }
            | SubjectVerbActionAst::AddCardTypes { .. }
            | SubjectVerbActionAst::RemoveCardTypes { .. }
            | SubjectVerbActionAst::AddSubtypes { .. }
            | SubjectVerbActionAst::AddColors { .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { .. }
            | SubjectVerbActionAst::BecomeAuraEnchantment { .. }
            | SubjectVerbActionAst::BecomeBasicLandType { .. }
            | SubjectVerbActionAst::SetColors { .. }
            | SubjectVerbActionAst::MakeColorless { .. }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice { .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { .. }
            | SubjectVerbActionAst::BecomeColorChoice { .. }
            | SubjectVerbActionAst::BecomeCopy { .. }
            | SubjectVerbActionAst::GrantAbilitiesAll { .. }
            | SubjectVerbActionAst::RemoveAbilitiesAll { .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceAll { .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { .. }
            | SubjectVerbActionAst::GrantToTarget { .. }
            | SubjectVerbActionAst::GrantBySpec { .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { .. }
            | SubjectVerbActionAst::SearchLibrary { count_value: None, .. }
            | SubjectVerbActionAst::Cant { .. }
            | SubjectVerbActionAst::ShuffleLibrary => {}
            SubjectVerbActionAst::CreateTokenCopy { count: amount, .. }
            | SubjectVerbActionAst::CreateTokenCopyFromSource { count: amount, .. } => {
                resolve_effect_result_value(amount, state)?;
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                count,
                dynamic_power_toughness: Some((power, toughness)),
                ..
            } => {
                resolve_effect_result_value(count, state)?;
                resolve_effect_result_value(power, state)?;
                resolve_effect_result_value(toughness, state)?;
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                count,
                dynamic_power_toughness: None,
                ..
            } => {
                resolve_effect_result_value(count, state)?;
            }
            SubjectVerbActionAst::ConsultTopOfLibrary { stop_rule, .. } => {
                if let crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value) =
                    stop_rule
                {
                    resolve_effect_result_value(value, state)?;
                }
            }
            SubjectVerbActionAst::SearchLibrary {
                count_value,
                library_position_from_top,
                ..
            } => {
                if let Some(count_value) = count_value {
                    resolve_effect_result_value(count_value, state)?;
                }
                if let Some(position) = library_position_from_top {
                    resolve_effect_result_value(position, state)?;
                }
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                count_value: Some(count_value),
                ..
            } => {
                resolve_effect_result_value(count_value, state)?;
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                count_value: None, ..
            } => {}
            SubjectVerbActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                ..
            } => {
                resolve_effect_result_value(put_count, state)?;
                resolve_effect_result_value(remove_count, state)?;
            }
            SubjectVerbActionAst::Pump {
                power, toughness, ..
            }
            | SubjectVerbActionAst::SetBasePowerToughness {
                power, toughness, ..
            }
            | SubjectVerbActionAst::PumpAll {
                power, toughness, ..
            } => {
                resolve_effect_result_value(power, state)?;
                resolve_effect_result_value(toughness, state)?;
            }
            SubjectVerbActionAst::SetBasePower { power, .. } => {
                resolve_effect_result_value(power, state)?;
            }
            SubjectVerbActionAst::PumpForEach { count, .. } => {
                resolve_effect_result_value(count, state)?;
            }
            SubjectVerbActionAst::Learn
            | SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { .. }
            | SubjectVerbActionAst::RegisterEnterUnderControlReplacement { .. } => {}
            SubjectVerbActionAst::AdditionalPhases { .. } => {}
        },
        EffectAst::ChooseObjects { count_value, .. }
        | EffectAst::ChooseObjectsAcrossZones { count_value, .. } => {
            if let Some(count_value) = count_value.as_mut() {
                resolve_effect_result_value(count_value, state)?;
            }
        }
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost { .. } => {}
        _ => {}
    }
    Ok(())
}

fn resolve_effect_result_values_in_total_cost(
    cost: &mut crate::cost::TotalCost,
    state: EffectReferenceResolutionState,
) -> Result<(), CardTextError> {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(_) => {
            let mut components = cost.costs().to_vec();
            for component in &mut components {
                resolve_effect_result_values_in_cost_component(component, state)?;
            }
            *cost = crate::cost::TotalCost::from_costs(components);
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut branches = branches.to_vec();
            for branch in &mut branches {
                resolve_effect_result_values_in_total_cost(branch, state)?;
            }
            *cost = crate::cost::TotalCost::one_of(branches);
        }
    }
    Ok(())
}

fn resolve_effect_result_values_in_cost_component(
    component: &mut crate::costs::Cost,
    state: EffectReferenceResolutionState,
) -> Result<(), CardTextError> {
    match component {
        crate::costs::Cost::DynamicMana(dynamic) => {
            if let Some(value) = dynamic.x_value.as_mut() {
                resolve_effect_result_value(value, state)?;
            }
            if let Some(value) = dynamic.additional_generic.as_mut() {
                resolve_effect_result_value(value, state)?;
            }
            if let Some(value) = dynamic.multiplier.as_mut() {
                resolve_effect_result_value(value, state)?;
            }
        }
        crate::costs::Cost::Energy(value)
        | crate::costs::Cost::Mill(value)
        | crate::costs::Cost::Life(value) => resolve_effect_result_value(value, state)?,
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
        Value::Add(left, right) | Value::Min(left, right) => {
            resolve_effect_result_value(left, state)?;
            resolve_effect_result_value(right, state)?;
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => {
            resolve_effect_result_value(inner, state)?;
        }
        Value::SurfaceHinted { value, .. } => {
            resolve_effect_result_value(value, state)?;
        }
        Value::PendingEffectMetric { source, metric } => {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError(
                    "pending effect metric requires a prior memory-producing effect".to_string(),
                )
            })?;
            *value = Value::EffectMetric {
                effect_id: id,
                source: *source,
                metric: *metric,
            };
        }
        Value::PendingEffectMetricOffset {
            source,
            metric,
            offset,
        } => {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError(
                    "pending effect metric requires a prior memory-producing effect".to_string(),
                )
            })?;
            *value = Value::EffectMetricOffset {
                effect_id: id,
                source: *source,
                metric: *metric,
                offset: *offset,
            };
        }
        Value::EventValue(EventValueSpec::Amount) if !state.allow_life_event_value => {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger or prior effect"
                        .to_string(),
                )
            })?;
            *value = Value::EffectValue(id);
        }
        Value::EventValue(EventValueSpec::LifeAmount) if !state.allow_life_event_value => {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger or prior effect"
                        .to_string(),
                )
            })?;
            *value = Value::EffectMetric {
                effect_id: id,
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::LifeLost,
            };
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
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
        Value::EventValueOffset(EventValueSpec::LifeAmount, offset)
            if !state.allow_life_event_value =>
        {
            let id = state.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger or prior effect"
                        .to_string(),
                )
            })?;
            *value = Value::EffectMetricOffset {
                effect_id: id,
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::LifeLost,
                offset: *offset,
            };
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
    let nested_seed = match effect {
        EffectAst::ForEachObject { .. } => TagKey::from(IT_TAG),
        _ => seed_tag.clone(),
    };
    for_each_nested_effects_mut(effect, true, |nested| {
        for inner in nested {
            replacements += bind_unresolved_it_in_effect(inner, &nested_seed);
        }
    });
    replacements
}

#[cfg(test)]
fn bind_unresolved_it_in_effect_fields(effect: &mut EffectAst, seed_tag: &TagKey) -> usize {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Draw { count }
            | SubjectVerbActionAst::Mill { count }
            | SubjectVerbActionAst::Scry { count }
            | SubjectVerbActionAst::Surveil { count }
            | SubjectVerbActionAst::Proliferate { count }
            | SubjectVerbActionAst::Investigate { count }
            | SubjectVerbActionAst::Discover { count }
            | SubjectVerbActionAst::Fateseal { count }
            | SubjectVerbActionAst::Populate { count, .. } => {
                bind_unresolved_it_in_value(count, seed_tag)
            }
            SubjectVerbActionAst::Incubate { amount, count } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_value(count, seed_tag)
            }
            SubjectVerbActionAst::Monstrosity { amount } => {
                bind_unresolved_it_in_value(amount, seed_tag)
            }
            SubjectVerbActionAst::RevealHand
            | SubjectVerbActionAst::ConniveIterated
            | SubjectVerbActionAst::EmitKeywordAction { .. }
            | SubjectVerbActionAst::Exploit
            | SubjectVerbActionAst::Amass { .. }
            | SubjectVerbActionAst::Bolster { .. }
            | SubjectVerbActionAst::Support { .. }
            | SubjectVerbActionAst::Adapt { .. }
            | SubjectVerbActionAst::OpenAttraction
            | SubjectVerbActionAst::ManifestTopCardOfLibrary
            | SubjectVerbActionAst::ManifestCardFromHand
            | SubjectVerbActionAst::ManifestDread
            | SubjectVerbActionAst::Earthbend { .. }
            | SubjectVerbActionAst::Behold { .. }
            | SubjectVerbActionAst::Clash { .. }
            | SubjectVerbActionAst::FlipCoin
            | SubjectVerbActionAst::RollDie { .. }
            | SubjectVerbActionAst::RollDiceChooseResult { .. }
            | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
            | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
            | SubjectVerbActionAst::ReorderGraveyard
            | SubjectVerbActionAst::ChooseColor
            | SubjectVerbActionAst::ChooseCardType { .. }
            | SubjectVerbActionAst::ChooseNamedOption { .. }
            | SubjectVerbActionAst::ChooseCreatureType { .. }
            | SubjectVerbActionAst::AddManaColorsAmong { .. }
            | SubjectVerbActionAst::AddManaImprintedColors
            | SubjectVerbActionAst::DoubleManaPool
            | SubjectVerbActionAst::EmptyManaPool
            | SubjectVerbActionAst::EndTurn
            | SubjectVerbActionAst::SkipTurn
            | SubjectVerbActionAst::SkipCombatPhases
            | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
            | SubjectVerbActionAst::SkipMainPhasesThisTurn
            | SubjectVerbActionAst::SkipCombatPhasesThisTurn
            | SubjectVerbActionAst::SkipDrawStep
            | SubjectVerbActionAst::PlayFromGraveyardUntilEot
            | SubjectVerbActionAst::RingTemptsYou
            | SubjectVerbActionAst::VentureIntoDungeon { .. }
            | SubjectVerbActionAst::BecomeMonarch
            | SubjectVerbActionAst::TakeInitiative
            | SubjectVerbActionAst::CreateEmblem { .. }
            | SubjectVerbActionAst::LoseGame
            | SubjectVerbActionAst::WinGame
            | SubjectVerbActionAst::PayAnyEnergy { .. }
            | SubjectVerbActionAst::PayAnyLife { .. }
            | SubjectVerbActionAst::PayMana { .. }
            | SubjectVerbActionAst::DiscardHand => 0,
            SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::PayEnergy { amount }
            | SubjectVerbActionAst::SetLifeTotal { amount } => {
                bind_unresolved_it_in_value(amount, seed_tag)
            }
            SubjectVerbActionAst::PoisonCounters { count }
            | SubjectVerbActionAst::EnergyCounters { count }
            | SubjectVerbActionAst::TicketCounters { count } => {
                bind_unresolved_it_in_value(count, seed_tag)
            }
            SubjectVerbActionAst::DealDamage { amount, target }
            | SubjectVerbActionAst::DealDistributedDamage { amount, target } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::ExileTopOfLibrary {
                count,
                tags,
                accumulated_tags,
            } => {
                let mut replacements = bind_unresolved_it_in_value(count, seed_tag);
                for tag in tags {
                    replacements += bind_unresolved_it_in_tag(tag, seed_tag);
                }
                for tag in accumulated_tags {
                    replacements += bind_unresolved_it_in_tag(tag, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::DrawForEachTaggedMatching { tag, filter } => {
                bind_unresolved_it_in_tag(tag, seed_tag)
                    + bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::DealDamageEach { amount, filter } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::PutCountersAll { count, filter, .. } => {
                bind_unresolved_it_in_value(count, seed_tag)
                    + bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::RemoveCountersAll { amount, filter, .. } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::DestroyAll { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::DestroyAllOfChosenColor { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::TapAll { filter }
            | SubjectVerbActionAst::UntapAll { filter }
            | SubjectVerbActionAst::PhaseOutAll { filter }
            | SubjectVerbActionAst::PhaseInAll { filter }
            | SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            } => {
                bind_unresolved_it_in_filter(tap_filter, seed_tag)
                    + bind_unresolved_it_in_filter(untap_filter, seed_tag)
            }
            SubjectVerbActionAst::ExileAll { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::ReturnAllToHand { filter } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::MoveToLibraryNthFromTop { target, position } => {
                bind_unresolved_it_in_target(target, seed_tag)
                    + bind_unresolved_it_in_value(position, seed_tag)
            }
            SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::DealDamageEqualToPower { source, target } => {
                bind_unresolved_it_in_target(source, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
            } => {
                bind_unresolved_it_in_target(creature1, seed_tag)
                    + bind_unresolved_it_in_target(creature2, seed_tag)
            }
            SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::GainControl { target, .. }
            | SubjectVerbActionAst::TapOrUntap { target }
            | SubjectVerbActionAst::PhaseOut { target }
            | SubjectVerbActionAst::PhaseIn { target }
            | SubjectVerbActionAst::Transform { target }
            | SubjectVerbActionAst::Convert { target }
            | SubjectVerbActionAst::Explore { target }
            | SubjectVerbActionAst::Endure { target, .. }
            | SubjectVerbActionAst::Connive { target, .. }
            | SubjectVerbActionAst::FightIterated { creature2: target }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::LookAtHand { target }
            | SubjectVerbActionAst::Counter { target }
            | SubjectVerbActionAst::CounterUnlessPays { target, .. }
            | SubjectVerbActionAst::ReturnToHand { target, .. }
            | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { target }
            | SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::SwitchPowerToughness { target, .. }
            | SubjectVerbActionAst::Detain { target }
            | SubjectVerbActionAst::Goad { target }
            | SubjectVerbActionAst::Suspect { target }
            | SubjectVerbActionAst::RemoveFromCombat { target }
            | SubjectVerbActionAst::Flip { target }
            | SubjectVerbActionAst::Regenerate { target } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::ClearSuspected {
                target: Some(target),
            } => bind_unresolved_it_in_target(target, seed_tag),
            SubjectVerbActionAst::ClearSuspected { target: None } => 0,
            SubjectVerbActionAst::RegenerateAll { filter } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::RevealTagged { tag }
            | SubjectVerbActionAst::RevealCardsFromHand { tag, .. } => {
                bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::RearrangeLookedCardsInLibrary { tag, .. }
            | SubjectVerbActionAst::ReorderTopOfLibrary { tag } => {
                bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::PutCounters { count, target, .. }
            | SubjectVerbActionAst::PutCounterChoice { count, target, .. } => {
                bind_unresolved_it_in_value(count, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::RemoveUpToAnyCounters { amount, target, .. } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::MoveAllCounters { from, to }
            | SubjectVerbActionAst::MoveOneCounter { from, to } => {
                bind_unresolved_it_in_target(from, seed_tag)
                    + bind_unresolved_it_in_target(to, seed_tag)
            }
            SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::Discard { count, filter, .. } => {
                let mut replacements = bind_unresolved_it_in_value(count, seed_tag);
                if let Some(filter) = filter.as_mut() {
                    replacements += bind_unresolved_it_in_filter(filter, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount }
            | SubjectVerbActionAst::AdditionalLandPlays { count: amount, .. } => {
                bind_unresolved_it_in_value(amount, seed_tag)
            }
            SubjectVerbActionAst::LookAtTopCards { count, tag, .. } => {
                bind_unresolved_it_in_value(count, seed_tag)
                    + bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::LookAtObjects { filter } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::LookAtTarget { target } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::PutIntoHand { object } => {
                bind_unresolved_it_in_object_ref_ast(object, seed_tag)
            }
            SubjectVerbActionAst::PutSomeIntoHandRestIntoGraveyard { .. }
            | SubjectVerbActionAst::PutSomeIntoHandRestOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::PutRestOnBottomOfLibrary
            | SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn => 0,
            SubjectVerbActionAst::MayMoveToZone { target, .. }
            | SubjectVerbActionAst::GrantProtectionChoice { target, .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSource { source: target, .. }
            | SubjectVerbActionAst::ExileWhenSourceLeaves { target }
            | SubjectVerbActionAst::SacrificeSourceWhenLeaves { target }
            | SubjectVerbActionAst::RegisterZoneReplacement { target, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::RegisterFutureZoneReplacement { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::DestroyAllAttachedTo { filter, target } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::Attach { object, target } => {
                bind_unresolved_it_in_target(object, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::Enchant {
                filter: crate::object::AuraAttachmentFilter::Object(filter),
            } => bind_unresolved_it_in_filter(filter, seed_tag),
            SubjectVerbActionAst::Enchant {
                filter: crate::object::AuraAttachmentFilter::Player(_),
            } => 0,
            SubjectVerbActionAst::ChooseSpellCastHistory { filter, tag, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
                    + bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                ..
            } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_filter(land_filter, seed_tag)
            }
            SubjectVerbActionAst::Sacrifice { filter, target, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
                    + target
                        .as_mut()
                        .map(|target| bind_unresolved_it_in_target(target, seed_tag))
                        .unwrap_or(0)
            }
            SubjectVerbActionAst::SacrificeAll { filter } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::DoubleCountersOnEach { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::DoubleCountersOnTarget { target, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::ChooseCardName { filter, tag } => {
                filter
                    .as_mut()
                    .map(|filter| bind_unresolved_it_in_filter(filter, seed_tag))
                    .unwrap_or(0)
                    + bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::ChoosePlayer { tag, .. } => {
                bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::ControlPlayer { player, .. } => {
                bind_unresolved_it_in_player_filter(player, seed_tag)
            }
            SubjectVerbActionAst::ReduceNextSpellCostThisTurn { filter, .. }
            | SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::PreventNextTimeDamage { source, .. } => {
                bind_unresolved_it_in_prevent_next_source(source, seed_tag)
            }
            SubjectVerbActionAst::SearchLibrarySlotsToHand {
                slots,
                progress_tag,
                ..
            } => {
                let mut replacements = bind_unresolved_it_in_tag(progress_tag, seed_tag);
                for slot in slots {
                    replacements += bind_unresolved_it_in_filter(&mut slot.filter, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
                filter, ..
            }
            | SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestOnBottomOfLibrary {
                filter,
                ..
            }
            | SubjectVerbActionAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
                filter, ..
            } => bind_unresolved_it_in_filter(filter, seed_tag),
            SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
                spell_filter,
                ..
            } => bind_unresolved_it_in_filter(spell_filter, seed_tag),
            SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary {
                ..
            } => 0,
            SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
                battlefield_filter,
                ..
            } => bind_unresolved_it_in_filter(battlefield_filter, seed_tag),
            SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
                battlefield_filter,
                hand_filter,
                ..
            } => {
                bind_unresolved_it_in_filter(battlefield_filter, seed_tag)
                    + bind_unresolved_it_in_filter(hand_filter, seed_tag)
            }
            SubjectVerbActionAst::RetargetStackObject { target, mode, .. } => {
                let mut replacements = bind_unresolved_it_in_target(target, seed_tag);
                if let RetargetModeAst::OneToFixed { target } = mode {
                    replacements += bind_unresolved_it_in_target(target, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::AddMana { .. } => 0,
            SubjectVerbActionAst::ExchangeControl { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::ExchangeLifeTotals { .. }
            | SubjectVerbActionAst::PreventAllCombatDamage { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToPlayers { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToYou { .. }
            | SubjectVerbActionAst::Meld { .. }
            | SubjectVerbActionAst::RevealTopChooseCardTypePutToHandRestBottom { .. }
            | SubjectVerbActionAst::GrantAbilityToSource { .. }
            | SubjectVerbActionAst::ExchangeZones { .. } => 0,
            SubjectVerbActionAst::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                ..
            } => {
                bind_unresolved_it_in_target(permanent1, seed_tag)
                    + bind_unresolved_it_in_target(permanent2, seed_tag)
            }
            SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn
            | SubjectVerbActionAst::ControlCombatChoicesThisTurn { .. } => 0,
            SubjectVerbActionAst::ExchangeTextBoxes { target } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::ExchangeValues { left, right, .. } => {
                let bind_operand =
                    |operand: &mut crate::cards::builders::ExchangeValueAst| match operand {
                        crate::cards::builders::ExchangeValueAst::LifeTotal(_) => 0,
                        crate::cards::builders::ExchangeValueAst::Stat { target, .. } => {
                            bind_unresolved_it_in_target(target, seed_tag)
                        }
                    };
                bind_operand(left) + bind_operand(right)
            }
            SubjectVerbActionAst::RevealTop | SubjectVerbActionAst::ExtraTurnAfterTurn { .. } => 0,
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount, target } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::RedirectNextTimeDamageToSource { source, target, .. } => {
                bind_unresolved_it_in_prevent_next_source(source, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source,
            } => bind_unresolved_it_in_target(source, seed_tag),
            SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget {
                object_filter,
                target,
                ..
            } => bind_unresolved_it_in_filter(object_filter, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag),
            SubjectVerbActionAst::PreventDamage {
                amount, target, ..
            } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::PreventDamageToTargetPutCounters { amount, target, .. } => {
                amount
                    .as_mut()
                    .map(|amount| bind_unresolved_it_in_value(amount, seed_tag))
                    .unwrap_or(0)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::PreventAllDamageToTarget { target, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::PreventAllDamageFromSourceFilter { source_filter, .. } => {
                bind_unresolved_it_in_filter(source_filter, seed_tag)
            }
            SubjectVerbActionAst::PreventDamageEach { amount, filter, .. } => {
                bind_unresolved_it_in_value(amount, seed_tag)
                    + bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                target,
                ..
            } => {
                bind_unresolved_it_in_value(put_count, seed_tag)
                    + bind_unresolved_it_in_value(remove_count, seed_tag)
                    + bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::CopySpell { target, count, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
                    + bind_unresolved_it_in_value(count, seed_tag)
            }
            SubjectVerbActionAst::CopySpellForEachTarget {
                target,
                object_filter,
                ..
            } => {
                let mut replacements = bind_unresolved_it_in_target(target, seed_tag);
                if let Some(filter) = object_filter {
                    replacements += bind_unresolved_it_in_filter(filter, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag, keep_tagged, ..
            } => {
                let mut replacements = bind_unresolved_it_in_tag(tag, seed_tag);
                if let Some(keep_tagged) = keep_tagged.as_mut() {
                    replacements += bind_unresolved_it_in_tag(keep_tagged, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::CastTagged { tag, .. } => {
                bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { tag, .. }
            | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                ..
            }
            | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { tag, .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource { tag, .. } => {
                bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                target,
                count_value,
                ..
            } => {
                bind_unresolved_it_in_target(target, seed_tag)
                    + count_value
                        .as_mut()
                        .map(|value| bind_unresolved_it_in_value(value, seed_tag))
                        .unwrap_or(0)
            }
            SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. }
            | SubjectVerbActionAst::TargetOnly { target }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::SetBasePowerToughness { target, .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
            | SubjectVerbActionAst::SetBasePower { target, .. }
            | SubjectVerbActionAst::PumpByLastEffect { target, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::PumpForEach { target, count, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
                    + bind_unresolved_it_in_value(count, seed_tag)
            }
            SubjectVerbActionAst::PumpAll { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::MoveToZone {
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
            SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::TagMatchingObjects { filter, tag, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
                    + bind_unresolved_it_in_tag(tag, seed_tag)
            }
            SubjectVerbActionAst::AddCardTypes { target, .. }
            | SubjectVerbActionAst::RemoveCardTypes { target, .. }
            | SubjectVerbActionAst::AddSubtypes { target, .. }
            | SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { target }
            | SubjectVerbActionAst::AddColors { target, .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::BecomeAuraEnchantment { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandType { target, .. }
            | SubjectVerbActionAst::SetColors { target, .. }
            | SubjectVerbActionAst::MakeColorless { target, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeColorChoice { target, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::BecomeCopy { target, source, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
                    + bind_unresolved_it_in_target(source, seed_tag)
            }
            SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                bind_unresolved_it_in_target(target, seed_tag)
            }
            SubjectVerbActionAst::GrantAbilitiesAll { filter, .. }
            | SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceAll { filter, .. }
            | SubjectVerbActionAst::GrantBySpec {
                spec: crate::grant::GrantSpec { filter, .. },
                ..
            } => bind_unresolved_it_in_filter(filter, seed_tag),
            SubjectVerbActionAst::ConsultTopOfLibrary {
                filter,
                stop_rule,
                all_tag,
                match_tag,
                ..
            } => {
                let mut replacements = bind_unresolved_it_in_filter(filter, seed_tag)
                    + bind_unresolved_it_in_tag(all_tag, seed_tag)
                    + bind_unresolved_it_in_tag(match_tag, seed_tag);
                if let crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value) =
                    stop_rule
                {
                    replacements += bind_unresolved_it_in_value(value, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::SearchLibrary { filter, .. } => {
                bind_unresolved_it_in_filter(filter, seed_tag)
            }
            SubjectVerbActionAst::Cant { restriction, .. } => {
                bind_unresolved_it_in_restriction(restriction, seed_tag)
            }
            SubjectVerbActionAst::CreateTokenCopy { object, count, .. } => {
                bind_unresolved_it_in_object_ref_ast(object, seed_tag)
                    + bind_unresolved_it_in_value(count, seed_tag)
            }
            SubjectVerbActionAst::CreateTokenCopyFromSource { source, count, .. } => {
                bind_unresolved_it_in_target(source, seed_tag)
                    + bind_unresolved_it_in_value(count, seed_tag)
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                count,
                dynamic_power_toughness,
                attached_to,
                ..
            } => {
                let mut replacements = bind_unresolved_it_in_value(count, seed_tag);
                if let Some((power, toughness)) = dynamic_power_toughness.as_mut() {
                    replacements += bind_unresolved_it_in_value(power, seed_tag);
                    replacements += bind_unresolved_it_in_value(toughness, seed_tag);
                }
                if let Some(target) = attached_to.as_mut() {
                    replacements += bind_unresolved_it_in_target(target, seed_tag);
                }
                replacements
            }
            SubjectVerbActionAst::AdditionalPhases { .. } => 0,
            SubjectVerbActionAst::RegisterEnterUnderControlReplacement { .. } => 0,
            SubjectVerbActionAst::Learn => 0,
            SubjectVerbActionAst::ShuffleLibrary => 0,
        },
        EffectAst::ForEachObject { filter, .. } => bind_unresolved_it_in_filter(filter, seed_tag),
        EffectAst::ForEachTagged { tag, .. }
        | EffectAst::ForEachTaggedPlayer { tag, .. } => bind_unresolved_it_in_tag(tag, seed_tag),
        EffectAst::ForEachPlayersFiltered { filter: player, .. } => {
            bind_unresolved_it_in_player_filter(player, seed_tag)
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { filter, .. } => {
            if let Some(filter) = filter.as_mut() {
                bind_unresolved_it_in_filter(filter, seed_tag)
            } else {
                0
            }
        }
        EffectAst::Conditional { predicate, .. } | EffectAst::SelfReplacement { predicate, .. } => {
            bind_unresolved_it_in_predicate(predicate, seed_tag)
        }
        EffectAst::ChooseObjects {
            filter,
            count_value,
            tag,
            ..
        } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
                + count_value
                    .as_mut()
                    .map(|value| bind_unresolved_it_in_value(value, seed_tag))
                    .unwrap_or(0)
                + bind_unresolved_it_in_tag(tag, seed_tag)
        }
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count_value,
            tag,
            ..
        } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
                + count_value
                    .as_mut()
                    .map(|value| bind_unresolved_it_in_value(value, seed_tag))
                    .unwrap_or(0)
                + bind_unresolved_it_in_tag(tag, seed_tag)
        }
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost { filter, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::RepeatThisProcess
        | EffectAst::RepeatThisProcessMay
        | EffectAst::RepeatThisProcessOnce => 0,
        EffectAst::ForEachOpponentDid {
            predicate: Some(predicate),
            ..
        }
        | EffectAst::ForEachPlayerDid {
            predicate: Some(predicate),
            ..
        } => bind_unresolved_it_in_predicate(predicate, seed_tag),
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
        PlayerFilter::ControllerOf(reference)
        | PlayerFilter::OwnerOf(reference)
        | PlayerFilter::AliasedOwnerOf(reference)
        | PlayerFilter::AliasedControllerOf(reference) => {
            bind_unresolved_it_in_runtime_object_ref(reference, seed_tag)
        }
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_comparison(comparison: &mut Comparison, seed_tag: &TagKey) -> usize {
    match comparison {
        Comparison::EqualExpr(value)
        | Comparison::NotEqualExpr(value)
        | Comparison::LessThanExpr(value)
        | Comparison::LessThanOrEqualExpr(value)
        | Comparison::GreaterThanExpr(value)
        | Comparison::GreaterThanOrEqualExpr(value) => bind_unresolved_it_in_value(value, seed_tag),
        _ => 0,
    }
}

#[cfg(test)]
fn bind_unresolved_it_in_filter(filter: &mut ObjectFilter, seed_tag: &TagKey) -> usize {
    let mut replacements = 0;
    for constraint in &mut filter.tagged_constraints {
        replacements += bind_unresolved_it_in_tag(&mut constraint.tag, seed_tag);
    }
    if let Some(power) = filter.power.as_mut() {
        replacements += bind_unresolved_it_in_comparison(power, seed_tag);
    }
    if let Some(toughness) = filter.toughness.as_mut() {
        replacements += bind_unresolved_it_in_comparison(toughness, seed_tag);
    }
    if let Some(mana_value) = filter.mana_value.as_mut() {
        replacements += bind_unresolved_it_in_comparison(mana_value, seed_tag);
    }
    if let Some(color_count) = filter.color_count.as_mut() {
        replacements += bind_unresolved_it_in_comparison(color_count, seed_tag);
    }
    if let Some(owner) = filter.owner.as_mut() {
        replacements += bind_unresolved_it_in_player_filter(owner, seed_tag);
    }
    if let Some(controller) = filter.controller.as_mut() {
        replacements += bind_unresolved_it_in_player_filter(controller, seed_tag);
    }
    if let Some(targetability) = filter.could_be_targeted_by.as_mut()
        && let crate::filter::ObjectRef::Tagged(tag) = &mut targetability.stack_object
    {
        replacements += bind_unresolved_it_in_tag(tag, seed_tag);
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
        Value::SurfaceHinted { value, .. } => bind_unresolved_it_in_value(value, seed_tag),
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
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => bind_unresolved_it_in_filter(filter, seed_tag),
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
        PredicateAst::ItMatches(filter)
        | PredicateAst::TargetMatches(filter)
        | PredicateAst::TaggedMatches(_, filter) => {
            let mut replacements = bind_unresolved_it_in_filter(filter, seed_tag);
            if let PredicateAst::TaggedMatches(tag, _) = predicate {
                replacements += bind_unresolved_it_in_tag(tag, seed_tag);
            }
            replacements
        }
        PredicateAst::TaggedWasCast(tag) => bind_unresolved_it_in_tag(tag, seed_tag),
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
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            bind_unresolved_it_in_predicate(left, seed_tag)
                + bind_unresolved_it_in_predicate(right, seed_tag)
        }
        PredicateAst::ValueComparison { left, right, .. } => {
            bind_unresolved_it_in_value(left, seed_tag)
                + bind_unresolved_it_in_value(right, seed_tag)
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
        | Restriction::MustBeBlocked(filter)
        | Restriction::Untap(filter)
        | Restriction::BeBlocked(filter)
        | Restriction::BeDestroyed(filter)
        | Restriction::BeRegenerated(filter)
        | Restriction::BeSacrificed(filter)
        | Restriction::HaveCountersPlaced(filter)
        | Restriction::BeTargeted(filter)
        | Restriction::BeCountered(filter)
        | Restriction::Transform(filter)
        | Restriction::PhaseOut(filter)
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
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, .. } => {
            bind_unresolved_it_in_filter(attackers, seed_tag)
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::super::reference_model::{
        RefState as ModelRefState, ReferenceImports as ModelReferenceImports,
    };
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

        let effects = vec![EffectAst::subject_verb_damage(
            Value::Count(filter),
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
        )];

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
            EffectAst::subject_verb_investigate(
                crate::cards::builders::PlayerAst::Implicit,
                Value::Fixed(1),
            ),
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![EffectAst::subject_verb_investigate(
                    crate::cards::builders::PlayerAst::Implicit,
                    Value::X,
                )],
            },
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
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
                    EffectAst::SubjectVerb(subject_verb)
                        if matches!(
                            &subject_verb.action,
                            SubjectVerbActionAst::Investigate { .. }
                        ) =>
                    {
                        let SubjectVerbActionAst::Investigate { count } = &subject_verb.action
                        else {
                            unreachable!()
                        };
                        assert_eq!(count, &Value::EffectValue(EffectId(0)));
                        assert_eq!(
                            subject_verb.subject.player,
                            crate::cards::builders::PlayerAst::Implicit
                        );
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
            EffectAst::subject_verb_exile_all(filter, false),
            EffectAst::subject_verb_reveal_hand(PlayerAst::That),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::with_last_object_tag("seeded"),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate same-controller follow-up");

        assert_eq!(
            annotated.effects[0].out_env.last_player_filter,
            ModelRefState::Known(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                "seeded"
            )))
        );
        assert_eq!(
            annotated.effects[1].in_env.last_player_filter,
            ModelRefState::Known(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                "seeded"
            )))
        );
    }

    #[test]
    fn resolves_event_amount_to_prior_effect_value_when_trigger_context_disallows_it() {
        let effects = vec![
            EffectAst::subject_verb_investigate(PlayerAst::Implicit, Value::Fixed(1)),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::Draw {
                    count: Value::EventValue(EventValueSpec::Amount),
                },
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig {
                allow_life_event_value: false,
                ..Default::default()
            },
            IdGenContext::default(),
        )
        .expect("annotate event-derived amount");

        assert_eq!(annotated.effects[0].assigned_effect_id, Some(EffectId(0)));

        match &annotated.effects[1].effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Draw { count },
                ..
            }) => {
                assert_eq!(count, &Value::EffectValue(EffectId(0)));
            }
            other => panic!("expected draw effect, got {other:?}"),
        }
    }

    #[test]
    fn annotates_followup_effect_with_explicit_object_reference_frame() {
        let effects = vec![
            EffectAst::subject_verb_destroy(TargetAst::Object(
                ObjectFilter::creature(),
                Some(TextSpan::synthetic()),
                None,
            )),
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                false,
                false,
                false,
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence metadata");

        assert_eq!(
            annotated.effects[1].in_env.last_object_tag,
            ModelRefState::Known(TagKey::from("destroyed_0"))
        );
    }

    #[test]
    fn annotate_effect_sequence_sets_followup_in_env_from_destroyed_tag() {
        let effects = vec![
            EffectAst::subject_verb_destroy(TargetAst::Object(
                ObjectFilter::creature(),
                Some(TextSpan::synthetic()),
                None,
            )),
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                false,
                false,
                false,
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert_eq!(
            annotated.effects[1].in_env.last_object_tag,
            ModelRefState::Known(TagKey::from("destroyed_0"))
        );
    }

    #[test]
    fn annotate_effect_sequence_sets_followup_in_env_from_countered_tag() {
        let effects = vec![
            EffectAst::subject_verb_counter(TargetAst::Spell(Some(TextSpan::synthetic()))),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::CreateTokenWithMods {
                    name: "Thopter".to_string(),
                    count: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)))),
                    dynamic_power_toughness: None,
                    player: PlayerAst::Implicit,
                    attached_to: None,
                    tapped: false,
                    attacking: false,
                    exile_at_end_of_combat: false,
                    sacrifice_at_end_of_combat: false,
                    sacrifice_at_next_end_step: false,
                    exile_at_next_end_step: false,
                    granted_abilities: Vec::new(),
                },
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate counter follow-up");

        assert_eq!(
            annotated.effects[1].in_env.last_object_tag,
            ModelRefState::Known(TagKey::from("countered_0"))
        );
    }

    #[test]
    fn annotate_effect_sequence_sets_followup_in_env_from_amassed_tag() {
        let effects = vec![
            EffectAst::subject_verb_amass(Some(Subtype::Orc), Value::Fixed(2)),
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                false,
                false,
                false,
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate amass follow-up");

        assert_eq!(
            annotated.effects[1].in_env.last_object_tag,
            ModelRefState::Known(TagKey::from("amassed_0"))
        );
    }

    #[test]
    fn annotate_effect_sequence_assigns_prior_effect_id_for_event_amount_followup() {
        let effects = vec![
            EffectAst::subject_verb_investigate(PlayerAst::Implicit, Value::Fixed(1)),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::Draw {
                    count: Value::EventValue(EventValueSpec::Amount),
                },
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert_eq!(annotated.effects[0].assigned_effect_id, Some(EffectId(0)));
        match &annotated.effects[1].effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Draw { count },
                ..
            }) => {
                assert_eq!(count, &Value::EffectValue(EffectId(0)));
            }
            other => panic!("expected draw effect, got {other:?}"),
        }
    }

    #[test]
    fn annotate_effect_sequence_binds_pending_effect_metric_to_prior_memory_effect() {
        let effects = vec![
            EffectAst::subject_verb_sacrifice_all(
                PlayerAst::You,
                ObjectFilter::creature().you_control(),
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::Draw {
                    count: Value::PendingEffectMetric {
                        source: EffectMetricSource::AffectedObjects,
                        metric: EffectMetric::Count,
                    },
                },
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate pending metric sequence");

        assert_eq!(annotated.effects[0].assigned_effect_id, Some(EffectId(0)));
        match &annotated.effects[1].effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Draw { count },
                ..
            }) => {
                assert_eq!(
                    count,
                    &Value::EffectMetric {
                        effect_id: EffectId(0),
                        source: EffectMetricSource::AffectedObjects,
                        metric: EffectMetric::Count,
                    }
                );
            }
            other => panic!("expected draw effect, got {other:?}"),
        }
    }

    #[test]
    fn annotate_effect_sequence_keeps_other_result_bound_to_roll_across_destroy() {
        let mut filter = ObjectFilter::creature();
        filter.power = Some(Comparison::GreaterThanOrEqualExpr(Box::new(
            Value::EventValue(EventValueSpec::Amount),
        )));
        let effects = vec![
            EffectAst::subject_verb_roll_dice_choose_result_with_die_text(
                PlayerAst::Implicit,
                2,
                6,
                Some("d6".to_string()),
            ),
            EffectAst::subject_verb_destroy_all(filter),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::CreateTokenWithMods {
                    name: "Knight".to_string(),
                    count: Value::PendingEffectMetric {
                        source: EffectMetricSource::Outcome,
                        metric: EffectMetric::OtherNumber,
                    },
                    dynamic_power_toughness: None,
                    player: PlayerAst::Implicit,
                    attached_to: None,
                    tapped: false,
                    attacking: false,
                    exile_at_end_of_combat: false,
                    sacrifice_at_end_of_combat: false,
                    sacrifice_at_next_end_step: false,
                    exile_at_next_end_step: false,
                    granted_abilities: Vec::new(),
                },
            ),
        ];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate roll-destroy-create sequence");

        assert_eq!(annotated.effects[0].assigned_effect_id, Some(EffectId(0)));
        assert_eq!(annotated.effects[1].assigned_effect_id, None);

        match &annotated.effects[2].effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenWithMods { count, .. },
                ..
            }) => assert_eq!(
                count,
                &Value::EffectMetric {
                    effect_id: EffectId(0),
                    source: EffectMetricSource::Outcome,
                    metric: EffectMetric::OtherNumber,
                }
            ),
            other => panic!("expected create-token effect, got {other:?}"),
        }
    }

    #[test]
    fn annotate_effect_sequence_joins_conditional_last_object_tag_when_branches_agree() {
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::YourTurn,
            if_true: Vec::new(),
            if_false: Vec::new(),
        }];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports {
                last_object_tag: Some(TagKey::from("seeded")),
                ..Default::default()
            },
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert_eq!(
            annotated.final_env.last_object_tag,
            ModelRefState::Known(TagKey::from("seeded"))
        );
    }

    #[test]
    fn annotate_effect_sequence_marks_conditional_last_object_tag_ambiguous_when_branches_diverge()
    {
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::YourTurn,
            if_true: vec![
                EffectAst::subject_verb_destroy(TargetAst::Object(
                    ObjectFilter::creature(),
                    Some(TextSpan::synthetic()),
                    None,
                )),
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                    TagKey::from(IT_TAG),
                    PlayerAst::You,
                    false,
                    false,
                    false,
                ),
            ],
            if_false: vec![
                EffectAst::subject_verb_exile(
                    TargetAst::Object(ObjectFilter::creature(), Some(TextSpan::synthetic()), None),
                    false,
                ),
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                    TagKey::from(IT_TAG),
                    PlayerAst::You,
                    false,
                    false,
                    false,
                ),
            ],
        }];

        let annotated = annotate_effect_sequence(
            &effects,
            &ModelReferenceImports::default(),
            EffectReferenceResolutionConfig::default(),
            IdGenContext::default(),
        )
        .expect("annotate sequence");

        assert!(matches!(
            annotated.final_env.last_object_tag,
            ModelRefState::Ambiguous
        ));
    }
}
