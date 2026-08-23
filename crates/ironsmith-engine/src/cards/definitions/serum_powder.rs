//! Serum Powder card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Serum Powder card definition.
pub fn serum_powder() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Serum Powder")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Any time you could mulligan and Serum Powder is in your hand, you may exile all the cards from your hand, then draw that many cards. (You can do this in addition to taking mulligans.)\n{T}: Add {C}.",
        )
        .expect("Card text should be supported")
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::PregameActionKind;

    #[test]
    fn test_serum_powder_parser_backed_mulligan_redraw_action() {
        let card = serum_powder();

        assert!(card.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if matches!(
                        static_ability.pregame_action_kind(),
                        Some(PregameActionKind::MulliganExileHandDrawSameCount)
                    )
            )
        }));
        assert!(
            card.abilities
                .iter()
                .any(|ability| ability.is_mana_ability())
        );
    }
}
