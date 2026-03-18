//! Thassa's Oracle card definition.

use crate::cards::CardDefinition;

/// Thassa's Oracle - {U}{U}
/// Creature — Merfolk Wizard
/// 1/3
/// Flying
/// When Thassa's Oracle enters the battlefield, look at the top X cards of your library,
/// where X is your devotion to blue. Put up to one of them on top of your library and the
/// rest on the bottom of your library in a random order. If X is greater than or equal to
/// the number of cards in your library, you win the game.
pub fn thassas_oracle() -> CardDefinition {
    crate::cards::handwritten_runtime::thassas_oracle()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_thassas_oracle_basic_properties() {
        let def = thassas_oracle();

        assert_eq!(def.name(), "Thassa's Oracle");
        assert!(def.is_creature());
        assert_eq!(def.card.mana_value(), 2);
        assert_eq!(def.abilities.len(), 2);
        assert!(def
            .abilities
            .iter()
            .any(|ability| matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability == &StaticAbility::flying()
            )));
        assert!(def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Triggered(triggered)
                if triggered.trigger.display().contains("enters")
        )));
    }

    struct ChooseNothingDm;

    impl DecisionMaker for ChooseNothingDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            Vec::new()
        }
    }

    #[test]
    fn test_thassas_oracle_wins_when_devotion_matches_library() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let oracle_def = thassas_oracle();
        let oracle_id = game.create_object_from_definition(&oracle_def, alice, Zone::Battlefield);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Library Card A")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Library Card B")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );

        let trigger = oracle_def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(&triggered.effects[0]),
                _ => None,
            })
            .expect("oracle should have an ETB trigger");

        let mut dm = ChooseNothingDm;
        let mut ctx = ExecutionContext::new(oracle_id, alice, &mut dm);
        let result = execute_effect(&mut game, trigger, &mut ctx);

        assert!(result.is_ok(), "oracle trigger should resolve");
        assert!(
            game.player(bob).expect("bob exists").has_lost,
            "Bob should lose when Oracle's devotion is at least the library size"
        );
    }

    #[test]
    fn test_thassas_oracle_reorders_top_cards_without_forcing_one_on_top() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let oracle_def = thassas_oracle();
        let oracle_id = game.create_object_from_definition(&oracle_def, alice, Zone::Battlefield);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Untouched Card")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Viewed Card A")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Viewed Card B")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );

        let trigger = oracle_def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(&triggered.effects[0]),
                _ => None,
            })
            .expect("oracle should have an ETB trigger");

        let mut dm = ChooseNothingDm;
        let mut ctx = ExecutionContext::new(oracle_id, alice, &mut dm);
        execute_effect(&mut game, trigger, &mut ctx).expect("oracle trigger should resolve");

        let library_names: Vec<_> = game
            .player(alice)
            .expect("alice exists")
            .library
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| obj.name.clone()))
            .collect();
        assert_eq!(
            library_names.last(),
            Some(&"Untouched Card".to_string()),
            "Choosing no card should not leave one of the viewed cards on top"
        );
        assert!(
            library_names.iter().any(|name| name == "Viewed Card A")
                && library_names.iter().any(|name| name == "Viewed Card B"),
            "Both viewed cards should still be present in the library"
        );
    }
}
