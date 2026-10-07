//! Runtime identity of abilities, independent of their semantic definitions.
use super::ContinuousEffect;
use crate::ability::Ability;
use crate::ids::{CardId, ObjectId};
use crate::object::SharedVec;
use crate::static_abilities::StaticAbilityInstanceId;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Stable occurrence of the ability generating one branch of a static effect.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContinuousAbilityOrigin {
    pub host: ObjectId,
    pub ability: AbilityOrigin,
    pub printed_face: Option<CardId>,
    pub branch: usize,
}

#[derive(Debug, Clone)]
pub struct AbilityEffectOrigin {
    source: ObjectId,
    replaces_rules_text: bool,
    registration_id: Option<super::ContinuousEffectId>,
    timestamp: u64,
    static_ability: Option<StaticAbilityInstanceId>,
    generated_by: Option<Box<ContinuousAbilityOrigin>>,
    // Captured only for the proved source-only Class gate. This never changes
    // ordinary Effect identity or another consumer's acquisition namespace.
    linked_exile_root_pair: Option<(ironsmith_core::LinkedExilePair, u32)>,
}
impl PartialEq for AbilityEffectOrigin {
    fn eq(&self, other: &Self) -> bool {
        match (self.registration_id, other.registration_id) {
            (Some(a), Some(b)) => a == b,
            (Some(_), None) | (None, Some(_)) => false,
            (None, None) => match (&self.generated_by, &other.generated_by) {
                (Some(a), Some(b)) => a == b,
                (None, None) => {
                    self.source == other.source
                        && self.timestamp == other.timestamp
                        && self.static_ability == other.static_ability
                }
                _ => false,
            },
        }
    }
}
impl Eq for AbilityEffectOrigin {}
impl Hash for AbilityEffectOrigin {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(id) = self.registration_id {
            0_u8.hash(state);
            id.hash(state);
        } else if let Some(parent) = &self.generated_by {
            1_u8.hash(state);
            parent.hash(state);
        } else {
            2_u8.hash(state);
            self.source.hash(state);
            self.timestamp.hash(state);
            self.static_ability.hash(state);
        }
    }
}
impl AbilityEffectOrigin {
    pub(crate) fn is_class_linked_exile_effect(effect: &ContinuousEffect) -> bool {
        class_linked_exile_root(effect).is_some()
    }
    pub(crate) fn is_source_class_level_effect(effect: &ContinuousEffect) -> bool {
        effect.source_type == super::EffectSourceType::StaticAbility
            && matches!(effect.applies_to, super::EffectTarget::Source)
            && matches!(effect.condition, Some(crate::ConditionExpr::SourceClassLevelAtLeast(_)))
            && matches!(effect.modification, super::Modification::AddAbilityGeneric(_))
    }
    pub(crate) fn linked_exile_parent(
        &self, host: ObjectId, pair: ironsmith_core::LinkedExilePair,
    ) -> Option<&AbilityOrigin> {
        if self.source != host || self.linked_exile_root_pair.map(|(member, _)| member) != Some(pair) { return None; }
        let parent = self.generated_by.as_ref()?;
        (parent.host == host).then_some(&parent.ability)
    }

    pub(crate) fn linked_exile_class_level(&self) -> Option<u32> {
        self.linked_exile_root_pair.map(|(_, level)| level)
    }

    /// The object whose effect granted the ability.
    pub fn source(&self) -> ObjectId {
        self.source
    }
    /// The static ability of `source` that generated the effect, when a static
    /// ability (rather than a resolving spell or ability) generated it.
    pub fn static_ability(&self) -> Option<StaticAbilityInstanceId>
    {
        self.static_ability
    }
}
impl From<&ContinuousEffect> for AbilityEffectOrigin {
    fn from(effect: &ContinuousEffect) -> Self {
        Self {
            source: effect.source,
            replaces_rules_text: matches!(effect.modification,
                super::Modification::CopyOf { .. } | super::Modification::SetTextBox(_)),
            linked_exile_root_pair: class_linked_exile_root(effect),
            registration_id: effect.registration_id,
            generated_by: effect.originating_ability.clone(),
            timestamp: effect.timestamp,
            static_ability: effect
                .originating_static_ability
                .as_ref()
                .map(|ability| ability.instance_id()),
        }
    }
}

/// The immutable definition and exact generating occurrence must both prove
/// this one wrapper. Broad effect grants, other levels, and resolving effects
/// retain their own acquisition identity.
fn class_linked_exile_root(effect: &ContinuousEffect) -> Option<(ironsmith_core::LinkedExilePair, u32)> {
    use super::{EffectSourceType, EffectTarget, Modification};
    if effect.source_type != EffectSourceType::StaticAbility
        || !matches!(&effect.applies_to, EffectTarget::Source)
            && !matches!(&effect.applies_to, EffectTarget::Specific(id) if *id == effect.source)
    { return None; }
    let Some(crate::ConditionExpr::SourceClassLevelAtLeast(level)) = &effect.condition else { return None; };
    let Modification::AddAbilityGeneric(ability) = &effect.modification else { return None; };
    let crate::ability::AbilityKind::Static(reader) = &ability.kind else { return None; };
    let spec = reader.grant_spec()?;
    if !spec.requires_linked_exile_pair || spec.linked_exile_class_level != Some(*level) { return None; }
    Some((spec.linked_exile_pair?, *level))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AbilityOrigin {
    Printed(usize),
    /// Mana supplied intrinsically by the object's current basic land type.
    IntrinsicBasicLandMana(crate::types::Subtype),
    IntrinsicStartingCounters(ironsmith_core::IntrinsicStartingCounter),
    Temporary(crate::object::TemporaryAbilityOrigin),
    Counter {
        occurrence: crate::object::CounterAbilityOrigin,
        slot: usize,
    },
    Level {
        printed_face: Option<CardId>,
        parent: Box<AbilityOrigin>,
        tier: usize,
        slot: usize,
    },
    Effect {
        effect: AbilityEffectOrigin,
        slot: usize,
    },
    Borrowed {
        effect: AbilityEffectOrigin,
        source: ObjectId,
        origin: Box<AbilityOrigin>,
    },
}

impl AbilityOrigin {
    /// Copy and text-box replacement define the object's text. Ordinary
    /// grants, borrowed abilities, counters and intrinsic rules do not (612.3).
    pub(crate) fn is_rules_text(&self) -> bool {
        match self {
            Self::Printed(_) => true,
            Self::Effect { effect, .. } => effect.replaces_rules_text,
            Self::IntrinsicBasicLandMana(_) | Self::IntrinsicStartingCounters(_)
            | Self::Temporary(_) | Self::Counter { .. } | Self::Level { .. }
            | Self::Borrowed { .. } => false,
        }
    }

    /// Independent grants materialized before the layer loop. Text/copy
    /// replacement must not erase them; ordinary layer-six clearing still can.
    pub(crate) fn is_independent_early_grant(&self) -> bool {
        match self {
            Self::Temporary(_) | Self::Counter { .. } => true,
            Self::Level { parent, .. } => parent.is_independent_early_grant(),
            _ => false,
        }
    }

    /// Return the permanent whose effect supplied this ability, when the
    /// ability was granted by a continuous effect.
    pub(crate) fn effect_source(&self) -> Option<ObjectId> {
        match self {
            Self::Printed(_) | Self::IntrinsicBasicLandMana(_) | Self::IntrinsicStartingCounters(_) | Self::Temporary(_) | Self::Counter { .. } | Self::Level { .. } => {
                None
            }
            Self::Effect { effect, .. } => Some(effect.source),
            Self::Borrowed { effect, .. } => Some(effect.source),
        }
    }

    /// The object whose effect granted this ability to the object that has
    /// it ("Equipped creature has '...'"), which the ability's own text may
    /// name. A borrowed ability keeps the grantor of the ability it copies.
    pub(crate) fn granting_source(&self) -> Option<ObjectId> {
        match self {
            Self::Printed(_) | Self::IntrinsicBasicLandMana(_) | Self::IntrinsicStartingCounters(_) | Self::Temporary(_) | Self::Counter { .. } | Self::Level { .. } => {
                None
            }
            Self::Effect { effect, .. } => Some(effect.source),
            Self::Borrowed { origin, .. } => origin.granting_source(),
        }
    }
}

/// Mutations preserve the origin paired with each definition. There is no
/// DerefMut: raw Vec edits must not silently detach an ability from its origin.
#[derive(Debug, Clone, Default)]
pub struct CalculatedAbilities {
    definitions: SharedVec<Ability>,
    origins: Vec<AbilityOrigin>,
    host: Option<ObjectId>,
    current_effect: Option<AbilityEffectOrigin>,
    next_effect_slot: usize,
}
impl CalculatedAbilities {
    /// Bind definition stamps before the first trigger can read history. A
    /// collection made without a host remains unproven for active text edits.
    pub(crate) fn bind_host(&mut self, host: ObjectId) {
        self.host = Some(host);
        self.bind_trigger_acquisitions();
    }
    fn bind_trigger_acquisitions(&mut self) {
        if self.definitions.iter().any(|ability| matches!(&ability.kind,
            crate::ability::AbilityKind::Triggered(triggered)
                if triggered.effects.retained_trigger_definition().is_some()))
        {
            for (ability, origin) in self.definitions.iter_mut().zip(&self.origins) {
                crate::triggers::acquisition::bind_ability(ability, self.host, origin);
            }
        }
    }
    pub(super) fn replace_with_origin(
        &mut self,
        abilities: Vec<Ability>,
        effect: Option<AbilityEffectOrigin>,
    ) {
        self.definitions = abilities.into();
        self.rebind_origin(effect);
    }
    pub fn origin(&self, index: usize) -> Option<&AbilityOrigin> {
        self.origins.get(index)
    }
    pub fn begin_effect(&mut self, effect: &ContinuousEffect) {
        self.current_effect = Some(effect.into());
        self.next_effect_slot = 0;
    }
    pub fn rebind(&mut self, effect: &ContinuousEffect) {
        self.rebind_origin(Some(effect.into()));
    }
    pub(super) fn rebind_origin(&mut self, effect: Option<AbilityEffectOrigin>) {
        self.origins = (0..self.len())
            .map(|slot| match &effect {
                Some(effect) => AbilityOrigin::Effect {
                    effect: effect.clone(),
                    slot,
                },
                None => AbilityOrigin::Printed(slot),
            })
            .collect();
        self.next_effect_slot = self.len();
        self.current_effect = effect;
        self.bind_trigger_acquisitions();
    }
    pub fn push(&mut self, ability: Ability) {
        let origin = match &self.current_effect {
            Some(effect) => {
                let slot = self.next_effect_slot;
                self.next_effect_slot += 1;
                AbilityOrigin::Effect {
                    effect: effect.clone(),
                    slot,
                }
            }
            None => AbilityOrigin::Printed(self.len()),
        };
        self.push_with_origin(ability, origin);
    }
    pub fn push_with_origin(&mut self, mut ability: Ability, origin: AbilityOrigin) {
        crate::triggers::acquisition::bind_ability(&mut ability, self.host, &origin);
        self.definitions.push(ability);
        self.origins.push(origin);
    }
    pub fn retain(&mut self, mut predicate: impl FnMut(&Ability) -> bool) {
        self.retain_with_origin(|ability, _| predicate(ability));
    }
    pub(crate) fn retain_with_origin(&mut self, mut predicate: impl FnMut(&Ability, &AbilityOrigin) -> bool) {
        let origins = &self.origins;
        let mut retained = Vec::new();
        let mut index = 0;
        self.definitions.retain(|ability| {
            let keep = predicate(ability, &origins[index]);
            if keep {
                retained.push(origins[index].clone());
            }
            index += 1;
            keep
        });
        self.origins = retained;
    }
    /// Transform definitions atomically while keeping every acquisition and
    /// generating occurrence paired with its original slot.
    pub(crate) fn try_map_rules_text<E>(
        &mut self,
        mut map: impl FnMut(&Ability) -> Result<Ability, E>,
    ) -> Result<(), E> {
        let mut definitions = self.definitions.to_vec();
        for (definition, origin) in definitions.iter_mut().zip(&self.origins) {
            if origin.is_rules_text() { *definition = map(definition)?; }
        }
        self.definitions = definitions.into();
        Ok(())
    }

    pub fn clear(&mut self) {
        self.definitions.clear();
        self.origins.clear();
    }
    pub fn as_slice(&self) -> &[Ability] {
        self.definitions.as_slice()
    }
    pub fn shared(&self) -> Arc<Vec<Ability>> {
        self.definitions.shared()
    }
    pub fn to_vec(&self) -> Vec<Ability> {
        self.definitions.to_vec()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Ability> {
        self.definitions.iter_mut()
    }
}
impl std::ops::Deref for CalculatedAbilities {
    type Target = [Ability];
    fn deref(&self) -> &[Ability] {
        self.as_slice()
    }
}
impl From<SharedVec<Ability>> for CalculatedAbilities {
    fn from(definitions: SharedVec<Ability>) -> Self {
        let origins = (0..definitions.len()).map(AbilityOrigin::Printed).collect();
        let mut abilities = Self {
            definitions,
            origins,
            host: None,
            current_effect: None,
            next_effect_slot: 0,
        };
        abilities.bind_trigger_acquisitions();
        abilities
    }
}
impl From<Vec<Ability>> for CalculatedAbilities {
    fn from(value: Vec<Ability>) -> Self {
        Self::from(SharedVec::from(value))
    }
}
impl From<Arc<Vec<Ability>>> for CalculatedAbilities {
    fn from(value: Arc<Vec<Ability>>) -> Self {
        Self::from(SharedVec::from(value))
    }
}
impl<'a> IntoIterator for &'a CalculatedAbilities {
    type Item = &'a Ability;
    type IntoIter = std::slice::Iter<'a, Ability>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a> IntoIterator for &'a mut CalculatedAbilities {
    type Item = &'a mut Ability;
    type IntoIter = std::slice::IterMut<'a, Ability>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
impl IntoIterator for CalculatedAbilities {
    type Item = Ability;
    type IntoIter = std::vec::IntoIter<Ability>;
    fn into_iter(self) -> Self::IntoIter {
        self.to_vec().into_iter()
    }
}

impl From<CalculatedAbilities> for Vec<Ability> {
    fn from(value: CalculatedAbilities) -> Self {
        value.to_vec()
    }
}

// Characteristic/template comparisons compare definitions. Activation-history
// lookup explicitly compares AbilityOrigin instead.
impl PartialEq for CalculatedAbilities {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl PartialEq<Vec<Ability>> for CalculatedAbilities {
    fn eq(&self, other: &Vec<Ability>) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<const N: usize> PartialEq<[Ability; N]> for CalculatedAbilities {
    fn eq(&self, other: &[Ability; N]) -> bool {
        self.as_slice() == other
    }
}

#[cfg(test)]
mod registered_origin_tests {
    use super::*;
    #[test]
    fn registered_origin_survives_retarget_and_mutable_metadata() {
        let mut manager = crate::continuous::ContinuousEffectManager::new();
        let source = ObjectId::from_raw(1); let other = ObjectId::from_raw(2);
        let player = crate::ids::PlayerId::from_index(0);
        let mut descriptor = ContinuousEffect::from_resolution(source, player, vec![source],
            crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::flying()));
        descriptor.timestamp = 7;
        let id = manager.add_effect(descriptor.clone());
        let original = AbilityEffectOrigin::from(&manager.effects()[0]);
        manager.retarget_sticker(id, other);
        let moved = AbilityEffectOrigin::from(&manager.effects()[0]);
        assert_eq!(original, moved);
        let mut changed = manager.effects()[0].clone();
        changed.timestamp = 99; changed.controller = crate::ids::PlayerId::from_index(1);
        changed.originating_static_ability = Some(crate::static_abilities::StaticAbility::haste());
        let changed = AbilityEffectOrigin::from(&changed);
        assert_eq!(original, changed);
        assert!(std::collections::HashSet::from([original]).contains(&changed));
        let independent = manager.add_effect(descriptor);
        assert_ne!(id, independent);
        assert_ne!(moved, AbilityEffectOrigin::from(&manager.effects()[1]));
    }
}
