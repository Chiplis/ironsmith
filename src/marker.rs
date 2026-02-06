//! Marker system for counters, emblems, and designations.
//!
//! Markers are game elements that can be placed on objects, players, or the game itself.
//! This module provides:
//! - `Marker` - the different types of markers
//! - `MarkerLocation` - where a marker can be placed
//! - `MarkerFilter` - for filtering/targeting markers
//!
//! # Examples
//!
//! ```ignore
//! // Hex Parasite: "Remove up to X counters from target permanent"
//! let filter = MarkerFilter::any_counter().on_permanent();
//!
//! // "Whenever a +1/+1 counter is placed on a creature you control"
//! let filter = MarkerFilter::counter(CounterType::PlusOnePlusOne)
//!     .on(ObjectFilter::creature().you_control());
//! ```

use crate::filter::{ObjectFilter, PlayerFilter};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;

/// A marker that can be placed on objects, players, or the game.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Marker {
    /// A counter on a permanent or player.
    Counter(CounterType),
    // Future: Emblem, Designation (monarch, city's blessing, day/night, etc.)
}

impl Marker {
    /// Create a counter marker.
    pub fn counter(counter_type: CounterType) -> Self {
        Marker::Counter(counter_type)
    }

    /// Get the counter type if this is a counter marker.
    pub fn as_counter(&self) -> Option<CounterType> {
        match self {
            Marker::Counter(ct) => Some(*ct),
        }
    }
}

impl From<CounterType> for Marker {
    fn from(counter_type: CounterType) -> Self {
        Marker::Counter(counter_type)
    }
}

/// Where a marker is located.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MarkerLocation {
    /// On a game object (permanent, card, etc.).
    Object(ObjectId),
    /// On a player.
    Player(PlayerId),
    // Future: Game (for global designations like day/night)
}

impl MarkerLocation {
    /// Create an object location.
    pub fn object(id: ObjectId) -> Self {
        MarkerLocation::Object(id)
    }

    /// Create a player location.
    pub fn player(id: PlayerId) -> Self {
        MarkerLocation::Player(id)
    }

    /// Get the object ID if this is an object location.
    pub fn as_object(&self) -> Option<ObjectId> {
        match self {
            MarkerLocation::Object(id) => Some(*id),
            _ => None,
        }
    }

    /// Get the player ID if this is a player location.
    pub fn as_player(&self) -> Option<PlayerId> {
        match self {
            MarkerLocation::Player(id) => Some(*id),
            _ => None,
        }
    }
}

impl From<ObjectId> for MarkerLocation {
    fn from(id: ObjectId) -> Self {
        MarkerLocation::Object(id)
    }
}

impl From<PlayerId> for MarkerLocation {
    fn from(id: PlayerId) -> Self {
        MarkerLocation::Player(id)
    }
}

/// Filter for matching marker types.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MarkerTypeFilter {
    /// Match any marker.
    #[default]
    Any,
    /// Match any counter (but not emblems or designations).
    AnyCounter,
    /// Match a specific counter type.
    Counter(CounterType),
    /// Match any of these counter types.
    CounterOneOf(Vec<CounterType>),
}

impl MarkerTypeFilter {
    /// Check if a marker matches this filter.
    pub fn matches(&self, marker: &Marker) -> bool {
        match self {
            MarkerTypeFilter::Any => true,
            MarkerTypeFilter::AnyCounter => matches!(marker, Marker::Counter(_)),
            MarkerTypeFilter::Counter(ct) => marker.as_counter() == Some(*ct),
            MarkerTypeFilter::CounterOneOf(types) => marker
                .as_counter()
                .map(|ct| types.contains(&ct))
                .unwrap_or(false),
        }
    }
}

/// Filter for matching marker locations.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MarkerLocationFilter {
    /// Match any location.
    #[default]
    Any,
    /// Match markers on objects matching a filter.
    Object(ObjectFilter),
    /// Match markers on players matching a filter.
    Player(PlayerFilter),
}

/// Filter for matching markers.
///
/// Combines type and location filters to find specific markers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarkerFilter {
    /// Filter on marker type.
    pub marker_type: MarkerTypeFilter,
    /// Filter on marker location.
    pub location: MarkerLocationFilter,
}

impl MarkerFilter {
    /// Create a filter for any marker.
    pub fn any() -> Self {
        Self::default()
    }

    /// Create a filter for any counter.
    pub fn any_counter() -> Self {
        Self {
            marker_type: MarkerTypeFilter::AnyCounter,
            ..Default::default()
        }
    }

    /// Create a filter for a specific counter type.
    pub fn counter(counter_type: CounterType) -> Self {
        Self {
            marker_type: MarkerTypeFilter::Counter(counter_type),
            ..Default::default()
        }
    }

    /// Create a filter for any of these counter types.
    pub fn counter_one_of(types: Vec<CounterType>) -> Self {
        Self {
            marker_type: MarkerTypeFilter::CounterOneOf(types),
            ..Default::default()
        }
    }

    /// Restrict to markers on permanents.
    pub fn on_permanent(mut self) -> Self {
        self.location = MarkerLocationFilter::Object(ObjectFilter::permanent());
        self
    }

    /// Restrict to markers on objects matching a filter.
    pub fn on(mut self, filter: ObjectFilter) -> Self {
        self.location = MarkerLocationFilter::Object(filter);
        self
    }

    /// Restrict to markers on players matching a filter.
    pub fn on_player(mut self, filter: PlayerFilter) -> Self {
        self.location = MarkerLocationFilter::Player(filter);
        self
    }
}

/// Extension trait for CounterType to get granted abilities.
impl CounterType {
    /// Returns the static ability this counter grants, if any.
    ///
    /// Ability counters from Ikoria grant keyword abilities to the permanent
    /// they're on. This is a continuous effect from the counter itself.
    pub fn granted_ability(&self) -> Option<StaticAbilityId> {
        match self {
            CounterType::Deathtouch => Some(StaticAbilityId::Deathtouch),
            CounterType::Flying => Some(StaticAbilityId::Flying),
            CounterType::FirstStrike => Some(StaticAbilityId::FirstStrike),
            CounterType::DoubleStrike => Some(StaticAbilityId::DoubleStrike),
            CounterType::Hexproof => Some(StaticAbilityId::Hexproof),
            CounterType::Indestructible => Some(StaticAbilityId::Indestructible),
            CounterType::Lifelink => Some(StaticAbilityId::Lifelink),
            CounterType::Menace => Some(StaticAbilityId::Menace),
            CounterType::Reach => Some(StaticAbilityId::Reach),
            CounterType::Trample => Some(StaticAbilityId::Trample),
            CounterType::Vigilance => Some(StaticAbilityId::Vigilance),
            CounterType::Haste => Some(StaticAbilityId::Haste),
            _ => None,
        }
    }

    /// Returns true if this counter grants an ability.
    pub fn is_ability_counter(&self) -> bool {
        self.granted_ability().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_creation() {
        let counter = Marker::counter(CounterType::PlusOnePlusOne);
        assert_eq!(counter.as_counter(), Some(CounterType::PlusOnePlusOne));
    }

    #[test]
    fn test_marker_location() {
        let obj_loc = MarkerLocation::object(ObjectId::from_raw(1));
        assert_eq!(obj_loc.as_object(), Some(ObjectId::from_raw(1)));
        assert_eq!(obj_loc.as_player(), None);

        let player_loc = MarkerLocation::player(PlayerId::from_index(0));
        assert_eq!(player_loc.as_player(), Some(PlayerId::from_index(0)));
        assert_eq!(player_loc.as_object(), None);
    }

    #[test]
    fn test_marker_type_filter() {
        let any = MarkerTypeFilter::Any;
        let any_counter = MarkerTypeFilter::AnyCounter;
        let specific = MarkerTypeFilter::Counter(CounterType::PlusOnePlusOne);

        let plus_one = Marker::counter(CounterType::PlusOnePlusOne);
        let loyalty = Marker::counter(CounterType::Loyalty);

        assert!(any.matches(&plus_one));
        assert!(any.matches(&loyalty));
        assert!(any_counter.matches(&plus_one));
        assert!(any_counter.matches(&loyalty));
        assert!(specific.matches(&plus_one));
        assert!(!specific.matches(&loyalty));
    }

    #[test]
    fn test_marker_filter_builders() {
        let any = MarkerFilter::any();
        assert_eq!(any.marker_type, MarkerTypeFilter::Any);

        let any_counter = MarkerFilter::any_counter();
        assert_eq!(any_counter.marker_type, MarkerTypeFilter::AnyCounter);

        let specific = MarkerFilter::counter(CounterType::Charge);
        assert_eq!(
            specific.marker_type,
            MarkerTypeFilter::Counter(CounterType::Charge)
        );
    }

    #[test]
    fn test_ability_granting_counters() {
        assert_eq!(
            CounterType::Deathtouch.granted_ability(),
            Some(StaticAbilityId::Deathtouch)
        );
        assert_eq!(
            CounterType::Flying.granted_ability(),
            Some(StaticAbilityId::Flying)
        );
        assert_eq!(
            CounterType::Trample.granted_ability(),
            Some(StaticAbilityId::Trample)
        );

        // Non-ability counters
        assert_eq!(CounterType::PlusOnePlusOne.granted_ability(), None);
        assert_eq!(CounterType::Loyalty.granted_ability(), None);
        assert_eq!(CounterType::Charge.granted_ability(), None);
    }

    #[test]
    fn test_is_ability_counter() {
        assert!(CounterType::Deathtouch.is_ability_counter());
        assert!(CounterType::Flying.is_ability_counter());
        assert!(!CounterType::PlusOnePlusOne.is_ability_counter());
        assert!(!CounterType::Loyalty.is_ability_counter());
    }
}
