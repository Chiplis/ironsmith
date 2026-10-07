use crate::filter::ObjectFilterExt as _;
use crate::marker::CounterTypeExt as _;
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

/// Display label for a face-down permanent or spell, which has no name
/// (CR 708.2a). Name comparisons treat it as nameless.
pub const FACE_DOWN_DISPLAY_NAME: &str = "Face-down creature";

pub use ironsmith_core::CounterType;

/// Stable occurrence of a keyword counter. The serial is a little-endian
/// integer with arbitrary precision, so removed registrations are never reused.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct CounterAbilityOrigin {
    pub counter_type: CounterType,
    pub serial: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CounterAbilityOccurrence {
    pub origin: CounterAbilityOrigin,
    pub abilities: Vec<Ability>,
}

/// Counter counts and their registered ability payloads mutate together.
/// Immutable map access is retained; mutable map access would bypass identity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectCounters {
    counts: std::collections::BTreeMap<CounterType, u32>,
    occurrences: std::collections::BTreeMap<CounterType, Vec<CounterAbilityOccurrence>>,
    next_serial: Vec<u32>,
}
impl ObjectCounters {
    pub fn counts(&self) -> &std::collections::BTreeMap<CounterType, u32> {
        &self.counts
    }

    pub fn insert(&mut self, kind: CounterType, count: u32) -> Option<u32> {
        let old = self.counts.insert(kind, count);
        if kind.is_ability_counter() {
            let mut occurrences = self.occurrences.remove(&kind).unwrap_or_default();
            occurrences.truncate(count as usize);
            while occurrences.len() < count as usize {
                let serial = self.next_serial.clone();
                let mut carry = true;
                for digit in &mut self.next_serial {
                    let (next, overflow) = digit.overflowing_add(1);
                    *digit = next;
                    carry = overflow;
                    if !carry {
                        break;
                    }
                }
                if carry {
                    self.next_serial.push(1);
                }
                occurrences.push(CounterAbilityOccurrence {
                    origin: CounterAbilityOrigin {
                        counter_type: kind,
                        serial,
                    },
                    abilities: counter_ability_payloads(kind),
                });
            }
            if !occurrences.is_empty() {
                self.occurrences.insert(kind, occurrences);
            }
        }
        old
    }

    pub fn add(&mut self, kind: CounterType, amount: u32) {
        if amount > 0 {
            let count = self.counts.get(&kind).copied().unwrap_or(0);
            self.insert(kind, count + amount);
        }
    }

    pub fn remove(&mut self, kind: &CounterType) -> Option<u32> {
        self.occurrences.remove(kind);
        self.counts.remove(kind)
    }

    pub fn clear(&mut self) {
        self.counts.clear();
        self.occurrences.clear();
    }

    pub(crate) fn ability_occurrences(&self, kind: CounterType) -> &[CounterAbilityOccurrence] {
        self.occurrences
            .get(&kind)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}
impl std::ops::Deref for ObjectCounters {
    type Target = std::collections::BTreeMap<CounterType, u32>;
    fn deref(&self) -> &Self::Target {
        &self.counts
    }
}
impl<'a> IntoIterator for &'a ObjectCounters {
    type Item = (&'a CounterType, &'a u32);
    type IntoIter = std::collections::btree_map::Iter<'a, CounterType, u32>;
    fn into_iter(self) -> Self::IntoIter {
        self.counts.iter()
    }
}
impl IntoIterator for ObjectCounters {
    type Item = (CounterType, u32);
    type IntoIter = std::collections::btree_map::IntoIter<CounterType, u32>;
    fn into_iter(self) -> Self::IntoIter {
        self.counts.into_iter()
    }
}
impl FromIterator<(CounterType, u32)> for ObjectCounters {
    fn from_iter<T: IntoIterator<Item = (CounterType, u32)>>(iter: T) -> Self {
        let mut counters = Self::default();
        for (kind, count) in iter {
            counters.insert(kind, count);
        }
        counters
    }
}

/// Construct a payload only when a counter is registered, never during queries.
fn counter_ability_payloads(kind: CounterType) -> Vec<Ability> {
    let keyword = match kind {
        CounterType::Deathtouch => Some(StaticAbility::deathtouch()),
        CounterType::Flying => Some(StaticAbility::flying()),
        CounterType::FirstStrike => Some(StaticAbility::first_strike()),
        CounterType::DoubleStrike => Some(StaticAbility::double_strike()),
        CounterType::Hexproof => Some(StaticAbility::hexproof()),
        CounterType::Indestructible => Some(StaticAbility::indestructible()),
        CounterType::Lifelink => Some(StaticAbility::lifelink()),
        CounterType::Menace => Some(StaticAbility::menace()),
        CounterType::Reach => Some(StaticAbility::reach()),
        CounterType::Trample => Some(StaticAbility::trample()),
        CounterType::Vigilance => Some(StaticAbility::vigilance()),
        CounterType::Haste => Some(StaticAbility::haste()),
        CounterType::Named(name) if name.eq_ignore_ascii_case("shadow") => {
            Some(StaticAbility::shadow())
        }
        _ => None,
    };
    if let Some(keyword) = keyword {
        return vec![Ability::static_ability(keyword)];
    }
    if kind == CounterType::Decayed {
        return vec![
            Ability::static_ability(StaticAbility::cant_block()),
            Ability::triggered(
                crate::triggers::Trigger::this_attacks(),
                crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                        crate::triggers::Trigger::end_of_combat(),
                        vec![crate::effect::Effect::sacrifice_source()],
                        true,
                        Vec::new(),
                        crate::target::PlayerFilter::You,
                    )),
                ]),
            ),
        ];
    }
    if matches!(kind, CounterType::Named(name) if name.eq_ignore_ascii_case("exalted")) {
        let attacker_tag = "exalted_attacker";
        return vec![Ability::triggered(
            crate::triggers::Trigger::attacks_alone(
                crate::target::ObjectFilter::creature().you_control(),
            ),
            crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::tag_triggering_object(attacker_tag),
                crate::effect::Effect::pump(
                    1,
                    1,
                    crate::target::ChooseSpec::Tagged(attacker_tag.into()),
                    crate::effect::Until::EndOfTurn,
                ),
            ]),
        )];
    }
    Vec::new()
}

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
    definition: Arc<crate::cards::CardDefinition>,
    name: SharedStr,
    first_printed_set_name: Option<SharedStr>,
    mana_cost: Option<SharedValue<ManaCost>>,
    supertypes: SharedVec<Supertype>,
    card_types: SharedVec<CardType>,
    subtypes: SharedVec<Subtype>,
    compiled_card_text: Arc<str>,
    ability_labels: SharedVec<String>,
    other_face_name: Option<SharedStr>,
    abilities: Arc<Vec<Ability>>,
    spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
    aura_attach_filter: Option<AuraAttachmentMetadata>,
    alternative_casts: SharedVec<AlternativeCastingMethod>,
    optional_costs: SharedVec<OptionalCost>,
    additional_cost: SharedValue<TotalCost>,
}

impl CardSharedHandles {
    /// A CardId identifies a card graph node, not a unique native executable
    /// snapshot. Reuse handles only for the same complete data and immutable
    /// executor/matcher/payer/static occurrences. Semantic PartialEq for native
    /// abilities and costs deliberately does not express these identities.
    pub(crate) fn definition(&self) -> &crate::cards::CardDefinition {
        self.definition.as_ref()
    }

    pub(crate) fn matches_definition(&self, definition: &crate::cards::CardDefinition) -> bool {
        fn key(definition: &crate::cards::CardDefinition) -> impl PartialEq {
            use std::convert::Infallible;
            fn effect(value: crate::effect::Effect) -> Result<(usize, Option<String>), Infallible> {
                Ok((
                    Arc::as_ptr(&value.0) as *const () as usize,
                    value.serialized_model().map(str::to_owned),
                ))
            }
            let identities = std::cell::RefCell::new(Vec::new());
            let cost = |value: crate::costs::Cost| {
                identities
                    .borrow_mut()
                    .push(Arc::as_ptr(&value.0) as *const () as usize);
                Ok::<_, Infallible>(value)
            };
            let model = definition
                .clone()
                .try_map(
                    |ability| {
                        ability.try_map(
                            |value| Ok::<_, Infallible>(value.instance_id()),
                            |value| Ok((value.runtime_matcher_identity(), value.intro_surface())),
                            effect,
                            &cost,
                            Ok,
                        )
                    },
                    effect,
                    &cost,
                    |method| method.try_map(effect, &cost),
                    |optional| optional.try_map(&cost),
                )
                .expect("identity projection is infallible");
            (model, identities.into_inner())
        }
        key(&self.definition) == key(definition)
    }

    pub(crate) fn from_definition(def: &crate::cards::CardDefinition) -> Self {
        let mut abilities = def.abilities.clone();
        // Attraction definitions store the Visit program in spell_effect.
        // Materialize it as an ordinary, copiable triggered ability so layer
        // 6 ability removal and grants apply to it (CR 702.159).
        if def.card.subtypes.contains(&Subtype::Attraction)
            && let Some(program) = &def.spell_effect
        {
            let mut visit = Ability::triggered(
                crate::triggers::Trigger::keyword_action_from_source(
                    crate::events::KeywordActionKind::VisitAttraction,
                    crate::target::PlayerFilter::Any,
                ),
                Vec::new(),
            );
            if let crate::ability::AbilityKind::Triggered(trigger) = &mut visit.kind {
                trigger.effects = program.clone();
                trigger.presentation_label = Some(ironsmith_core::PresentationLabel::AbilityWord(
                    "Visit".to_string(),
                ));
            }
            abilities.push(visit);
        }
        Self {
            definition: Arc::new(def.clone()),
            name: def.card.name.clone().into(),
            first_printed_set_name: def.card.first_printed_set_name.clone().map(Into::into),
            mana_cost: shared_optional_value(def.card.mana_cost.clone()),
            supertypes: def.card.supertypes.clone().into(),
            card_types: def.card.card_types.clone().into(),
            subtypes: def.card.subtypes.clone().into(),
            compiled_card_text: Object::compiled_display_text(def),
            ability_labels: Object::display_ability_labels(def),
            other_face_name: def.card.other_face_name.clone().map(Into::into),
            abilities: Arc::new(abilities),
            spell_effect: shared_optional_value(def.spell_effect.clone()),
            aura_attach_filter: def.aura_attach_filter.clone().map(Into::into),
            alternative_casts: def.alternative_casts.clone().into(),
            optional_costs: def.optional_costs.clone().into(),
            additional_cost: def.additional_cost.clone().into(),
        }
    }
}

/// The kind of game object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
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
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
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

/// Attachment metadata owns its legacy enchant ability occurrence. Creating
/// the metadata registers the payload; repeated layer reads only clone it.
#[derive(Debug, Clone, PartialEq)]
pub struct AuraAttachmentMetadata {
    filter: SharedValue<AuraAttachmentFilter>,
    enchant_ability: StaticAbility,
}

impl From<AuraAttachmentFilter> for AuraAttachmentMetadata {
    fn from(filter: AuraAttachmentFilter) -> Self {
        Self {
            enchant_ability: StaticAbility::enchant(filter.clone()),
            filter: filter.into(),
        }
    }
}

impl std::ops::Deref for AuraAttachmentMetadata {
    type Target = AuraAttachmentFilter;
    fn deref(&self) -> &Self::Target {
        &self.filter
    }
}

impl AuraAttachmentMetadata {
    pub fn to_owned_value(&self) -> AuraAttachmentFilter {
        self.filter.to_owned_value()
    }
    pub(crate) fn enchant_ability(&self) -> StaticAbility {
        self.enchant_ability.clone()
    }
}

/// Complete attachment payload including its retained enchant occurrence.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct RetainedAuraAttachmentMetadata<S> {
    pub filter: AuraAttachmentFilter,
    pub enchant_ability: S,
}

impl<S> RetainedAuraAttachmentMetadata<S> {
    pub fn try_map_ability<T, Error>(
        self,
        mut map: impl FnMut(S) -> Result<T, Error>,
    ) -> Result<RetainedAuraAttachmentMetadata<T>, Error> {
        let Self {
            filter,
            enchant_ability,
        } = self;
        Ok(RetainedAuraAttachmentMetadata {
            filter,
            enchant_ability: map(enchant_ability)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuraAttachmentAbilityMismatch;
impl std::fmt::Display for AuraAttachmentAbilityMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("attachment filter does not agree with its enchant ability")
    }
}
impl std::error::Error for AuraAttachmentAbilityMismatch {}

impl From<AuraAttachmentMetadata> for RetainedAuraAttachmentMetadata<StaticAbility> {
    fn from(value: AuraAttachmentMetadata) -> Self {
        let AuraAttachmentMetadata {
            filter,
            enchant_ability,
        } = value;
        Self {
            filter: filter.to_owned_value(),
            enchant_ability,
        }
    }
}

impl TryFrom<RetainedAuraAttachmentMetadata<StaticAbility>> for AuraAttachmentMetadata {
    type Error = AuraAttachmentAbilityMismatch;
    fn try_from(value: RetainedAuraAttachmentMetadata<StaticAbility>) -> Result<Self, Self::Error> {
        let RetainedAuraAttachmentMetadata {
            filter,
            enchant_ability,
        } = value;
        if enchant_ability.id() != StaticAbilityId::Enchant
            || !enchant_ability
                .compiled_model()
                .is_some_and(|model| model.id == Some(StaticAbilityId::Enchant))
            || enchant_ability.enchant_filter() != Some(&filter)
        {
            return Err(AuraAttachmentAbilityMismatch);
        }
        Ok(Self {
            filter: filter.into(),
            enchant_ability,
        })
    }
}
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
    pub aura_attach_filter: Option<AuraAttachmentMetadata>,
    pub spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
}

/// Object-owned original program restored when a stack-only program overlay ends.
/// The historical carrier name is retained for the saved-state schema.
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
    /// The printed line each entry of `abilities` reads as, when known.
    ///
    /// Aligned one-to-one with `abilities`; empty when no alignment is known,
    /// in which case `compiled_card_text` is consulted line by line only if it
    /// has exactly one line per ability.
    pub ability_labels: SharedVec<String>,
    pub rules_text_color_identity: ColorSet,
    pub base_power: Option<PtValue>,
    pub base_toughness: Option<PtValue>,
    pub base_loyalty: Option<u32>,
    pub base_defense: Option<u32>,
    pub abilities: Arc<Vec<Ability>>,
    pub spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
    pub aura_attach_filter: Option<AuraAttachmentMetadata>,
    pub optional_costs: SharedVec<OptionalCost>,
    pub additional_cost: SharedValue<TotalCost>,
    /// Public face-down kind: this object was cast face down using disguise,
    /// so the face-down overlay carries ward {2} (CR 702.168a).
    ///
    /// This is recorded when the overlay is applied and never re-derived from
    /// the hidden abilities above: peers that hold a hidden-card placeholder
    /// must agree on the face-down characteristics without knowing them.
    pub disguise_ward: bool,
}

/// Stored copiable fields of a permanent that entered the battlefield as a
/// copy of another object. The copy effect belongs to that permanent only, so
/// once it leaves the battlefield the card is its printed self again: a Clone
/// that was copying Grave Titan is just Clone in the graveyard (CR 707.2,
/// 400.7).
#[derive(Debug, Clone)]
pub struct EntersAsCopyRestoreState {
    pub printed: FaceDownCastState,
    pub other_face: Option<crate::ids::CardId>,
    pub other_face_name: Option<SharedStr>,
    pub linked_face_layout: LinkedFaceLayout,
    pub has_fuse: bool,
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

/// The combined characteristics of a split card's two halves (CR 709.4a-d),
/// precomputed so accessors can hand out slices.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitCombinedCharacteristics {
    /// Name of the half that isn't currently shown.
    pub other_half_name: SharedStr,
    /// Colors of the half that isn't currently shown.
    pub other_half_colors: ColorSet,
    pub card_types: SharedVec<CardType>,
    pub subtypes: SharedVec<Subtype>,
    pub supertypes: SharedVec<Supertype>,
}

impl SplitCombinedCharacteristics {
    /// Combine the shown half (`own`) with the other half's characteristics.
    pub fn from_halves(own: &Object, other: &Object) -> Self {
        fn union<T: Clone + PartialEq>(left: &[T], right: &[T]) -> Vec<T> {
            let mut combined = left.to_vec();
            for item in right {
                if !combined.contains(item) {
                    combined.push(item.clone());
                }
            }
            combined
        }
        Self {
            other_half_name: other.name.clone(),
            other_half_colors: other.own_colors(),
            card_types: union(&own.card_types, &other.card_types).into(),
            subtypes: union(&own.subtypes, &other.subtypes).into(),
            supertypes: union(&own.supertypes, &other.supertypes).into(),
        }
    }
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
    /// Initial controller, before layer-two control modifications. Not copiable.
    /// On the stack this records the caster; a resolving permanent retains it.
    pub initial_controller: PlayerId,

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
    /// The printed line each entry of `abilities` reads as, when known.
    ///
    /// Aligned one-to-one with `abilities`; empty when no alignment is known,
    /// in which case `compiled_card_text` is consulted line by line only if it
    /// has exactly one line per ability.
    pub ability_labels: SharedVec<String>,
    pub rules_text_color_identity: ColorSet,
    /// Optional reference to another face for flip/DFC style cards.
    ///
    /// This is copied from `Card::other_face` when the object is created.
    pub other_face: Option<CardId>,
    /// Linked face name for on-demand compilation without a global registry preload.
    pub other_face_name: Option<SharedStr>,
    /// Layout semantics for linked-face cards.
    pub linked_face_layout: LinkedFaceLayout,
    /// Mana cost of the linked face that isn't currently shown: the other half
    /// of a split card, or the front face of a transforming double-faced card
    /// whose back face is up. Only mana value reads it (CR 709.4, 712.8c/e);
    /// it's not a copiable value, so a copy of a back face has mana value 0.
    pub linked_face_mana_cost: Option<SharedValue<ManaCost>>,
    /// Both halves of a split card combined (CR 709.4): outside the stack and
    /// the battlefield a split card has the characteristics of both halves.
    /// Read through the zone-gated accessors (`split_combined_active`).
    pub split_combined: Option<SharedValue<SplitCombinedCharacteristics>>,
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
    pub counters: ObjectCounters,
    pub attached_to: Option<AttachmentTarget>,
    pub attachments: Vec<ObjectId>,

    // Spell-related state
    /// Spell effects (for instants/sorceries)
    pub spell_effect: Option<SharedValue<crate::resolution::ResolutionProgram>>,
    /// Original program before splice or another stack-only program overlay.
    pub splice_cast_state: Option<Box<SpliceCastState>>,
    /// For Auras: what this card can enchant (used for non-target attachments)
    pub aura_attach_filter: Option<AuraAttachmentMetadata>,
    /// Original copiable fields to restore if this permanent ends bestow.
    pub bestow_cast_state: Option<Box<BestowCastState>>,
    /// Original copiable fields to restore if this card was cast face down.
    pub face_down_cast_state: Option<Box<FaceDownCastState>>,
    /// Original copiable fields to restore if this card was cast prototyped.
    pub prototype_cast_state: Option<PrototypeCastState>,
    /// Printed copiable fields overwritten when this permanent entered as a
    /// copy (or with other "enters as" characteristic changes), restored when
    /// it leaves the battlefield (CR 707.2, 400.7).
    pub enters_as_copy_restore_state: Option<Box<EntersAsCopyRestoreState>>,
    /// Alternative casting methods (flashback, escape, etc.)
    pub alternative_casts: SharedVec<AlternativeCastingMethod>,
    /// Alternative method chosen for the current spell cast.
    pub cast_alternative_method: Option<Box<AlternativeCastingMethod>>,
    /// Permission constraints captured before the card leaves its casting zone.
    /// The grant itself expires on that zone change, but its cost rules apply
    /// to the proposed spell through total-cost calculation and payment.
    pub cast_play_from_constraints:
        Option<Box<(ObjectId, Zone, crate::grant_registry::PlayFromConstraints)>>,
    /// Once-turn permission captured before movement and retained through payment.
    pub cast_grant_usage_identity: Option<Box<crate::grant_registry::GrantPermissionIdentity>>,
    pub cast_price:
        Option<Box<CastPriceReceipt<TotalCost, crate::grant_registry::GrantPermissionIdentity>>>,
    /// Exact native casting authority and payer captured before any cost is paid.
    pub cast_play_permission: Option<Box<crate::alternative_cast::play_permission::PlayPermissionReceipt>>,
    /// True if this split card can be cast fused from hand.
    pub has_fuse: bool,
    /// Optional costs (kicker, buyback, etc.)
    pub optional_costs: SharedVec<OptionalCost>,
    /// Which optional costs were paid when this spell was cast (for ETB triggers)
    pub optional_costs_paid: OptionalCostsPaid,
    /// Mana actually spent to cast this object while it was a spell.
    /// Used by conditional text like "if at least three blue mana was spent to cast this spell".
    pub mana_spent_to_cast: ManaPool,
    /// Actual mana spent by the caster, excluding Assist payments by others.
    pub caster_mana_spent_to_cast: Option<u32>,
    /// None is unknown historical evidence, never an implicit zero payment.
    pub mana_spent_on_x: Option<crate::mana::XManaAllocation>,
    /// Mana spent from sources that were snow when they produced it, by actual color.
    pub snow_mana_spent_to_cast: ManaPool,
    /// Non-copiable static abilities granted until end of turn while this object is a spell or
    /// permanent. Stack-to-battlefield movement preserves these grants for the permanent that
    /// spell becomes; other zone changes clear them.
    pub temporary_static_ability_grants: TemporaryStaticAbilityGrants,
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

/// Deterministic registration identity, independent of refresh order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct TemporaryAbilityOrigin {
    source: ObjectId,
    serial: u64,
    /// A live grant's acquisition time in the game's continuous-effect clock.
    /// Pre-entry assembly grants use the entering object's timestamp instead.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Option::is_none"))]
    acquired_at: Option<u64>,
}

impl TemporaryAbilityOrigin {
    pub(crate) fn acquired_at(&self) -> Option<u64> {
        self.acquired_at
    }
}

/// Temporary grants paired with stable origins. Read access cannot detach
/// a grant from its identity; push always registers a new occurrence.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporaryStaticAbilityGrants {
    source: ObjectId,
    next_serial: u64,
    grants: Vec<TemporaryStaticAbilityGrant>,
    origins: Vec<TemporaryAbilityOrigin>,
}
impl TemporaryStaticAbilityGrants {
    pub fn new(source: ObjectId) -> Self {
        Self {
            source,
            next_serial: 0,
            grants: Vec::new(),
            origins: Vec::new(),
        }
    }
    pub fn origin(&self, index: usize) -> Option<&TemporaryAbilityOrigin> {
        self.origins.get(index)
    }
    /// Register during object assembly, before the object receives its zone
    /// timestamp. Live GameState grants must use `push_at_timestamp`.
    pub fn push(&mut self, grant: TemporaryStaticAbilityGrant) {
        self.register(grant, None);
    }
    pub(crate) fn push_at_timestamp(&mut self, grant: TemporaryStaticAbilityGrant, timestamp: u64) {
        self.register(grant, Some(timestamp));
    }
    fn register(&mut self, mut grant: TemporaryStaticAbilityGrant, acquired_at: Option<u64>) {
        // A registered keyword is one runtime ability occurrence. Materialize
        // its payload at registration so layer/query reads clone that ability
        // rather than allocate a new instance on each calculation.
        if grant.ability_payload.is_none() {
            grant.ability_payload = static_ability_from_id(grant.ability);
        }
        let serial = self.next_serial;
        self.next_serial = serial
            .checked_add(1)
            .expect("temporary ability identity exhausted");
        self.grants.push(grant);
        self.origins.push(TemporaryAbilityOrigin {
            source: self.source,
            serial,
            acquired_at,
        });
    }
    pub fn clear(&mut self) {
        self.grants.clear();
        self.origins.clear();
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&TemporaryStaticAbilityGrant) -> bool) {
        let mut retained = Vec::new();
        let mut index = 0;
        self.grants.retain(|grant| {
            let retain = keep(grant);
            if retain {
                retained.push(self.origins[index].clone());
            }
            index += 1;
            retain
        });
        self.origins = retained;
    }
    pub(crate) fn empty_with_allocator(&self) -> Self {
        Self {
            grants: Vec::new(),
            origins: Vec::new(),
            ..self.clone()
        }
    }
    /// Merge reconstruction retains component registrations, not new grants.
    pub(crate) fn extend_existing(&mut self, other: &Self) {
        for (grant, origin) in other.grants.iter().zip(&other.origins) {
            if origin.source == self.source {
                self.next_serial = self.next_serial.max(
                    origin
                        .serial
                        .checked_add(1)
                        .expect("temporary ability identity exhausted"),
                );
            }
            self.grants.push(grant.clone());
            self.origins.push(origin.clone());
        }
    }
}
impl std::ops::Deref for TemporaryStaticAbilityGrants {
    type Target = [TemporaryStaticAbilityGrant];
    fn deref(&self) -> &Self::Target {
        &self.grants
    }
}
impl<'a> IntoIterator for &'a TemporaryStaticAbilityGrants {
    type Item = &'a TemporaryStaticAbilityGrant;
    type IntoIter = std::slice::Iter<'a, TemporaryStaticAbilityGrant>;
    fn into_iter(self) -> Self::IntoIter {
        self.grants.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemporaryStaticAbilityGrant {
    pub ability: StaticAbilityId,
    pub ability_payload: Option<StaticAbility>,
    /// None lasts for this incarnation, including Stack -> Battlefield.
    pub expires_end_of_turn: Option<u32>,
}

impl TemporaryStaticAbilityGrant {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        self.expires_end_of_turn
            .is_some_and(|end| current_turn > end)
    }

    pub fn materialize(&self) -> Option<StaticAbility> {
        self.ability_payload
            .clone()
            .or_else(|| static_ability_from_id(self.ability))
    }
}

fn static_ability_from_id(ability: StaticAbilityId) -> Option<StaticAbility> {
    scalar_temporary_ability_constructor(ability).map(|constructor| constructor())
}

fn scalar_temporary_ability_constructor(ability: StaticAbilityId) -> Option<fn() -> StaticAbility> {
    match ability {
        StaticAbilityId::Deathtouch => Some(StaticAbility::deathtouch),
        StaticAbilityId::DoubleStrike => Some(StaticAbility::double_strike),
        StaticAbilityId::FirstStrike => Some(StaticAbility::first_strike),
        StaticAbilityId::Flying => Some(StaticAbility::flying),
        StaticAbilityId::Haste => Some(StaticAbility::haste),
        StaticAbilityId::Hexproof => Some(StaticAbility::hexproof),
        StaticAbilityId::Indestructible => Some(StaticAbility::indestructible),
        StaticAbilityId::Lifelink => Some(StaticAbility::lifelink),
        StaticAbilityId::Menace => Some(StaticAbility::menace),
        StaticAbilityId::Reach => Some(StaticAbility::reach),
        StaticAbilityId::Trample => Some(StaticAbility::trample),
        StaticAbilityId::Vigilance => Some(StaticAbility::vigilance),
        StaticAbilityId::ReadAhead => Some(StaticAbility::read_ahead),
        _ => None,
    }
}
impl Object {
    /// The X this object's own entering ability or replacement effect sees
    /// (CR 107.3m): the value chosen as it was cast, or 0 when it has X in
    /// its mana cost but entered without being cast. `None` when its mana
    /// cost has no X and nothing announced one.
    pub fn own_entry_x_value(&self) -> Option<u32> {
        self.x_value.or_else(|| {
            self.mana_cost
                .as_ref()
                .is_some_and(|cost| cost.has_x())
                .then_some(0)
        })
    }

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

    fn display_ability_labels(def: &crate::cards::CardDefinition) -> SharedVec<String> {
        crate::runtime_display::definition_ability_labels(def).into()
    }

    /// The printed line ability `index` reads as, when the object knows it.
    ///
    /// Labels carried from the definition win; without them the compiled text
    /// is indexed only when it has exactly one line per ability, since any
    /// other shape would attribute a neighbour's wording to the ability.
    pub fn ability_label(&self, index: usize) -> Option<String> {
        crate::runtime_display::aligned_ability_label(
            &self.ability_labels,
            &self.compiled_card_text,
            self.abilities.len(),
            index,
        )
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

    /// Mana value contributed by the linked face, when it replaces the value
    /// computed from `mana_cost` alone:
    /// - a split card outside the stack has the combined mana value of both
    ///   halves (CR 709.4);
    /// - a transforming double-faced card with its back face up has the mana
    ///   value of its front face (CR 712.8c, 712.8e);
    /// - a melded permanent (no linked-face layout, no mana cost of its own)
    ///   has the summed mana value of its two front faces (CR 712.8g). A copy
    ///   of it doesn't copy this, so its mana value is 0.
    pub fn linked_face_mana_value(&self) -> Option<u32> {
        let linked = self.linked_face_mana_cost.as_deref()?;
        match self.linked_face_layout {
            LinkedFaceLayout::None if self.mana_cost.is_none() => Some(linked.mana_value()),
            LinkedFaceLayout::Split if !matches!(self.zone, Zone::Stack | Zone::Battlefield) => {
                let own = self.mana_cost.as_deref().map_or(0, ManaCost::mana_value);
                Some(own + linked.mana_value())
            }
            LinkedFaceLayout::TransformLike if self.mana_cost.is_none() => {
                Some(linked.mana_value())
            }
            _ => None,
        }
    }

    pub fn spell_effect_owned(&self) -> Option<crate::resolution::ResolutionProgram> {
        owned_optional_value(&self.spell_effect)
    }

    pub fn aura_attach_filter_owned(&self) -> Option<AuraAttachmentFilter> {
        self.aura_attach_filter
            .as_ref()
            .map(AuraAttachmentMetadata::to_owned_value)
    }

    fn abilities_with_enchant_metadata(
        &self,
        metadata: Option<&AuraAttachmentMetadata>,
    ) -> Arc<Vec<Ability>> {
        let Some(metadata) = metadata else {
            return self.abilities.clone();
        };
        if self.abilities.iter().any(|ability| {
            matches!(
                &ability.kind, crate::ability::AbilityKind::Static(ability)
                    if ability.enchant_filter() == Some(&*metadata.filter)
            )
        }) {
            return self.abilities.clone();
        }
        let mut abilities = self.abilities.as_ref().clone();
        abilities.push(Ability::static_ability(metadata.enchant_ability.clone()));
        Arc::new(abilities)
    }

    pub(crate) fn materialized_text_box_abilities(&self) -> Arc<Vec<Ability>> {
        self.abilities_with_enchant_metadata(self.aura_attach_filter.as_ref())
    }

    pub(crate) fn materialized_copiable_abilities(&self) -> Arc<Vec<Ability>> {
        let metadata = self
            .bestow_cast_state
            .as_ref()
            .map_or(self.aura_attach_filter.as_ref(), |restore| {
                restore.aura_attach_filter.as_ref()
            });
        self.abilities_with_enchant_metadata(metadata)
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
            initial_controller: owner,
            name: card.name.clone().into(),
            first_printed_set_name: card.first_printed_set_name.clone().map(Into::into),
            mana_cost: shared_optional_value(card.mana_cost.clone()),
            color_override: card.color_indicator,
            supertypes: card.supertypes.clone().into(),
            card_types: card.card_types.clone().into(),
            subtypes: card.subtypes.clone().into(),
            compiled_card_text: Arc::from(""),
            ability_labels: Default::default(),
            rules_text_color_identity: card.rules_text_color_identity,
            other_face: card.other_face,
            other_face_name: card.other_face_name.clone().map(Into::into),
            linked_face_layout: card.linked_face_layout,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power,
            base_toughness,
            base_loyalty: card.loyalty,
            base_defense: card.defense,
            hand_modifier: card.hand_modifier,
            life_modifier: card.life_modifier,
            abilities: Arc::new(Vec::new()),
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
            initial_controller: owner,
            name: "Hidden Card".into(),
            first_printed_set_name: None,
            mana_cost: None,
            color_override: None,
            supertypes: Vec::new().into(),
            card_types: Vec::new().into(),
            subtypes: Vec::new().into(),
            compiled_card_text: Arc::from(""),
            ability_labels: Default::default(),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: None,
            base_toughness: None,
            base_loyalty: None,
            base_defense: None,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: Arc::new(Vec::new()),
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
        let initial_controller = self.initial_controller;
        let zone = self.zone;
        *self = Self::new_hidden_card(id, owner, zone);
        self.stable_id = stable_id;
        self.initial_controller = initial_controller;
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
        self.spell_effect = handles.spell_effect.clone();
        self.aura_attach_filter = handles.aura_attach_filter.clone();
        self.alternative_casts = handles.alternative_casts.clone();
        self.has_fuse = def.has_fuse;
        self.optional_costs = handles.optional_costs.clone();
        self.additional_cost = handles.additional_cost.clone();
    }

    /// Remove or reinstate a prospective entry presentation without replacing
    /// physical identity, zone, controller, counters, damage or attachments.
    /// This is the exact field set changed by entry face/face-down overlays;
    /// using a whole-object restore would erase already reserved entry costs.
    pub(crate) fn restore_entry_presentation_from(&mut self, original: &Object) {
        macro_rules! restore {
            ($($field:ident),* $(,)?) => { $(self.$field = original.$field.clone();)* };
        }
        restore!(name, first_printed_set_name, mana_cost, color_override, supertypes,
            card_types, subtypes, compiled_card_text, ability_labels, rules_text_color_identity,
            other_face, other_face_name, linked_face_layout, linked_face_mana_cost,
            base_power, base_toughness, base_loyalty, base_defense, hand_modifier, life_modifier,
            abilities, spell_effect, aura_attach_filter, bestow_cast_state, face_down_cast_state,
            prototype_cast_state, alternative_casts, cast_alternative_method, has_fuse,
            optional_costs, additional_cost, split_combined);
    }

    /// Apply the printed/copied characteristics of another card definition.
    ///
    /// Used for flip cards and similar "becomes this other face" mechanics.
    /// This preserves identity, ownership, controller, zone, counters, and attachments.
    pub fn apply_definition_face(&mut self, def: &crate::cards::CardDefinition) {
        let handles = CardSharedHandles::from_definition(def);
        self.apply_definition_face_with_shared(def, &handles);
    }

    /// Turn a flip card to its flipped half. The flipped half's name, text,
    /// types and power/toughness apply, but the card's color and mana cost
    /// don't change (CR 710.1b-c).
    pub fn apply_flipped_face(&mut self, def: &crate::cards::CardDefinition) {
        let mana_cost = self.mana_cost.clone();
        let color_override = self.color_override;
        self.apply_definition_face(def);
        self.mana_cost = mana_cost;
        self.color_override = color_override;
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

        // Turning to the linked face keeps the face being hidden as the linked
        // face's mana cost, which mana value reads (CR 709.4, 712.8c).
        let turning_to_linked_face = self.other_face.is_some_and(|id| id == def.card.id)
            || self
                .other_face_name
                .as_deref()
                .is_some_and(|name| name == def.card.name.as_str());
        let hidden_split_half = (turning_to_linked_face
            && def.card.linked_face_layout == LinkedFaceLayout::Split)
            .then(|| self.clone());
        if turning_to_linked_face {
            self.linked_face_mana_cost = self.mana_cost.take();
        }
        self.name = handles.name.clone();
        self.first_printed_set_name = handles.first_printed_set_name.clone();
        self.mana_cost = handles.mana_cost.clone();
        self.color_override = def.card.color_indicator;
        self.supertypes = handles.supertypes.clone();
        self.card_types = handles.card_types.clone();
        self.subtypes = handles.subtypes.clone();
        self.compiled_card_text = handles.compiled_card_text.clone();
        self.ability_labels = handles.ability_labels.clone();
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
        // CR 709.4: keep the combined characteristics in step with the half
        // now shown (the half being hidden becomes the "other" half).
        if let Some(hidden) = hidden_split_half {
            self.split_combined =
                Some(SplitCombinedCharacteristics::from_halves(self, &hidden).into());
        } else if self.linked_face_layout != LinkedFaceLayout::Split {
            self.split_combined = None;
        }
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
                // The modal/transforming distinction is read from the linked
                // face's registry definition, not carried on objects.
                transforming_dfc: false,
                is_token: matches!(self.kind, ObjectKind::Token),
            },
            canonical_text: self.compiled_card_text.to_string(),
            ability_labels: self.ability_labels.to_vec(),
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
            initial_controller: owner,
            name: name.into(),
            first_printed_set_name: None,
            mana_cost: None,
            color_override: Some(color),
            supertypes: Vec::new().into(),
            card_types: card_types.into(),
            subtypes: subtypes.into(),
            compiled_card_text: Arc::from(""),
            ability_labels: Default::default(),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: power.map(PtValue::Fixed),
            base_toughness: toughness.map(PtValue::Fixed),
            base_loyalty: None,
            base_defense: None,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: Arc::new(Vec::new()),
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
            initial_controller: owner,
            // Copiable values from source
            name: source.name.clone(),
            first_printed_set_name: source.first_printed_set_name.clone(),
            mana_cost: source.mana_cost.clone(),
            color_override: source.color_override,
            supertypes: source.supertypes.clone(),
            card_types,
            subtypes,
            compiled_card_text: source.compiled_card_text.clone(),
            ability_labels: source.ability_labels.clone(),
            rules_text_color_identity: source.rules_text_color_identity,
            other_face: source.other_face,
            other_face_name: source.other_face_name.clone(),
            linked_face_layout: source.linked_face_layout,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: source.base_power,
            base_toughness: source.base_toughness,
            base_loyalty: source.base_loyalty,
            base_defense: source.base_defense,
            hand_modifier: source.hand_modifier,
            life_modifier: source.life_modifier,
            abilities: source.abilities.clone(),
            // Non-copiable values reset to defaults
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            // Note: spell_effect is copiable for spell copies
            spell_effect,
            splice_cast_state: None,
            aura_attach_filter,
            bestow_cast_state: None,
            face_down_cast_state: source.face_down_cast_state.clone(),
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
            // Alternative casts are copiable (though tokens rarely use them)
            alternative_casts: source.alternative_casts.clone(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: source.has_fuse,
            // Optional costs are copiable
            optional_costs: source.optional_costs.clone(),
            // Optional costs paid is non-copiable (tokens weren't cast)
            optional_costs_paid: OptionalCostsPaid::default(),
            // Tokens are never cast.
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
            initial_controller: owner,
            name: source.name.clone(),
            first_printed_set_name: source.first_printed_set_name.clone(),
            mana_cost: source.mana_cost.clone(),
            color_override: source.color_override,
            supertypes: source.supertypes.clone(),
            card_types: source.card_types.clone(),
            subtypes: source.subtypes.clone(),
            compiled_card_text: source.compiled_card_text.clone(),
            ability_labels: source.ability_labels.clone(),
            rules_text_color_identity: source.rules_text_color_identity,
            other_face: source.other_face,
            other_face_name: source.other_face_name.clone(),
            linked_face_layout: source.linked_face_layout,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: source.base_power,
            base_toughness: source.base_toughness,
            base_loyalty: source.base_loyalty,
            base_defense: source.base_defense,
            hand_modifier: source.hand_modifier,
            life_modifier: source.life_modifier,
            abilities: source.abilities.clone(),
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: source.spell_effect.clone(),
            splice_cast_state: source.splice_cast_state.clone(),
            aura_attach_filter: source.aura_attach_filter.clone(),
            bestow_cast_state: source.bestow_cast_state.clone(),
            face_down_cast_state: source.face_down_cast_state.clone(),
            prototype_cast_state: source.prototype_cast_state.clone(),
            enters_as_copy_restore_state: source.enters_as_copy_restore_state.clone(),
            alternative_casts: source.alternative_casts.clone(),
            cast_alternative_method: source.cast_alternative_method.clone(),
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: source.has_fuse,
            optional_costs: source.optional_costs.clone(),
            optional_costs_paid: source.optional_costs_paid.clone(),
            // CR 707.10: mana isn't an object, so a copy of a spell has no mana
            // spent to cast it (converge, adamant, "if {G} was spent" read 0).
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            // CR 707.2/707.10: copy the spell's layer-one definition and
            // casting choices, not later ability-grant effects. Duration does
            // not make a grant copiable. Dash/Blitz reconstruct their riders
            // from the copied casting choice when the permanent resolves.
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
        copy.optional_costs_paid.clear_uncopied_cast_facts();
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
            initial_controller: owner,
            name: copiable.name.clone().into(),
            first_printed_set_name: snapshot.first_printed_set_name.clone().map(Into::into),
            mana_cost: shared_optional_value(copiable.mana_cost.clone()),
            color_override: (!copiable.colors.is_empty()).then_some(copiable.colors),
            supertypes: copiable.supertypes.clone().into(),
            card_types: copiable.card_types.clone().into(),
            subtypes: copiable.subtypes.clone().into(),
            compiled_card_text: Arc::from(copiable.compiled_card_text.as_str()),
            ability_labels: copiable.ability_labels.clone().into(),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: snapshot.other_face,
            other_face_name: snapshot.other_face_name.clone().map(Into::into),
            linked_face_layout: snapshot.linked_face_layout,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: copiable.power.map(PtValue::Fixed),
            base_toughness: copiable.toughness.map(PtValue::Fixed),
            base_loyalty: copiable.loyalty,
            base_defense: snapshot.defense,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: copiable.abilities.clone(),
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: copiable.aura_attach_filter.clone().map(Into::into),
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
            initial_controller: owner,
            name: name.into(),
            first_printed_set_name: None,
            mana_cost: None,
            color_override: None,
            supertypes: Vec::new().into(),
            card_types: Vec::new().into(),
            subtypes: Vec::new().into(),
            compiled_card_text: Arc::from(""),
            ability_labels: Default::default(),
            rules_text_color_identity: ColorSet::COLORLESS,
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: None,
            base_toughness: None,
            base_loyalty: None,
            base_defense: None,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: Arc::new(abilities),
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
            alternative_casts: Vec::new().into(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: false,
            optional_costs: Vec::new().into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
        // Only the face that's up is copied (CR 707.8). Whether the permanent
        // can transform depends on the entering card itself, not on what it
        // copies: a Clone copying Delver of Secrets isn't a double-faced card,
        // so it can't transform (CR 701.27a/c). Token copies keep both faces
        // separately (`token_copy_of`, CR 707.8a). A split permanent (a Room)
        // is copied with both halves (CR 709.5, 709.5b), so it keeps the link.
        if source.linked_face_layout == LinkedFaceLayout::Split {
            self.other_face = source.other_face;
            self.other_face_name = source.other_face_name.clone();
            self.linked_face_layout = source.linked_face_layout;
        }
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
        // A permanent copy receives the copied face's cost rather than the
        // source card's noncopiable linked-face mana-value contribution.
        self.linked_face_mana_cost = None;
        self.color_override = Some(values.colors);
        self.supertypes = values.supertypes.clone().into();
        self.card_types = values.card_types.clone().into();
        self.subtypes = values.subtypes.clone().into();
        self.compiled_card_text = values.compiled_card_text.clone().into();
        self.ability_labels = values.ability_labels.clone().into();
        self.base_power = values.power.map(PtValue::Fixed);
        self.base_toughness = values.toughness.map(PtValue::Fixed);
        self.base_loyalty = values.loyalty;
        self.base_defense = values.defense;
        self.abilities = values.abilities.clone();
        self.spell_effect = match &values.spell_effect {
            crate::snapshot::SpellProgramState::Present(program) => Some(program.clone().into()),
            crate::snapshot::SpellProgramState::Absent => None,
            crate::snapshot::SpellProgramState::Unavailable => Some(
                crate::resolution::ResolutionProgram::unavailable_copied_definition().into()),
        };
        self.aura_attach_filter = values.aura_attach_filter.clone().map(Into::into);
    }

    /// Install a frozen stack envelope while keeping a Bestow overlay's
    /// synthesized enchant occurrence in its metadata owner. Materializing
    /// that exact occurrence into the raw ability list would outlive Bestow.
    /// Independent printed/copied enchant occurrences remain untouched.
    pub(crate) fn copy_spell_values_from_values(&mut self, values: &CopiableValues) {
        let overlay_enchant = self.bestow_cast_state.as_ref().and_then(|_| {
            self.aura_attach_filter.as_ref().map(|metadata| metadata.enchant_ability.instance_id())
        });
        let mut values = values.clone();
        if let Some(overlay_enchant) = overlay_enchant {
            Arc::make_mut(&mut values.abilities).retain(|ability| !matches!(&ability.kind,
                crate::ability::AbilityKind::Static(ability) if ability.instance_id() == overlay_enchant));
        }
        self.copy_copiable_values_from_values(&values);
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

    /// Save this object's native program before its first stack-only modification.
    /// Further modifications retain the earliest snapshot, including its native
    /// executable identities; CardId cache entries are never restoration data.
    pub fn begin_stack_program_overlay(&mut self) -> bool {
        if self.splice_cast_state.is_some() {
            return false;
        }
        self.splice_cast_state = Some(Box::new(SpliceCastState {
            spell_effect: self.spell_effect.clone(),
        }));
        true
    }

    /// Restore this object's saved program exactly once when the overlay ends.
    pub fn end_stack_program_overlay(&mut self) -> bool {
        let Some(restore) = self.splice_cast_state.take() else {
            return false;
        };
        self.spell_effect = restore.spell_effect;
        true
    }

    /// Begin a splice modification, preserving any earlier stack program snapshot.
    pub fn begin_splice_cast_overlay(&mut self) -> bool {
        self.begin_stack_program_overlay()
    }

    /// Restore the object-owned program saved before its stack modifications.
    pub fn end_splice_cast_overlay(&mut self) -> bool {
        self.end_stack_program_overlay()
    }

    /// Whether this object's own (possibly hidden) abilities include disguise.
    ///
    /// Only the casting engine may consult this, at the moment it chooses the
    /// disguise casting permission; the result is then recorded publicly via
    /// [`Self::apply_face_down_cast_overlay_with_disguise_ward`].
    pub fn has_disguise_ability(&self) -> bool {
        self.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.is_disguise()
            )
        })
    }

    /// Apply the shared face-down overlay (a 2/2 nameless colorless creature,
    /// CR 708.2) without any cast-kind extras. Manifest, "turn face down", and
    /// "put onto the battlefield face down" effects use this: none of them
    /// grant disguise's ward, and none may depend on the hidden abilities.
    pub fn apply_face_down_cast_overlay(&mut self) -> bool {
        self.apply_face_down_cast_overlay_with_disguise_ward(false)
    }

    /// Apply the face-down overlay for a face-down cast. `disguise_ward` is the
    /// public cast kind (cast using disguise, CR 702.168a); it is stored on the
    /// face-down state so every peer derives the same ward {2} regardless of
    /// whether it knows the card's real abilities.
    pub fn apply_face_down_cast_overlay_with_disguise_ward(&mut self, disguise_ward: bool) -> bool {
        if self.face_down_cast_state.is_some() {
            return false;
        }

        self.face_down_cast_state = Some(Box::new(FaceDownCastState {
            name: self.name.clone(),
            first_printed_set_name: self.first_printed_set_name.clone(),
            mana_cost: self.mana_cost.clone(),
            color_override: self.color_override,
            supertypes: self.supertypes.clone(),
            card_types: self.card_types.clone(),
            subtypes: self.subtypes.clone(),
            compiled_card_text: self.compiled_card_text.clone(),
            ability_labels: self.ability_labels.clone(),
            rules_text_color_identity: self.rules_text_color_identity,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
            base_loyalty: self.base_loyalty,
            base_defense: self.base_defense,
            abilities: self.abilities.clone(),
            spell_effect: self.spell_effect.clone(),
            aura_attach_filter: self.aura_attach_filter.clone(),
            optional_costs: self.optional_costs.clone(),
            additional_cost: self.additional_cost.clone(),
            disguise_ward,
        }));

        self.name = FACE_DOWN_DISPLAY_NAME.into();
        self.first_printed_set_name = None;
        self.mana_cost = None;
        self.color_override = Some(ColorSet::COLORLESS);
        self.supertypes.clear();
        self.card_types = vec![CardType::Creature].into();
        self.subtypes.clear();
        self.compiled_card_text = Arc::from("");
        self.ability_labels = Default::default();
        self.base_power = Some(PtValue::Fixed(2));
        self.base_toughness = Some(PtValue::Fixed(2));
        self.base_loyalty = None;
        self.base_defense = None;
        // CR 708.2: a face-down permanent has no abilities. Morph, megamorph,
        // and disguise are not kept either: every peer (including those that
        // hold only a hidden-card placeholder) must derive the same ability
        // list, or ability-sensitive effects ("creatures with no abilities",
        // ability counts, keyword checks) diverge. The turn-face-up special
        // action (CR 702.37e, 702.168d) reads the face-up characteristics
        // stored in `face_down_cast_state`, which the controller knows and
        // other peers learn by opening the card before replaying the action.
        // Ward {2} for disguise is derived from the public cast kind below.
        self.abilities_mut().clear();
        if disguise_ward {
            self.abilities_mut()
                .push(Ability::static_ability(StaticAbility::ward(
                    TotalCost::mana(ManaCost::from_pips(vec![vec![
                        crate::mana::ManaSymbol::Generic(2),
                    ]])),
                )));
        }
        self.spell_effect = None;
        self.aura_attach_filter = None;
        self.optional_costs = Vec::new().into();
        self.additional_cost = TotalCost::free().into();
        self.bestow_cast_state = None;
        true
    }

    /// Learn the identity of a face-down object without turning it face up.
    ///
    /// Private knowledge (a controller looking at its own manifest, cloak, or
    /// morph, or an owner opening its own hidden slot) must not change the
    /// object's face-down characteristics: every peer, including those that
    /// only hold a hidden-card placeholder, has to keep deriving the same
    /// 2/2 nameless creature (CR 708.2) and the same public state. The printed
    /// characteristics are stored as the face-down restore state, so they take
    /// effect only when the object is turned face up (CR 708.8) or leaves.
    ///
    /// Returns `false` (and changes nothing) when the object has no face-down
    /// overlay.
    pub(crate) fn learn_face_down_identity_with_shared(
        &mut self,
        def: &crate::cards::CardDefinition,
        handles: &CardSharedHandles,
    ) -> bool {
        let Some(disguise_ward) = self
            .face_down_cast_state
            .as_ref()
            .map(|state| state.disguise_ward)
        else {
            return false;
        };
        let mut face_up = self.clone();
        face_up.face_down_cast_state = None;
        face_up.apply_card_definition_with_shared(def, handles);

        self.kind = face_up.kind;
        self.card = face_up.card;
        // Card-level data consulted only once the card is face up again.
        self.other_face = face_up.other_face;
        self.other_face_name = face_up.other_face_name.clone();
        self.linked_face_layout = face_up.linked_face_layout;
        self.alternative_casts = face_up.alternative_casts.clone();
        self.has_fuse = face_up.has_fuse;
        self.optional_costs = Vec::new().into();
        self.additional_cost = TotalCost::free().into();
        self.face_down_cast_state = Some(Box::new(FaceDownCastState {
            name: face_up.name,
            first_printed_set_name: face_up.first_printed_set_name,
            mana_cost: face_up.mana_cost,
            color_override: face_up.color_override,
            supertypes: face_up.supertypes,
            card_types: face_up.card_types,
            subtypes: face_up.subtypes,
            compiled_card_text: face_up.compiled_card_text,
            ability_labels: face_up.ability_labels,
            rules_text_color_identity: face_up.rules_text_color_identity,
            base_power: face_up.base_power,
            base_toughness: face_up.base_toughness,
            base_loyalty: face_up.base_loyalty,
            base_defense: face_up.base_defense,
            abilities: face_up.abilities,
            spell_effect: face_up.spell_effect,
            aura_attach_filter: face_up.aura_attach_filter,
            optional_costs: face_up.optional_costs,
            additional_cost: face_up.additional_cost,
            disguise_ward,
        }));
        true
    }

    /// The current face's name beneath a face-down overlay, otherwise the
    /// current copiable name. Physical-card authentication must use `card`:
    /// this name can belong to an alternate face or a copied card.
    pub fn identity_name(&self) -> &SharedStr {
        self.face_down_cast_state
            .as_ref()
            .map(|state| &state.name)
            .unwrap_or(&self.name)
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
        self.ability_labels = restore.ability_labels;
        self.rules_text_color_identity = restore.rules_text_color_identity;
        self.base_power = restore.base_power;
        self.base_toughness = restore.base_toughness;
        self.base_loyalty = restore.base_loyalty;
        self.base_defense = restore.base_defense;
        self.abilities = restore.abilities;
        self.spell_effect = restore.spell_effect;
        self.aura_attach_filter = restore.aura_attach_filter;
        self.optional_costs = restore.optional_costs;
        self.additional_cost = restore.additional_cost;
        true
    }

    /// Combined split-card characteristics, when they apply: a split card
    /// outside the stack and the battlefield (CR 709.4). Cheap check first.
    #[inline]
    pub fn split_combined_active(&self) -> Option<&SplitCombinedCharacteristics> {
        let combined = self.split_combined.as_deref()?;
        (!matches!(self.zone, Zone::Stack | Zone::Battlefield)).then_some(combined)
    }

    /// Card types in the object's current zone (both split halves outside the
    /// stack and battlefield, CR 709.4d).
    #[inline]
    pub fn zone_card_types(&self) -> &[CardType] {
        self.split_combined_active()
            .map_or(self.card_types.as_slice(), |combined| {
                combined.card_types.as_slice()
            })
    }

    /// Subtypes in the object's current zone (see `zone_card_types`).
    #[inline]
    pub fn zone_subtypes(&self) -> &[Subtype] {
        self.split_combined_active()
            .map_or(self.subtypes.as_slice(), |combined| {
                combined.subtypes.as_slice()
            })
    }

    /// Supertypes in the object's current zone (see `zone_card_types`).
    #[inline]
    pub fn zone_supertypes(&self) -> &[Supertype] {
        self.split_combined_active()
            .map_or(self.supertypes.as_slice(), |combined| {
                combined.supertypes.as_slice()
            })
    }

    /// Name of the other split half, when the object currently has both
    /// names (CR 709.4a).
    #[inline]
    pub fn split_other_half_name(&self) -> Option<&str> {
        self.split_combined_active()
            .map(|combined| combined.other_half_name.as_ref())
            .or_else(|| {
                (self.linked_face_layout == crate::card::LinkedFaceLayout::Split
                    && !matches!(self.zone, Zone::Stack | Zone::Battlefield))
                .then(|| self.other_face_name.as_deref())
                .flatten()
            })
    }

    /// Whether the object has `name`: either half's name for a split card
    /// outside the stack and battlefield (CR 709.4a).
    pub fn has_name(&self, name: &str) -> bool {
        self.name.as_ref() == name || self.split_other_half_name() == Some(name)
    }

    /// Save the printed copiable fields before an "enters as a copy" (or other
    /// enters-as characteristic change) overwrites them. The first capture
    /// wins, so repeated entry modifications keep the true printed values.
    pub fn capture_enters_as_copy_restore_state(&mut self) {
        if self.enters_as_copy_restore_state.is_some() {
            return;
        }
        // A face-down, prototyped or bestowed permanent's live copiable fields
        // are that overlay's values; the true printed values are the ones the
        // overlay saved (CR 708.9, 702.160, 702.103). Capturing the overlay
        // values instead would bring them back when the permanent leaves the
        // battlefield, even after it was turned face up.
        let mut printed = match self.face_down_cast_state.as_deref() {
            Some(face_down) => FaceDownCastState {
                disguise_ward: false,
                ..face_down.clone()
            },
            None => self.live_copiable_restore_fields(),
        };
        if let Some(prototype) = &self.prototype_cast_state {
            printed.mana_cost = prototype.mana_cost.clone();
            printed.color_override = prototype.color_override;
            printed.base_power = prototype.base_power;
            printed.base_toughness = prototype.base_toughness;
        }
        if let Some(bestow) = self.bestow_cast_state.as_deref() {
            printed.card_types = bestow.card_types.clone();
            printed.subtypes = bestow.subtypes.clone();
            printed.aura_attach_filter = bestow.aura_attach_filter.clone();
            printed.spell_effect = bestow.spell_effect.clone();
        }
        self.enters_as_copy_restore_state = Some(Box::new(EntersAsCopyRestoreState {
            printed,
            other_face: self.other_face,
            other_face_name: self.other_face_name.clone(),
            linked_face_layout: self.linked_face_layout,
            has_fuse: self.has_fuse,
        }));
    }

    fn live_copiable_restore_fields(&self) -> FaceDownCastState {
        FaceDownCastState {
            name: self.name.clone(),
            first_printed_set_name: self.first_printed_set_name.clone(),
            mana_cost: self.mana_cost.clone(),
            color_override: self.color_override,
            supertypes: self.supertypes.clone(),
            card_types: self.card_types.clone(),
            subtypes: self.subtypes.clone(),
            compiled_card_text: self.compiled_card_text.clone(),
            ability_labels: self.ability_labels.clone(),
            rules_text_color_identity: self.rules_text_color_identity,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
            base_loyalty: self.base_loyalty,
            base_defense: self.base_defense,
            abilities: self.abilities.clone(),
            spell_effect: self.spell_effect.clone(),
            aura_attach_filter: self.aura_attach_filter.clone(),
            optional_costs: self.optional_costs.clone(),
            additional_cost: self.additional_cost.clone(),
            disguise_ward: false,
        }
    }

    /// Restore the printed copiable fields saved by
    /// [`Self::capture_enters_as_copy_restore_state`] (CR 400.7).
    pub fn end_enters_as_copy_overlay(&mut self) -> bool {
        let Some(restore) = self.enters_as_copy_restore_state.take() else {
            return false;
        };
        let EntersAsCopyRestoreState {
            printed: restore,
            other_face,
            other_face_name,
            linked_face_layout,
            has_fuse,
        } = *restore;
        self.name = restore.name;
        self.first_printed_set_name = restore.first_printed_set_name;
        self.mana_cost = restore.mana_cost;
        self.color_override = restore.color_override;
        self.supertypes = restore.supertypes;
        self.card_types = restore.card_types;
        self.subtypes = restore.subtypes;
        self.compiled_card_text = restore.compiled_card_text;
        self.ability_labels = restore.ability_labels;
        self.rules_text_color_identity = restore.rules_text_color_identity;
        self.base_power = restore.base_power;
        self.base_toughness = restore.base_toughness;
        self.base_loyalty = restore.base_loyalty;
        self.base_defense = restore.base_defense;
        self.abilities = restore.abilities;
        self.spell_effect = restore.spell_effect;
        self.aura_attach_filter = restore.aura_attach_filter;
        self.optional_costs = restore.optional_costs;
        self.additional_cost = restore.additional_cost;
        self.other_face = other_face;
        self.other_face_name = other_face_name;
        self.linked_face_layout = linked_face_layout;
        self.has_fuse = has_fuse;
        true
    }

    /// Returns the colors of this object.
    pub fn colors(&self) -> ColorSet {
        let colors = self.own_colors();
        match self.split_combined_active() {
            // CR 709.4c: a split card has the colors of both halves.
            Some(combined) if !colors.is_empty() || !self.is_devoid() => {
                colors.union(combined.other_half_colors)
            }
            _ => colors,
        }
    }

    fn is_devoid(&self) -> bool {
        self.abilities.iter().any(|ability| {
            ability.functions_in(&self.zone)
                && matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability) if static_ability.is_devoid()
                )
        })
    }

    /// Colors of the face currently shown, ignoring the other split half.
    pub fn own_colors(&self) -> ColorSet {
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
        Self::color_identity_from_parts(
            self.mana_cost.as_deref(),
            self.color_override,
            self.rules_text_color_identity,
        )
    }

    /// Color identity from the printed characteristics (CR 903.4a): under a
    /// face-down overlay this reads the face-up card's values, which the
    /// overlay only hides.
    pub fn printed_color_identity(&self) -> ColorSet {
        match self.face_down_cast_state.as_deref() {
            Some(printed) => Self::color_identity_from_parts(
                printed.mana_cost.as_deref(),
                printed.color_override,
                printed.rules_text_color_identity,
            ),
            None => self.color_identity(),
        }
    }

    fn color_identity_from_parts(
        mana_cost: Option<&ManaCost>,
        color_override: Option<ColorSet>,
        rules_text_color_identity: ColorSet,
    ) -> ColorSet {
        use crate::color::Color;
        use crate::mana::ManaSymbol;

        let mut identity = ColorSet::COLORLESS;

        // Add colors from mana cost
        if let Some(mana_cost) = mana_cost {
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
        if let Some(override_colors) = color_override {
            identity = identity.union(override_colors);
        }

        identity.union(rules_text_color_identity)
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
        self.counters.add(counter_type, amount);
    }

    /// Removes counters of the specified type. Returns the number actually removed.
    pub fn remove_counters(&mut self, counter_type: CounterType, amount: u32) -> u32 {
        let current = self.counters.get(&counter_type).copied().unwrap_or(0);
        let removed = current.min(amount);
        let remaining = current - removed;
        if remaining == 0 {
            self.counters.remove(&counter_type);
        } else {
            self.counters.insert(counter_type, remaining);
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
        self.zone_card_types().contains(&card_type)
    }

    /// Returns true if this object has the given supertype.
    pub fn has_supertype(&self, supertype: Supertype) -> bool {
        self.zone_supertypes().contains(&supertype)
    }

    /// Returns true if this object has the given subtype.
    ///
    /// If the object has Changeling and is a creature, it has all creature types.
    pub fn has_subtype(&self, subtype: Subtype) -> bool {
        if self.zone_subtypes().contains(&subtype) {
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
            initial_controller: controller,
            name: handles.name.clone(),
            first_printed_set_name: handles.first_printed_set_name.clone(),
            mana_cost: handles.mana_cost.clone(), // Predefined card-name tokens retain their printed cost.
            color_override: def.card.color_indicator, // Use color indicator if set
            supertypes: handles.supertypes.clone(),
            card_types: handles.card_types.clone(),
            subtypes: handles.subtypes.clone(),
            compiled_card_text: handles.compiled_card_text.clone(),
            ability_labels: handles.ability_labels.clone(),
            rules_text_color_identity: def.card.rules_text_color_identity,
            other_face: def.card.other_face,
            other_face_name: handles.other_face_name.clone(),
            linked_face_layout: def.card.linked_face_layout,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: def.card.power_toughness.map(|pt| pt.power),
            base_toughness: def.card.power_toughness.map(|pt| pt.toughness),
            base_loyalty: def.card.loyalty,
            base_defense: def.card.defense,
            hand_modifier: def.card.hand_modifier,
            life_modifier: def.card.life_modifier,
            abilities: handles.abilities.clone(),
            counters: ObjectCounters::default(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: handles.spell_effect.clone(),
            splice_cast_state: None,
            aura_attach_filter: handles.aura_attach_filter.clone(),
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
            alternative_casts: handles.alternative_casts.clone(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: def.has_fuse,
            optional_costs: handles.optional_costs.clone(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: Some(crate::mana::XManaAllocation::default()),
            snow_mana_spent_to_cast: ManaPool::default(),
            temporary_static_ability_grants: TemporaryStaticAbilityGrants::new(id),
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
    use crate::game_state::GameState;

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
    fn face_down_costs_restore_after_hydration_unveil_and_copy_lifetimes() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let definition = crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Printed costs")
            .card_types(vec![CardType::Creature])
            .additional_cost(TotalCost::mana(ManaCost::new().add_generic(9)))
            .optional_cost(OptionalCost::custom("Printed option", TotalCost::mana(ManaCost::new().add_generic(4))))
            .build();
        let id = game.create_object_from_definition(&definition, player, Zone::Hand);
        let mut face = game.object(id).unwrap().clone();
        let costs = face.additional_cost.clone(); let optional = face.optional_costs.clone();
        assert!(face.apply_face_down_cast_overlay_with_disguise_ward(true));
        assert!(face.additional_cost.is_free()); assert!(face.optional_costs.is_empty());
        let token = Object::token_copy_of(&face, ObjectId::from_raw(900), player);
        assert!(token.additional_cost.is_free()); assert!(token.optional_costs.is_empty());
        face.capture_enters_as_copy_restore_state();
        let mut restored_copy = face.clone(); assert!(restored_copy.end_enters_as_copy_overlay());
        assert_eq!(restored_copy.additional_cost, costs); assert_eq!(restored_copy.optional_costs, optional);
        assert!(face.end_face_down_cast_overlay()); assert_eq!(face.additional_cost, costs); assert_eq!(face.optional_costs, optional);
        let token = Object::token_copy_of(&face, ObjectId::from_raw(901), player);
        assert_eq!(token.additional_cost, costs); assert_eq!(token.optional_costs, optional);
        let hidden = game.create_hidden_card_placeholder(player, Zone::Stack, 0, "overlay-costs".into());
        game.object_mut(hidden).unwrap().apply_face_down_cast_overlay_with_disguise_ward(false);
        assert!(game.reveal_hidden_card_with_definition(hidden, &definition).is_some());
        let learned = game.object_mut(hidden).unwrap(); assert!(learned.additional_cost.is_free()); assert!(learned.optional_costs.is_empty());
        assert!(learned.end_face_down_cast_overlay()); assert_eq!(learned.additional_cost, costs); assert_eq!(learned.optional_costs, optional);
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
    #[test]
    fn spell_copy_does_not_inherit_snow_mana_payment() {
        let card = CardBuilder::new(CardId::new(), "Snow-paid Spell")
            .card_types(vec![CardType::Creature])
            .build();
        let alice = PlayerId::from_index(0);
        let mut source = Object::from_card(ObjectId::from_raw(1), &card, alice, Zone::Stack);
        source.snow_mana_spent_to_cast.green = 2;
        source.caster_mana_spent_to_cast = Some(2);
        source.mana_spent_on_x = Some(crate::mana::XManaAllocation([0, 0, 1, 0, 0]));
        source.x_value = Some(3);
        let copy = Object::spell_copy_of(&source, ObjectId::from_raw(2), alice);
        assert_eq!(copy.snow_mana_spent_to_cast.total(), 0);
        assert_eq!(copy.caster_mana_spent_to_cast, None);
        assert_eq!(
            copy.mana_spent_on_x,
            Some(crate::mana::XManaAllocation::default())
        );
        assert_eq!(copy.x_value, Some(3));
        assert_eq!(source.snow_mana_spent_to_cast.green, 2);
    }

    #[test]
    fn snow_payment_survives_resolution_but_not_a_later_zone_instance() {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Snow-paid Creature")
            .card_types(vec![CardType::Creature])
            .build();
        for resolves in [false, true] {
            let spell = game.create_object_from_card(&card, alice, Zone::Stack);
            game.object_mut(spell)
                .unwrap()
                .snow_mana_spent_to_cast
                .green = 1;
            let current = if resolves {
                let entered = game
                    .move_object_by_effect(spell, Zone::Battlefield)
                    .unwrap();
                assert_eq!(
                    game.object(entered).unwrap().snow_mana_spent_to_cast.green,
                    1
                );
                entered
            } else {
                spell
            };
            let graveyard = game
                .move_object_by_effect(current, Zone::Graveyard)
                .unwrap();
            assert_eq!(
                game.object(graveyard)
                    .unwrap()
                    .snow_mana_spent_to_cast
                    .total(),
                0
            );
            let recast = game.move_object_by_effect(graveyard, Zone::Stack).unwrap();
            assert_eq!(
                game.object(recast).unwrap().snow_mana_spent_to_cast.total(),
                0
            );
        }
    }
}

#[cfg(test)]
mod temporary_ability_registration_tests {
    use super::*;
    #[test]
    fn scalar_temporary_ability_materialization_preserves_registered_identity() {
        let mut observations = Vec::new();
        for ability in [
            StaticAbilityId::Haste,
            StaticAbilityId::Flying,
            StaticAbilityId::Hexproof,
            StaticAbilityId::Vigilance,
        ] {
            let source = ObjectId::from_raw(90003);
            let mut grants = TemporaryStaticAbilityGrants::new(source);
            for _ in 0..2 {
                grants.push(TemporaryStaticAbilityGrant {
                    ability,
                    ability_payload: None,
                    expires_end_of_turn: Some(2),
                });
            }
            assert_ne!(
                grants.origin(0),
                grants.origin(1),
                "independently registered equal keywords retain separate occurrences"
            );
            let native_copy = grants.clone();
            let mut rebuilt = grants.empty_with_allocator();
            rebuilt.extend_existing(&grants);
            for slot in 0..2 {
                let first = grants[slot]
                    .materialize()
                    .expect("keyword is supported")
                    .instance_id();
                let repeated = grants[slot]
                    .materialize()
                    .expect("keyword is supported")
                    .instance_id();
                let cloned = native_copy[slot]
                    .materialize()
                    .expect("keyword is supported")
                    .instance_id();
                let reconstructed = rebuilt[slot]
                    .materialize()
                    .expect("keyword is supported")
                    .instance_id();
                observations.push((ability, slot, first, repeated, cloned, reconstructed));
            }
            grants.retain(|grant| grant.expires_end_of_turn.is_none_or(|end| end > 2));
            assert!(grants.is_empty(), "expiry still removes the registrations");
        }
        assert!(
            observations
                .iter()
                .all(|(_, _, first, repeated, cloned, rebuilt)| first == repeated
                    && first == cloned
                    && first == rebuilt),
            "reads, native copies and reconstruction must retain each registered ability identity: {observations:?}"
        );
    }

    #[test]
    fn temporary_ability_origins_survive_expiry_native_copy_and_reconstruction() {
        let source = ObjectId::from_raw(90001);
        let ability = crate::static_abilities::StaticAbility::haste();
        let grant = |expiry| TemporaryStaticAbilityGrant {
            ability: ability.id(),
            ability_payload: Some(ability.clone()),
            expires_end_of_turn: Some(expiry),
        };
        let mut grants = TemporaryStaticAbilityGrants::new(source);
        grants.push(grant(1));
        grants.push(grant(2));
        let first = grants.origin(0).unwrap().clone();
        let second = grants.origin(1).unwrap().clone();
        assert_ne!(first, second, "cloned payloads register independently");
        grants.retain(|grant| grant.expires_end_of_turn.is_none_or(|end| end > 1));
        assert_eq!(
            grants.origin(0),
            Some(&second),
            "expiry must not renumber survivor"
        );
        assert_eq!(grants.clone(), grants);
        let mut rebuilt = grants.empty_with_allocator();
        rebuilt.extend_existing(&grants);
        assert_eq!(rebuilt.origin(0), Some(&second));
        rebuilt.clear();
        rebuilt.push(grant(3));
        assert_ne!(rebuilt.origin(0), Some(&first));
        assert_ne!(rebuilt.origin(0), Some(&second));
        let mut other = TemporaryStaticAbilityGrants::new(ObjectId::from_raw(90002));
        other.push(grant(2));
        let other_origin = other.origin(0).unwrap().clone();
        rebuilt.extend_existing(&other);
        assert_eq!(
            rebuilt.origin(1),
            Some(&other_origin),
            "component origin survives reconstruction"
        );
        assert_ne!(rebuilt.origin(0), rebuilt.origin(1));
    }

    #[test]
    fn native_clone_empty_counter_store_preserves_removed_registration_allocator() {
        let mut counters = ObjectCounters::default();
        counters.add(CounterType::Flying, 2);
        let removed: Vec<_> = counters
            .occurrences
            .values()
            .flatten()
            .map(|entry| entry.origin.clone())
            .collect();
        counters.clear();
        let mut restored = counters.clone();
        counters.add(CounterType::Flying, 1);
        restored.add(CounterType::Flying, 1);
        assert_eq!(restored.next_serial, counters.next_serial);
        assert!(
            !removed.contains(&restored.occurrences[&CounterType::Flying][0].origin),
            "an empty native copy cannot recycle an old counter origin"
        );
    }
}

#[cfg(test)]
mod initial_control_representation_tests {
    use super::*;

    #[test]
    fn hidden_identity_redaction_preserves_initial_control_and_ownership() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let id = ObjectId::from_raw(99201);
        let mut object = Object::new_hidden_card(id, alice, Zone::Stack);
        object.initial_controller = bob;
        let stable_id = object.stable_id;
        object.redact_to_hidden_card();
        assert_eq!(object.initial_controller, bob);
        assert_eq!(object.owner, alice);
        assert_eq!(object.id, id);
        assert_eq!(object.stable_id, stable_id);
        assert_eq!(object.zone, Zone::Stack);
    }
}

#[cfg(test)]
mod native_temporary_registration_tests {
    use super::*;
    fn fixture() -> TemporaryStaticAbilityGrants {
        let mut grants = TemporaryStaticAbilityGrants::new(ObjectId::from_raw(9981));
        let shared = StaticAbility::flying();
        for expiry in [1, 2, 3] {
            grants.push(TemporaryStaticAbilityGrant {
                ability: shared.id(),
                ability_payload: Some(shared.clone()),
                expires_end_of_turn: Some(expiry),
            });
        }
        grants.retain(|grant| grant.expires_end_of_turn.is_none_or(|end| end > 1));
        let mut component = TemporaryStaticAbilityGrants::new(ObjectId::from_raw(9982));
        component.push(TemporaryStaticAbilityGrant {
            ability: shared.id(),
            ability_payload: Some(shared),
            expires_end_of_turn: Some(4),
        });
        grants.extend_existing(&component);
        grants
    }
    #[test]
    fn acquired_timestamps_survive_clone_retention_and_component_merge() {
        let mut grants = TemporaryStaticAbilityGrants::new(ObjectId::from_raw(9983));
        let shared = StaticAbility::set_colors(crate::target::ObjectFilter::source(), ColorSet::BLUE);
        for (expiry, timestamp) in [(1, 11), (2, 17)] {
            grants.push_at_timestamp(TemporaryStaticAbilityGrant {
                ability: shared.id(), ability_payload: Some(shared.clone()),
                expires_end_of_turn: Some(expiry),
            }, timestamp);
        }
        let kept = grants.origin(1).unwrap().clone();
        let mut cloned = grants.clone();
        cloned.retain(|grant| grant.expires_end_of_turn == Some(2));
        assert_eq!(cloned.origin(0), Some(&kept));
        assert_eq!(kept.acquired_at(), Some(17));
        let mut merged = TemporaryStaticAbilityGrants::new(ObjectId::from_raw(9984));
        merged.extend_existing(&cloned);
        assert_eq!(merged.origin(0), Some(&kept));
        assert_eq!(merged[0].materialize().unwrap().instance_id(), shared.instance_id());
        assert_ne!(grants.origin(0), grants.origin(1));
    }

    #[test]
    fn merged_color_grants_keep_acquisition_order_after_a_new_host_timestamp() {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let owner = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(CardId::new(), "Native chronology host")
            .card_types(vec![CardType::Creature]).color_indicator(ColorSet::WHITE)
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        let host = game.create_object_from_card(&card, owner, Zone::Battlefield);
        let component = game.create_object_from_card(&card, owner, Zone::Battlefield);
        for (id, colors) in [(host, ColorSet::BLUE), (component, ColorSet::GREEN)] {
            game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(id,
                StaticAbilityId::SetColors,
                Some(StaticAbility::set_colors(crate::target::ObjectFilter::source(), colors)));
        }
        let early = game.object(host).unwrap().temporary_static_ability_grants.clone();
        let late = game.object(component).unwrap().temporary_static_ability_grants.clone();
        let mut merged = TemporaryStaticAbilityGrants::new(host);
        // Component ordering must not become effect chronology after a merge.
        merged.extend_existing(&late);
        merged.extend_existing(&early);
        game.object_mut(host).unwrap().temporary_static_ability_grants = merged;
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_colors(host), Some(ColorSet::GREEN));
        assert!(game.set_face_down(host));
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_colors(host), Some(ColorSet::GREEN));
        assert_eq!(game.object(host).unwrap().temporary_static_ability_grants.origin(0), late.origin(0));
        assert_eq!(game.object(host).unwrap().temporary_static_ability_grants.origin(1), early.origin(0));
        let cloned = game.clone();
        assert_eq!(cloned.current_colors(host), Some(ColorSet::GREEN));
    }

    #[test]
    fn native_clone_temporary_registration_preserves_aliases_origins_expiry_and_allocator() {
        let original = fixture();
        let removed = TemporaryAbilityOrigin {
            source: original.source,
            serial: 0,
            acquired_at: None,
        };
        assert!(!original.origins.contains(&removed));
        assert_ne!(
            original.origins[0].source, original.origins[2].source,
            "merged component origin retained"
        );
        let mut restored = original.clone();
        for slot in 0..original.len() {
            assert_eq!(restored.origin(slot), original.origin(slot));
            assert_eq!(
                restored[slot].materialize().unwrap().instance_id(),
                original[slot].materialize().unwrap().instance_id()
            );
            for turn in [1, 2, 3, 4, 5] {
                assert_eq!(
                    restored[slot].is_expired(turn),
                    original[slot].is_expired(turn)
                );
            }
        }
        restored.clear();
        assert_eq!(restored.next_serial, 3);
        let mut restored = restored.clone();
        restored.push(TemporaryStaticAbilityGrant {
            ability: StaticAbilityId::Flying,
            ability_payload: None,
            expires_end_of_turn: Some(9),
        });
        assert_eq!(
            restored.origin(0).unwrap().serial,
            3,
            "clear/copy cannot recycle removed origins"
        );
        assert_ne!(restored.origin(0), Some(&removed));
    }
}

#[cfg(test)]
mod native_copy_restore_state_tests {
    use super::*;
    fn fixture() -> (
        crate::game_state::GameState,
        ObjectId,
        EntersAsCopyRestoreState,
    ) {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = crate::card::CardBuilder::new(CardId::new(), "Saved printed form")
            .card_types(vec![CardType::Creature])
            .build();
        let id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let object = game.object_mut(id).unwrap();
        object.first_printed_set_name = Some("Saved edition".into());
        object.base_power = Some(PtValue::Star);
        object.base_toughness = Some(PtValue::StarPlus(1));
        object.mana_cost =
            Some(ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Green]]).into());
        object.color_override = Some(ColorSet::GREEN);
        object.supertypes = vec![Supertype::Legendary].into();
        object.base_loyalty = Some(3);
        object.base_defense = Some(4);
        object.rules_text_color_identity = ColorSet::BLUE;
        object.compiled_card_text = "Saved flying lines".into();
        let flying = Ability::static_ability(StaticAbility::flying());
        object.abilities = vec![flying.clone(), flying].into();
        object.ability_labels = vec!["First flying".into(), "Second flying".into()].into();
        object.spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::gain_life(2),
            ])
            .into(),
        );
        object.aura_attach_filter =
            Some(AuraAttachmentFilter::from(crate::target::ObjectFilter::creature()).into());
        object.other_face = Some(CardId::new());
        object.other_face_name = Some("Saved linked face".into());
        object.linked_face_layout = LinkedFaceLayout::Split;
        object.has_fuse = true;
        object.capture_enters_as_copy_restore_state();
        let saved = object
            .enters_as_copy_restore_state
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        (game, id, saved)
    }

    #[test]
    fn native_clone_copy_restore_state_preserves_printed_payloads_aliases_and_actual_overlay_end() {
        let (mut game, id, original) = fixture();
        let restored = original.clone();
        for slot in 0..2 {
            let crate::ability::AbilityKind::Static(original_ability) =
                &original.printed.abilities[slot].kind
            else {
                panic!("saved static")
            };
            let crate::ability::AbilityKind::Static(restored_ability) =
                &restored.printed.abilities[slot].kind
            else {
                panic!("restored static")
            };
            assert_eq!(
                original_ability.instance_id(),
                restored_ability.instance_id()
            );
        }
        let object = game.object_mut(id).unwrap();
        object.name = "Copied presentation".into();
        object.base_power = Some(PtValue::Fixed(8));
        object.base_toughness = Some(PtValue::Fixed(8));
        object.abilities = vec![Ability::static_ability(StaticAbility::haste())].into();
        object.other_face = None;
        object.has_fuse = false;
        object.enters_as_copy_restore_state = Some(Box::new(restored));
        assert!(object.end_enters_as_copy_overlay());
        assert_eq!(object.name.as_ref(), "Saved printed form");
        assert_eq!(object.base_power, Some(PtValue::Star));
        assert_eq!(object.base_toughness, Some(PtValue::StarPlus(1)));
        assert_eq!(
            object.ability_labels.as_slice(),
            &["First flying".to_string(), "Second flying".to_string()]
        );
        assert_eq!(object.other_face, original.other_face);
        assert_eq!(object.linked_face_layout, LinkedFaceLayout::Split);
        assert!(object.has_fuse);
        assert!(object.enters_as_copy_restore_state.is_none());
    }
}

#[cfg(test)]
mod native_cast_overlay_tests {
    use super::*;
    fn object() -> Object {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = crate::card::CardBuilder::new(CardId::new(), "Saved cast form")
            .card_types(vec![CardType::Creature, CardType::Enchantment])
            .build();
        let id = game.create_object_from_card(&card, game.players[0].id, Zone::Stack);
        let mut object = game.object(id).unwrap().clone();
        object.base_power = Some(PtValue::Star);
        object.base_toughness = Some(PtValue::StarPlus(1));
        object.color_override = Some(ColorSet::BLUE);
        object.mana_cost =
            Some(ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Blue]]).into());
        object.spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::gain_life(2),
                crate::effect::Effect::gain_life(3),
            ])
            .into(),
        );
        object
    }
    #[test]
    fn native_clone_cast_overlays_restore_native_characteristics_and_programs() {
        let original = object();
        let mut bestow = original.clone();
        bestow.apply_bestow_cast_overlay();
        assert!(bestow.subtypes.contains(&Subtype::Aura));
        let saved = *bestow.bestow_cast_state.take().unwrap();
        bestow.bestow_cast_state = Some(Box::new(saved.clone()));
        assert!(bestow.end_bestow_cast_overlay());
        assert_eq!(bestow.card_types, original.card_types);
        assert_eq!(bestow.subtypes, original.subtypes);
        assert_eq!(
            bestow.spell_effect.as_ref().unwrap().segments[0]
                .default_effects
                .len(),
            2
        );
        assert!(!bestow.end_bestow_cast_overlay());

        let mut splice = original.clone();
        assert!(splice.begin_splice_cast_overlay());
        splice.spell_effect = None;
        let saved = *splice.splice_cast_state.take().unwrap();
        splice.splice_cast_state = Some(Box::new(saved.clone()));
        assert!(splice.end_splice_cast_overlay());
        assert_eq!(
            splice.spell_effect.as_ref().unwrap().segments[0]
                .default_effects
                .len(),
            2
        );
        assert!(!splice.end_splice_cast_overlay());

        let mut prototype = original.clone();
        assert!(prototype.apply_prototype_cast_overlay(
            ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Red]]),
            PowerToughness::fixed(1, 2)
        ));
        assert_eq!(prototype.base_power, Some(PtValue::Fixed(1)));
        let saved = prototype.prototype_cast_state.take().unwrap();
        prototype.prototype_cast_state = Some(saved.clone());
        assert!(prototype.end_prototype_cast_overlay());
        assert_eq!(prototype.mana_cost, original.mana_cost);
        assert_eq!(prototype.color_override, original.color_override);
        assert_eq!(prototype.base_power, original.base_power);
        assert_eq!(prototype.base_toughness, original.base_toughness);
        assert!(!prototype.end_prototype_cast_overlay());
    }
}

#[cfg(test)]
mod native_counter_store_tests {
    use super::*;
    use crate::ability::AbilityKind;

    fn static_ids(store: &ObjectCounters) -> Vec<crate::static_abilities::StaticAbilityInstanceId> {
        store
            .occurrences
            .values()
            .flatten()
            .flat_map(|entry| &entry.abilities)
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(ability.instance_id()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn native_clone_counter_store_preserves_shared_payloads_survivors_and_allocator() {
        let mut counters = ObjectCounters::default();
        counters.add(CounterType::Flying, 2);
        counters.add(CounterType::Decayed, 1);
        let original = static_ids(&counters);
        let mut restored = counters.clone();
        assert_eq!(static_ids(&restored), original);
        assert_eq!(restored.next_serial, counters.next_serial);
        assert!(matches!(
            restored.ability_occurrences(CounterType::Decayed)[0].abilities[1].kind,
            AbilityKind::Triggered(_)
        ));
        counters.insert(CounterType::Flying, 1);
        restored.insert(CounterType::Flying, 1);
        assert_eq!(static_ids(&restored), static_ids(&counters));
        counters.add(CounterType::Flying, 1);
        restored.add(CounterType::Flying, 1);
        assert_eq!(restored.next_serial, counters.next_serial);
        let flying_ids = |store: &ObjectCounters| {
            store
                .ability_occurrences(CounterType::Flying)
                .iter()
                .map(|entry| match &entry.abilities[0].kind {
                    AbilityKind::Static(ability) => ability.instance_id(),
                    _ => panic!("flying counter must be static"),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(flying_ids(&restored)[0], flying_ids(&counters)[0]);
        assert_ne!(
            flying_ids(&restored)[1],
            flying_ids(&counters)[1],
            "independent newly registered counters have distinct payload occurrences"
        );
    }
}

#[cfg(all(test, feature = "serialization"))]
mod permanent_permission_registration_tests {
    use super::*;
    #[test]
    fn indefinite_registration_is_explicit_and_retained_without_an_expiry() {
        let mut grants = TemporaryStaticAbilityGrants::new(ObjectId::from_raw(99999));
        grants.push(TemporaryStaticAbilityGrant {
            ability: StaticAbilityId::Flying,
            ability_payload: Some(StaticAbility::flying()),
            expires_end_of_turn: None,
        });
        assert!(!grants[0].is_expired(u32::MAX));
        let restored = grants.clone();
        assert_eq!(restored[0].expires_end_of_turn, None);
        assert!(!restored[0].is_expired(u32::MAX));
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct CastPriceReceipt<C, G> {
    pub identity: G,
    pub source: ObjectId,
    pub total_cost: C,
    /// Mana that the independent origin requires in addition to any price.
    pub origin_mana_surcharge: ManaCost,
    pub prototype: Option<usize>,
    pub constraints: crate::grant_registry::PlayFromConstraints,
}
