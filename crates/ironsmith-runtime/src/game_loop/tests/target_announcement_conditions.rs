use super::*;

fn push_and_record_spell(game: &mut GameState, caster: PlayerId, name: &str) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build();
    let spell = game.create_object_from_definition(&definition, caster, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, caster));
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
    spell
}

#[test]
fn target_cast_order_condition_filters_announcement_candidates() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first = push_and_record_spell(&mut game, alice, "First Stack Spell");
    let second = push_and_record_spell(&mut game, bob, "Second Stack Spell");
    let third = push_and_record_spell(&mut game, alice, "Third Stack Spell");
    let source_definition = CardDefinitionBuilder::new(CardId::new(), "Ordinal Counter")
        .card_types(vec![CardType::Instant])
        .build();
    let source = game.create_object_from_definition(&source_definition, alice, Zone::Stack);
    let effects = vec![Effect::conditional_only(
        crate::effect::Condition::TargetSpellCastOrderThisTurn(2),
        vec![Effect::counter(ChooseSpec::target_spell())],
    )];

    let requirements = extract_target_requirements(&game, &effects, alice, Some(source));
    assert_eq!(requirements.len(), 1);
    assert_eq!(
        requirements[0].legal_targets,
        vec![Target::Object(second)],
        "the relative cast-order clause is a target restriction, not a resolution-only no-op"
    );
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&Target::Object(first))
    );
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&Target::Object(third))
    );

    game.stack.retain(|entry| entry.object_id != second);
    game.move_object_by_effect(second, Zone::Graveyard)
        .expect("the second spell should leave the stack");
    assert!(
        !spell_has_legal_targets(&game, &effects, alice, Some(source)),
        "the guarded counter must have no legal target after the second spell leaves the stack"
    );
}
