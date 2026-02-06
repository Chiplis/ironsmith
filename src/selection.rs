//! Selection types for effects.
//!
//! This module provides types for specifying how effects select objects and players.
//! The key distinction is between targeted selection (requires targeting during casting,
//! can fizzle if targets become invalid) and non-targeted selection (filters at resolution,
//! cannot fizzle).
//!
//! # Examples
//!
//! ```ignore
//! // Targeted effect: "Destroy target creature"
//! let selection = ObjectSelection::Targeted {
//!     spec: ChooseSpec::creature(),
//!     count: ChoiceCount::exactly(1),
//! };
//!
//! // Non-targeted effect: "Destroy all creatures"
//! let selection = ObjectSelection::All(ObjectFilter::creature());
//! ```

use crate::effect::ChoiceCount;
use crate::target::{ObjectFilter, PlayerFilter, ChooseSpec};


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_selection_target() {
        let selection = ObjectSelection::target(ChooseSpec::creature());
        assert!(selection.is_targeted());
        assert!(selection.get_target_spec().is_some());
        assert_eq!(selection.get_target_count(), Some(ChoiceCount::exactly(1)));
    }

    #[test]
    fn test_object_selection_targets() {
        let selection =
            ObjectSelection::targets(ChooseSpec::creature(), ChoiceCount::up_to(2));
        assert!(selection.is_targeted());
        assert!(selection.get_target_spec().is_some());
        assert_eq!(selection.get_target_count(), Some(ChoiceCount::up_to(2)));
    }

    #[test]
    fn test_object_selection_all() {
        let selection = ObjectSelection::all(ObjectFilter::creature());
        assert!(!selection.is_targeted());
        assert!(selection.get_target_spec().is_none());
        assert!(selection.get_target_count().is_none());
    }

    #[test]
    fn test_object_selection_source() {
        let selection = ObjectSelection::source();
        assert!(!selection.is_targeted());
    }

    #[test]
    fn test_object_selection_iterated() {
        let selection = ObjectSelection::iterated();
        assert!(!selection.is_targeted());
    }

    #[test]
    fn test_player_selection_target() {
        let selection = PlayerSelection::target(PlayerFilter::Opponent);
        assert!(selection.is_targeted());
        assert!(selection.get_target_filter().is_some());
    }

    #[test]
    fn test_player_selection_you() {
        let selection = PlayerSelection::you();
        assert!(!selection.is_targeted());
    }

    #[test]
    fn test_player_selection_iterated() {
        let selection = PlayerSelection::iterated();
        assert!(!selection.is_targeted());
    }
}
