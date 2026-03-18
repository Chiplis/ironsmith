//! Tainted Pact card definition.

use crate::cards::CardDefinition;

/// Tainted Pact - {1}{B}
/// Instant
/// Exile the top card of your library. You may put that card into your hand
/// unless it has the same name as another card exiled this way. Repeat this
/// process until you put a card into your hand or you exile two cards with the
/// same name, whichever comes first.
pub fn tainted_pact() -> CardDefinition {
    crate::cards::handwritten_runtime::tainted_pact()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::game_loop::resolve_stack_entry_with;
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    struct BoolSequenceDm {
        answers: Vec<bool>,
        index: usize,
    }

    impl BoolSequenceDm {
        fn new(answers: Vec<bool>) -> Self {
            Self { answers, index: 0 }
        }
    }

    impl DecisionMaker for BoolSequenceDm {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let answer = self.answers.get(self.index).copied().unwrap_or(false);
            self.index += 1;
            answer
        }
    }

    #[test]
    fn test_tainted_pact_basic_properties() {
        let def = tainted_pact();

        assert_eq!(def.name(), "Tainted Pact");
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 2);
        assert_eq!(
            def.spell_effect.as_ref().expect("spell effect exists").len(),
            1
        );
    }

    #[test]
    fn test_tainted_pact_can_skip_first_card_and_take_second() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let spell_id = game.create_object_from_definition(&tainted_pact(), alice, Zone::Stack);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Second Card")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "First Card")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );

        game.stack.push(StackEntry::new(spell_id, alice));

        let mut dm = BoolSequenceDm::new(vec![false, true]);
        resolve_stack_entry_with(&mut game, &mut dm).expect("tainted pact should resolve");

        let hand_names: Vec<_> = game
            .player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| obj.name.clone()))
            .collect();
        assert!(
            hand_names.iter().any(|name| name == "Second Card"),
            "The second unique card should be put into hand"
        );

        let exile_names: Vec<_> = game
            .exile
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| obj.name.clone()))
            .collect();
        assert!(
            exile_names.iter().any(|name| name == "First Card"),
            "Declined cards should remain in exile"
        );
    }

    #[test]
    fn test_tainted_pact_stops_on_duplicate_name() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let spell_id = game.create_object_from_definition(&tainted_pact(), alice, Zone::Stack);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Duplicate Card")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Duplicate Card")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );

        game.stack.push(StackEntry::new(spell_id, alice));

        let mut dm = BoolSequenceDm::new(vec![false]);
        resolve_stack_entry_with(&mut game, &mut dm).expect("tainted pact should resolve");

        assert!(
            game.player(alice).expect("alice exists").hand.is_empty(),
            "No card should reach hand once a duplicate name stops the process"
        );

        let duplicate_count = game
            .exile
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Duplicate Card")
            .count();
        assert_eq!(
            duplicate_count, 2,
            "Both cards with the duplicate name should be exiled"
        );
    }
}
