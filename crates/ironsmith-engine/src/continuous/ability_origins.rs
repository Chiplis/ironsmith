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
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(bound(deserialize = "I: serde::Deserialize<'de>, C: serde::Deserialize<'de>")))]
pub struct ContinuousAbilityOrigin<I = StaticAbilityInstanceId, C = CardId> {
    pub host: ObjectId,
    pub ability: AbilityOrigin<I, C>,
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_origin_reference"))]
    pub printed_face: Option<C>,
    pub branch: usize,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(bound(deserialize = "I: serde::Deserialize<'de>, C: serde::Deserialize<'de>")))]
pub struct AbilityEffectOrigin<I = StaticAbilityInstanceId, C = CardId> {
    source: ObjectId,
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_origin_reference"))]
    registration_id: Option<super::ContinuousEffectId>,
    timestamp: u64,
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_origin_reference"))]
    static_ability: Option<I>,
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_origin_reference"))]
    generated_by: Option<Box<ContinuousAbilityOrigin<I, C>>>,
}
impl<I: PartialEq, C: PartialEq> PartialEq for AbilityEffectOrigin<I, C> {
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
impl<I: Eq, C: Eq> Eq for AbilityEffectOrigin<I, C> {}
impl<I: Hash, C: Hash> Hash for AbilityEffectOrigin<I, C> {
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
impl<I, C> AbilityEffectOrigin<I, C> {
    /// The object whose effect granted the ability.
    pub fn source(&self) -> ObjectId {
        self.source
    }
    /// The static ability of `source` that generated the effect, when a static
    /// ability (rather than a resolving spell or ability) generated it.
    pub fn static_ability(&self) -> Option<I>
    where
        I: Copy,
    {
        self.static_ability
    }
}
impl From<&ContinuousEffect> for AbilityEffectOrigin {
    fn from(effect: &ContinuousEffect) -> Self {
        Self {
            source: effect.source,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(bound(deserialize = "I: serde::Deserialize<'de>, C: serde::Deserialize<'de>")))]
pub enum AbilityOrigin<I = StaticAbilityInstanceId, C = CardId> {
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
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_origin_reference"))]
        printed_face: Option<C>,
        parent: Box<AbilityOrigin<I, C>>,
        tier: usize,
        slot: usize,
    },
    Effect {
        effect: AbilityEffectOrigin<I, C>,
        slot: usize,
    },
    Borrowed {
        effect: AbilityEffectOrigin<I, C>,
        source: ObjectId,
        origin: Box<AbilityOrigin<I, C>>,
    },
}

impl AbilityOrigin {
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

/// Convert identity references through one caller-owned checkpoint table.
/// Process-global native IDs are never interpreted as portable wire IDs.
impl<I, C> ContinuousAbilityOrigin<I, C> {
    pub fn try_map_static_instances<J, Error, F>(
        self,
        map: &mut F,
    ) -> Result<ContinuousAbilityOrigin<J, C>, Error>
    where
        F: FnMut(I) -> Result<J, Error> + ?Sized,
    {
        Ok(ContinuousAbilityOrigin {
            host: self.host,
            ability: self.ability.try_map_static_instances(map)?,
            printed_face: self.printed_face,
            branch: self.branch,
        })
    }
}

impl<I, C> AbilityEffectOrigin<I, C> {
    pub fn try_map_static_instances<J, Error, F>(
        self,
        map: &mut F,
    ) -> Result<AbilityEffectOrigin<J, C>, Error>
    where
        F: FnMut(I) -> Result<J, Error> + ?Sized,
    {
        Ok(AbilityEffectOrigin {
            source: self.source,
            registration_id: self.registration_id,
            timestamp: self.timestamp,
            static_ability: self.static_ability.map(&mut *map).transpose()?,
            generated_by: self
                .generated_by
                .map(|origin| origin.try_map_static_instances(map).map(Box::new))
                .transpose()?,
        })
    }
}

impl<I, C> AbilityOrigin<I, C> {
    pub fn try_map_static_instances<J, Error, F>(
        self,
        map: &mut F,
    ) -> Result<AbilityOrigin<J, C>, Error>
    where
        F: FnMut(I) -> Result<J, Error> + ?Sized,
    {
        Ok(match self {
            Self::Printed(slot) => AbilityOrigin::Printed(slot),
            Self::IntrinsicBasicLandMana(subtype) => AbilityOrigin::IntrinsicBasicLandMana(subtype),
            Self::IntrinsicStartingCounters(rule) => AbilityOrigin::IntrinsicStartingCounters(rule),
            Self::Temporary(origin) => AbilityOrigin::Temporary(origin),
            Self::Counter { occurrence, slot } => AbilityOrigin::Counter { occurrence, slot },
            Self::Level {
                printed_face,
                parent,
                tier,
                slot,
            } => AbilityOrigin::Level {
                printed_face,
                parent: Box::new(parent.try_map_static_instances(map)?),
                tier,
                slot,
            },
            Self::Effect { effect, slot } => AbilityOrigin::Effect {
                effect: effect.try_map_static_instances(map)?,
                slot,
            },
            Self::Borrowed {
                effect,
                source,
                origin,
            } => AbilityOrigin::Borrowed {
                effect: effect.try_map_static_instances(map)?,
                source,
                origin: Box::new(origin.try_map_static_instances(map)?),
            },
        })
    }
}

/// Rebind every printed-face allocation through the owning card-definition graph.
/// Instance mapping and face mapping are separate so each uses its own table.
impl<I, C> ContinuousAbilityOrigin<I, C> {
    pub fn try_map_card_ids<D, Error, F>(
        self,
        map: &mut F,
    ) -> Result<ContinuousAbilityOrigin<I, D>, Error>
    where
        F: FnMut(C) -> Result<D, Error> + ?Sized,
    {
        Ok(ContinuousAbilityOrigin {
            host: self.host,
            ability: self.ability.try_map_card_ids(map)?,
            printed_face: self.printed_face.map(&mut *map).transpose()?,
            branch: self.branch,
        })
    }
}
impl<I, C> AbilityEffectOrigin<I, C> {
    pub fn try_map_card_ids<D, Error, F>(
        self,
        map: &mut F,
    ) -> Result<AbilityEffectOrigin<I, D>, Error>
    where
        F: FnMut(C) -> Result<D, Error> + ?Sized,
    {
        Ok(AbilityEffectOrigin {
            source: self.source,
            registration_id: self.registration_id,
            timestamp: self.timestamp,
            static_ability: self.static_ability,
            generated_by: self
                .generated_by
                .map(|origin| origin.try_map_card_ids(map).map(Box::new))
                .transpose()?,
        })
    }
}
impl<I, C> AbilityOrigin<I, C> {
    pub fn try_map_card_ids<D, Error, F>(self, map: &mut F) -> Result<AbilityOrigin<I, D>, Error>
    where
        F: FnMut(C) -> Result<D, Error> + ?Sized,
    {
        Ok(match self {
            Self::Printed(slot) => AbilityOrigin::Printed(slot),
            Self::IntrinsicBasicLandMana(subtype) => AbilityOrigin::IntrinsicBasicLandMana(subtype),
            Self::IntrinsicStartingCounters(rule) => AbilityOrigin::IntrinsicStartingCounters(rule),
            Self::Temporary(origin) => AbilityOrigin::Temporary(origin),
            Self::Counter { occurrence, slot } => AbilityOrigin::Counter { occurrence, slot },
            Self::Level {
                printed_face,
                parent,
                tier,
                slot,
            } => AbilityOrigin::Level {
                printed_face: printed_face.map(&mut *map).transpose()?,
                parent: Box::new(parent.try_map_card_ids(map)?),
                tier,
                slot,
            },
            Self::Effect { effect, slot } => AbilityOrigin::Effect {
                effect: effect.try_map_card_ids(map)?,
                slot,
            },
            Self::Borrowed {
                effect,
                source,
                origin,
            } => AbilityOrigin::Borrowed {
                effect: effect.try_map_card_ids(map)?,
                source,
                origin: Box::new(origin.try_map_card_ids(map)?),
            },
        })
    }
}

/// Mutations preserve the origin paired with each definition. There is no
/// DerefMut: raw Vec edits must not silently detach an ability from its origin.
#[derive(Debug, Clone, Default)]
pub struct CalculatedAbilities {
    definitions: SharedVec<Ability>,
    origins: Vec<AbilityOrigin>,
    current_effect: Option<AbilityEffectOrigin>,
    next_effect_slot: usize,
}
impl CalculatedAbilities {
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
    pub fn push_with_origin(&mut self, ability: Ability, origin: AbilityOrigin) {
        self.definitions.push(ability);
        self.origins.push(origin);
    }
    pub fn retain(&mut self, mut predicate: impl FnMut(&Ability) -> bool) {
        let origins = &self.origins;
        let mut retained = Vec::new();
        let mut index = 0;
        self.definitions.retain(|ability| {
            let keep = predicate(ability);
            if keep {
                retained.push(origins[index].clone());
            }
            index += 1;
            keep
        });
        self.origins = retained;
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
        Self {
            definitions,
            origins,
            current_effect: None,
            next_effect_slot: 0,
        }
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

#[cfg(all(test, feature = "serialization"))]
mod complete_origin_schema_tests {
    use super::*;
    use crate::object::{
        CounterAbilityOrigin, TemporaryStaticAbilityGrant, TemporaryStaticAbilityGrants,
    };
    use crate::static_abilities::{StaticAbility, StaticAbilityId};

    fn origins(
        first: StaticAbilityInstanceId,
        second: StaticAbilityInstanceId,
    ) -> Vec<ContinuousAbilityOrigin> {
        let source = ObjectId::from_raw(11);
        let borrower = ObjectId::from_raw(12);
        let face = Some(CardId::new());
        let inner = AbilityEffectOrigin {
            source,
            registration_id: Some(super::super::ContinuousEffectId(7)),
            timestamp: 99,
            static_ability: Some(second),
            generated_by: None,
        };
        let outer = AbilityEffectOrigin {
            source: borrower,
            registration_id: None,
            timestamp: 101,
            static_ability: Some(first),
            generated_by: Some(Box::new(ContinuousAbilityOrigin {
                host: source,
                ability: AbilityOrigin::Effect {
                    effect: inner.clone(),
                    slot: 8,
                },
                printed_face: face,
                branch: 3,
            })),
        };
        let mut grants = TemporaryStaticAbilityGrants::new(source);
        grants.push(TemporaryStaticAbilityGrant {
            ability: StaticAbilityId::Flying,
            ability_payload: None,
            expires_end_of_turn: 4,
        });
        let families = vec![
            AbilityOrigin::Printed(5),
            AbilityOrigin::IntrinsicBasicLandMana(crate::types::Subtype::Forest),
            AbilityOrigin::IntrinsicStartingCounters(ironsmith_core::IntrinsicStartingCounter::Loyalty),
            AbilityOrigin::IntrinsicStartingCounters(ironsmith_core::IntrinsicStartingCounter::Defense),
            AbilityOrigin::Temporary(
                grants
                    .origin(0)
                    .expect("actual registered temporary origin")
                    .clone(),
            ),
            AbilityOrigin::Counter {
                occurrence: CounterAbilityOrigin {
                    counter_type: crate::object::CounterType::Flood,
                    serial: vec![3, u32::MAX, 4],
                },
                slot: 2,
            },
            AbilityOrigin::Level {
                printed_face: face,
                parent: Box::new(AbilityOrigin::Printed(9)),
                tier: 4,
                slot: 6,
            },
            AbilityOrigin::Effect {
                effect: outer.clone(),
                slot: 7,
            },
            AbilityOrigin::Borrowed {
                effect: outer,
                source,
                origin: Box::new(AbilityOrigin::Level {
                    printed_face: face,
                    parent: Box::new(AbilityOrigin::Effect {
                        effect: inner,
                        slot: 10,
                    }),
                    tier: 2,
                    slot: 11,
                }),
            },
        ];
        families
            .into_iter()
            .enumerate()
            .map(|(branch, ability)| ContinuousAbilityOrigin {
                host: borrower,
                ability,
                printed_face: face,
                branch,
            })
            .collect()
    }

    #[test]
    fn complete_origin_schema_round_trip_preserves_all_families_and_rebinds_local_instances() {
        let first = StaticAbility::flying().instance_id();
        let second = StaticAbility::haste().instance_id();
        let original = origins(first, second);
        let mut encode = |id| -> Result<u64, &'static str> {
            if id == first {
                Ok(1)
            } else if id == second {
                Ok(2)
            } else {
                Err("unbound native instance")
            }
        };
        let wire = original
            .clone()
            .into_iter()
            .map(|origin| origin.try_map_static_instances(&mut encode))
            .collect::<Result<Vec<_>, _>>()
            .expect("all source occurrences bound");
        let json = serde_json::to_value(&wire).expect("complete provenance encodes");
        let decoded: Vec<ContinuousAbilityOrigin<u64>> =
            serde_json::from_value(json.clone()).expect("complete provenance decodes");
        let mut restore_original = |id| match id {
            1 => Ok(first),
            2 => Ok(second),
            _ => Err("unbound wire instance"),
        };
        let same = decoded
            .clone()
            .into_iter()
            .map(|origin| origin.try_map_static_instances(&mut restore_original))
            .collect::<Result<Vec<_>, _>>()
            .expect("original instance table restores");
        assert_eq!(same, original);
        let keys = original
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        assert!(
            same.iter().all(|origin| keys.contains(origin)),
            "runtime application identity and hashes survive conversion"
        );
        let restored_first = StaticAbility::flying().instance_id();
        let restored_second = StaticAbility::haste().instance_id();
        assert_ne!(first, restored_first);
        assert_ne!(second, restored_second);
        let mut rebind = |id| match id {
            1 => Ok(restored_first),
            2 => Ok(restored_second),
            _ => Err("unbound wire instance"),
        };
        let restored = decoded
            .into_iter()
            .map(|origin| origin.try_map_static_instances(&mut rebind))
            .collect::<Result<Vec<_>, _>>()
            .expect("fresh peer instance table rebinds");
        let mut encode_restored = |id| -> Result<u64, &'static str> {
            if id == restored_first {
                Ok(1)
            } else if id == restored_second {
                Ok(2)
            } else {
                Err("unbound restored instance")
            }
        };
        let rebound_wire = restored
            .into_iter()
            .map(|origin| origin.try_map_static_instances(&mut encode_restored))
            .collect::<Result<Vec<_>, _>>()
            .expect("restored occurrences reencode");
        assert_eq!(
            serde_json::to_value(rebound_wire).unwrap(),
            json,
            "every metadata field survives, including fields excluded from runtime identity equality"
        );
    }

    #[test]
    fn complete_origin_schema_rejects_nested_unbound_identity_and_missing_required_metadata() {
        let first = StaticAbility::flying().instance_id();
        let second = StaticAbility::haste().instance_id();
        let source = origins(first, second)
            .pop()
            .expect("borrowed nested source");
        let mut reject_nested = |id| {
            if id == first {
                Ok(1_u64)
            } else {
                Err("unbound nested occurrence")
            }
        };
        assert_eq!(
            source.clone().try_map_static_instances(&mut reject_nested),
            Err("unbound nested occurrence")
        );
        let mut encode = |id| -> Result<u64, &'static str> {
            if id == first {
                Ok(1)
            } else if id == second {
                Ok(2)
            } else {
                Err("unbound native occurrence")
            }
        };
        let wire = source
            .try_map_static_instances(&mut encode)
            .expect("complete source encodes");
        let mut value = serde_json::to_value(wire.clone()).unwrap();
        value.as_object_mut().unwrap().remove("branch");
        assert!(
            serde_json::from_value::<ContinuousAbilityOrigin<u64>>(value).is_err(),
            "missing branch cannot become default provenance"
        );
        let mut no_peer_binding = |_id| -> Result<StaticAbilityInstanceId, &'static str> {
            Err("missing peer table binding")
        };
        assert_eq!(
            wire.try_map_static_instances(&mut no_peer_binding),
            Err("missing peer table binding")
        );
    }
}

#[cfg(test)]
mod origin_card_graph_tests {
    use super::*;
    fn fixture() -> ContinuousAbilityOrigin<u64, String> {
        let parent = ContinuousAbilityOrigin {
            host: ObjectId::from_raw(1),
            ability: AbilityOrigin::Printed(2),
            printed_face: Some("parent-face".into()),
            branch: 3,
        };
        let effect = AbilityEffectOrigin {
            source: ObjectId::from_raw(4),
            registration_id: None,
            timestamp: 5,
            static_ability: Some(6),
            generated_by: Some(Box::new(parent)),
        };
        ContinuousAbilityOrigin {
            host: ObjectId::from_raw(7),
            branch: 8,
            printed_face: Some("host-face".into()),
            ability: AbilityOrigin::Borrowed {
                effect: effect.clone(),
                source: ObjectId::from_raw(9),
                origin: Box::new(AbilityOrigin::Level {
                    printed_face: Some("level-face".into()),
                    parent: Box::new(AbilityOrigin::Effect { effect, slot: 10 }),
                    tier: 11,
                    slot: 12,
                }),
            },
        }
    }
    #[test]
    fn origin_card_graph_maps_every_nested_face_and_preserves_instance_mapping() {
        let original = fixture();
        let mut calls = Vec::new();
        let bound = original
            .clone()
            .try_map_card_ids(&mut |face: String| {
                calls.push(face.clone());
                Ok::<_, ()>(face.len() as u64)
            })
            .unwrap();
        assert_eq!(
            calls,
            ["parent-face", "level-face", "parent-face", "host-face"]
        );
        let rebound = bound
            .clone()
            .try_map_static_instances(&mut |instance| Ok::<_, ()>(instance + 100))
            .unwrap();
        let AbilityOrigin::Borrowed { effect, origin, .. } = &rebound.ability else {
            panic!("borrowed origin")
        };
        assert_eq!(effect.static_ability(), Some(106));
        assert_eq!(effect.generated_by.as_ref().unwrap().printed_face, Some(11));
        let AbilityOrigin::Level {
            printed_face,
            parent,
            ..
        } = origin.as_ref()
        else {
            panic!("level origin")
        };
        assert_eq!(*printed_face, Some(10));
        let AbilityOrigin::Effect { effect, .. } = parent.as_ref() else {
            panic!("effect origin")
        };
        assert_eq!(effect.static_ability(), Some(106));
        assert_eq!(rebound.printed_face, Some(9));
        let mut failing_calls = 0;
        assert!(
            original
                .try_map_card_ids(&mut |_| {
                    failing_calls += 1;
                    if failing_calls == 3 {
                        Err("missing nested face")
                    } else {
                        Ok(1u8)
                    }
                })
                .is_err()
        );
        assert_eq!(failing_calls, 3);
    }
    #[cfg(feature = "serialization")]
    #[test]
    fn origin_card_graph_wire_rebinds_printed_faces_without_sender_allocations() {
        let wire = fixture();
        let json = serde_json::to_value(&wire).unwrap();
        let wire: ContinuousAbilityOrigin<u64, String> = serde_json::from_value(json).unwrap();
        let expected = fixture()
            .try_map_card_ids(&mut |face| Ok::<_, ()>(format!("peer:{face}")))
            .unwrap();
        let restored = wire
            .try_map_card_ids(&mut |face| Ok::<_, ()>(format!("peer:{face}")))
            .unwrap();
        assert_eq!(restored, expected);
        assert_eq!(restored.printed_face.as_deref(), Some("peer:host-face"));
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn origin_card_graph_requires_optional_provenance_fields_and_preserves_explicit_null() {
        type Wire = ContinuousAbilityOrigin<u64, String>;
        let json = serde_json::to_value(fixture()).unwrap();
        for path in [
            vec!["printed_face"],
            vec!["ability", "Borrowed", "effect", "registration_id"],
            vec!["ability", "Borrowed", "effect", "static_ability"],
            vec!["ability", "Borrowed", "effect", "generated_by"],
            vec![
                "ability",
                "Borrowed",
                "effect",
                "generated_by",
                "printed_face",
            ],
            vec!["ability", "Borrowed", "origin", "Level", "printed_face"],
            vec![
                "ability",
                "Borrowed",
                "origin",
                "Level",
                "parent",
                "Effect",
                "effect",
                "registration_id",
            ],
            vec![
                "ability",
                "Borrowed",
                "origin",
                "Level",
                "parent",
                "Effect",
                "effect",
                "static_ability",
            ],
            vec![
                "ability",
                "Borrowed",
                "origin",
                "Level",
                "parent",
                "Effect",
                "effect",
                "generated_by",
            ],
            vec![
                "ability",
                "Borrowed",
                "origin",
                "Level",
                "parent",
                "Effect",
                "effect",
                "generated_by",
                "printed_face",
            ],
        ] {
            let mut bad = json.clone();
            let (field, parents) = path.split_last().unwrap();
            let mut node = &mut bad;
            for parent in parents {
                node = &mut node[*parent];
            }
            assert!(node.as_object_mut().unwrap().remove(*field).is_some());
            assert!(
                serde_json::from_value::<Wire>(bad).is_err(),
                "missing {path:?} silently erases provenance"
            );
        }
        let mut absent = fixture();
        absent.printed_face = None;
        let AbilityOrigin::Borrowed { effect, origin, .. } = &mut absent.ability else {
            panic!("borrowed")
        };
        effect.registration_id = None;
        effect.static_ability = None;
        effect.generated_by = None;
        let AbilityOrigin::Level { printed_face, .. } = origin.as_mut() else {
            panic!("level")
        };
        *printed_face = None;
        let json = serde_json::to_value(absent).unwrap();
        assert!(json["printed_face"].is_null());
        let restored: Wire = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), json);
    }
}

#[cfg(feature = "serialization")]
fn deserialize_present_origin_reference<
    'de,
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    <Option<T> as serde::Deserialize<'de>>::deserialize(deserializer)
}
