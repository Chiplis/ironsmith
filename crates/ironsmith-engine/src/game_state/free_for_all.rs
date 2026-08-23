use std::collections::HashSet;

use crate::ids::PlayerId;

use super::{AttackDirection, GameState};

/// The single combat topology selected for a CR 806 Free-for-All game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeForAllAttackOption {
    Left,
    Right,
    MultiplePlayers,
}

/// Stable pregame choices for the CR 806 Free-for-All variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeForAllState {
    pub(super) seats: Vec<PlayerId>,
    pub(super) attack_option: FreeForAllAttackOption,
    pub(super) range_of_influence: Option<u8>,
}

impl FreeForAllState {
    pub fn seats(&self) -> &[PlayerId] {
        &self.seats
    }

    pub fn attack_option(&self) -> FreeForAllAttackOption {
        self.attack_option
    }

    pub fn range_of_influence(&self) -> Option<u8> {
        self.range_of_influence
    }
}

impl GameState {
    /// Randomly seat every player, then atomically enable the CR 806 profile.
    pub fn enable_free_for_all(
        &mut self,
        attack_option: FreeForAllAttackOption,
        range_of_influence: Option<u8>,
    ) -> Result<(), String> {
        if self.players.len() < 3 {
            return Err("Free-for-All requires at least three players".into());
        }
        let mut seats = self
            .players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<_>>();
        self.shuffle_slice(&mut seats);
        self.restore_free_for_all(seats, attack_option, range_of_influence)
    }

    /// Restore already-randomized physical seats from a synchronized or
    /// restart/subgame boundary without consuming new randomness.
    pub fn restore_free_for_all(
        &mut self,
        seats: Vec<PlayerId>,
        attack_option: FreeForAllAttackOption,
        range_of_influence: Option<u8>,
    ) -> Result<(), String> {
        if self.players.len() < 3 {
            return Err("Free-for-All requires at least three players".into());
        }
        let known_players = self
            .players
            .iter()
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        let distinct_seats = seats.iter().copied().collect::<HashSet<_>>();
        if seats.len() != self.players.len()
            || distinct_seats.len() != seats.len()
            || distinct_seats != known_players
        {
            return Err("Free-for-All seats must contain every player exactly once".into());
        }

        let prepared_range = range_of_influence.map(|range| vec![range; seats.len()]);
        let attack_direction = match attack_option {
            FreeForAllAttackOption::Left => Some(AttackDirection::Left),
            FreeForAllAttackOption::Right => Some(AttackDirection::Right),
            FreeForAllAttackOption::MultiplePlayers => None,
        };

        // Every fallible check is complete before the live profile changes.
        self.teams = None;
        self.team_vs_team = None;
        self.emperor = None;
        self.two_headed_giant = None;
        self.alternating_teams = None;
        self.shared_team_turns = None;
        self.deploy_creatures = false;
        self.attack_direction = attack_direction;
        if let Some(ranges) = prepared_range {
            self.enable_limited_range_of_influence(seats.clone(), ranges)?;
        } else {
            self.range_of_influence = None;
        }
        self.free_for_all = Some(FreeForAllState {
            seats: seats.clone(),
            attack_option,
            range_of_influence,
        });

        // The first randomized seat is also a random starting player; turns
        // then proceed clockwise through the recorded seating order.
        self.turn_store.turn_order = seats;
        self.turn.active_player = self.turn_store.turn_order[0];
        self.turn.priority_player = Some(self.turn.active_player);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn free_for_all(&self) -> Option<&FreeForAllState> {
        self.free_for_all.as_ref()
    }

    /// Stable physical seating for multiplayer options. This remains distinct
    /// from the live turn order after a restart, subgame, or player departure.
    pub fn physical_seats(&self) -> &[PlayerId] {
        self.range_of_influence
            .as_ref()
            .map(|state| state.seats())
            .or_else(|| self.free_for_all.as_ref().map(|state| state.seats()))
            .or_else(|| self.team_vs_team.as_ref().map(|state| state.seats()))
            .or_else(|| self.alternating_teams.as_ref().map(|state| state.seats()))
            .or_else(|| self.shared_team_turns.as_ref().map(|state| state.seats()))
            .unwrap_or(self.turn_store.turn_order.as_slice())
    }
}
