//! Exact full sources, independent compiler/artifact routes and real resolution paths.
mod custom_final_next_wave_support;
use ironsmith::{CardDefinition,GameState,PlayerId,ObjectId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{execute_effect,EffectContext};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture(family:&str)->serde_json::Value { serde_json::from_str(match family {
 "Solidarity"=>include_str!("../../../fixtures/custom_mechanics/revolution-solidarity-complete.json"),
 "Possess"=>include_str!("../../../fixtures/custom_mechanics/revolution-possess-complete.json"), _=>unreachable!()
}).unwrap() }
fn source(family:&str,name:&str)->serde_json::Value {fixture(family)["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap().clone()}
fn definitions(family:&str,name:&str)->[CardDefinition;2]{custom_final_next_wave_support::definitions(&source(family,name)).unwrap_or_else(|e|panic!("{name}: {e}"))}
#[test]
fn every_exact_solidarity_and_possess_source_compiles_through_both_routes(){
 let mut errors=Vec::new();
 for family in ["Solidarity","Possess"]{
  let f=fixture(family);let cards=f["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),15);
  for card in cards {if let Err(error)=custom_final_next_wave_support::definitions(card){errors.push(format!("{}: {error}",card["name"]));}}
 }
 assert!(errors.is_empty(),"{}",errors.join("\n"));
}
fn body(game:&mut GameState,owner:PlayerId,types:&str)->ObjectId{
 let d=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent attacker",&format!("Type: Creature — {types}\nPower/Toughness: 2/2"),false).unwrap();game.create_object_from_definition(&d,owner,Zone::Battlefield)
}
fn queue_attack(game:&mut GameState,attacker:ObjectId,defender:PlayerId){
 let mut combat=ironsmith::combat_state::CombatState::default();combat.attackers.push(ironsmith::combat_state::AttackerInfo{creature:attacker,target:ironsmith::combat_state::AttackTarget::Player(defender)});
 let declaration=std::sync::Arc::from(combat.attackers.clone());game.combat=Some(combat);game.invalidate_continuous_state_public();
 game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(attacker,ironsmith::events::AttackEventTarget::Player(defender)).with_declared_attackers(declaration),Default::default()));
}
fn stack_triggers(game:&mut GameState)->usize{
 let mut q=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut q,&mut dm).unwrap();let n=q.entries.len();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut q,&mut dm).unwrap();n
}
fn resolve_all(game:&mut GameState){while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}}
fn banners(game:&GameState,owner:PlayerId)->Vec<ObjectId>{game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|o|o.name.as_str()=="Solidarity"&&game.controller_of(o)==owner)).collect()}
#[test]
fn solidarity_counts_distinct_attacking_types_and_shares_first_resolution_across_sources(){
 for definition in definitions("Solidarity","Barded Brachion") {for six in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;
  game.create_object_from_definition(&definition,A,Zone::Battlefield);game.create_object_from_definition(&definition,A,Zone::Battlefield);game.create_object_from_definition(&definition,B,Zone::Battlefield);
  let attacker=body(&mut game,A,if six {"Human Cleric Soldier Scout Warrior Wizard"}else{"Human Cleric Soldier Scout Warrior"});
  queue_attack(&mut game,attacker,B);assert_eq!(stack_triggers(&mut game),if six{2}else{0});resolve_all(&mut game);
  assert_eq!(banners(&game,A).len(),if six{1}else{0});assert!(banners(&game,B).is_empty());
  if !six{continue;}
  let token=banners(&game,A)[0];let chars=game.calculated_characteristics(token).unwrap();assert_eq!(chars.card_types,vec![ironsmith::types::CardType::Enchantment]);assert!(!game.current_abilities(token).unwrap().is_empty());
  assert_eq!(game.calculated_characteristics(attacker).unwrap().power,Some(3),"token anthem applies to attacking creatures");
  queue_attack(&mut game,attacker,B);assert_eq!(stack_triggers(&mut game),2);resolve_all(&mut game);assert_eq!(banners(&game,A).len(),1,"extra combat does not reset the player-turn limit");
  game.combat=None;game.invalidate_continuous_state_public();assert_eq!(game.calculated_characteristics(attacker).unwrap().power,Some(2));
  game.next_turn();game.turn.active_player=A;queue_attack(&mut game,attacker,B);assert_eq!(stack_triggers(&mut game),2);resolve_all(&mut game);assert_eq!(banners(&game,A).len(),2);
 }}
}
#[test]
fn countered_solidarity_does_not_consume_shared_resolution_and_departed_source_keeps_program(){
 for definition in definitions("Solidarity","Barded Brachion"){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;
  let one=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let attacker=body(&mut game,A,"Human Cleric Soldier Scout Warrior Wizard");queue_attack(&mut game,attacker,B);assert_eq!(stack_triggers(&mut game),2);
  let top=game.stack.last().unwrap().ability_id.expect("exact ability target");execute_effect(&mut game,&ironsmith::Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(top)),&mut EffectContext::new_default(one,A)).unwrap();assert_eq!(game.stack.len(),1);
  let remaining_source=game.stack.last().unwrap().object_id;
  execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(remaining_source),Zone::Graveyard,false),&mut EffectContext::new_default(one,A)).unwrap();
  resolve_all(&mut game);assert_eq!(banners(&game,A).len(),1,"countered occurrence is not a resolution; acquired source program survives departure");
 }
}
#[test]
fn possess_full_source_returns_as_targeted_aura_and_preserves_original_abilities(){
 for name in ["Breathstealer Spirit","Tarnished Swordsuit"] {for definition in definitions("Possess",name){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let actor=game.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=game.object(actor).unwrap().stable_id;let host=body(&mut game,B,"Human");
  execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(actor),Zone::Graveyard,false),&mut EffectContext::new_default(actor,A)).unwrap();
  assert_eq!(stack_triggers(&mut game),1);assert_eq!(game.stack.last().unwrap().targets.len(),1);resolve_all(&mut game);
  let returned=game.find_object_by_stable_id(stable).unwrap();let object=game.object(returned).unwrap();assert_eq!(object.zone,Zone::Battlefield);assert_eq!(object.attached_to,Some(ironsmith::object::AttachmentTarget::Object(host)));
  let chars=game.calculated_characteristics(returned).unwrap();assert_eq!(chars.card_types,vec![ironsmith::types::CardType::Enchantment]);assert!(chars.subtypes.contains(&ironsmith::types::Subtype::Aura));assert!(!game.current_abilities(returned).unwrap().is_empty());
  if name=="Breathstealer Spirit" {assert_eq!(game.current_controller(host),Some(A));}
  else {queue_attack(&mut game,host,A);assert_eq!(stack_triggers(&mut game),1);resolve_all(&mut game);assert_eq!(game.calculated_characteristics(host).unwrap().power,Some(4));assert!(game.object_has_static_ability_id(host,ironsmith::static_abilities::StaticAbilityId::Menace));}
 }}
}
#[test]
fn possess_requires_nontoken_creature_lki_and_a_legal_target(){
 for definition in definitions("Possess","Breathstealer Spirit") {for (token,creature,host) in [(true,true,true),(false,false,true),(false,true,false)]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let actor=game.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=game.object(actor).unwrap().stable_id;
  if token{game.object_mut(actor).unwrap().kind=ironsmith::object::ObjectKind::Token;}
  if !creature{let object=game.object_mut(actor).unwrap();object.card_types=vec![ironsmith::types::CardType::Enchantment].into();}
  if host{body(&mut game,B,"Human");}
  execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(actor),Zone::Graveyard,false),&mut EffectContext::new_default(actor,A)).unwrap();
  stack_triggers(&mut game);assert!(game.stack.is_empty());
  if !token{assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);}
 }}
}
#[test]
fn possess_cannot_attach_after_its_declared_target_departs(){
 for definition in definitions("Possess","Breathstealer Spirit"){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let actor=game.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=game.object(actor).unwrap().stable_id;let host=body(&mut game,B,"Human");
  execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(actor),Zone::Graveyard,false),&mut EffectContext::new_default(actor,A)).unwrap();assert_eq!(stack_triggers(&mut game),1);
  execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(host),Zone::Graveyard,false),&mut EffectContext::new_default(actor,A)).unwrap();resolve_all(&mut game);
  assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);
 }
}

#[test]
fn edgeland_postcreation_reflexive_modes_charge_only_the_selected_cost() {
    struct Mode(bool);
    impl ironsmith::decision::DecisionMaker for Mode {
        fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
            if ctx.options.len()==3 {if self.0 {vec![2]}else{Vec::new()}}
            else {ironsmith::decision::DecisionMaker::decide_options(&mut SelectFirstDecisionMaker,game,ctx)}
        }
    }
    for definition in definitions("Solidarity","Edgeland Pledge") {for choose in [false,true] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.turn.active_player=A;game.turn.priority_player=Some(A);
        game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
        // Resolve the source's actual stack program, then its separate reflexive
        // ability. The selected mode payment belongs to that second resolution.
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);
        game.push_to_stack(ironsmith::StackEntry::new(source,A));
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Green,1);
        let mut dm=Mode(choose);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let solidarity=game.battlefield.iter().copied().find(|id|game.object(*id).is_some_and(|object|object.name=="Solidarity")).unwrap();
        assert!(game.current_has_card_type(solidarity,ironsmith::CardType::Enchantment));
        assert!(!game.current_has_card_type(solidarity,ironsmith::CardType::Artifact));
        assert_eq!(game.player(A).unwrap().mana_pool.total(),1,"modal payment cannot be charged with token creation");
        let mut queue=ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
        assert_eq!(game.stack.len(),1,"postcreation choice is its own reflexive stack entry");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(),u32::from(!choose));
        let soldiers=game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|object|object.has_subtype(ironsmith::Subtype::Soldier))).collect::<Vec<_>>();
        assert_eq!(soldiers.len(),usize::from(choose));
        if let Some(soldier)=soldiers.first(){assert!(game.current_has_static_ability_id(*soldier,ironsmith::static_abilities::StaticAbilityId::Haste));}
    }}
}

#[test]
fn edgeland_red_mode_declares_its_target_before_resolving_the_mode_payment() {
    struct RedMode {targets:usize,payments:usize}
    impl ironsmith::decision::DecisionMaker for RedMode {
        fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
            if ctx.options.len()==3 {vec![0]} else {ironsmith::decision::DecisionMaker::decide_options(&mut SelectFirstDecisionMaker,game,ctx)}
        }
        fn decide_targets(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{
            assert_eq!(self.payments,0,"the target announcement precedes resolving payment");
            let target=ironsmith::Target::Player(B);assert!(ctx.requirements[0].legal_targets.contains(&target));self.targets+=1;vec![target]
        }
        fn decide_mana_payment(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::ManaPaymentContext)->ironsmith::mana_payment::ManaPaymentResponse{
            assert_eq!(self.targets,1);self.payments+=1;
            ironsmith::mana_payment::ManaPaymentResponse::Confirm{plan_id:ctx.plan.id,request_hash:ctx.plan.request_hash}
        }
    }
    for definition in definitions("Solidarity","Edgeland Pledge") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);game.push_to_stack(ironsmith::StackEntry::new(source,A));
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Red,1);
        let mut dm=RedMode{targets:0,payments:0};
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let mut queue=ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
        assert_eq!(game.stack.len(),1);assert_eq!(game.stack[0].targets,vec![ironsmith::Target::Player(B)]);
        assert_eq!(game.player(A).unwrap().mana_pool.red,1);assert_eq!(dm.payments,0);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().life,18);assert_eq!(game.player(A).unwrap().mana_pool.red,0);
        assert_eq!(dm.targets,1);assert_eq!(dm.payments,1);
    }
}
