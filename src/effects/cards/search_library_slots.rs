//! Search a library for multiple differently-constrained cards as one search.

use std::collections::HashSet;

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::events::{SearchLibraryEvent, ShuffleLibraryEvent};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchLibrarySlot {
    pub filter: ObjectFilter,
    pub required: bool,
}

impl SearchLibrarySlot {
    pub fn optional(filter: ObjectFilter) -> Self {
        Self {
            filter,
            required: false,
        }
    }

    pub fn required(filter: ObjectFilter) -> Self {
        Self {
            filter,
            required: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchLibrarySlotsEffect {
    pub slots: Vec<SearchLibrarySlot>,
    pub destination: Zone,
    pub chooser: PlayerFilter,
    pub player: PlayerFilter,
    pub reveal: bool,
    pub progress_tag: TagKey,
}

impl SearchLibrarySlotsEffect {
    pub fn new(
        slots: Vec<SearchLibrarySlot>,
        destination: Zone,
        chooser: PlayerFilter,
        player: PlayerFilter,
        reveal: bool,
        progress_tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            slots,
            destination,
            chooser,
            player,
            reveal,
            progress_tag: progress_tag.into(),
        }
    }

    pub fn to_hand(
        slots: Vec<SearchLibrarySlot>,
        player: PlayerFilter,
        reveal: bool,
        progress_tag: impl Into<TagKey>,
    ) -> Self {
        Self::new(
            slots,
            Zone::Hand,
            player.clone(),
            player,
            reveal,
            progress_tag,
        )
    }
}

impl EffectExecutor for SearchLibrarySlotsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser_id = resolve_player_filter(game, &self.chooser, ctx)?;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        if !game.can_search_library(chooser_id) {
            return Ok(EffectOutcome::prevented());
        }

        let search_event = TriggerEvent::new_with_provenance(
            SearchLibraryEvent::new(chooser_id, Some(player_id)),
            ctx.provenance,
        );
        let shuffle_event = TriggerEvent::new_with_provenance(
            ShuffleLibraryEvent::new(player_id, ctx.cause.clone()),
            ctx.provenance,
        );

        let mut chosen: Vec<ObjectSnapshot> = ctx
            .get_tagged_all(self.progress_tag.as_str())
            .cloned()
            .unwrap_or_default();
        if chosen.len() > self.slots.len() {
            chosen.clear();
            ctx.clear_object_tag(self.progress_tag.as_str());
        }

        for slot in self.slots.iter().skip(chosen.len()) {
            let filter_ctx = ctx.filter_context(game);
            let already_chosen: HashSet<ObjectId> =
                chosen.iter().map(|snapshot| snapshot.object_id).collect();
            let matching_cards: Vec<ObjectId> = game
                .player(player_id)
                .map(|player| {
                    player
                        .library
                        .iter()
                        .copied()
                        .filter(|id| !already_chosen.contains(id))
                        .filter(|id| {
                            game.object(*id)
                                .is_some_and(|obj| slot.filter.matches(obj, &filter_ctx, game))
                        })
                        .collect()
                })
                .unwrap_or_default();

            if matching_cards.is_empty() {
                continue;
            }

            let chosen_card = if slot.required {
                make_decision_with_fallback(
                    game,
                    &mut ctx.decision_maker,
                    chooser_id,
                    Some(ctx.source),
                    SearchSpec::mandatory(ctx.source, matching_cards, self.reveal),
                    FallbackStrategy::FirstOption,
                )
            } else {
                make_decision_with_fallback(
                    game,
                    &mut ctx.decision_maker,
                    chooser_id,
                    Some(ctx.source),
                    SearchSpec::new(ctx.source, matching_cards, self.reveal),
                    FallbackStrategy::FirstOption,
                )
            };

            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0).with_event(search_event));
            }

            let Some(card_id) = chosen_card else {
                continue;
            };
            let Some(snapshot) = game
                .object(card_id)
                .filter(|obj| obj.zone == Zone::Library)
                .map(|obj| ObjectSnapshot::from_object(obj, game))
            else {
                continue;
            };
            chosen.push(snapshot.clone());
            ctx.tag_object(self.progress_tag.clone(), snapshot);
        }

        let mut moved_ids = Vec::new();
        let chosen_ids: Vec<ObjectId> = chosen
            .iter()
            .map(|snapshot| snapshot.object_id)
            .filter(|id| {
                game.player(player_id)
                    .is_some_and(|player| player.library.contains(id))
            })
            .collect();

        if self.destination == Zone::Library {
            if let Some(player) = game.player_mut(player_id) {
                player.library.retain(|id| !chosen_ids.contains(id));
            }
            game.shuffle_player_library(player_id);
            if let Some(player) = game.player_mut(player_id) {
                for card_id in chosen_ids {
                    player.library.push(card_id);
                    moved_ids.push(card_id);
                }
            }
        } else {
            for card_id in chosen_ids {
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
                    game.move_object_by_effect(card_id, self.destination)
                };

                if let Some(new_id) = new_id {
                    moved_ids.push(new_id);
                }
            }
            game.shuffle_player_library(player_id);
        }

        ctx.clear_object_tag(self.progress_tag.as_str());

        if moved_ids.is_empty() {
            Ok(EffectOutcome::count(0).with_events([search_event, shuffle_event]))
        } else {
            Ok(EffectOutcome::with_objects(moved_ids).with_events([search_event, shuffle_event]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::{CardType, Subtype, Supertype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn push_basic_land(game: &mut GameState, controller: PlayerId, name: &str, subtype: Subtype) {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Basic])
            .subtypes(vec![subtype])
            .mana_cost(ManaCost::new())
            .build();
        game.create_object_from_card(&card, controller, Zone::Library);
    }

    struct PendingOnSecondChoiceDm {
        calls: usize,
    }

    impl DecisionMaker for PendingOnSecondChoiceDm {
        fn awaiting_choice(&self) -> bool {
            self.calls >= 2
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.calls += 1;
            if self.calls == 1 {
                ctx.candidates
                    .iter()
                    .find(|candidate| candidate.legal)
                    .map(|candidate| vec![candidate.id])
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn search_library_slots_moves_multiple_different_cards_to_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        push_basic_land(&mut game, alice, "Forest", Subtype::Forest);
        push_basic_land(&mut game, alice, "Plains", Subtype::Plains);

        let source = ObjectId::from_raw(999);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SearchLibrarySlotsEffect::to_hand(
            vec![
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Forest),
                ),
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Plains),
                ),
            ],
            PlayerFilter::You,
            true,
            "progress",
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("search should resolve");

        assert_eq!(outcome.output_objects().len(), 2);
        let hand_names: Vec<_> = game
            .player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.name.clone()))
            .collect();
        assert!(hand_names.iter().any(|name| name == "Forest"));
        assert!(hand_names.iter().any(|name| name == "Plains"));
        assert!(ctx.get_tagged_all("progress").is_none());
    }

    #[test]
    fn search_library_slots_keeps_progress_across_resume() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        push_basic_land(&mut game, alice, "Forest", Subtype::Forest);
        push_basic_land(&mut game, alice, "Plains", Subtype::Plains);

        let effect = SearchLibrarySlotsEffect::to_hand(
            vec![
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Forest),
                ),
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Plains),
                ),
            ],
            PlayerFilter::You,
            true,
            "progress",
        );
        let source = ObjectId::from_raw(1000);
        let ctx = ExecutionContext::new_default(source, alice);

        let mut pending_dm = PendingOnSecondChoiceDm { calls: 0 };
        let mut ctx = ctx.with_decision_maker(&mut pending_dm);
        let first = effect
            .execute(&mut game, &mut ctx)
            .expect("first pass should execute");
        assert_eq!(first.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            ctx.get_tagged_all("progress")
                .expect("first selected card should be remembered")
                .len(),
            1
        );
        assert_eq!(game.player(alice).expect("alice exists").hand.len(), 0);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);
        let second = effect
            .execute(&mut game, &mut ctx)
            .expect("resume should execute");
        assert_eq!(second.output_objects().len(), 2);
        assert!(ctx.get_tagged_all("progress").is_none());
        assert_eq!(game.player(alice).expect("alice exists").hand.len(), 2);
    }
}
