//! Explicit creation of real cards from an admitted definition pool.
use crate::tag::TagKeyWalk;
use crate::{ChooseSpec, ObjectFilter, PlayerFilter, Value, Zone};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum CardCreationSelection {
    Named(String),
    /// The singular name selected by an exact local ChooseCardName producer.
    ChosenName {
        tag: crate::tag::TagKey,
    },
    /// One random identity for the batch, or an independent choice per card.
    RandomMatching {
        filter: ObjectFilter,
        independent: bool,
    },
    /// Match an actual supplied identity to the selected object's names.
    Duplicate(ChooseSpec),
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, TagKeyWalk)]
pub enum CardCreationLibraryPosition {
    #[default]
    Top,
    Bottom,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct CreateCardsEffect<D> {
    /// Complete, explicitly supplied real-card definitions; never a token recipe.
    pub definitions: Vec<D>,
    /// Immutable lexical catalog identity. Scoped catalogs contain shallow nodes;
    /// references are exact identities, independent of live object names/copies.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub definition_scope: Option<crate::CardId>,
    /// Explicit catalog subset; empty is an empty named/random pool. Duplicate
    /// may additionally admit its selected real card's exact retained identity
    /// when the current name proves it still denotes that real definition.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub definition_refs: Vec<crate::CardId>,
    /// Optional opaque label of the authored operation enclosing this creation.
    pub operation_label: Option<String>,
    pub selection: CardCreationSelection,
    pub count: Value,
    /// The actor owns the created cards, independently of the effect controller.
    pub player: PlayerFilter,
    pub zone: Zone,
    pub library_position: CardCreationLibraryPosition,
}

/// Lexical immutable definition operands for a reusable executable program.
/// Catalog members remain shallow references; only an owning program carries
/// this finite declaration. Copying the program copies its lexical catalog.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct CardDefinitionScopeEffect<D,E> {
    pub scope: crate::CardId,
    pub definitions: Vec<D>,
    pub effect: Box<E>,
}
