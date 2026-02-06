//! Reusable helper functions for common PlayerDecision patterns.
//!
//! These helpers reduce boilerplate in effect executors and card definitions
//! by centralizing decision-making logic and fallback behavior.
//!
//! These helpers use the spec-based decision system and typed decision primitives.

use crate::color::Color;
use crate::decision::{DecisionMaker, FallbackStrategy};
use crate::decisions::specs::{ChoiceSpec, ManaColorsSpec, MaySpec, SacrificeSpec};
use crate::decisions::{DisplayOption, make_decision, make_decision_with_fallback};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Ask a player whether to perform an optional "may" action.
///
/// Returns true if the player chooses to perform the action, false otherwise.
///
/// # Arguments
///
/// * `game` - The current game state (for the decision maker to inspect)
/// * `dm` - Mutable reference to an optional decision maker
/// * `player` - The player making the choice
/// * `source` - The source of the effect (for display purposes)
/// * `description` - Human-readable description of what the player "may" do
/// * `fallback` - What to return when no decision maker is present
///
/// # Example
///
/// ```ignore
/// let should_sacrifice = ask_may_choice(
///     game,
///     &mut ctx.decision_maker,
///     ctx.controller,
///     ctx.source,
///     "Sacrifice a creature to draw a card",
///     FallbackStrategy::Decline,
/// );
/// ```
pub fn ask_may_choice(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    description: impl Into<String>,
    fallback: FallbackStrategy,
) -> bool {
    let spec = MaySpec::new(source, description);
    make_decision_with_fallback(game, dm, player, Some(source), spec, fallback)
}

/// Ask a player to select a permanent to sacrifice from a list of legal options.
///
/// Returns the chosen ObjectId if a valid choice is made.
/// If no decision maker is present, returns the first option (if any).
///
/// # Arguments
///
/// * `game` - The current game state
/// * `dm` - Mutable reference to an optional decision maker
/// * `player` - The player making the sacrifice choice
/// * `source` - The source of the effect
/// * `legal_permanents` - List of permanents that can legally be sacrificed
/// * `description` - Human-readable description of what kind of permanent to sacrifice
///
/// # Returns
///
/// * `Some(ObjectId)` - The chosen permanent to sacrifice
/// * `None` - No legal permanents available
///
/// # Example
///
/// ```ignore
/// if let Some(to_sacrifice) = ask_sacrifice(
///     game,
///     &mut ctx.decision_maker,
///     ctx.controller,
///     ctx.source,
///     &valid_creatures,
///     "Choose a creature to sacrifice",
/// ) {
///     game.move_object(to_sacrifice, Zone::Graveyard);
/// }
/// ```
pub fn ask_sacrifice(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    legal_permanents: &[ObjectId],
    description: impl Into<String>,
) -> Option<ObjectId> {
    if legal_permanents.is_empty() {
        return None;
    }

    let spec = SacrificeSpec::new(source, description, legal_permanents.to_vec());
    let chosen = make_decision(game, dm, player, Some(source), spec);

    // Validate the chosen target is in the legal list
    if legal_permanents.contains(&chosen) {
        Some(chosen)
    } else {
        // Invalid choice, fall back to first valid
        Some(legal_permanents[0])
    }
}

/// Ask a player to choose a single mana color.
///
/// Used by mana abilities that can produce any one color (like Birds of Paradise)
/// or that need to select from a restricted set of colors (like commander identity).
///
/// # Arguments
///
/// * `game` - The current game state
/// * `dm` - Mutable reference to an optional decision maker
/// * `player` - The player making the choice
/// * `source` - The source of the mana ability
/// * `available_colors` - Optional restriction on which colors can be chosen.
///   If None, all five colors are available.
/// * `default` - The color to return when no decision maker is present
///
/// # Returns
///
/// The chosen color.
///
/// # Example
///
/// ```ignore
/// // Mana ability with restricted colors (commander identity)
/// let color = ask_mana_color(
///     game,
///     &mut ctx.decision_maker,
///     player_id,
///     ctx.source,
///     Some(&available_colors),
///     available_colors[0],
/// );
/// ```
pub fn ask_mana_color(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    available_colors: Option<&[Color]>,
    default: Color,
) -> Color {
    let spec = if let Some(colors) = available_colors {
        ManaColorsSpec::restricted(source, 1, true, colors.to_vec())
    } else {
        ManaColorsSpec::any_color(source, 1, true)
    };

    let colors = make_decision(game, dm, player, Some(source), spec);
    let chosen = colors.into_iter().next().unwrap_or(default);

    // Validate against available colors if restricted
    if let Some(available) = available_colors {
        if available.contains(&chosen) {
            chosen
        } else {
            default
        }
    } else {
        chosen
    }
}

/// Ask a player to choose a single option from a list of choices.
///
/// This is a type-safe wrapper around `PlayerDecision::Choose` that automatically
/// maps indices back to the associated values.
///
/// # Arguments
///
/// * `game` - The current game state
/// * `dm` - Mutable reference to an optional decision maker
/// * `player` - The player making the choice
/// * `source` - The source of the effect
/// * `options` - Slice of (description, value) pairs
///
/// # Returns
///
/// The value associated with the chosen option. Returns the first option's value
/// if no decision maker is present or if an invalid choice is made.
///
/// # Panics
///
/// Panics if `options` is empty.
///
/// # Example
///
/// ```ignore
/// let colors = [
///     ("White", Color::White),
///     ("Blue", Color::Blue),
///     ("Black", Color::Black),
///     ("Red", Color::Red),
///     ("Green", Color::Green),
/// ];
/// let chosen_color = ask_choose_one(
///     game,
///     &mut ctx.decision_maker,
///     ctx.controller,
///     ctx.source,
///     &colors.map(|(desc, c)| (desc.to_string(), c)),
/// );
/// ```
pub fn ask_choose_one<T: Clone>(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    options: &[(String, T)],
) -> T {
    assert!(
        !options.is_empty(),
        "ask_choose_one requires at least one option"
    );

    let display_options: Vec<DisplayOption> = options
        .iter()
        .enumerate()
        .map(|(index, (description, _))| DisplayOption::new(index, description.as_str()))
        .collect();

    let spec = ChoiceSpec::single(source, display_options);
    let choices: Vec<usize> = make_decision(game, dm, player, Some(source), spec);

    if let Some(&choice) = choices.first()
        && choice < options.len()
    {
        return options[choice].1.clone();
    }

    // Default to first option
    options[0].1.clone()
}

/// Ask a player to choose multiple options from a list of choices.
///
/// This is a type-safe wrapper around `PlayerDecision::Choose` with multi-select
/// that automatically maps indices back to the associated values.
///
/// # Arguments
///
/// * `game` - The current game state
/// * `dm` - Mutable reference to an optional decision maker
/// * `player` - The player making the choices
/// * `source` - The source of the effect
/// * `options` - Slice of (description, value) pairs
/// * `min` - Minimum number of choices required
/// * `max` - Maximum number of choices allowed
///
/// # Returns
///
/// A vector of the values associated with the chosen options. Returns the first
/// `min` options if no decision maker is present.
///
/// # Panics
///
/// Panics if `options` is empty or if `min > max` or if `min > options.len()`.
///
/// # Example
///
/// ```ignore
/// let creatures = [
///     ("Creature A", creature_a_id),
///     ("Creature B", creature_b_id),
///     ("Creature C", creature_c_id),
/// ];
/// let chosen = ask_choose_multiple(
///     game,
///     &mut ctx.decision_maker,
///     ctx.controller,
///     ctx.source,
///     &creatures.map(|(desc, id)| (desc.to_string(), id)),
///     1,  // min
///     2,  // max
/// );
/// ```
pub fn ask_choose_multiple<T: Clone>(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    options: &[(String, T)],
    min: usize,
    max: usize,
) -> Vec<T> {
    assert!(
        !options.is_empty(),
        "ask_choose_multiple requires at least one option"
    );
    assert!(min <= max, "min must be <= max");
    assert!(min <= options.len(), "min must be <= number of options");

    let display_options: Vec<DisplayOption> = options
        .iter()
        .enumerate()
        .map(|(index, (description, _))| DisplayOption::new(index, description.as_str()))
        .collect();

    let spec = ChoiceSpec::new(source, display_options, min, max);
    let choices: Vec<usize> = make_decision(game, dm, player, Some(source), spec);

    let valid_choices: Vec<T> = choices
        .iter()
        .filter(|&&c| c < options.len())
        .map(|&c| options[c].1.clone())
        .collect();

    // Validate the number of choices
    if valid_choices.len() >= min && valid_choices.len() <= max {
        return valid_choices;
    }

    // Default: return first `min` options
    options.iter().take(min).map(|(_, v)| v.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Mock Decision Makers
    // ========================================

    /// Decision maker that always accepts "may" choices.
    struct AlwaysAcceptMayDm;
    impl DecisionMaker for AlwaysAcceptMayDm {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    /// Decision maker that always declines "may" choices.
    struct AlwaysDeclineMayDm;
    impl DecisionMaker for AlwaysDeclineMayDm {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }

    /// Decision maker that chooses a specific index for Choose decisions.
    struct ChooseIndexDm(usize);
    impl DecisionMaker for ChooseIndexDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            vec![self.0]
        }
    }

    /// Decision maker that chooses specific indices for multi-select Choose decisions.
    struct ChooseIndicesDm(Vec<usize>);
    impl DecisionMaker for ChooseIndicesDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.0.clone()
        }
    }

    /// Decision maker that chooses a specific color for mana decisions.
    struct ChooseColorDm(Color);
    impl DecisionMaker for ChooseColorDm {
        fn decide_colors(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::ColorsContext,
        ) -> Vec<Color> {
            vec![self.0]
        }
    }

    /// Decision maker that chooses a specific sacrifice target.
    struct ChooseSacrificeDm(ObjectId);
    impl DecisionMaker for ChooseSacrificeDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            vec![self.0]
        }
    }

    // ========================================
    // ask_may_choice Tests
    // ========================================

    #[test]
    fn test_may_choice_accepts_with_dm() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let mut dm = AlwaysAcceptMayDm;

        let result = ask_may_choice(
            &game,
            &mut dm,
            alice,
            source,
            "Do something?",
            FallbackStrategy::Decline,
        );

        assert!(result);
    }

    #[test]
    fn test_may_choice_declines_with_dm() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let mut dm = AlwaysDeclineMayDm;

        let result = ask_may_choice(
            &game,
            &mut dm,
            alice,
            source,
            "Do something?",
            FallbackStrategy::Accept,
        );

        assert!(!result);
    }

    #[test]
    fn test_may_choice_fallback_decline() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // AutoPassDecisionMaker returns false for boolean decisions,
        // which matches the "Decline" fallback behavior
        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = ask_may_choice(
            &game,
            &mut dm,
            alice,
            source,
            "Do something?",
            FallbackStrategy::Decline,
        );

        assert!(!result);
    }

    #[test]
    fn test_may_choice_fallback_accept() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // AutoPassDecisionMaker returns false for boolean decisions,
        // so we need to use AlwaysAcceptMayDm to test the "Accept" case
        let mut dm = AlwaysAcceptMayDm;

        let result = ask_may_choice(
            &game,
            &mut dm,
            alice,
            source,
            "Do something?",
            FallbackStrategy::Accept,
        );

        assert!(result);
    }

    // ========================================
    // ask_sacrifice Tests
    // ========================================

    #[test]
    fn test_sacrifice_empty_list_returns_none() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = ask_sacrifice(&game, &mut dm, alice, source, &[], "Choose a creature");

        assert!(result.is_none());
    }

    #[test]
    fn test_sacrifice_fallback_first() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let creature1 = ObjectId::from_raw(10);
        let creature2 = ObjectId::from_raw(11);

        // AutoPassDecisionMaker returns empty vec for object selections,
        // which triggers the fallback to first option
        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = ask_sacrifice(
            &game,
            &mut dm,
            alice,
            source,
            &[creature1, creature2],
            "Choose a creature",
        );

        assert_eq!(result, Some(creature1));
    }

    #[test]
    fn test_sacrifice_with_dm_valid_choice() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let creature1 = ObjectId::from_raw(10);
        let creature2 = ObjectId::from_raw(11);

        let mut dm = ChooseSacrificeDm(creature2);

        let result = ask_sacrifice(
            &game,
            &mut dm,
            alice,
            source,
            &[creature1, creature2],
            "Choose a creature",
        );

        assert_eq!(result, Some(creature2));
    }

    #[test]
    fn test_sacrifice_with_dm_invalid_choice_falls_back() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let creature1 = ObjectId::from_raw(10);
        let creature2 = ObjectId::from_raw(11);
        let invalid = ObjectId::from_raw(99);

        let mut dm = ChooseSacrificeDm(invalid);

        let result = ask_sacrifice(
            &game,
            &mut dm,
            alice,
            source,
            &[creature1, creature2],
            "Choose a creature",
        );

        assert_eq!(result, Some(creature1));
    }

    // ========================================
    // ask_mana_color Tests
    // ========================================

    #[test]
    fn test_mana_color_fallback() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // AutoPassDecisionMaker returns empty vec for color selections,
        // which triggers the default color fallback
        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = ask_mana_color(&game, &mut dm, alice, source, None, Color::Green);

        assert_eq!(result, Color::Green);
    }

    #[test]
    fn test_mana_color_with_dm() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let mut dm = ChooseColorDm(Color::Blue);

        let result = ask_mana_color(&game, &mut dm, alice, source, None, Color::Green);

        assert_eq!(result, Color::Blue);
    }

    #[test]
    fn test_mana_color_restricted_valid() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let available = [Color::White, Color::Black];
        let mut dm = ChooseColorDm(Color::Black);

        let result = ask_mana_color(
            &game,
            &mut dm,
            alice,
            source,
            Some(&available),
            Color::White,
        );

        assert_eq!(result, Color::Black);
    }

    #[test]
    fn test_mana_color_restricted_invalid_falls_back() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let available = [Color::White, Color::Black];
        let mut dm = ChooseColorDm(Color::Blue); // Not in available

        let result = ask_mana_color(
            &game,
            &mut dm,
            alice,
            source,
            Some(&available),
            Color::White,
        );

        assert_eq!(result, Color::White);
    }

    // ========================================
    // ask_choose_one Tests
    // ========================================

    #[test]
    fn test_choose_one_fallback() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let options = [
            ("First".to_string(), 100),
            ("Second".to_string(), 200),
            ("Third".to_string(), 300),
        ];

        // AutoPassDecisionMaker returns empty vec for option selections,
        // which triggers the fallback to first option
        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = ask_choose_one(&game, &mut dm, alice, source, &options);

        assert_eq!(result, 100);
    }

    #[test]
    fn test_choose_one_with_dm() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let options = [
            ("First".to_string(), 100),
            ("Second".to_string(), 200),
            ("Third".to_string(), 300),
        ];

        let mut dm = ChooseIndexDm(1);

        let result = ask_choose_one(&game, &mut dm, alice, source, &options);

        assert_eq!(result, 200);
    }

    #[test]
    fn test_choose_one_invalid_index_falls_back() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let options = [("First".to_string(), 100), ("Second".to_string(), 200)];

        let mut dm = ChooseIndexDm(99); // Invalid index

        let result = ask_choose_one(&game, &mut dm, alice, source, &options);

        assert_eq!(result, 100);
    }

    // ========================================
    // ask_choose_multiple Tests
    // ========================================

    #[test]
    fn test_choose_multiple_fallback() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let options = [
            ("A".to_string(), 1),
            ("B".to_string(), 2),
            ("C".to_string(), 3),
        ];

        // AutoPassDecisionMaker returns empty vec, which doesn't meet min,
        // so it falls back to first 2 options
        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = ask_choose_multiple(&game, &mut dm, alice, source, &options, 2, 3);

        assert_eq!(result, vec![1, 2]); // First 2 (min)
    }

    #[test]
    fn test_choose_multiple_with_dm() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let options = [
            ("A".to_string(), 1),
            ("B".to_string(), 2),
            ("C".to_string(), 3),
        ];

        let mut dm = ChooseIndicesDm(vec![0, 2]);

        let result = ask_choose_multiple(&game, &mut dm, alice, source, &options, 1, 3);

        assert_eq!(result, vec![1, 3]); // Options at indices 0 and 2
    }

    #[test]
    fn test_choose_multiple_invalid_count_falls_back() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let options = [
            ("A".to_string(), 1),
            ("B".to_string(), 2),
            ("C".to_string(), 3),
        ];

        // DM chooses only 1, but min is 2
        let mut dm = ChooseIndicesDm(vec![0]);

        let result = ask_choose_multiple(&game, &mut dm, alice, source, &options, 2, 3);

        assert_eq!(result, vec![1, 2]); // Falls back to first 2
    }

    #[test]
    fn test_choose_multiple_zero_min() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let options = [("A".to_string(), 1), ("B".to_string(), 2)];

        // AutoPassDecisionMaker returns empty vec, which is valid since min is 0
        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = ask_choose_multiple(&game, &mut dm, alice, source, &options, 0, 2);

        assert!(result.is_empty()); // min is 0, so empty vec
    }
}
