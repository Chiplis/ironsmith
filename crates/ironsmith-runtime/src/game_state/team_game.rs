use std::collections::HashSet;

use crate::ability::{Ability, ActivationTiming};
use crate::continuous::{CalculatedCharacteristics, EffectTarget};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{Effect, Until};
use crate::effects::TargetOnlyEffect;
use crate::effects::continuous::{ApplyContinuousEffect, RuntimeModification};
use crate::ids::PlayerId;
use crate::object::Object;
use crate::resolution::ResolutionProgram;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::GameState;

/// Explicit multiplayer team membership. Each nested vector is one team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamState {
    pub(super) teams: Vec<Vec<PlayerId>>,
}

impl TeamState {
    pub fn teams(&self) -> &[Vec<PlayerId>] {
        &self.teams
    }

    pub fn team_index(&self, player: PlayerId) -> Option<usize> {
        self.teams.iter().position(|team| team.contains(&player))
    }
}

/// Derived CR 805 identity for a match using shared team turns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedTeamTurnsState {
    seats: Vec<PlayerId>,
    team_order: Vec<usize>,
    primary_players: Vec<PlayerId>,
    member_orders: Vec<Vec<PlayerId>>,
}

impl SharedTeamTurnsState {
    pub fn seats(&self) -> &[PlayerId] {
        &self.seats
    }

    pub fn team_order(&self) -> &[usize] {
        &self.team_order
    }

    pub fn primary_players(&self) -> &[PlayerId] {
        &self.primary_players
    }

    /// Current team-selected order for choices/actions within each team.
    pub fn member_orders(&self) -> &[Vec<PlayerId>] {
        &self.member_orders
    }
}

fn deploy_creatures_ability() -> Ability {
    let teammate_target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Teammate));
    let declare_target = Effect::new(TargetOnlyEffect::new(teammate_target));
    let transfer_control = Effect::new(ApplyContinuousEffect::new_runtime(
        EffectTarget::Source,
        RuntimeModification::ChangeControllerToPlayer(PlayerFilter::Target(Box::new(
            PlayerFilter::Teammate,
        ))),
        Until::Forever,
    ));

    Ability::activated_with_timing(
        TotalCost::from_cost(Cost::tap()),
        ResolutionProgram::from_effects(vec![declare_target, transfer_control]),
        ActivationTiming::SorcerySpeed,
    )
}

impl GameState {
    /// Configure reusable team identity. Every game player must appear in
    /// exactly one nonempty team; singleton teams are permitted.
    pub fn set_teams(&mut self, teams: Vec<Vec<PlayerId>>) -> Result<(), String> {
        if self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.alternating_teams.is_some()
        {
            return Err("the active team-format profile fixes teams before play begins".into());
        }
        if self.free_for_all.is_some() {
            return Err("Free-for-All players compete individually and cannot form teams".into());
        }
        if teams.is_empty() || teams.iter().any(Vec::is_empty) {
            return Err("team setup requires at least one nonempty team".into());
        }

        let configured = teams.iter().flatten().copied().collect::<Vec<_>>();
        let distinct = configured.iter().copied().collect::<HashSet<_>>();
        let all_players = self
            .players
            .iter()
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        if configured.len() != distinct.len() || distinct != all_players {
            return Err("teams must contain every game player exactly once".into());
        }

        self.shared_team_turns = None;
        self.teams = Some(TeamState { teams });
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn clear_teams(&mut self) {
        if self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.alternating_teams.is_some()
        {
            return;
        }
        self.shared_team_turns = None;
        if self.teams.take().is_some() {
            self.bump_mutation_revision();
            self.mark_continuous_state_dirty();
        }
    }

    pub fn team_state(&self) -> Option<&TeamState> {
        self.teams.as_ref()
    }

    pub fn are_teammates(&self, first: PlayerId, second: PlayerId) -> bool {
        first != second
            && self.teams.as_ref().is_some_and(|state| {
                state.team_index(first).is_some()
                    && state.team_index(first) == state.team_index(second)
            })
    }

    /// With no team configuration, every other player is an opponent. With
    /// teams configured, only players assigned to different teams are.
    pub fn are_opponents(&self, first: PlayerId, second: PlayerId) -> bool {
        if first == second {
            return false;
        }
        self.teams.as_ref().map_or(true, |state| {
            state
                .team_index(first)
                .zip(state.team_index(second))
                .is_some_and(|(first_team, second_team)| first_team != second_team)
        })
    }

    /// Permanent team-scoped hand-review permission supplied by formats whose
    /// rules explicitly grant it. This is information access, not control.
    pub fn can_review_teammate_hand(&self, viewer: PlayerId, owner: PlayerId) -> bool {
        ((self.team_vs_team.is_some() || self.emperor.is_some() || self.two_headed_giant.is_some())
            && self.are_teammates(viewer, owner))
            || self.alternating_teams_adjacent_teammates(viewer, owner)
    }

    fn physical_seats_for_team_options(&self) -> &[PlayerId] {
        self.limited_range_of_influence()
            .map(|state| state.seats())
            .unwrap_or(self.turn_store.turn_order.as_slice())
    }

    /// Enable CR 805. Team members must occupy one circularly contiguous seat
    /// block. The final player in each block is its rightmost/primary player.
    pub fn enable_shared_team_turns(&mut self) -> Result<(), String> {
        if self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.alternating_teams.is_some()
        {
            return Err("the active team format uses individual turns".into());
        }
        let teams = self
            .teams
            .as_ref()
            .ok_or_else(|| "shared team turns requires configured teams".to_string())?;
        if teams.teams.len() < 2 {
            return Err("shared team turns requires at least two teams".into());
        }

        let seats = self.physical_seats_for_team_options().to_vec();
        if seats.len() != self.players.len() {
            return Err("shared team turns requires one physical seat per player".into());
        }
        let seat_teams = seats
            .iter()
            .map(|player| {
                teams.team_index(*player).ok_or_else(|| {
                    "shared team turns seat contains an unassigned player".to_string()
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut starts = vec![0usize; teams.teams.len()];
        for index in 0..seat_teams.len() {
            let previous = seat_teams[(index + seat_teams.len() - 1) % seat_teams.len()];
            if previous != seat_teams[index] {
                starts[seat_teams[index]] += 1;
            }
        }
        if starts.iter().any(|count| *count != 1) {
            return Err("members of every shared-turn team must sit in adjacent seats".into());
        }

        let mut primary_players = vec![seats[0]; teams.teams.len()];
        let mut team_order = Vec::with_capacity(teams.teams.len());
        for index in 0..seat_teams.len() {
            let next = (index + 1) % seat_teams.len();
            if seat_teams[index] != seat_teams[next] {
                let team = seat_teams[index];
                primary_players[team] = seats[index];
                team_order.push(seat_teams[next]);
            }
        }
        let active_team = teams
            .team_index(self.turn.active_player)
            .ok_or_else(|| "active player is not assigned to a shared-turn team".to_string())?;
        if let Some(index) = team_order.iter().position(|team| *team == active_team) {
            team_order.rotate_left(index);
        }

        let configured_active_primary = primary_players[active_team];
        let active_primary = if self
            .player(configured_active_primary)
            .is_some_and(|player| player.is_in_game())
        {
            configured_active_primary
        } else {
            seats
                .iter()
                .rev()
                .copied()
                .find(|player| {
                    teams.team_index(*player) == Some(active_team)
                        && self
                            .player(*player)
                            .is_some_and(|candidate| candidate.is_in_game())
                })
                .expect("the active shared-turn team has an in-game player")
        };
        let member_orders = (0..teams.teams.len())
            .map(|team| {
                seats
                    .iter()
                    .copied()
                    .filter(|player| teams.team_index(*player) == Some(team))
                    .collect()
            })
            .collect();
        self.shared_team_turns = Some(SharedTeamTurnsState {
            seats,
            team_order,
            primary_players,
            member_orders,
        });
        self.turn.active_player = active_primary;
        if self.turn.priority_player.is_some() {
            self.turn.priority_player = Some(active_primary);
        }
        self.bump_mutation_revision();
        Ok(())
    }

    pub fn disable_shared_team_turns(&mut self) {
        if self.two_headed_giant.is_some() {
            return;
        }
        if self.shared_team_turns.take().is_some() {
            self.bump_mutation_revision();
            self.reset_priority_for_new_window();
        }
    }

    pub fn shared_team_turns(&self) -> Option<&SharedTeamTurnsState> {
        self.shared_team_turns.as_ref()
    }

    pub fn shared_team_turns_enabled(&self) -> bool {
        self.shared_team_turns.is_some()
    }

    /// Record the order selected by a team for its simultaneous choices or
    /// actions (805.3a–b, 805.6–.7). Callers may update this before each batch;
    /// the physical seat order remains the deterministic primary fallback.
    pub fn set_shared_team_member_order(
        &mut self,
        team: usize,
        order: Vec<PlayerId>,
    ) -> Result<(), String> {
        let expected = self
            .teams
            .as_ref()
            .and_then(|state| state.teams.get(team))
            .ok_or_else(|| "shared-turn team index is out of range".to_string())?;
        if order.len() != expected.len()
            || order.iter().copied().collect::<HashSet<_>>()
                != expected.iter().copied().collect::<HashSet<_>>()
        {
            return Err("team member order must contain every member exactly once".into());
        }
        let shared = self
            .shared_team_turns
            .as_mut()
            .ok_or_else(|| "shared team turns is not enabled".to_string())?;
        shared.member_orders[team] = order;
        self.bump_mutation_revision();
        Ok(())
    }

    pub fn team_index_for(&self, player: PlayerId) -> Option<usize> {
        self.teams.as_ref()?.team_index(player)
    }

    /// Primary-player fallback, retaining the original seat direction if the
    /// printed primary player has left the game.
    pub fn primary_player_for_team(&self, team: usize) -> Option<PlayerId> {
        let shared = self.shared_team_turns.as_ref()?;
        let configured = *shared.primary_players.get(team)?;
        if self
            .player(configured)
            .is_some_and(|player| player.is_in_game())
        {
            return Some(configured);
        }
        shared.seats.iter().rev().copied().find(|player| {
            self.team_index_for(*player) == Some(team)
                && self
                    .player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
        })
    }

    pub fn primary_player_for(&self, player: PlayerId) -> Option<PlayerId> {
        self.primary_player_for_team(self.team_index_for(player)?)
    }

    pub fn active_team_index(&self) -> Option<usize> {
        self.shared_team_turns
            .as_ref()
            .and_then(|_| self.team_index_for(self.turn.active_player))
    }

    pub fn active_players(&self) -> Vec<PlayerId> {
        if self.grand_melee.is_some() {
            return self.grand_melee_active_players();
        }
        self.turn_players()
    }

    /// Players sharing the currently selected turn lane. This is distinct
    /// from `active_players` only in Grand Melee, where several independent
    /// active turns coexist but a step belongs to exactly one numbered marker.
    pub fn turn_players(&self) -> Vec<PlayerId> {
        let Some(active_team) = self.active_team_index() else {
            return self.active_player_id().into_iter().collect();
        };
        self.shared_team_turns
            .as_ref()
            .into_iter()
            .flat_map(|state| state.member_orders[active_team].iter().copied())
            .filter(|player| self.team_index_for(*player) == Some(active_team))
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .collect()
    }

    pub fn is_active_player(&self, player: PlayerId) -> bool {
        if self.grand_melee.is_some() {
            return self.grand_melee_active_players().contains(&player);
        }
        if let Some(active_team) = self.active_team_index() {
            self.team_index_for(player) == Some(active_team)
                && self
                    .player(player)
                    .is_some_and(|candidate| candidate.is_in_game())
        } else {
            self.active_player_id() == Some(player)
        }
    }

    /// Resolve a singular CR 805.9 "active player" reference. The effect
    /// controller's explicit selection wins when it names an active teammate;
    /// otherwise the primary player is the deterministic rules fallback.
    pub fn singular_active_player(&self, selected: Option<PlayerId>) -> Option<PlayerId> {
        selected
            .filter(|player| self.is_active_player(*player))
            .or_else(|| self.active_player_id())
    }

    pub fn team_turn_representative(&self, player: PlayerId) -> PlayerId {
        self.primary_player_for(player).unwrap_or(player)
    }

    /// In-game members of `player`'s team in the team's currently selected
    /// internal order. Outside shared-team-turn games this is just `player`.
    pub fn team_players_for(&self, player: PlayerId) -> Vec<PlayerId> {
        let Some(shared) = self.shared_team_turns.as_ref() else {
            return self
                .player(player)
                .filter(|candidate| candidate.is_in_game())
                .map(|_| vec![player])
                .unwrap_or_default();
        };
        let Some(team) = self.team_index_for(player) else {
            return Vec::new();
        };
        shared.member_orders[team]
            .iter()
            .copied()
            .filter(|member| {
                self.player(*member)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .collect()
    }

    pub fn priority_team_index(&self) -> Option<usize> {
        let priority = self.turn.priority_player?;
        self.shared_team_turns
            .as_ref()
            .and_then(|_| self.team_index_for(priority))
    }

    pub fn team_has_priority(&self, player: PlayerId) -> bool {
        if let Some(priority_team) = self.priority_team_index() {
            self.team_index_for(player) == Some(priority_team)
                && self
                    .player(player)
                    .is_some_and(|candidate| candidate.is_in_game())
        } else {
            self.turn.priority_player == Some(player)
        }
    }

    pub fn priority_team_players(&self) -> Vec<PlayerId> {
        let Some(priority_team) = self.priority_team_index() else {
            return self.turn.priority_player.into_iter().collect();
        };
        let mut players = self
            .shared_team_turns
            .as_ref()
            .into_iter()
            .flat_map(|state| state.member_orders[priority_team].iter().copied())
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .collect::<Vec<_>>();
        if let Some(primary) = self.primary_player_for_team(priority_team)
            && let Some(index) = players.iter().position(|player| *player == primary)
        {
            players.rotate_left(index);
        }
        players
    }

    pub fn teams_in_game(&self) -> usize {
        let Some(shared) = self.shared_team_turns.as_ref() else {
            return self.players_in_game();
        };
        shared
            .team_order
            .iter()
            .filter(|team| self.primary_player_for_team(**team).is_some())
            .count()
    }

    pub fn team_apnap_player_order(&self) -> Vec<PlayerId> {
        let Some(shared) = self.shared_team_turns.as_ref() else {
            let len = self.turn_store.turn_order.len();
            let start = self
                .turn_store
                .turn_order
                .iter()
                .position(|player| *player == self.turn.active_player)
                .unwrap_or(0);
            return (0..len)
                .map(|offset| self.turn_store.turn_order[(start + offset) % len])
                .filter(|player| {
                    self.player(*player)
                        .is_some_and(|candidate| candidate.is_in_game())
                })
                .collect();
        };
        let active_team = self.active_team_index().unwrap_or(0);
        let mut teams = shared.team_order.clone();
        if let Some(index) = teams.iter().position(|team| *team == active_team) {
            teams.rotate_left(index);
        }
        teams
            .into_iter()
            .flat_map(|team| shared.member_orders[team].iter().copied())
            .filter(|player| {
                self.player(*player)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
            .collect()
    }

    pub(crate) fn next_team_turn_representative_after(&self, player: PlayerId) -> Option<PlayerId> {
        let shared = self.shared_team_turns.as_ref()?;
        let current_team = self.team_index_for(player)?;
        let current = shared
            .team_order
            .iter()
            .position(|team| *team == current_team)
            .unwrap_or(0);
        (1..=shared.team_order.len()).find_map(|offset| {
            let team = shared.team_order[(current + offset) % shared.team_order.len()];
            self.primary_player_for_team(team)
        })
    }

    pub(crate) fn next_priority_team_representative_after(
        &self,
        player: PlayerId,
    ) -> Option<PlayerId> {
        self.next_team_turn_representative_after(player)
    }

    pub(crate) fn consume_team_turn_skip(&mut self, player: PlayerId) -> bool {
        let Some(team) = self
            .shared_team_turns
            .as_ref()
            .and_then(|_| self.team_index_for(player))
        else {
            return self.turn_store.skip_next_turn.remove(&player);
        };
        let Some(key) = self
            .turn_store
            .skip_next_turn
            .iter()
            .copied()
            .find(|candidate| self.team_index_for(*candidate) == Some(team))
        else {
            return false;
        };
        self.turn_store.skip_next_turn.remove(&key)
    }

    pub fn set_deploy_creatures(&mut self, enabled: bool) {
        if self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.alternating_teams.is_some()
            || enabled && self.free_for_all.is_some()
        {
            return;
        }
        if self.deploy_creatures != enabled {
            self.deploy_creatures = enabled;
            self.bump_mutation_revision();
            self.mark_continuous_state_dirty();
        }
    }

    pub fn deploy_creatures_enabled(&self) -> bool {
        self.deploy_creatures
    }

    /// CR 804.2 is a game-rule ability grant in layer 6. This hook runs after
    /// type changes, so animated permanents qualify, and before ordinary
    /// layer-6 effects, so an applicable ability-removing effect can remove it.
    pub(crate) fn apply_deploy_creatures_ability_layer(
        &self,
        object: &Object,
        chars: &mut CalculatedCharacteristics,
    ) {
        if self.deploy_creatures
            && object.zone == Zone::Battlefield
            && chars.card_types.contains(&CardType::Creature)
        {
            chars.abilities.push(deploy_creatures_ability());
        }
    }
}
