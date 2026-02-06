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
        Value::Count(filter) => format!("the number of {}", filter.description()),
        Value::CountPlayers(_) => "the number of players".to_string(),
        _ => format!("{:?}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
