use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::effects::{ExecutionContext, ResolvedTarget};
use crate::{Ability, AbilityKind, CardId, ChooseSpec, Effect, ObjectFilter, TotalCost};

fn equip(target: ChooseSpec) -> Ability {
    let mut ability = Ability::activated(
        TotalCost::free(),
        vec![Effect::attach_objects(ChooseSpec::Source, target)],
    );
    let AbilityKind::Activated(ref mut activated) = ability.kind else {
        unreachable!()
    };
    activated.keyword = Some(ironsmith_core::ActivatedAbilityKeyword::Equip);
    ability
}
fn fixture() -> (GameState, ObjectId, ObjectId, Effect) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let alice = crate::PlayerId::from_index(0);
    let planes = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::planeswalker().you_control(),
    ));
    let equipment =
        CardDefinitionBuilder::new(CardId::new(), "Equipment with distinct equip variants")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .with_ability(equip(planes.clone()))
            .with_ability(equip(ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::creature().you_control(),
            ))))
            .build();
    let source = game.create_object_from_definition(&equipment, alice, Zone::Battlefield);
    let host = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Noncreature planeswalker host")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(3)
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.object_mut(host).unwrap().counters.insert(crate::CounterType::Loyalty, 3);
    (
        game,
        source,
        host,
        Effect::attach_objects(ChooseSpec::Source, planes),
    )
}

#[test]
fn equip_planeswalker_permission_does_not_authorize_an_unrelated_attach_instruction() {
    let (mut game, source, host, _) = fixture();
    assert!(!attachment_can_attach_to_target(
        &game,
        source,
        AttachmentTarget::Object(host)
    ));
    assert!(!attach_battlefield_object_to_target(
        &mut game,
        source,
        AttachmentTarget::Object(host)
    ));
    let effect = Effect::attach_objects(
        ChooseSpec::SpecificObject(source),
        ChooseSpec::SpecificObject(host),
    );
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, crate::PlayerId::from_index(0), &mut dm);
    crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert!(game.object(source).unwrap().attached_to.is_none());
}

#[test]
fn captured_equip_variant_permits_the_instruction_after_ability_loss_but_not_ongoing_attachment() {
    for selected in [0, 1] {
        let (mut game, source, host, effect) = fixture();
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).unwrap(),
            &game,
        );
        let origin = snapshot.ability_origins.as_ref().unwrap()[selected].clone();
        // The selected ability retains its own acquisition even after the
        // source loses its rules text while that ability is on the stack.
        game.object_mut(source).unwrap().abilities_mut().clear();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, crate::PlayerId::from_index(0), &mut dm)
            .with_source_snapshot(snapshot)
            .with_targets(vec![ResolvedTarget::Object(host)]);
        ctx.ability_index = Some(selected);
        ctx.activation_origin = Some(origin);
        crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(
            game.object(source).unwrap().attached_to,
            if selected == 0 {
                Some(AttachmentTarget::Object(host))
            } else {
                None
            },
            "the selected equip variant, not another ability on the source, owns the exception"
        );
        assert!(!attachment_can_attach_to_target(
            &game,
            source,
            AttachmentTarget::Object(host)
        ));
        let mut queue = crate::triggers::TriggerQueue::new();
        crate::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert!(game.object(source).unwrap().attached_to.is_none());
        assert_eq!(game.object(host).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.counter_count(host, crate::CounterType::Loyalty), 3);
        game.validate_zone_consistency().unwrap();
    }
}
