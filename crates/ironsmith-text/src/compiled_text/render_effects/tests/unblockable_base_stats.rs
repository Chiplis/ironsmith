use super::*;
const TEXT: &str = "Equipped creature gets +1/+0.\nWhenever equipped creature attacks, choose up to one target creature. That creature can't be blocked this turn and has base power and toughness 1/1 until end of turn.\nEquip {2}";
#[test]
fn unblockable_base_stats_preserves_both_effects_on_the_chosen_creature() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Microsizer fixture")
        .card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Equipment]).parse_text(TEXT).unwrap();
    let trigger = definition.abilities.iter().find_map(|a| match &a.kind {
        AbilityKind::Triggered(t) => Some(t), _ => None,
    }).unwrap();
    for choose_target in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(4, 5)).build();
        let selected = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let other = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let _blocker = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let targets = if choose_target { vec![crate::effects::ResolvedTarget::Object(selected)] } else { vec![] };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_targets(targets);
        ctx.snapshot_targets(&game);
        for effect in &trigger.effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|error| panic!("chosen={choose_target}: {error:?}; {effect:#?}")); }
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(selected), Some(if choose_target { 1 } else { 4 }));
        assert_eq!(game.calculated_toughness(selected), Some(if choose_target { 1 } else { 5 }));
        assert_eq!(game.can_be_blocked(selected), !choose_target);
        assert_eq!(game.calculated_power(other), Some(4));
        assert!(game.can_be_blocked(other));
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.cleanup_restrictions_end_of_turn();
        game.refresh_continuous_state();
        game.update_cant_effects();
        assert_eq!(game.calculated_power(selected), Some(4));
        assert_eq!(game.calculated_toughness(selected), Some(5));
        assert!(game.can_be_blocked(selected));
    }
}
#[test]
fn unblockable_base_stats_renders_both_coordinated_predicates() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Microsizer fixture")
        .card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Equipment]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), TEXT);
}

#[test]
fn unblockable_base_stats_compaction_requires_the_same_referenced_object() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Microsizer fixture")
        .card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Equipment]).parse_text(TEXT).unwrap();
    let trigger = definition.abilities.iter().find_map(|a| match &a.kind {
        AbilityKind::Triggered(t) => Some(t), _ => None,
    }).unwrap();
    fn find_pair(effect: &Effect) -> Option<crate::effects::SequenceEffect> {
        let effect = super::super::structural_unwrap_render_wrappers(effect);
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
            && sequence.effects.len() == 2
            && super::super::structural_unwrap_render_wrappers(&sequence.effects[0]).downcast_ref::<crate::effects::CantEffect>().is_some()
        { return Some(sequence.clone()); }
        let mut found = None;
        effect.visit_child_effects(&mut |child| { if found.is_none() { found = find_pair(child); } });
        found
    }
    let original = trigger.effects.iter().find_map(find_pair).unwrap();
    assert_eq!(describe_coordinated_sequence(&original).unwrap(),
        "That creature can't be blocked this turn and has base power and toughness 1/1 until end of turn");
    let mut different_object = original.clone();
    let mut modification = super::super::structural_unwrap_render_wrappers(&different_object.effects[1]).downcast_ref::<crate::effects::ApplyContinuousEffect>().unwrap().clone();
    modification.target_spec = Some(ChooseSpec::Tagged(crate::TagKey::from("different_object")));
    different_object.effects[1] = Effect::new(modification);
    assert_ne!(describe_coordinated_sequence(&different_object), describe_coordinated_sequence(&original));
}
