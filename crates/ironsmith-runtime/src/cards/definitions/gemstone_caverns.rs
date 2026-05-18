//! Gemstone Caverns card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
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

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::decision::{SelectFirstDecisionMaker, compute_potential_mana};
    use crate::ids::PlayerId;
    use crate::object::CounterType;
    use crate::special_actions::perform_activate_mana_ability;
    use crate::static_abilities::PregameActionKind;
    use crate::zone::Zone;

    fn setup_game() -> crate::game_state::GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn mana_ability_index(card: &CardDefinition) -> usize {
        card.abilities
            .iter()
            .position(|ability| ability.is_mana_ability())
            .expect("Gemstone Caverns should have a mana ability")
    }

    fn colored_mana_total(pool: &crate::player::ManaPool) -> u32 {
        pool.white + pool.blue + pool.black + pool.red + pool.green
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_gemstone_caverns_parser_backed_pregame_action() {
        let card = gemstone_caverns();
        assert_eq!(card.card.name, "Gemstone Caverns");
        assert!(card.card.card_types.contains(&CardType::Land));
        assert!(card.card.supertypes.contains(&Supertype::Legendary));

        assert!(card.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if matches!(
                        static_ability.pregame_action_kind(),
                        Some(PregameActionKind::BeginOnBattlefield(spec))
                            if spec.require_not_starting_player
                                && spec.exile_cards_from_hand == 1
                                && spec.counters == vec![(CounterType::Luck, 1)]
                    )
            )
        }));
        assert!(
            card.abilities
                .iter()
                .any(|ability| ability.is_mana_ability())
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_gemstone_caverns_mana_ability_uses_self_replacement_for_instead_clause() {
        let card = gemstone_caverns();
        let mana_ability = card
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) if activated.is_mana_ability() => Some(activated),
                _ => None,
            })
            .expect("expected an activated mana ability");
        assert_eq!(mana_ability.effects.segments.len(), 1);
        let segment = &mana_ability.effects.segments[0];
        assert_eq!(
            segment.default_effects.len(),
            1,
            "Add {{C}} stays as the default effect"
        );
        assert_eq!(
            segment.self_replacements.len(),
            1,
            "the 'instead' clause must compile to a self-replacement, not an additive conditional"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_gemstone_caverns_compiled_text_matches_oracle() {
        let card = gemstone_caverns();
        let compiled = crate::compiled_text::compiled_text_lines(&card).join("\n");
        assert!(
            compiled.contains(
                "If this card is in your opening hand and you're not the starting player, \
                 you may begin the game with Gemstone Caverns on the battlefield with a luck counter on it."
            ),
            "expected pregame line to render structurally with card name and punctuation, got {compiled}"
        );
        assert!(
            compiled.contains("If you do, exile a card from your hand."),
            "expected exile follow-up sentence with comma + period, got {compiled}"
        );
        assert!(
            compiled.contains(
                "Add {C}. If Gemstone Caverns has a luck counter on it, instead add one mana of any color."
            ),
            "expected mana ability to render as 'Add {{C}}. If <cond>, instead add ...', got {compiled}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_gemstone_caverns_luck_counter_replaces_colorless_mana_at_runtime() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = gemstone_caverns();
        let ability_index = mana_ability_index(&card);
        let gemstone = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        game.object_mut(gemstone)
            .expect("Gemstone Caverns should exist")
            .add_counters(CounterType::Luck, 1);

        let potential = compute_potential_mana(&game, alice);
        assert_eq!(
            potential.colorless, 0,
            "a luck counter should make potential mana use the self-replacement branch"
        );
        assert!(
            potential.white > 0
                && potential.blue > 0
                && potential.black > 0
                && potential.red > 0
                && potential.green > 0,
            "Gemstone Caverns with a luck counter should be treated as able to add any color, got {potential:?}"
        );

        let mut dm = SelectFirstDecisionMaker;
        perform_activate_mana_ability(&mut game, alice, gemstone, ability_index, &mut dm)
            .expect("Gemstone Caverns mana ability should activate");

        let pool = &game.player(alice).expect("alice should exist").mana_pool;
        assert_eq!(
            pool.colorless, 0,
            "the default colorless mana must not be added when the luck-counter replacement applies"
        );
        assert_eq!(
            colored_mana_total(pool),
            1,
            "the replacement branch should add exactly one colored mana, got {pool:?}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_gemstone_caverns_without_luck_counter_adds_colorless_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = gemstone_caverns();
        let ability_index = mana_ability_index(&card);
        let gemstone = game.create_object_from_definition(&card, alice, Zone::Battlefield);

        let potential = compute_potential_mana(&game, alice);
        assert_eq!(
            potential.colorless, 1,
            "without a luck counter Gemstone Caverns should still be treated as a colorless source"
        );

        let mut dm = SelectFirstDecisionMaker;
        perform_activate_mana_ability(&mut game, alice, gemstone, ability_index, &mut dm)
            .expect("Gemstone Caverns mana ability should activate");

        let pool = &game.player(alice).expect("alice should exist").mana_pool;
        assert_eq!(pool.colorless, 1);
        assert_eq!(colored_mana_total(pool), 0);
    }
}
