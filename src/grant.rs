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
//!     grantable: Grantable::escape(3),
//!     filter: ObjectFilter::nonland(),
//!     zone: Zone::Graveyard,
//! })
//!
//! // Grant flashback until end of turn (Snapcaster Mage)
//! Effect::grant(
//!     Grantable::flashback_from_cards_mana_cost(),
//!     target,
//!     GrantDuration::UntilEndOfTurn,
//! )
//! ```

use crate::alternative_cast::AlternativeCastingMethod;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::object::Object;
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

/// A granted alternative cast whose exact cost is derived from the granted card.
#[derive(Debug, Clone, PartialEq)]
pub enum DerivedAlternativeCast {
    /// Flashback using the card's mana cost plus optional extra cost components.
    FlashbackFromCardManaCost { additional_costs: Vec<Cost> },
    /// Escape using the card's mana cost and exiling N other graveyard cards.
    EscapeFromCardManaCost { exile_count: u32 },
    /// Cast from hand by paying generic mana equal to the card's mana value.
    ManaValueAsGenericFromHand,
}

impl DerivedAlternativeCast {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FlashbackFromCardManaCost { .. } => "flashback",
            Self::EscapeFromCardManaCost { .. } => "Escape",
            Self::ManaValueAsGenericFromHand => "Pay mana value",
        }
    }

    pub fn materialize_for(&self, card: &Object) -> Option<AlternativeCastingMethod> {
        match self {
            Self::FlashbackFromCardManaCost { additional_costs } => {
                let mana_cost = card.mana_cost.clone()?;
                if !card.has_card_type(CardType::Instant) && !card.has_card_type(CardType::Sorcery)
                {
                    return None;
                }
                if card.zone != Zone::Graveyard {
                    return None;
                }

                let mut costs = vec![Cost::mana(mana_cost)];
                costs.extend(additional_costs.iter().cloned());
                Some(AlternativeCastingMethod::Flashback {
                    total_cost: TotalCost::from_costs(costs),
                })
            }
            Self::EscapeFromCardManaCost { exile_count } => {
                let mana_cost = card.mana_cost.clone()?;
                if card.zone != Zone::Graveyard {
                    return None;
                }
                Some(AlternativeCastingMethod::Escape {
                    cost: Some(mana_cost),
                    exile_count: *exile_count,
                })
            }
            Self::ManaValueAsGenericFromHand => {
                if card.zone != Zone::Hand {
                    return None;
                }
                let mana_value =
                    u8::try_from(card.mana_cost.as_ref().map_or(0, |c| c.mana_value())).ok()?;
                Some(AlternativeCastingMethod::alternative_cost(
                    "Pay mana value",
                    Some(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(mana_value),
                    ])),
                    Vec::new(),
                ))
            }
        }
    }
}

/// What can be granted to a card.
#[derive(Debug, Clone, PartialEq)]
pub enum Grantable {
    /// Grant a static ability (flash, flying, hexproof, etc.)
    Ability(StaticAbility),
    /// Grant an alternative casting method (flashback, escape, etc.)
    AlternativeCast(AlternativeCastingMethod),
    /// Grant an alternative casting method whose exact cost is derived from the card.
    DerivedAlternativeCast(DerivedAlternativeCast),
    /// Grant the ability to play a card from a non-hand zone as if it were in hand.
    /// This allows using the card's normal mana cost AND any alternative costs it has.
    /// Used by Yawgmoth's Will (graveyard), future effects could grant from exile, etc.
    /// The zone is specified in the GrantSpec, not here.
    PlayFrom,
}

impl Grantable {
    /// Create a grantable for flashback that uses the granted card's mana cost.
    pub fn flashback_from_cards_mana_cost() -> Self {
        Grantable::DerivedAlternativeCast(DerivedAlternativeCast::FlashbackFromCardManaCost {
            additional_costs: Vec::new(),
        })
    }

    /// Create a grantable for escape with the given exile count and the granted card's mana cost.
    pub fn escape(exile_count: u32) -> Self {
        Grantable::DerivedAlternativeCast(DerivedAlternativeCast::EscapeFromCardManaCost {
            exile_count,
        })
    }

    /// Create a grantable for casting from hand by paying generic mana equal
    /// to the granted card's mana value.
    pub fn mana_value_as_generic_from_hand() -> Self {
        Grantable::DerivedAlternativeCast(DerivedAlternativeCast::ManaValueAsGenericFromHand)
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
            Grantable::DerivedAlternativeCast(spec) => spec.display_name().to_string(),
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
    /// Which player may use the grant when rendered or applied statically.
    pub beneficiary: PlayerFilter,
}

impl GrantSpec {
    /// Create a new grant specification.
    pub fn new(grantable: Grantable, filter: ObjectFilter, zone: Zone) -> Self {
        Self {
            grantable,
            filter,
            zone,
            beneficiary: PlayerFilter::You,
        }
    }

    /// Return a copy of this grant specification with an explicit beneficiary.
    pub fn with_beneficiary(mut self, beneficiary: PlayerFilter) -> Self {
        self.beneficiary = beneficiary;
        self
    }

    /// Create a grant spec for flash to spells in hand.
    pub fn flash_to_spells() -> Self {
        Self::flash_to_spells_matching(ObjectFilter::nonland())
    }

    /// Create a grant spec for flash to matching spells in hand.
    pub fn flash_to_spells_matching(filter: ObjectFilter) -> Self {
        Self {
            grantable: Grantable::Ability(StaticAbility::flash()),
            filter,
            zone: Zone::Hand,
            beneficiary: PlayerFilter::You,
        }
    }

    /// Create a grant spec for flash to noncreature spells in hand.
    pub fn flash_to_noncreature_spells() -> Self {
        Self::flash_to_spells_matching(ObjectFilter::noncreature_spell())
    }

    /// Create a grant spec for playing cards from your graveyard.
    pub fn play_from_graveyard() -> Self {
        Self::new(
            Grantable::play_from(),
            ObjectFilter::default(),
            Zone::Graveyard,
        )
    }

    /// Create a grant spec for playing lands from your graveyard.
    pub fn play_lands_from_graveyard() -> Self {
        Self::new(
            Grantable::play_from(),
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
        )
    }

    /// Create a grant spec for casting matching spells from hand without paying mana cost.
    pub fn cast_from_hand_without_paying_mana_cost_matching(filter: ObjectFilter) -> Self {
        Self::new(
            Grantable::AlternativeCast(AlternativeCastingMethod::alternative_cost(
                "Without paying mana cost",
                None,
                Vec::new(),
            )),
            filter,
            Zone::Hand,
        )
    }

    /// Create a grant spec for escape to nonland cards in graveyard.
    pub fn escape_to_nonland(exile_count: u32) -> Self {
        Self {
            grantable: Grantable::escape(exile_count),
            filter: ObjectFilter::nonland(),
            zone: Zone::Graveyard,
            beneficiary: PlayerFilter::You,
        }
    }

    /// Get a display string for this grant specification.
    pub fn display(&self) -> String {
        fn small_number_word(n: u32) -> Option<&'static str> {
            match n {
                0 => Some("zero"),
                1 => Some("one"),
                2 => Some("two"),
                3 => Some("three"),
                4 => Some("four"),
                5 => Some("five"),
                6 => Some("six"),
                7 => Some("seven"),
                8 => Some("eight"),
                9 => Some("nine"),
                10 => Some("ten"),
                _ => None,
            }
        }

        fn zone_name(zone: Zone) -> &'static str {
            match zone {
                Zone::Battlefield => "battlefield",
                Zone::Hand => "hand",
                Zone::Library => "library",
                Zone::Graveyard => "graveyard",
                Zone::Exile => "exile",
                Zone::Stack => "stack",
                Zone::Command => "command zone",
            }
        }

        fn castable_filter_description(filter: &ObjectFilter) -> String {
            if !filter.any_of.is_empty() {
                return filter
                    .any_of
                    .iter()
                    .map(castable_filter_description)
                    .collect::<Vec<_>>()
                    .join(" or ");
            }
            if *filter == ObjectFilter::noncreature_spell() {
                return "noncreature spells".to_string();
            }
            let description = filter.description();
            if description.contains("permanent") {
                description.replace("permanent", "spell")
            } else if description.contains("spell") || description.contains("card") {
                description
            } else {
                format!("{description} spells")
            }
        }

        fn beneficiary_may_prefix(beneficiary: &PlayerFilter) -> String {
            match beneficiary {
                PlayerFilter::Any => "Any player may".to_string(),
                PlayerFilter::You => "You may".to_string(),
                PlayerFilter::NotYou => "Any player other than you may".to_string(),
                PlayerFilter::Opponent => "Opponent may".to_string(),
                PlayerFilter::Teammate => "A teammate may".to_string(),
                PlayerFilter::Active => "The active player may".to_string(),
                PlayerFilter::Defending => "The defending player may".to_string(),
                PlayerFilter::Attacking => "The attacking player may".to_string(),
                PlayerFilter::DamagedPlayer => "That player may".to_string(),
                PlayerFilter::EffectController => "The player who cast this spell may".to_string(),
                PlayerFilter::Specific(_) => "That player may".to_string(),
                PlayerFilter::MostLifeTied => {
                    "The player with the most life or tied for most life may".to_string()
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
                    "Any player who cast one or more {} spells this turn may",
                    card_type.to_string().to_ascii_lowercase()
                ),
                PlayerFilter::ChosenPlayer => "The chosen player may".to_string(),
                PlayerFilter::TaggedPlayer(_)
                | PlayerFilter::IteratedPlayer
                | PlayerFilter::AliasedOwnerOf(_)
                | PlayerFilter::AliasedControllerOf(_) => "That player may".to_string(),
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    "That player or that object's controller may".to_string()
                }
                PlayerFilter::Excluding { .. } => "A player may".to_string(),
                PlayerFilter::Target(_) => "Target player may".to_string(),
                PlayerFilter::ControllerOf(_) => "That object's controller may".to_string(),
                PlayerFilter::OwnerOf(_) => "That object's owner may".to_string(),
            }
        }

        let mut filter = self.filter.clone();
        filter.zone.get_or_insert(self.zone);
        let filter_desc = filter.description();
        let may_prefix = beneficiary_may_prefix(&self.beneficiary);

        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter.card_types.as_slice() == [CardType::Land]
        {
            return format!("{may_prefix} play lands from your graveyard");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter == ObjectFilter::default()
        {
            return format!("{may_prefix} play lands and cast spells from your graveyard");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter == ObjectFilter::default()
        {
            return format!("{may_prefix} play lands and cast spells from the top of your library");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter.card_types.as_slice() == [CardType::Land]
        {
            return format!("{may_prefix} play lands from the top of your library");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter.any_of.len() == 2
        {
            let land_branch = self
                .filter
                .any_of
                .iter()
                .find(|branch| branch.card_types.as_slice() == [CardType::Land]);
            let other_branch = self
                .filter
                .any_of
                .iter()
                .find(|branch| branch.card_types.as_slice() != [CardType::Land]);
            if land_branch.is_some()
                && let Some(other) = other_branch
            {
                return format!(
                    "{may_prefix} play lands and cast {} from the top of your library",
                    castable_filter_description(other)
                );
            }
        }
        if matches!(self.grantable, Grantable::PlayFrom) && self.zone == Zone::Library {
            return format!(
                "{may_prefix} play {} from the top of your library",
                filter_desc
            );
        }
        if let Grantable::AlternativeCast(method) = &self.grantable
            && self.zone == Zone::Hand
            && self.filter == ObjectFilter::nonland()
            && method.cast_from_zone() == Zone::Hand
            && method.mana_cost().is_none()
            && method.non_mana_costs().is_empty()
        {
            return format!(
                "{may_prefix} cast spells from your hand without paying their mana costs"
            );
        }
        if let Grantable::DerivedAlternativeCast(DerivedAlternativeCast::EscapeFromCardManaCost {
            exile_count,
        }) = &self.grantable
            && self.zone == Zone::Graveyard
        {
            let count_text = small_number_word(*exile_count)
                .map(str::to_string)
                .unwrap_or_else(|| exile_count.to_string());
            let graveyard = if matches!(filter.owner, Some(crate::filter::PlayerFilter::You)) {
                "your graveyard"
            } else {
                "that graveyard"
            };
            return format!(
                "Each {filter_desc} has escape. The escape cost is equal to the card's mana cost plus exile {count_text} other cards from {graveyard}"
            );
        }
        if let Grantable::DerivedAlternativeCast(DerivedAlternativeCast::ManaValueAsGenericFromHand) =
            &self.grantable
            && self.zone == Zone::Hand
        {
            return format!(
                "{may_prefix} pay {{X}} rather than pay the mana cost for {} you cast, where X is that spell's mana value",
                castable_filter_description(&self.filter)
            );
        }
        if let Grantable::Ability(ability) = &self.grantable
            && ability.has_flash()
            && self.zone == Zone::Hand
        {
            if self.filter == ObjectFilter::nonland() {
                return format!("{may_prefix} cast spells as though they had flash");
            }
            return format!(
                "{may_prefix} cast {} as though they had flash",
                castable_filter_description(&self.filter)
            );
        }
        format!(
            "Cards in {} have {}",
            zone_name(self.zone),
            self.grantable.display()
        )
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

        let flashback = Grantable::flashback_from_cards_mana_cost();
        assert_eq!(flashback.display(), "flashback");

        let escape = Grantable::escape(3);
        assert_eq!(escape.display(), "Escape");

        let mana_value = Grantable::mana_value_as_generic_from_hand();
        assert_eq!(mana_value.display(), "Pay mana value");
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
        assert_eq!(
            spec.display(),
            "You may cast noncreature spells as though they had flash"
        );
    }

    #[test]
    fn test_grant_spec_flash_to_spells() {
        let spec = GrantSpec::flash_to_spells();
        assert_eq!(spec.zone, Zone::Hand);
        assert!(matches!(spec.grantable, Grantable::Ability(_)));
        assert!(spec.filter.excluded_card_types.contains(&CardType::Land));
        assert_eq!(
            spec.display(),
            "You may cast spells as though they had flash"
        );
    }

    #[test]
    fn test_grant_spec_flash_to_spells_any_player_display() {
        let spec = GrantSpec::flash_to_spells().with_beneficiary(PlayerFilter::Any);
        assert_eq!(
            spec.display(),
            "Any player may cast spells as though they had flash"
        );
    }

    #[test]
    fn test_grant_spec_escape_to_nonland() {
        let spec = GrantSpec::escape_to_nonland(3);
        assert_eq!(spec.zone, Zone::Graveyard);
        assert!(matches!(
            spec.grantable,
            Grantable::DerivedAlternativeCast(DerivedAlternativeCast::EscapeFromCardManaCost {
                exile_count: 3
            })
        ));
        assert!(spec.filter.excluded_card_types.contains(&CardType::Land));
    }

    #[test]
    fn test_grant_spec_play_from_graveyard() {
        let spec = GrantSpec::play_from_graveyard();
        assert_eq!(spec.zone, Zone::Graveyard);
        assert_eq!(
            spec.display(),
            "You may play lands and cast spells from your graveyard"
        );
    }

    #[test]
    fn test_grant_spec_play_lands_from_graveyard_uses_nonbattlefield_land_filter() {
        let spec = GrantSpec::play_lands_from_graveyard();
        assert_eq!(spec.zone, Zone::Graveyard);
        assert_eq!(spec.filter.card_types, vec![CardType::Land]);
        assert_eq!(
            spec.filter.zone, None,
            "grant zone already scopes graveyard land permissions"
        );
        assert_eq!(spec.display(), "You may play lands from your graveyard");
    }

    #[test]
    fn test_grant_spec_cast_from_hand_without_paying_mana_cost_matching() {
        let spec =
            GrantSpec::cast_from_hand_without_paying_mana_cost_matching(ObjectFilter::nonland());
        assert_eq!(spec.zone, Zone::Hand);
        assert!(matches!(
            &spec.grantable,
            Grantable::AlternativeCast(method)
                if method.cast_from_zone() == Zone::Hand
                    && method.mana_cost().is_none()
                    && method.non_mana_costs().is_empty()
        ));
        assert_eq!(
            spec.display(),
            "You may cast spells from your hand without paying their mana costs"
        );
    }
}
