//! Savine's Reclamation card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Savine's Reclamation - {4}{W}
/// Sorcery
/// Return target permanent card with mana value 3 or less from your graveyard to the battlefield.
/// If this spell was cast from a graveyard, copy this spell and you may choose a new target for the copy.
/// Flashback {5}{W}
pub fn savines_reclamation() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Savine's Reclamation")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card with mana value 3 or less from your graveyard to the battlefield.\nIf this spell was cast from a graveyard, copy this spell and you may choose a new target for the copy.\nFlashback {5}{W}",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alternative_cast::{AlternativeCastingMethod, CastingMethod};
    use crate::cards::definitions::{grizzly_bears, ornithopter, savannah_lions};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::game_loop::resolve_stack_entry_with;
    use crate::game_state::{GameState, StackEntry, Target};
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    #[test]
    fn test_savines_reclamation_basic_properties() {
        let def = savines_reclamation();

        assert_eq!(def.name(), "Savine's Reclamation");
        assert!(def.card.is_sorcery());
        assert_eq!(def.card.mana_value(), 5);
        assert_eq!(def.alternative_casts.len(), 1);
        assert!(matches!(
            def.alternative_casts.first(),
            Some(AlternativeCastingMethod::Flashback { .. })
        ));
        assert_eq!(
            def.spell_effect
                .as_ref()
                .expect("spell effect exists")
                .len(),
            2
        );
    }

    #[test]
    fn test_savines_reclamation_returns_one_permanent_normally() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let target_id =
            game.create_object_from_definition(&savannah_lions(), alice, Zone::Graveyard);
        let spell_id =
            game.create_object_from_definition(&savines_reclamation(), alice, Zone::Stack);
        game.stack
            .push(StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(target_id)]));

        let mut dm = SelectFirstDecisionMaker;
        resolve_stack_entry_with(&mut game, &mut dm).expect("savine should resolve");

        assert!(
            game.battlefield.iter().any(|&id| {
                game.object(id)
                    .map(|obj| obj.name == "Savannah Lions")
                    .unwrap_or(false)
            }),
            "The targeted permanent should return to the battlefield"
        );

        let in_graveyard = game
            .player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .any(|&id| {
                game.object(id)
                    .map(|obj| obj.name == "Savannah Lions")
                    .unwrap_or(false)
            });
        assert!(!in_graveyard, "The target should leave the graveyard");
    }

    #[test]
    fn test_savines_reclamation_flashback_copies_and_exiles_spell() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let first_target =
            game.create_object_from_definition(&ornithopter(), alice, Zone::Graveyard);
        let second_target =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Graveyard);
        let spell_id =
            game.create_object_from_definition(&savines_reclamation(), alice, Zone::Stack);
        game.stack.push(
            StackEntry::new(spell_id, alice)
                .with_targets(vec![Target::Object(first_target)])
                .with_casting_method(CastingMethod::Alternative(0)),
        );

        let mut dm = SelectFirstDecisionMaker;
        resolve_stack_entry_with(&mut game, &mut dm).expect("flashback savine should resolve");

        for expected in ["Ornithopter", "Grizzly Bears"] {
            assert!(
                game.battlefield.iter().any(|&id| {
                    game.object(id)
                        .map(|obj| obj.name == expected)
                        .unwrap_or(false)
                }),
                "{expected} should be on the battlefield after Savine resolves"
            );
        }

        let spell_in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Savine's Reclamation")
                .unwrap_or(false)
        });
        assert!(
            spell_in_exile,
            "Flashback should exile Savine's Reclamation after it resolves"
        );

        assert!(
            game.player(alice)
                .expect("alice exists")
                .graveyard
                .iter()
                .all(|&id| {
                    game.object(id)
                        .map(|obj| obj.name != "Ornithopter" && obj.name != "Grizzly Bears")
                        .unwrap_or(true)
                }),
            "Both returned permanents should have left the graveyard"
        );
        let _ = second_target;
    }
}
