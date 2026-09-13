//! Frozen atlas observation 25063080: mana spent, entry controller and partner group.
use ironsmith_tools::{load_card_payloads_by_name,default_cards_path};

#[test]
fn abby_noncast_entry_changes_control_without_cast_tokens() {
    use ironsmith::{GameState, PlayerId, Zone, Effect};
    use ironsmith::decision::DecisionMaker;
    struct ChooseOpponent { selected: usize, calls: usize }
    impl DecisionMaker for ChooseOpponent {
        fn decide_options(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!((ctx.min, ctx.max), (1, 1));
            assert_eq!(ctx.options.iter().map(|option| option.index).collect::<Vec<_>>(), vec![1, 2]);
            self.calls += 1;
            vec![self.selected]
        }
    }
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::ChooseSpec;
    let payloads = load_card_payloads_by_name(default_cards_path().to_str().unwrap(), "Abby, Merciless Soldier").unwrap();
    let def = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    for selected in [1, 2] {
    let opponent = PlayerId::from_index(selected as u8);
    for origin in [Zone::Graveyard, Zone::Exile, Zone::Hand] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let source = game.create_object_from_definition(&def, alice, origin);
        let stable = game.object(source).unwrap().stable_id;
        let mut dm = ChooseOpponent { selected, calls: 0 };
        let mut ctx = EffectContext::new(source, alice, &mut dm);
        execute_effect(&mut game, &Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Battlefield, true), &mut ctx).unwrap();
        let entered = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(entered).unwrap().owner, alice);
        assert_eq!(game.current_controller(entered), Some(opponent));
        assert_eq!(dm.calls, 1);
        assert!(game.permanents_controlled_by(alice).is_empty());
        assert_eq!(game.permanents_controlled_by(opponent), vec![entered]);
        assert!(game.stack.is_empty());
        for event in game.take_pending_trigger_events() {
            assert!(ironsmith::triggers::check_triggers(&game, &event).is_empty(), "noncast entry must not trigger token creation");
        }
    }
}

}

#[test]
fn abby_normal_cast_creates_tokens_for_caster_then_enters_for_opponent() {
    use ironsmith::{GameState,PlayerId,Zone};
    use ironsmith::decision::{GameProgress,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse};
    use ironsmith::mana::ManaSymbol;
    let payloads=load_card_payloads_by_name(default_cards_path().to_str().unwrap(),"Abby, Merciless Soldier").unwrap();
    assert_eq!(payloads.len(),1);
    let snapshot=ironsmith_tools::compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(snapshot.parse_status,ironsmith_tools::ParseStatus::StrictCompiled,"{snapshot:#?}");
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
    let def=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
    for (alternative,paid,spell_leaves) in [(false,3,false),(true,0,false),(true,2,false),(true,5,false),(false,3,true)] {
    let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
    game.turn.active_player=alice;
    game.turn.phase=ironsmith::game_state::Phase::FirstMain;
    game.turn.priority_player=Some(alice);
    let source=game.create_object_from_definition(&def,alice,Zone::Hand);
    let stable=game.object(source).unwrap().stable_id;
    if alternative {
        use ironsmith::mana::ManaCost;
        game.effect_store.grant_registry.grant_alternative_cast_to_card(source,Zone::Hand,alice,
            ironsmith::alternative_cast::AlternativeCastingMethod::Composed {
                name:"Fixture alternative payment".into(),
                total_cost:ironsmith::cost::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(paid)])),
                condition:None,prototype_power_toughness:None,
            },ironsmith::grant_registry::GrantSource::Effect{source_id:source,expires_end_of_turn:game.turn.turn_number});
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,paid as u32);
    } else {
        for symbol in [ManaSymbol::Colorless,ManaSymbol::Red,ManaSymbol::Green] {
            game.player_mut(alice).unwrap().mana_pool.add(symbol,1);
        }
    }
    let action=compute_legal_actions(&game,alice).into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==source)).unwrap();
    let mut queue=ironsmith::triggers::TriggerQueue::new();
    let mut state=PriorityLoopState::new(game.players_in_game());
    let mut dm=SelectFirstDecisionMaker;
    let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty(){break;}
        let GameProgress::NeedsDecisionCtx(ctx)=progress else {panic!("{progress:?}");};
        progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),2,"cast trigger goes above creature spell");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(),0);
    if spell_leaves {
        let original=game.stack.remove(0).object_id;
        game.move_object_by_effect(original,Zone::Graveyard);
    }
    ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
    let tokens=game.permanents_controlled_by(alice);
    assert_eq!(tokens.len(),paid as usize,"token count must equal mana actually paid, including when the spell left");
    for token in tokens {
        let object=game.object(token).unwrap();
        assert_eq!(object.name.as_ref(),"Cordyceps Infected");
        assert_eq!(object.kind,ironsmith::object::ObjectKind::Token);
        assert_eq!(object.colors(),ironsmith::color::ColorSet::BLACK);
        assert_eq!(game.calculated_power(token),Some(1));
        assert_eq!(game.calculated_toughness(token),Some(1));
        assert!(object.subtypes.contains(&ironsmith::types::Subtype::Fungus));
        assert!(object.subtypes.contains(&ironsmith::types::Subtype::Zombie));
    }
    if spell_leaves {
        assert!(game.stack.is_empty());
        let retained=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(retained).unwrap().zone,Zone::Graveyard);
        continue;
    }
    ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
    let entered=game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(entered).unwrap().zone,Zone::Battlefield);
    assert_eq!(game.object(entered).unwrap().owner,alice);
    assert_eq!(game.current_controller(entered),Some(bob));
    assert_eq!(game.permanents_controlled_by(alice).len(),paid as usize);
}
}

#[test]
fn abby_entry_uses_proposed_controller_to_choose_opponent() {
    use ironsmith::{GameState, PlayerId, Zone, Effect};
    use ironsmith::decision::DecisionMaker;
    use ironsmith::effects::{EffectContext, execute_effect};
    struct ChooseOwner;
    impl DecisionMaker for ChooseOwner {
        fn decide_options(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.player, PlayerId::from_index(1));
            assert_eq!(ctx.options.iter().map(|option| option.index).collect::<Vec<_>>(), vec![0, 2]);
            vec![0]
        }
    }
    let payloads = load_card_payloads_by_name(default_cards_path().to_str().unwrap(), "Abby, Merciless Soldier").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    let stable = game.object(source).unwrap().stable_id;
    let mut movement = ironsmith::effects::MoveToZoneEffect::new(
        ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Battlefield, false,
    );
    movement.battlefield_controller = ironsmith::effects::BattlefieldController::You;
    let mut dm = ChooseOwner;
    let mut ctx = EffectContext::new(source, bob, &mut dm);
    execute_effect(&mut game, &Effect::new(movement), &mut ctx).unwrap();
    let entered = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.object(entered).unwrap().owner, alice);
    assert_eq!(game.current_controller(entered), Some(alice));
    assert_eq!(game.permanents_controlled_by(alice), vec![entered]);
    assert!(game.permanents_controlled_by(bob).is_empty());
}
