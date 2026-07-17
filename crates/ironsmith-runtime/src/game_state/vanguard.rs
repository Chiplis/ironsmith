use super::*;

use std::collections::HashSet;

impl GameState {
    /// Enable Vanguard and create one face-up command-zone card for every player.
    pub fn enable_vanguard(
        &mut self,
        cards: Vec<(PlayerId, crate::cards::CardDefinition)>,
    ) -> Result<(), String> {
        if self.vanguard.is_some() {
            return Err("Vanguard is already enabled".to_string());
        }
        let player_count = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .count();
        if cards.len() != player_count {
            return Err("each player must provide exactly one vanguard card".to_string());
        }

        let mut seen_players = HashSet::new();
        for (player, definition) in &cards {
            if !seen_players.insert(*player)
                || self
                    .player(*player)
                    .is_none_or(|candidate| !candidate.is_in_game())
            {
                return Err(
                    "vanguard cards must belong to distinct players in the game".to_string()
                );
            }
            if !definition.card.card_types.contains(&CardType::Vanguard) {
                return Err(format!("{} is not a Vanguard card", definition.name()));
            }
            if !definition.card.subtypes.is_empty() {
                return Err(format!(
                    "Vanguard card {} may not have subtypes",
                    definition.name()
                ));
            }
            7_i32
                .checked_add(definition.card.hand_modifier)
                .ok_or_else(|| format!("{} has an invalid hand modifier", definition.name()))?;
            20_i32
                .checked_add(definition.card.life_modifier)
                .ok_or_else(|| format!("{} has an invalid life modifier", definition.name()))?;
        }

        let mut state = VanguardState {
            cards: HashMap::new(),
            hand_modifiers: HashMap::new(),
            life_modifiers: HashMap::new(),
        };
        for (owner, mut definition) in cards {
            let hand_modifier = definition.card.hand_modifier;
            let life_modifier = definition.card.life_modifier;
            for ability in &mut definition.abilities {
                ability.functional_zones = vec![Zone::Command];
            }
            let object = self.create_object_from_definition(&definition, owner, Zone::Command);
            state.cards.insert(owner, object);
            state.hand_modifiers.insert(owner, hand_modifier);
            state.life_modifiers.insert(owner, life_modifier);

            let player = self
                .player_mut(owner)
                .expect("validated Vanguard owner must remain in game");
            player.starting_life = 20_i32.saturating_add(life_modifier);
            player.life = player.starting_life;
            player.max_hand_size = 7_i32.saturating_add(hand_modifier);
        }

        self.vanguard = Some(state);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn is_vanguard_card(&self, object: ObjectId) -> bool {
        self.vanguard
            .as_ref()
            .is_some_and(|state| state.cards.values().any(|candidate| *candidate == object))
    }

    pub fn vanguard_card(&self, player: PlayerId) -> Option<ObjectId> {
        self.vanguard.as_ref()?.cards.get(&player).copied()
    }

    pub fn vanguard_cards(&self) -> Vec<ObjectId> {
        let mut cards = self
            .vanguard
            .as_ref()
            .map(|state| state.cards.values().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        cards.sort_by_key(|object| object.0);
        cards
    }

    pub fn vanguard_hand_modifier(&self, player: PlayerId) -> i32 {
        self.vanguard
            .as_ref()
            .and_then(|state| state.hand_modifiers.get(&player))
            .copied()
            .unwrap_or(0)
    }

    pub fn vanguard_life_modifier(&self, player: PlayerId) -> i32 {
        self.vanguard
            .as_ref()
            .and_then(|state| state.life_modifiers.get(&player))
            .copied()
            .unwrap_or(0)
    }

    pub fn vanguard_starting_hand_size(&self, player: PlayerId) -> usize {
        7_i32
            .saturating_add(self.vanguard_hand_modifier(player))
            .max(0) as usize
    }

    pub fn vanguard_maximum_hand_size(&self, player: PlayerId) -> i32 {
        7_i32.saturating_add(self.vanguard_hand_modifier(player))
    }

    pub fn synchronize_vanguard_ability_zones(&mut self) {
        for object in self.vanguard_cards() {
            if let Some(card) = self.object_mut(object) {
                for ability in card.abilities_mut() {
                    ability.functional_zones = vec![Zone::Command];
                }
            }
        }
        self.mark_continuous_state_dirty();
    }

    pub(crate) fn handle_vanguard_player_departure(&mut self, player: PlayerId) {
        let Some(state) = self.vanguard.as_mut() else {
            return;
        };
        state.cards.remove(&player);
        state.hand_modifiers.remove(&player);
        state.life_modifiers.remove(&player);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }
}
