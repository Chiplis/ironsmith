//! Frozen atlas observation 25063103: source pronoun, power-up and fight.
#[test]
fn abomination_power_up_cannot_be_reactivated_on_the_same_object() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::decision::{LegalAction, GameProgress, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::mana::ManaSymbol;
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Abomination, Terrifying Titan").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            println!("Restrictions: {:?}; annotations: {:?}", activated.activation_restrictions, activated.additional_restrictions);
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    game.next_turn();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,10);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red,4);
    let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).expect("power-up permits zero fight targets");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    assert!(!compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "power-up is spent on activation, before its ability resolves");
    let mut countered_branch = game.clone();
    countered_branch.stack.clear();
    assert!(!compute_legal_actions(&countered_branch,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "removing the unresolved ability cannot refund its activation");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(),7,"later-turn activation pays full seven mana");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
    assert_eq!(game.object(source).unwrap().counters.get(&ironsmith::object::CounterType::PlusOnePlusOne),Some(&1));
    println!("Activation counts: {:?}",game.turn_store.ability_activations_per_object);
    assert!(!compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "the same power-up ability cannot be activated again on this object");
    // An independently granted copy has its own restriction (CR 602.5c).
    let mut granted = game.clone();
    let power_up = definition.abilities.iter().find(|ability| matches!(ability.kind,ironsmith::ability::AbilityKind::Activated(_))).unwrap().clone();
    let mut grant = ironsmith::effects::GrantObjectAbilityEffect::to_source(power_up);
    grant.allow_duplicates = true;
    use ironsmith::effects::EffectExecutor;
    grant.execute(&mut granted,&mut ironsmith::effects::EffectContext::new(source,alice,&mut dm)).unwrap();
    let new_index = granted.object(source).unwrap().abilities.len()-1;
    let actions = compute_legal_actions(&granted,alice);
    assert!(!actions.iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,ability_index:1} if *id==source)));
    let action = actions.into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,ability_index} if *id==source && *ability_index==new_index)).expect("newly granted power-up has its own unused activation");
    let mut granted_state = PriorityLoopState::new(granted.players_in_game());
    let mut granted_queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut granted,&mut granted_queue,&mut granted_state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !granted.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut granted,&mut granted_queue,&mut granted_state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(granted.stack.len(),1);
    assert_eq!(granted.turn_store.ability_activations_per_object.get(&(source,granted.current_characteristics(source).unwrap().abilities.origin(new_index).unwrap().clone())),Some(&1));
    assert!(!compute_legal_actions(&granted,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),"both independent uses are now spent");
    let mut copied = game.clone();
    let donor = copied.create_object_from_definition(&definition,PlayerId::from_index(1),Zone::Battlefield);
    let copy = ironsmith::effects::ApplyContinuousEffect::new_runtime(
        ironsmith::continuous::EffectTarget::Specific(source),
        ironsmith::effects::RuntimeModification::CopyOf {
            source: ironsmith::target::ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false, name_override: None,
            name_override_surface: None, add_supertypes: Vec::new(), copy_exception_surface: None,
        }, ironsmith::effect::Until::EndOfTurn);
    copy.execute(&mut copied,&mut ironsmith::effects::EffectContext::new(source,alice,&mut dm)).unwrap();
    assert!(compute_legal_actions(&copied,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "copying a permanent grants a new instance of its power-up ability");
    let action = compute_legal_actions(&copied,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).unwrap();
    let mut copied_state = PriorityLoopState::new(copied.players_in_game());
    let mut copied_queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut copied,&mut copied_queue,&mut copied_state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !copied.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut copied,&mut copied_queue,&mut copied_state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(copied.stack.len(),1);
    assert_eq!(copied.player(alice).unwrap().mana_pool.total(),0,"becoming a copy is not an entry event");
    ironsmith::game_loop::resolve_stack_entry_with(&mut copied,&mut dm).unwrap();
    assert!(!compute_legal_actions(&copied,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),"the acquired copy is spent after its own activation");
    copied.effect_store.continuous_effects.cleanup_end_of_turn();
    copied.next_turn();
    copied.turn.priority_player = Some(alice);
    assert!(matches!(copied.current_characteristics(source).unwrap().abilities.origin(1),Some(ironsmith::continuous::AbilityOrigin::Printed(1))),"temporary copy expired");
    assert!(!compute_legal_actions(&copied,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),"expiration restores the spent printed ability");
    copy.execute(&mut copied,&mut ironsmith::effects::EffectContext::new(source,alice,&mut dm)).unwrap();
    assert!(compute_legal_actions(&copied,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),"a distinct copy effect grants another fresh instance");

    let mut reindexed = game.clone();
    let remove_trample = ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(source),
        ironsmith::continuous::Modification::RemoveAbility(ironsmith::static_abilities::StaticAbility::trample()),
        ironsmith::effect::Until::EndOfTurn);
    remove_trample.execute(&mut reindexed,&mut ironsmith::effects::EffectContext::new(source,alice,&mut dm)).unwrap();
    assert_eq!(reindexed.current_characteristics(source).unwrap().abilities.len(),1);
    assert!(!compute_legal_actions(&reindexed,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),"removing a preceding ability must not refresh power-up");
    game.next_turn();
    let bob = PlayerId::from_index(1);
    game.set_current_controller(source,bob);
    game.turn.priority_player = Some(bob);
    game.player_mut(bob).unwrap().mana_pool.add(ManaSymbol::Colorless,5);
    game.player_mut(bob).unwrap().mana_pool.add(ManaSymbol::Red,2);
    assert!(!compute_legal_actions(&game,bob).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "turn and controller changes must not refresh a used power-up");
    let exiled = game.move_object_by_effect(source,Zone::Exile).unwrap();
    let returned = game.move_object_with_etb_processing(exiled,Zone::Battlefield).unwrap().new_id;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,5);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red,2);
    assert!(compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==returned)),
        "a new object instance has a fresh power-up ability");
}

#[test]
fn abomination_power_up_reduces_by_source_mana_cost_on_entry_turn() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::decision::{LegalAction, GameProgress, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::mana::ManaSymbol;
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Abomination, Terrifying Titan").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            println!("Restrictions: {:?}; annotations: {:?}", activated.activation_restrictions, activated.additional_restrictions);
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Hand);
    let source = game.move_object_with_etb_processing(source,Zone::Battlefield).unwrap().new_id;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,10);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red,4);
    let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).expect("power-up permits zero fight targets");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    assert!(!compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "power-up is spent on activation, before its ability resolves");
    let mut countered_branch = game.clone();
    countered_branch.stack.clear();
    assert!(!compute_legal_actions(&countered_branch,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "removing the unresolved ability cannot refund its activation");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(),11,"entry-turn activation reduces seven mana by the source cost of four");
}

#[test]
fn abomination_fight_uses_countered_power_and_rechecks_target_identity() {
    use ironsmith::{GameState, PlayerId, ObjectId, Zone, CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::decision::{DecisionMaker, LegalAction, GameProgress, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::game_state::Target;
    use ironsmith::mana::ManaSymbol;
    struct Pick(ObjectId);
    impl DecisionMaker for Pick {
        fn decide_targets(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
            assert_eq!(ctx.requirements.len(),1);
            let requirement = &ctx.requirements[0];
            assert_eq!(requirement.min_targets,0);
            assert_eq!(requirement.max_targets,Some(1));
            assert_eq!(requirement.legal_targets,vec![Target::Object(self.0)],"only the opposing creature is a fight target");
            vec![Target::Object(self.0)]
        }
    }
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Abomination, Terrifying Titan").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let creature = CardDefinitionBuilder::new(CardId::new(), "Fight fixture")
        .card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(4,8)).build();
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Noncreature fixture")
        .card_types(vec![CardType::Artifact]).build();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(),"Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let target = game.create_object_from_definition(&creature,bob,Zone::Battlefield);
    game.create_object_from_definition(&creature,alice,Zone::Battlefield);
    game.create_object_from_definition(&artifact,bob,Zone::Battlefield);
    game.next_turn();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,5);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red,2);
    let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Pick(target);
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    for change in 0..=4 {
        let mut branch = game.clone();
        let mut returned_target = None;
        match change {
            1 => { branch.move_object_by_effect(target,Zone::Exile).unwrap(); }
            2 => branch.set_current_controller(target,alice),
            3 => { branch.move_object_by_effect(source,Zone::Exile).unwrap(); }
            4 => {
                let exiled = branch.move_object_by_effect(target,Zone::Exile).unwrap();
                returned_target = Some(branch.move_object_with_etb_processing(exiled,Zone::Battlefield).unwrap().new_id);
            }
            _ => {}
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut branch,&mut dm).unwrap();
        if change != 3 {
            let counters = branch.object(source).unwrap().counters.get(&ironsmith::object::CounterType::PlusOnePlusOne).copied().unwrap_or(0);
            assert_eq!(counters,if change==0 {1} else {0},"illegal sole target stops the whole ability: case {change}");
            assert_eq!(branch.damage_on(source),if change==0 {4} else {0});
        }
        if matches!(change,0|2|3) {
            assert_eq!(branch.damage_on(target),if change==0 {5} else {0},"fight uses power after the counter and requires both creatures: case {change}");
        }
        if let Some(returned) = returned_target {assert_eq!(branch.damage_on(returned),0);}
        assert_eq!(branch.player(bob).unwrap().life,20,"trample does not carry fight damage to a player");
        assert_eq!(branch.turn_store.ability_activations_per_object.values().sum::<u32>(),1);
    }
}

#[test]
fn abomination_power_up_combines_with_generic_activation_reduction() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::decision::{LegalAction, GameProgress, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::mana::ManaSymbol;
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Abomination, Terrifying Titan").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            println!("Restrictions: {:?}; annotations: {:?}", activated.activation_restrictions, activated.additional_restrictions);
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Hand);
    let source = game.move_object_with_etb_processing(source,Zone::Battlefield).unwrap().new_id;
    let reducer = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(),"Activation reducer fixture")
        .card_types(vec![ironsmith::CardType::Artifact])
        .with_ability(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::reduce_activated_ability_costs(
            ironsmith::target::ObjectFilter::creature().you_control(),2,Some(1)))).build();
    game.create_object_from_definition(&reducer,alice,Zone::Battlefield);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,10);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red,4);
    let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).expect("power-up permits zero fight targets");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    assert!(!compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "power-up is spent on activation, before its ability resolves");
    let mut countered_branch = game.clone();
    countered_branch.stack.clear();
    assert!(!compute_legal_actions(&countered_branch,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "removing the unresolved ability cannot refund its activation");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(),13,"generic activation reduction combines with the entry-turn source-cost reduction");
}

#[test]
fn abomination_power_up_pays_mana_increases_before_reduction() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::decision::{LegalAction, GameProgress, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::mana::ManaSymbol;
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Abomination, Terrifying Titan").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            println!("Restrictions: {:?}; annotations: {:?}", activated.activation_restrictions, activated.additional_restrictions);
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Hand);
    let source = game.move_object_with_etb_processing(source,Zone::Battlefield).unwrap().new_id;
    let reducer = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(),"Activation reducer fixture")
        .card_types(vec![ironsmith::CardType::Artifact])
        .with_ability(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::increase_activated_ability_costs(
            ironsmith::target::ObjectFilter::creature().you_control(),ironsmith::cost::TotalCost::mana(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))))).build();
    game.create_object_from_definition(&reducer,alice,Zone::Battlefield);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,10);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red,4);
    let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).expect("power-up permits zero fight targets");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    assert!(!compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "power-up is spent on activation, before its ability resolves");
    let mut countered_branch = game.clone();
    countered_branch.stack.clear();
    assert!(!compute_legal_actions(&countered_branch,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "removing the unresolved ability cannot refund its activation");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(),9,"mana increase is added to the activation before source-cost reduction");
}

#[test]
fn abomination_power_up_uses_current_copied_mana_cost() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::decision::{LegalAction, GameProgress, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::mana::ManaSymbol;
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Abomination, Terrifying Titan").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            println!("Restrictions: {:?}; annotations: {:?}", activated.activation_restrictions, activated.additional_restrictions);
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Hand);
    let source = game.move_object_with_etb_processing(source,Zone::Battlefield).unwrap().new_id;
    let donor_definition = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(),"Copy donor fixture")
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(4,4))
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1),ManaSymbol::Red]))
        .with_ability(definition.abilities.iter().find(|ability| matches!(ability.kind,ironsmith::ability::AbilityKind::Activated(_))).unwrap().clone()).build();
    let donor = game.create_object_from_definition(&donor_definition,PlayerId::from_index(1),Zone::Battlefield);
    let copy = ironsmith::effects::ApplyContinuousEffect::new_runtime(
        ironsmith::continuous::EffectTarget::Specific(source),
        ironsmith::effects::RuntimeModification::CopyOf {
            source: ironsmith::target::ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false, name_override: None,
            name_override_surface: None, add_supertypes: Vec::new(), copy_exception_surface: None,
        }, ironsmith::effect::Until::EndOfTurn);
    use ironsmith::effects::EffectExecutor;
    copy.execute(&mut game,&mut ironsmith::effects::EffectContext::new_default(source,alice)).unwrap();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless,10);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red,4);
    let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).expect("power-up permits zero fight targets");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    assert!(!compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "power-up is spent on activation, before its ability resolves");
    let mut countered_branch = game.clone();
    countered_branch.stack.clear();
    assert!(!compute_legal_actions(&countered_branch,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),
        "removing the unresolved ability cannot refund its activation");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(),9,"entry-turn reduction uses current copied mana cost, not the printed original");
}

#[test]
fn abomination_activation_records_announced_ability_when_mana_changes_its_text() {
    use ironsmith::{GameState, PlayerId, Zone, CardType};
    use ironsmith::decision::{LegalAction, GameProgress, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::mana::ManaSymbol;
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Abomination, Terrifying Titan").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(),"Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let donor = game.create_object_from_definition(&definition,PlayerId::from_index(1),Zone::Battlefield);
    let copy = ironsmith::effects::ApplyContinuousEffect::new_runtime(
        ironsmith::continuous::EffectTarget::Specific(source),
        ironsmith::effects::RuntimeModification::CopyOf {
            source: ironsmith::target::ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false, name_override: None,
            name_override_surface: None, add_supertypes: Vec::new(), copy_exception_surface: None,
        }, ironsmith::effect::Until::EndOfTurn);
    let mut mana = ironsmith::Ability::mana(ironsmith::cost::TotalCost::free(),vec![ManaSymbol::Red;7]);
    if let ironsmith::ability::AbilityKind::Activated(activated) = &mut mana.kind {
        activated.effects = vec![ironsmith::effect::Effect::new(copy)].into();
    }
    let land = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(),"Mana and copy fixture")
        .card_types(vec![CardType::Land]).with_ability(mana).build();
    game.create_object_from_definition(&land,alice,Zone::Battlefield);
    game.next_turn();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let action = compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..32 {
        if !game.stack.is_empty() {break;}
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}");};
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    assert!(matches!(game.current_characteristics(source).unwrap().abilities.origin(1),Some(ironsmith::continuous::AbilityOrigin::Effect{..})),"mana ability changed the source during payment");
    assert!(compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),"the acquired ability was not the one activated");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    assert!(!compute_legal_actions(&game,alice).iter().any(|action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)),"the originally announced printed ability is spent");
}
