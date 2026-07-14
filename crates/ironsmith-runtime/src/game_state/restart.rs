use super::*;
use crate::object::ObjectKind;

#[derive(Debug, Clone)]
struct RestartCard {
    definition: crate::cards::CardDefinition,
    stable_id: StableId,
    owner: PlayerId,
    previous_zone: Zone,
    is_commander: bool,
}

impl GameState {
    /// Rebuild this state as a new game involving the players still in the old
    /// game. `starting_player` starts that game, while `exempt_objects` remain
    /// in exile and are returned as their new object IDs.
    ///
    /// This is the rules primitive used by restart effects. It deliberately
    /// rebuilds the state rather than moving cards one by one: continuous and
    /// replacement effects, emblems, tokens, counters, designations, combat,
    /// turn history, the stack, and other old-game state all cease to exist.
    pub fn restart_game(
        &mut self,
        starting_player: PlayerId,
        exempt_objects: &[ObjectId],
    ) -> Vec<ObjectId> {
        let players_in_new_game = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        let starting_player = if players_in_new_game.contains(&starting_player) {
            starting_player
        } else {
            self.turn_store
                .turn_order
                .iter()
                .copied()
                .find(|player| players_in_new_game.contains(player))
                .or_else(|| players_in_new_game.iter().copied().min())
                .unwrap_or(starting_player)
        };

        let exempt_stable_ids = exempt_objects
            .iter()
            .filter_map(|id| self.object(*id).map(|object| object.stable_id))
            .collect::<HashSet<_>>();
        let cards = self.restart_card_records(&players_in_new_game);

        let mut turn_order = self
            .turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|player| players_in_new_game.contains(player))
            .collect::<Vec<_>>();
        for player in self
            .players
            .iter()
            .map(|player| player.id)
            .filter(|player| players_in_new_game.contains(player))
        {
            if !turn_order.contains(&player) {
                turn_order.push(player);
            }
        }
        if let Some(start_idx) = turn_order
            .iter()
            .position(|player| *player == starting_player)
        {
            turn_order.rotate_left(start_idx);
        }

        let player_names = self
            .players
            .iter()
            .map(|player| player.name.clone())
            .collect::<Vec<_>>();
        let common_starting_life = self
            .players
            .first()
            .map_or(20, |player| player.starting_life);
        let mut restarted = GameState::new(player_names, common_starting_life);
        restarted.players = self
            .players
            .iter()
            .map(|old| {
                let mut player = Player::new(old.id, old.name.clone(), old.starting_life);
                player.has_left_game = !players_in_new_game.contains(&old.id);
                player
            })
            .collect();
        restarted.turn = TurnState::new(starting_player);
        restarted.turn_store.turn_order = turn_order.clone();
        restarted.runtime_cache = RuntimeCacheState::new(starting_player);
        restarted.auto_choose_single_object_decisions = self.auto_choose_single_object_decisions;
        restarted.next_object_id = self.next_object_id;

        // A restart begins a new game, not a new deterministic match. Preserve
        // the random stream and audit transcript while discarding gameplay
        // caches that belonged to the old game.
        restarted
            .runtime_cache
            .random_state
            .set(self.runtime_cache.random_state.get());
        restarted
            .runtime_cache
            .irreversible_random_count
            .set(self.runtime_cache.irreversible_random_count.get());
        *restarted.runtime_cache.forced_die_rolls.borrow_mut() =
            self.runtime_cache.forced_die_rolls.borrow().clone();
        *restarted.runtime_cache.transcript_random_seeds.borrow_mut() =
            self.runtime_cache.transcript_random_seeds.borrow().clone();
        *restarted
            .runtime_cache
            .transcript_library_shuffle_orders
            .borrow_mut() = self
            .runtime_cache
            .transcript_library_shuffle_orders
            .borrow()
            .clone();
        *restarted.runtime_cache.hidden_info_audit_log.borrow_mut() =
            self.runtime_cache.hidden_info_audit_log.borrow().clone();

        let mut exempt_new_ids = Vec::new();
        for card in cards {
            let exempt =
                card.previous_zone == Zone::Exile && exempt_stable_ids.contains(&card.stable_id);
            let destination = if card.previous_zone == Zone::OutsideGame {
                Zone::OutsideGame
            } else if exempt {
                Zone::Exile
            } else if card.is_commander {
                Zone::Command
            } else {
                Zone::Library
            };

            restarted.prime_linked_face_definitions(&card.definition);
            let new_id = restarted.new_object_id();
            let mut object =
                Object::from_card_definition(new_id, &card.definition, card.owner, destination);
            object.stable_id = card.stable_id;
            restarted.add_object(object);

            if card.is_commander {
                let commander_identity = card.stable_id.object_id();
                restarted
                    .commander_tracking_mut()
                    .commanders
                    .insert(commander_identity);
                if let Some(owner) = restarted.player_mut(card.owner) {
                    owner.add_commander(commander_identity);
                }
            }
            if exempt {
                exempt_new_ids.push(new_id);
            }
        }

        for player in turn_order {
            restarted.shuffle_player_library(player);
        }
        for player in restarted.turn_store.turn_order.clone() {
            restarted.draw_cards(player, 7);
        }

        // Opening-hand moves occur before the old effect finishes resolving.
        // No old-game source can trigger from them, and any new-game triggers
        // wait until after the restart procedure; the fresh game currently has
        // no active battlefield sources, so leave no synthetic queue entries.
        restarted.effect_store.pending_trigger_events.clear();
        *self = restarted;
        exempt_new_ids
    }

    fn restart_card_records(&self, players_in_new_game: &HashSet<PlayerId>) -> Vec<RestartCard> {
        let mut cards = Vec::new();
        for object in self.objects_in_deterministic_order() {
            if object.kind != ObjectKind::Card || !players_in_new_game.contains(&object.owner) {
                continue;
            }

            if let Some(melded) = self
                .commander_tracking
                .melded_permanents
                .get(&object.stable_id)
            {
                let mut recovered_all_components = true;
                let mut component_cards = Vec::new();
                for component in &melded.components {
                    let Some(definition) =
                        self.linked_face_definition_by_name_or_id(Some(&component.name), None)
                    else {
                        recovered_all_components = false;
                        break;
                    };
                    component_cards.push(RestartCard {
                        definition,
                        stable_id: component.stable_id,
                        owner: component.owner,
                        previous_zone: object.zone,
                        is_commander: self.players.iter().any(|player| {
                            player.commanders.contains(&component.stable_id.object_id())
                        }),
                    });
                }
                if recovered_all_components {
                    cards.extend(component_cards);
                    continue;
                }
            }

            cards.push(RestartCard {
                definition: object.to_card_definition(),
                stable_id: object.stable_id,
                owner: object.owner,
                previous_zone: object.zone,
                is_commander: self.is_commander(object.id),
            });
        }
        cards
    }
}
