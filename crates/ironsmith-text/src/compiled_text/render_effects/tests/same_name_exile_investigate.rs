use super::*;

#[test]
fn same_name_exile_investigates_only_for_the_actual_nontoken_exiled_set() {
    let oracle = "Exile target creature and all other creatures its controller controls with the same name as that creature. That player investigates for each nontoken creature exiled this way.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Exile Set Probe")
        .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
    for (target_token, animated_land, redirected) in [(false, false, false), (true, false, false), (false, true, false), (true, true, false), (false, false, true), (true, false, true), (false, true, true), (true, true, true)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Matching Creature")
            .card_types(vec![if animated_land { CardType::Land } else { CardType::Creature }]).build();
        let target = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        if target_token { game.object_mut(target).unwrap().kind = crate::object::ObjectKind::Token; }
        let sibling = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let token = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        game.object_mut(token).unwrap().kind = crate::object::ObjectKind::Token;
        let other_controller = game.create_object_from_card(&creature, carol, Zone::Battlefield);
        game.create_object_from_card(&creature, bob, Zone::Exile);
        let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Unrelated Creature")
            .card_types(vec![CardType::Creature]).build();
        let unrelated = game.create_object_from_card(&other, bob, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        if animated_land {
            let animate = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(ObjectFilter::default().named("Matching Creature")),
                crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
                Until::EndOfTurn,
            );
            crate::effects::execute_effect(&mut game, &Effect::new(animate), &mut ctx).unwrap();
            assert!(game.current_is_creature(target));
        }
        if redirected {
            let replacement = crate::effects::RegisterFutureZoneReplacementEffect::new(
                ObjectFilter::specific(target), Some(Zone::Battlefield), Some(Zone::Exile),
                Zone::Hand, crate::effects::ReplacementApplyMode::OneShot,
            );
            crate::effects::execute_effect(&mut game, &Effect::new(replacement), &mut ctx).unwrap();
        }
        ctx.snapshot_targets(&game);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|err| panic!("token={target_token} animated={animated_land} redirected={redirected}: {err:?} effect={effect:#?}")); }
        }
        for id in [target, sibling, token] { assert!(game.object(id).is_none()); }
        for id in [other_controller, unrelated] { assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield); }
        let clues: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id))
            .filter(|object| object.name == "Clue").collect();
        assert_eq!(clues.len(), if target_token || redirected { 1 } else { 2 }, "token={target_token}, animated={animated_land}, redirected={redirected}");
        assert!(clues.iter().all(|clue| game.controller_of(clue) == bob));
    }
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition), [oracle]);
}
