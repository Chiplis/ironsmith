use std::collections::{HashMap, HashSet};

use crate::ids::{ObjectId, PlayerId};
use crate::object::AttachmentTarget;
use crate::snapshot::ObjectSnapshot;
use crate::types::CardType;

use super::GameState;

/// Stable seating and the CR 801.2c range membership frozen at turn start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitedRangeOfInfluenceState {
    seats: Vec<PlayerId>,
    ranges: HashMap<PlayerId, u8>,
    turn_snapshot: HashMap<PlayerId, HashSet<PlayerId>>,
}

impl LimitedRangeOfInfluenceState {
    pub fn from_restore_snapshot(
        seats: Vec<PlayerId>,
        ranges: Vec<u8>,
        turn_snapshot: Vec<(PlayerId, Vec<PlayerId>)>,
    ) -> Result<Self, String> {
        if seats.len() != ranges.len()
            || seats.iter().copied().collect::<HashSet<_>>().len() != seats.len()
        {
            return Err("range-of-influence restore requires one range per distinct seat".into());
        }
        let known = seats.iter().copied().collect::<HashSet<_>>();
        if turn_snapshot.iter().any(|(observer, players)| {
            !known.contains(observer) || players.iter().any(|player| !known.contains(player))
        }) {
            return Err("range-of-influence restore contains an unknown seat".into());
        }
        let mut state = Self {
            ranges: seats.iter().copied().zip(ranges).collect(),
            seats,
            turn_snapshot: HashMap::new(),
        };
        state.replace_turn_snapshot(turn_snapshot);
        Ok(state)
    }

    pub fn seats(&self) -> &[PlayerId] {
        &self.seats
    }

    pub fn configured_range(&self, player: PlayerId) -> Option<u8> {
        self.ranges.get(&player).copied()
    }

    pub fn players_in_turn_snapshot(&self, player: PlayerId) -> Vec<PlayerId> {
        self.turn_snapshot
            .get(&player)
            .map(|players| {
                self.seats
                    .iter()
                    .copied()
                    .filter(|candidate| players.contains(candidate))
                    .collect()
            })
            .unwrap_or_else(|| vec![player])
    }

    pub(crate) fn player_is_in_turn_snapshot(&self, observer: PlayerId, subject: PlayerId) -> bool {
        observer == subject
            || self
                .turn_snapshot
                .get(&observer)
                .is_some_and(|players| players.contains(&subject))
    }

    pub(crate) fn replace_turn_snapshot(&mut self, snapshot: Vec<(PlayerId, Vec<PlayerId>)>) {
        self.turn_snapshot = snapshot
            .into_iter()
            .map(|(observer, players)| (observer, players.into_iter().collect()))
            .collect();
    }
}

impl GameState {
    /// Enable CR 801 using a stable circular seating order and one configured
    /// range per seat. Membership is immediately snapshotted for the current
    /// turn and changes again only as a later turn begins.
    pub fn enable_limited_range_of_influence(
        &mut self,
        seats: Vec<PlayerId>,
        ranges: Vec<u8>,
    ) -> Result<(), String> {
        if self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.alternating_teams.is_some()
        {
            return Err("the active multiplayer profile fixes range before play begins".into());
        }
        if seats.len() != self.players.len() || ranges.len() != seats.len() {
            return Err("range-of-influence setup requires one seat and range per player".into());
        }
        let distinct = seats.iter().copied().collect::<HashSet<_>>();
        if distinct.len() != seats.len()
            || self
                .players
                .iter()
                .any(|player| !distinct.contains(&player.id))
        {
            return Err("range-of-influence seats must contain every player exactly once".into());
        }
        if let Some(profile) = self.free_for_all.as_ref() {
            if ranges
                .first()
                .is_some_and(|first| ranges.iter().any(|range| range != first))
            {
                return Err("Free-for-All requires one common range of influence".into());
            }
            if seats != profile.seats || ranges.first().copied() != profile.range_of_influence {
                return Err("Free-for-All multiplayer options are fixed before play begins".into());
            }
        }
        let free_for_all_range = ranges.first().copied();
        self.range_of_influence = Some(LimitedRangeOfInfluenceState {
            ranges: seats.iter().copied().zip(ranges.iter().copied()).collect(),
            seats: seats.clone(),
            turn_snapshot: HashMap::new(),
        });
        self.refresh_range_of_influence_snapshot();
        if let Some(state) = self.free_for_all.as_mut() {
            state.seats = seats;
            state.range_of_influence = free_for_all_range;
        }
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn disable_limited_range_of_influence(&mut self) {
        if self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.alternating_teams.is_some()
        {
            return;
        }
        if self
            .free_for_all
            .as_ref()
            .is_some_and(|profile| profile.range_of_influence.is_some())
        {
            return;
        }
        if self.range_of_influence.is_none() {
            return;
        }
        self.range_of_influence = None;
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }

    /// Restore a serialized CR 801 snapshot without recomputing adjacency in
    /// the middle of a turn after one or more players have left.
    pub fn restore_limited_range_of_influence(
        &mut self,
        seats: Vec<PlayerId>,
        ranges: Vec<u8>,
        turn_snapshot: Vec<(PlayerId, Vec<PlayerId>)>,
    ) -> Result<(), String> {
        if self.emperor.is_some() || self.alternating_teams.is_some() {
            let configured = self
                .range_of_influence
                .as_ref()
                .ok_or_else(|| "Emperor requires limited range of influence".to_string())?;
            let configured_ranges = configured
                .seats
                .iter()
                .map(|player| configured.ranges[player])
                .collect::<Vec<_>>();
            if seats != configured.seats || ranges != configured_ranges {
                return Err("the active multiplayer profile fixes seats and ranges".into());
            }
            let known = self
                .players
                .iter()
                .map(|player| player.id)
                .collect::<HashSet<_>>();
            if turn_snapshot.iter().any(|(observer, players)| {
                !known.contains(observer) || players.iter().any(|player| !known.contains(player))
            }) {
                return Err("range-of-influence snapshot contains an unknown player".into());
            }
            self.range_of_influence
                .as_mut()
                .expect("validated fixed-profile range")
                .replace_turn_snapshot(turn_snapshot);
            return Ok(());
        }
        self.enable_limited_range_of_influence(seats, ranges)?;
        let all_players = self
            .players
            .iter()
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        if turn_snapshot.iter().any(|(observer, players)| {
            !all_players.contains(observer)
                || players.iter().any(|player| !all_players.contains(player))
        }) {
            return Err("range-of-influence snapshot contains an unknown player".into());
        }
        if let Some(state) = self.range_of_influence.as_mut() {
            state.replace_turn_snapshot(turn_snapshot);
        }
        Ok(())
    }

    pub fn limited_range_of_influence(&self) -> Option<&LimitedRangeOfInfluenceState> {
        self.range_of_influence.as_ref()
    }

    /// Freeze the live circular adjacency graph for the turn that is beginning.
    pub(crate) fn refresh_range_of_influence_snapshot(&mut self) {
        let Some(state) = self.range_of_influence.as_ref() else {
            return;
        };
        let seats = state.seats.clone();
        let ranges = state.ranges.clone();
        let live = seats
            .iter()
            .copied()
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|player| player.is_in_game())
            })
            .collect::<Vec<_>>();
        let mut snapshot = HashMap::new();
        for (observer_index, observer) in live.iter().copied().enumerate() {
            let maximum = ranges.get(&observer).copied().unwrap_or(0) as usize;
            let mut in_range = HashSet::from([observer]);
            if live.len() > 1 {
                for (subject_index, subject) in live.iter().copied().enumerate() {
                    let clockwise = observer_index.abs_diff(subject_index);
                    let distance = clockwise.min(live.len() - clockwise);
                    if distance <= maximum {
                        in_range.insert(subject);
                    }
                }
            }
            snapshot.insert(observer, in_range);
        }
        if let Some(state) = self.range_of_influence.as_mut() {
            state.turn_snapshot = snapshot;
        }
    }

    pub fn player_is_within_range(&self, observer: PlayerId, subject: PlayerId) -> bool {
        if observer == subject {
            return true;
        }
        self.range_of_influence
            .as_ref()
            .is_none_or(|state| state.player_is_in_turn_snapshot(observer, subject))
    }

    pub fn players_within_range(&self, observer: PlayerId) -> Vec<PlayerId> {
        self.range_of_influence.as_ref().map_or_else(
            || {
                self.players
                    .iter()
                    .filter(|player| player.is_in_game())
                    .map(|player| player.id)
                    .collect()
            },
            |state| state.players_in_turn_snapshot(observer),
        )
    }

    /// Plane and phenomenon sources are exempt in non-Grand-Melee Planechase
    /// under CR 801.18. Ironsmith's Planechase profile is the ordinary variant.
    pub fn source_is_exempt_from_range(&self, source: Option<ObjectId>) -> bool {
        self.grand_melee.is_none()
            && source
                .and_then(|source| self.object(source))
                .is_some_and(|source| {
                    source.card_types.contains(&CardType::Plane)
                        || source.card_types.contains(&CardType::Phenomenon)
                })
    }

    pub(crate) fn source_snapshot_is_exempt_from_range(
        &self,
        source: Option<ObjectId>,
        source_snapshot: Option<&ObjectSnapshot>,
    ) -> bool {
        self.source_is_exempt_from_range(source)
            || (self.grand_melee.is_none()
                && source_snapshot.is_some_and(|source| {
                    source.card_types.contains(&CardType::Plane)
                        || source.card_types.contains(&CardType::Phenomenon)
                }))
    }

    /// CR 801.7 requires a trigger event to happen entirely inside the
    /// triggered ability controller's range. Zone-change snapshots supply the
    /// required before-event controller for look-back events.
    pub(crate) fn trigger_event_is_entirely_within_range(
        &self,
        observer: PlayerId,
        event: &dyn crate::events::GameEventType,
        source: Option<ObjectId>,
        source_snapshot: Option<&ObjectSnapshot>,
    ) -> bool {
        if self.range_of_influence.is_none()
            || self.source_snapshot_is_exempt_from_range(source, source_snapshot)
        {
            return true;
        }

        if event
            .player()
            .is_some_and(|player| !self.player_is_within_range(observer, player))
            || event
                .controller()
                .is_some_and(|player| !self.player_is_within_range(observer, player))
        {
            return false;
        }

        let snapshots = event.snapshots();
        if snapshots
            .iter()
            .any(|snapshot| !self.snapshot_is_within_range(observer, snapshot, source))
        {
            return false;
        }
        if let Some(object) = event.object_id()
            && !snapshots
                .iter()
                .any(|snapshot| snapshot.object_id == object)
            && self.object(object).is_some()
            && !self.object_is_within_range(observer, object, source)
        {
            return false;
        }

        event
            .redirectable_targets()
            .into_iter()
            .all(|redirectable| match redirectable.target {
                crate::game_state::Target::Object(object) => {
                    self.object_is_within_range(observer, object, source)
                }
                crate::game_state::Target::Player(player) => {
                    self.player_is_within_range(observer, player)
                }
            })
    }

    pub(crate) fn range_players_for_source(
        &self,
        observer: PlayerId,
        source: Option<ObjectId>,
    ) -> Option<Vec<PlayerId>> {
        (self.range_of_influence.is_some() && !self.source_is_exempt_from_range(source))
            .then(|| self.players_within_range(observer))
    }

    pub fn object_is_within_range(
        &self,
        observer: PlayerId,
        object: ObjectId,
        source: Option<ObjectId>,
    ) -> bool {
        if self.range_of_influence.is_none() || self.source_is_exempt_from_range(source) {
            return true;
        }
        let Some(object) = self.object(object) else {
            return false;
        };
        self.player_is_within_range(observer, self.controller_of(object))
    }

    pub fn snapshot_is_within_range(
        &self,
        observer: PlayerId,
        snapshot: &ObjectSnapshot,
        source: Option<ObjectId>,
    ) -> bool {
        self.range_of_influence.is_none()
            || self.source_is_exempt_from_range(source)
            || self.player_is_within_range(observer, snapshot.controller)
    }

    pub fn attachment_target_is_within_range(
        &self,
        observer: PlayerId,
        target: AttachmentTarget,
        source: Option<ObjectId>,
    ) -> bool {
        match target {
            AttachmentTarget::Object(object) => {
                self.object_is_within_range(observer, object, source)
            }
            AttachmentTarget::Player(player) => {
                self.source_is_exempt_from_range(source)
                    || self.player_is_within_range(observer, player)
            }
        }
    }

    /// CR 801.5c fallback direction, retaining the stable physical seat order
    /// while skipping players who have left the game.
    pub fn closest_in_game_player_to_left_matching(
        &self,
        observer: PlayerId,
        mut predicate: impl FnMut(PlayerId) -> bool,
    ) -> Option<PlayerId> {
        let seats = self
            .range_of_influence
            .as_ref()
            .map(|state| state.seats.as_slice())
            .unwrap_or(self.turn_store.turn_order.as_slice());
        let start = seats.iter().position(|player| *player == observer)?;
        (1..=seats.len())
            .map(|offset| seats[(start + offset) % seats.len()])
            .find(|player| {
                self.player(*player)
                    .is_some_and(|player| player.is_in_game())
                    && predicate(*player)
            })
    }

    /// Apply a draw to exactly these players. Under CR 104.5 each affected
    /// player leaves the game without losing it.
    pub fn draw_game_for_players(
        &mut self,
        players: impl IntoIterator<Item = PlayerId>,
    ) -> Vec<PlayerId> {
        let requested = players.into_iter().collect::<Vec<_>>();
        let mut players = requested.clone();
        for player in requested {
            if let Some(team) = self.emperor_team_members(player) {
                players.extend(team);
            }
        }
        players.sort_by_key(|player| player.0);
        players.dedup();
        let drawn = players
            .into_iter()
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|player| player.is_in_game())
            })
            .collect::<Vec<_>>();
        for player in &drawn {
            self.leave_game(*player);
        }
        drawn
    }

    /// CR 801.15 limits a spell-or-ability draw to its controller and all
    /// players in that controller's frozen range snapshot.
    pub fn draw_game_for_controller_and_range(&mut self, controller: PlayerId) -> Vec<PlayerId> {
        let players = self.players_within_range(controller);
        self.draw_game_for_players(players)
    }
}
