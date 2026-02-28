//! Search library effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// Effect that searches a player's library for a card.
///
/// The player can choose which matching card to find, or "fail to find" for hidden searches.
/// The library is always shuffled after searching.
///
/// # Fields
///
/// * `filter` - Filter for which cards can be found
/// * `destination` - Where to put the found card
/// * `player` - Whose library to search
/// * `reveal` - Whether the found card must be revealed
///
/// # Example
///
/// ```ignore
/// // Search for a basic land and put it onto the battlefield
/// let effect = SearchLibraryEffect::new(
///     ObjectFilter::basic_land(),
///     Zone::Battlefield,
///     PlayerFilter::You,
///     true,
/// );
///
/// // Tutor effect (search for any card)
/// let effect = SearchLibraryEffect::to_hand(ObjectFilter::default(), PlayerFilter::You, true);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SearchLibraryEffect {
    /// Filter for which cards can be found.
    pub filter: ObjectFilter,
    /// Where to put the found card.
    pub destination: Zone,
    /// Whose library to search.
    pub player: PlayerFilter,
    /// Whether the found card must be revealed.
    pub reveal: bool,
}

impl SearchLibraryEffect {
    /// Create a new search library effect.
    pub fn new(
        filter: ObjectFilter,
        destination: Zone,
        player: PlayerFilter,
        reveal: bool,
    ) -> Self {
        Self {
            filter,
            destination,
            player,
            reveal,
        }
    }

    /// Search for a card and put it into your hand.
    pub fn to_hand(filter: ObjectFilter, player: PlayerFilter, reveal: bool) -> Self {
        Self::new(filter, Zone::Hand, player, reveal)
    }

    /// Search for a card and put it onto the battlefield.
    pub fn to_battlefield(filter: ObjectFilter, player: PlayerFilter, reveal: bool) -> Self {
        Self::new(filter, Zone::Battlefield, player, reveal)
    }

    /// Search for a card and put it on top of your library.
    pub fn to_library_top(filter: ObjectFilter, player: PlayerFilter, reveal: bool) -> Self {
        Self::new(filter, Zone::Library, player, reveal)
    }
}

impl EffectExecutor for SearchLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Check if player can search libraries (Leonin Arbiter, Aven Mindcensor effects)
        if !game.can_search_library(player_id) {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        // Track that this player searched their library (for trap conditions like Archive Trap)
        game.library_searches_this_turn.insert(player_id);

        let filter_ctx = ctx.filter_context(game);

        // Get all cards in the player's library that match the filter
        let matching_cards: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| {
                p.library
                    .iter()
                    .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                    .filter(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect()
            })
            .unwrap_or_default();

        // Let the player choose a card (or fail to find) using the spec-based system
        let spec = SearchSpec::new(ctx.source, matching_cards.clone(), self.reveal);
        let chosen_card = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            spec,
            FallbackStrategy::FirstOption, // Auto-select first card when no decision maker
        );

        // If a card was chosen, move it to the destination
        if let Some(card_id) = chosen_card {
            // Verify the card is still in the library (in case decision maker did something weird)
            let still_in_library = game
                .player(player_id)
                .is_some_and(|p| p.library.contains(&card_id));

            if still_in_library {
                // For "put on top of library" effects (like Vampiric Tutor), we need to:
                // 1. Remove the card from the library
                // 2. Shuffle the library
                // 3. Put the card on top
                // This matches the card text "then shuffle and put that card on top"
                if self.destination == Zone::Library {
                    // Remove the card from library first
                    if let Some(p) = game.player_mut(player_id) {
                        p.library.retain(|&id| id != card_id);
                    }
                    // Shuffle the remaining library
                    if let Some(p) = game.player_mut(player_id) {
                        p.shuffle_library();
                    }
                    // Now put the card on top (push adds to end, which is the top)
                    if let Some(p) = game.player_mut(player_id) {
                        p.library.push(card_id);
                    }
                    return Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                        card_id,
                    ])));
                }

                // For other destinations, move then shuffle
                let new_id = if self.destination == Zone::Battlefield {
                    match move_to_battlefield_with_options(
                        game,
                        ctx,
                        card_id,
                        BattlefieldEntryOptions::preserve(false),
                    ) {
                        BattlefieldEntryOutcome::Moved(new_id) => Some(new_id),
                        BattlefieldEntryOutcome::Prevented => None,
                    }
                } else {
                    game.move_object(card_id, self.destination)
                };

                if let Some(new_id) = new_id {
                    // Shuffle the library after searching
                    if let Some(p) = game.player_mut(player_id) {
                        p.shuffle_library();
                    }
                    return Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                        new_id,
                    ])));
                }
            }
        }

        // No card found or chosen - still shuffle (searching always shuffles)
        if let Some(p) = game.player_mut(player_id) {
            p.shuffle_library();
        }

        Ok(EffectOutcome::count(0))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
