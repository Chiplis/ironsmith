use super::*;

#[test]
fn cohort_targeted_tokens_iterate_only_announced_targets_with_dynamic_limit() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Aura Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("For each of up to X target creatures, create a red Aura enchantment token named Smoke Blessing attached to that creature. Those tokens have enchant creature and \"When enchanted creature dies, it deals 1 damage to its controller and you create a Treasure token.\"").unwrap();
    let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Creature Probe")
        .card_types(vec![CardType::Creature])
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Stack);
    let targets = [alice, bob]
        .map(|owner| game.create_object_from_definition(&creature, owner, Zone::Battlefield));
    game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let program = card.spell_effect.as_ref().unwrap();
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        program,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1, "{requirements:#?}");
    assert!(requirements[0].spec.is_target());
    assert_eq!(requirements[0].spec.count().min, 0);
    assert!(requirements[0].spec.count().is_up_to_dynamic_x());
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(targets.map(crate::effects::ResolvedTarget::Object).to_vec());
    ctx.snapshot_targets(&game);
    for effect in program.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let auras = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| object.name == "Smoke Blessing")
        .collect::<Vec<_>>();
    assert_eq!(auras.len(), 2);
    for target in targets {
        assert!(auras.iter().any(|aura| aura.attached_to == Some(crate::object::AttachmentTarget::Object(target))));
    }
}
