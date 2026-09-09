use super::*;

fn all_creatures_controlled_by(player: PlayerFilter) -> ChooseSpec {
    let mut creatures = ObjectFilter::creature();
    creatures.zone = Some(Zone::Battlefield);
    creatures.controller = Some(player);
    ChooseSpec::All(creatures)
}

#[test]
fn opponent_control_and_not_you_remain_distinct_in_goad_surfaces() {
    let opponents = Effect::goad(all_creatures_controlled_by(PlayerFilter::Opponent));
    assert_eq!(
        describe_effect(&opponents),
        "Goad all creatures your opponents control"
    );

    let not_you = Effect::goad(all_creatures_controlled_by(PlayerFilter::NotYou));
    assert_eq!(
        describe_effect(&not_you),
        "Goad all creatures you don't control"
    );
}

#[test]
fn linked_all_goaded_set_keeps_its_plural_restriction_back_reference() {
    let goad = Effect::goad(all_creatures_controlled_by(PlayerFilter::Opponent)).tag("goaded_0");
    let mut goaded_set = ObjectFilter::default();
    goaded_set
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from("goaded_0"),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    goaded_set.set_plural_object_noun_surface(true);
    let cant_block = Effect::cant_until(
        crate::effect::Restriction::Block(goaded_set),
        Until::YourNextTurn,
    );
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![goad]),
        crate::resolution::ResolutionSegment::from_effects(vec![cant_block]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Goad all creatures your opponents control. Until your next turn, those creatures can't block"
    );
}

#[test]
fn linked_goaded_set_rejects_a_different_result_tag() {
    let goad = Effect::goad(all_creatures_controlled_by(PlayerFilter::Opponent)).tag("goaded_0");
    let mut different_set = ObjectFilter::default();
    different_set
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from("other_set"),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    let cant_block = Effect::cant_until(
        crate::effect::Restriction::Block(different_set),
        Until::YourNextTurn,
    );
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![goad]),
        crate::resolution::ResolutionSegment::from_effects(vec![cant_block]),
    ]);

    assert_ne!(
        super::super::ast_render::describe_resolution_program(&program),
        "Goad all creatures your opponents control. Until your next turn, those creatures can't block"
    );
}

#[test]
fn investigate_counts_goaded_creatures_once_regardless_of_goaders() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goad count fixture")
        .card_types(vec![CardType::Instant]).parse_text("Investigate for each goaded creature you control.").unwrap();
    for goaded in 0..=3 {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id; let carol = game.players[2].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature fixture").card_types(vec![CardType::Creature]).build();
        for i in 0..3 {
            let id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
            if i < goaded {
                game.add_goad_effect(id, bob, Until::YourNextTurn, source);
                game.add_goad_effect(id, carol, Until::YourNextTurn, source);
            }
        }
        let other = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        game.add_goad_effect(other, alice, Until::YourNextTurn, source);
        game.refresh_continuous_state();
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        let clues = game.battlefield.iter().filter(|id| game.current_has_subtype(**id, Subtype::Clue)).count();
        assert_eq!(clues, goaded);
    }
}

#[test]
fn investigate_then_clear_goad_preserves_order_and_controller_scope() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goad clearing fixture")
        .card_types(vec![CardType::Instant])
        .parse_text("Investigate for each goaded creature you control. Then each creature you control is no longer goaded.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = game.players[0].id; let bob = game.players[1].id; let carol = game.players[2].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature fixture").card_types(vec![CardType::Creature]).build();
    let own = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let other = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    game.add_goad_effect(own, bob, Until::YourNextTurn, source);
    game.add_goad_effect(own, carol, Until::YourNextTurn, source);
    game.add_goad_effect(other, alice, Until::YourNextTurn, source);
    game.refresh_continuous_state();
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for segment in &definition.spell_effect.as_ref().unwrap().segments {
        for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    }
    assert_eq!(game.battlefield.iter().filter(|id| game.current_has_subtype(**id, Subtype::Clue)).count(), 1);
    assert!(!game.is_goaded(own), "{:#?}", definition.spell_effect);
    assert!(game.is_goaded(other));
    game.add_goad_effect(own, bob, Until::YourNextTurn, source);
    assert!(game.is_goaded(own));
}

#[test]
fn serene_sleuth_combat_ability_investigates_before_clearing_goad() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Serene Sleuth")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, investigate.\nAt the beginning of combat on your turn, investigate for each goaded creature you control. Then each creature you control is no longer goaded.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = game.players[0].id; let bob = game.players[1].id; let carol = game.players[2].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature fixture").card_types(vec![CardType::Creature]).build();
    let own = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let other = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    game.add_goad_effect(own, bob, Until::YourNextTurn, source);
    game.add_goad_effect(own, carol, Until::YourNextTurn, source);
    game.add_goad_effect(other, alice, Until::YourNextTurn, source);
    game.refresh_continuous_state();
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    assert_eq!(definition.abilities.len(), 2);
    let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[1].kind else { panic!("combat trigger"); };
    for segment in &triggered.effects.segments {
        for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    }
    assert_eq!(game.battlefield.iter().filter(|id| game.current_has_subtype(**id, Subtype::Clue)).count(), 1);
    assert!(!game.is_goaded(own), "{:#?}", triggered.effects);
    assert!(game.is_goaded(other));
    game.add_goad_effect(own, bob, Until::YourNextTurn, source);
    assert!(game.is_goaded(own));
}

#[test]
fn clear_goad_suppresses_old_aura_but_a_later_attachment_goads_again() {
    let aura = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goad aura fixture")
        .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature\nEnchanted creature is goaded.").unwrap();
    let clear = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Clear goad fixture")
        .card_types(vec![CardType::Instant])
        .parse_text("Each creature you control is no longer goaded.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id; let bob = game.players[1].id;
    let source = game.create_object_from_definition(&clear, alice, Zone::Stack);
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature fixture")
        .card_types(vec![CardType::Creature]).build();
    let first = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let attached = game.create_object_from_definition(&aura, bob, Zone::Battlefield);
    let attach = |game: &mut crate::game_state::GameState, target| {
        let mut ctx = crate::effects::EffectContext::new_default(attached, bob);
        crate::effects::execute_effect(game, &Effect::attach_to(ChooseSpec::SpecificObject(target)), &mut ctx).unwrap();
        assert_eq!(game.object(attached).unwrap().attached_to, Some(crate::object::AttachmentTarget::Object(target)));
    };
    attach(&mut game, first);
    game.refresh_continuous_state();
    assert!(game.is_goaded(first), "aura={:#?}; timestamp={:?}", aura.abilities, game.effect_store.continuous_effects.get_object_timestamp(attached));
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for segment in &clear.spell_effect.as_ref().unwrap().segments {
        for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    }
    game.refresh_continuous_state();
    assert!(!game.is_goaded(first));
    assert_eq!(game.object(attached).unwrap().attached_to, Some(crate::object::AttachmentTarget::Object(first)));
    attach(&mut game, second);
    game.refresh_continuous_state();
    assert!(!game.is_goaded(first));
    assert!(game.is_goaded(second));
    attach(&mut game, first);
    game.refresh_continuous_state();
    assert!(game.is_goaded(first), "aura={:#?}; timestamp={:?}", aura.abilities, game.effect_store.continuous_effects.get_object_timestamp(attached));
    assert!(!game.is_goaded(second));
}

#[test]
fn investigate_for_each_count_keeps_the_typed_surface() {
    let oracle = "Investigate for each goaded creature you control.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Investigate surface fixture")
        .card_types(vec![CardType::Instant]).parse_text(oracle).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle, "{:#?}", definition.spell_effect);
}
