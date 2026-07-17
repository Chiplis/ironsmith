//! Enter battlefield event implementation.

use std::any::Any;

use crate::ability::Ability;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

/// An enter battlefield event with ETB-specific modifiers.
///
/// This is a specialized zone change event for objects entering the battlefield,
/// allowing replacement effects to modify how the permanent enters (tapped,
/// with counters, etc.).
#[derive(Debug, Clone)]
pub struct EnterBattlefieldEvent {
    /// The object entering
    pub object: ObjectId,
    /// The zone it's coming from
    pub from: Zone,
    /// Whether it enters tapped (may be modified by replacement effects)
    pub enters_tapped: bool,
    /// Counters it enters with (may be modified by replacement effects)
    pub enters_with_counters: Vec<(CounterType, u32)>,
    /// Objects exiled and linked to this permanent as part of an as-enters choice.
    pub linked_exile_with_entering: Vec<ObjectId>,
    /// If set, the object enters as a copy of this source object.
    pub enters_as_copy_of: Option<ObjectId>,
    /// If set, the copied characteristics expire at this duration.
    pub copy_duration: Option<crate::effect::Until>,
    /// If set, overrides the copied object's name as it enters.
    pub copy_name_override: Option<String>,
    /// Additional card types granted by the copy-as-enters replacement.
    pub added_card_types: Vec<CardType>,
    /// Supertypes removed by the copy-as-enters replacement.
    pub removed_supertypes: Vec<Supertype>,
    /// Additional subtypes granted by the copy-as-enters replacement.
    pub added_subtypes: Vec<Subtype>,
    /// Additional abilities granted by the copy-as-enters replacement.
    pub added_abilities: Vec<Ability>,
    /// Base power/toughness set as the object enters.
    pub set_base_power_toughness: Option<(i32, i32)>,
    /// If set, the object enters under this player's control.
    pub controller_override: Option<PlayerId>,
    /// As-entry choices already collected against this provisional object.
    pub(crate) prepared_choices: Option<crate::game_state::PreparedEtbChoices>,
}

impl EnterBattlefieldEvent {
    /// Create a new enter battlefield event.
    pub fn new(object: ObjectId, from: Zone) -> Self {
        Self {
            object,
            from,
            enters_tapped: false,
            enters_with_counters: Vec::new(),
            linked_exile_with_entering: Vec::new(),
            enters_as_copy_of: None,
            copy_duration: None,
            copy_name_override: None,
            added_card_types: Vec::new(),
            removed_supertypes: Vec::new(),
            added_subtypes: Vec::new(),
            added_abilities: Vec::new(),
            set_base_power_toughness: None,
            controller_override: None,
            prepared_choices: None,
        }
    }

    /// Create an event where the permanent enters tapped.
    pub fn tapped(object: ObjectId, from: Zone) -> Self {
        Self {
            object,
            from,
            enters_tapped: true,
            enters_with_counters: Vec::new(),
            linked_exile_with_entering: Vec::new(),
            enters_as_copy_of: None,
            copy_duration: None,
            copy_name_override: None,
            added_card_types: Vec::new(),
            removed_supertypes: Vec::new(),
            added_subtypes: Vec::new(),
            added_abilities: Vec::new(),
            set_base_power_toughness: None,
            controller_override: None,
            prepared_choices: None,
        }
    }

    /// Return a new event with enters_tapped set to true.
    pub fn with_tapped(&self) -> Self {
        Self {
            enters_tapped: true,
            ..self.clone()
        }
    }

    /// Return a new event with additional counters.
    pub fn with_counters(&self, counter_type: CounterType, count: u32) -> Self {
        let mut counters = self.enters_with_counters.clone();

        // Add to existing count if same type, otherwise add new entry
        if let Some((_, existing)) = counters.iter_mut().find(|(ct, _)| *ct == counter_type) {
            *existing = existing.saturating_add(count);
        } else {
            counters.push((counter_type, count));
        }

        Self {
            enters_with_counters: counters,
            ..self.clone()
        }
    }

    pub fn with_linked_exile_objects(&self, object_ids: &[ObjectId]) -> Self {
        let mut linked_exile_with_entering = self.linked_exile_with_entering.clone();
        for object_id in object_ids {
            if !linked_exile_with_entering.contains(object_id) {
                linked_exile_with_entering.push(*object_id);
            }
        }
        Self {
            linked_exile_with_entering,
            ..self.clone()
        }
    }

    /// Return a new event where the object enters as a copy of `source_id`.
    pub fn with_copy_of(&self, source_id: ObjectId) -> Self {
        Self {
            enters_as_copy_of: Some(source_id),
            ..self.clone()
        }
    }

    pub fn with_copy_duration(&self, duration: Option<crate::effect::Until>) -> Self {
        Self {
            copy_duration: duration,
            ..self.clone()
        }
    }

    pub fn with_copy_name_override(&self, name: Option<String>) -> Self {
        Self {
            copy_name_override: name,
            ..self.clone()
        }
    }

    /// Return a new event with additional card types granted as it enters.
    pub fn with_added_card_types(&self, card_types: &[CardType]) -> Self {
        let mut added_card_types = self.added_card_types.clone();
        for card_type in card_types {
            if !added_card_types.contains(card_type) {
                added_card_types.push(*card_type);
            }
        }
        Self {
            added_card_types,
            ..self.clone()
        }
    }

    /// Return a new event with additional subtypes granted as it enters.
    pub fn with_added_subtypes(&self, subtypes: &[Subtype]) -> Self {
        let mut added_subtypes = self.added_subtypes.clone();
        for subtype in subtypes {
            if !added_subtypes.contains(subtype) {
                added_subtypes.push(*subtype);
            }
        }
        Self {
            added_subtypes,
            ..self.clone()
        }
    }

    /// Return a new event with supertypes removed as it enters.
    pub fn with_removed_supertypes(&self, supertypes: &[Supertype]) -> Self {
        let mut removed_supertypes = self.removed_supertypes.clone();
        for supertype in supertypes {
            if !removed_supertypes.contains(supertype) {
                removed_supertypes.push(*supertype);
            }
        }
        Self {
            removed_supertypes,
            ..self.clone()
        }
    }

    /// Return a new event with additional abilities granted as it enters.
    pub fn with_added_abilities(&self, abilities: &[Ability]) -> Self {
        let mut added_abilities = self.added_abilities.clone();
        for ability in abilities {
            if !added_abilities.contains(ability) {
                added_abilities.push(ability.clone());
            }
        }
        Self {
            added_abilities,
            ..self.clone()
        }
    }

    /// Return a new event with base power/toughness set as it enters.
    pub fn with_base_power_toughness(&self, power: i32, toughness: i32) -> Self {
        Self {
            set_base_power_toughness: Some((power, toughness)),
            ..self.clone()
        }
    }

    pub fn with_controller_override(&self, controller: PlayerId) -> Self {
        Self {
            controller_override: Some(controller),
            ..self.clone()
        }
    }

    /// Build the battlefield state used to decide whether another replacement
    /// or "can't" effect applies to this evolving entry proposal.
    ///
    /// CR 614.12 and 614.17d require this view to include higher-priority copy
    /// and control changes, earlier entry modifications, the entrant's own
    /// battlefield static effects, and continuous effects already present.
    /// The returned state is isolated from the live game and never commits the
    /// zone change.
    pub(crate) fn prospective_game_state(&self, game: &GameState) -> Option<GameState> {
        let mut prospective = game.clone();
        let source_object = self
            .enters_as_copy_of
            .and_then(|source| game.object(source).cloned());
        let copiable_values = self.enters_as_copy_of.and_then(|source| {
            let effects = game.all_continuous_effects();
            crate::continuous::copiable_values_with_effects(
                source,
                game.objects_map(),
                &effects,
                &game.battlefield,
                game.commander_objects(),
                game,
            )
        });

        {
            let object = prospective.object_mut(self.object)?;
            object.zone = Zone::Battlefield;
            object.attached_to = None;
            object.attachments.clear();
            object.counters.clear();

            if let Some(source) = source_object.as_ref() {
                object.copy_copiable_values_from(source);
            }
            if let Some(values) = copiable_values {
                object.copy_copiable_values_from_values(&values);
            }
            if let Some(name) = &self.copy_name_override {
                object.name = name.clone().into();
            }
            for card_type in &self.added_card_types {
                if !object.card_types.contains(card_type) {
                    object.card_types.push(*card_type);
                }
            }
            object
                .supertypes
                .retain(|supertype| !self.removed_supertypes.contains(supertype));
            for subtype in &self.added_subtypes {
                if !object.subtypes.contains(subtype) {
                    object.subtypes.push(*subtype);
                }
            }
            for ability in &self.added_abilities {
                if !object.abilities.contains(ability) {
                    object.abilities_mut().push(ability.clone());
                }
            }
            if let Some((power, toughness)) = self.set_base_power_toughness {
                object.base_power = Some(crate::card::PtValue::Fixed(power));
                object.base_toughness = Some(crate::card::PtValue::Fixed(toughness));
            }
            for (counter_type, count) in &self.enters_with_counters {
                if *count > 0 {
                    object.counters.insert(*counter_type, *count);
                }
            }
        }

        if !prospective.battlefield.contains(&self.object) {
            prospective.battlefield.push(self.object);
        }
        if let Some(controller) = self.controller_override {
            prospective.set_current_controller(self.object, controller);
        }
        if let Some(choices) = &self.prepared_choices {
            if let Some(color) = choices.chosen_color {
                prospective.set_chosen_color(self.object, color);
            }
            if let Some(subtype) = choices.chosen_basic_land_type {
                prospective.set_chosen_basic_land_type(self.object, subtype);
            }
            if let Some(subtype) = choices.chosen_land_type {
                prospective.set_chosen_land_type(self.object, subtype);
            }
            if let Some(subtype) = choices.chosen_creature_type {
                prospective.set_chosen_creature_type(self.object, subtype);
            }
            if let Some(card_type) = choices.chosen_card_type {
                prospective.set_chosen_card_type(self.object, card_type);
            }
            if let Some(player) = choices.chosen_player {
                prospective.set_chosen_player(self.object, player);
            }
            if let Some(option) = &choices.chosen_named_option {
                prospective.set_chosen_named_option(self.object, option.clone());
            }
            for (power, toughness, granted_abilities) in &choices.power_toughness_choices {
                if let Some(object) = prospective.object_mut(self.object) {
                    object.base_power = Some(crate::card::PtValue::Fixed(*power));
                    object.base_toughness = Some(crate::card::PtValue::Fixed(*toughness));
                    for granted in granted_abilities {
                        let ability = Ability::static_ability(granted.clone());
                        if !object.abilities.contains(&ability) {
                            object.abilities_mut().push(ability);
                        }
                    }
                }
            }
        }
        Some(prospective)
    }

    /// Get the total count of a specific counter type.
    pub fn counter_count(&self, counter_type: CounterType) -> u32 {
        self.enters_with_counters
            .iter()
            .filter(|(ct, _)| *ct == counter_type)
            .map(|(_, count)| count)
            .sum()
    }
}

impl GameEventType for EnterBattlefieldEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::EnterBattlefield
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.object)
            .map(|o| game.controller_of(o))
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        let mut desc = "Enter the battlefield".to_string();
        if self.enters_tapped {
            desc.push_str(" tapped");
        }
        if !self.enters_with_counters.is_empty() {
            desc.push_str(" with counters");
        }
        if self.enters_as_copy_of.is_some() {
            desc.push_str(" as copy");
        }
        if self.set_base_power_toughness.is_some() {
            desc.push_str(" with base power and toughness");
        }
        desc
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_battlefield_event_creation() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand);

        assert_eq!(event.from, Zone::Hand);
        assert!(!event.enters_tapped);
        assert!(event.enters_with_counters.is_empty());
    }

    #[test]
    fn test_enter_battlefield_tapped() {
        let event = EnterBattlefieldEvent::tapped(ObjectId::from_raw(1), Zone::Hand);
        assert!(event.enters_tapped);
    }

    #[test]
    fn test_enter_battlefield_with_counters() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 3);

        assert_eq!(event.counter_count(CounterType::PlusOnePlusOne), 3);
    }

    #[test]
    fn test_enter_battlefield_with_multiple_counter_types() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 2)
            .with_counters(CounterType::Loyalty, 3);

        assert_eq!(event.counter_count(CounterType::PlusOnePlusOne), 2);
        assert_eq!(event.counter_count(CounterType::Loyalty), 3);
    }

    #[test]
    fn test_enter_battlefield_counter_stacking() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 2)
            .with_counters(CounterType::PlusOnePlusOne, 3);

        assert_eq!(event.counter_count(CounterType::PlusOnePlusOne), 5);
    }

    #[test]
    fn test_enter_battlefield_event_kind() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand);
        assert_eq!(event.event_kind(), EventKind::EnterBattlefield);
    }

    #[test]
    fn test_enter_battlefield_display() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand);
        assert_eq!(event.display(), "Enter the battlefield");

        let tapped_event = event.with_tapped();
        assert_eq!(tapped_event.display(), "Enter the battlefield tapped");

        let with_counters = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 3);
        assert_eq!(
            with_counters.display(),
            "Enter the battlefield with counters"
        );
    }
}
