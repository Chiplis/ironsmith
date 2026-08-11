use std::collections::HashMap;

use crate::combat_state::CombatState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;

use super::{
    FreeForAllAttackOption, GameState, LimitedRangeOfInfluenceState, StackEntry, TurnState,
    TurnStore,
};

/// Whether a Grand Melee turn marker currently represents a running turn or
/// is waiting for the spacing rule in CR 807.4d/807.4i.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrandMeleeMarkerStatus {
    Active,
    Waiting,
}

/// Public, serialization-friendly summary of one numbered turn marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrandMeleeMarkerView {
    pub number: u32,
    pub holder: PlayerId,
    pub status: GrandMeleeMarkerStatus,
    pub stack_size: usize,
    pub removal_designations: usize,
    pub normal_turn_pending: bool,
    pub retained_extra_turn_waiting: bool,
}

/// Serializable runtime payload used by hosts to restore marker lanes.
#[derive(Debug, Clone)]
pub struct GrandMeleeMarkerRestore {
    pub number: u32,
    pub holder: PlayerId,
    pub status: GrandMeleeMarkerStatus,
    pub removal_designations: usize,
    pub normal_turn_pending: bool,
    pub retained_extra_turn_waiting: bool,
    pub turn: TurnState,
    pub turn_store: TurnStore,
    pub stack: Vec<StackEntry>,
    pub combat: Option<CombatState>,
    pub range_of_influence: Option<LimitedRangeOfInfluenceState>,
}

#[derive(Debug, Clone)]
pub struct GrandMeleeRestore {
    pub seats: Vec<PlayerId>,
    pub starting_player_count: usize,
    pub focused_marker: u32,
    pub markers: Vec<GrandMeleeMarkerRestore>,
    pub deferred_extra_turns: Vec<(PlayerId, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaitingTurnKind {
    Normal,
    RetainedExtra,
}

/// Everything that belongs to one simultaneous Grand Melee turn.
///
/// The ordinary engine continues to execute one selected lane at a time. A
/// lane switch stores and restores all turn-local state, including the stack
/// and the CR 801.2c range snapshot, so no stack or newly-adjacent seat leaks
/// into another marker's turn.
#[derive(Debug, Clone)]
struct GrandMeleeTurnLane {
    turn: TurnState,
    turn_store: TurnStore,
    stack: Vec<StackEntry>,
    combat: Option<CombatState>,
    range_of_influence: Option<LimitedRangeOfInfluenceState>,
}

#[derive(Debug, Clone)]
struct GrandMeleeTurnMarker {
    number: u32,
    holder: PlayerId,
    status: GrandMeleeMarkerStatus,
    waiting_kind: Option<WaitingTurnKind>,
    removal_designations: usize,
    /// An extra turn is running immediately before this player's ordinary
    /// marked turn. When it ends, the marker stays for the ordinary turn.
    normal_turn_pending: bool,
    lane: GrandMeleeTurnLane,
}

/// Persistent CR 807 match state.
#[derive(Debug, Clone)]
pub struct GrandMeleeState {
    seats: Vec<PlayerId>,
    starting_player_count: usize,
    focused_marker: u32,
    markers: Vec<GrandMeleeTurnMarker>,
    deferred_extra_turns: HashMap<PlayerId, usize>,
    marker_reducing_departures: std::collections::HashSet<PlayerId>,
    prepared_simultaneous_departures: std::collections::HashSet<PlayerId>,
    /// Provenance roots of stack objects, retained after an object is popped
    /// so triggers emitted during its resolution remain bound to its lane.
    stack_provenance_markers: HashMap<ProvNodeId, u32>,
}

impl GrandMeleeState {
    pub fn seats(&self) -> &[PlayerId] {
        &self.seats
    }

    pub fn starting_player_count(&self) -> usize {
        self.starting_player_count
    }

    pub fn focused_marker(&self) -> u32 {
        self.focused_marker
    }

    pub fn marker_count(&self) -> usize {
        self.markers.len()
    }
}

impl GameState {
    fn grand_melee_lane(&self) -> GrandMeleeTurnLane {
        GrandMeleeTurnLane {
            turn: self.turn.clone(),
            turn_store: self.turn_store.clone(),
            stack: self.stack.clone(),
            combat: self.combat.clone(),
            range_of_influence: self.range_of_influence.clone(),
        }
    }

    fn load_grand_melee_lane(&mut self, lane: &GrandMeleeTurnLane) {
        self.turn = lane.turn.clone();
        self.turn_store = lane.turn_store.clone();
        self.stack = lane.stack.clone();
        self.combat = lane.combat.clone();
        self.range_of_influence = lane.range_of_influence.clone();
    }

    fn save_focused_grand_melee_lane(&mut self) {
        let lane = self.grand_melee_lane();
        let Some(state) = self.grand_melee.as_mut() else {
            return;
        };
        if let Some(marker) = state
            .markers
            .iter_mut()
            .find(|marker| marker.number == state.focused_marker)
        {
            marker.lane = lane;
        }
    }

    fn switch_grand_melee_lane_unchecked(&mut self, marker_number: u32) -> Result<(), String> {
        self.save_focused_grand_melee_lane();
        let lane = self
            .grand_melee
            .as_ref()
            .and_then(|state| {
                state
                    .markers
                    .iter()
                    .find(|marker| marker.number == marker_number)
                    .map(|marker| marker.lane.clone())
            })
            .ok_or_else(|| format!("unknown Grand Melee turn marker {marker_number}"))?;
        self.grand_melee
            .as_mut()
            .expect("Grand Melee state was checked")
            .focused_marker = marker_number;
        self.load_grand_melee_lane(&lane);
        if let Some(holder) = self.focused_grand_melee_holder() {
            self.focus_planar_controller_for_grand_melee(holder);
        }
        self.bump_mutation_revision();
        Ok(())
    }

    /// Randomly seats the players and enables the fixed Grand Melee options.
    pub fn enable_grand_melee(&mut self) -> Result<(), String> {
        if self.players_in_game() < 4 {
            return Err("Grand Melee requires at least four players per turn marker".into());
        }
        let mut seats = self
            .players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<_>>();
        self.shuffle_slice(&mut seats);
        self.restore_grand_melee(seats)
    }

    /// Restore a synchronized randomized seating order and construct one
    /// numbered marker for every full four starting players (CR 807.3-.4b).
    pub fn restore_grand_melee(&mut self, seats: Vec<PlayerId>) -> Result<(), String> {
        let starting_player = seats
            .iter()
            .copied()
            .find(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .ok_or_else(|| "Grand Melee requires an in-game starting player".to_string())?;
        self.restore_grand_melee_with_starting_player(seats, starting_player)
    }

    /// Restore Grand Melee across a restart/subgame boundary while preserving
    /// physical seats and using the new game's chosen starting player.
    pub fn restore_grand_melee_with_starting_player(
        &mut self,
        seats: Vec<PlayerId>,
        starting_player: PlayerId,
    ) -> Result<(), String> {
        let starting_player_count = self.players_in_game();
        if starting_player_count < 4 {
            return Err("Grand Melee requires at least four players per turn marker".into());
        }
        self.restore_free_for_all(seats.clone(), FreeForAllAttackOption::Left, Some(1))?;

        let mut live_seats = seats
            .iter()
            .copied()
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .collect::<Vec<_>>();
        let start_index = live_seats
            .iter()
            .position(|player| *player == starting_player)
            .ok_or_else(|| "Grand Melee starting player must be in the game".to_string())?;
        live_seats.rotate_left(start_index);
        let marker_count = live_seats.len() / 4;
        let base_lane = self.grand_melee_lane();
        let markers = (0..marker_count)
            .map(|index| {
                let holder = live_seats[index * 4];
                let mut lane = base_lane.clone();
                lane.turn = TurnState::new(holder);
                lane.turn_store.turn_order = seats.clone();
                lane.stack.clear();
                lane.combat = None;
                GrandMeleeTurnMarker {
                    number: index as u32 + 1,
                    holder,
                    status: GrandMeleeMarkerStatus::Active,
                    waiting_kind: None,
                    removal_designations: 0,
                    normal_turn_pending: false,
                    lane,
                }
            })
            .collect::<Vec<_>>();
        let first_lane = markers[0].lane.clone();
        self.grand_melee = Some(GrandMeleeState {
            seats,
            starting_player_count,
            focused_marker: 1,
            markers,
            deferred_extra_turns: HashMap::new(),
            marker_reducing_departures: std::collections::HashSet::new(),
            prepared_simultaneous_departures: std::collections::HashSet::new(),
            stack_provenance_markers: HashMap::new(),
        });
        self.load_grand_melee_lane(&first_lane);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn grand_melee(&self) -> Option<&GrandMeleeState> {
        self.grand_melee.as_ref()
    }

    pub fn grand_melee_marker_views(&self) -> Vec<GrandMeleeMarkerView> {
        let focused = self.grand_melee.as_ref().map(|state| state.focused_marker);
        self.grand_melee
            .as_ref()
            .into_iter()
            .flat_map(|state| state.markers.iter())
            .map(|marker| GrandMeleeMarkerView {
                number: marker.number,
                holder: marker.holder,
                status: marker.status,
                stack_size: if focused == Some(marker.number) {
                    self.stack.len()
                } else {
                    marker.lane.stack.len()
                },
                removal_designations: marker.removal_designations,
                normal_turn_pending: marker.normal_turn_pending,
                retained_extra_turn_waiting: marker.waiting_kind
                    == Some(WaitingTurnKind::RetainedExtra),
            })
            .collect()
    }

    pub fn grand_melee_restore_snapshot(&self) -> Option<GrandMeleeRestore> {
        let state = self.grand_melee.as_ref()?;
        Some(GrandMeleeRestore {
            seats: state.seats.clone(),
            starting_player_count: state.starting_player_count,
            focused_marker: state.focused_marker,
            markers: state
                .markers
                .iter()
                .map(|marker| GrandMeleeMarkerRestore {
                    number: marker.number,
                    holder: marker.holder,
                    status: marker.status,
                    removal_designations: marker.removal_designations,
                    normal_turn_pending: marker.normal_turn_pending,
                    retained_extra_turn_waiting: marker.waiting_kind
                        == Some(WaitingTurnKind::RetainedExtra),
                    turn: if marker.number == state.focused_marker {
                        self.turn.clone()
                    } else {
                        marker.lane.turn.clone()
                    },
                    turn_store: if marker.number == state.focused_marker {
                        self.turn_store.clone()
                    } else {
                        marker.lane.turn_store.clone()
                    },
                    stack: if marker.number == state.focused_marker {
                        self.stack.clone()
                    } else {
                        marker.lane.stack.clone()
                    },
                    combat: if marker.number == state.focused_marker {
                        self.combat.clone()
                    } else {
                        marker.lane.combat.clone()
                    },
                    range_of_influence: if marker.number == state.focused_marker {
                        self.range_of_influence.clone()
                    } else {
                        marker.lane.range_of_influence.clone()
                    },
                })
                .collect(),
            deferred_extra_turns: state
                .deferred_extra_turns
                .iter()
                .map(|(player, count)| (*player, *count))
                .collect(),
        })
    }

    pub fn restore_grand_melee_snapshot(
        &mut self,
        snapshot: GrandMeleeRestore,
    ) -> Result<(), String> {
        let first_holder = snapshot
            .markers
            .iter()
            .find(|marker| marker.number == 1)
            .map(|marker| marker.holder)
            .ok_or_else(|| "Grand Melee checkpoint is missing marker 1".to_string())?;
        self.restore_grand_melee_with_starting_player(snapshot.seats, first_holder)?;
        let expected = self
            .grand_melee
            .as_ref()
            .map_or(0, |state| state.markers.len());
        if snapshot.markers.len() != expected {
            return Err("Grand Melee checkpoint has the wrong marker count".to_string());
        }
        let fallback_range = self.range_of_influence.clone();
        let Some(state) = self.grand_melee.as_mut() else {
            unreachable!();
        };
        for restored in snapshot.markers {
            let marker = state
                .markers
                .iter_mut()
                .find(|marker| marker.number == restored.number)
                .ok_or_else(|| {
                    format!(
                        "Grand Melee checkpoint contains unknown marker {}",
                        restored.number
                    )
                })?;
            marker.holder = restored.holder;
            marker.status = restored.status;
            marker.waiting_kind = (restored.status == GrandMeleeMarkerStatus::Waiting).then_some(
                if restored.retained_extra_turn_waiting {
                    WaitingTurnKind::RetainedExtra
                } else {
                    WaitingTurnKind::Normal
                },
            );
            marker.removal_designations = restored.removal_designations;
            marker.normal_turn_pending = restored.normal_turn_pending;
            marker.lane.turn = restored.turn;
            marker.lane.turn_store = restored.turn_store;
            marker.lane.stack = restored.stack;
            marker.lane.combat = restored.combat;
            marker.lane.range_of_influence = restored
                .range_of_influence
                .or_else(|| fallback_range.clone());
        }
        state.starting_player_count = snapshot.starting_player_count;
        state.deferred_extra_turns = snapshot.deferred_extra_turns.into_iter().collect();
        state.stack_provenance_markers = state
            .markers
            .iter()
            .flat_map(|marker| {
                marker
                    .lane
                    .stack
                    .iter()
                    .filter(|entry| entry.provenance != ProvNodeId::default())
                    .map(|entry| (entry.provenance, marker.number))
            })
            .collect();
        if !state
            .markers
            .iter()
            .any(|marker| marker.number == snapshot.focused_marker)
        {
            return Err("Grand Melee checkpoint focuses an unknown marker".to_string());
        }
        state.focused_marker = snapshot.focused_marker;
        let lane = state
            .markers
            .iter()
            .find(|marker| marker.number == state.focused_marker)
            .expect("validated focused marker")
            .lane
            .clone();
        self.load_grand_melee_lane(&lane);
        if let Some(holder) = self.focused_grand_melee_holder() {
            self.focus_planar_controller_for_grand_melee(holder);
        }
        Ok(())
    }

    /// All players whose numbered markers currently represent active turns.
    pub fn grand_melee_active_players(&self) -> Vec<PlayerId> {
        self.grand_melee
            .as_ref()
            .into_iter()
            .flat_map(|state| state.markers.iter())
            .filter(|marker| marker.status == GrandMeleeMarkerStatus::Active)
            .filter_map(|marker| {
                self.player(marker.holder)
                    .is_some_and(|player| player.is_in_game())
                    .then_some(marker.holder)
            })
            .collect()
    }

    /// Select the turn/stack lane on which subsequent actions occur.
    pub fn select_grand_melee_turn_marker(&mut self, marker_number: u32) -> Result<(), String> {
        let marker = self
            .grand_melee
            .as_ref()
            .and_then(|state| {
                state
                    .markers
                    .iter()
                    .find(|marker| marker.number == marker_number)
            })
            .ok_or_else(|| format!("unknown Grand Melee turn marker {marker_number}"))?;
        if marker.status != GrandMeleeMarkerStatus::Active {
            return Err(format!(
                "Grand Melee turn marker {marker_number} is waiting and has no active stack"
            ));
        }
        self.switch_grand_melee_lane_unchecked(marker_number)
    }

    /// Select a marker only when `player` is permitted to receive priority for
    /// its stack under CR 807.5a.
    pub fn select_grand_melee_stack_for_player(
        &mut self,
        player: PlayerId,
        marker_number: u32,
    ) -> Result<(), String> {
        if !self
            .grand_melee_priority_players_for(marker_number)
            .contains(&player)
        {
            return Err(format!(
                "player {} is outside the priority range of Grand Melee marker {marker_number}",
                player.0
            ));
        }
        self.select_grand_melee_turn_marker(marker_number)
    }

    pub fn grand_melee_priority_players_for(&self, marker_number: u32) -> Vec<PlayerId> {
        let Some(state) = self.grand_melee.as_ref() else {
            return self
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.id)
                .collect();
        };
        let Some(marker) = state
            .markers
            .iter()
            .find(|marker| marker.number == marker_number)
        else {
            return Vec::new();
        };
        if marker.status != GrandMeleeMarkerStatus::Active {
            return Vec::new();
        }
        let stack = if state.focused_marker == marker_number {
            &self.stack
        } else {
            &marker.lane.stack
        };
        let range = if state.focused_marker == marker_number {
            self.range_of_influence.as_ref()
        } else {
            marker.lane.range_of_influence.as_ref()
        };
        let within_marker_lane_range = |observer, subject| {
            range.is_none_or(|range| range.player_is_in_turn_snapshot(observer, subject))
        };
        state
            .seats
            .iter()
            .copied()
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .filter(|observer| {
                within_marker_lane_range(*observer, marker.holder)
                    || stack
                        .iter()
                        .any(|entry| within_marker_lane_range(*observer, entry.controller))
            })
            .collect()
    }

    /// Active marker stacks for which `player` is entitled to priority.
    ///
    /// CR 807.5b uses this set when a spell or ability is not already bound to
    /// the stack containing its cause or target.
    pub fn grand_melee_priority_markers_for(&self, player: PlayerId) -> Vec<u32> {
        self.grand_melee
            .as_ref()
            .into_iter()
            .flat_map(|state| state.markers.iter())
            .filter(|marker| marker.status == GrandMeleeMarkerStatus::Active)
            .filter(|marker| {
                self.grand_melee_priority_players_for(marker.number)
                    .contains(&player)
            })
            .map(|marker| marker.number)
            .collect()
    }

    /// Find the marker stack containing an object, or whose stack execution
    /// provenance is an ancestor of `provenance`.
    ///
    /// The provenance lookup keeps triggers caused by a resolving stack object
    /// on that object's marker even after the object itself has left the stack.
    pub(crate) fn grand_melee_stack_marker_for_cause(
        &self,
        object: Option<ObjectId>,
        provenance: ProvNodeId,
    ) -> Option<u32> {
        let state = self.grand_melee.as_ref()?;
        state
            .markers
            .iter()
            .find_map(|marker| {
                let stack = if marker.number == state.focused_marker {
                    &self.stack
                } else {
                    &marker.lane.stack
                };
                stack
                    .iter()
                    .any(|entry| {
                        object == Some(entry.object_id)
                            || self
                                .provenance_graph()
                                .is_descendant_of(provenance, entry.provenance)
                    })
                    .then_some(marker.number)
            })
            .or_else(|| {
                state
                    .stack_provenance_markers
                    .iter()
                    .find_map(|(ancestor, marker)| {
                        self.provenance_graph()
                            .is_descendant_of(provenance, *ancestor)
                            .then_some(*marker)
                    })
            })
    }

    pub(crate) fn record_grand_melee_stack_provenance(&mut self, provenance: ProvNodeId) {
        if provenance == ProvNodeId::default() {
            return;
        }
        let Some(state) = self.grand_melee.as_mut() else {
            return;
        };
        state
            .stack_provenance_markers
            .insert(provenance, state.focused_marker);
    }

    pub fn object_is_on_current_stack(&self, object: crate::ids::ObjectId) -> bool {
        self.stack.iter().any(|entry| entry.object_id == object)
    }

    pub fn priority_players_for_current_turn(&self) -> Vec<PlayerId> {
        self.grand_melee.as_ref().map_or_else(
            || {
                self.players
                    .iter()
                    .filter(|player| player.is_in_game())
                    .map(|player| player.id)
                    .collect()
            },
            |state| self.grand_melee_priority_players_for(state.focused_marker),
        )
    }

    pub(crate) fn next_grand_melee_priority_player_after(
        &self,
        player: PlayerId,
    ) -> Option<PlayerId> {
        let eligible = self.priority_players_for_current_turn();
        let index = eligible.iter().position(|candidate| *candidate == player)?;
        (1..=eligible.len())
            .map(|offset| eligible[(index + offset) % eligible.len()])
            .next()
    }

    fn live_grand_melee_seats(&self) -> Vec<PlayerId> {
        self.grand_melee
            .as_ref()
            .map(|state| {
                state
                    .seats
                    .iter()
                    .copied()
                    .filter(|player| {
                        self.player(*player)
                            .is_some_and(|candidate| candidate.is_in_game())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn grand_melee_left_distance(&self, from: PlayerId, to: PlayerId) -> Option<usize> {
        let seats = self.live_grand_melee_seats();
        let from = seats.iter().position(|player| *player == from)?;
        let to = seats.iter().position(|player| *player == to)?;
        Some((to + seats.len() - from) % seats.len())
    }

    fn grand_melee_marker_can_start(&self, marker_number: u32, holder: PlayerId) -> bool {
        self.grand_melee.as_ref().is_some_and(|state| {
            state
                .markers
                .iter()
                .filter(|marker| marker.number != marker_number)
                .all(|marker| {
                    !matches!(
                        self.grand_melee_left_distance(holder, marker.holder),
                        Some(1..=3)
                    )
                })
        })
    }

    fn next_live_grand_melee_seat_left(&self, player: PlayerId) -> Option<PlayerId> {
        let seats = self.live_grand_melee_seats();
        let index = self
            .grand_melee
            .as_ref()?
            .seats
            .iter()
            .position(|candidate| *candidate == player)?;
        (1..=self.grand_melee.as_ref()?.seats.len())
            .map(|offset| {
                self.grand_melee.as_ref().unwrap().seats
                    [(index + offset) % self.grand_melee.as_ref().unwrap().seats.len()]
            })
            .find(|candidate| seats.contains(candidate))
    }

    fn begin_focused_grand_melee_turn(&mut self, player: PlayerId, is_extra_turn: bool) {
        self.turn_store.extra_turns.clear();
        self.turn_store.extra_turns.push(player);
        self.next_turn_single_lane_with_extra_turn_override(Some(is_extra_turn));
        self.save_focused_grand_melee_lane();
    }

    fn defer_extra_turns(&mut self, players: impl IntoIterator<Item = PlayerId>) {
        let players = players
            .into_iter()
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .collect::<Vec<_>>();
        let Some(state) = self.grand_melee.as_mut() else {
            return;
        };
        for player in players {
            *state.deferred_extra_turns.entry(player).or_default() += 1;
        }
    }

    fn begin_received_marker_turn(&mut self, marker_number: u32, holder: PlayerId) {
        let deferred = self
            .grand_melee
            .as_mut()
            .and_then(|state| state.deferred_extra_turns.get_mut(&holder))
            .is_some_and(|count| {
                if *count == 0 {
                    false
                } else {
                    *count -= 1;
                    true
                }
            });
        if let Some(state) = self.grand_melee.as_mut() {
            if state.deferred_extra_turns.get(&holder) == Some(&0) {
                state.deferred_extra_turns.remove(&holder);
            }
            if let Some(marker) = state
                .markers
                .iter_mut()
                .find(|marker| marker.number == marker_number)
            {
                marker.status = GrandMeleeMarkerStatus::Active;
                marker.waiting_kind = None;
                marker.normal_turn_pending = deferred;
            }
        }
        let previous_focus = self
            .grand_melee
            .as_ref()
            .map(|state| state.focused_marker)
            .unwrap_or(marker_number);
        self.switch_grand_melee_lane_unchecked(marker_number)
            .expect("marker exists while beginning its turn");
        self.begin_focused_grand_melee_turn(holder, deferred);
        if previous_focus != marker_number
            && self.grand_melee.as_ref().is_some_and(|state| {
                state.markers.iter().any(|marker| {
                    marker.number == previous_focus
                        && marker.status == GrandMeleeMarkerStatus::Active
                })
            })
        {
            self.switch_grand_melee_lane_unchecked(previous_focus)
                .expect("previous focused marker remains present");
        }
    }

    fn reevaluate_waiting_grand_melee_markers(&mut self) {
        loop {
            let ready = self.grand_melee.as_ref().and_then(|state| {
                state
                    .markers
                    .iter()
                    .find(|marker| {
                        marker.status == GrandMeleeMarkerStatus::Waiting
                            && self.grand_melee_marker_can_start(marker.number, marker.holder)
                    })
                    .map(|marker| (marker.number, marker.holder, marker.waiting_kind))
            });
            let Some((number, holder, kind)) = ready else {
                break;
            };
            match kind.unwrap_or(WaitingTurnKind::Normal) {
                WaitingTurnKind::Normal => self.begin_received_marker_turn(number, holder),
                WaitingTurnKind::RetainedExtra => {
                    if let Some(marker) = self.grand_melee.as_mut().and_then(|state| {
                        state
                            .markers
                            .iter_mut()
                            .find(|marker| marker.number == number)
                    }) {
                        marker.status = GrandMeleeMarkerStatus::Active;
                        marker.waiting_kind = None;
                        marker.normal_turn_pending = false;
                    }
                    let old_focus = self.grand_melee.as_ref().unwrap().focused_marker;
                    self.switch_grand_melee_lane_unchecked(number)
                        .expect("waiting marker still exists");
                    self.begin_focused_grand_melee_turn(holder, true);
                    if old_focus != number
                        && self.grand_melee.as_ref().is_some_and(|state| {
                            state.markers.iter().any(|marker| {
                                marker.number == old_focus
                                    && marker.status == GrandMeleeMarkerStatus::Active
                            })
                        })
                    {
                        self.switch_grand_melee_lane_unchecked(old_focus)
                            .expect("old focus still exists");
                    }
                }
            }
        }
    }

    fn focus_any_active_grand_melee_marker(&mut self) {
        if self.grand_melee.as_ref().is_some_and(|state| {
            state.markers.iter().any(|marker| {
                marker.number == state.focused_marker
                    && marker.status == GrandMeleeMarkerStatus::Active
            })
        }) {
            return;
        }
        let next = self.grand_melee.as_ref().and_then(|state| {
            state
                .markers
                .iter()
                .filter(|marker| marker.status == GrandMeleeMarkerStatus::Active)
                .min_by_key(|marker| marker.number)
                .map(|marker| marker.number)
        });
        if let Some(next) = next {
            let _ = self.switch_grand_melee_lane_unchecked(next);
        }
    }

    fn focus_next_active_grand_melee_marker_after(&mut self, marker_number: u32) {
        let next = self.grand_melee.as_ref().and_then(|state| {
            let mut active = state
                .markers
                .iter()
                .filter(|marker| marker.status == GrandMeleeMarkerStatus::Active)
                .map(|marker| marker.number)
                .collect::<Vec<_>>();
            active.sort_unstable();
            active
                .iter()
                .copied()
                .find(|number| *number > marker_number)
                .or_else(|| active.first().copied())
        });
        if let Some(next) = next {
            let _ = self.switch_grand_melee_lane_unchecked(next);
        }
    }

    fn marker_immediately_right_of_seat(&self, seat: PlayerId) -> Option<u32> {
        let state = self.grand_melee.as_ref()?;
        let seat_index = state.seats.iter().position(|player| *player == seat)?;
        state
            .markers
            .iter()
            .filter_map(|marker| {
                let marker_index = state
                    .seats
                    .iter()
                    .position(|player| *player == marker.holder)?;
                let distance = (seat_index + state.seats.len() - marker_index) % state.seats.len();
                Some((distance, marker.number))
            })
            .filter(|(distance, _)| *distance > 0)
            .min()
            .map(|(_, number)| number)
            .or_else(|| state.markers.iter().map(|marker| marker.number).min())
    }

    fn remove_grand_melee_marker(&mut self, marker_number: u32) {
        self.save_focused_grand_melee_lane();
        let Some((holder, extra_designations)) = self.grand_melee.as_ref().and_then(|state| {
            state
                .markers
                .iter()
                .find(|marker| marker.number == marker_number)
                .map(|marker| (marker.holder, marker.removal_designations.saturating_sub(1)))
        }) else {
            return;
        };
        if let Some(state) = self.grand_melee.as_mut() {
            state
                .markers
                .retain(|marker| marker.number != marker_number);
            state
                .stack_provenance_markers
                .retain(|_, marker| *marker != marker_number);
        }
        if extra_designations > 0
            && let Some(next) = self.marker_immediately_right_of_seat(holder)
            && let Some(marker) = self.grand_melee.as_mut().and_then(|state| {
                state
                    .markers
                    .iter_mut()
                    .find(|marker| marker.number == next)
            })
        {
            marker.removal_designations += extra_designations;
        }
        self.focus_any_active_grand_melee_marker();
    }

    /// Turn-completion hook used by `GameState::next_turn` while CR 807 is on.
    pub(crate) fn next_grand_melee_turn(&mut self) {
        let marker_number = self
            .grand_melee
            .as_ref()
            .expect("Grand Melee hook requires state")
            .focused_marker;
        let (holder, normal_pending, removal_designations) = self
            .grand_melee
            .as_ref()
            .and_then(|state| {
                state
                    .markers
                    .iter()
                    .find(|marker| marker.number == marker_number)
            })
            .map(|marker| {
                (
                    marker.holder,
                    marker.normal_turn_pending,
                    marker.removal_designations,
                )
            })
            .expect("focused marker exists");

        if normal_pending {
            let has_more_deferred = self
                .grand_melee
                .as_mut()
                .and_then(|state| state.deferred_extra_turns.get_mut(&holder))
                .is_some_and(|count| {
                    if *count == 0 {
                        false
                    } else {
                        *count -= 1;
                        true
                    }
                });
            if !has_more_deferred
                && let Some(marker) = self.grand_melee.as_mut().and_then(|state| {
                    state
                        .markers
                        .iter_mut()
                        .find(|marker| marker.number == marker_number)
                })
            {
                marker.normal_turn_pending = false;
            }
            self.begin_focused_grand_melee_turn(holder, has_more_deferred);
            return;
        }

        let extra_turns = std::mem::take(&mut self.turn_store.extra_turns);
        if let Some(next_extra) = extra_turns.last().copied() {
            self.defer_extra_turns(extra_turns[..extra_turns.len() - 1].iter().copied());
            if next_extra == holder {
                let (near_left, near_right) = self
                    .grand_melee
                    .as_ref()
                    .map(|state| {
                        state
                            .markers
                            .iter()
                            .filter(|marker| marker.number != marker_number)
                            .fold((false, false), |(left, right), marker| {
                                let left_distance =
                                    self.grand_melee_left_distance(holder, marker.holder);
                                let right_distance =
                                    self.grand_melee_left_distance(marker.holder, holder);
                                (
                                    left || matches!(left_distance, Some(1..=3)),
                                    right || matches!(right_distance, Some(1..=3)),
                                )
                            })
                    })
                    .unwrap_or_default();
                if near_left {
                    if let Some(marker) = self.grand_melee.as_mut().and_then(|state| {
                        state
                            .markers
                            .iter_mut()
                            .find(|marker| marker.number == marker_number)
                    }) {
                        marker.status = GrandMeleeMarkerStatus::Waiting;
                        marker.waiting_kind = Some(WaitingTurnKind::RetainedExtra);
                    }
                    self.save_focused_grand_melee_lane();
                    self.focus_any_active_grand_melee_marker();
                    return;
                }
                if !near_right {
                    self.begin_focused_grand_melee_turn(holder, true);
                    return;
                }
            }
            self.defer_extra_turns([next_extra]);
        }

        if removal_designations > 0 {
            self.remove_grand_melee_marker(marker_number);
            self.reevaluate_waiting_grand_melee_markers();
            return;
        }

        let Some(receiver) = self.next_live_grand_melee_seat_left(holder) else {
            return;
        };
        if let Some(marker) = self.grand_melee.as_mut().and_then(|state| {
            state
                .markers
                .iter_mut()
                .find(|marker| marker.number == marker_number)
        }) {
            marker.holder = receiver;
            marker.status = GrandMeleeMarkerStatus::Waiting;
            marker.waiting_kind = Some(WaitingTurnKind::Normal);
            marker.normal_turn_pending = false;
        }
        self.save_focused_grand_melee_lane();
        if self.grand_melee_marker_can_start(marker_number, receiver) {
            self.begin_received_marker_turn(marker_number, receiver);
        } else {
            self.focus_any_active_grand_melee_marker();
        }
        self.reevaluate_waiting_grand_melee_markers();
        self.focus_next_active_grand_melee_marker_after(marker_number);
    }

    /// Update marker movement/removal immediately after a player is marked as
    /// having left the game. The ordinary leave-game procedure continues to
    /// clean the selected stack; every dormant stack is pruned afterward.
    pub(crate) fn handle_grand_melee_player_departure(&mut self, player: PlayerId) {
        if self.grand_melee.is_none() {
            return;
        }

        let waiting_marker = self.grand_melee.as_ref().and_then(|state| {
            state
                .markers
                .iter()
                .find(|marker| {
                    marker.holder == player && marker.status == GrandMeleeMarkerStatus::Waiting
                })
                .map(|marker| marker.number)
        });
        if let Some(number) = waiting_marker
            && let Some(receiver) = self.next_live_grand_melee_seat_left(player)
            && let Some(marker) = self.grand_melee.as_mut().and_then(|state| {
                state
                    .markers
                    .iter_mut()
                    .find(|marker| marker.number == number)
            })
        {
            marker.holder = receiver;
            marker.waiting_kind = Some(WaitingTurnKind::Normal);
            marker.normal_turn_pending = false;
        }

        let prepared_simultaneously = self
            .grand_melee
            .as_mut()
            .is_some_and(|state| state.prepared_simultaneous_departures.remove(&player));
        let live_count = self.players_in_game();
        let desired_count = live_count / 4;
        let (marker_count, already_designated) = self
            .grand_melee
            .as_ref()
            .map(|state| {
                (
                    state.markers.len(),
                    state
                        .markers
                        .iter()
                        .map(|marker| marker.removal_designations)
                        .sum::<usize>(),
                )
            })
            .unwrap_or_default();
        let needed = if prepared_simultaneously {
            0
        } else {
            marker_count
                .saturating_sub(already_designated)
                .saturating_sub(desired_count)
        };
        if needed > 0
            && let Some(state) = self.grand_melee.as_mut()
        {
            state.marker_reducing_departures.insert(player);
        }
        for _ in 0..needed {
            let Some(number) = self.marker_immediately_right_of_seat(player) else {
                break;
            };
            let remove_immediately = if let Some(marker) =
                self.grand_melee.as_mut().and_then(|state| {
                    state
                        .markers
                        .iter_mut()
                        .find(|marker| marker.number == number)
                }) {
                marker.removal_designations += 1;
                marker.status == GrandMeleeMarkerStatus::Waiting
            } else {
                false
            };
            if remove_immediately {
                self.remove_grand_melee_marker(number);
            }
        }

        self.save_focused_grand_melee_lane();
        if let Some(state) = self.grand_melee.as_mut() {
            for marker in &mut state.markers {
                marker
                    .lane
                    .turn_store
                    .extra_turns
                    .retain(|turn| *turn != player);
                marker.lane.stack.retain(|entry| entry.controller != player);
            }
            state.deferred_extra_turns.remove(&player);
        }
        self.reevaluate_waiting_grand_melee_markers();
        self.focus_any_active_grand_melee_marker();
        self.bump_mutation_revision();
    }

    /// Pre-designate the lowest-numbered eligible marker before a known group
    /// of players leaves simultaneously (CR 807.4e-f).
    pub(crate) fn prepare_grand_melee_simultaneous_departures(&mut self, players: &[PlayerId]) {
        if self.grand_melee.is_none() {
            return;
        }
        let departing = players
            .iter()
            .copied()
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .collect::<std::collections::HashSet<_>>();
        if departing.is_empty() {
            return;
        }
        let (marker_count, already_designated) = self
            .grand_melee
            .as_ref()
            .map(|state| {
                (
                    state.markers.len(),
                    state
                        .markers
                        .iter()
                        .map(|marker| marker.removal_designations)
                        .sum::<usize>(),
                )
            })
            .unwrap_or_default();
        let desired_count = self.players_in_game().saturating_sub(departing.len()) / 4;
        let needed = marker_count
            .saturating_sub(already_designated)
            .saturating_sub(desired_count);
        if let Some(state) = self.grand_melee.as_mut() {
            state
                .prepared_simultaneous_departures
                .extend(departing.iter().copied());
            if needed > 0 {
                state
                    .marker_reducing_departures
                    .extend(departing.iter().copied());
            }
        }
        if needed == 0 {
            return;
        }
        let selected = departing
            .iter()
            .filter_map(|player| self.marker_immediately_right_of_seat(*player))
            .min();
        let Some(selected) = selected else {
            return;
        };
        let remove_immediately = if let Some(marker) = self.grand_melee.as_mut().and_then(|state| {
            state
                .markers
                .iter_mut()
                .find(|marker| marker.number == selected)
        }) {
            marker.removal_designations += needed;
            marker.status == GrandMeleeMarkerStatus::Waiting
        } else {
            false
        };
        if remove_immediately {
            self.remove_grand_melee_marker(selected);
        }
    }

    pub(crate) fn take_grand_melee_marker_reducing_departure(&mut self, player: PlayerId) -> bool {
        self.grand_melee
            .as_mut()
            .is_some_and(|state| state.marker_reducing_departures.remove(&player))
    }

    pub(crate) fn focused_grand_melee_holder(&self) -> Option<PlayerId> {
        self.grand_melee.as_ref().and_then(|state| {
            state
                .markers
                .iter()
                .find(|marker| marker.number == state.focused_marker)
                .map(|marker| marker.holder)
        })
    }

    pub(crate) fn prune_grand_melee_stacks_for_departure(
        &mut self,
        player: PlayerId,
        removed_objects: &std::collections::HashSet<crate::ids::ObjectId>,
    ) {
        self.save_focused_grand_melee_lane();
        if let Some(state) = self.grand_melee.as_mut() {
            for marker in &mut state.markers {
                marker.lane.stack.retain(|entry| {
                    entry.controller != player
                        && (entry.is_ability || !removed_objects.contains(&entry.object_id))
                });
            }
        }
    }

    pub(crate) fn synchronize_focused_grand_melee_lane(&mut self) {
        self.save_focused_grand_melee_lane();
    }
}
