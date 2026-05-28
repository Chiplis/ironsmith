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

use crate::color::ColorSet;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object::{CounterType, Object, ObjectKind};
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbilityId;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
pub use ironsmith_core::filter_model::{
    AlternativeCastKind, Comparison, CounterConstraint, ObjectFilter, ObjectRef, ParityRequirement,
    PlayerFilter, PtReference, SourcePowerRelation, StackObjectKind, TaggedObjectConstraint,
    TaggedOpbjectRelation, TargetabilityConstraint,
};

fn normalize_name_for_match(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn names_match(lhs: &str, rhs: &str) -> bool {
    lhs.eq_ignore_ascii_case(rhs) || normalize_name_for_match(lhs) == normalize_name_for_match(rhs)
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

fn expand_semantic_subtypes(chars: &mut crate::continuous::CalculatedCharacteristics) {
    let has_changeling = chars
        .static_abilities
        .iter()
        .any(|ability| ability.id() == StaticAbilityId::Changeling);
    let can_have_creature_subtypes = chars
        .card_types
        .iter()
        .any(|card_type| matches!(card_type, CardType::Creature | CardType::Kindred));
    if has_changeling && can_have_creature_subtypes {
        for subtype in Subtype::all_creature_types() {
            if !chars.subtypes.contains(subtype) {
                chars.subtypes.push(*subtype);
            }
        }
    }
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
        self.mana_cost
            .as_ref()
            .map_or(0, |mana_cost| mana_cost.mana_value() as i32)
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
        self.mana_cost
            .as_ref()
            .map_or(0, |mana_cost| mana_cost.mana_value() as i32)
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

fn snapshot_matches_subtype(snapshot: &ObjectSnapshot, subtype: Subtype, game: &GameState) -> bool {
    snapshot.subtypes.contains(&subtype)
        || (subtype == Subtype::Adventure
            && linked_face_has_adventure(
                game,
                snapshot.other_face_name.as_deref(),
                snapshot.other_face,
            ))
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
        TaggedOpbjectRelation::IsTaggedObject => tagged_snapshots
            .iter()
            .any(|snapshot| snapshot.object_id == subject.subject_object_id()),
        TaggedOpbjectRelation::SharesCardType => {
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
        TaggedOpbjectRelation::SharesColorWithTagged => tagged_snapshots.iter().any(|snapshot| {
            !subject
                .subject_colors()
                .intersection(snapshot.colors)
                .is_empty()
        }),
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
            snapshot
                .mana_cost
                .as_ref()
                .map_or(0, |mana_cost| mana_cost.mana_value() as i32)
                == subject.subject_mana_value()
        }),
        TaggedOpbjectRelation::ManaValueLteTagged => tagged_snapshots.iter().any(|snapshot| {
            subject.subject_mana_value()
                <= snapshot
                    .mana_cost
                    .as_ref()
                    .map_or(0, |mana_cost| mana_cost.mana_value() as i32)
        }),
        TaggedOpbjectRelation::ManaValueLtTagged => tagged_snapshots.iter().any(|snapshot| {
            subject.subject_mana_value()
                < snapshot
                    .mana_cost
                    .as_ref()
                    .map_or(0, |mana_cost| mana_cost.mana_value() as i32)
        }),
        TaggedOpbjectRelation::AttachedToTaggedObject => tagged_snapshots
            .iter()
            .any(|snapshot| subject.subject_attached_to() == Some(snapshot.object_id)),
        TaggedOpbjectRelation::SoulbondPartnerOfTagged => tagged_snapshots.iter().any(|snapshot| {
            game.soulbond_partner(snapshot.object_id) == Some(subject.subject_object_id())
        }),
        TaggedOpbjectRelation::IsNotTaggedObject => tagged_snapshots
            .iter()
            .all(|snapshot| snapshot.object_id != subject.subject_object_id()),
    }
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

    /// The player casting the spell currently being evaluated, if any.
    pub caster: Option<PlayerId>,

    /// The active player (whose turn it is)
    pub active_player: Option<PlayerId>,

    /// Players who are opponents of "you"
    pub opponents: Vec<PlayerId>,

    /// Players who are teammates of "you" (for team games)
    pub teammates: Vec<PlayerId>,

    /// The defending player (in combat)
    pub defending_player: Option<PlayerId>,

    /// The attacking player (in combat)
    pub attacking_player: Option<PlayerId>,

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

    match rhs {
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
                        .unwrap_or_else(|| object.subtypes.clone());
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
                        .unwrap_or_else(|| object.card_types.clone());
                    for card_type in card_types {
                        seen.insert(card_type);
                    }
                }
            }
            Some(seen.len() as i32)
        }
        Value::DistinctPowers(filter) => {
            let mut seen = std::collections::HashSet::new();
            for object in game.objects_in_deterministic_order() {
                if filter.matches(object, ctx, game) {
                    if let Some(power) = game.calculated_power(object.id).or_else(|| object.power())
                    {
                        seen.insert(power);
                    }
                }
            }
            Some(seen.len() as i32)
        }
        Value::CountersOnSource(counter_type) => {
            let source = game.object(ctx.source?)?;
            Some(source.counters.get(counter_type).copied().unwrap_or(0) as i32)
        }
        Value::SourcePower => current_object_pt(game, ctx.source?, true),
        Value::SourceToughness => current_object_pt(game, ctx.source?, false),
        Value::PowerOf(spec) => resolve_pt_choose_spec(spec, game, ctx, true),
        Value::ToughnessOf(spec) => resolve_pt_choose_spec(spec, game, ctx, false),
        Value::CountersOn(spec, counter_type) => match spec.as_ref() {
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
        Value::ManaValueOf(spec) => match spec.as_ref() {
            ChooseSpec::Source => {
                let source = game.object(ctx.source?)?;
                Some(
                    source
                        .mana_cost
                        .as_ref()
                        .map_or(0, |cost| cost.mana_value() as i32),
                )
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

fn effects_for_stack_entry(
    game: &crate::game_state::GameState,
    entry: &crate::game_state::StackEntry,
) -> Vec<crate::effect::Effect> {
    if let Some(ref effects) = entry.ability_effects {
        return effects.to_vec();
    }

    game.object(entry.object_id)
        .and_then(|object| object.spell_effect.clone())
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
        match self {
            PlayerFilter::Any => true,

            PlayerFilter::You => ctx.you.is_some_and(|you| player == you),

            PlayerFilter::NotYou => ctx.you.map_or(true, |you| player != you),

            PlayerFilter::Opponent => ctx.opponents.contains(&player),

            PlayerFilter::Teammate => ctx.teammates.contains(&player),

            PlayerFilter::Active => ctx.active_player.is_some_and(|ap| player == ap),

            PlayerFilter::Defending => ctx.defending_player.is_some_and(|dp| player == dp),

            PlayerFilter::Attacking => ctx.attacking_player.is_some_and(|ap| player == ap),

            // Resolved from the triggering event during effect execution.
            PlayerFilter::DamagedPlayer => false,

            PlayerFilter::EffectController => false,

            PlayerFilter::Specific(id) => player == *id,
            PlayerFilter::MostLifeTied => false,
            PlayerFilter::LowestLifeTied => false,
            PlayerFilter::MostCardsInHand => false,
            PlayerFilter::CastCardTypeThisTurn(_) => false,
            PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. } => {
                base.matches_player(player, ctx)
            }
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
                if !ctx.target_players.is_empty() {
                    return ctx.target_players.contains(&player)
                        && inner.matches_player(player, ctx);
                }
                ctx.iterated_player.is_some_and(|p| p == player)
                    && inner.matches_player(player, ctx)
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
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            player_filter_matches_game(base, player, game, ctx)
                && game.has_max_speed(player) == *has_max_speed
        }
        PlayerFilter::Target(inner) => {
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
        self.matches_internal(object, ctx, game, true, None)
    }

    fn matches_with_view(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        view: &crate::derived_view::DerivedGameView<'_>,
    ) -> bool {
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

        if self.has_tap_activated_ability && !subject.tail_has_tap_activated_ability() {
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

        if let Some(player_filter) = &self.attached_to_player {
            let Some(attached_player) = subject.subject_attached_to_player() else {
                return false;
            };
            if !player_filter.matches_player(attached_player, ctx) {
                return false;
            }
        }

        let object_id = subject.tail_object_id();

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

        if self.entered_graveyard_this_turn
            && (object.zone != Zone::Graveyard
                || !game
                    .turn_store
                    .turn_history
                    .object_was_put_into_graveyard_this_turn(object.stable_id))
        {
            return false;
        }

        if self.was_dealt_damage_this_turn && !game.creature_was_damaged_this_turn(object.id) {
            return false;
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
                if game
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
                let cast_from_zone = match &entry.casting_method {
                    crate::alternative_cast::CastingMethod::Normal => Zone::Hand,
                    crate::alternative_cast::CastingMethod::FaceDown => Zone::Hand,
                    crate::alternative_cast::CastingMethod::SplitOtherHalf
                    | crate::alternative_cast::CastingMethod::Fuse => Zone::Hand,
                    crate::alternative_cast::CastingMethod::Alternative(index) => object
                        .alternative_casts
                        .get(*index)
                        .map(|method| method.cast_from_zone())
                        .unwrap_or(Zone::Hand),
                    crate::alternative_cast::CastingMethod::GrantedEscape { .. }
                    | crate::alternative_cast::CastingMethod::GrantedFlashback => Zone::Graveyard,
                    crate::alternative_cast::CastingMethod::PlayFrom { zone, .. } => *zone,
                    crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom {
                        zone, ..
                    } => *zone,
                };
                if cast_from_zone != *zone {
                    return false;
                }
            } else if object.zone != *zone {
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

        let mut adjusted_object_storage = None;
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
        if should_calculate_chars
            && let Some(mut chars) = view
                .and_then(|view| {
                    if object.zone != Zone::Battlefield
                        || needs_pt
                        || view.requires_battlefield_characteristic_calculation(object.id)
                    {
                        view.calculated_characteristics(object.id)
                    } else {
                        None
                    }
                })
                .or_else(|| game.current_characteristics(object.id))
        {
            if object.zone != Zone::Battlefield {
                expand_semantic_subtypes(&mut chars);
            }
            let mut adjusted = object.clone();
            adjusted.name = chars.name;
            adjusted.card_types = chars.card_types;
            adjusted.subtypes = chars.subtypes;
            adjusted.supertypes = chars.supertypes;
            adjusted.color_override = Some(chars.colors);
            adjusted.abilities = chars.abilities;
            adjusted_object_storage = Some(adjusted);
        }
        let object = adjusted_object_storage.as_ref().unwrap_or(object);

        if self.modified {
            if object.zone != Zone::Battlefield || !object.card_types.contains(&CardType::Creature)
            {
                return false;
            }

            let has_counters = object.counters.values().any(|count| *count > 0);
            let has_equipment = object.attachments.iter().any(|attachment_id| {
                game.object(*attachment_id).is_some_and(|attachment| {
                    let attachment_subtypes = if allow_calculated_pt
                        && attachment.zone == Zone::Battlefield
                        && view.is_none_or(|view| {
                            view.requires_battlefield_characteristic_calculation(*attachment_id)
                        }) {
                        view.map(|view| view.calculated_subtypes(*attachment_id))
                            .unwrap_or_else(|| game.calculated_subtypes(*attachment_id))
                    } else {
                        attachment.subtypes.clone()
                    };
                    attachment_subtypes.contains(&Subtype::Equipment)
                })
            });
            let has_controlled_aura = ctx.you.is_some_and(|you| {
                object.attachments.iter().any(|attachment_id| {
                    game.object(*attachment_id).is_some_and(|attachment| {
                        let attachment_subtypes = if allow_calculated_pt
                            && attachment.zone == Zone::Battlefield
                            && view.is_none_or(|view| {
                                view.requires_battlefield_characteristic_calculation(*attachment_id)
                            }) {
                            view.map(|view| view.calculated_subtypes(*attachment_id))
                                .unwrap_or_else(|| game.calculated_subtypes(*attachment_id))
                        } else {
                            attachment.subtypes.clone()
                        };
                        game.current_controller(*attachment_id)
                            .is_some_and(|controller| controller == you)
                            && attachment_subtypes.contains(&Subtype::Aura)
                    })
                })
            });
            if !(has_counters || has_equipment || has_controlled_aura) {
                return false;
            }
        }

        // Controller check
        if let Some(controller_filter) = &self.controller
            && !game
                .current_controller(object.id)
                .is_some_and(|controller| controller_filter.matches_player(controller, ctx))
        {
            return false;
        }

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

        if self.first_spell_cast_each_turn
            && game.turn_store.turn_history.spell_cast_order(object.id) != Some(1)
        {
            return false;
        }

        // Owner check
        if let Some(owner_filter) = &self.owner
            && !owner_filter.matches_player(object.owner, ctx)
        {
            return false;
        }

        if self.type_or_subtype_union {
            let type_match = !self.card_types.is_empty()
                && self
                    .card_types
                    .iter()
                    .any(|t| object.card_types.contains(t));
            let subtype_match = !self.subtypes.is_empty()
                && self
                    .subtypes
                    .iter()
                    .any(|t| object_matches_subtype(object, *t, game));
            if (!self.card_types.is_empty() || !self.subtypes.is_empty())
                && !(type_match || subtype_match)
            {
                return false;
            }
        } else if !self.card_types.is_empty()
            && !self
                .card_types
                .iter()
                .any(|t| object.card_types.contains(t))
        {
            return false;
        }

        // Card types (must have all if specified)
        if !self.all_card_types.is_empty()
            && !self
                .all_card_types
                .iter()
                .all(|t| object.card_types.contains(t))
        {
            return false;
        }

        // Excluded card types (must have none of these)
        if self
            .excluded_card_types
            .iter()
            .any(|t| object.card_types.contains(t))
        {
            return false;
        }

        // Subtypes (must have at least one if specified)
        if !self.type_or_subtype_union
            && !self.subtypes.is_empty()
            && !self
                .subtypes
                .iter()
                .any(|t| object_matches_subtype(object, *t, game))
        {
            return false;
        }

        // Excluded subtypes (must have none of these)
        if self
            .excluded_subtypes
            .iter()
            .any(|t| object_matches_subtype(object, *t, game))
        {
            return false;
        }
        if self.chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            if let Some(chosen_type) = game.chosen_creature_type(source) {
                if !object.subtypes.contains(&chosen_type) {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_card_type(source) {
                if !object.card_types.contains(&chosen_type) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if self.excluded_chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            if let Some(chosen_type) = game.chosen_creature_type(source) {
                if object.subtypes.contains(&chosen_type) {
                    return false;
                }
            } else if let Some(chosen_type) = game.chosen_card_type(source) {
                if object.card_types.contains(&chosen_type) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Supertypes (must have at least one if specified)
        if !self.supertypes.is_empty()
            && !self
                .supertypes
                .iter()
                .any(|t| object.supertypes.contains(t))
        {
            return false;
        }

        // Excluded supertypes (must have none of these)
        if self
            .excluded_supertypes
            .iter()
            .any(|t| object.supertypes.contains(t))
        {
            return false;
        }

        // Color check
        if let Some(required_colors) = &self.colors {
            let obj_colors = object.colors();
            if required_colors.intersection(obj_colors).is_empty() {
                return false;
            }
        }
        if self.chosen_color {
            let Some(chosen_color) = ctx.source.and_then(|source| game.chosen_color(source)) else {
                return false;
            };
            if !object.colors().contains(chosen_color) {
                return false;
            }
        }

        // Excluded colors check
        if !self.excluded_colors.is_empty()
            && !self
                .excluded_colors
                .intersection(object.colors())
                .is_empty()
        {
            return false;
        }

        // Colorless check
        if self.colorless && !object.colors().is_empty() {
            return false;
        }

        // Multicolored check
        if self.multicolored && object.colors().count() < 2 {
            return false;
        }

        // Monocolored check
        if self.monocolored && object.colors().count() != 1 {
            return false;
        }

        if let Some(require_all_colors) = self.all_colors {
            let is_all_colors = object.colors().count() == 5;
            if require_all_colors != is_all_colors {
                return false;
            }
        }

        if let Some(require_exactly_two_colors) = self.exactly_two_colors {
            let is_exactly_two_colors = object.colors().count() == 2;
            if require_exactly_two_colors != is_exactly_two_colors {
                return false;
            }
        }
        if let Some(color_count_cmp) = &self.color_count {
            let color_count = object.colors().count() as i32;
            if !color_count_cmp.satisfies_with_context(color_count, game, ctx, stack_entry) {
                return false;
            }
        }

        let is_historic = object.card_types.contains(&CardType::Artifact)
            || object.supertypes.contains(&Supertype::Legendary)
            || object.subtypes.contains(&Subtype::Saga);
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

        // "Other" check (not the source)
        if self.other
            && ctx.target_objects.is_empty()
            && let Some(source_id) = ctx.source
            && object.id == source_id
        {
            return false;
        }
        if self.other
            && ctx
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
        if self.attacked_this_turn && !game.creature_attacked_this_turn(object.id) {
            return false;
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let Some(defending_player) = attacking_defending_player_for_object(object.id, game)
            else {
                return false;
            };
            if !player_filter.matches_player(defending_player, ctx) {
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
        if self.in_combat_with_source {
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(combat) = &game.combat else {
                return false;
            };
            let source_attacks_object =
                crate::combat_state::get_blockers(combat, source_id).contains(&object.id);
            let source_blocks_object = crate::combat_state::get_blocked_attacker(combat, source_id)
                .is_some_and(|attacker| attacker == object.id);
            if !source_attacks_object && !source_blocks_object {
                return false;
            }
        }

        // Power check
        if let Some(power_cmp) = &self.power {
            if let Some(power) = resolve_object_power_for_filter(
                object,
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
            if let Some(power) = resolve_object_power_for_filter(
                object,
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
            let Some(effective_power) = resolve_object_power_for_filter(
                object,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            let Some(base_power) = resolve_object_power_for_filter(
                object,
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

        if let Some(relation) = self.power_relative_to_source {
            let Some(candidate_power) = resolve_object_power_for_filter(
                object,
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
            if let Some(toughness) = resolve_object_toughness_for_filter(
                object,
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
            let Some(power) = resolve_object_power_for_filter(
                object,
                game,
                PtReference::Effective,
                allow_calculated_pt,
            ) else {
                return false;
            };
            let Some(toughness) = resolve_object_toughness_for_filter(
                object,
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
            let mv = object
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
            if !mv_cmp.satisfies_with_context(mv, game, ctx, stack_entry) {
                return false;
            }
        }
        if let Some(mana_value_parity) = self.mana_value_parity {
            let mv = object
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
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
            let mv = object
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
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

        // No X in cost check
        if self.no_x_in_cost
            && let Some(mc) = &object.mana_cost
            && mc.has_x()
        {
            return false;
        }
        if self.has_x_in_cost
            && !object
                .mana_cost
                .as_ref()
                .is_some_and(crate::mana::ManaCost::has_x)
        {
            return false;
        }

        self.matches_shared_tail(object, ctx, game, stack_entry)
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

        if let Some(targetability) = &self.could_be_targeted_by
            && !object_could_be_targeted_by(snapshot.object_id, targetability, ctx, game)
        {
            return false;
        }

        if self.entered_since_your_last_turn_ended && !game.is_summoning_sick(snapshot.object_id) {
            return false;
        }

        // Zone check
        if let Some(zone) = &self.zone
            && snapshot.zone != *zone
        {
            return false;
        }

        // Controller check
        if let Some(controller_filter) = &self.controller
            && !controller_filter.matches_player(snapshot.controller, ctx)
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

        if self.first_spell_cast_each_turn
            && game
                .turn_store
                .turn_history
                .spell_cast_order(snapshot.object_id)
                != Some(1)
        {
            return false;
        }

        // Owner check
        if let Some(owner_filter) = &self.owner
            && !owner_filter.matches_player(snapshot.owner, ctx)
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
            if let Some(chosen_type) = game.chosen_creature_type(source) {
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
        if self.excluded_chosen_creature_type {
            let Some(source) = ctx.source else {
                return false;
            };
            if let Some(chosen_type) = game.chosen_creature_type(source) {
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

        // "Other" check (not the source)
        if self.other
            && ctx.target_objects.is_empty()
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
            && ctx.target_objects.iter().any(|target| {
                target.object_id == snapshot.object_id || target.stable_id == snapshot.stable_id
            })
        {
            return false;
        }

        if self.tapped && !snapshot.tapped {
            return false;
        }
        if self.untapped && snapshot.tapped {
            return false;
        }
        if self.enlist_eligible && !object_is_enlist_eligible(game, snapshot.object_id) {
            return false;
        }
        if self.attacked_this_turn && !game.creature_attacked_this_turn(snapshot.object_id) {
            return false;
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let Some(defending_player) =
                attacking_defending_player_for_object(snapshot.object_id, game)
            else {
                return false;
            };
            if !player_filter.matches_player(defending_player, ctx) {
                return false;
            }
        }
        if self.in_combat_with_source {
            let Some(source_id) = ctx.source else {
                return false;
            };
            let Some(combat) = &game.combat else {
                return false;
            };
            let source_attacks_object =
                crate::combat_state::get_blockers(combat, source_id).contains(&snapshot.object_id);
            let source_blocks_object = crate::combat_state::get_blocked_attacker(combat, source_id)
                .is_some_and(|attacker| attacker == snapshot.object_id);
            if !source_attacks_object && !source_blocks_object {
                return false;
            }
        }
        if let Some(blocker_ref) = &self.blocked_by
            && !creature_was_blocked_by_ref(game, ctx, snapshot.object_id, blocker_ref)
        {
            return false;
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
            let mv = snapshot
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
            if !mv_cmp.satisfies_with_context(mv, game, ctx, None) {
                return false;
            }
        }
        if let Some(mana_value_parity) = self.mana_value_parity {
            let mv = snapshot
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
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
            let mv = snapshot
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
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

        self.matches_shared_tail(snapshot, ctx, game, None)
    }

    /// Generate a human-readable description of this filter.
    ///
    /// Used primarily for trigger display text.
    fn description(&self) -> String {
        let any_of_keyword_clause = describe_simple_any_of_keyword_clause(&self.any_of);
        if any_of_keyword_clause.is_none() && !self.any_of.is_empty() {
            return self
                .any_of
                .iter()
                .map(ObjectFilter::description)
                .collect::<Vec<_>>()
                .join(" or ");
        }

        let mut parts = Vec::new();
        let mut post_noun_qualifiers: Vec<String> = Vec::new();
        let append_token_after_type = self.token;
        let mut controller_suffix: Option<String> = None;
        let mut owner_suffix: Option<String> = None;

        // Handle "other" modifier
        if self.other {
            parts.push("another".to_string());
        }
        let has_target_tag = self.tagged_constraints.iter().any(|constraint| {
            matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                && constraint.tag.as_str().starts_with("targeted")
        });
        if has_target_tag {
            parts.push("target".to_string());
        }
        if self.source {
            parts.push("this".to_string());
        }
        if self.modified {
            parts.push("modified".to_string());
        }

        let has_leading_determiner = self.other || has_target_tag || self.source;

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
                PlayerFilter::Opponent => parts.push("an opponent's".to_string()),
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
                PlayerFilter::MaxSpeed { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => parts.push(format!(
                    "a player who cast one or more {} spells this turn's",
                    card_type.to_string().to_ascii_lowercase()
                )),
                PlayerFilter::ChosenPlayer => parts.push("the chosen player's".to_string()),
                PlayerFilter::TaggedPlayer(_) => parts.push("that player's".to_string()),
                PlayerFilter::Teammate => parts.push("a teammate's".to_string()),
                PlayerFilter::Defending => parts.push("the defending player's".to_string()),
                PlayerFilter::Attacking => parts.push("an attacking player's".to_string()),
                PlayerFilter::DamagedPlayer => parts.push("the damaged player's".to_string()),
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
                PlayerFilter::Excluding { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::Target(inner) => {
                    let inner_desc = describe_player_filter(inner.as_ref());
                    if inner_desc == "player" {
                        parts.push("target player's".to_string());
                    } else {
                        parts.push(format!("target {inner_desc}'s"));
                    }
                }
                PlayerFilter::ControllerOf(_) => parts.push("a controller's".to_string()),
                PlayerFilter::OwnerOf(_) => parts.push("an owner's".to_string()),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    parts.push("that player's".to_string())
                }
            }
        }

        if let Some(cast_by) = &self.cast_by {
            post_noun_qualifiers.push(format!("cast by {}", describe_player_filter(cast_by)));
        }

        // Handle owner on object-level filters (battlefield/stack/any-zone object references).
        // Zone-restricted card references (e.g. "in your graveyard") already encode ownership.
        let owner_conveyed_by_zone = matches!(
            self.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        );
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
                PlayerFilter::MaxSpeed { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
                    "a player who cast one or more {} spells this turn owns",
                    card_type.to_string().to_ascii_lowercase()
                ),
                PlayerFilter::ChosenPlayer => "the chosen player owns".to_string(),
                PlayerFilter::TaggedPlayer(_) => "that player owns".to_string(),
                PlayerFilter::Teammate => "a teammate owns".to_string(),
                PlayerFilter::Defending => "the defending player owns".to_string(),
                PlayerFilter::Attacking => "an attacking player owns".to_string(),
                PlayerFilter::DamagedPlayer => "the damaged player owns".to_string(),
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
        if let Some(colors) = self.colors {
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
                    parts.push(color_words.join(" or "));
                }
            }
        }
        if self.chosen_color {
            post_noun_qualifiers.push("of the chosen color".to_string());
        }
        if self.chosen_creature_type {
            post_noun_qualifiers.push("of the chosen type".to_string());
        }
        if self.excluded_chosen_creature_type {
            post_noun_qualifiers.push("that aren't of the chosen type".to_string());
        }
        for constraint in &self.tagged_constraints {
            match constraint.relation {
                TaggedOpbjectRelation::IsTaggedObject => match constraint.tag.as_str() {
                    "it" => parts.push("that".to_string()),
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
                },
                TaggedOpbjectRelation::IsNotTaggedObject => {
                    parts.push("other".to_string());
                }
                TaggedOpbjectRelation::SameNameAsTagged => {
                    post_noun_qualifiers.push("with the same name as that object".to_string());
                }
                TaggedOpbjectRelation::DifferentNameFromTagged => {
                    post_noun_qualifiers
                        .push("with a different name from those objects".to_string());
                }
                TaggedOpbjectRelation::SameControllerAsTagged => {
                    post_noun_qualifiers.push("controlled by that object's controller".to_string());
                }
                TaggedOpbjectRelation::SameManaValueAsTagged => {
                    if constraint.tag.as_str().starts_with("sacrifice_cost_") {
                        post_noun_qualifiers.push(
                            "with the same mana value as the sacrificed creature".to_string(),
                        );
                    } else {
                        post_noun_qualifiers
                            .push("with the same mana value as that object".to_string());
                    }
                }
                TaggedOpbjectRelation::ManaValueLteTagged => {
                    if constraint.tag.as_str() == "triggering" {
                        post_noun_qualifiers
                            .push("with equal or lesser mana value than that spell".to_string());
                    } else {
                        post_noun_qualifiers.push(
                            "with mana value less than or equal to that object's mana value"
                                .to_string(),
                        );
                    }
                }
                TaggedOpbjectRelation::ManaValueLtTagged => {
                    post_noun_qualifiers
                        .push("with lesser mana value than that object".to_string());
                }
                TaggedOpbjectRelation::SharesColorWithTagged => {
                    post_noun_qualifiers.push("that shares a color with that object".to_string());
                }
                TaggedOpbjectRelation::SharesSubtypeWithTagged => {
                    post_noun_qualifiers
                        .push("that shares a creature type with that object".to_string());
                }
                TaggedOpbjectRelation::SharesCardType => {
                    let permanent_type_context = self.zone == Some(Zone::Battlefield)
                        || (!self.card_types.is_empty()
                            && self.card_types.iter().all(|card_type| {
                                matches!(
                                    card_type,
                                    CardType::Artifact
                                        | CardType::Creature
                                        | CardType::Enchantment
                                        | CardType::Land
                                        | CardType::Planeswalker
                                        | CardType::Battle
                                )
                            }));
                    if permanent_type_context {
                        post_noun_qualifiers
                            .push("that shares a permanent type with that object".to_string());
                    } else {
                        post_noun_qualifiers
                            .push("that shares a card type with that object".to_string());
                    }
                }
                TaggedOpbjectRelation::AttachedToTaggedObject => {
                    post_noun_qualifiers.push("attached to that object".to_string());
                }
                TaggedOpbjectRelation::SoulbondPartnerOfTagged => {
                    post_noun_qualifiers.push("paired with that object".to_string());
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
            if exactly_two_colors {
                post_noun_qualifiers.push("that are exactly two colors".to_string());
            } else {
                post_noun_qualifiers.push("that are not exactly two colors".to_string());
            }
        }
        if self.historic {
            parts.push("historic".to_string());
        }
        if self.nonhistoric {
            post_noun_qualifiers.push("that's not historic".to_string());
        }
        if self.is_commander {
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
        if self.attacking && self.blocking {
            parts.push("attacking/blocking".to_string());
        } else {
            if self.attacking {
                parts.push("attacking".to_string());
            }
            if self.blocking {
                parts.push("blocking".to_string());
            }
        }
        if self.attacked_this_turn {
            post_noun_qualifiers.push("that attacked this turn".to_string());
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let player_text = player_filter.description();
            post_noun_qualifiers.push(format!(
                "attacking {player_text} or a planeswalker controlled by {player_text}"
            ));
        }
        if self.in_combat_with_source {
            post_noun_qualifiers.push("blocking or blocked by this creature".to_string());
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
        if self.tapped && self.untapped {
            parts.push("tapped/untapped".to_string());
        } else if self.tapped {
            parts.push("tapped".to_string());
        } else if self.untapped {
            parts.push("untapped".to_string());
        }
        if self.entered_since_your_last_turn_ended {
            post_noun_qualifiers.push("that entered since your last turn ended".to_string());
        }
        if self.no_abilities {
            post_noun_qualifiers.push("with no abilities".to_string());
        }

        let subtype_implies_type = !self.subtypes.is_empty()
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
                    describe_card_type_source_phrase(&self.card_types)
                ));
                Some((false, describe_stack_object_kind(kind).to_string()))
            } else if has_all_permanent_types {
                Some((true, "permanent".to_string()))
            } else {
                Some((true, describe_card_type_list(&self.card_types)))
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
                    | Some(Zone::OutsideGame) => "card",
                    _ => "source",
                }
            } else {
                match self.zone {
                    Some(Zone::Battlefield) | None => "permanent",
                    Some(Zone::Stack) => {
                        let kind = self.stack_kind.unwrap_or_else(|| {
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
                    | Some(Zone::OutsideGame) => "card",
                }
            };
            Some((false, default_noun.to_string()))
        } else {
            None
        };

        let subtype_phrase = if !self.subtypes.is_empty() {
            let mut parts = Vec::new();
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
            Some(parts.join(" or "))
        } else {
            None
        };

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
            && matches!(self.zone, None | Some(Zone::Battlefield));
        if self.type_or_subtype_union {
            match (type_phrase, subtype_phrase) {
                (Some((_, type_phrase)), Some(subtype_phrase)) => {
                    parts.push(format!("{type_phrase} or {subtype_phrase}"));
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
        if !post_noun_qualifiers.is_empty() {
            parts.extend(post_noun_qualifiers);
        }
        if self.distinct_names {
            parts.push("with different names".to_string());
        }
        if self.distinct_powers {
            parts.push("with different powers".to_string());
        }

        // Handle name
        if let Some(ref name) = self.name {
            return format!("a {} named {}", parts.join(" "), name);
        }
        if let Some(ref name) = self.excluded_name {
            return format!("{} not named {}", parts.join(" "), name);
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
        for ability in &self.excluded_static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("without {}", label));
            }
        }
        for marker in &self.excluded_ability_markers {
            parts.push(format!("without {}", marker.to_ascii_lowercase()));
        }
        if let Some(counter_requirement) = self.with_counter {
            parts.push(format!(
                "with {} on it",
                describe_counter_constraint(counter_requirement)
            ));
        }
        if let Some(counter_exclusion) = self.without_counter {
            parts.push(format!(
                "without {} on it",
                describe_counter_constraint(counter_exclusion)
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
                Zone::OutsideGame => Some("outside the game"),
            };
            if zone == Zone::Exile && has_source_exiled_constraint {
                // Keep wording compact: "card exiled with this permanent" is
                // clearer than appending an extra "in exile" qualifier.
            } else if let Some(zone_name) = zone_name {
                if let Some(owner) = &self.owner {
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

        if (self.entered_battlefield_this_turn || self.entered_battlefield_controller.is_some())
            && self.zone == Some(Zone::Battlefield)
        {
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

        if self.entered_graveyard_from_battlefield_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from the battlefield this turn".to_string());
        } else if self.entered_graveyard_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from anywhere this turn".to_string());
        }

        if self.was_dealt_damage_this_turn {
            parts.push("that was dealt damage this turn".to_string());
        }
        if self.drawn_this_turn {
            parts.push("drawn this turn".to_string());
        }

        match (controller_suffix, owner_suffix) {
            (Some(controller), Some(owner))
                if controller == "you control" && owner == "you own" =>
            {
                parts.push("you both own and control".to_string());
            }
            (Some(controller), Some(owner))
                if controller == "that player controls" && owner == "that player owns" =>
            {
                parts.push("that player both owns and controls".to_string());
            }
            (Some(controller), Some(owner)) => {
                parts.push(controller);
                parts.push(owner);
            }
            (Some(controller), None) => parts.push(controller),
            (None, Some(owner)) => parts.push(owner),
            (None, None) => {}
        }

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
                        "or"
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
                    let joiner = if self.targets_any_of { "or" } else { "and" };
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

        parts.join(" ")
    }
}

#[allow(dead_code)]
fn describe_simple_any_of_keyword_clause(any_of: &[ObjectFilter]) -> Option<String> {
    if any_of.len() < 2 {
        return None;
    }

    let mut labels = Vec::new();
    for filter in any_of {
        if !filter.any_of.is_empty() {
            return None;
        }

        let mut stripped = filter.clone();
        stripped.static_abilities.clear();
        stripped.excluded_static_abilities.clear();
        stripped.ability_markers.clear();
        stripped.excluded_ability_markers.clear();
        if stripped != ObjectFilter::default() {
            return None;
        }

        if filter.static_abilities.len() == 1 && filter.ability_markers.is_empty() {
            let label = describe_filter_static_ability(filter.static_abilities[0])?;
            labels.push(label.to_string());
            continue;
        }
        if filter.ability_markers.len() == 1 && filter.static_abilities.is_empty() {
            labels.push(filter.ability_markers[0].to_ascii_lowercase());
            continue;
        }

        return None;
    }

    Some(labels.join(" or "))
}

fn plus_minus_counter_delta(counters: &std::collections::HashMap<CounterType, u32>) -> i32 {
    let plus = counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0) as i32;
    let minus = counters
        .get(&CounterType::MinusOneMinusOne)
        .copied()
        .unwrap_or(0) as i32;
    plus - minus
}

fn object_base_power_for_filter(object: &Object) -> Option<i32> {
    if let Some(power) = object.power() {
        return Some(power - plus_minus_counter_delta(&object.counters));
    }
    object.base_power.as_ref().map(|pt| pt.base_value())
}

fn object_base_toughness_for_filter(object: &Object) -> Option<i32> {
    if let Some(toughness) = object.toughness() {
        return Some(toughness - plus_minus_counter_delta(&object.counters));
    }
    object.base_toughness.as_ref().map(|pt| pt.base_value())
}

fn resolve_object_power_for_filter(
    object: &Object,
    game: &crate::game_state::GameState,
    reference: PtReference,
    allow_calculated_pt: bool,
) -> Option<i32> {
    match reference {
        PtReference::Base => object_base_power_for_filter(object),
        PtReference::Effective => {
            if allow_calculated_pt {
                game.calculated_power(object.id).or_else(|| object.power())
            } else {
                object.power()
            }
        }
    }
}

fn resolve_object_toughness_for_filter(
    object: &Object,
    game: &crate::game_state::GameState,
    reference: PtReference,
    allow_calculated_pt: bool,
) -> Option<i32> {
    match reference {
        PtReference::Base => object_base_toughness_for_filter(object),
        PtReference::Effective => {
            if allow_calculated_pt {
                game.calculated_toughness(object.id)
                    .or_else(|| object.toughness())
            } else {
                object.toughness()
            }
        }
    }
}

fn snapshot_base_power_for_filter(snapshot: &crate::snapshot::ObjectSnapshot) -> Option<i32> {
    if let Some(power) = snapshot.power {
        return Some(power - plus_minus_counter_delta(&snapshot.counters));
    }
    snapshot.base_power
}

fn snapshot_base_toughness_for_filter(snapshot: &crate::snapshot::ObjectSnapshot) -> Option<i32> {
    if let Some(toughness) = snapshot.toughness {
        return Some(toughness - plus_minus_counter_delta(&snapshot.counters));
    }
    snapshot.base_toughness
}

fn resolve_snapshot_power_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
    reference: PtReference,
) -> Option<i32> {
    match reference {
        PtReference::Effective => snapshot.power,
        PtReference::Base => snapshot_base_power_for_filter(snapshot),
    }
}

fn resolve_snapshot_toughness_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
    reference: PtReference,
) -> Option<i32> {
    match reference {
        PtReference::Effective => snapshot.toughness,
        PtReference::Base => snapshot_base_toughness_for_filter(snapshot),
    }
}

fn attacking_defending_player_for_object(
    object_id: ObjectId,
    game: &crate::game_state::GameState,
) -> Option<PlayerId> {
    let combat = game.combat.as_ref()?;
    let target = crate::combat_state::get_attack_target(combat, object_id)?;
    match target {
        crate::combat_state::AttackTarget::Player(player_id) => Some(*player_id),
        crate::combat_state::AttackTarget::Planeswalker(planeswalker_id) => game
            .object(*planeswalker_id)
            .map(|object| game.controller_of(object)),
    }
}

#[allow(dead_code)]
fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "a player's".to_string(),
        PlayerFilter::You => "your".to_string(),
        PlayerFilter::NotYou => "a non-you player's".to_string(),
        PlayerFilter::Opponent => "an opponent's".to_string(),
        PlayerFilter::Teammate => "a teammate's".to_string(),
        PlayerFilter::Active => "the active player's".to_string(),
        PlayerFilter::Defending => "the defending player's".to_string(),
        PlayerFilter::Attacking => "an attacking player's".to_string(),
        PlayerFilter::DamagedPlayer => "the damaged player's".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell's".to_string(),
        PlayerFilter::Specific(_) => "that player's".to_string(),
        PlayerFilter::MostLifeTied => "the chosen player's".to_string(),
        PlayerFilter::LowestLifeTied => "the chosen player's".to_string(),
        PlayerFilter::MostCardsInHand => "the player with the most cards in hand's".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "a player who cast one or more {} spells this turn's",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
            format!("{}'s", describe_player_filter(filter))
        }
        PlayerFilter::MaxSpeed { .. } => format!("{}'s", describe_player_filter(filter)),
        PlayerFilter::ChosenPlayer => "the chosen player's".to_string(),
        PlayerFilter::TaggedPlayer(_) => "that player's".to_string(),
        PlayerFilter::IteratedPlayer => "that player's".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller's".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_possessive_player_filter(base),
            describe_possessive_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => {
            let base = match inner.as_ref() {
                PlayerFilter::Any => "target player".to_string(),
                other => format!("target {}", describe_player_filter(other)),
            };
            format!("{base}'s")
        }
        PlayerFilter::ControllerOf(_) => "that object's controller's".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner's".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player's".to_string()
        }
    }
}

pub(crate) fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "player".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::NotYou => "player other than you".to_string(),
        PlayerFilter::Opponent => "opponent".to_string(),
        PlayerFilter::Teammate => "teammate".to_string(),
        PlayerFilter::Active => "active player".to_string(),
        PlayerFilter::Defending => "defending player".to_string(),
        PlayerFilter::Attacking => "attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "damaged player".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell".to_string(),
        PlayerFilter::Specific(_) => "player".to_string(),
        PlayerFilter::MostLifeTied => "player with the most life or tied for most life".to_string(),
        PlayerFilter::LowestLifeTied => {
            "player with the lowest life or tied for lowest life".to_string()
        }
        PlayerFilter::MostCardsInHand => "the player who has the most cards in hand".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "player who cast one or more {} spells this turn",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let count_text = count.to_string();
            format!(
                "{} who has at least {count_text} more cards in hand than you do",
                describe_player_filter(base)
            )
        }
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            let verb = if *has_max_speed {
                "has max speed"
            } else {
                "doesn't have max speed"
            };
            format!("{} who {verb}", describe_player_filter(base))
        }
        PlayerFilter::ChosenPlayer => "chosen player".to_string(),
        PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
            "enchanted player".to_string()
        }
        PlayerFilter::TaggedPlayer(_) => "that player".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_player_filter(base),
            describe_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => format!("target {}", describe_player_filter(inner)),
        PlayerFilter::ControllerOf(_) => "controller".to_string(),
        PlayerFilter::OwnerOf(_) => "owner".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player".to_string()
        }
    }
}

#[allow(dead_code)]
fn describe_card_type_word(card_type: CardType) -> &'static str {
    card_type.name()
}

#[allow(dead_code)]
fn describe_card_type_list(card_types: &[CardType]) -> String {
    match card_types {
        [] => String::new(),
        [single] => single.name().to_string(),
        [first, second] => format!("{} or {}", first.name(), second.name()),
        _ => {
            let mut names = card_types
                .iter()
                .map(|card_type| card_type.name())
                .collect::<Vec<_>>();
            let last = names.pop().expect("card type list is non-empty");
            format!("{}, or {}", names.join(", "), last)
        }
    }
}

#[allow(dead_code)]
fn describe_card_type_source_phrase(card_types: &[CardType]) -> String {
    let types = describe_card_type_list(card_types);
    if types.is_empty() {
        return "a source".to_string();
    }
    let article = if types
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {types}")
}

#[allow(dead_code)]
fn describe_stack_object_kind(kind: StackObjectKind) -> &'static str {
    match kind {
        StackObjectKind::Spell => "spell",
        StackObjectKind::Ability => "ability",
        StackObjectKind::ActivatedAbility => "activated ability",
        StackObjectKind::TriggeredAbility => "triggered ability",
        StackObjectKind::SpellOrAbility => "spell or ability",
    }
}

fn alternative_cast_matches_kind(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    kind: AlternativeCastKind,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    match (kind, method) {
        (AlternativeCastKind::Blitz, AlternativeCastingMethod::Blitz { .. }) => true,
        (AlternativeCastKind::Dash, AlternativeCastingMethod::Dash { .. }) => true,
        (AlternativeCastKind::Flashback, AlternativeCastingMethod::Flashback { .. }) => true,
        (AlternativeCastKind::JumpStart, AlternativeCastingMethod::JumpStart) => true,
        (AlternativeCastKind::Escape, AlternativeCastingMethod::Escape { .. }) => true,
        (AlternativeCastKind::Madness, AlternativeCastingMethod::Madness { .. }) => true,
        (AlternativeCastKind::Miracle, AlternativeCastingMethod::Miracle { .. }) => true,
        _ => false,
    }
}

fn object_has_alternative_cast_kind(
    object: &Object,
    kind: AlternativeCastKind,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
) -> bool {
    if object
        .alternative_casts
        .iter()
        .any(|method| alternative_cast_matches_kind(method, kind))
    {
        return true;
    }

    // Include temporary grants (e.g., Snapcaster Mage granting flashback).
    let Some(player) = ctx.you else {
        return false;
    };
    game.effect_store
        .grant_registry
        .granted_alternative_casts_for_card(game, object.id, object.zone, player)
        .iter()
        .any(|grant| alternative_cast_matches_kind(&grant.method, kind))
}

fn object_has_static_ability_id(object: &Object, ability_id: StaticAbilityId) -> bool {
    use crate::ability::AbilityKind;

    let has_regular = object.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            static_ability.id() == ability_id
        } else {
            false
        }
    });
    if has_regular {
        return true;
    }

    object
        .level_granted_abilities()
        .iter()
        .any(|ability| ability.id() == ability_id)
}

fn object_has_ability_marker(object: &Object, marker: &str) -> bool {
    use crate::ability::AbilityKind;

    let normalized_marker = marker.trim().to_ascii_lowercase();
    if matches!(
        normalized_marker.as_str(),
        "mana ability" | "mana abilities"
    ) {
        return object_has_mana_ability(object);
    }
    if normalized_marker == "cycling" && object.abilities.iter().any(ability_is_structural_cycling)
    {
        return true;
    }
    if normalized_marker == "craft" && object.abilities.iter().any(ability_is_structural_craft) {
        return true;
    }

    let has_regular = object.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            matches!(
                static_ability.id(),
                StaticAbilityId::KeywordMarker | StaticAbilityId::KeywordText
            ) && static_ability.display().eq_ignore_ascii_case(marker)
        } else {
            false
        }
    });
    if has_regular {
        return true;
    }

    if object
        .abilities
        .iter()
        .any(|ability| ability_text_has_marker(ability, marker))
    {
        return true;
    }

    object.level_granted_abilities().iter().any(|ability| {
        matches!(
            ability.id(),
            StaticAbilityId::KeywordMarker | StaticAbilityId::KeywordText
        ) && ability.display().eq_ignore_ascii_case(marker)
    })
}

fn object_has_mana_ability(object: &Object) -> bool {
    object
        .abilities
        .iter()
        .any(|ability| ability.is_mana_ability())
}

fn object_has_tap_activated_ability(object: &Object) -> bool {
    use crate::ability::AbilityKind;
    object.abilities.iter().any(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => activated.has_tap_cost(),
        _ => false,
    })
}

fn snapshot_has_static_ability_id(
    snapshot: &crate::snapshot::ObjectSnapshot,
    ability_id: StaticAbilityId,
) -> bool {
    snapshot.has_static_ability_id(ability_id)
}

fn snapshot_has_ability_marker(snapshot: &crate::snapshot::ObjectSnapshot, marker: &str) -> bool {
    use crate::ability::AbilityKind;

    let normalized_marker = marker.trim().to_ascii_lowercase();
    if matches!(
        normalized_marker.as_str(),
        "mana ability" | "mana abilities"
    ) {
        return snapshot_has_mana_ability(snapshot);
    }
    if normalized_marker == "cycling"
        && snapshot.abilities.iter().any(ability_is_structural_cycling)
    {
        return true;
    }
    if normalized_marker == "craft" && snapshot.abilities.iter().any(ability_is_structural_craft) {
        return true;
    }

    snapshot.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind
            && matches!(
                static_ability.id(),
                StaticAbilityId::KeywordMarker | StaticAbilityId::KeywordText
            )
            && static_ability.display().eq_ignore_ascii_case(marker)
        {
            return true;
        }
        ability_text_has_marker(ability, marker)
    })
}

fn ability_is_structural_cycling(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };
    if !ability.functional_zones.contains(&Zone::Hand)
        || !matches!(activated.timing, crate::ability::ActivationTiming::AnyTime)
    {
        return false;
    }
    let costs = activated.mana_cost.costs();
    costs.iter().any(cost_is_discard_this_card) && costs.iter().any(cost_is_cycle_keyword_action)
}

fn ability_is_structural_craft(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };
    if !ability.functional_zones.contains(&Zone::Battlefield)
        || !matches!(
            activated.timing,
            crate::ability::ActivationTiming::SorcerySpeed
        )
    {
        return false;
    }
    let costs = activated.mana_cost.costs();
    costs.iter().any(cost_is_exile_this_source) && costs.iter().any(cost_is_craft_keyword_action)
}

fn cost_is_discard_this_card(cost: &crate::costs::Cost) -> bool {
    let Some(discard) = cost
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::DiscardEffect>())
    else {
        return false;
    };
    discard.count == crate::effect::Value::Fixed(1)
        && discard.player == PlayerFilter::You
        && !discard.random
        && discard
            .card_filter
            .as_ref()
            .is_some_and(|filter| filter.source && filter.zone == Some(Zone::Hand))
}

fn cost_is_exile_this_source(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
        .is_some_and(|exile| matches!(exile.spec, ChooseSpec::Source) && !exile.face_down)
}

fn cost_is_cycle_keyword_action(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::EmitKeywordActionEffect>())
        .is_some_and(|emit| {
            emit.action == crate::events::KeywordActionKind::Cycle && emit.amount == 1
        })
}

fn cost_is_craft_keyword_action(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::EmitKeywordActionEffect>())
        .is_some_and(|emit| {
            emit.action == crate::events::KeywordActionKind::Craft && emit.amount == 1
        })
}

fn snapshot_has_mana_ability(snapshot: &crate::snapshot::ObjectSnapshot) -> bool {
    snapshot
        .abilities
        .iter()
        .any(|ability| ability.is_mana_ability())
}

fn ability_text_has_marker(ability: &crate::ability::Ability, marker: &str) -> bool {
    let marker = marker.trim().to_ascii_lowercase();
    if marker.is_empty() {
        return false;
    }
    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    let text = static_ability.display();

    let words = text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '\'')))
                .to_ascii_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return false;
    }

    if marker == "cycling" {
        if !ability.functional_zones.contains(&crate::zone::Zone::Hand) {
            return false;
        }
        return words
            .iter()
            .any(|word| word == "cycling" || word.ends_with("cycling"));
    }

    let marker_words = marker
        .split_whitespace()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if marker_words.is_empty() {
        return false;
    }
    if marker_words.len() == 1 {
        return words.iter().any(|word| word == &marker_words[0]);
    }

    words.windows(marker_words.len()).any(|window| {
        window
            .iter()
            .zip(marker_words.iter())
            .all(|(word, marker_word)| word == marker_word)
    })
}

fn snapshot_has_tap_activated_ability(snapshot: &crate::snapshot::ObjectSnapshot) -> bool {
    use crate::ability::AbilityKind;
    snapshot
        .abilities
        .iter()
        .any(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.has_tap_cost(),
            _ => false,
        })
}

#[allow(dead_code)]
fn describe_counter_constraint(constraint: CounterConstraint) -> String {
    match constraint {
        CounterConstraint::Any => "a counter".to_string(),
        CounterConstraint::Typed(counter_type) => {
            format!("a {} counter", counter_type.description())
        }
    }
}

#[allow(dead_code)]
fn describe_alternative_cast_kind(kind: AlternativeCastKind) -> &'static str {
    match kind {
        AlternativeCastKind::Blitz => "blitz",
        AlternativeCastKind::Dash => "dash",
        AlternativeCastKind::Flashback => "flashback",
        AlternativeCastKind::JumpStart => "jump-start",
        AlternativeCastKind::Escape => "escape",
        AlternativeCastKind::Madness => "madness",
        AlternativeCastKind::Miracle => "miracle",
    }
}

#[allow(dead_code)]
fn describe_filter_static_ability(ability_id: StaticAbilityId) -> Option<&'static str> {
    use StaticAbilityId::*;
    match ability_id {
        Flying => Some("flying"),
        FirstStrike => Some("first strike"),
        DoubleStrike => Some("double strike"),
        Deathtouch => Some("deathtouch"),
        Defender => Some("defender"),
        Flash => Some("flash"),
        Haste => Some("haste"),
        Hexproof => Some("hexproof"),
        Indestructible => Some("indestructible"),
        Intimidate => Some("intimidate"),
        Lifelink => Some("lifelink"),
        Menace => Some("menace"),
        Reach => Some("reach"),
        Skulk => Some("skulk"),
        Shroud => Some("shroud"),
        Trample => Some("trample"),
        Vigilance => Some("vigilance"),
        Fear => Some("fear"),
        Flanking => Some("flanking"),
        Landwalk => Some("landwalk"),
        Bloodthirst => Some("bloodthirst"),
        Morph => Some("morph"),
        Disguise => Some("disguise"),
        Megamorph => Some("megamorph"),
        Shadow => Some("shadow"),
        Horsemanship => Some("horsemanship"),
        Wither => Some("wither"),
        Infect => Some("infect"),
        Changeling => Some("changeling"),
        _ => None,
    }
}

#[allow(dead_code)]
fn describe_comparison(cmp: &Comparison) -> String {
    fn describe_value_expr(value: &crate::effect::Value) -> String {
        use crate::effect::Value;
        match value {
            Value::Fixed(v) => v.to_string(),
            Value::X => "X".to_string(),
            Value::Count(filter) => format!("the number of {}", filter.description()),
            Value::CountScaled(filter, factor) => {
                format!("{factor} times the number of {}", filter.description())
            }
            Value::LandsEnteredBattlefieldThisTurn(player) => {
                format!(
                    "the number of lands that entered the battlefield under {:?}'s control this turn",
                    player
                )
            }
            Value::ColorsAmong(filter) => {
                format!("the number of colors among {}", filter.description())
            }
            Value::CreatureTypesAmong(filter) => {
                format!(
                    "the number of creature types among {}",
                    filter.description()
                )
            }
            Value::CardTypesAmong(filter) => {
                format!("the number of card types among {}", filter.description())
            }
            Value::DistinctPowers(filter) => {
                format!(
                    "the number of different powers among {}",
                    filter.description()
                )
            }
            Value::CountersOnSource(counter_type) => {
                format!(
                    "the number of {} counters on this",
                    counter_type.description()
                )
            }
            Value::CountersOn(_, Some(counter_type)) => {
                format!("the number of {} counters", counter_type.description())
            }
            Value::CountersOn(_, None) => "the number of counters".to_string(),
            Value::SourcePower => "this creature's power".to_string(),
            Value::SourceToughness => "this creature's toughness".to_string(),
            Value::ManaValueOf(spec) => {
                if let ChooseSpec::Tagged(tag) = spec.base()
                    && tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                {
                    "the exiled spell's mana value".to_string()
                } else {
                    "that card's mana value".to_string()
                }
            }
            Value::Add(left, right) => {
                format!(
                    "{} plus {}",
                    describe_value_expr(left),
                    describe_value_expr(right)
                )
            }
            _ => "a dynamic value".to_string(),
        }
    }

    let describe_values = |values: &[i32]| -> String {
        match values.len() {
            0 => String::new(),
            1 => values[0].to_string(),
            2 => format!("{} or {}", values[0], values[1]),
            _ => {
                let head = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}, or {}", values[values.len() - 1])
            }
        }
    };
    match cmp {
        Comparison::Equal(v) => format!("{v}"),
        Comparison::OneOf(values) => describe_values(values),
        Comparison::NotEqual(v) => format!("not equal to {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
        Comparison::EqualExpr(value) => format!("equal to {}", describe_value_expr(value)),
        Comparison::NotEqualExpr(value) => {
            format!("not equal to {}", describe_value_expr(value))
        }
        Comparison::LessThanExpr(value) => format!("less than {}", describe_value_expr(value)),
        Comparison::LessThanOrEqualExpr(value) => {
            format!("{} or less", describe_value_expr(value))
        }
        Comparison::GreaterThanExpr(value) => {
            format!("greater than {}", describe_value_expr(value))
        }
        Comparison::GreaterThanOrEqualExpr(value) => {
            format!("{} or greater", describe_value_expr(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison() {
        assert!(Comparison::Equal(5).satisfies(5));
        assert!(!Comparison::Equal(5).satisfies(4));

        assert!(Comparison::LessThanOrEqual(2).satisfies(2));
        assert!(Comparison::LessThanOrEqual(2).satisfies(1));
        assert!(!Comparison::LessThanOrEqual(2).satisfies(3));

        assert!(Comparison::GreaterThan(3).satisfies(4));
        assert!(!Comparison::GreaterThan(3).satisfies(3));
    }

    #[test]
    fn test_creature_filter() {
        let filter = ObjectFilter::creature();
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn blocked_by_tagged_filter_matches_current_combat_relationship() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Attacker")
                .card_types(vec![CardType::Creature])
                .build();
        let blocker_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(2), "Blocker")
                .card_types(vec![CardType::Creature])
                .build();
        let attacker = Object::from_card(
            ObjectId::from_raw(1),
            &attacker_card,
            alice,
            Zone::Battlefield,
        );
        let blocker =
            Object::from_card(ObjectId::from_raw(2), &blocker_card, bob, Zone::Battlefield);
        game.add_object(attacker.clone());
        game.add_object(blocker.clone());
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker.id,
                target: crate::combat_state::AttackTarget::Player(bob),
            }],
            blockers: std::collections::HashMap::from([(attacker.id, vec![blocker.id])]),
            damage_assignment_order: std::collections::HashMap::new(),
            attacking_bands: Vec::new(),
        });

        let blocker_snapshot =
            ObjectSnapshot::from_object_with_calculated_characteristics(&blocker, &game);
        let ctx = FilterContext::new(alice).with_tagged_objects(&std::collections::HashMap::from(
            [(TagKey::from("chosen_blockers"), vec![blocker_snapshot])],
        ));
        let filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Creature],
            blocked_by: Some(ObjectRef::Tagged(TagKey::from("chosen_blockers"))),
            ..ObjectFilter::default()
        };

        assert!(filter.matches(game.object(attacker.id).unwrap(), &ctx, &game));
    }

    #[test]
    fn test_filter_chaining() {
        let filter = ObjectFilter::creature()
            .you_control()
            .other()
            .with_power(Comparison::GreaterThanOrEqual(3));

        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.other);
        assert!(filter.power.is_some());
    }

    #[test]
    fn test_nonland_filter() {
        let filter = ObjectFilter::nonland();
        assert!(filter.excluded_card_types.contains(&CardType::Land));
    }

    #[test]
    fn test_filter_with_subtypes() {
        let filter = ObjectFilter::creature()
            .with_subtype(crate::types::Subtype::Elf)
            .with_subtype(crate::types::Subtype::Warrior);

        assert_eq!(filter.subtypes.len(), 2);
    }

    #[test]
    fn test_adventure_subtype_filter_matches_front_face_linked_to_adventure() {
        use crate::card::{LinkedFaceLayout, PowerToughness};
        use crate::cards::CardDefinitionBuilder;
        use crate::ids::CardId;
        use crate::snapshot::ObjectSnapshot;
        use crate::zone::Zone;

        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let front_id = CardId::from_raw(47_100);
        let adventure_id = CardId::from_raw(47_101);
        let front = CardDefinitionBuilder::new(front_id, "Linked Adventure Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(2, 2))
            .other_face(adventure_id)
            .other_face_name("Linked Adventure Spell")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        let adventure = CardDefinitionBuilder::new(adventure_id, "Linked Adventure Spell")
            .card_types(vec![CardType::Sorcery])
            .subtypes(vec![Subtype::Adventure])
            .other_face(front_id)
            .other_face_name("Linked Adventure Creature")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&adventure);

        let object_id = game.create_object_from_definition(&front, alice, Zone::Hand);
        let object = game
            .object(object_id)
            .expect("linked adventure creature should exist");
        let ctx = FilterContext::new(alice);
        let adventure_filter = ObjectFilter::default().with_subtype(Subtype::Adventure);

        assert!(adventure_filter.matches(object, &ctx, &game));

        let snapshot = ObjectSnapshot::from_object(object, &game);
        assert!(adventure_filter.matches_snapshot(&snapshot, &ctx, &game));
        assert!(
            !ObjectFilter::default()
                .without_subtype(Subtype::Adventure)
                .matches(object, &ctx, &game)
        );
    }

    #[test]
    fn test_spell_zone_filter_matches_stack_spell_cast_from_graveyard() {
        use crate::alternative_cast::CastingMethod;
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::zone::Zone;

        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let spell = CardBuilder::new(CardId::from_raw(1), "Graveyard Cast Probe")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .build();
        let graveyard_id = game.create_object_from_card(&spell, alice, Zone::Graveyard);
        let stack_id = game
            .move_object_by_effect(graveyard_id, Zone::Stack)
            .expect("move probe spell to stack");
        game.push_to_stack(
            crate::game_state::StackEntry::new(stack_id, alice).with_casting_method(
                CastingMethod::PlayFrom {
                    source: stack_id,
                    zone: Zone::Graveyard,
                    use_alternative: None,
                },
            ),
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(stack_id, alice, Zone::Graveyard),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);

        let filter = ObjectFilter::spell().in_zone(Zone::Graveyard);
        let ctx = FilterContext::new(alice);
        let object = game.object(stack_id).expect("stack spell should exist");
        assert!(
            filter.matches(object, &ctx, &game),
            "spell cast from graveyard should satisfy graveyard origin filter"
        );
    }

    #[test]
    fn test_spell_zone_filter_matches_stack_spell_with_graveyard_alternative_cast() {
        use crate::alternative_cast::{AlternativeCastingMethod, CastingMethod};
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::zone::Zone;

        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let spell = CardBuilder::new(CardId::from_raw(2), "Flashback Probe")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .build();
        let graveyard_id = game.create_object_from_card(&spell, alice, Zone::Graveyard);
        let stack_id = game
            .move_object_by_effect(graveyard_id, Zone::Stack)
            .expect("move flashback probe to stack");
        game.object_mut(stack_id)
            .expect("stack spell should exist")
            .alternative_casts
            .push(AlternativeCastingMethod::Flashback {
                total_cost: crate::cost::TotalCost::mana(ManaCost::default()),
            });
        game.push_to_stack(
            crate::game_state::StackEntry::new(stack_id, alice)
                .with_casting_method(CastingMethod::Alternative(0)),
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(stack_id, alice, Zone::Graveyard),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);

        let filter = ObjectFilter::spell().in_zone(Zone::Graveyard);
        let ctx = FilterContext::new(alice);
        let object = game.object(stack_id).expect("stack spell should exist");
        assert!(
            filter.matches(object, &ctx, &game),
            "spell cast with a graveyard alternative method should satisfy graveyard origin filter"
        );
    }

    #[test]
    fn test_graveyard_filter_uses_current_subtypes() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::cards::CardDefinitionBuilder;
        use crate::ids::CardId;
        use crate::static_abilities::StaticAbility;
        use crate::zone::Zone;

        let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let _beacon_id = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::from_raw(30), "Graveyard Beacon")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::add_subtypes(
                    ObjectFilter::default()
                        .in_zone(Zone::Graveyard)
                        .owned_by(PlayerFilter::You)
                        .with_type(CardType::Creature),
                    vec![Subtype::Wizard],
                )))
                .build(),
            alice,
            Zone::Battlefield,
        );

        let graveyard_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(31), "Vanilla Bear")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build(),
            alice,
            Zone::Graveyard,
        );

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .with_type(CardType::Creature)
            .with_subtype(Subtype::Wizard);
        let ctx = FilterContext::new(alice);
        let object = game
            .object(graveyard_id)
            .expect("graveyard card should exist");

        assert!(
            filter.matches(object, &ctx, &game),
            "off-battlefield subtype filters should use current characteristics"
        );
    }

    #[test]
    fn test_filter_cast_by_matches_context_caster_for_nonstack_cards() {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::zone::Zone;

        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell = CardBuilder::new(CardId::from_raw(3), "Borrowed Probe")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, bob, Zone::Exile);
        let object = game.object(spell_id).expect("spell card should exist");

        let filter = ObjectFilter::default()
            .with_type(CardType::Instant)
            .cast_by_you();
        let alice_casting_ctx = FilterContext::new(alice).with_caster(Some(alice));
        assert!(
            filter.matches(object, &alice_casting_ctx, &game),
            "cast-by filter should use context caster for non-stack card objects"
        );

        let bob_casting_ctx = FilterContext::new(alice).with_caster(Some(bob));
        assert!(
            !filter.matches(object, &bob_casting_ctx, &game),
            "cast-by filter should reject when context caster does not match"
        );

        let no_caster_ctx = FilterContext::new(alice);
        assert!(
            !filter.matches(object, &no_caster_ctx, &game),
            "cast-by filter should not match non-stack cards without explicit caster context"
        );
    }

    #[test]
    fn test_filter_cast_by_uses_stack_controller_when_caster_missing() {
        use crate::alternative_cast::CastingMethod;
        use crate::card::CardBuilder;
        use crate::game_state::StackEntry;
        use crate::ids::CardId;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::zone::Zone;

        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell = CardBuilder::new(CardId::from_raw(4), "Stack Probe")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .build();
        let hand_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let stack_id = game
            .move_object_by_effect(hand_id, Zone::Stack)
            .expect("move spell to stack");
        game.push_to_stack(
            StackEntry::new(stack_id, alice).with_casting_method(CastingMethod::Normal),
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(stack_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);

        let filter = ObjectFilter::spell().cast_by_you();
        let object = game.object(stack_id).expect("stack spell should exist");
        let alice_ctx = FilterContext::new(alice);
        assert!(
            filter.matches(object, &alice_ctx, &game),
            "cast-by filter should fall back to stack controller when caster context is absent"
        );
        let bob_ctx = FilterContext::new(bob);
        assert!(
            !filter.matches(object, &bob_ctx, &game),
            "cast-by filter should respect 'you' against the stack spell controller"
        );
    }

    #[test]
    fn test_filter_description_includes_positive_colors() {
        let filter =
            ObjectFilter::creature().with_colors(ColorSet::from_color(crate::color::Color::Blue));
        assert_eq!(filter.description(), "blue creature");
    }

    #[test]
    fn test_filter_description_includes_tapped_state() {
        let filter = ObjectFilter::creature().tapped();
        assert_eq!(filter.description(), "tapped creature");
    }

    #[test]
    fn test_filter_description_includes_modified_state() {
        let filter = ObjectFilter::creature().modified();
        assert_eq!(filter.description(), "modified creature");
    }

    #[test]
    fn test_filter_description_includes_face_down_state() {
        let filter = ObjectFilter::creature().face_down();
        assert_eq!(filter.description(), "face-down creature");
    }

    #[test]
    fn test_filter_matches_face_down_state() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::game_state::GameState;
        use crate::ids::CardId;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let controller = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(1), "Face-Down Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object_id = game.create_object_from_card(&card, controller, Zone::Battlefield);

        let ctx = FilterContext::new(controller).with_source(object_id);
        let face_down_filter = ObjectFilter::creature().face_down();
        let face_up_filter = ObjectFilter::creature().face_up();

        let object = game.object(object_id).expect("created object should exist");
        assert!(
            face_up_filter.matches(object, &ctx, &game),
            "face-up filter should match by default"
        );
        assert!(
            !face_down_filter.matches(object, &ctx, &game),
            "face-down filter should not match a face-up object"
        );

        game.set_face_down(object_id);
        let object = game.object(object_id).expect("created object should exist");
        assert!(
            face_down_filter.matches(object, &ctx, &game),
            "face-down filter should match after object is set face down"
        );
        assert!(
            !face_up_filter.matches(object, &ctx, &game),
            "face-up filter should not match a face-down object"
        );
    }

    #[test]
    fn test_filter_description_includes_all_card_types() {
        let filter = ObjectFilter::default()
            .with_all_type(CardType::Artifact)
            .with_all_type(CardType::Creature);
        assert_eq!(filter.description(), "artifact creature");
    }

    #[test]
    fn test_filter_description_includes_excluded_subtypes() {
        let filter = ObjectFilter::creature()
            .without_subtype(crate::types::Subtype::Vampire)
            .without_subtype(crate::types::Subtype::Werewolf)
            .without_subtype(crate::types::Subtype::Zombie);
        assert_eq!(
            filter.description(),
            "non-vampire non-werewolf non-zombie creature"
        );
    }

    #[test]
    fn test_filter_description_compacts_full_outlaw_subtype_pack() {
        let filter = ObjectFilter::creature()
            .with_subtype(crate::types::Subtype::Assassin)
            .with_subtype(crate::types::Subtype::Mercenary)
            .with_subtype(crate::types::Subtype::Pirate)
            .with_subtype(crate::types::Subtype::Rogue)
            .with_subtype(crate::types::Subtype::Warlock);
        assert_eq!(filter.description(), "outlaw creature");
    }

    #[test]
    fn test_filter_description_compacts_outlaw_pack_with_extra_subtypes() {
        let filter = ObjectFilter::creature()
            .with_subtype(crate::types::Subtype::Assassin)
            .with_subtype(crate::types::Subtype::Mercenary)
            .with_subtype(crate::types::Subtype::Pirate)
            .with_subtype(crate::types::Subtype::Rogue)
            .with_subtype(crate::types::Subtype::Warlock)
            .with_subtype(crate::types::Subtype::Wizard);
        assert_eq!(filter.description(), "outlaw or Wizard creature");
    }

    #[test]
    fn test_filter_description_includes_skulk() {
        let mut filter = ObjectFilter::creature();
        filter.static_abilities.push(StaticAbilityId::Skulk);
        assert_eq!(filter.description(), "creature with skulk");
    }

    #[test]
    fn test_filter_description_includes_excluded_colors() {
        let filter = ObjectFilter::creature().without_colors(
            ColorSet::from_color(crate::color::Color::Black)
                .union(ColorSet::from_color(crate::color::Color::Red)),
        );
        assert_eq!(filter.description(), "nonblack nonred creature");
    }

    #[test]
    fn test_filter_description_includes_chosen_color_clause() {
        let filter = ObjectFilter::spell().of_chosen_color();
        assert_eq!(filter.description(), "spell of the chosen color");
    }

    #[test]
    fn test_filter_description_includes_entered_since_last_turn_ended_clause() {
        let filter = ObjectFilter {
            card_types: vec![CardType::Creature],
            entered_since_your_last_turn_ended: true,
            ..Default::default()
        };
        assert_eq!(
            filter.description(),
            "creature that entered since your last turn ended"
        );
    }

    #[test]
    fn test_filter_description_includes_commander_owner_and_controller_distinction() {
        let filter = ObjectFilter::creature()
            .commander()
            .owned_by(PlayerFilter::You)
            .controlled_by(PlayerFilter::Opponent);
        assert_eq!(
            filter.description(),
            "an opponent's commander creature you own"
        );
    }

    fn setup_modified_filter_game() -> crate::game_state::GameState {
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_modified_test_creature(
        game: &mut crate::game_state::GameState,
        controller: PlayerId,
    ) -> ObjectId {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::ids::CardId;
        use crate::types::{CardType, Subtype};
        use crate::zone::Zone;

        let card = CardBuilder::new(CardId::from_raw(1), "Test Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_modified_test_equipment(
        game: &mut crate::game_state::GameState,
        controller: PlayerId,
    ) -> ObjectId {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::types::{CardType, Subtype};
        use crate::zone::Zone;

        let card = CardBuilder::new(CardId::from_raw(2), "Test Equipment")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_modified_test_aura(
        game: &mut crate::game_state::GameState,
        controller: PlayerId,
    ) -> ObjectId {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::types::{CardType, Subtype};
        use crate::zone::Zone;

        let card = CardBuilder::new(CardId::from_raw(3), "Test Aura")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_filter_matches_modified_by_counter() {
        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_modified_test_creature(&mut game, alice);

        let ctx = FilterContext::new(alice).with_source(creature_id);
        let filter = ObjectFilter::creature().you_control().modified();

        let creature = game.object(creature_id).expect("creature exists");
        assert!(
            !filter.matches(creature, &ctx, &game),
            "unmodified creature should not match"
        );

        game.object_mut(creature_id)
            .expect("creature exists")
            .counters
            .insert(CounterType::PlusOnePlusOne, 1);
        let creature = game.object(creature_id).expect("creature exists");
        assert!(
            filter.matches(creature, &ctx, &game),
            "creature with a counter should match"
        );
    }

    #[test]
    fn test_filter_matches_modified_by_equipment() {
        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_modified_test_creature(&mut game, alice);
        let equipment_id = create_modified_test_equipment(&mut game, bob);

        game.object_mut(creature_id)
            .expect("creature exists")
            .attachments
            .push(equipment_id);

        let ctx = FilterContext::new(alice).with_source(creature_id);
        let filter = ObjectFilter::creature().you_control().modified();
        let creature = game.object(creature_id).expect("creature exists");
        assert!(
            filter.matches(creature, &ctx, &game),
            "equipped creature should match regardless of equipment controller"
        );
    }

    #[test]
    fn test_filter_matches_intrinsically_equipped_creature_without_tag_context() {
        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_modified_test_creature(&mut game, alice);
        let equipment_id = create_modified_test_equipment(&mut game, bob);

        game.object_mut(creature_id)
            .expect("creature exists")
            .attachments
            .push(equipment_id);

        let mut filter = ObjectFilter::creature().you_control();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("equipped"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        let ctx = FilterContext::new(alice).with_source(creature_id);
        let creature = game.object(creature_id).expect("creature exists");
        assert!(
            filter.matches(creature, &ctx, &game),
            "unbound equipped adjective should match a creature with Equipment attached"
        );
    }

    #[test]
    fn test_filter_matches_intrinsically_equipped_snapshot_without_tag_context() {
        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_modified_test_creature(&mut game, alice);
        let equipment_id = create_modified_test_equipment(&mut game, bob);

        game.object_mut(creature_id)
            .expect("creature exists")
            .attachments
            .push(equipment_id);

        let mut filter = ObjectFilter::creature().you_control();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("equipped"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        let ctx = FilterContext::new(alice).with_source(creature_id);
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(creature_id).expect("creature exists"),
            &game,
        );
        assert!(
            filter.matches_snapshot(&snapshot, &ctx, &game),
            "unbound equipped adjective should match LKI for a creature with Equipment attached"
        );
    }

    #[test]
    fn test_filter_matches_modified_by_controlled_aura() {
        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_modified_test_creature(&mut game, alice);
        let aura_id = create_modified_test_aura(&mut game, alice);

        game.object_mut(creature_id)
            .expect("creature exists")
            .attachments
            .push(aura_id);

        let ctx = FilterContext::new(alice).with_source(creature_id);
        let filter = ObjectFilter::creature().you_control().modified();
        let creature = game.object(creature_id).expect("creature exists");
        assert!(
            filter.matches(creature, &ctx, &game),
            "creature enchanted by an Aura you control should match"
        );
    }

    #[test]
    fn test_filter_does_not_match_modified_by_opponent_aura() {
        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_modified_test_creature(&mut game, alice);
        let aura_id = create_modified_test_aura(&mut game, bob);

        game.object_mut(creature_id)
            .expect("creature exists")
            .attachments
            .push(aura_id);

        let ctx = FilterContext::new(alice).with_source(creature_id);
        let filter = ObjectFilter::creature().you_control().modified();
        let creature = game.object(creature_id).expect("creature exists");
        assert!(
            !filter.matches(creature, &ctx, &game),
            "Aura controlled by opponent should not make creature modified"
        );
    }

    #[test]
    fn test_filter_matches_permanent_attached_to_player() {
        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let aura_id = create_modified_test_aura(&mut game, bob);

        game.object_mut(aura_id).expect("aura exists").attached_to =
            Some(crate::object::AttachmentTarget::Player(alice));

        let mut filter = ObjectFilter::permanent().with_subtype(Subtype::Aura);
        filter.attached_to_player = Some(PlayerFilter::You);

        let aura = game.object(aura_id).expect("aura exists");
        assert!(
            filter.matches(aura, &FilterContext::new(alice), &game),
            "Aura attached to Alice should match attached_to_player=You from Alice's context"
        );
        assert!(
            !filter.matches(aura, &FilterContext::new(bob), &game),
            "Aura attached to Alice should not match attached_to_player=You from Bob's context"
        );
    }

    #[test]
    fn different_name_from_tagged_excludes_all_tagged_names() {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::snapshot::ObjectSnapshot;

        let mut game = setup_modified_filter_game();
        let alice = PlayerId::from_index(0);
        let tag = TagKey::from("attached_curses");

        let attached_misfortunes = CardBuilder::new(CardId::from_raw(10), "Curse of Misfortunes")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura, Subtype::Curse])
            .build();
        let attached_thirst = CardBuilder::new(CardId::from_raw(11), "Curse of Thirst")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura, Subtype::Curse])
            .build();
        let candidate_same = CardBuilder::new(CardId::from_raw(12), "Curse of Misfortunes")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura, Subtype::Curse])
            .build();
        let candidate_different = CardBuilder::new(CardId::from_raw(13), "Curse of Death's Hold")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura, Subtype::Curse])
            .build();

        let attached_misfortunes_id =
            game.create_object_from_card(&attached_misfortunes, alice, Zone::Battlefield);
        let attached_thirst_id =
            game.create_object_from_card(&attached_thirst, alice, Zone::Battlefield);
        let candidate_same_id = game.create_object_from_card(&candidate_same, alice, Zone::Library);
        let candidate_different_id =
            game.create_object_from_card(&candidate_different, alice, Zone::Library);

        let mut ctx = FilterContext::new(alice);
        ctx.tagged_objects.insert(
            tag.clone(),
            vec![
                ObjectSnapshot::from_object(
                    game.object(attached_misfortunes_id)
                        .expect("attached object"),
                    &game,
                ),
                ObjectSnapshot::from_object(
                    game.object(attached_thirst_id).expect("attached object"),
                    &game,
                ),
            ],
        );

        let mut filter = ObjectFilter::default().with_subtype(Subtype::Curse);
        filter.zone = Some(Zone::Library);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::DifferentNameFromTagged,
        });

        assert!(
            !filter.matches(
                game.object(candidate_same_id).expect("same-name candidate"),
                &ctx,
                &game
            ),
            "candidate sharing any tagged Curse name should not match"
        );
        assert!(
            filter.matches(
                game.object(candidate_different_id)
                    .expect("different-name candidate"),
                &ctx,
                &game
            ),
            "candidate with a name different from every tagged Curse should match"
        );
    }

    #[test]
    fn test_player_filter_matching() {
        let you = PlayerId::from_index(0);
        let opponent = PlayerId::from_index(1);

        let ctx = FilterContext::new(you).with_opponents(vec![opponent]);

        assert!(PlayerFilter::Any.matches_player(you, &ctx));
        assert!(PlayerFilter::Any.matches_player(opponent, &ctx));

        assert!(PlayerFilter::You.matches_player(you, &ctx));
        assert!(!PlayerFilter::You.matches_player(opponent, &ctx));

        assert!(!PlayerFilter::Opponent.matches_player(you, &ctx));
        assert!(PlayerFilter::Opponent.matches_player(opponent, &ctx));

        assert!(PlayerFilter::Specific(you).matches_player(you, &ctx));
        assert!(!PlayerFilter::Specific(you).matches_player(opponent, &ctx));
    }

    #[test]
    fn test_player_filter_controller_of_target_uses_target_snapshot() {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::snapshot::ObjectSnapshot;

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = crate::tests::test_helpers::setup_two_player_game();

        let land = CardBuilder::new(CardId::from_raw(1001), "Target Forest")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build();
        let land_id = game.create_object_from_card(&land, bob, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(land_id).expect("target land exists"), &game);

        let ctx = FilterContext::new(alice).with_target_objects(vec![snapshot]);
        let controller_filter = PlayerFilter::ControllerOf(ObjectRef::Target);
        let owner_filter = PlayerFilter::OwnerOf(ObjectRef::Target);

        assert!(controller_filter.matches_player(bob, &ctx));
        assert!(!controller_filter.matches_player(alice, &ctx));
        assert!(owner_filter.matches_player(bob, &ctx));
        assert!(!owner_filter.matches_player(alice, &ctx));
    }

    #[test]
    fn test_excluded_supertypes_builder() {
        use crate::types::Supertype;

        let filter = ObjectFilter::land().without_supertype(Supertype::Basic);
        assert_eq!(filter.excluded_supertypes, vec![Supertype::Basic]);
    }

    #[test]
    fn test_nonbasic_shorthand() {
        use crate::types::Supertype;

        let filter = ObjectFilter::land().nonbasic();
        assert_eq!(filter.excluded_supertypes, vec![Supertype::Basic]);
    }

    #[test]
    fn test_excluded_supertypes_matching() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::object::Object;
        use crate::types::Supertype;

        let p0 = PlayerId::from_index(0);

        // Create a basic land
        let basic_forest_card = CardBuilder::new(CardId::from_raw(1), "Forest")
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Basic])
            .subtypes(vec![crate::types::Subtype::Forest])
            .build();
        let basic_forest = Object::from_card(
            crate::ids::ObjectId::from_raw(1),
            &basic_forest_card,
            p0,
            Zone::Battlefield,
        );

        // Create a nonbasic land
        let nonbasic_land_card = CardBuilder::new(CardId::from_raw(2), "Steam Vents")
            .card_types(vec![CardType::Land])
            .subtypes(vec![
                crate::types::Subtype::Island,
                crate::types::Subtype::Mountain,
            ])
            .build();
        let nonbasic_land = Object::from_card(
            crate::ids::ObjectId::from_raw(2),
            &nonbasic_land_card,
            p0,
            Zone::Battlefield,
        );

        // Filter for nonbasic lands (excludes Basic supertype)
        let nonbasic_filter = ObjectFilter::land().nonbasic();
        let ctx = FilterContext::new(p0);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        // Basic land should NOT match (has Basic supertype which is excluded)
        assert!(
            !nonbasic_filter.matches(&basic_forest, &ctx, &game),
            "Basic Forest should not match nonbasic filter"
        );

        // Nonbasic land SHOULD match (doesn't have Basic supertype)
        assert!(
            nonbasic_filter.matches(&nonbasic_land, &ctx, &game),
            "Steam Vents should match nonbasic filter"
        );
    }

    #[test]
    fn test_blood_moon_filter_for_nonbasic_lands() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::types::Supertype;

        let p0 = PlayerId::from_index(0);

        // Blood Moon filter: nonbasic lands on the battlefield
        let blood_moon_filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            excluded_supertypes: vec![Supertype::Basic],
            ..Default::default()
        };

        // Create basic Plains
        let plains_card = CardBuilder::new(CardId::from_raw(1), "Plains")
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Basic])
            .subtypes(vec![crate::types::Subtype::Plains])
            .build();
        let plains = Object::from_card(
            crate::ids::ObjectId::from_raw(1),
            &plains_card,
            p0,
            Zone::Battlefield,
        );

        // Create Breeding Pool (nonbasic)
        let breeding_pool_card = CardBuilder::new(CardId::from_raw(2), "Breeding Pool")
            .card_types(vec![CardType::Land])
            .subtypes(vec![
                crate::types::Subtype::Forest,
                crate::types::Subtype::Island,
            ])
            .build();
        let breeding_pool = Object::from_card(
            crate::ids::ObjectId::from_raw(2),
            &breeding_pool_card,
            p0,
            Zone::Battlefield,
        );

        let ctx = FilterContext::new(p0);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        // Blood Moon should NOT affect basic Plains
        assert!(
            !blood_moon_filter.matches(&plains, &ctx, &game),
            "Blood Moon filter should not match basic Plains"
        );

        // Blood Moon SHOULD affect Breeding Pool
        assert!(
            blood_moon_filter.matches(&breeding_pool, &ctx, &game),
            "Blood Moon filter should match Breeding Pool"
        );
    }

    #[test]
    fn test_commander_filter_matches_true_commander_regardless_of_ctx_owner_list() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::{CardId, ObjectId};
        use crate::object::Object;

        let you = PlayerId::from_index(0);
        let opponent = PlayerId::from_index(1);

        let commander_card = CardBuilder::new(CardId::from_raw(99), "Opponent Commander")
            .card_types(vec![CardType::Creature])
            .build();
        let commander_obj = Object::from_card(
            ObjectId::from_raw(99),
            &commander_card,
            opponent,
            Zone::Battlefield,
        );

        let mut game = GameState::new(vec!["You".to_string(), "Opponent".to_string()], 20);
        game.add_object(commander_obj.clone());
        game.set_as_commander(commander_obj.id, opponent);

        let filter = ObjectFilter::creature().commander();
        let ctx = FilterContext::new(you).with_your_commanders(Vec::new());
        assert!(
            filter.matches(&commander_obj, &ctx, &game),
            "commander filter should rely on game commander identity, not ctx.your_commanders"
        );
    }

    #[test]
    fn test_historic_and_nonhistoric_filters_match_correctly() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::{CardId, ObjectId};
        use crate::object::Object;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let artifact_card = CardBuilder::new(CardId::from_raw(1), "Mox")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_obj = Object::from_card(
            ObjectId::from_raw(1),
            &artifact_card,
            you,
            Zone::Battlefield,
        );
        game.add_object(artifact_obj.clone());

        let creature_card = CardBuilder::new(CardId::from_raw(2), "Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_obj = Object::from_card(
            ObjectId::from_raw(2),
            &creature_card,
            you,
            Zone::Battlefield,
        );
        game.add_object(creature_obj.clone());

        let ctx = FilterContext::new(you);
        assert!(
            ObjectFilter::permanent()
                .historic()
                .matches(&artifact_obj, &ctx, &game)
        );
        assert!(
            !ObjectFilter::permanent()
                .historic()
                .matches(&creature_obj, &ctx, &game)
        );
        assert!(
            ObjectFilter::permanent()
                .nonhistoric()
                .matches(&creature_obj, &ctx, &game)
        );
        assert!(
            !ObjectFilter::permanent()
                .nonhistoric()
                .matches(&artifact_obj, &ctx, &game)
        );
    }

    #[test]
    fn test_shares_color_with_tagged_constraint() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::{CardId, ObjectId};
        use crate::object::Object;
        use crate::snapshot::ObjectSnapshot;
        use crate::tag::TagKey;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let red_card = CardBuilder::new(CardId::from_raw(10), "Red Creature")
            .card_types(vec![CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Red,
            ]]))
            .build();
        let red_obj = Object::from_card(ObjectId::from_raw(10), &red_card, you, Zone::Battlefield);
        game.add_object(red_obj.clone());

        let blue_card = CardBuilder::new(CardId::from_raw(11), "Blue Creature")
            .card_types(vec![CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let blue_obj =
            Object::from_card(ObjectId::from_raw(11), &blue_card, you, Zone::Battlefield);
        game.add_object(blue_obj.clone());

        let mut tagged = std::collections::HashMap::new();
        tagged.insert(
            TagKey::from("it"),
            vec![ObjectSnapshot::from_object(&red_obj, &game)],
        );
        let ctx = FilterContext::new(you).with_tagged_objects(&tagged);
        let filter = ObjectFilter::creature().shares_color_with_tagged("it");

        assert!(filter.matches(&red_obj, &ctx, &game));
        assert!(!filter.matches(&blue_obj, &ctx, &game));
    }

    #[test]
    fn test_base_power_builder_sets_reference() {
        let filter = ObjectFilter::creature().with_base_power(Comparison::LessThanOrEqual(2));
        assert_eq!(filter.power, Some(Comparison::LessThanOrEqual(2)));
        assert_eq!(filter.power_reference, PtReference::Base);
        assert_eq!(filter.description(), "creature with base power 2 or less");
    }

    #[test]
    fn test_filter_can_match_base_vs_effective_power() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::object::CounterType;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let card = CardBuilder::new(CardId::from_raw(30), "Counter Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object_id = game.create_object_from_card(&card, you, Zone::Battlefield);
        if let Some(obj) = game.object_mut(object_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        }

        let obj = game.object(object_id).expect("object should exist");
        let ctx = FilterContext::new(you);

        let effective_filter =
            ObjectFilter::creature().with_power(Comparison::GreaterThanOrEqual(3));
        let base_filter =
            ObjectFilter::creature().with_base_power(Comparison::GreaterThanOrEqual(3));

        assert!(
            effective_filter.matches(obj, &ctx, &game),
            "effective power should include +1/+1 counters"
        );
        assert!(
            !base_filter.matches(obj, &ctx, &game),
            "base power should ignore +1/+1 counters"
        );
    }

    #[test]
    fn test_non_recursive_match_avoids_calculated_power() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::static_abilities::StaticAbility;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let card = CardBuilder::new(CardId::from_raw(31), "Anthem Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object_id = game.create_object_from_card(&card, you, Zone::Battlefield);
        if let Some(obj) = game.object_mut(object_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::anthem(
                    ObjectFilter::source(),
                    2,
                    0,
                )));
        }

        let obj = game.object(object_id).expect("object should exist");
        let ctx = FilterContext::new(you);
        let filter = ObjectFilter::creature().with_power(Comparison::GreaterThanOrEqual(4));

        assert!(
            filter.matches(obj, &ctx, &game),
            "regular matching should use calculated power"
        );
        assert!(
            !filter.matches_non_recursive(obj, &ctx, &game),
            "non-recursive matching should avoid layer-calculated power"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_filter_matches_earthbent_land_as_creature() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::cards::definitions::basic_mountain;
        use crate::effect::Effect;
        use crate::effects::EarthbendEffect;
        use crate::effects::{ExecutionContext, execute_effect};
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::target::ChooseSpec;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let source_card = CardBuilder::new(CardId::from_raw(32), "Earthbend Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source_id = game.create_object_from_card(&source_card, you, Zone::Battlefield);
        let land_id = game.create_object_from_definition(&basic_mountain(), you, Zone::Battlefield);

        let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
        let mut exec_ctx = ExecutionContext::new_default(source_id, you);
        execute_effect(&mut game, &effect, &mut exec_ctx).expect("earthbend should resolve");

        let filter_ctx = FilterContext::new(you).with_source(source_id);
        let land = game.object(land_id).expect("earthbent land should exist");

        assert!(
            ObjectFilter::creature().matches(land, &filter_ctx, &game),
            "calculated creature type should make the animated land match creature filters"
        );
        assert!(
            !ObjectFilter::creature().matches_non_recursive(land, &filter_ctx, &game),
            "non-recursive matching should keep using base types for layer calculations"
        );
    }

    #[test]
    fn test_filter_matches_creature_dealt_damage_this_turn() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::game_state::GameState;
        use crate::ids::CardId;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let card = CardBuilder::new(CardId::from_raw(40), "Damaged Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&card, you, Zone::Battlefield);
        let ctx = FilterContext::new(you);

        let mut filter = ObjectFilter::creature();
        filter.was_dealt_damage_this_turn = true;

        let creature = game.object(creature_id).expect("creature should exist");
        assert!(!filter.matches(creature, &ctx, &game));

        let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent {
                source: ObjectId::from_raw(500),
                target: crate::events::DamageTarget::Object(creature_id),
                amount: 1,
                is_combat: false,
                is_unpreventable: false,
                cause: crate::events::cause::EventCause::effect(),
                remainder: None,
                target_snapshot: None,
            },
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&damage_event);
        let creature = game.object(creature_id).expect("creature should exist");
        assert!(filter.matches(creature, &ctx, &game));
    }
}
