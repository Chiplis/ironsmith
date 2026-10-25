use crate::ids::{ObjectId, PlayerId, StableId};

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
            && self.chosen_defending_player_allows(attacker, defending_player)
            && self.emperor_attack_allows_defender(attacker, defending_player)
            && self.alternating_teams_attack_allows_defender(attacker, defending_player)
    }

    /// Enable or disable the CR 802 attack multiple players option.
    pub fn set_attack_multiple_players(&mut self, enabled: bool) {
        if self.attack_multiple_players != enabled {
            self.attack_multiple_players = enabled;
            self.bump_mutation_revision();
        }
    }

    /// CR 802 / 806.2b / 808.3a / 904.2 etc.: whether all of the attacking
    /// player's opponents automatically become defending players. Variants
    /// whose rules use the attack multiple players option imply it.
    pub fn attack_multiple_players_enabled(&self) -> bool {
        self.attack_multiple_players
            || self.free_for_all.as_ref().is_some_and(|profile| {
                profile.attack_option == super::FreeForAllAttackOption::MultiplePlayers
            })
            || self.alternating_teams.as_ref().is_some_and(|profile| {
                profile.attack_option() == super::FreeForAllAttackOption::MultiplePlayers
            })
            || self.team_vs_team.is_some()
            || self.emperor.is_some()
            || self.two_headed_giant.is_some()
            || self.shared_team_turns_enabled()
            || self.archenemy.is_some()
            || self.planechase.is_some()
            || self.conspiracy.is_some()
    }

    /// CR 506.2a / 507.1 / 703.4h: the opponents the active player may choose
    /// as the defending player at the beginning of combat, or `None` when no
    /// choice is made (two-player game, attack multiple players, or a CR 803
    /// direction that already dictates the defender).
    pub fn defending_player_choice_candidates(&self) -> Option<Vec<PlayerId>> {
        if self.attack_multiple_players_enabled() || self.attack_direction.is_some() {
            return None;
        }
        let active = self.turn.active_player;
        let candidates: Vec<PlayerId> = self
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != active)
            .filter(|player| self.are_opponents(active, player.id))
            .map(|player| player.id)
            .collect();
        (candidates.len() > 1).then_some(candidates)
    }

    /// Record the CR 507.1 defending player for the current combat phase.
    pub fn set_chosen_defending_player(&mut self, player: Option<PlayerId>) {
        self.chosen_defending_player = player.map(|player| {
            (
                self.turn.turn_number,
                self.turn_store.combat_phases_started_this_turn,
                player,
            )
        });
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }

    /// The defending player chosen for the current combat phase, if any.
    pub fn chosen_defending_player(&self) -> Option<PlayerId> {
        self.chosen_defending_player
            .filter(|(turn, combat, _)| {
                *turn == self.turn.turn_number
                    && *combat == self.turn_store.combat_phases_started_this_turn
            })
            .map(|(_, _, player)| player)
    }

    /// CR 508.1b / 508.7d: without the attack multiple players option only
    /// the chosen defending player (and their planeswalkers/battles) can be
    /// attacked by the active player.
    fn chosen_defending_player_allows(&self, attacker: PlayerId, defending_player: PlayerId) -> bool {
        if attacker != self.turn.active_player || self.attack_multiple_players_enabled() {
            return true;
        }
        self.chosen_defending_player()
            .is_none_or(|chosen| chosen == defending_player)
    }

    /// Cheap and full fingerprints of the visible game state used to
    /// recognize a game state that has been reached before (CR 732.3). The
    /// full one includes current characteristics and is computed only when
    /// the cheap one matches.
    fn fragmented_loop_cheap_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.turn.turn_number.hash(&mut hasher);
        format!("{:?}{:?}", self.turn.phase, self.turn.step).hash(&mut hasher);
        self.turn.active_player.hash(&mut hasher);
        for player in self.players.iter() {
            player.id.hash(&mut hasher);
            player.life.hash(&mut hasher);
            player.poison_counters.hash(&mut hasher);
            format!("{:?}", player.mana_pool).hash(&mut hasher);
            for id in player.hand.iter() {
                id.hash(&mut hasher);
            }
            for id in player.library.iter() {
                id.hash(&mut hasher);
            }
            for id in player.graveyard.iter() {
                id.hash(&mut hasher);
            }
        }
        for entry in self.stack.iter() {
            entry.object_id.hash(&mut hasher);
            entry.controller.hash(&mut hasher);
        }
        for &id in self.battlefield.iter() {
            let Some(object) = self.object(id) else {
                continue;
            };
            object.stable_id.hash(&mut hasher);
            self.controller_of(object).hash(&mut hasher);
            self.is_tapped(id).hash(&mut hasher);
            format!("{:?}", object.counters).hash(&mut hasher);
        }
        hasher.finish()
    }

    fn fragmented_loop_full_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.fragmented_loop_cheap_fingerprint().hash(&mut hasher);
        for &id in self.battlefield.iter() {
            if let Some(chars) = self.current_characteristics(id) {
                chars.name.hash(&mut hasher);
                (chars.power, chars.toughness).hash(&mut hasher);
                format!("{:?}{:?}", chars.card_types, chars.subtypes).hash(&mut hasher);
                for ability in chars.static_abilities.iter() {
                    format!("{:?}", ability.id()).hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }

    /// Record a non-mana activation for CR 732.3 fragmented-loop detection.
    pub(crate) fn record_activation_for_fragmented_loop(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: usize,
    ) {
        let Some(stable) = self.object(source).map(|object| object.stable_id) else {
            return;
        };
        let turn = self.turn.turn_number;
        self.fragmented_loop_history
            .retain(|(entry_turn, ..)| *entry_turn == turn);
        if self.fragmented_loop_history.len() >= 64 {
            self.fragmented_loop_history.remove(0);
        }
        let fingerprint = (
            self.fragmented_loop_cheap_fingerprint(),
            self.fragmented_loop_full_fingerprint(),
        );
        self.fragmented_loop_history
            .push((turn, fingerprint, player, stable, ability_index));
    }

    /// CR 732.3: once each player involved in a fragmented loop has acted and
    /// the game has returned to an earlier state, the active player (or the
    /// first involved player in turn order) must make a different choice.
    pub(crate) fn fragmented_loop_forbidden_activations(
        &self,
        player: PlayerId,
    ) -> Vec<(ObjectId, usize)> {
        let turn = self.turn.turn_number;
        let entries: Vec<_> = self
            .fragmented_loop_history
            .iter()
            .filter(|(entry_turn, ..)| *entry_turn == turn)
            .collect();
        if entries.len() < 2 || !entries.iter().any(|(_, _, who, ..)| *who == player) {
            return Vec::new();
        }
        let involved: Vec<PlayerId> = entries.iter().map(|(_, _, who, ..)| *who).collect();
        if !involved.iter().any(|who| *who != player) {
            return Vec::new();
        }
        let active = self.turn.active_player;
        let chooser = if involved.contains(&active) {
            active
        } else {
            match self
                .turn_store
                .turn_order
                .iter()
                .copied()
                .find(|candidate| involved.contains(candidate))
            {
                Some(first) => first,
                None => return Vec::new(),
            }
        };
        if chooser != player {
            return Vec::new();
        }
        let cheap = self.fragmented_loop_cheap_fingerprint();
        let mut full = None;
        let mut forbidden = Vec::new();
        for (index, (_, (entry_cheap, entry_full), who, stable, ability_index)) in
            entries.iter().enumerate()
        {
            if *who != player || *entry_cheap != cheap {
                continue;
            }
            if *full.get_or_insert_with(|| self.fragmented_loop_full_fingerprint()) != *entry_full {
                continue;
            }
            if !entries[index + 1..].iter().any(|(_, _, other, ..)| *other != player) {
                continue;
            }
            if let Some(source) = self.find_object_by_stable_id(*stable) {
                forbidden.push((source, *ability_index));
            }
        }
        forbidden
    }
}
