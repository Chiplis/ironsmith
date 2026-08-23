//! Party-size rules support (CR 700.8-700.8c).

use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::types::{CardType, Subtype};

const PARTY_ROLES: [Subtype; 4] = [
    Subtype::Cleric,
    Subtype::Rogue,
    Subtype::Warrior,
    Subtype::Wizard,
];

/// Return the largest legal party the player can form from creatures they control.
///
/// A creature can fill at most one role, even when it has multiple party creature
/// types. With only four roles, a bitmask dynamic program is both exact and tiny.
pub(crate) fn party_size(game: &GameState, player_id: PlayerId) -> i32 {
    // Bit `assignment` is set when that subset of the four roles is reachable.
    let mut reachable_assignments = 1u16;

    for object_id in game.battlefield.iter().copied() {
        if game.current_controller(object_id) != Some(player_id)
            || !game.current_has_card_type(object_id, CardType::Creature)
        {
            continue;
        }

        let mut creature_roles = 0u8;
        for (index, role) in PARTY_ROLES.iter().copied().enumerate() {
            if game.current_has_subtype(object_id, role) {
                creature_roles |= 1 << index;
            }
        }

        let previous_assignments = reachable_assignments;
        for assignment in 0u8..16 {
            if previous_assignments & (1u16 << assignment) == 0 {
                continue;
            }
            let available_roles = creature_roles & !assignment;
            for role_index in 0..4 {
                let role = 1u8 << role_index;
                if available_roles & role != 0 {
                    reachable_assignments |= 1u16 << (assignment | role);
                }
            }
        }
    }

    (0u8..16)
        .filter(|assignment| reachable_assignments & (1u16 << assignment) != 0)
        .map(|assignment| assignment.count_ones() as i32)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::effect::Until;
    use crate::ids::CardId;
    use crate::zone::Zone;

    fn add_creature(
        game: &mut GameState,
        player: PlayerId,
        card_id: u32,
        subtypes: Vec<Subtype>,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), format!("Party Member {card_id}"))
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&card, player, Zone::Battlefield)
    }

    #[test]
    fn multitype_creature_can_fill_only_one_party_role() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        add_creature(
            &mut game,
            alice,
            1,
            vec![
                Subtype::Cleric,
                Subtype::Rogue,
                Subtype::Warrior,
                Subtype::Wizard,
            ],
        );

        assert_eq!(party_size(&game, alice), 1);
    }

    #[test]
    fn party_assignment_maximizes_roles_instead_of_greedily_claiming_them() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        add_creature(&mut game, alice, 2, vec![Subtype::Cleric, Subtype::Rogue]);
        add_creature(&mut game, alice, 3, vec![Subtype::Cleric]);
        add_creature(&mut game, alice, 4, vec![Subtype::Warrior]);
        add_creature(&mut game, alice, 5, vec![Subtype::Wizard]);

        assert_eq!(party_size(&game, alice), 4);
    }

    #[test]
    fn party_size_uses_current_layered_creature_types() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let cleric = add_creature(&mut game, alice, 6, vec![Subtype::Cleric]);
        let source = add_creature(&mut game, alice, 7, vec![]);

        assert_eq!(party_size(&game, alice), 1);

        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                source,
                alice,
                EffectTarget::Specific(cleric),
                Modification::AddSubtypes(vec![Subtype::Rogue]),
            )
            .until(Until::EndOfTurn),
        );

        // The Cleric gained another eligible role, but it is still only one
        // creature and therefore still fills only one party slot.
        assert_eq!(party_size(&game, alice), 1);

        add_creature(&mut game, alice, 8, vec![Subtype::Cleric]);
        assert_eq!(party_size(&game, alice), 2);
    }
}
