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

mod matching;
mod subject;
pub(crate) use subject::ObjectSubject;

mod descriptions;
pub(crate) use descriptions::describe_comparison;
pub use descriptions::describe_player_filter;
use descriptions::*;

#[cfg(test)]
mod subject_tests;
#[cfg(test)]
mod tests;

pub(crate) mod name_groups;

pub(crate) fn normalize_name_for_match(name: &str) -> String {
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

/// Face-down permanents and spells have no name (CR 708.2a); the engine
/// labels them "Face-down creature" for display only.
pub(crate) fn name_is_nameless(name: &str) -> bool {
    name.trim().is_empty() || name == crate::object::FACE_DOWN_DISPLAY_NAME
}

/// CR 201.3: pairs of cards with interchangeable names (a Universes Beyond
/// card and its Universes Within reprint, rule 201.3c). The second name of
/// each pair is its counterpart as listed in the local Oracle catalog.
const INTERCHANGEABLE_NAME_PAIRS: &[(&str, &str)] = &[
    ("Ken, Burning Brawler", "Aisha of Sparks and Smoke"),
    ("Mind Flayer, the Shadow", "Arvinox, the Mind Flail"),
    ("E. Honda, Sumo Champion", "Baldin, Century Herdmaster"),
    ("Lucas, the Sharpshooter", "Bjorna, Nightfall Alchemist"),
    ("Edgin, Larcenous Lutenist", "Bohn, Beguiling Balladeer"),
    ("Doric, Nature's Warden", "Casal, Lurkwood Pathfinder"),
    ("Doric, Owlbear Avenger", "Casal, Pathbreaker Owlbear"),
    ("Eleven, the Mage", "Cecily, Haunted Mage"),
    ("Max, the Daredevil", "Elmar, Ulvenwald Informant"),
    ("Michonne, Ruthless Survivor", "Enkira, Hostile Scavenger"),
    ("Forge, Neverwinter Charlatan", "Evin, Waterdeep Opportunist"),
    ("Lucille", "Gisa's Favorite Shovel"),
    ("Glenn, the Voice of Calm", "Gregor, Shrewd Magistrate"),
    ("Rick, Steadfast Leader", "Greymond, Avacyn's Stalwart"),
    ("Daryl, Hunter of Walkers", "Hansk, Slayer Zealot"),
    ("Dustin, Gadget Genius", "Hargilde, Kindly Runechanter"),
    ("Hawkins National Laboratory", "Havengul Laboratory"),
    ("The Upside Down", "Havengul Mystery"),
    ("Guile, Sonic Soldier", "Immard, the Stormcleaver"),
    ("Holga, Relentless Rager", "Jurin, Leading the Charge"),
    ("Zangief, the Red Cyclone", "Maarika, Brutal Gladiator"),
    ("Negan, the Cold-Blooded", "Malik, Grim Manipulator"),
    ("Simon, Wild Magic Sorcerer", "Mathise, Surge Channeler"),
    ("Mike, the Dungeon Master", "Othelm, Sigardian Outcast"),
    ("Xenk, Paladin Unbroken", "Rashel, Fist of Torm"),
    ("Chief Jim Hopper", "Sophina, Spearsage Deserter"),
    ("Dhalsim, Pliable Pacifist", "Tadeas, Juniper Ascendant"),
    ("Blanka, Ferocious Friend", "The Howling Abomination"),
    ("Ryu, World Warrior", "Vikya, Scorching Stalwart"),
    ("Will the Wise", "Wernog, Rider's Chaplain"),
    ("Chun-Li, Countless Kicks", "Zethi, Arcane Blademaster"),
];

fn normalized_interchangeable_pairs() -> &'static [(String, String)] {
    static PAIRS: std::sync::OnceLock<Vec<(String, String)>> = std::sync::OnceLock::new();
    PAIRS.get_or_init(|| {
        INTERCHANGEABLE_NAME_PAIRS
            .iter()
            .map(|(first, second)| {
                (
                    normalize_name_for_match(first),
                    normalize_name_for_match(second),
                )
            })
            .collect()
    })
}

fn interchangeable_pair(name: &str) -> Option<(&'static str, &'static str)> {
    let normalized = normalize_name_for_match(name);
    normalized_interchangeable_pairs()
        .iter()
        .position(|(first, second)| *first == normalized || *second == normalized)
        .map(|index| INTERCHANGEABLE_NAME_PAIRS[index])
}

/// Group interchangeable printings under one identity (CR 201.3a).
pub(crate) fn interchangeable_name_key(name: &str) -> &str {
    interchangeable_pair(name).map_or(name, |(original, _)| original)
}

/// Resolve either interchangeable printing to the name used for Oracle lookups.
pub fn oracle_name_for_lookup(name: &str) -> &str {
    interchangeable_pair(name).map_or(name, |(_, catalog)| catalog)
}

/// Both arguments are already normalized with `normalize_name_for_match`.
fn normalized_names_are_interchangeable(lhs: &str, rhs: &str) -> bool {
    normalized_interchangeable_pairs()
        .iter()
        .any(|(first, second)| (lhs == first && rhs == second) || (lhs == second && rhs == first))
}

/// CR 201.2a: an object with no name doesn't have the same name as any other
/// object, including another object with no name. CR 201.3a: interchangeable
/// names are the same name.
pub(crate) fn names_match(lhs: &str, rhs: &str) -> bool {
    if name_is_nameless(lhs) || name_is_nameless(rhs) {
        return false;
    }
    lhs.split(" // ").any(|left| {
        rhs.split(" // ").any(|right| {
            let (left, right) = (left.trim(), right.trim());
            if name_is_nameless(left) || name_is_nameless(right) {
                return false;
            }
            if left.eq_ignore_ascii_case(right) {
                return true;
            }
            let left = normalize_name_for_match(left);
            let right = normalize_name_for_match(right);
            left == right || normalized_names_are_interchangeable(&left, &right)
        })
    })
}

/// Whether `name` is the name of a nonlegendary creature card (Spy Kit). The
/// card catalog decides; a name the catalog doesn't carry falls back to the
/// physical cards in this game.
pub(crate) fn is_nonlegendary_creature_card_name(name: &str, game: &GameState) -> bool {
    let records = crate::cards::oracle_face_types(oracle_name_for_lookup(name));
    if !records.is_empty() {
        return records.iter().any(|(supertypes, types)| {
            types.contains(&CardType::Creature) && !supertypes.contains(&Supertype::Legendary)
        });
    }
    // Games can also register custom card definitions outside the Oracle catalog.
    !name_is_nameless(name) && game.objects_map().values().any(|object| {
        object.kind == ObjectKind::Card && names_match(&object.name, name)
            && object.card_types.contains(&CardType::Creature)
            && !object.supertypes.contains(&Supertype::Legendary)
    })
}

/// CR 612.7 includes nonlegendary creature names on either face of a card.
pub(crate) fn all_nonlegendary_creature_card_names(game: &GameState) -> impl Iterator<Item = &str> {
    use std::sync::OnceLock;
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    let oracle = NAMES.get_or_init(|| {
        let mut names: Vec<_> = crate::cards::oracle_face_names().filter(|name| {
            crate::cards::oracle_face_types(name).iter().any(|(supertypes, types)| {
                types.contains(&CardType::Creature) && !supertypes.contains(&Supertype::Legendary)
            })
        }).map(interchangeable_name_key).collect();
        names.sort_unstable();
        names.dedup();
        names
    });
    let custom = game.objects_map().values().filter(|object| {
        object.kind == ObjectKind::Card && object.card_types.contains(&CardType::Creature)
            && !object.supertypes.contains(&Supertype::Legendary)
            && !crate::cards::is_oracle_card_name(&object.name)
    }).map(|object| object.name.as_ref());
    oracle.iter().copied().chain(custom)
}

/// Whether one of `attachments` (still attached on the battlefield) gives its
/// host every nonlegendary creature card name in addition to its own (Spy
/// Kit). The names come from that source's static ability, not the host's.
pub(crate) fn grants_creature_names(
    attachments: &[ObjectId],
    name_set: Option<u64>,
    game: &GameState,
) -> bool {
    attachments.iter().any(|id| {
        game.object(*id).is_some_and(|attachment| {
            attachment.zone == Zone::Battlefield && !game.is_phased_out(*id)
                && attachment.abilities.iter().any(|ability| {
                    matches!(&ability.kind, AbilityKind::Static(rule)
                        if rule.id() == StaticAbilityId::AttachedHasAllNonlegendaryCreatureCardNames)
                })
                && name_set.is_none_or(|stamp| game.effect_store.continuous_effects
                    .get_object_timestamp(*id).unwrap_or(0) > stamp)
        })
    })
}

/// Snapshot and raw-filter callers may run during layer discovery. Reuse a held
/// frame when available, without starting another characteristic calculation.
pub(crate) fn raw_grants_creature_names(id: ObjectId, attachments: &[ObjectId], game: &GameState) -> bool {
    if attachments.is_empty() {
        return false;
    }
    let name_set = if let Some(chars) = crate::continuous::in_progress_characteristics(game, id) {
        chars.name_set_timestamp
    } else if let Some(Some(chars)) = game.calculated_characteristics_if_held(id) {
        chars.name_set_timestamp
    } else {
        game.effect_store.continuous_effects.effects().iter().filter(|effect| {
            matches!(effect.modification, crate::continuous::Modification::SetName(_))
                && (matches!(effect.applies_to, crate::continuous::EffectTarget::Specific(target) if target == id)
                    || matches!(&effect.source_type, crate::continuous::EffectSourceType::Resolution { locked_targets }
                        if locked_targets.contains(&id)))
        }).map(|effect| effect.timestamp).max()
    };
    grants_creature_names(attachments, name_set, game)
}

#[derive(Clone, Copy)]
pub(crate) struct ObjectNames<'a> {
    pub name: &'a str,
    pub alternate: Option<&'a str>,
    pub creature_names: bool,
}

impl<'a> ObjectNames<'a> {
    pub fn layered(
        object: &Object,
        chars: &'a CalculatedCharacteristics,
        game: &GameState,
    ) -> Self {
        Self {
            name: &chars.name,
            alternate: chars.alternate_name.as_deref(),
            creature_names: grants_creature_names(
                &object.attachments,
                chars.name_set_timestamp,
                game,
            ),
        }
    }

    pub fn snapshot(snapshot: &'a ObjectSnapshot) -> Self {
        Self {
            name: &snapshot.name,
            alternate: snapshot.split_other_half_name(),
            creature_names: snapshot.all_nonlegendary_creature_names,
        }
    }

    pub fn literals(self) -> impl Iterator<Item = &'a str> {
        std::iter::once(self.name)
            .chain(self.alternate)
            .filter(move |_| !name_is_nameless(self.name))
            .flat_map(|name| name.split(" // "))
            .map(str::trim)
            .filter(|name| !name_is_nameless(name))
    }

    pub fn shares(self, other: Self, game: &GameState) -> bool {
        names_share(self.name, self.alternate, other.name, other.alternate)
            || (self.creature_names && other.creature_names)
            || (self.creature_names
                && other
                    .literals()
                    .any(|n| is_nonlegendary_creature_card_name(n, game)))
            || (other.creature_names
                && self
                    .literals()
                    .any(|n| is_nonlegendary_creature_card_name(n, game)))
    }
}

/// Whether two objects share a name, each given as its primary name plus the
/// other half's name when it's a split card outside the stack and battlefield
/// (CR 709.4a: such a card has both names).
pub(crate) fn names_share(
    lhs: &str,
    lhs_other: Option<&str>,
    rhs: &str,
    rhs_other: Option<&str>,
) -> bool {
    // Alternate split-card metadata is retained even when a current effect
    // makes the object nameless. It cannot restore a hidden or removed name.
    if name_is_nameless(lhs) || name_is_nameless(rhs) {
        return false;
    }
    std::iter::once(lhs).chain(lhs_other).any(|left| {
        std::iter::once(rhs)
            .chain(rhs_other)
            .any(|right| names_match(left, right))
    })
}

pub(crate) fn object_mana_value_for_filter(object: &Object) -> i32 {
    let value = object.linked_face_mana_value().unwrap_or_else(|| {
        object.mana_cost.as_ref().map_or(0, |cost| {
            if object.zone == Zone::Stack {
                cost.mana_value_with_x(object.x_value.unwrap_or(0))
            } else {
                cost.mana_value()
            }
        })
    });
    ironsmith_core::quantity::clamp_i32_from_u32(value)
}

pub(crate) fn snapshot_mana_value_for_filter(snapshot: &ObjectSnapshot) -> i32 {
    ironsmith_core::quantity::clamp_i32_from_u32(snapshot.mana_value())
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
    Some(match entry.casting_method.origin_method() {
        crate::alternative_cast::CastingMethod::AlternativePrice { .. }
        | crate::alternative_cast::CastingMethod::ExactPermission { .. } => return None,
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
        | crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom { zone, .. }
        | crate::alternative_cast::CastingMethod::FaceDownPlayFrom { zone, .. } => *zone,
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
        false,
        object_id,
        ctx,
        game,
        fallback_cast_player,
    )
}

fn matching_spell_cast_ordinal_each_turn_matches(
    filter: &ObjectFilter,
    ordinal: u32,
    at_least: bool,
    object_id: ObjectId,
    ctx: &FilterContext,
    game: &GameState,
    fallback_cast_player: Option<PlayerId>,
) -> bool {
    let ordinal_matches = |actual: u32| {
        if at_least {
            actual >= ordinal
        } else {
            actual == ordinal
        }
    };
    let mut matching_filter = filter.clone();
    matching_filter.first_spell_cast_each_turn = false;
    matching_filter.spell_cast_ordinal_each_turn = None;
    matching_filter.spell_cast_minimum_each_turn = None;

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
                return ordinal_matches(matching_ordinal);
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
        return current_live_match && ordinal_matches(matching_ordinal.saturating_add(1));
    }

    // Cost previews ask about the next matching cast before a stack entry
    // or cast event exists. Ordinary object queries must not assume a cast.
    if ctx.prospective_cast == Some(object_id) && ctx.caster.is_some() {
        return ordinal_matches(matching_ordinal.saturating_add(1));
    }

    let cast_order = fallback_cast_player
        .and_then(|player| {
            game.turn_store
                .turn_history
                .spell_cast_order_for_player(object_id, player)
        })
        .or_else(|| game.turn_store.turn_history.spell_cast_order(object_id));
    cast_order.is_some_and(ordinal_matches)
}

pub(crate) trait TaggedConstraintSubject {
    fn subject_object_id(&self) -> ObjectId;
    fn subject_stable_id(&self) -> StableId;
    fn subject_name(&self) -> &str;
    /// The other half's name of a split card outside the stack and
    /// battlefield, which it also has (CR 709.4a).
    fn subject_alternate_name(&self) -> Option<&str> {
        None
    }
    fn subject_controller(&self) -> PlayerId;
    fn subject_owner(&self) -> PlayerId;
    fn subject_card_types(&self) -> &[CardType];
    fn subject_subtypes(&self) -> &[Subtype];
    fn subject_colors(&self) -> ColorSet;
    fn subject_mana_value(&self) -> i32;
    fn subject_attached_to(&self) -> Option<ObjectId>;
    fn subject_attached_to_player(&self) -> Option<PlayerId>;
    fn subject_attachments(&self) -> &[ObjectId];
    fn subject_creature_names(&self, game: &GameState) -> bool {
        raw_grants_creature_names(self.subject_object_id(), self.subject_attachments(), game)
    }
    fn subject_names(&self, game: &GameState) -> ObjectNames<'_> {
        ObjectNames {
            name: self.subject_name(),
            alternate: self.subject_alternate_name(),
            creature_names: self.subject_creature_names(game),
        }
    }
    fn subject_was_enchanted(&self) -> bool;
}

pub(crate) trait TailMatchSubject: TaggedConstraintSubject {
    fn tail_object_id(&self) -> ObjectId;
    fn tail_mana_snapshot(&self) -> Option<&ObjectSnapshot> { None }
    fn tail_name(&self) -> &str;
    /// A second name the subject has: the other half of a split card outside
    /// the stack and battlefield (CR 709.4a).
    fn tail_alternate_name(&self) -> Option<&str> {
        self.subject_alternate_name()
    }
    fn tail_first_printed_set_name(&self) -> Option<&str>;
    fn tail_counters(&self) -> &std::collections::BTreeMap<CounterType, u32>;
    fn tail_abilities(&self) -> &[crate::ability::Ability];
    fn tail_has_casting_ability(&self) -> bool;
    fn tail_has_complete_abilities(&self) -> bool {
        true
    }
    /// Frozen attached objects for a historical subject, including an empty set.
    fn tail_attachment_snapshots(&self) -> Option<&[ObjectSnapshot]> {
        None
    }
    fn tail_related_exiled_snapshots(&self) -> Option<&std::collections::BTreeMap<String, Vec<ObjectSnapshot>>> { None }

    fn tail_has_alternative_cast_kind(
        &self,
        kind: AlternativeCastKind,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
    ) -> bool;
    fn tail_has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool;
    /// CR 702.124n: partner, partner with, and partner—[text] (not "Choose a
    /// Background" or other pairing variants).
    fn tail_has_partner_family(&self) -> bool;
    fn tail_has_ability_marker(&self, marker: &str) -> bool;
    /// A casting keyword granted by another effect (Snapcaster Mage's
    /// flashback), read from the subject's current object.
    fn tail_has_granted_casting_keyword(
        &self,
        marker: &str,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
    ) -> bool {
        game.object(self.subject_object_id())
            .is_some_and(|object| object_has_granted_casting_keyword(object, marker, game, ctx))
    }
    fn tail_has_tap_activated_ability(&self) -> bool;
    fn tail_has_non_mana_activated_ability(&self) -> bool;
    fn tail_is_commander(&self, game: &crate::game_state::GameState) -> bool;
}

fn static_ability_is_partner_family(ability: &crate::static_abilities::StaticAbility) -> bool {
    match ability.id() {
        StaticAbilityId::PartnerWith => true,
        StaticAbilityId::Partner => !matches!(
            ability.commander_pairing(),
            Some(crate::static_abilities::CommanderPairing::ChooseBackground)
        ),
        _ => false,
    }
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

    fn subject_alternate_name(&self) -> Option<&str> {
        self.split_other_half_name()
    }

    fn subject_owner(&self) -> PlayerId {
        self.owner
    }

    fn subject_controller(&self) -> PlayerId {
        self.owner
    }

    fn subject_card_types(&self) -> &[CardType] {
        self.zone_card_types()
    }

    fn subject_subtypes(&self) -> &[Subtype] {
        self.zone_subtypes()
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

    fn tail_counters(&self) -> &std::collections::BTreeMap<CounterType, u32> {
        &self.counters
    }

    fn tail_has_casting_ability(&self) -> bool {
        self.has_intrinsic_casting_ability()
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

    fn tail_has_partner_family(&self) -> bool {
        self.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(s) if static_ability_is_partner_family(s))
        })
    }

    fn tail_has_ability_marker(&self, marker: &str) -> bool {
        object_has_ability_marker(self, marker)
    }

    fn tail_has_tap_activated_ability(&self) -> bool {
        object_has_tap_activated_ability(self)
    }

    fn tail_has_non_mana_activated_ability(&self) -> bool {
        abilities_have_non_mana_activated_ability(&self.abilities)
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

    fn subject_alternate_name(&self) -> Option<&str> {
        self.chars.alternate_name.as_deref()
    }

    fn subject_owner(&self) -> PlayerId {
        self.object.owner
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
        calculated_mana_value_for_filter(self.object, self.chars)
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

    fn subject_creature_names(&self, game: &GameState) -> bool {
        grants_creature_names(&self.object.attachments, self.chars.name_set_timestamp, game)
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

    fn tail_counters(&self) -> &std::collections::BTreeMap<CounterType, u32> {
        &self.object.counters
    }

    fn tail_has_casting_ability(&self) -> bool {
        self.chars.has_casting_ability(self.object)
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
        object_has_alternative_cast_kind_in_view(
            self.object,
            self.chars.abilities.as_slice(),
            self.chars.abilities.casting_ability_loss.is_none(),
            kind,
            game,
            ctx,
        )
    }

    fn tail_has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool {
        self.chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == ability_id)
    }

    fn tail_has_partner_family(&self) -> bool {
        self.chars
            .static_abilities
            .iter()
            .any(static_ability_is_partner_family)
    }

    fn tail_has_ability_marker(&self, marker: &str) -> bool {
        calculated_object_has_ability_marker(self.object, self.chars, marker)
    }

    fn tail_has_tap_activated_ability(&self) -> bool {
        abilities_have_tap_activated_ability(&self.chars.abilities)
    }

    fn tail_has_non_mana_activated_ability(&self) -> bool {
        abilities_have_non_mana_activated_ability(&self.chars.abilities)
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

    fn subject_alternate_name(&self) -> Option<&str> {
        self.split_other_half_name()
    }

    fn subject_owner(&self) -> PlayerId {
        self.owner
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

    fn subject_creature_names(&self, _: &GameState) -> bool {
        self.all_nonlegendary_creature_names
    }

    fn subject_attachments(&self) -> &[ObjectId] {
        &self.attachments
    }

    fn subject_was_enchanted(&self) -> bool {
        self.was_enchanted
    }
}

impl TailMatchSubject for ObjectSnapshot {
    fn tail_mana_snapshot(&self) -> Option<&ObjectSnapshot> { Some(self) }
    fn tail_related_exiled_snapshots(&self) -> Option<&std::collections::BTreeMap<String, Vec<ObjectSnapshot>>> { Some(&self.related_exiled_cards) }
    fn tail_attachment_snapshots(&self) -> Option<&[ObjectSnapshot]> {
        Some(&self.attachment_snapshots)
    }
    fn tail_object_id(&self) -> ObjectId {
        self.object_id
    }

    fn tail_name(&self) -> &str {
        &self.name
    }

    fn tail_first_printed_set_name(&self) -> Option<&str> {
        self.first_printed_set_name.as_deref()
    }

    fn tail_counters(&self) -> &std::collections::BTreeMap<CounterType, u32> {
        &self.counters
    }

    fn tail_has_casting_ability(&self) -> bool {
        self.has_casting_ability
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

    fn tail_has_granted_casting_keyword(
        &self,
        marker: &str,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
    ) -> bool {
        if let Some(object) = game.object(self.object_id) {
            return object_has_granted_casting_keyword(object, marker, game, ctx);
        }
        // The card left the zone this snapshot recorded (a discarded card is
        // already in exile or the graveyard when "if it has madness" is
        // checked): match grants in the recorded zone against the card's
        // current incarnation (Falkenrath Gorger's grant to Vampire cards).
        game.find_object_by_stable_id(self.stable_id)
            .and_then(|id| game.object(id))
            .is_some_and(|object| {
                object_has_granted_casting_keyword_in_zone(object, self.zone, marker, game, ctx)
            })
    }

    fn tail_has_complete_abilities(&self) -> bool {
        self.ability_origins
            .as_ref()
            .is_some_and(|origins| origins.len() == self.abilities.len())
    }

    fn tail_has_static_ability_id(&self, ability_id: StaticAbilityId) -> bool {
        snapshot_has_static_ability_id(self, ability_id)
    }

    fn tail_has_partner_family(&self) -> bool {
        self.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(s) if static_ability_is_partner_family(s))
        })
    }

    fn tail_has_ability_marker(&self, marker: &str) -> bool {
        snapshot_has_ability_marker(self, marker)
    }

    fn tail_has_tap_activated_ability(&self) -> bool {
        snapshot_has_tap_activated_ability(self)
    }

    fn tail_has_non_mana_activated_ability(&self) -> bool {
        abilities_have_non_mana_activated_ability(&self.abilities)
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
    let produced = crate::effects::mana::permanent_mana_types(game, subject.tail_object_id(),
        subject.subject_controller(), subject.tail_abilities(), subject.tail_mana_snapshot());
    required.iter().any(|symbol| produced.contains(symbol))
}

fn linked_face_has_adventure(
    game: &GameState,
    name: Option<&str>,
    id: Option<crate::ids::CardId>,
) -> bool {
    game.linked_face_definition_by_name_or_id(name, id)
        .is_some_and(|def| def.card.subtypes.contains(&Subtype::Adventure))
}

/// Whether a filter subject has `name`, counting both names of a split card
/// outside the stack and battlefield (CR 709.4a).
fn subject_has_name<S: TailMatchSubject>(subject: &S, name: &str) -> bool {
    names_match(subject.tail_name(), name)
        || subject
            .tail_alternate_name()
            .is_some_and(|other| names_match(other, name))
}

/// `subject_has_name`, also counting the names an attached Spy Kit adds.
fn subject_has_name_in_game<S: TailMatchSubject>(
    subject: &S,
    name: &str,
    game: &GameState,
) -> bool {
    subject_has_name(subject, name)
        || (subject.subject_creature_names(game) && is_nonlegendary_creature_card_name(name, game))
}

/// CR 206.3a-c: the names originally printed in the sets that Oracle text
/// still refers to ("with a name originally printed in the ... expansion").
const NAMES_ORIGINALLY_PRINTED_IN_SET: &[(&str, &[&str])] = &[
    (
        "Arabian Nights",
        &[
            "Abu Ja'far",
            "Aladdin",
            "Aladdin's Lamp",
            "Aladdin's Ring",
            "Ali Baba",
            "Ali from Cairo",
            "Army of Allah",
            "Bazaar of Baghdad",
            "Bird Maiden",
            "Bottle of Suleiman",
            "Brass Man",
            "Camel",
            "City in a Bottle",
            "City of Brass",
            "Cuombajj Witches",
            "Cyclone",
            "Dancing Scimitar",
            "Dandân",
            "Desert",
            "Desert Nomads",
            "Desert Twister",
            "Diamond Valley",
            "Drop of Honey",
            "Ebony Horse",
            "Elephant Graveyard",
            "El-Hajjâj",
            "Erg Raiders",
            "Erhnam Djinn",
            "Eye for an Eye",
            "Fishliver Oil",
            "Flying Carpet",
            "Flying Men",
            "Ghazbán Ogre",
            "Giant Tortoise",
            "Guardian Beast",
            "Hasran Ogress",
            "Hurr Jackal",
            "Ifh-Biff Efreet",
            "Island Fish Jasconius",
            "Island of Wak-Wak",
            "Jandor's Ring",
            "Jandor's Saddlebags",
            "Jeweled Bird",
            "Jihad",
            "Junún Efreet",
            "Juzám Djinn",
            "Khabál Ghoul",
            "King Suleiman",
            "Kird Ape",
            "Library of Alexandria",
            "Magnetic Mountain",
            "Merchant Ship",
            "Metamorphosis",
            "Mijae Djinn",
            "Moorish Cavalry",
            "Nafs Asp",
            "Oasis",
            "Old Man of the Sea",
            "Oubliette",
            "Piety",
            "Pyramids",
            "Repentant Blacksmith",
            "Ring of Ma'rûf",
            "Rukh Egg",
            "Sandals of Abdallah",
            "Sandstorm",
            "Serendib Djinn",
            "Serendib Efreet",
            "Shahrazad",
            "Sindbad",
            "Singing Tree",
            "Sorceress Queen",
            "Stone-Throwing Devils",
            "Unstable Mutation",
            "War Elephant",
            "Wyluli Wolf",
            "Ydwen Efreet",
        ],
    ),
    (
        "Antiquities",
        &[
            "Amulet of Kroog",
            "Argivian Archaeologist",
            "Argivian Blacksmith",
            "Argothian Pixies",
            "Argothian Treefolk",
            "Armageddon Clock",
            "Artifact Blast",
            "Artifact Possession",
            "Artifact Ward",
            "Ashnod's Altar",
            "Ashnod's Battle Gear",
            "Ashnod's Transmogrant",
            "Atog",
            "Battering Ram",
            "Bronze Tablet",
            "Candelabra of Tawnos",
            "Circle of Protection: Artifacts",
            "Citanul Druid",
            "Clay Statue",
            "Clockwork Avian",
            "Colossus of Sardia",
            "Coral Helm",
            "Crumble",
            "Cursed Rack",
            "Damping Field",
            "Detonate",
            "Drafna's Restoration",
            "Dragon Engine",
            "Dwarven Weaponsmith",
            "Energy Flux",
            "Feldon's Cane",
            "Gaea's Avenger",
            "Gate to Phyrexia",
            "Goblin Artisans",
            "Golgothian Sylex",
            "Grapeshot Catapult",
            "Haunting Wind",
            "Hurkyl's Recall",
            "Ivory Tower",
            "Jalum Tome",
            "Martyrs of Korlis",
            "Mightstone",
            "Millstone",
            "Mishra's Factory",
            "Mishra's War Machine",
            "Mishra's Workshop",
            "Obelisk of Undoing",
            "Onulet",
            "Orcish Mechanics",
            "Ornithopter",
            "Phyrexian Gremlins",
            "Power Artifact",
            "Powerleech",
            "Priest of Yawgmoth",
            "Primal Clay",
            "The Rack",
            "Rakalite",
            "Reconstruction",
            "Reverse Polarity",
            "Rocket Launcher",
            "Sage of Lat-Nam",
            "Shapeshifter",
            "Shatterstorm",
            "Staff of Zegon",
            "Strip Mine",
            "Su-Chi",
            "Tablet of Epityr",
            "Tawnos's Coffin",
            "Tawnos's Wand",
            "Tawnos's Weaponry",
            "Tetravus",
            "Titania's Song",
            "Transmute Artifact",
            "Triskelion",
            "Urza's Avenger",
            "Urza's Chalice",
            "Urza's Mine",
            "Urza's Miter",
            "Urza's Power Plant",
            "Urza's Tower",
            "Wall of Spears",
            "Weakstone",
            "Xenic Poltergeist",
            "Yawgmoth Demon",
            "Yotian Soldier",
        ],
    ),
    (
        "Homelands",
        &[
            "Abbey Gargoyles",
            "Abbey Matron",
            "Aether Storm",
            "Aliban's Tower",
            "Ambush",
            "Ambush Party",
            "Anaba Ancestor",
            "Anaba Bodyguard",
            "Anaba Shaman",
            "Anaba Spirit Crafter",
            "An-Havva Constable",
            "An-Havva Inn",
            "An-Havva Township",
            "An-Zerrin Ruins",
            "Apocalypse Chime",
            "Autumn Willow",
            "Aysen Abbey",
            "Aysen Bureaucrats",
            "Aysen Crusader",
            "Aysen Highway",
            "Baki's Curse",
            "Baron Sengir",
            "Beast Walkers",
            "Black Carriage",
            "Broken Visage",
            "Carapace",
            "Castle Sengir",
            "Cemetery Gate",
            "Chain Stasis",
            "Chandler",
            "Clockwork Gnomes",
            "Clockwork Steed",
            "Clockwork Swarm",
            "Coral Reef",
            "Dark Maze",
            "Daughter of Autumn",
            "Death Speakers",
            "Didgeridoo",
            "Drudge Spell",
            "Dry Spell",
            "Dwarven Pony",
            "Dwarven Sea Clan",
            "Dwarven Trader",
            "Ebony Rhino",
            "Eron the Relentless",
            "Evaporate",
            "Faerie Noble",
            "Feast of the Unicorn",
            "Feroz's Ban",
            "Folk of An-Havva",
            "Forget",
            "Funeral March",
            "Ghost Hounds",
            "Giant Albatross",
            "Giant Oyster",
            "Grandmother Sengir",
            "Greater Werewolf",
            "Hazduhr the Abbot",
            "Headstone",
            "Heart Wolf",
            "Hungry Mist",
            "Ihsan's Shade",
            "Irini Sengir",
            "Ironclaw Curse",
            "Jinx",
            "Joven",
            "Joven's Ferrets",
            "Joven's Tools",
            "Koskun Falls",
            "Koskun Keep",
            "Labyrinth Minotaur",
            "Leaping Lizard",
            "Leeches",
            "Mammoth Harness",
            "Marjhan",
            "Memory Lapse",
            "Merchant Scroll",
            "Mesa Falcon",
            "Mystic Decree",
            "Narwhal",
            "Orcish Mine",
            "Primal Order",
            "Prophecy",
            "Rashka the Slayer",
            "Reef Pirates",
            "Renewal",
            "Retribution",
            "Reveka, Wizard Savant",
            "Root Spider",
            "Roots",
            "Roterothopter",
            "Rysorian Badger",
            "Samite Alchemist",
            "Sea Sprite",
            "Sea Troll",
            "Sengir Autocrat",
            "Sengir Bats",
            "Serra Aviary",
            "Serra Bestiary",
            "Serra Inquisitors",
            "Serra Paladin",
            "Serrated Arrows",
            "Shrink",
            "Soraya the Falconer",
            "Spectral Bears",
            "Timmerian Fiends",
            "Torture",
            "Trade Caravan",
            "Truce",
            "Veldrane of Sengir",
            "Wall of Kelp",
            "Willow Faerie",
            "Willow Priestess",
            "Winter Sky",
            "Wizards' School",
        ],
    ),
];

/// CR 206.3a-c / 201.2a: whether the subject has a name (its current name or
/// an additional name it has) originally printed in `set_name`: its printing
/// metadata while it keeps its printed name, or, for the sets the rules
/// enumerate, any current name on that list (renames and copies included).
fn subject_has_name_originally_printed_in_set<S: TailMatchSubject>(
    subject: &S,
    set_name: &str,
    game: &GameState,
) -> bool {
    subject
        .tail_first_printed_set_name()
        .is_some_and(|printed| printed.eq_ignore_ascii_case(set_name))
        || NAMES_ORIGINALLY_PRINTED_IN_SET
            .iter()
            .find(|(set, _)| set.eq_ignore_ascii_case(set_name))
            .is_some_and(|(_, names)| {
                names
                    .iter()
                    .any(|name| subject_has_name_in_game(subject, name, game))
            })
}

fn object_matches_subtype(object: &Object, subtype: Subtype, game: &GameState) -> bool {
    object.zone_subtypes().contains(&subtype)
        || (subtype == Subtype::Adventure
            && !game.is_face_down(object.id)
            && object.copied_face_values.as_ref().map_or_else(
                || linked_face_has_adventure(game, object.other_face_name.as_deref(), object.other_face),
                |values| copied_values_have_adventure(values),
            ))
}

fn copied_values_have_adventure(values: &crate::snapshot::CopiableValues) -> bool {
    values.other_copy_face.as_ref()
        .is_some_and(|other| other.subtypes.contains(&Subtype::Adventure))
}

fn layered_subject_matches_subtype(
    subject: &LayeredSubject<'_>,
    subtype: Subtype,
    _game: &GameState,
) -> bool {
    subject.chars.subtypes.contains(&subtype)
        || (subtype == Subtype::Adventure
            && subject.chars.copy_values.as_ref()
                .is_some_and(|values| copied_values_have_adventure(values)))
}

fn filter_card_types<'a>(
    object: &'a Object,
    chars: Option<&'a CalculatedCharacteristics>,
) -> &'a [CardType] {
    chars.map_or(object.zone_card_types(), |chars| {
        chars.card_types.as_slice()
    })
}

fn filter_subtypes<'a>(
    object: &'a Object,
    chars: Option<&'a CalculatedCharacteristics>,
) -> &'a [Subtype] {
    chars.map_or(object.zone_subtypes(), |chars| chars.subtypes.as_slice())
}

fn filter_supertypes<'a>(
    object: &'a Object,
    chars: Option<&'a CalculatedCharacteristics>,
) -> &'a [Supertype] {
    chars.map_or(object.zone_supertypes(), |chars| {
        chars.supertypes.as_slice()
    })
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
                    && chars.copy_values.as_ref()
                        .is_some_and(|values| copied_values_have_adventure(values)));
        }
    }
    object_matches_subtype(object, subtype, game)
}

fn snapshot_matches_subtype(snapshot: &ObjectSnapshot, subtype: Subtype, _game: &GameState) -> bool {
    snapshot.subtypes.contains(&subtype)
        || (subtype == Subtype::Adventure
            && copied_values_have_adventure(&snapshot.copiable_values))
}

fn subject_creature_subtypes(
    subject: &impl TaggedConstraintSubject,
    game: &GameState,
) -> Vec<Subtype> {
    game.current_subtypes(subject.subject_object_id())
        .unwrap_or_else(|| subject.subject_subtypes().to_vec())
        .into_iter()
        .filter(|subtype| game.admitted_subtype(*subtype).is_creature_type())
        .collect()
}

/// A spell's creature types for cost conditions such as prowl, including
/// every creature type for changeling (CR 702.73a applies in all zones).
pub(crate) fn object_creature_subtypes_for_cost(object: &Object, game: &GameState) -> Vec<Subtype> {
    let can_have_creature_types = object
        .card_types
        .iter()
        .any(|card_type| matches!(card_type, CardType::Creature | CardType::Kindred));
    if can_have_creature_types && object.has_changeling() {
        return game.subtypes_in_family(SubtypeFamily::Creature);
    }
    object_creature_subtypes(object, game)
}

fn object_creature_subtypes(object: &Object, game: &GameState) -> Vec<Subtype> {
    game.current_subtypes(object.id)
        .unwrap_or_else(|| object.subtypes.to_vec())
        .into_iter()
        .filter(|subtype| game.admitted_subtype(*subtype).is_creature_type())
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
        .filter(|subtype| subtype.belongs_to_family(family))
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
        .filter(|subtype| subtype.belongs_to_family(family))
        .collect()
}

/// An object's current mana value, reading layer-1 copy effects (CR 202.3,
/// 707.2): emerge and offering reduce by this, not by the raw printed cost.
pub(crate) fn object_current_mana_value(game: &GameState, id: ObjectId) -> u32 {
    game.object(id).map_or(0, |object| {
        object_current_mana_value_for_relation(object, game).max(0) as u32
    })
}

/// An object's current mana cost, reading layer-1 copy effects.
pub(crate) fn object_current_mana_cost(
    game: &GameState,
    id: ObjectId,
) -> Option<crate::mana::ManaCost> {
    game.current_characteristics(id)
        .map(|characteristics| characteristics.mana_cost.as_deref().cloned())
        .unwrap_or_else(|| {
            game.object(id)
                .and_then(|object| object.mana_cost.as_deref().cloned())
        })
}

pub(crate) fn calculated_mana_value_for_filter(
    object: &Object,
    chars: &CalculatedCharacteristics,
) -> i32 {
    let value = chars.linked_face_mana_value.unwrap_or_else(|| {
        chars.mana_cost.as_ref().map_or(0, |cost| {
            if object.zone == Zone::Stack {
                cost.mana_value_with_x(object.x_value.unwrap_or(0))
            } else {
                cost.mana_value()
            }
        })
    });
    ironsmith_core::quantity::clamp_i32_from_u32(value)
}

pub(crate) fn object_current_mana_value_for_relation(object: &Object, game: &GameState) -> i32 {
    game.current_characteristics(object.id).map_or_else(
        || object_mana_value_for_filter(object),
        |chars| calculated_mana_value_for_filter(object, &chars),
    )
}

/// Bool characteristic comparisons run beneath checked query/action owners.
/// Keep unavailable characteristic evidence on their shared incomplete-execution
/// latch so negation and zero counts cannot turn it into success.
fn checked_relation_characteristics(
    game: &GameState,
    object: ObjectId,
) -> Result<
    Option<CalculatedCharacteristics>,
    crate::static_ability_processor::StaticEffectDiscoveryError,
> {
    game.try_current_characteristics(object)
        .inspect_err(|error| {
            game.record_token_resource_failure(
                &crate::effects::ExecutionError::ContinuousDiscovery(error.clone()),
            );
        })
}

fn subject_shares_characteristic_with_object(
    subject: &impl TaggedConstraintSubject,
    object: &Object,
    characteristic: ObjectCharacteristic,
    game: &GameState,
) -> bool {
    match characteristic {
        ObjectCharacteristic::AllCardTypes => checked_relation_characteristics(game, object.id)
            .ok()
            .flatten()
            .is_some_and(|chars| {
                ObjectCharacteristic::card_type_sets_match(
                    subject.subject_card_types(),
                    chars.card_types.as_slice(),
                )
            }),
        ObjectCharacteristic::CardType | ObjectCharacteristic::PermanentType => {
            checked_relation_characteristics(game, object.id)
                .ok()
                .flatten()
                .is_some_and(|chars| {
                    subject
                        .subject_card_types()
                        .iter()
                        .any(|kind| chars.card_types.contains(kind))
                })
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
        ObjectCharacteristic::Name => checked_relation_characteristics(game, object.id)
            .ok()
            .flatten()
            .is_some_and(|chars| {
                subject.subject_names(game).shares(ObjectNames::layered(object, &chars, game), game)
            }),
    }
}

fn characteristic_relation_matches_subject(
    subject: &impl TaggedConstraintSubject,
    relation: &ObjectCharacteristicRelation,
    ctx: &FilterContext,
    game: &GameState,
) -> bool {
    let mut comparison_context = ctx.clone();
    if relation
        .characteristics
        .contains(&ObjectCharacteristic::Name)
    {
        comparison_context.filter_candidate_players =
            Some((subject.subject_controller(), subject.subject_owner()));
    }
    let checks_characteristics = relation.characteristics.iter().any(|characteristic| {
        matches!(
            characteristic,
            ObjectCharacteristic::CardType
                | ObjectCharacteristic::PermanentType
                | ObjectCharacteristic::AllCardTypes
                | ObjectCharacteristic::Name
        )
    });
    if relation.kind == ObjectCharacteristicRelationKind::HasUnrepresentedSubtype {
        // Compare the complete union, not pairwise disjointness. A Human Elf
        // can contribute Elf even when Human is already represented.
        return relation.characteristics.iter().any(|characteristic| {
            let ObjectCharacteristic::Subtype(family)=characteristic else { return false; };
            let candidates=subject_subtypes_in_family(subject,*family);
            candidates.into_iter().any(|subtype| {
                !game.objects_in_deterministic_order().into_iter().any(|object| {
                    !(relation.exclude_candidate && object.id==subject.subject_object_id())
                        && !relation.comparison.zone.is_some_and(|zone|object.zone!=zone)
                        && !(object.zone==Zone::Battlefield && game.is_phased_out(object.id))
                        && relation.comparison.matches(object,&comparison_context,game)
                        && object_subtypes_in_family(object,*family,game).contains(&subtype)
                })
            })
        });
    }
    let mut shares = false;
    for object in game.objects_in_deterministic_order() {
        if (relation.exclude_candidate && object.id == subject.subject_object_id())
            || relation
                .comparison
                .zone
                .is_some_and(|zone| object.zone != zone)
            || (object.zone == Zone::Battlefield && game.is_phased_out(object.id))
        {
            continue;
        }
        if checks_characteristics {
            match checked_relation_characteristics(game, object.id) {
                Ok(Some(_)) => {}
                Ok(None) => continue,
                Err(_) => return false,
            }
        }
        if !relation
            .comparison
            .matches(object, &comparison_context, game)
        {
            continue;
        }
        shares = relation.characteristics.iter().any(|characteristic| {
            subject_shares_characteristic_with_object(subject, object, *characteristic, game)
        });
        if shares {
            break;
        }
    }
    match relation.kind {
        ObjectCharacteristicRelationKind::HasUnrepresentedSubtype => unreachable!("handled above"),
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

/// The host of either source-relative attachment reference. The authored
/// word survives copy/type changes, so the source's subtype is irrelevant.
fn source_attachment_host_for_tag(
    source: Option<crate::ids::ObjectId>,
    tag: &TagKey,
    game: &GameState,
) -> Option<crate::ids::ObjectId> {
    if !matches!(tag.as_str(), "enchanted" | "equipped" | "fortified") {
        return None;
    }
    let source = game.object(source?)?;
    if source.zone != crate::zone::Zone::Battlefield {
        return None;
    }
    source.attached_to.as_ref()?.object_id()
}

/// Two different attacking creatures in the same attacking band (CR 702.22).
fn objects_are_banded_together(game: &GameState, first: ObjectId, second: ObjectId) -> bool {
    first != second
        && game.combat.as_ref().is_some_and(|combat| {
            combat
                .attacking_bands
                .iter()
                .any(|band| band.contains(&first) && band.contains(&second))
        })
}

fn intrinsic_attachment_tag_constraint_matches_subject(
    subject: &impl TaggedConstraintSubject,
    tag: &TagKey,
    relation: TaggedOpbjectRelation,
    game: &GameState,
) -> Option<bool> {
    let matches_intrinsic = match tag.as_str() {
        "equipped" => subject_has_attached_subtype(subject, Subtype::Equipment, game),
        "fortified" => subject_has_attached_subtype(subject, Subtype::Fortification, game),
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

/// A tagged snapshot captured on the battlefield whose permanent is no longer
/// there (its id now names a graveyard/exile object, or nothing at all).
fn tagged_host_left_battlefield(snapshot: &ObjectSnapshot, game: &GameState) -> bool {
    snapshot.zone == Zone::Battlefield
        && !game.object(snapshot.object_id).is_some_and(|object| {
            object.zone == Zone::Battlefield && object.stable_id == snapshot.stable_id
        })
}

/// Canonical source-linked exile is current exact membership. The source
/// itself may use exact departure/phasing LKI; arbitrary result tags retain
/// their historical snapshot semantics.
fn source_characteristic_constraint_matches_subject(
    subject: &impl TaggedConstraintSubject,
    constraint: &TaggedObjectConstraint,
    ctx: &FilterContext,
    game: &GameState,
) -> Option<bool> {
    if !matches!(
        constraint.relation,
        TaggedOpbjectRelation::SharesCardType
            | TaggedOpbjectRelation::SharesPermanentType
            | TaggedOpbjectRelation::SameNameAsTagged
    ) || !matches!(
        constraint.tag.as_str(),
        crate::tag::SOURCE_EXILED_TAG | crate::tag::SOURCE_OBJECT_TAG
    ) {
        return None;
    }
    let Some(source) = ctx.source else {
        game.record_token_resource_failure(&crate::effects::ExecutionError::IncompleteEvidence(
            "characteristic comparison has no exact source identity".into(),
        ));
        return Some(false);
    };
    let matches_characteristics = |types: &[CardType], name: &str, alternate: Option<&str>| {
        if constraint.relation == TaggedOpbjectRelation::SameNameAsTagged {
            names_share(
                subject.subject_name(),
                subject.subject_alternate_name(),
                name,
                alternate,
            )
        } else {
            subject
                .subject_card_types()
                .iter()
                .any(|kind| types.contains(kind))
        }
    };
    if constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG {
        return Some(game.get_exiled_with_source_links(source).iter().any(|id| {
            let Some(_object) = game.object(*id).filter(|object| object.zone == Zone::Exile) else {
                return false;
            };
            checked_relation_characteristics(game, *id)
                .ok()
                .flatten()
                .is_some_and(|chars| {
                    matches_characteristics(
                        &chars.card_types,
                        &chars.name,
                        chars.alternate_name.as_deref(),
                    )
                })
        }));
    }
    if game.object(source).is_some() {
        match checked_relation_characteristics(game, source) {
            Ok(Some(chars)) => {
                return Some(matches_characteristics(
                    &chars.card_types,
                    &chars.name,
                    chars.alternate_name.as_deref(),
                ));
            }
            Err(_) => return Some(false),
            Ok(None) => {}
        }
    }
    let retained = game
        .source_last_known_snapshot(source)
        .or_else(|| {
            ctx.source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.object_id == source)
        })
        .or_else(|| {
            ctx.tagged_objects
                .get(crate::tag::SOURCE_OBJECT_TAG)
                .and_then(|snapshots| {
                    snapshots
                        .iter()
                        .find(|snapshot| snapshot.object_id == source)
                })
        });
    Some(match retained {
        Some(snapshot) => matches_characteristics(
            &snapshot.card_types,
            &snapshot.name,
            snapshot.split_other_half_name(),
        ),
        None => {
            game.record_token_resource_failure(&crate::effects::ExecutionError::IncompleteEvidence(
                format!("characteristic comparison lacks current or retained source evidence for {source:?}"),
            ));
            false
        }
    })
}

fn tagged_constraint_matches_subject(
    subject: &impl TaggedConstraintSubject,
    tagged_snapshots: &[ObjectSnapshot],
    relation: TaggedOpbjectRelation,
    game: &GameState,
) -> bool {
    match relation {
        TaggedOpbjectRelation::SameObjectId => tagged_snapshots
            .iter()
            .any(|snapshot| snapshot.object_id == subject.subject_object_id()),
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
        TaggedOpbjectRelation::SameNameAsTagged => tagged_snapshots.iter().any(|snapshot| {
            ObjectNames::snapshot(snapshot).shares(subject.subject_names(game), game)
        }),
        // CR 201.2c: a nameless object never has a different name.
        TaggedOpbjectRelation::DifferentNameFromTagged => {
            let names = subject.subject_names(game);
            (names.creature_names || names.literals().next().is_some())
                && tagged_snapshots.iter().all(|snapshot| {
                    !ObjectNames::snapshot(snapshot).shares(names, game)
                })
        }
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
        TaggedOpbjectRelation::AttachedToTaggedObject => tagged_snapshots.iter().any(|snapshot| {
            subject.subject_attached_to() == Some(snapshot.object_id)
                // CR 608.2h: once the tagged host has left the battlefield
                // (e.g. "whenever an equipped creature dies, attach all
                // Equipment attached to that creature to ..."), the state-based
                // action that unattached its Equipment/Auras has already run,
                // so "attached to that creature" uses the host's last-known
                // information: the attachments recorded on its battlefield
                // snapshot.
                || (tagged_host_left_battlefield(snapshot, game)
                    && game
                        .source_departure_snapshot(snapshot.object_id)
                        .filter(|last| last.stable_id == snapshot.stable_id && last.zone == snapshot.zone)
                        .unwrap_or(snapshot)
                        .attachments
                        .contains(&subject.subject_object_id()))
        }),
        TaggedOpbjectRelation::WasAttachedToTaggedObject => tagged_snapshots
            .iter()
            .any(|snapshot| snapshot.attachments.contains(&subject.subject_object_id())),
        TaggedOpbjectRelation::SameNameAsTaggedChosenByOwner => tagged_snapshots
            .iter()
            .filter(|snapshot| snapshot.controller == subject.subject_owner())
            .any(|snapshot| {
                ObjectNames::snapshot(snapshot).shares(subject.subject_names(game), game)
            }),
        TaggedOpbjectRelation::SharesNoCardTypeWithTagged => {
            !tagged_snapshots.is_empty()
                && !tagged_snapshots.iter().any(|snapshot| {
                    subject
                        .subject_card_types()
                        .iter()
                        .any(|card_type| snapshot.card_types.contains(card_type))
                })
        }
        TaggedOpbjectRelation::WasHostOfTaggedObject => tagged_snapshots.iter().any(|snapshot| {
            snapshot.attached_to.and_then(|target| target.object_id())
                == Some(subject.subject_object_id())
        }),
        // "another permanent it can enchant" (Aura Graft): the tagged Aura's
        // enchant restriction and protection allow attaching it to the
        // candidate (CR 303.4), judged for the Aura's current controller.
        // "Another" is relative to the Aura's current host: re-attaching to
        // that permanent does nothing (CR 701.3b), so it is never a new host.
        TaggedOpbjectRelation::CouldBeEnchantedByTagged => tagged_snapshots.iter().any(|snapshot| {
            let aura_id = if game.object(snapshot.object_id).is_some() {
                Some(snapshot.object_id)
            } else {
                game.find_object_by_stable_id(snapshot.stable_id)
            };
            let Some(aura_id) = aura_id else {
                return false;
            };
            let current_host = game
                .object(aura_id)
                .and_then(|aura| aura.attached_to)
                .and_then(|target| target.object_id());
            if current_host == Some(subject.subject_object_id()) {
                return false;
            }
            let controller = game
                .object(aura_id)
                .map(|aura| game.controller_of(aura))
                .unwrap_or(snapshot.controller);
            crate::effects::permanents::aura_can_enter_attached_to(
                game,
                aura_id,
                controller,
                crate::object::AttachmentTarget::Object(subject.subject_object_id()),
            )
        }),
        TaggedOpbjectRelation::WasDealtDamageByTaggedThisTurn => {
            let subject_id = subject.subject_object_id();
            let subject_stable = Some(subject.subject_stable_id());
            tagged_snapshots.iter().any(|snapshot| {
                game.turn_store
                    .turn_history
                    .creature_was_damaged_by_source_identity_this_turn(
                        subject_id,
                        subject_stable,
                        snapshot.object_id,
                        Some(snapshot.stable_id),
                    )
            })
        }
        TaggedOpbjectRelation::DealtDamageToTaggedThisTurn { combat_only } => {
            game.turn_store.turn_history.projected_records().any(|record| {
                let Some(damage) = record.event.downcast::<crate::events::DamageEvent>() else { return false; };
                damage.amount > 0 && (!combat_only || damage.is_combat)
                    && damage.source == subject.subject_object_id()
                    && matches!(damage.target, crate::events::DamageTarget::Object(recipient)
                        if tagged_snapshots.iter().any(|snapshot| snapshot.object_id == recipient))
            })
        }
        TaggedOpbjectRelation::SoulbondPartnerOfTagged => tagged_snapshots.iter().any(|snapshot| {
            game.soulbond_partner(snapshot.object_id) == Some(subject.subject_object_id())
        }),
        TaggedOpbjectRelation::BandedWithTagged => tagged_snapshots.iter().any(|snapshot| {
            objects_are_banded_together(game, snapshot.object_id, subject.subject_object_id())
        }),
        // The exact negation of `IsTaggedObject`: a tagged card that a
        // resolving instruction just moved ("exile that card. If you do,
        // return each other card exiled with this artifact", Mimic Vat) is
        // the same card under its stable identity, not an "other" card.
        TaggedOpbjectRelation::IsNotTaggedObject => tagged_snapshots.iter().all(|snapshot| {
            snapshot.object_id != subject.subject_object_id()
                && snapshot.stable_id != subject.subject_stable_id()
        }),
        // The tagged stack object's chosen object targets. A spell names its
        // own object id; an ability names its source's id (or its reserved
        // stack id), so either identity selects the entry.
        // CR 106.1b: compare the mana types each object's mana abilities
        // could produce.
        TaggedOpbjectRelation::SharesProducibleManaTypeWithTagged => {
            let producible = |id: ObjectId, controller: PlayerId| -> Vec<crate::mana::ManaSymbol> {
                let Some(object) = game.object(id) else {
                    return Vec::new();
                };
                object
                    .abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        AbilityKind::Activated(activated) => {
                            Some(activated.inferred_mana_symbols(game, id, controller))
                        }
                        _ => None,
                    })
                    .flatten()
                    .collect()
            };
            let subject_symbols =
                producible(subject.subject_object_id(), subject.subject_controller());
            !subject_symbols.is_empty()
                && tagged_snapshots.iter().any(|snapshot| {
                    producible(snapshot.object_id, snapshot.controller)
                        .iter()
                        .any(|symbol| subject_symbols.contains(symbol))
                })
        }
        TaggedOpbjectRelation::TargetedByTaggedObject => {
            let subject_id = subject.subject_object_id();
            tagged_snapshots.iter().any(|snapshot| {
                game.stack.iter().any(|entry| {
                    (entry.object_id == snapshot.object_id
                        || entry.target_id() == snapshot.object_id)
                        && entry.targets.iter().any(|target| {
                            matches!(
                                target,
                                crate::game_state::Target::Object(id) if *id == subject_id
                            )
                        })
                })
            })
        }
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
    pub source_number_owner: Option<crate::source_numbers::NumberChoiceOwner>,

    /// The player casting the spell currently being evaluated, if any.
    pub caster: Option<PlayerId>,

    /// The candidate spell whose cost is being evaluated before casting.
    pub prospective_cast: Option<ObjectId>,

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
    pub defending_player_reference: Option<crate::combat_state::DefendingPlayerReference>,

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
    pub counter_removal_declaration: Option<crate::cost::CounterRemovalDeclaration>,
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

    /// The stack object (by [`crate::game_state::StackEntry::target_id`]) a
    /// stack-qualified filter is being evaluated for. Abilities share their
    /// source's object id, so this picks which stack object the source's
    /// characteristics are matched on behalf of.
    pub stack_entry: Option<ObjectId>,

    /// Controller and owner of the candidate object an enclosing filter is
    /// comparing against a candidate-relative operand
    /// ([`ObjectRef::FilterCandidate`]).
    pub filter_candidate_players: Option<(PlayerId, PlayerId)>,

    /// Last-known battlefield snapshots of the permanents that just left the
    /// battlefield in the event being matched. A leaves-the-battlefield
    /// trigger looks back in time (CR 603.10a), so battlefield aggregates
    /// ("the greatest power among creatures that player controls") still
    /// count them.
    pub departed_battlefield_lookback: Option<std::sync::Arc<[crate::snapshot::ObjectSnapshot]>>,

    /// The event that triggered the ability whose filter this is, and the
    /// amount it bound, so a filter bound such as "with mana value less than
    /// or equal to that damage" reads the same event its effect does.
    pub triggering_event: Option<crate::triggers::TriggerEvent>,
    pub event_value_amount: Option<i32>,
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

    pub fn with_prospective_cast(mut self, spell: ObjectId) -> Self {
        self.prospective_cast = Some(spell);
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
                    .is_some_and(|rhs| i64::from(value) == rhs)
            }
            Comparison::NotEqualExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| i64::from(value) != rhs)
            }
            Comparison::LessThanExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| i64::from(value) < rhs)
            }
            Comparison::LessThanOrEqualExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| i64::from(value) <= rhs)
            }
            Comparison::GreaterThanExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| i64::from(value) > rhs)
            }
            Comparison::GreaterThanOrEqualExpr(rhs) => {
                resolve_filter_comparison_rhs_value(rhs, game, ctx, stack_entry)
                    .is_some_and(|rhs| i64::from(value) >= rhs)
            }
            _ => self.satisfies(value),
        }
    }
}

pub(crate) trait ParityRequirementRuntimeExt {
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
            Self::NotChosen => match Self::Chosen.resolve(game, source)? {
                Self::Odd => Some(Self::Even),
                Self::Even => Some(Self::Odd),
                _ => None,
            },
        }
    }

    fn matches(self, value: i32, game: &crate::game_state::GameState, ctx: &FilterContext) -> bool {
        match self.resolve(game, ctx.source) {
            Some(Self::Odd) => value.rem_euclid(2) == 1,
            Some(Self::Even) => value.rem_euclid(2) == 0,
            Some(Self::Chosen | Self::NotChosen) | None => false,
        }
    }
}

fn resolve_filter_comparison_rhs_value(
    rhs: &crate::effect::Value,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
    stack_entry: Option<&crate::game_state::StackEntry>,
) -> Option<i64> {
    use crate::effect::Value;
    use crate::target::ChooseSpec;

    fn total_counters(counters: &std::collections::BTreeMap<CounterType, u32>) -> i64 {
        counters.values().map(|n| i64::from(*n)).sum::<i64>()
    }

    fn resolve_x_value(
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
        stack_entry: Option<&crate::game_state::StackEntry>,
    ) -> Option<i64> {
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
            .map(|value| value as i64)
    }

    fn snapshot_pt(snapshot: &ObjectSnapshot, power: bool) -> Option<i64> {
        if power {
            snapshot.power.map(i64::from)
        } else {
            snapshot.toughness.map(i64::from)
        }
    }

    fn current_object_pt(
        game: &crate::game_state::GameState,
        object_id: ObjectId,
        power: bool,
    ) -> Option<i64> {
        let object = game.object(object_id)?;
        if power {
            game.calculated_power(object_id)
                .or_else(|| object.power())
                .map(i64::from)
        } else {
            game.calculated_toughness(object_id)
                .or_else(|| object.toughness())
                .map(i64::from)
        }
    }

    fn referenced_snapshot_pt(
        game: &crate::game_state::GameState,
        snapshot: &ObjectSnapshot,
        power: bool,
    ) -> Option<i64> {
        // A captured identity is a reference, not a frozen characteristic.
        // Read the same live incarnation through the layer system; use its
        // recorded LKI only after that incarnation leaves its expected zone.
        if game
            .object(snapshot.object_id)
            .is_some_and(|object| object.zone == snapshot.zone && !game.is_phased_out(object.id))
        {
            current_object_pt(game, snapshot.object_id, power)
        } else {
            // A tag may have been captured before the move while another
            // effect was still resolving, so it need not have passed through
            // the pending-stack departure refresh. Prefer the exact departure
            // receipt (including a currently staged simultaneous move).
            let departure = crate::effects::helpers::latest_zone_change_snapshot_for_object(
                game,
                snapshot.object_id,
            )
            .filter(|departure| departure.zone == snapshot.zone);
            snapshot_pt(departure.as_ref().unwrap_or(snapshot), power)
        }
    }

    fn resolve_pt_choose_spec(
        spec: &ChooseSpec,
        game: &crate::game_state::GameState,
        ctx: &FilterContext,
        power: bool,
    ) -> Option<i64> {
        match spec.base() {
            ChooseSpec::Source => current_object_pt(game, ctx.source?, power).or_else(|| {
                ctx.source_snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot_pt(snapshot, power))
            }),
            ChooseSpec::SpecificObject(object_id) => current_object_pt(game, *object_id, power),
            ChooseSpec::Tagged(tag) => ctx
                .tagged_objects
                .get(tag)
                .and_then(|snapshots| snapshots.first())
                .and_then(|snapshot| referenced_snapshot_pt(game, snapshot, power)),
            ChooseSpec::Object(_) | ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget
                if spec.is_target() =>
            {
                ctx.target_objects
                    .first()
                    .and_then(|snapshot| referenced_snapshot_pt(game, snapshot, power))
            }
            _ => None,
        }
    }

    fn aggregate_tagged_snapshots<'a>(
        filter: &ObjectFilter,
        ctx: &'a FilterContext,
    ) -> Option<Vec<&'a ObjectSnapshot>> {
        let only_is_tagged_constraints = !filter.tagged_constraints.iter().any(|entry| entry.tag.as_str().starts_with("source_collection:"))
            && !filter.tagged_constraints.is_empty()
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
    ) -> Option<i64> {
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
            .filter_map(|object| current_object_pt(game, object.id, power))
            .chain(
                departed_battlefield_lookback(filter, game, ctx)
                    .filter_map(|snapshot| snapshot_pt(snapshot, power)),
            );
        if greatest { values.max() } else { values.min() }
    }

    /// The permanents that just left the battlefield in the event being
    /// matched, as they last existed there, when `filter` looks at the
    /// battlefield (CR 603.10a). Permanents still on the battlefield are
    /// already counted live.
    fn departed_battlefield_lookback<'c>(
        filter: &'c ObjectFilter,
        game: &'c GameState,
        ctx: &'c FilterContext,
    ) -> impl Iterator<Item = &'c crate::snapshot::ObjectSnapshot> + 'c {
        let applies = filter.zone == Some(Zone::Battlefield);
        ctx.departed_battlefield_lookback
            .as_deref()
            .filter(|_| applies)
            .unwrap_or(&[])
            .iter()
            .filter(move |snapshot| {
                snapshot.zone == Zone::Battlefield
                    && !game
                        .object(snapshot.object_id)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
                    && filter.matches_snapshot(snapshot, ctx, game)
            })
    }

    fn aggregate_mana_value(
        filter: &ObjectFilter,
        game: &GameState,
        ctx: &FilterContext,
        greatest: bool,
    ) -> Option<i64> {
        if filter.cast_this_turn && filter.zone == Some(Zone::Stack) {
            let snapshots = game.turn_store.turn_history.spell_cast_snapshot_history();
            let values = snapshots
                .iter()
                .filter(|snapshot| filter.matches_snapshot(snapshot, ctx, game))
                .map(|snapshot| i64::from(snapshot_mana_value_for_filter(snapshot)));
            return if greatest { values.max() } else { values.min() };
        }

        if let Some(snapshots) = aggregate_tagged_snapshots(filter, ctx) {
            let values = snapshots
                .into_iter()
                .filter(|snapshot| filter.matches_snapshot(snapshot, ctx, game))
                .map(|snapshot| i64::from(snapshot_mana_value_for_filter(snapshot)));
            return if greatest { values.max() } else { values.min() };
        }

        let values = game
            .objects_in_deterministic_order()
            .into_iter()
            .filter(|object| filter.matches(object, ctx, game))
            .map(|object| i64::from(object_mana_value_for_filter(object)))
            .chain(
                departed_battlefield_lookback(filter, game, ctx)
                    .map(|snapshot| i64::from(snapshot_mana_value_for_filter(snapshot))),
            );
        if greatest { values.max() } else { values.min() }
    }

    match rhs {
        Value::SurfaceHinted { value, .. } => {
            resolve_filter_comparison_rhs_value(value, game, ctx, stack_entry)
        }
        Value::SourceChosenNumber { if_unset, pair } => {
            // A delayed trigger's watched object can differ from the host
            // that owns this linked choice. Admission fixes the numeric host.
            let source = ctx
                .source_number_owner
                .as_ref()
                .map(|owner| owner.host)
                .or(ctx.source)?;
            match crate::source_numbers::read(
                game,
                source,
                *pair,
                ctx.source_number_owner.as_ref(),
                ctx.source_snapshot.as_ref(),
            ) {
                Ok(number) => number.map(i64::from).or_else(|| if_unset.map(i64::from)),
                Err(error) => {
                    game.record_token_resource_failure(&error);
                    None
                }
            }
        }
        Value::Fixed(value) => Some(i64::from(*value)),
        Value::DraftNotedHighestNumber { card_name } => Some(i64::from(
            game.draft_noted_highest_number(ctx.you?, card_name),
        )),
        Value::X => resolve_x_value(game, ctx, stack_entry),
        Value::XTimes(multiplier) => {
            resolve_x_value(game, ctx, stack_entry)
                .map(|value| value.saturating_mul(i64::from(*multiplier)))
        }
        value
            if !ctx
                .effect_outcomes
                .contains_key(&crate::effect::EffectId::ACTIVATION_COUNTER_COST)
                && ctx
                    .counter_removal_declaration
                    .is_some_and(|declared| declared.value(ctx.source, value).is_some()) =>
        {
            ctx.counter_removal_declaration
                .and_then(|declared| declared.value(ctx.source, value))
        }
        Value::EffectMetric { .. }
        | Value::EffectMetricOffset { .. }
        | Value::PriorEffectMetric { .. } => {
            // Metrics describe a previous instruction, not the candidate
            // object's characteristics. Retain the producer's outcome memory.
            let mut execution =
                crate::effects::ExecutionContext::new_default(ctx.source?, ctx.you?);
            execution.effect_outcomes = ctx.effect_outcomes.clone();
            execution.tagged_objects = ctx.tagged_objects.clone();
            execution.source_snapshot = ctx.source_snapshot.clone();
            execution.x_value = ctx.x_value;
            crate::effects::helpers::resolve_value_wide(game, rhs, &execution).ok()
        }
        Value::EffectValue(effect_id) => ctx.effect_outcomes.get(effect_id).and_then(|outcome| {
            outcome.as_count().or_else(|| {
                outcome
                    .explicit_objects()
                    .map(|objects| objects.len() as i64)
            })
        }),
        Value::EffectValueOffset(effect_id, offset) => ctx
            .effect_outcomes
            .get(effect_id)
            .and_then(|outcome| {
                outcome.as_count().or_else(|| {
                    outcome
                        .explicit_objects()
                        .map(|objects| objects.len() as i64)
                })
            })
            .map(|value| value.saturating_add(i64::from(*offset))),
        Value::Add(left, right) => Some(
            resolve_filter_comparison_rhs_value(left, game, ctx, stack_entry)?
                .saturating_add(resolve_filter_comparison_rhs_value(right, game, ctx, stack_entry)?),
        ),
        Value::Scaled(inner, multiplier) => {
            resolve_filter_comparison_rhs_value(inner, game, ctx, stack_entry)
                .map(|value| value.saturating_mul(i64::from(*multiplier)))
        }
        Value::DividedRoundedDown(inner, divisor) if *divisor != 0 => {
            resolve_filter_comparison_rhs_value(inner, game, ctx, stack_entry).map(|value| {
                value.checked_div_euclid(i64::from(*divisor)).unwrap_or(i64::MAX)
            })
        }
        Value::Min(left, right) => Some(
            resolve_filter_comparison_rhs_value(left, game, ctx, stack_entry)?.min(
                resolve_filter_comparison_rhs_value(right, game, ctx, stack_entry)?,
            ),
        ),
        Value::Count(filter) => {
            let objects = game.objects_in_deterministic_order();
            let matching = objects.iter().copied().filter(|object| filter.matches(object, ctx, game));
            Some(crate::static_abilities::set_constrained_count(filter, matching, game) as i64)
        }
        Value::DistinctNames(filter) => {
            let mut names = Vec::new();
            if let Some(snapshots) = aggregate_tagged_snapshots(filter, ctx) {
                for snapshot in snapshots {
                    if filter.matches_snapshot(snapshot, ctx, game) {
                        names.push(ObjectNames::snapshot(snapshot).into());
                    }
                }
            } else {
                let ids: Vec<_> = game.objects_in_deterministic_order().into_iter()
                    .filter(|object| !game.is_phased_out(object.id) && filter.matches(object, ctx, game))
                    .map(|object| object.id).collect();
                let frame = game.try_current_characteristics_batch(&ids).ok()?;
                for id in ids {
                    names.push(ObjectNames::layered(game.object(id)?, frame.get(&id)?, game).into());
                }
            }
            Some(name_groups::distinct_count(names, filter.distinct_names, game))
        }
        Value::CountScaled(filter, factor) => {
            let objects = game.objects_in_deterministic_order();
            let matching = objects.iter().copied().filter(|object| filter.matches(object, ctx, game));
            let count = crate::static_abilities::set_constrained_count(filter, matching, game) as i64;
            Some(count.saturating_mul(i64::from(*factor)))
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
                        let count = counts.entry((controller_group, subtype)).or_insert(0i64);
                        *count = count.saturating_add(1);
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
                return Some(colors.count() as i64);
            }
            let mut colors = ColorSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game)
                    && let Some(chars) = checked_relation_characteristics(game, object.id).ok()?
                {
                    colors = colors.union(chars.colors);
                }
            }
            Some(colors.count() as i64)
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
            Some(seen.len() as i64)
        }
        Value::PlaneswalkerTypesAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    let subtypes = game
                        .current_subtypes(object.id)
                        .unwrap_or_else(|| object.subtypes.to_vec());
                    for subtype in subtypes {
                        if subtype.is_planeswalker_subtype() {
                            seen.insert(subtype);
                        }
                    }
                }
            }
            Some(seen.len() as i64)
        }
        Value::SubtypesAmong { family, filter } => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    let subtypes = game
                        .current_subtypes(object.id)
                        .unwrap_or_else(|| object.subtypes.to_vec());
                    for subtype in subtypes {
                        if family.is_none_or(|family| family.contains(subtype)) {
                            seen.insert(subtype);
                        }
                    }
                }
            }
            Some(seen.len() as i64)
        }
        Value::CardTypesAmong(filter) | Value::PermanentTypesAmong(filter) => {
            let permanent_only = matches!(rhs, Value::PermanentTypesAmong(_));
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    let card_types = game
                        .current_card_types(object.id)
                        .unwrap_or_else(|| object.card_types.to_vec());
                    for card_type in card_types {
                        if !permanent_only || card_type.is_permanent_type() {
                            seen.insert(card_type);
                        }
                    }
                }
            }
            Some(seen.len() as i64)
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
            Some(seen.len() as i64)
        }
        Value::UnlockedDoorsAmong(filter) => Some(
            game.objects_in_deterministic_order()
                .into_iter()
                .filter(|object| filter.matches(object, ctx, game))
                .map(|object| {
                    i64::from(crate::effects::helpers::room_unlocked_door_count(
                        game, object,
                    ))
                })
                .sum(),
        ),
        Value::DistinctManaValues(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    seen.insert(object_mana_value_for_filter(object));
                }
            }
            Some(seen.len() as i64)
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
            Some(seen.len() as i64)
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
            Some(seen.len() as i64)
        }
        Value::DistinctCounterTypesAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    seen.extend(object.counters.keys().copied());
                }
            }
            Some(seen.len() as i64)
        }
        Value::GreatestPower(filter) => aggregate_pt(filter, game, ctx, true, true),
        Value::GreatestToughness(filter) => aggregate_pt(filter, game, ctx, false, true),
        Value::GreatestManaValue(filter) => aggregate_mana_value(filter, game, ctx, true),
        Value::LeastPower(filter) => aggregate_pt(filter, game, ctx, true, false),
        Value::LeastToughness(filter) => aggregate_pt(filter, game, ctx, false, false),
        Value::LeastManaValue(filter) => aggregate_mana_value(filter, game, ctx, false),
        Value::PlayerCounters(player, counter_type) => {
            if *player == PlayerFilter::You && ctx.you.is_none() {
                return None;
            }
            let mut seen_teams = std::collections::HashSet::new();
            let count = game
                .players
                .iter()
                .filter(|candidate| player_filter_matches_game(player, candidate.id, game, ctx))
                .filter(|candidate| {
                    *counter_type != CounterType::Poison
                        || game.two_headed_giant().is_none()
                        || game
                            .team_index_for(candidate.id)
                            .is_none_or(|team| seen_teams.insert(team))
                })
                .fold(0i64, |count, candidate| {
                    count.saturating_add(i64::from(candidate.counter_count(*counter_type)))
                });
            Some(count)
        }
        Value::CountersOnSource(counter_type) => {
            let counters = game
                .object(ctx.source?)
                .map(|source| source.counters.counts())
                .or_else(|| {
                    ctx.source_snapshot
                        .as_ref()
                        .map(|snapshot| &snapshot.counters)
                })?;
            Some(counters.get(counter_type).copied().unwrap_or(0) as i64)
        }
        Value::SourcePower => current_object_pt(game, ctx.source?, true).or_else(|| {
            ctx.source_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot_pt(snapshot, true))
        }),
        Value::SourceToughness => current_object_pt(game, ctx.source?, false).or_else(|| {
            ctx.source_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot_pt(snapshot, false))
        }),
        Value::BasePowerOf(spec) => {
            let live = |id| {
                game.calculated_characteristics(id)
                    .and_then(|chars| chars.base_power)
            };
            let recorded = |snapshot: &ObjectSnapshot| {
                if game.object(snapshot.object_id).is_some_and(|object| {
                    object.zone == snapshot.zone && !game.is_phased_out(object.id)
                }) {
                    live(snapshot.object_id)
                } else {
                    crate::effects::helpers::latest_zone_change_snapshot_for_object(
                        game,
                        snapshot.object_id,
                    )
                    .filter(|departure| departure.zone == snapshot.zone)
                    .as_ref()
                    .unwrap_or(snapshot)
                    .base_power
                }
            };
            match spec.base() {
                ChooseSpec::Source => ctx
                    .source
                    .and_then(live)
                    .or_else(|| ctx.source_snapshot.as_ref().and_then(recorded))
                    .map(i64::from),
                ChooseSpec::SpecificObject(id) => live(*id).map(i64::from),
                ChooseSpec::Tagged(tag) => ctx
                    .tagged_objects
                    .get(tag)
                    .and_then(|objects| objects.first())
                    .and_then(recorded)
                    .map(i64::from),
                ChooseSpec::Object(_) if spec.is_target() => {
                    ctx.target_objects.first().and_then(recorded).map(i64::from)
                }
                _ => None,
            }
        }
        Value::PowerOf(spec) => resolve_pt_choose_spec(spec, game, ctx, true),
        Value::ToughnessOf(spec) => resolve_pt_choose_spec(spec, game, ctx, false),
        Value::NamedActionCount {object,label,this_turn} => {
            let id=match object.base() {ChooseSpec::Source=>ctx.source?,ChooseSpec::SpecificObject(id)=>*id,
                ChooseSpec::Tagged(tag)=>ctx.tagged_objects.get(tag)?.first()?.object_id,_=>return None};
            Some(i64::from(game.named_action_count(id,label,*this_turn)))
        }
        Value::CountersOn(spec, counter_type) => match spec.base() {
            ChooseSpec::Source => {
                let source = game.object(ctx.source?)?;
                Some(match counter_type {
                    Some(counter_type) => {
                        source.counters.get(counter_type).copied().unwrap_or(0) as i64
                    }
                    None => total_counters(&source.counters),
                })
            }
            ChooseSpec::Tagged(tag) => {
                let snapshots = ctx.tagged_objects.get(tag)?;
                let snapshot = snapshots.first()?;
                Some(match counter_type {
                    Some(counter_type) => {
                        snapshot.counters.get(counter_type).copied().unwrap_or(0) as i64
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
                    .map(|cost| cost.mana_value() as i64)
                    .or_else(|| {
                        ctx.source_snapshot
                            .as_ref()
                            .and_then(|snapshot| snapshot.mana_cost.as_ref())
                            .map(|cost| cost.mana_value() as i64)
                    });
                Some(mana_value.unwrap_or(0))
            }
            ChooseSpec::Tagged(tag) => {
                let snapshot = ctx.tagged_objects.get(tag)?.first()?;
                Some(
                    snapshot
                        .mana_cost
                        .as_ref()
                        .map_or(0, |cost| cost.mana_value() as i64),
                )
            }
            // "with greater mana value than that creature" (Evil's Thrall):
            // a gate comparing against the announced target reads it from
            // the resolving context's targets, as `resolve_pt_choose_spec`
            // does for power and toughness.
            ChooseSpec::Object(_) | ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget
                if spec.is_target() =>
            {
                let snapshot = ctx.target_objects.first()?;
                Some(
                    snapshot
                        .mana_cost
                        .as_ref()
                        .map_or(0, |cost| cost.mana_value() as i64),
                )
            }
            _ => None,
        },
        Value::LifeGainedThisTurn(player_filter) | Value::LifeLostThisTurn(player_filter) => {
            let players = game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game() && player_filter.matches_player(player.id, ctx)
                })
                .map(|player| player.id)
                .collect::<Vec<_>>();
            let history = &game.turn_store.turn_history;
            Some(if matches!(rhs, Value::LifeGainedThisTurn(_)) {
                history.total_life_gained_for_players(&players)
            } else {
                history.total_life_lost_for_players(&players)
            } as i64)
        }
        Value::UnspentMana(player_filter) => Some(
            game.players
                .iter()
                .filter(|player| {
                    player.is_in_game() && player_filter.matches_player(player.id, ctx)
                })
                .filter_map(|player| game.player_mana_pool(player.id))
                .map(|pool| pool.total_wide() as i64)
                .sum(),
        ),
        Value::DevotionToColors { player, colors } => Some(
            game.players.iter()
                .filter(|candidate| candidate.is_in_game() && player.matches_player(candidate.id, ctx))
                .map(|candidate| game.devotion_to_colors(candidate.id, *colors) as i64).sum(),
        ),
        Value::Devotion { player, color } => Some(
            game.players
                .iter()
                .filter(|candidate| {
                    candidate.is_in_game() && player.matches_player(candidate.id, ctx)
                })
                .map(|candidate| game.devotion_to_color(candidate.id, *color) as i64)
                .sum(),
        ),
        // Every other quantity ("your life total", "the amount of mana spent
        // to cast it", "that damage") means what it means anywhere else, so
        // evaluate it with the shared value interpreter rather than failing
        // the comparison closed.
        _ => resolve_filter_comparison_rhs_with_value_eval(rhs, game, ctx),
    }
}

fn resolve_filter_comparison_rhs_with_value_eval(
    rhs: &crate::effect::Value,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
) -> Option<i64> {
    let controller = ctx.you?;
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let source = ctx
        .source
        .or_else(|| ctx.source_snapshot.as_ref().map(|snapshot| snapshot.object_id))
        .unwrap_or(ObjectId::from_raw(u64::MAX));
    let mut exec = crate::effects::ExecutionContext::new(source, controller, &mut decision_maker);
    exec.source_snapshot = ctx.source_snapshot.clone();
    exec.x_value = ctx.x_value;
    exec.iteration.iterated_player = ctx.iterated_player;
    exec.tagged_objects = ctx.tagged_objects.clone();
    exec.tagged_players = ctx.tagged_players.clone();
    exec.effect_outcomes = ctx.effect_outcomes.clone();
    if let Some(event) = ctx.triggering_event.clone() {
        exec = exec.with_triggering_event(event);
    }
    exec.event_value_amount = ctx.event_value_amount;
    crate::effects::helpers::resolve_value(game, rhs, &exec)
        .ok()
        .map(i64::from)
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
        ObjectRef::FilterCandidate => None,
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
        ObjectRef::FilterCandidate => None,
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
        ObjectRef::FilterCandidate => Vec::new(),
    }
}

fn creature_was_blocked_by_ref(
    game: &GameState,
    ctx: &FilterContext,
    attacker: ObjectId,
    blocker_ref: &ObjectRef,
) -> bool {
    let mut blockers = resolve_object_ref_ids(blocker_ref, ctx);
    // "When this dies during combat, it deals 6 damage to each creature it
    // blocked this combat" (Cathedral Membrane): the triggering reference was
    // re-pointed at the graveyard card, but the block was declared by the
    // battlefield incarnation, so consult its pre-move identity (CR 608.2h).
    if let ObjectRef::Tagged(tag) = blocker_ref
        && let Some(history) = ctx
            .tagged_objects
            .get(&crate::tag::TagKey::from(format!("__pre_move_history__{}", tag.as_str())))
    {
        blockers.extend(history.iter().map(|snapshot| snapshot.object_id));
    }
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

/// "another target creature attacking the same player or planeswalker"
/// (Kitesail Skirmisher): both creatures are attacking, and their attack
/// targets name the same player, planeswalker, or battle (CR 506.3).
fn object_attacks_same_defender_as_source(
    game: &GameState,
    ctx: &FilterContext,
    object_id: ObjectId,
) -> bool {
    let Some(combat) = &game.combat else {
        return false;
    };
    let Some(subject_target) = crate::combat_state::get_attack_target(combat, object_id) else {
        return false;
    };
    let source_ids = ctx.source.into_iter().chain(
        ctx.source_snapshot
            .as_ref()
            .map(|snapshot| snapshot.object_id),
    );
    source_ids.into_iter().any(|source_id| {
        crate::combat_state::get_attack_target(combat, source_id)
            .is_some_and(|source_target| source_target == subject_target)
    })
}

/// "a creature at random this Aura can enchant" (Infectious Rage): the
/// current source Aura's enchant restriction and protection (CR 303.4,
/// 702.16c) allow it to be attached to the candidate. The Aura is usually
/// no longer on the battlefield, so follow its stable identity to the card
/// in its current zone.
fn object_could_be_enchanted_by_source(
    game: &GameState,
    ctx: &FilterContext,
    object_id: ObjectId,
) -> bool {
    let current_source = ctx
        .source
        .filter(|source_id| game.object(*source_id).is_some())
        .or_else(|| {
            ctx.source_snapshot
                .as_ref()
                .and_then(|snapshot| game.find_object_by_stable_id(snapshot.stable_id))
        });
    let Some(aura_id) = current_source else {
        return false;
    };
    let Some(controller) = ctx
        .you
        .or_else(|| game.object(aura_id).map(|aura| aura.owner))
    else {
        return false;
    };
    crate::effects::permanents::aura_can_enter_attached_to(
        game,
        aura_id,
        controller,
        crate::object::AttachmentTarget::Object(object_id),
    )
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
            || combat.blockers.get(&object_id).is_some_and(|blockers| blockers.contains(&source_id))
            // A source that left the battlefield was removed from combat
            // (CR 506.4), but its ability still refers to the creatures it
            // was in combat with through last known information (CR 608.2h).
            || (game
                .object(source_id)
                .is_none_or(|source| source.zone != crate::zone::Zone::Battlefield)
                && (game.turn_store.turn_history.creature_was_blocked_by_in_combat(
                    source_id, object_id, game.turn_store.combat_phases_started_this_turn,
                ) || game.turn_store.turn_history.creature_was_blocked_by_in_combat(
                    object_id, source_id, game.turn_store.combat_phases_started_this_turn,
                )))
    })
}

fn creature_blocked_or_was_blocked_by_matching_this_turn(
    game: &GameState,
    ctx: &FilterContext,
    creature: ObjectId,
    partner_filter: &ObjectFilter,
) -> bool {
    let mut missing_evidence = false;
    for event in game
        .turn_store
        .turn_history
        .projected_records()
        .filter_map(|record| {
            record
                .event
                .downcast::<crate::events::combat::CreatureBlockedEvent>()
        })
    {
        let (partner_id, snapshot) = if event.blocker == creature {
            (event.attacker, event.attacker_snapshot.as_ref())
        } else if event.attacker == creature {
            (event.blocker, event.blocker_snapshot.as_ref())
        } else {
            continue;
        };
        let Some(snapshot) = snapshot.filter(|snapshot| snapshot.object_id == partner_id) else {
            missing_evidence = true;
            continue;
        };
        if partner_filter.matches_snapshot(snapshot, ctx, game) {
            return true;
        }
    }
    // One complete matching occurrence proves the existential query even when
    // a different historical occurrence is unavailable. Otherwise unknown is
    // not a negative result and cannot be inverted or published as complete.
    if missing_evidence {
        game.record_token_resource_failure(&crate::effects::ExecutionError::IncompleteEvidence(
            "combat partner history requires exact declaration characteristics".into(),
        ));
    }
    false
}

/// Legacy boolean filter adapters cannot return an execution error. Preserve
/// it on the shared checked-action latch before returning any provisional
/// boolean, so neither negation nor a zero match count can complete the action.
pub(crate) fn checked_spell_program_for_filter(
    game: &GameState,
    object: ObjectId,
) -> Result<
    crate::resolution::ResolutionProgram,
    crate::static_ability_processor::StaticEffectDiscoveryError,
> {
    game.current_spell_program(object).inspect_err(|error| {
        game.record_token_resource_failure(&crate::effects::ExecutionError::ContinuousDiscovery(
            error.clone(),
        ));
    })
}

fn effects_for_stack_entry(
    game: &crate::game_state::GameState,
    entry: &crate::game_state::StackEntry,
) -> Vec<crate::effect::Effect> {
    if let Some(ref effects) = entry.ability_effects {
        return effects.to_vec();
    }

    match checked_spell_program_for_filter(game, entry.object_id) {
        Ok(effects) => effects.to_vec(),
        Err(_) => Vec::new(), // The checked owner retains the incomplete latch.
    }
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

    if constraint.exclude_current_target_controllers {
        let Some(candidate) = game.object(object_id) else {
            return false;
        };
        let controller = game.controller_of(candidate);
        if entry.targets.iter().any(|target| match target {
            crate::game_state::Target::Object(id) => game
                .object(*id)
                .is_some_and(|object| game.controller_of(object) == controller),
            _ => false,
        }) {
            return false;
        }
    }

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
            PlayerFilter::PlayerToYourLeft | PlayerFilter::OpponentToYourLeft | PlayerFilter::PlayerToYourRight => false,

            // The monarch designation is game state, read by
            // `player_filter_matches_game`.
            PlayerFilter::Monarch => false,

            PlayerFilter::Active => ctx.active_player.is_some_and(|ap| player == ap),

            PlayerFilter::Defending => {
                if ctx.defending_player_reference.is_none() && ctx.defending_players.is_empty() {
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
            PlayerFilter::TurnHistory(_) => false,
            // Source-relative turn history requires access to GameState and
            // is evaluated by `player_filter_matches_game` below.
            PlayerFilter::AttackedBySourceThisTurn => false,
            PlayerFilter::WasDealtDamageBySourceThisGame { base, .. } => {
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
            PlayerFilter::ControlsMost { .. } | PlayerFilter::ControlsFewestTied { .. } | PlayerFilter::OwnsMatchingObject { .. } | PlayerFilter::ControlsMatchingObject { .. } | PlayerFilter::WithMatchingAttachments { .. } | PlayerFilter::MostLifeTiedAmong { .. } | PlayerFilter::QuantityGreaterThanPlayer { .. } => false,
            PlayerFilter::OpponentOf(_)
            | PlayerFilter::PlayerToLeftOf(_)
            | PlayerFilter::MaxSpeed { .. } => false,
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
            // A battle's protector is game state, read by
            // `player_filter_matches_game`; the owner is on the snapshot.
            PlayerFilter::TargetPlayerOrRelatedPlayerOfTarget(relation) => {
                ctx.target_players.contains(&player)
                    || (*relation == ironsmith_core::TargetObjectPlayerRelation::Owner
                        && ctx
                            .target_objects
                            .first()
                            .is_some_and(|snapshot| snapshot.owner == player))
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
            PlayerFilter::ControllerOf(ObjectRef::FilterCandidate) => ctx
                .filter_candidate_players
                .is_some_and(|(controller, _)| controller == player),
            PlayerFilter::OwnerOf(ObjectRef::FilterCandidate) => ctx
                .filter_candidate_players
                .is_some_and(|(_, owner)| owner == player),
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

/// Whether `player` did the given action this turn (CR 500.1 turn history).
pub(crate) fn player_turn_history_matches(
    game: &crate::game_state::GameState,
    player: PlayerId,
    history: ironsmith_core::PlayerTurnHistoryFilter,
) -> bool {
    match history {
        ironsmith_core::PlayerTurnHistoryFilter::CastSpell => game
            .turn_store
            .turn_history
            .spell_cast_snapshot_history()
            .iter()
            .any(|snapshot| snapshot.controller == player),
        ironsmith_core::PlayerTurnHistoryFilter::AttackedWithCreature => game
            .turn_store
            .turn_history
            .creatures_attacked_by_player_this_turn
            .get(&player)
            .is_some_and(|creatures| !creatures.is_empty()),
        ironsmith_core::PlayerTurnHistoryFilter::WasDealtDamage => game
            .turn_store
            .turn_history
            .player_was_dealt_damage_this_turn(player),
        // Needs the filter context's controller; see
        // `player_attacked_you_or_your_planeswalker_this_turn`.
        ironsmith_core::PlayerTurnHistoryFilter::AttackedYouOrYourPlaneswalker => false,
    }
}

/// CR 508.1: `player` declared one or more creatures attacking `you` or a
/// planeswalker `you` controlled this turn.
pub(crate) fn player_attacked_you_or_your_planeswalker_this_turn(
    game: &crate::game_state::GameState,
    player: PlayerId,
    you: PlayerId,
) -> bool {
    let Some(attackers) = game
        .turn_store
        .turn_history
        .creatures_attacked_by_player_this_turn
        .get(&player)
    else {
        return false;
    };
    game.turn_store
        .turn_history
        .projected_records()
        .any(|record| {
            let Some(event) = record.event.downcast::<CreatureAttackedEvent>() else {
                return false;
            };
            attackers.contains(&event.attacker)
                && match event.target {
                    crate::triggers::event::AttackEventTarget::Player(defender) => defender == you,
                    crate::triggers::event::AttackEventTarget::Planeswalker(planeswalker) => game
                        .controller_of_id(planeswalker)
                        .is_some_and(|controller| controller == you),
                    _ => false,
                }
        })
}

pub(crate) fn player_filter_matches_game(
    filter: &PlayerFilter,
    player: PlayerId,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
) -> bool {
    match filter {
        PlayerFilter::TurnHistory(
            ironsmith_core::PlayerTurnHistoryFilter::AttackedYouOrYourPlaneswalker,
        ) => ctx.you.is_some_and(|you| {
            player_attacked_you_or_your_planeswalker_this_turn(game, player, you)
        }),
        PlayerFilter::TurnHistory(history) => player_turn_history_matches(game, player, *history),
        PlayerFilter::Defending if ctx.defending_player_reference.is_some() => {
            match game.defending_player_candidates(ctx.defending_player_reference.unwrap()) {
                Ok(players) => {
                    players.contains(&player)
                        && ctx
                            .players_in_range
                            .as_ref()
                            .is_none_or(|range| range.contains(&player))
                }
                Err(error) => {
                    game.record_token_resource_failure(&error);
                    false
                }
            }
        }

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
        PlayerFilter::WasDealtDamageBySourceThisGame { base, this_turn } => {
            let Some(source) = ctx.source else {
                return false;
            };
            player_filter_matches_game(base, player, game, ctx)
                && if *this_turn {
                    game.source_dealt_damage_to_player_this_turn(source, player)
                } else {
                    game.source_dealt_damage_to_player_this_game(source, player)
                }
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
        PlayerFilter::OpponentToYourLeft => ctx.you.is_some_and(|you| {
            game.closest_in_game_player_to_left_matching(you, |candidate| {
                game.are_opponents(you, candidate)
            }) == Some(player)
        }),
        PlayerFilter::Monarch => game.is_monarch(player),
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
        PlayerFilter::MostLifeTiedAmong { base } => {
            if !player_filter_matches_game(base, player, game, ctx) {
                return false;
            }
            let Some(candidate_life) = game
                .player(player)
                .filter(|candidate| candidate.is_in_game())
                .map(|candidate| candidate.life)
            else {
                return false;
            };
            game.players
                .iter()
                .filter(|other| other.is_in_game() && other.id != player)
                .filter(|other| player_filter_matches_game(base, other.id, game, ctx))
                .all(|other| other.life <= candidate_life)
        }
        PlayerFilter::QuantityGreaterThanPlayer {
            base,
            other,
            quantity,
        } => {
            if !player_filter_matches_game(base, player, game, ctx) {
                return false;
            }
            let quantity_of = |id: PlayerId| {
                game.player(id).map(|p| match quantity {
                    ironsmith_core::PlayerQuantity::Life => i64::from(p.life),
                    ironsmith_core::PlayerQuantity::CardsInHand => p.hand.len() as i64,
                })
            };
            let Some(candidate_quantity) = quantity_of(player) else {
                return false;
            };
            let compared = game
                .players
                .iter()
                .filter(|p| p.is_in_game())
                .filter(|p| player_filter_matches_game(other, p.id, game, ctx))
                .map(|p| p.id)
                .collect::<Vec<_>>();
            !compared.is_empty()
                && compared
                    .iter()
                    .all(|id| quantity_of(*id).is_some_and(|q| candidate_quantity > q))
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan {
            player: reference_filter,
            filter: object_filter,
            fewer,
            minimum_difference,
            ..
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
                    if !game.are_opponents(reference.id, player) {
                        return false;
                    }
                    let candidate_count =
                        controlled_matching_object_count(game, player, object_filter, ctx);
                    let reference_count =
                        controlled_matching_object_count(game, reference.id, object_filter, ctx);
                    let margin = (*minimum_difference).max(1) as usize;
                    if *fewer {
                        reference_count >= candidate_count.saturating_add(margin)
                    } else {
                        candidate_count >= reference_count.saturating_add(margin)
                    }
                })
        }
        PlayerFilter::OwnsMatchingObject { filter } => {
            if ctx.players_in_range.as_ref().is_some_and(|players| !players.contains(&player)) {
                return false;
            }
            let mut object_ctx = ctx.clone();
            object_ctx.iterated_player = Some(player);
            game.zone_ids(filter.zone.unwrap_or(crate::Zone::Battlefield))
                .filter_map(|id| game.object(id))
                .any(|object| object.owner == player && filter.matches(object, &object_ctx, game))
        }
        PlayerFilter::WithMatchingAttachments { base, filter } => {
            if !player_filter_matches_game(base, player, game, ctx) { return false; }
            let mut object_ctx = ctx.clone();
            object_ctx.iterated_player = Some(player);
            game.battlefield.iter().copied().filter(|id| !game.is_phased_out(*id))
                .filter_map(|id| game.object(id))
                .any(|object| object.attached_to == Some(crate::object::AttachmentTarget::Player(player))
                    && filter.matches(object, &object_ctx, game))
        }
        PlayerFilter::ControlsMatchingObject { filter } => {
            if ctx.players_in_range.as_ref().is_some_and(|players| !players.contains(&player)) {
                return false;
            }
            let mut object_ctx = ctx.clone();
            object_ctx.iterated_player = Some(player);
            game.zone_ids(filter.zone.unwrap_or(crate::Zone::Battlefield))
                .filter_map(|id| game.object(id))
                .any(|object| {
                    game.controller_of(object) == player
                        && filter.matches(object, &object_ctx, game)
                })
        }
        PlayerFilter::ControlsFewestTied {
            filter: object_filter,
        } => {
            if ctx
                .players_in_range
                .as_ref()
                .is_some_and(|players| !players.contains(&player))
            {
                return false;
            }
            let counts = game
                .players
                .iter()
                .filter(|candidate| candidate.is_in_game())
                .filter(|candidate| {
                    ctx.players_in_range
                        .as_ref()
                        .is_none_or(|players| players.contains(&candidate.id))
                })
                .map(|candidate| {
                    (
                        candidate.id,
                        controlled_matching_object_count(game, candidate.id, object_filter, ctx),
                    )
                })
                .collect::<Vec<_>>();
            let Some(minimum) = counts.iter().map(|(_, count)| *count).min() else {
                return false;
            };
            counts
                .iter()
                .any(|(candidate, count)| *candidate == player && *count == minimum)
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
        PlayerFilter::OpponentOf(base) => game.players.iter().any(|other| {
            other.is_in_game()
                && game.are_opponents(other.id, player)
                && player_filter_matches_game(base, other.id, game, ctx)
        }),
        // "the player to their left" (CR 101.4a seating): the nearest in-game
        // player to the left of a player `base` names.
        PlayerFilter::PlayerToLeftOf(base) => game.players.iter().any(|other| {
            other.is_in_game()
                && player_filter_matches_game(base, other.id, game, ctx)
                && game.closest_in_game_player_to_left_matching(other.id, |_| true)
                    == Some(player)
        }),
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
        PlayerFilter::TargetPlayerOrRelatedPlayerOfTarget(
            ironsmith_core::TargetObjectPlayerRelation::Protector,
        ) => {
            ctx.target_players.contains(&player)
                || ctx
                    .target_objects
                    .first()
                    .and_then(|snapshot| game.battle_protector(snapshot.object_id))
                    == Some(player)
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

    // A non-battlefield zone counts the cards each player owns there ("whose
    // graveyard has fewer creature cards in it", Oath of Ghouls).
    if let Some(zone) = filter.zone
        && zone != crate::zone::Zone::Battlefield
    {
        return game
            .zone_ids(zone)
            .filter_map(|object_id| game.object(object_id))
            .filter(|object| object.owner == controller)
            .filter(|object| filter.matches(object, &object_ctx, game))
            .count();
    }
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
}

fn aura_could_enchant_host_filter(
    attachment: &crate::object::AuraAttachmentFilter,
    host_filter: &ObjectFilter,
    ctx: &FilterContext,
    game: &GameState,
) -> bool {
    use crate::object::AuraAttachmentFilterRuntimeExt;
    let concrete_host = host_filter.source || host_filter.specific.is_some()
        || host_filter.tagged_constraints.iter().any(|constraint| matches!(constraint.relation,
            TaggedOpbjectRelation::IsTaggedObject | TaggedOpbjectRelation::SameObjectId));
    if concrete_host {
        let remembered = ctx.tagged_objects.values().flatten()
            .chain(ctx.source_snapshot.iter())
            .chain(ctx.source.and_then(|source| game.source_characteristic_lki_snapshot(source, ctx.source_snapshot.as_ref())))
            .chain(host_filter.specific.and_then(|host| game.source_characteristic_lki_snapshot(host, None)))
            .filter(|snapshot| snapshot.zone == Zone::Battlefield
                && host_filter.matches_snapshot(snapshot, ctx, game))
            .collect::<Vec<_>>();
        if !remembered.is_empty() {
            return remembered.into_iter().any(|captured| {
                if let Some(current) = game.object(captured.object_id).filter(|object|
                    object.zone == Zone::Battlefield && object.stable_id == captured.stable_id
                    && !game.is_phased_out(object.id))
                {
                    return host_filter.matches(current, ctx, game)
                        && attachment.matches_target(crate::object::AttachmentTarget::Object(current.id), ctx, game);
                }
                let Some(last_known) = game.source_characteristic_lki_snapshot(captured.object_id, Some(captured)) else { return false; };
                let crate::object::AuraAttachmentFilter::Object(enchant_filter) = attachment else { return false; };
                last_known.zone == Zone::Battlefield
                    && host_filter.matches_snapshot(last_known, ctx, game)
                    && enchant_filter.matches_snapshot(last_known, ctx, game)
            });
        }
    }
    game.battlefield.iter().filter_map(|id| game.object(*id))
        .filter(|host| host_filter.matches(host, ctx, game))
        .any(|host| attachment.matches_target(crate::object::AttachmentTarget::Object(host.id), ctx, game))
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
        if !object_group_filter_matches(self,subject.tail_object_id(),ctx,game){return false}
        if let Some(query)=&self.named_action_recipient_this_turn {
            let actors=match query.1.base(){
                ChooseSpec::Source=>ctx.source.into_iter().collect::<Vec<_>>(),
                ChooseSpec::SpecificObject(id)=>vec![*id],
                ChooseSpec::Tagged(tag)=>ctx.tagged_objects.get(tag).into_iter().flatten().map(|snapshot|snapshot.object_id).collect(),
                _=>return false,
            };
            if !actors.into_iter().any(|actor|game.named_action_affected_this_turn(actor,&query.0,subject.tail_object_id())){return false;}
        }
        if let Some(history)=&self.shares_name_with_named_action_objects {
            let matches=|receipt:&crate::triggers::TriggerEvent| {
                if receipt.completed_action_provenance().is_none(){return false;}
                let Some(action)=receipt.downcast::<crate::events::NamedActionEvent>() else{return false};
                action.label.eq_ignore_ascii_case(history.label.trim())
                    && history.player.matches_player(action.player,ctx)
                    && action.objects.iter().any(|snapshot| {
                        std::iter::once(snapshot.tail_name()).chain(snapshot.tail_alternate_name())
                            .any(|name|subject_has_name_in_game(subject,name,game))
                    })
            };
            let observed=if history.this_turn {
                game.turn_store.turn_history.projected_records().any(|record|matches(&record.event))
            }else{game.retained_action_observations().any(matches)};
            if !observed{return false;}
        }
        if let Some(owner)=&self.name_noted_for {
            let owners=match owner.base(){
                ChooseSpec::Source=>ctx.source.into_iter().collect::<Vec<_>>(),
                ChooseSpec::SpecificObject(id)=>vec![*id],
                ChooseSpec::Tagged(tag)=>ctx.tagged_objects.get(tag).into_iter().flatten().map(|snapshot|snapshot.object_id).collect(),
                _=>Vec::new(),
            };
            if !owners.into_iter().any(|source| {
                // Missing/ambiguous acquisition evidence cannot borrow another link.
                match crate::source_numbers::resolve_owner(game,source,self.noted_name_pair,ctx.source_number_owner.as_ref(),
                    ctx.source_snapshot.as_ref().filter(|snapshot|snapshot.object_id==source)) {
                    Ok(owner)=>game.noted_card_names_for_owner(&owner).iter().any(|name|subject_has_name_in_game(subject,name,game)),
                    Err(error)=>{game.record_token_resource_failure(&error);false},
                }
            }){return false;}
        }
        if let Some(boundary)=&self.library_boundary {
            let Some(object)=game.object(subject.subject_object_id()).filter(|object|object.zone==Zone::Library)else{return false;};
            let Some(library)=game.player(object.owner).map(|player|&player.library)else{return false;};
            let Some(position)=library.iter().position(|id|*id==object.id)else{return false;};
            let Some(count)=resolve_filter_comparison_rhs_value(boundary.count.as_ref(),game,ctx,stack_entry).and_then(|count|usize::try_from(count.max(0)).ok())else{return false;};
            let included=if boundary.from_top{position>=library.len().saturating_sub(count)}else{position<count};
            if !included{return false;}
        }
        if let Some(relation) = &self.related_exile_anchor {
            if !related_exile_anchor_contains(game,ctx,relation,subject.subject_object_id()) { return false; }
        }
        if let Some(relation) = &self.related_exile_members {
            let members = subject.tail_related_exiled_snapshots().map(|collections|
                collections.get(&relation.relationship.trim().to_ascii_lowercase()).cloned().unwrap_or_default())
                .unwrap_or_else(|| game.related_exiled_cards(&relation.relationship,subject.subject_object_id()).into_iter()
                    .filter_map(|id|game.object(id)).map(|object|ObjectSnapshot::from_object(object,game)).collect());
            let count=members.iter().filter(|card|relation.filter.matches_snapshot(card,ctx,game)).count();
            if !relation.count.satisfies_with_context(count.min(i32::MAX as usize) as i32,game,ctx,stack_entry) {return false;}
        }
        // Name check
        if let Some(required_name) = &self.name {
            if required_name == "{chosen name}" {
                let Some(source) = ctx.source else {
                    return false;
                };
                let Some(chosen_name) = game.chosen_named_option(source) else {
                    return false;
                };
                if !subject_has_name_in_game(subject, chosen_name, game) {
                    return false;
                }
            } else if !subject_has_name_in_game(subject, required_name, game) {
                return false;
            }
        }
        if let Some(excluded_name) = &self.excluded_name
            && subject_has_name_in_game(subject, excluded_name, game)
        {
            return false;
        }
        if let Some(required_set_name) = &self.name_originally_printed_in_set
            && !subject_has_name_originally_printed_in_set(subject, required_set_name, game)
        {
            return false;
        }
        // "creatures that don't have a name" (Pompous Gadabout, CR 708.2a).
        if self.nameless && !name_is_nameless(subject.tail_name()) {
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

        // CR 702.11e: an effect that looks for a permanent with hexproof also
        // finds one with a "hexproof from [quality]" ability.
        let has_static_ability_id = |ability_id: StaticAbilityId| {
            if ability_id == StaticAbilityId::Partner {
                return subject.tail_has_partner_family();
            }
            subject.tail_has_static_ability_id(ability_id)
                || (ability_id == StaticAbilityId::Hexproof
                    && subject.tail_has_static_ability_id(StaticAbilityId::HexproofFrom))
                // Megamorph is a variant of morph (CR 702.37b); disguise
                // is a separate ability. Use the current layer-derived
                // abilities, never the hidden face-down restoration state.
                || (ability_id == StaticAbilityId::Morph
                    && subject.tail_has_static_ability_id(StaticAbilityId::Megamorph))
        };

        // Required static ability IDs
        if self
            .static_abilities
            .iter()
            .any(|ability_id| !has_static_ability_id(*ability_id))
        {
            return false;
        }

        // Excluded static ability IDs
        if self
            .excluded_static_abilities
            .iter()
            .any(|ability_id| has_static_ability_id(*ability_id))
        {
            return false;
        }

        if let Some(required) = self.has_cumulative_upkeep {
            if !subject.tail_has_complete_abilities() {
                game.record_token_resource_failure(
                    &crate::effects::ExecutionError::IncompleteEvidence(
                        "cumulative-upkeep predicate has no exact captured ability definition"
                            .into(),
                    ),
                );
                return false;
            }
            let has = subject.tail_abilities().iter().any(|ability| {
                let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
                    return false;
                };
                triggered.effects.all_effects().into_iter().any(|effect| {
                    // Result/tag wrappers retain this instruction's mechanic;
                    // quoted grant definitions are separate abilities.
                    let mut instruction = effect;
                    while let Some(inner) = instruction.transparent_child_effect() {
                        instruction = inner;
                    }
                    instruction
                        .downcast_ref::<crate::effects::CumulativeUpkeepEffect>()
                        .is_some_and(|upkeep| {
                            upkeep.kind == ironsmith_core::effect::UpkeepPaymentKind::Cumulative
                        })
                })
            });
            if has != required {
                return false;
            }
        }

        // Required/excluded ability markers
        if self.ability_markers.iter().any(|marker| {
            !subject.tail_has_ability_marker(marker)
                && !stack_entry
                    .is_some_and(|entry| stack_entry_has_ability_marker(game, entry, marker))
                && !subject.tail_has_granted_casting_keyword(marker, game, ctx)
        }) {
            return false;
        }
        if self.excluded_ability_markers.iter().any(|marker| {
            subject.tail_has_ability_marker(marker)
                || stack_entry
                    .is_some_and(|entry| stack_entry_has_ability_marker(game, entry, marker))
                || subject.tail_has_granted_casting_keyword(marker, game, ctx)
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
        if self.has_non_mana_activated_ability && !subject.tail_has_non_mana_activated_ability() {
            return false;
        }
        if self.has_activated_ability
            && !subject
                .tail_abilities()
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_)))
        {
            return false;
        }
        if let Some(minimum) = self.min_activated_abilities
            && subject
                .tail_abilities()
                .iter()
                .filter(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_)))
                .count()
                < minimum as usize
        {
            return false;
        }
        if !self.could_produce_mana.is_empty()
            && !subject_could_produce_any_mana_symbol(subject, &self.could_produce_mana, game)
        {
            return false;
        }
        if self.no_abilities && (!subject.tail_abilities().is_empty() || subject.tail_has_casting_ability()) {
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
            if let Some(name) = constraint.tag.as_str().strip_prefix("source_collection:") {
                let Some(source) = ctx.source else { return false; };
                let member = game.source_collection_objects(source, name).contains(&subject.subject_object_id())
                    || ctx.departed_battlefield_lookback.as_deref().unwrap_or(&[]).iter()
                        .find(|snapshot|snapshot.object_id==subject.subject_object_id())
                        .is_some_and(|snapshot|game.source_collection_had_member(source,name,snapshot.object_id,snapshot.zone));
                let expected = match constraint.relation {
                    TaggedOpbjectRelation::IsTaggedObject | TaggedOpbjectRelation::SameObjectId => true,
                    TaggedOpbjectRelation::IsNotTaggedObject => false,
                    _ => return false,
                };
                if member != expected { return false; }
                continue;
            }
            if let Some(matches) =
                source_characteristic_constraint_matches_subject(subject, constraint, ctx, game)
            {
                if !matches {
                    return false;
                }
                continue;
            }
            if constraint.relation == TaggedOpbjectRelation::SharesMostCommonPermanentColor {
                if !subject_shares_most_common_permanent_color(subject, game) {
                    return false;
                }
                continue;
            }
            let Some(tagged_snapshots) = ctx.tagged_objects.get(constraint.tag.as_str()) else {
                // "that card" naming the top card of your library (Crown of
                // Convergence, Conspicuous Snoop): read the live library top
                // of the filter's "you" (CR 401.1).
                if constraint.tag.as_str() == crate::tag::TOP_OF_YOUR_LIBRARY_TAG {
                    let top = ctx
                        .you
                        .and_then(|you| game.player(you))
                        .and_then(|player| player.library.last().copied())
                        .and_then(|id| game.object(id))
                        .map(|top| {
                            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                                top, game,
                            )
                        });
                    let snapshots: Vec<crate::snapshot::ObjectSnapshot> = top.into_iter().collect();
                    if !tagged_constraint_matches_subject(
                        subject,
                        &snapshots,
                        constraint.relation,
                        game,
                    ) {
                        return false;
                    }
                    continue;
                }
                // "cards they exiled with this enchantment" (Shared Fate):
                // linked to the filter's source and exiled by its "you". A
                // per-beneficiary permission binds "you" to that player.
                let this_turn = constraint.tag.as_str() == crate::tag::SOURCE_EXILED_BY_YOU_THIS_TURN_TAG;
                if (this_turn || constraint.tag.as_str() == crate::tag::SOURCE_EXILED_BY_YOU_TAG)
                    && matches!(
                        constraint.relation,
                        TaggedOpbjectRelation::IsTaggedObject
                            | TaggedOpbjectRelation::IsNotTaggedObject
                    )
                {
                    let subject_id = subject.subject_object_id();
                    let exiled_by_you = ctx.source.zip(ctx.you).is_some_and(|(source, you)| {
                        if this_turn {
                            game.exiled_with_source_by_player_this_turn(source, subject_id, you)
                        } else {
                            game.exiled_with_source_by_player(source, subject_id, you)
                        }
                    });
                    if exiled_by_you
                        != (constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
                    {
                        return false;
                    }
                    continue;
                }
                // "cards you exiled" (Haldan): exiled cards linked to a source
                // whose controller is the filter's "you".
                if constraint.tag.as_str() == crate::tag::EXILED_BY_YOU_TAG
                    && matches!(
                        constraint.relation,
                        TaggedOpbjectRelation::IsTaggedObject
                            | TaggedOpbjectRelation::IsNotTaggedObject
                    )
                {
                    let subject_id = subject.subject_object_id();
                    let exiled_by_you = ctx.you.is_some_and(|you| {
                        game.exiled_with_source_entries().any(|(source, exiled)| {
                            exiled.contains(&subject_id)
                                && game
                                    .object(*source)
                                    .is_some_and(|source| game.controller_of(source) == you)
                        })
                    });
                    if exiled_by_you
                        != (constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
                    {
                        return false;
                    }
                    continue;
                }
                // "a creature it's paired with" in a trigger or static
                // filter: the source object is always known from the filter
                // context even when no resolution tag has been captured.
                // "that dealt damage to this creature this turn" (Giant
                // Albatross): the recipient is the source, live or by its
                // last-known incarnation after it left the battlefield.
                if constraint.tag.as_str() == crate::tag::SOURCE_OBJECT_TAG
                    && let TaggedOpbjectRelation::DealtDamageToTaggedThisTurn { combat_only } =
                        constraint.relation
                {
                    let recipients = ctx
                        .source
                        .into_iter()
                        .chain(ctx.source_snapshot.as_ref().map(|snapshot| snapshot.object_id))
                        .collect::<Vec<_>>();
                    let dealer = subject.subject_object_id();
                    let dealt = game.turn_store.turn_history.projected_records().any(|record| {
                        let Some(damage) = record.event.downcast::<crate::events::DamageEvent>()
                        else {
                            return false;
                        };
                        damage.amount > 0
                            && (!combat_only || damage.is_combat)
                            && damage.source == dealer
                            && matches!(damage.target, crate::events::DamageTarget::Object(recipient)
                                if recipients.contains(&recipient))
                    });
                    if !dealt {
                        return false;
                    }
                    continue;
                }
                if constraint.tag.as_str() == crate::tag::SOURCE_OBJECT_TAG
                    && constraint.relation == TaggedOpbjectRelation::BandedWithTagged
                {
                    let banded = ctx.source.is_some_and(|source| {
                        objects_are_banded_together(game, source, subject.subject_object_id())
                    });
                    if !banded {
                        return false;
                    }
                    continue;
                }
                if constraint.tag.as_str() == crate::tag::SOURCE_OBJECT_TAG
                    && constraint.relation == TaggedOpbjectRelation::SoulbondPartnerOfTagged
                {
                    let paired = ctx.source.is_some_and(|source| {
                        game.soulbond_partner(source) == Some(subject.subject_object_id())
                    });
                    if !paired {
                        return false;
                    }
                    continue;
                }
                // An Aura's or Equipment's own "enchanted/equipped creature"
                // before the resolution tag is bound (e.g. while choosing
                // targets for Kjeldoran Pride's "target creature other than
                // enchanted creature") is the permanent the source is
                // attached to, not any permanent with an Aura/Equipment.
                if let Some(host) = source_attachment_host_for_tag(
                    ctx.source,
                    &constraint.tag,
                    game,
                )
                .or_else(|| {
                    let source = ctx.source_snapshot.as_ref()?;
                    let departed = ctx.source.is_some_and(|id| {
                        game.object(id).is_none_or(|live| {
                            live.zone != source.zone || live.stable_id != source.stable_id
                        })
                    });
                    if !departed
                        || !matches!(constraint.tag.as_str(), "enchanted" | "equipped" | "fortified")
                    {
                        return None;
                    }
                    match source.attached_to {
                        Some(crate::object::AttachmentTarget::Object(host)) => Some(host),
                        _ => None,
                    }
                }) && matches!(
                    constraint.relation,
                    TaggedOpbjectRelation::IsTaggedObject
                        | TaggedOpbjectRelation::IsNotTaggedObject
                ) {
                    let is_host = subject.subject_object_id() == host;
                    if is_host != (constraint.relation == TaggedOpbjectRelation::IsTaggedObject) {
                        return false;
                    }
                    continue;
                }
                // A source-relative attachment reference has no subject when
                // that Aura or Equipment is unattached.
                if matches!(constraint.tag.as_str(), "enchanted" | "equipped" | "fortified")
                    && ctx
                        .source
                        .and_then(|source| game.object(source))
                        .is_some_and(|source| {
                            source.subtypes.contains(&crate::types::Subtype::Aura)
                                || source.subtypes.contains(&crate::types::Subtype::Equipment)
                                || source
                                    .subtypes
                                    .contains(&crate::types::Subtype::Fortification)
                        })
                    && matches!(
                        constraint.relation,
                        TaggedOpbjectRelation::IsTaggedObject
                            | TaggedOpbjectRelation::IsNotTaggedObject
                    )
                {
                    if constraint.relation == TaggedOpbjectRelation::IsTaggedObject {
                        return false;
                    }
                    continue;
                }
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
            // This canonical tag is a persistent source choice, not a local
            // move-and-follow tag. A new zone incarnation is a different object
            // even though it retains the card's stable id (CR 400.7).
            if constraint.tag.as_str() == crate::tag::CHOSEN_OBJECTS_TAG
                && matches!(
                    constraint.relation,
                    TaggedOpbjectRelation::IsTaggedObject
                        | TaggedOpbjectRelation::IsNotTaggedObject
                )
            {
                let is_chosen = tagged_snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == subject.subject_object_id());
                if is_chosen != (constraint.relation == TaggedOpbjectRelation::IsTaggedObject) {
                    return false;
                }
                continue;
            }
            // "a name chosen for this enchantment" (Day of the Moon) is every
            // name chosen for the source so far, while the resolution tag holds
            // only this resolution's choice (CR 607.2, 700.2).
            if constraint.relation == TaggedOpbjectRelation::SameNameAsTagged
                && self.chosen_name_source_surface().is_some()
                && let Some(source) = ctx.source
                && let Some(names) = game.chosen_named_option(source)
                && names.lines().map(str::trim).any(|name| {
                    !name.is_empty() && name.eq_ignore_ascii_case(subject.subject_name())
                })
            {
                continue;
            }
            // A reference to the departing permanent uses its battlefield names.
            // A reference to the card in its new zone uses its new characteristics.
            if constraint.relation == TaggedOpbjectRelation::SameNameAsTagged
                && self.same_name_antecedent_surface() != Some(SameNameAntecedentSurface::Card)
                && let Some(history) = ctx.tagged_objects.get(&ironsmith_core::tag::pre_move_history_tag(&constraint.tag))
            {
                let matches = tagged_snapshots.iter().any(|result| {
                    let before = history.iter()
                        .find(|old| old.stable_id == result.stable_id && old.zone == Zone::Battlefield)
                        .unwrap_or(result);
                    ObjectNames::snapshot(before).shares(subject.subject_names(game), game)
                });
                if !matches {
                    return false;
                }
            } else if !tagged_constraint_matches_subject(
                subject,
                tagged_snapshots,
                constraint.relation,
                game,
            ) {
                return false;
            }
            // "each other creature that shares a creature type with it"
            // (Coat of Arms): "other" is relative to the compared object, so
            // that object never counts as sharing with itself.
            if self.other
                && matches!(
                    constraint.relation,
                    TaggedOpbjectRelation::SameNameAsTagged
                        | TaggedOpbjectRelation::SharesCardType
                        | TaggedOpbjectRelation::SharesPermanentType
                        | TaggedOpbjectRelation::SharesColorWithTagged
                        | TaggedOpbjectRelation::SharesSubtypeWithTagged
                        | TaggedOpbjectRelation::SharesSubtypeWithEachTagged
                        | TaggedOpbjectRelation::SharesProducibleManaTypeWithTagged
                )
                && tagged_snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == subject.subject_object_id())
            {
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

        if let Some(host_filter) = &self.could_enchant_object {
            // A concrete host uses current characteristics while present,
            // then its last known battlefield characteristics after departure.
            let could_enchant = game
                .object(subject.subject_object_id())
                .and_then(|aura| aura.aura_attach_filter_owned())
                .is_some_and(|attach| aura_could_enchant_host_filter(&attach, host_filter, ctx, game));
            if !could_enchant {
                return false;
            }
        }

        if let Some(with_attached_filter) = &self.with_attached_object {
            let has_matching_attachment =
                if let Some(snapshots) = subject.tail_attachment_snapshots() {
                    snapshots.iter().any(|attachment| {
                        with_attached_filter.matches_snapshot(attachment, ctx, game)
                    })
                } else {
                    subject.subject_attachments().iter().any(|&id| {
                        game.object(id).is_some_and(|attachment| {
                            with_attached_filter.matches(attachment, ctx, game)
                        })
                    })
                };
            if !has_matching_attachment {
                return false;
            }
        }

        if let Some(controlled) = &self.controller_controls
            && controlled_matching_object_count(game, subject.subject_controller(), controlled, ctx)
                == 0
        {
            return false;
        }

        if let Some(without_attached_filter) = &self.without_attached_object {
            let has_forbidden_attachment =
                if let Some(snapshots) = subject.tail_attachment_snapshots() {
                    if subject
                        .subject_attachments()
                        .iter()
                        .any(|id| !snapshots.iter().any(|snapshot| snapshot.object_id == *id))
                    {
                        // A raw/legacy snapshot names an attachment but has no
                        // historical characteristics for it. Unknown does not prove absence.
                        return false;
                    }
                    snapshots.iter().any(|attachment| {
                        without_attached_filter.matches_snapshot(attachment, ctx, game)
                    })
                } else {
                    subject.subject_attachments().iter().any(|&id| {
                        game.object(id).is_some_and(|attachment| {
                            without_attached_filter.matches(attachment, ctx, game)
                        })
                    })
                };
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

        // CR 608.2: a resolving spell is still on the stack, but its entry was
        // popped for resolution; its targets are read from the cast record.
        let resolving_spell_entry = || {
            game.object(object_id)
                .filter(|object| object.zone == Zone::Stack)
                .and_then(|_| game.turn_store.cast_spell_lki.get(&object_id))
                .map(|lki| &lki.1)
        };
        // Targeting checks (spell/ability targets on the stack)
        if self.targets_player.is_some() || self.targets_object.is_some() {
            let Some(entry) = stack_entry
                .or_else(|| game.stack.iter().find(|e| e.object_id == object_id))
                .or_else(resolving_spell_entry)
            else {
                return false;
            };

            // CR 115.9b: a targeted player who left the game is ignored.
            let matches_player = self.targets_player.as_ref().is_none_or(|player_filter| {
                entry.targets.iter().any(|target| match target {
                    crate::game_state::Target::Player(pid) => {
                        game.player(*pid).is_some_and(|player| player.is_in_game())
                            && player_filter.matches_player(*pid, ctx)
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
            // A stack-object snapshot ("if it has a single target", Grip of
            // Chaos) names an ability by its own stack id, not its source's.
            let Some(entry) = stack_entry
                .or_else(|| game.stack.iter().find(|e| e.ability_id == Some(object_id)))
                .or_else(|| game.stack.iter().find(|e| e.object_id == object_id))
                .or_else(resolving_spell_entry)
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
                // CR 115.9c: "targets only" requires exactly one different
                // object or player among the chosen targets.
                if entry.targets.iter().any(|target| target != &entry.targets[0]) {
                    return false;
                }

                let matches_target = |target: &crate::game_state::Target| -> bool {
                    let matches_player = self.targets_only_player.as_ref().is_some_and(
                        |player_filter| match target {
                            crate::game_state::Target::Player(pid) => {
                                game.player(*pid).is_some_and(|player| player.is_in_game())
                                    && player_filter.matches_player(*pid, ctx)
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
        if self.zone.is_some() && game.is_originless_card_entry_pending(object.id) { return false; }
        if self.castable_face_match {
            let mut filter = self.clone();
            filter.castable_face_match = false;
            if filter.matches_internal(object, ctx, game, allow_calculated_pt, view) {
                return true;
            }
            // Outside the stack a split card has both halves' characteristics
            // (CR 709.4); each castable face is evaluated alone (709.3a,
            // 715.3, 712.12).
            let matches_face = |face: &Object| {
                let snapshot = crate::snapshot::ObjectSnapshot::from_object(face, game);
                filter.matches_snapshot(&snapshot, ctx, game)
            };
            return (object.linked_face_layout == crate::card::LinkedFaceLayout::Split
                && matches_face(object.projected_front_half().as_ref().unwrap_or(object)))
                || crate::decision::spell_view_for_split_other_half_cast(game, object)
                    .as_ref().is_some_and(matches_face);
        }
        if self.use_face_up_characteristics && object.face_down_cast_state.is_some() {
            let mut face_up = object.clone();
            face_up.end_face_down_cast_overlay();
            let snapshot = crate::snapshot::ObjectSnapshot::from_object(&face_up, game);
            let mut filter = self.clone();
            filter.use_face_up_characteristics = false;
            filter.match_current_state = false;
            return filter.matches_snapshot(&snapshot, ctx, game);
        }
        matching::matches_subject(
            self,
            ObjectSubject::Live(object),
            ctx,
            game,
            allow_calculated_pt,
            view,
        )
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
        if self.zone.is_some() && game.is_originless_card_entry_pending(snapshot.object_id) { return false; }
        if self.match_current_state {
            let Some(current) = game.object(snapshot.object_id) else {
                return false;
            };
            let mut live_filter = self.clone();
            live_filter.match_current_state = false;
            return live_filter.matches(current, ctx, game);
        }
        if ctx
            .you
            .is_some_and(|observer| !game.snapshot_is_within_range(observer, snapshot, ctx.source))
        {
            return false;
        }
        matching::matches_subject(
            self,
            ObjectSubject::Snapshot(snapshot),
            ctx,
            game,
            false,
            None,
        )
    }
}

#[cfg(test)]
mod permanent_spell_description_tests {
    use super::*;

    #[test]
    fn complete_permanent_spell_type_set_compacts_to_permanent_spell() {
        let filter = ObjectFilter::from(ironsmith_core::filter_model::ObjectFilterFields {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::Spell),
            card_types: vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Planeswalker,
                CardType::Battle,
            ],
            ..ironsmith_core::filter_model::ObjectFilterFields::from(ObjectFilter::default())
        });

        assert_eq!(filter.description(), "permanent spell");
    }
}

#[cfg(test)]
mod fewest_controller_set_tests {
    use super::*;
    #[test]
    fn fewest_set_includes_zero_and_all_ties_and_tracks_effective_control_and_departed_players() {
        let mut game =
            crate::GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let [a, b, c] = [
            crate::PlayerId::from_index(0),
            crate::PlayerId::from_index(1),
            crate::PlayerId::from_index(2),
        ];
        let card =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counted creature")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(1, 4))
                .build();
        let source = game.create_object_from_definition(&card, a, crate::Zone::Battlefield);
        let bob = game.create_object_from_definition(&card, b, crate::Zone::Battlefield);
        for _ in 0..2 {
            game.create_object_from_definition(&card, c, crate::Zone::Battlefield);
        }
        let filter = PlayerFilter::ControlsFewestTied {
            filter: Box::new(ObjectFilter::creature()),
        };
        let selected = |game: &crate::GameState| {
            let ctx = game.filter_context_for(a, Some(source));
            game.players
                .iter()
                .filter(|player| player_filter_matches_game(&filter, player.id, game, &ctx))
                .map(|player| player.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(selected(&game), vec![a, b]);
        let mut ctx = crate::effects::ExecutionContext::new_default(source, a);
        let effect = crate::effect::Effect::new(crate::effects::GainControlEffect::new(
            crate::target::ChooseSpec::SpecificObject(bob),
            crate::effect::Until::EndOfTurn,
        ));
        crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(selected(&game), vec![b]);
        let mut restricted = game.filter_context_for(a, Some(source));
        restricted.players_in_range = Some(vec![a, c]);
        // The out-of-range zero must not suppress both in-range tied players.
        assert!(player_filter_matches_game(&filter, a, &game, &restricted));
        assert!(player_filter_matches_game(&filter, c, &game, &restricted));
        assert!(!player_filter_matches_game(&filter, b, &game, &restricted));
        let filter_ctx = ctx.filter_context(&game);
        assert_eq!(
            crate::effects::helpers::resolve_player_filter_to_list(
                &game,
                &filter,
                &filter_ctx,
                &ctx
            )
            .unwrap(),
            vec![b]
        );
        assert!(
            game.leave_game(b)
                .expect("checked designation/departure fixture")
        );
        assert_eq!(selected(&game), vec![a]);
        assert!(
            game.leave_game(c)
                .expect("checked designation/departure fixture")
        );
        assert_eq!(selected(&game), vec![a]);
        assert!(
            game.leave_game(a)
                .expect("checked designation/departure fixture")
        );
        assert!(selected(&game).is_empty());
    }
}

#[cfg(test)]
#[path = "filter/shared_card_type_reference_tests.rs"]
mod shared_card_type_reference_tests;

#[cfg(test)]
mod player_to_left_of_tests {
    use super::*;

    #[test]
    fn player_to_their_left_follows_the_iterated_player_seat() {
        // "the player to their left" (CR 101.4a seating) is relative to the
        // player the enclosing per-player process is for.
        let game = crate::GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let players = [
            crate::PlayerId::from_index(0),
            crate::PlayerId::from_index(1),
            crate::PlayerId::from_index(2),
        ];
        let filter = PlayerFilter::PlayerToLeftOf(Box::new(PlayerFilter::IteratedPlayer));
        for iterated in players {
            let ctx = FilterContext::new(players[0]).with_iterated_player(Some(iterated));
            let expected = game.closest_in_game_player_to_left_matching(iterated, |_| true);
            let matching = players
                .iter()
                .copied()
                .filter(|player| player_filter_matches_game(&filter, *player, &game, &ctx))
                .collect::<Vec<_>>();
            assert_eq!(matching, expected.into_iter().collect::<Vec<_>>());
            assert!(!matching.contains(&iterated));
        }
    }
}

#[cfg(test)]
mod owned_zone_participant_tests {
    use super::*;
    #[test]
    fn personal_zone_existence_uses_owners_and_complete_object_filter() {
        let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let alice = crate::PlayerId(0);
        let bob = crate::PlayerId(1);
        let cara = crate::PlayerId(2);
        let card = crate::card::CardBuilder::new(crate::CardId::new(), "Graveyard instant")
            .card_types(vec![crate::CardType::Instant]).build();
        for _ in 0..2 { game.create_object_from_card(&card, bob, crate::Zone::Graveyard); }
        game.create_object_from_card(&card, cara, crate::Zone::Hand);
        let filter = PlayerFilter::OwnsMatchingObject {
            filter: Box::new(ObjectFilter::default().with_type(crate::CardType::Instant).in_zone(crate::Zone::Graveyard)),
        };
        let selected = |game: &crate::GameState| {
            let ctx = game.filter_context_for(alice, None);
            game.players.iter().filter(|player| player_filter_matches_game(&filter, player.id, game, &ctx))
                .map(|player| player.id).collect::<Vec<_>>()
        };
        assert_eq!(selected(&game), vec![bob]);
        game.create_object_from_card(&card, cara, crate::Zone::Graveyard);
        assert_eq!(selected(&game), vec![bob, cara]);
    }
}

/// Structural keyword test for a frozen copy envelope.
pub(crate) fn copy_abilities_have_marker(abilities: &[crate::ability::Ability], marker: &str) -> bool { descriptions::abilities_have_marker(abilities, marker) }

/// Associations use exact historical object identities for tagged/source references.
fn object_group_filter_matches(filter:&ObjectFilter,subject:ObjectId,ctx:&FilterContext,game:&GameState)->bool{
 fn references(spec:&crate::target::ChooseSpec,ctx:&FilterContext,game:&GameState)->Vec<ObjectId>{
  match spec.base(){
   crate::target::ChooseSpec::Source=>ctx.source.or_else(||ctx.source_snapshot.as_ref().map(|snapshot|snapshot.object_id)).into_iter().collect(),
   crate::target::ChooseSpec::SpecificObject(id)=>vec![*id],
   crate::target::ChooseSpec::Tagged(tag)=>ctx.tagged_objects.get(tag).map(|objects|objects.iter().map(|snapshot|snapshot.object_id).collect()).unwrap_or_default(),
   crate::target::ChooseSpec::Object(reference)|crate::target::ChooseSpec::All(reference)=>{
    if reference.source{return ctx.source.or_else(||ctx.source_snapshot.as_ref().map(|snapshot|snapshot.object_id)).into_iter().collect()}
    if let Some(id)=reference.specific{return vec![id]}
    if spec.is_target(){return ctx.target_objects.iter().filter(|snapshot|reference.matches_snapshot(snapshot,ctx,game)).map(|snapshot|snapshot.object_id).collect()}
    let mut found=ctx.tagged_objects.values().flatten().filter(|snapshot|reference.matches_snapshot(snapshot,ctx,game)).map(|snapshot|snapshot.object_id).collect::<Vec<_>>();
    let candidates=if reference.zone.is_some(){game.object_ids_in_deterministic_order()}else{game.battlefield.iter().copied().collect()};
    found.extend(candidates.into_iter().filter(|id|game.object(*id).is_some_and(|object|reference.matches(object,ctx,game))));found.sort();found.dedup();found
   },
   _=>Vec::new(),
  }
 }
 if filter.group_member_of.as_ref().is_some_and(|reference|!references(reference,ctx,game).iter().any(|owner|game.object_group_members_owned_by(*owner).contains(&subject))){return false}
 if filter.same_group_as.as_ref().is_some_and(|reference|!references(reference,ctx,game).iter().any(|owner|game.object_group_members_with(*owner).contains(&subject))){return false}
 true
}

/// Pure host reference resolution for persistent object associations. The
/// relationship filter cannot introduce a new choice or a target.
pub(crate) fn related_exile_host_ids(game:&GameState,ctx:&FilterContext,spec:&ChooseSpec)->Vec<ObjectId>{
    match spec.base(){
        ChooseSpec::Source=>ctx.source.into_iter().collect(),
        ChooseSpec::SpecificObject(id)=>vec![*id],
        ChooseSpec::Tagged(tag)=>ctx.tagged_objects.get(tag).into_iter().flatten().map(|object|object.object_id).collect(),
        ChooseSpec::All(filter)|ChooseSpec::Object(filter)=>game.objects_in_deterministic_order().into_iter()
            .filter(|object|filter.matches_non_recursive(object,ctx,game)).map(|object|object.id).collect(),
        _=>Vec::new(),
    }
}
pub(crate) fn related_exile_anchor_contains(game:&GameState,ctx:&FilterContext,relation:&ironsmith_core::filter_model::RelatedExileAnchor,member:ObjectId)->bool{
    related_exile_host_ids(game,ctx,&relation.host).into_iter().any(|host|{
        if game.object(host).is_some_and(|object|object.zone==Zone::Battlefield){
            return game.has_related_exiled_card(&relation.relationship,host,member);
        }
        ctx.source_snapshot.iter().chain(ctx.tagged_objects.values().flatten()).filter(|snapshot|snapshot.object_id==host)
            .any(|snapshot|snapshot.related_exiled_cards.get(&relation.relationship.trim().to_ascii_lowercase()).is_some_and(|cards|cards.iter().any(|card|card.object_id==member)))
    })
}
#[cfg(test)]
mod unrepresented_subtype_relation_tests {
    use super::*;
    #[test]
    fn subtype_set_difference_allows_partial_overlap_and_uses_only_controlled_creatures() {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let a=PlayerId(0);let b=PlayerId(1);
        let human=crate::CardDefinitionBuilder::new(crate::CardId::new(),"Represented type").card_types(vec![crate::CardType::Creature]).subtypes(vec![crate::Subtype::Human]).build();
        let mixed=crate::CardDefinitionBuilder::new(crate::CardId::new(),"Partial overlap").card_types(vec![crate::CardType::Creature]).subtypes(vec![crate::Subtype::Human,crate::Subtype::Elf]).build();
        let own=game.create_object_from_definition(&human,a,Zone::Battlefield);
        let candidate=game.create_object_from_definition(&mixed,a,Zone::Library);
        // `creature()` is a battlefield-permanent filter. This unit probes a
        // library card, just as the permission reader does after normalizing
        // its outer candidate zone; retain the comparison's battlefield zone.
        assert!(!ObjectFilter::creature().matches(game.object(candidate).unwrap(),&FilterContext::new(a),&game));
        let mut filter=ObjectFilter::creature().in_zone(Zone::Library);
        filter.characteristic_relations.push(ObjectCharacteristicRelation{kind:ObjectCharacteristicRelationKind::HasUnrepresentedSubtype,characteristics:vec![ObjectCharacteristic::Subtype(ironsmith_core::types::SubtypeFamily::Creature)],comparison:ObjectFilter::creature().you_control().in_zone(Zone::Battlefield),exclude_candidate:false});
        let ctx=FilterContext::new(a);
        assert!(filter.matches(game.object(candidate).unwrap(),&ctx,&game),"Elf is absent despite existing Human overlap");
        game.create_object_from_definition(&mixed,b,Zone::Battlefield);
        assert!(filter.matches(game.object(candidate).unwrap(),&ctx,&game),"opponent types do not count");
        game.create_object_from_definition(&mixed,a,Zone::Battlefield);
        assert!(!filter.matches(game.object(candidate).unwrap(),&ctx,&game),"union covers both candidate types");
        assert!(game.object(own).is_some());
    }
}


/// Source-local successful entry exile, retaining the original ObjectId. The
/// historical receipt stays available separately for event-time predicates.
pub(crate) fn entry_exiled_reference_snapshots(
    game: &GameState, source: ObjectId, source_snapshot: Option<&ObjectSnapshot>, receipt_tag: &str,
) -> Vec<ObjectSnapshot> {
    let receipt=source_entry_exiled_receipt(game,source,source_snapshot,receipt_tag);
    receipt.into_iter().flatten().filter_map(|snapshot| {
        game.object(snapshot.object_id).filter(|object|
            object.zone == Zone::Exile && object.stable_id == snapshot.stable_id
            && game.find_object_by_stable_id(snapshot.stable_id) == Some(snapshot.object_id))
            .and_then(|object| ObjectSnapshot::capture_for_execution(object, game))
    }).collect()
}

pub(crate) fn entry_exiled_reference_domains(
    game:&GameState,source:ObjectId,source_snapshot:Option<&ObjectSnapshot>,
)->Vec<(TagKey,Vec<ObjectSnapshot>)> {
    let retained=source_snapshot.filter(|snapshot|snapshot.object_id==source);
    let mut receipts=std::collections::BTreeSet::from(["__entry_action_exiled__".to_string()]);
    for map in [retained.map(|snapshot|&snapshot.entry_action_objects),
        game.object(source).map(|object|&object.cast_tagged_objects),
        game.source_characteristic_lki_snapshot(source,retained).map(|snapshot|&snapshot.entry_action_objects)]
        .into_iter().flatten()
    {
        receipts.extend(map.keys().filter(|tag|tag.as_str().starts_with("__entry_action_exiled_by__:"))
            .map(ToString::to_string));
    }
    receipts.into_iter().map(|receipt| {
        let reference=receipt.strip_prefix("__entry_action_exiled_by__:")
            .map(|label|format!("__entry_reference_exiled_by__:{label}"))
            .unwrap_or_else(||"__entry_reference_exiled__".to_string());
        (reference.into(),entry_exiled_reference_snapshots(game,source,source_snapshot,&receipt))
    }).collect()
}

/// An entry receipt has an exact producer incarnation even when its ability is
/// resolving after that source departed. Anonymous context tags are not owners.
pub(crate) fn source_entry_exiled_receipt<'a>(
    game:&'a GameState,source:ObjectId,source_snapshot:Option<&'a ObjectSnapshot>,receipt_tag:&str,
)->Option<&'a Vec<ObjectSnapshot>> {
    let retained=source_snapshot.filter(|snapshot|snapshot.object_id==source);
    retained.and_then(|snapshot|snapshot.entry_action_objects.get(receipt_tag))
        .or_else(||game.object(source).and_then(|object|object.cast_tagged_objects.get(receipt_tag)))
        .or_else(||game.source_characteristic_lki_snapshot(source,retained)
            .and_then(|snapshot|snapshot.entry_action_objects.get(receipt_tag)))
}

/// Exact current contributor policy shared by selection and actual payment.
/// A generic named tap payment never bypasses a source's authored prohibition.
pub(crate) fn admits_named_tap_payment(game: &GameState, object: ObjectId, name: &str) -> bool {
    fn prohibits(game: &GameState, object: ObjectId, model: &crate::static_abilities::CompiledStaticAbility, name: &str) -> bool {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::GenericManaTapPaymentProhibition { name: denied } => denied.eq_ignore_ascii_case(name),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition, .. }
                if game.current_controller(object).is_some_and(|controller| crate::condition_eval::evaluate_condition_cast_time(game, condition, controller, object)) => prohibits(game, object, ability, name),
            _ => false,
        }
    }
    game.object(object).is_some() && !game.current_abilities(object).unwrap_or_default().iter().any(|ability|
        matches!(&ability.kind, crate::ability::AbilityKind::Static(rule)
            if rule.compiled_model().is_some_and(|model| prohibits(game, object, model, name))))
}
