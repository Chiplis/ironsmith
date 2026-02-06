//! Protection-related static abilities.
//!
//! This includes Protection, Ward, and conditional Hexproof.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::ability::ProtectionFrom;
use crate::cost::TotalCost;
use crate::target::ObjectFilter;

/// Protection from [quality].
///
/// A creature with protection from [quality] can't be:
/// - Damaged by sources with that quality
/// - Enchanted/equipped by permanents with that quality
/// - Blocked by creatures with that quality
/// - Targeted by spells/abilities with that quality
#[derive(Debug, Clone, PartialEq)]
pub struct Protection {
    pub from: ProtectionFrom,
}

impl Protection {
    pub fn new(from: ProtectionFrom) -> Self {
        Self { from }
    }

    pub fn from_color(color: crate::color::Color) -> Self {
        Self::new(ProtectionFrom::Color(color.into()))
    }

    pub fn from_all_colors() -> Self {
        Self::new(ProtectionFrom::AllColors)
    }

    pub fn from_everything() -> Self {
        Self::new(ProtectionFrom::Everything)
    }

    pub fn from_card_type(card_type: crate::types::CardType) -> Self {
        Self::new(ProtectionFrom::CardType(card_type))
    }
}

impl StaticAbilityKind for Protection {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Protection
    }

    fn display(&self) -> String {
        match &self.from {
            ProtectionFrom::Color(color) => format!("Protection from {:?}", color).to_lowercase(),
            ProtectionFrom::AllColors => "Protection from all colors".to_string(),
            ProtectionFrom::Colorless => "Protection from colorless".to_string(),
            ProtectionFrom::Everything => "Protection from everything".to_string(),
            ProtectionFrom::CardType(ct) => format!("Protection from {:?}s", ct).to_lowercase(),
            ProtectionFrom::Creatures => "Protection from creatures".to_string(),
            ProtectionFrom::Permanents(_) => "Protection from permanents".to_string(),
        }
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn has_protection(&self) -> bool {
        true
    }

    fn protection_from(&self) -> Option<&ProtectionFrom> {
        Some(&self.from)
    }
}

/// Hexproof from [quality].
///
/// A creature with hexproof from [quality] can't be the target of spells
/// or abilities controlled by opponents that have that quality.
#[derive(Debug, Clone, PartialEq)]
pub struct HexproofFrom {
    pub filter: ObjectFilter,
}

impl HexproofFrom {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for HexproofFrom {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::HexproofFrom
    }

    fn display(&self) -> String {
        format!("Hexproof from {}", self.filter.description())
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn has_hexproof(&self) -> bool {
        // HexproofFrom is NOT full hexproof - return false so that
        // the special HexproofFrom check handles it instead.
        false
    }

    fn hexproof_from_filter(&self) -> Option<&crate::target::ObjectFilter> {
        Some(&self.filter)
    }
}

/// Ward {cost}.
///
/// Whenever this permanent becomes the target of a spell or ability
/// an opponent controls, counter it unless that player pays {cost}.
#[derive(Debug, Clone, PartialEq)]
pub struct Ward {
    pub cost: TotalCost,
}

impl Ward {
    pub fn new(cost: TotalCost) -> Self {
        Self { cost }
    }
}

impl StaticAbilityKind for Ward {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Ward
    }

    fn display(&self) -> String {
        format!("Ward {}", self.cost.display())
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn ward_cost(&self) -> Option<&TotalCost> {
        Some(&self.cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn test_protection_from_color() {
        let prot = Protection::from_color(Color::Black);
        assert_eq!(prot.id(), StaticAbilityId::Protection);
        assert!(prot.has_protection());
        // from_color converts Color to ColorSet, so check it contains Black
        if let Some(ProtectionFrom::Color(colors)) = prot.protection_from() {
            assert!(colors.contains(Color::Black));
        } else {
            panic!("Expected ProtectionFrom::Color");
        }
    }

    #[test]
    fn test_protection_from_all_colors() {
        let prot = Protection::from_all_colors();
        assert!(prot.has_protection());
        assert!(matches!(
            prot.protection_from(),
            Some(ProtectionFrom::AllColors)
        ));
    }

    #[test]
    fn test_ward() {
        use crate::costs::Cost;
        let cost = TotalCost::from_cost(Cost::life(2));
        let ward = Ward::new(cost.clone());
        assert_eq!(ward.id(), StaticAbilityId::Ward);
        assert!(ward.ward_cost().is_some());
    }

    #[test]
    fn test_hexproof_from() {
        let filter = ObjectFilter::default();
        let hexproof = HexproofFrom::new(filter.clone());
        assert_eq!(hexproof.id(), StaticAbilityId::HexproofFrom);
        // HexproofFrom is NOT full hexproof - it only blocks specific sources
        assert!(!hexproof.has_hexproof());
        // But it should report the filter
        assert!(hexproof.hexproof_from_filter().is_some());
        assert_eq!(hexproof.hexproof_from_filter(), Some(&filter));
    }
}
