//! Card definition for Maskwood Nexus.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Maskwood Nexus {4}
/// Artifact
/// Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.
/// {3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling. (It is every creature type.)
pub fn maskwood_nexus() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Maskwood Nexus")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\n\
{3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling. (It is every creature type.)",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::cards::definitions::{grizzly_bears, lightning_bolt};
    use crate::compiled_text::compiled_lines;
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbilityId;
    use crate::types::Subtype;
    use crate::zone::Zone;

    #[test]
    fn test_maskwood_nexus_has_generic_subtype_static_effects() {
        let def = maskwood_nexus();
        let static_ids: Vec<_> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.id()),
                _ => None,
            })
            .collect();

        assert_eq!(
            static_ids
                .iter()
                .filter(|id| **id == StaticAbilityId::AddAllSubtypesOfFamily)
                .count(),
            3,
            "expected battlefield, stack, and one disjunctive off-battlefield zone effect"
        );
        assert!(
            def.abilities.iter().any(|ability| {
                matches!(&ability.kind, AbilityKind::Activated(_) if ability.text.as_deref().is_some_and(|text| text.contains("Create a 2/2 blue Shapeshifter creature token")))
            }),
            "expected activated token ability"
        );

        let compiled = compiled_lines(&def).join(" | ");
        assert!(
            compiled.contains("every creature type"),
            "compiled text should mention the generic subtype effect: {compiled}"
        );
    }

    #[test]
    fn test_maskwood_nexus_updates_creature_types_across_zones() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        game.create_object_from_definition(&maskwood_nexus(), alice, Zone::Battlefield);
        let battlefield_creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let hand_creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Hand);
        let library_creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Library);
        let graveyard_creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Graveyard);
        let exile_creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Exile);
        let command_creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Command);
        let stack_creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Stack);
        game.stack.push(StackEntry::new(stack_creature_id, alice));

        let noncreature_graveyard_id =
            game.create_object_from_definition(&lightning_bolt(), alice, Zone::Graveyard);

        for id in [
            battlefield_creature_id,
            hand_creature_id,
            library_creature_id,
            graveyard_creature_id,
            exile_creature_id,
            command_creature_id,
            stack_creature_id,
        ] {
            assert!(
                game.current_has_subtype(id, Subtype::Wizard),
                "expected {id:?} to gain Wizard under Maskwood Nexus"
            );
            assert!(
                game.current_has_subtype(id, Subtype::Elf),
                "expected {id:?} to gain Elf under Maskwood Nexus"
            );
        }

        assert!(
            !game.current_has_subtype(noncreature_graveyard_id, Subtype::Wizard),
            "noncreature cards should not gain creature types"
        );
    }
}
