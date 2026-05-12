//! Search library effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, OutcomeObjectMemory, SearchSelectionMode};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, view_hidden_candidate_objects};
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{SearchLibraryEvent, ShuffleLibraryEvent};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::search_overrides::{
    begin_opposition_agent_search_control, exile_found_cards_for_opposition_agent,
    finish_opposition_agent_search_control, offer_library_search_casts, opposition_agent_search,
};

pub type SearchLibraryEffect = ironsmith_core::SearchLibraryEffect;

impl EffectExecutor for SearchLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser_id = resolve_player_filter(game, &self.chooser, ctx)?;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let search_override = opposition_agent_search(game, chooser_id, player_id);

        // Check if the searching player can search libraries.
        if !game.can_search_library(chooser_id) {
            return Ok(EffectOutcome::prevented());
        }

        let search_control =
            begin_opposition_agent_search_control(game, chooser_id, search_override);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            let search_viewer = game.controlling_player_for(chooser_id);
            let library_cards = game
                .player(player_id)
                .map(|player| player.library.clone())
                .unwrap_or_default();
            view_hidden_candidate_objects(
                game,
                ctx,
                search_viewer,
                &library_cards,
                "Search library",
                false,
            );

            offer_library_search_casts(game, ctx, player_id)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            let search_event = TriggerEvent::new_with_provenance(
                SearchLibraryEvent::new(chooser_id, Some(player_id)),
                ctx.provenance,
            );
            let shuffle_event = TriggerEvent::new_with_provenance(
                ShuffleLibraryEvent::new(player_id, ctx.cause.clone()),
                ctx.provenance,
            );

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
            let unknown_hidden_cards: Vec<ObjectId> = if matching_cards.is_empty() {
                library_cards
                    .iter()
                    .copied()
                    .filter(|id| game.is_hidden_card_placeholder(*id))
                    .collect()
            } else {
                Vec::new()
            };
            let decision_candidates = if matching_cards.is_empty() {
                unknown_hidden_cards
            } else {
                matching_cards.clone()
            };

            // Let the player choose a card (or fail to find) using the spec-based system
            let may_fail_to_find = match self.search_mode {
                SearchSelectionMode::Exact => self.filter.has_search_stated_quality(),
                SearchSelectionMode::Optional | SearchSelectionMode::AllMatching => true,
            };
            let spec = if may_fail_to_find {
                SearchSpec::new(ctx.source, decision_candidates, self.reveal)
            } else {
                SearchSpec::mandatory(ctx.source, decision_candidates, self.reveal)
            };
            let mut chosen_card = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                chooser_id,
                Some(ctx.source),
                spec,
                FallbackStrategy::FirstOption, // Auto-select first card when no decision maker
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0).with_event(search_event));
            }
            if chosen_card.is_none() && !may_fail_to_find {
                chosen_card = matching_cards.first().copied();
            }

            // If a card was chosen, move it to the destination
            if let Some(card_id) = chosen_card {
                let chosen_matches = game
                    .object(card_id)
                    .is_some_and(|obj| self.filter.matches(obj, &filter_ctx, game));
                if !chosen_matches && !game.is_hidden_card_placeholder(card_id) {
                    return Err(ExecutionError::InvalidTarget);
                }
                // Verify the card is still in the library (in case decision maker did something weird)
                let still_in_library = game
                    .player(player_id)
                    .is_some_and(|p| p.library.contains(&card_id));

                if still_in_library {
                    if self.reveal {
                        view_hidden_candidate_objects(
                            game,
                            ctx,
                            search_viewer,
                            &[card_id],
                            "Reveal searched card",
                            true,
                        );
                    }
                    let chosen_memory = OutcomeObjectMemory::from_object_id(game, card_id);
                    // For "put on top of library" effects (like Vampiric Tutor), we need to:
                    // 1. Remove the card from the library
                    // 2. Shuffle the library
                    // 3. Put the card on top
                    // This matches the card text "then shuffle and put that card on top"
                    if self.destination == Zone::Library && search_override.is_none() {
                        // Remove the card from library first
                        if let Some(p) = game.player_mut(player_id) {
                            p.library.retain(|&id| id != card_id);
                        }
                        // Shuffle the remaining library
                        game.shuffle_player_library(player_id);
                        // Now put the card on top (push adds to end, which is the top)
                        if let Some(p) = game.player_mut(player_id) {
                            p.library.push(card_id);
                        }
                        let mut outcome = EffectOutcome::with_objects(vec![card_id])
                            .with_events([search_event.clone(), shuffle_event.clone()]);
                        if let Some(memory) = chosen_memory {
                            outcome = outcome
                                .with_chosen_object_memory(vec![memory.clone()])
                                .with_affected_object_memory(vec![memory]);
                        }
                        return Ok(outcome);
                    }

                    // For other destinations, move then shuffle
                    let new_id = if let Some(search_override) = search_override {
                        exile_found_cards_for_opposition_agent(game, &[card_id], search_override)
                            .first()
                            .copied()
                    } else if self.destination == Zone::Battlefield {
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
                        game.move_object_by_effect(card_id, self.destination)
                    };

                    if let Some(new_id) = new_id {
                        // Shuffle the library after searching
                        game.shuffle_player_library(player_id);
                        let mut outcome = EffectOutcome::with_objects(vec![new_id])
                            .with_affected_objects(vec![new_id])
                            .with_events([search_event.clone(), shuffle_event.clone()]);
                        if let Some(memory) = chosen_memory {
                            outcome = outcome
                                .with_chosen_object_memory(vec![memory.clone()])
                                .with_affected_object_memory(vec![memory]);
                        }
                        return Ok(outcome);
                    }
                }
            }

            // No card found or chosen - still shuffle (searching always shuffles)
            game.shuffle_player_library(player_id);

            Ok(EffectOutcome::count(0).with_events([search_event, shuffle_event]))
        })();

        if result.is_err() || !ctx.decision_maker.awaiting_choice() {
            finish_opposition_agent_search_control(game, search_control);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::target::PlayerFilter;
    use crate::types::CardType;

    #[derive(Debug)]
    struct ViewCall {
        viewer: PlayerId,
        subject: PlayerId,
        zone: Zone,
        public: bool,
        cards: Vec<ObjectId>,
    }

    #[derive(Debug, Default)]
    struct CaptureSearchDm {
        calls: Vec<ViewCall>,
    }

    impl DecisionMaker for CaptureSearchDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls.push(ViewCall {
                viewer,
                subject: ctx.subject,
                zone: ctx.zone,
                public: ctx.public,
                cards: cards.to_vec(),
            });
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    #[derive(Debug, Default)]
    struct PendingSearchDm {
        calls: Vec<ViewCall>,
        pending: bool,
        candidates: Vec<ObjectId>,
    }

    impl DecisionMaker for PendingSearchDm {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }

        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls.push(ViewCall {
                viewer,
                subject: ctx.subject,
                zone: ctx.zone,
                public: ctx.public,
                cards: cards.to_vec(),
            });
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.pending = true;
            self.candidates = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect();
            Vec::new()
        }
    }

    fn add_library_creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    #[test]
    fn search_library_emits_private_view_for_searchable_library() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let first = add_library_creature(&mut game, alice, "First Hidden Creature");
        let second = add_library_creature(&mut game, alice, "Second Hidden Creature");
        let source = game.new_object_id();
        let mut dm = CaptureSearchDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = SearchLibraryEffect::to_hand(
            ObjectFilter::default().with_type(CardType::Creature),
            PlayerFilter::You,
            false,
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("search should resolve");

        assert_eq!(dm.calls.len(), 1);
        let call = &dm.calls[0];
        assert_eq!(call.viewer, alice);
        assert_eq!(call.subject, alice);
        assert_eq!(call.zone, Zone::Library);
        assert!(!call.public);
        assert_eq!(call.cards, vec![first, second]);
    }

    #[test]
    fn revealed_search_emits_public_view_for_found_card_without_losing_search_view() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let found = add_library_creature(&mut game, alice, "Found Hidden Creature");
        let _other = add_library_creature(&mut game, alice, "Other Hidden Creature");
        let source = game.new_object_id();
        let mut dm = CaptureSearchDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = SearchLibraryEffect::to_hand(
            ObjectFilter::default().with_type(CardType::Creature),
            PlayerFilter::You,
            true,
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("search should resolve");

        assert!(
            dm.calls.iter().any(|call| {
                !call.public && call.viewer == alice && call.cards.contains(&found)
            })
        );
        assert!(
            dm.calls
                .iter()
                .any(|call| call.public && call.cards == vec![found])
        );
    }

    #[test]
    fn search_library_prompts_with_hidden_library_placeholders() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let first = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "first-hidden-commitment".to_string(),
        );
        let second = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            1,
            "second-hidden-commitment".to_string(),
        );
        let source = game.new_object_id();
        let mut dm = PendingSearchDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = SearchLibraryEffect::new(
            ObjectFilter::default().with_type(CardType::Instant),
            Zone::Library,
            PlayerFilter::You,
            PlayerFilter::You,
            true,
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("search should pause for hidden library choices");

        assert!(dm.pending, "hidden library search should surface a prompt");
        assert_eq!(dm.candidates, vec![first, second]);
        assert!(
            dm.calls.iter().any(|call| {
                !call.public
                    && call.viewer == alice
                    && call.subject == alice
                    && call.zone == Zone::Library
                    && call.cards == vec![first, second]
            }),
            "hidden placeholders should be opened privately for the searching player"
        );
    }
}
