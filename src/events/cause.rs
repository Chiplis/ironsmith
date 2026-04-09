//! Event causation tracking for composable replacement effect matching.
//!
//! This module provides types for tracking what caused a game event to happen,
//! enabling replacement effects to match based on the cause (e.g., "if an effect
//! causes you to discard" vs "if you discard as a cost").
//!
//! # Example
//!
//! ```ignore
//! use ironsmith::events::cause::{EventCause, CauseType, CauseFilter};
//!
//! // Library of Leng: only matches discards from effects
//! let filter = CauseFilter::from_effect();
//!
//! // Hypothetical: only matches discards caused by a Goblin
//! let filter = CauseFilter::effect_from_source(ObjectFilter::has_subtype(Goblin));
//! ```

use crate::filter::FilterContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ObjectFilter;

/// What type of game action caused an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CauseType {
    /// Paying a cost (e.g., "Discard a card:" as an activation cost).
    /// Library of Leng does NOT apply to cost-based discards.
    Cost,

    /// A resolving spell or ability effect.
    /// Library of Leng applies to effect-based discards.
    Effect,

    /// State-based actions (e.g., creature with 0 toughness dies).
    StateBasedAction,

    /// Game rules (e.g., cleanup step discard to hand size, draw for turn).
    /// Library of Leng applies to game rule discards (they're effect-like).
    GameRule,

    /// Combat damage assignment.
    CombatDamage,

    /// A special action such as playing a land, foretelling a card, or suspending a card.
    SpecialAction,

    /// A zone change specifically caused by applying the legend rule.
    ///
    /// This is technically a state-based action, but we track it distinctly so cards and
    /// tests can tell it apart from generic SBA-driven moves.
    LegendRule,
    // A replacement effect caused this (the event was transformed).
    // Replacement,

    // Mana ability activation.
    // ManaAbility,
}

impl CauseType {
    /// Returns true if this cause type is considered "from an effect" for
    /// replacement purposes like Library of Leng.
    ///
    /// This intentionally excludes costs, combat damage, state-based actions,
    /// special actions, and legend-rule moves.
    pub fn is_effect_like(&self) -> bool {
        match self {
            CauseType::Effect => true,
            CauseType::GameRule => false,
            CauseType::CombatDamage => false,
            CauseType::Cost => false,
            CauseType::StateBasedAction => false,
            CauseType::SpecialAction => false,
            CauseType::LegendRule => false,
            // Might be needed in the future but not so far, uncomment if required
            // CauseType::ManaAbility => false, // Mana abilities are cost-related
            // CauseType::Replacement => true, // Replacement effects are effect-like
        }
    }
}

/// Context about what caused a game event to happen.
///
/// This is attached to events that care about their cause, enabling
/// replacement effects to match based on cause properties.
#[derive(Debug, Clone)]
pub struct EventCause {
    /// What type of game action caused this event.
    pub cause_type: CauseType,

    /// The source object that caused this event (if any).
    /// For effects, this is the spell/ability on the stack.
    /// For costs, this is the permanent/spell whose cost is being paid.
    pub source: Option<ObjectId>,

    /// The controller of the source (if known).
    pub source_controller: Option<PlayerId>,
}

impl EventCause {
    /// Create a cause from an effect when no specific source information is available.
    pub fn effect() -> Self {
        Self {
            cause_type: CauseType::Effect,
            source: None,
            source_controller: None,
        }
    }

    /// Create a cause from paying a cost.
    pub fn from_cost(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::Cost,
            source: Some(source),
            source_controller: Some(controller),
        }
    }

    /// Create a cause from a resolving effect.
    pub fn from_effect(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::Effect,
            source: Some(source),
            source_controller: Some(controller),
        }
    }

    /// Create a cause from state-based actions.
    pub fn from_sba() -> Self {
        Self {
            cause_type: CauseType::StateBasedAction,
            source: None,
            source_controller: None,
        }
    }

    /// Create a cause from game rules (e.g., cleanup discard).
    pub fn from_game_rule() -> Self {
        Self {
            cause_type: CauseType::GameRule,
            source: None,
            source_controller: None,
        }
    }

    /// Create a cause from combat damage.
    pub fn from_combat_damage(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::CombatDamage,
            source: Some(source),
            source_controller: Some(controller),
        }
    }

    /// Create a combat-damage cause when only the source object is known.
    pub fn combat_damage(source: ObjectId) -> Self {
        Self {
            cause_type: CauseType::CombatDamage,
            source: Some(source),
            source_controller: None,
        }
    }

    // Create a cause from a mana ability.
    // pub fn from_mana_ability(source: ObjectId, controller: PlayerId) -> Self {
    //     Self {
    //         cause_type: CauseType::ManaAbility,
    //         source: Some(source),
    //         source_controller: Some(controller),
    //     }
    // }

    /// Create a cause from a special action.
    pub fn from_special_action(source: Option<ObjectId>, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::SpecialAction,
            source,
            source_controller: Some(controller),
        }
    }

    /// Create a cause from the legend rule.
    pub fn from_legend_rule(controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::LegendRule,
            source: None,
            source_controller: Some(controller),
        }
    }

    // Create a cause from a replacement effect.
    // pub fn from_replacement(source: ObjectId, controller: PlayerId) -> Self {
    //     Self {
    //         cause_type: CauseType::Replacement,
    //         source: Some(source),
    //         source_controller: Some(controller),
    //     }
    // }
}

/// Filter for matching event causes.
///
/// This enables composable matching on cause properties, such as:
/// - "If an effect causes..." (cause type filter)
/// - "If a Goblin causes..." (source object filter)
/// - "If an opponent causes..." (source controller filter)
#[derive(Debug, Clone, PartialEq)]
pub struct CauseFilter {
    /// Filter on cause type. None means match any cause type.
    pub cause_type: Option<CauseTypeFilter>,

    /// Filter on the source object. None means match any source.
    pub source_filter: Option<ObjectFilter>,

    /// Filter on the source controller. None means match any controller.
    pub controller_filter: Option<ControllerFilter>,
}

/// Filter for cause types.
#[derive(Debug, Clone, PartialEq)]
pub enum CauseTypeFilter {
    /// Match a specific cause type.
    Exact(CauseType),

    /// Match any cause type except the given one.
    Not(CauseType),

    /// Match any cause considered "effect-like" by [`CauseType::is_effect_like`].
    EffectLike,

    /// Match any cause type that is NOT a cost.
    NotCost,

    /// Match multiple cause types.
    OneOf(Vec<CauseType>),
}

impl CauseTypeFilter {
    /// Check if a cause type matches this filter.
    pub fn matches(&self, cause_type: CauseType) -> bool {
        match self {
            CauseTypeFilter::Exact(ct) => cause_type == *ct,
            CauseTypeFilter::Not(ct) => cause_type != *ct,
            CauseTypeFilter::EffectLike => cause_type.is_effect_like(),
            CauseTypeFilter::NotCost => cause_type != CauseType::Cost,
            CauseTypeFilter::OneOf(types) => types.contains(&cause_type),
        }
    }
}

/// Filter for source controller.
#[derive(Debug, Clone, PartialEq)]
pub enum ControllerFilter {
    /// Match if controlled by a specific player.
    Player(PlayerId),

    /// Match if controlled by the affected player (same as event's player).
    You,

    /// Match if controlled by an opponent of the affected player.
    Opponent,

    /// Match any controller.
    Any,
}

impl CauseFilter {
    /// Create a filter that matches any cause.
    pub fn any() -> Self {
        Self {
            cause_type: None,
            source_filter: None,
            controller_filter: None,
        }
    }

    /// Create a filter for effect-like causes.
    pub fn effect_like() -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::EffectLike),
            source_filter: None,
            controller_filter: None,
        }
    }

    /// Create a filter for exact cause type match.
    pub fn exact(cause_type: CauseType) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::Exact(cause_type)),
            source_filter: None,
            controller_filter: None,
        }
    }

    /// Create a filter that matches any cause type except the given one.
    pub fn not_type(cause_type: CauseType) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::Not(cause_type)),
            source_filter: None,
            controller_filter: None,
        }
    }

    /// Create a filter for effects only.
    pub fn from_effect() -> Self {
        Self::exact(CauseType::Effect)
    }

    /// Create a filter for costs only.
    pub fn from_cost() -> Self {
        Self::exact(CauseType::Cost)
    }

    /// Create a filter for effect-like causes from a specific source type.
    ///
    /// Example: "If a Goblin's effect causes you to discard..."
    pub fn effect_from_source(source_filter: ObjectFilter) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::EffectLike),
            source_filter: Some(source_filter),
            controller_filter: None,
        }
    }

    /// Create a filter for any cause from a specific source type.
    pub fn from_source(source_filter: ObjectFilter) -> Self {
        Self {
            cause_type: None,
            source_filter: Some(source_filter),
            controller_filter: None,
        }
    }

    /// Add a source filter to this cause filter.
    pub fn with_source(mut self, source_filter: ObjectFilter) -> Self {
        self.source_filter = Some(source_filter);
        self
    }

    /// Add a controller filter to this cause filter.
    pub fn with_controller(mut self, controller_filter: ControllerFilter) -> Self {
        self.controller_filter = Some(controller_filter);
        self
    }

    /// Check if this filter matches the given cause.
    ///
    /// # Arguments
    /// * `cause` - The event cause to check
    /// * `game` - Game state for evaluating source filters
    /// * `affected_player` - The player affected by the event (for "You"/"Opponent" filters)
    pub fn matches(&self, cause: &EventCause, game: &GameState, affected_player: PlayerId) -> bool {
        // Check cause type filter
        if let Some(ref type_filter) = self.cause_type
            && !type_filter.matches(cause.cause_type)
        {
            return false;
        }

        // Check source filter
        if let Some(ref source_filter) = self.source_filter {
            let Some(source_id) = cause.source else {
                return false; // No source, but filter requires one
            };
            let Some(source_obj) = game.object(source_id) else {
                return false; // Source doesn't exist
            };
            // Create a filter context for the affected player
            let filter_ctx = FilterContext::new(affected_player);
            if !source_filter.matches(source_obj, &filter_ctx, game) {
                return false;
            }
        }

        // Check controller filter
        if let Some(ref controller_filter) = self.controller_filter {
            let matches_controller = match controller_filter {
                ControllerFilter::Player(player) => cause.source_controller == Some(*player),
                ControllerFilter::You => cause.source_controller == Some(affected_player),
                ControllerFilter::Opponent => cause
                    .source_controller
                    .is_some_and(|c| c != affected_player),
                ControllerFilter::Any => true,
            };
            if !matches_controller {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cause_type_is_effect_like() {
        assert!(CauseType::Effect.is_effect_like());
        assert!(!CauseType::GameRule.is_effect_like());
        assert!(!CauseType::StateBasedAction.is_effect_like());
        assert!(!CauseType::CombatDamage.is_effect_like());
        assert!(!CauseType::Cost.is_effect_like());
        assert!(!CauseType::SpecialAction.is_effect_like());
        assert!(!CauseType::LegendRule.is_effect_like());
        // assert!(CauseType::Replacement.is_effect_like());
        // assert!(!CauseType::ManaAbility.is_effect_like());
    }

    #[test]
    fn test_cause_filter_any() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let filter = CauseFilter::any();

        // Should match any cause
        let cause_effect = EventCause::from_effect(ObjectId::from_raw(1), alice);
        assert!(filter.matches(&cause_effect, &game, alice));

        let cause_cost = EventCause::from_cost(ObjectId::from_raw(1), alice);
        assert!(filter.matches(&cause_cost, &game, alice));
    }

    #[test]
    fn test_cause_filter_effect_like() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let filter = CauseFilter::effect_like();

        // Should match effect-like causes
        let cause_effect = EventCause::from_effect(ObjectId::from_raw(1), alice);
        assert!(filter.matches(&cause_effect, &game, alice));

        let cause_rule = EventCause::from_game_rule();
        assert!(!filter.matches(&cause_rule, &game, alice));

        // Should NOT match cost
        let cause_cost = EventCause::from_cost(ObjectId::from_raw(1), alice);
        assert!(!filter.matches(&cause_cost, &game, alice));
    }

    #[test]
    fn test_cause_type_filter_exact() {
        let filter = CauseTypeFilter::Exact(CauseType::Effect);

        assert!(filter.matches(CauseType::Effect));
        assert!(!filter.matches(CauseType::Cost));
        assert!(!filter.matches(CauseType::GameRule));
    }

    #[test]
    fn test_cause_type_filter_one_of() {
        let filter = CauseTypeFilter::OneOf(vec![CauseType::Effect, CauseType::GameRule]);

        assert!(filter.matches(CauseType::Effect));
        assert!(filter.matches(CauseType::GameRule));
        assert!(!filter.matches(CauseType::Cost));
    }

    #[test]
    fn test_cause_filter_not_type() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let filter = CauseFilter::not_type(CauseType::SpecialAction);

        let cause_effect = EventCause::from_effect(ObjectId::from_raw(1), alice);
        assert!(filter.matches(&cause_effect, &game, alice));

        let cause_special = EventCause::from_special_action(Some(ObjectId::from_raw(1)), alice);
        assert!(!filter.matches(&cause_special, &game, alice));
    }

    #[test]
    fn test_event_cause_constructors() {
        let source = ObjectId::from_raw(1);
        let player = PlayerId::from_index(0);

        let cost = EventCause::from_cost(source, player);
        assert_eq!(cost.cause_type, CauseType::Cost);
        assert_eq!(cost.source, Some(source));

        let effect = EventCause::from_effect(source, player);
        assert_eq!(effect.cause_type, CauseType::Effect);

        let sba = EventCause::from_sba();
        assert_eq!(sba.cause_type, CauseType::StateBasedAction);
        assert_eq!(sba.source, None);

        let rule = EventCause::from_game_rule();
        assert_eq!(rule.cause_type, CauseType::GameRule);

        let special = EventCause::from_special_action(Some(source), player);
        assert_eq!(special.cause_type, CauseType::SpecialAction);
        assert_eq!(special.source, Some(source));
        assert_eq!(special.source_controller, Some(player));

        let legend_rule = EventCause::from_legend_rule(player);
        assert_eq!(legend_rule.cause_type, CauseType::LegendRule);
        assert_eq!(legend_rule.source, None);
        assert_eq!(legend_rule.source_controller, Some(player));
    }
}
