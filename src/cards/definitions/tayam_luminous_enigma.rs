//! Tayam, Luminous Enigma card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Tayam, Luminous Enigma - {1}{W}{B}{G}
/// Legendary Creature — Nightmare Beast (3/3)
/// Each other creature you control enters the battlefield with an additional vigilance counter on it.
/// {3}, Remove three counters from among creatures you control:
/// Mill three cards, then return a permanent card with mana value 3 or less from your graveyard
/// to the battlefield.
pub fn tayam_luminous_enigma() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Tayam, Luminous Enigma")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Green],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Nightmare, Subtype::Beast])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Each other creature you control enters the battlefield with an additional vigilance counter on it.\n\
             {3}, Remove three counters from among creatures you control: Mill three cards, then return a permanent card with mana value 3 or less from your graveyard to the battlefield.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_tayam_basic_properties() {
        let def = tayam_luminous_enigma();
        assert_eq!(def.name(), "Tayam, Luminous Enigma");
        assert!(def.card.supertypes.contains(&Supertype::Legendary));
        assert!(def.card.card_types.contains(&CardType::Creature));
        assert!(def.card.subtypes.contains(&Subtype::Nightmare));
        assert!(def.card.subtypes.contains(&Subtype::Beast));
        assert_eq!(def.card.mana_value(), 4);
    }

    #[test]
    fn test_tayam_has_etb_counter_static_and_activated_ability() {
        let def = tayam_luminous_enigma();
        assert!(
            def.abilities.iter().any(|ability| {
                matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::EnterWithCountersForFilter
                )
            }),
            "expected ETB replacement static ability"
        );
        assert!(
            def.abilities
                .iter()
                .any(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        );
    }
}
