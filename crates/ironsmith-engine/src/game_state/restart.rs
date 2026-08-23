use super::*;
use crate::object::ObjectKind;

#[derive(Debug, Clone)]
struct RestartCard {
    definition: crate::cards::CardDefinition,
    stable_id: StableId,
    owner: PlayerId,
    previous_zone: Zone,
    is_commander: bool,
    is_vanguard: bool,
    is_scheme: bool,
    is_conspiracy: bool,
    agenda_names: Vec<String>,
    vanguard_hand_modifier: i32,
    vanguard_life_modifier: i32,
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
        let grand_melee_seats = self.grand_melee().map(|state| state.seats().to_vec());
        let shared_team_setup = (self.two_headed_giant().is_none())
            .then(|| {
                self.shared_team_turns()
                    .map(|state| (state.seats().to_vec(), state.member_orders().to_vec()))
            })
            .flatten();
        let team_vs_team_setup = self
            .team_vs_team()
            .map(|state| (state.teams().to_vec(), state.seats().to_vec()));
        let emperor_setup = self.emperor().map(|state| {
            (
                state.teams().to_vec(),
                state.seats().to_vec(),
                state.starting_team(),
                state.starting_emperor(),
                state.ranges().to_vec(),
            )
        });
        let two_headed_giant_setup = self.two_headed_giant().map(|state| state.teams().to_vec());
        let alternating_teams_setup = self.alternating_teams().map(|state| {
            (
                state.teams().to_vec(),
                state.seats().to_vec(),
                state.attack_option(),
                state.range_of_influence(),
                state.deploy_creatures(),
            )
        });
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
        // CR 801.17 exempts the restart instruction from range, but the new
        // game still uses the match's limited-range option and physical seats.
        restarted.range_of_influence = self.range_of_influence.clone();
        restarted.refresh_range_of_influence_snapshot();
        restarted.free_for_all = self.free_for_all.clone();
        restarted.attack_direction = self.attack_direction;
        restarted.teams = self.teams.clone();
        restarted.deploy_creatures = self.deploy_creatures;
        if let Some(teams) = two_headed_giant_setup {
            let starting_team = teams
                .iter()
                .position(|team| team.contains(&starting_player))
                .expect("restart starting player belongs to a Two-Headed Giant team");
            let starting_primary = *teams[starting_team]
                .last()
                .expect("Two-Headed Giant teams are nonempty");
            restarted
                .restore_two_headed_giant_new_game(teams, starting_team, starting_primary)
                .expect("a restart preserves the Two-Headed Giant profile");
        }
        if let Some((teams, seats, starting_team, starting_emperor, ranges)) = emperor_setup {
            let live_turn_order = restarted.turn_store.turn_order.clone();
            restarted
                .restore_emperor(teams, seats, starting_team, starting_emperor, ranges)
                .expect("a restart preserves the Emperor profile");
            restarted.turn_store.turn_order = live_turn_order;
            restarted.turn.active_player = starting_player;
            restarted.turn.priority_player = Some(starting_player);
        }
        if let Some((teams, seats)) = team_vs_team_setup {
            let starting_team = teams
                .iter()
                .position(|team| team.contains(&starting_player))
                .expect("restart starting player belongs to a Team vs. Team team");
            let live_turn_order = restarted.turn_store.turn_order.clone();
            restarted
                .restore_team_vs_team(teams, seats, starting_team, starting_player)
                .expect("a restart preserves the Team vs. Team profile");
            restarted.turn_store.turn_order = live_turn_order;
        }
        if let Some((teams, seats, attack_option, range, deploy_creatures)) =
            alternating_teams_setup
        {
            let live_turn_order = restarted.turn_store.turn_order.clone();
            restarted
                .restore_alternating_teams(
                    teams,
                    seats,
                    starting_player,
                    attack_option,
                    range,
                    deploy_creatures,
                )
                .expect("a restart preserves the Alternating Teams profile");
            restarted.turn_store.turn_order = live_turn_order;
            restarted.turn.active_player = starting_player;
            restarted.turn.priority_player = Some(starting_player);
        }
        if let Some(seats) = grand_melee_seats {
            restarted
                .restore_grand_melee_with_starting_player(seats, starting_player)
                .expect("a restart preserves the Grand Melee profile");
        }
        if let Some((seats, member_orders)) = shared_team_setup {
            // Departed players remain part of the match's physical seating.
            // Re-derive the printed primary players from that full seat map,
            // then restore the live-player turn order for the restarted game.
            let live_turn_order = restarted.turn_store.turn_order.clone();
            restarted.turn_store.turn_order = seats;
            restarted
                .enable_shared_team_turns()
                .expect("a restart preserves valid shared-team seating");
            restarted.turn_store.turn_order = live_turn_order;
            for (team, order) in member_orders.into_iter().enumerate() {
                restarted
                    .set_shared_team_member_order(team, order)
                    .expect("a restart preserves each team's selected member order");
            }
        }

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
        let preserve_vanguard = self.vanguard.is_some();
        let preserved_archenemy = self
            .archenemy
            .as_ref()
            .map(|state| (state.variant, state.archenemies.clone()));
        let mut restarted_vanguard = VanguardState {
            cards: HashMap::new(),
            hand_modifiers: HashMap::new(),
            life_modifiers: HashMap::new(),
        };
        let mut restarted_scheme_decks = HashMap::<PlayerId, Vec<ObjectId>>::new();
        let mut restarted_conspiracy = ConspiracyState::default();
        for card in cards {
            let exempt =
                card.previous_zone == Zone::Exile && exempt_stable_ids.contains(&card.stable_id);
            let destination = if card.previous_zone == Zone::OutsideGame {
                Zone::OutsideGame
            } else if exempt {
                Zone::Exile
            } else if card.is_commander || card.is_vanguard || card.is_scheme || card.is_conspiracy
            {
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

            if card.is_scheme {
                if let Some(scheme) = restarted.object_mut(new_id) {
                    for ability in scheme.abilities_mut() {
                        ability.functional_zones.clear();
                    }
                }
                restarted_scheme_decks
                    .entry(card.owner)
                    .or_default()
                    .push(new_id);
            }

            if card.is_conspiracy {
                restarted_conspiracy
                    .cards
                    .entry(card.owner)
                    .or_default()
                    .push(new_id);
                if !card.agenda_names.is_empty() {
                    restarted.set_chosen_named_option(new_id, card.agenda_names.join("\n"));
                    restarted_conspiracy.face_down.insert(new_id);
                    restarted_conspiracy
                        .agenda_names
                        .insert(new_id, card.agenda_names.clone());
                    if let Some(conspiracy) = restarted.object_mut(new_id) {
                        for ability in conspiracy.abilities_mut() {
                            ability.functional_zones.clear();
                        }
                    }
                }
            }

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
            if card.is_vanguard {
                restarted_vanguard.cards.insert(card.owner, new_id);
                restarted_vanguard
                    .hand_modifiers
                    .insert(card.owner, card.vanguard_hand_modifier);
                restarted_vanguard
                    .life_modifiers
                    .insert(card.owner, card.vanguard_life_modifier);
                if let Some(owner) = restarted.player_mut(card.owner) {
                    owner.starting_life = 20_i32.saturating_add(card.vanguard_life_modifier);
                    owner.life = owner.starting_life;
                    owner.max_hand_size = 7_i32.saturating_add(card.vanguard_hand_modifier);
                }
            }
            if exempt {
                exempt_new_ids.push(new_id);
            }
        }

        if preserve_vanguard {
            restarted.vanguard = Some(restarted_vanguard);
            restarted.synchronize_vanguard_ability_zones();
        }

        if let Some((variant, archenemies)) = preserved_archenemy {
            for deck in restarted_scheme_decks.values_mut() {
                restarted.shuffle_slice(deck);
            }
            restarted.archenemy = Some(ArchenemyState {
                variant,
                archenemies: archenemies
                    .into_iter()
                    .filter(|player| players_in_new_game.contains(player))
                    .collect(),
                scheme_decks: restarted_scheme_decks,
                face_up: Vec::new(),
            });
            let archenemies = restarted
                .archenemy
                .as_ref()
                .expect("restarted Archenemy state")
                .archenemies
                .clone();
            for player in &mut restarted.players {
                let starting_life = if archenemies.contains(&player.id) {
                    if variant == ArchenemyVariant::Commander {
                        60
                    } else {
                        40
                    }
                } else if variant == ArchenemyVariant::Default {
                    20
                } else {
                    player.starting_life
                };
                player.starting_life = starting_life;
                player.life = starting_life;
            }
            restarted.synchronize_scheme_ability_zones();
        }

        if self.conspiracy.is_some() {
            restarted.conspiracy = Some(restarted_conspiracy);
            restarted.synchronize_conspiracy_ability_zones();
            for player in &mut restarted.players {
                if player.is_in_game() {
                    player.starting_life = 20;
                    player.life = 20;
                }
            }
        }

        for player in turn_order {
            restarted.shuffle_player_library(player);
        }
        for player in restarted.turn_store.turn_order.clone() {
            let opening_hand_size = if preserve_vanguard {
                restarted.vanguard_starting_hand_size(player)
            } else {
                7
            };
            restarted.draw_cards(player, opening_hand_size);
        }

        // Opening-hand moves occur before the old effect finishes resolving.
        // No old-game source can trigger from them, and any new-game triggers
        // wait until after the restart procedure; the fresh game currently has
        // no active battlefield sources, so leave no synthetic queue entries.
        restarted.effect_store.pending_trigger_events.clear();
        // CR 727.6: restarting a subgame restarts only that game. Keep the
        // suspended parent frame attached to the rebuilt child so the parent
        // remains wholly unaffected and can still resume normally.
        restarted.subgame_parent = self.subgame_parent.take();
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
                        is_vanguard: false,
                        is_scheme: false,
                        is_conspiracy: false,
                        agenda_names: Vec::new(),
                        vanguard_hand_modifier: 0,
                        vanguard_life_modifier: 0,
                    });
                }
                if recovered_all_components {
                    cards.extend(component_cards);
                    continue;
                }
            }

            let is_vanguard = self.is_vanguard_card(object.id);
            let is_scheme = self.is_scheme_card(object.id);
            let is_conspiracy = self.is_conspiracy_card(object.id);
            cards.push(RestartCard {
                definition: object.to_card_definition(),
                stable_id: object.stable_id,
                owner: object.owner,
                previous_zone: object.zone,
                is_commander: self.is_commander(object.id),
                is_vanguard,
                is_scheme,
                is_conspiracy,
                agenda_names: self
                    .conspiracy
                    .as_ref()
                    .and_then(|state| state.agenda_names.get(&object.id))
                    .cloned()
                    .unwrap_or_default(),
                vanguard_hand_modifier: if is_vanguard {
                    self.vanguard_hand_modifier(object.owner)
                } else {
                    0
                },
                vanguard_life_modifier: if is_vanguard {
                    self.vanguard_life_modifier(object.owner)
                } else {
                    0
                },
            });
        }
        cards
    }
}
