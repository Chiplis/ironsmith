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
