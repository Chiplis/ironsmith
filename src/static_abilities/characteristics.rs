//! Characteristic-defining abilities.
//!
//! These abilities define characteristics of permanents like power/toughness
//! that are calculated dynamically.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::continuous::{
    ContinuousEffect, EffectSourceType, EffectTarget, Modification, PtSublayer,
};
use crate::effect::Value;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;

/// Characteristic-defining ability for power/toughness.
///
/// These are applied in layer 7a before other P/T modifications.
/// Used for creatures like Tarmogoyf or Construct tokens from Urza's Saga.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacteristicDefiningPT {
    pub power: Value,
    pub toughness: Value,
}

impl CharacteristicDefiningPT {
    pub fn new(power: Value, toughness: Value) -> Self {
        Self { power, toughness }
    }

    /// Create a fixed P/T (e.g., for a token).
    pub fn fixed(power: i32, toughness: i32) -> Self {
        Self::new(Value::Fixed(power), Value::Fixed(toughness))
    }
}

impl StaticAbilityKind for CharacteristicDefiningPT {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CharacteristicDefiningPT
    }

    fn display(&self) -> String {
        if self.power == self.toughness {
            format!(
                "This creature's power and toughness are each equal to {}",
                display_value(&self.power)
            )
        } else {
            format!(
                "This creature's power is {}, and its toughness is {}",
                display_value(&self.power),
                display_value(&self.toughness)
            )
        }
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Specific(source), // Applies to itself
                Modification::SetPowerToughness {
                    power: self.power.clone(),
                    toughness: self.toughness.clone(),
                    sublayer: PtSublayer::CharacteristicDefining,
                },
            )
            .with_source_type(EffectSourceType::CharacteristicDefining),
        ]
    }
}

fn display_value(value: &Value) -> String {
    let strip_article = |text: &str| {
        text.strip_prefix("a ")
            .or_else(|| text.strip_prefix("an "))
            .or_else(|| text.strip_prefix("the "))
            .unwrap_or(text)
            .to_string()
    };
    let pluralize_phrase = |text: &str| {
        let normalized = text.trim();
        if normalized.is_empty() {
            return "objects".to_string();
        }
        let mut words: Vec<String> = normalized.split_whitespace().map(str::to_string).collect();
        let qualifier_start = words.iter().position(|word| {
            matches!(
                word.as_str(),
                "you"
                    | "an"
                    | "a"
                    | "target"
                    | "that"
                    | "in"
                    | "from"
                    | "with"
                    | "without"
                    | "on"
                    | "under"
            )
        });
        let noun_idx = qualifier_start
            .and_then(|idx| idx.checked_sub(1))
            .unwrap_or_else(|| words.len().saturating_sub(1));
        if let Some(noun) = words.get_mut(noun_idx)
            && !noun.ends_with('s')
        {
            noun.push('s');
        }
        words.join(" ")
    };

    match value {
        Value::Fixed(n) => n.to_string(),
        Value::Add(left, right) => format!("{} plus {}", display_value(left), display_value(right)),
        Value::X => "X".to_string(),
        Value::XTimes(k) => {
            if *k == 1 {
                "X".to_string()
            } else {
                format!("{} times X", k)
            }
        }
        Value::SourcePower => "its power".to_string(),
        Value::SourceToughness => "its toughness".to_string(),
        Value::Count(filter) => {
            let desc = strip_article(&filter.description());
            let mut phrase = pluralize_phrase(&desc);
            if matches!(filter.controller, Some(PlayerFilter::You))
                && !phrase.contains("you control")
            {
                phrase.push_str(" you control");
            } else if matches!(filter.controller, Some(PlayerFilter::Opponent))
                && !phrase.contains("an opponent controls")
                && !phrase.contains("opponents control")
            {
                phrase.push_str(" an opponent controls");
            }
            format!("the number of {phrase}")
        }
        Value::CountScaled(filter, multiplier) => {
            let desc = strip_article(&filter.description());
            let phrase = pluralize_phrase(&desc);
            format!("{multiplier} times the number of {phrase}")
        }
        Value::CreaturesDiedThisTurn => "the number of creatures that died this turn".to_string(),
        Value::CountPlayers(_) => "the number of players".to_string(),
        _ => format!("{:?}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::ObjectFilter;

    #[test]
    fn test_characteristic_defining_pt() {
        let cdp = CharacteristicDefiningPT::fixed(3, 3);
        assert_eq!(cdp.id(), StaticAbilityId::CharacteristicDefiningPT);
    }

    #[test]
    fn test_generates_effects() {
        let cdp = CharacteristicDefiningPT::fixed(2, 2);
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);

        let effects = cdp.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].source_type,
            EffectSourceType::CharacteristicDefining
        ));
    }

    #[test]
    fn test_display_count_strips_leading_article() {
        let ability = CharacteristicDefiningPT::new(
            Value::Count(ObjectFilter::creature().you_control()),
            Value::Count(ObjectFilter::creature().you_control()),
        );
        assert_eq!(
            ability.display(),
            "This creature's power and toughness are each equal to the number of creatures you control"
        );
    }

    #[test]
    fn test_display_additive_count_value() {
        let value = Value::Add(
            Box::new(Value::Fixed(2)),
            Box::new(Value::Count(ObjectFilter::creature().you_control())),
        );
        let ability = CharacteristicDefiningPT::new(value.clone(), value);
        assert_eq!(
            ability.display(),
            "This creature's power and toughness are each equal to 2 plus the number of creatures you control"
        );
    }

    #[test]
    fn test_display_count_with_color_adjective_pluralizes_card_not_color() {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(crate::zone::Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        filter.colors = Some(crate::color::ColorSet::BLACK);
        let ability = CharacteristicDefiningPT::new(Value::Count(filter.clone()), Value::Count(filter));
        assert!(
            ability.display().contains("black cards in your graveyard"),
            "expected color-adjective count to pluralize 'card', got {}",
            ability.display()
        );
    }
}
