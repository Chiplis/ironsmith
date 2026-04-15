//! Tayam, Luminous Enigma card definition.

use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Tayam, Luminous Enigma - {1}{W}{B}{G}
/// Legendary Creature — Nightmare Beast (3/3)
/// Each other creature you control enters with an additional vigilance counter on it.
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
            "Each other creature you control enters with an additional vigilance counter on it.\n\
             {3}, Remove three counters from among creatures you control: Mill three cards, then return a permanent card with mana value 3 or less from your graveyard to the battlefield.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::object::CounterType;
    use crate::static_abilities::StaticAbilityId;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

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

    #[test]
    fn test_tayam_grants_vigilance_counter_to_other_creatures_entering() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let _tayam_id =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let test_creature = CardBuilder::new(CardId::new(), "Tayam Counter Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let alice_hand_id = game.create_object_from_card(&test_creature, alice, Zone::Hand);
        let alice_entry = game
            .move_object_with_etb_processing(alice_hand_id, Zone::Battlefield)
            .expect("alice creature should enter battlefield");
        let alice_entered = game
            .object(alice_entry.new_id)
            .expect("alice entered object should exist");
        assert_eq!(
            alice_entered.counters.get(&CounterType::Vigilance).copied(),
            Some(1),
            "other creature you control should enter with a vigilance counter"
        );

        let bob_hand_id = game.create_object_from_card(&test_creature, bob, Zone::Hand);
        let bob_entry = game
            .move_object_with_etb_processing(bob_hand_id, Zone::Battlefield)
            .expect("bob creature should enter battlefield");
        let bob_entered = game
            .object(bob_entry.new_id)
            .expect("bob entered object should exist");
        assert_eq!(
            bob_entered.counters.get(&CounterType::Vigilance),
            None,
            "opponents' creatures should not get Tayam's vigilance counter"
        );
    }
}
