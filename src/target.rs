//! Target specification system for spells and abilities.
//!
//! Targets in MTG can be players, objects (permanents, spells, cards in zones),
//! or special references like "this permanent" or "you".
//!
//! This module re-exports filter types from the `filter` module for convenience.

use crate::effect::ChoiceCount;
use crate::ids::PlayerId;
use crate::tag::TagKey;

// Re-export all filter types
pub use crate::filter::{
    Comparison, FilterContext, ObjectFilter, ObjectRef, PlayerFilter, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};

/// Specifies what can be chosen or targeted by an effect.
///
/// # Targeting vs Choosing
///
/// In Magic: The Gathering, there's an important distinction between targeting and choosing:
///
/// - **Targeting** (wrap in `Target`): Requires legal target selection during casting/activation,
///   checks hexproof/shroud/protection, spell fizzles if all targets become invalid.
///   Example: "Destroy target creature"
///
/// - **Choosing** (no `Target` wrapper): Selection happens during resolution or cost payment,
///   doesn't check hexproof/shroud/protection, cannot cause fizzle.
///   Example: "Choose a creature you control" (for sacrifice costs)
///
/// Use `ChooseSpec::target(...)` to wrap a spec when it represents a target.
#[derive(Debug, Clone, PartialEq)]
pub enum ChooseSpec {
    /// Wraps another spec to indicate this is a TARGET.
    ///
    /// Effects with targeted specs:
    /// - Require target selection during casting/activation
    /// - Check hexproof, shroud, protection
    /// - Cause the spell/ability to fizzle if all targets become invalid
    ///
    /// Example: `ChooseSpec::target(ChooseSpec::creature())` for "target creature"
    Target(Box<ChooseSpec>),

    /// Choose one or more players matching a filter (not targeting)
    Player(PlayerFilter),

    /// Choose one or more objects matching a filter (not targeting)
    Object(ObjectFilter),

    /// A specific object (used after targeting is locked in)
    SpecificObject(crate::ids::ObjectId),

    /// A specific player (used after targeting is locked in)
    SpecificPlayer(PlayerId),

    /// Any target (creature, planeswalker, battle, or player) - the MTG "any target" text
    /// Note: This is inherently a target, so it doesn't need Target wrapper
    AnyTarget,

    /// The source of this ability/effect ("this permanent", "this spell")
    Source,

    /// The controller of the source (\"you\" on a permanent's ability)
    SourceController,

    /// The owner of the source
    SourceOwner,

    /// Reference objects tagged by a prior effect in the same spell/ability.
    ///
    /// This enables patterns like:
    /// - "Choose a creature. Sacrifice it." (choose_objects tags, sacrifice uses Tagged)
    /// - "Destroy target permanent. Its controller loses 2 life." (destroy tags, lose_life uses Tagged)
    ///
    /// When resolved, looks up the tagged objects from ExecutionContext.tagged_objects.
    Tagged(TagKey),

    /// All objects matching a filter (non-targeted, no choice).
    ///
    /// Used for effects like "Destroy all creatures" where there's no selection -
    /// the effect simply applies to everything matching the filter.
    All(ObjectFilter),

    /// All players matching a filter.
    ///
    /// Used for effects like "Each player discards a card".
    EachPlayer(PlayerFilter),

    /// The current object being iterated in a ForEach loop.
    ///
    /// Used inside `ForEachObject` effects to reference the current iteration target.
    Iterated,

    /// Wraps another spec with a custom count (min/max).
    ///
    /// Used for effects like "up to two target creatures" or "any number of target spells".
    /// The default count (if not wrapped) is exactly 1.
    ///
    /// Example: `ChooseSpec::target_creature().with_count(ChoiceCount::up_to(2))`
    WithCount(Box<ChooseSpec>, ChoiceCount),
}

impl ChooseSpec {
    // ========================================================================
    // Target wrapper
    // ========================================================================

    /// Wrap a spec to indicate it's a target (can fizzle, checks hexproof).
    ///
    /// Use this for effects that say "target" in their text:
    /// - "Destroy target creature" → `ChooseSpec::target(ChooseSpec::creature())`
    /// - "Target player discards" → `ChooseSpec::target(ChooseSpec::any_player())`
    ///
    /// Do NOT use for choices:
    /// - "Choose a creature you control" → `ChooseSpec::Object(ObjectFilter::creature().you_control())`
    /// - "Sacrifice a creature" → use ChooseObjectsEffect + Tagged
    pub fn target(inner: ChooseSpec) -> Self {
        if inner.is_target() {
            inner
        } else {
            Self::Target(Box::new(inner))
        }
    }

    /// Returns true if this spec represents a target (can fizzle, checks hexproof).
    pub fn is_target(&self) -> bool {
        match self {
            Self::Target(_) | Self::AnyTarget => true,
            Self::WithCount(inner, _) => inner.is_target(),
            _ => false,
        }
    }

    /// Unwrap the Target wrapper if present, returning the inner spec.
    /// Returns self if not wrapped.
    pub fn inner(&self) -> &ChooseSpec {
        match self {
            Self::Target(inner) => inner.as_ref(),
            Self::WithCount(inner, _) => inner.inner(),
            _ => self,
        }
    }

    /// Unwrap Target and WithCount recursively to get the base spec.
    pub fn base(&self) -> &ChooseSpec {
        match self {
            Self::Target(inner) => inner.base(),
            Self::WithCount(inner, _) => inner.base(),
            _ => self,
        }
    }

    // ========================================================================
    // Count wrapper
    // ========================================================================

    /// Wrap this spec with a custom count (min/max).
    ///
    /// Example: `ChooseSpec::target_creature().with_count(ChoiceCount::up_to(2))`
    pub fn with_count(self, count: ChoiceCount) -> Self {
        match self {
            Self::WithCount(inner, _) => Self::WithCount(inner, count),
            other => Self::WithCount(Box::new(other), count),
        }
    }

    /// Get the count for this spec.
    ///
    /// Returns the explicit count if wrapped with `WithCount`, otherwise `exactly(1)`.
    pub fn count(&self) -> ChoiceCount {
        match self {
            Self::WithCount(_, count) => *count,
            Self::Target(inner) => inner.count(),
            _ => ChoiceCount::default(), // exactly(1)
        }
    }

    /// Returns true if this spec selects exactly one object/player.
    pub fn is_single(&self) -> bool {
        self.count().is_single()
    }

    // ========================================================================
    // All / Each constructors
    // ========================================================================

    /// Create a spec for all objects matching a filter (no choice, no targeting).
    ///
    /// Example: "Destroy all creatures" → `ChooseSpec::all(ObjectFilter::creature())`
    pub fn all(filter: ObjectFilter) -> Self {
        Self::All(filter)
    }

    /// Create a spec for all creatures.
    pub fn all_creatures() -> Self {
        Self::All(ObjectFilter::creature())
    }

    /// Create a spec for all permanents.
    pub fn all_permanents() -> Self {
        Self::All(ObjectFilter::permanent())
    }

    /// Create a spec for each player matching a filter.
    ///
    /// Example: "Each player discards" → `ChooseSpec::each_player(PlayerFilter::Any)`
    pub fn each_player(filter: PlayerFilter) -> Self {
        Self::EachPlayer(filter)
    }

    /// Create a spec for all opponents.
    pub fn each_opponent() -> Self {
        Self::EachPlayer(PlayerFilter::Opponent)
    }

    /// Create a spec for the iterated object in a ForEach loop.
    pub fn iterated() -> Self {
        Self::Iterated
    }

    /// Returns true if this is an "all matching" spec (no choice involved).
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All(_) | Self::EachPlayer(_))
    }

    // ========================================================================
    // Object specs (for choosing, not targeting)
    // ========================================================================

    /// Create a spec for choosing any creature (not a target).
    /// For "target creature", use `ChooseSpec::target(ChooseSpec::creature())`
    pub fn creature() -> Self {
        Self::Object(ObjectFilter::creature())
    }

    /// Create a spec for choosing any permanent (not a target).
    /// For "target permanent", use `ChooseSpec::target(ChooseSpec::permanent())`
    pub fn permanent() -> Self {
        Self::Object(ObjectFilter::permanent())
    }

    /// Create a spec for choosing any spell (not a target).
    /// For "target spell", use `ChooseSpec::target(ChooseSpec::spell())`
    pub fn spell() -> Self {
        Self::Object(ObjectFilter::spell())
    }

    /// Create a spec for a card in a specific zone.
    pub fn card_in_zone(zone: crate::zone::Zone) -> Self {
        Self::Object(ObjectFilter::default().in_zone(zone))
    }

    // ========================================================================
    // Player specs (for choosing, not targeting)
    // ========================================================================

    /// Create a spec for choosing any player (not a target).
    /// For "target player", use `ChooseSpec::target(ChooseSpec::any_player())`
    pub fn any_player() -> Self {
        Self::Player(PlayerFilter::Any)
    }

    /// Create a spec for choosing an opponent (not a target).
    /// For "target opponent", use `ChooseSpec::target(ChooseSpec::opponent())`
    pub fn opponent() -> Self {
        Self::Player(PlayerFilter::Opponent)
    }

    /// Create a spec for referring to yourself ("you").
    pub fn you() -> Self {
        Self::Player(PlayerFilter::You)
    }

    // ========================================================================
    // Reference specs
    // ========================================================================

    /// Create a spec referencing objects tagged by a prior effect.
    ///
    /// Example: Sacrifice effect referencing objects chosen by ChooseObjectsEffect
    /// ```ignore
    /// vec![
    ///     Effect::choose_objects(ObjectFilter::creature().you_control(), 1, PlayerFilter::You, "sac"),
    ///     Effect::sacrifice(ChooseSpec::tagged("sac")),
    /// ]
    /// ```
    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::Tagged(tag.into())
    }

    // ========================================================================
    // Convenience targeted constructors
    // ========================================================================

    /// "target creature" - shorthand for `ChooseSpec::target(ChooseSpec::creature())`
    pub fn target_creature() -> Self {
        Self::target(Self::creature())
    }

    /// "target permanent" - shorthand for `ChooseSpec::target(ChooseSpec::permanent())`
    pub fn target_permanent() -> Self {
        Self::target(Self::permanent())
    }

    /// "target player" - shorthand for `ChooseSpec::target(ChooseSpec::any_player())`
    pub fn target_player() -> Self {
        Self::target(Self::any_player())
    }

    /// "target opponent" - shorthand for `ChooseSpec::target(ChooseSpec::opponent())`
    pub fn target_opponent() -> Self {
        Self::target(Self::opponent())
    }

    /// "target spell" - shorthand for `ChooseSpec::target(ChooseSpec::spell())`
    pub fn target_spell() -> Self {
        Self::target(Self::spell())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_comparison() {
        assert!(Comparison::Equal(5).satisfies(5));
        assert!(!Comparison::Equal(5).satisfies(4));

        assert!(Comparison::LessThanOrEqual(2).satisfies(2));
        assert!(Comparison::LessThanOrEqual(2).satisfies(1));
        assert!(!Comparison::LessThanOrEqual(2).satisfies(3));

        assert!(Comparison::GreaterThan(3).satisfies(4));
        assert!(!Comparison::GreaterThan(3).satisfies(3));
    }

    #[test]
    fn test_creature_filter() {
        let filter = ObjectFilter::creature();
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn test_creature_you_control() {
        let filter = ObjectFilter::creature().you_control();
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn test_creature_with_power_2_or_less() {
        let filter = ObjectFilter::creature().with_power(Comparison::LessThanOrEqual(2));
        assert!(filter.power.is_some());
    }

    #[test]
    fn test_nonland_permanent() {
        let filter = ObjectFilter::nonland_permanent();
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.excluded_card_types, vec![CardType::Land]);
    }

    #[test]
    fn test_target_spec_builders() {
        let creature = ChooseSpec::creature();
        assert!(matches!(creature, ChooseSpec::Object(_)));

        let opponent = ChooseSpec::opponent();
        assert!(matches!(
            opponent,
            ChooseSpec::Player(PlayerFilter::Opponent)
        ));
    }

    #[test]
    fn test_target_wrapper() {
        // Non-targeted spec
        let creature = ChooseSpec::creature();
        assert!(!creature.is_target());

        // Targeted spec
        let target_creature = ChooseSpec::target(ChooseSpec::creature());
        assert!(target_creature.is_target());

        // AnyTarget is inherently a target
        let any_target = ChooseSpec::AnyTarget;
        assert!(any_target.is_target());
    }

    #[test]
    fn test_target_inner() {
        let target_creature = ChooseSpec::target(ChooseSpec::creature());

        // inner() should unwrap one level
        let inner = target_creature.inner();
        assert!(matches!(inner, ChooseSpec::Object(_)));

        // For non-wrapped specs, inner() returns self
        let creature = ChooseSpec::creature();
        let inner = creature.inner();
        assert!(matches!(inner, ChooseSpec::Object(_)));
    }

    #[test]
    fn test_target_convenience_constructors() {
        let target_creature = ChooseSpec::target_creature();
        assert!(target_creature.is_target());
        assert!(matches!(target_creature.inner(), ChooseSpec::Object(_)));

        let target_player = ChooseSpec::target_player();
        assert!(target_player.is_target());
        assert!(matches!(target_player.inner(), ChooseSpec::Player(_)));

        let target_spell = ChooseSpec::target_spell();
        assert!(target_spell.is_target());
    }

    #[test]
    fn test_tagged_is_not_target() {
        let tagged = ChooseSpec::tagged("sac");
        assert!(!tagged.is_target());
    }

    #[test]
    fn test_complex_filter() {
        // "Target nontoken creature an opponent controls with power 3 or greater"
        let filter = ObjectFilter::creature()
            .opponent_controls()
            .nontoken()
            .with_power(Comparison::GreaterThanOrEqual(3));

        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
        assert!(filter.nontoken);
        assert_eq!(filter.power, Some(Comparison::GreaterThanOrEqual(3)));
    }
}
