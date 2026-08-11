use super::*;

use std::collections::HashSet;

use crate::ability::{Ability, AbilityKind};
use crate::effect::Effect;
use crate::events::other::DieRolledEvent;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::provenance::ProvNodeId;
use crate::triggers::{
    Trigger, TriggerEvent, TriggerIdentity, TriggeredAbilityEntry, TriggeredAbilitySourceKind,
};

impl GameState {
    /// Enable ordinary per-player Planechase and create each supplementary deck.
    ///
    /// Face-down planar cards begin with dormant abilities. Turning one face up
    /// enables those abilities in the command zone, where CR 311/312 keeps it.
    pub fn enable_planechase(
        &mut self,
        decks: Vec<(
            PlayerId,
            Vec<(crate::cards::CardDefinition, PlanarCardKind)>,
        )>,
    ) -> Result<(), String> {
        if self.planechase.is_some() {
            return Err("Planechase is already enabled".to_string());
        }
        if decks.len()
            != self
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .count()
        {
            return Err("each player must provide exactly one planar deck".to_string());
        }

        let mut seen_players = HashSet::new();
        for (player, cards) in &decks {
            if !seen_players.insert(*player)
                || self
                    .player(*player)
                    .is_none_or(|candidate| !candidate.is_in_game())
            {
                return Err("planar decks must belong to distinct players in the game".to_string());
            }
            Self::validate_planar_card_list(cards, 10, 2)?;
        }

        let planar_controller = self.turn.active_player;
        let mut state = PlanechaseState {
            decks: HashMap::new(),
            communal_deck: None,
            deck_owners: HashMap::new(),
            card_kinds: HashMap::new(),
            face_up: Vec::new(),
            planar_controller,
            planar_controllers: HashSet::from([planar_controller]),
            face_up_controllers: HashMap::new(),
            voluntary_rolls_this_turn: HashMap::new(),
            planeswalk_count: 0,
        };

        for (owner, cards) in decks {
            let mut deck = Vec::with_capacity(cards.len());
            for (mut definition, kind) in cards {
                for ability in &mut definition.abilities {
                    ability.functional_zones.clear();
                }
                let object = self.create_object_from_definition(&definition, owner, Zone::Command);
                deck.push(object);
                state.deck_owners.insert(object, owner);
                state.card_kinds.insert(object, kind);
            }
            self.shuffle_slice(&mut deck);
            state.decks.insert(owner, deck);
        }

        self.planechase = Some(state);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    /// Enable the CR 901.15 single communal planar-deck option.
    pub fn enable_planechase_communal(
        &mut self,
        cards: Vec<(crate::cards::CardDefinition, PlanarCardKind)>,
    ) -> Result<(), String> {
        if self.planechase.is_some() {
            return Err("Planechase is already enabled".to_string());
        }
        let player_count = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .count();
        let minimum = 40usize.min(player_count.saturating_mul(10));
        Self::validate_planar_card_list(&cards, minimum, player_count.saturating_mul(2))?;

        let planar_controller = self.turn.active_player;
        let mut state = PlanechaseState {
            decks: HashMap::new(),
            communal_deck: Some(Vec::with_capacity(cards.len())),
            deck_owners: HashMap::new(),
            card_kinds: HashMap::new(),
            face_up: Vec::new(),
            planar_controller,
            planar_controllers: HashSet::from([planar_controller]),
            face_up_controllers: HashMap::new(),
            voluntary_rolls_this_turn: HashMap::new(),
            planeswalk_count: 0,
        };
        for (mut definition, kind) in cards {
            for ability in &mut definition.abilities {
                ability.functional_zones.clear();
            }
            let object =
                self.create_object_from_definition(&definition, planar_controller, Zone::Command);
            state.card_kinds.insert(object, kind);
            state
                .communal_deck
                .as_mut()
                .expect("communal deck initialized")
                .push(object);
        }
        self.shuffle_slice(
            state
                .communal_deck
                .as_mut()
                .expect("communal deck initialized"),
        );
        self.planechase = Some(state);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    fn validate_planar_card_list(
        cards: &[(crate::cards::CardDefinition, PlanarCardKind)],
        minimum: usize,
        maximum_phenomena: usize,
    ) -> Result<(), String> {
        if cards.len() < minimum {
            return Err(format!(
                "a planar deck must contain at least {minimum} cards"
            ));
        }
        let phenomena = cards
            .iter()
            .filter(|(_, kind)| *kind == PlanarCardKind::Phenomenon)
            .count();
        if phenomena > maximum_phenomena {
            return Err(format!(
                "a planar deck may contain no more than {maximum_phenomena} phenomenon cards"
            ));
        }
        let mut names = HashSet::new();
        if let Some(duplicate) = cards.iter().find_map(|(definition, _)| {
            let normalized = definition.name().trim().to_ascii_lowercase();
            (!names.insert(normalized)).then(|| definition.name().to_string())
        }) {
            return Err(format!(
                "a planar deck may not contain two cards named {duplicate}"
            ));
        }
        Ok(())
    }

    pub fn planar_controller(&self) -> Option<PlayerId> {
        self.planechase
            .as_ref()
            .map(|state| state.planar_controller)
    }

    pub fn set_planar_controller(&mut self, player: PlayerId) -> bool {
        if self
            .player(player)
            .is_none_or(|candidate| !candidate.is_in_game())
        {
            return false;
        }
        let grand_melee_controllers = self.grand_melee().map(|_| {
            self.grand_melee_active_players()
                .into_iter()
                .collect::<HashSet<_>>()
        });
        let Some(state) = self.planechase.as_mut() else {
            return false;
        };
        if state.planar_controller == player {
            return false;
        }
        let old_controller = state.planar_controller;
        state.planar_controller = player;
        if let Some(controllers) = grand_melee_controllers {
            for controller in state.face_up_controllers.values_mut() {
                if *controller == old_controller {
                    *controller = player;
                }
            }
            state.planar_controllers = controllers;
        } else {
            state.planar_controllers.clear();
            state.planar_controllers.insert(player);
            for controller in state.face_up_controllers.values_mut() {
                *controller = player;
            }
        }
        let communal_cards = state
            .communal_deck
            .as_ref()
            .map(|deck| {
                deck.iter()
                    .copied()
                    .chain(state.face_up.iter().copied())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for object in communal_cards {
            if let Some(card) = self.object_mut(object) {
                card.owner = player;
            }
        }
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        true
    }

    pub(crate) fn focus_planar_controller_for_grand_melee(&mut self, player: PlayerId) {
        if self.grand_melee().is_none() {
            return;
        }
        if let Some(state) = self.planechase.as_mut() {
            state.planar_controller = player;
        }
    }

    pub fn planar_controllers(&self) -> Vec<PlayerId> {
        self.planechase
            .as_ref()
            .map(|state| {
                let mut controllers = state.planar_controllers.iter().copied().collect::<Vec<_>>();
                controllers.sort();
                controllers
            })
            .unwrap_or_default()
    }

    pub fn planar_controller_of_face(&self, object: ObjectId) -> Option<PlayerId> {
        self.planechase
            .as_ref()?
            .face_up_controllers
            .get(&object)
            .copied()
    }

    pub fn reset_planar_rolls_for_turn(&mut self) {
        let grand_melee_turn_players = self
            .grand_melee()
            .is_some()
            .then(|| self.turn_players().into_iter().collect::<HashSet<_>>());
        let Some(state) = self.planechase.as_mut() else {
            return;
        };
        let before = state.voluntary_rolls_this_turn.len();
        if let Some(players) = grand_melee_turn_players {
            state
                .voluntary_rolls_this_turn
                .retain(|player, _| !players.contains(player));
        } else {
            state.voluntary_rolls_this_turn.clear();
        }
        if state.voluntary_rolls_this_turn.len() != before {
            self.bump_mutation_revision();
        }
    }

    /// Transfer control of the planar zone before its current controller
    /// leaves the game. Abilities of planar cards already pending or on the
    /// stack remain in the game under the new planar controller (CR 901.10).
    pub(crate) fn prepare_planechase_player_departure(&mut self, player: PlayerId) {
        if self.grand_melee().is_some() {
            let marker_reducing = self.take_grand_melee_marker_reducing_departure(player);
            let controlled_faces = self
                .planechase
                .as_ref()
                .map(|state| {
                    state
                        .face_up_controllers
                        .iter()
                        .filter_map(|(object, controller)| {
                            (*controller == player).then_some(*object)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if let Some(state) = self.planechase.as_mut() {
                state.planar_controllers.remove(&player);
            }
            if marker_reducing {
                for object in controlled_faces {
                    let _ = self.turn_face_up_planar_card_down(object);
                }
            }
            return;
        }
        if self.planar_controller() != Some(player) {
            return;
        }
        let Some(next) = self.next_player_in_game_after(player) else {
            return;
        };
        let (planar_objects, planar_stable_ids) = self
            .planechase
            .as_ref()
            .map(|state| {
                let objects = state.card_kinds.keys().copied().collect::<HashSet<_>>();
                let stable_ids = objects
                    .iter()
                    .filter_map(|object| self.object(*object).map(|object| object.stable_id))
                    .collect::<HashSet<_>>();
                (objects, stable_ids)
            })
            .unwrap_or_default();

        self.set_planar_controller(next);
        for entry in &mut self.effect_store.pending_trigger_entries {
            if entry.controller == player
                && (planar_objects.contains(&entry.source)
                    || planar_stable_ids.contains(&entry.source_stable_id))
            {
                entry.controller = next;
            }
        }
        for entry in &mut self.stack {
            if entry.controller == player
                && (planar_objects.contains(&entry.object_id)
                    || entry
                        .source_stable_id
                        .is_some_and(|stable| planar_stable_ids.contains(&stable)))
            {
                entry.controller = next;
            }
        }
    }

    pub fn planar_die_roll_cost(&self, player: PlayerId) -> Option<u32> {
        self.planechase.as_ref().map(|state| {
            state
                .voluntary_rolls_this_turn
                .get(&player)
                .copied()
                .unwrap_or(0)
        })
    }

    pub fn face_up_planar_objects(&self) -> &[ObjectId] {
        self.planechase
            .as_ref()
            .map_or(&[], |state| state.face_up.as_slice())
    }

    pub fn is_face_up_planar_object(&self, object: ObjectId) -> bool {
        self.planechase
            .as_ref()
            .is_some_and(|state| state.face_up.contains(&object))
    }

    pub fn planar_card_kind(&self, object: ObjectId) -> Option<PlanarCardKind> {
        self.planechase.as_ref()?.card_kinds.get(&object).copied()
    }

    pub fn is_planar_card(&self, object: ObjectId) -> bool {
        self.planar_card_kind(object).is_some()
    }

    /// Reapply the face-up/dormant command-zone ability boundary after a
    /// serialized Planechase state has been restored.
    pub fn synchronize_planar_ability_zones(&mut self) {
        let Some(state) = self.planechase.as_ref() else {
            return;
        };
        let face_up = state.face_up.iter().copied().collect::<HashSet<_>>();
        let planar_objects = state.card_kinds.keys().copied().collect::<Vec<_>>();
        for object in planar_objects {
            let is_face_up = face_up.contains(&object);
            if let Some(card) = self.object_mut(object) {
                for ability in card.abilities_mut() {
                    if is_face_up {
                        ability.functional_zones = vec![Zone::Command];
                    } else {
                        ability.functional_zones.clear();
                    }
                }
            }
        }
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }

    pub fn planar_deck(&self, player: PlayerId) -> Option<&[ObjectId]> {
        let state = self.planechase.as_ref()?;
        if let Some(communal) = state.communal_deck.as_ref() {
            Some(communal)
        } else {
            state.decks.get(&player).map(Vec::as_slice)
        }
    }

    pub(crate) fn move_planar_deck_card_to_bottom(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), String> {
        let state = self
            .planechase
            .as_mut()
            .ok_or_else(|| "Planechase is not enabled".to_string())?;
        let deck = if let Some(communal) = state.communal_deck.as_mut() {
            communal
        } else {
            state
                .decks
                .get_mut(&player)
                .ok_or_else(|| "the relevant planar deck is missing".to_string())?
        };
        let position = deck
            .iter()
            .position(|candidate| *candidate == card)
            .ok_or_else(|| "the chosen card is not in the relevant planar deck".to_string())?;
        deck.remove(position);
        deck.insert(0, card);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    /// Reveal the starting plane without causing encounter or planeswalk triggers.
    pub fn reveal_starting_plane(&mut self) -> Result<ObjectId, String> {
        if self.grand_melee().is_some() {
            return Err(
                "Grand Melee Planechase must reveal one starting plane per turn marker".to_string(),
            );
        }
        let player = self
            .planar_controller()
            .ok_or_else(|| "Planechase is not enabled".to_string())?;
        if !self.face_up_planar_objects().is_empty() {
            return Err("the starting plane has already been revealed".to_string());
        }

        let maximum_attempts = self
            .planar_deck(player)
            .map(|deck| deck.len())
            .unwrap_or(0)
            .saturating_add(1);
        for _ in 0..maximum_attempts {
            let object = self.turn_up_top_planar_card(player)?;
            if self.planar_card_kind(object) == Some(PlanarCardKind::Plane) {
                return Ok(object);
            }
            self.turn_face_up_planar_card_down(object)?;
        }
        Err("the planar deck contains no plane card".to_string())
    }

    /// CR 901.14a: every initially marked player is a planar controller and
    /// independently establishes a starting plane.
    pub fn reveal_grand_melee_starting_planes(&mut self) -> Result<Vec<ObjectId>, String> {
        let controllers = self.grand_melee_active_players();
        if controllers.is_empty() || self.planechase.is_none() {
            return Err("Grand Melee Planechase is not enabled".to_string());
        }
        if !self.face_up_planar_objects().is_empty() {
            return Err("the starting planes have already been revealed".to_string());
        }
        if let Some(state) = self.planechase.as_mut() {
            state.planar_controllers = controllers.iter().copied().collect();
        }
        let mut faces = Vec::with_capacity(controllers.len());
        for controller in controllers {
            let maximum_attempts = self
                .planar_deck(controller)
                .map(|deck| deck.len())
                .unwrap_or(0)
                .saturating_add(1);
            let mut starting_plane = None;
            for _ in 0..maximum_attempts {
                let object = self.turn_up_top_planar_card(controller)?;
                if self.planar_card_kind(object) == Some(PlanarCardKind::Plane) {
                    starting_plane = Some(object);
                    break;
                }
                self.turn_face_up_planar_card_down(object)?;
            }
            faces.push(starting_plane.ok_or_else(|| {
                format!(
                    "planar deck for player {} contains no plane card",
                    controller.0
                )
            })?);
        }
        if let Some(holder) = self.focused_grand_melee_holder() {
            self.focus_planar_controller_for_grand_melee(holder);
        }
        Ok(faces)
    }

    fn turn_up_top_planar_card(&mut self, player: PlayerId) -> Result<ObjectId, String> {
        let object = {
            let state = self
                .planechase
                .as_mut()
                .ok_or_else(|| "Planechase is not enabled".to_string())?;
            if let Some(communal) = state.communal_deck.as_mut() {
                communal.pop()
            } else {
                state.decks.get_mut(&player).and_then(Vec::pop)
            }
            .ok_or_else(|| "the relevant planar deck is empty".to_string())?
        };
        if let Some(card) = self.object_mut(object) {
            for ability in card.abilities_mut() {
                ability.functional_zones = vec![Zone::Command];
            }
        }
        self.planechase
            .as_mut()
            .expect("Planechase state exists")
            .face_up
            .push(object);
        self.planechase
            .as_mut()
            .expect("Planechase state exists")
            .face_up_controllers
            .insert(object, player);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(object)
    }

    /// Turn a face-up planar card face down and put the resulting new object on
    /// the bottom of the appropriate planar deck.
    fn turn_face_up_planar_card_down(&mut self, old_id: ObjectId) -> Result<ObjectId, String> {
        let Some(mut object) = self.object(old_id).cloned() else {
            return Err("face-up planar object is missing".to_string());
        };
        let (kind, deck_owner, communal) = {
            let state = self
                .planechase
                .as_ref()
                .ok_or_else(|| "Planechase is not enabled".to_string())?;
            if !state.face_up.contains(&old_id) {
                return Err("the planar card is not face up".to_string());
            }
            (
                state
                    .card_kinds
                    .get(&old_id)
                    .copied()
                    .ok_or_else(|| "planar card kind is missing".to_string())?,
                state.deck_owners.get(&old_id).copied(),
                state.communal_deck.is_some(),
            )
        };

        let stable_id = object.stable_id;
        self.remove_object(old_id);
        let new_id = self.new_object_id();
        object.id = new_id;
        object.stable_id = stable_id;
        object.last_modified = 0;
        object.zone = Zone::Command;
        for ability in object.abilities_mut() {
            ability.functional_zones.clear();
        }
        self.add_object(object);

        let state = self
            .planechase
            .as_mut()
            .ok_or_else(|| "Planechase is not enabled".to_string())?;
        state.face_up.retain(|candidate| *candidate != old_id);
        state.face_up_controllers.remove(&old_id);
        state.card_kinds.remove(&old_id);
        state.deck_owners.remove(&old_id);
        state.card_kinds.insert(new_id, kind);
        if communal {
            state
                .communal_deck
                .as_mut()
                .expect("communal deck exists")
                .insert(0, new_id);
        } else {
            let owner = deck_owner.ok_or_else(|| "planar deck owner is missing".to_string())?;
            state.deck_owners.insert(new_id, owner);
            state.decks.entry(owner).or_default().insert(0, new_id);
        }
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(new_id)
    }

    /// Perform the complete planeswalk keyword action and queue its observations.
    pub fn planeswalk(&mut self, player: PlayerId, source: ObjectId) -> Result<ObjectId, String> {
        let is_planar_controller = if self.grand_melee().is_some() {
            self.planar_controllers().contains(&player)
        } else {
            self.planar_controller() == Some(player)
        };
        if !is_planar_controller {
            return Err("only the planar controller may planeswalk".to_string());
        }
        let old_faces = self
            .face_up_planar_objects()
            .iter()
            .copied()
            .filter(|object| self.planar_controller_of_face(*object) == Some(player))
            .collect::<Vec<_>>();
        let old_face_snapshots = old_faces
            .iter()
            .filter_map(|object| {
                self.object(*object)
                    .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, self))
            })
            .collect::<Vec<_>>();
        for object in old_faces {
            self.turn_face_up_planar_card_down(object)?;
        }
        let destination = self.turn_up_top_planar_card(player)?;
        if let Some(state) = self.planechase.as_mut() {
            state.planeswalk_count = state.planeswalk_count.saturating_add(1);
        }

        let provenance = ProvNodeId::default();
        self.queue_trigger_event(
            provenance,
            TriggerEvent::new(
                KeywordActionEvent::new(KeywordActionKind::Planeswalk, player, source, 1),
                provenance,
            )
            .with_lookback_source_snapshots(old_face_snapshots),
        );
        if self.planar_card_kind(destination) == Some(PlanarCardKind::Phenomenon) {
            let provenance = ProvNodeId::default();
            self.queue_trigger_event(
                provenance,
                TriggerEvent::new(
                    KeywordActionEvent::new(
                        KeywordActionKind::EncounterPhenomenon,
                        player,
                        destination,
                        1,
                    ),
                    provenance,
                ),
            );
        }
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(destination)
    }

    pub fn chaos_ensues(&mut self, player: PlayerId, source: ObjectId) -> Result<(), String> {
        if self.planechase.is_none() {
            return Err("chaos can ensue only during a Planechase game".to_string());
        }
        let provenance = ProvNodeId::default();
        self.queue_trigger_event(
            provenance,
            TriggerEvent::new(
                KeywordActionEvent::new(KeywordActionKind::ChaosEnsues, player, source, 1),
                provenance,
            ),
        );
        Ok(())
    }

    /// Roll the planar die. Voluntary rolls increase the next roll's cost;
    /// effect-driven rolls pass `voluntary = false` and do not.
    pub fn roll_planar_die(
        &mut self,
        player: PlayerId,
        voluntary: bool,
    ) -> Result<PlanarDieFace, String> {
        if self.planechase.is_none() {
            return Err("the planar die exists only during a Planechase game".to_string());
        }
        if voluntary {
            let state = self.planechase.as_mut().expect("checked above");
            *state.voluntary_rolls_this_turn.entry(player).or_default() = state
                .voluntary_rolls_this_turn
                .get(&player)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
        }

        let (random_count_before, random_count_after) = self.record_irreversible_random();
        let raw = self
            .take_forced_die_roll()
            .unwrap_or_else(|| (self.next_random_u64() % 6) as u32 + 1);
        self.push_hidden_info_operation(HiddenInfoOperation::FairRandom {
            random_count_before,
            random_count_after,
            reason: "planar die roll".to_string(),
        });
        let face = match raw {
            1 => PlanarDieFace::Planeswalker,
            2 => PlanarDieFace::Chaos,
            _ => PlanarDieFace::Blank,
        };

        let source = self
            .face_up_planar_objects()
            .iter()
            .copied()
            .find(|object| self.planar_controller_of_face(*object) == Some(player))
            .unwrap_or(ObjectId::from_raw(0));
        let provenance = ProvNodeId::default();
        let die_event =
            TriggerEvent::new(DieRolledEvent::new_planar(player, source, raw), provenance);
        self.queue_trigger_event(provenance, die_event.clone());
        self.record_ui_effect_event(
            "planar_die_roll",
            Some(player),
            None,
            Vec::new(),
            Some(i64::from(raw)),
            Some(
                match face {
                    PlanarDieFace::Blank => "blank",
                    PlanarDieFace::Chaos => "chaos",
                    PlanarDieFace::Planeswalker => "planeswalker",
                }
                .to_string(),
            ),
        );

        match face {
            PlanarDieFace::Blank => {}
            PlanarDieFace::Chaos => self.chaos_ensues(player, source)?,
            PlanarDieFace::Planeswalker => {
                let AbilityKind::Triggered(ability) = Ability::triggered(
                    Trigger::keyword_action(
                        KeywordActionKind::Planeswalk,
                        crate::PlayerFilter::Any,
                    ),
                    vec![Effect::emit_keyword_action(
                        KeywordActionKind::Planeswalk,
                        1,
                    )],
                )
                .kind
                else {
                    unreachable!();
                };
                self.defer_trigger_entries([TriggeredAbilityEntry {
                    source: ObjectId::from_raw(0),
                    controller: player,
                    x_value: None,
                    event_value_amount: None,
                    ability,
                    triggering_event: die_event,
                    source_stable_id: StableId::from_raw(0),
                    source_name: "Planeswalking ability".to_string(),
                    source_snapshot: None,
                    tagged_objects: HashMap::new(),
                    source_kind: TriggeredAbilitySourceKind::GameRule,
                    trigger_identity: TriggerIdentity(0x9018),
                }]);
            }
        }
        self.bump_mutation_revision();
        Ok(face)
    }

    /// Remove stale planar-card bookkeeping when a player leaves, then reveal
    /// the new planar controller's top card if the departed player owned a
    /// face-up plane or phenomenon (CR 901.10).
    pub(crate) fn handle_planechase_player_departure(
        &mut self,
        player: PlayerId,
        removed_objects: &HashSet<ObjectId>,
    ) {
        let grand_melee = self.grand_melee().is_some();
        let next_planar_controller = self
            .turn_store
            .turn_order
            .iter()
            .copied()
            .find(|candidate| {
                *candidate != player
                    && self
                        .player(*candidate)
                        .is_some_and(|remaining| remaining.is_in_game())
            });
        let Some(state) = self.planechase.as_mut() else {
            return;
        };
        let lost_face = state
            .face_up
            .iter()
            .any(|object| removed_objects.contains(object));
        state
            .face_up
            .retain(|object| !removed_objects.contains(object));
        state
            .face_up_controllers
            .retain(|object, _| !removed_objects.contains(object));
        state
            .card_kinds
            .retain(|object, _| !removed_objects.contains(object));
        state
            .deck_owners
            .retain(|object, _| !removed_objects.contains(object));
        for deck in state.decks.values_mut() {
            deck.retain(|object| !removed_objects.contains(object));
        }
        if let Some(deck) = state.communal_deck.as_mut() {
            deck.retain(|object| !removed_objects.contains(object));
        }
        state.decks.remove(&player);
        state.voluntary_rolls_this_turn.remove(&player);

        if !grand_melee && state.planar_controller == player {
            if let Some(next) = next_planar_controller {
                state.planar_controller = next;
            }
        }
        let next_controller = state.planar_controller;
        if lost_face && !grand_melee {
            let _ = self.turn_up_top_planar_card(next_controller);
        }
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }

    pub fn planeswalk_count(&self) -> Option<u64> {
        self.planechase.as_ref().map(|state| state.planeswalk_count)
    }
}
