use crate::effects::ExecutionContext;
use crate::effects::ExecutionError;
use crate::effects::helpers::resolve_value;
use crate::events::EnterBattlefieldEvent;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Controller policy when an object enters the battlefield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryController {
    Preserve,
    Owner,
    Specific(PlayerId),
}

/// Config for moving an object to the battlefield through ETB processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BattlefieldEntryOptions {
    pub controller: BattlefieldEntryController,
    pub tapped: bool,
    pub initial_counters: Vec<(crate::object::CounterType, u32)>,
}

impl BattlefieldEntryOptions {
    pub(crate) fn preserve(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Preserve,
            tapped,
            initial_counters: Vec::new(),
        }
    }

    pub(crate) fn owner(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Owner,
            tapped,
            initial_counters: Vec::new(),
        }
    }

    pub(crate) fn specific(controller: PlayerId, tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Specific(controller),
            tapped,
            initial_counters: Vec::new(),
        }
    }

    pub(crate) fn with_initial_counters(
        mut self,
        counters: Vec<(crate::object::CounterType, u32)>,
    ) -> Self {
        self.initial_counters = counters;
        self
    }
}

/// Resolve authored one-shot entry-counter metadata before the object changes
/// zones. The resulting concrete counters are passed into ETB replacement
/// processing as part of the original enter event.
pub(crate) fn resolve_battlefield_entry_counters(
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: ObjectId,
    specs: &[ironsmith_core::BattlefieldEntryCounterSpec],
) -> Result<Vec<(crate::object::CounterType, u32)>, ExecutionError> {
    let mut counters = Vec::new();
    for spec in specs {
        if let Some(condition) = &spec.condition
            && !crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)?
        {
            continue;
        }
        if let Some(filter) = &spec.object_filter {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            let filter_ctx = ctx.filter_context(game);
            if !filter.matches(object, &filter_ctx, game) {
                continue;
            }
        }
        let amount = resolve_value(game, &spec.amount, ctx)?.max(0) as u32;
        if amount > 0 {
            counters.push((spec.counter_type, amount));
        }
    }
    Ok(counters)
}

/// Result for a move-to-battlefield attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryOutcome {
    Moved(ObjectId),
    Prevented,
}

#[derive(Debug, Clone, Copy)]
struct ReservedZonePosition {
    object: ObjectId,
    zone: Zone,
    owner: PlayerId,
    index: usize,
}

fn reserve_object_zone_position(
    game: &mut GameState,
    object: ObjectId,
) -> Option<ReservedZonePosition> {
    let current = game.object(object)?;
    let zone = current.zone;
    let owner = current.owner;
    let index = match zone {
        Zone::Battlefield => game.battlefield.iter().position(|id| *id == object),
        Zone::Command => game.command_zone.iter().position(|id| *id == object),
        Zone::Exile => game.exile.iter().position(|id| *id == object),
        Zone::Ante => game.ante.iter().position(|id| *id == object),
        Zone::Library => game
            .player(owner)
            .and_then(|player| player.library.iter().position(|id| *id == object)),
        Zone::Hand => game
            .player(owner)
            .and_then(|player| player.hand.iter().position(|id| *id == object)),
        Zone::Graveyard => game
            .player(owner)
            .and_then(|player| player.graveyard.iter().position(|id| *id == object)),
        Zone::OutsideGame => game
            .player(owner)
            .and_then(|player| player.sideboard.iter().position(|id| *id == object)),
        Zone::Stack => None,
    }?;

    match zone {
        Zone::Battlefield => game.battlefield.remove(index),
        Zone::Command => game.command_zone.remove(index),
        Zone::Exile => game.exile.remove(index),
        Zone::Ante => game.ante.remove(index),
        Zone::Library => {
            game.reserve_library_object_position(owner, object)?;
            object
        }
        Zone::Hand => game.player_mut(owner)?.hand.remove(index),
        Zone::Graveyard => game.player_mut(owner)?.graveyard.remove(index),
        Zone::OutsideGame => game.player_mut(owner)?.sideboard.remove(index),
        Zone::Stack => return None,
    };
    Some(ReservedZonePosition {
        object,
        zone,
        owner,
        index,
    })
}

fn restore_reserved_zone_position(game: &mut GameState, reservation: ReservedZonePosition) {
    if game
        .object(reservation.object)
        .is_none_or(|object| object.zone != reservation.zone)
    {
        return;
    }

    let insert = |objects: &mut Vec<ObjectId>| {
        if !objects.contains(&reservation.object) {
            objects.insert(reservation.index.min(objects.len()), reservation.object);
        }
    };
    match reservation.zone {
        Zone::Battlefield => insert(&mut game.battlefield),
        Zone::Command => insert(&mut game.command_zone),
        Zone::Exile => insert(&mut game.exile),
        Zone::Ante => insert(&mut game.ante),
        Zone::Library => {
            game.restore_library_object_position(
                reservation.owner,
                reservation.object,
                reservation.index,
            );
        }
        Zone::Hand => {
            if let Some(player) = game.player_mut(reservation.owner) {
                insert(&mut player.hand);
            }
        }
        Zone::Graveyard => {
            if let Some(player) = game.player_mut(reservation.owner) {
                insert(&mut player.graveyard);
            }
        }
        Zone::OutsideGame => {
            if let Some(player) = game.player_mut(reservation.owner) {
                insert(&mut player.sideboard);
            }
        }
        Zone::Stack => {}
    }
}

fn battlefield_entry_controller(
    game: &GameState,
    object: ObjectId,
    options: &BattlefieldEntryOptions,
) -> Option<PlayerId> {
    let object = game.object(object)?;
    Some(match options.controller {
        BattlefieldEntryController::Specific(controller) => controller,
        BattlefieldEntryController::Owner => object.owner,
        BattlefieldEntryController::Preserve => game.controller_of(object),
    })
}

fn apnap_position(game: &GameState, player: PlayerId) -> usize {
    game.team_apnap_player_order()
        .iter()
        .position(|candidate| *candidate == player)
        .unwrap_or(usize::MAX)
}

fn finish_battlefield_entry(
    game: &mut GameState,
    ctx: &ExecutionContext,
    old_zone: Zone,
    options: &BattlefieldEntryOptions,
    result: crate::game_state::EntersResult,
) -> BattlefieldEntryOutcome {
    let new_id = result.new_id;
    if game
        .object(new_id)
        .is_none_or(|object| object.zone != Zone::Battlefield)
    {
        return BattlefieldEntryOutcome::Prevented;
    }

    game.add_battlefield_put_with_source_link(ctx.source, new_id);
    let enters_tapped = result.enters_tapped || options.tapped;
    if options.tapped && !result.enters_tapped {
        game.tap(new_id);
    }

    let event = if enters_tapped {
        TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::tapped(new_id, old_zone),
            ProvNodeId::default(),
        )
    } else {
        TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(new_id, old_zone),
            ProvNodeId::default(),
        )
    };
    game.queue_trigger_event(ctx.provenance, event);
    BattlefieldEntryOutcome::Moved(new_id)
}

/// Prepare and commit a simultaneous group of battlefield entries.
///
/// Replacement choices are gathered in APNAP order while every entering card
/// remains reserved outside all zone indexes. This gives each proposal the same
/// pre-entry battlefield, prevents another entrant (or an object already chosen
/// by an entry replacement) from being selected again, and keeps combined costs
/// visible to later choices. The fully prepared entries are then committed on a
/// clone and published as one state transition.
pub(crate) fn move_to_battlefield_batch_with_options(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requests: Vec<(ObjectId, BattlefieldEntryOptions)>,
) -> Vec<BattlefieldEntryOutcome> {
    if requests.is_empty() {
        return Vec::new();
    }

    let mut working = game.clone();
    let eligible_indices = requests
        .iter()
        .enumerate()
        .filter_map(|(index, (object, options))| {
            if working.card_cannot_enter_battlefield(*object) {
                return None;
            }
            battlefield_entry_controller(&working, *object, options)
                .filter(|controller| {
                    working
                        .player(*controller)
                        .is_some_and(|player| player.is_in_game())
                })
                .map(|_| index)
        })
        .collect::<std::collections::HashSet<_>>();
    let mut reserved_objects = requests
        .iter()
        .enumerate()
        .filter(|(index, _)| eligible_indices.contains(index))
        .map(|(_, (object, _))| *object)
        .collect::<std::collections::HashSet<_>>();
    let mut reservations = reserved_objects
        .iter()
        .filter_map(|object| reserve_object_zone_position(&mut working, *object))
        .collect::<Vec<_>>();

    let mut proposal_order = eligible_indices.iter().copied().collect::<Vec<_>>();
    proposal_order.sort_by_key(|index| {
        let (object, options) = &requests[*index];
        let controller =
            battlefield_entry_controller(&working, *object, options).unwrap_or(ctx.controller);
        (apnap_position(&working, controller), *index)
    });

    let preparation_order = proposal_order.clone();
    let mut proposals = vec![None; requests.len()];
    for index in proposal_order {
        let (object, options) = &requests[index];
        let Some(old_zone) = working.object(*object).map(|object| object.zone) else {
            continue;
        };
        let entering_controller = match options.controller {
            BattlefieldEntryController::Specific(controller) => Some(controller),
            BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
        };
        let result = crate::events::processing::process_etb_batch_proposal_with_initial_counters(
            &mut working,
            *object,
            old_zone,
            &mut ctx.decision_maker,
            options.initial_counters.clone(),
            entering_controller,
            &reserved_objects,
        );
        for linked in &result.linked_exile_with_entering {
            if reserved_objects.insert(*linked)
                && let Some(reservation) = reserve_object_zone_position(&mut working, *linked)
            {
                reservations.push(reservation);
            }
        }
        proposals[index] = Some((old_zone, result));
    }

    let mut prepared_entries = vec![None; requests.len()];
    for index in preparation_order {
        let Some((old_zone, proposal)) = proposals[index].take() else {
            continue;
        };
        let (object, options) = &requests[index];
        let entering_controller = match options.controller {
            BattlefieldEntryController::Specific(controller) => Some(controller),
            BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
        };
        let Some(prepared) = working.prepare_etb_entry_with_controller_and_dm(
            *object,
            proposal,
            entering_controller,
            &mut ctx.decision_maker,
        ) else {
            return vec![BattlefieldEntryOutcome::Prevented; requests.len()];
        };
        if ctx.decision_maker.awaiting_choice() {
            return vec![BattlefieldEntryOutcome::Prevented; requests.len()];
        }
        prepared_entries[index] = Some((old_zone, prepared));
    }

    // Reverse removal order restores the original relative order even when
    // several reserved cards occupied adjacent slots in an ordered zone.
    for reservation in reservations.into_iter().rev() {
        restore_reserved_zone_position(&mut working, reservation);
    }

    let mut outcomes = vec![BattlefieldEntryOutcome::Prevented; requests.len()];
    for (index, (object, options)) in requests.iter().enumerate() {
        let Some((old_zone, prepared_entry)) = prepared_entries[index].take() else {
            continue;
        };
        let entering_controller = match options.controller {
            BattlefieldEntryController::Specific(controller) => Some(controller),
            BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
        };
        let Some(result) = working.commit_prepared_etb_with_controller_and_dm(
            *object,
            prepared_entry,
            entering_controller,
            &mut ctx.decision_maker,
        ) else {
            if ctx.decision_maker.awaiting_choice() {
                return vec![BattlefieldEntryOutcome::Prevented; requests.len()];
            }
            continue;
        };
        outcomes[index] = finish_battlefield_entry(&mut working, ctx, old_zone, options, result);
    }

    *game = working;
    outcomes
}

/// Move an object to the battlefield with ETB replacement processing and policy hooks.
pub(crate) fn move_to_battlefield_with_options(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object_id: ObjectId,
    options: BattlefieldEntryOptions,
) -> BattlefieldEntryOutcome {
    if battlefield_entry_controller(game, object_id, &options).is_none_or(|controller| {
        !game
            .player(controller)
            .is_some_and(|player| player.is_in_game())
    }) {
        // CR 800.4b: an object that would enter under the control of a player
        // who left remains in its current zone.
        return BattlefieldEntryOutcome::Prevented;
    }
    let old_zone = game.object(object_id).map(|obj| obj.zone);
    let entering_controller = match options.controller {
        BattlefieldEntryController::Specific(controller) => Some(controller),
        BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
    };
    let Some(result) = game
        .move_object_with_etb_processing_with_initial_counters_and_controller_with_dm(
            object_id,
            Zone::Battlefield,
            options.initial_counters.clone(),
            entering_controller,
            &mut ctx.decision_maker,
        )
    else {
        return BattlefieldEntryOutcome::Prevented;
    };

    let Some(old_zone) = old_zone else {
        return BattlefieldEntryOutcome::Prevented;
    };
    finish_battlefield_entry(game, ctx, old_zone, &options, result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{ColorsContext, SelectObjectsContext};
    use crate::ids::CardId;
    use crate::object::{AttachmentTarget, AuraAttachmentFilter, Object};
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(id, &card, owner, Zone::Battlefield));
        id
    }

    fn create_hand_aura(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        filter: ObjectFilter,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let mut object = Object::from_card(id, &card, owner, Zone::Hand);
        object.aura_attach_filter = Some(AuraAttachmentFilter::from(filter).into());
        game.add_object(object);
        id
    }

    struct ChooseObjectDm {
        desired: ObjectId,
        chooser: Option<PlayerId>,
    }

    impl DecisionMaker for ChooseObjectDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.chooser = Some(ctx.player);
            ctx.candidates
                .iter()
                .any(|candidate| candidate.id == self.desired)
                .then_some(vec![self.desired])
                .unwrap_or_default()
        }
    }

    fn create_color_chooser(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let mut object = Object::from_card(id, &card, owner, zone);
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ));
        game.add_object(object);
        id
    }

    struct InspectColorChoiceDm {
        color: Color,
        awaiting: bool,
        sources: Vec<Option<ObjectId>>,
        source_zones: Vec<Option<Zone>>,
        battlefield_sizes: Vec<usize>,
    }

    impl InspectColorChoiceDm {
        fn synchronous(color: Color) -> Self {
            Self {
                color,
                awaiting: false,
                sources: Vec::new(),
                source_zones: Vec::new(),
                battlefield_sizes: Vec::new(),
            }
        }
    }

    impl DecisionMaker for InspectColorChoiceDm {
        fn awaiting_choice(&self) -> bool {
            self.awaiting
        }

        fn decide_colors(&mut self, game: &GameState, ctx: &ColorsContext) -> Vec<Color> {
            self.sources.push(ctx.source);
            self.source_zones.push(
                ctx.source
                    .and_then(|source| game.object(source).map(|object| object.zone)),
            );
            self.battlefield_sizes.push(game.battlefield.len());
            vec![self.color]
        }
    }

    #[test]
    fn as_enters_choice_is_requested_before_the_destination_object_is_committed() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = create_color_chooser(&mut game, "Chromatic Reliquary", alice, Zone::Hand);
        let mut dm = InspectColorChoiceDm::synchronous(Color::Blue);

        let result = game
            .move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm)
            .expect("the permanent should enter after its choice is complete");

        assert_eq!(dm.sources, vec![Some(entrant)]);
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand)]);
        assert_eq!(dm.battlefield_sizes, vec![0]);
        assert_eq!(game.chosen_color(result.new_id), Some(Color::Blue));
    }

    #[test]
    fn instant_card_is_rejected_before_etb_replacements_or_choices_are_proposed() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(entrant.0 as u32), "Impossible Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let mut object = Object::from_card(entrant, &card, alice, Zone::Hand);
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ));
        game.add_object(object);
        let mut dm = InspectColorChoiceDm::synchronous(Color::Blue);

        let result =
            game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm);

        assert!(result.is_none());
        assert!(dm.sources.is_empty(), "no ETB choice should be proposed");
        assert_eq!(
            game.object(entrant).expect("instant remains").zone,
            Zone::Hand
        );
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn suspended_as_enters_choice_keeps_the_whole_entry_uncommitted() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = create_color_chooser(&mut game, "Pending Reliquary", alice, Zone::Hand);
        let mut dm = InspectColorChoiceDm {
            awaiting: true,
            ..InspectColorChoiceDm::synchronous(Color::Red)
        };

        let result =
            game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm);

        assert!(result.is_none());
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand)]);
        assert_eq!(
            game.object(entrant).map(|object| object.zone),
            Some(Zone::Hand)
        );
        assert!(game.battlefield.is_empty());
        assert!(game.chosen_color(entrant).is_none());
    }

    #[test]
    fn copied_as_enters_abilities_are_used_by_the_prospective_choice_record() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let copy_source =
            create_color_chooser(&mut game, "Chromatic Blueprint", alice, Zone::Battlefield);
        let entrant_id = game.new_object_id();
        let entrant_card =
            CardBuilder::new(CardId::from_raw(entrant_id.0 as u32), "Unfinished Replica")
                .card_types(vec![CardType::Artifact])
                .build();
        game.add_object(Object::from_card(
            entrant_id,
            &entrant_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = InspectColorChoiceDm::synchronous(Color::Black);
        let proposal = crate::events::processing::EtbEventResult {
            enters_as_copy_of: Some(copy_source),
            ..Default::default()
        };

        let prepared = game
            .prepare_etb_entry_with_controller_and_dm(entrant_id, proposal, None, &mut dm)
            .expect("copy-derived entry choice should be prepared");
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand)]);
        let result = game
            .commit_prepared_etb_with_controller_and_dm(entrant_id, prepared, None, &mut dm)
            .expect("the prepared copy should enter");

        assert_eq!(game.chosen_color(result.new_id), Some(Color::Black));
        assert_eq!(
            game.object(result.new_id)
                .map(|object| object.name.as_ref()),
            Some("Chromatic Blueprint")
        );
    }

    #[test]
    fn simultaneous_as_enters_choices_are_all_collected_before_batch_commit() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let first = create_color_chooser(&mut game, "First Prism", alice, Zone::Hand);
        let second = create_color_chooser(&mut game, "Second Prism", alice, Zone::Hand);
        let mut dm = InspectColorChoiceDm::synchronous(Color::Green);
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(9_031), alice, &mut dm);

        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (first, BattlefieldEntryOptions::preserve(false)),
                (second, BattlefieldEntryOptions::preserve(false)),
            ],
        );

        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome, BattlefieldEntryOutcome::Moved(_)))
        );
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand), Some(Zone::Hand)]);
        assert_eq!(dm.battlefield_sizes, vec![0, 0]);
    }

    #[test]
    fn simultaneous_entry_batch_skips_instant_without_blocking_legal_permanent() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let instant_id = game.new_object_id();
        let instant_card = CardBuilder::new(
            CardId::from_raw(instant_id.0 as u32),
            "Batch Impossible Instant",
        )
        .card_types(vec![CardType::Instant])
        .build();
        game.add_object(Object::from_card(
            instant_id,
            &instant_card,
            alice,
            Zone::Hand,
        ));
        let permanent_id = game.new_object_id();
        let permanent_card =
            CardBuilder::new(CardId::from_raw(permanent_id.0 as u32), "Batch Bear")
                .card_types(vec![CardType::Creature])
                .build();
        game.add_object(Object::from_card(
            permanent_id,
            &permanent_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = InspectColorChoiceDm::synchronous(Color::Green);
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(9_032), alice, &mut dm);

        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (instant_id, BattlefieldEntryOptions::preserve(false)),
                (permanent_id, BattlefieldEntryOptions::preserve(false)),
            ],
        );

        assert!(matches!(outcomes[0], BattlefieldEntryOutcome::Prevented));
        assert!(matches!(outcomes[1], BattlefieldEntryOutcome::Moved(_)));
        assert_eq!(
            game.object(instant_id).expect("instant remains").zone,
            Zone::Hand
        );
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Batch Bear")
        }));
    }

    #[test]
    fn discard_hand_as_enters_finishes_before_the_battlefield_zone_change() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant_id = game.new_object_id();
        let entrant_card = CardBuilder::new(CardId::from_raw(entrant_id.0 as u32), "Memory Eraser")
            .card_types(vec![CardType::Artifact])
            .build();
        let mut entrant = Object::from_card(entrant_id, &entrant_card, alice, Zone::Hand);
        entrant.abilities_mut().push(Ability::static_ability(
            StaticAbility::discard_hand_as_enters("As this enters, discard your hand.".into()),
        ));
        game.add_object(entrant);

        let discarded_id = game.new_object_id();
        let discarded_card =
            CardBuilder::new(CardId::from_raw(discarded_id.0 as u32), "Forgotten Card").build();
        game.add_object(Object::from_card(
            discarded_id,
            &discarded_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let entered = game
            .move_object_with_etb_processing_with_dm(entrant_id, Zone::Battlefield, &mut dm)
            .expect("the permanent should enter after discarding the rest of the hand");
        let discarded_new_id = game
            .player(alice)
            .and_then(|player| player.graveyard.first().copied())
            .expect("the other hand card should be discarded");
        let zone_change_objects = game
            .take_pending_trigger_events()
            .into_iter()
            .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
            .filter_map(|event| event.object_id())
            .collect::<Vec<_>>();

        assert_eq!(zone_change_objects, vec![discarded_new_id, entered.new_id]);
    }

    struct RevealFromHandDm {
        desired: ObjectId,
        choice_source_zone: Option<Zone>,
        view_source_zones: Vec<Option<Zone>>,
        viewed_cards: Vec<Vec<ObjectId>>,
    }

    impl DecisionMaker for RevealFromHandDm {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.choice_source_zone = ctx
                .source
                .and_then(|source| game.object(source).map(|object| object.zone));
            vec![self.desired]
        }

        fn view_cards(
            &mut self,
            game: &GameState,
            _viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.view_source_zones.push(
                ctx.source
                    .and_then(|source| game.object(source).map(|object| object.zone)),
            );
            self.viewed_cards.push(cards.to_vec());
        }
    }

    #[test]
    fn reveal_from_hand_as_enters_is_selected_and_published_before_commit() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant_id = game.new_object_id();
        let entrant_card =
            CardBuilder::new(CardId::from_raw(entrant_id.0 as u32), "Revealing Reliquary")
                .card_types(vec![CardType::Artifact])
                .build();
        let mut entrant = Object::from_card(entrant_id, &entrant_card, alice, Zone::Hand);
        entrant.abilities_mut().push(Ability::static_ability(
            StaticAbility::reveal_from_hand_as_enters(
                ObjectFilter::creature().in_zone(Zone::Hand),
                crate::ChoiceCount::any_number(),
                true,
                "As this enters, you may reveal any number of creature cards from your hand."
                    .into(),
            ),
        ));
        game.add_object(entrant);

        let creature_id = game.new_object_id();
        let creature_card =
            CardBuilder::new(CardId::from_raw(creature_id.0 as u32), "Revealed Bear")
                .card_types(vec![CardType::Creature])
                .build();
        game.add_object(Object::from_card(
            creature_id,
            &creature_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = RevealFromHandDm {
            desired: creature_id,
            choice_source_zone: None,
            view_source_zones: Vec::new(),
            viewed_cards: Vec::new(),
        };

        game.move_object_with_etb_processing_with_dm(entrant_id, Zone::Battlefield, &mut dm)
            .expect("the revealing permanent should enter");

        assert_eq!(dm.choice_source_zone, Some(Zone::Hand));
        assert_eq!(
            dm.view_source_zones,
            vec![Some(Zone::Hand), Some(Zone::Hand)]
        );
        assert_eq!(dm.viewed_cards, vec![vec![creature_id], vec![creature_id]]);
    }

    #[test]
    fn nonspell_aura_entry_uses_the_entering_controller_to_choose_attachment() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let _alice_creature = create_creature(&mut game, "Alice Bear", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let aura = create_hand_aura(
            &mut game,
            "Borrowed Blessing",
            alice,
            ObjectFilter::creature().you_control(),
        );
        let mut dm = ChooseObjectDm {
            desired: bob_creature,
            chooser: None,
        };

        let outcome = {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(9000), bob, &mut dm);
            move_to_battlefield_with_options(
                &mut game,
                &mut ctx,
                aura,
                BattlefieldEntryOptions::specific(bob, false),
            )
        };

        let BattlefieldEntryOutcome::Moved(new_id) = outcome else {
            panic!("Aura should enter attached");
        };
        assert_eq!(dm.chooser, Some(bob));
        assert_eq!(game.current_controller(new_id), Some(bob));
        assert_eq!(
            game.object(new_id).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(bob_creature))
        );
    }

    #[test]
    fn nonspell_aura_with_no_legal_attachment_remains_in_its_current_zone() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let aura = create_hand_aura(
            &mut game,
            "Lonely Blessing",
            alice,
            ObjectFilter::creature(),
        );
        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(9001), alice);

        let outcome = move_to_battlefield_with_options(
            &mut game,
            &mut ctx,
            aura,
            BattlefieldEntryOptions::specific(alice, false),
        );

        assert_eq!(outcome, BattlefieldEntryOutcome::Prevented);
        assert_eq!(
            game.object(aura).map(|object| object.zone),
            Some(Zone::Hand)
        );
        assert!(
            game.player(alice)
                .is_some_and(|player| player.hand.contains(&aura))
        );
        assert!(game.battlefield.iter().all(|id| *id != aura));
        assert!(
            game.player(alice)
                .is_some_and(|player| player.graveyard.is_empty())
        );
    }

    #[test]
    fn simultaneous_entrants_are_reserved_from_as_enters_zone_change_choices() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let mox_id = game.new_object_id();
        let mox = CardBuilder::new(CardId::from_raw(mox_id.0 as u32), "Batch Mox")
            .card_types(vec![CardType::Artifact])
            .build();
        let mut mox = Object::from_card(mox_id, &mox, alice, Zone::Hand);
        mox.abilities_mut().push(Ability::static_ability(
            StaticAbility::discard_or_redirect_replacement(
                ObjectFilter::land().in_zone(Zone::Hand),
                Zone::Graveyard,
            ),
        ));
        game.add_object(mox);

        let land_id = game.new_object_id();
        let land = CardBuilder::new(CardId::from_raw(land_id.0 as u32), "Entering Land")
            .card_types(vec![CardType::Land])
            .build();
        game.add_object(Object::from_card(land_id, &land, alice, Zone::Hand));

        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(9002), alice);
        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (mox_id, BattlefieldEntryOptions::preserve(false)),
                (land_id, BattlefieldEntryOptions::preserve(false)),
            ],
        );

        assert_eq!(outcomes[0], BattlefieldEntryOutcome::Prevented);
        assert!(matches!(outcomes[1], BattlefieldEntryOutcome::Moved(_)));
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Entering Land")
        }));
        assert!(game.player(alice).is_some_and(|player| {
            player.graveyard.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Batch Mox")
            })
        }));
    }

    #[test]
    fn multiplayer_800_4b_keeps_object_in_zone_when_entering_controller_left() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(90_003), "Stranded Permanent")
            .card_types(vec![CardType::Artifact])
            .build();
        game.add_object(Object::from_card(card_id, &card, bob, Zone::Graveyard));
        game.player_mut(alice).expect("Alice").has_left_game = true;
        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(9003), bob);

        let outcome = move_to_battlefield_with_options(
            &mut game,
            &mut ctx,
            card_id,
            BattlefieldEntryOptions::specific(alice, false),
        );

        assert_eq!(outcome, BattlefieldEntryOutcome::Prevented);
        assert_eq!(
            game.object(card_id).map(|object| object.zone),
            Some(Zone::Graveyard)
        );
    }
}
