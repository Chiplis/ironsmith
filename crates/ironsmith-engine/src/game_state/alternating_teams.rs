use std::collections::HashSet;

use crate::ids::PlayerId;

use super::{AttackDirection, FreeForAllAttackOption, GameState, TeamState};

/// Stable CR 811 teams, physical seats, and pregame multiplayer options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternatingTeamsState {
    teams: Vec<Vec<PlayerId>>,
    seats: Vec<PlayerId>,
    starting_player: PlayerId,
    attack_option: FreeForAllAttackOption,
    range_of_influence: Option<u8>,
    deploy_creatures: bool,
}

impl AlternatingTeamsState {
    pub fn teams(&self) -> &[Vec<PlayerId>] {
        &self.teams
    }

    pub fn seats(&self) -> &[PlayerId] {
        &self.seats
    }

    pub fn starting_player(&self) -> PlayerId {
        self.starting_player
    }

    pub fn attack_option(&self) -> FreeForAllAttackOption {
        self.attack_option
    }

    pub fn range_of_influence(&self) -> Option<u8> {
        self.range_of_influence
    }

    pub fn deploy_creatures(&self) -> bool {
        self.deploy_creatures
    }
}

impl GameState {
    /// Enable CR 811 and select a random starting player without changing the
    /// round-robin physical seating derived from the teams' chosen orders.
    pub fn enable_alternating_teams(
        &mut self,
        teams: Vec<Vec<PlayerId>>,
        attack_option: FreeForAllAttackOption,
        range_of_influence: Option<u8>,
        deploy_creatures: bool,
    ) -> Result<(), String> {
        let seats = Self::validate_alternating_teams_members(&self.players, &teams)?;
        let mut starting_candidates = seats.clone();
        self.shuffle_slice(&mut starting_candidates);
        self.restore_alternating_teams(
            teams,
            seats,
            starting_candidates[0],
            attack_option,
            range_of_influence,
            deploy_creatures,
        )
    }

    /// Restore an already-selected CR 811 profile without consuming RNG.
    pub fn restore_alternating_teams(
        &mut self,
        teams: Vec<Vec<PlayerId>>,
        seats: Vec<PlayerId>,
        starting_player: PlayerId,
        attack_option: FreeForAllAttackOption,
        range_of_influence: Option<u8>,
        deploy_creatures: bool,
    ) -> Result<(), String> {
        let expected_seats = Self::validate_alternating_teams_members(&self.players, &teams)?;
        if seats != expected_seats {
            return Err(
                "Alternating Teams seats must repeat each team's selected member order".into(),
            );
        }
        if !seats.contains(&starting_player) {
            return Err("Alternating Teams starting player must occupy a configured seat".into());
        }
        if range_of_influence == Some(0) {
            return Err("Alternating Teams range of influence must be positive".into());
        }

        let attack_direction = match attack_option {
            FreeForAllAttackOption::Left => Some(AttackDirection::Left),
            FreeForAllAttackOption::Right => Some(AttackDirection::Right),
            FreeForAllAttackOption::MultiplePlayers => None,
        };
        let mut turn_order = seats.clone();
        let start = turn_order
            .iter()
            .position(|player| *player == starting_player)
            .expect("validated Alternating Teams starting player");
        turn_order.rotate_left(start);

        // Finish every fallible profile check before changing live state.
        self.free_for_all = None;
        self.grand_melee = None;
        self.team_vs_team = None;
        self.emperor = None;
        self.two_headed_giant = None;
        self.alternating_teams = None;
        self.shared_team_turns = None;
        self.teams = Some(TeamState {
            teams: teams.clone(),
        });
        self.attack_direction = attack_direction;
        self.deploy_creatures = deploy_creatures;
        if let Some(range) = range_of_influence {
            self.enable_limited_range_of_influence(seats.clone(), vec![range; seats.len()])?;
        } else {
            self.range_of_influence = None;
        }
        self.alternating_teams = Some(AlternatingTeamsState {
            teams,
            seats,
            starting_player,
            attack_option,
            range_of_influence,
            deploy_creatures,
        });
        self.turn_store.turn_order = turn_order;
        self.turn.active_player = starting_player;
        self.turn.priority_player = Some(starting_player);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    fn validate_alternating_teams_members(
        players: &[crate::player::Player],
        teams: &[Vec<PlayerId>],
    ) -> Result<Vec<PlayerId>, String> {
        let Some(team_size) = teams.first().map(Vec::len) else {
            return Err("Alternating Teams requires at least two teams".into());
        };
        if teams.len() < 2 || team_size == 0 || teams.iter().any(|team| team.len() != team_size) {
            return Err("Alternating Teams requires at least two equal nonempty teams".into());
        }
        let configured = teams.iter().flatten().copied().collect::<Vec<_>>();
        let distinct = configured.iter().copied().collect::<HashSet<_>>();
        let expected = players
            .iter()
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        if configured.len() != distinct.len() || distinct != expected {
            return Err("Alternating Teams teams must contain every player exactly once".into());
        }

        let mut seats = Vec::with_capacity(configured.len());
        for member in 0..team_size {
            for team in teams {
                seats.push(team[member]);
            }
        }
        Ok(seats)
    }

    pub fn alternating_teams(&self) -> Option<&AlternatingTeamsState> {
        self.alternating_teams.as_ref()
    }

    pub(crate) fn alternating_teams_attack_allows_defender(
        &self,
        attacker: PlayerId,
        defending_player: PlayerId,
    ) -> bool {
        let Some(profile) = self.alternating_teams.as_ref() else {
            return true;
        };
        if !self.are_opponents(attacker, defending_player) {
            return false;
        }
        let Some(index) = profile.seats.iter().position(|player| *player == attacker) else {
            return false;
        };
        let left = profile.seats[(index + 1) % profile.seats.len()];
        let right = profile.seats[(index + profile.seats.len() - 1) % profile.seats.len()];
        (defending_player == left || defending_player == right)
            && self
                .player(defending_player)
                .is_some_and(|player| player.is_in_game())
    }

    pub(crate) fn alternating_teams_adjacent_teammates(
        &self,
        first: PlayerId,
        second: PlayerId,
    ) -> bool {
        if !self.are_teammates(first, second) {
            return false;
        }
        let Some(profile) = self.alternating_teams.as_ref() else {
            return false;
        };
        let Some(index) = profile.seats.iter().position(|player| *player == first) else {
            return false;
        };
        profile.seats[(index + 1) % profile.seats.len()] == second
            || profile.seats[(index + profile.seats.len() - 1) % profile.seats.len()] == second
    }
}
