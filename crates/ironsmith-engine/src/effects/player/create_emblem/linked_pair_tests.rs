use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::linked_exile::{LinkedExileAcquisition, LinkedExileOwner};
use crate::{CardId, CardType, ObjectFilter, PlayerId};

#[test]
fn emblem_alias_tracks_later_exiles_and_native_clones_without_crossing_pairs_or_acquisitions() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let alice = PlayerId::from_index(0);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Emblem pair fixture")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition([85; 32]),
        pair: 0,
    };
    let creator = LinkedExileOwner {
        host: source,
        pair,
        acquisition: LinkedExileAcquisition::Printed,
    };
    let mut ctx =
        ExecutionContext::new_default(source, alice).with_linked_exile_owner(Some(creator.clone()));
    let result = CreateEmblemEffect::new(EmblemDescription::new("Linked emblem", ""))
        .execute(&mut game, &mut ctx)
        .unwrap();
    let emblem = result.explicit_objects().unwrap()[0];
    let reader = LinkedExileOwner {
        host: emblem,
        ..creator.clone()
    };
    assert!(game.linked_exile_pair_members(&reader).unwrap().is_empty());
    game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    crate::effects::ExileEffect::all(ObjectFilter::artifact().in_zone(Zone::Graveyard))
        .execute(&mut game, &mut ctx)
        .unwrap();
    let member = game.linked_exile_pair_members(&creator).unwrap()[0];
    assert_eq!(game.linked_exile_pair_members(&reader).unwrap(), &[member]);
    let mut branch = game.clone();
    game.move_object_by_game_rule(member, Zone::Hand).unwrap();
    assert!(game.linked_exile_pair_members(&reader).unwrap().is_empty());
    assert_eq!(
        branch.linked_exile_pair_members(&reader).unwrap(),
        &[member]
    );
    let departed = branch.move_object_by_game_rule(source, Zone::Hand).unwrap();
    assert_ne!(departed, source);
    assert_eq!(
        branch.linked_exile_pair_members(&reader).unwrap(),
        &[member]
    );
    let new_owner = LinkedExileOwner {
        host: departed,
        ..creator.clone()
    };
    assert!(
        branch
            .linked_exile_pair_members(&new_owner)
            .unwrap()
            .is_empty()
    );
    let other_pair = LinkedExileOwner {
        pair: ironsmith_core::LinkedExilePair { pair: 1, ..pair },
        ..reader.clone()
    };
    assert!(
        branch
            .linked_exile_pair_members(&other_pair)
            .unwrap()
            .is_empty()
    );
    let gained = LinkedExileOwner {
        acquisition: LinkedExileAcquisition::EffectGroup(
            branch
                .effect_store
                .continuous_effects
                .next_effect_group_id(),
        ),
        ..reader.clone()
    };
    assert!(
        branch
            .linked_exile_pair_members(&gained)
            .unwrap()
            .is_empty()
    );
    let raw = branch
        .exiled_with_source_entries()
        .map(|(source, members)| (*source, members.clone()))
        .collect();
    branch.replace_exiled_with_source_links(raw);
    assert!(matches!(
        branch.linked_exile_pair_members(&reader),
        Err(ExecutionError::IncompleteEvidence(_))
    ));
    assert!(
        game.linked_exile_pair_members(&reader).unwrap().is_empty(),
        "source-only import affects only its native branch"
    );
}
