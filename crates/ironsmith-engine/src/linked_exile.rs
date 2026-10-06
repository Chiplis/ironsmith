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
        Some(Self { host, pair: pair?, acquisition: LinkedExileAcquisition::from_origin(origin?)? })
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
