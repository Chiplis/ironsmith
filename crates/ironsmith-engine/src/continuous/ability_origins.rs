//! Runtime identity of abilities, independent of their semantic definitions.
use super::ContinuousEffect;
use crate::ability::Ability;
use crate::ids::ObjectId;
use crate::object::SharedVec;
use crate::static_abilities::StaticAbilityInstanceId;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbilityEffectOrigin {
    source: ObjectId,
    timestamp: u64,
    static_ability: Option<StaticAbilityInstanceId>,
}
impl From<&ContinuousEffect> for AbilityEffectOrigin {
    fn from(effect: &ContinuousEffect) -> Self {
        Self {
            source: effect.source,
            timestamp: effect.timestamp,
            static_ability: effect
                .originating_static_ability
                .as_ref()
                .map(|ability| ability.instance_id()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AbilityOrigin {
    Printed(usize),
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
    /// Return the permanent whose effect supplied this ability, when the
    /// ability was granted by a continuous effect.
    pub(crate) fn effect_source(&self) -> Option<ObjectId> {
        match self {
            Self::Printed(_) => None,
            Self::Effect { effect, .. } => Some(effect.source),
            Self::Borrowed { effect, .. } => Some(effect.source),
        }
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
