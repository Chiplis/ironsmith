use std::collections::HashSet;

use crate::ids::PlayerId;

use super::{GameState, TeamState};

/// Stable CR 808 Team vs. Team seating and starting-player result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamVsTeamState {
    teams: Vec<Vec<PlayerId>>,
    seats: Vec<PlayerId>,
    starting_team: usize,
    starting_player: PlayerId,
}

impl TeamVsTeamState {
    pub fn teams(&self) -> &[Vec<PlayerId>] {
        &self.teams
    }

    pub fn seats(&self) -> &[PlayerId] {
        &self.seats
    }

    pub fn starting_team(&self) -> usize {
        self.starting_team
    }

    pub fn starting_player(&self) -> PlayerId {
        self.starting_player
    }
}

impl GameState {
    /// Enable CR 808, choosing the starting team from the match RNG and then
    /// applying that team's center/left-of-midpoint starting seat.
    pub fn enable_team_vs_team(&mut self, teams: Vec<Vec<PlayerId>>) -> Result<(), String> {
        Self::validate_team_vs_team_members(&self.players, &teams)?;
        let mut team_indices = (0..teams.len()).collect::<Vec<_>>();
        self.shuffle_slice(&mut team_indices);
        let starting_team = team_indices[0];
        let starting_player = teams[starting_team][(teams[starting_team].len() - 1) / 2];
        let seats = teams.iter().flatten().copied().collect();
        self.restore_team_vs_team(teams, seats, starting_team, starting_player)
    }

    /// Restore an already-selected CR 808 profile without consuming random
    /// state at restart, subgame, or synchronized-checkpoint boundaries.
    pub fn restore_team_vs_team(
        &mut self,
        teams: Vec<Vec<PlayerId>>,
        seats: Vec<PlayerId>,
        starting_team: usize,
        starting_player: PlayerId,
    ) -> Result<(), String> {
        Self::validate_team_vs_team_members(&self.players, &teams)?;
        if seats != teams.iter().flatten().copied().collect::<Vec<_>>() {
            return Err(
                "Team vs. Team seats must preserve each team's selected contiguous order".into(),
            );
        }
        if teams.get(starting_team).is_none() || !teams[starting_team].contains(&starting_player) {
            return Err("Team vs. Team starting player must belong to the starting team".into());
        }

        let mut turn_order = seats.clone();
        let start = turn_order
            .iter()
            .position(|player| *player == starting_player)
            .expect("validated Team vs. Team starting player");
        turn_order.rotate_left(start);

        self.free_for_all = None;
        self.grand_melee = None;
        self.emperor = None;
        self.two_headed_giant = None;
        self.alternating_teams = None;
        self.range_of_influence = None;
        self.attack_direction = None;
        self.deploy_creatures = false;
        self.shared_team_turns = None;
        self.teams = Some(TeamState {
            teams: teams.clone(),
        });
        self.team_vs_team = Some(TeamVsTeamState {
            teams,
            seats,
            starting_team,
            starting_player,
        });
        self.turn_store.turn_order = turn_order;
        self.turn.active_player = starting_player;
        self.turn.priority_player = Some(starting_player);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    fn validate_team_vs_team_members(
        players: &[crate::player::Player],
        teams: &[Vec<PlayerId>],
    ) -> Result<(), String> {
        if teams.len() < 2 || teams.iter().any(Vec::is_empty) {
            return Err("Team vs. Team requires at least two nonempty teams".into());
        }
        let configured = teams.iter().flatten().copied().collect::<Vec<_>>();
        let distinct = configured.iter().copied().collect::<HashSet<_>>();
        let expected = players
            .iter()
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        if configured.len() != distinct.len() || distinct != expected {
            return Err("Team vs. Team teams must contain every player exactly once".into());
        }
        Ok(())
    }

    pub fn team_vs_team(&self) -> Option<&TeamVsTeamState> {
        self.team_vs_team.as_ref()
    }

    /// CR 104.2c winners when exactly one configured team still has a player
    /// in the game. The full original team is returned, including teammates
    /// who previously lost.
    pub fn sole_surviving_team_winners(&self) -> Option<Vec<PlayerId>> {
        let teams = self.teams.as_ref()?;
        if teams.teams.len() < 2 {
            return None;
        }
        let live_teams = teams
            .teams
            .iter()
            .enumerate()
            .filter(|(_, team)| {
                team.iter().any(|player| {
                    self.player(*player)
                        .is_some_and(|candidate| candidate.is_in_game())
                })
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        (live_teams.len() == 1).then(|| teams.teams[live_teams[0]].clone())
    }

    pub(crate) fn mark_team_winner(&mut self, player: PlayerId) {
        if let Some(current) = self.players.get_mut(player.index()) {
            current.has_won = true;
        }
        if let Some(history) = self.turn_store.departed_player_history.get_mut(&player) {
            history.player_lki.has_won = true;
        }
    }
}
