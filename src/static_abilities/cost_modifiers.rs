//! Cost modification static abilities.
//!
//! These abilities modify the costs of spells being cast.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::ability::SpellFilter;
use crate::effect::Value;

/// Affinity for artifacts - This spell costs {1} less to cast for each artifact you control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AffinityForArtifacts;

impl StaticAbilityKind for AffinityForArtifacts {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AffinityForArtifacts
    }

    fn display(&self) -> String {
        "Affinity for artifacts".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_affinity(&self) -> bool {
        true
    }
}

/// Delve - Each card you exile from your graveyard while casting this spell pays for {1}.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Delve;

impl StaticAbilityKind for Delve {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Delve
    }

    fn display(&self) -> String {
        "Delve".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_delve(&self) -> bool {
        true
    }
}

/// Convoke - Your creatures can help cast this spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Convoke;

impl StaticAbilityKind for Convoke {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Convoke
    }

    fn display(&self) -> String {
        "Convoke".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_convoke(&self) -> bool {
        true
    }
}

/// Improvise - Your artifacts can help cast this spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Improvise;

impl StaticAbilityKind for Improvise {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Improvise
    }

    fn display(&self) -> String {
        "Improvise".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_improvise(&self) -> bool {
        true
    }
}

/// Cost reduction: "Spells cost {N} less to cast"
#[derive(Debug, Clone, PartialEq)]
pub struct CostReduction {
    pub filter: SpellFilter,
    pub reduction: Value,
}

impl CostReduction {
    pub fn new(filter: SpellFilter, reduction: Value) -> Self {
        Self { filter, reduction }
    }
}

impl StaticAbilityKind for CostReduction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostReduction
    }

    fn display(&self) -> String {
        "Spells cost less to cast".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_reduction(&self) -> Option<&CostReduction> {
        Some(self)
    }
}

/// Cost increase: "Spells cost {N} more to cast"
#[derive(Debug, Clone, PartialEq)]
pub struct CostIncrease {
    pub filter: SpellFilter,
    pub increase: Value,
}

impl CostIncrease {
    pub fn new(filter: SpellFilter, increase: Value) -> Self {
        Self { filter, increase }
    }
}

impl StaticAbilityKind for CostIncrease {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostIncrease
    }

    fn display(&self) -> String {
        "Spells cost more to cast".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_increase(&self) -> Option<&CostIncrease> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affinity() {
        let affinity = AffinityForArtifacts;
        assert_eq!(affinity.id(), StaticAbilityId::AffinityForArtifacts);
        assert!(affinity.modifies_costs());
    }

    #[test]
    fn test_delve() {
        let delve = Delve;
        assert_eq!(delve.id(), StaticAbilityId::Delve);
        assert!(delve.modifies_costs());
    }

    #[test]
    fn test_convoke() {
        let convoke = Convoke;
        assert_eq!(convoke.id(), StaticAbilityId::Convoke);
        assert!(convoke.modifies_costs());
    }
}
