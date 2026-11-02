//! Exact source programs and generic player-operation policies, through independent routes.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith_core::value_model::PlayerActionWindow;
const A: PlayerId=PlayerId(0);
const B: PlayerId=PlayerId(1);
fn fixture()->serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-favored-indulgence-complete.json")).unwrap()
}
fn source(name:&str)->[CardDefinition;2] {
    let fixture=fixture();
    let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
    source_support::definitions(card).unwrap_or_else(|error|panic!("{name}: {error}"))
}
fn simple(name:&str,text:&str)->CardDefinition {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap().into_iter().next().unwrap()
}
fn move_to(game:&mut GameState,source:ObjectId,controller:PlayerId,object:ObjectId,zone:Zone) {
    execute_effect(game,&Effect::move_to_zone(ChooseSpec::SpecificObject(object),zone,false),&mut EffectContext::new_default(source,controller)).unwrap();
}
fn phase(game:&mut GameState,player:PlayerId,combat:bool)->TriggerQueue {
    game.turn.phase=if combat{ironsmith::Phase::Combat}else{ironsmith::Phase::Ending};
    game.turn.step=Some(if combat{ironsmith::Step::BeginCombat}else{ironsmith::Step::End});
    let event=if combat {TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfCombatEvent::new(player),Default::default())}
        else {TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfEndStepEvent::new(player),Default::default())};
    let event=game.ensure_trigger_event_provenance(event);
    game.queue_trigger_event(event.provenance(),event);
    let mut queue=TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
    queue
}
fn resolve(game:&mut GameState,queue:&mut TriggerQueue,dm:&mut dyn DecisionMaker) {
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game,queue,dm).unwrap();
    while !game.stack_is_empty() {ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();}
}
#[test]
fn all_twenty_favored_and_twenty_two_indulgence_physical_sources_keep_complete_strict_programs() {
    let fixture=fixture();let mut physical=0;let mut faces=0;let mut failures=Vec::new();
    for row in fixture["mechanics"].as_array().unwrap() {
        assert!(matches!(row["id"].as_str(),Some("rev-favored-178"|"rev-indulgence-177")));
        let cards=row["representative_cards"].as_array().unwrap();
        assert_eq!(cards.len(),if row["keyword"]=="Favored"{20}else{22});
        for card in cards {
            physical+=1;
            let key=format!("{} / {}",row["id"],card["source_card_key"]);
            let originals=card["source_faces"].as_array().expect("exact physical faces");
            let executable=card["executable_faces"].as_array().expect("root must materialize every physical face before native tests");
            assert_eq!(originals.len(),executable.len(),"{key}");faces+=executable.len();
            match source_support::linked_definitions(card) {
                Err(error)=>failures.push(format!("{key}: {error}")),
                Ok(routes)=>for (index,[direct,artifact]) in routes.into_iter().enumerate() {
                    assert_eq!(direct.card.name,originals[index]["name"].as_str().unwrap());
                    assert_eq!(direct.abilities,artifact.abilities,"{key}");
                    assert_eq!(direct.spell_effect,artifact.spell_effect,"{key}");
                    let is_consumer=card["source_role"]=="named_reference_consumer";
                    for definition in [&direct,&artifact] {
                        let policies=definition.abilities.iter().filter(|ability|match &ability.kind {
                            AbilityKind::Static(ability)=>matches!(ability.compiled_model().map(|m|&m.payload),Some(ironsmith_core::StaticAbilityPayload::PhaseBoundaryPlayerAction{..})),_=>false,
                        }).count();
                        assert_eq!(policies,usize::from(row["keyword"]=="Indulgence"),"{key}: source-authored automatic policy");
                        if is_consumer {assert!(!definition.abilities.iter().any(|ability|match &ability.kind {
                            AbilityKind::Static(ability)=>matches!(ability.compiled_model().map(|m|&m.payload),Some(ironsmith_core::StaticAbilityPayload::CustomKeywordIdentity{name})if name.eq_ignore_ascii_case("favored")),_=>false,
                        }),"consumer-only source gained Favored");}
                    }
                }
            }
        }
    }
    assert_eq!((physical,faces),(42,42));
    assert!(failures.is_empty(),"{} exact source failures:\n{}",failures.len(),failures.join("\n\n"));
}
#[test]
fn indulgence_threshold_is_boundary_frozen_or_and_multiple_providers_emit_one_player_completion() {
    for definition in source("Lavish Host") {for gained in [0,2,3] {for drawn in [0,1,2] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let first=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let second=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let opposing=game.create_object_from_definition(&definition,B,Zone::Battlefield);
        let filler=simple("Library paper","Type: Sorcery\nYou gain 1 life.");
        for _ in 0..4 {game.create_object_from_definition(&filler,A,Zone::Library);}
        if gained>0 {execute_effect(&mut game,&Effect::gain_life(gained),&mut EffectContext::new_default(first,A)).unwrap();}
        if drawn>0 {execute_effect(&mut game,&Effect::draw(drawn),&mut EffectContext::new_default(first,A)).unwrap();}
        let mut queue=phase(&mut game,A,false);let eligible=gained>=3||drawn>=2;
        assert_eq!(queue.entries.len(),if eligible{2}else{0},"gained={gained},drawn={drawn}; policy is not an extra stack trigger");
        for entry in &queue.entries {
            let event=entry.triggering_event.downcast::<ironsmith::events::NamedActionEvent>().unwrap();
            assert!(event.player_only_performer);assert_eq!(event.player,A);assert_eq!(event.amount,1);
        }
        assert_eq!(game.player_named_action_count(A,"indulge",PlayerActionWindow::ThisTurn),u32::from(eligible));
        assert_eq!(game.player_named_action_count(B,"indulge",PlayerActionWindow::ThisTurn),0);
        assert_eq!(game.named_action_count(first,"indulge",true),0,"player-only completion cannot designate an arbitrary provider actor");
        resolve(&mut game,&mut queue,&mut SelectFirstDecisionMaker);
        for own in [first,second] {assert_eq!(game.counter_count(own,ironsmith::object::CounterType::PlusOnePlusOne),u32::from(eligible));}
        assert_eq!(game.counter_count(opposing,ironsmith::object::CounterType::PlusOnePlusOne),0);
    }}}
}
#[test]
fn an_ineligible_boundary_does_not_gain_a_late_action_and_a_captured_completion_survives_provider_removal() {
    for definition in source("Lavish Host") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let provider=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let mut queue=phase(&mut game,A,false);assert!(queue.entries.is_empty());
        execute_effect(&mut game,&Effect::gain_life(3),&mut EffectContext::new_default(provider,A)).unwrap();
        assert_eq!(game.player_named_action_count(A,"indulge",PlayerActionWindow::ThisTurn),0);
        // A distinct actual end-step occurrence is eligible; receiving three
        // life after the preceding occurrence did not retroactively qualify it.
        queue=phase(&mut game,A,false);assert_eq!(queue.entries.len(),1);
        move_to(&mut game,provider,A,provider,Zone::Graveyard);
        assert_eq!(game.player_named_action_count(A,"indulge",PlayerActionWindow::ThisTurn),1);
        resolve(&mut game,&mut queue,&mut SelectFirstDecisionMaker);
        assert_eq!(game.player_named_action_count(A,"indulge",PlayerActionWindow::ThisTurn),1);
    }
}
struct Choose { target:ObjectId, declines:usize, boolean_calls:usize }
impl DecisionMaker for Choose {
    fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool {
        self.boolean_calls+=1;
        if self.declines>0 {self.declines-=1;false}else{true}
    }
    fn decide_objects(&mut self,game:&GameState,context:&SelectObjectsContext)->Vec<ObjectId> {
        if context.candidates.iter().any(|object|object.id==self.target&&object.legal){vec![self.target]}
        else {SelectFirstDecisionMaker.decide_objects(game,context)}
    }
}
fn assert_champion(game:&GameState,id:ObjectId) {
    let view=game.current_characteristics(id).unwrap();
    assert_eq!(view.name.as_ref(),"The Champion");
    assert_eq!((game.calculated_power(id),game.calculated_toughness(id)),(Some(3),Some(4)));
    assert!(view.supertypes.contains(&ironsmith::types::Supertype::Legendary));
    assert!(view.supertypes.contains(&ironsmith::types::Supertype::Snow));
    assert!(view.card_types.contains(&ironsmith::types::CardType::Artifact));
    assert!(view.card_types.contains(&ironsmith::types::CardType::Creature));
    assert!(view.subtypes.contains(&ironsmith::types::Subtype::Bear));
    assert!(game.object(id).unwrap().counters.is_empty(),"continuous +1/+1 is not a counter");
    assert!(game.current_abilities(id).unwrap().iter().any(|ability|match &ability.kind {
        AbilityKind::Static(ability)=>ability.id()==ironsmith_core::StaticAbilityId::Vigilance,_=>false,
    }),"the name/supertype template must retain unrelated printed abilities");
}
#[test]
fn favored_shared_combat_gate_consumes_only_an_accepted_choice_and_effects_survive_the_source() {
    for definition in source("Hero in the Making") {for declines in [0,1,2] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let first=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let second=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let body=simple("Winter engine","Type: Snow Artifact Creature — Bear\nPower/Toughness: 2/3\nVigilance");
        let chosen=game.create_object_from_definition(&body,A,Zone::Battlefield);
        let other=game.create_object_from_definition(&body,A,Zone::Battlefield);
        game.mark_combat_phase_started();
        let mut queue=phase(&mut game,A,true);assert_eq!(queue.entries.len(),2);
        let mut dm=Choose{target:chosen,declines,boolean_calls:0};
        resolve(&mut game,&mut queue,&mut dm);
        let accepted=declines<2;
        assert_eq!(dm.boolean_calls,if declines==0{1}else{2},"intervening-if rechecks shared history before asking another optional choice");
        assert_eq!(game.player_named_action_count(A,"champion selection",PlayerActionWindow::ThisCombat),u32::from(accepted));
        if accepted {
            assert_champion(&game,chosen);
            move_to(&mut game,first,A,first,Zone::Graveyard);
            move_to(&mut game,second,A,second,Zone::Graveyard);
            assert_champion(&game,chosen);
            execute_effect(&mut game,&Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(ChooseSpec::SpecificObject(chosen),1,PlayerFilter::You)),&mut EffectContext::new_default(chosen,A)).unwrap();
            let token=*game.battlefield.iter().find(|id|game.object(**id).unwrap().kind==ironsmith::object::ObjectKind::Token).unwrap();
            assert_eq!(game.current_characteristics(token).unwrap().name.as_ref(),"Winter engine");
            assert_eq!(game.calculated_power(token),Some(2));
            assert!(!game.current_characteristics(token).unwrap().supertypes.contains(&ironsmith::types::Supertype::Legendary));
            let stable=game.object(chosen).unwrap().stable_id;
            move_to(&mut game,other,A,chosen,Zone::Hand);
            let returned=game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.current_characteristics(returned).unwrap().name.as_ref(),"Winter engine");
            assert_eq!(game.player_named_action_count(A,"champion selection",PlayerActionWindow::ThisCombat),1,"leaving does not grant a second choice");
        }
        game.mark_combat_phase_started();
        assert_eq!(game.player_named_action_count(A,"champion selection",PlayerActionWindow::ThisCombat),0,"extra combat resets eligibility even during the same turn");
        assert_eq!(game.player_named_action_count(A,"champion selection",PlayerActionWindow::ThisTurn),u32::from(accepted));
        game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let mut next=phase(&mut game,A,true);
        assert_eq!(next.entries.len(),if accepted{1}else{3});
        let mut choose_next=Choose{target:other,declines:0,boolean_calls:0};
        resolve(&mut game,&mut next,&mut choose_next);
        assert_eq!(choose_next.boolean_calls,1);
        assert_champion(&game,other);
        assert_eq!(game.player_named_action_count(A,"champion selection",PlayerActionWindow::ThisCombat),1);
        assert_eq!(game.player_named_action_count(A,"champion selection",PlayerActionWindow::ThisTurn),u32::from(accepted)+1);
    }}
}
#[test]
fn palace_gargoyle_reads_the_controllers_last_own_turn_not_the_previous_players_turn() {
    for definition in source("Palace Gargoyle") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let gargoyle=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.next_turn();game.next_turn();
        assert!(!ironsmith::rules::combat::can_attack(game.object(gargoyle).unwrap(),&game));
        let effect=Effect::new(ironsmith_core::PerformNamedActionEffect{label:"indulge".into(),performer:None,objects:None,prior_objects:None});
        execute_effect(&mut game,&effect,&mut EffectContext::new_default(gargoyle,A)).unwrap();
        assert_eq!(game.player_named_action_count(A,"indulge",PlayerActionWindow::LastOwnTurn),0);
        game.next_turn();
        assert_eq!(game.player_named_action_count(A,"indulge",PlayerActionWindow::LastOwnTurn),1);
        game.next_turn();
        assert!(ironsmith::rules::combat::can_attack(game.object(gargoyle).unwrap(),&game));
        assert_eq!(game.player_named_action_count(A,"indulge",PlayerActionWindow::ThisTurn),0);
        game.next_turn();game.next_turn();
        assert!(!ironsmith::rules::combat::can_attack(game.object(gargoyle).unwrap(),&game));
    }
}

#[test]
fn champion_is_a_current_controller_name_query_and_not_an_original_chooser_designation() {
    let favored=source("Hero in the Making");let grants=source("Petraform Hermit");let consumers=source("Xarr's Mockery");
    for route in 0..2 {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.create_object_from_definition(&favored[route],A,Zone::Battlefield);
        let source_a=game.create_object_from_definition(&grants[route],A,Zone::Battlefield);
        game.create_object_from_definition(&consumers[route],A,Zone::Battlefield);
        game.create_object_from_definition(&consumers[route],B,Zone::Battlefield);
        let body=simple("Winter engine","Type: Snow Artifact Creature — Bear\nPower/Toughness: 2/3\nVigilance");
        let chosen=game.create_object_from_definition(&body,A,Zone::Battlefield);
        game.mark_combat_phase_started();let mut queue=phase(&mut game,A,true);
        resolve(&mut game,&mut queue,&mut Choose{target:chosen,declines:0,boolean_calls:0});
        let deathtouch=|game:&GameState|game.current_abilities(chosen).unwrap().iter().any(|ability|match &ability.kind {
            AbilityKind::Static(ability)=>ability.id()==ironsmith_core::StaticAbilityId::Deathtouch,_=>false,
        });
        assert!(deathtouch(&game));assert_champion(&game,chosen);
        execute_effect(&mut game,&Effect::gain_control_with_duration(ChooseSpec::SpecificObject(chosen),ironsmith::Until::Forever),&mut EffectContext::new_default(source_a,B)).unwrap();
        assert_eq!(game.current_controller(chosen),Some(B));assert_champion(&game,chosen);
        assert!(!deathtouch(&game),"former controller's name query must stop granting");
        game.create_object_from_definition(&grants[route],B,Zone::Battlefield);
        assert!(deathtouch(&game),"new controller's ordinary named filter must grant");
        let attack=TriggerEvent::new_with_provenance(ironsmith::events::combat::CreatureAttackedEvent::new(chosen,ironsmith::triggers::AttackEventTarget::Player(A)),Default::default());
        let observed=ironsmith::triggers::check_triggers(&game,&attack);
        assert_eq!(observed.len(),1,"only the current controller's consumer observes this named permanent attacking");
        assert_eq!(observed[0].controller,B);
    }
}

#[test]
fn generic_boundary_policy_groups_identical_instructions_but_keeps_distinct_programs_and_player_roles() {
    let text="Type: Enchantment\nAs your combat begins, if you gained 1 or more life this turn, you perform the rejoice action once rather than once for each permanent with this ability.\nAs your combat begins, if you gained 2 or more life this turn, you perform the rejoice action once rather than once for each permanent with this ability.";
    for definition in source_support::definitions(&serde_json::json!({"name":"Boundary instructions","executable_text":text})).unwrap() {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let first=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let observer=simple("Action witness","Type: Artifact\nWhenever you perform the rejoice action, you gain 1 life.\nWhenever this artifact performs the rejoice action, you gain 100 life.");
        game.create_object_from_definition(&observer,A,Zone::Battlefield);
        execute_effect(&mut game,&Effect::gain_life(2),&mut EffectContext::new_default(first,A)).unwrap();
        game.mark_combat_phase_started();let mut queue=phase(&mut game,A,true);
        assert_eq!(queue.entries.len(),2,"two distinct source-authored programs, each reduced across its duplicate providers");
        assert_eq!(game.player_named_action_count(A,"rejoice",PlayerActionWindow::ThisCombat),2);
        resolve(&mut game,&mut queue,&mut SelectFirstDecisionMaker);
        assert_eq!(game.player(A).unwrap().life,24,"player-only operation must not act through the attribution provider as an object performer");
    }
}

#[test]
fn favored_with_no_eligible_creature_does_not_consume_selection_or_choose_an_opponents_creature() {
    for definition in source("Invest in the Future") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let own=simple("Old legend","Type: Legendary Creature — Bear\nPower/Toughness: 2/3");
        let own=game.create_object_from_definition(&own,A,Zone::Battlefield);
        let other=simple("Other bear","Type: Creature — Bear\nPower/Toughness: 2/3");
        let other=game.create_object_from_definition(&other,B,Zone::Battlefield);
        game.mark_combat_phase_started();let mut queue=phase(&mut game,A,true);
        assert_eq!(queue.entries.len(),1);
        resolve(&mut game,&mut queue,&mut Choose{target:other,declines:0,boolean_calls:0});
        assert_eq!(game.player_named_action_count(A,"champion selection",PlayerActionWindow::ThisCombat),0);
        assert_eq!(game.current_characteristics(own).unwrap().name.as_ref(),"Old legend");
        assert_eq!(game.current_characteristics(other).unwrap().name.as_ref(),"Other bear");
    }
}
