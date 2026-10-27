use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::effects::{
    CreateEmblemEffect, EffectExecutor, ExecutionContext, ExileEffect, MoveToZoneEffect,
};
use crate::linked_exile::{LinkedExileAcquisition, LinkedExileOwner};
use crate::{Ability, CardId, CardType, ChooseSpec, Effect, ObjectFilter, StaticAbility};

#[test]
fn entry_transfers_only_new_pair_members_and_new_emblem_bindings() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let alice = PlayerId::from_index(0);
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition([86; 32]),
        pair: 0,
    };
    let create = CreateEmblemEffect::new(crate::effect::EmblemDescription::new(
        "Entry pair emblem",
        "",
    ));
    let program = crate::resolution::ResolutionProgram::from_effects(vec![
        Effect::new(ExileEffect::all(
            ObjectFilter::artifact().in_zone(Zone::Graveyard),
        )),
        Effect::new(create.clone()),
    ])
    .with_linked_exile_pair(pair);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Paired entry source")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::from_model(
            crate::static_abilities::CompiledStaticAbility::as_enters_effect_program(
                program,
                "this artifact",
                false,
                false,
                None,
            ),
        )))
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let stable = game.object(source).unwrap().stable_id;
    let owner = LinkedExileOwner {
        host: source,
        pair,
        acquisition: LinkedExileAcquisition::Printed,
    };
    let resource = CardDefinitionBuilder::new(CardId::new(), "Linked entry resource")
        .card_types(vec![CardType::Artifact])
        .build();
    // Initial native history: this pre-entry incarnation already has a link.
    let prior = game.create_object_from_definition(&resource, alice, Zone::Exile);
    game.add_linked_exile_pair_member(owner.clone(), prior);
    let new_resource = game.create_object_from_definition(&resource, alice, Zone::Graveyard);
    let new_stable = game.object(new_resource).unwrap().stable_id;
    let old_emblem = create
        .execute(
            &mut game,
            &mut ExecutionContext::new_default(source, alice)
                .with_linked_exile_owner(Some(owner.clone())),
        )
        .unwrap()
        .explicit_objects()
        .unwrap()[0];
    let instruction = game.create_object_from_definition(&resource, alice, Zone::Battlefield);
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(source), Zone::Battlefield, false)
        .execute(
            &mut game,
            &mut ExecutionContext::new_default(instruction, alice),
        )
        .unwrap();
    let entrant = game.find_object_by_stable_id(stable).unwrap();
    let member = game.find_object_by_stable_id(new_stable).unwrap();
    assert_ne!(entrant, source);
    assert_eq!(game.object(member).unwrap().zone, Zone::Exile);
    let entered_owner = LinkedExileOwner {
        host: entrant,
        ..owner.clone()
    };
    assert_eq!(
        game.linked_exile_pair_members(&entered_owner).unwrap(),
        &[member]
    );
    assert_eq!(game.linked_exile_pair_members(&owner).unwrap(), &[prior]);
    let old_reader = LinkedExileOwner {
        host: old_emblem,
        ..owner.clone()
    };
    assert_eq!(
        game.linked_exile_pair_members(&old_reader).unwrap(),
        &[prior]
    );
    let emblems: Vec<_> = game
        .command_zone
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.kind == crate::object::ObjectKind::Emblem)
        })
        .collect();
    assert_eq!(emblems.len(), 2);
    let new_emblem = *emblems.iter().find(|id| **id != old_emblem).unwrap();
    let new_reader = LinkedExileOwner {
        host: new_emblem,
        ..owner
    };
    assert_eq!(
        game.linked_exile_pair_members(&new_reader).unwrap(),
        &[member]
    );
    game.validate_zone_consistency().unwrap();
}
