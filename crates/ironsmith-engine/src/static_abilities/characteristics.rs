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
use crate::runtime_display::describe_value;
use crate::target::ChooseSpec;

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

    fn prefers_card_name_subject(&self) -> bool {
        self.power
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject)
            || self
                .toughness
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject)
    }

    fn display(&self) -> String {
        let describe_characteristic_value = |value: &Value| {
            describe_value(value).replace(" counters on this source", " counters on it")
        };
        if self.power.unhinted() == self.toughness.unhinted() {
            format!(
                "This creature's power and toughness are each equal to {}",
                describe_characteristic_value(&self.power)
            )
        } else if let Some(offset) = toughness_is_power_plus_fixed(&self.power, &self.toughness) {
            format!(
                "This creature's power is equal to {} and its toughness is equal to that number plus {}",
                describe_characteristic_value(&self.power),
                offset
            )
        } else if is_own_power(&self.power) {
            format!(
                "This creature's toughness is equal to {}",
                describe_characteristic_value(&self.toughness)
            )
        } else if is_own_toughness(&self.toughness) {
            format!(
                "This creature's power is equal to {}",
                describe_characteristic_value(&self.power)
            )
        } else {
            format!(
                "This creature's power is {}, and its toughness is {}",
                describe_characteristic_value(&self.power),
                describe_characteristic_value(&self.toughness)
            )
        }
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

/// The source's own power, in either the authored (`SourcePower`) or lowered
/// (`PowerOf(Source)`) spelling — lowering rewrites the former into the latter,
/// so a display that only matches one form falls through to the generic
/// two-clause phrasing and prints a tautological "power is this power" half.
fn is_own_power(value: &Value) -> bool {
    match value.unhinted() {
        Value::SourcePower => true,
        Value::PowerOf(spec) => matches!(spec.unhinted(), ChooseSpec::Source),
        _ => false,
    }
}

/// The source's own toughness — see [`is_own_power`].
fn is_own_toughness(value: &Value) -> bool {
    match value.unhinted() {
        Value::SourceToughness => true,
        Value::ToughnessOf(spec) => matches!(spec.unhinted(), ChooseSpec::Source),
        _ => false,
    }
}

fn toughness_is_power_plus_fixed(power: &Value, toughness: &Value) -> Option<i32> {
    let power = power.unhinted();
    match toughness.unhinted() {
        Value::Add(left, right) if left.unhinted() == power => match right.unhinted() {
            Value::Fixed(offset) if *offset > 0 => Some(*offset),
            _ => None,
        },
        Value::Add(left, right) if right.unhinted() == power => match left.unhinted() {
            Value::Fixed(offset) if *offset > 0 => Some(*offset),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::ObjectFilter;
    use crate::target::PlayerFilter;

    #[test]
    fn test_characteristic_defining_pt() {
        let cdp = CharacteristicDefiningPT::fixed(3, 3);
        assert_eq!(cdp.id(), StaticAbilityId::CharacteristicDefiningPT);
    }

    #[test]
    fn named_source_surface_prefers_the_card_name_subject() {
        let value =
            Value::Fixed(3).with_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject);
        let cdp = CharacteristicDefiningPT::new(value.clone(), value);

        assert!(cdp.prefers_card_name_subject());
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
    fn test_display_uses_self_pronoun_for_counters_on_source() {
        let value = Value::CountersOnSource(crate::object::CounterType::Time);
        let ability = CharacteristicDefiningPT::new(value.clone(), value);

        assert_eq!(
            ability.display(),
            "This creature's power and toughness are each equal to the number of time counters on it"
        );
    }

    #[test]
    fn test_display_power_only_omits_source_toughness_placeholder() {
        let mut filter = ObjectFilter::land();
        filter.zone = Some(crate::zone::Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        let ability = CharacteristicDefiningPT::new(Value::Count(filter), Value::SourceToughness);
        assert_eq!(
            ability.display(),
            "This creature's power is equal to the number of land cards in your graveyard"
        );
    }

    #[test]
    fn test_display_toughness_only_omits_source_power_placeholder() {
        let ability = CharacteristicDefiningPT::new(
            Value::SourcePower,
            Value::Count(
                ObjectFilter::default()
                    .with_subtype(crate::types::Subtype::Knight)
                    .you_control(),
            ),
        );
        assert_eq!(
            ability.display(),
            "This creature's toughness is equal to the number of Knights you control"
        );
    }

    #[test]
    fn test_display_count_with_color_adjective_pluralizes_card_not_color() {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(crate::zone::Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        filter.colors = Some(crate::color::ColorSet::BLACK);
        let ability =
            CharacteristicDefiningPT::new(Value::Count(filter.clone()), Value::Count(filter));
        assert!(
            ability.display().contains("black cards in your graveyard"),
            "expected color-adjective count to pluralize 'card', got {}",
            ability.display()
        );
    }
}
