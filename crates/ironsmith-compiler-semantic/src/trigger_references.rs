//! What a trigger's object reference means.
//!
//! "That creature", after a trigger, refers to whatever the trigger's event was
//! about. Which object that is depends only on the trigger's shape, so the
//! answer is an AST query — recognition and lowering both ask it.

use crate::cards::builders::{TagKey, TriggerSpec};
use crate::target::{ObjectFilter, ObjectRef, PlayerFilter};

/// The native grouped zone-change matcher captures the size of its exact
/// matching object set, after the event's owner/type/zone filters are applied.
pub fn trigger_binds_grouped_zone_amount(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } | TriggerSpec::ConditionQualified { trigger, .. } =>
            trigger_binds_grouped_zone_amount(trigger),
        TriggerSpec::Either(left, right) => trigger_binds_grouped_zone_amount(left)
            && trigger_binds_grouped_zone_amount(right),
        TriggerSpec::ZoneChange(trigger) => trigger.count == ironsmith_core::trigger_model::CountMode::OneOrMore,
        TriggerSpec::PutIntoGraveyardOneOrMore(_) | TriggerSpec::DiesOneOrMore(_) => true,
        TriggerSpec::PutIntoGraveyardFromZone { one_or_more, .. }
        | TriggerSpec::PutIntoGraveyardFromAnyExcept { one_or_more, .. }
        | TriggerSpec::LeavesBattlefieldWithoutDying { one_or_more, .. }
        | TriggerSpec::DiesDuringTurn { one_or_more, .. }
        | TriggerSpec::DiesDuringCombat { one_or_more, .. } => *one_or_more,
        _ => false,
    }
}

#[cfg(test)]
mod grouped_zone_amount_tests {
    use super::*;
    #[test]
    fn only_a_proven_grouped_zone_trigger_supplies_the_matching_object_count() {
        let grouped = TriggerSpec::PutIntoGraveyardOneOrMore(ObjectFilter::creature());
        let singular = TriggerSpec::PutIntoGraveyard(ObjectFilter::creature());
        assert!(trigger_binds_grouped_zone_amount(&grouped));
        assert!(!trigger_binds_grouped_zone_amount(&singular));
        assert!(!trigger_binds_grouped_zone_amount(&TriggerSpec::Either(Box::new(grouped.clone()), Box::new(singular))));
        let mut canonical = ironsmith_core::trigger_model::ZoneChangeTrigger::new();
        assert!(!trigger_binds_grouped_zone_amount(&TriggerSpec::ZoneChange(canonical.clone())));
        canonical.count = ironsmith_core::trigger_model::CountMode::OneOrMore;
        assert!(trigger_binds_grouped_zone_amount(&TriggerSpec::ZoneChange(canonical)));
    }
}

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
            | TriggerSpec::EndOfCombat
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
    if matches!(trigger, TriggerSpec::PlayerPaysLife(_)) { return None; }
    if matches!(trigger,TriggerSpec::PlayerBecomesMonarch(_)){return None;}
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return default_trigger_last_object_tag(trigger);
    }
    if let Some(tag) = phase_step_trigger_object_reference_tag(trigger) {
        return Some(tag);
    }
    if trigger_die_event_grouped(trigger).is_some() || matches!(trigger, TriggerSpec::PlayerRollsNthDie { .. }) {
        // The die-producing object is not the implicit subject of a roll trigger.
        return None;
    }
    if matches!(trigger, TriggerSpec::PlayerBecomesTargeted { .. }) { return None; }
    if matches!(trigger, TriggerSpec::DamageReceived { target: crate::target::ChooseSpec::Player(_), .. }) { return None; }
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
        TriggerSpec::DamageReceived { .. }
            | TriggerSpec::ThisIsDealtDamage
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
        | TriggerSpec::ThisAttacksPlayerWithMostLife
        | TriggerSpec::ThisBlocks
        | TriggerSpec::ThisIsDealtDamage
        | TriggerSpec::ThisIsDealtCombatDamage => return Some(WatchedPermanent::Source),
        TriggerSpec::DamageReceived { target: crate::target::ChooseSpec::Source, .. } => return Some(WatchedPermanent::Source),
        TriggerSpec::DamageReceived { target: crate::target::ChooseSpec::Object(filter), .. } if filter.source => return Some(WatchedPermanent::Source),
        TriggerSpec::DamageReceived { target: crate::target::ChooseSpec::Object(filter), .. }
        | TriggerSpec::PermanentBecomesTapped(filter)
        | TriggerSpec::PermanentBecomesTappedOneOrMore(filter)
        | TriggerSpec::PermanentBecomesUntapped { filter, .. }
        | TriggerSpec::PlayerChangesTapState { filter, .. }
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

/// Only these typed triggers export excess, rather than ordinary damage/life,
/// as their ambient amount. This capability survives reference-frame scopes.
pub fn trigger_binds_excess_damage_amount(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_binds_excess_damage_amount(trigger),
        TriggerSpec::Either(left, right) => {
            trigger_binds_excess_damage_amount(left) && trigger_binds_excess_damage_amount(right)
        }
        TriggerSpec::IsDealtExcessNoncombatDamage(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod excess_damage_amount_tests {
    use super::*;
    #[test]
    fn only_excess_triggers_export_excess_amounts() {
        let excess = TriggerSpec::IsDealtExcessNoncombatDamage(ObjectFilter::creature());
        assert!(trigger_binds_excess_damage_amount(&excess));
        assert!(!trigger_binds_excess_damage_amount(
            &TriggerSpec::ThisIsDealtDamage
        ));
        assert!(!trigger_binds_excess_damage_amount(
            &TriggerSpec::YouGainLife
        ));
        assert!(trigger_binds_excess_damage_amount(&TriggerSpec::Either(
            Box::new(excess.clone()),
            Box::new(excess.clone())
        )));
        assert!(!trigger_binds_excess_damage_amount(&TriggerSpec::Either(
            Box::new(excess),
            Box::new(TriggerSpec::ThisIsDealtDamage)
        )));
    }
}

/// The exact card predicate used by the event's numeric count. A query can use
/// that number only after proving semantic equality, including all qualifiers.
/// Mixed-event or differently filtered alternatives deliberately have no proof.
pub fn trigger_milling_event_filter(trigger: &TriggerSpec) -> Option<std::sync::Arc<ObjectFilter>> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } | TriggerSpec::ConditionQualified { trigger, .. } => trigger_milling_event_filter(trigger),
        TriggerSpec::CardsMilled { filter, one_or_more: true, .. } => Some(std::sync::Arc::new(filter.clone().unwrap_or_default())),
        TriggerSpec::Either(left, right) => {
            let left = trigger_milling_event_filter(left)?;
            let right = trigger_milling_event_filter(right)?;
            (left == right).then_some(left)
        }
        _ => None,
    }
}

/// Evidence for one affected player's actual life-change event. The action
/// direction is distinct from a generic numeric event (damage, cards, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct LifeEventBinding {
    pub metric: ironsmith_core::EffectMetric,
    pub player: PlayerFilter,
}

/// A compatible lexical life instruction, independent of an intervening cost's
/// result ID. Optional instructions retain an outcome ID even when declined.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeAmountProducer {
    pub effect_id: ironsmith_core::EffectId,
    pub metric: ironsmith_core::EffectMetric,
    pub player: PlayerFilter,
}

pub fn trigger_life_event_binding(trigger: &TriggerSpec) -> Option<std::sync::Arc<LifeEventBinding>> {
    use ironsmith_core::EffectMetric;
    let (metric, player) = match trigger {
        TriggerSpec::WithIntro { trigger, .. } | TriggerSpec::ConditionQualified { trigger, .. } => return trigger_life_event_binding(trigger),
        TriggerSpec::YouGainLife | TriggerSpec::YouGainLifeCausedBy(_) | TriggerSpec::YouGainLifeDuringTurn(_) => (EffectMetric::LifeGained, PlayerFilter::You),
        TriggerSpec::PlayerGainsLife { player, .. } => (EffectMetric::LifeGained, player.clone()),
        TriggerSpec::PlayerLosesLife(player) | TriggerSpec::PlayerLosesLifeDuringTurn { player, .. } => (EffectMetric::LifeLost, player.clone()),
        TriggerSpec::Either(left, right) => {
            let left = trigger_life_event_binding(left)?;
            let right = trigger_life_event_binding(right)?;
            return (left == right).then_some(left);
        }
        _ => return None,
    };
    Some(std::sync::Arc::new(LifeEventBinding { metric, player }))
}

/// Only a typed compatible roll event can supply a bare result. Either must
/// retain the same singular/batch contract in both arms. Ordinals do not
/// identify which physical die in a simultaneous group was "the third".
pub fn trigger_die_event_grouped(trigger: &TriggerSpec) -> Option<bool> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } | TriggerSpec::ConditionQualified { trigger, .. } => trigger_die_event_grouped(trigger),
        TriggerSpec::PlayerRollsDie { one_or_more, .. } => Some(*one_or_more),
        TriggerSpec::PlayerRollsToVisitAttractions { .. } | TriggerSpec::PlayerRollsResult { .. }
        | TriggerSpec::PlayerRollsResultMatching { .. } | TriggerSpec::PlayerRollsHighestNaturalResult { .. } => Some(false),
        TriggerSpec::Either(a,b) => { let a = trigger_die_event_grouped(a)?; (trigger_die_event_grouped(b) == Some(a)).then_some(a) }
        _ => None,
    }
}

/// A bare demonstrative belongs to the single quantitative restriction on a
/// completed cast. Multiple different quantities are intentionally ambiguous.
pub fn trigger_cast_event_quantity(trigger: &TriggerSpec) -> Option<ironsmith_core::CastEventQuantity> {
    use ironsmith_core::CastEventQuantity;
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } | TriggerSpec::ConditionQualified { trigger, .. } => trigger_cast_event_quantity(trigger),
        TriggerSpec::Either(left, right) => {
            let left = trigger_cast_event_quantity(left)?;
            (trigger_cast_event_quantity(right) == Some(left)).then_some(left)
        }
        TriggerSpec::SpellCast { filter: Some(filter), .. }
        | TriggerSpec::SpellCastSameNameCardInZone { filter: Some(filter), .. } => {
            let mut quantities = Vec::new();
            if filter.mana_value_eq_counters_on_source.is_some() { quantities.push(CastEventQuantity::ManaValue); }
            if let Some((color, _)) = filter.mana_symbol_count { quantities.push(CastEventQuantity::ManaSymbols(color)); }
            // Qualified target relations retain their separately captured subset
            // count. A bare arity counts the completed cast's distinct targets.
            if filter.target_count.is_some() && filter.targets_player.is_none()
                && filter.targets_object.is_none() && filter.targets_only_player.is_none()
                && filter.targets_only_object.is_none() {
                quantities.push(CastEventQuantity::DistinctTargets);
            }
            if filter.any_of.is_empty() && quantities.len() == 1 { quantities.pop() } else { None }
        }
        _ => None,
    }
}

/// Legacy relation-qualified target counts use the matcher's captured subset.
/// An additional quantified characteristic cannot silently win that binding.
pub fn spell_cast_filter_binds_target_count(filter: &ObjectFilter) -> bool {
    if filter.mana_symbol_count.is_some() || filter.mana_value_eq_counters_on_source.is_some() {
        return false;
    }
    filter.targets_player.is_some() || filter.targets_object.is_some()
        || filter.targets_only_player.is_some() || filter.targets_only_object.is_some()
        || filter.target_count.is_some()
        || filter.any_of.iter().any(spell_cast_filter_binds_target_count)
}
