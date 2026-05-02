//! Card definition for Flooded Strand.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Flooded Strand card definition.
///
/// Flooded Strand
/// Land
/// {T}, Pay 1 life, Sacrifice Flooded Strand: Search your library for an Island or Plains card,
/// put it onto the battlefield, then shuffle.
pub fn flooded_strand() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Flooded Strand")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}, Pay 1 life, Sacrifice Flooded Strand: Search your library for an Island or Plains card, put it onto the battlefield, then shuffle.",
        )
        .expect("Card text should be supported")
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_flooded_strand_basic_properties() {
        let def = flooded_strand();
        assert_eq!(def.name(), "Flooded Strand");
        assert!(def.card.is_land());
        assert_eq!(def.card.mana_value(), 0);
        assert_eq!(def.card.colors().count(), 0);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_flooded_strand_has_activated_ability() {
        let def = flooded_strand();
        assert_eq!(def.abilities.len(), 1);
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_flooded_strand_ability_costs() {
        let def = flooded_strand();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // Check non-mana costs for tap, life, and sacrifice
            assert!(activated.has_tap_cost());
            assert_eq!(activated.life_cost_amount(), Some(1));
            assert!(activated.has_sacrifice_self_cost());
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_flooded_strand_search_filter() {
        let def = flooded_strand();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            let debug_str = format!("{:?}", activated.effects[0]);
            assert!(debug_str.contains("Island"));
            assert!(debug_str.contains("Plains"));
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_flooded_strand_not_mana_ability() {
        let def = flooded_strand();
        assert!(!def.abilities[0].is_mana_ability());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_flooded_strand_compiled_text_uses_card_subject_for_sacrifice_cost() {
        let rendered = crate::compiled_text::compiled_text_lines(&flooded_strand()).join(" ");

        assert!(
            rendered.contains("Sacrifice this land"),
            "expected source sacrifice cost to render with card subject, got {rendered}"
        );
        assert!(
            !rendered.contains('~'),
            "source placeholder should not leak into compiled text, got {rendered}"
        );
    }
}
