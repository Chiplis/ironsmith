//! Command the Mind card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;

/// Command the Mind - {U}
/// Sorcery
/// Gain control of target opponent until end of turn.
///
/// NOTE: This is a small demo card used to exercise player-control UI flows.
pub fn command_the_mind() -> CardDefinition {
    let text = "Mana cost: {U}\n\
Type: Sorcery\n\
Gain control of target opponent until end of turn.";

    CardDefinitionBuilder::new(CardId::new(), "Command the Mind")
        .parse_text(text)
        .expect("Command the Mind text should be supported")
}

#[cfg(test)]
mod tests {
    use crate::cards::CardRegistry;
    use crate::decision::{DecisionRouter, NumericInputDecisionMaker};
    use crate::game_loop::run_priority_loop_with;
    use crate::game_state::GameState;
    use crate::game_state::Phase;
    use crate::ids::PlayerId;
    use crate::triggers::TriggerQueue;
    use crate::zone::Zone;

    /// Replay: cast Command the Mind, then cast Tivit so the controlled player votes.
    /// Alice's inputs should be used for Bob's vote while control is active.
    #[test]
    fn test_replay_control_player_during_own_turn() {
        let registry = CardRegistry::with_builtin_cards_for_names([
            "Command the Mind",
            "Tivit, Seller of Secrets",
            "Island",
            "Plains",
            "Swamp",
        ]);
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let find = |name: &str| registry.get(name).cloned().expect("Card not found");

        game.create_object_from_definition(&find("Command the Mind"), alice, Zone::Hand);
        game.create_object_from_definition(&find("Tivit, Seller of Secrets"), alice, Zone::Hand);

        for land in [
            "Island", "Plains", "Swamp", "Plains", "Island", "Swamp", "Plains",
        ] {
            game.create_object_from_definition(&find(land), alice, Zone::Battlefield);
        }

        // Start in main phase with Alice having priority.
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let inputs = vec![
            "1", "0", "0", "", "1", "0", "0", "0", "0", "1", "0", "1", "1", "",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let alice_dm = NumericInputDecisionMaker::new(inputs);
        // If Bob were not controlled, he'd vote "evidence" (index 0).
        let bob_dm = NumericInputDecisionMaker::from_strs(&["0"]);
        let mut dm = DecisionRouter::new(Box::new(alice_dm)).with_player(bob, Box::new(bob_dm));
        let mut trigger_queue = TriggerQueue::new();

        run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
            .expect("priority loop should complete");

        let clue_count = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Clue" && o.controller == alice)
                    .unwrap_or(false)
            })
            .count();
        let treasure_count = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Treasure" && o.controller == alice)
                    .unwrap_or(false)
            })
            .count();

        assert_eq!(
            clue_count, 1,
            "Alice should have 1 Clue from 1 evidence vote"
        );
        assert_eq!(
            treasure_count, 2,
            "Alice should have 2 Treasures from 2 bribery votes"
        );
    }
}
