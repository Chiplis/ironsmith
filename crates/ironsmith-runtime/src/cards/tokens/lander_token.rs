//! Lander token definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Subtype};

/// Creates a Lander token.
///
/// A Lander is an artifact token with:
/// "{2}, {T}, Sacrifice this token: Search your library for a basic land card,
/// put it onto the battlefield tapped, then shuffle."
pub fn lander_token_definition() -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Lander")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Lander]);
    builder
        .clone()
        .parse_text(
            "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        )
        .unwrap_or_else(|_| builder.build())
}

#[cfg(test)]
mod tests {
    use super::lander_token_definition;
    use crate::ability::AbilityKind;
    use crate::types::{CardType, Subtype};

    #[test]
    fn lander_token_has_expected_basic_land_search_ability() {
        let lander = lander_token_definition();
        assert!(lander.card.is_token);
        assert!(lander.card.card_types.contains(&CardType::Artifact));
        assert!(lander.card.subtypes.contains(&Subtype::Lander));
        assert_eq!(lander.abilities.len(), 1);
        match &lander.abilities[0].kind {
            AbilityKind::Activated(_) => {}
            other => panic!("expected activated ability, got {other:?}"),
        }
        let lines = crate::compiled_text::compiled_lines(&lander);
        let joined = lines.join("\n");
        assert!(
            joined.contains("Search your library for a basic land card, put it onto the battlefield tapped, then shuffle"),
            "expected search-to-battlefield-tapped text, got {joined}"
        );
    }
}
