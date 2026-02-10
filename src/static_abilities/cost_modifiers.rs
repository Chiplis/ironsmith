//! Cost modification static abilities.
//!
//! These abilities modify the costs of spells being cast.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::ability::SpellFilter;
use crate::effect::Value;
use crate::filter::ObjectFilter;
use crate::target::PlayerFilter;

fn strip_indefinite_article(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("a ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("an ") {
        return rest;
    }
    trimmed
}

fn pluralize_spell_filter_text(text: &str) -> String {
    let mut out = strip_indefinite_article(text).to_string();
    if out.starts_with("spell ") {
        out = out.replacen("spell ", "spells ", 1);
    } else if out.contains(" spell") {
        out = out.replacen(" spell", " spells", 1);
    } else if out == "spell" {
        out = "spells".to_string();
    }
    out = out.replace(" that targets ", " that target ");
    out = out.replace(" that targets", " that target");
    out
}

fn describe_cost_modifier_amount(amount: &Value) -> (String, Option<String>) {
    match amount {
        Value::Fixed(n) => (format!("{{{n}}}"), None),
        Value::X => ("{X}".to_string(), None),
        Value::Count(filter) => (
            "{1}".to_string(),
            Some(format!("for each {}", filter.description())),
        ),
        Value::CardTypesInGraveyard(player) => {
            let owner = match player {
                PlayerFilter::You => "your",
                PlayerFilter::Opponent => "an opponent's",
                _ => "a player's",
            };
            (
                "{1}".to_string(),
                Some(format!(
                    "for each card type among cards in {owner} graveyard"
                )),
            )
        }
        _ => ("{X}".to_string(), None),
    }
}

fn describe_spell_filter(filter: &SpellFilter) -> String {
    let mut object_filter = ObjectFilter::default();
    object_filter.zone = Some(crate::zone::Zone::Stack);
    object_filter.card_types = filter.card_types.clone();
    object_filter.subtypes = filter.subtypes.clone();
    object_filter.colors = filter.colors;
    object_filter.controller = filter.controller.clone();
    object_filter.targets_player = filter.targets_player.clone();
    object_filter.targets_object = filter.targets_object.clone().map(Box::new);
    pluralize_spell_filter_text(&object_filter.description())
}

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
        let (amount_text, tail) = describe_cost_modifier_amount(&self.reduction);
        let mut line = format!(
            "{} cost {} less to cast",
            describe_spell_filter(&self.filter),
            amount_text
        );
        if let Some(tail) = tail {
            line.push(' ');
            line.push_str(&tail);
        }
        line
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
        let (amount_text, tail) = describe_cost_modifier_amount(&self.increase);
        let mut line = format!(
            "{} cost {} more to cast",
            describe_spell_filter(&self.filter),
            amount_text
        );
        if let Some(tail) = tail {
            line.push(' ');
            line.push_str(&tail);
        }
        line
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

/// Cost increase per additional target:
/// "This spell costs {N} more to cast for each target beyond the first."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostIncreasePerAdditionalTarget {
    pub amount: u32,
}

impl CostIncreasePerAdditionalTarget {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for CostIncreasePerAdditionalTarget {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostIncreasePerAdditionalTarget
    }

    fn display(&self) -> String {
        format!(
            "This spell costs {{{}}} more to cast for each target beyond the first",
            self.amount
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_increase_per_additional_target(&self) -> Option<u32> {
        Some(self.amount)
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
