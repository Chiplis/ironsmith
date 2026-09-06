//! What a trigger's object reference means.
//!
//! "That creature", after a trigger, refers to whatever the trigger's event was
//! about. Which object that is depends only on the trigger's shape, so the
//! answer is an AST query — recognition and lowering both ask it.

use crate::cards::builders::{TagKey, TriggerSpec};
use crate::target::{ObjectFilter, ObjectRef, PlayerFilter};

pub fn phase_step_trigger_object_reference_tag(trigger: &TriggerSpec) -> Option<TagKey> {
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return phase_step_trigger_object_reference_tag(trigger);
    }
    let player = match trigger {
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::BeginningOfPostcombatMain { player, .. } => player,
        _ => return None,
    };
    match player {
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
        | PlayerFilter::OwnerOf(ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedOwnerOf(ObjectRef::Tagged(tag)) => Some(tag.clone()),
        _ => None,
    }
}

pub fn phase_step_trigger_has_no_object_reference(trigger: &TriggerSpec) -> bool {
    if phase_step_trigger_object_reference_tag(trigger).is_some() {
        return false;
    }
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return phase_step_trigger_has_no_object_reference(trigger);
    }
    matches!(
        trigger,
        TriggerSpec::BeginningOfUpkeep(_)
            | TriggerSpec::BeginningOfDrawStep(_)
            | TriggerSpec::BeginningOfCombat(_)
            | TriggerSpec::BeginningOfEndStep(_)
            | TriggerSpec::BeginningOfTheEndStep
            | TriggerSpec::BeginningOfMonarchEndStep
            | TriggerSpec::BeginningOfPrecombatMain(_)
            | TriggerSpec::BeginningOfPostcombatMain { .. }
    )
}

pub fn this_blocks_or_becomes_blocked_other_filter(trigger: &TriggerSpec) -> Option<&ObjectFilter> {
    fn pair<'a>(blocks: &'a TriggerSpec, blocked_by: &'a TriggerSpec) -> Option<&'a ObjectFilter> {
        let TriggerSpec::ThisBlocksObject {
            filter: blocked_filter,
            min_blocked_objects: None,
        } = blocks
        else {
            return None;
        };
        let TriggerSpec::ThisBecomesBlockedByObject(blocker_filter) = blocked_by else {
            return None;
        };
        (blocked_filter == blocker_filter).then_some(blocked_filter)
    }

    let trigger = match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger.as_ref(),
        trigger => trigger,
    };
    let TriggerSpec::Either(left, right) = trigger else {
        return None;
    };
    pair(left, right).or_else(|| pair(right, left))
}

pub fn default_trigger_last_object_tag(trigger: &TriggerSpec) -> Option<TagKey> {
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return default_trigger_last_object_tag(trigger);
    }
    if let Some(tag) = phase_step_trigger_object_reference_tag(trigger) {
        return Some(tag);
    }
    if phase_step_trigger_has_no_object_reference(trigger) {
        return None;
    }
    if this_blocks_or_becomes_blocked_other_filter(trigger).is_some() {
        return Some((crate::tag::CompilerReferenceTag::Blocking.bind()).into());
    }
    if matches!(trigger, TriggerSpec::BlocksOrBecomesBlockedByObject { .. }) {
        return Some((crate::tag::CompilerReferenceTag::Blocking.bind()).into());
    }
    if match trigger {
        TriggerSpec::ThisBecomesBlockedByObject(_)
        | TriggerSpec::BecomesBlockedByObjectWithLesserPower { .. } => true,
        TriggerSpec::WithIntro { trigger, .. } => {
            matches!(
                **trigger,
                TriggerSpec::ThisBecomesBlockedByObject(_)
                    | TriggerSpec::BecomesBlockedByObjectWithLesserPower { .. }
            )
        }
        _ => false,
    } {
        return Some((crate::tag::CompilerReferenceTag::Blocking.bind()).into());
    }
    if matches!(
        trigger,
        TriggerSpec::ThisBlocksObject { .. } | TriggerSpec::BlocksObjectWithLesserPower { .. }
    ) {
        return Some((crate::tag::CompilerReferenceTag::Blocked.bind()).into());
    }
    if matches!(
        trigger,
        TriggerSpec::KeywordActionTaggedObject { object_tag, .. }
            if object_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    ) {
        return Some((crate::tag::CompilerReferenceTag::It.bind()).into());
    }
    if matches!(
        trigger,
        TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ManifestDread,
            ..
        }
    ) {
        return Some((crate::tag::CompilerReferenceTag::ManifestDreadGraveyard.bind()).into());
    }
    if matches!(
        trigger,
        TriggerSpec::ThisIsDealtDamage
            | TriggerSpec::ThisIsDealtCombatDamage
            | TriggerSpec::IsDealtDamage(_)
            | TriggerSpec::IsDealtCombatDamage(_)
            | TriggerSpec::IsDealtExcessNoncombatDamage(_)
            | TriggerSpec::ThisDealsDamageTo(_)
            | TriggerSpec::ThisDealsCombatDamageTo(_)
            | TriggerSpec::DealsDamageTo { .. }
            | TriggerSpec::DealsExactDamageToObjectOrPlayer { .. }
            | TriggerSpec::DealsCombatDamageTo { .. }
    ) {
        Some((crate::tag::CompilerReferenceTag::Damaged.bind()).into())
    } else {
        Some((crate::tag::CompilerReferenceTag::Triggering.bind()).into())
    }
}
