use super::*;

use crate::static_abilities::StaticAbilityId;

/// Visibility of a card in a drafted pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftVisibility {
    FaceDown,
    FaceUp,
}

/// Stable draft-local identity and state for one card.
#[derive(Debug, Clone)]
pub struct DraftCard {
    pub id: u64,
    pub definition: crate::cards::CardDefinition,
    pub visibility: DraftVisibility,
    /// Public information noted while this card was revealed as drafted.
    pub public_note: Option<String>,
}

/// One player's simultaneous choice for a single draft step.
#[derive(Debug, Clone, Default)]
pub struct DraftSelection {
    pub player: PlayerId,
    /// The format's ordinary pick count, plus any explicitly granted extra pick.
    pub card_ids: Vec<u64>,
    /// A previously drafted face-up card returned to the current pack.
    pub exchange_face_up: Option<u64>,
    /// Public notes required by reveal-as-drafted cards, keyed by draft-card ID.
    pub public_notes: HashMap<u64, String>,
}

/// Viewer-safe description of one drafted card slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftCardView {
    pub id: u64,
    pub name: Option<String>,
    pub face_up: bool,
    pub public_note: Option<String>,
}

/// Three-round Conspiracy draft state. Packs are passed by seat after each
/// simultaneous batch of picks; no active player or priority is involved.
#[derive(Debug, Clone)]
pub struct ConspiracyDraftState {
    players: Vec<PlayerId>,
    unopened: HashMap<PlayerId, Vec<Vec<crate::cards::CardDefinition>>>,
    current_packs: HashMap<PlayerId, Vec<DraftCard>>,
    drafted: HashMap<PlayerId, Vec<DraftCard>>,
    round: u8,
    next_card_id: u64,
    ordinary_pick_count: usize,
    complete: bool,
}

fn draft_rule_labels(definition: &crate::cards::CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::DraftRuleText => {
                Some(ability.display().to_ascii_lowercase())
            }
            _ => None,
        })
        .collect()
}

fn is_drafted_face_up(definition: &crate::cards::CardDefinition) -> bool {
    draft_rule_labels(definition)
        .iter()
        .any(|label| label.trim_end_matches('.') == "draft this card face up")
}

fn is_revealed_and_noted(definition: &crate::cards::CardDefinition) -> bool {
    draft_rule_labels(definition)
        .iter()
        .any(|label| label.starts_with("reveal this card as you draft it"))
}

fn grants_exchange_pick(definition: &crate::cards::CardDefinition) -> bool {
    draft_rule_labels(definition).iter().any(|label| {
        label.contains("draft an additional card from that booster pack")
            && label.contains("put this card into that booster pack")
    })
}

impl ConspiracyDraftState {
    /// Create a canonical three-round draft. Every player supplies exactly one
    /// pack for each round in round order.
    pub fn new(
        players: Vec<PlayerId>,
        packs: Vec<(PlayerId, Vec<Vec<crate::cards::CardDefinition>>)>,
    ) -> Result<Self, String> {
        Self::new_with_pick_count(players, packs, 1)
    }

    pub(super) fn new_with_pick_count(
        players: Vec<PlayerId>,
        packs: Vec<(PlayerId, Vec<Vec<crate::cards::CardDefinition>>)>,
        ordinary_pick_count: usize,
    ) -> Result<Self, String> {
        if players.len() < 2 {
            return Err("a draft requires at least two players".to_string());
        }
        if ordinary_pick_count == 0 {
            return Err("a draft must select at least one card per pick".to_string());
        }
        let player_set = players.iter().copied().collect::<HashSet<_>>();
        if player_set.len() != players.len() {
            return Err("draft seats must contain distinct players".to_string());
        }
        if packs.len() != players.len() {
            return Err("each draft seat must provide three booster packs".to_string());
        }
        let mut unopened = HashMap::new();
        for (player, player_packs) in packs {
            if !player_set.contains(&player) || unopened.contains_key(&player) {
                return Err("booster packs must belong to distinct draft seats".to_string());
            }
            if player_packs.len() != 3 || player_packs.iter().any(Vec::is_empty) {
                return Err("each draft seat must provide exactly three nonempty packs".to_string());
            }
            unopened.insert(player, player_packs);
        }
        let mut state = Self {
            drafted: players
                .iter()
                .copied()
                .map(|player| (player, Vec::new()))
                .collect(),
            players,
            unopened,
            current_packs: HashMap::new(),
            round: 0,
            next_card_id: 1,
            ordinary_pick_count,
            complete: false,
        };
        state.open_round()?;
        Ok(state)
    }

    pub fn round(&self) -> u8 {
        self.round
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    fn wrap_pack(&mut self, definitions: Vec<crate::cards::CardDefinition>) -> Vec<DraftCard> {
        definitions
            .into_iter()
            .map(|definition| {
                let id = self.next_card_id;
                self.next_card_id = self.next_card_id.saturating_add(1);
                DraftCard {
                    id,
                    definition,
                    visibility: DraftVisibility::FaceDown,
                    public_note: None,
                }
            })
            .collect()
    }

    fn open_round(&mut self) -> Result<(), String> {
        if self.round >= 3 {
            self.complete = true;
            self.current_packs.clear();
            return Ok(());
        }
        self.current_packs.clear();
        for player in self.players.clone() {
            let definitions = self
                .unopened
                .get_mut(&player)
                .and_then(|packs| (!packs.is_empty()).then(|| packs.remove(0)))
                .ok_or_else(|| format!("missing round {} pack", self.round + 1))?;
            let pack = self.wrap_pack(definitions);
            self.current_packs.insert(player, pack);
        }
        self.round += 1;
        Ok(())
    }

    /// Resolve one simultaneous set of picks, then pass every nonempty pack in
    /// the canonical direction. The operation validates against a clone first,
    /// so an illegal batch cannot partially mutate private draft state.
    pub fn draft_step(&mut self, selections: Vec<DraftSelection>) -> Result<(), String> {
        if self.complete {
            return Err("the draft is complete".to_string());
        }
        let mut next = self.clone();
        next.apply_draft_step(selections)?;
        *self = next;
        Ok(())
    }

    fn apply_draft_step(&mut self, selections: Vec<DraftSelection>) -> Result<(), String> {
        let expected = self
            .players
            .iter()
            .filter(|player| {
                self.current_packs
                    .get(player)
                    .is_some_and(|pack| !pack.is_empty())
            })
            .copied()
            .collect::<HashSet<_>>();
        let supplied = selections
            .iter()
            .map(|selection| selection.player)
            .collect::<HashSet<_>>();
        if supplied.len() != selections.len() || supplied != expected {
            return Err(
                "each player holding a nonempty pack must make exactly one draft selection"
                    .to_string(),
            );
        }

        for selection in selections {
            let extra = selection.exchange_face_up.is_some();
            let pack_len = self
                .current_packs
                .get(&selection.player)
                .expect("validated current pack")
                .len();
            let required = self.ordinary_pick_count.min(pack_len) + usize::from(extra);
            if selection.card_ids.len() != required {
                return Err(format!(
                    "player {:?} must draft {required} card(s)",
                    selection.player
                ));
            }
            let mut unique = selection.card_ids.clone();
            unique.sort_unstable();
            unique.dedup();
            if unique.len() != selection.card_ids.len() {
                return Err("a draft selection cannot choose the same card twice".to_string());
            }

            let pack = self
                .current_packs
                .get_mut(&selection.player)
                .expect("validated current pack");
            let mut indices = Vec::new();
            for card_id in &selection.card_ids {
                let index = pack
                    .iter()
                    .position(|card| card.id == *card_id)
                    .ok_or_else(|| {
                        format!("draft card {card_id} is not in that player's current pack")
                    })?;
                indices.push(index);
            }
            indices.sort_unstable_by(|left, right| right.cmp(left));
            let mut picked = indices
                .into_iter()
                .map(|index| pack.remove(index))
                .collect::<Vec<_>>();
            picked.sort_by_key(|card| {
                selection
                    .card_ids
                    .iter()
                    .position(|id| *id == card.id)
                    .unwrap_or(usize::MAX)
            });

            if let Some(exchange) = selection.exchange_face_up {
                let drafted = self
                    .drafted
                    .get_mut(&selection.player)
                    .expect("validated draft seat");
                let index = drafted
                    .iter()
                    .position(|card| card.id == exchange)
                    .ok_or_else(|| {
                        "the exchanged card is not in that player's drafted pile".to_string()
                    })?;
                if drafted[index].visibility != DraftVisibility::FaceUp
                    || !grants_exchange_pick(&drafted[index].definition)
                {
                    return Err("an additional pick requires returning a face-up card whose draft ability grants it".to_string());
                }
                let mut returned = drafted.remove(index);
                returned.visibility = DraftVisibility::FaceDown;
                returned.public_note = None;
                pack.push(returned);
            }

            for mut card in picked {
                if is_revealed_and_noted(&card.definition) {
                    let note = selection
                        .public_notes
                        .get(&card.id)
                        .map(|note| note.trim())
                        .filter(|note| !note.is_empty())
                        .ok_or_else(|| {
                            format!("draft card {} requires public noted information", card.id)
                        })?;
                    card.public_note = Some(note.to_string());
                    card.visibility = DraftVisibility::FaceDown;
                } else if selection.public_notes.contains_key(&card.id) {
                    return Err(format!(
                        "draft card {} does not instruct its drafter to note information",
                        card.id
                    ));
                } else if is_drafted_face_up(&card.definition) {
                    card.visibility = DraftVisibility::FaceUp;
                }
                self.drafted
                    .get_mut(&selection.player)
                    .expect("validated drafted pile")
                    .push(card);
            }
        }

        if self.current_packs.values().all(Vec::is_empty) {
            return self.open_round();
        }

        let pass_left = self.round == 1 || self.round == 3;
        let mut passed = HashMap::new();
        let player_count = self.players.len();
        for (index, holder) in self.players.iter().copied().enumerate() {
            let destination_index = if pass_left {
                (index + 1) % player_count
            } else {
                (index + player_count - 1) % player_count
            };
            let destination = self.players[destination_index];
            let pack = self.current_packs.remove(&holder).unwrap_or_default();
            passed.insert(destination, pack);
        }
        self.current_packs = passed;
        Ok(())
    }

    /// The current pack is visible only to the player presently holding it.
    pub fn current_pack_view(&self, viewer: PlayerId, holder: PlayerId) -> Vec<DraftCardView> {
        self.current_packs
            .get(&holder)
            .into_iter()
            .flatten()
            .map(|card| DraftCardView {
                id: card.id,
                name: (viewer == holder).then(|| card.definition.name().to_string()),
                face_up: false,
                public_note: None,
            })
            .collect()
    }

    /// Owners may inspect their whole drafted pile; opponents see names only
    /// for face-up picks. Public notes remain visible even after a card turns down.
    pub fn drafted_view(&self, viewer: PlayerId, owner: PlayerId) -> Vec<DraftCardView> {
        self.drafted
            .get(&owner)
            .into_iter()
            .flatten()
            .map(|card| DraftCardView {
                id: card.id,
                name: (viewer == owner || card.visibility == DraftVisibility::FaceUp)
                    .then(|| card.definition.name().to_string()),
                face_up: card.visibility == DraftVisibility::FaceUp,
                public_note: card.public_note.clone(),
            })
            .collect()
    }

    /// Return a player's limited card pool after all three rounds finish.
    pub fn card_pool(&self, player: PlayerId) -> Result<Vec<crate::cards::CardDefinition>, String> {
        if !self.complete {
            return Err(
                "the draft card pool is not final until all three rounds finish".to_string(),
            );
        }
        Ok(self
            .drafted
            .get(&player)
            .into_iter()
            .flatten()
            .map(|card| card.definition.clone())
            .collect())
    }

    /// Validate a Conspiracy limited main deck against the player's completed
    /// draft pool. Basic lands are available without limit; every other card
    /// must have been drafted, and conspiracies may never be in the deck.
    pub fn validate_deck(
        &self,
        player: PlayerId,
        deck: &[crate::cards::CardDefinition],
    ) -> Result<(), String> {
        if deck.len() < 40 {
            return Err("a Conspiracy Draft deck must contain at least 40 cards".to_string());
        }
        let mut available = HashMap::<String, usize>::new();
        for definition in self.card_pool(player)? {
            *available
                .entry(definition.name().trim().to_ascii_lowercase())
                .or_default() += 1;
        }
        for definition in deck {
            if definition.card.has_card_type(CardType::Conspiracy) {
                return Err(format!(
                    "{} cannot be included in a deck",
                    definition.name()
                ));
            }
            if definition
                .card
                .has_supertype(crate::types::Supertype::Basic)
                && definition.card.has_card_type(CardType::Land)
            {
                continue;
            }
            let name = definition.name().trim().to_ascii_lowercase();
            let remaining = available.get_mut(&name).ok_or_else(|| {
                format!(
                    "{} is not in that player's drafted card pool",
                    definition.name()
                )
            })?;
            if *remaining == 0 {
                return Err(format!(
                    "the deck contains more copies of {} than that player drafted",
                    definition.name()
                ));
            }
            *remaining -= 1;
        }
        Ok(())
    }
}

fn agenda_name_count(definition: &crate::cards::CardDefinition) -> usize {
    let mut count = 0;
    for ability in &definition.abilities {
        let AbilityKind::Static(ability) = &ability.kind else {
            continue;
        };
        count = match ability.id() {
            StaticAbilityId::DoubleAgenda => count.max(2),
            StaticAbilityId::HiddenAgenda => count.max(1),
            _ => count,
        };
    }
    count
}

impl GameState {
    pub(crate) fn face_down_conspiracy_characteristics(
        &self,
        object: ObjectId,
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        if !self.is_face_down_conspiracy(object) {
            return None;
        }
        let owner = self.object(object)?.owner;
        Some(crate::continuous::CalculatedCharacteristics {
            name: "".into(),
            mana_cost: None,
            compiled_card_text: std::sync::Arc::<str>::from(""),
            power: None,
            toughness: None,
            card_types: Vec::new().into(),
            subtypes: Vec::new().into(),
            supertypes: Vec::new().into(),
            world_supertype_since: None,
            colors: crate::color::ColorSet::COLORLESS,
            loyalty: None,
            abilities: Vec::new().into(),
            static_abilities: Vec::new().into(),
            ability_gain_prohibitions: Vec::new(),
            aura_attach_filter: None,
            controller: owner,
        })
    }

    /// Enable the post-draft Conspiracy game and put the selected sideboard
    /// conspiracies into command before libraries are shuffled.
    pub fn enable_conspiracy(
        &mut self,
        selections: Vec<(PlayerId, Vec<ConspiracySetupCard>)>,
    ) -> Result<(), String> {
        if self.conspiracy.is_some() {
            return Err("Conspiracy Draft is already enabled".to_string());
        }
        let live_players = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        let mut seen = HashSet::new();
        for (player, cards) in &selections {
            if !live_players.contains(player) || !seen.insert(*player) {
                return Err(
                    "conspiracy selections must belong to distinct players in the game".to_string(),
                );
            }
            for card in cards {
                if !card
                    .definition
                    .card
                    .card_types
                    .contains(&CardType::Conspiracy)
                {
                    return Err(format!(
                        "{} is not a Conspiracy card",
                        card.definition.name()
                    ));
                }
                if !card.definition.card.subtypes.is_empty() {
                    return Err(format!(
                        "Conspiracy card {} may not have subtypes",
                        card.definition.name()
                    ));
                }
                let expected = agenda_name_count(&card.definition);
                if card.agenda_names.len() != expected {
                    return Err(format!(
                        "{} requires exactly {expected} secret agenda name(s)",
                        card.definition.name()
                    ));
                }
                let normalized = card
                    .agenda_names
                    .iter()
                    .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
                    .collect::<Vec<_>>();
                if normalized.iter().any(String::is_empty)
                    || normalized
                        .iter()
                        .map(|name| name.to_ascii_lowercase())
                        .collect::<HashSet<_>>()
                        .len()
                        != normalized.len()
                {
                    return Err("agenda names must be nonempty and different".to_string());
                }
            }
        }

        let mut state = ConspiracyState::default();
        for (owner, cards) in selections {
            for mut setup in cards {
                let hidden = agenda_name_count(&setup.definition) > 0;
                for ability in &mut setup.definition.abilities {
                    if hidden {
                        ability.functional_zones.clear();
                    } else {
                        ability.functional_zones = vec![Zone::Command];
                    }
                }
                let object =
                    self.create_object_from_definition(&setup.definition, owner, Zone::Command);
                state.cards.entry(owner).or_default().push(object);
                if hidden {
                    self.set_chosen_named_option(object, setup.agenda_names.join("\n"));
                    state.face_down.insert(object);
                    state.agenda_names.insert(
                        object,
                        setup
                            .agenda_names
                            .into_iter()
                            .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
                            .collect(),
                    );
                }
            }
        }
        for player in &mut self.players {
            if player.is_in_game() {
                player.starting_life = 20;
                player.life = 20;
            }
        }
        self.conspiracy = Some(state);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn conspiracy_cards(&self) -> Vec<ObjectId> {
        let mut cards = self
            .conspiracy
            .as_ref()
            .into_iter()
            .flat_map(|state| state.cards.values().flatten().copied())
            .collect::<Vec<_>>();
        cards.sort_by_key(|card| card.0);
        cards
    }

    pub fn is_conspiracy_card(&self, object: ObjectId) -> bool {
        self.conspiracy_cards().contains(&object)
    }

    pub fn is_face_down_conspiracy(&self, object: ObjectId) -> bool {
        self.conspiracy
            .as_ref()
            .is_some_and(|state| state.face_down.contains(&object))
    }

    pub fn agenda_names_for(&self, viewer: PlayerId, object: ObjectId) -> Option<&[String]> {
        let state = self.conspiracy.as_ref()?;
        let card = self.object(object)?;
        (card.owner == viewer || !state.face_down.contains(&object))
            .then(|| state.agenda_names.get(&object).map(Vec::as_slice))
            .flatten()
    }

    pub fn turn_conspiracy_face_up(
        &mut self,
        player: PlayerId,
        object: ObjectId,
    ) -> Result<(), String> {
        let card = self
            .object(object)
            .ok_or_else(|| "conspiracy is missing".to_string())?;
        if card.owner != player
            || !self
                .conspiracy
                .as_ref()
                .is_some_and(|state| state.face_down.contains(&object))
        {
            return Err(
                "only a face-down conspiracy you control may be turned face up".to_string(),
            );
        }
        self.conspiracy
            .as_mut()
            .expect("validated Conspiracy state")
            .face_down
            .remove(&object);
        if let Some(card) = self.object_mut(object) {
            for ability in card.abilities_mut() {
                ability.functional_zones = vec![Zone::Command];
            }
        }
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn reveal_conspiracies_for_player(&mut self, player: PlayerId) {
        let objects = self
            .conspiracy
            .as_ref()
            .and_then(|state| state.cards.get(&player))
            .cloned()
            .unwrap_or_default();
        for object in objects {
            if self.is_face_down_conspiracy(object) {
                let _ = self.turn_conspiracy_face_up(player, object);
            }
        }
    }

    pub fn synchronize_conspiracy_ability_zones(&mut self) {
        let hidden = self
            .conspiracy
            .as_ref()
            .map(|state| state.face_down.clone())
            .unwrap_or_default();
        for object in self.conspiracy_cards() {
            if let Some(card) = self.object_mut(object) {
                for ability in card.abilities_mut() {
                    ability.functional_zones = if hidden.contains(&object) {
                        Vec::new()
                    } else {
                        vec![Zone::Command]
                    };
                }
            }
        }
        self.mark_continuous_state_dirty();
    }

    pub(crate) fn handle_conspiracy_player_departure(&mut self, player: PlayerId) {
        self.reveal_conspiracies_for_player(player);
        if let Some(state) = self.conspiracy.as_mut() {
            if let Some(cards) = state.cards.remove(&player) {
                for object in cards {
                    state.face_down.remove(&object);
                    state.agenda_names.remove(&object);
                }
            }
        }
    }
}
