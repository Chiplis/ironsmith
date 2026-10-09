//! Enter battlefield event implementation.

use std::any::Any;

use crate::ability::Ability;
use crate::color::ColorSet;
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
    /// Frozen only when the committed notification is queued. Replacement
    /// proposals keep this absent and continue using prospective state.
    pub completed_snapshot: Option<crate::snapshot::ObjectSnapshot>,
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
    /// Consequences of the copy choice, applied as the copy enters.
    pub copy_followups: Vec<ironsmith_core::EnterAsCopyFollowup>,
    /// If set, the copied characteristics expire at this duration.
    pub copy_duration: Option<crate::effect::Until>,
    /// If set, overrides the copied object's name as it enters.
    pub copy_name_override: Option<String>,
    /// Additional colors granted by the copy-as-enters replacement.
    pub added_colors: ColorSet,
    /// Additional card types granted by the copy-as-enters replacement.
    pub added_card_types: Vec<CardType>,
    /// The copy's card types are exactly `added_card_types` (Imposter Mech).
    pub removes_other_card_types: bool,
    /// Supertypes removed by the copy-as-enters replacement.
    pub added_supertypes: Vec<Supertype>,
    pub removed_supertypes: Vec<Supertype>,
    /// Additional subtypes granted by the copy-as-enters replacement.
    pub added_subtypes: Vec<Subtype>,
    /// Additional abilities granted by the copy-as-enters replacement.
    pub added_abilities: Vec<Ability>,
    /// Base power/toughness set as the object enters.
    pub set_base_power_toughness: Option<(i32, i32)>,
    /// If set, the object enters under this player's control.
    pub controller_override: Option<PlayerId>,
    /// Selected entry program awaiting execution without committing the event.
    pub(crate) pending_program: Option<(crate::resolution::ResolutionProgram, PlayerId)>,
    /// Effects and links accumulated by entry programs before final choices.
    pub(crate) program_choices: crate::game_state::PreparedEtbChoices,
    /// As-entry choices already collected against this provisional object.
    pub(crate) prepared_choices: Option<crate::game_state::PreparedEtbChoices>,
    /// Frozen paid-cost evidence for this exact stack-to-battlefield
    /// incarnation; it is not part of the permanent's copiable values.
    pub emerge_sacrifice: Option<Vec<crate::snapshot::ObjectSnapshot>>,
}

impl EnterBattlefieldEvent {
    /// Create a new enter battlefield event.
    pub fn new(object: ObjectId, from: Zone) -> Self {
        Self {
            object,
            completed_snapshot: None,
            emerge_sacrifice: None,
            from,
            enters_tapped: false,
            enters_with_counters: Vec::new(),
            linked_exile_with_entering: Vec::new(),
            enters_as_copy_of: None,
            copy_followups: Vec::new(),
            copy_duration: None,
            copy_name_override: None,
            added_colors: ColorSet::new(),
            added_card_types: Vec::new(),
            removes_other_card_types: false,
            added_supertypes: Vec::new(),
            removed_supertypes: Vec::new(),
            added_subtypes: Vec::new(),
            added_abilities: Vec::new(),
            set_base_power_toughness: None,
            controller_override: None,
            prepared_choices: None,
            pending_program: None,
            program_choices: Default::default(),
        }
    }

    /// Create an event where the permanent enters tapped.
    pub fn tapped(object: ObjectId, from: Zone) -> Self {
        Self {
            object,
            completed_snapshot: None,
            emerge_sacrifice: None,
            from,
            enters_tapped: true,
            enters_with_counters: Vec::new(),
            linked_exile_with_entering: Vec::new(),
            enters_as_copy_of: None,
            copy_followups: Vec::new(),
            copy_duration: None,
            copy_name_override: None,
            added_colors: ColorSet::new(),
            added_card_types: Vec::new(),
            removes_other_card_types: false,
            added_supertypes: Vec::new(),
            removed_supertypes: Vec::new(),
            added_subtypes: Vec::new(),
            added_abilities: Vec::new(),
            set_base_power_toughness: None,
            controller_override: None,
            prepared_choices: None,
            pending_program: None,
            program_choices: Default::default(),
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
        let mut next = self.clone();
        next.enters_as_copy_of = Some(source_id);
        // A later replacement copy is a new text acquisition even when its
        // definition is identical. Never borrow an earlier copy's choice.
        next.program_choices.entry_copy_registration = None;
        next
    }

    pub fn with_copy_followups(&self, followups: &[ironsmith_core::EnterAsCopyFollowup]) -> Self {
        Self {
            copy_followups: followups.to_vec(),
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

    /// Return a new event with colors added to its copied characteristics.
    pub fn with_added_colors(&self, colors: ColorSet) -> Self {
        Self {
            added_colors: self.added_colors.union(colors),
            ..self.clone()
        }
    }

    /// Return a new event with additional card types granted as it enters.
    pub fn with_removes_other_card_types(&self, removes: bool) -> Self {
        Self {
            removes_other_card_types: removes,
            ..self.clone()
        }
    }

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

    /// Return a new event with additional copiable supertypes as it enters.
    pub fn with_added_supertypes(&self, supertypes: &[Supertype]) -> Self {
        let mut added_supertypes = self.added_supertypes.clone();
        for supertype in supertypes {
            if !added_supertypes.contains(supertype) {
                added_supertypes.push(*supertype);
            }
        }
        Self {
            added_supertypes,
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
        // Ability instances are ordered occurrences (CR 113.2c), even when
        // their text and executable payloads compare equal.
        added_abilities.extend_from_slice(abilities);
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
            if !self.added_colors.is_empty() {
                object.color_override = Some(object.colors().union(self.added_colors));
            }
            for card_type in &self.added_card_types {
                if !object.card_types.contains(card_type) {
                    object.card_types.push(*card_type);
                }
            }
            if self.removes_other_card_types {
                crate::continuous::replace_card_types_and_prune_subtypes(
                    &mut object.card_types, &mut object.subtypes, &self.added_card_types,
                );
            }
            object
                .supertypes
                .retain(|supertype| !self.removed_supertypes.contains(supertype));
            for supertype in &self.added_supertypes {
                if !object.supertypes.contains(supertype) {
                    object.supertypes.push(*supertype);
                }
            }
            for subtype in &self.added_subtypes {
                if !object.subtypes.contains(subtype) {
                    object.subtypes.push(*subtype);
                }
            }
            object.abilities_mut().extend(self.added_abilities.iter().cloned());
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

        // A duration copy is one acquired text box. Reuse the reservation
        // owned by entry preparation so choices, prospective CDA/restrictions,
        // and the committed layer-one effect have the same native owner.
        if let (Some(copy_source), Some(duration)) = (self.enters_as_copy_of, self.copy_duration.clone()) {
            let values = crate::snapshot::CopiableValues::from_object(prospective.object(self.object)?);
            let controller = self.controller_override
                .or_else(|| prospective.current_controller(self.object))?;
            let registration = self.program_choices.entry_copy_registration
                .or_else(|| self.prepared_choices.as_ref().and_then(|choices| choices.entry_copy_registration))
                .unwrap_or_else(|| prospective.effect_store.continuous_effects.reserve_entry_effect());
            let expires = matches!(&duration, crate::effect::Until::EndOfTurn
                | crate::effect::Until::YourNextTurn | crate::effect::Until::YourNextUpkeep
                | crate::effect::Until::ControllersNextUntapStep)
                .then_some(prospective.turn.turn_number).unwrap_or(u32::MAX);
            let effect = crate::continuous::ContinuousEffect::new(self.object, controller,
                crate::continuous::EffectTarget::Specific(self.object),
                crate::continuous::Modification::CopyOf {
                    target_id: copy_source, copiable_values: Box::new(values),
                    preserve_source_abilities: false, name_override: None,
                    name_override_surface: None, add_supertypes: Vec::new(),
                }).until(duration).with_expires_end_of_turn(expires)
                .with_source_type(crate::continuous::EffectSourceType::Resolution {
                    locked_targets: vec![self.object],
                });
            prospective.effect_store.continuous_effects
                .add_reserved_entry_effect(registration, effect).ok()?;
        }

        if !prospective.battlefield.contains(&self.object) {
            prospective.battlefield.push(self.object);
        }
        if let Some(controller) = self.controller_override {
            prospective.stage_initial_controller_for_assembly(self.object, controller);
        }
        if let Some(choices) = &self.prepared_choices {
            if let Some(object) = prospective.object_mut(self.object) {
                object
                    .cast_tagged_objects
                    .extend(choices.as_enters_tagged_objects.clone());
            }
            if let Some(color) = choices.chosen_color {
                prospective.set_chosen_color(self.object, color);
            }
            if let Some(colors) = choices.chosen_color_set {
                prospective.set_chosen_colors(self.object, colors);
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
            if let Some(players) = choices.chosen_player_set.clone() {
                prospective.set_chosen_players(self.object, players);
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
                        object.abilities_mut().push(ability);
                    }
                }
            }
        }
        Some(prospective)
    }

    /// Construct a complete prospective query world without publishing the
    /// entry or running state-based game procedures. Missing objects remain
    /// distinct from failed continuous-effect discovery.
    pub(crate) fn try_prospective_game_state(
        &self,
        game: &GameState,
    ) -> Result<Option<GameState>, crate::static_ability_processor::StaticEffectDiscoveryError> {
        if game.object(self.object).is_none() {
            return Ok(None);
        }
        // Copy values must be read from a complete original world, before the
        // entrant's own changes are applied in the separate prospective world.
        let original = game.continuous_query_snapshot()?;
        // Assemble every entry field, including the initial controller, before
        // evaluating the complete world. Initial control is below layer two.
        let Some(prospective) = self.prospective_game_state(&original) else {
            return Ok(None);
        };
        // Completeness does not carry across the zone, copy, control, counter,
        // ability and prepared-choice modifications.
        prospective.continuous_query_snapshot().map(Some)
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
        if let Some(snapshot) = &self.completed_snapshot { return snapshot.controller; }
        // CR 616.1 / 110.2a: the permanent's controller chooses the order of
        // its entry replacements, and a permanent entering under a player's
        // control is controlled by that player.
        if let Some(controller) = self.controller_override {
            return controller;
        }
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

    fn snapshot(&self) -> Option<&crate::snapshot::ObjectSnapshot> {
        self.completed_snapshot.as_ref()
    }

    fn controller(&self) -> Option<PlayerId> {
        self.completed_snapshot.as_ref().map(|snapshot| snapshot.controller)
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

#[cfg(test)]
mod entry_granted_occurrence_tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::{StaticAbility, StaticAbilityInstanceId};

    fn independent_grants() -> Vec<Ability> {
        let first = StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 1);
        let second = StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 1);
        assert_eq!(first, second, "semantic equality is deliberately not occurrence identity");
        assert_ne!(first.instance_id(), second.instance_id());
        vec![Ability::static_ability(first), Ability::static_ability(second)]
    }
    fn ids(abilities: &[Ability]) -> Vec<StaticAbilityInstanceId> {
        abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Static(value) if value.id() == crate::static_abilities::StaticAbilityId::EnterWithCounters => Some(value.instance_id()),
            _ => None,
        }).collect()
    }
    fn setup() -> (GameState, ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Independent entry abilities")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 1)))
            .build();
        let entrant = game.create_object_from_definition(&definition, alice, Zone::Hand);
        game.take_pending_trigger_events();
        (game, entrant)
    }

    #[test]
    fn entry_granted_occurrences_survive_event_composition() {
        let grants = independent_grants();
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(10), Zone::Hand)
            .with_added_abilities(&grants[..1]).with_added_abilities(&grants[1..]);
        assert_eq!(event.added_abilities.len(), 2);
        assert_eq!(ids(&event.added_abilities), ids(&grants));
    }

    #[test]
    fn entry_granted_occurrences_survive_prospective_world_with_equal_printed_ability() {
        let (game, entrant) = setup();
        let grants = independent_grants();
        let expected: Vec<_> = ids(&game.object(entrant).unwrap().abilities).into_iter()
            .chain(ids(&grants)).collect();
        let mut event = EnterBattlefieldEvent::new(entrant, Zone::Hand);
        // Construct the carrier directly to isolate prospective-world assembly
        // from the independent event-composition regression.
        event.added_abilities = grants;
        let prospective = event.try_prospective_game_state(&game).unwrap().unwrap();
        assert_eq!(ids(&prospective.object(entrant).unwrap().abilities), expected);
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert_eq!(ids(&game.object(entrant).unwrap().abilities).len(), 1);
    }

    #[test]
    fn entry_granted_occurrences_apply_independently_and_survive_public_movement() {
        let (mut game, entrant) = setup();
        let grants = independent_grants();
        let alice = PlayerId::from_index(0);
        game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
            entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
            crate::replacement::ReplacementAction::EnterWithCounters {
                counter_type: CounterType::PlusOnePlusOne, count: crate::effect::Value::Fixed(0),
                count_condition: None, otherwise_count: None, added_subtypes: Vec::new(), added_abilities: grants,
            },
        ));
        let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
        assert!(!receipt.pending && receipt.programs.is_empty());
        let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("original entry completes"); };
        let object = game.object(result.new_id).unwrap();
        assert_eq!(object.zone, Zone::Battlefield);
        assert_eq!(ids(&object.abilities).len(), 3, "printed ability plus two independent grants survive commit");
        assert_eq!(object.counters.get(&CounterType::PlusOnePlusOne).copied(), Some(3),
            "all three independently applicable entry replacements apply once");
    }
    #[test]
    fn entry_granted_occurrences_survive_permanent_and_temporary_copy_commit() {
        for duration in [None, Some(crate::effect::Until::EndOfTurn)] {
            let (mut game, copy_source) = setup();
            let receipt = game.move_object_with_etb_processing(copy_source, Zone::Battlefield).unwrap();
            assert!(!receipt.pending && receipt.programs.is_empty());
            let crate::events::processing::EventOutcome::Proceed(source) = receipt.original else { panic!("copy source enters"); };
            let alice = PlayerId::from_index(0);
            let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy entrant")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
            let entrant = game.create_object_from_definition(&definition, alice, Zone::Hand);
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                crate::replacement::ReplacementAction::EnterAsCopy {
                    source: source.new_id, enters_tapped: false, copy_duration: duration.clone(),
                    linked_exile_objects: Vec::new(), additional_counters: Vec::new(), name_override: None,
                    added_colors: ColorSet::new(), added_card_types: Vec::new(), removes_other_card_types: false,
                    added_supertypes: Vec::new(), removed_supertypes: Vec::new(), added_subtypes: Vec::new(),
                    added_abilities: independent_grants(), set_base_power_toughness: None, copy_followups: Vec::new(),
                },
            ));
            let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
            assert!(!receipt.pending && receipt.programs.is_empty());
            let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("copy entry completes"); };
            let instances = ids(&game.current_abilities(result.new_id).unwrap());
            assert_eq!(instances.len(), 3, "copy and both independent grants survive: {duration:?}");
            assert_eq!(instances.iter().copied().collect::<std::collections::HashSet<_>>().len(), 3);
            assert_eq!(game.object(result.new_id).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied(), Some(3),
                "copied and granted replacements each apply once: {duration:?}");
        }
    }

    #[test]
    fn entry_granted_occurrences_survive_prepared_power_toughness_choice_commit() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Characteristic choice entrant")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::toxic(1)))
            .with_ability(Ability::static_ability(StaticAbility::choose_power_toughness_options_as_enters_or_turns_face_up(
                vec![crate::static_abilities::PowerToughnessChoiceOption::with_abilities(4, 5,
                    vec![StaticAbility::toxic(1), StaticAbility::toxic(1)])], "choose characteristics".into(),
            ))).build();
        let entrant = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
        assert!(!receipt.pending && receipt.programs.is_empty());
        let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("choice entry completes"); };
        let toxic: Vec<_> = crate::ability::extract_static_abilities(&game.current_abilities(result.new_id).unwrap())
            .into_iter().filter_map(|ability| ability.toxic_amount()).collect();
        assert_eq!(toxic, vec![1, 1, 1], "printed occurrence and both selected occurrences survive");
        assert_eq!(game.calculated_power(result.new_id), Some(4));
        assert_eq!(game.calculated_toughness(result.new_id), Some(5));
    }
}

#[cfg(test)]
mod entry_type_projection_tests {
    use super::*;
    use crate::target::ObjectFilter;
    fn setup(kindred: bool) -> (GameState, ObjectId, ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let mut types = vec![CardType::Artifact, CardType::Creature];
        if kindred { types.push(CardType::Kindred); }
        let source_definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy type source")
            .card_types(types).subtypes(vec![Subtype::Elf, Subtype::Equipment])
            .supertypes(vec![Supertype::Legendary])
            .power_toughness(crate::card::PowerToughness::fixed(3, 4)).build();
        let source = game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
        let entrant_definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy type entrant")
            .card_types(vec![CardType::Artifact]).build();
        let entrant = game.create_object_from_definition(&entrant_definition, alice, Zone::Hand);
        game.take_pending_trigger_events();
        (game, entrant, source)
    }
    fn copy_action(source: ObjectId, remove: bool, duration: Option<crate::effect::Until>) -> crate::replacement::ReplacementAction {
        crate::replacement::ReplacementAction::EnterAsCopy {
            source, enters_tapped: false, copy_duration: duration,
            linked_exile_objects: Vec::new(), additional_counters: Vec::new(), name_override: None,
            added_colors: ColorSet::new(), added_card_types: vec![CardType::Artifact], removes_other_card_types: remove,
            added_supertypes: Vec::new(), removed_supertypes: Vec::new(), added_subtypes: vec![Subtype::Vehicle],
            added_abilities: Vec::new(), set_base_power_toughness: None, copy_followups: Vec::new(),
        }
    }
    fn enter(game: &mut GameState, entrant: ObjectId, source: ObjectId, remove: bool, duration: Option<crate::effect::Until>) -> ObjectId {
        game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
            entrant, PlayerId::from_index(0), crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
            copy_action(source, remove, duration),
        ));
        let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
        assert!(!receipt.pending && receipt.programs.is_empty());
        let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("copy enters"); };
        result.new_id
    }
    #[test]
    fn entry_type_projection_replaces_types_and_prunes_only_unsupported_subtype_families() {
        let (game, entrant, source) = setup(false);
        let event = EnterBattlefieldEvent::new(entrant, Zone::Hand).with_copy_of(source)
            .with_added_card_types(&[CardType::Artifact]).with_removes_other_card_types(true)
            .with_added_subtypes(&[Subtype::Vehicle]);
        let prospective = event.try_prospective_game_state(&game).unwrap().unwrap();
        let object = prospective.object(entrant).unwrap();
        assert_eq!(object.card_types.as_slice(), &[CardType::Artifact]);
        assert_eq!(object.subtypes.as_slice(), &[Subtype::Equipment, Subtype::Vehicle]);
        assert_eq!(object.supertypes.as_slice(), &[Supertype::Legendary]);
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert!(game.object(source).unwrap().card_types.contains(&CardType::Creature));
        assert!(game.object(source).unwrap().subtypes.contains(&Subtype::Elf));
    }
    #[test]
    fn entry_type_projection_kindred_retains_creature_subtypes_after_creature_type_removed() {
        let (game, entrant, source) = setup(true);
        let event = EnterBattlefieldEvent::new(entrant, Zone::Hand).with_copy_of(source)
            .with_added_card_types(&[CardType::Artifact, CardType::Kindred]).with_removes_other_card_types(true);
        let prospective = event.try_prospective_game_state(&game).unwrap().unwrap();
        let object = prospective.object(entrant).unwrap();
        assert_eq!(object.card_types.as_slice(), &[CardType::Artifact, CardType::Kindred]);
        assert_eq!(object.subtypes.as_slice(), &[Subtype::Elf, Subtype::Equipment]);
    }
    #[test]
    fn entry_type_projection_public_copy_commit_prunes_subtypes_for_both_copy_durations() {
        for duration in [None, Some(crate::effect::Until::EndOfTurn)] {
            let (mut game, entrant, source) = setup(false);
            let entered = enter(&mut game, entrant, source, true, duration.clone());
            assert_eq!(game.calculated_card_types(entered).as_slice(), &[CardType::Artifact], "{duration:?}");
            assert_eq!(game.calculated_subtypes(entered).as_slice(), &[Subtype::Equipment, Subtype::Vehicle], "{duration:?}");
            assert_eq!(game.object(source).unwrap().subtypes.as_slice(), &[Subtype::Elf, Subtype::Equipment]);
        }
    }
    #[test]
    fn entry_type_projection_replacement_matching_uses_resolved_copy_types_and_subtypes() {
        for remove in [true, false] {
            for duration in [None, Some(crate::effect::Until::EndOfTurn)] {
                let (mut game, entrant, source) = setup(false);
                let mut watcher = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Type-filtered entry replacements")
                    .card_types(vec![CardType::Enchantment]);
                for (filter, amount) in [
                    (ObjectFilter::creature(), 1), (ObjectFilter::artifact(), 2),
                    (ObjectFilter::permanent().with_subtype(Subtype::Elf), 4),
                    (ObjectFilter::permanent().with_subtype(Subtype::Vehicle), 8),
                    (ObjectFilter::permanent().with_subtype(Subtype::Equipment), 16),
                ] {
                    watcher = watcher.with_ability(Ability::static_ability(crate::static_abilities::StaticAbility::enters_with_counters_for_filter(
                        filter, CounterType::PlusOnePlusOne, amount,
                    )));
                }
                game.create_object_from_definition(&watcher.build(), PlayerId::from_index(0), Zone::Battlefield);
                let entered = enter(&mut game, entrant, source, remove, duration.clone());
                assert_eq!(game.object(entered).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied(),
                    Some(if remove { 26 } else { 31 }), "resolved type/subtype applicability: remove={remove}, duration={duration:?}");
                assert_eq!(game.calculated_card_types(entered).contains(&CardType::Creature), !remove);
                assert_eq!(game.calculated_subtypes(entered).contains(&Subtype::Elf), !remove);
            }
        }
    }
}

#[cfg(test)]
mod entry_copiable_choice_tests {
    use super::*;
    #[test]
    fn entry_copiable_choice_copy_of_temporary_copy_executes_inherited_choice() {
        for duration in [None, Some(crate::effect::Until::EndOfTurn)] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice copy base")
                .card_types(vec![CardType::Artifact]).build();
            let base = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let first = game.create_object_from_definition(&definition, alice, Zone::Hand);
            let choice = Ability::static_ability(crate::static_abilities::StaticAbility::choose_color_as_enters(None, "choose a color".into()));
            let action = |source, copy_duration, added_abilities| crate::replacement::ReplacementAction::EnterAsCopy {
                source, enters_tapped: false, copy_duration,
                linked_exile_objects: Vec::new(), additional_counters: Vec::new(), name_override: None,
                added_colors: ColorSet::new(), added_card_types: Vec::new(), removes_other_card_types: false,
                added_supertypes: Vec::new(), removed_supertypes: Vec::new(), added_subtypes: Vec::new(),
                added_abilities, set_base_power_toughness: None, copy_followups: Vec::new(),
            };
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                first, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                action(base, Some(crate::effect::Until::EndOfTurn), vec![choice]),
            ));
            let receipt = game.move_object_with_etb_processing(first, Zone::Battlefield).unwrap();
            assert!(!receipt.pending && receipt.programs.is_empty());
            let crate::events::processing::EventOutcome::Proceed(first) = receipt.original else { panic!("first copy enters"); };
            assert_eq!(game.chosen_color(first.new_id), Some(crate::color::Color::White));
            assert!(game.object(first.new_id).unwrap().abilities.is_empty(), "temporary copy keeps printed base separately");
            assert!(crate::ability::extract_static_abilities(&game.current_abilities(first.new_id).unwrap()).iter()
                .any(|ability| ability.color_choice_as_enters().is_some()), "layer-one copy contains the choice");
            let second = game.create_object_from_definition(&definition, alice, Zone::Hand);
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                second, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                action(first.new_id, duration.clone(), Vec::new()),
            ));
            let receipt = game.move_object_with_etb_processing(second, Zone::Battlefield).unwrap();
            assert!(!receipt.pending && receipt.programs.is_empty());
            let crate::events::processing::EventOutcome::Proceed(second) = receipt.original else { panic!("second copy enters"); };
            assert_eq!(game.chosen_color(second.new_id), Some(crate::color::Color::White),
                "copied entry choice executes from resolved copiable values: {duration:?}");
            assert_eq!(game.chosen_color(first.new_id), Some(crate::color::Color::White));
            assert_eq!(game.chosen_color(base), None);
        }
    }
}

#[cfg(test)]
mod entry_copiable_battle_tests {
    use super::*;
    struct SelectLast;
    impl crate::decision::DecisionMaker for SelectLast {
        fn decide_options(&mut self, _game: &GameState, context: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
            context.options.iter().rev().find(|option| option.legal).map(|option| vec![option.index]).unwrap_or_default()
        }
    }
    #[test]
    fn entry_copiable_battle_copy_of_temporary_copy_asks_for_protector() {
        for duration in [None, Some(crate::effect::Until::EndOfTurn)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let alice = PlayerId::from_index(0);
            let charlie = PlayerId::from_index(2);
            let battle = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copied siege")
                .card_types(vec![CardType::Battle]).subtypes(vec![Subtype::Siege]).defense(5).build();
            let base = game.create_object_from_definition(&battle, alice, Zone::Battlefield);
            let blank = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Battle copy entrant")
                .card_types(vec![CardType::Artifact]).build();
            let action = |source, copy_duration| crate::replacement::ReplacementAction::EnterAsCopy {
                source, enters_tapped: false, copy_duration,
                linked_exile_objects: Vec::new(), additional_counters: Vec::new(), name_override: None,
                added_colors: ColorSet::new(), added_card_types: Vec::new(), removes_other_card_types: false,
                added_supertypes: Vec::new(), removed_supertypes: Vec::new(), added_subtypes: Vec::new(),
                added_abilities: Vec::new(), set_base_power_toughness: None, copy_followups: Vec::new(),
            };
            let mut source = base;
            for (copy_duration, stage) in [(Some(crate::effect::Until::EndOfTurn), "first"), (duration.clone(), "second")] {
                let entrant = game.create_object_from_definition(&blank, alice, Zone::Hand);
                game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                    entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    action(source, copy_duration),
                ));
                let receipt = game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut SelectLast).unwrap();
                assert!(!receipt.pending && receipt.programs.is_empty());
                let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("copy enters"); };
                assert!(game.calculated_card_types(result.new_id).contains(&CardType::Battle));
                assert!(game.calculated_subtypes(result.new_id).contains(&Subtype::Siege));
                assert_eq!(game.battle_protector(result.new_id), Some(charlie),
                    "resolved copied siege must ask for its protector: {stage}, {duration:?}");
                source = result.new_id;
            }
        }
    }
}

#[cfg(test)]
mod entry_copiable_intrinsic_counter_tests {
    use super::*;
    fn copy_chain(card_type: CardType, counter: CounterType) {
        for first_duration in [None, Some(crate::effect::Until::EndOfTurn)] {
            for second_duration in [None, Some(crate::effect::Until::EndOfTurn)] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let alice = PlayerId::from_index(0);
                let builder = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Intrinsic copy source").card_types(vec![card_type]);
                let definition = if card_type == CardType::Battle { builder.subtypes(vec![Subtype::Siege]).defense(5).build() }
                    else { builder.loyalty(5).build() };
                let base = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                // The currently remaining counters are not the copied printed number.
                game.object_mut(base).unwrap().counters.insert(counter, 2);
                let blank = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Intrinsic copy entrant")
                    .card_types(vec![CardType::Artifact]).build();
                let mut source = base;
                for (duration, stage) in [(first_duration.clone(), "first"), (second_duration.clone(), "second")] {
                    let entrant = game.create_object_from_definition(&blank, alice, Zone::Hand);
                    game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                        entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                        crate::replacement::ReplacementAction::EnterAsCopy {
                            source, enters_tapped: false, copy_duration: duration,
                            linked_exile_objects: Vec::new(), additional_counters: Vec::new(), name_override: None,
                            added_colors: ColorSet::new(), added_card_types: Vec::new(), removes_other_card_types: false,
                            added_supertypes: Vec::new(), removed_supertypes: Vec::new(), added_subtypes: Vec::new(),
                            added_abilities: Vec::new(), set_base_power_toughness: None, copy_followups: Vec::new(),
                        },
                    ));
                    let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
                    assert!(!receipt.pending && receipt.programs.is_empty());
                    let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("copy enters"); };
                    assert!(game.calculated_card_types(result.new_id).contains(&card_type));
                    assert_eq!(game.counter_count(result.new_id, counter), 5,
                        "copied printed number survives: {counter:?}, {stage}, {first_duration:?}, {second_duration:?}");
                    source = result.new_id;
                }
                assert_eq!(game.counter_count(base, counter), 2);
            }
        }
    }
    #[test]
    fn entry_copiable_intrinsic_defense_survives_copy_chains_and_ignores_remaining_counters() {
        copy_chain(CardType::Battle, CounterType::Defense);
    }
    #[test]
    fn entry_copiable_intrinsic_loyalty_survives_copy_chains_and_ignores_remaining_counters() {
        copy_chain(CardType::Planeswalker, CounterType::Loyalty);
    }
}

#[cfg(test)]
mod entry_copy_extra_defense_tests {
    use super::*;
    #[test]
    fn entry_copy_extra_defense_preserves_authored_counters_beside_intrinsic_copy_counters() {
        for duration in [None, Some(crate::effect::Until::EndOfTurn)] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let source = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Extra defense source")
                .card_types(vec![CardType::Battle]).subtypes(vec![Subtype::Siege]).defense(5).build();
            let source = game.create_object_from_definition(&source, alice, Zone::Battlefield);
            let entrant = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Extra defense entrant")
                .card_types(vec![CardType::Artifact]).build();
            let entrant = game.create_object_from_definition(&entrant, alice, Zone::Hand);
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                crate::replacement::ReplacementAction::EnterAsCopy {
                    source, enters_tapped: false, copy_duration: duration.clone(), linked_exile_objects: Vec::new(),
                    additional_counters: vec![(CounterType::Defense, 3)], name_override: None,
                    added_colors: ColorSet::new(), added_card_types: Vec::new(), removes_other_card_types: false,
                    added_supertypes: Vec::new(), removed_supertypes: Vec::new(), added_subtypes: Vec::new(),
                    added_abilities: Vec::new(), set_base_power_toughness: None, copy_followups: Vec::new(),
                },
            ));
            let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
            assert!(!receipt.pending && receipt.programs.is_empty());
            let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("copy enters"); };
            assert_eq!(game.counter_count(result.new_id, CounterType::Defense), 8,
                "intrinsic five plus independently authored three: {duration:?}");
        }
    }
}

#[cfg(test)]
mod entry_copy_intrinsic_contribution_tests {
    use super::*;
    #[test]
    fn entry_copy_intrinsic_contribution_replaces_prior_number_preserves_extra_and_obeys_final_type() {
        for (card_type, counter) in [(CardType::Battle, CounterType::Defense), (CardType::Planeswalker, CounterType::Loyalty)] {
            for duration in [None, Some(crate::effect::Until::EndOfTurn)] {
                for remove_type in [false, true] {
                    let mut game = crate::tests::test_helpers::setup_two_player_game();
                    let alice = PlayerId::from_index(0);
                    let build = |number| {
                        let builder = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Intrinsic contribution")
                            .card_types(vec![card_type]);
                        if card_type == CardType::Battle { builder.subtypes(vec![Subtype::Siege]).defense(number).build() }
                        else { builder.loyalty(number).build() }
                    };
                    let source = game.create_object_from_definition(&build(5), alice, Zone::Battlefield);
                    let entrant = game.create_object_from_definition(&build(7), alice, Zone::Hand);
                    game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                        entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                        crate::replacement::ReplacementAction::EnterAsCopy {
                            source, enters_tapped: false, copy_duration: duration.clone(), linked_exile_objects: Vec::new(),
                            additional_counters: vec![(counter, 3)], name_override: None,
                            added_colors: ColorSet::new(), added_card_types: if remove_type { vec![CardType::Artifact] } else { Vec::new() },
                            removes_other_card_types: remove_type, added_supertypes: Vec::new(), removed_supertypes: Vec::new(),
                            added_subtypes: Vec::new(), added_abilities: Vec::new(), set_base_power_toughness: None, copy_followups: Vec::new(),
                        },
                    ));
                    let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
                    assert!(!receipt.pending && receipt.programs.is_empty());
                    let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("copy enters"); };
                    assert_eq!(game.counter_count(result.new_id, counter), if remove_type { 3 } else { 8 },
                        "replace intrinsic seven with copied five only if final type supports it; retain extra three: {counter:?}, {duration:?}, remove={remove_type}");
                    assert_eq!(game.calculated_card_types(result.new_id).contains(&card_type), !remove_type);
                }
            }
        }
    }
}

#[cfg(test)]
mod entry_intrinsic_ability_loss_tests {
    use super::*;
    fn enter_under_loss(card_type: CardType, counter: CounterType, copy: bool) {
        for creature in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let watcher = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ability loss source")
                .card_types(vec![CardType::Enchantment]).build();
            let watcher = game.create_object_from_definition(&watcher, alice, Zone::Battlefield);
            game.effect_store.continuous_effects.add_effect(crate::continuous::ContinuousEffect::new(
                watcher, alice, crate::continuous::EffectTarget::Filter(crate::target::ObjectFilter::creature()),
                crate::continuous::Modification::RemoveAllAbilities,
            ));
            let mut types = vec![card_type]; if creature { types.push(CardType::Creature); }
            let builder = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Intrinsic ability entrant")
                .card_types(types).power_toughness(crate::card::PowerToughness::fixed(1, 1))
                .with_ability(Ability::static_ability(crate::static_abilities::StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 4)));
            let definition = if card_type == CardType::Battle { builder.subtypes(vec![Subtype::Siege]).defense(5).build() }
                else { builder.loyalty(5).build() };
            let entrant = if copy {
                let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let blank = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Intrinsic copy under loss")
                    .card_types(vec![CardType::Artifact]).build();
                let entrant = game.create_object_from_definition(&blank, alice, Zone::Hand);
                game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                    entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    crate::replacement::ReplacementAction::EnterAsCopy {
                        source, enters_tapped: false, copy_duration: None, linked_exile_objects: Vec::new(),
                        additional_counters: Vec::new(), name_override: None, added_colors: ColorSet::new(),
                        added_card_types: Vec::new(), removes_other_card_types: false, added_supertypes: Vec::new(),
                        removed_supertypes: Vec::new(), added_subtypes: Vec::new(), added_abilities: Vec::new(),
                        set_base_power_toughness: None, copy_followups: Vec::new(),
                    },
                ));
                entrant
            } else { game.create_object_from_definition(&definition, alice, Zone::Hand) };
            let receipt = game.move_object_with_etb_processing_with_initial_counters_with_dm(
                entrant, Zone::Battlefield, vec![(counter, 2)], &mut crate::decision::SelectFirstDecisionMaker,
            ).unwrap();
            assert!(!receipt.pending && receipt.programs.is_empty());
            let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("entry completes"); };
            assert_eq!(game.counter_count(result.new_id, CounterType::PlusOnePlusOne), if creature { 0 } else { 4 },
                "printed entry ability demonstrates actual loss domain");
            assert_eq!(game.counter_count(result.new_id, counter), if creature { 2 } else { 7 },
                "intrinsic entry ability obeys the same loss; authored two remain: {counter:?}, copy={copy}, creature={creature}");
        }
    }
    #[test] fn entry_intrinsic_ability_loss_suppresses_direct_battle_defense() { enter_under_loss(CardType::Battle, CounterType::Defense, false); }
    #[test] fn entry_intrinsic_ability_loss_suppresses_direct_planeswalker_loyalty() { enter_under_loss(CardType::Planeswalker, CounterType::Loyalty, false); }
    #[test] fn entry_intrinsic_ability_loss_suppresses_copied_battle_defense() { enter_under_loss(CardType::Battle, CounterType::Defense, true); }
    #[test] fn entry_intrinsic_ability_loss_suppresses_copied_planeswalker_loyalty() { enter_under_loss(CardType::Planeswalker, CounterType::Loyalty, true); }
    #[test]
    fn entry_intrinsic_ability_loss_basic_land_mana_is_not_readded_after_loss() {
        for creature in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0);
            let mut types = vec![CardType::Land]; if creature { types.push(CardType::Creature); }
            let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Intrinsic Forest ability")
                .card_types(types).subtypes(vec![Subtype::Forest]).power_toughness(crate::card::PowerToughness::fixed(1, 1)).build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.effect_store.continuous_effects.add_effect(crate::continuous::ContinuousEffect::new(
                source, alice, crate::continuous::EffectTarget::Filter(crate::target::ObjectFilter::creature()),
                crate::continuous::Modification::RemoveAllAbilities,
            ));
            game.refresh_continuous_state().unwrap();
            let intrinsic = Ability::basic_land_mana(Subtype::Forest).unwrap();
            assert_eq!(game.current_abilities(source).unwrap().contains(&intrinsic), !creature,
                "intrinsic land mana must participate in layer-six ability removal: creature={creature}");
        }
    }
}

#[cfg(test)]
mod entry_intrinsic_counter_width_tests {
    use super::*;
    #[test]
    fn entry_intrinsic_starting_counters_preserve_unsigned_printed_values_for_direct_and_copied_entries() {
        for rule in [ironsmith_core::IntrinsicStartingCounter::Loyalty, ironsmith_core::IntrinsicStartingCounter::Defense] {
            for copy in [false, true] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let alice = PlayerId::from_index(0);
                let printed = 1_u32 << 31;
                let builder = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unsigned intrinsic number")
                    .card_types(vec![rule.card_type()]);
                let definition = match rule {
                    ironsmith_core::IntrinsicStartingCounter::Loyalty => builder.loyalty(printed).build(),
                    ironsmith_core::IntrinsicStartingCounter::Defense => builder.subtypes(vec![Subtype::Siege]).defense(printed).build(),
                };
                let entrant = if copy {
                    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    game.add_counters(source, rule.counter_type(), 2).unwrap();
                    let blank = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unsigned intrinsic copy")
                        .card_types(vec![CardType::Artifact]).build();
                    let entrant = game.create_object_from_definition(&blank, alice, Zone::Hand);
                    game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                        entrant, alice, crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                        crate::replacement::ReplacementAction::EnterAsCopy {
                            source, enters_tapped: false, copy_duration: None, linked_exile_objects: Vec::new(),
                            additional_counters: Vec::new(), name_override: None, added_colors: ColorSet::new(),
                            added_card_types: Vec::new(), removes_other_card_types: false, added_supertypes: Vec::new(),
                            removed_supertypes: Vec::new(), added_subtypes: Vec::new(), added_abilities: Vec::new(),
                            set_base_power_toughness: None, copy_followups: Vec::new(),
                        },
                    ));
                    entrant
                } else { game.create_object_from_definition(&definition, alice, Zone::Hand) };
                let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
                assert!(!receipt.pending && receipt.programs.is_empty());
                let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("entry completes"); };
                assert_eq!(game.counter_count(result.new_id, rule.counter_type()), printed, "unsigned value survives: {rule:?}, copy={copy}");
            }
        }
    }
}

#[cfg(test)]
mod entry_intrinsic_counter_overflow_tests {
    use super::*;
    #[test]
    fn entry_intrinsic_starting_counter_overflow_returns_error_without_committing_entry() {
        for rule in [ironsmith_core::IntrinsicStartingCounter::Loyalty, ironsmith_core::IntrinsicStartingCounter::Defense] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let builder = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Overflowing intrinsic number")
                .card_types(vec![rule.card_type()]);
            let definition = match rule {
                ironsmith_core::IntrinsicStartingCounter::Loyalty => builder.loyalty(u32::MAX).build(),
                ironsmith_core::IntrinsicStartingCounter::Defense => builder.subtypes(vec![Subtype::Siege]).defense(u32::MAX).build(),
            };
            let entrant = game.create_object_from_definition(&definition, alice, Zone::Hand);
            let before_ids = game.object_ids_in_deterministic_order();
            let before_battlefield = game.battlefield.clone();
            let result = game.move_object_with_etb_processing_with_initial_counters_with_dm(
                entrant, Zone::Battlefield, vec![(rule.counter_type(), 1)], &mut crate::decision::SelectFirstDecisionMaker,
            );
            assert!(matches!(result, Err(crate::effects::ExecutionError::InternalError(ref message)) if message.contains("counter contribution overflow")));
            assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
            assert_eq!(game.counter_count(entrant, rule.counter_type()), 0);
            assert_eq!(game.object_ids_in_deterministic_order(), before_ids);
            assert_eq!(game.battlefield, before_battlefield);
        }
    }
}

#[cfg(test)]
mod entry_intrinsic_counter_zero_tests {
    use super::*;
    #[test]
    fn entry_intrinsic_zero_numbers_preserve_one_shot_until_positive_counter_placement() {
        for rule in [ironsmith_core::IntrinsicStartingCounter::Loyalty, ironsmith_core::IntrinsicStartingCounter::Defense] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zero counter shield source")
                .card_types(vec![CardType::Artifact]).build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let ability = crate::static_abilities::StaticAbility::double_counters_replacement(
                crate::target::ObjectFilter::permanent(), Some(rule.counter_type()),
                "If matching counters would be put, put twice that many instead".into());
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ability.generate_replacement_effect(source, alice).unwrap());
            for printed in [None, Some(0), Some(1)] {
                let builder = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zero counter recipient")
                    .card_types(vec![rule.card_type()]);
                let definition = match (rule, printed) {
                    (ironsmith_core::IntrinsicStartingCounter::Loyalty, Some(count)) => builder.loyalty(count).build(),
                    (ironsmith_core::IntrinsicStartingCounter::Defense, Some(count)) => builder.subtypes(vec![Subtype::Siege]).defense(count).build(),
                    (ironsmith_core::IntrinsicStartingCounter::Loyalty, None) => builder.build(),
                    (ironsmith_core::IntrinsicStartingCounter::Defense, None) => builder.subtypes(vec![Subtype::Siege]).build(),
                };
                let entrant = game.create_object_from_definition(&definition, alice, Zone::Hand);
                game.take_pending_trigger_events();
                let receipt = game.move_object_with_etb_processing(entrant, Zone::Battlefield).unwrap();
                assert!(!receipt.pending && receipt.programs.is_empty());
                let crate::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("entry completes"); };
                let positive = printed == Some(1);
                assert_eq!(game.counter_count(result.new_id, rule.counter_type()), if positive { 2 } else { 0 },
                    "zero does not become a placement and positive control actually doubles: {rule:?}, {printed:?}");
                assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), !positive,
                    "one-shot consumption requires a real positive placement: {rule:?}, {printed:?}");
                let counter_events = game.take_pending_trigger_events().into_iter()
                    .filter(|event| event.downcast::<crate::events::MarkersChangedEvent>().is_some()).count();
                assert_eq!(counter_events, usize::from(positive),
                    "zero placements publish no counter event: {rule:?}, {printed:?}");
            }
        }
    }
}
