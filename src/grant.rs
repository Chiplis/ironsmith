//! Unified grant system for granting abilities and alternative casting methods.
//!
//! This module provides a unified way to grant:
//! - Static abilities (flash, flying, hexproof, etc.)
//! - Alternative casting methods (flashback, escape, etc.)
//!
//! Grants can be applied through:
//! - Static abilities on permanents (while the source is on the battlefield)
//! - One-shot effects from resolving spells/abilities (with a duration like "until end of turn")
//!
//! # Example
//!
//! ```ignore
//! // Grant flash to noncreature spells in hand (Valley Floodcaller)
//! StaticAbility::grants(GrantSpec {
//!     grantable: Grantable::Ability(StaticAbility::flash()),
//!     filter: ObjectFilter::noncreature_spell(),
//!     zone: Zone::Hand,
//! })
//!
//! // Grant escape to nonland cards in graveyard (Underworld Breach)
//! StaticAbility::grants(GrantSpec {
//!     grantable: Grantable::AlternativeCast(AlternativeCastingMethod::Escape {
//!         cost: None,
//!         exile_count: 3,
//!     }),
//!     filter: ObjectFilter::nonland(),
//!     zone: Zone::Graveyard,
//! })
//!
//! // Grant flashback until end of turn (Snapcaster Mage)
//! Effect::grant(
//!     Grantable::flashback_use_targets_cost(),
//!     target,
//!     GrantDuration::UntilEndOfTurn,
//! )
//! ```

use crate::alternative_cast::AlternativeCastingMethod;
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::zone::Zone;

/// What can be granted to a card.
#[derive(Debug, Clone, PartialEq)]
pub enum Grantable {
    /// Grant a static ability (flash, flying, hexproof, etc.)
    Ability(StaticAbility),
    /// Grant an alternative casting method (flashback, escape, etc.)
    AlternativeCast(AlternativeCastingMethod),
    /// Grant flashback using the target card's mana cost.
    /// This is a special case for Snapcaster Mage-style effects where
    /// the flashback cost equals the card's own mana cost.
    FlashbackUseTargetsCost,
    /// Grant the ability to play a card from a non-hand zone as if it were in hand.
    /// This allows using the card's normal mana cost AND any alternative costs it has.
    /// Used by Yawgmoth's Will (graveyard), future effects could grant from exile, etc.
    /// The zone is specified in the GrantSpec, not here.
    PlayFrom,
}

impl Grantable {
    /// Create a grantable for flashback that uses the target's mana cost.
    pub fn flashback_use_targets_cost() -> Self {
        Grantable::FlashbackUseTargetsCost
    }

    /// Create a grantable for escape with the given exile count.
    /// The escape cost uses the card's normal mana cost.
    pub fn escape(exile_count: u32) -> Self {
        Grantable::AlternativeCast(AlternativeCastingMethod::Escape {
            cost: None,
            exile_count,
        })
    }

    /// Create a grantable for a static ability.
    pub fn ability(ability: StaticAbility) -> Self {
        Grantable::Ability(ability)
    }

    /// Create a grantable for playing cards from a non-hand zone as if from hand.
    /// Used by Yawgmoth's Will (graveyard), future effects could grant from exile, etc.
    /// The zone is specified when creating the Grant, not here.
    pub fn play_from() -> Self {
        Grantable::PlayFrom
    }

    /// Get a display string for this grantable.
    pub fn display(&self) -> String {
        match self {
            Grantable::Ability(a) => a.display(),
            Grantable::AlternativeCast(m) => m.name().to_string(),
            Grantable::FlashbackUseTargetsCost => "flashback".to_string(),
            Grantable::PlayFrom => "play from zone".to_string(),
        }
    }
}

/// A grant specification describing what to grant and to whom.
///
/// This is used by both static abilities (permanent grants while source is on battlefield)
/// and one-shot effects (temporary grants with a duration).
#[derive(Debug, Clone, PartialEq)]
pub struct GrantSpec {
    /// What to grant (ability or alternative casting method).
    pub grantable: Grantable,
    /// Filter for cards that receive this grant.
    pub filter: ObjectFilter,
    /// The zone where this grant applies.
    pub zone: Zone,
}

impl GrantSpec {
    /// Create a new grant specification.
    pub fn new(grantable: Grantable, filter: ObjectFilter, zone: Zone) -> Self {
        Self {
            grantable,
            filter,
            zone,
        }
    }

    /// Create a grant spec for flash to noncreature spells in hand.
    pub fn flash_to_noncreature_spells() -> Self {
        Self {
            grantable: Grantable::Ability(StaticAbility::flash()),
            filter: ObjectFilter::noncreature_spell(),
            zone: Zone::Hand,
        }
    }

    /// Create a grant spec for escape to nonland cards in graveyard.
    pub fn escape_to_nonland(exile_count: u32) -> Self {
        Self {
            grantable: Grantable::escape(exile_count),
            filter: ObjectFilter::nonland(),
            zone: Zone::Graveyard,
        }
    }

    /// Get a display string for this grant specification.
    pub fn display(&self) -> String {
        format!("Cards in {:?} have {}", self.zone, self.grantable.display())
    }
}

/// Duration for one-shot grant effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantDuration {
    /// Until end of turn.
    UntilEndOfTurn,
    /// Permanent (for effects that say "gains X" without duration).
    /// Note: This is rare for granted effects - most have a duration.
    Forever,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CardType;

    #[test]
    fn test_grantable_display() {
        let flash = Grantable::Ability(StaticAbility::flash());
        assert_eq!(flash.display(), "Flash");

        let flashback = Grantable::flashback_use_targets_cost();
        assert_eq!(flashback.display(), "flashback");

        let escape = Grantable::escape(3);
        assert_eq!(escape.display(), "Escape");
    }

    #[test]
    fn test_grant_spec_flash_to_noncreature_spells() {
        let spec = GrantSpec::flash_to_noncreature_spells();
        assert_eq!(spec.zone, Zone::Hand);
        assert!(matches!(spec.grantable, Grantable::Ability(_)));
        assert!(
            spec.filter
                .excluded_card_types
                .contains(&CardType::Creature)
        );
    }

    #[test]
    fn test_grant_spec_escape_to_nonland() {
        let spec = GrantSpec::escape_to_nonland(3);
        assert_eq!(spec.zone, Zone::Graveyard);
        assert!(matches!(
            spec.grantable,
            Grantable::AlternativeCast(AlternativeCastingMethod::Escape { exile_count: 3, .. })
        ));
        assert!(spec.filter.excluded_card_types.contains(&CardType::Land));
    }
}
