//! Nemesis Mask card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Nemesis Mask - {3}
/// Artifact — Equipment
/// All creatures able to block equipped creature do so.
/// Equip {3}
pub fn nemesis_mask() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Nemesis Mask")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text("All creatures able to block equipped creature do so.\nEquip {3}")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{AbilityKind, ActivationTiming};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::{AttachmentTarget, Object};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbilityId;
    use crate::target::ChooseSpec;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_equipment(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = nemesis_mask();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    fn attach_equipment(game: &mut GameState, equipment_id: ObjectId, creature_id: ObjectId) {
        if let Some(equipment) = game.object_mut(equipment_id) {
            equipment.attached_to = Some(AttachmentTarget::Object(creature_id));
        }
        if let Some(creature) = game.object_mut(creature_id) {
            creature.attachments.push(equipment_id);
        }
    }

    fn detach_equipment(game: &mut GameState, equipment_id: ObjectId, creature_id: ObjectId) {
        if let Some(equipment) = game.object_mut(equipment_id) {
            equipment.attached_to = None;
        }
        if let Some(creature) = game.object_mut(creature_id) {
            creature.attachments.retain(|attached| *attached != equipment_id);
        }
    }

    #[test]
    fn test_nemesis_mask_basic_properties() {
        let def = nemesis_mask();
        assert_eq!(def.name(), "Nemesis Mask");
        assert!(def.card.is_artifact());
        assert!(!def.card.is_creature());
        assert!(def.card.subtypes.contains(&Subtype::Equipment));
        assert_eq!(def.card.mana_value(), 3);
        assert!(def.spell_effect.is_none(), "Nemesis Mask should be a static equipment");
        assert_eq!(def.abilities.len(), 2, "Nemesis Mask should have equip plus one static line");
    }

    #[test]
    fn test_nemesis_mask_has_equip_and_must_block_grant() {
        let def = nemesis_mask();

        let mut saw_equipment_grant = false;
        let mut saw_equip = false;
        for ability in &def.abilities {
            match &ability.kind {
                AbilityKind::Static(static_ability) => {
                    if static_ability.id() == StaticAbilityId::AttachedAbilityGrant {
                        saw_equipment_grant = true;
                        assert!(
                            static_ability
                                .display()
                                .contains("All creatures able to block equipped creature do so")
                        );
                    }
                }
                AbilityKind::Activated(activated) => {
                    saw_equip = true;
                    assert_eq!(activated.timing, ActivationTiming::SorcerySpeed);
                    assert!(activated.mana_cost.mana_cost().is_some());
                    assert_eq!(activated.mana_cost.mana_cost().unwrap().mana_value(), 3);
                    assert_eq!(activated.choices.len(), 1);
                    let target_spec = match &activated.choices[0] {
                        ChooseSpec::Target(inner) => inner.as_ref(),
                        other => other,
                    };
                    if let ChooseSpec::Object(filter) = target_spec {
                        assert!(filter.card_types.contains(&CardType::Creature));
                    } else {
                        panic!("Equip should target a creature");
                    }
                }
                _ => {}
            }
        }

        assert!(saw_equipment_grant, "expected attached must-block grant");
        assert!(saw_equip, "expected equip activated ability");
    }

    #[test]
    fn test_nemesis_mask_requires_each_legal_blocker_while_attached() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = create_creature(&mut game, "Attacker", alice);
        let blocker_one = create_creature(&mut game, "Blocker One", bob);
        let blocker_two = create_creature(&mut game, "Blocker Two", bob);
        let equipment = create_equipment(&mut game, alice);

        game.update_cant_effects();
        assert!(
            !game.must_block_attacker(blocker_one, attacker)
                && !game.must_block_attacker(blocker_two, attacker),
            "unattached equipment should not impose any block requirement"
        );

        attach_equipment(&mut game, equipment, attacker);
        game.update_cant_effects();
        assert!(
            game.must_block_attacker(blocker_one, attacker)
                && game.must_block_attacker(blocker_two, attacker),
            "equipped creature should be required blocking by every legal blocker"
        );

        detach_equipment(&mut game, equipment, attacker);
        game.update_cant_effects();
        assert!(
            !game.must_block_attacker(blocker_one, attacker)
                && !game.must_block_attacker(blocker_two, attacker),
            "the block requirement should end once Nemesis Mask is no longer attached"
        );
    }
}
