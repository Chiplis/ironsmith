use crate::cards::{CardDefinition, CardRegistry};
use crate::game_state::GameState;
use crate::ids::CardId;

/// Runtime-facing catalog abstraction.
///
/// Adapters can provide concrete card-definition lookup without coupling engine
/// entry points to a specific registry implementation.
pub trait CardCatalog {
    fn card_definition(&self, name: &str) -> Option<&CardDefinition>;
    fn card_definition_by_id(&self, id: CardId) -> Option<&CardDefinition>;
    fn linked_face_definition(
        &self,
        face_name: Option<&str>,
        id: Option<CardId>,
    ) -> Option<&CardDefinition>;
}

impl CardCatalog for CardRegistry {
    fn card_definition(&self, name: &str) -> Option<&CardDefinition> {
        self.get(name)
    }

    fn card_definition_by_id(&self, id: CardId) -> Option<&CardDefinition> {
        self.get_by_id(id)
    }

    fn linked_face_definition(
        &self,
        face_name: Option<&str>,
        id: Option<CardId>,
    ) -> Option<&CardDefinition> {
        self.linked_face_definition_by_name_or_id(face_name, id)
    }
}

/// Explicit runtime entry point that owns gameplay state plus external services.
pub struct GameSession<C> {
    state: GameState,
    catalog: C,
}

impl<C> GameSession<C> {
    pub fn new(state: GameState, catalog: C) -> Self {
        Self { state, catalog }
    }

    pub fn into_parts(self) -> (GameState, C) {
        (self.state, self.catalog)
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut GameState {
        &mut self.state
    }

    pub fn catalog(&self) -> &C {
        &self.catalog
    }

    pub fn catalog_mut(&mut self) -> &mut C {
        &mut self.catalog
    }
}

impl<C: CardCatalog> GameSession<C> {
    pub fn card_definition(&self, name: &str) -> Option<&CardDefinition> {
        self.catalog.card_definition(name)
    }

    pub fn card_definition_by_id(&self, id: CardId) -> Option<&CardDefinition> {
        self.catalog.card_definition_by_id(id)
    }

    pub fn linked_face_definition(
        &self,
        face_name: Option<&str>,
        id: Option<CardId>,
    ) -> Option<&CardDefinition> {
        self.catalog.linked_face_definition(face_name, id)
    }
}
