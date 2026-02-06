//! Search library effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
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
                    // Use ETB processing for battlefield to handle replacement effects
                    game.move_object_with_etb_processing_with_dm(
                        card_id,
                        Zone::Battlefield,
                        &mut ctx.decision_maker,
                    )
                    .map(|r| r.new_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::{CardType, Supertype};

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_land_card(card_id: u32, name: &str, basic: bool) -> Card {
        let mut builder =
            CardBuilder::new(CardId::from_raw(card_id), name).card_types(vec![CardType::Land]);
        if basic {
            builder = builder.supertypes(vec![Supertype::Basic]);
        }
        builder.build()
    }

    fn make_creature_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build()
    }

    fn add_card_to_library(game: &mut GameState, card: Card, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj); // add_object automatically updates player.library for Zone::Library
        id
    }

    #[test]
    fn test_search_library_finds_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add some cards to library
        let land_card = make_land_card(100, "Forest", true);
        let _land_id = add_card_to_library(&mut game, land_card, alice);

        let creature_card = make_creature_card(101, "Bear");
        let _creature_id = add_card_to_library(&mut game, creature_card, alice);

        // Use SelectFirstDecisionMaker to select a card from the search
        let mut ctx = ExecutionContext::new_default(source, alice);
        // Search for a land - note: don't use ObjectFilter::land() as it has zone=Battlefield
        let effect = SearchLibraryEffect::to_hand(
            ObjectFilter::default().with_type(CardType::Land),
            PlayerFilter::You,
            true,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should find the land
        assert!(matches!(result.result, EffectResult::Objects(_)));
    }

    #[test]
    fn test_search_library_no_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add only creatures
        let creature_card = make_creature_card(100, "Bear");
        add_card_to_library(&mut game, creature_card, alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        // Search for a land - note: don't use ObjectFilter::land() as it has zone=Battlefield
        let effect = SearchLibraryEffect::to_hand(
            ObjectFilter::default().with_type(CardType::Land),
            PlayerFilter::You,
            true,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_search_library_prevented() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Simulate Leonin Arbiter effect
        game.cant_effects.cant_search.insert(alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SearchLibraryEffect::to_hand(ObjectFilter::default(), PlayerFilter::You, true);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
    }

    #[test]
    fn test_search_library_tracks_searches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        assert!(!game.library_searches_this_turn.contains(&alice));

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SearchLibraryEffect::to_hand(ObjectFilter::default(), PlayerFilter::You, true);
        let _ = effect.execute(&mut game, &mut ctx);

        // Should be tracked for Archive Trap
        assert!(game.library_searches_this_turn.contains(&alice));
    }

    #[test]
    fn test_search_library_clone_box() {
        let effect = SearchLibraryEffect::to_hand(
            ObjectFilter::default().with_type(CardType::Land),
            PlayerFilter::You,
            true,
        );
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("SearchLibraryEffect"));
    }

    #[test]
    fn test_search_library_to_top_puts_card_on_top() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add multiple cards to library to ensure shuffling doesn't mess with top card
        let target_card = make_creature_card(100, "Target Card");
        let target_id = add_card_to_library(&mut game, target_card, alice);

        // Add more cards so the target isn't already on top
        for i in 1..10 {
            let filler = make_creature_card(100 + i, &format!("Filler {}", i));
            add_card_to_library(&mut game, filler, alice);
        }

        // Verify target is NOT on top initially
        let top_before = game.player(alice).unwrap().library.last().copied();
        assert_ne!(
            top_before,
            Some(target_id),
            "Target shouldn't be on top initially"
        );

        // Use SelectFirstDecisionMaker to select a card from the search
        let mut ctx = ExecutionContext::new_default(source, alice);
        // Search for the specific card and put on top of library (Vampiric Tutor style)
        let effect = SearchLibraryEffect::to_library_top(
            ObjectFilter::default(), // Match any card
            PlayerFilter::You,
            false,
        );

        // Execute - SelectFirstDecisionMaker picks the first matching card
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have found a card
        assert!(matches!(result.result, EffectResult::Objects(_)));

        if let EffectResult::Objects(found) = result.result {
            assert_eq!(found.len(), 1);
            let found_id = found[0];

            // The found card should now be on top of the library
            let top_after = game.player(alice).unwrap().library.last().copied();
            assert_eq!(
                top_after,
                Some(found_id),
                "Found card should be on top of library after search"
            );
        }
    }

    #[test]
    fn test_search_library_to_top_preserves_card_after_shuffle() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add a specific named card that we want to find
        let target_card = make_creature_card(999, "Unique Target");
        let _target_id = add_card_to_library(&mut game, target_card, alice);

        // Add many filler cards
        for i in 1..20 {
            let filler = make_creature_card(i, &format!("Filler {}", i));
            add_card_to_library(&mut game, filler, alice);
        }

        let initial_library_size = game.player(alice).unwrap().library.len();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect =
            SearchLibraryEffect::to_library_top(ObjectFilter::default(), PlayerFilter::You, false);
        let _ = effect.execute(&mut game, &mut ctx).unwrap();

        // Library size should be unchanged (card stayed in library)
        let final_library_size = game.player(alice).unwrap().library.len();
        assert_eq!(
            final_library_size, initial_library_size,
            "Library size should be unchanged"
        );

        // The top card should be the one that was found
        // (we can't guarantee WHICH card was picked without a decision maker,
        // but whatever was picked should be on top)
        let top = game.player(alice).unwrap().library.last().copied();
        assert!(top.is_some(), "Library should have a top card");
    }
}
