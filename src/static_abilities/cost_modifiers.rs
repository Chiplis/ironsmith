//! Cost modification static abilities.
//!
//! These abilities modify the costs of spells being cast.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::ability::SpellFilter;
use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::filter::{AlternativeCastKind, Comparison};
use crate::target::PlayerFilter;
use crate::types::CardType;

fn join_with_and(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let mut out = String::new();
            for (idx, part) in parts.iter().enumerate() {
                if idx > 0 {
                    out.push_str(" and ");
                }
                out.push_str(part);
            }
            out
        }
    }
}

fn describe_comparison(cmp: &Comparison) -> String {
    match cmp {
        Comparison::Equal(v) => v.to_string(),
        Comparison::NotEqual(v) => format!("not equal to {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
    }
}

fn describe_card_type(card_type: CardType) -> &'static str {
    match card_type {
        CardType::Artifact => "artifact",
        CardType::Battle => "battle",
        CardType::Creature => "creature",
        CardType::Enchantment => "enchantment",
        CardType::Instant => "instant",
        CardType::Land => "land",
        CardType::Planeswalker => "planeswalker",
        CardType::Sorcery => "sorcery",
        CardType::Kindred => "kindred",
    }
}

fn describe_colors(colors: ColorSet) -> String {
    let mut words = Vec::new();
    if colors.contains(Color::White) {
        words.push("white".to_string());
    }
    if colors.contains(Color::Blue) {
        words.push("blue".to_string());
    }
    if colors.contains(Color::Black) {
        words.push("black".to_string());
    }
    if colors.contains(Color::Red) {
        words.push("red".to_string());
    }
    if colors.contains(Color::Green) {
        words.push("green".to_string());
    }
    join_with_and(&words)
}

fn describe_player_filter_for_spell_target(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        PlayerFilter::Target(inner) => {
            format!("target {}", describe_player_filter_for_spell_target(inner))
        }
        _ => "a player".to_string(),
    }
}

fn describe_alternative_cast_kind(kind: AlternativeCastKind) -> &'static str {
    match kind {
        AlternativeCastKind::Flashback => "flashback",
        AlternativeCastKind::JumpStart => "jump-start",
        AlternativeCastKind::Escape => "escape",
        AlternativeCastKind::Madness => "madness",
        AlternativeCastKind::Miracle => "miracle",
    }
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
    let mut qualifiers = Vec::<String>::new();
    if let Some(colors) = filter.colors {
        let color_text = describe_colors(colors);
        if !color_text.is_empty() {
            qualifiers.push(color_text);
        }
    }
    for card_type in &filter.excluded_card_types {
        qualifiers.push(format!("non{}", describe_card_type(*card_type)));
    }
    if !filter.subtypes.is_empty() {
        let subtypes = filter
            .subtypes
            .iter()
            .map(|subtype| format!("{subtype:?}"))
            .collect::<Vec<_>>();
        qualifiers.push(join_with_and(&subtypes));
    }
    if !filter.card_types.is_empty() {
        let types = filter
            .card_types
            .iter()
            .map(|card_type| describe_card_type(*card_type).to_string())
            .collect::<Vec<_>>();
        qualifiers.push(join_with_and(&types));
    }

    let mut description = if qualifiers.is_empty() {
        "spells".to_string()
    } else {
        format!("{} spells", qualifiers.join(" "))
    };
    match filter.controller.as_ref() {
        Some(PlayerFilter::You) => description.push_str(" you cast"),
        Some(PlayerFilter::Opponent) => description.push_str(" your opponents cast"),
        _ => {}
    }
    if let Some(power) = &filter.power {
        description.push_str(" with power ");
        description.push_str(&describe_comparison(power));
    }
    if let Some(toughness) = &filter.toughness {
        description.push_str(" with toughness ");
        description.push_str(&describe_comparison(toughness));
    }
    if let Some(mana_value) = &filter.mana_value {
        description.push_str(" with mana value ");
        description.push_str(&describe_comparison(mana_value));
    }
    if let Some(player_filter) = &filter.targets_player {
        description.push_str(" that target ");
        description.push_str(&describe_player_filter_for_spell_target(player_filter));
    }
    if let Some(object_filter) = &filter.targets_object {
        description.push_str(" that target ");
        description.push_str(&object_filter.description());
    }
    if let Some(kind) = filter.alternative_cast {
        description.push_str(" with ");
        description.push_str(describe_alternative_cast_kind(kind));
    }

    description
}

fn describe_flashback_cost_subject(filter: &SpellFilter) -> Option<&'static str> {
    if filter.alternative_cast != Some(AlternativeCastKind::Flashback)
        || !filter.card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.colors.is_some()
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
    {
        return None;
    }
    match filter.controller.as_ref() {
        Some(PlayerFilter::You) => Some("Flashback costs you pay"),
        Some(PlayerFilter::Opponent) => Some("Flashback costs your opponents pay"),
        None | Some(PlayerFilter::Any) => Some("Flashback costs"),
        _ => None,
    }
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
        if let Some(subject) = describe_flashback_cost_subject(&self.filter) {
            let mut line = format!("{subject} cost {amount_text} less");
            if let Some(tail) = tail {
                line.push(' ');
                line.push_str(&tail);
            }
            return line;
        }
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
        if let Some(subject) = describe_flashback_cost_subject(&self.filter) {
            let mut line = format!("{subject} cost {amount_text} more");
            if let Some(tail) = tail {
                line.push(' ');
                line.push_str(&tail);
            }
            return line;
        }
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
