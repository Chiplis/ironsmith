use super::*;

impl GameState {
    /// Enable an Archenemy profile and construct each face-down scheme deck.
    pub fn enable_archenemy(
        &mut self,
        variant: ArchenemyVariant,
        decks: Vec<(PlayerId, Vec<crate::cards::CardDefinition>)>,
    ) -> Result<(), String> {
        if self.archenemy.is_some() {
            return Err("Archenemy is already enabled".to_string());
        }
        let live_players = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        let expected_decks = match variant {
            ArchenemyVariant::Default | ArchenemyVariant::Commander => 1,
            ArchenemyVariant::SupervillainRumble => live_players.len(),
        };
        if decks.len() != expected_decks {
            return Err(match variant {
                ArchenemyVariant::SupervillainRumble => {
                    "Supervillain Rumble requires one scheme deck per player".to_string()
                }
                _ => "Archenemy requires exactly one designated archenemy".to_string(),
            });
        }

        let mut seen = HashSet::new();
        for (owner, cards) in &decks {
            if !live_players.contains(owner) || !seen.insert(*owner) {
                return Err("scheme decks must belong to distinct players in the game".to_string());
            }
            Self::validate_scheme_deck(cards, variant)?;
        }

        let archenemies = decks
            .iter()
            .map(|(owner, _)| *owner)
            .collect::<HashSet<_>>();
        let mut state = ArchenemyState {
            variant,
            archenemies: archenemies.clone(),
            scheme_decks: HashMap::new(),
            face_up: Vec::new(),
        };
        for (owner, cards) in decks {
            let mut deck = Vec::with_capacity(cards.len());
            for mut definition in cards {
                for ability in &mut definition.abilities {
                    ability.functional_zones.clear();
                }
                deck.push(self.create_object_from_definition(&definition, owner, Zone::Command));
            }
            self.shuffle_slice(&mut deck);
            state.scheme_decks.insert(owner, deck);
        }

        for player in &mut self.players {
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

        if variant != ArchenemyVariant::SupervillainRumble {
            let archenemy = *archenemies.iter().next().expect("validated one archenemy");
            if let Some(index) = self
                .turn_store
                .turn_order
                .iter()
                .position(|player| *player == archenemy)
            {
                self.turn_store.turn_order.rotate_left(index);
            }
            self.turn.active_player = archenemy;
            self.turn.priority_player = Some(archenemy);
        }

        self.archenemy = Some(state);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    fn validate_scheme_deck(
        cards: &[crate::cards::CardDefinition],
        variant: ArchenemyVariant,
    ) -> Result<(), String> {
        let minimum = if variant == ArchenemyVariant::Commander {
            10
        } else {
            20
        };
        if cards.len() < minimum {
            return Err(format!(
                "a scheme deck for this Archenemy variant must contain at least {minimum} cards"
            ));
        }
        let maximum_copies = if variant == ArchenemyVariant::Commander {
            1
        } else {
            2
        };
        let mut names = HashMap::<String, usize>::new();
        for definition in cards {
            if !definition.card.card_types.contains(&CardType::Scheme) {
                return Err(format!("{} is not a Scheme card", definition.name()));
            }
            if !definition.card.subtypes.is_empty() {
                return Err(format!(
                    "Scheme card {} may not have subtypes",
                    definition.name()
                ));
            }
            let count = names
                .entry(definition.name().trim().to_ascii_lowercase())
                .or_default();
            *count += 1;
            if *count > maximum_copies {
                return Err(format!(
                    "this scheme deck may contain no more than {maximum_copies} card(s) named {}",
                    definition.name()
                ));
            }
        }
        Ok(())
    }

    pub fn is_archenemy(&self, player: PlayerId) -> bool {
        self.archenemy
            .as_ref()
            .is_some_and(|state| state.archenemies.contains(&player))
    }

    pub fn scheme_deck(&self, player: PlayerId) -> Option<&[ObjectId]> {
        self.archenemy
            .as_ref()?
            .scheme_decks
            .get(&player)
            .map(Vec::as_slice)
    }

    pub fn face_up_schemes(&self) -> &[ObjectId] {
        self.archenemy
            .as_ref()
            .map(|state| state.face_up.as_slice())
            .unwrap_or(&[])
    }

    pub fn is_face_up_scheme(&self, object: ObjectId) -> bool {
        self.face_up_schemes().contains(&object)
    }

    pub fn is_scheme_card(&self, object: ObjectId) -> bool {
        self.archenemy.as_ref().is_some_and(|state| {
            state.face_up.contains(&object)
                || state
                    .scheme_decks
                    .values()
                    .any(|deck| deck.contains(&object))
        })
    }

    pub fn scheme_is_ongoing(&self, object: ObjectId) -> bool {
        self.object(object)
            .is_some_and(|card| card.supertypes.contains(&crate::types::Supertype::Ongoing))
    }

    /// Set the top card of one archenemy's scheme deck in motion.
    pub fn set_scheme_in_motion(&mut self, player: PlayerId) -> Result<ObjectId, String> {
        if !self.is_archenemy(player) {
            return Err("only an archenemy may set a scheme in motion".to_string());
        }
        let object = self
            .archenemy
            .as_mut()
            .and_then(|state| state.scheme_decks.get_mut(&player))
            .and_then(Vec::pop)
            .ok_or_else(|| "the archenemy's scheme deck is empty".to_string())?;
        if let Some(card) = self.object_mut(object) {
            for ability in card.abilities_mut() {
                ability.functional_zones = vec![Zone::Command];
            }
        }
        self.archenemy
            .as_mut()
            .expect("Archenemy state exists")
            .face_up
            .push(object);
        let provenance = ProvNodeId::default();
        self.queue_trigger_event(
            provenance,
            crate::triggers::TriggerEvent::new(
                crate::events::KeywordActionEvent::new(
                    KeywordActionKind::SetSchemeInMotion,
                    player,
                    object,
                    1,
                ),
                provenance,
            ),
        );
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(object)
    }

    /// Abandon one face-up ongoing scheme and return it face down to its deck.
    pub fn abandon_scheme(&mut self, object: ObjectId) -> Result<ObjectId, String> {
        if !self.is_face_up_scheme(object) || !self.scheme_is_ongoing(object) {
            return Err("only a face-up ongoing scheme may be abandoned".to_string());
        }
        let player = self
            .object(object)
            .map(|card| card.owner)
            .ok_or_else(|| "the face-up scheme is missing".to_string())?;
        let recycled = self.turn_face_up_scheme_down(object)?;
        let provenance = ProvNodeId::default();
        self.queue_trigger_event(
            provenance,
            crate::triggers::TriggerEvent::new(
                crate::events::KeywordActionEvent::new(
                    KeywordActionKind::AbandonScheme,
                    player,
                    object,
                    1,
                ),
                provenance,
            ),
        );
        Ok(recycled)
    }

    pub(crate) fn turn_face_up_scheme_down(
        &mut self,
        old_id: ObjectId,
    ) -> Result<ObjectId, String> {
        let Some(mut object) = self.object(old_id).cloned() else {
            return Err("the face-up scheme is missing".to_string());
        };
        if !self.is_face_up_scheme(old_id) {
            return Err("the scheme is not face up".to_string());
        }
        let owner = object.owner;
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
            .archenemy
            .as_mut()
            .ok_or_else(|| "Archenemy is not enabled".to_string())?;
        state.face_up.retain(|candidate| *candidate != old_id);
        state
            .scheme_decks
            .entry(owner)
            .or_default()
            .insert(0, new_id);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(new_id)
    }

    pub fn synchronize_scheme_ability_zones(&mut self) {
        let Some(state) = self.archenemy.as_ref() else {
            return;
        };
        let face_up = state.face_up.iter().copied().collect::<HashSet<_>>();
        let all = state
            .scheme_decks
            .values()
            .flat_map(|deck| deck.iter().copied())
            .chain(state.face_up.iter().copied())
            .collect::<Vec<_>>();
        for object in all {
            if let Some(card) = self.object_mut(object) {
                let zones = face_up.contains(&object).then_some(Zone::Command);
                for ability in card.abilities_mut() {
                    ability.functional_zones = zones.into_iter().collect();
                }
            }
        }
        self.mark_continuous_state_dirty();
    }

    pub(crate) fn handle_archenemy_player_departure(&mut self, player: PlayerId) {
        let departing_schemes = self
            .archenemy
            .as_ref()
            .map(|state| {
                state
                    .face_up
                    .iter()
                    .copied()
                    .filter(|object| {
                        self.object_store
                            .object(*object)
                            .is_some_and(|card| card.owner == player)
                    })
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        let Some(state) = self.archenemy.as_mut() else {
            return;
        };
        state.archenemies.remove(&player);
        state.scheme_decks.remove(&player);
        state
            .face_up
            .retain(|object| !departing_schemes.contains(object));
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }
}
