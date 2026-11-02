//! Self player actions retain the same existing unless-payment semantics.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Self action", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn self_player_payment_does_not_use_opponents_resources_or_sacrifice_lands() {
    for definition in definitions("Type: Sorcery\nYou lose 3 life unless you discard a card or sacrifice a nonland permanent.") {
        for resource in ["none", "card", "permanent", "land"] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Reserve", "Type: Creature\nPower/Toughness: 1/1", false).unwrap();
            let opponent_card = game.create_object_from_definition(&body, B, Zone::Hand);
            let opponent_body = game.create_object_from_definition(&body, B, Zone::Battlefield);
            let payment = match resource {
                "card" => Some(game.create_object_from_definition(&body, A, Zone::Hand)),
                "permanent" => Some(game.create_object_from_definition(&body, A, Zone::Battlefield)),
                "land" => {
                    let land = ironsmith_compiler_runtime::compile_to_runtime_definition("Field", "Type: Land", false).unwrap();
                    Some(game.create_object_from_definition(&land, A, Zone::Battlefield))
                },
                _ => None,
            };
            let source = game.create_object_from_definition(&definition, A, Zone::Stack);
            let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
            execute_effect(&mut game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_decision_maker(&mut SelectFirstDecisionMaker)).unwrap();
            assert_eq!(game.player(A).unwrap().life, if matches!(resource, "card"|"permanent") {20} else {17}, "resource={resource}");
            assert_eq!(game.player(B).unwrap().life, 20);
            assert!(game.player(B).unwrap().hand.contains(&opponent_card));
            assert!(game.battlefield.contains(&opponent_body));
            if resource == "land" {assert!(game.battlefield.contains(&payment.unwrap()));}
        }
    }
}

#[test]
fn bounty_granted_death_trigger_is_owned_by_the_marked_permanent_and_counter_gated() {
    use ironsmith::effects::ResolvedTarget;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::game_loop::{drain_pending_trigger_events_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
    const TEXT: &str = "Type: Sorcery\nChoose target permanent. Put a bounty counter on that permanent. For as long as that permanent has a bounty counter on it, that permanent has \"When this permanent is put into a graveyard from the battlefield, each opponent draws a card.\"";
    for definition in definitions(TEXT) {for keep_counter in [false,true] {
        let mut game = GameState::new(vec!["A".into(),"B".into(),"C".into()],20);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Marked Body", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let marked = game.create_object_from_definition(&body, B, Zone::Battlefield);
        for player in [A,B,PlayerId(2)] {game.create_object_from_definition(&body,player,Zone::Library);}
        let source = game.create_object_from_definition(&definition,A,Zone::Stack);
        let mut ctx = EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(marked)]);
        let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ctx).unwrap();
        if !keep_counter {game.object_mut(marked).unwrap().counters.clear();}
        game.move_object(marked,Zone::Graveyard,ironsmith::events::EventCause::effect()).unwrap();
        let mut queue=TriggerQueue::new();
        drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
        put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
        while !game.stack.is_empty() {resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
        assert_eq!(game.player(A).unwrap().hand.len(),usize::from(keep_counter));
        assert_eq!(game.player(B).unwrap().hand.len(),0,"opponents are relative to the marked permanent's controller");
        assert_eq!(game.player(PlayerId(2)).unwrap().hand.len(),usize::from(keep_counter));
    }}
}

#[derive(Default)]
struct CounterChoices { sizes: Vec<usize> }
impl ironsmith::decision::DecisionMaker for CounterChoices {
    fn decide_options(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        self.sizes.push(ctx.options.len());
        ctx.options.iter().find(|o|o.legal).map(|o| vec![o.index]).unwrap_or_default()
    }
}
#[test]
fn chosen_counter_menu_excludes_present_kinds_reopens_removed_kinds_and_obeys_prevention() {
    use ironsmith::effects::ResolvedTarget;
    use ironsmith::object::CounterType;
    use ironsmith::target::{ChooseSpec,ObjectFilter};
    const TEXT:&str="Type: Sorcery\nChoose a kind of counter that target creature doesn't have on it from among flying, lifelink, and +1/+1. Put a counter of that kind on it.";
    for definition in definitions(TEXT) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
        let recipient=game.create_object_from_definition(&body,A,Zone::Battlefield);
        let bystander=game.create_object_from_definition(&body,B,Zone::Battlefield);
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);
        let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
        let program=Effect::new(ironsmith::effects::SequenceEffect::new(effects));
        let mut dm=CounterChoices::default();
        for count in 1..=4 {
            let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm);
            execute_effect(&mut game,&program,&mut ctx).unwrap();
            let total:u32=game.object(recipient).unwrap().counters.values().sum();
            assert_eq!(total,count.min(3));
        }
        assert_eq!(dm.sizes,vec![3,2,1],"an exhausted menu asks no impossible choice");
        assert!(game.object(bystander).unwrap().counters.is_empty());
        game.object_mut(recipient).unwrap().remove_counters(CounterType::Flying,1);
        execute_effect(&mut game,&program,&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
        assert_eq!(game.counter_count(recipient,CounterType::Flying),1);
        assert_eq!(dm.sizes,vec![3,2,1,1]);
        game.object_mut(recipient).unwrap().remove_counters(CounterType::Flying,1);
        execute_effect(&mut game,&Effect::new(ironsmith::effects::CantEffect::new(ironsmith::effect::Restriction::have_counters_placed(ObjectFilter::specific(recipient)),ironsmith::effect::Until::EndOfTurn)),&mut EffectContext::new_default(source,A)).unwrap();
        game.update_cant_effects();
        execute_effect(&mut game,&program,&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
        assert_eq!(game.counter_count(recipient,CounterType::Flying),0,"counter placement uses the existing prevention/restriction path");
        let _ = ChooseSpec::SpecificObject(recipient);
    }
}

#[test]
fn prowess_counter_grants_keyword_and_loses_it_when_the_last_counter_is_removed() {
    use ironsmith::object::CounterType;
    use ironsmith::static_abilities::StaticAbilityId;
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Prowess recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
    let recipient=game.create_object_from_definition(&body,A,Zone::Battlefield);
    let kind=CounterType::Named("prowess".into());
    game.add_counters(recipient,kind,2).unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(game.calculated_characteristics(recipient).unwrap().static_abilities.iter().any(|a|a.id()==StaticAbilityId::Prowess));
    game.object_mut(recipient).unwrap().remove_counters(kind,1);
    game.refresh_continuous_state().unwrap();
    assert!(game.calculated_characteristics(recipient).unwrap().static_abilities.iter().any(|a|a.id()==StaticAbilityId::Prowess));
    game.object_mut(recipient).unwrap().remove_counters(kind,1);
    game.refresh_continuous_state().unwrap();
    assert!(!game.calculated_characteristics(recipient).unwrap().static_abilities.iter().any(|a|a.id()==StaticAbilityId::Prowess));
}

#[test]
fn fixed_counter_menu_repeats_choice_independently_for_each_matching_creature() {
    use ironsmith::object::CounterType;
    const TEXT:&str="Type: Sorcery\nFor each creature you control, choose a kind of counter that that creature doesn't have on it from among flying and lifelink. Put a counter of that kind on that creature.";
    for definition in definitions(TEXT) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
        let first=game.create_object_from_definition(&body,A,Zone::Battlefield);
        let second=game.create_object_from_definition(&body,A,Zone::Battlefield);
        let enemy=game.create_object_from_definition(&body,B,Zone::Battlefield);
        game.add_counters(first,CounterType::Flying,1).unwrap();
        game.add_counters(second,CounterType::Lifelink,1).unwrap();
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);
        let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
        let mut dm=CounterChoices::default();
        execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
        assert_eq!(dm.sizes,vec![1,1]);
        for id in [first,second] {
            assert_eq!(game.counter_count(id,CounterType::Flying),1);
            assert_eq!(game.counter_count(id,CounterType::Lifelink),1);
        }
        assert!(game.object(enemy).unwrap().counters.is_empty());
    }
}


mod custom_final_next_wave_support;
struct UpgradePerformer { recipient: ObjectId, object_choices: usize, kind_choices: usize }
impl ironsmith::decision::DecisionMaker for UpgradePerformer {
 fn decide_objects(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
  assert_eq!(ctx.player,B,"enchanted player chooses the source controller's creature");
  assert!(ctx.candidates.iter().filter(|o|o.legal).any(|o|o.id==self.recipient));
  self.object_choices+=1;vec![self.recipient]
 }
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
  assert_eq!(ctx.player,B,"the same performer chooses the absent counter kind");
  self.kind_choices+=1;ctx.options.iter().find(|o|o.legal).map(|o|vec![o.index]).unwrap_or_default()
 }
}
#[test]
fn actual_upgrade_curse_keeps_enchanted_performer_and_source_controller_distinct(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-upgrade-companion.json")).unwrap();
 let source=&fixture["mechanics"][0]["representative_cards"][0];
 assert!(source["original_text"].as_str().unwrap().contains("that player upgrades a creature you control"));
 let faces=custom_final_next_wave_support::linked_definitions(source).unwrap();
 let pair=faces.into_iter().find(|pair|pair[0].name=="Curse of Rebalancing").expect("actual back face retained");
 for definition in pair{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Upgrade recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
  let recipient=game.create_object_from_definition(&body,A,Zone::Battlefield);
  let enemy=game.create_object_from_definition(&body,B,Zone::Battlefield);
  let curse=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  game.object_mut(curse).unwrap().attached_to=Some(ironsmith::object::AttachmentTarget::Player(B));
  let mut dm=UpgradePerformer{recipient,object_choices:0,kind_choices:0};
  for expected in 1..=4{
   let event=ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(B),Default::default());
   game.queue_trigger_event(event.provenance(),event);
   let mut queue=ironsmith::triggers::TriggerQueue::new();
   ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
   ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
   assert_eq!(game.stack.len(),1);
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
   assert_eq!(game.object(recipient).unwrap().counters.values().sum::<u32>(),expected.min(3));
   assert!(game.object(enemy).unwrap().counters.is_empty());
  }
  assert_eq!(dm.object_choices,4);assert_eq!(dm.kind_choices,3,"exhausted menu makes no kind choice");
 }
}
#[test]
fn every_existing_upgrade_source_compiles_independently_on_both_routes(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-action-status-complete.json")).unwrap();
 let mut seen=std::collections::HashSet::new();
 for row in fixture["mechanics"].as_array().unwrap().iter().filter(|row|matches!(row["id"].as_str(),Some("msem-365"|"msem-366"|"msem-367"))){
  for card in row["representative_cards"].as_array().unwrap(){
   if seen.insert(card["source_card_key"].as_str().unwrap_or_else(||card["name"].as_str().unwrap()).to_string()){
    custom_final_next_wave_support::linked_definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"]));
   }
  }
 }
 assert!(seen.len()>=30,"Upgrade whole-source collection must not shrink to representatives");
}


#[derive(Default)]
struct UpgradeReplacementChoices { accept:bool, offers:Vec<PlayerId>, counter_menus:Vec<PlayerId> }
impl ironsmith::decision::DecisionMaker for UpgradeReplacementChoices {
 fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool {self.offers.push(ctx.player);self.accept}
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
  self.counter_menus.push(ctx.player);ctx.options.iter().find(|option|option.legal).map(|option|vec![option.index]).unwrap_or_default()
 }
}
fn actual_affluent_definitions()->[CardDefinition;2] {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-middle-whole-source-input.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap())
  .find(|card|card["source_card_key"]=="Affluent Cetanthrope_AKD").unwrap();
 assert!(card["original_text"].as_str().unwrap().contains("If you would upgrade a creature you control, you may put a +1/+1 counter on it instead."));
 custom_final_next_wave_support::definitions(card).unwrap()
}
const UPGRADE_ENVELOPE:&str="Type: Sorcery\nPerform the upgrade action on target creature as follows, completing the action only if all choices were available: [Choose a kind of counter that the proposed objects doesn't have on it from among flying and lifelink; Put a counter of that kind on the proposed objects].";
#[test]
fn affluent_replaces_the_whole_upgrade_before_counter_choice_and_exports_no_upgrade_completion() {
 use ironsmith::object::CounterType;
 for (route,affluent_definition) in actual_affluent_definitions().into_iter().enumerate() {
  let operation_definition=definitions(UPGRADE_ENVELOPE)[route].clone();
  for (accept,exhausted,prevent_counter) in [(true,false,false),(false,false,false),(true,true,false),(false,true,false),(true,false,true),(false,false,true)] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let affluent=game.create_object_from_definition(&affluent_definition,A,Zone::Battlefield);
   let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
   let recipient=game.create_object_from_definition(&body,A,Zone::Battlefield);
   if exhausted {game.add_counters(recipient,CounterType::Flying,1).unwrap();game.add_counters(recipient,CounterType::Lifelink,1).unwrap();}
   let source=game.create_object_from_definition(&operation_definition,A,Zone::Stack);
   if prevent_counter {execute_effect(&mut game,&Effect::new(ironsmith::effects::CantEffect::new(ironsmith::effect::Restriction::have_counters_placed(ironsmith::target::ObjectFilter::specific(recipient)),ironsmith::effect::Until::EndOfTurn)),&mut EffectContext::new_default(source,A)).unwrap();game.update_cant_effects();}
   let program=Effect::new(ironsmith::effects::SequenceEffect::new(operation_definition.spell_effect.as_ref().unwrap().all_effects_owned()));
   let mut dm=UpgradeReplacementChoices{accept,..Default::default()};
   let outcome=execute_effect(&mut game,&program,&mut EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
   assert_eq!(dm.offers,vec![A],"the replacement belongs to Affluent's controller even for an exhausted menu");
   assert_eq!(dm.counter_menus.len(),usize::from(!accept&&!exhausted),"acceptance bypasses the counter-kind decision itself");
   assert_eq!(game.counter_count(recipient,CounterType::PlusOnePlusOne),u32::from(accept&&!prevent_counter));
   assert_eq!(game.counter_count(recipient,CounterType::Flying),u32::from(exhausted||(!accept&&!prevent_counter)));
   assert!(game.object(affluent).unwrap().counters.is_empty(),"replacement it is the proposed recipient, not the performer or provider");
   let receipts=outcome.events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).filter(|event|event.label=="upgrade").collect::<Vec<_>>();
   assert_eq!(receipts.len(),usize::from(!accept&&!exhausted),"unavailable choices and replacement do not complete the original operation even if the replacement counter is prevented");
   if let Some(receipt)=receipts.first(){assert_eq!(receipt.source,source);assert_eq!(receipt.objects[0].object_id,recipient);}
  }
 }
}
#[test]
fn affluent_matches_both_the_operation_player_and_the_controlled_recipient() {
 use ironsmith::object::CounterType;
 for (route,affluent_definition) in actual_affluent_definitions().into_iter().enumerate(){
  let operation_definition=definitions(UPGRADE_ENVELOPE)[route].clone();
  for (performer,recipient_controller) in [(A,B),(B,A)] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   game.create_object_from_definition(&affluent_definition,A,Zone::Battlefield);
   let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
   let recipient=game.create_object_from_definition(&body,recipient_controller,Zone::Battlefield);
   let source=game.create_object_from_definition(&operation_definition,performer,Zone::Stack);
   let program=Effect::new(ironsmith::effects::SequenceEffect::new(operation_definition.spell_effect.as_ref().unwrap().all_effects_owned()));
   let mut dm=UpgradeReplacementChoices{accept:true,..Default::default()};
   execute_effect(&mut game,&program,&mut EffectContext::new_default(source,performer).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
   assert!(dm.offers.is_empty());assert_eq!(dm.counter_menus,vec![performer]);
   assert_eq!(game.counter_count(recipient,CounterType::PlusOnePlusOne),0);assert_eq!(game.counter_count(recipient,CounterType::Flying),1);
  }
 }
}
#[test]
fn explicit_other_player_operation_uses_the_retained_player_for_proposal_and_counter_menu() {
 use ironsmith::object::CounterType;
 const TEXT:&str="Type: Sorcery\nTarget opponent draws a card. That player performs the upgrade action on target creature as follows, completing the action only if all choices were available: [Choose a kind of counter that the proposed objects doesn't have on it from among flying and lifelink; Put a counter of that kind on the proposed objects].";
 for (route,affluent_definition) in actual_affluent_definitions().into_iter().enumerate(){
  let operation_definition=definitions(TEXT)[route].clone();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&affluent_definition,A,Zone::Battlefield);
  let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
  let recipient=game.create_object_from_definition(&body,A,Zone::Battlefield);
  game.create_object_from_definition(&body,B,Zone::Library);
  let source=game.create_object_from_definition(&operation_definition,A,Zone::Stack);
  let program=Effect::new(ironsmith::effects::SequenceEffect::new(operation_definition.spell_effect.as_ref().unwrap().all_effects_owned()));
  let mut dm=UpgradeReplacementChoices{accept:true,..Default::default()};
  let outcome=execute_effect(&mut game,&program,&mut EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Player(B),ironsmith::effects::ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
  assert!(dm.offers.is_empty(),"Affluent's you condition cannot match the other operation player");
  assert_eq!(dm.counter_menus,vec![B]);assert_eq!(game.counter_count(recipient,CounterType::Flying),1);
  let receipt=outcome.events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).find(|event|event.label=="upgrade").unwrap();
  assert_eq!(receipt.player,B);assert_eq!(receipt.source,source);assert_eq!(receipt.objects[0].object_id,recipient);
 }
}
