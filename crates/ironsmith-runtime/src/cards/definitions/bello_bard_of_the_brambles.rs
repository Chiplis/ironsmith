//! Card definition for Bello, Bard of the Brambles.

use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Bello, Bard of the Brambles {2}{R}{G}
/// Legendary Creature — Raccoon Bard
/// During your turn, each non-Equipment artifact and non-Aura enchantment you control
/// with mana value 4 or greater is a 4/4 Elemental creature in addition to its other
/// types and has indestructible, haste, and "Whenever this creature deals combat damage
/// to a player, draw a card."
/// 3/3
pub fn bello_bard_of_the_brambles() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Bello, Bard of the Brambles")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Green],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Raccoon, Subtype::Bard])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "During your turn, each non-Equipment artifact and non-Aura enchantment you control \
             with mana value 4 or greater is a 4/4 Elemental creature in addition to its other \
             types and has indestructible, haste, and \"Whenever this creature deals combat \
             damage to a player, draw a card.\"",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_bello_basic_properties() {
        let def = bello_bard_of_the_brambles();
        assert_eq!(def.card.name, "Bello, Bard of the Brambles");
        assert_eq!(def.card.mana_cost.as_ref().unwrap().mana_value(), 4);
        assert!(def.card.is_creature());
        assert_eq!(def.abilities.len(), 6);
        assert!(
            def.abilities
                .iter()
                .all(|ability| matches!(ability.kind, AbilityKind::Static(_))),
        );
    }

    #[test]
    fn test_bello_animates_non_equipment_artifacts_too() {
        use crate::cards::definitions::mycosynth_lattice;
        use crate::tests::test_helpers::setup_two_player_game;
        use crate::zone::Zone;

        let mut game = setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.create_object_from_definition(&bello_bard_of_the_brambles(), alice, Zone::Battlefield);
        let eye_id =
            game.create_object_from_definition(&mycosynth_lattice(), alice, Zone::Battlefield);
        game.turn.active_player = alice;

        let eye = game
            .calculated_characteristics(eye_id)
            .expect("calculated characteristics should exist");
        assert!(eye.card_types.contains(&CardType::Creature));
        assert!(eye.subtypes.contains(&Subtype::Elemental));
        assert_eq!(game.calculated_power(eye_id), Some(4));
        assert_eq!(game.calculated_toughness(eye_id), Some(4));

        let has_haste = eye
            .abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_haste()));
        assert!(
            has_haste,
            "Bello should grant haste to qualifying artifacts too"
        );
    }

    /// Humility first, then Bello.
    ///
    /// Bello applies in layers 4, 6, 7b. Humility applies in layers 6, 7b.
    /// In layer 6, Humility depends on Bello's ability-granting effect and is applied after it,
    /// so the granted abilities are removed. In layer 7b, timestamp order applies, so Bello's
    /// later timestamp sets power/toughness to 4/4.
    #[test]
    fn test_replay_bello_humility_humility_first_then_bello() {
        let mut game = run_replay_test(
            vec![""],
            ReplayTestConfig::new().p1_battlefield(vec!["Humility", "Bello, Bard of the Brambles"]),
        );

        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        let humility_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id)
                    .map(|o| o.name == "Humility" && o.controller == alice)
                    .unwrap_or(false)
            })
            .expect("Humility should be on battlefield");

        assert_eq!(game.calculated_power(humility_id), Some(4));
        assert_eq!(game.calculated_toughness(humility_id), Some(4));

        let abilities = game
            .calculated_characteristics(humility_id)
            .map(|c| c.abilities)
            .expect("calculated abilities should exist");
        let has_indestructible = abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_indestructible()));
        let has_haste = abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_haste()));
        let has_draw_trigger = abilities.iter().any(|a| {
            matches!(
                &a.kind,
                AbilityKind::Triggered(t)
                    if t.trigger.display().contains("deals combat damage to a player")
            )
        });
        assert!(
            !has_indestructible,
            "Humility should remove Bello-granted indestructible when it depends on Bello"
        );
        assert!(
            !has_haste,
            "Humility should remove Bello-granted haste when it depends on Bello"
        );
        assert!(
            !has_draw_trigger,
            "Humility should remove Bello-granted combat-damage draw trigger when it depends on Bello"
        );
    }

    /// Bello first, then Humility.
    ///
    /// Per Gatherer layering notes:
    /// - With Bello earlier and Humility later, Humility removes the abilities Bello granted
    ///   in layer 6 and overrides the 4/4 setting to 1/1 in layer 7b.
    #[test]
    fn test_replay_bello_humility_bello_first_then_humility() {
        let mut game = run_replay_test(
            vec![""],
            ReplayTestConfig::new().p1_battlefield(vec!["Bello, Bard of the Brambles", "Humility"]),
        );

        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        let humility_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id)
                    .map(|o| o.name == "Humility" && o.controller == alice)
                    .unwrap_or(false)
            })
            .expect("Humility should be on battlefield");

        assert_eq!(game.calculated_power(humility_id), Some(1));
        assert_eq!(game.calculated_toughness(humility_id), Some(1));

        let abilities = game
            .calculated_characteristics(humility_id)
            .map(|c| c.abilities)
            .expect("calculated abilities should exist");
        let has_indestructible = abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_indestructible()));
        let has_haste = abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_haste()));
        let has_draw_trigger = abilities.iter().any(|a| {
            matches!(
                &a.kind,
                AbilityKind::Triggered(t)
                    if t.trigger.display().contains("deals combat damage to a player")
            )
        });
        assert!(
            !has_indestructible,
            "Humility should remove Bello-granted indestructible when Humility has later timestamp"
        );
        assert!(
            !has_haste,
            "Humility should remove Bello-granted haste when Humility has later timestamp"
        );
        assert!(
            !has_draw_trigger,
            "Humility should remove Bello-granted trigger when Humility has later timestamp"
        );
    }
}
