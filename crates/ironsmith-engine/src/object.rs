use crate::filter::ObjectFilterExt as _;
use std::collections::HashMap;
use std::sync::Arc;

use crate::ability::Ability;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::{Card, LinkedFaceLayout, PowerToughness, PtValue};
use crate::color::{Color, ColorSet};
use crate::cost::{OptionalCost, OptionalCostsPaid, TotalCost};
use crate::filter::PlayerFilterExt;
use crate::ids::{CardId, ObjectId, PlayerId, StableId};
use crate::mana::ManaCost;
use crate::player::ManaPool;
use crate::snapshot::{CopiableValues, ObjectSnapshot};
use crate::static_abilities::{StaticAbility, StaticAbilityId};
use crate::tag::TagKey;
use crate::target::FilterContext;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
pub use ironsmith_core::CounterType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SharedStr(Arc<str>);

impl From<String> for SharedStr {
    fn from(value: String) -> Self {
        Self(Arc::from(value.into_boxed_str()))
    }
}

impl From<&str> for SharedStr {
    fn from(value: &str) -> Self {
        Self(Arc::from(value))
    }
}

impl From<Arc<str>> for SharedStr {
    fn from(value: Arc<str>) -> Self {
        Self(value)
    }
}

impl From<SharedStr> for String {
    fn from(value: SharedStr) -> Self {
        value.to_owned_string()
    }
}

impl From<&SharedStr> for String {
    fn from(value: &SharedStr) -> Self {
        value.to_owned_string()
    }
}

impl std::ops::Deref for SharedStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for SharedStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for SharedStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for SharedStr {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for SharedStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<SharedStr> for &str {
    fn eq(&self, other: &SharedStr) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<SharedStr> for String {
    fn eq(&self, other: &SharedStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl SharedStr {
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn to_owned_string(&self) -> String {
        self.as_str().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedVec<T>(Arc<Vec<T>>);

impl<T> Default for SharedVec<T> {
    fn default() -> Self {
        Self(Arc::new(Vec::new()))
    }
}

impl<T> From<Vec<T>> for SharedVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> From<Arc<Vec<T>>> for SharedVec<T> {
    fn from(value: Arc<Vec<T>>) -> Self {
        Self(value)
    }
}

impl<T: Clone> From<SharedVec<T>> for Vec<T> {
    fn from(value: SharedVec<T>) -> Self {
        value.to_vec()
    }
}

impl<T> std::iter::FromIterator<T> for SharedVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}

impl<T> std::ops::Deref for SharedVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> std::ops::DerefMut for SharedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<'a, T> IntoIterator for &'a SharedVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a mut SharedVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::make_mut(&mut self.0).iter_mut()
    }
}

impl<T: Clone> IntoIterator for SharedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.to_vec().into_iter()
    }
}

impl<T> SharedVec<T> {
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Clone of the backing `Arc` without copying the elements.
    pub fn shared(&self) -> Arc<Vec<T>> {
        Arc::clone(&self.0)
    }
}

impl<T: Clone> SharedVec<T> {
    pub fn to_vec(&self) -> Vec<T> {
        self.0.as_ref().clone()
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for SharedVec<T> {
    fn eq(&self, other: &[T; N]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq> PartialEq<[T]> for SharedVec<T> {
    fn eq(&self, other: &[T]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for SharedVec<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedValue<T>(Arc<T>);

impl<T> From<T> for SharedValue<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> std::ops::Deref for SharedValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> SharedValue<T> {
    pub fn to_owned_value(&self) -> T {
        self.0.as_ref().clone()
    }
}

fn shared_optional_value<T>(value: Option<T>) -> Option<SharedValue<T>> {
    value.map(SharedValue::from)
}

fn owned_optional_value<T: Clone>(value: &Option<SharedValue<T>>) -> Option<T> {
    value.as_ref().map(SharedValue::to_owned_value)
}

#[derive(Debug, Clone)]
pub(crate) struct CardSharedHandles {
    name: SharedStr,
    first_printed_set_name: Option<SharedStr>,
    mana_cost: Option<SharedValue<ManaCost>>,
    supertypes: SharedVec<Supertype>,
    card_types: SharedVec<CardType>,
    subtypes: SharedVec<Subtype>,
    compiled_card_text: Arc<str>,
    other_face_name: Option<SharedStr>,
    abilities: Arc<Vec<Ability>>,
    spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
    aura_attach_filter: Option<SharedValue<AuraAttachmentFilter>>,
    alternative_casts: SharedVec<AlternativeCastingMethod>,
    optional_costs: SharedVec<OptionalCost>,
    additional_cost: SharedValue<TotalCost>,
}

impl CardSharedHandles {
    pub(crate) fn from_definition(def: &crate::cards::CardDefinition) -> Self {
        Self {
            name: def.card.name.clone().into(),
            first_printed_set_name: def.card.first_printed_set_name.clone().map(Into::into),
            mana_cost: shared_optional_value(def.card.mana_cost.clone()),
            supertypes: def.card.supertypes.clone().into(),
            card_types: def.card.card_types.clone().into(),
            subtypes: def.card.subtypes.clone().into(),
            compiled_card_text: Object::compiled_display_text(def),
            other_face_name: def.card.other_face_name.clone().map(Into::into),
            abilities: Arc::new(def.abilities.clone()),
            spell_effect: shared_optional_value(def.spell_effect.clone()),
            aura_attach_filter: shared_optional_value(def.aura_attach_filter.clone()),
            alternative_casts: def.alternative_casts.clone().into(),
            optional_costs: def.optional_costs.clone().into(),
            additional_cost: def.additional_cost.clone().into(),
        }
    }
}

/// The kind of game object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// A physical card
    Card,
    /// A token permanent
    Token,
    /// A copy of a spell on the stack
    SpellCopy,
    /// An emblem (from planeswalker ultimates)
    Emblem,
}

impl ObjectKind {
    pub fn name(self) -> &'static str {
        match self {
            ObjectKind::Card => "card",
            ObjectKind::Token => "token",
            ObjectKind::SpellCopy => "spell copy",
            ObjectKind::Emblem => "emblem",
        }
    }
}

impl std::fmt::Display for ObjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A legal thing an attachment can be attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentTarget {
    Object(ObjectId),
    Player(PlayerId),
}

impl AttachmentTarget {
    pub fn object_id(self) -> Option<ObjectId> {
        match self {
            Self::Object(id) => Some(id),
            Self::Player(_) => None,
        }
    }

    pub fn player_id(self) -> Option<PlayerId> {
        match self {
            Self::Object(_) => None,
            Self::Player(id) => Some(id),
        }
    }
}

pub use ironsmith_core::AuraAttachmentFilter;

pub trait AuraAttachmentFilterRuntimeExt {
    fn matches_target(
        &self,
        target: AttachmentTarget,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool;
}

impl AuraAttachmentFilterRuntimeExt for AuraAttachmentFilter {
    fn matches_target(
        &self,
        target: AttachmentTarget,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        match (self, target) {
            (Self::Object(filter), AttachmentTarget::Object(id)) => game
                .object(id)
                .is_some_and(|object| filter.matches(object, ctx, game)),
            (Self::Player(filter), AttachmentTarget::Player(id)) => filter.matches_player(id, ctx),
            _ => false,
        }
    }
}

/// Stored copiable fields needed to end a bestow cast and restore creature form.
#[derive(Debug, Clone)]
pub struct BestowCastState {
    pub card_types: SharedVec<CardType>,
    pub subtypes: SharedVec<Subtype>,
    pub aura_attach_filter: Option<SharedValue<AuraAttachmentFilter>>,
    pub spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
}

/// Original spell program restored when a spell modified by splice leaves the stack.
///
/// Splice is a text-changing effect on the spell, not a change to the physical
/// card's copiable values outside the stack (CR 702.47c, 702.47e). Keeping the
/// pre-splice program on the object also lets spell copies inherit the active
/// overlay and then shed it through the ordinary stack-to-zone transition.
#[derive(Debug, Clone)]
pub struct SpliceCastState {
    pub spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
}

/// Stored copiable fields needed to restore a card after a face-down cast.
#[derive(Debug, Clone)]
pub struct FaceDownCastState {
    pub name: SharedStr,
    pub first_printed_set_name: Option<SharedStr>,
    pub mana_cost: Option<SharedValue<ManaCost>>,
    pub color_override: Option<ColorSet>,
    pub supertypes: SharedVec<Supertype>,
    pub card_types: SharedVec<CardType>,
    pub subtypes: SharedVec<Subtype>,
    pub compiled_card_text: Arc<str>,
    pub rules_text_color_identity: ColorSet,
    pub base_power: Option<PtValue>,
    pub base_toughness: Option<PtValue>,
    pub base_loyalty: Option<u32>,
    pub base_defense: Option<u32>,
    pub abilities: Arc<Vec<Ability>>,
    pub spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
    pub aura_attach_filter: Option<SharedValue<AuraAttachmentFilter>>,
}

/// Stored copiable fields needed to restore a prototype card outside the stack
/// or battlefield.
#[derive(Debug, Clone)]
pub struct PrototypeCastState {
    pub mana_cost: Option<SharedValue<ManaCost>>,
    pub color_override: Option<ColorSet>,
    pub base_power: Option<PtValue>,
    pub base_toughness: Option<PtValue>,
}

/// Runtime representation of a game object.
/// Contains both copiable values (layer 1) and non-copiable state.
#[derive(Debug, Clone)]
pub struct Object {
    // Identity
    pub id: ObjectId,
    /// Stable identifier that persists across zone changes.
    /// Unlike `id` which changes when an object moves zones (per MTG rule 400.7),
    /// `stable_id` stays constant for the lifetime of this card/token instance.
    /// Useful for tracking "this specific card" for display and triggered abilities.
    pub stable_id: StableId,
    /// Game-local mutation revision stamped by `GameState::object_mut`.
    ///
    /// This is clone/rollback state, not an id source and not a serialization surface.
    pub last_modified: u64,
    pub kind: ObjectKind,
    /// Reference to the original card definition (None for pure tokens)
    pub card: Option<CardId>,
    pub zone: Zone,

    // Ownership (normally immutable; CR 407.2 changes ownership at the end of
    // a game played for ante)
    pub owner: PlayerId,

    // Copiable values (what Clone effects copy)
    pub name: SharedStr,
    /// Earliest eligible paper set for the oracle identity represented by the
    /// current copiable name, when registry metadata is available.
    pub first_printed_set_name: Option<SharedStr>,
    pub mana_cost: Option<SharedValue<ManaCost>>,
    pub color_override: Option<ColorSet>,
    pub supertypes: SharedVec<Supertype>,
    pub card_types: SharedVec<CardType>,
    pub subtypes: SharedVec<Subtype>,
    pub compiled_card_text: Arc<str>,
    pub rules_text_color_identity: ColorSet,
    /// Optional reference to another face for flip/DFC style cards.
    ///
    /// This is copied from `Card::other_face` when the object is created.
    pub other_face: Option<CardId>,
    /// Linked face name for on-demand compilation without a global registry preload.
    pub other_face_name: Option<SharedStr>,
    /// Layout semantics for linked-face cards.
    pub linked_face_layout: LinkedFaceLayout,
    pub base_power: Option<PtValue>,
    pub base_toughness: Option<PtValue>,
    pub base_loyalty: Option<u32>,
    pub base_defense: Option<u32>,
    /// Copiable printed Vanguard hand modifier.
    pub hand_modifier: i32,
    /// Copiable printed Vanguard life modifier.
    pub life_modifier: i32,
    /// Abilities this object has (copiable)
    pub abilities: Arc<Vec<Ability>>,

    // Non-copiable values (kept on Object)
    pub counters: HashMap<CounterType, u32>,
    pub attached_to: Option<AttachmentTarget>,
    pub attachments: Vec<ObjectId>,

    // Spell-related state
    /// Spell effects (for instants/sorceries)
    pub spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
    /// Pre-splice program while this object is a spell on the stack.
    pub splice_cast_state: Option<Box<SpliceCastState>>,
    /// For Auras: what this card can enchant (used for non-target attachments)
    pub aura_attach_filter: Option<SharedValue<AuraAttachmentFilter>>,
    /// Original copiable fields to restore if this permanent ends bestow.
    pub bestow_cast_state: Option<Box<BestowCastState>>,
    /// Original copiable fields to restore if this card was cast face down.
    pub face_down_cast_state: Option<Box<FaceDownCastState>>,
    /// Original copiable fields to restore if this card was cast prototyped.
    pub prototype_cast_state: Option<PrototypeCastState>,
    /// Alternative casting methods (flashback, escape, etc.)
    pub alternative_casts: SharedVec<AlternativeCastingMethod>,
    /// Alternative method chosen for the current spell cast.
    pub cast_alternative_method: Option<Box<AlternativeCastingMethod>>,
    /// True if this split card can be cast fused from hand.
    pub has_fuse: bool,
    /// Optional costs (kicker, buyback, etc.)
    pub optional_costs: SharedVec<OptionalCost>,
    /// Which optional costs were paid when this spell was cast (for ETB triggers)
    pub optional_costs_paid: OptionalCostsPaid,
    /// Mana actually spent to cast this object while it was a spell.
    /// Used by conditional text like "if at least three blue mana was spent to cast this spell".
    pub mana_spent_to_cast: ManaPool,
    /// Non-copiable static abilities granted until end of turn while this object is a spell or
    /// permanent. Stack-to-battlefield movement preserves these grants for the permanent that
    /// spell becomes; other zone changes clear them.
    pub temporary_static_ability_grants: Vec<TemporaryStaticAbilityGrant>,
    /// X value chosen for this object when it was cast (if any).
    /// Used by ETB and other triggered abilities that reference X from the mana cost.
    pub x_value: Option<u32>,
    /// Permanents that contributed keyword-ability alternative payments while casting this object
    /// as a spell (e.g., Convoke/Improvise). Used by later resolution-time references like
    /// "each creature that convoked it".
    pub keyword_payment_contributions_to_cast: Vec<crate::decision::KeywordPaymentContribution>,
    /// Object snapshots captured while paying costs for this spell cast.
    ///
    /// This lets replacement/trigger text on the resolving permanent reference cards or permanents
    /// used to pay costs, such as "the discarded card's mana value".
    pub cast_tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    /// Additional non-printed costs paid while casting this object as a spell.
    pub additional_cost: SharedValue<TotalCost>,
    // Note: The following fields have been moved to GameState extension maps:
    // - tapped -> GameState::tapped_permanents
    // - flipped -> GameState::flipped
    // - face_down -> GameState::face_down
    // - phased_out -> GameState::phased_out
    // - damage_marked -> GameState::damage_marked
    // - summoning_sick -> GameState::summoning_sick
    // - is_monstrous -> GameState::monstrous
    // - regeneration_shields -> GameState::regeneration_shields
    // - madness_exiled -> GameState::madness_exiled
    // - is_commander -> GameState::commanders
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemporaryStaticAbilityGrant {
    pub ability: StaticAbilityId,
    pub ability_payload: Option<StaticAbility>,
    pub expires_end_of_turn: u32,
}

impl TemporaryStaticAbilityGrant {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        current_turn > self.expires_end_of_turn
    }

    pub fn materialize(&self) -> Option<StaticAbility> {
        self.ability_payload
            .clone()
            .or_else(|| static_ability_from_id(self.ability))
    }
}

fn static_ability_from_id(ability: StaticAbilityId) -> Option<StaticAbility> {
    match ability {
        StaticAbilityId::Deathtouch => Some(StaticAbility::deathtouch()),
        StaticAbilityId::DoubleStrike => Some(StaticAbility::double_strike()),
        StaticAbilityId::FirstStrike => Some(StaticAbility::first_strike()),
        StaticAbilityId::Flying => Some(StaticAbility::flying()),
        StaticAbilityId::Haste => Some(StaticAbility::haste()),
        StaticAbilityId::Hexproof => Some(StaticAbility::hexproof()),
        StaticAbilityId::Indestructible => Some(StaticAbility::indestructible()),
        StaticAbilityId::Lifelink => Some(StaticAbility::lifelink()),
        StaticAbilityId::Menace => Some(StaticAbility::menace()),
        StaticAbilityId::Reach => Some(StaticAbility::reach()),
        StaticAbilityId::Trample => Some(StaticAbility::trample()),
        StaticAbilityId::Vigilance => Some(StaticAbility::vigilance()),
        StaticAbilityId::ReadAhead => Some(StaticAbility::read_ahead()),
        _ => None,
    }
}

impl Object {
    /// Returns a mutable view of this object's copiable abilities.
    ///
    /// Abilities are shared across object clones and repeated definitions, so live
    /// object mutations must pass through Arc COW to preserve value semantics.
    pub fn abilities_mut(&mut self) -> &mut Vec<Ability> {
        Arc::make_mut(&mut self.abilities)
    }

    pub fn abilities_vec(&self) -> Vec<Ability> {
        self.abilities.as_ref().clone()
    }

    fn compiled_display_text(def: &crate::cards::CardDefinition) -> Arc<str> {
        Arc::from(crate::runtime_display::compiled_text_lines(def).join("\n"))
    }

    fn extend_unique<T: PartialEq + Clone>(base: &mut Vec<T>, extra: &[T]) {
        for item in extra {
            if !base.contains(item) {
                base.push(item.clone());
            }
        }
    }

    /// Returns non-mana additional cost components for this object.
    pub fn additional_non_mana_costs(&self) -> Vec<crate::costs::Cost> {
        self.additional_cost.non_mana_costs().cloned().collect()
    }

    pub fn mana_cost_owned(&self) -> Option<ManaCost> {
        owned_optional_value(&self.mana_cost)
    }

    pub fn spell_effect_owned(&self) -> Option<crate::resolution::ResolutionProgram> {
        owned_optional_value(&self.spell_effect)
    }

    pub fn aura_attach_filter_owned(&self) -> Option<AuraAttachmentFilter> {
        owned_optional_value(&self.aura_attach_filter)
    }

    pub fn cast_alternative_method_owned(&self) -> Option<AlternativeCastingMethod> {
        self.cast_alternative_method
            .as_ref()
            .map(|method| method.as_ref().clone())
    }

    /// Creates a new object from a card definition.
    pub fn from_card(id: ObjectId, card: &Card, owner: PlayerId, zone: Zone) -> Self {
        let (base_power, base_toughness) = card
            .power_toughness
            .map(|pt| (Some(pt.power), Some(pt.toughness)))
            .unwrap_or((None, None));
        let is_token = card.is_token;

        Self {
            id,
            stable_id: StableId::from(id), // Set to same as id initially; preserved across zone changes
            last_modified: 0,
            kind: if is_token {
                ObjectKind::Token
            } else {
                ObjectKind::Card
            },
            card: (!is_token).then_some(card.id),
            zone,
            owner,
            name: card.name.clone().into(),
            first_printed_set_name: card.first_printed_set_name.clone().map(Into::into),
            // Tokens are not cards and have no mana cost, even if a reusable
            // token template accidentally carries a card-like cost.
            mana_cost: if is_token {
                None
            } else {
                shared_optional_value(card.mana_cost.clone())
            },
            color_override: card.color_indicator,
            supertypes: card.supertypes.clone().into(),
            card_types: card.card_types.clone().into(),
            subtypes: card.subtypes.clone().into(),
            compiled_card_text: Arc::from(""),
            rules_text_color_identity: card.rules_text_color_identity,
            other_face: card.other_face,
            other_face_name: card.other_face_name.clone().map(Into::into),
            linked_face_layout: card.linked_face_layout,
            base_power,
            base_toughness,
            base_loyalty: card.loyalty,
            base_defense: card.defense,
            hand_modifier: card.hand_modifier,
            life_modifier: card.life_modifier,
            abilities: Arc::new(Vec::new()),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: Vec::new(),
            x_value: None,
            keyword_payment_contributions_to_cast: Vec::new(),
            cast_tagged_objects: HashMap::new(),
            additional_cost: TotalCost::free().into(),
        }
    }

    /// Creates a new object from a CardDefinition (card + abilities + spell effects).
    pub fn from_card_definition(
        id: ObjectId,
        def: &crate::cards::CardDefinition,
        owner: PlayerId,
        zone: Zone,
    ) -> Self {
        let handles = CardSharedHandles::from_definition(def);
        Self::from_card_definition_with_shared(id, def, owner, zone, &handles)
    }

    pub(crate) fn from_card_definition_with_shared(
        id: ObjectId,
        def: &crate::cards::CardDefinition,
        owner: PlayerId,
        zone: Zone,
        handles: &CardSharedHandles,
    ) -> Self {
        let mut obj = Self::from_card(id, &def.card, owner, zone);
        obj.apply_card_definition_with_shared(def, handles);
        obj
    }

    /// Creates a hidden physical card placeholder for cryptographic deck custody.
    ///
    /// The object can move through hidden zones before its printed identity is
    /// opened. A verified reveal should call `apply_card_definition` on the same
    /// object instance rather than replacing zone membership by hand.
    pub fn new_hidden_card(id: ObjectId, owner: PlayerId, zone: Zone) -> Self {
        Self {
            id,
            stable_id: StableId::from(id),
            last_modified: 0,
            kind: ObjectKind::Card,
            card: None,
            zone,
            owner,
            name: "Hidden Card".into(),
            first_printed_set_name: None,
            mana_cost: None,
            color_override: None,
            supertypes: Vec::new().into(),
            card_types: Vec::new().into(),
            subtypes: Vec::new().into(),
            compiled_card_text: Arc::from(""),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            base_power: None,
            base_toughness: None,
            base_loyalty: None,
            base_defense: None,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: Arc::new(Vec::new()),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: Vec::new(),
            x_value: None,
            keyword_payment_contributions_to_cast: Vec::new(),
            cast_tagged_objects: HashMap::new(),
            additional_cost: TotalCost::free().into(),
        }
    }

    pub fn redact_to_hidden_card(&mut self) {
        let id = self.id;
        let stable_id = self.stable_id;
        let owner = self.owner;
        let zone = self.zone;
        *self = Self::new_hidden_card(id, owner, zone);
        self.stable_id = stable_id;
    }

    pub fn apply_card_definition(&mut self, def: &crate::cards::CardDefinition) {
        let handles = CardSharedHandles::from_definition(def);
        self.apply_card_definition_with_shared(def, &handles);
    }

    pub(crate) fn apply_card_definition_with_shared(
        &mut self,
        def: &crate::cards::CardDefinition,
        handles: &CardSharedHandles,
    ) {
        let is_token = def.card.is_token;
        self.kind = if is_token {
            ObjectKind::Token
        } else {
            ObjectKind::Card
        };
        self.card = (!is_token).then_some(def.card.id);
        self.apply_definition_face_with_shared(def, handles);
        if is_token {
            self.mana_cost = None;
        }
        self.spell_effect = handles.spell_effect.clone();
        self.aura_attach_filter = handles.aura_attach_filter.clone();
        self.alternative_casts = handles.alternative_casts.clone();
        self.has_fuse = def.has_fuse;
        self.optional_costs = handles.optional_costs.clone();
        self.additional_cost = handles.additional_cost.clone();
    }

    /// Apply the printed/copied characteristics of another card definition.
    ///
    /// Used for flip cards and similar "becomes this other face" mechanics.
    /// This preserves identity, ownership, controller, zone, counters, and attachments.
    pub fn apply_definition_face(&mut self, def: &crate::cards::CardDefinition) {
        let handles = CardSharedHandles::from_definition(def);
        self.apply_definition_face_with_shared(def, &handles);
    }

    pub(crate) fn apply_definition_face_with_shared(
        &mut self,
        def: &crate::cards::CardDefinition,
        handles: &CardSharedHandles,
    ) {
        let (base_power, base_toughness) = def
            .card
            .power_toughness
            .map(|pt| (Some(pt.power), Some(pt.toughness)))
            .unwrap_or((None, None));

        self.name = handles.name.clone();
        self.first_printed_set_name = handles.first_printed_set_name.clone();
        self.mana_cost = handles.mana_cost.clone();
        self.color_override = def.card.color_indicator;
        self.supertypes = handles.supertypes.clone();
        self.card_types = handles.card_types.clone();
        self.subtypes = handles.subtypes.clone();
        self.compiled_card_text = handles.compiled_card_text.clone();
        self.rules_text_color_identity = def.card.rules_text_color_identity;
        self.other_face = def.card.other_face;
        self.other_face_name = handles.other_face_name.clone();
        self.linked_face_layout = def.card.linked_face_layout;
        self.base_power = base_power;
        self.base_toughness = base_toughness;
        self.base_loyalty = def.card.loyalty;
        self.base_defense = def.card.defense;
        self.hand_modifier = def.card.hand_modifier;
        self.life_modifier = def.card.life_modifier;
        self.abilities = handles.abilities.clone();

        self.spell_effect = handles.spell_effect.clone();
        self.aura_attach_filter = handles.aura_attach_filter.clone();
        self.bestow_cast_state = None;
        self.face_down_cast_state = None;
        self.prototype_cast_state = None;
        self.alternative_casts = handles.alternative_casts.clone();
        self.cast_alternative_method = None;
        self.has_fuse = def.has_fuse;
        self.optional_costs = handles.optional_costs.clone();
        self.additional_cost = handles.additional_cost.clone();
    }

    /// Restore the printed spell program after a stack-only text/effect
    /// overlay (for example Overload, Cleave, or Awaken) ends.
    pub(crate) fn restore_printed_spell_effect(&mut self, handles: &CardSharedHandles) {
        self.spell_effect = handles.spell_effect.clone();
    }

    /// Apply the temporary stack characteristics of a fused split spell.
    pub fn apply_fused_split_spell_overlay(&mut self, other: &crate::cards::CardDefinition) {
        let mut mana_pips = Vec::new();
        if let Some(cost) = &self.mana_cost {
            mana_pips.extend(cost.pips().iter().cloned());
        }
        if let Some(cost) = &other.card.mana_cost {
            mana_pips.extend(cost.pips().iter().cloned());
        }

        self.name = format!("{} // {}", self.name, other.card.name).into();
        self.first_printed_set_name = None;
        self.mana_cost = if mana_pips.is_empty() {
            None
        } else {
            Some(ManaCost::from_pips(mana_pips).into())
        };
        self.color_override = match (self.color_override, other.card.color_indicator) {
            (Some(left), Some(right)) => Some(left.union(right)),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        };
        Self::extend_unique(&mut self.supertypes, &other.card.supertypes);
        Self::extend_unique(&mut self.card_types, &other.card.card_types);
        Self::extend_unique(&mut self.subtypes, &other.card.subtypes);
        self.rules_text_color_identity = self
            .rules_text_color_identity
            .union(other.card.rules_text_color_identity);
        self.base_power = None;
        self.base_toughness = None;
        self.base_loyalty = None;
        self.base_defense = None;
        self.abilities_mut().extend(other.abilities.iter().cloned());

        let mut effects = self.spell_effect_owned().unwrap_or_default();
        effects.extend(other.spell_effect.clone().unwrap_or_default());
        self.spell_effect = Some(effects.into());
        self.aura_attach_filter = None;
        self.bestow_cast_state = None;
        self.prototype_cast_state = None;
        self.linked_face_layout = LinkedFaceLayout::Split;
    }

    /// Reconstructs a CardDefinition from this object's fields.
    /// Used for rendering compiled text in the UI.
    pub fn to_card_definition(&self) -> crate::cards::CardDefinition {
        use crate::card::PowerToughness;

        let power_toughness = match (self.base_power, self.base_toughness) {
            (Some(p), Some(t)) => Some(PowerToughness::new(p, t)),
            _ => None,
        };
        crate::cards::CardDefinition {
            card: Card {
                id: self.card.unwrap_or_default(),
                name: self.name.to_owned_string(),
                first_printed_set_name: self
                    .first_printed_set_name
                    .as_ref()
                    .map(SharedStr::to_owned_string),
                attraction_lights: Vec::new(),
                mana_cost: self.mana_cost_owned(),
                color_indicator: self.color_override,
                supertypes: self.supertypes.to_vec(),
                card_types: self.card_types.to_vec(),
                subtypes: self.subtypes.to_vec(),
                rules_text_color_identity: self.rules_text_color_identity,
                power_toughness,
                loyalty: self.base_loyalty,
                defense: self.base_defense,
                hand_modifier: self.hand_modifier,
                life_modifier: self.life_modifier,
                other_face: self.other_face,
                other_face_name: self
                    .other_face_name
                    .as_ref()
                    .map(SharedStr::to_owned_string),
                linked_face_layout: self.linked_face_layout,
                is_token: matches!(self.kind, ObjectKind::Token),
            },
            canonical_text: self.compiled_card_text.to_string(),
            ability_labels: Vec::new(),
            abilities: self.abilities_vec(),
            spell_effect: self.spell_effect_owned(),
            aura_attach_filter: self.aura_attach_filter_owned(),
            alternative_casts: self.alternative_casts.to_vec(),
            has_fuse: self.has_fuse,
            optional_costs: self.optional_costs.to_vec(),
            additional_cost: self.additional_cost.to_owned_value(),
            refers_to_ante: false,
        }
    }

    /// Creates a new token.
    #[allow(clippy::too_many_arguments)]
    pub fn new_token(
        id: ObjectId,
        owner: PlayerId,
        name: String,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        power: Option<i32>,
        toughness: Option<i32>,
        color: ColorSet,
    ) -> Self {
        Self {
            id,
            stable_id: StableId::from(id), // New token gets its own stable_id
            last_modified: 0,
            kind: ObjectKind::Token,
            card: None,
            zone: Zone::Battlefield,
            owner,
            name: name.into(),
            first_printed_set_name: None,
            mana_cost: None,
            color_override: Some(color),
            supertypes: Vec::new().into(),
            card_types: card_types.into(),
            subtypes: subtypes.into(),
            compiled_card_text: Arc::from(""),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            base_power: power.map(PtValue::Fixed),
            base_toughness: toughness.map(PtValue::Fixed),
            base_loyalty: None,
            base_defense: None,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: Arc::new(Vec::new()),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: Vec::new(),
            x_value: None,
            keyword_payment_contributions_to_cast: Vec::new(),
            cast_tagged_objects: HashMap::new(),
            additional_cost: TotalCost::free().into(),
        }
    }

    /// Creates a token that's a copy of another object.
    /// Per MTG rules, tokens copy copiable values but not non-copiable state.
    /// Note: Battlefield state (tapped, summoning_sick, etc.) is managed via GameState extension maps.
    pub fn token_copy_of(source: &Object, id: ObjectId, owner: PlayerId) -> Self {
        let bestow_restore = source.bestow_cast_state.as_ref();
        let card_types = bestow_restore
            .map(|restore| restore.card_types.clone())
            .unwrap_or_else(|| source.card_types.clone());
        let subtypes = bestow_restore
            .map(|restore| restore.subtypes.clone())
            .unwrap_or_else(|| source.subtypes.clone());
        let spell_effect = bestow_restore
            .map(|restore| restore.spell_effect.clone())
            .unwrap_or_else(|| source.spell_effect.clone());
        let aura_attach_filter = bestow_restore
            .map(|restore| restore.aura_attach_filter.clone())
            .unwrap_or_else(|| source.aura_attach_filter.clone());
        let mut token = Self {
            id,
            stable_id: StableId::from(id), // Token copy is a new instance
            last_modified: 0,
            kind: ObjectKind::Token,
            card: None,
            zone: Zone::Battlefield,
            owner,
            // Copiable values from source
            name: source.name.clone(),
            first_printed_set_name: source.first_printed_set_name.clone(),
            mana_cost: source.mana_cost.clone(),
            color_override: source.color_override,
            supertypes: source.supertypes.clone(),
            card_types,
            subtypes,
            compiled_card_text: source.compiled_card_text.clone(),
            rules_text_color_identity: source.rules_text_color_identity,
            other_face: source.other_face,
            other_face_name: source.other_face_name.clone(),
            linked_face_layout: source.linked_face_layout,
            base_power: source.base_power,
            base_toughness: source.base_toughness,
            base_loyalty: source.base_loyalty,
            base_defense: source.base_defense,
            hand_modifier: source.hand_modifier,
            life_modifier: source.life_modifier,
            abilities: source.abilities.clone(),
            // Non-copiable values reset to defaults
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            // Note: spell_effect is copiable for spell copies
            spell_effect,
            splice_cast_state: None,
            aura_attach_filter,
            bestow_cast_state: None,
            face_down_cast_state: source.face_down_cast_state.clone(),
            prototype_cast_state: None,
            // Alternative casts are copiable (though tokens rarely use them)
            alternative_casts: source.alternative_casts.clone(),
            cast_alternative_method: None,
            has_fuse: source.has_fuse,
            // Optional costs are copiable
            optional_costs: source.optional_costs.clone(),
            // Optional costs paid is non-copiable (tokens weren't cast)
            optional_costs_paid: OptionalCostsPaid::default(),
            // Tokens are never cast.
            mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: Vec::new(),
            x_value: None,
            keyword_payment_contributions_to_cast: Vec::new(),
            cast_tagged_objects: HashMap::new(),
            // Cost effects are copiable
            additional_cost: source.additional_cost.clone(),
            // Saga fields - copiable (a token copy of a saga is also a saga)
        };
        // Planeswalker tokens enter with loyalty counters equal to base loyalty
        if let Some(loyalty) = source.base_loyalty {
            token.add_counters(CounterType::Loyalty, loyalty);
        }
        token
    }

    /// Creates a copy of a spell on the stack.
    ///
    /// Unlike token copies of permanents, spell copies copy the spell's current
    /// copiable characteristics on the stack, including temporary cast overlays
    /// such as bestow.
    pub fn spell_copy_of(source: &Object, id: ObjectId, owner: PlayerId) -> Self {
        let mut copy = Self {
            id,
            stable_id: StableId::from(id),
            last_modified: 0,
            kind: ObjectKind::SpellCopy,
            card: None,
            zone: Zone::Stack,
            owner,
            name: source.name.clone(),
            first_printed_set_name: source.first_printed_set_name.clone(),
            mana_cost: source.mana_cost.clone(),
            color_override: source.color_override,
            supertypes: source.supertypes.clone(),
            card_types: source.card_types.clone(),
            subtypes: source.subtypes.clone(),
            compiled_card_text: source.compiled_card_text.clone(),
            rules_text_color_identity: source.rules_text_color_identity,
            other_face: source.other_face,
            other_face_name: source.other_face_name.clone(),
            linked_face_layout: source.linked_face_layout,
            base_power: source.base_power,
            base_toughness: source.base_toughness,
            base_loyalty: source.base_loyalty,
            base_defense: source.base_defense,
            hand_modifier: source.hand_modifier,
            life_modifier: source.life_modifier,
            abilities: source.abilities.clone(),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: source.spell_effect.clone(),
            splice_cast_state: source.splice_cast_state.clone(),
            aura_attach_filter: source.aura_attach_filter.clone(),
            bestow_cast_state: source.bestow_cast_state.clone(),
            face_down_cast_state: source.face_down_cast_state.clone(),
            prototype_cast_state: source.prototype_cast_state.clone(),
            alternative_casts: source.alternative_casts.clone(),
            cast_alternative_method: source.cast_alternative_method.clone(),
            has_fuse: source.has_fuse,
            optional_costs: source.optional_costs.clone(),
            optional_costs_paid: source.optional_costs_paid.clone(),
            mana_spent_to_cast: source.mana_spent_to_cast.clone(),
            temporary_static_ability_grants: source.temporary_static_ability_grants.clone(),
            x_value: source.x_value,
            keyword_payment_contributions_to_cast: source
                .keyword_payment_contributions_to_cast
                .clone(),
            cast_tagged_objects: source.cast_tagged_objects.clone(),
            additional_cost: source.additional_cost.clone(),
        };
        if let Some(loyalty) = source.base_loyalty {
            copy.add_counters(CounterType::Loyalty, loyalty);
        }
        copy
    }

    /// Creates a token using last-known-information copiable values.
    ///
    /// This is used when the source object no longer exists, but a resolving effect
    /// still needs to copy what it looked like at the relevant earlier moment.
    pub fn token_copy_from_snapshot(
        snapshot: &crate::snapshot::ObjectSnapshot,
        id: ObjectId,
        owner: PlayerId,
    ) -> Self {
        let copiable = &snapshot.copiable_values;
        let mut token = Self {
            id,
            stable_id: StableId::from(id),
            last_modified: 0,
            kind: ObjectKind::Token,
            card: None,
            zone: Zone::Battlefield,
            owner,
            name: copiable.name.clone().into(),
            first_printed_set_name: snapshot.first_printed_set_name.clone().map(Into::into),
            mana_cost: shared_optional_value(copiable.mana_cost.clone()),
            color_override: (!copiable.colors.is_empty()).then_some(copiable.colors),
            supertypes: copiable.supertypes.clone().into(),
            card_types: copiable.card_types.clone().into(),
            subtypes: copiable.subtypes.clone().into(),
            compiled_card_text: Arc::from(copiable.compiled_card_text.as_str()),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: snapshot.other_face,
            other_face_name: snapshot.other_face_name.clone().map(Into::into),
            linked_face_layout: snapshot.linked_face_layout,
            base_power: copiable.power.map(PtValue::Fixed),
            base_toughness: copiable.toughness.map(PtValue::Fixed),
            base_loyalty: copiable.loyalty,
            base_defense: snapshot.defense,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: copiable.abilities.clone(),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: shared_optional_value(copiable.aura_attach_filter.clone()),
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: Vec::new(),
            x_value: None,
            keyword_payment_contributions_to_cast: Vec::new(),
            cast_tagged_objects: HashMap::new(),
            additional_cost: TotalCost::free().into(),
        };
        if let Some(loyalty) = copiable.loyalty {
            token.add_counters(CounterType::Loyalty, loyalty);
        }
        token
    }

    /// Creates a new emblem in the command zone.
    ///
    /// Emblems are permanent game objects created by planeswalker ultimates.
    /// They exist in the command zone and cannot be interacted with by most
    /// game mechanics (they have no controller change, can't be destroyed, etc.)
    pub fn new_emblem(
        id: ObjectId,
        owner: PlayerId,
        name: String,
        abilities: Vec<Ability>,
    ) -> Self {
        Self {
            id,
            stable_id: StableId::from(id), // Emblems get their own stable_id
            last_modified: 0,
            kind: ObjectKind::Emblem,
            card: None,
            zone: Zone::Command,
            owner,
            name: name.into(),
            first_printed_set_name: None,
            mana_cost: None,
            color_override: None,
            supertypes: Vec::new().into(),
            card_types: Vec::new().into(),
            subtypes: Vec::new().into(),
            compiled_card_text: Arc::from(""),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            base_power: None,
            base_toughness: None,
            base_loyalty: None,
            base_defense: None,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: Arc::new(abilities),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: Vec::new(),
            x_value: None,
            keyword_payment_contributions_to_cast: Vec::new(),
            cast_tagged_objects: HashMap::new(),
            additional_cost: TotalCost::free().into(),
        }
    }

    /// Copies copiable values from another object (for Clone effects).
    /// Per MTG rule 707.2, copiable values are: name, mana cost, color, card types,
    /// subtypes, supertypes, rules text, power, toughness, loyalty, and abilities.
    /// Non-copiable state (counters, damage, etc.) is NOT copied.
    pub fn copy_copiable_values_from(&mut self, source: &Object) {
        self.copy_copiable_values_from_values(&CopiableValues::from_object(source));
        let bestow_restore = source.bestow_cast_state.as_ref();
        self.first_printed_set_name = source.first_printed_set_name.clone();
        self.rules_text_color_identity = source.rules_text_color_identity;
        self.other_face = source.other_face;
        self.other_face_name = source.other_face_name.clone();
        self.linked_face_layout = source.linked_face_layout;
        self.base_defense = source.base_defense;
        self.aura_attach_filter = bestow_restore
            .map(|restore| restore.aura_attach_filter.clone())
            .unwrap_or_else(|| source.aura_attach_filter.clone());
        self.has_fuse = source.has_fuse;
    }

    /// Apply an already-frozen layer-1 copiable-values record.
    ///
    /// Game-aware copy paths use this after calculating the source through
    /// layers 1a and 1b, so a copy of a copy does not fall back to the source
    /// object's printed/raw fields (CR 707.2–707.3).
    pub fn copy_copiable_values_from_values(&mut self, values: &CopiableValues) {
        self.name = values.name.clone().into();
        self.mana_cost = shared_optional_value(values.mana_cost.clone());
        self.color_override = Some(values.colors);
        self.supertypes = values.supertypes.clone().into();
        self.card_types = values.card_types.clone().into();
        self.subtypes = values.subtypes.clone().into();
        self.compiled_card_text = values.compiled_card_text.clone().into();
        self.base_power = values.power.map(PtValue::Fixed);
        self.base_toughness = values.toughness.map(PtValue::Fixed);
        self.base_loyalty = values.loyalty;
        self.abilities = values.abilities.clone();
        self.aura_attach_filter = shared_optional_value(values.aura_attach_filter.clone());
    }

    /// Apply the temporary "cast with bestow" Aura overlay.
    ///
    /// This stores original copiable fields so state-based actions can restore
    /// creature form when the permanent stops being attached.
    pub fn apply_bestow_cast_overlay(&mut self) {
        if self.bestow_cast_state.is_some() {
            return;
        }

        self.bestow_cast_state = Some(Box::new(BestowCastState {
            card_types: self.card_types.clone(),
            subtypes: self.subtypes.clone(),
            aura_attach_filter: self.aura_attach_filter.clone(),
            spell_effect: self.spell_effect.clone(),
        }));

        let mut card_types = self.card_types.clone();
        card_types.retain(|card_type| *card_type != CardType::Creature);
        if !card_types.contains(&CardType::Enchantment) {
            card_types.push(CardType::Enchantment);
        }
        self.card_types = card_types;

        let mut subtypes = self.subtypes.clone();
        subtypes.retain(|subtype| !subtype.is_creature_type() && *subtype != Subtype::Aura);
        subtypes.push(Subtype::Aura);
        self.subtypes = subtypes;

        self.aura_attach_filter =
            Some(AuraAttachmentFilter::from(crate::target::ObjectFilter::creature()).into());
        self.ensure_aura_cast_spell_effect();
    }

    /// Synthesize the cast-time attach effect for Aura spells that only carry an
    /// enchant restriction on the definition.
    pub fn ensure_aura_cast_spell_effect(&mut self) {
        if self.spell_effect.is_some() || !self.subtypes.contains(&Subtype::Aura) {
            return;
        }

        let Some(filter) = self.aura_attach_filter_owned() else {
            return;
        };

        let target_spec = filter.target_spec();
        self.spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::attach_to(target_spec),
            ])
            .into(),
        );
    }

    /// Returns true if this object is currently in the temporary bestow Aura form.
    pub fn is_bestow_overlay_active(&self) -> bool {
        self.bestow_cast_state.is_some()
    }

    fn colors_from_mana_cost(cost: &ManaCost) -> ColorSet {
        use crate::mana::ManaSymbol;

        let mut colors = ColorSet::COLORLESS;
        for pip in cost.pips() {
            for symbol in pip {
                colors = match symbol {
                    ManaSymbol::White => colors.with(Color::White),
                    ManaSymbol::Blue => colors.with(Color::Blue),
                    ManaSymbol::Black => colors.with(Color::Black),
                    ManaSymbol::Red => colors.with(Color::Red),
                    ManaSymbol::Green => colors.with(Color::Green),
                    _ => colors,
                };
            }
        }
        colors
    }

    pub fn apply_prototype_cast_overlay(
        &mut self,
        cost: ManaCost,
        power_toughness: PowerToughness,
    ) -> bool {
        if self.prototype_cast_state.is_some() {
            return false;
        }

        self.prototype_cast_state = Some(PrototypeCastState {
            mana_cost: self.mana_cost.clone(),
            color_override: self.color_override,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
        });

        let colors = Self::colors_from_mana_cost(&cost);
        self.mana_cost = Some(cost.into());
        self.color_override = (!colors.is_empty()).then_some(colors);
        self.base_power = Some(power_toughness.power);
        self.base_toughness = Some(power_toughness.toughness);
        true
    }

    pub fn end_prototype_cast_overlay(&mut self) -> bool {
        let Some(restore) = self.prototype_cast_state.take() else {
            return false;
        };

        self.mana_cost = restore.mana_cost;
        self.color_override = restore.color_override;
        self.base_power = restore.base_power;
        self.base_toughness = restore.base_toughness;
        true
    }

    /// End bestow Aura form and restore original copiable fields.
    pub fn end_bestow_cast_overlay(&mut self) -> bool {
        let Some(restore) = self.bestow_cast_state.take() else {
            return false;
        };
        self.card_types = restore.card_types;
        self.subtypes = restore.subtypes;
        self.aura_attach_filter = restore.aura_attach_filter;
        self.spell_effect = restore.spell_effect;
        true
    }

    /// Begin the stack-only text-changing overlay created by splice.
    pub fn begin_splice_cast_overlay(&mut self) -> bool {
        if self.splice_cast_state.is_some() {
            return false;
        }
        self.splice_cast_state = Some(Box::new(SpliceCastState {
            spell_effect: self.spell_effect.clone(),
        }));
        true
    }

    /// End the splice overlay and restore the physical card's original program.
    pub fn end_splice_cast_overlay(&mut self) -> bool {
        let Some(restore) = self.splice_cast_state.take() else {
            return false;
        };
        self.spell_effect = restore.spell_effect;
        true
    }

    /// Apply the shared face-down cast overlay used by morph-style casting.
    pub fn apply_face_down_cast_overlay(&mut self) -> bool {
        if self.face_down_cast_state.is_some() {
            return false;
        }

        let has_disguise = self.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.is_disguise()
            )
        });

        self.face_down_cast_state = Some(Box::new(FaceDownCastState {
            name: self.name.clone(),
            first_printed_set_name: self.first_printed_set_name.clone(),
            mana_cost: self.mana_cost.clone(),
            color_override: self.color_override,
            supertypes: self.supertypes.clone(),
            card_types: self.card_types.clone(),
            subtypes: self.subtypes.clone(),
            compiled_card_text: self.compiled_card_text.clone(),
            rules_text_color_identity: self.rules_text_color_identity,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
            base_loyalty: self.base_loyalty,
            base_defense: self.base_defense,
            abilities: self.abilities.clone(),
            spell_effect: self.spell_effect.clone(),
            aura_attach_filter: self.aura_attach_filter.clone(),
        }));

        self.name = "Face-down creature".into();
        self.first_printed_set_name = None;
        self.mana_cost = None;
        self.color_override = Some(ColorSet::COLORLESS);
        self.supertypes.clear();
        self.card_types = vec![CardType::Creature].into();
        self.subtypes.clear();
        self.compiled_card_text = Arc::from("");
        self.base_power = Some(PtValue::Fixed(2));
        self.base_toughness = Some(PtValue::Fixed(2));
        self.base_loyalty = None;
        self.base_defense = None;
        self.abilities_mut().retain(|ability| {
            matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.turn_face_up_cost().is_some()
            )
        });
        if has_disguise {
            self.abilities_mut()
                .push(Ability::static_ability(StaticAbility::ward(
                    TotalCost::mana(ManaCost::from_pips(vec![vec![
                        crate::mana::ManaSymbol::Generic(2),
                    ]])),
                )));
        }
        self.spell_effect = None;
        self.aura_attach_filter = None;
        self.bestow_cast_state = None;
        true
    }

    /// End the shared face-down cast overlay and restore printed characteristics.
    pub fn end_face_down_cast_overlay(&mut self) -> bool {
        let Some(restore) = self.face_down_cast_state.take() else {
            return false;
        };
        let restore = *restore;

        self.name = restore.name;
        self.first_printed_set_name = restore.first_printed_set_name;
        self.mana_cost = restore.mana_cost;
        self.color_override = restore.color_override;
        self.supertypes = restore.supertypes;
        self.card_types = restore.card_types;
        self.subtypes = restore.subtypes;
        self.compiled_card_text = restore.compiled_card_text;
        self.rules_text_color_identity = restore.rules_text_color_identity;
        self.base_power = restore.base_power;
        self.base_toughness = restore.base_toughness;
        self.base_loyalty = restore.base_loyalty;
        self.base_defense = restore.base_defense;
        self.abilities = restore.abilities;
        self.spell_effect = restore.spell_effect;
        self.aura_attach_filter = restore.aura_attach_filter;
        true
    }

    /// Returns the colors of this object.
    pub fn colors(&self) -> ColorSet {
        // Devoid applies in all functional zones of the ability.
        if self.abilities.iter().any(|ability| {
            ability.functions_in(&self.zone)
                && matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability) if static_ability.is_devoid()
                )
        }) {
            return ColorSet::COLORLESS;
        }

        if let Some(override_colors) = self.color_override {
            return override_colors;
        }

        let Some(mana_cost) = &self.mana_cost else {
            return ColorSet::COLORLESS;
        };

        use crate::color::Color;
        use crate::mana::ManaSymbol;

        let mut colors = ColorSet::COLORLESS;
        for pip in mana_cost.pips() {
            for symbol in pip {
                match symbol {
                    ManaSymbol::White => colors = colors.with(Color::White),
                    ManaSymbol::Blue => colors = colors.with(Color::Blue),
                    ManaSymbol::Black => colors = colors.with(Color::Black),
                    ManaSymbol::Red => colors = colors.with(Color::Red),
                    ManaSymbol::Green => colors = colors.with(Color::Green),
                    _ => {}
                }
            }
        }
        colors
    }

    /// Returns the color identity of this object (for Commander format).
    /// Color identity includes colors from:
    /// - Mana cost
    /// - Color indicator/override
    /// - Mana symbols in rules text (e.g., "{T}: Add {G}")
    pub fn color_identity(&self) -> ColorSet {
        use crate::color::Color;
        use crate::mana::ManaSymbol;

        let mut identity = ColorSet::COLORLESS;

        // Add colors from mana cost
        if let Some(mana_cost) = &self.mana_cost {
            for pip in mana_cost.pips() {
                for symbol in pip {
                    match symbol {
                        ManaSymbol::White => identity = identity.with(Color::White),
                        ManaSymbol::Blue => identity = identity.with(Color::Blue),
                        ManaSymbol::Black => identity = identity.with(Color::Black),
                        ManaSymbol::Red => identity = identity.with(Color::Red),
                        ManaSymbol::Green => identity = identity.with(Color::Green),
                        _ => {}
                    }
                }
            }
        }

        // Add colors from color indicator/override
        if let Some(override_colors) = self.color_override {
            identity = identity.union(override_colors);
        }

        identity = identity.union(self.rules_text_color_identity);

        identity
    }

    /// Returns the current power of this creature.
    /// Returns None if this is not a creature.
    pub fn power(&self) -> Option<i32> {
        // Check for level abilities first - they can override base P/T
        let base = if let Some((power, _)) = self.level_ability_pt() {
            power
        } else {
            self.base_power?.base_value()
        };
        let (power_delta, _) = self.pt_counter_deltas();
        Some(base + power_delta)
    }

    /// Returns the current toughness of this creature.
    /// Returns None if this is not a creature.
    pub fn toughness(&self) -> Option<i32> {
        // Check for level abilities first - they can override base P/T
        let base = if let Some((_, toughness)) = self.level_ability_pt() {
            toughness
        } else {
            self.base_toughness?.base_value()
        };
        let (_, toughness_delta) = self.pt_counter_deltas();
        Some(base + toughness_delta)
    }

    pub fn pt_counter_deltas(&self) -> (i32, i32) {
        let mut power = 0i32;
        let mut toughness = 0i32;
        for (counter_type, count) in &self.counters {
            if let Some((dp, dt)) = counter_type.pt_delta() {
                power += dp * (*count as i32);
                toughness += dt * (*count as i32);
            }
        }
        (power, toughness)
    }

    /// Returns the P/T override from level abilities if applicable.
    /// Returns None if there are no level abilities or the current level tier has no P/T override.
    fn level_ability_pt(&self) -> Option<(i32, i32)> {
        use crate::ability::AbilityKind;

        let level_count = self.counters.get(&CounterType::Level).copied().unwrap_or(0);

        for ability in self.abilities.iter() {
            if let AbilityKind::Static(s) = &ability.kind
                && let Some(levels) = s.level_abilities()
            {
                // Find the matching tier (highest tier that applies)
                for tier in levels.iter().rev() {
                    if level_count >= tier.min_level
                        && tier.max_level.is_none_or(|max| level_count <= max)
                    {
                        return tier.power_toughness;
                    }
                }
            }
        }
        None
    }

    /// Returns all static abilities granted by the current level tier.
    pub fn level_granted_abilities(&self) -> Vec<crate::static_abilities::StaticAbility> {
        use crate::ability::AbilityKind;

        let level_count = self.counters.get(&CounterType::Level).copied().unwrap_or(0);

        for ability in self.abilities.iter() {
            if let AbilityKind::Static(s) = &ability.kind
                && let Some(levels) = s.level_abilities()
            {
                // Find the matching tier (highest tier that applies)
                for tier in levels.iter().rev() {
                    if level_count >= tier.min_level
                        && tier.max_level.is_none_or(|max| level_count <= max)
                    {
                        // Abilities are now stored as the new type directly
                        return tier.abilities.clone();
                    }
                }
            }
        }
        Vec::new()
    }

    /// Returns the current loyalty of this planeswalker.
    pub fn loyalty(&self) -> Option<u32> {
        let base = self.base_loyalty?;
        Some(
            self.counters
                .get(&CounterType::Loyalty)
                .copied()
                .unwrap_or(base),
        )
    }

    /// Returns the printed defense value of this battle.
    pub fn defense(&self) -> Option<u32> {
        self.base_defense
    }

    /// Adds counters of the specified type.
    pub fn add_counters(&mut self, counter_type: CounterType, amount: u32) {
        *self.counters.entry(counter_type).or_insert(0) += amount;
    }

    /// Removes counters of the specified type. Returns the number actually removed.
    pub fn remove_counters(&mut self, counter_type: CounterType, amount: u32) -> u32 {
        let current = self.counters.entry(counter_type).or_insert(0);
        let removed = (*current).min(amount);
        *current -= removed;
        if *current == 0 {
            self.counters.remove(&counter_type);
        }
        removed
    }

    /// Returns true if this creature has taken lethal damage.
    /// `damage_marked` should be obtained from GameState::damage_on(id).
    pub fn has_lethal_damage(&self, damage_marked: u32) -> bool {
        if let Some(toughness) = self.toughness() {
            toughness <= 0 || damage_marked >= toughness as u32
        } else {
            false
        }
    }

    /// Returns true if this object has the given card type.
    pub fn has_card_type(&self, card_type: CardType) -> bool {
        self.card_types.contains(&card_type)
    }

    /// Returns true if this object has the given supertype.
    pub fn has_supertype(&self, supertype: Supertype) -> bool {
        self.supertypes.contains(&supertype)
    }

    /// Returns true if this object has the given subtype.
    ///
    /// If the object has Changeling and is a creature, it has all creature types.
    pub fn has_subtype(&self, subtype: Subtype) -> bool {
        if self.subtypes.contains(&subtype) {
            return true;
        }

        // Changeling means this creature is every creature type
        if subtype.is_creature_type() && self.is_creature() && self.has_changeling() {
            return true;
        }

        false
    }

    /// Returns true if this object has the Changeling ability.
    pub fn has_changeling(&self) -> bool {
        use crate::ability::AbilityKind;
        self.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.is_changeling()
            } else {
                false
            }
        })
    }

    /// Returns true if this is a creature.
    pub fn is_creature(&self) -> bool {
        self.has_card_type(CardType::Creature)
    }

    /// Returns true if this is a land.
    pub fn is_land(&self) -> bool {
        self.has_card_type(CardType::Land)
    }

    /// Returns true if this is a permanent type.
    pub fn is_permanent(&self) -> bool {
        self.has_card_type(CardType::Creature)
            || self.has_card_type(CardType::Artifact)
            || self.has_card_type(CardType::Enchantment)
            || self.has_card_type(CardType::Land)
            || self.has_card_type(CardType::Planeswalker)
            || self.has_card_type(CardType::Battle)
    }

    /// Returns true if this is legendary.
    pub fn is_legendary(&self) -> bool {
        self.has_supertype(Supertype::Legendary)
    }

    /// Returns true if this object has the given static ability.
    /// This includes abilities granted by level tiers.
    pub fn has_static_ability(&self, ability: &crate::static_abilities::StaticAbility) -> bool {
        use crate::ability::AbilityKind;

        // Check regular static abilities
        let has_regular = self.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s == ability
            } else {
                false
            }
        });

        if has_regular {
            return true;
        }

        // Check level-granted abilities
        self.level_granted_abilities().iter().any(|a| a == ability)
    }

    /// Returns true if this object has a static ability with the given ID.
    /// This includes abilities granted by level tiers.
    pub fn has_static_ability_id(
        &self,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        use crate::ability::AbilityKind;

        let has_regular = self.abilities.iter().any(|ability| {
            if let AbilityKind::Static(static_ability) = &ability.kind {
                static_ability.id() == ability_id
            } else {
                false
            }
        });
        if has_regular {
            return true;
        }

        self.level_granted_abilities()
            .iter()
            .any(|ability| ability.id() == ability_id)
    }

    /// Returns true if this object has indestructible.
    pub fn has_indestructible(&self) -> bool {
        self.has_static_ability(&crate::static_abilities::StaticAbility::indestructible())
    }

    /// Creates a token from a CardDefinition.
    ///
    /// The CardDefinition should have been built with `.token()` to mark it as a token.
    /// This is the preferred way to create tokens - use CardDefinitionBuilder with all
    /// the normal ability methods instead of the deprecated TokenDescription.
    /// Note: Battlefield state (summoning_sick, etc.) is managed via GameState extension maps.
    pub fn from_token_definition(
        id: ObjectId,
        def: &crate::cards::CardDefinition,
        controller: PlayerId,
    ) -> Self {
        let handles = CardSharedHandles::from_definition(def);
        Self::from_token_definition_with_shared(id, def, controller, &handles)
    }

    pub(crate) fn from_token_definition_with_shared(
        id: ObjectId,
        def: &crate::cards::CardDefinition,
        controller: PlayerId,
        handles: &CardSharedHandles,
    ) -> Self {
        Self {
            id,
            stable_id: StableId::from(id),
            last_modified: 0,
            kind: ObjectKind::Token,
            card: None,
            zone: Zone::Battlefield,
            owner: controller,
            name: handles.name.clone(),
            first_printed_set_name: handles.first_printed_set_name.clone(),
            mana_cost: None,                          // Tokens don't have mana costs
            color_override: def.card.color_indicator, // Use color indicator if set
            supertypes: handles.supertypes.clone(),
            card_types: handles.card_types.clone(),
            subtypes: handles.subtypes.clone(),
            compiled_card_text: handles.compiled_card_text.clone(),
            rules_text_color_identity: def.card.rules_text_color_identity,
            other_face: def.card.other_face,
            other_face_name: handles.other_face_name.clone(),
            linked_face_layout: def.card.linked_face_layout,
            base_power: def.card.power_toughness.map(|pt| pt.power),
            base_toughness: def.card.power_toughness.map(|pt| pt.toughness),
            base_loyalty: def.card.loyalty,
            base_defense: def.card.defense,
            hand_modifier: def.card.hand_modifier,
            life_modifier: def.card.life_modifier,
            abilities: handles.abilities.clone(),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: handles.spell_effect.clone(),
            splice_cast_state: None,
            aura_attach_filter: handles.aura_attach_filter.clone(),
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            alternative_casts: handles.alternative_casts.clone(),
            cast_alternative_method: None,
            has_fuse: def.has_fuse,
            optional_costs: handles.optional_costs.clone(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: Vec::new(),
            x_value: None,
            keyword_payment_contributions_to_cast: Vec::new(),
            cast_tagged_objects: HashMap::new(),
            additional_cost: handles.additional_cost.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::mana::ManaSymbol;
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;

    #[test]
    fn test_object_from_card() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        assert_eq!(obj.name, "Grizzly Bears");
        assert_eq!(obj.power(), Some(2));
        assert_eq!(obj.toughness(), Some(2));
        assert!(obj.is_creature());
        assert!(obj.colors().contains(Color::Green));
    }

    #[test]
    fn token_markers_survive_card_and_definition_construction_into_snapshots() {
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mana_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Green]]);

        let token_card = CardBuilder::new(CardId::from_raw(2), "Card Token")
            .mana_cost(mana_cost.clone())
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .token()
            .build();
        let token_card_id = game.create_object_from_card(&token_card, alice, Zone::Battlefield);
        let token_card_object = game.object(token_card_id).expect("token card object");
        assert_eq!(token_card_object.kind, ObjectKind::Token);
        assert_eq!(token_card_object.card, None);
        assert_eq!(token_card_object.mana_cost, None);
        assert!(
            ObjectSnapshot::from_object(token_card_object, &game).is_token,
            "LKI must retain the token identity used by token/nontoken filters"
        );

        let token_definition =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(3), "Definition Token")
                .mana_cost(mana_cost.clone())
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(1, 1))
                .token()
                .build();
        let token_definition_id =
            game.create_object_from_definition(&token_definition, alice, Zone::Graveyard);
        let token_definition_object = game
            .object(token_definition_id)
            .expect("token definition object");
        assert_eq!(token_definition_object.kind, ObjectKind::Token);
        assert_eq!(token_definition_object.card, None);
        assert_eq!(token_definition_object.mana_cost, None);
        assert_eq!(token_definition_object.zone, Zone::Graveyard);
        assert!(
            ObjectSnapshot::from_object(token_definition_object, &game).is_token,
            "full definitions must preserve token identity in LKI"
        );

        let physical_card = CardBuilder::new(CardId::from_raw(4), "Physical Card")
            .mana_cost(mana_cost)
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build();
        let physical_card_id =
            game.create_object_from_card(&physical_card, alice, Zone::Battlefield);
        let physical_card_object = game.object(physical_card_id).expect("physical card object");
        assert_eq!(physical_card_object.kind, ObjectKind::Card);
        assert_eq!(physical_card_object.card, Some(physical_card.id));
        assert!(physical_card_object.mana_cost.is_some());
        assert!(!ObjectSnapshot::from_object(physical_card_object, &game).is_token);
    }

    #[test]
    fn cloned_object_shared_payload_mutations_do_not_leak() {
        let mut original = Object::new_token(
            ObjectId::from_raw(44),
            PlayerId::from_index(0),
            "Payload Probe".to_string(),
            vec![CardType::Creature],
            vec![Subtype::Human],
            Some(1),
            Some(1),
            ColorSet::WHITE,
        );
        original.compiled_card_text = Arc::from("Original text");
        original
            .optional_costs
            .push(OptionalCost::custom("Probe", TotalCost::free()));

        let mut clone = original.clone();
        clone.card_types.push(CardType::Artifact);
        clone.subtypes.push(Subtype::Construct);
        clone.compiled_card_text = Arc::from("Changed text");
        clone
            .optional_costs
            .push(OptionalCost::custom("Clone-only", TotalCost::free()));

        assert!(!original.card_types.contains(&CardType::Artifact));
        assert!(!original.subtypes.contains(&Subtype::Construct));
        assert_eq!(original.compiled_card_text.as_ref(), "Original text");
        assert_eq!(original.optional_costs.len(), 1);

        assert!(clone.card_types.contains(&CardType::Artifact));
        assert!(clone.subtypes.contains(&Subtype::Construct));
        assert_eq!(clone.compiled_card_text.as_ref(), "Changed text");
        assert_eq!(clone.optional_costs.len(), 2);
    }

    #[test]
    fn typed_characteristic_ability_survives_object_construction() {
        let domain_value =
            crate::effect::Value::BasicLandTypesAmong(ObjectFilter::land().you_control());
        let definition =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(99), "Territorial Kavu")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Bear])
                .power_toughness(crate::card::PowerToughness::new(
                    PtValue::Star,
                    PtValue::Star,
                ))
                .with_ability(Ability::static_ability(
                    StaticAbility::characteristic_defining_pt(domain_value.clone(), domain_value),
                ))
                .build();

        let obj = Object::from_card_definition(
            ObjectId::from_raw(1),
            &definition,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        assert!(obj.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::CharacteristicDefiningPT
            )
        }));
        assert_eq!(obj.base_power, Some(PtValue::Star));
        assert_eq!(obj.base_toughness, Some(PtValue::Star));
    }

    #[test]
    fn test_token_creation() {
        let token = Object::new_token(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            "Soldier".to_string(),
            vec![CardType::Creature],
            vec![Subtype::Soldier],
            Some(1),
            Some(1),
            ColorSet::WHITE,
        );

        assert_eq!(token.name, "Soldier");
        assert_eq!(token.kind, ObjectKind::Token);
        assert_eq!(token.power(), Some(1));
        assert_eq!(token.toughness(), Some(1));
        assert!(token.colors().contains(Color::White));
        // Note: summoning_sick is now tracked in GameState::summoning_sick
    }

    #[test]
    fn test_devoid_applies_in_hand() {
        let card = CardBuilder::new(CardId::from_raw(1), "Devoid Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 1))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Hand,
        );
        obj.abilities_mut().push(
            Ability::static_ability(StaticAbility::make_colorless(ObjectFilter::source()))
                .in_zones(vec![
                    Zone::Battlefield,
                    Zone::Stack,
                    Zone::Hand,
                    Zone::Library,
                    Zone::Graveyard,
                    Zone::Exile,
                    Zone::Command,
                ]),
        );

        assert!(
            obj.colors().is_empty(),
            "devoid object in hand should be colorless"
        );
    }

    #[test]
    fn test_make_colorless_ability_respects_functional_zone() {
        let card = CardBuilder::new(CardId::from_raw(1), "Color Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 1))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Hand,
        );
        obj.abilities_mut()
            .push(Ability::static_ability(StaticAbility::make_colorless(
                ObjectFilter::source(),
            )));

        assert!(
            obj.colors().contains(Color::Blue),
            "battlefield-only make-colorless should not apply in hand"
        );
    }

    #[test]
    fn test_counters() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        // Add +1/+1 counters
        obj.add_counters(CounterType::PlusOnePlusOne, 3);
        assert_eq!(obj.power(), Some(5));
        assert_eq!(obj.toughness(), Some(5));

        // Remove some counters
        let removed = obj.remove_counters(CounterType::PlusOnePlusOne, 2);
        assert_eq!(removed, 2);
        assert_eq!(obj.power(), Some(3));
        assert_eq!(obj.toughness(), Some(3));
    }

    #[test]
    fn test_loyalty_uses_loyalty_counters_when_present() {
        let card = CardBuilder::new(CardId::from_raw(7), "Test Walker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(6)
            .build();
        let mut obj = Object::from_card(
            ObjectId::from_raw(7),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        assert_eq!(
            obj.loyalty(),
            Some(6),
            "without counters, loyalty should fall back to printed value"
        );

        obj.add_counters(CounterType::Loyalty, 4);
        assert_eq!(
            obj.loyalty(),
            Some(4),
            "with counters present, loyalty should reflect counters, not base+counter"
        );
    }

    #[test]
    fn test_lethal_damage() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        // damage_marked is now tracked in GameState::damage_marked
        assert!(!obj.has_lethal_damage(0));
        assert!(!obj.has_lethal_damage(1));
        assert!(obj.has_lethal_damage(2));
    }

    #[test]
    fn test_minus_counters() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        obj.add_counters(CounterType::MinusOneMinusOne, 1);
        assert_eq!(obj.power(), Some(1));
        assert_eq!(obj.toughness(), Some(1));

        // With enough -1/-1 counters, toughness goes to 0 or below
        obj.add_counters(CounterType::MinusOneMinusOne, 1);
        assert_eq!(obj.toughness(), Some(0));
        assert!(obj.has_lethal_damage(0)); // 0 toughness = lethal even with no damage
    }

    #[test]
    fn test_non_standard_pt_counters() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        obj.add_counters(CounterType::PlusOnePlusZero, 1);
        assert_eq!(obj.power(), Some(3));
        assert_eq!(obj.toughness(), Some(2));

        obj.add_counters(CounterType::PlusZeroPlusOne, 2);
        assert_eq!(obj.power(), Some(3));
        assert_eq!(obj.toughness(), Some(4));

        obj.add_counters(CounterType::MinusZeroMinusTwo, 1);
        assert_eq!(obj.power(), Some(3));
        assert_eq!(obj.toughness(), Some(2));

        obj.add_counters(CounterType::PlusOnePlusTwo, 1);
        assert_eq!(obj.power(), Some(4));
        assert_eq!(obj.toughness(), Some(4));
    }

    #[test]
    fn test_counter_type_description() {
        assert_eq!(CounterType::PlusOnePlusOne.description(), "+1/+1");
        assert_eq!(CounterType::PlusOnePlusZero.description(), "+1/+0");
        assert_eq!(CounterType::DoubleStrike.description(), "double strike");
        assert_eq!(CounterType::Finality.description(), "finality");
        assert_eq!(CounterType::Named("burden".into()).description(), "burden");
    }

    #[test]
    fn test_token_copy_of() {
        let definition =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(1), "Serra Angel")
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(3)],
                    vec![ManaSymbol::White],
                    vec![ManaSymbol::White],
                ]))
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Angel])
                .power_toughness(crate::card::PowerToughness::fixed(4, 4))
                .flying()
                .vigilance()
                .build();

        let mut original = Object::from_card_definition(
            ObjectId::from_raw(1),
            &definition,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        // Add some non-copiable state to the original
        original.add_counters(CounterType::PlusOnePlusOne, 2);
        // Note: tapped, damage_marked, summoning_sick are now in GameState extension maps

        // Create a token copy
        let token =
            Object::token_copy_of(&original, ObjectId::from_raw(2), PlayerId::from_index(1));

        // Copiable values should match
        assert_eq!(token.name, "Serra Angel");
        assert_eq!(token.base_power, Some(PtValue::Fixed(4)));
        assert_eq!(token.base_toughness, Some(PtValue::Fixed(4)));
        assert!(token.has_subtype(Subtype::Angel));
        assert_eq!(token.compiled_card_text, original.compiled_card_text);
        assert!(
            token.compiled_card_text.contains("Flying")
                && token.compiled_card_text.contains("Vigilance"),
            "token copy should preserve the AST-rendered text box, got {}",
            token.compiled_card_text
        );

        // Non-copiable state should NOT be copied
        assert_eq!(token.counters.get(&CounterType::PlusOnePlusOne), None);
        // Note: damage_marked, tapped, summoning_sick are now in GameState extension maps

        // Token-specific properties
        assert_eq!(token.kind, ObjectKind::Token);
        assert_eq!(token.owner, PlayerId::from_index(1));
    }

    #[test]
    fn test_copy_copiable_values_from() {
        let bear_card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let angel_card = CardBuilder::new(CardId::from_raw(2), "Serra Angel")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::White],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .oracle_text("Flying, vigilance")
            .power_toughness(crate::card::PowerToughness::fixed(4, 4))
            .build();

        // Create a Clone creature that enters as a copy of Serra Angel
        let mut clone = Object::from_card(
            ObjectId::from_raw(1),
            &bear_card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );
        clone.add_counters(CounterType::PlusOnePlusOne, 1);

        let angel = Object::from_card(
            ObjectId::from_raw(2),
            &angel_card,
            PlayerId::from_index(1),
            Zone::Battlefield,
        );

        // Clone copies the angel
        clone.copy_copiable_values_from(&angel);

        // Copiable values now match the angel
        assert_eq!(clone.name, "Serra Angel");
        assert_eq!(clone.power(), Some(5)); // 4 base + 1 counter
        assert_eq!(clone.toughness(), Some(5));
        assert!(clone.has_subtype(Subtype::Angel));
        assert!(!clone.has_subtype(Subtype::Bear));

        // But identity fields remain unchanged
        assert_eq!(clone.id, ObjectId::from_raw(1));
        assert_eq!(clone.owner, PlayerId::from_index(0));

        // And counters are preserved (non-copiable)
        assert_eq!(clone.counters.get(&CounterType::PlusOnePlusOne), Some(&1));
    }
}
