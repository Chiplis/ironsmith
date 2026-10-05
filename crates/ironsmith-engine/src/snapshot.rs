//! Unified object snapshot system.
//!
//! This module provides a comprehensive snapshot type that captures all relevant
//! object information for "last known information" (LKI) lookups. This is used when:
//!
//! - Triggers need to know what a creature looked like when it died
//! - Effects need to check characteristics of objects that have left the battlefield
//! - Resolution effects need to track target characteristics
//!
//! Per MTG rules:
//! - Rule 400.7h: LKI is used when a triggered ability refers to the characteristics
//!   of the object that triggered it but that object has left its previous zone.
//! - Rule 608.2h: If a spell or ability needs to use information about an object
//!   that has left a zone, it uses the object's last known information.

use std::sync::Arc;

use crate::ability::{Ability, AbilityKind};
use crate::card::LinkedFaceLayout;
use crate::color::ColorSet;
use crate::continuous::{CalculatedCharacteristics, ContinuousEffect};
use crate::ids::CardId;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::mana::ManaCost;
use crate::object::AuraAttachmentFilter;
use crate::object::{AttachmentTarget, CounterType, Object, ObjectKind};
use crate::player::ManaPool;
use crate::static_abilities::StaticAbilityId;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

/// The values a copy effect is allowed to copy after layers 1a and 1b.
///
/// These are stored separately from the effective characteristics in an LKI
/// snapshot because effects from later layers are not copiable (CR 707.2).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct CopiableValues {
    pub name: String,
    pub mana_cost: Option<ManaCost>,
    pub compiled_card_text: String,
    /// The printed line each entry of `abilities` reads as (see `Object::ability_labels`).
    pub ability_labels: Vec<String>,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub supertypes: Vec<Supertype>,
    pub colors: ColorSet,
    pub loyalty: Option<u32>,
    /// Printed defense number, distinct from the source's remaining counters.
    pub defense: Option<u32>,
    /// Public claim snapshots intentionally omit executable abilities.
    /// Registered copy effects must use [`RetainedCopiableValues`] instead.
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub abilities: Arc<Vec<Ability>>,
    pub aura_attach_filter: Option<AuraAttachmentFilter>,
}

/// Lossless copy-effect payload, separate from public claim snapshots.
/// Every executable ability must be translated successfully by the owning codec.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct RetainedCopiableValues<A> {
    pub name: String,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub mana_cost: Option<ManaCost>,
    pub compiled_card_text: String,
    pub ability_labels: Vec<String>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub power: Option<i32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub toughness: Option<i32>,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub supertypes: Vec<Supertype>,
    pub colors: ColorSet,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub loyalty: Option<u32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub defense: Option<u32>,
    pub abilities: Vec<A>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub aura_attach_filter: Option<AuraAttachmentFilter>,
}

/// An optional characteristic must be explicitly present, including as null.
/// A truncated payload cannot silently erase it by using serde's Option default.
#[cfg(feature = "serialization")]
fn deserialize_present_optional<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    <Option<T> as serde::Deserialize<'de>>::deserialize(deserializer)
}

impl<A> RetainedCopiableValues<A> {
    pub fn try_map_abilities<B, Error>(
        self,
        map: impl FnMut(A) -> Result<B, Error>,
    ) -> Result<RetainedCopiableValues<B>, Error> {
        let Self {
            name,
            mana_cost,
            compiled_card_text,
            ability_labels,
            power,
            toughness,
            card_types,
            subtypes,
            supertypes,
            colors,
            loyalty,
            defense,
            abilities,
            aura_attach_filter,
        } = self;
        Ok(RetainedCopiableValues {
            name,
            mana_cost,
            compiled_card_text,
            ability_labels,
            power,
            toughness,
            card_types,
            subtypes,
            supertypes,
            colors,
            loyalty,
            defense,
            abilities: abilities
                .into_iter()
                .map(map)
                .collect::<Result<Vec<_>, _>>()?,
            aura_attach_filter,
        })
    }
}

impl From<CopiableValues> for RetainedCopiableValues<Ability> {
    fn from(values: CopiableValues) -> Self {
        let CopiableValues {
            name,
            mana_cost,
            compiled_card_text,
            ability_labels,
            power,
            toughness,
            card_types,
            subtypes,
            supertypes,
            colors,
            loyalty,
            defense,
            abilities,
            aura_attach_filter,
        } = values;
        Self {
            name,
            mana_cost,
            compiled_card_text,
            ability_labels,
            power,
            toughness,
            card_types,
            subtypes,
            supertypes,
            colors,
            loyalty,
            defense,
            abilities: abilities.iter().cloned().collect(),
            aura_attach_filter,
        }
    }
}

impl From<RetainedCopiableValues<Ability>> for CopiableValues {
    fn from(values: RetainedCopiableValues<Ability>) -> Self {
        let RetainedCopiableValues {
            name,
            mana_cost,
            compiled_card_text,
            ability_labels,
            power,
            toughness,
            card_types,
            subtypes,
            supertypes,
            colors,
            loyalty,
            defense,
            abilities,
            aura_attach_filter,
        } = values;
        Self {
            name,
            mana_cost,
            compiled_card_text,
            ability_labels,
            power,
            toughness,
            card_types,
            subtypes,
            supertypes,
            colors,
            loyalty,
            defense,
            abilities: Arc::new(abilities),
            aura_attach_filter,
        }
    }
}

impl CopiableValues {
    pub fn from_object(obj: &Object) -> Self {
        let bestow_restore = obj.bestow_cast_state.as_ref();
        Self {
            name: obj.name.to_owned_string(),
            mana_cost: obj.mana_cost_owned(),
            compiled_card_text: obj.compiled_card_text.to_string(),
            ability_labels: obj.ability_labels.to_vec(),
            power: obj.base_power.as_ref().map(|power| power.base_value()),
            toughness: obj
                .base_toughness
                .as_ref()
                .map(|toughness| toughness.base_value()),
            card_types: bestow_restore
                .map(|restore| restore.card_types.to_vec())
                .unwrap_or_else(|| obj.card_types.to_vec()),
            subtypes: bestow_restore
                .map(|restore| restore.subtypes.to_vec())
                .unwrap_or_else(|| obj.subtypes.to_vec()),
            supertypes: obj.supertypes.to_vec(),
            colors: obj.colors(),
            loyalty: obj.base_loyalty,
            defense: obj.base_defense,
            abilities: obj.materialized_copiable_abilities(),
            aura_attach_filter: if let Some(restore) = bestow_restore {
                restore
                    .aura_attach_filter
                    .as_ref()
                    .map(|filter| filter.to_owned_value())
            } else {
                obj.aura_attach_filter_owned()
            },
        }
    }

    pub fn from_calculated(chars: &CalculatedCharacteristics) -> Self {
        Self {
            name: chars.name.to_owned_string(),
            mana_cost: chars.mana_cost.clone(),
            compiled_card_text: chars.compiled_card_text.to_string(),
            ability_labels: chars.ability_labels.to_vec(),
            power: chars.power,
            toughness: chars.toughness,
            card_types: chars.card_types.to_vec(),
            subtypes: chars.subtypes.to_vec(),
            supertypes: chars.supertypes.to_vec(),
            colors: chars.colors,
            loyalty: chars.loyalty,
            defense: chars.defense,
            abilities: Arc::new(chars.abilities.to_vec()),
            aura_attach_filter: chars.aura_attach_filter.clone(),
        }
    }
}

/// A comprehensive snapshot of an object's state.
///
/// This unified type replaces the previous separate snapshot types.
///
/// It captures all relevant fields from an Object for LKI purposes.
///
/// With the `serialization` feature the snapshot has a serde encoding used by
/// the hidden-claim ledger of peer matches. The encoding omits the two fields
/// that cannot (or must not) travel: compiled `abilities` and the secretly
/// chosen subtype. It is lossless only for snapshots in public claim form
/// ([`ObjectSnapshot::is_public_claim_form`]); encoders must check that.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ObjectSnapshot {
    /// Noncopiable choices needed by abilities after this exact object leaves.
    pub chosen_subtype: Option<Subtype>,
    pub chosen_object: Option<Box<ObjectSnapshot>>,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub(crate) secret_chosen_subtype: Option<(PlayerId, Subtype)>,
    // === Identity ===
    /// The object's ID at the time of snapshot.
    pub object_id: ObjectId,
    /// The stable instance ID (persists across zone changes).
    pub stable_id: StableId,
    /// The type of game object (card, token, etc.).
    pub kind: ObjectKind,
    /// Reference to the original card definition (None for pure tokens).
    pub card: Option<CardId>,

    // === Ownership ===
    /// The controller at the moment of snapshot.
    pub controller: PlayerId,
    /// The owner.
    pub owner: PlayerId,

    // === Copiable characteristics ===
    /// The object's name.
    pub name: String,
    /// Earliest eligible paper set for the oracle identity represented by the
    /// current name, when registry metadata is available.
    pub first_printed_set_name: Option<String>,
    /// The mana cost (if any).
    pub mana_cost: Option<ManaCost>,
    /// Colors of the object.
    pub colors: ColorSet,
    /// Supertypes (Legendary, Basic, etc.).
    pub supertypes: Vec<Supertype>,
    /// Card types (Creature, Artifact, etc.).
    pub card_types: Vec<CardType>,
    /// Subtypes (Human, Equipment, Forest, etc.).
    pub subtypes: Vec<Subtype>,
    /// Oracle text / rules text.
    pub compiled_card_text: String,
    /// The printed line each entry of `abilities` reads as (see `Object::ability_labels`).
    pub ability_labels: Vec<String>,
    /// Optional reference to another face for flip/DFC style cards.
    pub other_face: Option<CardId>,
    /// Linked face name for on-demand compilation without a global registry preload.
    pub other_face_name: Option<String>,
    /// Layout semantics for linked-face cards.
    pub linked_face_layout: LinkedFaceLayout,
    /// Mana value from the linked face when it differs from `mana_cost`'s
    /// (split cards outside the stack, back-face-up transforming DFCs).
    pub linked_face_mana_value: Option<u32>,
    /// Base power (if creature).
    pub power: Option<i32>,
    /// Base toughness (if creature).
    pub toughness: Option<i32>,
    /// Base power value (for CDA evaluation).
    pub base_power: Option<i32>,
    /// Base toughness value (for CDA evaluation).
    pub base_toughness: Option<i32>,
    /// Loyalty (if planeswalker).
    pub loyalty: Option<u32>,
    /// Defense (if battle).
    pub defense: Option<u32>,
    /// Abilities the object had. Not encoded (see the type docs).
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub abilities: Arc<Vec<Ability>>,
    /// For Auras: what this object can enchant.
    pub aura_attach_filter: Option<AuraAttachmentFilter>,
    /// Frozen layer-1 copiable values at the instant this snapshot was made.
    pub copiable_values: CopiableValues,
    /// For sagas: maximum chapter number.
    /// X value chosen when this object was cast (if any).
    pub x_value: Option<u32>,
    /// 1-based cast index for this spell cast during the current turn, if this snapshot
    /// represents a spell that was cast this turn.
    pub cast_order_this_turn: Option<u32>,
    /// Mana spent to cast this object when it was a spell on the stack.
    pub mana_spent_to_cast: ManaPool,
    /// Actual mana spent by the caster, excluding Assist payments by others.
    #[cfg_attr(feature = "serialization", serde(default))]
    pub caster_mana_spent_to_cast: Option<u32>,
    /// Optional costs paid for this cast, retained for historical spell filters.
    pub optional_costs_paid: crate::cost::OptionalCostsPaid,
    pub snow_mana_spent_to_cast: ManaPool,
    /// Last-known snapshots of the sources that produced mana spent to cast
    /// this object. This remains available after the spell leaves the stack
    /// so first-matching-spell predicates can inspect earlier casts.
    pub mana_sources_spent_to_cast: Vec<ObjectSnapshot>,

    // === Non-copiable state ===
    /// Counters on the object.
    #[cfg_attr(feature = "serialization", serde(with = "counter_pairs"))]
    pub counters: std::collections::BTreeMap<CounterType, u32>,
    /// Whether this was a token.
    pub is_token: bool,
    /// Whether the object was tapped.
    pub tapped: bool,
    /// Whether the object was attacking.
    pub attacking: bool,
    /// Last-known goad designation for a snapshot with calculated state.
    /// Raw snapshots leave this unset rather than recursively calculating layers.
    pub goaded: Option<bool>,
    /// Whether the object was flipped.
    pub flipped: bool,
    /// Whether the object was face-down.
    pub face_down: bool,
    /// How many times the permanent had transformed in its current battlefield life.
    pub transform_count: u64,
    /// What the object was attached to (for Auras/Equipment).
    pub attached_to: Option<AttachmentTarget>,
    /// What was attached to the object.
    pub attachments: Vec<ObjectId>,
    /// One level of attachment characteristics captured for last-known counts.
    /// Nested snapshots do not recursively capture their own attachments.
    pub attachment_snapshots: Vec<ObjectSnapshot>,
    /// Whether the object had any Auras attached.
    pub was_enchanted: bool,
    /// Whether the permanent was monstrous.
    pub is_monstrous: bool,
    /// Whether the permanent was prepared.
    pub is_prepared: bool,
    /// Whether this object is a commander.
    pub is_commander: bool,
    /// The zone the object was in.
    pub zone: Zone,
}

/// Counters encoded as `(kind, count)` pairs: a named counter kind is not a
/// valid JSON map key.
#[cfg(feature = "serialization")]
mod counter_pairs {
    use super::CounterType;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub(super) fn serialize<S: Serializer>(
        counters: &BTreeMap<CounterType, u32>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        counters
            .iter()
            .map(|(kind, count)| (*kind, *count))
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<CounterType, u32>, D::Error> {
        Ok(Vec::<(CounterType, u32)>::deserialize(deserializer)?
            .into_iter()
            .collect())
    }
}

impl ObjectSnapshot {
    /// A snapshot carrying only what every peer knows about an object whose
    /// identity is hidden from some player: its identity, ownership, zone
    /// and public status. Every characteristic is left empty, exactly as a
    /// peer holding a hidden-card placeholder would see it.
    pub fn public_placeholder(
        object_id: ObjectId,
        stable_id: StableId,
        owner: PlayerId,
        controller: PlayerId,
        zone: Zone,
    ) -> Self {
        Self {
            chosen_subtype: None,
            chosen_object: None,
            secret_chosen_subtype: None,
            object_id,
            stable_id,
            kind: ObjectKind::Card,
            card: None,
            controller,
            owner,
            name: String::new(),
            first_printed_set_name: None,
            mana_cost: None,
            colors: ColorSet::default(),
            supertypes: Vec::new(),
            card_types: Vec::new(),
            subtypes: Vec::new(),
            compiled_card_text: String::new(),
            ability_labels: Vec::new(),
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::default(),
            linked_face_mana_value: None,
            power: None,
            toughness: None,
            base_power: None,
            base_toughness: None,
            loyalty: None,
            defense: None,
            abilities: Arc::new(Vec::new()),
            aura_attach_filter: None,
            copiable_values: CopiableValues::default(),
            x_value: None,
            cast_order_this_turn: None,
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            snow_mana_spent_to_cast: ManaPool::default(),
            mana_sources_spent_to_cast: Vec::new(),
            counters: std::collections::BTreeMap::new(),
            is_token: false,
            tapped: false,
            attacking: false,
            goaded: None,
            flipped: false,
            face_down: false,
            transform_count: 0,
            attached_to: None,
            attachments: Vec::new(),
            attachment_snapshots: Vec::new(),
            was_enchanted: false,
            is_monstrous: false,
            is_prepared: false,
            is_commander: false,
            zone,
        }
    }

    /// Whether this snapshot (and every nested snapshot) is in the public
    /// claim form recorded by the hidden-claim ledger: no compiled abilities,
    /// no secretly chosen subtype, and no card-definition ids (`card`,
    /// `other_face`: allocated per engine in load order, so they differ
    /// between peers; names carry the identity instead). Only such snapshots
    /// have a lossless, engine-independent serde encoding.
    pub fn is_public_claim_form(&self) -> bool {
        self.card.is_none()
            && self.other_face.is_none()
            && self.abilities.is_empty()
            && self.copiable_values.abilities.is_empty()
            && self.secret_chosen_subtype.is_none()
            && self
                .chosen_object
                .as_deref()
                .is_none_or(ObjectSnapshot::is_public_claim_form)
            && self
                .mana_sources_spent_to_cast
                .iter()
                .all(ObjectSnapshot::is_public_claim_form)
            && self
                .attachment_snapshots
                .iter()
                .all(ObjectSnapshot::is_public_claim_form)
    }

    /// Drop what the public claim form cannot carry (see
    /// [`ObjectSnapshot::is_public_claim_form`]) from this snapshot itself;
    /// callers canonicalize nested snapshots.
    pub(crate) fn strip_to_public_claim_form(&mut self) {
        self.card = None;
        self.other_face = None;
        self.abilities = Arc::new(Vec::new());
        self.copiable_values.abilities = Arc::new(Vec::new());
        self.secret_chosen_subtype = None;
    }
}

impl ObjectSnapshot {
    /// The other half's name when this was a split card outside the stack
    /// and battlefield, which then also had that name (CR 709.4a).
    pub fn split_other_half_name(&self) -> Option<&str> {
        if self.linked_face_layout == crate::card::LinkedFaceLayout::Split
            && !matches!(self.zone, Zone::Stack | Zone::Battlefield)
        {
            self.other_face_name.as_deref()
        } else {
            None
        }
    }

    /// Create a snapshot from an object with game state access.
    ///
    /// Captures all relevant characteristics at the current moment.
    /// Game state is required to access battlefield state like tapped, flipped, etc.
    pub fn from_object(obj: &Object, game: &crate::game_state::GameState) -> Self {
        let was_enchanted = obj.attachments.iter().any(|&attachment_id| {
            game.object(attachment_id).is_some_and(|attachment| {
                attachment.card_types.contains(&CardType::Enchantment)
                    && attachment.subtypes.contains(&Subtype::Aura)
            })
        });
        Self {
            // Identity
            object_id: obj.id,
            stable_id: obj.stable_id,
            kind: obj.kind,
            card: obj.card,

            // Ownership
            controller: game.controller_of(obj),
            owner: obj.owner,

            // Copiable characteristics
            name: obj.name.to_string(),
            first_printed_set_name: obj
                .first_printed_set_name
                .as_ref()
                .map(|set_name| set_name.to_owned_string()),
            mana_cost: obj.mana_cost_owned(),
            colors: obj.colors(),
            // CR 709.4: a split card outside the stack/battlefield has both
            // halves' types.
            supertypes: obj.zone_supertypes().to_vec(),
            card_types: obj.zone_card_types().to_vec(),
            subtypes: obj.zone_subtypes().to_vec(),
            compiled_card_text: obj.compiled_card_text.to_string(),
            ability_labels: obj.ability_labels.to_vec(),
            other_face: obj.other_face,
            other_face_name: obj
                .other_face_name
                .as_ref()
                .map(|name| name.to_owned_string()),
            linked_face_layout: obj.linked_face_layout,
            linked_face_mana_value: obj.linked_face_mana_value(),
            power: obj.power(),
            toughness: obj.toughness(),
            base_power: obj.base_power.as_ref().map(|p| p.base_value()),
            base_toughness: obj.base_toughness.as_ref().map(|t| t.base_value()),
            loyalty: obj.loyalty(),
            defense: obj.base_defense,
            abilities: obj.abilities.clone(),
            aura_attach_filter: obj.aura_attach_filter_owned(),
            chosen_subtype: game.chosen_subtype(obj.id),
            chosen_object: game.chosen_object(obj.id).cloned().map(Box::new),
            secret_chosen_subtype: game.secret_subtype_snapshot(obj.id),
            copiable_values: CopiableValues::from_object(obj),
            x_value: obj.x_value,
            cast_order_this_turn: game.turn_store.turn_history.spell_cast_order(obj.id),
            mana_spent_to_cast: obj.mana_spent_to_cast.clone(),
            caster_mana_spent_to_cast: obj.caster_mana_spent_to_cast,
            optional_costs_paid: obj.optional_costs_paid.clone(),
            snow_mana_spent_to_cast: obj.snow_mana_spent_to_cast.clone(),
            mana_sources_spent_to_cast: obj
                .cast_tagged_objects
                .get(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG)
                .cloned()
                .unwrap_or_default(),

            // Non-copiable state (from game state extension maps)
            counters: obj.counters.counts().clone(),
            is_token: obj.kind == ObjectKind::Token,
            tapped: game.is_tapped(obj.id),
            attacking: game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_attacking(combat, obj.id)),
            goaded: None,
            flipped: game.is_flipped(obj.id),
            face_down: game.is_face_down(obj.id),
            transform_count: game.transform_count(obj.id),
            attached_to: obj.attached_to,
            attachments: obj.attachments.clone(),
            attachment_snapshots: Vec::new(),
            was_enchanted,
            is_monstrous: game.is_monstrous(obj.id),
            is_prepared: game.is_prepared(obj.id),
            is_commander: game.is_commander(obj.id),
            zone: obj.zone,
        }
    }

    /// Create a snapshot from an object with enchantment check.
    ///
    /// This version checks if any of the object's attachments are Auras.
    pub fn from_object_with_enchantment_check(
        obj: &Object,
        game: &crate::game_state::GameState,
    ) -> Self {
        let mut snapshot = Self::from_object(obj, game);

        // Check if any attachment is an Aura
        snapshot.was_enchanted = obj.attachments.iter().any(|&attachment_id| {
            game.object(attachment_id)
                .map(|att| {
                    att.card_types.contains(&CardType::Enchantment)
                        && att.subtypes.contains(&Subtype::Aura)
                })
                .unwrap_or(false)
        });

        snapshot
    }

    /// Create a snapshot from an object with calculated characteristics.
    ///
    /// Per MTG Rule 400.7h and Rule 704.7, when capturing last known information (LKI),
    /// the snapshot should reflect the object's characteristics including all applicable
    /// continuous effects that were active at the time.
    ///
    /// This method should be used when capturing LKI for:
    /// - Creatures dying as state-based actions (Rule 704.7)
    /// - Objects leaving zones where LKI is needed for triggers
    /// - Any situation where the "true" characteristics matter
    ///
    /// # Arguments
    /// * `obj` - The object to snapshot
    /// * `game` - The game state (needed to compute continuous effects)
    ///
    /// # Returns
    /// A snapshot with power/toughness reflecting all continuous effects, or base+counters
    /// if the object is not on the battlefield or has no calculated characteristics.
    pub fn from_object_with_calculated_characteristics(
        obj: &Object,
        game: &crate::game_state::GameState,
    ) -> Self {
        let all_effects = game.all_continuous_effects();
        Self::from_object_with_calculated_characteristics_and_effects(obj, game, &all_effects)
    }

    /// Create a snapshot from an object with calculated characteristics using precomputed effects.
    pub fn from_object_with_calculated_characteristics_and_effects(
        obj: &Object,
        game: &crate::game_state::GameState,
        effects: &[ContinuousEffect],
    ) -> Self {
        let calculated = game.calculated_characteristics_with_effects(obj.id, effects);
        let copiable_values = crate::continuous::copiable_values_with_effects(
            obj.id,
            game.objects_map(),
            effects,
            &game.battlefield,
            game.commander_objects(),
            game,
        );
        Self::from_object_with_known_characteristics_and_copiable_values(
            obj,
            game,
            calculated.as_ref(),
            copiable_values,
        )
    }

    /// Create a snapshot using characteristics already calculated for the same
    /// game-state instant. Callers that batch characteristic work can use this
    /// to preserve LKI without rerunning the layer system for each object.
    pub fn from_object_with_known_characteristics(
        obj: &Object,
        game: &crate::game_state::GameState,
        calculated: Option<&CalculatedCharacteristics>,
    ) -> Self {
        let effects = game.all_continuous_effects();
        let copiable_values = crate::continuous::copiable_values_with_effects(
            obj.id,
            game.objects_map(),
            &effects,
            &game.battlefield,
            game.commander_objects(),
            game,
        );
        Self::from_object_with_known_characteristics_and_copiable_values(
            obj,
            game,
            calculated,
            copiable_values,
        )
    }

    fn from_object_with_known_characteristics_and_copiable_values(
        obj: &Object,
        game: &crate::game_state::GameState,
        calculated: Option<&CalculatedCharacteristics>,
        copiable_values: Option<CopiableValues>,
    ) -> Self {
        let mut snapshot = Self::from_object(obj, game);
        snapshot.goaded = Some(obj.zone == Zone::Battlefield && game.is_goaded(obj.id));
        if let Some(copiable_values) = copiable_values {
            snapshot.copiable_values = copiable_values;
        }

        snapshot.apply_calculated_characteristics(obj, calculated);
        if !obj.attachments.is_empty() {
            let effects = game.all_continuous_effects();
            snapshot.attachment_snapshots = obj
                .attachments
                .iter()
                .filter_map(|id| game.object(*id))
                .map(|attachment| {
                    let mut child = Self::from_object(attachment, game);
                    let calculated =
                        game.calculated_characteristics_with_effects(attachment.id, &effects);
                    child.apply_calculated_characteristics(attachment, calculated.as_ref());
                    child
                })
                .collect();
        }
        snapshot
    }

    fn apply_calculated_characteristics(
        &mut self,
        obj: &Object,
        calculated: Option<&CalculatedCharacteristics>,
    ) {
        let snapshot = self;
        if let Some(calculated) = calculated {
            if calculated.name.as_str() != obj.name.as_ref() {
                snapshot.first_printed_set_name = None;
            }
            snapshot.name = calculated.name.to_string();
            snapshot.mana_cost = calculated.mana_cost.clone();
            snapshot.linked_face_mana_value = calculated.linked_face_mana_value;
            snapshot.compiled_card_text = calculated.compiled_card_text.to_string();
            snapshot.ability_labels = calculated.ability_labels.to_vec();
            snapshot.power = calculated.power;
            snapshot.toughness = calculated.toughness;
            snapshot.card_types = calculated.card_types.to_vec();
            snapshot.subtypes = calculated.subtypes.to_vec();
            snapshot.supertypes = calculated.supertypes.to_vec();
            snapshot.colors = calculated.colors;
            snapshot.abilities = Arc::new(calculated.abilities.to_vec());
        }
    }

    // === Type checks ===

    /// Check if this object was a creature.
    pub fn is_creature(&self) -> bool {
        self.card_types.contains(&CardType::Creature)
    }

    /// Check if this object was a land.
    pub fn is_land(&self) -> bool {
        self.card_types.contains(&CardType::Land)
    }

    /// Check if this object was an artifact.
    pub fn is_artifact(&self) -> bool {
        self.card_types.contains(&CardType::Artifact)
    }

    /// Check if this object was an enchantment.
    pub fn is_enchantment(&self) -> bool {
        self.card_types.contains(&CardType::Enchantment)
    }

    /// Check if this object was a planeswalker.
    pub fn is_planeswalker(&self) -> bool {
        self.card_types.contains(&CardType::Planeswalker)
    }

    /// Check if this object had a specific card type.
    pub fn has_card_type(&self, card_type: CardType) -> bool {
        self.card_types.contains(&card_type)
    }

    /// Check if this object had a specific subtype.
    pub fn has_subtype(&self, subtype: &Subtype) -> bool {
        self.subtypes.contains(subtype)
    }

    /// Check if this object had a specific supertype.
    pub fn has_supertype(&self, supertype: &Supertype) -> bool {
        self.supertypes.contains(supertype)
    }

    /// Check if this object was legendary.
    pub fn is_legendary(&self) -> bool {
        self.supertypes.contains(&Supertype::Legendary)
    }

    // === Ability checks ===

    /// Check if this object had a static ability with the given ID.
    pub fn has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool {
        self.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == ability_id
            } else {
                false
            }
        })
    }

    /// Check if this object had a triggered ability with a trigger that matches the given display text.
    pub fn has_trigger_display(&self, display_text: &str) -> bool {
        self.abilities.iter().any(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                t.trigger.display() == display_text
            } else {
                false
            }
        })
    }

    /// Check if this object had flying.
    pub fn has_flying(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Flying)
    }

    /// Check if this object had deathtouch.
    pub fn has_deathtouch(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Deathtouch)
    }

    /// Check if this object had lifelink.
    pub fn has_lifelink(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Lifelink)
    }

    /// Check if this object had first strike.
    pub fn has_first_strike(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::FirstStrike)
    }

    /// Check if this object had double strike.
    pub fn has_double_strike(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::DoubleStrike)
    }

    /// Check if this object had trample.
    pub fn has_trample(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Trample)
    }

    /// Check if this object had vigilance.
    pub fn has_vigilance(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Vigilance)
    }

    /// Check if this object had haste.
    pub fn has_haste(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Haste)
    }

    /// Check if this object had indestructible.
    pub fn has_indestructible(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Indestructible)
    }

    /// Check if this object had hexproof.
    pub fn has_hexproof(&self) -> bool {
        self.has_static_ability_id(StaticAbilityId::Hexproof)
    }

    // === Counter checks ===

    /// Get the number of +1/+1 counters.
    pub fn plus_one_counters(&self) -> u32 {
        self.counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0)
    }

    /// Get the number of -1/-1 counters.
    pub fn minus_one_counters(&self) -> u32 {
        self.counters
            .get(&CounterType::MinusOneMinusOne)
            .copied()
            .unwrap_or(0)
    }

    /// Get the count of a specific counter type.
    pub fn counter_count(&self, counter_type: CounterType) -> u32 {
        self.counters.get(&counter_type).copied().unwrap_or(0)
    }

    // === Special checks ===

    /// Check if this creature had Undying and qualifies for return.
    /// Undying triggers when the creature dies without +1/+1 counters.
    pub fn qualifies_for_undying(&self) -> bool {
        self.is_creature() && self.has_trigger_display("Undying") && self.plus_one_counters() == 0
    }

    /// Check if this creature had Persist and qualifies for return.
    /// Persist triggers when the creature dies without -1/-1 counters.
    pub fn qualifies_for_persist(&self) -> bool {
        self.is_creature() && self.has_trigger_display("Persist") && self.minus_one_counters() == 0
    }

    /// Get the mana value (converted mana cost) of this object.
    pub fn mana_value(&self) -> u32 {
        if let Some(mana_value) = self.linked_face_mana_value {
            return mana_value;
        }
        self.mana_cost
            .as_ref()
            .map(|mc| mc.mana_value())
            .unwrap_or(0)
    }

    /// Create a minimal snapshot for testing purposes.
    ///
    /// This creates a snapshot with sensible defaults that can be customized
    /// via the builder pattern methods.
    #[cfg(test)]
    pub fn for_testing(object_id: ObjectId, controller: PlayerId, name: &str) -> Self {
        Self {
            object_id,
            stable_id: object_id.into(),
            chosen_subtype: None,
            secret_chosen_subtype: None,
            chosen_object: None,
            kind: ObjectKind::Card,
            card: None,
            controller,
            owner: controller,
            name: name.to_string(),
            first_printed_set_name: None,
            mana_cost: None,
            colors: ColorSet::default(),
            supertypes: vec![],
            card_types: vec![],
            subtypes: vec![],
            compiled_card_text: String::new(),
            ability_labels: Vec::new(),
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            linked_face_mana_value: None,
            power: None,
            toughness: None,
            base_power: None,
            base_toughness: None,
            loyalty: None,
            defense: None,
            abilities: Arc::new(vec![]),
            aura_attach_filter: None,
            copiable_values: CopiableValues {
                name: name.to_string(),
                ..CopiableValues::default()
            },
            x_value: None,
            cast_order_this_turn: None,
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            snow_mana_spent_to_cast: ManaPool::default(),
            mana_sources_spent_to_cast: Vec::new(),
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            counters: std::collections::BTreeMap::new(),
            is_token: false,
            tapped: false,
            attacking: false,
            goaded: Some(false),
            flipped: false,
            face_down: false,
            transform_count: 0,
            attached_to: None,
            attachments: vec![],
            attachment_snapshots: Vec::new(),
            was_enchanted: false,
            is_monstrous: false,
            is_prepared: false,
            is_commander: false,
            zone: Zone::Battlefield,
        }
    }

    /// Set the card types for testing.
    #[cfg(test)]
    pub fn with_card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_types = types.clone();
        self.copiable_values.card_types = types;
        self
    }

    /// Set power and toughness for testing.
    #[cfg(test)]
    pub fn with_pt(mut self, power: i32, toughness: i32) -> Self {
        self.power = Some(power);
        self.toughness = Some(toughness);
        self.base_power = Some(power);
        self.base_toughness = Some(toughness);
        self.copiable_values.power = Some(power);
        self.copiable_values.toughness = Some(toughness);
        self
    }

    /// Set subtypes for testing.
    #[cfg(test)]
    pub fn with_subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.subtypes = subtypes.clone();
        self.copiable_values.subtypes = subtypes;
        self
    }

    /// Set colors for testing.
    #[cfg(test)]
    pub fn with_colors(mut self, colors: ColorSet) -> Self {
        self.colors = colors;
        self.copiable_values.colors = colors;
        self
    }

    /// Set counters for testing.
    #[cfg(test)]
    pub fn with_counters(mut self, counters: std::collections::BTreeMap<CounterType, u32>) -> Self {
        self.counters = counters;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::mana::ManaSymbol;
    use crate::triggers::Trigger;

    fn grizzly_bears_object() -> Object {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        )
    }

    fn test_game_state() -> GameState {
        GameState::new(vec!["Alice".to_string()], 20)
    }

    #[test]
    fn test_snapshot_captures_basic_info() {
        let obj = grizzly_bears_object();
        let game = test_game_state();
        let snapshot = ObjectSnapshot::from_object(&obj, &game);

        assert_eq!(snapshot.name, "Grizzly Bears");
        assert_eq!(snapshot.object_id, obj.id);
        assert_eq!(snapshot.controller, PlayerId::from_index(0));
        assert_eq!(snapshot.power, Some(2));
        assert_eq!(snapshot.toughness, Some(2));
    }

    #[test]
    fn test_snapshot_type_checks() {
        let obj = grizzly_bears_object();
        let game = test_game_state();
        let snapshot = ObjectSnapshot::from_object(&obj, &game);

        assert!(snapshot.is_creature());
        assert!(!snapshot.is_land());
        assert!(!snapshot.is_artifact());
        assert!(snapshot.has_card_type(CardType::Creature));
        assert!(snapshot.has_subtype(&Subtype::Bear));
    }

    #[test]
    fn test_snapshot_captures_counters() {
        let mut obj = grizzly_bears_object();
        obj.add_counters(CounterType::PlusOnePlusOne, 3);
        obj.add_counters(CounterType::MinusOneMinusOne, 1);

        let game = test_game_state();
        let snapshot = ObjectSnapshot::from_object(&obj, &game);

        assert_eq!(snapshot.plus_one_counters(), 3);
        assert_eq!(snapshot.minus_one_counters(), 1);
        assert_eq!(snapshot.counter_count(CounterType::PlusOnePlusOne), 3);
        // Power should include counter modifications
        assert_eq!(snapshot.power, Some(4)); // 2 + 3 - 1
    }

    #[test]
    fn test_snapshot_captures_state() {
        let obj = grizzly_bears_object();
        let mut game = test_game_state();
        // Set state via GameState extension maps
        game.tap(obj.id);
        game.set_monstrous(obj.id);

        let snapshot = ObjectSnapshot::from_object(&obj, &game);

        assert!(snapshot.tapped);
        assert!(snapshot.is_monstrous);
    }

    #[test]
    fn test_undying_qualification() {
        use crate::ability::Ability;

        let mut obj = grizzly_bears_object();

        // Add undying trigger (now using Trigger struct)
        obj.abilities_mut()
            .push(Ability::triggered(Trigger::undying(), vec![]));

        let game = test_game_state();
        let snapshot = ObjectSnapshot::from_object(&obj, &game);
        assert!(snapshot.qualifies_for_undying());

        // Now add +1/+1 counter - should no longer qualify
        obj.add_counters(CounterType::PlusOnePlusOne, 1);
        let snapshot2 = ObjectSnapshot::from_object(&obj, &game);
        assert!(!snapshot2.qualifies_for_undying());
    }

    #[test]
    fn test_persist_qualification() {
        use crate::ability::Ability;

        let mut obj = grizzly_bears_object();

        // Add persist trigger (now using Trigger struct)
        obj.abilities_mut()
            .push(Ability::triggered(Trigger::persist(), vec![]));

        let game = test_game_state();
        let snapshot = ObjectSnapshot::from_object(&obj, &game);
        assert!(snapshot.qualifies_for_persist());

        // Now add -1/-1 counter - should no longer qualify
        obj.add_counters(CounterType::MinusOneMinusOne, 1);
        let snapshot2 = ObjectSnapshot::from_object(&obj, &game);
        assert!(!snapshot2.qualifies_for_persist());
    }

    #[test]
    fn test_mana_value() {
        let obj = grizzly_bears_object();
        let game = test_game_state();
        let snapshot = ObjectSnapshot::from_object(&obj, &game);

        // Grizzly Bears costs {1}{G} = mana value 2
        assert_eq!(snapshot.mana_value(), 2);
    }
}

/// Complete historical rules state for checkpoints, distinct from public claims.
/// The owning exporter controls perspective redaction. Every executable ability,
/// secret choice, nested history and card-definition reference is retained.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    feature = "serialization",
    serde(bound(deserialize = "A: serde::Deserialize<'de>, I: serde::Deserialize<'de>"))
)]
pub struct RetainedObjectSnapshot<A, I = CardId> {
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub chosen_subtype: Option<Subtype>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub chosen_object: Option<Box<RetainedObjectSnapshot<A, I>>>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub secret_chosen_subtype: Option<(PlayerId, Subtype)>,
    pub object_id: ObjectId,
    pub stable_id: StableId,
    pub kind: ObjectKind,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub card: Option<I>,
    pub controller: PlayerId,
    pub owner: PlayerId,
    pub name: String,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub first_printed_set_name: Option<String>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub mana_cost: Option<ManaCost>,
    pub colors: ColorSet,
    pub supertypes: Vec<Supertype>,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub compiled_card_text: String,
    pub ability_labels: Vec<String>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub other_face: Option<I>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub other_face_name: Option<String>,
    pub linked_face_layout: LinkedFaceLayout,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub linked_face_mana_value: Option<u32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub power: Option<i32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub toughness: Option<i32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub base_power: Option<i32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub base_toughness: Option<i32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub loyalty: Option<u32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub defense: Option<u32>,
    pub abilities: Vec<A>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub aura_attach_filter: Option<AuraAttachmentFilter>,
    pub copiable_values: RetainedCopiableValues<A>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub x_value: Option<u32>,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub cast_order_this_turn: Option<u32>,
    pub mana_spent_to_cast: ManaPool,
    /// Actual mana spent by the caster, excluding Assist payments by others.
    /// Missing in older retained snapshots means unknown payer evidence.
    #[cfg_attr(feature = "serialization", serde(default))]
    pub caster_mana_spent_to_cast: Option<u32>,
    pub optional_costs_paid: crate::cost::OptionalCostsPaid,
    pub snow_mana_spent_to_cast: ManaPool,
    pub mana_sources_spent_to_cast: Vec<RetainedObjectSnapshot<A, I>>,
    #[cfg_attr(
        feature = "serialization",
        serde(
            serialize_with = "counter_pairs::serialize",
            deserialize_with = "deserialize_unique_snapshot_counters"
        )
    )]
    pub counters: std::collections::BTreeMap<CounterType, u32>,
    pub is_token: bool,
    pub tapped: bool,
    pub attacking: bool,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub goaded: Option<bool>,
    pub flipped: bool,
    pub face_down: bool,
    pub transform_count: u64,
    #[cfg_attr(
        feature = "serialization",
        serde(deserialize_with = "deserialize_present_optional")
    )]
    pub attached_to: Option<AttachmentTarget>,
    pub attachments: Vec<ObjectId>,
    pub attachment_snapshots: Vec<RetainedObjectSnapshot<A, I>>,
    pub was_enchanted: bool,
    pub is_monstrous: bool,
    pub is_prepared: bool,
    pub is_commander: bool,
    pub zone: Zone,
}
impl From<ObjectSnapshot> for RetainedObjectSnapshot<Ability> {
    fn from(value: ObjectSnapshot) -> Self {
        let ObjectSnapshot {
            chosen_subtype,
            chosen_object,
            secret_chosen_subtype,
            object_id,
            stable_id,
            kind,
            card,
            controller,
            owner,
            name,
            first_printed_set_name,
            mana_cost,
            colors,
            supertypes,
            card_types,
            subtypes,
            compiled_card_text,
            ability_labels,
            other_face,
            other_face_name,
            linked_face_layout,
            linked_face_mana_value,
            power,
            toughness,
            base_power,
            base_toughness,
            loyalty,
            defense,
            abilities,
            aura_attach_filter,
            copiable_values,
            x_value,
            cast_order_this_turn,
            mana_spent_to_cast,
            caster_mana_spent_to_cast,
            optional_costs_paid,
            snow_mana_spent_to_cast,
            mana_sources_spent_to_cast,
            counters,
            is_token,
            tapped,
            attacking,
            goaded,
            flipped,
            face_down,
            transform_count,
            attached_to,
            attachments,
            attachment_snapshots,
            was_enchanted,
            is_monstrous,
            is_prepared,
            is_commander,
            zone,
        } = value;
        Self {
            chosen_subtype: chosen_subtype,
            chosen_object: chosen_object.map(|value| Box::new((*value).into())),
            secret_chosen_subtype: secret_chosen_subtype,
            object_id: object_id,
            stable_id: stable_id,
            kind: kind,
            card: card,
            controller: controller,
            owner: owner,
            name: name,
            first_printed_set_name: first_printed_set_name,
            mana_cost: mana_cost,
            colors: colors,
            supertypes: supertypes,
            card_types: card_types,
            subtypes: subtypes,
            compiled_card_text: compiled_card_text,
            ability_labels: ability_labels,
            other_face: other_face,
            other_face_name: other_face_name,
            linked_face_layout: linked_face_layout,
            linked_face_mana_value: linked_face_mana_value,
            power: power,
            toughness: toughness,
            base_power: base_power,
            base_toughness: base_toughness,
            loyalty: loyalty,
            defense: defense,
            abilities: abilities.as_ref().clone(),
            aura_attach_filter: aura_attach_filter,
            copiable_values: copiable_values.into(),
            x_value: x_value,
            cast_order_this_turn: cast_order_this_turn,
            mana_spent_to_cast: mana_spent_to_cast,
            caster_mana_spent_to_cast: caster_mana_spent_to_cast,
            optional_costs_paid: optional_costs_paid,
            snow_mana_spent_to_cast: snow_mana_spent_to_cast,
            mana_sources_spent_to_cast: mana_sources_spent_to_cast
                .into_iter()
                .map(Into::into)
                .collect(),
            counters: counters,
            is_token: is_token,
            tapped: tapped,
            attacking: attacking,
            goaded: goaded,
            flipped: flipped,
            face_down: face_down,
            transform_count: transform_count,
            attached_to: attached_to,
            attachments: attachments,
            attachment_snapshots: attachment_snapshots.into_iter().map(Into::into).collect(),
            was_enchanted: was_enchanted,
            is_monstrous: is_monstrous,
            is_prepared: is_prepared,
            is_commander: is_commander,
            zone: zone,
        }
    }
}
impl From<RetainedObjectSnapshot<Ability>> for ObjectSnapshot {
    fn from(value: RetainedObjectSnapshot<Ability>) -> Self {
        let RetainedObjectSnapshot {
            chosen_subtype,
            chosen_object,
            secret_chosen_subtype,
            object_id,
            stable_id,
            kind,
            card,
            controller,
            owner,
            name,
            first_printed_set_name,
            mana_cost,
            colors,
            supertypes,
            card_types,
            subtypes,
            compiled_card_text,
            ability_labels,
            other_face,
            other_face_name,
            linked_face_layout,
            linked_face_mana_value,
            power,
            toughness,
            base_power,
            base_toughness,
            loyalty,
            defense,
            abilities,
            aura_attach_filter,
            copiable_values,
            x_value,
            cast_order_this_turn,
            mana_spent_to_cast,
            caster_mana_spent_to_cast,
            optional_costs_paid,
            snow_mana_spent_to_cast,
            mana_sources_spent_to_cast,
            counters,
            is_token,
            tapped,
            attacking,
            goaded,
            flipped,
            face_down,
            transform_count,
            attached_to,
            attachments,
            attachment_snapshots,
            was_enchanted,
            is_monstrous,
            is_prepared,
            is_commander,
            zone,
        } = value;
        Self {
            chosen_subtype: chosen_subtype,
            chosen_object: chosen_object.map(|value| Box::new((*value).into())),
            secret_chosen_subtype: secret_chosen_subtype,
            object_id: object_id,
            stable_id: stable_id,
            kind: kind,
            card: card,
            controller: controller,
            owner: owner,
            name: name,
            first_printed_set_name: first_printed_set_name,
            mana_cost: mana_cost,
            colors: colors,
            supertypes: supertypes,
            card_types: card_types,
            subtypes: subtypes,
            compiled_card_text: compiled_card_text,
            ability_labels: ability_labels,
            other_face: other_face,
            other_face_name: other_face_name,
            linked_face_layout: linked_face_layout,
            linked_face_mana_value: linked_face_mana_value,
            power: power,
            toughness: toughness,
            base_power: base_power,
            base_toughness: base_toughness,
            loyalty: loyalty,
            defense: defense,
            abilities: abilities.into(),
            aura_attach_filter: aura_attach_filter,
            copiable_values: copiable_values.into(),
            x_value: x_value,
            cast_order_this_turn: cast_order_this_turn,
            mana_spent_to_cast: mana_spent_to_cast,
            caster_mana_spent_to_cast: caster_mana_spent_to_cast,
            optional_costs_paid: optional_costs_paid,
            snow_mana_spent_to_cast: snow_mana_spent_to_cast,
            mana_sources_spent_to_cast: mana_sources_spent_to_cast
                .into_iter()
                .map(Into::into)
                .collect(),
            counters: counters,
            is_token: is_token,
            tapped: tapped,
            attacking: attacking,
            goaded: goaded,
            flipped: flipped,
            face_down: face_down,
            transform_count: transform_count,
            attached_to: attached_to,
            attachments: attachments,
            attachment_snapshots: attachment_snapshots.into_iter().map(Into::into).collect(),
            was_enchanted: was_enchanted,
            is_monstrous: is_monstrous,
            is_prepared: is_prepared,
            is_commander: is_commander,
            zone: zone,
        }
    }
}

impl<A, I> RetainedObjectSnapshot<A, I> {
    pub fn try_map_payloads<B, J, Error>(
        self,
        mut ability: impl FnMut(A) -> Result<B, Error>,
        mut card: impl FnMut(I) -> Result<J, Error>,
    ) -> Result<RetainedObjectSnapshot<B, J>, Error> {
        self.try_map_with(&mut ability, &mut card)
    }
    fn try_map_with<B, J, Error>(
        self,
        ability: &mut impl FnMut(A) -> Result<B, Error>,
        card: &mut impl FnMut(I) -> Result<J, Error>,
    ) -> Result<RetainedObjectSnapshot<B, J>, Error> {
        Ok(RetainedObjectSnapshot {
            chosen_subtype: self.chosen_subtype,
            chosen_object: self
                .chosen_object
                .map(|value| value.try_map_with(ability, card).map(Box::new))
                .transpose()?,
            secret_chosen_subtype: self.secret_chosen_subtype,
            object_id: self.object_id,
            stable_id: self.stable_id,
            kind: self.kind,
            card: self.card.map(&mut *card).transpose()?,
            controller: self.controller,
            owner: self.owner,
            name: self.name,
            first_printed_set_name: self.first_printed_set_name,
            mana_cost: self.mana_cost,
            colors: self.colors,
            supertypes: self.supertypes,
            card_types: self.card_types,
            subtypes: self.subtypes,
            compiled_card_text: self.compiled_card_text,
            ability_labels: self.ability_labels,
            other_face: self.other_face.map(&mut *card).transpose()?,
            other_face_name: self.other_face_name,
            linked_face_layout: self.linked_face_layout,
            linked_face_mana_value: self.linked_face_mana_value,
            power: self.power,
            toughness: self.toughness,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
            loyalty: self.loyalty,
            defense: self.defense,
            abilities: self
                .abilities
                .into_iter()
                .map(&mut *ability)
                .collect::<Result<Vec<_>, _>>()?,
            aura_attach_filter: self.aura_attach_filter,
            copiable_values: self.copiable_values.try_map_abilities(&mut *ability)?,
            x_value: self.x_value,
            cast_order_this_turn: self.cast_order_this_turn,
            mana_spent_to_cast: self.mana_spent_to_cast,
            caster_mana_spent_to_cast: self.caster_mana_spent_to_cast,
            optional_costs_paid: self.optional_costs_paid,
            snow_mana_spent_to_cast: self.snow_mana_spent_to_cast,
            mana_sources_spent_to_cast: self
                .mana_sources_spent_to_cast
                .into_iter()
                .map(|value| value.try_map_with(ability, card))
                .collect::<Result<Vec<_>, _>>()?,
            counters: self.counters,
            is_token: self.is_token,
            tapped: self.tapped,
            attacking: self.attacking,
            goaded: self.goaded,
            flipped: self.flipped,
            face_down: self.face_down,
            transform_count: self.transform_count,
            attached_to: self.attached_to,
            attachments: self.attachments,
            attachment_snapshots: self
                .attachment_snapshots
                .into_iter()
                .map(|value| value.try_map_with(ability, card))
                .collect::<Result<Vec<_>, _>>()?,
            was_enchanted: self.was_enchanted,
            is_monstrous: self.is_monstrous,
            is_prepared: self.is_prepared,
            is_commander: self.is_commander,
            zone: self.zone,
        })
    }
}
#[cfg(feature = "serialization")]
fn deserialize_unique_snapshot_counters<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<CounterType, u32>, D::Error> {
    let pairs: Vec<(CounterType, u32)> = serde::Deserialize::deserialize(deserializer)?;
    let mut counters = std::collections::BTreeMap::new();
    for (kind, count) in pairs {
        if counters.insert(kind, count).is_some() {
            return Err(serde::de::Error::custom(
                "duplicate retained snapshot counter",
            ));
        }
    }
    Ok(counters)
}

#[cfg(test)]
mod retained_historical_snapshot_schema_tests {
    use super::*;
    fn fixture() -> ObjectSnapshot {
        let mut snapshot = ObjectSnapshot::public_placeholder(
            ObjectId::from_raw(10),
            StableId::from_raw(11),
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            Zone::Battlefield,
        );
        snapshot.card = Some(CardId::new());
        snapshot.caster_mana_spent_to_cast = Some(2);
        snapshot.other_face = Some(CardId::new());
        snapshot.secret_chosen_subtype = Some((PlayerId::from_index(1), Subtype::Elf));
        snapshot.chosen_subtype = Some(Subtype::Human);
        snapshot.counters.insert(CounterType::PlusOnePlusOne, 2);
        let ability = Ability::static_ability(crate::static_abilities::StaticAbility::flying());
        snapshot.abilities = vec![ability.clone()].into();
        snapshot.copiable_values.abilities = vec![ability].into();
        snapshot.chosen_object = Some(Box::new(snapshot.clone()));
        snapshot
            .mana_sources_spent_to_cast
            .push(snapshot.chosen_object.as_ref().unwrap().as_ref().clone());
        snapshot
            .attachment_snapshots
            .push(snapshot.chosen_object.as_ref().unwrap().as_ref().clone());
        snapshot
    }
    #[test]
    fn retained_historical_snapshot_preserves_native_fields_secrets_nested_state_and_bindings() {
        let original = fixture();
        let retained = RetainedObjectSnapshot::from(original.clone());
        let restored: ObjectSnapshot = retained
            .try_map_payloads(Ok::<_, &'static str>, Ok::<_, &'static str>)
            .unwrap()
            .into();
        assert_eq!(restored, original);
        for child in [
            restored.chosen_object.as_ref().unwrap().as_ref(),
            &restored.mana_sources_spent_to_cast[0],
            &restored.attachment_snapshots[0],
        ] {
            assert_eq!(child.secret_chosen_subtype, original.secret_chosen_subtype);
            let AbilityKind::Static(ability) = &child.abilities[0].kind else {
                panic!("static occurrence")
            };
            let AbilityKind::Static(copiable) = &child.copiable_values.abilities[0].kind else {
                panic!("copiable occurrence")
            };
            assert_eq!(ability.instance_id(), copiable.instance_id());
        }
        assert!(
            RetainedObjectSnapshot::from(original.clone())
                .try_map_payloads(|_| Err::<(), _>("ability"), Ok::<_, &'static str>)
                .is_err()
        );
        assert!(
            RetainedObjectSnapshot::from(original)
                .try_map_payloads(Ok::<_, &'static str>, |_| Err::<(), _>("card graph"))
                .is_err()
        );
    }
    #[cfg(feature = "serialization")]
    #[test]
    fn retained_historical_snapshot_requires_original_fields_and_rejects_duplicate_counters() {
        type Wire = RetainedObjectSnapshot<u8, u8>;
        let retained: Wire = RetainedObjectSnapshot::from(fixture())
            .try_map_payloads(|_| Ok::<_, ()>(1), |_| Ok::<_, ()>(2))
            .unwrap();
        let json = serde_json::to_value(retained).unwrap();
        let _: Wire = serde_json::from_value(json.clone()).unwrap();
        for field in json.as_object().unwrap().keys() {
            if field == "caster_mana_spent_to_cast" { continue; }
            let mut bad = json.clone();
            bad.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<Wire>(bad).is_err(),
                "missing {field}"
            );
        }
        let mut legacy = json.clone();
        legacy.as_object_mut().unwrap().remove("caster_mana_spent_to_cast");
        let restored: Wire = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.caster_mana_spent_to_cast, None);
        assert_eq!(serde_json::to_value(&restored.mana_spent_to_cast).unwrap(), json["mana_spent_to_cast"]);
        let mut explicit_unknown = json.clone();
        explicit_unknown["caster_mana_spent_to_cast"] = serde_json::Value::Null;
        let restored: Wire = serde_json::from_value(explicit_unknown.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), explicit_unknown);
        let mut bad = json.clone();
        bad["chosen_object"]
            .as_object_mut()
            .unwrap()
            .remove("abilities");
        assert!(serde_json::from_value::<Wire>(bad).is_err());
        let mut bad = json;
        let counter = bad["counters"][0].clone();
        bad["counters"].as_array_mut().unwrap().push(counter);
        assert!(serde_json::from_value::<Wire>(bad).is_err());
    }
}
