use crate::ids::PlayerId;

use super::GameState;

/// CR 803 optional multiplayer attack direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackDirection {
    Left,
    Right,
}

impl GameState {
    pub fn set_attack_direction(&mut self, direction: Option<AttackDirection>) {
        if self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.alternating_teams.is_some()
        {
            return;
        }
        if let Some(profile) = self.free_for_all.as_ref() {
            let configured = match profile.attack_option {
                super::FreeForAllAttackOption::Left => Some(AttackDirection::Left),
                super::FreeForAllAttackOption::Right => Some(AttackDirection::Right),
                super::FreeForAllAttackOption::MultiplePlayers => None,
            };
            if direction != configured {
                return;
            }
        }
        if self.attack_direction == direction {
            return;
        }
        self.attack_direction = direction;
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }

    pub fn attack_direction(&self) -> Option<AttackDirection> {
        self.attack_direction
    }

    /// Return the physical adjacent seat dictated by CR 803. Unlike turn
    /// progression and CR 801.5c chooser fallback, this deliberately does not
    /// skip a player who has left the game: an opponent farther than one seat
    /// away is not a legal defender.
    pub fn adjacent_player_in_attack_direction(&self, attacker: PlayerId) -> Option<PlayerId> {
        let direction = self.attack_direction?;
        let seats = self.physical_seats();
        if seats.len() < 2 {
            return None;
        }
        let index = seats.iter().position(|player| *player == attacker)?;
        let adjacent = match direction {
            AttackDirection::Left => seats[(index + 1) % seats.len()],
            AttackDirection::Right => seats[(index + seats.len() - 1) % seats.len()],
        };
        self.player(adjacent)
            .is_some_and(|player| player.is_in_game() && adjacent != attacker)
            .then_some(adjacent)
    }

    /// Apply CR 803 to a defending player. The caller supplies a
    /// planeswalker's controller or a Battle's protector when applicable.
    pub fn attack_direction_allows_defender(
        &self,
        attacker: PlayerId,
        defending_player: PlayerId,
    ) -> bool {
        (self.attack_direction.is_none()
            || self.adjacent_player_in_attack_direction(attacker) == Some(defending_player))
            && self.emperor_attack_allows_defender(attacker, defending_player)
            && self.alternating_teams_attack_allows_defender(attacker, defending_player)
    }
}
