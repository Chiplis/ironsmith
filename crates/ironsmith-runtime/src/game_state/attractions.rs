use super::*;

/// Snapshot of one face-up Attraction that will be visited by a roll.
#[derive(Debug, Clone)]
pub struct AttractionVisitProfile {
    pub object: ObjectId,
    pub stable_id: StableId,
    pub controller: PlayerId,
    pub name: String,
    pub program: crate::resolution::ResolutionProgram,
}

impl GameState {
    /// Enable Attraction supplementary decks for the players who brought one.
    ///
    /// Constructed decks require ten differently named Attractions; Limited
    /// decks require at least three and may contain duplicate names (CR
    /// 717.2a-b). Each supplied definition is one physical printing, so its
    /// printing-specific `attraction_lights` are retained by stable identity.
    pub fn enable_attractions(
        &mut self,
        decks: Vec<(
            PlayerId,
            AttractionDeckFormat,
            Vec<crate::cards::CardDefinition>,
        )>,
    ) -> Result<(), String> {
        if self.attractions.is_some() {
            return Err("Attraction decks are already enabled".to_string());
        }
        if decks.is_empty() {
            return Err("at least one player must provide an Attraction deck".to_string());
        }

        let live_players = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        let mut seen_players = HashSet::new();
        for (owner, format, cards) in &decks {
            if !live_players.contains(owner) || !seen_players.insert(*owner) {
                return Err(
                    "Attraction decks must belong to distinct players in the game".to_string(),
                );
            }
            Self::validate_attraction_deck(cards, *format)?;
        }

        let mut state = AttractionState {
            decks: HashMap::new(),
            face_up: Vec::new(),
            lights: HashMap::new(),
            visit_programs: HashMap::new(),
        };
        for (owner, _format, cards) in decks {
            let mut deck = Vec::with_capacity(cards.len());
            for definition in cards {
                let visit_program = definition
                    .spell_effect
                    .clone()
                    .expect("validated Attraction visit program");
                let lights = definition.card.attraction_lights.clone();
                let object = self.create_object_from_definition(&definition, owner, Zone::Command);
                let stable_id = self
                    .object(object)
                    .expect("new Attraction object")
                    .stable_id;
                state.lights.insert(stable_id, lights);
                state.visit_programs.insert(stable_id, visit_program);
                deck.push(object);
            }
            self.shuffle_slice(&mut deck);
            state.decks.insert(owner, deck);
        }

        self.attractions = Some(state);
        self.bump_mutation_revision();
        Ok(())
    }

    fn validate_attraction_deck(
        cards: &[crate::cards::CardDefinition],
        format: AttractionDeckFormat,
    ) -> Result<(), String> {
        let minimum = match format {
            AttractionDeckFormat::Constructed => 10,
            AttractionDeckFormat::Limited => 3,
        };
        if cards.len() < minimum {
            return Err(format!(
                "an Attraction deck for this format must contain at least {minimum} cards"
            ));
        }

        let mut names = HashSet::new();
        for definition in cards {
            if !definition.card.subtypes.contains(&Subtype::Attraction) {
                return Err(format!("{} is not an Attraction card", definition.name()));
            }
            if definition.spell_effect.is_none() {
                return Err(format!(
                    "{} has no executable Visit ability program",
                    definition.name()
                ));
            }
            let lights = &definition.card.attraction_lights;
            if lights.is_empty() || lights.iter().any(|light| !(1..=6).contains(light)) {
                return Err(format!(
                    "{} must have printed Attraction lights between 1 and 6",
                    definition.name()
                ));
            }
            if format == AttractionDeckFormat::Constructed {
                let normalized = definition.name().trim().to_ascii_lowercase();
                if !names.insert(normalized) {
                    return Err(format!(
                        "a constructed Attraction deck may not contain two cards named {}",
                        definition.name()
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn has_attraction_deck(&self, player: PlayerId) -> bool {
        self.attractions
            .as_ref()
            .is_some_and(|state| state.decks.contains_key(&player))
    }

    pub fn attraction_deck(&self, player: PlayerId) -> Option<&[ObjectId]> {
        self.attractions
            .as_ref()?
            .decks
            .get(&player)
            .map(Vec::as_slice)
    }

    pub fn face_up_attractions(&self) -> &[ObjectId] {
        self.attractions
            .as_ref()
            .map(|state| state.face_up.as_slice())
            .unwrap_or(&[])
    }

    pub fn attraction_lights(&self, object: ObjectId) -> Option<&[u8]> {
        let stable_id = self.object(object)?.stable_id;
        self.attractions
            .as_ref()?
            .lights
            .get(&stable_id)
            .map(Vec::as_slice)
    }

    pub(crate) fn top_attraction(&self, player: PlayerId) -> Option<ObjectId> {
        self.attraction_deck(player)?.last().copied()
    }

    /// Commit the result of attempting to open the current top Attraction.
    pub(crate) fn finish_opening_attraction(
        &mut self,
        player: PlayerId,
        old_object: ObjectId,
        battlefield_object: Option<ObjectId>,
    ) {
        let Some(state) = self.attractions.as_mut() else {
            return;
        };
        if let Some(deck) = state.decks.get_mut(&player) {
            deck.retain(|candidate| *candidate != old_object);
        }
        if let Some(object) = battlefield_object
            && !state.face_up.contains(&object)
        {
            state.face_up.push(object);
        }
        self.bump_mutation_revision();
    }

    pub(crate) fn note_attraction_left_battlefield(&mut self, object: ObjectId) {
        if let Some(state) = self.attractions.as_mut() {
            state.face_up.retain(|candidate| *candidate != object);
        }
    }

    pub(crate) fn note_attraction_entered_battlefield(&mut self, object: ObjectId) {
        let stable_id = self.object(object).map(|candidate| candidate.stable_id);
        let Some(state) = self.attractions.as_mut() else {
            return;
        };
        if stable_id.is_some_and(|stable_id| state.lights.contains_key(&stable_id))
            && !state.face_up.contains(&object)
        {
            state.face_up.push(object);
        }
    }

    /// Face-up Attractions controlled by `player` whose printed lights match
    /// `roll`, together with their Visit programs.
    pub(crate) fn attraction_visit_profiles(
        &self,
        player: PlayerId,
        roll: u32,
    ) -> Vec<AttractionVisitProfile> {
        let Some(state) = self.attractions.as_ref() else {
            return Vec::new();
        };
        state
            .face_up
            .iter()
            .filter_map(|object_id| {
                let object = self.object(*object_id)?;
                if object.zone != Zone::Battlefield
                    || self.current_controller(*object_id) != Some(player)
                    || !state
                        .lights
                        .get(&object.stable_id)
                        .is_some_and(|lights| lights.contains(&(roll as u8)))
                {
                    return None;
                }
                let program = state.visit_programs.get(&object.stable_id)?.clone();
                Some(AttractionVisitProfile {
                    object: *object_id,
                    stable_id: object.stable_id,
                    controller: player,
                    name: object.name.to_string(),
                    program,
                })
            })
            .collect()
    }

    pub(crate) fn handle_attraction_player_departure(&mut self, player: PlayerId) {
        let departing_faces = self
            .attractions
            .as_ref()
            .map(|state| {
                state
                    .face_up
                    .iter()
                    .copied()
                    .filter(|object| {
                        self.object(*object)
                            .is_none_or(|candidate| candidate.owner == player)
                    })
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        let Some(state) = self.attractions.as_mut() else {
            return;
        };
        state.decks.remove(&player);
        state
            .face_up
            .retain(|object| !departing_faces.contains(object));
    }
}
