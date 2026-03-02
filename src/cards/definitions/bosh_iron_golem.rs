//! Bosh, Iron Golem card definition.

use crate::ability::Ability;
use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{Effect, Value};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::{CardType, Subtype, Supertype};

/// Bosh, Iron Golem - {8}
/// Legendary Artifact Creature — Golem
/// Trample
/// {3}{R}, Sacrifice an artifact: Bosh, Iron Golem deals damage equal to the
/// sacrificed artifact's mana value to any target.
/// 6/7
pub fn bosh_iron_golem() -> CardDefinition {
    let activation_cost = TotalCost::from_costs(vec![
        Cost::mana(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
        ])),
        Cost::sacrifice(ObjectFilter::artifact().you_control()),
    ]);
    let damage_effect = Effect::deal_damage(
        Value::ManaValueOf(Box::new(ChooseSpec::tagged("sacrifice_cost_0"))),
        ChooseSpec::AnyTarget,
    );

    CardDefinitionBuilder::new(CardId::new(), "Bosh, Iron Golem")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(8)]]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(6, 7))
        .oracle_text(
            "Trample\n{3}{R}, Sacrifice an artifact: Bosh, Iron Golem deals damage equal to the sacrificed artifact's mana value to any target.",
        )
        .trample()
        .with_ability(
            Ability::activated(activation_cost, vec![damage_effect]).with_text(
                "{3}{R}, Sacrifice an artifact: Bosh, Iron Golem deals damage equal to the sacrificed artifact's mana value to any target.",
            ),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_bosh_iron_golem_basic_properties() {
        let def = bosh_iron_golem();
        assert_eq!(def.name(), "Bosh, Iron Golem");
        assert!(def.card.card_types.contains(&CardType::Artifact));
        assert!(def.card.card_types.contains(&CardType::Creature));
        assert_eq!(def.card.mana_value(), 8);

        let pt = def
            .card
            .power_toughness
            .as_ref()
            .expect("Bosh should have power/toughness");
        assert_eq!(pt.power.base_value(), 6);
        assert_eq!(pt.toughness.base_value(), 7);
    }

    #[test]
    fn test_bosh_iron_golem_has_activated_ability() {
        let def = bosh_iron_golem();
        assert!(def
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))));
    }
}
