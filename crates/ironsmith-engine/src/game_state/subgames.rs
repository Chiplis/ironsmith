use super::*;

use crate::decision::{DecisionMaker, GameResult};
use crate::effect::Effect;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::object::ObjectKind;
use crate::tag::TagKey;
use crate::types::CardType;

const SUBGAME_NONWINNERS_TAG: &str = "__subgame_nonwinners__";

/// How a physical card entered the child game under CR 729.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubgameTransferKind {
    Library,
    Commander,
    Vanguard,
    PlanarDeck {
        owner: PlayerId,
        kind: PlanarCardKind,
        communal: bool,
    },
    SchemeDeck {
        owner: PlayerId,
    },
    Explicit,
}

#[derive(Debug, Clone)]
pub(super) struct SubgameFrame {
    parent: Box<GameState>,
    participants: Vec<PlayerId>,
    resolving_object: Option<ObjectId>,
    controller: PlayerId,
    nonwinner_effects: Vec<Effect>,
    original_definitions: HashMap<StableId, crate::cards::CardDefinition>,
    transfer_kinds: HashMap<StableId, SubgameTransferKind>,
    vanguard_modifiers: HashMap<PlayerId, (i32, i32)>,
    archenemy_variant: Option<ArchenemyVariant>,
    archenemies: HashSet<PlayerId>,
}

/// Observable result of restoring one suspended parent game.
#[derive(Debug, Clone)]
pub struct SubgameCompletion {
    pub result: GameResult,
    pub nonwinners: Vec<PlayerId>,
    pub returned_cards: Vec<ObjectId>,
    pub resumed_depth: usize,
}

#[derive(Debug, Clone)]
struct ReturnedCard {
    stable_id: StableId,
    owner: PlayerId,
    definition: crate::cards::CardDefinition,
    destination: ReturnedDestination,
    commander: bool,
}

#[derive(Debug, Clone, Copy)]
enum ReturnedDestination {
    Library,
    Commander,
    Vanguard,
    PlanarDeck {
        owner: PlayerId,
        kind: PlanarCardKind,
        communal: bool,
    },
    SchemeDeck {
        owner: PlayerId,
    },
}

fn is_nontraditional(definition: &crate::cards::CardDefinition) -> bool {
    definition.card.card_types.iter().any(|card_type| {
        matches!(
            card_type,
            CardType::Plane
                | CardType::Phenomenon
                | CardType::Vanguard
                | CardType::Scheme
                | CardType::Conspiracy
        )
    })
}

impl GameState {
    fn create_transferred_object(
        &mut self,
        definition: &crate::cards::CardDefinition,
        owner: PlayerId,
        zone: Zone,
        stable_id: StableId,
    ) -> ObjectId {
        self.prime_linked_face_definitions(definition);
        let id = self.new_object_id();
        let mut object = crate::object::Object::from_card_definition(id, definition, owner, zone);
        object.stable_id = stable_id;
        self.add_object(object);
        id
    }

    /// Number of suspended parent games beneath the active game.
    pub fn subgame_depth(&self) -> usize {
        self.subgame_parent
            .as_ref()
            .map_or(0, |frame| 1 + frame.parent.subgame_depth())
    }

    pub fn is_subgame(&self) -> bool {
        self.subgame_parent.is_some()
    }

    pub fn subgame_starting_procedure_pending(&self) -> bool {
        self.subgame_starting_procedure_pending
    }

    pub fn complete_subgame_starting_procedure(&mut self) {
        self.subgame_starting_procedure_pending = false;
    }

    pub fn take_subgame_just_resumed(&mut self) -> bool {
        std::mem::take(&mut self.subgame_just_resumed)
    }

    /// Create a fully isolated child game from the current libraries and
    /// suspend this game. The deterministic parent RNG supplies both the child
    /// seed and its random starting player.
    pub fn begin_subgame(
        &mut self,
        resolving_object: Option<ObjectId>,
        controller: PlayerId,
        nonwinner_effects: Vec<Effect>,
    ) -> Result<(), String> {
        let grand_melee_seats = self.grand_melee().map(|state| state.seats().to_vec());
        let participants = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect::<Vec<_>>();
        if participants.is_empty() {
            return Err("a subgame requires at least one participating player".to_string());
        }

        // Preserve the match-wide player IDs. Constructing the child from only
        // live participants would renumber a sparse participant set after a
        // player had left the parent game.
        let names = self
            .players
            .iter()
            .map(|player| player.name.clone())
            .collect::<Vec<_>>();
        let starting_lives = participants
            .iter()
            .map(|player| {
                (
                    *player,
                    self.player(*player)
                        .expect("live participant")
                        .starting_life,
                )
            })
            .collect::<HashMap<_, _>>();
        let child_seed = self.next_random_u64();

        let mut transfer_ids = Vec::<(ObjectId, SubgameTransferKind)>::new();
        for player in &participants {
            transfer_ids.extend(
                self.player(*player)
                    .expect("live participant")
                    .library
                    .iter()
                    .copied()
                    .map(|object| (object, SubgameTransferKind::Library)),
            );
        }

        let vanguard_modifiers = self
            .vanguard
            .as_ref()
            .map(|state| {
                state
                    .cards
                    .iter()
                    .filter_map(|(owner, object)| {
                        participants.contains(owner).then_some((
                            *owner,
                            (
                                state.hand_modifiers.get(owner).copied().unwrap_or(0),
                                state.life_modifiers.get(owner).copied().unwrap_or(0),
                            ),
                            *object,
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        transfer_ids.extend(
            vanguard_modifiers
                .iter()
                .map(|(_, _, object)| (*object, SubgameTransferKind::Vanguard)),
        );

        for player in &participants {
            let commander_ids = self
                .player(*player)
                .expect("live participant")
                .commanders
                .clone();
            for commander in commander_ids {
                let Some(current) = self.current_commander_object(commander) else {
                    continue;
                };
                if self
                    .object(current)
                    .is_some_and(|object| object.zone == Zone::Command)
                {
                    transfer_ids.push((current, SubgameTransferKind::Commander));
                }
            }
        }

        if let Some(state) = self.planechase.as_ref() {
            for (owner, deck) in &state.decks {
                for object in deck {
                    if let Some(kind) = state.card_kinds.get(object).copied() {
                        transfer_ids.push((
                            *object,
                            SubgameTransferKind::PlanarDeck {
                                owner: *owner,
                                kind,
                                communal: false,
                            },
                        ));
                    }
                }
            }
            if let Some(deck) = &state.communal_deck {
                for object in deck {
                    if let Some(kind) = state.card_kinds.get(object).copied() {
                        let owner = state
                            .deck_owners
                            .get(object)
                            .copied()
                            .or_else(|| self.object(*object).map(|card| card.owner))
                            .unwrap_or(self.turn.active_player);
                        transfer_ids.push((
                            *object,
                            SubgameTransferKind::PlanarDeck {
                                owner,
                                kind,
                                communal: true,
                            },
                        ));
                    }
                }
            }
        }

        let archenemy_variant = self.archenemy.as_ref().map(|state| state.variant);
        let archenemies = self
            .archenemy
            .as_ref()
            .map(|state| state.archenemies.clone())
            .unwrap_or_default();
        if let Some(state) = self.archenemy.as_ref() {
            for (owner, deck) in &state.scheme_decks {
                transfer_ids.extend(
                    deck.iter()
                        .copied()
                        .map(|object| (object, SubgameTransferKind::SchemeDeck { owner: *owner })),
                );
            }
        }

        let mut seen = HashSet::new();
        transfer_ids.retain(|(object, _)| seen.insert(*object));
        let mut transferred = Vec::with_capacity(transfer_ids.len());
        let mut original_definitions = HashMap::new();
        let mut transfer_kinds = HashMap::new();
        for (object_id, kind) in transfer_ids {
            let object = self
                .object(object_id)
                .cloned()
                .ok_or_else(|| format!("subgame transfer object {} is missing", object_id.0))?;
            let definition = object.to_card_definition();
            original_definitions.insert(object.stable_id, definition.clone());
            transfer_kinds.insert(object.stable_id, kind);
            transferred.push((object.stable_id, object.owner, definition, kind));
            self.remove_object(object_id);
            for player in &mut self.players {
                player
                    .commanders
                    .retain(|commander| *commander != object_id);
            }
        }

        if let Some(state) = self.vanguard.as_mut() {
            state.cards.retain(|_, object| !seen.contains(object));
        }
        if let Some(state) = self.planechase.as_mut() {
            state.decks.clear();
            state.communal_deck = state.communal_deck.as_ref().map(|_| Vec::new());
            state.deck_owners.clear();
            state
                .card_kinds
                .retain(|object, _| state.face_up.contains(object));
        }
        if let Some(state) = self.archenemy.as_mut() {
            state.scheme_decks.clear();
        }

        let default_life = starting_lives.get(&participants[0]).copied().unwrap_or(20);
        let mut child = GameState::new(names, default_life);
        child.set_random_seed(child_seed);
        child.set_commander_damage_loss_enabled(self.commander_damage_loss_enabled());
        for child_player in &mut child.players {
            child_player.has_left_game = !participants.contains(&child_player.id);
            if participants.contains(&child_player.id) {
                let player = child_player.id;
                let starting_life = starting_lives.get(&player).copied().unwrap_or(default_life);
                child_player.starting_life = starting_life;
                child_player.life = starting_life;
            }
        }
        child.turn_store.turn_order = participants.clone();
        child.range_of_influence = self.range_of_influence.clone();
        child.refresh_range_of_influence_snapshot();
        child.free_for_all = self.free_for_all.clone();
        child.team_vs_team = self.team_vs_team.clone();
        child.attack_direction = self.attack_direction;
        child.teams = self.teams.clone();
        child.deploy_creatures = self.deploy_creatures;

        let mut child_vanguards = VanguardState {
            cards: HashMap::new(),
            hand_modifiers: HashMap::new(),
            life_modifiers: HashMap::new(),
        };
        let mut child_planar = PlanechaseState {
            decks: HashMap::new(),
            communal_deck: None,
            deck_owners: HashMap::new(),
            card_kinds: HashMap::new(),
            face_up: Vec::new(),
            planar_controller: child.turn.active_player,
            planar_controllers: HashSet::from([child.turn.active_player]),
            face_up_controllers: HashMap::new(),
            voluntary_rolls_this_turn: HashMap::new(),
            planeswalk_count: 0,
        };
        let mut child_schemes = archenemy_variant.map(|variant| ArchenemyState {
            variant,
            archenemies: archenemies.clone(),
            scheme_decks: HashMap::new(),
            face_up: Vec::new(),
        });
        for (stable_id, owner, mut definition, kind) in transferred {
            let zone = match kind {
                SubgameTransferKind::Library | SubgameTransferKind::Explicit => Zone::Library,
                _ => Zone::Command,
            };
            if !matches!(
                kind,
                SubgameTransferKind::Vanguard | SubgameTransferKind::Commander
            ) {
                for ability in &mut definition.abilities {
                    if zone == Zone::Command {
                        ability.functional_zones.clear();
                    }
                }
            }
            let child_id = child.create_transferred_object(&definition, owner, zone, stable_id);
            match kind {
                SubgameTransferKind::Commander => child.set_as_commander(child_id, owner),
                SubgameTransferKind::Vanguard => {
                    let (hand, life) = vanguard_modifiers
                        .iter()
                        .find_map(|(candidate, modifiers, _)| {
                            (*candidate == owner).then_some(*modifiers)
                        })
                        .unwrap_or_default();
                    child_vanguards.cards.insert(owner, child_id);
                    child_vanguards.hand_modifiers.insert(owner, hand);
                    child_vanguards.life_modifiers.insert(owner, life);
                }
                SubgameTransferKind::PlanarDeck {
                    owner,
                    kind,
                    communal,
                } => {
                    child_planar.card_kinds.insert(child_id, kind);
                    child_planar.deck_owners.insert(child_id, owner);
                    if communal {
                        child_planar
                            .communal_deck
                            .get_or_insert_with(Vec::new)
                            .push(child_id);
                    } else {
                        child_planar.decks.entry(owner).or_default().push(child_id);
                    }
                }
                SubgameTransferKind::SchemeDeck { owner } => {
                    if let Some(state) = child_schemes.as_mut() {
                        state.scheme_decks.entry(owner).or_default().push(child_id);
                    }
                }
                SubgameTransferKind::Library | SubgameTransferKind::Explicit => {}
            }
        }

        for player in &participants {
            child.shuffle_player_library(*player);
        }
        for deck in child_planar.decks.values_mut() {
            child.shuffle_slice(deck);
        }
        if let Some(deck) = child_planar.communal_deck.as_mut() {
            child.shuffle_slice(deck);
        }
        if !child_planar.decks.is_empty() || child_planar.communal_deck.is_some() {
            child.planechase = Some(child_planar);
        }
        if !child_vanguards.cards.is_empty() {
            child.vanguard = Some(child_vanguards);
            child.synchronize_vanguard_ability_zones();
            for player in &participants {
                if let Some(child_player) = child.player_mut(*player) {
                    let hand_modifier = vanguard_modifiers
                        .iter()
                        .find_map(|(owner, (hand, _), _)| (*owner == *player).then_some(*hand))
                        .unwrap_or(0);
                    child_player.max_hand_size = 7_i32.saturating_add(hand_modifier);
                }
            }
        }
        if let Some(mut state) = child_schemes {
            for deck in state.scheme_decks.values_mut() {
                child.shuffle_slice(deck);
            }
            child.archenemy = Some(state);
        }

        let start_index = (child_seed as usize) % participants.len();
        child.turn_store.turn_order.rotate_left(start_index);
        child.turn.active_player = child.turn_store.turn_order[0];
        child.turn.priority_player = None;
        if let Some(profile) = self.team_vs_team() {
            let teams = profile.teams().to_vec();
            let seats = profile.seats().to_vec();
            let mut live_team_indices = teams
                .iter()
                .enumerate()
                .filter(|(_, team)| team.iter().any(|player| participants.contains(player)))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            child.shuffle_slice(&mut live_team_indices);
            let starting_team = live_team_indices[0];
            let live_starting_team = teams[starting_team]
                .iter()
                .copied()
                .filter(|player| participants.contains(player))
                .collect::<Vec<_>>();
            let starting_player =
                live_starting_team[(live_starting_team.len().saturating_sub(1)) / 2];
            child
                .restore_team_vs_team(teams, seats, starting_team, starting_player)
                .map_err(|error| format!("failed to preserve Team vs. Team: {error}"))?;
            child.turn_store.turn_order = participants.clone();
            let start = child
                .turn_store
                .turn_order
                .iter()
                .position(|player| *player == starting_player)
                .expect("Team vs. Team starting player participates in the subgame");
            child.turn_store.turn_order.rotate_left(start);
            child.turn.active_player = starting_player;
            child.turn.priority_player = None;
        }
        if let Some(profile) = self.emperor() {
            let teams = profile.teams().to_vec();
            let seats = profile.seats().to_vec();
            let ranges = profile.ranges().to_vec();
            let mut live_team_indices = profile
                .emperors()
                .iter()
                .enumerate()
                .filter(|(_, emperor)| participants.contains(emperor))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            child.shuffle_slice(&mut live_team_indices);
            let starting_team = live_team_indices[0];
            let starting_emperor = profile.emperors()[starting_team];
            child
                .restore_emperor(teams, seats, starting_team, starting_emperor, ranges)
                .map_err(|error| format!("failed to preserve Emperor: {error}"))?;
            child.turn_store.turn_order = participants.clone();
            let start = child
                .turn_store
                .turn_order
                .iter()
                .position(|player| *player == starting_emperor)
                .expect("Emperor starting player participates in the subgame");
            child.turn_store.turn_order.rotate_left(start);
            child.turn.active_player = starting_emperor;
            child.turn.priority_player = None;
        }
        if let Some(profile) = self.two_headed_giant() {
            child
                .enable_two_headed_giant(profile.teams().to_vec())
                .map_err(|error| format!("failed to preserve Two-Headed Giant: {error}"))?;
            child.turn.priority_player = None;
        }
        if let Some(profile) = self.alternating_teams() {
            let teams = profile.teams().to_vec();
            let seats = profile.seats().to_vec();
            let attack_option = profile.attack_option();
            let range = profile.range_of_influence();
            let deploy_creatures = profile.deploy_creatures();
            let starting_player = child.turn.active_player;
            child
                .restore_alternating_teams(
                    teams,
                    seats,
                    starting_player,
                    attack_option,
                    range,
                    deploy_creatures,
                )
                .map_err(|error| format!("failed to preserve Alternating Teams: {error}"))?;
            child.turn_store.turn_order = participants.clone();
            let start = child
                .turn_store
                .turn_order
                .iter()
                .position(|player| *player == starting_player)
                .expect("Alternating Teams starting player participates in the subgame");
            child.turn_store.turn_order.rotate_left(start);
            child.turn.active_player = starting_player;
            child.turn.priority_player = None;
        }
        if let Some(seats) = grand_melee_seats {
            let starting_player = child.turn.active_player;
            child
                .restore_grand_melee_with_starting_player(seats, starting_player)
                .map_err(|error| format!("failed to preserve Grand Melee: {error}"))?;
            child.turn.priority_player = None;
        }
        if self.two_headed_giant().is_none()
            && let Some(shared) = self.shared_team_turns()
        {
            let seats = shared.seats().to_vec();
            let member_orders = shared.member_orders().to_vec();
            let participant_turn_order = child.turn_store.turn_order.clone();
            child.turn_store.turn_order = seats;
            child
                .enable_shared_team_turns()
                .map_err(|error| format!("failed to preserve shared team turns: {error}"))?;
            child.turn_store.turn_order = participant_turn_order;
            for (team, order) in member_orders.into_iter().enumerate() {
                child
                    .set_shared_team_member_order(team, order)
                    .map_err(|error| {
                        format!("failed to preserve shared team member order: {error}")
                    })?;
            }
            child.turn.priority_player = None;
        }
        if child.planechase.is_some() {
            child.reveal_starting_plane().map_err(|error| {
                format!("failed to reveal the subgame's starting plane: {error}")
            })?;
        }
        for player in &participants {
            let opening_size = (7_i32 + child.vanguard_hand_modifier(*player)).max(0) as usize;
            child.draw_cards(*player, opening_size);
        }
        child.subgame_starting_procedure_pending = true;

        let parent = std::mem::replace(self, child);
        self.subgame_parent = Some(Box::new(SubgameFrame {
            parent: Box::new(parent),
            participants,
            resolving_object,
            controller,
            nonwinner_effects,
            original_definitions,
            transfer_kinds,
            vanguard_modifiers: vanguard_modifiers
                .into_iter()
                .map(|(owner, modifiers, _)| (owner, modifiers))
                .collect(),
            archenemy_variant,
            archenemies,
        }));
        Ok(())
    }

    /// Explicitly bring a traditional card from the suspended parent into the
    /// active child. Parent zone-change triggers remain queued in the suspended
    /// state until it resumes.
    pub fn bring_parent_card_into_subgame(&mut self, object: ObjectId) -> Result<ObjectId, String> {
        let (stable_id, owner, definition) = {
            let frame = self
                .subgame_parent
                .as_mut()
                .ok_or_else(|| "there is no suspended parent game".to_string())?;
            let definition = frame
                .parent
                .object(object)
                .cloned()
                .ok_or_else(|| "the parent-game card is missing".to_string())?
                .to_card_definition();
            if is_nontraditional(&definition) {
                return Err(
                    "only a traditional card may be brought into a subgame this way".to_string(),
                );
            }
            let moved = frame
                .parent
                .move_object_by_effect(object, Zone::OutsideGame)
                .ok_or_else(|| "the parent-game card could not leave its zone".to_string())?;
            let object = frame
                .parent
                .object(moved)
                .cloned()
                .ok_or_else(|| "the moved parent-game card is missing".to_string())?;
            frame
                .original_definitions
                .insert(object.stable_id, definition.clone());
            frame
                .transfer_kinds
                .insert(object.stable_id, SubgameTransferKind::Explicit);
            frame.parent.remove_object(moved);
            (object.stable_id, object.owner, definition)
        };
        Ok(self.create_transferred_object(&definition, owner, Zone::OutsideGame, stable_id))
    }

    /// End the active child, restore exactly one parent frame, execute the
    /// creating instruction's nonwinner continuation, then finish resolving
    /// the creating stack object. Deferred parent triggers remain queued for
    /// the ordinary post-resolution trigger pass.
    pub fn finish_subgame_with(
        &mut self,
        result: GameResult,
        decision_maker: &mut dyn DecisionMaker,
    ) -> Result<SubgameCompletion, ExecutionError> {
        let frame = self
            .subgame_parent
            .take()
            .ok_or_else(|| ExecutionError::Impossible("there is no active subgame".to_string()))?;
        let winners = match &result {
            GameResult::Winner(winner) => vec![*winner],
            GameResult::Remaining(players) => players.clone(),
            GameResult::Draw => Vec::new(),
        };
        if winners
            .iter()
            .any(|winner| !frame.participants.contains(winner))
        {
            self.subgame_parent = Some(frame);
            return Err(ExecutionError::Impossible(
                "subgame result names a nonparticipant as a winner".to_string(),
            ));
        }
        let nonwinners = frame
            .participants
            .iter()
            .copied()
            .filter(|player| !winners.contains(player))
            .collect::<Vec<_>>();

        let mut returned = Vec::new();
        for object in self.objects_in_deterministic_order() {
            if object.kind != ObjectKind::Card {
                continue;
            }
            let current_definition = object.to_card_definition();
            let kind = frame.transfer_kinds.get(&object.stable_id).copied();
            let definition = frame
                .original_definitions
                .get(&object.stable_id)
                .cloned()
                .unwrap_or(current_definition);
            let destination = match kind {
                Some(SubgameTransferKind::Vanguard) => ReturnedDestination::Vanguard,
                Some(SubgameTransferKind::Commander) if object.zone == Zone::Command => {
                    ReturnedDestination::Commander
                }
                Some(SubgameTransferKind::PlanarDeck {
                    owner,
                    kind,
                    communal,
                }) => ReturnedDestination::PlanarDeck {
                    owner,
                    kind,
                    communal,
                },
                Some(SubgameTransferKind::SchemeDeck { owner }) => {
                    ReturnedDestination::SchemeDeck { owner }
                }
                _ if !is_nontraditional(&definition) => ReturnedDestination::Library,
                _ => continue,
            };
            returned.push(ReturnedCard {
                stable_id: object.stable_id,
                owner: object.owner,
                definition,
                destination,
                commander: matches!(kind, Some(SubgameTransferKind::Commander)),
            });
        }

        let SubgameFrame {
            parent,
            resolving_object,
            controller,
            nonwinner_effects,
            vanguard_modifiers,
            archenemy_variant,
            archenemies,
            ..
        } = *frame;
        *self = *parent;
        self.subgame_just_resumed = true;

        let mut returned_ids = Vec::with_capacity(returned.len());
        let mut libraries_to_shuffle = HashSet::new();
        let mut returned_planar = Vec::new();
        let mut returned_schemes = Vec::new();
        let mut returned_vanguards = HashMap::new();
        for mut card in returned {
            let zone = match card.destination {
                ReturnedDestination::Library => Zone::Library,
                _ => Zone::Command,
            };
            if zone == Zone::Command {
                for ability in &mut card.definition.abilities {
                    ability.functional_zones = match card.destination {
                        ReturnedDestination::Vanguard | ReturnedDestination::Commander => {
                            vec![Zone::Command]
                        }
                        _ => Vec::new(),
                    };
                }
            }
            let object =
                self.create_transferred_object(&card.definition, card.owner, zone, card.stable_id);
            returned_ids.push(object);
            match card.destination {
                ReturnedDestination::Library => {
                    libraries_to_shuffle.insert(card.owner);
                    if card.commander {
                        self.set_as_commander(object, card.owner);
                    }
                }
                ReturnedDestination::Commander => self.set_as_commander(object, card.owner),
                ReturnedDestination::Vanguard => {
                    returned_vanguards.insert(card.owner, object);
                }
                ReturnedDestination::PlanarDeck {
                    owner,
                    kind,
                    communal,
                } => returned_planar.push((owner, object, kind, communal)),
                ReturnedDestination::SchemeDeck { owner } => {
                    returned_schemes.push((owner, object));
                }
            }
        }

        for player in libraries_to_shuffle {
            self.shuffle_player_library(player);
        }
        if !returned_vanguards.is_empty() {
            self.vanguard = Some(VanguardState {
                cards: returned_vanguards,
                hand_modifiers: vanguard_modifiers
                    .iter()
                    .map(|(owner, (hand, _))| (*owner, *hand))
                    .collect(),
                life_modifiers: vanguard_modifiers
                    .iter()
                    .map(|(owner, (_, life))| (*owner, *life))
                    .collect(),
            });
            self.synchronize_vanguard_ability_zones();
        }
        if !returned_planar.is_empty() {
            let communal = returned_planar.iter().any(|(_, _, _, communal)| *communal);
            let mut state = self.planechase.take().unwrap_or(PlanechaseState {
                decks: HashMap::new(),
                communal_deck: communal.then(Vec::new),
                deck_owners: HashMap::new(),
                card_kinds: HashMap::new(),
                face_up: Vec::new(),
                planar_controller: self.turn.active_player,
                planar_controllers: HashSet::from([self.turn.active_player]),
                face_up_controllers: HashMap::new(),
                voluntary_rolls_this_turn: HashMap::new(),
                planeswalk_count: 0,
            });
            for (owner, object, kind, is_communal) in returned_planar {
                state.deck_owners.insert(object, owner);
                state.card_kinds.insert(object, kind);
                if is_communal {
                    state
                        .communal_deck
                        .get_or_insert_with(Vec::new)
                        .push(object);
                } else {
                    state.decks.entry(owner).or_default().push(object);
                }
            }
            for deck in state.decks.values_mut() {
                self.shuffle_slice(deck);
            }
            if let Some(deck) = state.communal_deck.as_mut() {
                self.shuffle_slice(deck);
            }
            self.planechase = Some(state);
        }
        if !returned_schemes.is_empty() {
            let mut state = self.archenemy.take().unwrap_or(ArchenemyState {
                variant: archenemy_variant.unwrap_or(ArchenemyVariant::Default),
                archenemies,
                scheme_decks: HashMap::new(),
                face_up: Vec::new(),
            });
            for (owner, object) in returned_schemes {
                state.scheme_decks.entry(owner).or_default().push(object);
            }
            for deck in state.scheme_decks.values_mut() {
                self.shuffle_slice(deck);
            }
            self.archenemy = Some(state);
        }

        if !nonwinner_effects.is_empty() && !nonwinners.is_empty() {
            let source = resolving_object.unwrap_or_else(|| ObjectId::from_raw(0));
            let mut ctx = ExecutionContext::new(source, controller, decision_maker);
            let tag = TagKey::from(SUBGAME_NONWINNERS_TAG);
            ctx.tagged_players.insert(tag.clone(), nonwinners.clone());
            let continuation = crate::effects::ForPlayersEffect::new(
                crate::target::PlayerFilter::TaggedPlayer(tag),
                nonwinner_effects,
            );
            let outcome = execute_effect(self, &Effect::new(continuation), &mut ctx)?;
            for event in outcome.events {
                self.queue_trigger_event(ctx.provenance, event);
            }
        }

        if let Some(resolving_object) = resolving_object
            && let Some(object) = self.object(resolving_object).cloned()
            && object.zone == Zone::Stack
        {
            if object.is_permanent() {
                let _ = self.move_object_with_etb_processing_with_dm(
                    resolving_object,
                    Zone::Battlefield,
                    decision_maker,
                );
            } else {
                let _ = self.move_object_by_effect(resolving_object, Zone::Graveyard);
            }
        }

        Ok(SubgameCompletion {
            result,
            nonwinners,
            returned_cards: returned_ids,
            resumed_depth: self.subgame_depth(),
        })
    }
}
