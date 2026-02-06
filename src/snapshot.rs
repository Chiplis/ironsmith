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

use std::collections::HashMap;

use crate::ability::{Ability, AbilityKind};
use crate::color::ColorSet;
use crate::ids::CardId;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::mana::ManaCost;
use crate::object::{CounterType, Object, ObjectKind};
use crate::static_abilities::StaticAbilityId;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

/// A comprehensive snapshot of an object's state.
///
/// This unified type replaces the previous separate snapshot types.
///
/// It captures all relevant fields from an Object for LKI purposes.
#[derive(Debug, Clone)]
pub struct ObjectSnapshot {
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
    /// Abilities the object had.
    pub abilities: Vec<Ability>,

    // === Non-copiable state ===
    /// Counters on the object.
    pub counters: HashMap<CounterType, u32>,
    /// Whether this was a token.
    pub is_token: bool,
    /// Whether the object was tapped.
    pub tapped: bool,
    /// Whether the object was flipped.
    pub flipped: bool,
    /// Whether the object was face-down.
    pub face_down: bool,
    /// What the object was attached to (for Auras/Equipment).
    pub attached_to: Option<ObjectId>,
    /// What was attached to the object.
    pub attachments: Vec<ObjectId>,
    /// Whether the object had any Auras attached.
    pub was_enchanted: bool,
    /// Whether the permanent was monstrous.
    pub is_monstrous: bool,
    /// Whether this object is a commander.
    pub is_commander: bool,
    /// The zone the object was in.
    pub zone: Zone,
}

impl ObjectSnapshot {
    /// Create a snapshot from an object with game state access.
    ///
    /// Captures all relevant characteristics at the current moment.
    /// Game state is required to access battlefield state like tapped, flipped, etc.
    pub fn from_object(obj: &Object, game: &crate::game_state::GameState) -> Self {
        Self {
            // Identity
            object_id: obj.id,
            stable_id: obj.stable_id,
            kind: obj.kind,
            card: obj.card,

            // Ownership
            controller: obj.controller,
            owner: obj.owner,

            // Copiable characteristics
            name: obj.name.clone(),
            mana_cost: obj.mana_cost.clone(),
            colors: obj.colors(),
            supertypes: obj.supertypes.clone(),
            card_types: obj.card_types.clone(),
            subtypes: obj.subtypes.clone(),
            power: obj.power(),
            toughness: obj.toughness(),
            base_power: obj.base_power.as_ref().map(|p| p.base_value()),
            base_toughness: obj.base_toughness.as_ref().map(|t| t.base_value()),
            loyalty: obj.loyalty(),
            abilities: obj.abilities.clone(),

            // Non-copiable state (from game state extension maps)
            counters: obj.counters.clone(),
            is_token: obj.kind == ObjectKind::Token,
            tapped: game.is_tapped(obj.id),
            flipped: game.is_flipped(obj.id),
            face_down: game.is_face_down(obj.id),
            attached_to: obj.attached_to,
            attachments: obj.attachments.clone(),
            was_enchanted: false, // Set later via with_enchantment_check if needed
            is_monstrous: game.is_monstrous(obj.id),
            is_commander: game.is_commander_object(obj.id),
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
        let mut snapshot = Self::from_object(obj, game);

        // If the object is on the battlefield, use calculated characteristics
        // which include continuous effects like anthems, pumps, etc.
        if let Some(calculated) = game.calculated_characteristics(obj.id) {
            // Override with calculated values (these include continuous effects)
            snapshot.power = calculated.power;
            snapshot.toughness = calculated.toughness;
            snapshot.card_types = calculated.card_types;
            snapshot.subtypes = calculated.subtypes;
            snapshot.supertypes = calculated.supertypes;
            snapshot.colors = calculated.colors;
            // Note: we keep the original abilities from obj since calculated.abilities
            // may not include all the original ability definitions
        }

        snapshot
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
            kind: ObjectKind::Card,
            card: None,
            controller,
            owner: controller,
            name: name.to_string(),
            mana_cost: None,
            colors: ColorSet::default(),
            supertypes: vec![],
            card_types: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            base_power: None,
            base_toughness: None,
            loyalty: None,
            abilities: vec![],
            counters: HashMap::new(),
            is_token: false,
            tapped: false,
            flipped: false,
            face_down: false,
            attached_to: None,
            attachments: vec![],
            was_enchanted: false,
            is_monstrous: false,
            is_commander: false,
            zone: Zone::Battlefield,
        }
    }

    /// Set the card types for testing.
    #[cfg(test)]
    pub fn with_card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_types = types;
        self
    }

    /// Set power and toughness for testing.
    #[cfg(test)]
    pub fn with_pt(mut self, power: i32, toughness: i32) -> Self {
        self.power = Some(power);
        self.toughness = Some(toughness);
        self.base_power = Some(power);
        self.base_toughness = Some(toughness);
        self
    }

    /// Set subtypes for testing.
    #[cfg(test)]
    pub fn with_subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.subtypes = subtypes;
        self
    }

    /// Set colors for testing.
    #[cfg(test)]
    pub fn with_colors(mut self, colors: ColorSet) -> Self {
        self.colors = colors;
        self
    }

    /// Set counters for testing.
    #[cfg(test)]
    pub fn with_counters(mut self, counters: HashMap<CounterType, u32>) -> Self {
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
        obj.abilities
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
        obj.abilities
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
