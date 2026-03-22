//! Demonic Consultation card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Demonic Consultation - {B}
/// Instant
/// Choose a card name. Exile the top six cards of your library, then reveal
/// cards from the top of your library until you reveal the chosen card. Put
/// that card into your hand and exile all other cards revealed this way.
pub fn demonic_consultation() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Demonic Consultation")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal the chosen card. Put that card into your hand and exile all other cards revealed this way.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::cards::definitions::lightning_bolt;
    use crate::decision::DecisionMaker;
    use crate::game_loop::resolve_stack_entry_with;
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    struct ChooseBoltDm;

    impl DecisionMaker for ChooseBoltDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description == "Lightning Bolt")
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    #[test]
    fn test_demonic_consultation_basic_properties() {
        let def = demonic_consultation();

        assert_eq!(def.name(), "Demonic Consultation");
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 1);
        assert_eq!(
            def.spell_effect
                .as_ref()
                .expect("spell effect exists")
                .len(),
            2
        );
    }

    #[test]
    fn test_demonic_consultation_exiles_six_then_finds_named_card() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let spell_def = demonic_consultation();
        let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);

        game.create_object_from_definition(&lightning_bolt(), alice, Zone::Library);
        for idx in 0..6 {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), format!("Filler {idx}"))
                    .card_types(vec![CardType::Artifact])
                    .build(),
                alice,
                Zone::Library,
            );
        }

        game.stack.push(StackEntry::new(spell_id, alice));

        let mut dm = ChooseBoltDm;
        resolve_stack_entry_with(&mut game, &mut dm).expect("consultation should resolve");

        let hand_names: Vec<_> = game
            .player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| obj.name.clone()))
            .collect();
        assert!(
            hand_names.iter().any(|name| name == "Lightning Bolt"),
            "The chosen card should end up in hand"
        );

        let exiled_fillers = game
            .exile
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name.starts_with("Filler "))
            .count();
        assert_eq!(
            exiled_fillers, 6,
            "The six cards exiled before the reveal should stay in exile"
        );
    }
}
