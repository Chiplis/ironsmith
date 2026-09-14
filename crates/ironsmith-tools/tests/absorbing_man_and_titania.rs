//! Frozen atlas observation 24871583: source-filtered damage multiplication.
#[test]
fn canonical_multiplier_applies_to_all_damage_recipients_only_from_your_creatures() {
    use ironsmith::{GameState,PlayerId,Zone,CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    let payloads=ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Absorbing Man and Titania").unwrap();
    let definition=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let creature=CardDefinitionBuilder::new(CardId::new(),"Creature damage fixture").card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(20,20)).build();
    let artifact=CardDefinitionBuilder::new(CardId::new(),"Artifact damage fixture").card_types(vec![CardType::Artifact]).build();
    let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
    for source_kind in 0..4 {for recipient in 0..3 {for amount in [0,3] {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let doubler=game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let source=match source_kind {0=>doubler,1=>game.create_object_from_definition(&creature,alice,Zone::Battlefield),2=>game.create_object_from_definition(&creature,bob,Zone::Battlefield),_=>game.create_object_from_definition(&artifact,alice,Zone::Battlefield)};
        let victim=game.create_object_from_definition(&creature,bob,Zone::Battlefield);
        let target=match recipient {0=>ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(bob)),1=>ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(alice)),_=>ironsmith::target::ChooseSpec::SpecificObject(victim)};
        let controller=game.current_controller(source).unwrap();
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(source,controller,vec![ironsmith::Effect::deal_damage(amount,target)]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        let expected=amount * if source_kind<2 {2}else{1};
        assert_eq!(game.player(bob).unwrap().life,20-if recipient==0 {expected}else{0});
        assert_eq!(game.player(alice).unwrap().life,20-if recipient==1 {expected}else{0});
        assert_eq!(game.damage_on(victim),if recipient==2 {expected as u32}else{0});
    }}}
}
#[test]
fn canonical_multiplier_doubles_after_trample_assignment() {
    use ironsmith::{GameState,PlayerId,Zone,CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::combat_state::{AttackTarget,AttackerInfo,CombatState};
    let payloads=ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Absorbing Man and Titania").unwrap();
    let definition=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
    let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
    game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let attack=CardDefinitionBuilder::new(CardId::new(),"Trample fixture").card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(5,5)).with_ability(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::trample())).build();
    let block=CardDefinitionBuilder::new(CardId::new(),"Block fixture").card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(2,2)).build();
    let attacker=game.create_object_from_definition(&attack,alice,Zone::Battlefield);let blocker=game.create_object_from_definition(&block,bob,Zone::Battlefield);
    let mut combat=CombatState::default();combat.attackers.push(AttackerInfo{creature:attacker,target:AttackTarget::Player(bob)});combat.blockers.insert(attacker,vec![blocker]);
    ironsmith::game_loop::execute_combat_damage_step(&mut game,&combat,false);
    assert_eq!(game.damage_on(blocker),4);assert_eq!(game.damage_on(attacker),2);assert_eq!(game.player(bob).unwrap().life,14);
}
#[test]
fn canonical_multiplier_uses_current_source_characteristics_and_departed_source_lki() {
    use ironsmith::{GameState,PlayerId,Zone,CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::effects::EffectExecutor;
    let payloads=ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Absorbing Man and Titania").unwrap();
    let definition=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let creature=CardDefinitionBuilder::new(CardId::new(),"Changing damage source fixture").card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(3,3)).build();
    let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
    // unchanged, control changes, type changes, leaves, doubler leaves.
    for change in 0..7 {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let doubler=game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let source=game.create_object_from_definition(&creature,alice,Zone::Battlefield);
        let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(),&game);
        let entry=ironsmith::game_state::StackEntry::ability(source,alice,vec![ironsmith::Effect::deal_damage(3,ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(bob)))]).with_source_snapshot(snapshot);
        game.push_to_stack(entry);
        let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
        match change {
            1=>game.set_current_controller(source,bob),
            2=>{ironsmith::effects::ApplyContinuousEffect::new(ironsmith::continuous::EffectTarget::Specific(source),ironsmith::continuous::Modification::SetCardTypes(vec![CardType::Artifact]),ironsmith::effect::Until::EndOfTurn).execute(&mut game,&mut ironsmith::effects::EffectContext::new(doubler,alice,&mut dm)).unwrap();},
            3=>{game.move_object_by_effect(source,Zone::Graveyard).unwrap();},
            4=>{game.move_object_by_effect(doubler,Zone::Graveyard).unwrap();},
            5=>{game.set_current_controller(source,bob);game.move_object_by_effect(source,Zone::Graveyard).unwrap();},
            6=>{ironsmith::effects::ApplyContinuousEffect::new(ironsmith::continuous::EffectTarget::Specific(source),ironsmith::continuous::Modification::SetCardTypes(vec![CardType::Artifact]),ironsmith::effect::Until::EndOfTurn).execute(&mut game,&mut ironsmith::effects::EffectContext::new(doubler,alice,&mut dm)).unwrap();game.move_object_by_effect(source,Zone::Graveyard).unwrap();},
            _=>{}
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.player(bob).unwrap().life,if change==0 || change==3 {14}else{17},"change={change}");
    }
}
#[test]
fn canonical_multiplier_and_prevention_respect_affected_players_order() {
    use ironsmith::{GameState,PlayerId,Zone,CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    struct Order {last:bool,player:PlayerId,choices:usize}
    impl ironsmith::decision::DecisionMaker for Order {
        fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
            assert_eq!(ctx.player,self.player,"the damaged player chooses replacement order");
            self.choices+=1;
            let options=ctx.options.iter().filter(|o|o.legal).collect::<Vec<_>>();
            vec![if self.last {options.last().unwrap().index}else{options[0].index}]
        }
    }
    let payloads=ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Absorbing Man and Titania").unwrap();
    let definition=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
    let mut results=Vec::new();
    for last in [false,true] {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let source=game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let shield=CardDefinitionBuilder::new(CardId::new(),"Prevention fixture").card_types(vec![CardType::Enchantment]).with_ability(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::prevent_damage_to_you_from_source_filter(2,ironsmith::target::ObjectFilter::default(),"Prevent 2 damage to you"))).build();
        game.create_object_from_definition(&shield,bob,Zone::Battlefield);
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(source,alice,vec![ironsmith::Effect::deal_damage(3,ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(bob)))]));
        let mut dm=Order{last,player:bob,choices:0};
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert!(dm.choices>0,"both replacement orders must be offered");
        results.push(20-game.player(bob).unwrap().life);
    }
    results.sort();assert_eq!(results,vec![2,4],"(3-2)*2 or 3*2-2");
}
#[test]
fn canonical_multiplier_structure_has_source_filter_and_no_recipient_restriction() {
    let payloads=ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Absorbing Man and Titania").unwrap();
    let definition=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    assert_eq!(definition.abilities.len(),1);
    let ironsmith::ability::AbilityKind::Static(ability)=&definition.abilities[0].kind else {panic!("expected static replacement")};
    let ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement{source_filter,target_player_filter,target_object_filter,factor,combat_only,..}=&ability.compiled_model().unwrap().payload else {panic!("expected typed multiplier")};
    assert_eq!(source_filter.zone,None,"source noun is not limited to battlefield");
    assert_eq!(source_filter.card_types,vec![ironsmith::CardType::Creature]);
    assert_eq!(source_filter.controller,Some(ironsmith::target::PlayerFilter::You));
    assert_eq!(target_player_filter,&Some(ironsmith::target::PlayerFilter::Any));
    assert_eq!(target_object_filter,&Some(ironsmith::target::ObjectFilter::default()));
    assert_eq!(*factor,2);assert!(!combat_only);
    assert_eq!(definition.abilities[0].functional_zones,vec![ironsmith::Zone::Battlefield]);
    let snapshot=ironsmith_tools::compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(snapshot.parse_status,ironsmith_tools::ParseStatus::StrictCompiled);assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
}
#[test]
fn canonical_multiplier_includes_creature_sources_outside_the_battlefield() {
    use ironsmith::{GameState,PlayerId,Zone,CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    let payloads=ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Absorbing Man and Titania").unwrap();
    let definition=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let creature=CardDefinitionBuilder::new(CardId::new(),"Off-battlefield creature source fixture").card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(3,3)).build();
    let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
    for zone in [Zone::Stack,Zone::Hand,Zone::Graveyard] {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let source=game.create_object_from_definition(&creature,alice,zone);
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(source,alice,vec![ironsmith::Effect::deal_damage(3,ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(bob)))]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(bob).unwrap().life,14,"creature source in {zone:?}, CR109.2c");
    }
}
