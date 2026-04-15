//! Tainted Pact card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Tainted Pact - {1}{B}
/// Instant
/// Exile the top card of your library. You may put that card into your hand
/// unless it has the same name as another card exiled this way. Repeat this
/// process until you put a card into your hand or you exile two cards with the
/// same name, whichever comes first.
pub fn tainted_pact() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Tainted Pact")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::game_loop::resolve_stack_entry_with;
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::{CardId, ObjectId, PlayerId};
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

    #[derive(Default)]
    struct PromptOnlyDecisionMaker {
        prompted: bool,
    }

    impl DecisionMaker for PromptOnlyDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.prompted
        }

        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.prompted = true;
            false
        }
    }

    #[test]
    fn test_tainted_pact_basic_properties() {
        let def = tainted_pact();

        assert_eq!(def.name(), "Tainted Pact");
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 2);
        assert_eq!(
            def.spell_effect
                .as_ref()
                .expect("spell effect exists")
                .len(),
            1
        );
    }

    #[test]
    fn test_tainted_pact_repeat_process_keeps_progress_across_prompt_resume() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
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

        let effect = tainted_pact()
            .spell_effect
            .as_ref()
            .expect("spell effect exists")
            .segments[0]
            .default_effects[0]
            .clone();
        let source = ObjectId::from_raw(999);

        let ctx = ExecutionContext::new_default(source, alice);
        let mut prompt_dm = PromptOnlyDecisionMaker::default();
        let mut ctx = ctx.with_decision_maker(&mut prompt_dm);

        let first =
            execute_effect(&mut game, &effect, &mut ctx).expect("first prompt should execute");
        assert_eq!(first.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(
            game.player(alice).expect("alice exists").library.len(),
            1,
            "the first card should already be exiled before the prompt resolves"
        );

        let mut decline_dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut decline_dm);
        let second = execute_effect(&mut game, &effect, &mut ctx).expect("resume should execute");
        assert_eq!(second.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(
            game.player(alice).expect("alice exists").library.len(),
            0,
            "resuming after declining should continue exiling through the library"
        );
        assert_eq!(
            game.exile
                .iter()
                .filter(|id| game
                    .object(**id)
                    .is_some_and(|obj| { obj.name == "First Card" || obj.name == "Second Card" }))
                .count(),
            2,
            "both distinct cards should be exiled this way"
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
