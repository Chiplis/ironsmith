//! Immutable pair ownership plus a runtime rules-text acquisition (CR 607).
use crate::continuous::{AbilityEffectOrigin, AbilityOrigin};
use crate::ids::{CardId, ObjectId};
use ironsmith_core::LinkedExilePair;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LinkedExileAcquisition {
    Printed,
    Effect(AbilityEffectOrigin),
    Borrowed {
        effect: AbilityEffectOrigin,
        donor: ObjectId,
        origin: Box<LinkedExileAcquisition>,
    },
    Temporary(crate::object::TemporaryAbilityOrigin),
    Counter(crate::object::CounterAbilityOrigin),
    Level {
        printed_face: Option<CardId>,
        parent: Box<AbilityOrigin>,
        tier: usize,
    },
}

impl LinkedExileAcquisition {
    fn for_pair(host: ObjectId, pair: LinkedExilePair, origin: &AbilityOrigin) -> Option<Self> {
        match origin {
            AbilityOrigin::Effect { effect, .. } => match effect.linked_exile_parent(host, pair) {
                Some(parent) => Self::for_pair(host, pair, parent),
                None => Self::from_origin(origin),
            },
            AbilityOrigin::Borrowed { effect, source, origin } => Some(Self::Borrowed {
                effect: effect.clone(), donor: *source,
                origin: Box::new(Self::for_pair(*source, pair, origin)?),
            }),
            _ => Self::from_origin(origin),
        }
    }

    /// Only the paired member's slot is erased. The acquisition, donor's
    /// exact incarnation, and enclosing origin remain part of its identity.
    fn from_origin(origin: &AbilityOrigin) -> Option<Self> {
        Some(match origin {
            AbilityOrigin::Printed(_) => Self::Printed,
            AbilityOrigin::Effect { effect, .. } => Self::Effect(effect.clone()),
            AbilityOrigin::Borrowed { effect, source, origin } => Self::Borrowed {
                effect: effect.clone(), donor: *source,
                origin: Box::new(Self::from_origin(origin)?),
            },
            AbilityOrigin::Temporary(origin) => Self::Temporary(origin.clone()),
            AbilityOrigin::Counter { occurrence, .. } => Self::Counter(occurrence.clone()),
            AbilityOrigin::Level { printed_face, parent, tier, .. } => Self::Level {
                printed_face: *printed_face, parent: parent.clone(), tier: *tier,
            },
            AbilityOrigin::IntrinsicBasicLandMana(_) | AbilityOrigin::IntrinsicStartingCounters(_) => return None,
        })
    }
}

/// Captured at admission, retained by stack copies, delays, and checkpoints.
/// A new host or donor incarnation never inherits an earlier pair's members.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinkedExileOwner {
    pub host: ObjectId,
    pub pair: LinkedExilePair,
    pub acquisition: LinkedExileAcquisition,
}

impl LinkedExileOwner {
    pub fn capture(host: ObjectId, pair: Option<LinkedExilePair>, origin: Option<&AbilityOrigin>) -> Option<Self> {
        let pair = pair?;
        Some(Self { host, pair, acquisition: LinkedExileAcquisition::for_pair(host, pair, origin?)? })
    }
}

pub(crate) fn validate_program_owner(
    pair: Option<LinkedExilePair>,
    owner: Option<&LinkedExileOwner>,
) -> Result<(), crate::effects::ExecutionError> {
    if let Some(pair) = pair
        && !owner.is_some_and(|owner| owner.pair == pair)
    {
        return Err(crate::effects::ExecutionError::IncompleteEvidence(
            "linked ability admission omitted its exact rules-text acquisition; native recovery or replay required".into()));
    }
    Ok(())
}

/// A level-gated reader must actually arrive through its captured Class grant.
/// A native caller cannot turn a marked reader into an unconditional printed
/// permission by attaching the proof metadata alone.
pub(crate) fn has_class_linked_exile_bridge(host: ObjectId, pair: LinkedExilePair, level: u32, origin: &AbilityOrigin) -> bool {
    match origin {
        AbilityOrigin::Effect { effect, .. } => effect.linked_exile_class_level() == Some(level) && effect.linked_exile_parent(host, pair).is_some(),
        AbilityOrigin::Borrowed { source, origin, .. } => has_class_linked_exile_bridge(*source, pair, level, origin),
        _ => false,
    }
}

/// Discovery must evaluate the Class designation on the receiving permanent,
/// including when a complete rules-text group was acquired from elsewhere.
pub(crate) fn is_class_linked_exile_wrapper(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Static(wrapper) = &ability.kind else { return false; };
    let Some(crate::ConditionExpr::SourceClassLevelAtLeast(level)) = wrapper.granted_inline_condition() else { return false; };
    let readers = wrapper.source_granted_inline_abilities();
    let [reader] = readers.as_slice() else { return false; };
    let crate::ability::AbilityKind::Static(reader) = &reader.kind else { return false; };
    reader.grant_spec().is_some_and(|spec| spec.requires_linked_exile_pair
        && spec.linked_exile_pair.is_some() && spec.linked_exile_class_level == Some(*level))
}

#[cfg(test)]
mod class_acquisition_tests {
    use super::*;
    use crate::ability::{Ability, AbilityKind};
    use crate::continuous::{ContinuousAbilityOrigin, ContinuousEffect, EffectTarget, Modification};
    use crate::grant::{GrantSpec, Grantable};
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;
    use crate::zone::Zone;
    fn pair(byte: u8) -> LinkedExilePair { LinkedExilePair { definition: ironsmith_core::LinkedExileDefinition([byte; 32]), pair: 0 } }
    fn gate(host: ObjectId, parent: AbilityOrigin, pair: LinkedExilePair) -> ContinuousEffect {
        let mut spec = GrantSpec::new(Grantable::PlayFrom, ObjectFilter::default(), Zone::Exile);
        spec.requires_linked_exile_pair = true; spec.linked_exile_pair = Some(pair); spec.linked_exile_class_level = Some(3);
        let mut gate = ContinuousEffect::new(host, PlayerId::from_index(0), EffectTarget::Source,
            Modification::AddAbilityGeneric(Ability::static_ability(StaticAbility::grants(spec))))
            .with_condition(crate::ConditionExpr::SourceClassLevelAtLeast(3));
        gate.originating_ability = Some(Box::new(ContinuousAbilityOrigin { host, ability: parent, printed_face: None, branch: 0 })); gate
    }
    fn owner(host: ObjectId, pair: LinkedExilePair, origin: &AbilityOrigin) -> LinkedExileOwner { LinkedExileOwner::capture(host, Some(pair), Some(origin)).unwrap() }
    #[test]
    fn only_matching_pair_and_source_gate_project_to_the_captured_parent() {
        let host = ObjectId::from_raw(1); let member = pair(1); let other = pair(2);
        let original = AbilityOrigin::Printed(0); let descriptor = gate(host, AbilityOrigin::Printed(4), member);
        let reader = AbilityOrigin::Effect { effect: (&descriptor).into(), slot: 0 };
        assert_eq!(owner(host, member, &original), owner(host, member, &reader));
        assert!(has_class_linked_exile_bridge(host, member, 3, &reader));
        assert!(!has_class_linked_exile_bridge(host, member, 2, &reader));
        assert!(!has_class_linked_exile_bridge(host, other, 3, &reader));
        assert_ne!(owner(host, other, &original), owner(host, other, &reader));
        assert!(matches!(LinkedExileAcquisition::from_origin(&reader), Some(LinkedExileAcquisition::Effect(_))), "other consumers keep ordinary effect identity");
        for mutation in 0..5 {
            let mut descriptor = descriptor.clone();
            match mutation {
                0 => descriptor.condition = None,
                1 => descriptor.applies_to = EffectTarget::AllPermanents,
                2 => descriptor.originating_ability = None,
                3 => descriptor.originating_ability.as_mut().unwrap().host = ObjectId::from_raw(2),
                _ => descriptor.source_type = crate::continuous::EffectSourceType::Resolution { locked_targets: vec![host] },
            }
            let unproved = AbilityOrigin::Effect { effect: (&descriptor).into(), slot: 0 };
            assert!(!has_class_linked_exile_bridge(host, member, 3, &unproved));
            assert_ne!(owner(host, member, &original), owner(host, member, &unproved));
        }
        // A scalar or unmarked reader in the same ordinary effect does not
        // acquire this Class projection merely because its host matches.
        let mut descriptor = descriptor;
        if let Modification::AddAbilityGeneric(ability) = &mut descriptor.modification {
            ability.kind = AbilityKind::Static(StaticAbility::menace());
        }
        assert!(AbilityEffectOrigin::from(&descriptor).linked_exile_parent(host, member).is_none());
    }
    #[test]
    fn borrowed_and_level_parents_keep_their_outer_acquisition_and_donor_incarnation() {
        let host = ObjectId::from_raw(1); let donor = ObjectId::from_raw(2); let member = pair(3);
        let descriptor = gate(donor, AbilityOrigin::Printed(4), member);
        let reader = AbilityOrigin::Effect { effect: (&descriptor).into(), slot: 0 };
        let outer = AbilityEffectOrigin::from(&ContinuousEffect::new(host, PlayerId::from_index(0), EffectTarget::Specific(host), Modification::RemoveAllAbilities));
        let borrowed_producer = AbilityOrigin::Borrowed { effect: outer.clone(), source: donor, origin: Box::new(AbilityOrigin::Printed(0)) };
        let borrowed_reader = AbilityOrigin::Borrowed { effect: outer.clone(), source: donor, origin: Box::new(reader) };
        assert_eq!(owner(host, member, &borrowed_producer), owner(host, member, &borrowed_reader));
        assert!(has_class_linked_exile_bridge(host, member, 3, &borrowed_reader));
        let mut new_donor = borrowed_reader.clone(); if let AbilityOrigin::Borrowed { source, .. } = &mut new_donor { *source = ObjectId::from_raw(3); }
        assert!(!has_class_linked_exile_bridge(host, member, 3, &new_donor));
        assert_ne!(owner(host, member, &borrowed_producer), owner(host, member, &new_donor));
        let parent = AbilityOrigin::Level { printed_face: None, parent: Box::new(AbilityOrigin::Printed(6)), tier: 1, slot: 4 };
        let descriptor = gate(host, parent.clone(), member); let reader = AbilityOrigin::Effect { effect: (&descriptor).into(), slot: 0 };
        assert_eq!(owner(host, member, &parent), owner(host, member, &reader));
        assert!(matches!(owner(host, member, &reader).acquisition, LinkedExileAcquisition::Level { tier: 1, .. }));
        assert_ne!(owner(host, member, &AbilityOrigin::Printed(0)), owner(host, member, &reader));
    }
}
