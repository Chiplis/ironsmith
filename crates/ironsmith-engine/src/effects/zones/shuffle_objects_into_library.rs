//! Shuffle specific objects into a library, then shuffle that library.

use crate::effect::EffectOutcome;
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_objects_from_spec, resolve_player_filter,
};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, SimultaneousEffectCommit};
use crate::effects::{ExecutionContext, ExecutionError};
// Retained for the existing inline scenarios through super::*.
#[allow(unused_imports)]
use crate::events::ShuffleLibraryEvent;
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;
pub use ironsmith_core::ShuffleObjectsIntoLibraryEffect;

use crate::events::processing::{
    PreparedEventOutcome, PreparedZoneChange, ReplacementEventContext,
};

fn uses_affected_object_owner(player: &PlayerFilter) -> bool {
    matches!(
        player,
        PlayerFilter::OwnerOf(_) | PlayerFilter::AliasedOwnerOf(_)
    )
}

fn push_unique_player(players: &mut Vec<crate::ids::PlayerId>, player: crate::ids::PlayerId) {
    if !players.contains(&player) {
        players.push(player);
    }
}

fn expected_zone_for_object(
    target: &ChooseSpec,
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: crate::ids::ObjectId,
) -> Option<Zone> {
    match target.base() {
        ChooseSpec::Object(filter) => filter.zone,
        ChooseSpec::Tagged(tag) => ctx
            .get_tagged_all(tag)
            .and_then(|snapshots| snapshots.iter().find(|s| s.object_id == object_id))
            .map(|snapshot| snapshot.zone),
        ChooseSpec::Source => game.object(ctx.source).map(|obj| obj.zone),
        _ => None,
    }
}

#[derive(Debug)]
struct PreparedShuffleObjectsAction {
    objects: Vec<ObjectSnapshot>,
    players_to_shuffle: Vec<crate::ids::PlayerId>,
}

fn prepare_shuffle_objects_action_from_ids(
    effect: &ShuffleObjectsIntoLibraryEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    object_ids: Vec<crate::ids::ObjectId>,
) -> Result<PreparedShuffleObjectsAction, ExecutionError> {
    let objects = object_ids
        .iter()
        .filter_map(|&object_id| {
            let object = game.object(object_id)?;
            if expected_zone_for_object(&effect.target, game, ctx, object_id)
                .is_some_and(|zone| object.zone != zone)
            {
                return None;
            }
            Some(ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game))
        })
        .collect::<Result<Vec<_>, ExecutionError>>()?;

    let shuffle_affected_owners =
        effect.owner_library_destination || uses_affected_object_owner(&effect.player);
    let mut players_to_shuffle = Vec::new();
    if shuffle_affected_owners {
        for object_id in &object_ids {
            if let Some(owner) = game.object(*object_id).map(|object| object.owner) {
                push_unique_player(&mut players_to_shuffle, owner);
            }
        }
        if players_to_shuffle.is_empty()
            && let Ok(player) = resolve_player_filter(game, &effect.player, ctx)
        {
            push_unique_player(&mut players_to_shuffle, player);
        }
    } else {
        push_unique_player(
            &mut players_to_shuffle,
            resolve_player_filter(game, &effect.player, ctx)?,
        );
    }

    if effect.shuffle_subject_library {
        push_unique_player(
            &mut players_to_shuffle,
            resolve_player_filter(game, &effect.player, ctx)?,
        );
    }

    Ok(PreparedShuffleObjectsAction {
        objects,
        players_to_shuffle,
    })
}

fn prepare_shuffle_objects_action(
    effect: &ShuffleObjectsIntoLibraryEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<PreparedShuffleObjectsAction, ExecutionError> {
    let object_ids = match resolve_objects_for_effect(game, ctx, &effect.target) {
        Ok(ids) => ids,
        Err(ExecutionError::InvalidTarget) => Vec::new(),
        Err(err) => return Err(err),
    };
    prepare_shuffle_objects_action_from_ids(effect, game, ctx, object_ids)
}

fn prepare_simultaneous_shuffle_objects_action(
    effect: &ShuffleObjectsIntoLibraryEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<PreparedShuffleObjectsAction, ExecutionError> {
    // Simultaneous proposals are prepared from a read-only game state. Any
    // choices have already been made by the preceding selection effect, so
    // resolve only the objects currently recorded in the execution context.
    let object_ids = match resolve_objects_from_spec(game, &effect.target, ctx) {
        Ok(ids) => ids,
        Err(ExecutionError::InvalidTarget) => Vec::new(),
        Err(err) => return Err(err),
    };
    prepare_shuffle_objects_action_from_ids(effect, game, ctx, object_ids)
}

#[derive(Debug)]
struct ShuffleObjectsIntoLibraryProposal {
    prepared: PreparedShuffleObjectsAction,
    zones: Option<
        Vec<(
            ObjectSnapshot,
            PreparedEventOutcome<PreparedZoneChange>,
            std::ops::Range<usize>,
        )>,
    >,
    draws: crate::events::processing::ZoneDrawContinuations,
}

impl ShuffleObjectsIntoLibraryProposal {
    fn new(prepared: PreparedShuffleObjectsAction) -> Self {
        Self {
            prepared,
            zones: None,
            draws: Default::default(),
        }
    }

    fn prepare_zones(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        let additional = ctx.additional_replacement_effects_snapshot();
        let mut zones = Vec::new();
        for snapshot in &self.prepared.objects {
            let object_id = snapshot.object_id;
            if snapshot.zone == Zone::Library
                || !game
                    .object(object_id)
                    .is_some_and(|object| object.zone == snapshot.zone)
            {
                // A revealed library remainder keeps its original identities.
                continue;
            }
            let scope = ReplacementEventContext::with_scope(
                game,
                crate::events::Event::zone_change(
                    object_id,
                    snapshot.zone,
                    Zone::Library,
                    ctx.cause.clone(),
                    Some(snapshot.clone()),
                )
                .with_provenance(ctx.provenance),
                &ctx.replacement,
            );
            let draw_start = self.draws.0.len();
            let proposal = crate::events::processing::prepare_zone_change_scoped_with_draws(
                game,
                object_id,
                snapshot.zone,
                Zone::Library,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
                &additional,
                Some(snapshot.clone()),
                Some(&scope),
                Vec::new(),
                None,
                Some(&mut self.draws),
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
            zones.push((snapshot.clone(), proposal, draw_start..self.draws.0.len()));
        }
        self.zones = Some(zones);
        Ok(())
    }

    fn commit_zones(
        mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let zones = self.zones.ok_or_else(|| {
            ExecutionError::InternalError("shuffle original was not prepared".into())
        })?;
        let mut moved_ids = Vec::new();
        let mut receipts = Vec::new();
        for (snapshot, proposal, draws) in zones {
            let object_id = snapshot.object_id;
            let start = self.draws.0.len();
            self.draws
                .commit_pending_replacement(game, object_id, &mut *ctx.decision_maker)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            let mut ranges = Vec::new();
            if !draws.is_empty() {
                ranges.push(draws);
            }
            if start < self.draws.0.len() {
                ranges.push(start..self.draws.0.len());
            }
            let receipt = if matches!(&proposal.original, EventOutcome::Proceed(_))
                && !game
                    .object(object_id)
                    .is_some_and(|object| object.zone == snapshot.zone)
            {
                // An earlier replacement prefix may remove another original.
                // Do not revive that object's new incarnation or lose additions.
                PreparedEventOutcome {
                    original: EventOutcome::NotApplicable,
                    programs: proposal.programs,
                }
            } else {
                let committed = super::commit_zone_change_proposal_with_outputs(
                    game,
                    object_id,
                    proposal,
                    &mut *ctx.decision_maker,
                )?;
                crate::effects::PublishedEffectOutputs::append_distinct(
                    &mut self.draws.1,
                    committed.published_outputs,
                );
                committed.receipt
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            if let EventOutcome::Proceed(result) = &receipt.original {
                if !result.new_object_ids.is_empty() {
                    ctx.refresh_target_snapshot(snapshot.clone());
                    if object_id == ctx.source {
                        ctx.refresh_source_snapshot(snapshot);
                    }
                }
                if result.final_zone == Zone::Library {
                    for &new_id in &result.new_object_ids {
                        if let Some(owner) = game.object(new_id).map(|object| object.owner) {
                            crate::effects::cards::position_library_card(
                                game,
                                owner,
                                new_id,
                                crate::effects::cards::LibraryCardPosition::Bottom,
                                "card moved into library before shuffle",
                            );
                        }
                    }
                    moved_ids.extend(result.new_object_ids.iter().copied());
                }
            }
            receipts.push(((object_id, receipt), ranges));
        }
        let mut original = if moved_ids.is_empty() {
            EffectOutcome::count(0)
        } else {
            EffectOutcome::with_objects(moved_ids)
        };
        // Randomization is part of this original instruction. All participants
        // complete it before any appended zone-replacement program executes.
        let mut shuffle_outputs = Vec::new();
        for player in self.prepared.players_to_shuffle {
            let shuffle = crate::effects::cards::commit_library_shuffle_with_outputs(
                game,
                player,
                &[],
                1,
                "library shuffled",
                ctx.cause.clone(),
                |game| {
                    game.alloc_child_event_provenance(
                        ctx.provenance,
                        crate::events::EventKind::ShuffleLibrary,
                    )
                },
            );
            original
                .events
                .extend(shuffle.outcome.events.iter().cloned());
            shuffle_outputs.push(shuffle);
        }
        let mut committed = super::prepare_zone_instruction_completion(
            original,
            receipts,
            self.draws,
            ctx.iteration.iterated_player,
        )
        .into_retained();
        for shuffle in shuffle_outputs {
            committed.outcome.retain_owned_child(shuffle);
        }
        Ok(committed)
    }
}

impl crate::effects::SimultaneousEffectProposal for ShuffleObjectsIntoLibraryProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_zones(game, ctx)
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        (*self)
            .commit_zones(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        (*self).commit_zones(game, ctx)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, true)
            .map(CompletedEffectOutputs::into_outcome)
    }
}

impl EffectExecutor for ShuffleObjectsIntoLibraryEffect {
    fn own_preflight_object_specs(&self) -> Vec<ChooseSpec> {
        vec![self.target.clone()]
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Shuffled)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(ShuffleObjectsIntoLibraryProposal::new(
            prepare_simultaneous_shuffle_objects_action(self, game, ctx)?,
        )))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let prepared = prepare_shuffle_objects_action(self, game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                crate::effects::composition::complete_prepared_original_with_outputs(
                    Box::new(ShuffleObjectsIntoLibraryProposal::new(prepared)),
                    game,
                    ctx,
                    true,
                )
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "objects to shuffle"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::ChoiceCount;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::target::{ObjectFilter, PlayerFilter};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_card_in_zone(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn explicit_shuffle_participant_is_deduplicated_with_selected_owners() {
        for include_subject in [false, true] {
            for owner_index in [None, Some(0), Some(1)] {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                if let Some(index) = owner_index {
                    create_card_in_zone(
                        &mut game,
                        PlayerId::from_index(index),
                        Zone::Graveyard,
                        "Selected",
                    );
                }
                let source = game.new_object_id();
                let mut ctx = ExecutionContext::new_default(source, alice);
                let mut effect = ShuffleObjectsIntoLibraryEffect::new(
                    ChooseSpec::All(ObjectFilter::default().in_zone(Zone::Graveyard)),
                    PlayerFilter::You,
                )
                .with_owner_library_destination();
                effect.shuffle_subject_library = include_subject;
                let outcome = effect.execute(&mut game, &mut ctx).unwrap();
                let mut shuffled: Vec<_> = outcome
                    .events
                    .iter()
                    .filter_map(|event| {
                        event
                            .downcast::<ShuffleLibraryEvent>()
                            .map(|event| event.player)
                    })
                    .collect();
                let mut expected = vec![PlayerId::from_index(owner_index.unwrap_or(0))];
                if include_subject && owner_index == Some(1) {
                    expected.push(alice);
                }
                shuffled.sort();
                expected.sort();
                assert_eq!(
                    shuffled, expected,
                    "include={include_subject}; owner={owner_index:?}"
                );
            }
        }
    }

    #[test]
    fn shuffle_objects_already_in_library_preserves_identity_and_emits_only_shuffle() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_card_in_zone(&mut game, alice, Zone::Library, "A");
        let second = create_card_in_zone(&mut game, alice, Zone::Library, "B");
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ShuffleObjectsIntoLibraryEffect::new(
            ChooseSpec::All(ObjectFilter::default().in_zone(Zone::Library)),
            PlayerFilter::You,
        );
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        let library = &game.player(alice).unwrap().library;
        assert_eq!(library.len(), 2);
        assert!(library.contains(&first) && library.contains(&second));
        assert_eq!(outcome.events.len(), 1);
        assert!(
            outcome.events[0]
                .downcast::<ShuffleLibraryEvent>()
                .is_some()
        );
    }

    #[test]
    fn shuffle_objects_into_library_still_shuffles_with_zero_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_card_in_zone(&mut game, alice, Zone::Library, "A");
        create_card_in_zone(&mut game, alice, Zone::Library, "B");
        let before = game.irreversible_random_count();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let spec = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        ))
        .with_count(ChoiceCount::up_to(2));
        let effect = ShuffleObjectsIntoLibraryEffect::new(spec, PlayerFilter::You);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("shuffle should resolve");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "zero-target shuffle-into-library effects should still shuffle"
        );
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.downcast::<ShuffleLibraryEvent>().is_some()),
            "zero-target shuffle-into-library effects should still emit a shuffle event"
        );
    }

    #[test]
    fn shuffle_objects_into_library_still_shuffles_if_object_left_expected_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let graveyard_card = create_card_in_zone(&mut game, alice, Zone::Graveyard, "Target");
        create_card_in_zone(&mut game, alice, Zone::Library, "A");
        create_card_in_zone(&mut game, alice, Zone::Library, "B");
        let before = game.irreversible_random_count();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let spec = ChooseSpec::Object(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
        let effect = ShuffleObjectsIntoLibraryEffect::new(spec, PlayerFilter::You);

        let moved = game
            .move_object_by_effect(graveyard_card, Zone::Exile)
            .expect("card should move to exile");
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("shuffle should resolve");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "library should still shuffle when the object left the expected zone"
        );
        assert!(
            outcome.events.iter().any(|event| event
                .downcast::<ShuffleLibraryEvent>()
                .is_some_and(|shuffle| { shuffle.player == alice })),
            "shuffle-into-library effects should emit a shuffle event even if nothing moves"
        );
        assert_eq!(
            game.object(moved).expect("moved card").zone,
            Zone::Exile,
            "object should remain in its new zone rather than being moved back into library"
        );
    }

    #[test]
    fn owner_library_destination_shuffles_each_affected_owner_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        create_card_in_zone(&mut game, alice, Zone::Graveyard, "Alice One");
        create_card_in_zone(&mut game, alice, Zone::Graveyard, "Alice Two");
        create_card_in_zone(&mut game, bob, Zone::Graveyard, "Bob One");
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ShuffleObjectsIntoLibraryEffect::new(
            ChooseSpec::All(ObjectFilter::creature().in_zone(Zone::Graveyard)),
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Target),
        )
        .with_owner_library_destination();
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("owner-library shuffle should resolve");

        let shuffle_players = outcome
            .events
            .iter()
            .filter_map(|event| {
                event
                    .downcast::<ShuffleLibraryEvent>()
                    .map(|shuffle| shuffle.player)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            shuffle_players
                .iter()
                .filter(|player| **player == alice)
                .count(),
            1,
            "multiple objects owned by one player should cause one shuffle"
        );
        assert_eq!(
            shuffle_players
                .iter()
                .filter(|player| **player == bob)
                .count(),
            1,
            "each affected owner should shuffle their own library"
        );
        assert_eq!(shuffle_players.len(), 2);
        assert_eq!(game.player(alice).expect("Alice").graveyard.len(), 0);
        assert_eq!(game.player(bob).expect("Bob").graveyard.len(), 0);
        assert_eq!(game.player(alice).expect("Alice").library.len(), 2);
        assert_eq!(game.player(bob).expect("Bob").library.len(), 1);
    }
}

#[cfg(test)]
#[path = "shuffle_objects_into_library_tests.rs"]
mod transaction_tests;
