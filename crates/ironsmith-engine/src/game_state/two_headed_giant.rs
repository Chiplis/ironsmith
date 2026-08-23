use std::collections::HashSet;

use crate::ids::PlayerId;

use super::{GameState, TeamState};

/// Stable CR 810 teams, seats, first-team result, and shared-pool limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoHeadedGiantState {
    teams: Vec<Vec<PlayerId>>,
    seats: Vec<PlayerId>,
    starting_team: usize,
    starting_player: PlayerId,
    starting_life: i32,
    poison_threshold: u32,
}

impl TwoHeadedGiantState {
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

    pub fn starting_life(&self) -> i32 {
        self.starting_life
    }

    pub fn poison_threshold(&self) -> u32 {
        self.poison_threshold
    }

    pub fn team_index(&self, player: PlayerId) -> Option<usize> {
        self.teams.iter().position(|team| team.contains(&player))
    }
}

impl GameState {
    /// Enable CR 810, choosing the starting team from the match RNG.
    pub fn enable_two_headed_giant(&mut self, teams: Vec<Vec<PlayerId>>) -> Result<(), String> {
        let mut team_indices = vec![0usize, 1usize];
        self.shuffle_slice(&mut team_indices);
        let starting_team = team_indices[0];
        let starting_player = teams
            .get(starting_team)
            .and_then(|team| team.last())
            .copied()
            .ok_or_else(|| "Two-Headed Giant requires two nonempty teams".to_string())?;
        self.configure_two_headed_giant(teams, starting_team, starting_player, true)
    }

    /// Restore a serialized CR 810 profile without replacing current pools.
    pub fn restore_two_headed_giant(
        &mut self,
        teams: Vec<Vec<PlayerId>>,
        starting_team: usize,
        starting_player: PlayerId,
    ) -> Result<(), String> {
        self.configure_two_headed_giant(teams, starting_team, starting_player, false)
    }

    pub(crate) fn restore_two_headed_giant_new_game(
        &mut self,
        teams: Vec<Vec<PlayerId>>,
        starting_team: usize,
        starting_player: PlayerId,
    ) -> Result<(), String> {
        self.configure_two_headed_giant(teams, starting_team, starting_player, true)
    }

    fn configure_two_headed_giant(
        &mut self,
        teams: Vec<Vec<PlayerId>>,
        starting_team: usize,
        starting_player: PlayerId,
        initialize_pools: bool,
    ) -> Result<(), String> {
        let (seats, starting_life, poison_threshold) =
            Self::validate_two_headed_giant_profile(&self.players, &teams)?;
        if teams
            .get(starting_team)
            .and_then(|team| team.last())
            .copied()
            != Some(starting_player)
        {
            return Err(
                "the Two-Headed Giant starting player must be the starting team's primary player"
                    .into(),
            );
        }

        if !initialize_pools {
            for team in &teams {
                let Some(first) = self.player(team[0]) else {
                    return Err("Two-Headed Giant team contains an unknown player".into());
                };
                if team.iter().any(|player| {
                    self.player(*player).is_none_or(|candidate| {
                        candidate.life != first.life
                            || candidate.poison_counters != first.poison_counters
                    })
                }) {
                    return Err(
                        "Two-Headed Giant checkpoint members must agree on shared life and poison"
                            .into(),
                    );
                }
            }
        }

        self.free_for_all = None;
        self.grand_melee = None;
        self.team_vs_team = None;
        self.emperor = None;
        self.alternating_teams = None;
        self.two_headed_giant = None;
        self.range_of_influence = None;
        self.attack_direction = None;
        self.shared_team_turns = None;
        self.deploy_creatures = false;
        self.teams = Some(TeamState {
            teams: teams.clone(),
        });

        if initialize_pools {
            for player in &mut self.players {
                player.starting_life = starting_life;
                player.life = starting_life;
                player.poison_counters = 0;
            }
        }

        let mut turn_order = seats.clone();
        let start = turn_order
            .iter()
            .position(|player| *player == starting_player)
            .expect("validated Two-Headed Giant starting player");
        turn_order.rotate_left(start);
        self.turn_store.turn_order = turn_order;
        self.turn.active_player = starting_player;
        self.turn.priority_player = Some(starting_player);
        self.enable_shared_team_turns()?;
        self.two_headed_giant = Some(TwoHeadedGiantState {
            teams,
            seats,
            starting_team,
            starting_player,
            starting_life,
            poison_threshold,
        });
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    fn validate_two_headed_giant_profile(
        players: &[crate::player::Player],
        teams: &[Vec<PlayerId>],
    ) -> Result<(Vec<PlayerId>, i32, u32), String> {
        let Some(team_size) = teams.first().map(Vec::len) else {
            return Err("Two-Headed Giant requires exactly two teams".into());
        };
        if teams.len() != 2 || team_size < 2 || teams.iter().any(|team| team.len() != team_size) {
            return Err(
                "Two-Headed Giant requires exactly two equally sized teams of two or more".into(),
            );
        }
        let seats = teams.iter().flatten().copied().collect::<Vec<_>>();
        let distinct = seats.iter().copied().collect::<HashSet<_>>();
        let expected = players
            .iter()
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        if seats.len() != distinct.len() || distinct != expected {
            return Err("Two-Headed Giant teams must contain every player exactly once".into());
        }
        let additional_heads =
            u32::try_from(team_size - 2).map_err(|_| "Two-Headed Giant team size exceeds u32")?;
        let starting_life = 30_i32
            .saturating_add(i32::try_from(additional_heads.saturating_mul(15)).unwrap_or(i32::MAX));
        let poison_threshold = 15_u32.saturating_add(additional_heads.saturating_mul(5));
        Ok((seats, starting_life, poison_threshold))
    }

    pub fn two_headed_giant(&self) -> Option<&TwoHeadedGiantState> {
        self.two_headed_giant.as_ref()
    }

    pub(crate) fn two_headed_giant_team_members(&self, player: PlayerId) -> Option<Vec<PlayerId>> {
        let state = self.two_headed_giant.as_ref()?;
        Some(state.teams[state.team_index(player)?].clone())
    }

    pub(crate) fn write_shared_life(&mut self, player: PlayerId, life: i32) -> bool {
        let members = self
            .two_headed_giant_team_members(player)
            .unwrap_or_else(|| vec![player]);
        let mut found = false;
        for member in members {
            if let Some(candidate) = self.player_mut(member) {
                candidate.life = life;
                found = true;
            }
        }
        found
    }

    pub(crate) fn write_shared_poison(&mut self, player: PlayerId, poison: u32) -> bool {
        let members = self
            .two_headed_giant_team_members(player)
            .unwrap_or_else(|| vec![player]);
        let mut found = false;
        for member in members {
            if let Some(candidate) = self.player_mut(member) {
                candidate.poison_counters = poison;
                found = true;
            }
        }
        found
    }

    pub(crate) fn two_headed_giant_poison_threshold(&self, player: PlayerId) -> Option<u32> {
        self.two_headed_giant
            .as_ref()
            .and_then(|state| state.team_index(player).map(|_| state.poison_threshold))
    }
}
