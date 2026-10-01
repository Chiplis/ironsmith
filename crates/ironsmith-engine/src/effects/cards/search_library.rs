//! Search library effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, OutcomeObjectMemory, SearchSelectionMode};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_player_filter, resolve_value, view_hidden_candidate_objects,
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
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let instruction = (|| -> Result<EffectOutcome, ExecutionError> {
        let chooser_id =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let search_override = opposition_agent_search(game, chooser_id, player_id);

        // Check if the searching player can search libraries.
        if !game.can_search_library_from_effect(chooser_id, player_id, ctx.controller) {
            return Ok(EffectOutcome::prevented());
        }

        let search_control =
            begin_opposition_agent_search_control(game, chooser_id, search_override);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            let mut library_cards: Vec<ObjectId> = game
                .player(player_id)
                .map(|player| player.library.iter().copied().collect())
                .unwrap_or_default();
            game.restrict_library_search_candidates(chooser_id, &mut library_cards);
            let search_viewer = chooser_id;
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
            let mut matching_cards: Vec<ObjectId> = game
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
            game.restrict_library_search_candidates(chooser_id, &mut matching_cards);
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
                if may_fail_to_find {
                    FallbackStrategy::Decline
                } else {
                    FallbackStrategy::FirstOption
                },
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
                        let position_from_top =
                            if let Some(position) = self.library_position_from_top.as_ref() {
                                resolve_value(game, position, ctx)?.max(1) as usize
                            } else {
                                1
                            };
                        game.shuffle_library_except_then_insert_from_top(
                            player_id,
                            &[card_id],
                            position_from_top,
                            "searched card restored after library shuffle",
                        );
                        let mut outcome = EffectOutcome::with_objects(vec![card_id])
                            .with_events([search_event.clone(), shuffle_event.clone()]);
                        if let Some(memory) = chosen_memory {
                            outcome = outcome
                                .with_chosen_object_memory(vec![memory.clone()])
                                .with_affected_object_memory(vec![memory]);
                        }
                        return Ok(outcome);
                    }

                    // Complete the found-card instruction and all permission
                    // links before additions; shuffle only after it resolves.
                    let mut movement = if search_override.is_some() {
                        let found = exile_found_cards_for_opposition_agent(game, ctx, &[card_id], chooser_id)?;
                        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                        let mut original = if found.moved_ids.is_empty() { EffectOutcome::count(0) }
                            else { EffectOutcome::with_objects(found.moved_ids.clone())
                                .with_affected_objects(found.moved_ids) };
                        if let Some(memory) = chosen_memory.clone() {
                            original = original.with_chosen_object_memory(vec![memory]);
                        }
                        crate::effects::zones::finish_zone_change_receipts(game, ctx, original, found.receipts)?
                    } else {
                        let move_effect = crate::effect::Effect::move_to_zone(
                            crate::target::ChooseSpec::SpecificObject(card_id), self.destination, false,
                        );
                        crate::effects::execute_effect(game, &move_effect, ctx)?
                    };
                    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                    // Only actual arrivals belong to this move. A later
                    // movement by an addition is a different object/event.
                    // Added actions may report affected objects even when the
                    // original move was prevented. They are not search arrivals.
                    let ids = movement.objects().unwrap_or(&[]).to_vec();
                    let mut outcome = if ids.is_empty() { EffectOutcome::count(0) }
                        else { EffectOutcome::with_objects(ids.clone()).with_affected_objects(ids) };
                    outcome.events.push(search_event);
                    outcome.events.append(&mut movement.events);
                    outcome.execution_facts.append(&mut movement.execution_facts);
                    if let Some(memory) = chosen_memory {
                        outcome = outcome.with_chosen_object_memory(vec![memory]);
                    }
                    game.shuffle_player_library(player_id);
                    outcome.events.push(shuffle_event);
                    return Ok(outcome);
                }
            }

            // No card found or chosen - still shuffle (searching always shuffles)
            game.shuffle_player_library(player_id);

            Ok(EffectOutcome::count(0).with_events([search_event, shuffle_event]))
        })();

        if result.is_ok() && ctx.decision_maker.awaiting_choice() {
            game.capture_pending_decision_controllers();
        }
        // Active scopes always unwind. Only the pending routing view survives.
        finish_opposition_agent_search_control(game, search_control);
        result
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || instruction.is_err() { game.restore_execution_checkpoint(checkpoint, pending && instruction.is_ok()); context_checkpoint.restore(ctx); }
        if pending { return instruction.map(|_| EffectOutcome::count(0)); }
        instruction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
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

    fn add_library_card_of_type(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        card_type: CardType,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
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

        assert!(
            dm.calls.iter().any(|call| {
                call.viewer == alice
                    && call.subject == alice
                    && call.zone == Zone::Library
                    && !call.public
                    && call.cards == vec![first, second]
            }),
            "search should privately expose the searchable library to the searching player"
        );
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
    fn search_library_can_restore_found_card_third_from_top_after_shuffle() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        add_library_card_of_type(&mut game, alice, "Bottom Land", CardType::Land);
        let found =
            add_library_card_of_type(&mut game, alice, "Long-Term Target", CardType::Instant);
        add_library_card_of_type(&mut game, alice, "Middle Land", CardType::Land);
        add_library_card_of_type(&mut game, alice, "Top Land", CardType::Land);
        let source = game.new_object_id();
        let mut dm = CaptureSearchDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = SearchLibraryEffect::new(
            ObjectFilter::default().with_type(CardType::Instant),
            Zone::Library,
            PlayerFilter::You,
            PlayerFilter::You,
            false,
        )
        .with_library_position_from_top(crate::effect::Value::Fixed(3));

        effect
            .execute(&mut game, &mut ctx)
            .expect("search should resolve");

        let library = &game.player(alice).expect("alice exists").library;
        assert_eq!(library.len(), 4);
        assert_eq!(
            library[library.len() - 3],
            found,
            "searched card should be third from the top after the shuffle"
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

    #[test]
    fn transfigure_search_uses_the_sacrificed_sources_last_known_mana_value() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::new(), "Transfigure Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).expect("source exists"),
                &game,
            );
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should be sacrificed before resolution");

        let matching = CardBuilder::new(CardId::new(), "Matching Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Creature])
            .build();
        let _matching = game.create_object_from_card(&matching, alice, Zone::Library);
        let wrong_value = CardBuilder::new(CardId::new(), "Wrong-Value Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Creature])
            .build();
        let wrong_value = game.create_object_from_card(&wrong_value, alice, Zone::Library);
        let wrong_type = CardBuilder::new(CardId::new(), "Wrong-Type Relic")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Artifact])
            .build();
        let wrong_type = game.create_object_from_card(&wrong_type, alice, Zone::Library);

        let filter = ObjectFilter::default()
            .with_type(CardType::Creature)
            .with_mana_value(crate::filter::Comparison::EqualExpr(Box::new(
                crate::effect::Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Source)),
            )));
        let effect = SearchLibraryEffect::to_battlefield(filter, PlayerFilter::You, false);
        let mut dm = CaptureSearchDm::default();
        let mut ctx =
            ExecutionContext::new(source, alice, &mut dm).with_source_snapshot(source_snapshot);
        let filter_ctx = ctx.filter_context(&game);
        assert!(
            effect.filter.matches(
                game.object(_matching).expect("matching card exists"),
                &filter_ctx,
                &game,
            ),
            "the dynamic filter should match against source LKI before the search"
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("search resolves");

        assert!(game.battlefield.iter().any(|object| {
            game.object(*object)
                .is_some_and(|object| object.name == "Matching Creature")
        }));
        assert_eq!(
            game.object(wrong_value)
                .expect("wrong-value card exists")
                .zone,
            Zone::Library
        );
        assert_eq!(
            game.object(wrong_type)
                .expect("wrong-type card exists")
                .zone,
            Zone::Library
        );
    }
}

#[cfg(test)]
mod replacement_search_owner_contract_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, PlayerId, StableId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::static_abilities::StaticAbility;
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::types::CardType;

    struct Answers { found: StableId, agent: Option<ObjectId>, pause: bool, pending: bool, calls: usize, binding: bool, searching_player: PlayerId, prompt_controller: Option<PlayerId> }
    impl DecisionMaker for Answers {
        fn decide_objects(&mut self, game: &GameState, c: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            c.candidates.iter().filter(|candidate| candidate.legal && game.object(candidate.id)
                .is_some_and(|card| card.stable_id == self.found)).map(|candidate| candidate.id).collect()
        }
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            self.prompt_controller = Some(game.controlling_player_for(self.searching_player));
            let id = game.find_object_by_stable_id(self.found).unwrap();
            assert_eq!(game.object(id).unwrap().zone, if self.agent.is_some() {Zone::Exile} else {Zone::Hand});
            if let Some(agent) = self.agent {
                assert!(game.effect_store.grant_registry.card_can_play_from_zone(game, id, Zone::Exile,
                    game.controller_of(game.object(agent).unwrap())), "found-card permission precedes additions");
            }
            if self.binding { assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 1); }
            self.pending = self.pause;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, name: &str, owner: PlayerId, zone: Zone) -> ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature]).build(), owner, zone)
    }
    fn search(kind: u8, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        if kind == 2 {
            crate::effects::ChooseObjectsEffect::new(ObjectFilter::default().in_zone(Zone::Library)
                .owned_by(PlayerFilter::You), 1, PlayerFilter::You, "found")
                .in_zone(Zone::Library).as_search().execute(game, ctx)
        } else {
            SearchLibraryEffect::to_hand(ObjectFilter::default().with_type(CardType::Creature),
                PlayerFilter::You, false).execute(game, ctx)
        }
    }
    fn check(kind: u8, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let parent = card(&mut game, "Search parent", alice, Zone::Battlefield);
        let replacement_source = card(&mut game, "Search addition source", bob, Zone::Battlefield);
        let agent = if kind == 0 {None} else {
            let definition = CardDefinitionBuilder::new(CardId::new(), "Search controller")
                .card_types(vec![CardType::Creature])
                .with_ability(Ability::static_ability(StaticAbility::control_opponents_while_searching_libraries()))
                .with_ability(Ability::static_ability(StaticAbility::opponent_search_exile_found_cards())).build();
            Some(game.create_object_from_definition(&definition, bob, Zone::Battlefield))
        };
        let found = card(&mut game, "Search found", alice, Zone::Library);
        let stable = game.object(found).unwrap().stable_id;
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let effects = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne, 1,
                ChooseSpec::tagged("it"))), Effect::may(vec![Effect::gain_life(0)])],
            _ => vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])],
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement_source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(found), Some(Zone::Library), Some(if agent.is_some() {Zone::Exile} else {Zone::Hand})),
            ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events();
        let before_ids = game.next_object_id_counter();
        let library = game.player(alice).unwrap().library.clone();
        let mut dm = Answers { found: stable, agent, pause: mode == 2, pending: false, calls: 0, binding: mode == 3, searching_player: alice, prompt_controller: None };
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
        ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        ctx.set_tagged_objects("found", vec![sentinel.clone()]);
        let result = search(kind, &mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 {
            assert!(ctx.decision_maker.awaiting_choice());
            assert!(result.unwrap().events.is_empty(), "pending search must publish no completed search or shuffle");
        } else {
            let outcome = result.unwrap(); assert_eq!(outcome.output_objects().len(), 1);
            let arrival = outcome.output_objects()[0];
            assert_eq!(game.object(arrival).unwrap().zone, if agent.is_some() {Zone::Exile} else {Zone::Hand});
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, if mode == 3 {20} else {27});
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert!(!outcome.events.is_empty());
            if mode == 3 { assert_eq!(game.counter_count(arrival, CounterType::PlusOnePlusOne), 1); }
        }
        if mode != 2 {
            assert_eq!(game.controlling_player_for(alice), alice, "completed or failed search must release control");
        }
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, sentinel.object_id);
        assert_eq!(game.counter_count(parent, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert_eq!(game.player(alice).unwrap().library, library);
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(ctx.get_tagged_all("found").unwrap()[0].object_id, sentinel.object_id);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode == 2 {
            // A pending routing view must describe the actual prompt, even
            // though physical state and active scopes roll back. Completion
            // below separately proves that no control survives the resume.
            assert_eq!(game.controlling_player_for(alice), dm.prompt_controller.expect("actual added-instruction prompt"));
        }
        if mode == 0 || mode == 3 { assert_eq!(dm.calls, 1, "arrival inspection must run"); }
        if mode == 2 {
            assert_eq!(dm.calls, 1); dm.pause = false; dm.pending = false;
            let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
            let outcome = search(kind, &mut game, &mut ctx).unwrap();
            assert_eq!(outcome.output_objects().len(), 1);
            assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.calls, 2);
            assert_eq!(game.controlling_player_for(alice), alice);
        }
    }
    #[test] fn ordinary_addition_executes_before_shuffle() { check(0,0); }
    #[test] fn ordinary_error_restores_search_owner() { check(0,1); }
    #[test] fn ordinary_pending_replays_search_owner() { check(0,2); }
    #[test] fn ordinary_addition_binds_arrival() { check(0,3); }
    #[test] fn agent_addition_executes_after_permission() { check(1,0); }
    #[test] fn agent_error_restores_search_owner() { check(1,1); }
    #[test] fn agent_pending_replays_search_owner() { check(1,2); }
    #[test] fn agent_addition_binds_arrival() { check(1,3); }
    #[test] fn choice_addition_executes_after_permission() { check(2,0); }
    #[test] fn choice_error_restores_search_owner() { check(2,1); }
    #[test] fn choice_pending_replays_search_owner() { check(2,2); }
    #[test] fn choice_addition_binds_arrival() { check(2,3); }
}
