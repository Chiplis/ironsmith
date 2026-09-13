//! Frozen atlas observation 25063087: counted reanimation with entry counters.
#[test]
fn aberrant_return_canonical_structure() {
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(), "Aberrant Return",
    ).unwrap();
    assert_eq!(payloads.len(), 1);
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    println!("{definition:#?}");
    let program = definition.spell_effect.as_ref().unwrap();
    assert_eq!(program.segments.len(), 1);
    let effects = &program.segments[0].default_effects;
    assert_eq!(effects.len(), 1, "entry counters must be fused into the move");
    let tagged = effects[0].downcast_ref::<ironsmith::effects::TaggedEffect>().unwrap();
    let moved = tagged.effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>().unwrap();
    assert_eq!(moved.zone, ironsmith::Zone::Battlefield);
    assert_eq!(moved.target.count().min, 1);
    assert_eq!(moved.target.count().max, Some(3));
    let [counter] = moved.enters_with_counters.as_slice() else { panic!("{moved:#?}"); };
    assert_eq!(counter.counter_type, ironsmith::object::CounterType::MinusOneMinusOne);
    assert_eq!(counter.amount.unhinted(), &ironsmith::effect::Value::Fixed(1));
    assert!(counter.condition.is_none() && counter.object_filter.is_none(),
        "oracle has no entry-time creature condition: {counter:#?}");
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payloads[0]);
    println!("{snapshot:#?}");
    assert_eq!(snapshot.parse_status, ironsmith_tools::ParseStatus::StrictCompiled);
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
}

#[test]
fn aberrant_return_counted_targets_return_only_legal_identities_with_counters() {
    use ironsmith::{GameState, PlayerId, Zone, ObjectId, CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::game_state::Target;
    use ironsmith::mana::ManaSymbol;
    struct Pick { selected: Vec<ObjectId>, excluded: Vec<ObjectId>, prompts: usize }
    impl DecisionMaker for Pick {
        fn decide_targets(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
            self.prompts += 1;
            assert_eq!(ctx.requirements.len(), 1);
            let requirement = &ctx.requirements[0];
            assert_eq!(requirement.min_targets, 1);
            assert_eq!(requirement.max_targets, Some(3));
            use ironsmith::targeting::validate_flat_target_assignment;
            assert_eq!(requirement.legal_targets.len(), 4);
            assert!(!validate_flat_target_assignment(&ctx.requirements, &[]));
            assert!(!validate_flat_target_assignment(&ctx.requirements, &requirement.legal_targets));
            assert!(!validate_flat_target_assignment(&ctx.requirements, &[requirement.legal_targets[0], requirement.legal_targets[0]]));
            for amount in 1..=3 {
                assert!(validate_flat_target_assignment(&ctx.requirements, &requirement.legal_targets[..amount]));
            }
            let mut separate = requirement.clone();
            separate.min_targets = 1;
            separate.max_targets = Some(1);
            let duplicate = [requirement.legal_targets[0], requirement.legal_targets[0]];
            assert!(validate_flat_target_assignment(&[separate.clone(), separate.clone()], &duplicate),
                "separate authored target groups may reuse an object");
            separate.distinct_player_group = Some(0);
            assert!(!validate_flat_target_assignment(&[separate.clone(), separate], &duplicate));
            for id in &self.selected { assert!(requirement.legal_targets.contains(&Target::Object(*id))); }
            for id in &self.excluded { assert!(!requirement.legal_targets.contains(&Target::Object(*id))); }
            self.selected.iter().copied().map(Target::Object).collect()
        }
    }
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Aberrant Return").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let creature = CardDefinitionBuilder::new(CardId::new(), "Reanimation fixture")
        .card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(3,3)).build();
    let intrinsic = CardDefinitionBuilder::new(CardId::new(), "Intrinsic counter fixture")
        .card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(6,6))
        .with_ability(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::enters_with_counters(ironsmith::object::CounterType::MinusOneMinusOne,2))).build();
    let counterproof = CardDefinitionBuilder::new(CardId::new(), "Counter prohibition fixture")
        .card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(3,3))
        .with_ability(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::cant_have_counters_placed())).build();
    let noncreature = CardDefinitionBuilder::new(CardId::new(), "Noncreature fixture").card_types(vec![CardType::Artifact]).build();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for count in 1..=3 {
        // 0: unchanged; 1: one illegal; 2: all illegal; 3: one leaves and returns.
        for change in 0..=3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()],20);
            game.turn.active_player = alice;
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.priority_player = Some(alice);
            let ids = (0..4).map(|i| game.create_object_from_definition(if i==1 {&intrinsic} else if i==2 {&counterproof} else {&creature}, if i%2==0 {bob} else {alice}, Zone::Graveyard)).collect::<Vec<_>>();
            let stable = ids.iter().map(|id| game.object(*id).unwrap().stable_id).collect::<Vec<_>>();
            let wrong_zone = game.create_object_from_definition(&creature,alice,Zone::Hand);
            let wrong_type = game.create_object_from_definition(&noncreature,bob,Zone::Graveyard);
            let source = game.create_object_from_definition(&definition,alice,Zone::Hand);
            game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black,2);
            game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,4);
            let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==source)).unwrap();
            let mut dm = Pick {selected: ids[..count].to_vec(), excluded: vec![wrong_zone,wrong_type], prompts: 0};
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut state = PriorityLoopState::new(game.players_in_game());
            let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
            for _ in 0..24 {
                if !game.stack.is_empty() {break;}
                let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
                progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
            }
            assert_eq!(game.stack.len(),1);
            assert_eq!(dm.prompts,1);
            assert_eq!(game.player(alice).unwrap().mana_pool.total(),0);
            if change != 0 {
                for id in ids.iter().take(if change==2 {count} else {1}) {
                    let exiled = game.move_object_by_effect(*id,Zone::Exile).unwrap();
                    if change==3 {game.move_object_by_effect(exiled,Zone::Graveyard);}
                }
            }
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            for i in 0..4 {
                let current = game.find_object_by_stable_id(stable[i]).unwrap();
                let object = game.object(current).unwrap();
                let invalid = i<count && (change==2 || (change!=0 && i==0));
                let returned = i<count && !invalid;
                let expected_zone = if returned {Zone::Battlefield} else if invalid && change!=3 {Zone::Exile} else {Zone::Graveyard};
                assert_eq!(object.zone,expected_zone,"count={count}, change={change}, i={i}");
                assert_eq!(object.owner,if i%2==0 {bob} else {alice});
                assert_eq!(object.counters.get(&ironsmith::object::CounterType::MinusOneMinusOne).copied().unwrap_or(0),if returned {if i==1 {3} else if i==2 {0} else {1}} else {0});
                if returned {
                    assert_eq!(game.current_controller(current),Some(alice));
                    assert_eq!(game.calculated_power(current),Some(if i==1 || i==2 {3} else {2}));
                    assert_eq!(game.calculated_toughness(current),Some(if i==1 || i==2 {3} else {2}));
                }
            }
            assert_eq!(game.object(wrong_type).unwrap().zone,Zone::Graveyard);
            assert_eq!(game.object(wrong_zone).unwrap().zone,Zone::Hand);
        }
    }
}

#[test]
fn aberrant_return_cannot_cast_without_a_legal_target() {
    use ironsmith::{GameState, PlayerId, Zone, CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Aberrant Return").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()],20);
    game.turn.active_player = alice;
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.priority_player = Some(alice);
    let source = game.create_object_from_definition(&definition,alice,Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black,2);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,4);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Legality fixture").card_types(vec![CardType::Creature]).build();
    let noncreature = CardDefinitionBuilder::new(CardId::new(), "Artifact fixture").card_types(vec![CardType::Artifact]).build();
    game.create_object_from_definition(&creature,alice,Zone::Hand);
    game.create_object_from_definition(&noncreature,alice,Zone::Graveyard);
    let can_cast = |game: &GameState| compute_legal_actions(game,alice).iter().any(|action| matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==source));
    assert!(!can_cast(&game));
    game.create_object_from_definition(&creature,PlayerId::from_index(1),Zone::Graveyard);
    assert!(can_cast(&game), "a creature in the opponent's graveyard enables casting");
}
