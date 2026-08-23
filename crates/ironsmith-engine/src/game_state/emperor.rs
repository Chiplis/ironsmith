use std::collections::{HashMap, HashSet};

use crate::ids::PlayerId;

use super::{GameState, TeamState};

/// Stable CR 809 roles, seats, ranges, and starting-emperor result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmperorState {
    teams: Vec<Vec<PlayerId>>,
    seats: Vec<PlayerId>,
    emperors: Vec<PlayerId>,
    ranges: Vec<u8>,
    starting_team: usize,
    starting_emperor: PlayerId,
}

impl EmperorState {
    pub fn teams(&self) -> &[Vec<PlayerId>] {
        &self.teams
    }

    pub fn seats(&self) -> &[PlayerId] {
        &self.seats
    }

    pub fn emperors(&self) -> &[PlayerId] {
        &self.emperors
    }

    pub fn ranges(&self) -> &[u8] {
        &self.ranges
    }

    pub fn starting_team(&self) -> usize {
        self.starting_team
    }

    pub fn starting_emperor(&self) -> PlayerId {
        self.starting_emperor
    }

    pub fn is_emperor(&self, player: PlayerId) -> bool {
        self.emperors.contains(&player)
    }

    pub fn team_index(&self, player: PlayerId) -> Option<usize> {
        self.teams.iter().position(|team| team.contains(&player))
    }
}

impl GameState {
    /// Enable CR 809, choosing one emperor from the match RNG.
    pub fn enable_emperor(&mut self, teams: Vec<Vec<PlayerId>>) -> Result<(), String> {
        let (seats, emperors, ranges) = Self::validate_emperor_profile(&self.players, &teams)?;
        let mut team_indices = (0..teams.len()).collect::<Vec<_>>();
        self.shuffle_slice(&mut team_indices);
        let starting_team = team_indices[0];
        let starting_emperor = emperors[starting_team];
        self.restore_emperor(teams, seats, starting_team, starting_emperor, ranges)
    }

    /// Restore an already-selected CR 809 profile without consuming random
    /// state at restart or synchronized-checkpoint boundaries.
    pub fn restore_emperor(
        &mut self,
        teams: Vec<Vec<PlayerId>>,
        seats: Vec<PlayerId>,
        starting_team: usize,
        starting_emperor: PlayerId,
        ranges: Vec<u8>,
    ) -> Result<(), String> {
        let (expected_seats, emperors, expected_ranges) =
            Self::validate_emperor_profile(&self.players, &teams)?;
        if seats != expected_seats {
            return Err("Emperor seats must preserve each team's selected contiguous order".into());
        }
        if ranges != expected_ranges {
            return Err("Emperor ranges must match the minimums derived from the seating".into());
        }
        if emperors.get(starting_team).copied() != Some(starting_emperor) {
            return Err("the Emperor starting player must be the selected team's emperor".into());
        }

        self.free_for_all = None;
        self.grand_melee = None;
        self.team_vs_team = None;
        self.two_headed_giant = None;
        self.alternating_teams = None;
        self.emperor = None;
        self.range_of_influence = None;
        self.attack_direction = None;
        self.shared_team_turns = None;
        self.teams = Some(TeamState {
            teams: teams.clone(),
        });
        self.enable_limited_range_of_influence(seats.clone(), ranges.clone())?;
        self.deploy_creatures = true;
        self.emperor = Some(EmperorState {
            teams,
            seats: seats.clone(),
            emperors,
            ranges,
            starting_team,
            starting_emperor,
        });

        let mut turn_order = seats;
        let start = turn_order
            .iter()
            .position(|player| *player == starting_emperor)
            .expect("validated Emperor starting player");
        turn_order.rotate_left(start);
        self.turn_store.turn_order = turn_order;
        self.turn.active_player = starting_emperor;
        self.turn.priority_player = Some(starting_emperor);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    fn validate_emperor_profile(
        players: &[crate::player::Player],
        teams: &[Vec<PlayerId>],
    ) -> Result<(Vec<PlayerId>, Vec<PlayerId>, Vec<u8>), String> {
        let Some(team_size) = teams.first().map(Vec::len) else {
            return Err("Emperor requires at least two teams".into());
        };
        if teams.len() < 2 || team_size < 3 || teams.iter().any(|team| team.len() != team_size) {
            return Err(
                "Emperor requires at least two equally sized teams of three or more".into(),
            );
        }
        let seats = teams.iter().flatten().copied().collect::<Vec<_>>();
        let distinct = seats.iter().copied().collect::<HashSet<_>>();
        let expected = players
            .iter()
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        if seats.len() != distinct.len() || distinct != expected {
            return Err("Emperor teams must contain every player exactly once".into());
        }

        let emperor_offset = (team_size - 1) / 2;
        let emperors = teams
            .iter()
            .map(|team| team[emperor_offset])
            .collect::<Vec<_>>();
        let seat_index = seats
            .iter()
            .copied()
            .enumerate()
            .map(|(index, player)| (player, index))
            .collect::<HashMap<_, _>>();
        let distance = |first: PlayerId, second: PlayerId| {
            let first = seat_index[&first];
            let second = seat_index[&second];
            let direct = first.abs_diff(second);
            direct.min(seats.len() - direct)
        };
        let emperor_set = emperors.iter().copied().collect::<HashSet<_>>();
        let mut ranges = Vec::with_capacity(seats.len());
        for player in &seats {
            let team = teams
                .iter()
                .position(|team| team.contains(player))
                .expect("validated Emperor team membership");
            let mut opposing_general_distances = teams
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != team)
                .flat_map(|(_, opponents)| opponents.iter().copied())
                .filter(|opponent| !emperor_set.contains(opponent))
                .map(|opponent| distance(*player, opponent))
                .collect::<Vec<_>>();
            opposing_general_distances.sort_unstable();
            let required_index = usize::from(emperor_set.contains(player));
            let range = opposing_general_distances
                .get(required_index)
                .copied()
                .ok_or_else(|| "Emperor seating has too few opposing generals".to_string())?;
            ranges.push(u8::try_from(range).map_err(|_| "Emperor range exceeds 255 seats")?);
        }
        for emperor in &emperors {
            let range = ranges[seat_index[emperor]];
            if emperors
                .iter()
                .any(|other| other != emperor && distance(*emperor, *other) <= usize::from(range))
            {
                return Err(
                    "Emperor seating puts one emperor within another emperor's range".into(),
                );
            }
        }
        Ok((seats, emperors, ranges))
    }

    pub fn emperor(&self) -> Option<&EmperorState> {
        self.emperor.as_ref()
    }

    pub fn is_emperor(&self, player: PlayerId) -> bool {
        self.emperor
            .as_ref()
            .is_some_and(|state| state.is_emperor(player))
    }

    pub(crate) fn emperor_team_members(&self, emperor: PlayerId) -> Option<Vec<PlayerId>> {
        let state = self.emperor.as_ref()?;
        let team = state.team_index(emperor)?;
        (state.emperors[team] == emperor).then(|| state.teams[team].clone())
    }

    pub(crate) fn emperor_attack_allows_defender(
        &self,
        attacker: PlayerId,
        defender: PlayerId,
    ) -> bool {
        let Some(state) = self.emperor.as_ref() else {
            return true;
        };
        let Some(index) = state.seats.iter().position(|player| *player == attacker) else {
            return false;
        };
        let left = state.seats[(index + 1) % state.seats.len()];
        let right = state.seats[(index + state.seats.len() - 1) % state.seats.len()];
        (defender == left || defender == right)
            && self.are_opponents(attacker, defender)
            && self
                .player(defender)
                .is_some_and(|player| player.is_in_game())
    }
}
