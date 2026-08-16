//! Filter system for selecting objects in the game.
//!
//! This module provides filters for selecting objects (permanents, spells, cards)
//! based on various criteria like card types, colors, power/toughness, etc.
//!
//! Filters are used by:
//! - Target specifications (for spells and abilities that target)
//! - Effect conditions (for effects that affect "all creatures" etc.)
//! - Cost requirements (for sacrifice costs, etc.)
//! - Triggered ability conditions (for triggers that watch for specific events)

use crate::ability::{AbilityKind, ActivatedAbilityRuntimeExt};
use crate::color::{Color, ColorSet};
use crate::continuous::CalculatedCharacteristics;
use crate::events::{CreatureAttackedEvent, MarkersChangedEvent};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object::{CounterType, Object, ObjectKind};
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbilityId;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype, SubtypeFamily, Supertype};
use crate::zone::Zone;
pub use ironsmith_core::filter_model::{
    AlternativeCastKind, Comparison, CounterConstraint, CountersPutOnThisTurnConstraint,
    ExcludedNameSurface, GlobalCharacteristicDomainSurface, ObjectCharacteristic,
    ObjectCharacteristicRelation, ObjectCharacteristicRelationKind, ObjectFilter,
    ObjectFilterUnionConnective, ObjectFilterUnionSurface, ObjectRef, ParityRequirement,
    PlayerFilter, PowerToughnessRelation, PtReference, SameNameAntecedentSurface,
    SourcePowerRelation, StackObjectKind, TaggedObjectConstraint, TaggedOpbjectRelation,
    TargetabilityConstraint,
};

mod descriptions;
pub(crate) use descriptions::describe_player_filter;
use descriptions::*;

#[cfg(test)]
mod tests;

fn ensure_filter_indefinite_article(text: String) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("each ")
        || lower.starts_with("all ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("those ")
        || lower.starts_with("target ")
        || lower.starts_with("any ")
        || lower.starts_with("up to ")
        || lower.starts_with("at least ")
        || lower.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }
    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

fn correct_filter_leading_indefinite_article(text: String) -> String {
    let Some(rest) = text.strip_prefix("a ") else {
        return text;
    };
    if rest
        .chars()
        .next()
        .is_some_and(|first| matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        format!("an {rest}")
    } else {
        text
    }
}

fn describe_conjunctive_filter_members(mut parts: Vec<String>) -> String {
    match parts.as_slice() {
        [] => return String::new(),
        [single] => return single.clone(),
        [first, second] => return format!("{first} and {second}"),
        _ => {}
    }
    let last = parts
        .pop()
        .expect("conjunctive filter has at least three members");
    format!("{}, and {last}", parts.join(", "))
}

fn normalize_name_for_match(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn counters_put_on_exact_object_this_turn(
    game: &GameState,
    object_id: ObjectId,
    constraint: &CountersPutOnThisTurnConstraint,
    ctx: &FilterContext,
) -> u32 {
    game.turn_store
        .turn_history
        .projected_records()
        .filter_map(|record| record.event.downcast::<MarkersChangedEvent>())
        .filter(|event| {
            event.is_added()
                && event.object() == Some(object_id)
                && event.amount > 0
                && constraint
                    .counter_type
                    .is_none_or(|counter_type| event.marker.as_counter() == Some(counter_type))
                && event.source_controller.is_some_and(|source_controller| {
                    constraint
                        .source_controller
                        .matches_player(source_controller, ctx)
                })
        })
        .fold(0u32, |total, event| total.saturating_add(event.amount))
}

pub(crate) fn names_match(lhs: &str, rhs: &str) -> bool {
    lhs.eq_ignore_ascii_case(rhs) || normalize_name_for_match(lhs) == normalize_name_for_match(rhs)
}

fn object_mana_value_for_filter(object: &Object) -> i32 {
    object.mana_cost.as_ref().map_or(0, |mana_cost| {
        if object.zone == Zone::Stack {
            mana_cost.mana_value_with_x(object.x_value.unwrap_or(0)) as i32
        } else {
            mana_cost.mana_value() as i32
        }
    })
}

pub(crate) fn snapshot_mana_value_for_filter(snapshot: &ObjectSnapshot) -> i32 {
    snapshot.mana_cost.as_ref().map_or(0, |mana_cost| {
        if snapshot.zone == Zone::Stack {
            mana_cost.mana_value_with_x(snapshot.x_value.unwrap_or(0)) as i32
        } else {
            mana_cost.mana_value() as i32
        }
    })
}

fn object_is_enlist_eligible(game: &GameState, id: ObjectId) -> bool {
    if game.is_tapped(id) {
        return false;
    }
    if game
        .combat
        .as_ref()
        .is_some_and(|combat| crate::combat_state::is_attacking(combat, id))
    {
        return false;
    }
    !game.is_summoning_sick(id) || game.current_has_static_ability_id(id, StaticAbilityId::Haste)
}

fn stack_spell_cast_origin_zone(
    object: &Object,
    entry: &crate::game_state::StackEntry,
) -> Option<Zone> {
    if entry.is_ability || object.kind == ObjectKind::SpellCopy {
        return None;
    }
    Some(match &entry.casting_method {
        crate::alternative_cast::CastingMethod::Normal
        | crate::alternative_cast::CastingMethod::FaceDown
        | crate::alternative_cast::CastingMethod::SplitOtherHalf
        | crate::alternative_cast::CastingMethod::Fuse => Zone::Hand,
        crate::alternative_cast::CastingMethod::Alternative(index) => object
            .alternative_casts
            .get(*index)
            .map(|method| method.cast_from_zone())
            .unwrap_or(Zone::Hand),
        crate::alternative_cast::CastingMethod::GrantedEscape { .. }
        | crate::alternative_cast::CastingMethod::GrantedFlashback => Zone::Graveyard,
        crate::alternative_cast::CastingMethod::PlayFrom { zone, .. }
        | crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom { zone, .. } => *zone,
    })
}

fn mana_from_matching_source_was_spent_to_cast(
    source_filter: &ObjectFilter,
    sources: &[crate::snapshot::ObjectSnapshot],
    ctx: &FilterContext,
    game: &GameState,
) -> bool {
    sources
        .iter()
        .any(|source| source_filter.matches_snapshot(source, ctx, game))
}

fn first_matching_spell_cast_each_turn_matches(
    filter: &ObjectFilter,
    object_id: ObjectId,
    ctx: &FilterContext,
    game: &GameState,
    fallback_cast_player: Option<PlayerId>,
) -> bool {
    matching_spell_cast_ordinal_each_turn_matches(
        filter,
        1,
        object_id,
        ctx,
        game,
        fallback_cast_player,
    )
}

fn matching_spell_cast_ordinal_each_turn_matches(
    filter: &ObjectFilter,
    ordinal: u32,
    object_id: ObjectId,
    ctx: &FilterContext,
    game: &GameState,
    fallback_cast_player: Option<PlayerId>,
) -> bool {
    let mut matching_filter = filter.clone();
    matching_filter.first_spell_cast_each_turn = false;
    matching_filter.spell_cast_ordinal_each_turn = None;

    let cast_origin_zone = matching_filter.zone.filter(|zone| *zone != Zone::Stack);
    let excluded_cast_origin_zone = matching_filter.excluded_cast_origin_zone.take();
    if cast_origin_zone.is_some() {
        // Historical cast snapshots describe the spell on the stack. The
        // authored origin is carried by SpellCastEvent::from_zone, so match it
        // there and use Stack only for the remaining structural constraints.
        matching_filter.zone = Some(Zone::Stack);
    }

    // Continuous characteristics can be queried while a spell is being put
    // onto the stack, before its SpellCastEvent has reached turn history. In
    // that window the generic cast-order fallback cannot distinguish "first
    // spell" from "first matching spell" (for example, first from exile).
    // Evaluate the live stack entry authoritatively so that a result cached
    // during casting remains correct after the event is recorded.
    let current_live_match = game.object(object_id).and_then(|object| {
        let entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == object_id)?;
        let origin = stack_spell_cast_origin_zone(object, entry)?;
        if cast_origin_zone.is_some_and(|zone| origin != zone)
            || excluded_cast_origin_zone.is_some_and(|zone| origin == zone)
        {
            return Some(false);
        }
        let mut live_ctx = ctx.clone();
        live_ctx.caster = Some(entry.controller);
        Some(matching_filter.matches_non_recursive(object, &live_ctx, game))
    });

    let mut saw_current_spell_cast = false;
    let mut matching_ordinal = 0u32;
    for record in game.turn_store.turn_history.projected_records() {
        let Some(event) = record
            .event
            .downcast::<crate::events::spells::SpellCastEvent>()
        else {
            continue;
        };
        if event.spell == object_id {
            saw_current_spell_cast = true;
        }
        if cast_origin_zone.is_some_and(|zone| event.from_zone != zone) {
            continue;
        }
        if excluded_cast_origin_zone.is_some_and(|zone| event.from_zone == zone) {
            continue;
        }
        let Some(snapshot) = record.object_snapshot.as_ref() else {
            continue;
        };
        let mut history_ctx = ctx.clone();
        history_ctx.caster = Some(event.caster);
        if matching_filter.matches_snapshot(snapshot, &history_ctx, game) {
            matching_ordinal = matching_ordinal.saturating_add(1);
            if event.spell == object_id {
                return matching_ordinal == ordinal;
            }
        }
    }

    // If the current spell has an explicit history record, reaching this
    // point means it failed one of the authored matching constraints (most
    // commonly the origin zone or caster). Do not reinterpret "first" as the
    // first spell of any kind via the generic cast-order fallback below.
    if saw_current_spell_cast {
        return false;
    }

    if let Some(current_live_match) = current_live_match {
        return current_live_match && matching_ordinal.saturating_add(1) == ordinal;
    }

    let cast_order = fallback_cast_player
        .and_then(|player| {
            game.turn_store
                .turn_history
                .spell_cast_order_for_player(object_id, player)
        })
        .or_else(|| game.turn_store.turn_history.spell_cast_order(object_id));
    cast_order == Some(ordinal)
}

pub(crate) trait TaggedConstraintSubject {
    fn subject_object_id(&self) -> ObjectId;
    fn subject_stable_id(&self) -> StableId;
    fn subject_name(&self) -> &str;
    fn subject_controller(&self) -> PlayerId;
    fn subject_card_types(&self) -> &[CardType];
    fn subject_subtypes(&self) -> &[Subtype];
    fn subject_colors(&self) -> ColorSet;
    fn subject_mana_value(&self) -> i32;
    fn subject_attached_to(&self) -> Option<ObjectId>;
    fn subject_attached_to_player(&self) -> Option<PlayerId>;
    fn subject_attachments(&self) -> &[ObjectId];
    fn subject_was_enchanted(&self) -> bool;
}

pub(crate) trait TailMatchSubject: TaggedConstraintSubject {
    fn tail_object_id(&self) -> ObjectId;
    fn tail_name(&self) -> &str;
    fn tail_first_printed_set_name(&self) -> Option<&str>;
    fn tail_counters(&self) -> &std::collections::HashMap<CounterType, u32>;
    fn tail_abilities(&self) -> &[crate::ability::Ability];
    fn tail_has_alternative_cast_kind(
        &self,
        kind: AlternativeCastKind,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
    ) -> bool;
    fn tail_has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool;
    fn tail_has_ability_marker(&self, marker: &str) -> bool;
    fn tail_has_tap_activated_ability(&self) -> bool;
    fn tail_is_commander(&self, game: &crate::game_state::GameState) -> bool;
}

impl TaggedConstraintSubject for Object {
    fn subject_object_id(&self) -> ObjectId {
        self.id
    }

    fn subject_stable_id(&self) -> StableId {
        self.stable_id
    }

    fn subject_name(&self) -> &str {
        &self.name
    }

    fn subject_controller(&self) -> PlayerId {
        self.owner
    }

    fn subject_card_types(&self) -> &[CardType] {
        &self.card_types
    }

    fn subject_subtypes(&self) -> &[Subtype] {
        &self.subtypes
    }

    fn subject_colors(&self) -> ColorSet {
        self.colors()
    }

    fn subject_mana_value(&self) -> i32 {
        object_mana_value_for_filter(self)
    }

    fn subject_attached_to(&self) -> Option<ObjectId> {
        self.attached_to.and_then(|target| target.object_id())
    }

    fn subject_attached_to_player(&self) -> Option<PlayerId> {
        self.attached_to.and_then(|target| target.player_id())
    }

    fn subject_attachments(&self) -> &[ObjectId] {
        &self.attachments
    }

    fn subject_was_enchanted(&self) -> bool {
        false
    }
}

impl TailMatchSubject for Object {
    fn tail_object_id(&self) -> ObjectId {
        self.id
    }

    fn tail_name(&self) -> &str {
        &self.name
    }

    fn tail_first_printed_set_name(&self) -> Option<&str> {
        self.first_printed_set_name.as_deref()
    }

    fn tail_counters(&self) -> &std::collections::HashMap<CounterType, u32> {
        &self.counters
    }

    fn tail_abilities(&self) -> &[crate::ability::Ability] {
        &self.abilities
    }

    fn tail_has_alternative_cast_kind(
        &self,
        kind: AlternativeCastKind,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
    ) -> bool {
        object_has_alternative_cast_kind(self, kind, game, ctx)
    }

    fn tail_has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool {
        object_has_static_ability_id(self, ability_id)
    }

    fn tail_has_ability_marker(&self, marker: &str) -> bool {
        object_has_ability_marker(self, marker)
    }

    fn tail_has_tap_activated_ability(&self) -> bool {
        object_has_tap_activated_ability(self)
    }

    fn tail_is_commander(&self, game: &crate::game_state::GameState) -> bool {
        game.is_commander(self.id)
    }
}

pub(crate) struct LayeredSubject<'a> {
    pub object: &'a Object,
    pub chars: &'a CalculatedCharacteristics,
}

impl TaggedConstraintSubject for LayeredSubject<'_> {
    fn subject_object_id(&self) -> ObjectId {
        self.object.id
    }

    fn subject_stable_id(&self) -> StableId {
        self.object.stable_id
    }

    fn subject_name(&self) -> &str {
        self.chars.name.as_str()
    }

    fn subject_controller(&self) -> PlayerId {
        self.chars.controller
    }

    fn subject_card_types(&self) -> &[CardType] {
        self.chars.card_types.as_slice()
    }

    fn subject_subtypes(&self) -> &[Subtype] {
        self.chars.subtypes.as_slice()
    }

    fn subject_colors(&self) -> ColorSet {
        self.chars.colors
    }

    fn subject_mana_value(&self) -> i32 {
        object_mana_value_for_filter(self.object)
    }

    fn subject_attached_to(&self) -> Option<ObjectId> {
        self.object
            .attached_to
            .and_then(|target| target.object_id())
    }

    fn subject_attached_to_player(&self) -> Option<PlayerId> {
        self.object
            .attached_to
            .and_then(|target| target.player_id())
    }

    fn subject_attachments(&self) -> &[ObjectId] {
        &self.object.attachments
    }

    fn subject_was_enchanted(&self) -> bool {
        false
    }
}

impl TailMatchSubject for LayeredSubject<'_> {
    fn tail_object_id(&self) -> ObjectId {
        self.object.id
    }

    fn tail_name(&self) -> &str {
        self.chars.name.as_str()
    }

    fn tail_first_printed_set_name(&self) -> Option<&str> {
        if self.chars.name.as_str() == self.object.name.as_ref() {
            self.object.first_printed_set_name.as_deref()
        } else {
            None
        }
    }

    fn tail_counters(&self) -> &std::collections::HashMap<CounterType, u32> {
        &self.object.counters
    }

    fn tail_abilities(&self) -> &[crate::ability::Ability] {
        self.chars.abilities.as_slice()
    }

    fn tail_has_alternative_cast_kind(
        &self,
        kind: AlternativeCastKind,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
    ) -> bool {
        object_has_alternative_cast_kind(self.object, kind, game, ctx)
    }

    fn tail_has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool {
        self.chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == ability_id)
    }

    fn tail_has_ability_marker(&self, marker: &str) -> bool {
        object_has_ability_marker(self.object, marker)
            || aura_attachment_has_ability_marker(self.chars.aura_attach_filter.as_ref(), marker)
            || abilities_have_marker(&self.chars.abilities, marker)
    }

    fn tail_has_tap_activated_ability(&self) -> bool {
        abilities_have_tap_activated_ability(&self.chars.abilities)
    }

    fn tail_is_commander(&self, game: &crate::game_state::GameState) -> bool {
        game.is_commander(self.object.id)
    }
}

impl TaggedConstraintSubject for ObjectSnapshot {
    fn subject_object_id(&self) -> ObjectId {
        self.object_id
    }

    fn subject_stable_id(&self) -> StableId {
        self.stable_id
    }

    fn subject_name(&self) -> &str {
        &self.name
    }

    fn subject_controller(&self) -> PlayerId {
        self.controller
    }

    fn subject_card_types(&self) -> &[CardType] {
        &self.card_types
    }

    fn subject_subtypes(&self) -> &[Subtype] {
        &self.subtypes
    }

    fn subject_colors(&self) -> ColorSet {
        self.colors
    }

    fn subject_mana_value(&self) -> i32 {
        snapshot_mana_value_for_filter(self)
    }

    fn subject_attached_to(&self) -> Option<ObjectId> {
        self.attached_to.and_then(|target| target.object_id())
    }

    fn subject_attached_to_player(&self) -> Option<PlayerId> {
        self.attached_to.and_then(|target| target.player_id())
    }

    fn subject_attachments(&self) -> &[ObjectId] {
        &self.attachments
    }

    fn subject_was_enchanted(&self) -> bool {
        self.was_enchanted
    }
}

impl TailMatchSubject for ObjectSnapshot {
    fn tail_object_id(&self) -> ObjectId {
        self.object_id
    }

    fn tail_name(&self) -> &str {
        &self.name
    }

    fn tail_first_printed_set_name(&self) -> Option<&str> {
        self.first_printed_set_name.as_deref()
    }

    fn tail_counters(&self) -> &std::collections::HashMap<CounterType, u32> {
        &self.counters
    }

    fn tail_abilities(&self) -> &[crate::ability::Ability] {
        &self.abilities
    }

    fn tail_has_alternative_cast_kind(
        &self,
        kind: AlternativeCastKind,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
    ) -> bool {
        game.object(self.object_id)
            .is_some_and(|obj| object_has_alternative_cast_kind(obj, kind, game, ctx))
    }

    fn tail_has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool {
        snapshot_has_static_ability_id(self, ability_id)
    }

    fn tail_has_ability_marker(&self, marker: &str) -> bool {
        snapshot_has_ability_marker(self, marker)
    }

    fn tail_has_tap_activated_ability(&self) -> bool {
        snapshot_has_tap_activated_ability(self)
    }

    fn tail_is_commander(&self, _game: &crate::game_state::GameState) -> bool {
        self.is_commander
    }
}

fn subject_has_attached_subtype(
    subject: &impl TaggedConstraintSubject,
    subtype: Subtype,
    game: &GameState,
) -> bool {
    subject.subject_attachments().iter().any(|attachment_id| {
        game.object(*attachment_id)
            .is_some_and(|attachment| attachment.subtypes.contains(&subtype))
    })
}

fn subject_could_produce_any_mana_symbol(
    subject: &impl TailMatchSubject,
    required: &[crate::mana::ManaSymbol],
    game: &GameState,
) -> bool {
    subject.tail_abilities().iter().any(|ability| {
        let AbilityKind::Activated(activated) = &ability.kind else {
            return false;
        };
        let produced = activated.inferred_mana_symbols(
            game,
            subject.tail_object_id(),
            subject.subject_controller(),
        );
        required.iter().any(|symbol| produced.contains(symbol))
    })
}

fn linked_face_has_adventure(
    game: &GameState,
    name: Option<&str>,
    id: Option<crate::ids::CardId>,
) -> bool {
    game.linked_face_definition_by_name_or_id(name, id)
        .is_some_and(|def| def.card.subtypes.contains(&Subtype::Adventure))
}

fn object_matches_subtype(object: &Object, subtype: Subtype, game: &GameState) -> bool {
    object.subtypes.contains(&subtype)
        || (subtype == Subtype::Adventure
            && linked_face_has_adventure(
                game,
                object.other_face_name.as_deref(),
                object.other_face,
            ))
}

fn layered_subject_matches_subtype(
    subject: &LayeredSubject<'_>,
    subtype: Subtype,
    game: &GameState,
) -> bool {
    subject.chars.subtypes.contains(&subtype)
        || (subtype == Subtype::Adventure
            && linked_face_has_adventure(
                game,
                subject.object.other_face_name.as_deref(),
                subject.object.other_face,
            ))
}

fn filter_card_types<'a>(
    object: &'a Object,
    chars: Option<&'a CalculatedCharacteristics>,
) -> &'a [CardType] {
    chars.map_or(&object.card_types, |chars| chars.card_types.as_slice())
}

fn filter_subtypes<'a>(
    object: &'a Object,
    chars: Option<&'a CalculatedCharacteristics>,
) -> &'a [Subtype] {
    chars.map_or(&object.subtypes, |chars| chars.subtypes.as_slice())
}

fn filter_supertypes<'a>(
    object: &'a Object,
    chars: Option<&'a CalculatedCharacteristics>,
) -> &'a [Supertype] {
    chars.map_or(&object.supertypes, |chars| chars.supertypes.as_slice())
}

fn filter_colors(object: &Object, chars: Option<&CalculatedCharacteristics>) -> ColorSet {
    chars.map_or_else(|| object.colors(), |chars| chars.colors)
}

fn filter_subject_matches_subtype(
    object: &Object,
    subject: Option<&LayeredSubject<'_>>,
    subtype: Subtype,
    game: &GameState,
) -> bool {
    subject.map_or_else(
        || object_matches_subtype(object, subtype, game),
        |subject| layered_subject_matches_subtype(subject, subtype, game),
    )
}

fn filter_object_has_subtype_with_view(
    object: &Object,
    subtype: Subtype,
    allow_calculated_pt: bool,
    view: Option<&crate::derived_view::DerivedGameView<'_>>,
    game: &GameState,
) -> bool {
    if allow_calculated_pt
        && object.zone == Zone::Battlefield
        && view.is_none_or(|view| view.requires_battlefield_characteristic_calculation(object.id))
    {
        let chars = view
            .and_then(|view| view.calculated_characteristics_arc(object.id))
            .or_else(|| game.calculated_characteristics_arc(object.id));
        if let Some(chars) = chars {
            return chars.subtypes.contains(&subtype)
                || (subtype == Subtype::Adventure
                    && linked_face_has_adventure(
                        game,
                        object.other_face_name.as_deref(),
                        object.other_face,
                    ));
        }
    }
    object_matches_subtype(object, subtype, game)
}

fn snapshot_matches_subtype(snapshot: &ObjectSnapshot, subtype: Subtype, game: &GameState) -> bool {
    snapshot.subtypes.contains(&subtype)
        || (subtype == Subtype::Adventure
            && linked_face_has_adventure(
                game,
                snapshot.other_face_name.as_deref(),
                snapshot.other_face,
            ))
}

fn subject_creature_subtypes(
    subject: &impl TaggedConstraintSubject,
    game: &GameState,
) -> Vec<Subtype> {
    game.current_subtypes(subject.subject_object_id())
        .unwrap_or_else(|| subject.subject_subtypes().to_vec())
        .into_iter()
        .filter(|subtype| subtype.is_creature_type())
        .collect()
}

fn object_creature_subtypes(object: &Object, game: &GameState) -> Vec<Subtype> {
    game.current_subtypes(object.id)
        .unwrap_or_else(|| object.subtypes.to_vec())
        .into_iter()
        .filter(|subtype| subtype.is_creature_type())
        .collect()
}

fn subject_shares_creature_type_with_filter(
    subject: &impl TaggedConstraintSubject,
    comparison_filter: &ObjectFilter,
    ctx: &FilterContext,
    game: &GameState,
) -> bool {
    let subject_subtypes = subject_creature_subtypes(subject, game);
    if subject_subtypes.is_empty() {
        return false;
    }

    for object in game.objects_in_deterministic_order() {
        if object.id == subject.subject_object_id() {
            continue;
        }
        if !comparison_filter.matches(object, ctx, game) {
            continue;
        }
        let object_subtypes = object_creature_subtypes(object, game);
        if subject_subtypes
            .iter()
            .any(|subtype| object_subtypes.contains(subtype))
        {
            return true;
        }
    }

    false
}

fn subject_subtypes_in_family(
    subject: &impl TaggedConstraintSubject,
    family: SubtypeFamily,
) -> Vec<Subtype> {
    subject
        .subject_subtypes()
        .iter()
        .copied()
        .filter(|subtype| family.all_subtypes().contains(subtype))
        .collect()
}

fn object_subtypes_in_family(
    object: &Object,
    family: SubtypeFamily,
    game: &GameState,
) -> Vec<Subtype> {
    game.current_subtypes(object.id)
        .unwrap_or_else(|| object.subtypes.to_vec())
        .into_iter()
        .filter(|subtype| family.all_subtypes().contains(subtype))
        .collect()
}

fn object_current_mana_value_for_relation(object: &Object, game: &GameState) -> i32 {
    let mana_cost = game
        .current_characteristics(object.id)
        .and_then(|characteristics| characteristics.mana_cost.clone())
        .or_else(|| object.mana_cost.as_deref().cloned());
    mana_cost.map_or(0, |mana_cost| {
        if object.zone == Zone::Stack {
            mana_cost.mana_value_with_x(object.x_value.unwrap_or(0)) as i32
        } else {
            mana_cost.mana_value() as i32
        }
    })
}

fn subject_shares_characteristic_with_object(
    subject: &impl TaggedConstraintSubject,
    object: &Object,
    characteristic: ObjectCharacteristic,
    game: &GameState,
) -> bool {
    match characteristic {
        ObjectCharacteristic::CardType | ObjectCharacteristic::PermanentType => {
            let object_types = game
                .current_card_types(object.id)
                .unwrap_or_else(|| object.card_types.to_vec());
            subject
                .subject_card_types()
                .iter()
                .any(|card_type| object_types.contains(card_type))
        }
        ObjectCharacteristic::Subtype(family) => {
            let subject_subtypes = subject_subtypes_in_family(subject, family);
            let object_subtypes = object_subtypes_in_family(object, family, game);
            subject_subtypes
                .iter()
                .any(|subtype| object_subtypes.contains(subtype))
        }
        ObjectCharacteristic::Color => {
            let object_colors = game
                .current_colors(object.id)
                .unwrap_or_else(|| object.colors());
            !subject
                .subject_colors()
                .intersection(object_colors)
                .is_empty()
        }
        ObjectCharacteristic::ManaValue => {
            subject.subject_mana_value() == object_current_mana_value_for_relation(object, game)
        }
    }
}

fn characteristic_relation_matches_subject(
    subject: &impl TaggedConstraintSubject,
    relation: &ObjectCharacteristicRelation,
    ctx: &FilterContext,
    game: &GameState,
) -> bool {
    let shares = game
        .objects_in_deterministic_order()
        .into_iter()
        .any(|object| {
            relation.comparison.matches(object, ctx, game)
                && relation.characteristics.iter().any(|characteristic| {
                    subject_shares_characteristic_with_object(
                        subject,
                        object,
                        *characteristic,
                        game,
                    )
                })
        });
    match relation.kind {
        ObjectCharacteristicRelationKind::SharesAny => shares,
        ObjectCharacteristicRelationKind::SharesNone => !shares,
    }
}

fn subject_shares_creature_type_with_source(
    subject: &impl TaggedConstraintSubject,
    ctx: &FilterContext,
    game: &GameState,
) -> bool {
    let Some(source_id) = ctx.source else {
        return false;
    };
    let Some(source) = game.object(source_id) else {
        return false;
    };
    let subject_subtypes = subject_creature_subtypes(subject, game);
    if subject_subtypes.is_empty() {
        return false;
    }
    let source_subtypes = object_creature_subtypes(source, game);
    subject_subtypes
        .iter()
        .any(|subtype| source_subtypes.contains(subtype))
}

fn intrinsic_attachment_tag_constraint_matches_subject(
    subject: &impl TaggedConstraintSubject,
    tag: &TagKey,
    relation: TaggedOpbjectRelation,
    game: &GameState,
) -> Option<bool> {
    let matches_intrinsic = match tag.as_str() {
        "equipped" => subject_has_attached_subtype(subject, Subtype::Equipment, game),
        "enchanted" => {
            subject.subject_was_enchanted()
                || subject_has_attached_subtype(subject, Subtype::Aura, game)
        }
        _ => return None,
    };

    match relation {
        TaggedOpbjectRelation::IsTaggedObject => Some(matches_intrinsic),
        TaggedOpbjectRelation::IsNotTaggedObject => Some(!matches_intrinsic),
        _ => None,
    }
}

fn tagged_constraint_matches_subject(
    subject: &impl TaggedConstraintSubject,
    tagged_snapshots: &[ObjectSnapshot],
    relation: TaggedOpbjectRelation,
    game: &GameState,
) -> bool {
    match relation {
        TaggedOpbjectRelation::IsTaggedObject
        | TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered => {
            tagged_snapshots.iter().any(|snapshot| {
                snapshot.object_id == subject.subject_object_id()
                    || snapshot.stable_id == subject.subject_stable_id()
            })
        }
        TaggedOpbjectRelation::SharesCardType | TaggedOpbjectRelation::SharesPermanentType => {
            let tagged_types: std::collections::HashSet<CardType> = tagged_snapshots
                .iter()
                .flat_map(|snapshot| snapshot.card_types.iter().copied())
                .collect();
            subject
                .subject_card_types()
                .iter()
                .any(|card_type| tagged_types.contains(card_type))
        }
        TaggedOpbjectRelation::SharesSubtypeWithTagged => {
            let tagged_subtypes: std::collections::HashSet<Subtype> = tagged_snapshots
                .iter()
                .flat_map(|snapshot| snapshot.subtypes.iter().copied())
                .collect();
            subject
                .subject_subtypes()
                .iter()
                .any(|subtype| tagged_subtypes.contains(subtype))
        }
        TaggedOpbjectRelation::SharesSubtypeWithEachTagged => {
            !tagged_snapshots.is_empty()
                && tagged_snapshots.iter().all(|snapshot| {
                    subject
                        .subject_subtypes()
                        .iter()
                        .any(|subtype| snapshot.subtypes.contains(subtype))
                })
        }
        TaggedOpbjectRelation::SharesColorWithTagged => tagged_snapshots.iter().any(|snapshot| {
            !subject
                .subject_colors()
                .intersection(snapshot.colors)
                .is_empty()
        }),
        TaggedOpbjectRelation::SharesMostCommonPermanentColor => {
            subject_shares_most_common_permanent_color(subject, game)
        }
        TaggedOpbjectRelation::SameStableId => tagged_snapshots
            .iter()
            .any(|snapshot| snapshot.stable_id == subject.subject_stable_id()),
        TaggedOpbjectRelation::SameNameAsTagged => tagged_snapshots
            .iter()
            .any(|snapshot| names_match(&snapshot.name, subject.subject_name())),
        TaggedOpbjectRelation::DifferentNameFromTagged => tagged_snapshots
            .iter()
            .all(|snapshot| !names_match(&snapshot.name, subject.subject_name())),
        TaggedOpbjectRelation::SameControllerAsTagged => tagged_snapshots
            .iter()
            .any(|snapshot| snapshot.controller == subject.subject_controller()),
        TaggedOpbjectRelation::SameManaValueAsTagged => tagged_snapshots.iter().any(|snapshot| {
            snapshot_mana_value_for_filter(snapshot) == subject.subject_mana_value()
        }),
        TaggedOpbjectRelation::SameManaValueAsAnotherTagged => {
            tagged_snapshots.iter().any(|snapshot| {
                snapshot.stable_id != subject.subject_stable_id()
                    && snapshot_mana_value_for_filter(snapshot) == subject.subject_mana_value()
            })
        }
        TaggedOpbjectRelation::ManaValueLteTagged => tagged_snapshots.iter().any(|snapshot| {
            subject.subject_mana_value() <= snapshot_mana_value_for_filter(snapshot)
        }),
        TaggedOpbjectRelation::ManaValueLtTagged => tagged_snapshots.iter().any(|snapshot| {
            subject.subject_mana_value() < snapshot_mana_value_for_filter(snapshot)
        }),
        TaggedOpbjectRelation::AttachedToTaggedObject => tagged_snapshots
            .iter()
            .any(|snapshot| subject.subject_attached_to() == Some(snapshot.object_id)),
        TaggedOpbjectRelation::WasAttachedToTaggedObject => tagged_snapshots
            .iter()
            .any(|snapshot| snapshot.attachments.contains(&subject.subject_object_id())),
        TaggedOpbjectRelation::SoulbondPartnerOfTagged => tagged_snapshots.iter().any(|snapshot| {
            game.soulbond_partner(snapshot.object_id) == Some(subject.subject_object_id())
        }),
        TaggedOpbjectRelation::IsNotTaggedObject => tagged_snapshots
            .iter()
            .all(|snapshot| snapshot.object_id != subject.subject_object_id()),
    }
}

fn most_common_permanent_colors(game: &GameState) -> ColorSet {
    let mut counts = [0u32; 5];
    for object_id in game.zone_ids(Zone::Battlefield) {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        let colors = game
            .current_colors(object_id)
            .unwrap_or_else(|| object.colors());
        for (idx, color) in Color::ALL.into_iter().enumerate() {
            if colors.contains(color) {
                counts[idx] += 1;
            }
        }
    }

    let max_count = counts.into_iter().max().unwrap_or(0);
    if max_count == 0 {
        return ColorSet::new();
    }

    Color::ALL
        .into_iter()
        .enumerate()
        .filter_map(|(idx, color)| (counts[idx] == max_count).then_some(color))
        .collect()
}

fn subject_shares_most_common_permanent_color(
    subject: &impl TaggedConstraintSubject,
    game: &GameState,
) -> bool {
    let most_common_colors = most_common_permanent_colors(game);
    let subject_colors = game
        .current_colors(subject.subject_object_id())
        .unwrap_or_else(|| subject.subject_colors());
    !most_common_colors.is_empty() && !subject_colors.intersection(most_common_colors).is_empty()
}

// ============================================================================
// Object Reference (for cross-effect tagging)
// ============================================================================

/// Context needed for evaluating filters.
///
/// Provides information about "you" (the controller), the source object,
/// active player, and other contextual details.
#[derive(Debug, Clone, Default)]
pub struct FilterContext {
    /// The controller of the source ability ("you")
    pub you: Option<PlayerId>,

    /// The source object of the ability
    pub source: Option<ObjectId>,

    /// Last known source characteristics when the source left its zone while
    /// paying a cost or during resolution.
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,

    /// The player casting the spell currently being evaluated, if any.
    pub caster: Option<PlayerId>,

    /// The active player (whose turn it is)
    pub active_player: Option<PlayerId>,

    /// Players who are opponents of "you"
    pub opponents: Vec<PlayerId>,

    /// Players who are teammates of "you" (for team games)
    pub teammates: Vec<PlayerId>,

    /// Frozen CR 801.2c membership for this source/controller. `None` means
    /// unlimited range or an exempt Planechase source.
    pub players_in_range: Option<Vec<PlayerId>>,

    /// The defending player (in combat)
    pub defending_player: Option<PlayerId>,

    /// Candidate defending-team players before CR 805.10e selects one.
    pub defending_players: Vec<PlayerId>,

    /// The attacking player (in combat)
    pub attacking_player: Option<PlayerId>,

    /// Candidate attacking-team players before CR 805.10c selects one.
    pub attacking_players: Vec<PlayerId>,

    /// Commander IDs controlled by "you" (for Commander format)
    pub your_commanders: Vec<ObjectId>,

    /// The current iterated player (for ForEachOpponent/ForEachPlayer effects)
    pub iterated_player: Option<PlayerId>,

    /// X value carried by the current resolving spell or ability, if any.
    pub x_value: Option<u32>,

    /// The player chosen for the source permanent or spell, if any.
    pub chosen_player: Option<PlayerId>,

    /// Resolved player targets from the current execution context.
    pub target_players: Vec<PlayerId>,

    /// Resolved object targets from the current execution context.
    ///
    /// Stored as snapshots so target-dependent controller/owner filters continue
    /// to work after the target has changed zones.
    pub target_objects: Vec<crate::snapshot::ObjectSnapshot>,

    /// Tagged objects from prior effects in the same spell/ability.
    /// Used by tag-aware object filter constraints.
    pub tagged_objects: std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,

    /// Tagged players from prior effects in the same spell/ability.
    pub tagged_players: std::collections::HashMap<TagKey, Vec<PlayerId>>,

    /// Outcomes from prior effects in the same spell/ability.
    pub effect_outcomes:
        std::collections::HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
}

impl FilterContext {
    /// Create a new context with the controller specified.
    pub fn new(you: PlayerId) -> Self {
        Self {
            you: Some(you),
            ..Default::default()
        }
    }

    /// Set the source object.
    pub fn with_source(mut self, source: ObjectId) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_source_snapshot(
        mut self,
        snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> Self {
        self.source_snapshot = snapshot;
        self
    }

    /// Set the caster for cast-context filter evaluation.
    pub fn with_caster(mut self, caster: Option<PlayerId>) -> Self {
        self.caster = caster;
        self
    }

    /// Set the active player.
    pub fn with_active_player(mut self, active: PlayerId) -> Self {
        self.active_player = Some(active);
        self
    }

    /// Set the opponents.
    pub fn with_opponents(mut self, opponents: Vec<PlayerId>) -> Self {
        self.opponents = opponents;
        self
    }

    /// Set your commanders (for Commander format filtering).
    pub fn with_your_commanders(mut self, commanders: Vec<ObjectId>) -> Self {
        self.your_commanders = commanders;
        self
    }

    /// Set the iterated player (for ForEachOpponent/ForEachPlayer effects).
    pub fn with_iterated_player(mut self, player: Option<PlayerId>) -> Self {
        self.iterated_player = player;
        self
    }

    /// Set the current X value for dynamic comparisons in filters.
    pub fn with_x_value(mut self, x_value: Option<u32>) -> Self {
        self.x_value = x_value;
        self
    }

    /// Set the chosen player for the source, if any.
    pub fn with_chosen_player(mut self, player: Option<PlayerId>) -> Self {
        self.chosen_player = player;
        self
    }

    /// Set resolved player targets from the execution context.
    pub fn with_target_players(mut self, players: Vec<PlayerId>) -> Self {
        self.target_players = players;
        self
    }

    /// Set resolved object targets from the execution context.
    pub fn with_target_objects(mut self, objects: Vec<crate::snapshot::ObjectSnapshot>) -> Self {
        self.target_objects = objects;
        self
    }

    /// Set tagged objects from the execution context.
    pub fn with_tagged_objects(
        mut self,
        tagged: &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects.extend(tagged.clone());
        self
    }

    /// Set tagged players from the execution context.
    pub fn with_tagged_players(
        mut self,
        tagged: &std::collections::HashMap<TagKey, Vec<PlayerId>>,
    ) -> Self {
        self.tagged_players.extend(tagged.clone());
        self
    }

    /// Set prior effect outcomes from the execution context.
    pub fn with_effect_outcomes(
        mut self,
        outcomes: &std::collections::HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
    ) -> Self {
        self.effect_outcomes.extend(outcomes.clone());
        self
    }
}

pub(crate) trait ComparisonRuntimeExt {
    fn satisfies_with_context(
        &self,
        value: i32,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
        stack_entry: Option<&crate::game_state::StackEntry>,
    ) -> bool;
}

impl ComparisonRuntimeExt for Comparison {
    fn satisfies_with_context(
        &self,
        value: i32,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
        stack_entry: Option<&crate::game_state::StackEntry>,
    ) -> bool {
        match self {
            Comparison::EqualExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| value == rhs)
            }
            Comparison::NotEqualExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| value != rhs)
            }
            Comparison::LessThanExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| value < rhs)
            }
            Comparison::LessThanOrEqualExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| value <= rhs)
            }
            Comparison::GreaterThanExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| value > rhs)
            }
            Comparison::GreaterThanOrEqualExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| value >= rhs)
            }
            _ => self.satisfies(value),
        }
    }
}

trait ParityRequirementRuntimeExt {
    fn resolve(self, game: &crate::game_state::GameState, source: Option<ObjectId>) -> Option<Self>
    where
        Self: Sized;

    fn matches(self, value: i32, game: &crate::game_state::GameState, ctx: &FilterContext) -> bool;
}

impl ParityRequirementRuntimeExt for ParityRequirement {
    fn resolve(
        self,
        game: &crate::game_state::GameState,
        source: Option<ObjectId>,
    ) -> Option<Self> {
        match self {
            Self::Odd | Self::Even => Some(self),
            Self::Chosen => {
                let source = source?;
                let chosen = game.chosen_named_option(source)?;
                if chosen.eq_ignore_ascii_case("odd") {
                    Some(Self::Odd)
                } else if chosen.eq_ignore_ascii_case("even") {
                    Some(Self::Even)
                } else {
                    None
                }
            }
        }
    }

    fn matches(self, value: i32, game: &crate::game_state::GameState, ctx: &FilterContext) -> bool {
        match self.resolve(game, ctx.source) {
            Some(Self::Odd) => value.rem_euclid(2) == 1,
            Some(Self::Even) => value.rem_euclid(2) == 0,
            Some(Self::Chosen) | None => false,
        }
    }
}

fn resolve_filter_comparison_rhs_value(
    rhs: &crate::effect::Value,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
    stack_entry: Option<&crate::game_state::StackEntry>,
) -> Option<i32> {
    use crate::effect::Value;
    use crate::target::ChooseSpec;

    fn total_counters(counters: &std::collections::HashMap<CounterType, u32>) -> i32 {
        counters.values().copied().sum::<u32>() as i32
    }

    fn resolve_x_value(
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
        stack_entry: Option<&crate::game_state::StackEntry>,
    ) -> Option<i32> {
        ctx.x_value
            .or_else(|| stack_entry.and_then(|entry| entry.x_value))
            .or_else(|| {
                ctx.source.and_then(|source| {
                    game.stack
                        .iter()
                        .find(|entry| entry.object_id == source)
                        .and_then(|entry| entry.x_value)
                })
            })
            .or_else(|| {
                ctx.source
                    .and_then(|source| game.object(source).and_then(|object| object.x_value))
            })
            .map(|value| value as i32)
    }

    fn snapshot_pt(snapshot: &ObjectSnapshot, power: bool) -> Option<i32> {
        if power {
            snapshot.power
        } else {
            snapshot.toughness
        }
    }

    fn current_object_pt(
        game: &crate::game_state::GameState,
        object_id: ObjectId,
        power: bool,
    ) -> Option<i32> {
        let object = game.object(object_id)?;
        if power {
            game.calculated_power(object_id).or_else(|| object.power())
        } else {
            game.calculated_toughness(object_id)
                .or_else(|| object.toughness())
        }
    }

    fn resolve_pt_choose_spec(
        spec: &ChooseSpec,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
        power: bool,
    ) -> Option<i32> {
        match spec.base() {
            ChooseSpec::Source => current_object_pt(game, ctx.source?, power),
            ChooseSpec::SpecificObject(object_id) => current_object_pt(game, *object_id, power),
            ChooseSpec::Tagged(tag) => ctx
                .tagged_objects
                .get(tag)
                .and_then(|snapshots| snapshots.first())
                .and_then(|snapshot| snapshot_pt(snapshot, power)),
            ChooseSpec::Object(_) | ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget
                if spec.is_target() =>
            {
                ctx.target_objects
                    .first()
                    .and_then(|snapshot| snapshot_pt(snapshot, power))
            }
            _ => None,
        }
    }

    fn aggregate_tagged_snapshots<'a>(
        filter: &ObjectFilter,
        ctx: &'a FilterContext,
    ) -> Option<Vec<&'a ObjectSnapshot>> {
        let only_is_tagged_constraints = !filter.tagged_constraints.is_empty()
            && filter
                .tagged_constraints
                .iter()
                .all(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject);
        if !only_is_tagged_constraints {
            return None;
        }

        let mut seen = std::collections::HashSet::new();
        let mut snapshots = Vec::new();
        for constraint in &filter.tagged_constraints {
            let Some(tagged) = ctx.tagged_objects.get(&constraint.tag) else {
                continue;
            };
            for snapshot in tagged {
                if seen.insert(snapshot.object_id) {
                    snapshots.push(snapshot);
                }
            }
        }
        Some(snapshots)
    }

    fn aggregate_pt(
        filter: &ObjectFilter,
        game: &GameState,
        ctx: &FilterContext,
        power: bool,
        greatest: bool,
    ) -> Option<i32> {
        if let Some(snapshots) = aggregate_tagged_snapshots(filter, ctx) {
            let values = snapshots
                .into_iter()
                .filter(|snapshot| filter.matches_snapshot(snapshot, ctx, game))
                .filter_map(|snapshot| snapshot_pt(snapshot, power));
            return if greatest { values.max() } else { values.min() };
        }

        let values = game
            .objects_in_deterministic_order()
            .into_iter()
            .filter(|object| filter.matches(object, ctx, game))
            .filter_map(|object| current_object_pt(game, object.id, power));
        if greatest { values.max() } else { values.min() }
    }

    fn aggregate_mana_value(
        filter: &ObjectFilter,
        game: &GameState,
        ctx: &FilterContext,
        greatest: bool,
    ) -> Option<i32> {
        if filter.cast_this_turn && filter.zone == Some(Zone::Stack) {
            let snapshots = game.turn_store.turn_history.spell_cast_snapshot_history();
            let values = snapshots
                .iter()
                .filter(|snapshot| filter.matches_snapshot(snapshot, ctx, game))
                .map(snapshot_mana_value_for_filter);
            return if greatest { values.max() } else { values.min() };
        }

        if let Some(snapshots) = aggregate_tagged_snapshots(filter, ctx) {
            let values = snapshots
                .into_iter()
                .filter(|snapshot| filter.matches_snapshot(snapshot, ctx, game))
                .map(snapshot_mana_value_for_filter);
            return if greatest { values.max() } else { values.min() };
        }

        let values = game
            .objects_in_deterministic_order()
            .into_iter()
            .filter(|object| filter.matches(object, ctx, game))
            .map(object_mana_value_for_filter);
        if greatest { values.max() } else { values.min() }
    }

    match rhs {
        Value::SurfaceHinted { value, .. } => {
            resolve_filter_comparison_rhs_value(value, game, ctx, stack_entry)
        }
        Value::Fixed(value) => Some(*value),
        Value::X => resolve_x_value(game, ctx, stack_entry),
        Value::XTimes(multiplier) => {
            resolve_x_value(game, ctx, stack_entry).map(|value| value * multiplier)
        }
        Value::EffectValue(effect_id) => ctx
            .effect_outcomes
            .get(effect_id)
            .and_then(|outcome| outcome.as_count()),
        Value::EffectValueOffset(effect_id, offset) => ctx
            .effect_outcomes
            .get(effect_id)
            .and_then(|outcome| outcome.as_count())
            .map(|value| value + offset),
        Value::Add(left, right) => Some(
            resolve_filter_comparison_rhs_value(left, game, ctx, stack_entry)?
                + resolve_filter_comparison_rhs_value(right, game, ctx, stack_entry)?,
        ),
        Value::Scaled(inner, multiplier) => {
            resolve_filter_comparison_rhs_value(inner, game, ctx, stack_entry)
                .map(|value| value * multiplier)
        }
        Value::DividedRoundedDown(inner, divisor) if *divisor != 0 => {
            resolve_filter_comparison_rhs_value(inner, game, ctx, stack_entry)
                .map(|value| value.div_euclid(*divisor))
        }
        Value::Min(left, right) => Some(
            resolve_filter_comparison_rhs_value(left, game, ctx, stack_entry)?.min(
                resolve_filter_comparison_rhs_value(right, game, ctx, stack_entry)?,
            ),
        ),
        Value::Count(filter) => {
            let mut count = 0i32;
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    count += 1;
                }
            }
            Some(count)
        }
        Value::CountScaled(filter, factor) => {
            let mut count = 0i32;
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    count += 1;
                }
            }
            Some(count * *factor)
        }
        Value::GreatestSharedCreatureTypeCount(filter) => {
            let mut counts = std::collections::HashMap::new();
            for object in game.objects_in_deterministic_order() {
                if !filter.matches(object, ctx, game) {
                    continue;
                }
                let controller_group = filter
                    .controller
                    .as_ref()
                    .map(|_| game.controller_of(object));
                let subtypes = game
                    .current_subtypes(object.id)
                    .unwrap_or_else(|| object.subtypes.to_vec());
                let mut types_on_object = std::collections::HashSet::new();
                for subtype in subtypes {
                    if subtype.is_creature_type() && types_on_object.insert(subtype) {
                        *counts.entry((controller_group, subtype)).or_insert(0i32) += 1;
                    }
                }
            }
            Some(counts.into_values().max().unwrap_or(0))
        }
        Value::ColorsAmong(filter) => {
            let only_is_tagged_constraints = !filter.tagged_constraints.is_empty()
                && filter.tagged_constraints.iter().all(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                });
            if only_is_tagged_constraints {
                let mut seen = std::collections::HashSet::new();
                let mut colors = ColorSet::new();
                for constraint in &filter.tagged_constraints {
                    let Some(snapshots) = ctx.tagged_objects.get(constraint.tag.as_str()) else {
                        continue;
                    };
                    for snapshot in snapshots {
                        if seen.insert(snapshot.object_id) {
                            colors = colors.union(snapshot.colors);
                        }
                    }
                }
                return Some(colors.count() as i32);
            }
            let mut colors = ColorSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    colors = colors.union(object.colors());
                }
            }
            Some(colors.count() as i32)
        }
        Value::CreatureTypesAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    let subtypes = game
                        .current_subtypes(object.id)
                        .unwrap_or_else(|| object.subtypes.to_vec());
                    for subtype in subtypes {
                        if subtype.is_creature_type() {
                            seen.insert(subtype);
                        }
                    }
                }
            }
            Some(seen.len() as i32)
        }
        Value::CardTypesAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    let card_types = game
                        .current_card_types(object.id)
                        .unwrap_or_else(|| object.card_types.to_vec());
                    for card_type in card_types {
                        seen.insert(card_type);
                    }
                }
            }
            Some(seen.len() as i32)
        }
        Value::StaticAbilitiesAmong { filter, abilities } => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    for ability_id in abilities {
                        if game.current_has_static_ability_id(object.id, *ability_id) {
                            seen.insert(*ability_id);
                        }
                    }
                }
            }
            Some(seen.len() as i32)
        }
        Value::DistinctPowers(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game)
                    && let Some(power) = game.calculated_power(object.id).or_else(|| object.power())
                {
                    seen.insert(power);
                }
            }
            Some(seen.len() as i32)
        }
        Value::ColorPairsAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    let colors = object.colors();
                    if colors.count() == 2 {
                        seen.insert(colors);
                    }
                }
            }
            Some(seen.len() as i32)
        }
        Value::DistinctCounterTypesAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    seen.extend(object.counters.keys().copied());
                }
            }
            Some(seen.len() as i32)
        }
        Value::GreatestPower(filter) => aggregate_pt(filter, game, ctx, true, true),
        Value::GreatestToughness(filter) => aggregate_pt(filter, game, ctx, false, true),
        Value::GreatestManaValue(filter) => aggregate_mana_value(filter, game, ctx, true),
        Value::LeastPower(filter) => aggregate_pt(filter, game, ctx, true, false),
        Value::LeastToughness(filter) => aggregate_pt(filter, game, ctx, false, false),
        Value::LeastManaValue(filter) => aggregate_mana_value(filter, game, ctx, false),
        Value::CountersOnSource(counter_type) => {
            let source = game.object(ctx.source?)?;
            Some(source.counters.get(counter_type).copied().unwrap_or(0) as i32)
        }
        Value::SourcePower => current_object_pt(game, ctx.source?, true),
        Value::SourceToughness => current_object_pt(game, ctx.source?, false),
        Value::PowerOf(spec) => resolve_pt_choose_spec(spec, game, ctx, true),
        Value::ToughnessOf(spec) => resolve_pt_choose_spec(spec, game, ctx, false),
        Value::CountersOn(spec, counter_type) => match spec.base() {
            ChooseSpec::Source => {
                let source = game.object(ctx.source?)?;
                Some(match counter_type {
                    Some(counter_type) => {
                        source.counters.get(counter_type).copied().unwrap_or(0) as i32
                    }
                    None => total_counters(&source.counters),
                })
            }
            ChooseSpec::Tagged(tag) => {
                let snapshots = ctx.tagged_objects.get(tag)?;
                let snapshot = snapshots.first()?;
                Some(match counter_type {
                    Some(counter_type) => {
                        snapshot.counters.get(counter_type).copied().unwrap_or(0) as i32
                    }
                    None => total_counters(&snapshot.counters),
                })
            }
            _ => None,
        },
        Value::ManaValueOf(spec) => match spec.base() {
            ChooseSpec::Source => {
                let mana_value = game
                    .object(ctx.source?)
                    .and_then(|source| source.mana_cost.as_ref())
                    .map(|cost| cost.mana_value() as i32)
                    .or_else(|| {
                        ctx.source_snapshot
                            .as_ref()
                            .and_then(|snapshot| snapshot.mana_cost.as_ref())
                            .map(|cost| cost.mana_value() as i32)
                    });
                Some(mana_value.unwrap_or(0))
            }
            ChooseSpec::Tagged(tag) => {
                let snapshot = ctx.tagged_objects.get(tag)?.first()?;
                Some(
                    snapshot
                        .mana_cost
                        .as_ref()
                        .map_or(0, |cost| cost.mana_value() as i32),
                )
            }
            _ => None,
        },
        Value::UnspentMana(player_filter) => Some(
            game.players
                .iter()
                .filter(|player| {
                    player.is_in_game() && player_filter.matches_player(player.id, ctx)
                })
                .map(|player| player.mana_pool.total() as i32)
                .sum(),
        ),
        Value::Devotion { player, color } => Some(
            game.players
                .iter()
                .filter(|candidate| {
                    candidate.is_in_game() && player.matches_player(candidate.id, ctx)
                })
                .map(|candidate| game.devotion_to_color(candidate.id, *color) as i32)
                .sum(),
        ),
        _ => None,
    }
}

fn resolve_player_filter_object_ref<'a>(
    object_ref: &ObjectRef,
    ctx: &'a FilterContext,
) -> Option<&'a ObjectSnapshot> {
    match object_ref {
        ObjectRef::Target => ctx.target_objects.first(),
        ObjectRef::Specific(object_id) => ctx
            .target_objects
            .iter()
            .find(|snapshot| snapshot.object_id == *object_id)
            .or_else(|| {
                ctx.tagged_objects
                    .values()
                    .flat_map(|snapshots| snapshots.iter())
                    .find(|snapshot| snapshot.object_id == *object_id)
            }),
        ObjectRef::Tagged(tag) => ctx
            .tagged_objects
            .get(tag)
            .and_then(|snapshots| snapshots.first()),
    }
}

fn resolve_object_ref_id(object_ref: &ObjectRef, ctx: &FilterContext) -> Option<ObjectId> {
    match object_ref {
        ObjectRef::Target => ctx
            .target_objects
            .first()
            .map(|snapshot| snapshot.object_id),
        ObjectRef::Specific(object_id) => Some(*object_id),
        ObjectRef::Tagged(tag) => ctx
            .tagged_objects
            .get(tag)
            .and_then(|snapshots| snapshots.first())
            .map(|snapshot| snapshot.object_id),
    }
}

fn resolve_object_ref_ids(object_ref: &ObjectRef, ctx: &FilterContext) -> Vec<ObjectId> {
    match object_ref {
        ObjectRef::Target => ctx
            .target_objects
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect(),
        ObjectRef::Specific(object_id) => vec![*object_id],
        ObjectRef::Tagged(tag) => ctx
            .tagged_objects
            .get(tag)
            .map(|snapshots| {
                snapshots
                    .iter()
                    .map(|snapshot| snapshot.object_id)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn creature_was_blocked_by_ref(
    game: &GameState,
    ctx: &FilterContext,
    attacker: ObjectId,
    blocker_ref: &ObjectRef,
) -> bool {
    let blockers = resolve_object_ref_ids(blocker_ref, ctx);
    if blockers.is_empty() {
        return false;
    }
    if let Some(combat) = &game.combat {
        let current_blockers = crate::combat_state::get_blockers(combat, attacker);
        if blockers
            .iter()
            .any(|blocker| current_blockers.contains(blocker))
        {
            return true;
        }
    }
    blockers
        .iter()
        .any(|blocker| game.creature_was_blocked_by_this_turn(attacker, *blocker))
}

fn object_is_in_combat_with_source_lki(
    game: &GameState,
    ctx: &FilterContext,
    object_id: ObjectId,
) -> bool {
    let Some(combat) = &game.combat else {
        return false;
    };
    let source_ids = ctx.source.into_iter().chain(
        ctx.source_snapshot
            .as_ref()
            .map(|snapshot| snapshot.object_id),
    );
    source_ids.into_iter().any(|source_id| {
        crate::combat_state::get_blockers(combat, source_id).contains(&object_id)
            || crate::combat_state::get_blocked_attacker(combat, source_id)
                .is_some_and(|attacker| attacker == object_id)
    })
}

fn creature_blocked_or_was_blocked_by_matching_this_turn(
    game: &GameState,
    ctx: &FilterContext,
    creature: ObjectId,
    partner_filter: &ObjectFilter,
) -> bool {
    game.turn_store
        .turn_history
        .projected_records()
        .filter_map(|record| {
            record
                .event
                .downcast::<crate::events::combat::CreatureBlockedEvent>()
        })
        .any(|event| {
            let (partner_id, partner_snapshot) = if event.blocker == creature {
                (event.attacker, event.attacker_snapshot.as_ref())
            } else if event.attacker == creature {
                (event.blocker, event.blocker_snapshot.as_ref())
            } else {
                return false;
            };

            partner_snapshot
                .is_some_and(|snapshot| partner_filter.matches_snapshot(snapshot, ctx, game))
                || (partner_snapshot.is_none()
                    && game
                        .object(partner_id)
                        .is_some_and(|partner| partner_filter.matches(partner, ctx, game)))
        })
}

fn effects_for_stack_entry(
    game: &crate::game_state::GameState,
    entry: &crate::game_state::StackEntry,
) -> Vec<crate::effect::Effect> {
    if let Some(ref effects) = entry.ability_effects {
        return effects.to_vec();
    }

    game.object(entry.object_id)
        .and_then(|object| object.spell_effect_owned())
        .map(|effects| effects.to_vec())
        .unwrap_or_default()
}

fn stack_entry_has_ability_marker(
    game: &crate::game_state::GameState,
    entry: &crate::game_state::StackEntry,
    marker: &str,
) -> bool {
    let normalized = marker.trim().to_ascii_lowercase();
    if normalized == "backup" || normalized == "backup ability" {
        return effects_for_stack_entry(game, entry).iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::BackupEffect>()
                .is_some()
        });
    }
    false
}

fn object_could_be_targeted_by(
    object_id: ObjectId,
    constraint: &TargetabilityConstraint,
    ctx: &FilterContext,
    game: &crate::game_state::GameState,
) -> bool {
    let Some(stack_object_id) = resolve_object_ref_id(&constraint.stack_object, ctx) else {
        return false;
    };
    let Some(entry) = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_object_id)
    else {
        return false;
    };

    effects_for_stack_entry(game, entry).iter().any(|effect| {
        let Some(spec) = effect.0.get_target_spec() else {
            return false;
        };
        crate::targeting::compute_legal_targets_with_tagged_objects(
            game,
            spec,
            entry.controller,
            Some(entry.object_id),
            if entry.tagged_objects.is_empty() {
                None
            } else {
                Some(&entry.tagged_objects)
            },
        )
        .into_iter()
        .any(|target| matches!(target, crate::game_state::Target::Object(id) if id == object_id))
    })
}

pub trait PlayerFilterExt {
    fn matches_player(&self, player: PlayerId, ctx: &FilterContext) -> bool;
}

impl PlayerFilterExt for PlayerFilter {
    /// Check if a player matches this filter.
    ///
    /// Note: Some variants (EachOpponent, EachPlayer, Target, ControllerOf, OwnerOf, IteratedPlayer)
    /// are resolved at runtime during effect execution, not through this method.
    fn matches_player(&self, player: PlayerId, ctx: &FilterContext) -> bool {
        if ctx
            .players_in_range
            .as_ref()
            .is_some_and(|players| !players.contains(&player))
        {
            return false;
        }
        match self {
            PlayerFilter::Any => true,

            PlayerFilter::You => ctx.you.is_some_and(|you| player == you),

            PlayerFilter::NotYou => ctx.you != Some(player),

            PlayerFilter::Opponent => ctx.opponents.contains(&player),

            PlayerFilter::Teammate => ctx.teammates.contains(&player),

            // Seat-relative filters require the game's stable physical seat
            // order and are handled by `player_filter_matches_game`.
            PlayerFilter::PlayerToYourLeft | PlayerFilter::PlayerToYourRight => false,

            PlayerFilter::Active => ctx.active_player.is_some_and(|ap| player == ap),

            PlayerFilter::Defending => {
                if ctx.defending_players.is_empty() {
                    ctx.defending_player.is_some_and(|dp| player == dp)
                } else {
                    ctx.defending_players.contains(&player)
                }
            }

            PlayerFilter::Attacking => {
                if ctx.attacking_players.is_empty() {
                    ctx.attacking_player.is_some_and(|ap| player == ap)
                } else {
                    ctx.attacking_players.contains(&player)
                }
            }

            PlayerFilter::DamagedPlayer => ctx
                .tagged_players
                .get("damaged_player")
                .is_some_and(|players| players.contains(&player)),

            PlayerFilter::EffectController => false,

            PlayerFilter::Specific(id) => player == *id,
            PlayerFilter::MostLifeTied => false,
            PlayerFilter::LowestLifeTied => false,
            PlayerFilter::MostCardsInHand => false,
            PlayerFilter::CastCardTypeThisTurn(_) => false,
            // Source-relative turn history requires access to GameState and
            // is evaluated by `player_filter_matches_game` below.
            PlayerFilter::AttackedBySourceThisTurn => false,
            PlayerFilter::WasDealtDamageBySourceThisGame { base } => {
                base.matches_player(player, ctx)
            }
            PlayerFilter::WasDealtCombatDamageBySourcesThisGame { base, .. } => {
                base.matches_player(player, ctx)
            }
            PlayerFilter::LostLifeThisTurn { base } => base.matches_player(player, ctx),
            PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, .. } => {
                base.matches_player(player, ctx)
            }
            PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. } => {
                base.matches_player(player, ctx)
            }
            PlayerFilter::HasMoreLifeThanYou { base } => base.matches_player(player, ctx),
            PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => false,
            PlayerFilter::ControlsMost { .. } => false,
            PlayerFilter::MaxSpeed { .. } => false,
            PlayerFilter::ChosenPlayer => ctx.chosen_player.is_some_and(|chosen| chosen == player),
            PlayerFilter::TaggedPlayer(tag) => ctx
                .tagged_players
                .get(tag)
                .is_some_and(|players| players.contains(&player)),

            // These are resolved at runtime during effect execution
            PlayerFilter::IteratedPlayer => ctx.iterated_player.is_some_and(|p| p == player),
            PlayerFilter::TargetPlayerOrControllerOfTarget => {
                ctx.target_players.contains(&player)
                    || ctx
                        .target_objects
                        .first()
                        .is_some_and(|snapshot| snapshot.controller == player)
            }
            PlayerFilter::Excluding { base, excluded } => {
                base.matches_player(player, ctx) && !excluded.matches_player(player, ctx)
            }
            PlayerFilter::Target(inner) => {
                let inner = inner
                    .relative_target_exclusion_base()
                    .unwrap_or(inner.as_ref());
                if !ctx.target_players.is_empty() {
                    return ctx.target_players.contains(&player)
                        && inner.matches_player(player, ctx);
                }
                ctx.iterated_player.is_some_and(|p| p == player)
                    && inner.matches_player(player, ctx)
            }
            PlayerFilter::AliasedTarget(inner) => {
                let inner = inner
                    .relative_target_exclusion_base()
                    .unwrap_or(inner.as_ref());
                ctx.target_players.contains(&player) && inner.matches_player(player, ctx)
            }
            PlayerFilter::ControllerOf(object_ref) => {
                resolve_player_filter_object_ref(object_ref, ctx)
                    .is_some_and(|snapshot| snapshot.controller == player)
            }
            PlayerFilter::OwnerOf(object_ref) => resolve_player_filter_object_ref(object_ref, ctx)
                .is_some_and(|snapshot| snapshot.owner == player),
            PlayerFilter::AliasedControllerOf(object_ref) => {
                resolve_player_filter_object_ref(object_ref, ctx)
                    .is_some_and(|snapshot| snapshot.controller == player)
            }
            PlayerFilter::AliasedOwnerOf(object_ref) => {
                resolve_player_filter_object_ref(object_ref, ctx)
                    .is_some_and(|snapshot| snapshot.owner == player)
            }
        }
    }
}

pub(crate) fn player_filter_matches_game(
    filter: &PlayerFilter,
    player: PlayerId,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
) -> bool {
    match filter {
        PlayerFilter::AttackedBySourceThisTurn => {
            let Some(source) = ctx.source else {
                return false;
            };

            game.turn_store
                .turn_history
                .projected_records()
                .any(|record| {
                    let Some(event) = record.event.downcast::<CreatureAttackedEvent>() else {
                        return false;
                    };
                    if !matches!(
                        event.target,
                        crate::triggers::event::AttackEventTarget::Player(defender)
                            if defender == player
                    ) {
                        return false;
                    }

                    // Object identity is intentional. A permanent that leaves
                    // and returns is a new game object and must not inherit
                    // the old object's attack history merely because the
                    // underlying card retains its engine stable ID.
                    event.attacker == source
                })
        }
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => {
            let Some(source) = ctx.source else {
                return false;
            };
            player_filter_matches_game(base, player, game, ctx)
                && game.source_dealt_damage_to_player_this_game(source, player)
        }
        PlayerFilter::WasDealtCombatDamageBySourcesThisGame { base, sources } => {
            if !player_filter_matches_game(base, player, game, ctx) {
                return false;
            }
            game.players.iter().any(|involved| {
                game.action_history_for_player(involved.id).any(|record| {
                    let Some(damage) = record.event.downcast::<crate::events::DamageEvent>() else {
                        return false;
                    };
                    if !damage.is_combat
                        || damage.amount == 0
                        || damage.target != crate::events::DamageTarget::Player(player)
                    {
                        return false;
                    }
                    record
                        .source_snapshot
                        .as_ref()
                        .or(record.object_snapshot.as_ref())
                        .is_some_and(|snapshot| sources.matches_snapshot(snapshot, ctx, game))
                })
            })
        }
        PlayerFilter::LostLifeThisTurn { base } => {
            player_filter_matches_game(base, player, game, ctx)
                && game
                    .turn_store
                    .turn_history
                    .player_lost_life_this_turn(player)
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base,
            sources,
            minimum,
        } => {
            if !player_filter_matches_game(base, player, game, ctx) {
                return false;
            }
            let distinct_sources = game
                .turn_store
                .turn_history
                .projected_records()
                .filter_map(|record| {
                    let damage = record.event.downcast::<crate::events::DamageEvent>()?;
                    if !damage.is_combat
                        || damage.amount == 0
                        || damage.target != crate::events::DamageTarget::Player(player)
                    {
                        return None;
                    }
                    let snapshot = record
                        .source_snapshot
                        .as_ref()
                        .or(record.object_snapshot.as_ref())?;
                    sources
                        .matches_snapshot(snapshot, ctx, game)
                        .then_some(damage.source)
                })
                .collect::<std::collections::HashSet<_>>();
            distinct_sources.len() >= *minimum as usize
        }
        PlayerFilter::PlayerToYourLeft => ctx.you.is_some_and(|you| {
            game.closest_in_game_player_to_left_matching(you, |_| true) == Some(player)
        }),
        PlayerFilter::PlayerToYourRight => ctx.you.is_some_and(|you| {
            game.closest_in_game_player_to_right_matching(you, |_| true) == Some(player)
        }),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            if !player_filter_matches_game(base, player, game, ctx) {
                return false;
            }
            let Some(you) = ctx.you else {
                return false;
            };
            let candidate_hand = game.player(player).map(|p| p.hand.len()).unwrap_or(0);
            let your_hand = game.player(you).map(|p| p.hand.len()).unwrap_or(0);
            candidate_hand >= your_hand.saturating_add(*count as usize)
        }
        PlayerFilter::HasMoreLifeThanYou { base } => {
            if !player_filter_matches_game(base, player, game, ctx) {
                return false;
            }
            let Some(you) = ctx.you else {
                return false;
            };
            let candidate_life = game.player(player).map(|p| p.life).unwrap_or(0);
            let your_life = game.player(you).map(|p| p.life).unwrap_or(0);
            candidate_life > your_life
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan {
            player: reference_filter,
            filter: object_filter,
        } => {
            if ctx
                .players_in_range
                .as_ref()
                .is_some_and(|players| !players.contains(&player))
            {
                return false;
            }

            game.players
                .iter()
                .filter(|candidate| candidate.is_in_game())
                .filter(|candidate| {
                    player_filter_matches_game(reference_filter, candidate.id, game, ctx)
                })
                .any(|reference| {
                    game.are_opponents(reference.id, player)
                        && controlled_matching_object_count(game, player, object_filter, ctx)
                            > controlled_matching_object_count(
                                game,
                                reference.id,
                                object_filter,
                                ctx,
                            )
                })
        }
        PlayerFilter::ControlsMost {
            filter: object_filter,
        } => {
            if ctx
                .players_in_range
                .as_ref()
                .is_some_and(|players| !players.contains(&player))
            {
                return false;
            }
            let mut leaders = game
                .players
                .iter()
                .filter(|candidate| candidate.is_in_game())
                .map(|candidate| {
                    (
                        candidate.id,
                        controlled_matching_object_count(game, candidate.id, object_filter, ctx),
                    )
                })
                .collect::<Vec<_>>();
            let Some(maximum) = leaders.iter().map(|(_, count)| *count).max() else {
                return false;
            };
            leaders.retain(|(_, count)| *count == maximum);
            matches!(leaders.as_slice(), [(leader, _)] if *leader == player)
        }
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            player_filter_matches_game(base, player, game, ctx)
                && game.has_max_speed(player) == *has_max_speed
        }
        PlayerFilter::Target(inner) => {
            let inner = inner
                .relative_target_exclusion_base()
                .unwrap_or(inner.as_ref());
            if !ctx.target_players.is_empty() {
                return ctx.target_players.contains(&player)
                    && player_filter_matches_game(inner, player, game, ctx);
            }
            ctx.iterated_player.is_some_and(|p| p == player)
                && player_filter_matches_game(inner, player, game, ctx)
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_matches_game(base, player, game, ctx)
                && !player_filter_matches_game(excluded, player, game, ctx)
        }
        other => other.matches_player(player, ctx),
    }
}

fn controlled_matching_object_count(
    game: &crate::game_state::GameState,
    controller: PlayerId,
    filter: &ObjectFilter,
    ctx: &FilterContext,
) -> usize {
    let mut object_ctx = ctx.clone();
    object_ctx.you = Some(controller);
    object_ctx.opponents = game
        .players
        .iter()
        .filter(|candidate| candidate.is_in_game() && game.are_opponents(controller, candidate.id))
        .map(|candidate| candidate.id)
        .collect();
    object_ctx.teammates = game
        .players
        .iter()
        .filter(|candidate| candidate.is_in_game() && game.are_teammates(controller, candidate.id))
        .map(|candidate| candidate.id)
        .collect();
    object_ctx.your_commanders = game
        .player(controller)
        .map(|player| player.commanders.clone())
        .unwrap_or_default();

    game.battlefield
        .iter()
        .filter_map(|object_id| game.object(*object_id))
        .filter(|object| game.controller_of(object) == controller)
        .filter(|object| filter.matches(object, &object_ctx, game))
        .count()
}

pub(crate) trait ObjectFilterExt {
    fn matches(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool;

    fn matches_with_view(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        view: &crate::derived_view::DerivedGameView<'_>,
    ) -> bool;

    fn matches_non_recursive(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool;

    fn matches_shared_tail<S: TailMatchSubject>(
        &self,
        subject: &S,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        stack_entry: Option<&crate::game_state::StackEntry>,
    ) -> bool;

    fn matches_layered_tail(
        &self,
        subject: &LayeredSubject<'_>,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool;

    fn matches_internal(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        allow_calculated_pt: bool,
        view: Option<&crate::derived_view::DerivedGameView<'_>>,
    ) -> bool;

    fn stack_entry_matches_kind(
        entry: &crate::game_state::StackEntry,
        kind: StackObjectKind,
    ) -> bool
    where
        Self: Sized;

    fn tagged_constraint_requires_existing_tag(relation: TaggedOpbjectRelation) -> bool
    where
        Self: Sized;

    fn matches_snapshot(
        &self,
        snapshot: &crate::snapshot::ObjectSnapshot,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool;

    #[allow(dead_code)]
    fn description(&self) -> String;
}

impl ObjectFilterExt for ObjectFilter {
    /// Check if an object matches this filter, with access to game state.
    ///
    /// # Arguments
    /// * `object` - The object to check
    /// * `ctx` - Context providing information about "you", the source, etc.
    /// * `game` - Game state for checking tapped/untapped status
    fn matches(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        if ctx
            .you
            .is_some_and(|observer| !game.object_is_within_range(observer, object.id, ctx.source))
        {
            return false;
        }
        if object.zone == crate::zone::Zone::Battlefield && game.is_phased_out(object.id) {
            return false;
        }
        self.matches_internal(object, ctx, game, true, None)
    }

    fn matches_with_view(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        view: &crate::derived_view::DerivedGameView<'_>,
    ) -> bool {
        if ctx
            .you
            .is_some_and(|observer| !game.object_is_within_range(observer, object.id, ctx.source))
        {
            return false;
        }
        if object.zone == crate::zone::Zone::Battlefield && game.is_phased_out(object.id) {
            return false;
        }
        self.matches_internal(object, ctx, game, true, Some(view))
    }

    /// Check if an object matches this filter without consulting calculated characteristics.
    ///
    /// This is used by layer-calculation paths that must avoid recursively
    /// re-entering characteristic computation.
    fn matches_non_recursive(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        if ctx
            .you
            .is_some_and(|observer| !game.object_is_within_range(observer, object.id, ctx.source))
        {
            return false;
        }
        if object.zone == crate::zone::Zone::Battlefield && game.is_phased_out(object.id) {
            return false;
        }
        self.matches_internal(object, ctx, game, false, None)
    }

    fn matches_shared_tail<S: TailMatchSubject>(
        &self,
        subject: &S,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        stack_entry: Option<&crate::game_state::StackEntry>,
    ) -> bool {
        // Name check
        if let Some(required_name) = &self.name {
            if required_name == "{chosen name}" {
                let Some(source) = ctx.source else {
                    return false;
                };
                let Some(chosen_name) = game.chosen_named_option(source) else {
                    return false;
                };
                if !names_match(subject.tail_name(), chosen_name) {
                    return false;
                }
            } else if !names_match(subject.tail_name(), required_name) {
                return false;
            }
        }
        if let Some(excluded_name) = &self.excluded_name
            && names_match(subject.tail_name(), excluded_name)
        {
            return false;
        }
        if let Some(required_set_name) = &self.name_originally_printed_in_set
            && !subject
                .tail_first_printed_set_name()
                .is_some_and(|set_name| set_name.eq_ignore_ascii_case(required_set_name))
        {
            return false;
        }

        if let Some(counter_requirement) = self.with_counter {
            let has_counter = match counter_requirement {
                CounterConstraint::Any => subject.tail_counters().values().any(|count| *count > 0),
                CounterConstraint::Typed(counter_type) => {
                    subject
                        .tail_counters()
                        .get(&counter_type)
                        .copied()
                        .unwrap_or(0)
                        > 0
                }
                CounterConstraint::AtLeast {
                    counter_type,
                    count,
                } => {
                    let actual = counter_type.map_or_else(
                        || subject.tail_counters().values().copied().sum(),
                        |counter_type| {
                            subject
                                .tail_counters()
                                .get(&counter_type)
                                .copied()
                                .unwrap_or(0)
                        },
                    );
                    actual >= count
                }
            };
            if !has_counter {
                return false;
            }
        }
        if let Some(counter_exclusion) = self.without_counter {
            let has_excluded_counter = match counter_exclusion {
                CounterConstraint::Any => subject.tail_counters().values().any(|count| *count > 0),
                CounterConstraint::Typed(counter_type) => {
                    subject
                        .tail_counters()
                        .get(&counter_type)
                        .copied()
                        .unwrap_or(0)
                        > 0
                }
                CounterConstraint::AtLeast {
                    counter_type,
                    count,
                } => {
                    let actual = counter_type.map_or_else(
                        || subject.tail_counters().values().copied().sum(),
                        |counter_type| {
                            subject
                                .tail_counters()
                                .get(&counter_type)
                                .copied()
                                .unwrap_or(0)
                        },
                    );
                    actual >= count
                }
            };
            if has_excluded_counter {
                return false;
            }
        }

        if let Some(kind) = self.alternative_cast
            && !subject.tail_has_alternative_cast_kind(kind, game, ctx)
        {
            return false;
        }

        // Required static ability IDs
        if self
            .static_abilities
            .iter()
            .any(|ability_id| !subject.tail_has_static_ability_id(*ability_id))
        {
            return false;
        }

        // Excluded static ability IDs
        if self
            .excluded_static_abilities
            .iter()
            .any(|ability_id| subject.tail_has_static_ability_id(*ability_id))
        {
            return false;
        }

        // Required/excluded ability markers
        if self.ability_markers.iter().any(|marker| {
            !subject.tail_has_ability_marker(marker)
                && !stack_entry
                    .is_some_and(|entry| stack_entry_has_ability_marker(game, entry, marker))
        }) {
            return false;
        }
        if self.excluded_ability_markers.iter().any(|marker| {
            subject.tail_has_ability_marker(marker)
                || stack_entry
                    .is_some_and(|entry| stack_entry_has_ability_marker(game, entry, marker))
        }) {
            return false;
        }

        if self
            .no_shared_creature_types_with
            .iter()
            .any(|comparison_filter| {
                subject_shares_creature_type_with_filter(subject, comparison_filter, ctx, game)
            })
        {
            return false;
        }
        if self
            .characteristic_relations
            .iter()
            .any(|relation| !characteristic_relation_matches_subject(subject, relation, ctx, game))
        {
            return false;
        }
        if self.shares_creature_type_with_source
            && !subject_shares_creature_type_with_source(subject, ctx, game)
        {
            return false;
        }

        if self.has_tap_activated_ability && !subject.tail_has_tap_activated_ability() {
            return false;
        }
        if !self.could_produce_mana.is_empty()
            && !subject_could_produce_any_mana_symbol(subject, &self.could_produce_mana, game)
        {
            return false;
        }
        if self.no_abilities && !subject.tail_abilities().is_empty() {
            return false;
        }

        // Commander check
        if self.is_commander && !subject.tail_is_commander(game) {
            return false;
        }
        if self.noncommander && subject.tail_is_commander(game) {
            return false;
        }

        for constraint in &self.tagged_constraints {
            if constraint.relation == TaggedOpbjectRelation::SharesMostCommonPermanentColor {
                if !subject_shares_most_common_permanent_color(subject, game) {
                    return false;
                }
                continue;
            }
            let Some(tagged_snapshots) = ctx.tagged_objects.get(constraint.tag.as_str()) else {
                if let Some(matches) = intrinsic_attachment_tag_constraint_matches_subject(
                    subject,
                    &constraint.tag,
                    constraint.relation,
                    game,
                ) {
                    if !matches {
                        return false;
                    }
                    continue;
                }
                if Self::tagged_constraint_requires_existing_tag(constraint.relation) {
                    return false;
                }
                continue;
            };
            if !tagged_constraint_matches_subject(
                subject,
                tagged_snapshots,
                constraint.relation,
                game,
            ) {
                return false;
            }
        }

        if let Some(attached_to_filter) = &self.attached_to_object {
            let matches_current_attachment = subject
                .subject_attached_to()
                .and_then(|attached_to_id| game.object(attached_to_id))
                .is_some_and(|attached_to| attached_to_filter.matches(attached_to, ctx, game));
            let matches_departed_source_lki = !matches_current_attachment
                && attached_to_filter.source
                && ctx.source_snapshot.as_ref().is_some_and(|source_snapshot| {
                    let source_has_departed = ctx.source.is_some_and(|source_id| {
                        game.object(source_id).is_none_or(|current_source| {
                            current_source.stable_id != source_snapshot.stable_id
                                || current_source.zone != source_snapshot.zone
                        })
                    });
                    source_has_departed
                        && source_snapshot
                            .attachments
                            .contains(&subject.subject_object_id())
                        && attached_to_filter.matches_snapshot(source_snapshot, ctx, game)
                });
            if !matches_current_attachment && !matches_departed_source_lki {
                return false;
            }
        }

        if let Some(with_attached_filter) = &self.with_attached_object {
            let has_matching_attachment = subject.subject_attachments().iter().any(|&id| {
                game.object(id)
                    .is_some_and(|attachment| with_attached_filter.matches(attachment, ctx, game))
            });
            if !has_matching_attachment {
                return false;
            }
        }

        if let Some(without_attached_filter) = &self.without_attached_object {
            let has_forbidden_attachment = subject.subject_attachments().iter().any(|&id| {
                game.object(id).is_some_and(|attachment| {
                    without_attached_filter.matches(attachment, ctx, game)
                })
            });
            if has_forbidden_attachment {
                return false;
            }
        }

        if let Some(player_filter) = &self.attached_to_player {
            let Some(attached_player) = subject.subject_attached_to_player() else {
                return false;
            };
            if !player_filter.matches_player(attached_player, ctx) {
                return false;
            }
        }

        let object_id = subject.tail_object_id();
        if let Some(partner_filter) = &self.blocked_or_was_blocked_by_this_turn
            && !creature_blocked_or_was_blocked_by_matching_this_turn(
                game,
                ctx,
                object_id,
                partner_filter,
            )
        {
            return false;
        }

        // Targeting checks (spell/ability targets on the stack)
        if self.targets_player.is_some() || self.targets_object.is_some() {
            let Some(entry) =
                stack_entry.or_else(|| game.stack.iter().find(|e| e.object_id == object_id))
            else {
                return false;
            };

            let matches_player = self.targets_player.as_ref().is_none_or(|player_filter| {
                entry.targets.iter().any(|target| match target {
                    crate::game_state::Target::Player(pid) => {
                        player_filter.matches_player(*pid, ctx)
                    }
                    _ => false,
                })
            });

            let matches_object = self.targets_object.as_ref().is_none_or(|object_filter| {
                entry.targets.iter().any(|target| match target {
                    crate::game_state::Target::Object(obj_id) => game
                        .object(*obj_id)
                        .is_some_and(|obj| object_filter.matches(obj, ctx, game)),
                    _ => false,
                })
            });

            let matches = if self.targets_any_of
                && self.targets_player.is_some()
                && self.targets_object.is_some()
            {
                matches_player || matches_object
            } else {
                matches_player && matches_object
            };
            if !matches {
                return false;
            }
        }

        if self.target_count.is_some()
            || self.targets_only_player.is_some()
            || self.targets_only_object.is_some()
        {
            let Some(entry) =
                stack_entry.or_else(|| game.stack.iter().find(|e| e.object_id == object_id))
            else {
                return false;
            };

            if let Some(count) = self.target_count {
                let total = entry.targets.len();
                if total < count.min {
                    return false;
                }
                if let Some(max) = count.max
                    && total > max
                {
                    return false;
                }
            }

            if self.targets_only_player.is_some() || self.targets_only_object.is_some() {
                if entry.targets.is_empty() {
                    return false;
                }

                let matches_target = |target: &crate::game_state::Target| -> bool {
                    let matches_player = self.targets_only_player.as_ref().is_some_and(
                        |player_filter| match target {
                            crate::game_state::Target::Player(pid) => {
                                player_filter.matches_player(*pid, ctx)
                            }
                            _ => false,
                        },
                    );
                    let matches_object = self.targets_only_object.as_ref().is_some_and(
                        |object_filter| match target {
                            crate::game_state::Target::Object(obj_id) => game
                                .object(*obj_id)
                                .is_some_and(|obj| object_filter.matches(obj, ctx, game)),
                            _ => false,
                        },
                    );

                    if self.targets_only_player.is_some() && self.targets_only_object.is_some() {
                        matches_player || matches_object
                    } else if self.targets_only_player.is_some() {
                        matches_player
                    } else {
                        matches_object
                    }
                };

                if !entry.targets.iter().all(matches_target) {
                    return false;
                }
            }
        }

        true
    }

    fn matches_layered_tail(
        &self,
        subject: &LayeredSubject<'_>,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        self.matches_shared_tail(subject, ctx, game, None)
    }

    fn matches_internal(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        allow_calculated_pt: bool,
        view: Option<&crate::derived_view::DerivedGameView<'_>>,
    ) -> bool {
        // Specific object check
        if let Some(id) = self.specific
            && object.id != id
        {
            return false;
        }

        if self.source && ctx.source.is_none_or(|source_id| object.id != source_id) {
            return false;
        }

        if self.put_onto_battlefield_with_source
            && ctx.source.is_none_or(|source_id| {
                !game.was_put_onto_battlefield_with_source(source_id, object.id)
            })
        {
            return false;
        }

        if self.created_with_source {
            let source_stable_id = ctx
                .source
                .and_then(|source_id| game.object(source_id).map(|source| source.stable_id))
                .or_else(|| ctx.source_snapshot.as_ref().map(|source| source.stable_id));
            if source_stable_id.is_none_or(|source_stable_id| {
                !game.was_token_created_with_source(source_stable_id, object.stable_id)
            }) {
                return false;
            }
        }

        if self.was_dealt_damage_by_source_this_game {
            let Some(source) = ctx.source else {
                return false;
            };
            if !game.source_dealt_damage_to_object_this_game(source, object.id) {
                return false;
            }
        }

        if let Some(constraint) = &self.counters_put_on_this_turn
            && counters_put_on_exact_object_this_turn(game, object.id, constraint, ctx)
                < constraint.minimum
        {
            return false;
        }

        if let Some(targetability) = &self.could_be_targeted_by
            && !object_could_be_targeted_by(object.id, targetability, ctx, game)
        {
            return false;
        }

        if !self.any_of.is_empty()
            && !self
                .any_of
                .iter()
                .any(|filter| filter.matches_internal(object, ctx, game, allow_calculated_pt, view))
        {
            return false;
        }

        if self.entered_since_your_last_turn_ended && !game.is_summoning_sick(object.id) {
            return false;
        }

        if self.didnt_enter_battlefield_this_turn
            && game
                .turn_store
                .turn_history
                .object_entered_battlefield_controller_this_turn(object.stable_id)
                .is_some()
        {
            return false;
        }

        if self.entered_battlefield_this_turn || self.entered_battlefield_controller.is_some() {
            if object.zone != Zone::Battlefield {
                return false;
            }
            let Some(entry_controller) = game
                .turn_store
                .turn_history
                .object_entered_battlefield_controller_this_turn(object.stable_id)
            else {
                return false;
            };
            if let Some(filter) = &self.entered_battlefield_controller
                && !filter.matches_player(entry_controller, ctx)
            {
                return false;
            }
        }

        if self.entered_graveyard_from_battlefield_this_turn
            && (object.zone != Zone::Graveyard
                || !game
                    .turn_store
                    .turn_history
                    .object_was_put_into_graveyard_from_battlefield_this_turn(object.stable_id))
        {
            return false;
        }

        if self.entered_graveyard_from_library_this_turn
            && (object.zone != Zone::Graveyard
                || !game
                    .turn_store
                    .turn_history
                    .object_was_put_into_graveyard_from_zone_this_turn(
                        object.stable_id,
                        Zone::Library,
                    ))
        {
            return false;
        }

        if self.entered_graveyard_this_turn
            && (object.zone != Zone::Graveyard
                || !game
                    .turn_store
                    .turn_history
                    .object_was_put_into_graveyard_this_turn(object.stable_id))
        {
            return false;
        }

        if self.surveilled_this_turn
            && !game
                .turn_store
                .turn_history
                .object_was_surveilled_this_turn(object.stable_id)
        {
            return false;
        }

        if let Some(player_filter) = &self.discarded_or_cycled_this_turn_by {
            let matches_player = game.players.iter().any(|player| {
                player.is_in_game()
                    && player_filter.matches_player(player.id, ctx)
                    && game
                        .turn_store
                        .turn_history
                        .object_was_discarded_or_cycled_by_this_turn(
                            object.id,
                            object.stable_id,
                            player.id,
                        )
            });
            if !matches_player {
                return false;
            }
        }

        if self.was_dealt_damage_this_turn && !game.creature_was_damaged_this_turn(object.id) {
            return false;
        }

        if self.dealt_damage_this_turn && !game.source_dealt_damage_this_turn(object.id) {
            return false;
        }

        if let Some(damager) = &self.dealt_damage_by_source_this_turn {
            let Some(source) = ctx.source else {
                return false;
            };
            let damage_source = match damager {
                ironsmith_core::DamagedBySource::ThisCreature => Some(source),
                ironsmith_core::DamagedBySource::EquippedCreature
                | ironsmith_core::DamagedBySource::EnchantedCreature => game
                    .object(source)
                    .and_then(|obj| obj.attached_to.as_ref())
                    .and_then(|target| match target {
                        crate::object::AttachmentTarget::Object(id) => Some(*id),
                        _ => None,
                    }),
            };
            let Some(damage_source) = damage_source else {
                return false;
            };
            if !game
                .turn_store
                .turn_history
                .creature_was_damaged_by_source_identity_this_turn(
                    object.id,
                    Some(object.stable_id),
                    damage_source,
                    game.object(damage_source).map(|obj| obj.stable_id),
                )
            {
                return false;
            }
        }

        if self.was_dealt_damage_by_source_this_game {
            let Some(source) = ctx.source else {
                return false;
            };
            if !game.source_dealt_damage_to_object_this_game(source, object.id) {
                return false;
            }
        }

        if let Some(player_filter) = &self.dealt_damage_to_player_this_turn {
            let dealt_damage_to_matching_player = game.players.iter().any(|player| {
                player.is_in_game()
                    && player_filter.matches_player(player.id, ctx)
                    && game.source_dealt_damage_to_player_this_turn(object.id, player.id)
            });
            if !dealt_damage_to_matching_player {
                return false;
            }
        }

        if self.drawn_this_turn
            && !game
                .turn_store
                .turn_history
                .object_was_drawn_this_turn(object.id)
        {
            return false;
        }

        // Zone check (with special handling for stack entries)
        let wants_stack = self.zone == Some(Zone::Stack)
            || self.stack_kind.is_some()
            || self.excluded_cast_origin_zone.is_some()
            || self.target_count.is_some()
            || self.targets_only_player.is_some()
            || self.targets_only_object.is_some()
            || self.targets_player.is_some()
            || self.targets_object.is_some()
            || (self.zone.is_some_and(|zone| zone != Zone::Stack) && object.zone == Zone::Stack);

        let mut stack_entry = None;
        if wants_stack {
            stack_entry = game.stack.iter().find(|e| e.object_id == object.id);
            let can_treat_stack_object_as_spell_without_entry = object.zone == Zone::Stack
                && self.stack_kind == Some(StackObjectKind::Spell)
                && ctx.caster.is_some()
                && self.target_count.is_none()
                && self.targets_only_player.is_none()
                && self.targets_only_object.is_none()
                && self.targets_player.is_none()
                && self.targets_object.is_none();
            if (self.zone == Some(Zone::Stack) || self.stack_kind.is_some())
                && stack_entry.is_none()
                && !can_treat_stack_object_as_spell_without_entry
            {
                return false;
            }
        }

        if let Some(zone) = &self.zone
            && *zone != Zone::Stack
        {
            if object.zone == Zone::Stack {
                // For stack spells, non-stack zone filters mean
                // "cast from <zone>" (e.g. "target spell cast from a graveyard").
                // A live StackEntry's casting method is authoritative even
                // before the SpellCastEvent has been appended to turn
                // history. Restrict that early window to an explicitly typed
                // spell so an arbitrary stack object cannot acquire origin
                // semantics from a non-stack zone filter.
                if self.stack_kind != Some(StackObjectKind::Spell)
                    && game
                        .turn_store
                        .turn_history
                        .spell_cast_order(object.id)
                        .is_none()
                {
                    return false;
                }
                let Some(entry) = stack_entry else {
                    return false;
                };
                let Some(cast_from_zone) = stack_spell_cast_origin_zone(object, entry) else {
                    return false;
                };
                if cast_from_zone != *zone {
                    return false;
                }
            } else if object.zone != *zone {
                return false;
            }
        }

        if let Some(excluded_zone) = self.excluded_cast_origin_zone {
            if object.zone != Zone::Stack {
                return false;
            }
            if stack_entry.and_then(|entry| stack_spell_cast_origin_zone(object, entry))
                == Some(excluded_zone)
            {
                return false;
            }
        }

        if let Some(kind) = self.stack_kind {
            if let Some(entry) = stack_entry {
                if !Self::stack_entry_matches_kind(entry, kind) {
                    return false;
                }
            } else if !(object.zone == Zone::Stack
                && kind == StackObjectKind::Spell
                && ctx.caster.is_some())
            {
                return false;
            }
        }

        let needs_pt = self.uses_power_or_toughness_characteristics();
        let needs_non_pt = self.uses_non_pt_battlefield_characteristics();
        let should_consider_adjusted_object = allow_calculated_pt && (needs_pt || needs_non_pt);
        let should_calculate_chars = should_consider_adjusted_object
            && match view {
                Some(view) if object.zone == Zone::Battlefield => {
                    needs_pt || view.requires_battlefield_characteristic_calculation(object.id)
                }
                Some(_) => true,
                None => true,
            };
        let calculated_chars: Option<std::sync::Arc<CalculatedCharacteristics>> =
            if should_calculate_chars {
                if object.zone == Zone::Battlefield {
                    view.and_then(|view| {
                        if needs_pt
                            || view.requires_battlefield_characteristic_calculation(object.id)
                        {
                            view.calculated_characteristics_arc(object.id)
                        } else {
                            None
                        }
                    })
                    .or_else(|| game.calculated_characteristics_arc(object.id))
                } else {
                    view.and_then(|view| view.current_characteristics_arc(object.id))
                        .or_else(|| {
                            game.current_characteristics(object.id)
                                .map(std::sync::Arc::new)
                        })
                }
            } else {
                None
            };
        let calculated_chars_ref = calculated_chars.as_deref();
        let layered_subject_storage =
            calculated_chars_ref.map(|chars| LayeredSubject { object, chars });
        let layered_subject = layered_subject_storage.as_ref();
        let object_card_types = filter_card_types(object, calculated_chars_ref);
        let object_subtypes = filter_subtypes(object, calculated_chars_ref);
        let object_supertypes = filter_supertypes(object, calculated_chars_ref);
        let object_colors = filter_colors(object, calculated_chars_ref);

        if self.modified {
            if object.zone != Zone::Battlefield || !object_card_types.contains(&CardType::Creature)
            {
                return false;
            }

            let has_counters = object.counters.values().any(|count| *count > 0);
            let has_equipment = object.attachments.iter().any(|attachment_id| {
                game.object(*attachment_id).is_some_and(|attachment| {
                    filter_object_has_subtype_with_view(
                        attachment,
                        Subtype::Equipment,
                        allow_calculated_pt,
                        view,
                        game,
                    )
                })
            });
            let has_controlled_aura = ctx.you.is_some_and(|you| {
                object.attachments.iter().any(|attachment_id| {
                    game.object(*attachment_id).is_some_and(|attachment| {
                        game.current_controller(*attachment_id)
                            .is_some_and(|controller| controller == you)
                            && filter_object_has_subtype_with_view(
                                attachment,
                                Subtype::Aura,
                                allow_calculated_pt,
                                view,
                                game,
                            )
                    })
                })
            });
            if !(has_counters || has_equipment || has_controlled_aura) {
                return false;
            }
        }

        if self.suspected && (object.zone != Zone::Battlefield || !game.is_suspected(object.id)) {
            return false;
        }

        // Controller check
        if let Some(controller_filter) = &self.controller
            && !game
                .current_controller(object.id)
                .is_some_and(|controller| {
                    player_filter_matches_game(controller_filter, controller, game, ctx)
                })
        {
            return false;
        }

        let mut resolved_cast_player = None;

        // Caster check
        if let Some(caster_filter) = &self.cast_by {
            let cast_player = ctx.caster.or_else(|| {
                if object.zone == Zone::Stack {
                    stack_entry.map(|entry| entry.controller)
                } else {
                    None
                }
            });
            let Some(cast_player) = cast_player else {
                return false;
            };
            if !caster_filter.matches_player(cast_player, ctx) {
                return false;
            }
            resolved_cast_player = Some(cast_player);
        }

        if self.cast_this_turn
            && game
                .turn_store
                .turn_history
                .spell_cast_order(object.id)
                .is_none()
        {
            return false;
        }

        if let Some(source_filter) = &self.mana_from_source_spent_to_cast {
            let tag = ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG;
            if object.cast_tagged_objects.get(tag).is_none_or(|sources| {
                !mana_from_matching_source_was_spent_to_cast(source_filter, sources, ctx, game)
            }) {
                return false;
            }
        }

        if self.first_spell_cast_each_turn
            && !first_matching_spell_cast_each_turn_matches(
                self,
                object.id,
                ctx,
                game,
                resolved_cast_player,
            )
        {
            return false;
        }
        if let Some(ordinal) = self.spell_cast_ordinal_each_turn
            && !matching_spell_cast_ordinal_each_turn_matches(
                self,
                ordinal,
                object.id,
                ctx,
                game,
                resolved_cast_player,
            )
        {
            return false;
        }

        // Owner check
        if let Some(owner_filter) = &self.owner
            && !player_filter_matches_game(owner_filter, object.owner, game, ctx)
        {
            return false;
        }

        if self.type_or_subtype_union {
            let type_match = !self.card_types.is_empty()
                && self
                    .card_types
                    .iter()
                    .any(|t| object_card_types.contains(t));
            let subtype_match = !self.subtypes.is_empty()
                && self
                    .subtypes
                    .iter()
                    .any(|t| filter_subject_matches_subtype(object, layered_subject, *t, game));
            if (!self.card_types.is_empty() || !self.subtypes.is_empty())
                && !(type_match || subtype_match)
            {
                return false;
            }
        } else if !self.card_types.is_empty()
            && !self
                .card_types
                .iter()
                .any(|t| object_card_types.contains(t))
        {
            return false;
        }

        // Card types (must have all if specified)
        if !self.all_card_types.is_empty()
            && !self
                .all_card_types
                .iter()
                .all(|t| object_card_types.contains(t))
        {
            return false;
        }

        // Excluded card types (must have none of these)
        if self
            .excluded_card_types
            .iter()
            .any(|t| object_card_types.contains(t))
        {
            return false;
        }

        // Subtypes (must have at least one if specified)
        if !self.type_or_subtype_union
            && !self.subtypes.is_empty()
            && !self
                .subtypes
                .iter()
                .any(|t| filter_subject_matches_subtype(object, layered_subject, *t, game))
        {
            return false;
        }
        // Compound subtype phrases such as "Eldrazi Spawn" require every
        // authored subtype, unlike the inclusive-any `subtypes` collection.
        if !self.all_subtypes.is_empty()
            && !self
                .all_subtypes
                .iter()
                .all(|t| filter_subject_matches_subtype(object, layered_subject, *t, game))
        {
            return false;
        }

        // Excluded subtypes (must have none of these)
        if self
            .excluded_subtypes
            .iter()
            .any(|t| filter_subject_matches_subtype(object, layered_subject, *t, game))
        {
            return false;
        }
        if self.chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            if self.has_chosen_type_this_way_surface() {
                let Some(chosen_types) = game.chosen_subtypes(source) else {
                    return false;
                };
                if !chosen_types
                    .iter()
                    .any(|chosen_type| object_subtypes.contains(chosen_type))
                {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_subtype(source) {
                if !object_subtypes.contains(&chosen_type) {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_card_type(source) {
                if !object_card_types.contains(&chosen_type) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if self.chosen_land_type {
            let Some(chosen_type) = ctx.source.and_then(|source| game.chosen_land_type(source))
            else {
                return false;
            };
            if !filter_subject_matches_subtype(object, layered_subject, chosen_type, game) {
                return false;
            }
        }
        if self.has_basic_land_type
            && !object_subtypes
                .iter()
                .any(|subtype| subtype.is_basic_land_type())
        {
            return false;
        }
        if self.has_nonbasic_land_type
            && !object_subtypes
                .iter()
                .any(|subtype| subtype.is_land_subtype() && !subtype.is_basic_land_type())
        {
            return false;
        }
        if self.chosen_card_type {
            let Some(chosen_type) = ctx.source.and_then(|source| game.chosen_card_type(source))
            else {
                return false;
            };
            if !object_card_types.contains(&chosen_type) {
                return false;
            }
        }
        if self.excluded_chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            if let Some(chosen_type) = game.chosen_subtype(source) {
                if object_subtypes.contains(&chosen_type) {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_card_type(source) {
                if object_card_types.contains(&chosen_type) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if self.excluded_any_chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            let Some(chosen_types) = game.chosen_subtypes(source) else {
                return false;
            };
            if chosen_types
                .iter()
                .any(|chosen_type| object_subtypes.contains(chosen_type))
            {
                return false;
            }
        }

        // Supertypes (must have at least one if specified)
        if !self.supertypes.is_empty()
            && !self
                .supertypes
                .iter()
                .any(|t| object_supertypes.contains(t))
        {
            return false;
        }

        // Excluded supertypes (must have none of these)
        if self
            .excluded_supertypes
            .iter()
            .any(|t| object_supertypes.contains(t))
        {
            return false;
        }

        // Color check
        if let Some(required_colors) = self.required_colors
            && !object_colors.contains_all(required_colors)
        {
            return false;
        }
        if let Some(required_colors) = &self.colors
            && required_colors.intersection(object_colors).is_empty()
        {
            return false;
        }
        if self.chosen_color {
            let Some(chosen_color) = ctx.source.and_then(|source| game.chosen_color(source)) else {
                return false;
            };
            if !object_colors.contains(chosen_color) {
                return false;
            }
        }
        if let Some(card_name) = &self.colors_chosen_while_drafting_named {
            let Some(player) = ctx.you else {
                return false;
            };
            let drafted = game.draft_chosen_colors(player, card_name);
            if drafted.intersection(object_colors).is_empty() {
                return false;
            }
        }

        // Excluded colors check
        if !self.excluded_colors.is_empty()
            && !self.excluded_colors.intersection(object_colors).is_empty()
        {
            return false;
        }

        // Colorless check
        if self.colorless && !object_colors.is_empty() {
            return false;
        }

        // Multicolored check
        if self.multicolored && object_colors.count() < 2 {
            return false;
        }

        // Monocolored check
        if self.monocolored && object_colors.count() != 1 {
            return false;
        }

        if let Some(require_all_colors) = self.all_colors {
            let is_all_colors = object_colors.count() == 5;
            if require_all_colors != is_all_colors {
                return false;
            }
        }

        if let Some(require_exactly_two_colors) = self.exactly_two_colors {
            let is_exactly_two_colors = object_colors.count() == 2;
            if require_exactly_two_colors != is_exactly_two_colors {
                return false;
            }
        }
        if let Some(color_count_cmp) = &self.color_count {
            let color_count = object_colors.count() as i32;
            if !color_count_cmp.satisfies_with_context(color_count, game, ctx, stack_entry) {
                return false;
            }
        }

        let is_historic = object_card_types.contains(&CardType::Artifact)
            || object_supertypes.contains(&Supertype::Legendary)
            || object_subtypes.contains(&Subtype::Saga);
        if self.historic && !is_historic {
            return false;
        }
        if self.nonhistoric && is_historic {
            return false;
        }

        // Token/nontoken check
        if self.token && object.kind != ObjectKind::Token {
            return false;
        }
        if self.nontoken && object.kind == ObjectKind::Token {
            return false;
        }
        if let Some(require_face_down) = self.face_down
            && game.is_face_down(object.id) != require_face_down
        {
            return false;
        }
        if self.foretold && !game.is_foretold(object.id) {
            return false;
        }

        // "Other" ordinarily excludes announced target objects. When the
        // filter itself is an exact tagged-set reference, however, it means
        // the other member of that set relative to a temporarily rebound
        // source (for example, each of two chosen creatures affecting the
        // other). In that shape, exclude the source rather than the full
        // announced target set.
        let other_member_of_tagged_set = self.other
            && self.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Those)
            && self.tagged_constraints.len() == 1
            && self.tagged_constraints[0].relation == TaggedOpbjectRelation::IsTaggedObject;
        if self.other
            && (ctx.target_objects.is_empty() || other_member_of_tagged_set)
            && let Some(source_id) = ctx.source
            && object.id == source_id
        {
            return false;
        }
        if self.other
            && !other_member_of_tagged_set
            && ctx
                .target_objects
                .iter()
                .any(|target| target.object_id == object.id || target.stable_id == object.stable_id)
        {
            return false;
        }
        if self.is_target_object
            && !ctx
                .target_objects
                .iter()
                .any(|target| target.object_id == object.id || target.stable_id == object.stable_id)
        {
            return false;
        }

        let is_tapped = game.is_tapped(object.id);
        if self.tapped && !is_tapped {
            return false;
        }
        if self.untapped && is_tapped {
            return false;
        }
        if self.enlist_eligible && !object_is_enlist_eligible(game, object.id) {
            return false;
        }
        if self.attacking
            && !game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_attacking(combat, object.id))
        {
            return false;
        }
        if self.attacking_alone {
            let Some(combat) = game.combat.as_ref() else {
                return false;
            };
            let controller = game.controller_of_id(object.id);
            if !crate::combat_state::is_attacking(combat, object.id)
                || combat
                    .attackers
                    .iter()
                    .filter(|attacker| game.controller_of_id(attacker.creature) == controller)
                    .count()
                    != 1
            {
                return false;
            }
        }
        if self.attacked_this_turn && !game.creature_attacked_this_turn(object.id) {
            return false;
        }
        if self.ability_activated_this_turn
            && !game
                .turn_store
                .turn_history
                .activated_abilities_this_turn
                .iter()
                .any(|(source, _)| *source == object.id)
        {
            return false;
        }
        if self.blocked_this_turn && !game.creature_blocked_this_turn(object.id) {
            return false;
        }
        if self.didnt_attack_this_turn && game.creature_attacked_this_turn(object.id) {
            return false;
        }
        if self.could_have_attacked_this_turn && !crate::rules::combat::can_attack(object, game) {
            return false;
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let defending_player = if self.attacking_player_only {
                attacking_player_for_object(object.id, game)
            } else {
                attacking_defending_player_for_object(object.id, game)
            };
            let Some(defending_player) = defending_player else {
                return false;
            };
            if !player_filter.matches_player(defending_player, ctx) {
                return false;
            }
        }
        if let Some(player_filter) = &self.protected_by {
            let Some(protector) = game.battle_protector(object.id) else {
                return false;
            };
            if !player_filter.matches_player(protector, ctx) {
                return false;
            }
        }
        if self.blocking
            && !game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_blocking(combat, object.id))
        {
            return false;
        }
        if self.nonattacking
            && game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_attacking(combat, object.id))
        {
            return false;
        }
        if self.nonblocking
            && game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_blocking(combat, object.id))
        {
            return false;
        }
        if self.blocked
            && !game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_blocked(combat, object.id))
        {
            return false;
        }
        if self.unblocked
            && !game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_unblocked(combat, object.id))
        {
            return false;
        }
        if let Some(blocker_ref) = &self.blocked_by
            && !creature_was_blocked_by_ref(game, ctx, object.id, blocker_ref)
        {
            return false;
        }
        if self.blocked_by_source {
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(combat) = &game.combat else {
                return false;
            };
            if !combat
                .blockers
                .get(&object.id)
                .is_some_and(|blockers| blockers.contains(&source_id))
            {
                return false;
            }
        }
        if self.in_combat_with_source && !object_is_in_combat_with_source_lki(game, ctx, object.id)
        {
            return false;
        }
        if let Some(reference) = &self.in_combat_with {
            let partners = resolve_object_ref_ids(reference, ctx);
            let Some(combat) = &game.combat else {
                return false;
            };
            if partners.is_empty()
                || !partners.iter().any(|partner| {
                    crate::combat_state::get_blockers(combat, *partner).contains(&object.id)
                        || crate::combat_state::get_blocked_attacker(combat, *partner)
                            .is_some_and(|attacker| attacker == object.id)
                })
            {
                return false;
            }
        }

        // Power check
        if let Some(power_cmp) = &self.power {
            if let Some(power) = resolve_layered_object_power_for_filter(
                object,
                calculated_chars_ref,
                game,
                self.power_reference,
                allow_calculated_pt,
            ) {
                if !power_cmp.satisfies_with_context(power, game, ctx, stack_entry) {
                    return false;
                }
            } else {
                return false; // No power means not a creature
            }
        }
        if let Some(power_parity) = self.power_parity {
            if let Some(power) = resolve_layered_object_power_for_filter(
                object,
                calculated_chars_ref,
                game,
                self.power_reference,
                allow_calculated_pt,
            ) {
                if !power_parity.matches(power, game, ctx) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if self.power_greater_than_base_power {
            let Some(effective_power) = resolve_layered_object_power_for_filter(
                object,
                calculated_chars_ref,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            let Some(base_power) = resolve_layered_object_power_for_filter(
                object,
                calculated_chars_ref,
                game,
                PtReference::Base,
                allow_calculated_pt,
            ) else {
                return false;
            };
            if effective_power <= base_power {
                return false;
            }
        }
        if let Some(relation) = self.power_toughness_relation {
            let Some(power) = resolve_layered_object_power_for_filter(
                object,
                calculated_chars_ref,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            let Some(toughness) = resolve_layered_object_toughness_for_filter(
                object,
                calculated_chars_ref,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            match relation {
                PowerToughnessRelation::PowerGreaterThanToughness if power <= toughness => {
                    return false;
                }
                PowerToughnessRelation::ToughnessGreaterThanPower if toughness <= power => {
                    return false;
                }
                PowerToughnessRelation::NotEqual if power == toughness => return false,
                _ => {}
            }
        }

        if let Some(relation) = self.power_relative_to_source {
            let Some(candidate_power) = resolve_layered_object_power_for_filter(
                object,
                calculated_chars_ref,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(source_obj) = game.object(source_id) else {
                return false;
            };
            let Some(source_power) = resolve_object_power_for_filter(
                source_obj,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            match relation {
                SourcePowerRelation::LessThanSource => {
                    if candidate_power >= source_power {
                        return false;
                    }
                }
            }
        }

        // Toughness check
        if let Some(toughness_cmp) = &self.toughness {
            if let Some(toughness) = resolve_layered_object_toughness_for_filter(
                object,
                calculated_chars_ref,
                game,
                self.toughness_reference,
                allow_calculated_pt,
            ) {
                if !toughness_cmp.satisfies_with_context(toughness, game, ctx, stack_entry) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(total_cmp) = &self.total_power_toughness {
            let Some(power) = resolve_layered_object_power_for_filter(
                object,
                calculated_chars_ref,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            let Some(toughness) = resolve_layered_object_toughness_for_filter(
                object,
                calculated_chars_ref,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            if !total_cmp.satisfies_with_context(power + toughness, game, ctx, stack_entry) {
                return false;
            }
        }

        // Mana value check
        if let Some(mv_cmp) = &self.mana_value {
            let mv = object_mana_value_for_filter(object);
            if !mv_cmp.satisfies_with_context(mv, game, ctx, stack_entry) {
                return false;
            }
        }
        if let Some(mana_value_parity) = self.mana_value_parity {
            let mv = object_mana_value_for_filter(object);
            if !mana_value_parity.matches(mv, game, ctx) {
                return false;
            }
        }
        if let Some(counter_type) = self.mana_value_eq_counters_on_source {
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(source) = game.object(source_id) else {
                return false;
            };
            let required = source.counters.get(&counter_type).copied().unwrap_or(0) as i32;
            let mv = object_mana_value_for_filter(object);
            if mv != required {
                return false;
            }
        }
        if let Some(total_counters_parity) = self.total_counters_parity {
            let total_counters = object.counters.values().copied().sum::<u32>() as i32;
            if !total_counters_parity.matches(total_counters, game, ctx) {
                return false;
            }
        }

        // Has mana cost check (must have a non-empty mana cost)
        if self.has_mana_cost
            && !(object.zone == Zone::Stack
                && (self.zone == Some(Zone::Stack)
                    || self.stack_kind == Some(StackObjectKind::Spell)))
        {
            match &object.mana_cost {
                Some(mc) if !mc.is_empty() => {} // Has a mana cost, OK
                _ => return false,               // No mana cost or empty
            }
        }
        if self.has_phyrexian_mana_symbol
            && !object.mana_cost.as_ref().is_some_and(|cost| {
                cost.pips().iter().any(|pip| {
                    pip.iter()
                        .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::Life(_)))
                })
            })
        {
            return false;
        }

        // No X in cost check
        if self.no_x_in_cost
            && let Some(mc) = &object.mana_cost
            && mc.has_x()
        {
            return false;
        }
        if self.has_x_in_cost && !object.mana_cost.as_ref().is_some_and(|cost| cost.has_x()) {
            return false;
        }

        if let Some(sticker) = self.sticker
            && game.sticker_count_on_object(object.id, sticker, None) == 0
        {
            return false;
        }

        if let Some(subject) = layered_subject {
            self.matches_shared_tail(subject, ctx, game, stack_entry)
        } else {
            self.matches_shared_tail(object, ctx, game, stack_entry)
        }
    }

    fn stack_entry_matches_kind(
        entry: &crate::game_state::StackEntry,
        kind: StackObjectKind,
    ) -> bool {
        match kind {
            StackObjectKind::Spell => !entry.is_ability,
            StackObjectKind::Ability => entry.is_ability,
            StackObjectKind::ActivatedAbility => {
                entry.is_ability && entry.triggering_event.is_none()
            }
            StackObjectKind::TriggeredAbility => {
                entry.is_ability && entry.triggering_event.is_some()
            }
            StackObjectKind::SpellOrAbility => true,
        }
    }

    fn tagged_constraint_requires_existing_tag(relation: TaggedOpbjectRelation) -> bool {
        !matches!(
            relation,
            TaggedOpbjectRelation::IsNotTaggedObject
                | TaggedOpbjectRelation::DifferentNameFromTagged
                | TaggedOpbjectRelation::SharesMostCommonPermanentColor
        )
    }

    /// Check if a snapshot matches this filter.
    ///
    /// This is used for LKI/tagged-object comparisons where the object
    /// may no longer be available in the game state.
    fn matches_snapshot(
        &self,
        snapshot: &crate::snapshot::ObjectSnapshot,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        if ctx
            .you
            .is_some_and(|observer| !game.snapshot_is_within_range(observer, snapshot, ctx.source))
        {
            return false;
        }
        if let Some(id) = self.specific
            && snapshot.object_id != id
        {
            return false;
        }

        if !self.any_of.is_empty()
            && !self
                .any_of
                .iter()
                .any(|filter| filter.matches_snapshot(snapshot, ctx, game))
        {
            return false;
        }

        if self.source
            && ctx
                .source
                .is_none_or(|source_id| snapshot.object_id != source_id)
        {
            return false;
        }

        if self.put_onto_battlefield_with_source
            && ctx.source.is_none_or(|source_id| {
                !game.was_put_onto_battlefield_with_source(source_id, snapshot.object_id)
            })
        {
            return false;
        }

        if self.created_with_source {
            let source_stable_id = ctx
                .source
                .and_then(|source_id| game.object(source_id).map(|source| source.stable_id))
                .or_else(|| ctx.source_snapshot.as_ref().map(|source| source.stable_id));
            if source_stable_id.is_none_or(|source_stable_id| {
                !game.was_token_created_with_source(source_stable_id, snapshot.stable_id)
            }) {
                return false;
            }
        }

        if let Some(constraint) = &self.counters_put_on_this_turn
            && counters_put_on_exact_object_this_turn(game, snapshot.object_id, constraint, ctx)
                < constraint.minimum
        {
            return false;
        }

        if let Some(targetability) = &self.could_be_targeted_by
            && !object_could_be_targeted_by(snapshot.object_id, targetability, ctx, game)
        {
            return false;
        }

        if self.entered_since_your_last_turn_ended && !game.is_summoning_sick(snapshot.object_id) {
            return false;
        }
        if self.didnt_enter_battlefield_this_turn
            && game
                .turn_store
                .turn_history
                .object_entered_battlefield_controller_this_turn(snapshot.stable_id)
                .is_some()
        {
            return false;
        }
        if self.entered_graveyard_from_library_this_turn
            && (snapshot.zone != Zone::Graveyard
                || !game
                    .turn_store
                    .turn_history
                    .object_was_put_into_graveyard_from_zone_this_turn(
                        snapshot.stable_id,
                        Zone::Library,
                    ))
        {
            return false;
        }

        // Zone check
        if let Some(zone) = &self.zone
            && snapshot.zone != *zone
        {
            if snapshot.zone == Zone::Stack
                && *zone != Zone::Stack
                && self.stack_kind == Some(StackObjectKind::Spell)
            {
                let cast_origin = game
                    .cast_origin_snapshot(snapshot.object_id)
                    .map(|origin| origin.zone)
                    .or_else(|| {
                        let object = game.object(snapshot.object_id)?;
                        let entry = game
                            .stack
                            .iter()
                            .find(|entry| entry.object_id == snapshot.object_id)?;
                        stack_spell_cast_origin_zone(object, entry)
                    });
                if cast_origin != Some(*zone) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(excluded_zone) = self.excluded_cast_origin_zone {
            if snapshot.zone != Zone::Stack {
                return false;
            }
            let cast_origin = if snapshot.kind == ObjectKind::SpellCopy {
                None
            } else {
                game.cast_origin_snapshot(snapshot.object_id)
                    .map(|origin| origin.zone)
                    .or_else(|| {
                        let object = game.object(snapshot.object_id)?;
                        let entry = game
                            .stack
                            .iter()
                            .find(|entry| entry.object_id == snapshot.object_id)?;
                        stack_spell_cast_origin_zone(object, entry)
                    })
            };
            if cast_origin == Some(excluded_zone) {
                return false;
            }
        }

        // Controller check
        if let Some(controller_filter) = &self.controller
            && !player_filter_matches_game(controller_filter, snapshot.controller, game, ctx)
        {
            return false;
        }

        // Caster check
        if let Some(caster_filter) = &self.cast_by {
            let cast_player = ctx.caster.or_else(|| {
                if snapshot.zone == Zone::Stack {
                    Some(snapshot.controller)
                } else {
                    None
                }
            });
            let Some(cast_player) = cast_player else {
                return false;
            };
            if !caster_filter.matches_player(cast_player, ctx) {
                return false;
            }
        }

        if self.cast_this_turn
            && snapshot.cast_order_this_turn.is_none()
            && game
                .turn_store
                .turn_history
                .spell_cast_order(snapshot.object_id)
                .is_none()
        {
            return false;
        }

        if let Some(source_filter) = &self.mana_from_source_spent_to_cast
            && !mana_from_matching_source_was_spent_to_cast(
                source_filter,
                &snapshot.mana_sources_spent_to_cast,
                ctx,
                game,
            )
        {
            return false;
        }

        if self.first_spell_cast_each_turn
            && !first_matching_spell_cast_each_turn_matches(
                self,
                snapshot.object_id,
                ctx,
                game,
                None,
            )
        {
            return false;
        }
        if let Some(ordinal) = self.spell_cast_ordinal_each_turn
            && !matching_spell_cast_ordinal_each_turn_matches(
                self,
                ordinal,
                snapshot.object_id,
                ctx,
                game,
                None,
            )
        {
            return false;
        }

        // LKI filters must retain the same damage-history semantics as live
        // object filters. Death triggers are matched against the departing
        // object's snapshot, so omitting this check widens "a creature dealt
        // damage by that creature" to every matching creature that dies.
        if let Some(damager) = &self.dealt_damage_by_source_this_turn {
            let Some(source) = ctx.source else {
                return false;
            };
            let damage_source = match damager {
                ironsmith_core::DamagedBySource::ThisCreature => Some(source),
                ironsmith_core::DamagedBySource::EquippedCreature
                | ironsmith_core::DamagedBySource::EnchantedCreature => game
                    .object(source)
                    .and_then(|obj| obj.attached_to.as_ref())
                    .and_then(|target| match target {
                        crate::object::AttachmentTarget::Object(id) => Some(*id),
                        _ => None,
                    }),
            };
            let Some(damage_source) = damage_source else {
                return false;
            };
            let damage_source_stable_id = game
                .object(damage_source)
                .map(|object| object.stable_id)
                .or_else(|| {
                    if damage_source == source {
                        ctx.source_snapshot
                            .as_ref()
                            .map(|snapshot| snapshot.stable_id)
                    } else {
                        None
                    }
                });
            if !game
                .turn_store
                .turn_history
                .creature_was_damaged_by_source_identity_this_turn(
                    snapshot.object_id,
                    Some(snapshot.stable_id),
                    damage_source,
                    damage_source_stable_id,
                )
            {
                return false;
            }
        }

        // Owner check
        if let Some(owner_filter) = &self.owner
            && !player_filter_matches_game(owner_filter, snapshot.owner, game, ctx)
        {
            return false;
        }

        // Card types (must have at least one if specified)
        if !self.card_types.is_empty()
            && !self
                .card_types
                .iter()
                .any(|t| snapshot.card_types.contains(t))
        {
            return false;
        }

        // Card types (must have all if specified)
        if !self.all_card_types.is_empty()
            && !self
                .all_card_types
                .iter()
                .all(|t| snapshot.card_types.contains(t))
        {
            return false;
        }

        // Excluded card types (must have none of these)
        if self
            .excluded_card_types
            .iter()
            .any(|t| snapshot.card_types.contains(t))
        {
            return false;
        }

        // Subtypes (must have at least one if specified)
        if !self.subtypes.is_empty()
            && !self
                .subtypes
                .iter()
                .any(|t| snapshot_matches_subtype(snapshot, *t, game))
        {
            return false;
        }
        if !self.all_subtypes.is_empty()
            && !self
                .all_subtypes
                .iter()
                .all(|t| snapshot_matches_subtype(snapshot, *t, game))
        {
            return false;
        }

        // Excluded subtypes (must have none of these)
        if self
            .excluded_subtypes
            .iter()
            .any(|t| snapshot_matches_subtype(snapshot, *t, game))
        {
            return false;
        }
        if self.chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            if self.has_chosen_type_this_way_surface() {
                let Some(chosen_types) = game.chosen_subtypes(source) else {
                    return false;
                };
                if !chosen_types
                    .iter()
                    .any(|chosen_type| snapshot.subtypes.contains(chosen_type))
                {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_subtype(source) {
                if !snapshot.subtypes.contains(&chosen_type) {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_card_type(source) {
                if !snapshot.card_types.contains(&chosen_type) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if self.chosen_land_type {
            let Some(chosen_type) = ctx.source.and_then(|source| game.chosen_land_type(source))
            else {
                return false;
            };
            if !snapshot_matches_subtype(snapshot, chosen_type, game) {
                return false;
            }
        }
        if self.has_basic_land_type
            && !snapshot
                .subtypes
                .iter()
                .any(|subtype| subtype.is_basic_land_type())
        {
            return false;
        }
        if self.has_nonbasic_land_type
            && !snapshot
                .subtypes
                .iter()
                .any(|subtype| subtype.is_land_subtype() && !subtype.is_basic_land_type())
        {
            return false;
        }
        if self.chosen_card_type {
            let Some(chosen_type) = ctx.source.and_then(|source| game.chosen_card_type(source))
            else {
                return false;
            };
            if !snapshot.card_types.contains(&chosen_type) {
                return false;
            }
        }
        if self.excluded_chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            if let Some(chosen_type) = game.chosen_subtype(source) {
                if snapshot.subtypes.contains(&chosen_type) {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_card_type(source) {
                if snapshot.card_types.contains(&chosen_type) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if self.excluded_any_chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            let Some(chosen_types) = game.chosen_subtypes(source) else {
                return false;
            };
            if chosen_types
                .iter()
                .any(|chosen_type| snapshot.subtypes.contains(chosen_type))
            {
                return false;
            }
        }

        // Supertypes (must have at least one if specified)
        if !self.supertypes.is_empty()
            && !self
                .supertypes
                .iter()
                .any(|t| snapshot.supertypes.contains(t))
        {
            return false;
        }

        // Excluded supertypes (must have none of these)
        if self
            .excluded_supertypes
            .iter()
            .any(|t| snapshot.supertypes.contains(t))
        {
            return false;
        }

        // Color check
        if let Some(required_colors) = self.required_colors
            && !snapshot.colors.contains_all(required_colors)
        {
            return false;
        }
        if let Some(required_colors) = &self.colors
            && required_colors.intersection(snapshot.colors).is_empty()
        {
            return false;
        }
        if self.chosen_color {
            let Some(chosen_color) = ctx.source.and_then(|source| game.chosen_color(source)) else {
                return false;
            };
            if !snapshot.colors.contains(chosen_color) {
                return false;
            }
        }
        if let Some(card_name) = &self.colors_chosen_while_drafting_named {
            let Some(player) = ctx.you else {
                return false;
            };
            let drafted = game.draft_chosen_colors(player, card_name);
            if drafted.intersection(snapshot.colors).is_empty() {
                return false;
            }
        }

        // Excluded colors check
        if !self.excluded_colors.is_empty()
            && !self
                .excluded_colors
                .intersection(snapshot.colors)
                .is_empty()
        {
            return false;
        }

        // Colorless check
        if self.colorless && !snapshot.colors.is_empty() {
            return false;
        }

        // Multicolored check
        if self.multicolored && snapshot.colors.count() < 2 {
            return false;
        }

        // Monocolored check
        if self.monocolored && snapshot.colors.count() != 1 {
            return false;
        }

        if let Some(require_all_colors) = self.all_colors {
            let is_all_colors = snapshot.colors.count() == 5;
            if require_all_colors != is_all_colors {
                return false;
            }
        }

        if let Some(require_exactly_two_colors) = self.exactly_two_colors {
            let is_exactly_two_colors = snapshot.colors.count() == 2;
            if require_exactly_two_colors != is_exactly_two_colors {
                return false;
            }
        }
        if let Some(color_count_cmp) = &self.color_count {
            let color_count = snapshot.colors.count() as i32;
            if !color_count_cmp.satisfies_with_context(color_count, game, ctx, None) {
                return false;
            }
        }

        let is_historic = snapshot.card_types.contains(&CardType::Artifact)
            || snapshot.supertypes.contains(&Supertype::Legendary)
            || snapshot.subtypes.contains(&Subtype::Saga);
        if self.historic && !is_historic {
            return false;
        }
        if self.nonhistoric && is_historic {
            return false;
        }

        // Token/nontoken check
        if self.token && !snapshot.is_token {
            return false;
        }
        if self.nontoken && snapshot.is_token {
            return false;
        }
        if let Some(require_face_down) = self.face_down
            && snapshot.face_down != require_face_down
        {
            return false;
        }
        if self.foretold && !game.is_foretold(snapshot.object_id) {
            return false;
        }

        // See the live-object branch above: a tagged-set `other` is relative
        // to the rebound source, not an instruction to exclude every announced
        // member of that same set.
        let other_member_of_tagged_set = self.other
            && self.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Those)
            && self.tagged_constraints.len() == 1
            && self.tagged_constraints[0].relation == TaggedOpbjectRelation::IsTaggedObject;
        if self.other
            && (ctx.target_objects.is_empty() || other_member_of_tagged_set)
            && let Some(source_id) = ctx.source
        {
            if snapshot.object_id == source_id {
                return false;
            }
            if let Some(source) = game.object(source_id)
                && snapshot.stable_id == source.stable_id
            {
                return false;
            }
        }
        if self.other
            && !other_member_of_tagged_set
            && ctx.target_objects.iter().any(|target| {
                target.object_id == snapshot.object_id || target.stable_id == snapshot.stable_id
            })
        {
            return false;
        }
        if self.is_target_object
            && !ctx.target_objects.iter().any(|target| {
                target.object_id == snapshot.object_id || target.stable_id == snapshot.stable_id
            })
        {
            return false;
        }

        if self.suspected
            && (snapshot.zone != Zone::Battlefield || !game.is_suspected(snapshot.object_id))
        {
            return false;
        }

        if self.tapped && !snapshot.tapped {
            return false;
        }
        if self.untapped && snapshot.tapped {
            return false;
        }
        if self.attacking && !snapshot.attacking {
            return false;
        }
        if self.attacking_alone {
            let Some(combat) = game.combat.as_ref() else {
                return false;
            };
            let controller = game.controller_of_id(snapshot.object_id);
            if !snapshot.attacking
                || combat
                    .attackers
                    .iter()
                    .filter(|attacker| game.controller_of_id(attacker.creature) == controller)
                    .count()
                    != 1
            {
                return false;
            }
        }
        if self.nonattacking && snapshot.attacking {
            return false;
        }
        if self.enlist_eligible && !object_is_enlist_eligible(game, snapshot.object_id) {
            return false;
        }
        if self.attacked_this_turn && !game.creature_attacked_this_turn(snapshot.object_id) {
            return false;
        }
        if self.ability_activated_this_turn
            && !game
                .turn_store
                .turn_history
                .activated_abilities_this_turn
                .iter()
                .any(|(source, _)| *source == snapshot.object_id)
        {
            return false;
        }
        if self.blocked_this_turn && !game.creature_blocked_this_turn(snapshot.object_id) {
            return false;
        }
        if self.didnt_attack_this_turn && game.creature_attacked_this_turn(snapshot.object_id) {
            return false;
        }
        if self.could_have_attacked_this_turn
            && !game
                .object(snapshot.object_id)
                .is_some_and(|object| crate::rules::combat::can_attack(object, game))
        {
            return false;
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let defending_player = if self.attacking_player_only {
                attacking_player_for_object(snapshot.object_id, game)
            } else {
                attacking_defending_player_for_object(snapshot.object_id, game)
            };
            let Some(defending_player) = defending_player else {
                return false;
            };
            if !player_filter.matches_player(defending_player, ctx) {
                return false;
            }
        }
        if let Some(player_filter) = &self.protected_by {
            let Some(protector) = game.battle_protector(snapshot.object_id) else {
                return false;
            };
            if !player_filter.matches_player(protector, ctx) {
                return false;
            }
        }
        if self.in_combat_with_source
            && !object_is_in_combat_with_source_lki(game, ctx, snapshot.object_id)
        {
            return false;
        }
        if let Some(reference) = &self.in_combat_with {
            let partners = resolve_object_ref_ids(reference, ctx);
            let Some(combat) = &game.combat else {
                return false;
            };
            if partners.is_empty()
                || !partners.iter().any(|partner| {
                    crate::combat_state::get_blockers(combat, *partner)
                        .contains(&snapshot.object_id)
                        || crate::combat_state::get_blocked_attacker(combat, *partner)
                            .is_some_and(|attacker| attacker == snapshot.object_id)
                })
            {
                return false;
            }
        }
        if let Some(blocker_ref) = &self.blocked_by
            && !creature_was_blocked_by_ref(game, ctx, snapshot.object_id, blocker_ref)
        {
            return false;
        }
        if self.blocked_by_source {
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(combat) = &game.combat else {
                return false;
            };
            if !combat
                .blockers
                .get(&snapshot.object_id)
                .is_some_and(|blockers| blockers.contains(&source_id))
            {
                return false;
            }
        }

        // Power check
        if let Some(power_cmp) = &self.power {
            if let Some(power) = resolve_snapshot_power_for_filter(snapshot, self.power_reference) {
                if !power_cmp.satisfies_with_context(power, game, ctx, None) {
                    return false;
                }
            } else {
                return false; // No power means not a creature
            }
        }
        if let Some(power_parity) = self.power_parity {
            if let Some(power) = resolve_snapshot_power_for_filter(snapshot, self.power_reference) {
                if !power_parity.matches(power, game, ctx) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if self.power_greater_than_base_power {
            let Some(effective_power) =
                resolve_snapshot_power_for_filter(snapshot, PtReference::Effective)
            else {
                return false;
            };
            let Some(base_power) = resolve_snapshot_power_for_filter(snapshot, PtReference::Base)
            else {
                return false;
            };
            if effective_power <= base_power {
                return false;
            }
        }
        if let Some(relation) = self.power_toughness_relation {
            let Some(power) = resolve_snapshot_power_for_filter(snapshot, PtReference::Effective)
            else {
                return false;
            };
            let Some(toughness) =
                resolve_snapshot_toughness_for_filter(snapshot, PtReference::Effective)
            else {
                return false;
            };
            match relation {
                PowerToughnessRelation::PowerGreaterThanToughness if power <= toughness => {
                    return false;
                }
                PowerToughnessRelation::ToughnessGreaterThanPower if toughness <= power => {
                    return false;
                }
                PowerToughnessRelation::NotEqual if power == toughness => return false,
                _ => {}
            }
        }

        if let Some(relation) = self.power_relative_to_source {
            let Some(candidate_power) =
                resolve_snapshot_power_for_filter(snapshot, PtReference::Effective)
            else {
                return false;
            };
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(source_obj) = game.object(source_id) else {
                return false;
            };
            let Some(source_power) = game
                .calculated_power(source_id)
                .or_else(|| source_obj.power())
            else {
                return false;
            };
            match relation {
                SourcePowerRelation::LessThanSource => {
                    if candidate_power >= source_power {
                        return false;
                    }
                }
            }
        }

        // Toughness check
        if let Some(toughness_cmp) = &self.toughness {
            if let Some(toughness) =
                resolve_snapshot_toughness_for_filter(snapshot, self.toughness_reference)
            {
                if !toughness_cmp.satisfies_with_context(toughness, game, ctx, None) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(total_cmp) = &self.total_power_toughness {
            let Some(power) = resolve_snapshot_power_for_filter(snapshot, PtReference::Effective)
            else {
                return false;
            };
            let Some(toughness) =
                resolve_snapshot_toughness_for_filter(snapshot, PtReference::Effective)
            else {
                return false;
            };
            if !total_cmp.satisfies_with_context(power + toughness, game, ctx, None) {
                return false;
            }
        }

        // Mana value check
        if let Some(mv_cmp) = &self.mana_value {
            let mv = snapshot_mana_value_for_filter(snapshot);
            if !mv_cmp.satisfies_with_context(mv, game, ctx, None) {
                return false;
            }
        }
        if let Some(mana_value_parity) = self.mana_value_parity {
            let mv = snapshot_mana_value_for_filter(snapshot);
            if !mana_value_parity.matches(mv, game, ctx) {
                return false;
            }
        }
        if let Some(counter_type) = self.mana_value_eq_counters_on_source {
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(source) = game.object(source_id) else {
                return false;
            };
            let required = source.counters.get(&counter_type).copied().unwrap_or(0) as i32;
            let mv = snapshot_mana_value_for_filter(snapshot);
            if mv != required {
                return false;
            }
        }
        if let Some(total_counters_parity) = self.total_counters_parity {
            let total_counters = snapshot.counters.values().copied().sum::<u32>() as i32;
            if !total_counters_parity.matches(total_counters, game, ctx) {
                return false;
            }
        }

        // Has mana cost check (must have a non-empty mana cost)
        if self.has_mana_cost
            && !(snapshot.zone == Zone::Stack
                && (self.zone == Some(Zone::Stack)
                    || self.stack_kind == Some(StackObjectKind::Spell)))
        {
            match &snapshot.mana_cost {
                Some(mc) if !mc.is_empty() => {}
                _ => return false,
            }
        }
        if self.has_phyrexian_mana_symbol
            && !snapshot.mana_cost.as_ref().is_some_and(|cost| {
                cost.pips().iter().any(|pip| {
                    pip.iter()
                        .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::Life(_)))
                })
            })
        {
            return false;
        }

        // No X in cost check
        if self.no_x_in_cost
            && let Some(mc) = &snapshot.mana_cost
            && mc.has_x()
        {
            return false;
        }
        if self.has_x_in_cost
            && !snapshot
                .mana_cost
                .as_ref()
                .is_some_and(crate::mana::ManaCost::has_x)
        {
            return false;
        }

        if let Some(sticker) = self.sticker
            && game.sticker_count_on_object(snapshot.object_id, sticker, None) == 0
        {
            return false;
        }

        self.matches_shared_tail(snapshot, ctx, game, None)
    }

    /// Generate a human-readable description of this filter.
    ///
    /// Used primarily for trigger display text.
    fn description(&self) -> String {
        if !self.could_produce_mana.is_empty()
            || self
                .any_of
                .iter()
                .any(|branch| !branch.could_produce_mana.is_empty())
        {
            return ObjectFilter::description(self);
        }
        // The specialized core describers below don't model has_x_in_cost;
        // re-attach the qualifier so "target spell with {X} in its mana cost"
        // survives whichever shape claims the rest of the filter.
        let with_x_in_cost = |description: String| {
            if self.has_x_in_cost && !description.contains("{X}") {
                format!("{description} with {{X}} in its mana cost")
            } else {
                description
            }
        };
        let any_of_keyword_clause =
            describe_simple_any_of_keyword_clause(&self.any_of, self.union_connective());
        let owner_or_controller_clause =
            describe_you_own_or_control_union(&self.any_of, self.union_connective());
        if let Some(description) =
            ironsmith_core::filter_model::describe_relative_characteristic_list_filter(self)
        {
            return with_x_in_cost(description);
        }
        if let Some(description) =
            ironsmith_core::filter_model::describe_branch_scoped_card_type_union(self)
        {
            return with_x_in_cost(description);
        }
        if let Some(description) =
            ironsmith_core::filter_model::describe_controlled_battlefield_and_owned_nonbattlefield_card_union(
                self,
            )
        {
            return with_x_in_cost(description);
        }
        if let Some(description) =
            ironsmith_core::filter_model::describe_owned_nonbattlefield_card_union(self)
        {
            return with_x_in_cost(description);
        }
        if let Some(description) =
            ironsmith_core::filter_model::describe_owner_scoped_zone_union(self)
        {
            return with_x_in_cost(description);
        }
        if let Some(description) = owner_or_controller_clause {
            return with_x_in_cost(description);
        }
        if any_of_keyword_clause.is_none() && !self.any_of.is_empty() {
            let descriptions = self
                .any_of
                .iter()
                .map(ObjectFilter::description)
                .collect::<Vec<_>>();
            return match self.union_connective() {
                ObjectFilterUnionConnective::Or => descriptions.join(" or "),
                ObjectFilterUnionConnective::AndOr => match descriptions.as_slice() {
                    [] => String::new(),
                    [single] => single.clone(),
                    [first, second] => format!("{first} and/or {second}"),
                    _ => {
                        let mut descriptions = descriptions;
                        let last = descriptions
                            .pop()
                            .expect("union has at least three branches");
                        format!("{}, and/or {last}", descriptions.join(", "))
                    }
                },
            };
        }

        let mut parts = Vec::new();
        let mut post_noun_qualifiers: Vec<String> = Vec::new();
        let append_token_after_type = self.token;
        let mut controller_suffix: Option<String> = None;
        let mut owner_suffix: Option<String> = None;
        let other_source_surface_text = if self.other && !self.source {
            self.source_surface
                .as_ref()
                .map(crate::target::SourceReferenceSurface::display_text)
        } else {
            None
        };

        // Handle "other" modifier
        if self.other && other_source_surface_text.is_none() {
            parts.push("another".to_string());
        }
        if self.is_target_object {
            parts.push("target".to_string());
        }
        let has_target_tag = self.tagged_constraints.iter().any(|constraint| {
            matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                && constraint.tag.as_str().starts_with("targeted")
        });
        if has_target_tag {
            parts.push("target".to_string());
        }
        let has_chosen_tag = self.tagged_constraints.iter().any(|constraint| {
            matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                && constraint.tag.as_str() == "__chosen_objects__"
        });
        if has_chosen_tag {
            parts.push("the chosen".to_string());
        }
        if self.source {
            parts.push("this".to_string());
        }
        if self.modified {
            parts.push("modified".to_string());
        }
        if self.suspected {
            parts.push("suspected".to_string());
        }

        let has_leading_determiner =
            self.other || self.is_target_object || has_target_tag || has_chosen_tag || self.source;

        // Handle controller
        if let Some(ref ctrl) = self.controller {
            match ctrl {
                PlayerFilter::You => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("you control".to_string());
                }
                PlayerFilter::NotYou => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("you don't control".to_string());
                }
                PlayerFilter::Opponent => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("an opponent controls".to_string());
                }
                PlayerFilter::Any => {}
                PlayerFilter::Active => parts.push("the active player's".to_string()),
                PlayerFilter::EffectController => {
                    parts.push("the player who cast this spell's".to_string())
                }
                PlayerFilter::Specific(_) => parts.push("a specific player's".to_string()),
                PlayerFilter::MostLifeTied => {
                    parts.push("the player with the most life's".to_string())
                }
                PlayerFilter::LowestLifeTied => {
                    parts.push("the player with the lowest life's".to_string())
                }
                PlayerFilter::MostCardsInHand => {
                    parts.push("the player with the most cards in hand's".to_string())
                }
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::HasMoreLifeThanYou { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::ControlsMost { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::MaxSpeed { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => parts.push(format!(
                    "a player who cast one or more {} spells this turn's",
                    card_type.to_string().to_ascii_lowercase()
                )),
                PlayerFilter::AttackedBySourceThisTurn => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::WasDealtDamageBySourceThisGame { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::LostLifeThisTurn { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    post_noun_qualifiers.push(format!("controlled by {}", ctrl.description()));
                }
                PlayerFilter::ChosenPlayer => parts.push("the chosen player's".to_string()),
                PlayerFilter::TaggedPlayer(_) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string());
                }
                PlayerFilter::Teammate => parts.push("a teammate's".to_string()),
                PlayerFilter::PlayerToYourLeft | PlayerFilter::PlayerToYourRight => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::Defending => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("defending player controls".to_string());
                }
                PlayerFilter::Attacking => parts.push("an attacking player's".to_string()),
                PlayerFilter::DamagedPlayer => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string());
                }
                PlayerFilter::IteratedPlayer => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string())
                }
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix =
                        Some("that player or that object's controller controls".to_string())
                }
                PlayerFilter::Excluding { .. } if ctrl.is_your_team() => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("your team controls".to_string());
                }
                PlayerFilter::Excluding { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::Target(inner) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some(if inner.relative_target_exclusion_base().is_some() {
                        "another target player controls".to_string()
                    } else {
                        let inner_desc = describe_player_filter(inner.as_ref());
                        let target_kind = inner_desc
                            .strip_prefix("a ")
                            .or_else(|| inner_desc.strip_prefix("an "))
                            .unwrap_or(&inner_desc);
                        format!("target {target_kind} controls")
                    });
                }
                PlayerFilter::AliasedTarget(_) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string());
                }
                PlayerFilter::ControllerOf(_) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("its controller controls".to_string());
                }
                PlayerFilter::OwnerOf(_) => parts.push("an owner's".to_string()),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    parts.push("that player's".to_string())
                }
            }
        }

        if let Some(cast_by) = &self.cast_by {
            post_noun_qualifiers.push(format!("cast by {}", describe_player_filter(cast_by)));
        }
        if let Some(zone) = self.excluded_cast_origin_zone {
            let origin = match zone {
                Zone::Hand => "its owner's hand".to_string(),
                Zone::Graveyard => "a graveyard".to_string(),
                Zone::Library => "a library".to_string(),
                Zone::Exile => "exile".to_string(),
                Zone::Command => "the command zone".to_string(),
                Zone::Battlefield => "the battlefield".to_string(),
                Zone::Stack => "the stack".to_string(),
                Zone::Ante => "ante".to_string(),
                Zone::OutsideGame => "outside the game".to_string(),
            };
            post_noun_qualifiers.push(format!("that wasn't cast from {origin}"));
        }

        // Handle owner on object-level filters (battlefield/stack/any-zone object references).
        // Zone-restricted card references (e.g. "in your graveyard") already encode ownership.
        let owner_conveyed_by_zone = matches!(
            self.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        ) && !self.foretold;
        if !owner_conveyed_by_zone && let Some(ref owner) = self.owner {
            owner_suffix = Some(match owner {
                PlayerFilter::You => "you own".to_string(),
                PlayerFilter::NotYou => "you don't own".to_string(),
                PlayerFilter::Opponent => "an opponent owns".to_string(),
                PlayerFilter::Any => "a player owns".to_string(),
                PlayerFilter::Active => "the active player owns".to_string(),
                PlayerFilter::EffectController => "the player who cast this spell owns".to_string(),
                PlayerFilter::Specific(_) => "that player owns".to_string(),
                PlayerFilter::MostLifeTied => {
                    "the player with the most life or tied for most life owns".to_string()
                }
                PlayerFilter::LowestLifeTied => {
                    "the player with the lowest life or tied for lowest life owns".to_string()
                }
                PlayerFilter::MostCardsInHand => {
                    "the player who has the most cards in hand owns".to_string()
                }
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::HasMoreLifeThanYou { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::ControlsMost { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::MaxSpeed { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
                    "a player who cast one or more {} spells this turn owns",
                    card_type.to_string().to_ascii_lowercase()
                ),
                PlayerFilter::AttackedBySourceThisTurn => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::WasDealtDamageBySourceThisGame { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::LostLifeThisTurn { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::ChosenPlayer => "the chosen player owns".to_string(),
                PlayerFilter::TaggedPlayer(_) => "that player owns".to_string(),
                PlayerFilter::Teammate => "a teammate owns".to_string(),
                PlayerFilter::PlayerToYourLeft | PlayerFilter::PlayerToYourRight => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::Defending => "the defending player owns".to_string(),
                PlayerFilter::Attacking => "an attacking player owns".to_string(),
                PlayerFilter::DamagedPlayer => "that player owns".to_string(),
                PlayerFilter::IteratedPlayer => "that player owns".to_string(),
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    "that player or that object's controller owns".to_string()
                }
                PlayerFilter::Excluding { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::Target(inner) => {
                    format!("target {} owns", describe_player_filter(inner.as_ref()))
                }
                PlayerFilter::AliasedTarget(_) => "that player owns".to_string(),
                PlayerFilter::ControllerOf(_) => "that object's controller owns".to_string(),
                PlayerFilter::OwnerOf(_) => "that object's owner owns".to_string(),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    "that player owns".to_string()
                }
            });
        }

        // Handle token/nontoken
        if self.nontoken {
            parts.push("nontoken".to_string());
        }
        if let Some(face_down) = self.face_down {
            parts.push(if face_down {
                "face-down".to_string()
            } else {
                "face-up".to_string()
            });
        }
        if self.foretold {
            parts.push("foretold".to_string());
        }
        if let Some(colors) = self.required_colors {
            let mut color_words = Vec::new();
            if colors.contains(Color::White) {
                color_words.push("white");
            }
            if colors.contains(Color::Blue) {
                color_words.push("blue");
            }
            if colors.contains(Color::Black) {
                color_words.push("black");
            }
            if colors.contains(Color::Red) {
                color_words.push("red");
            }
            if colors.contains(Color::Green) {
                color_words.push("green");
            }
            if !color_words.is_empty() {
                parts.push(format!("both {}", color_words.join(" and ")));
            }
        } else if let Some(colors) = self.colors {
            if colors.contains_all(
                crate::color::Color::ALL
                    .into_iter()
                    .collect::<crate::color::ColorSet>(),
            ) {
                parts.push("colored".to_string());
            } else {
                let mut color_words = Vec::new();
                if colors.contains(crate::color::Color::White) {
                    color_words.push("white");
                }
                if colors.contains(crate::color::Color::Blue) {
                    color_words.push("blue");
                }
                if colors.contains(crate::color::Color::Black) {
                    color_words.push("black");
                }
                if colors.contains(crate::color::Color::Red) {
                    color_words.push("red");
                }
                if colors.contains(crate::color::Color::Green) {
                    color_words.push("green");
                }
                if !color_words.is_empty() {
                    parts.push(describe_filter_union_list(
                        color_words.into_iter().map(str::to_string).collect(),
                        self.union_connective(),
                        false,
                    ));
                }
            }
        }
        // Chosen-quality back-references follow the controller suffix in
        // oracle order ("creatures you control of the chosen type") — but
        // only when there IS one; zone qualifiers ("cards of that type from
        // their graveyard") keep the chosen phrase next to the noun.
        let defer_chosen_qualifiers = controller_suffix.is_some() || owner_suffix.is_some();
        let mut chosen_trailing_qualifiers: Vec<String> = Vec::new();
        let push_chosen_qualifier =
            |text: &str,
             post_noun_qualifiers: &mut Vec<String>,
             chosen_trailing_qualifiers: &mut Vec<String>| {
                if defer_chosen_qualifiers {
                    chosen_trailing_qualifiers.push(text.to_string());
                } else {
                    post_noun_qualifiers.push(text.to_string());
                }
            };
        if self.chosen_color {
            push_chosen_qualifier(
                "of the chosen color",
                &mut post_noun_qualifiers,
                &mut chosen_trailing_qualifiers,
            );
        }
        if let Some(card_name) = &self.colors_chosen_while_drafting_named {
            post_noun_qualifiers.push(format!(
                "that's one or more of the colors chosen as you drafted cards named {card_name}"
            ));
        }
        if let Some(sticker) = self.sticker {
            let sticker = match sticker {
                crate::events::KeywordActionKind::ArtSticker => "an art sticker",
                crate::events::KeywordActionKind::AbilitySticker => "an ability sticker",
                crate::events::KeywordActionKind::PowerToughnessSticker => {
                    "a power and toughness sticker"
                }
                crate::events::KeywordActionKind::NameSticker => "a name sticker",
                _ => "a sticker",
            };
            post_noun_qualifiers.push(format!("with {sticker} on it"));
        }
        if self.chosen_creature_type {
            push_chosen_qualifier(
                if self.has_chosen_type_this_way_surface() {
                    "of a type chosen this way"
                } else {
                    "of the chosen type"
                },
                &mut post_noun_qualifiers,
                &mut chosen_trailing_qualifiers,
            );
        }
        if self.chosen_card_type {
            push_chosen_qualifier(
                "of the chosen type",
                &mut post_noun_qualifiers,
                &mut chosen_trailing_qualifiers,
            );
        }
        if self.excluded_chosen_creature_type || self.excluded_any_chosen_creature_type {
            let qualifier = if self.has_chosen_type_this_way_surface() {
                "that aren't of a type chosen this way"
            } else {
                "that aren't of the chosen type"
            };
            push_chosen_qualifier(
                qualifier,
                &mut post_noun_qualifiers,
                &mut chosen_trailing_qualifiers,
            );
        }
        if !self.no_shared_creature_types_with.is_empty() {
            let comparison = self
                .no_shared_creature_types_with
                .iter()
                .map(|filter| ensure_filter_indefinite_article(filter.description()))
                .collect::<Vec<_>>()
                .join(" or ");
            post_noun_qualifiers.push(format!(
                "that doesn't share a creature type with {comparison}"
            ));
        }
        for relation in &self.characteristic_relations {
            let characteristics = relation
                .characteristics
                .iter()
                .map(|characteristic| characteristic.sharing_phrase())
                .collect::<Vec<_>>()
                .join(" or ");
            let verb = match relation.kind {
                ObjectCharacteristicRelationKind::SharesAny => "shares",
                ObjectCharacteristicRelationKind::SharesNone => "doesn't share",
            };
            post_noun_qualifiers.push(format!(
                "that {verb} {characteristics} with {}",
                relation.comparison_description()
            ));
        }
        if self.shares_creature_type_with_source {
            post_noun_qualifiers.push("that shares a creature type with this creature".to_string());
        }
        for constraint in &self.tagged_constraints {
            match constraint.relation {
                TaggedOpbjectRelation::IsTaggedObject
                | TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered => {
                    match constraint.tag.as_str() {
                        "it" | "__it__" | "blocking" => parts.push("that".to_string()),
                        "enchanted" => parts.push("enchanted".to_string()),
                        "equipped" => parts.push("equipped".to_string()),
                        "convoked_this_spell" => {
                            post_noun_qualifiers.push("that convoked this spell".to_string());
                        }
                        "improvised_this_spell" => {
                            post_noun_qualifiers.push("that improvised this spell".to_string());
                        }
                        "crewed_it_this_turn" => {
                            post_noun_qualifiers.push("that crewed it this turn".to_string());
                        }
                        "saddled_it_this_turn" => {
                            post_noun_qualifiers.push("that saddled it this turn".to_string());
                        }
                        crate::tag::SOURCE_EXILED_TAG => {
                            post_noun_qualifiers.push("exiled with this permanent".to_string());
                        }
                        _ => {}
                    }
                }
                TaggedOpbjectRelation::IsNotTaggedObject => {
                    parts.push("other".to_string());
                }
                TaggedOpbjectRelation::SameNameAsTagged => {
                    let antecedent = self
                        .same_name_antecedent_surface()
                        .map(SameNameAntecedentSurface::phrase)
                        .unwrap_or("it");
                    post_noun_qualifiers.push(format!("with the same name as {antecedent}"));
                }
                TaggedOpbjectRelation::DifferentNameFromTagged => {
                    post_noun_qualifiers
                        .push("with a different name from those objects".to_string());
                }
                TaggedOpbjectRelation::SameControllerAsTagged => {
                    post_noun_qualifiers.push("controlled by its controller".to_string());
                }
                TaggedOpbjectRelation::SameManaValueAsTagged => {
                    if constraint.tag.as_str().starts_with("sacrifice_cost_") {
                        post_noun_qualifiers.push(
                            "with the same mana value as the sacrificed creature".to_string(),
                        );
                    } else {
                        post_noun_qualifiers.push("with the same mana value as it".to_string());
                    }
                }
                TaggedOpbjectRelation::SameManaValueAsAnotherTagged => {
                    post_noun_qualifiers
                        .push("with the same mana value as another tagged object".to_string());
                }
                TaggedOpbjectRelation::ManaValueLteTagged => {
                    if self.union_surface.equal_or_lesser_mana_value() {
                        post_noun_qualifiers.push("with equal or lesser mana value".to_string());
                    } else if constraint.tag.as_str() == "triggering" {
                        post_noun_qualifiers
                            .push("with equal or lesser mana value than that spell".to_string());
                    } else {
                        post_noun_qualifiers.push(
                            "with mana value less than or equal to its mana value".to_string(),
                        );
                    }
                }
                TaggedOpbjectRelation::ManaValueLtTagged => {
                    post_noun_qualifiers.push("with lesser mana value than it".to_string());
                }
                TaggedOpbjectRelation::SharesColorWithTagged => {
                    post_noun_qualifiers.push("that shares a color with it".to_string());
                }
                TaggedOpbjectRelation::SharesMostCommonPermanentColor => {
                    post_noun_qualifiers.push(
                        "that shares a color with the most common color among all permanents or a color tied for most common"
                            .to_string(),
                    );
                }
                TaggedOpbjectRelation::SharesSubtypeWithTagged => {
                    post_noun_qualifiers.push("that shares a creature type with it".to_string());
                }
                TaggedOpbjectRelation::SharesSubtypeWithEachTagged => {
                    post_noun_qualifiers.push(
                        "that shares a creature type with each creature tapped this way"
                            .to_string(),
                    );
                }
                TaggedOpbjectRelation::SharesCardType => {
                    if constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG {
                        post_noun_qualifiers.push(
                            "that shares a card type with a card exiled with this permanent"
                                .to_string(),
                        );
                        continue;
                    }
                    if constraint.tag.as_str().starts_with("sacrificed_") {
                        post_noun_qualifiers.push(
                            "that shares a card type with the sacrificed permanent".to_string(),
                        );
                        continue;
                    }
                    post_noun_qualifiers.push("that shares a card type with it".to_string());
                }
                TaggedOpbjectRelation::SharesPermanentType => {
                    post_noun_qualifiers.push("that shares a permanent type with it".to_string());
                }
                TaggedOpbjectRelation::AttachedToTaggedObject => {
                    post_noun_qualifiers.push("attached to it".to_string());
                }
                TaggedOpbjectRelation::WasAttachedToTaggedObject => {
                    post_noun_qualifiers.push("that was attached to it".to_string());
                }
                TaggedOpbjectRelation::SoulbondPartnerOfTagged => {
                    post_noun_qualifiers.push("paired with it".to_string());
                }
                TaggedOpbjectRelation::SameStableId => {}
            }
        }
        if !self.supertypes.is_empty() {
            for supertype in &self.supertypes {
                parts.push(supertype.name().to_string());
            }
        }
        if !self.excluded_card_types.is_empty() {
            for card_type in &self.excluded_card_types {
                parts.push(format!("non{}", describe_card_type_word(*card_type)));
            }
        }
        if !self.excluded_supertypes.is_empty() {
            for supertype in &self.excluded_supertypes {
                parts.push(format!("non{}", supertype.name()));
            }
        }
        if !self.excluded_subtypes.is_empty() {
            let mut remaining = self.excluded_subtypes.clone();
            let outlaw_pack = [
                Subtype::Assassin,
                Subtype::Mercenary,
                Subtype::Pirate,
                Subtype::Rogue,
                Subtype::Warlock,
            ];
            if outlaw_pack
                .iter()
                .all(|subtype| remaining.contains(subtype))
            {
                parts.push("non-outlaw".to_string());
                remaining.retain(|subtype| !outlaw_pack.contains(subtype));
            }
            for subtype in &remaining {
                parts.push(format!("non-{}", subtype.to_string().to_ascii_lowercase()));
            }
        }
        if !self.excluded_colors.is_empty() {
            if self.excluded_colors.contains(crate::color::Color::White) {
                parts.push("nonwhite".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Blue) {
                parts.push("nonblue".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Black) {
                parts.push("nonblack".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Red) {
                parts.push("nonred".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Green) {
                parts.push("nongreen".to_string());
            }
        }
        if self.colorless {
            parts.push("colorless".to_string());
        }
        if self.multicolored {
            parts.push("multicolored".to_string());
        }
        if self.monocolored {
            parts.push("monocolored".to_string());
        }
        if let Some(all_colors) = self.all_colors {
            if all_colors {
                post_noun_qualifiers.push("that are all colors".to_string());
            } else {
                post_noun_qualifiers.push("that are not all colors".to_string());
            }
        }
        if let Some(exactly_two_colors) = self.exactly_two_colors {
            // Oracle order puts the color-count clause after the controller
            // suffix ("permanents you control that are exactly two colors").
            let clause = if exactly_two_colors {
                "that are exactly two colors"
            } else {
                "that are not exactly two colors"
            };
            push_chosen_qualifier(
                clause,
                &mut post_noun_qualifiers,
                &mut chosen_trailing_qualifiers,
            );
        }
        if self.historic {
            parts.push("historic".to_string());
        }
        if self.nonhistoric {
            post_noun_qualifiers.push("that's not historic".to_string());
        }
        if self.is_commander
            && !(self.card_types.is_empty()
                && self.all_card_types.is_empty()
                && self.subtypes.is_empty()
                && self.all_subtypes.is_empty()
                && !self.token)
        {
            // With no type noun, "commander" IS the noun ("Commanders you
            // control"), handled by the default-noun selection below.
            parts.push("commander".to_string());
        }
        if self.noncommander {
            parts.push("noncommander".to_string());
        }
        if self.blocked && self.unblocked {
            parts.push("blocked/unblocked".to_string());
        } else {
            if self.blocked {
                parts.push("blocked".to_string());
            }
            if self.unblocked {
                parts.push("unblocked".to_string());
            }
        }
        if let Some(blocker) = &self.blocked_by {
            let blocker_text = match blocker {
                ObjectRef::Target => "target creature",
                ObjectRef::Specific(_) => "that creature",
                ObjectRef::Tagged(tag) if tag.as_str() == "blocking" => "the blocking creature",
                ObjectRef::Tagged(_) => "one of those creatures",
            };
            post_noun_qualifiers.push(format!("blocked by {blocker_text} this turn"));
        }
        if self.blocked_by_source {
            post_noun_qualifiers.push("blocked by this creature this turn".to_string());
        }
        if let Some(combat_partner) = &self.blocked_or_was_blocked_by_this_turn {
            let mut partner_description = combat_partner.description();
            if combat_partner.card_types.as_slice() == [CardType::Creature]
                && !combat_partner.subtypes.is_empty()
            {
                partner_description = partner_description.replacen(" creature", "", 1);
            }
            post_noun_qualifiers.push(format!(
                "that blocked or was blocked by {} this turn",
                ensure_filter_indefinite_article(partner_description)
            ));
        }
        if self.tapped && self.untapped {
            parts.push("tapped/untapped".to_string());
        } else if self.tapped {
            parts.push("tapped".to_string());
        } else if self.untapped {
            parts.push("untapped".to_string());
        }
        if self.attacking && self.blocking {
            parts.push("attacking/blocking".to_string());
        } else {
            if self.attacking
                && !self.attacking_alone
                && self
                    .attacking_player_or_planeswalker_controlled_by
                    .is_none()
            {
                parts.push("attacking".to_string());
            }
            if self.blocking && !self.in_combat_with_source && self.in_combat_with.is_none() {
                parts.push("blocking".to_string());
            }
        }
        if self.attacking_alone {
            post_noun_qualifiers.push("that's attacking alone".to_string());
        }
        if self.attacked_this_turn {
            post_noun_qualifiers.push("that attacked this turn".to_string());
        }
        if self.ability_activated_this_turn {
            let clause = if self.card_types == [CardType::Planeswalker] {
                "that was activated this turn"
            } else {
                "that had an ability activated this turn"
            };
            post_noun_qualifiers.push(clause.to_string());
        }
        if self.blocked_this_turn {
            post_noun_qualifiers.push("that blocked this turn".to_string());
        }
        if self.didnt_attack_this_turn {
            let clause = if self.could_have_attacked_this_turn {
                "that didn't attack this turn, except for creatures that couldn't attack"
            } else if self.didnt_enter_battlefield_this_turn {
                "that didn't attack or enter this turn"
            } else {
                "that didn't attack this turn"
            };
            post_noun_qualifiers.push(clause.to_string());
        } else if self.could_have_attacked_this_turn {
            post_noun_qualifiers.push("that could have attacked this turn".to_string());
        }
        if let Some(with_attached) = &self.with_attached_object {
            let inner = with_attached.description();
            if inner.starts_with("another ") || inner.starts_with("other ") {
                post_noun_qualifiers.push(format!("with {inner} attached to it"));
            } else {
                let article =
                    if inner.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) {
                        "an"
                    } else {
                        "a"
                    };
                post_noun_qualifiers.push(format!("with {article} {inner} attached to it"));
            }
        }
        if let Some(without_attached) = &self.without_attached_object {
            let is_aura = without_attached.zone == Some(Zone::Battlefield)
                && without_attached.card_types == [CardType::Enchantment]
                && without_attached.subtypes == [Subtype::Aura]
                && {
                    let mut semantic = (**without_attached).clone();
                    semantic.zone = None;
                    semantic.card_types.clear();
                    semantic.subtypes.clear();
                    semantic == ObjectFilter::default()
                };
            if is_aura {
                post_noun_qualifiers.push("that isn't enchanted".to_string());
            } else {
                let inner = without_attached.description();
                let article =
                    if inner.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) {
                        "an"
                    } else {
                        "a"
                    };
                post_noun_qualifiers.push(format!("without {article} {inner} attached to it"));
            }
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let player_text = if matches!(player_filter, PlayerFilter::ChosenPlayer) {
                "the last chosen player".to_string()
            } else if self.attacking_player_only && matches!(player_filter, PlayerFilter::Defending)
            {
                "that player".to_string()
            } else {
                player_filter.description()
            };
            if self.attacking_player_only {
                let relation = if matches!(player_filter, PlayerFilter::ChosenPlayer) {
                    format!("attacking {player_text}")
                } else {
                    format!("that's attacking {player_text}")
                };
                post_noun_qualifiers.push(relation);
            } else {
                let controller_pronoun = if matches!(player_filter, PlayerFilter::You) {
                    "you"
                } else {
                    "they"
                };
                post_noun_qualifiers.push(format!(
                    "that's attacking {player_text} or a planeswalker {controller_pronoun} control"
                ));
            }
        }
        if let Some(player_filter) = &self.protected_by {
            let player = match player_filter {
                PlayerFilter::IteratedPlayer => "that player".to_string(),
                other => other.description(),
            };
            post_noun_qualifiers.push(format!("{player} protects"));
        }
        if self.in_combat_with_source {
            post_noun_qualifiers.push(if self.blocking {
                "blocking this creature".to_string()
            } else {
                "blocking or blocked by this creature".to_string()
            });
        }
        if let Some(reference) = &self.in_combat_with {
            let reference = match reference {
                ObjectRef::Target => "target creature",
                ObjectRef::Specific(_) => "that creature",
                ObjectRef::Tagged(tag) if tag.as_str() == "blocking" => "the blocking creature",
                ObjectRef::Tagged(_) => "that creature",
            };
            post_noun_qualifiers.push(if self.blocking {
                format!("blocking {reference}")
            } else {
                format!("blocking or blocked by {reference}")
            });
        }
        if self.nonattacking && self.nonblocking {
            parts.push("nonattacking, nonblocking".to_string());
        } else {
            if self.nonattacking {
                parts.push("nonattacking".to_string());
            }
            if self.enlist_eligible {
                parts.push("enlist-eligible".to_string());
            }
            if self.nonblocking {
                parts.push("nonblocking".to_string());
            }
        }
        if self.entered_since_your_last_turn_ended {
            post_noun_qualifiers.push("that entered since your last turn ended".to_string());
        }
        if self.didnt_enter_battlefield_this_turn && !self.didnt_attack_this_turn {
            post_noun_qualifiers.push("that didn't enter this turn".to_string());
        }
        if self.no_abilities {
            post_noun_qualifiers.push("with no abilities".to_string());
        }

        let subtype_implies_type = (!self.subtypes.is_empty() || !self.all_subtypes.is_empty())
            && matches!(self.zone, None | Some(Zone::Battlefield))
            && self.all_card_types.is_empty()
            && self.card_types.is_empty();

        let has_all_permanent_types = {
            let required = [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ];
            self.card_types.len() == required.len()
                && required
                    .iter()
                    .all(|card_type| self.card_types.contains(card_type))
        };

        let has_all_permanent_spell_types = {
            let required = [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Planeswalker,
                CardType::Battle,
            ];
            matches!(self.zone, Some(Zone::Stack))
                && matches!(self.stack_kind, Some(StackObjectKind::Spell))
                && self.card_types.len() == required.len()
                && required
                    .iter()
                    .all(|card_type| self.card_types.contains(card_type))
        };

        let stack_source_ability = matches!(self.zone, Some(Zone::Stack))
            && matches!(
                self.stack_kind,
                Some(
                    StackObjectKind::Ability
                        | StackObjectKind::ActivatedAbility
                        | StackObjectKind::TriggeredAbility
                )
            );

        let mut type_phrase = if !self.all_card_types.is_empty() {
            Some((
                true,
                self.all_card_types
                    .iter()
                    .map(|t| t.name().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        } else if !self.card_types.is_empty() {
            if stack_source_ability {
                let kind = self.stack_kind.unwrap_or(StackObjectKind::Ability);
                post_noun_qualifiers.push(format!(
                    "from {} source",
                    describe_card_type_source_phrase(&self.card_types, self.union_connective())
                ));
                Some((false, describe_stack_object_kind(kind).to_string()))
            } else if has_all_permanent_types || has_all_permanent_spell_types {
                Some((true, "permanent".to_string()))
            } else {
                let card_type_phrase = if self.has_conjunctive_set_surface() {
                    describe_conjunctive_filter_members(
                        self.card_types
                            .iter()
                            .map(|card_type| card_type.name().to_string())
                            .collect(),
                    )
                } else {
                    describe_card_type_list(&self.card_types, self.union_connective())
                };
                Some((true, card_type_phrase))
            }
        } else if !self.token && !subtype_implies_type {
            // Default noun depends on zone context.
            let default_noun = if self.source {
                match self.zone {
                    Some(Zone::Graveyard)
                    | Some(Zone::Hand)
                    | Some(Zone::Library)
                    | Some(Zone::Exile)
                    | Some(Zone::Command)
                    | Some(Zone::Ante)
                    | Some(Zone::OutsideGame) => "card",
                    _ => "source",
                }
            } else {
                match self.zone {
                    Some(Zone::Battlefield) | None if self.is_commander => "commander",
                    Some(Zone::Battlefield) | None => "permanent",
                    Some(Zone::Stack) => {
                        let kind = self.stack_kind.unwrap_or({
                            if self.has_mana_cost {
                                StackObjectKind::Spell
                            } else {
                                StackObjectKind::SpellOrAbility
                            }
                        });
                        describe_stack_object_kind(kind)
                    }
                    Some(Zone::Graveyard)
                    | Some(Zone::Hand)
                    | Some(Zone::Library)
                    | Some(Zone::Exile)
                    | Some(Zone::Command)
                    | Some(Zone::Ante)
                    | Some(Zone::OutsideGame) => "card",
                }
            };
            Some((false, default_noun.to_string()))
        } else {
            None
        };

        let subtype_parts = if !self.subtypes.is_empty() || !self.all_subtypes.is_empty() {
            let mut parts = if self.all_subtypes.is_empty() {
                Vec::new()
            } else {
                vec![
                    self.all_subtypes
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                ]
            };
            let mut remaining = self.subtypes.clone();
            let outlaw_pack = [
                Subtype::Assassin,
                Subtype::Mercenary,
                Subtype::Pirate,
                Subtype::Rogue,
                Subtype::Warlock,
            ];
            if outlaw_pack
                .iter()
                .all(|subtype| remaining.contains(subtype))
            {
                parts.push("outlaw".to_string());
                remaining.retain(|subtype| !outlaw_pack.contains(subtype));
            }
            parts.extend(remaining.iter().map(std::string::ToString::to_string));
            parts
        } else {
            Vec::new()
        };
        let subtype_phrase = (!subtype_parts.is_empty()).then(|| {
            let description = describe_filter_union_list(
                subtype_parts.clone(),
                self.union_connective(),
                self.has_serial_or_list_surface(),
            );
            if self.has_shared_indefinite_article_surface() {
                ensure_filter_indefinite_article(description)
            } else {
                description
            }
        });

        if let Some((type_is_card_type, phrase)) = type_phrase.as_mut()
            && *type_is_card_type
            && matches!(
                self.zone,
                Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
            )
            && !phrase.ends_with(" card")
        {
            phrase.push_str(" card");
        }
        if let Some((type_is_card_type, phrase)) = type_phrase.as_mut()
            && *type_is_card_type
            && matches!(self.zone, Some(Zone::Stack))
            && !phrase.ends_with(" spell")
        {
            phrase.push_str(" spell");
        }

        let creature_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Creature;
        let land_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Land
            && !matches!(self.zone, Some(Zone::Stack));
        if self.type_or_subtype_union {
            match (type_phrase, subtype_phrase) {
                (Some((_, mut type_phrase)), Some(subtype_phrase)) => {
                    let terminal_noun = self
                        .has_terminal_noun_after_type_subtype_union_surface()
                        .then(|| {
                            [" card", " spell"].into_iter().find_map(|suffix| {
                                type_phrase
                                    .strip_suffix(suffix)
                                    .map(|head| (head.to_string(), suffix.trim().to_string()))
                            })
                        })
                        .flatten();
                    if let Some((head, _)) = &terminal_noun {
                        type_phrase = head.clone();
                    }

                    let subtype_first = self.has_subtype_before_card_type_union_surface();
                    let preserve_individual_arms = subtype_first || terminal_noun.is_some();
                    let mut union_parts = if preserve_individual_arms {
                        subtype_parts
                    } else {
                        vec![subtype_phrase]
                    };
                    if subtype_first {
                        union_parts.push(type_phrase);
                    } else {
                        union_parts.insert(0, type_phrase);
                    }
                    let mut description = describe_filter_union_list(
                        union_parts,
                        self.union_connective(),
                        preserve_individual_arms,
                    );
                    if let Some((_, terminal_noun)) = terminal_noun {
                        description.push(' ');
                        description.push_str(&terminal_noun);
                    }
                    parts.push(description);
                }
                (Some((_, type_phrase)), None) => parts.push(type_phrase),
                (None, Some(subtype_phrase)) => parts.push(subtype_phrase),
                (None, None) => {}
            }
        } else {
            match (type_phrase, subtype_phrase) {
                (Some((_, type_phrase)), Some(subtype_phrase)) if creature_only => {
                    parts.push(subtype_phrase);
                    parts.push(type_phrase);
                }
                (Some((_, _type_phrase)), Some(subtype_phrase)) if land_only => {
                    parts.push(subtype_phrase);
                    if matches!(
                        self.zone,
                        Some(
                            Zone::Graveyard
                                | Zone::Hand
                                | Zone::Library
                                | Zone::Exile
                                | Zone::Command
                                | Zone::OutsideGame
                        )
                    ) {
                        parts.push("card".to_string());
                    }
                }
                (Some((type_is_card_type, type_phrase)), Some(subtype_phrase))
                    if !type_is_card_type && type_phrase == "card" =>
                {
                    parts.push(subtype_phrase);
                    parts.push(type_phrase);
                }
                (Some((_, type_phrase)), Some(subtype_phrase)) => {
                    parts.push(type_phrase);
                    parts.push(subtype_phrase);
                }
                (Some((_, type_phrase)), None) => parts.push(type_phrase),
                (None, Some(subtype_phrase)) => parts.push(subtype_phrase),
                (None, None) => {}
            }
        }
        if append_token_after_type {
            parts.push("token".to_string());
        }

        // Oracle places controller and owner scope immediately after the noun,
        // before restrictive qualifiers: "a creature you control with
        // deathtouch", not "a creature with deathtouch you control". Keep the
        // scope attached to the noun here so every later AST-derived
        // qualifier (power, mana value, abilities, counters, zones, and
        // tagged relationships) follows it consistently.
        match (controller_suffix.take(), owner_suffix.take()) {
            (Some(controller), Some(owner))
                if controller == "you control" && owner == "you own" =>
            {
                parts.push("you both own and control".to_string());
            }
            (Some(controller), Some(owner))
                if controller == "you control" && owner == "you don't own" =>
            {
                parts.push("you control but don't own".to_string());
            }
            (Some(controller), Some(owner))
                if controller == "that player controls" && owner == "that player owns" =>
            {
                parts.push("that player both owns and controls".to_string());
            }
            (Some(controller), Some(owner)) => {
                parts.push(format!("{owner} but {controller}"));
            }
            (Some(controller), None) => parts.push(controller),
            (None, Some(owner)) => parts.push(owner),
            (None, None) => {}
        }

        if !post_noun_qualifiers.is_empty() {
            parts.extend(post_noun_qualifiers);
        }
        if let Some(surface_text) = other_source_surface_text {
            parts.push(format!("other than {surface_text}"));
        }
        if self.distinct_names {
            parts.push("with different names".to_string());
        }
        if self.distinct_mana_values {
            parts.push("with different mana values".to_string());
        }
        if self.distinct_powers {
            parts.push("with different powers".to_string());
        }
        if self.one_per_card_type {
            parts.push("with at most one card of each card type".to_string());
        }

        // Handle name
        if let Some(ref name) = self.name {
            match (&controller_suffix, &owner_suffix) {
                (Some(controller), Some(owner)) => {
                    if controller == "you control" && owner == "you own" {
                        parts.push("you both own and control".to_string());
                    } else if controller == "you control" && owner == "you don't own" {
                        parts.push("you control but don't own".to_string());
                    } else if controller == "that player controls" && owner == "that player owns" {
                        parts.push("that player both owns and controls".to_string());
                    } else {
                        parts.push(controller.clone());
                        parts.push(owner.clone());
                    }
                }
                (Some(controller), None) => parts.push(controller.clone()),
                (None, Some(owner)) => parts.push(owner.clone()),
                (None, None) => {}
            }
            if name == "{chosen name}" {
                // The name is a runtime back-reference to a previously chosen
                // card name, not a literal card name.
                let subject = parts.join(" ");
                let article = if subject.starts_with("a ") || subject.starts_with("an ") {
                    ""
                } else {
                    "a "
                };
                return format!("{article}{subject} with that name");
            }
            let subject = parts.join(" ");
            let article = if subject.starts_with("a ") || subject.starts_with("an ") {
                ""
            } else {
                "a "
            };
            return format!("{article}{subject} named {name}");
        }
        if let Some(ref name) = self.excluded_name {
            let name = self.excluded_name_surface().unwrap_or(name);
            return format!("{} not named {}", parts.join(" "), name);
        }

        if self.power_toughness_relation.is_some()
            && owner_suffix.is_none()
            && let Some(controller) = controller_suffix.take()
        {
            parts.push(controller);
        }

        if let (Some(power), Some(toughness)) = (&self.power, &self.toughness)
            && let (Comparison::Equal(power_value), Comparison::Equal(toughness_value)) =
                (power, toughness)
            && self.power_reference == self.toughness_reference
        {
            let label = match self.power_reference {
                PtReference::Effective => "power and toughness",
                PtReference::Base => "base power and toughness",
            };
            parts.push(format!("with {label} {power_value}/{toughness_value}"));
        } else {
            if let Some(ref power) = self.power {
                let label = match self.power_reference {
                    PtReference::Effective => "power",
                    PtReference::Base => "base power",
                };
                parts.push(format!("with {label} {}", describe_comparison(power)));
            }
            if let Some(power_parity) = self.power_parity {
                let axis = match self.power_reference {
                    PtReference::Effective => "power",
                    PtReference::Base => "base power",
                };
                parts.push(power_parity.describe_axis(axis));
            }
            if self.power_greater_than_base_power {
                parts.push("with power greater than its base power".to_string());
            }
            if let Some(relation) = self.power_toughness_relation {
                match relation {
                    PowerToughnessRelation::PowerGreaterThanToughness => {
                        parts.push("with power greater than its toughness".to_string());
                    }
                    PowerToughnessRelation::ToughnessGreaterThanPower => {
                        parts.push("with toughness greater than its power".to_string());
                    }
                    PowerToughnessRelation::NotEqual => {
                        parts.push("with power and toughness that aren't equal".to_string());
                    }
                }
            }
            if let Some(relation) = self.power_relative_to_source {
                match relation {
                    SourcePowerRelation::LessThanSource => {
                        parts.push("with power less than this creature's power".to_string());
                    }
                }
            }
            if let Some(ref toughness) = self.toughness {
                let label = match self.toughness_reference {
                    PtReference::Effective => "toughness",
                    PtReference::Base => "base toughness",
                };
                parts.push(format!("with {label} {}", describe_comparison(toughness)));
            }
        }
        if let Some(ref total_power_toughness) = self.total_power_toughness {
            parts.push(format!(
                "with total power and toughness {}",
                describe_comparison(total_power_toughness)
            ));
        }
        if let Some(ref mana_value) = self.mana_value {
            parts.push(format!(
                "with mana value {}",
                describe_comparison(mana_value)
            ));
        }
        if self.has_x_in_cost {
            parts.push("with {X} in its mana cost".to_string());
        }
        if self.no_x_in_cost {
            parts.push("with no {X} in its mana cost".to_string());
        }
        if let Some(ref color_count) = self.color_count {
            parts.push(format!(
                "with color count {}",
                describe_comparison(color_count)
            ));
        }
        if let Some(mana_value_parity) = self.mana_value_parity {
            parts.push(mana_value_parity.describe_axis("mana value"));
        }
        if let Some(counter_type) = self.mana_value_eq_counters_on_source {
            parts.push(format!(
                "with mana value equal to the number of {} counters on this artifact",
                counter_type.description()
            ));
        }
        if let Some(clause) = any_of_keyword_clause {
            parts.push(format!("with {clause}"));
        }
        for ability in &self.static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("with {}", label));
            }
        }
        for marker in &self.ability_markers {
            parts.push(format!("with {}", marker.to_ascii_lowercase()));
        }
        if self.excluded_static_abilities.len() > 1 {
            // Oracle writes a multi-keyword exclusion as one serial clause
            // ("that doesn't have first strike, double strike, vigilance, or
            // haste"), never as repeated "without" parts.
            let labels = self
                .excluded_static_abilities
                .iter()
                .filter_map(|ability| describe_filter_static_ability(*ability))
                .collect::<Vec<_>>();
            if let [leading @ .., last] = labels.as_slice()
                && !leading.is_empty()
            {
                parts.push(format!(
                    "that doesn't have {}, or {last}",
                    leading.join(", ")
                ));
            }
        } else {
            for ability in &self.excluded_static_abilities {
                if let Some(label) = describe_filter_static_ability(*ability) {
                    parts.push(format!("without {}", label));
                }
            }
        }
        for marker in &self.excluded_ability_markers {
            parts.push(format!("without {}", marker.to_ascii_lowercase()));
        }
        if let Some(counter_requirement) = self.with_counter {
            let (one_or_more, plural_noun, plural_subject) = self.counter_requirement_surface();
            parts.push(format!(
                "with {}{} on {}",
                if one_or_more { "one or more " } else { "" },
                describe_counter_constraint(counter_requirement, plural_noun),
                if plural_subject { "them" } else { "it" }
            ));
        }
        if let Some(counter_exclusion) = self.without_counter {
            let (plural_noun, plural_subject) = self.counter_exclusion_surface();
            parts.push(format!(
                "without {} on {}",
                describe_counter_constraint(counter_exclusion, plural_noun),
                if plural_subject { "them" } else { "it" }
            ));
        }
        if let Some(total_counters_parity) = self.total_counters_parity {
            match total_counters_parity {
                ParityRequirement::Odd | ParityRequirement::Even => parts.push(format!(
                    "with an {} number of counters on it",
                    total_counters_parity.explicit_label().unwrap_or("")
                )),
                ParityRequirement::Chosen => {
                    parts.push("with a number of counters on it of the chosen quality".to_string())
                }
            }
        }
        if let Some(kind) = self.alternative_cast {
            parts.push(format!("with {}", describe_alternative_cast_kind(kind)));
        }
        if self.has_tap_activated_ability {
            parts.push("that has an activated ability with {T} in its cost".to_string());
        }

        let has_source_exiled_constraint = self.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        });
        if let Some(zone) = self.zone {
            let zone_name = match zone {
                Zone::Battlefield => None,
                Zone::Graveyard => Some("graveyard"),
                Zone::Hand => Some("hand"),
                Zone::Library => Some("library"),
                Zone::Exile => Some("exile"),
                Zone::Stack => None,
                Zone::Command => Some("command zone"),
                Zone::Ante => Some("ante"),
                Zone::OutsideGame => Some("outside the game"),
            };
            if zone == Zone::Exile && has_source_exiled_constraint {
                // Keep wording compact: "card exiled with this permanent" is
                // clearer than appending an extra "in exile" qualifier.
            } else if let Some(zone_name) = zone_name {
                if self.foretold && zone == Zone::Exile {
                    parts.push("in exile".to_string());
                } else if let Some(owner) = &self.owner {
                    parts.push(format!(
                        "in {} {}",
                        describe_possessive_player_filter(owner),
                        zone_name
                    ));
                } else if zone == Zone::Graveyard && self.single_graveyard {
                    parts.push("in single graveyard".to_string());
                } else if zone == Zone::Graveyard {
                    parts.push("in a graveyard".to_string());
                } else {
                    parts.push(format!("in {}", zone_name));
                }
            } else if zone == Zone::Stack {
                // "on stack" is usually implicit in Oracle text (e.g., "target spell").
                // Avoid adding it to reduce render-only mismatches.
            }
        }

        let has_entered_battlefield_this_turn_clause = (self.entered_battlefield_this_turn
            || self.entered_battlefield_controller.is_some())
            && self.zone == Some(Zone::Battlefield);
        if has_entered_battlefield_this_turn_clause
            && self.entered_battlefield_controller.is_none()
            && owner_suffix.is_none()
            && let Some(controller) = controller_suffix.take()
        {
            parts.push(controller);
        }

        if has_entered_battlefield_this_turn_clause {
            let clause = if let Some(controller) = &self.entered_battlefield_controller {
                match controller {
                    PlayerFilter::You => {
                        "that entered the battlefield under your control this turn".to_string()
                    }
                    PlayerFilter::Opponent => {
                        "that entered the battlefield under an opponent's control this turn"
                            .to_string()
                    }
                    PlayerFilter::Any => "that entered this turn".to_string(),
                    other => format!(
                        "that entered the battlefield under {} control this turn",
                        describe_possessive_player_filter(other)
                    ),
                }
            } else {
                "that entered this turn".to_string()
            };
            parts.push(clause);
        }

        if self.put_onto_battlefield_with_source {
            let source = self
                .put_onto_battlefield_with_source_surface
                .as_ref()
                .map(ironsmith_core::SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this permanent".to_string());
            parts.push(format!("put onto the battlefield with {source}"));
        }

        if self.created_with_source {
            let source = self
                .created_with_source_surface
                .as_ref()
                .map(ironsmith_core::SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this permanent".to_string());
            parts.push(format!("created with {source}"));
        }

        if self.entered_graveyard_from_library_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from their library this turn".to_string());
        } else if self.entered_graveyard_from_battlefield_this_turn
            && self.zone == Some(Zone::Graveyard)
        {
            parts.push("that was put there from the battlefield this turn".to_string());
        } else if self.entered_graveyard_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from anywhere this turn".to_string());
        }

        if let Some(constraint) = &self.counters_put_on_this_turn {
            parts.push(describe_counters_put_on_this_turn_constraint(constraint));
        }

        if self.was_dealt_damage_this_turn {
            parts.push("that was dealt damage this turn".to_string());
        }
        if self.dealt_damage_this_turn {
            parts.push("that dealt damage this turn".to_string());
        }
        if let Some(damager) = &self.dealt_damage_by_source_this_turn {
            let source = match damager {
                ironsmith_core::DamagedBySource::ThisCreature => "this creature",
                ironsmith_core::DamagedBySource::EquippedCreature => "equipped creature",
                ironsmith_core::DamagedBySource::EnchantedCreature => "enchanted creature",
            };
            parts.push(format!("that was dealt damage by {source} this turn"));
        }
        if self.was_dealt_damage_by_source_this_game {
            parts.push("that this source has dealt damage to this game".to_string());
        }
        if let Some(player) = &self.dealt_damage_to_player_this_turn {
            parts.push(format!(
                "that dealt damage to {} this turn",
                describe_player_filter(player)
            ));
        }
        if self.drawn_this_turn {
            parts.push("drawn this turn".to_string());
        }

        parts.extend(chosen_trailing_qualifiers);

        let ensure_indefinite_article = |text: String| -> String {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return "a permanent".to_string();
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("a ")
                || lower.starts_with("an ")
                || lower.starts_with("the ")
                || lower.starts_with("another ")
                || lower.starts_with("each ")
                || lower.starts_with("all ")
                || lower.starts_with("this ")
                || lower.starts_with("that ")
                || lower.starts_with("those ")
                || lower.starts_with("target ")
                || lower.starts_with("any ")
                || lower.starts_with("up to ")
                || lower.starts_with("at least ")
                || lower.chars().next().is_some_and(|ch| ch.is_ascii_digit())
            {
                return trimmed.to_string();
            }
            let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
            let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
                "an"
            } else {
                "a"
            };
            format!("{article} {trimmed}")
        };

        let mut appended_targeting_only = false;
        if self.targets_only_player.is_some() || self.targets_only_object.is_some() {
            let mut target_fragments = Vec::new();
            if let Some(player_filter) = &self.targets_only_player {
                let mut text = describe_player_filter(player_filter);
                if text != "you" {
                    text = ensure_indefinite_article(text);
                }
                target_fragments.push(text);
            }
            if let Some(object_filter) = &self.targets_only_object {
                let mut text = ensure_indefinite_article(object_filter.description());
                if let Some(count) = self.target_count
                    && count.is_single()
                    && (text.starts_with("a ") || text.starts_with("an "))
                {
                    if let Some(rest) = text.strip_prefix("a ") {
                        text = format!("a single {rest}");
                    } else if let Some(rest) = text.strip_prefix("an ") {
                        text = format!("a single {rest}");
                    }
                }
                target_fragments.push(text);
            }
            if !target_fragments.is_empty() {
                let target_text = if target_fragments.len() == 2 {
                    let joiner = if self.targets_only_any_of {
                        match self.union_connective() {
                            ObjectFilterUnionConnective::Or => "or",
                            ObjectFilterUnionConnective::AndOr => "and/or",
                        }
                    } else {
                        "and"
                    };
                    format!("{} {} {}", target_fragments[0], joiner, target_fragments[1])
                } else {
                    target_fragments[0].clone()
                };
                parts.push(format!("that targets only {target_text}"));
                appended_targeting_only = true;
            }
        }

        if let Some(count) = self.target_count
            && !appended_targeting_only
        {
            let phrase = if count.is_single() {
                Some("with a single target".to_string())
            } else if let Some(max) = count.max {
                if count.min == max {
                    Some(format!("with {} targets", max))
                } else if count.min == 0 {
                    Some(format!("with up to {} targets", max))
                } else {
                    Some(format!("with between {} and {} targets", count.min, max))
                }
            } else if count.min == 0 {
                Some("with any number of targets".to_string())
            } else {
                Some(format!("with at least {} targets", count.min))
            };
            if let Some(phrase) = phrase {
                parts.push(phrase);
            }
        }

        if !appended_targeting_only {
            let mut target_fragments = Vec::new();
            if let Some(player_filter) = &self.targets_player {
                let mut text = describe_player_filter(player_filter);
                if text != "you" {
                    text = ensure_indefinite_article(text);
                }
                target_fragments.push(text);
            }
            if let Some(object_filter) = &self.targets_object {
                target_fragments.push(ensure_indefinite_article(object_filter.description()));
            }
            if !target_fragments.is_empty() {
                let target_text = if target_fragments.len() == 2 {
                    let joiner = if self.targets_any_of {
                        match self.union_connective() {
                            ObjectFilterUnionConnective::Or => "or",
                            ObjectFilterUnionConnective::AndOr => "and/or",
                        }
                    } else {
                        "and"
                    };
                    format!("{} {} {}", target_fragments[0], joiner, target_fragments[1])
                } else {
                    target_fragments[0].clone()
                };
                parts.push(format!("that targets {target_text}"));
            }
        }

        if let Some(targetability) = &self.could_be_targeted_by {
            let stack_text = match &targetability.stack_object {
                ObjectRef::Target => "that spell",
                ObjectRef::Tagged(tag)
                    if matches!(tag.as_str(), "triggering" | "__it__" | "it") =>
                {
                    "that spell"
                }
                ObjectRef::Tagged(tag) if tag.as_str().contains("copied") => "the copy",
                ObjectRef::Tagged(_) | ObjectRef::Specific(_) => "that object",
            };
            parts.push(format!("{stack_text} could target"));
        }

        correct_filter_leading_indefinite_article(parts.join(" "))
    }
}

#[cfg(test)]
mod permanent_spell_description_tests {
    use super::*;

    #[test]
    fn complete_permanent_spell_type_set_compacts_to_permanent_spell() {
        let filter = ObjectFilter {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::Spell),
            card_types: vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Planeswalker,
                CardType::Battle,
            ],
            ..ObjectFilter::default()
        };

        assert_eq!(filter.description(), "permanent spell");
    }
}
