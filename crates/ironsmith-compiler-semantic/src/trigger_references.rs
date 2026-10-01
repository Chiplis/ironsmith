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
    // "Whenever this creature blocks two or more creatures, it gains first
    // strike": a plural block has no singular blocked-creature antecedent, so
    // a following `it` names the blocking source.
    if matches!(
        trigger,
        TriggerSpec::ThisBlocksObject {
            min_blocked_objects: Some(_),
            ..
        }
    ) {
        return None;
    }
    if matches!(
        trigger,
        TriggerSpec::ThisBlocksObject { .. }
            | TriggerSpec::BlocksObjectWithLesserPower { .. }
            | TriggerSpec::BlocksObject { .. }
    ) {
        return Some((crate::tag::CompilerReferenceTag::Blocked.bind()).into());
    }
    // "Whenever this creature saddles a Mount or crews a Vehicle, that Mount
    // or Vehicle ...": when both arms of a disjunctive trigger name the same
    // event object, the shared reference is that object. When one arm has
    // no event object ("When this enchantment enters and whenever you expend
    // 4, put a stash counter on it"), the shared `it` can only be the source.
    if let TriggerSpec::Either(left, right) = trigger {
        let left_tag = default_trigger_last_object_tag(left);
        let right_tag = default_trigger_last_object_tag(right);
        if left_tag == right_tag {
            return left_tag;
        }
        if left_tag.is_none() || right_tag.is_none() {
            return None;
        }
        // "When enchanted creature becomes tapped or is dealt damage, destroy
        // it" (Cryoshatter): the arms' event objects differ (a damage event's
        // object is its source, so the damage arm names the recipient), but
        // both arms watch one permanent. That permanent is the reference.
        match (watched_permanent(left), watched_permanent(right)) {
            (Some(WatchedPermanent::Source), Some(WatchedPermanent::Source)) => return None,
            (Some(WatchedPermanent::Attached(left)), Some(WatchedPermanent::Attached(right)))
                if left == right =>
            {
                return Some(left);
            }
            _ => {}
        }
    }
    // An expend event's object is the spell being cast; the trigger text
    // never refers back to it.
    if matches!(
        trigger,
        TriggerSpec::Expend { .. }
            | TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Expend,
                ..
            }
    ) {
        return None;
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

enum WatchedPermanent {
    Source,
    Attached(TagKey),
}

/// The one permanent a tap/attack/block/damage trigger arm watches, when it is
/// this source or the permanent this source is attached to.
fn watched_permanent(trigger: &TriggerSpec) -> Option<WatchedPermanent> {
    let filter = match trigger {
        TriggerSpec::WithIntro { trigger, .. } => return watched_permanent(trigger),
        TriggerSpec::ThisBecomesTapped
        | TriggerSpec::ThisBecomesUntapped
        | TriggerSpec::ThisAttacks
        | TriggerSpec::ThisBlocks
        | TriggerSpec::ThisIsDealtDamage
        | TriggerSpec::ThisIsDealtCombatDamage => return Some(WatchedPermanent::Source),
        TriggerSpec::PermanentBecomesTapped(filter)
        | TriggerSpec::Attacks(filter)
        | TriggerSpec::Blocks(filter)
        | TriggerSpec::IsDealtDamage(filter)
        | TriggerSpec::IsDealtCombatDamage(filter) => filter,
        _ => return None,
    };
    [
        crate::tag::CompilerReferenceTag::Enchanted,
        crate::tag::CompilerReferenceTag::Equipped,
    ]
    .into_iter()
    .find(|tag| {
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == tag.as_str()
                && matches!(constraint.relation, crate::filter::TaggedOpbjectRelation::IsTaggedObject)
        })
    })
    .map(|tag| WatchedPermanent::Attached(tag.bind().into()))
}
