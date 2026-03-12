//! Gemstone Caverns card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::ManaCost;
use crate::types::{CardType, Supertype};

/// Creates the Gemstone Caverns card definition.
pub fn gemstone_caverns() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Gemstone Caverns")
        .mana_cost(ManaCost::new())
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Land])
        .parse_text(
            "If this card is in your opening hand and you're not the starting player, you may begin the game with Gemstone Caverns on the battlefield with a luck counter on it. If you do, exile a card from your hand.\n{T}: Add {C}. If Gemstone Caverns has a luck counter on it, instead add one mana of any color.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::object::CounterType;
    use crate::static_abilities::PregameActionKind;

    #[test]
    fn test_gemstone_caverns_parser_backed_pregame_action() {
        let card = gemstone_caverns();
        assert_eq!(card.card.name, "Gemstone Caverns");
        assert!(card.card.card_types.contains(&CardType::Land));
        assert!(card.card.supertypes.contains(&Supertype::Legendary));

        assert!(matches!(
            &card.abilities[0].kind,
            AbilityKind::Static(static_ability)
                if matches!(
                    static_ability.pregame_action_kind(),
                    Some(PregameActionKind::BeginOnBattlefield(spec))
                        if spec.require_not_starting_player
                            && spec.exile_cards_from_hand == 1
                            && spec.counters == vec![(CounterType::Luck, 1)]
                )
        ));
        assert!(
            card.abilities
                .iter()
                .any(|ability| ability.is_mana_ability())
        );
    }
}
