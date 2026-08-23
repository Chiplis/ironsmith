use std::collections::HashSet;

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::events::BeginningOfUpkeepEvent;
use ironsmith::game_loop::put_triggers_on_stack_with_dm;
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::{
    Trigger, TriggerEvent, TriggerQueue, TriggeredAbilityEntry, TriggeredAbilitySourceKind,
    compute_trigger_identity,
};
use ironsmith::{
    AttackDirection, CardId, CardType, FreeForAllAttackOption, GameState, GrandMeleeMarkerStatus,
    PlanarCardKind, PlayerId, ResolutionProgram, StackEntry, TriggeredAbility, Zone,
};

fn game_with_players(count: usize) -> (GameState, Vec<PlayerId>) {
    let players = (0..count)
        .map(|index| PlayerId::from_index(index as u8))
        .collect::<Vec<_>>();
    let game = GameState::new(
        (0..count).map(|index| format!("Player {index}")).collect(),
        20,
    );
    (game, players)
}

fn stack_object(game: &mut GameState, controller: PlayerId) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Grand Melee Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Stack)
}

fn planar_deck(owner: usize) -> Vec<(ironsmith::cards::CardDefinition, PlanarCardKind)> {
    (0..10)
        .map(|index| {
            (
                CardDefinitionBuilder::new(
                    CardId::new(),
                    format!("Grand Melee Plane {owner}-{index}"),
                )
                .card_types(vec![CardType::Plane])
                .build(),
                PlanarCardKind::Plane,
            )
        })
        .collect()
}

fn queued_upkeep_trigger(
    game: &GameState,
    source: ironsmith::ObjectId,
    controller: PlayerId,
    triggering_event: TriggerEvent,
) -> TriggeredAbilityEntry {
    let ability = TriggeredAbility {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: ResolutionProgram::default(),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        event_value_amount: None,
        ability: ability.clone(),
        triggering_event,
        source_stable_id: game.object(source).expect("trigger source").stable_id,
        source_name: "Grand Melee Trigger".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: TriggeredAbilitySourceKind::Object,
        trigger_identity: compute_trigger_identity(&ability),
    }
}

#[derive(Debug)]
struct ChooseSecondGrandMeleeStack {
    stack_prompts: usize,
}

impl DecisionMaker for ChooseSecondGrandMeleeStack {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose the Grand Melee stack") {
            self.stack_prompts += 1;
            vec![1]
        } else {
            vec![0]
        }
    }
}

#[test]
fn u072_profile_numbers_simultaneous_markers_four_seats_apart() {
    let (mut game, players) = game_with_players(10);
    game.restore_grand_melee(players.clone())
        .expect("fixed synchronized Grand Melee seats");

    let state = game.grand_melee().expect("Grand Melee state");
    assert_eq!(state.seats(), players);
    assert_eq!(state.starting_player_count(), 10);
    assert_eq!(state.marker_count(), 2);
    assert_eq!(state.focused_marker(), 1);
    assert_eq!(
        game.free_for_all().unwrap().attack_option(),
        FreeForAllAttackOption::Left
    );
    assert_eq!(game.free_for_all().unwrap().range_of_influence(), Some(1));
    assert_eq!(game.attack_direction(), Some(AttackDirection::Left));
    assert!(!game.deploy_creatures_enabled());

    let markers = game.grand_melee_marker_views();
    assert_eq!(markers.len(), 2);
    assert_eq!((markers[0].number, markers[0].holder), (1, players[0]));
    assert_eq!((markers[1].number, markers[1].holder), (2, players[4]));
    assert!(
        markers
            .iter()
            .all(|marker| marker.status == GrandMeleeMarkerStatus::Active)
    );
    assert_eq!(
        game.active_players().into_iter().collect::<HashSet<_>>(),
        HashSet::from([players[0], players[4]])
    );
    assert_eq!(game.turn_players(), vec![players[0]]);
}

#[test]
fn u072_each_marker_owns_a_stack_and_priority_is_range_gated() {
    let (mut game, players) = game_with_players(10);
    game.restore_grand_melee(players.clone()).unwrap();

    let first_spell = stack_object(&mut game, players[2]);
    game.push_to_stack(StackEntry::new(first_spell, players[2]));
    assert_eq!(game.stack.len(), 1);
    assert_eq!(
        game.grand_melee_priority_players_for(1),
        vec![players[0], players[1], players[2], players[3], players[9]]
    );
    assert!(
        game.select_grand_melee_stack_for_player(players[5], 1)
            .is_err()
    );

    game.select_grand_melee_stack_for_player(players[5], 2)
        .expect("marker two and its holder are in player five's range");
    assert!(game.stack.is_empty(), "marker two has a distinct stack");
    let second_spell = stack_object(&mut game, players[5]);
    game.push_to_stack(StackEntry::new(second_spell, players[5]));
    assert!(
        ironsmith::targeting::can_target_object(&game, first_spell, second_spell, players[5],)
            .is_invalid(),
        "a stack object on another marker cannot be targeted from this stack",
    );
    assert!(
        ironsmith::targeting::can_target_object(&game, second_spell, second_spell, players[5],)
            .is_legal(),
        "objects on the selected marker's own stack remain targetable",
    );

    game.select_grand_melee_turn_marker(1).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].object_id, first_spell);
    let views = game.grand_melee_marker_views();
    assert_eq!(views[0].stack_size, 1);
    assert_eq!(views[1].stack_size, 1);
}

#[test]
fn u072_unbound_triggers_choose_a_stack_but_stack_causes_force_their_own_lane() {
    let (mut game, players) = game_with_players(10);
    game.restore_grand_melee(players.clone()).unwrap();

    // A marker-one stack object controlled from seat 2 expands priority there
    // far enough that seat 3 has priority for both marker stacks.
    let range_spell = stack_object(&mut game, players[2]);
    game.push_to_stack(StackEntry::new(range_spell, players[2]));
    assert_eq!(
        game.grand_melee_priority_markers_for(players[3]),
        vec![1, 2]
    );

    let source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Grand Melee Trigger")
            .card_types(vec![CardType::Enchantment])
            .build(),
        players[3],
        Zone::Battlefield,
    );
    let mut queue = TriggerQueue::new();
    queue.add(queued_upkeep_trigger(
        &game,
        source,
        players[3],
        TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(players[0]),
            ironsmith::ProvNodeId::default(),
        ),
    ));
    let mut chooser = ChooseSecondGrandMeleeStack { stack_prompts: 0 };
    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut chooser).unwrap();
    assert_eq!(chooser.stack_prompts, 1);
    assert_eq!(game.grand_melee().unwrap().focused_marker(), 1);
    assert_eq!(game.grand_melee_marker_views()[0].stack_size, 1);
    assert_eq!(game.grand_melee_marker_views()[1].stack_size, 1);
    game.select_grand_melee_turn_marker(2).unwrap();
    assert_eq!(
        game.stack
            .last()
            .and_then(|entry| entry.source_name.as_deref()),
        Some("Grand Melee Trigger")
    );

    // A descendant event of marker one's stack object is forced back to marker
    // one and does not offer the otherwise available destination choice.
    game.select_grand_melee_turn_marker(1).unwrap();
    let cause_provenance = game
        .provenance_graph_mut()
        .alloc_root_event(ironsmith::events::EventKind::SpellCast);
    let causal_spell = stack_object(&mut game, players[2]);
    game.push_to_stack(StackEntry::new(causal_spell, players[2]).with_provenance(cause_provenance));
    let event_provenance = game.provenance_graph_mut().alloc_child_event(
        cause_provenance,
        ironsmith::events::EventKind::BeginningOfUpkeep,
    );
    let mut forced_queue = TriggerQueue::new();
    forced_queue.add(queued_upkeep_trigger(
        &game,
        source,
        players[3],
        TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(players[0]),
            event_provenance,
        ),
    ));
    let mut forced_chooser = ChooseSecondGrandMeleeStack { stack_prompts: 0 };
    put_triggers_on_stack_with_dm(&mut game, &mut forced_queue, &mut forced_chooser).unwrap();
    assert_eq!(forced_chooser.stack_prompts, 0);
    assert_eq!(game.grand_melee_marker_views()[0].stack_size, 3);
    assert_eq!(game.grand_melee_marker_views()[1].stack_size, 1);
}

#[test]
fn u072_marker_waits_until_the_other_marker_moves_four_seats_left() {
    let (mut game, players) = game_with_players(8);
    game.restore_grand_melee(players.clone()).unwrap();

    game.next_turn();
    let markers = game.grand_melee_marker_views();
    assert_eq!(markers[0].holder, players[1]);
    assert_eq!(markers[0].status, GrandMeleeMarkerStatus::Waiting);
    assert_eq!(game.grand_melee().unwrap().focused_marker(), 2);

    game.next_turn();
    let markers = game.grand_melee_marker_views();
    assert_eq!(markers[0].holder, players[1]);
    assert_eq!(markers[0].status, GrandMeleeMarkerStatus::Active);
    assert_eq!(markers[1].holder, players[5]);
    assert_eq!(markers[1].status, GrandMeleeMarkerStatus::Active);
}

#[test]
fn u072_extra_turn_stays_with_a_spaced_marker_and_departure_reduces_marker_count() {
    let (mut game, players) = game_with_players(8);
    game.restore_grand_melee(players.clone()).unwrap();

    game.turn_store.extra_turns.push(players[0]);
    game.next_turn();
    assert_eq!(game.grand_melee_marker_views()[0].holder, players[0]);
    assert_eq!(game.turn.active_player, players[0]);

    assert!(game.leave_game(players[7]));
    let marker_two = game
        .grand_melee_marker_views()
        .into_iter()
        .find(|marker| marker.number == 2)
        .expect("active marker is designated, not removed immediately");
    assert_eq!(marker_two.removal_designations, 1);
    game.select_grand_melee_turn_marker(2).unwrap();
    game.next_turn();
    assert_eq!(game.grand_melee().unwrap().marker_count(), 1);
    assert_eq!(game.grand_melee_marker_views()[0].number, 1);
}

#[test]
fn u072_close_markers_wait_or_defer_extra_turns_on_the_correct_side() {
    let (mut left_game, players) = game_with_players(10);
    left_game.restore_grand_melee(players.clone()).unwrap();
    assert!(left_game.leave_game(players[1]));
    left_game.turn_store.extra_turns.push(players[0]);
    left_game.next_turn();
    let marker_one = &left_game.grand_melee_marker_views()[0];
    assert_eq!(marker_one.status, GrandMeleeMarkerStatus::Waiting);
    assert!(marker_one.retained_extra_turn_waiting);

    let checkpoint = left_game
        .grand_melee_restore_snapshot()
        .expect("Grand Melee restore snapshot");
    left_game
        .restore_grand_melee_snapshot(checkpoint)
        .expect("restore retained-extra waiting state");
    assert!(
        left_game.grand_melee_marker_views()[0].retained_extra_turn_waiting,
        "checkpointing preserves why a marker is waiting",
    );
    left_game.next_turn();
    assert_eq!(left_game.turn.active_player, players[0]);
    assert_eq!(
        left_game.grand_melee_marker_views()[0].status,
        GrandMeleeMarkerStatus::Active,
    );

    let (mut right_game, players) = game_with_players(10);
    right_game.restore_grand_melee(players.clone()).unwrap();
    right_game.select_grand_melee_turn_marker(2).unwrap();
    assert!(right_game.leave_game(players[1]));
    assert_eq!(right_game.grand_melee().unwrap().focused_marker(), 2);
    right_game.turn_store.extra_turns.push(players[4]);
    right_game.next_turn();
    let marker_two = right_game
        .grand_melee_marker_views()
        .into_iter()
        .find(|marker| marker.number == 2)
        .unwrap();
    assert_eq!(marker_two.holder, players[5]);
    assert!(
        right_game
            .grand_melee_restore_snapshot()
            .unwrap()
            .deferred_extra_turns
            .contains(&(players[4], 1)),
        "a marker too close on the right passes and defers the extra turn",
    );
}

#[test]
fn u072_departure_adjacency_is_frozen_until_each_markers_next_turn() {
    let (mut game, players) = game_with_players(10);
    game.restore_grand_melee(players.clone()).unwrap();
    assert!(!game.player_is_within_range(players[0], players[2]));

    assert!(game.leave_game(players[1]));
    assert!(
        !game.player_is_within_range(players[0], players[2]),
        "new neighbors do not enter the current marker's frozen range"
    );

    game.next_turn();
    game.next_turn();
    game.next_turn();
    game.select_grand_melee_turn_marker(1).unwrap();
    assert!(
        game.player_is_within_range(players[0], players[2]),
        "the new adjacency appears when marker one's next turn begins"
    );
}

#[test]
fn u072_multiple_removal_designations_cascade_to_the_marker_on_the_right() {
    let (mut game, players) = game_with_players(12);
    game.restore_grand_melee(players.clone()).unwrap();

    for departed in [players[1], players[5], players[6], players[9], players[2]] {
        assert!(game.leave_game(departed));
    }
    let marker_one = game
        .grand_melee_marker_views()
        .into_iter()
        .find(|marker| marker.number == 1)
        .unwrap();
    assert_eq!(marker_one.removal_designations, 2);

    game.select_grand_melee_turn_marker(1).unwrap();
    game.next_turn();
    assert_eq!(game.grand_melee().unwrap().marker_count(), 2);
    let marker_three = game
        .grand_melee_marker_views()
        .into_iter()
        .find(|marker| marker.number == 3)
        .expect("the marker immediately right of removed marker one remains");
    assert_eq!(marker_three.removal_designations, 1);
}

#[test]
fn u072_simultaneous_departures_choose_the_lowest_numbered_eligible_marker() {
    let (mut game, players) = game_with_players(12);
    game.restore_grand_melee(players.clone()).unwrap();

    let departed = game.mark_players_lost_simultaneously(&[
        players[1],
        players[5],
        players[6],
        players[9],
        players[10],
    ]);
    assert_eq!(departed.len(), 5);
    let markers = game.grand_melee_marker_views();
    assert_eq!(markers[0].number, 1);
    assert_eq!(markers[0].removal_designations, 2);
    assert_eq!(markers[1].removal_designations, 0);
    assert_eq!(markers[2].removal_designations, 0);
}

#[test]
fn u072_designated_marker_that_has_not_begun_is_removed_immediately() {
    let (mut game, players) = game_with_players(8);
    game.restore_grand_melee(players.clone()).unwrap();
    game.next_turn();
    assert_eq!(
        game.grand_melee_marker_views()[0].status,
        GrandMeleeMarkerStatus::Waiting,
    );

    assert!(game.leave_game(players[2]));
    assert_eq!(game.grand_melee().unwrap().marker_count(), 1);
    assert_eq!(game.grand_melee_marker_views()[0].number, 2);
}

#[test]
fn u072_planechase_has_one_controller_and_starting_plane_per_initial_marker() {
    let (mut game, players) = game_with_players(8);
    game.restore_grand_melee(players.clone()).unwrap();
    game.enable_planechase(
        players
            .iter()
            .enumerate()
            .map(|(index, player)| (*player, planar_deck(index)))
            .collect(),
    )
    .unwrap();

    let faces = game.reveal_grand_melee_starting_planes().unwrap();
    assert_eq!(faces.len(), 2);
    assert_eq!(
        game.planar_controllers()
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([players[0], players[4]])
    );
    assert_eq!(game.planar_controller_of_face(faces[0]), Some(players[0]));
    assert_eq!(game.planar_controller_of_face(faces[1]), Some(players[4]));
    assert_eq!(game.controller_of_id(faces[0]), Some(players[0]));
    assert_eq!(game.controller_of_id(faces[1]), Some(players[4]));
    assert!(
        !game.source_is_exempt_from_range(Some(faces[0])),
        "planes lose the ordinary Planechase range exemption in Grand Melee"
    );
    game.planechase
        .as_mut()
        .unwrap()
        .voluntary_rolls_this_turn
        .insert(players[4], 2);
    game.turn_store.extra_turns.push(players[0]);
    game.next_turn();
    assert_eq!(
        game.planar_die_roll_cost(players[4]),
        Some(2),
        "starting an extra turn on one marker does not reset another marker's planar die cost",
    );

    let planeswalks_before = game.planechase.as_ref().unwrap().planeswalk_count;
    assert!(game.leave_game(players[4]));
    assert!(!game.planar_controllers().contains(&players[4]));
    assert!(!game.face_up_planar_objects().contains(&faces[1]));
    assert_eq!(
        game.planechase.as_ref().unwrap().planeswalk_count,
        planeswalks_before,
        "marker-reducing departure bottoms planes without planeswalking"
    );
}

#[test]
fn u072_restart_and_subgame_rebuild_fresh_marker_lanes() {
    let (mut game, players) = game_with_players(8);
    game.restore_grand_melee(players.clone()).unwrap();
    game.restart_game(players[2], &[]);
    let markers = game.grand_melee_marker_views();
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].holder, players[2]);
    assert_eq!(markers[1].holder, players[6]);
    assert!(markers.iter().all(|marker| marker.stack_size == 0));

    game.begin_subgame(None, players[2], Vec::new()).unwrap();
    let child = game.grand_melee().expect("child Grand Melee profile");
    assert_eq!(child.seats(), players);
    assert_eq!(child.marker_count(), 2);
    assert!(
        game.grand_melee_marker_views()
            .iter()
            .all(|marker| marker.stack_size == 0)
    );
}
