//! Protection-related static abilities.
//!
//! This includes Protection, Ward, and conditional Hexproof.

use super::{StaticAbilityId, StaticAbilityKind, text_utils::join_with_and};
use crate::ability::ProtectionFrom;
use crate::color::Color;
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

fn describe_color_set(colors: crate::color::ColorSet) -> String {
    let mut names = Vec::new();
    if colors.contains(Color::White) {
        names.push("white");
    }
    if colors.contains(Color::Blue) {
        names.push("blue");
    }
    if colors.contains(Color::Black) {
        names.push("black");
    }
    if colors.contains(Color::Red) {
        names.push("red");
    }
    if colors.contains(Color::Green) {
        names.push("green");
    }
    join_with_and(&names)
}

impl StaticAbilityKind for Protection {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Protection
    }

    fn display(&self) -> String {
        match &self.from {
            ProtectionFrom::Color(colors) => {
                let described = describe_color_set(*colors);
                if described.is_empty() {
                    "Protection from colorless".to_string()
                } else {
                    format!("Protection from {}", described)
                }
            }
            ProtectionFrom::AllColors => "Protection from all colors".to_string(),
            ProtectionFrom::Colorless => "Protection from colorless".to_string(),
            ProtectionFrom::Everything => "Protection from everything".to_string(),
            ProtectionFrom::ChosenPlayer => "Protection from the chosen player".to_string(),
            ProtectionFrom::CardType(ct) => format!("Protection from {}", ct.plural_name()),
            ProtectionFrom::Creatures => "Protection from creatures".to_string(),
            ProtectionFrom::Permanents(filter) => {
                let description = filter.description();
                let description = if matches!(filter.zone, Some(crate::zone::Zone::Stack))
                    && filter.stack_kind == Some(crate::filter::StackObjectKind::Spell)
                    && !description.ends_with("spells")
                {
                    description
                        .strip_suffix(" spell")
                        .map(|prefix| format!("{prefix} spells"))
                        .unwrap_or(description)
                } else {
                    description
                };
                format!("Protection from {description}")
            }
        }
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

fn describe_hexproof_from_filter(filter: &ObjectFilter) -> String {
    if !filter.any_of.is_empty() {
        return filter
            .any_of
            .iter()
            .map(describe_hexproof_from_filter)
            .collect::<Vec<_>>()
            .join(" or ");
    }

    let description = filter.description();
    description
        .strip_suffix(" permanent")
        .or_else(|| description.strip_suffix(" spell"))
        .or_else(|| description.strip_suffix(" source"))
        .unwrap_or(description.as_str())
        .to_string()
}

impl StaticAbilityKind for HexproofFrom {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::HexproofFrom
    }

    fn display(&self) -> String {
        format!(
            "Hexproof from {}",
            describe_hexproof_from_filter(&self.filter)
        )
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
    fn test_protection_display_single_color() {
        let prot = Protection::from_color(Color::Black);
        assert_eq!(prot.display(), "Protection from black");
    }

    #[test]
    fn test_protection_display_multi_color() {
        let colors = crate::color::ColorSet::WHITE.union(crate::color::ColorSet::BLUE);
        let prot = Protection::new(ProtectionFrom::Color(colors));
        assert_eq!(prot.display(), "Protection from white and blue");
    }

    #[test]
    fn test_protection_display_permanents_filter() {
        let filter = ObjectFilter::artifact();
        let prot = Protection::new(ProtectionFrom::Permanents(filter));
        assert_eq!(prot.display(), "Protection from artifact");
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
