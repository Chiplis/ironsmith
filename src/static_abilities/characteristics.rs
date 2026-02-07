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
        let mut parts = normalized.splitn(2, ' ');
        let first = parts.next().unwrap_or_default();
        let rest = parts.next();
        let plural_first = if first.ends_with('s') {
            first.to_string()
        } else {
            format!("{first}s")
        };
        if let Some(rest) = rest {
            format!("{plural_first} {rest}")
        } else {
            plural_first
        }
    };

    match value {
        Value::Fixed(n) => n.to_string(),
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
}
