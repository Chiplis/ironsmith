//! Search library effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, SearchSelectionMode};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::effects::{
    CompletedEffectOutputs, OriginalPhaseStatus, SimultaneousEffectCommit,
    SimultaneousEffectCompletion,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::SearchLibraryEvent;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;
use std::borrow::Cow;

use super::search_overrides::{
    LibrarySearchLook, LibrarySearchRequest, LibrarySearchScope, OppositionAgentSearch,
    execute_library_search_scope_after_look,
};

pub type SearchLibraryEffect = ironsmith_core::SearchLibraryEffect;

impl EffectExecutor for SearchLibraryEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let search = SingleCardSearch::new(Cow::Owned(self.clone()), game, ctx)?;
        let request = search.request(ctx);
        Ok(super::simultaneous_search::prepare_search(search, request))
    }

    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let search = SingleCardSearch::new(Cow::Borrowed(self), game, ctx)?;
        search.run(game, ctx, None, None)
    }
}

#[derive(Debug)]
struct SingleCardSearch<'a> {
    effect: Cow<'a, SearchLibraryEffect>,
    chooser: PlayerId,
    owner: PlayerId,
}

#[derive(Debug)]
struct FoundCard {
    chosen: Option<ObjectId>,
    allowed: bool,
    policy: Option<OppositionAgentSearch>,
    event: Option<crate::triggers::TriggerEvent>,
    reveal: Option<CompletedEffectOutputs>,
}

impl<'a> SingleCardSearch<'a> {
    fn new(
        effect: Cow<'a, SearchLibraryEffect>,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<Self, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &effect.chooser, ctx)?;
        let owner = crate::effects::helpers::resolve_player_filter(game, &effect.player, ctx)?;
        Ok(Self {
            effect,
            chooser,
            owner,
        })
    }
    fn request(&self, ctx: &ExecutionContext) -> LibrarySearchRequest {
        LibrarySearchRequest {
            chooser: self.chooser,
            library_owner: Some(self.owner),
            search_library: true,
            require_library_access: ctx.optional_action,
            library_scope_cards: None,
            refresh_library_access: false,
        }
    }
    fn choose(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        search: &LibrarySearchScope,
    ) -> Result<Option<ObjectId>, ExecutionError> {
        if !search.library_access {
            return Ok(None);
        }
        let chooser_id = self.chooser;
        let library_cards = &search.library_cards;
        let filter_ctx = ctx.filter_context(game);
        let matching_cards: Vec<_> = library_cards
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|obj| {
                    obj.zone == Zone::Library && self.effect.filter.matches(obj, &filter_ctx, game)
                })
            })
            .collect();
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
        let may_fail_to_find = match self.effect.search_mode {
            SearchSelectionMode::Exact => self.effect.filter.has_search_stated_quality(),
            SearchSelectionMode::Optional | SearchSelectionMode::AllMatching => true,
        };
        let spec = if may_fail_to_find {
            SearchSpec::new(ctx.source, decision_candidates, self.effect.reveal)
        } else {
            SearchSpec::mandatory(ctx.source, decision_candidates, self.effect.reveal)
        }
        .with_viewed_cards(library_cards.to_vec());
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
            return Ok(None);
        }
        if chosen_card.is_none() && !may_fail_to_find {
            chosen_card = matching_cards.first().copied();
        }

        // Validate the selected physical identity before any participant moves it.
        if let Some(card_id) = chosen_card {
            if !library_cards.contains(&card_id) {
                return Err(ExecutionError::InvalidTarget);
            }
            let chosen_matches = game
                .object(card_id)
                .is_some_and(|obj| self.effect.filter.matches(obj, &filter_ctx, game));
            if !chosen_matches && !game.is_hidden_card_placeholder(card_id) {
                return Err(ExecutionError::InvalidTarget);
            }
        }
        Ok(chosen_card)
    }

    fn reveal(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        found: &mut FoundCard,
    ) -> Result<(), ExecutionError> {
        let Some(card_id) = found.chosen.filter(|_| self.effect.reveal) else {
            return Ok(());
        };
        let chooser_id = self.chooser;
        let selected = ObjectSnapshot::from_object_id(game, card_id).ok_or_else(|| {
            ExecutionError::IncompleteEvidence("selected search card has no object snapshot".into())
        })?;
        let outputs = super::reveal_objects_with_outputs(
            game,
            ctx,
            vec![selected],
            Some(chooser_id),
            "Reveal searched card",
            None,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }

        found.reveal = Some(outputs);
        Ok(())
    }

    fn move_found_card(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        mut found: FoundCard,
    ) -> Result<(CompletedEffectOutputs, SearchShuffle), ExecutionError> {
        let mut children = Vec::new();
        let mut disclosures = Vec::new();
        if let Some(reveal) = found.reveal.take() {
            disclosures.push(reveal.outcome.clone());
            children.push(reveal);
        }
        let mut shuffle = SearchShuffle {
            owner: self.owner,
            restore: None,
        };
        let outcome = self.movement(game, ctx, found, &disclosures, &mut children, &mut shuffle)?;
        Ok((
            CompletedEffectOutputs::from_children(children, |_| outcome),
            shuffle,
        ))
    }

    fn movement(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        found: FoundCard,
        disclosures: &[EffectOutcome],
        retained_children: &mut Vec<CompletedEffectOutputs>,
        shuffle: &mut SearchShuffle,
    ) -> Result<EffectOutcome, ExecutionError> {
        let FoundCard {
            chosen,
            allowed,
            policy: search_override,
            event: search_event,
            ..
        } = found;
        let chooser_id = self.chooser;
        let player_id = self.owner;
        if !allowed {
            // Prohibiting the mandatory search leaves its shuffle instruction intact.
            return Ok(EffectOutcome::prevented());
        }
        if let Some(card_id) = chosen.filter(|id| {
            game.player(player_id)
                .is_some_and(|p| p.library.contains(id))
        }) {
            let chosen_memory = ObjectSnapshot::from_object_id(game, card_id);
            // For "put on top of library" effects (like Vampiric Tutor), we need to:
            // 1. Remove the card from the library
            // 2. Shuffle the library
            // 3. Put the card on top
            // This matches the card text "then shuffle and put that card on top"
            if self.effect.destination == Zone::Library && search_override.is_none() {
                let position_from_top =
                    if let Some(position) = self.effect.library_position_from_top.as_ref() {
                        resolve_value(game, position, ctx)?.max(1) as usize
                    } else {
                        1
                    };
                shuffle.restore =
                    Some((card_id, position_from_top, chooser_id, self.effect.reveal));
                let mut outcome =
                    EffectOutcome::with_objects(vec![card_id]).with_events(search_event.clone());
                if let Some(memory) = chosen_memory {
                    outcome = outcome
                        .with_chosen_object_memory(vec![memory.clone()])
                        .with_affected_object_memory(vec![memory]);
                }
                let outcome = include_search_disclosures(outcome, disclosures);
                return Ok(outcome);
            }

            // Complete the found-card instruction and all permission
            // links before additions; shuffle only after it resolves.
            let movement_outputs = if search_override.is_some() {
                let (found, published_outputs) = super::search_result_program::prepare_found_cards(
                    game,
                    ctx,
                    &[card_id],
                    self.effect.destination,
                    chooser_id,
                    true,
                    false,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let mut original = if found.moved_ids.is_empty() {
                    EffectOutcome::count(0)
                } else {
                    EffectOutcome::with_objects(found.moved_ids.clone())
                        .with_affected_objects(found.moved_ids.clone())
                };
                if let Some(memory) = chosen_memory.clone() {
                    original = original.with_chosen_object_memory(vec![memory]);
                }
                super::search_result_program::finish_found_cards(
                    game,
                    ctx,
                    found,
                    published_outputs,
                    original,
                )?
            } else {
                let move_to = |zone: Zone| {
                    crate::effect::Effect::move_to_zone(
                        crate::target::ChooseSpec::SpecificObject(card_id),
                        zone,
                        false,
                    )
                };
                // "put it into your hand or graveyard" (Dina's
                // Guidance): the searcher picks the destination
                // after the found card is revealed.
                let move_effect = if let Some(alternative) = self.effect.alternative_destination {
                    crate::effect::Effect::new(
                        crate::effects::ChooseModeEffect::choose_one(vec![
                            crate::effect::EffectMode {
                                source_text: format!("Put it into {:?}", self.effect.destination),
                                effects: vec![move_to(self.effect.destination)],
                            },
                            crate::effect::EffectMode {
                                source_text: format!("Put it into {alternative:?}"),
                                effects: vec![move_to(alternative)],
                            },
                        ])
                        .with_chooser(crate::target::PlayerFilter::Specific(chooser_id)),
                    )
                } else {
                    move_to(self.effect.destination)
                };
                crate::effects::execute_effect_with_outputs(game, &move_effect, ctx)?
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let mut movement = movement_outputs.outcome.clone();
            retained_children.push(movement_outputs);
            // Only actual arrivals belong to this move. A later
            // movement by an addition is a different object/event.
            // Added actions may report affected objects even when the
            // original move was prevented. They are not search arrivals.
            let ids = movement.objects().unwrap_or(&[]).to_vec();
            let mut outcome = if ids.is_empty() {
                EffectOutcome::count(0)
            } else {
                EffectOutcome::with_objects(ids.clone()).with_affected_objects(ids)
            };
            outcome.events.extend(search_event);
            outcome.events.append(&mut movement.events);
            outcome
                .execution_facts
                .append(&mut movement.execution_facts);
            if let Some(memory) = chosen_memory {
                outcome = outcome.with_chosen_object_memory(vec![memory]);
            }
            let outcome = include_search_disclosures(outcome, disclosures);

            return Ok(outcome);
        }

        Ok(EffectOutcome::count(0).with_events(search_event))
    }

    fn run(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        look: Option<LibrarySearchLook>,
        mut selected: Option<&mut Option<FoundCard>>,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        game.clear_pending_decision_controllers();
        let mut published = Vec::new();
        let mut retained = Vec::new();
        let instruction =
            crate::effects::composition::execute_result_transaction(game, ctx, |game, ctx| {
                execute_library_search_scope_after_look(
                    game,
                    ctx,
                    self.request(ctx),
                    look,
                    |game, ctx, mut search| {
                        published.append(&mut search.completed_outputs);
                        let chosen = self.choose(game, ctx, &search)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        let mut found = FoundCard {
                            chosen,
                            allowed: search.library_access,
                            policy: search.found_card_policy,
                            event: search.event,
                            reveal: None,
                        };
                        if let Some(output) = selected.as_mut() {
                            **output = Some(found);
                            return Ok(EffectOutcome::resolved());
                        }
                        self.reveal(game, ctx, &mut found)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        let (moved, shuffle) = self.move_found_card(game, ctx, found)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        let outputs = shuffle.apply(game, ctx, moved)?;
                        let outcome = outputs.outcome.clone();
                        retained.push(outputs);
                        Ok(outcome)
                    },
                )
            });
        if ctx.decision_maker.awaiting_choice() {
            return instruction
                .map(|_| CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
        }
        instruction.map(|outcome| {
            if selected.as_deref().is_none_or(Option::is_none) {
                super::search_overrides::retain_search_participants(
                    ctx,
                    self.effect.participant_tag.as_ref(),
                    &outcome,
                );
            }
            let mut outputs = CompletedEffectOutputs::from_children(retained, |_| outcome);
            outputs.retain_published_references(published);
            outputs
        })
    }
}

impl super::simultaneous_search::LibrarySearchAction for SingleCardSearch<'static> {
    type Selection = (Option<FoundCard>, CompletedEffectOutputs);
    fn select(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        look: LibrarySearchLook,
    ) -> Result<Self::Selection, ExecutionError> {
        let mut selected = None;
        let inputs = self.run(game, ctx, Some(look), Some(&mut selected))?;
        Ok((selected, inputs))
    }
    fn disclose(
        &self,
        (selected, _): &mut Self::Selection,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if let Some(found) = selected {
            self.reveal(game, ctx, found)?;
        }
        Ok(())
    }
    fn finish(
        &self,
        (selected, inputs): Self::Selection,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let Some(found) = selected else {
            return Ok(SimultaneousEffectCommit::finished(inputs));
        };
        let event = found.event.clone();
        let (moved, shuffle) = self.move_found_card(game, ctx, found)?;
        super::search_overrides::retain_search_participants(
            ctx,
            self.effect.participant_tag.as_ref(),
            &moved.outcome,
        );
        if ctx.resolution_stopped() || ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                moved.append_owned_child(inputs),
            ));
        }
        let prefix = inputs.project_aggregate(EffectOutcome::resolved().with_events(event));
        Ok(SimultaneousEffectCommit {
            outcome: prefix,
            completion: Some(Box::new(FoundCardCompletion { moved, shuffle })),
        })
    }
}

/// Physical movements run in the open simultaneous action. Retain their observations
/// until every search has been recorded, then shuffle every participating library.
struct FoundCardCompletion {
    moved: CompletedEffectOutputs,
    shuffle: SearchShuffle,
}
impl SimultaneousEffectCompletion for FoundCardCompletion {
    fn freeze(&mut self, _: &mut GameState) -> Result<(), ExecutionError> {
        Ok(())
    }
    fn original_phase_status(&self) -> OriginalPhaseStatus {
        OriginalPhaseStatus::Retained
    }
    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        _: &mut GameState,
        _: &mut ExecutionContext,
        prefix: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let mut moved = self.moved;
        crate::effects::composition::inherit_original_observations(
            &mut moved.outcome,
            &prefix.outcome.events,
        );
        moved.retain_owned_child(prefix);
        Ok(SimultaneousEffectCommit {
            outcome: moved,
            completion: Some(Box::new(self.shuffle)),
        })
    }
    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        prefix: CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let receipt = self.complete_original_phase_from_outputs(game, ctx, prefix)?;
        crate::effects::composition::complete_committed_original_with_outputs(game, ctx, receipt)
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        prefix: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_from_original_outputs(
            game,
            ctx,
            CompletedEffectOutputs::aggregate_only(prefix),
        )
        .map(CompletedEffectOutputs::into_outcome)
    }
}

struct SearchShuffle {
    owner: PlayerId,
    // Card, position from the top, searcher, and whether placement is public.
    restore: Option<(ObjectId, usize, PlayerId, bool)>,
}
impl SearchShuffle {
    fn apply(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        moved: CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let shuffle = if let Some((card, position, searcher, public)) = self.restore {
            let result = super::search_result_program::shuffle_after_search(
                game,
                ctx,
                self.owner,
                &[card],
                position,
                "searched card restored after library shuffle",
            )?;
            if !ctx.decision_maker.awaiting_choice()
                && game.object(card).is_some_and(|o| o.zone == Zone::Library)
            {
                let viewers: Vec<_> = game
                    .players
                    .iter()
                    .map(|p| p.id)
                    .filter(|&p| public || p == searcher)
                    .collect();
                for viewer in viewers {
                    game.remember_library_group(viewer, self.owner, &[card], false);
                }
            }
            result
        } else {
            super::search_result_program::shuffle_after_search(
                game,
                ctx,
                self.owner,
                &[],
                1,
                "library shuffled after search",
            )?
        };
        Ok(moved.append_batch_completion_outputs(shuffle))
    }
}
impl SimultaneousEffectCompletion for SearchShuffle {
    fn freeze(&mut self, _: &mut GameState) -> Result<(), ExecutionError> {
        Ok(())
    }
    fn original_phase_status(&self) -> OriginalPhaseStatus {
        OriginalPhaseStatus::Complete
    }
    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.apply(game, ctx, original)
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.apply(game, ctx, CompletedEffectOutputs::aggregate_only(original))
            .map(CompletedEffectOutputs::into_outcome)
    }
}

/// The search event is the first observation in these authored projections.
/// Keep actual reveal observations between that search and its movement/shuffle,
/// while their object facts remain auxiliary to the unchanged search result.
fn include_search_disclosures(
    original: EffectOutcome,
    disclosures: &[EffectOutcome],
) -> EffectOutcome {
    if disclosures.is_empty() {
        return original;
    }
    // CR 701.23h. A later selection in the same search has no second
    // search event. Its reveal precedes that selection's other observations.
    let after_search = usize::from(
        original
            .events
            .first()
            .is_some_and(|event| event.downcast::<SearchLibraryEvent>().is_some()),
    );
    let original_event_count = original.events.len();
    let events = disclosures
        .iter()
        .flat_map(|outcome| outcome.events.iter().cloned())
        .collect::<Vec<_>>();
    let mut outcome =
        EffectOutcome::aggregate_replacement_outcomes(original, disclosures.iter().cloned());
    // These are alternate projections of actual child packets, not new events.
    outcome.events.truncate(original_event_count);
    outcome.events.splice(after_search..after_search, events);
    outcome
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
