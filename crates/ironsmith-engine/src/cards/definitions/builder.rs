use crate::card::{LinkedFaceLayout, PowerToughness};
use crate::cards::{
    CardDefinition,
    builders::{CardDefinitionBuilder as RawCardDefinitionBuilder, CardTextError},
};
use crate::ids::CardId;
use crate::mana::ManaCost;
use crate::types::{CardType, Subtype, Supertype};
use crate::{ability::Ability, effect::Effect};

/// Restricted builder surface for hand-written runtime card definitions.
///
/// New definitions use explicit typed runtime abilities and effects. The parser
/// entrypoint remains only for legacy definitions pending migration to registry
/// compilation; runtime fixtures must not depend on it.
#[derive(Debug, Clone)]
pub(crate) struct CardDefinitionBuilder(RawCardDefinitionBuilder);

impl CardDefinitionBuilder {
    pub(crate) fn new(id: CardId, name: impl Into<String>) -> Self {
        Self(RawCardDefinitionBuilder::new(id, name))
    }

    pub(crate) fn mana_cost(self, cost: ManaCost) -> Self {
        Self(self.0.mana_cost(cost))
    }

    pub(crate) fn supertypes(self, supertypes: Vec<Supertype>) -> Self {
        Self(self.0.supertypes(supertypes))
    }

    pub(crate) fn card_types(self, types: Vec<CardType>) -> Self {
        Self(self.0.card_types(types))
    }

    pub(crate) fn subtypes(self, subtypes: Vec<Subtype>) -> Self {
        Self(self.0.subtypes(subtypes))
    }

    pub(crate) fn other_face(self, face: CardId) -> Self {
        Self(self.0.other_face(face))
    }

    pub(crate) fn other_face_name(self, name: impl Into<String>) -> Self {
        Self(self.0.other_face_name(name))
    }

    pub(crate) fn linked_face_layout(self, layout: LinkedFaceLayout) -> Self {
        Self(self.0.linked_face_layout(layout))
    }

    pub(crate) fn has_fuse(self) -> Self {
        Self(self.0.has_fuse())
    }

    pub(crate) fn power_toughness(self, pt: PowerToughness) -> Self {
        Self(self.0.power_toughness(pt))
    }

    pub(crate) fn oracle_text(self, text: impl Into<String>) -> Self {
        Self(self.0.oracle_text(text))
    }

    pub(crate) fn with_ability(self, ability: Ability) -> Self {
        Self(self.0.with_ability(ability))
    }

    pub(crate) fn with_abilities(self, abilities: Vec<Ability>) -> Self {
        Self(self.0.with_abilities(abilities))
    }

    pub(crate) fn with_spell_effect(self, effects: Vec<Effect>) -> Self {
        Self(self.0.with_spell_effect(effects))
    }

    pub(crate) fn build(self) -> CardDefinition {
        self.0.build()
    }

    #[cfg(test)]
    pub(crate) fn token(self) -> Self {
        Self(self.0.token())
    }

    pub(crate) fn parse_text(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        self.0.parse_text(text)
    }
}
