//! Reminder-derived Gorge candidates: optional existing costs and effects.
//! Authored complete source reminders supply authority; numbered heading is empty.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith_compiler_runtime::compile_builder_to_runtime_definition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
fn definition(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name).with_custom_keyword("Gorge");
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || compile_builder_to_runtime_definition(builder, text, false)).unwrap()
}
struct Payment { accept: bool, mode: usize }
impl DecisionMaker for Payment {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        ctx.options.iter().find(|option| option.index == self.mode && option.legal).map(|option| vec![option.index])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(game, ctx))
    }
}
fn enter(game: &mut GameState, card: &ironsmith::cards::CardDefinition) -> ObjectId {
    let a = PlayerId::from_index(0); let hand = game.create_object_from_definition(card, a, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    let effect = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(hand), Zone::Battlefield, false));
    let mut ctx = ironsmith::effects::EffectContext::new_default(hand, a);
    let result = ironsmith::effects::execute_effect(game, &effect, &mut ctx).unwrap();
    for event in result.events { game.queue_trigger_event(Default::default(), event); }
    game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn gorge_reminder_candidate_requires_nonland_discard_or_another_creature_before_reward() {
    for quoted in [false,true] {
    let offer=if quoted {"you may pay the cost \"Discard a nonland card or sacrifice a creature other than this creature\""}
        else {"you may discard a nonland card or sacrifice another creature"};
    let card = definition("Ravager", &format!("Type: Creature\nPower/Toughness: 2/2\nWhen this creature enters, {offer}. If you do, put a +1/+1 counter on this creature and draw a card."));
    for mode in [0, 1] { for available in [false, true] { for accept in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let land = game.create_object_from_definition(&definition("Land", "Type: Land"), a, Zone::Hand);
        let nonland = (available && mode == 0).then(|| game.create_object_from_definition(&definition("Discard", "Type: Artifact"), a, Zone::Hand));
        let fodder = (available && mode == 1).then(|| game.create_object_from_definition(&definition("Fodder", "Type: Creature\nPower/Toughness: 1/1"), a, Zone::Battlefield));
        let opponent = game.create_object_from_definition(&definition("Opponent", "Type: Creature\nPower/Toughness: 1/1"), b, Zone::Battlefield);
        game.create_object_from_definition(&definition("Drawn", "Type: Artifact"), a, Zone::Library);
        let source = enter(&mut game, &card);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Payment { accept, mode }).unwrap();
        let paid = available && accept;
        assert_eq!(game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne), u32::from(paid), "mode={mode} available={available} accept={accept}");
        assert_eq!(game.player(a).unwrap().library.len(), usize::from(!paid));
        assert_eq!(game.object(land).unwrap().zone, Zone::Hand, "a land is not a legal discard payment");
        assert!(game.battlefield.contains(&source), "another creature excludes the gorging creature");
        assert!(game.battlefield.contains(&opponent), "the payer cannot sacrifice an opponent's creature");
        if let Some(card) = nonland { assert_eq!(game.player(a).unwrap().hand.contains(&card), !paid); assert_eq!(game.player(a).unwrap().graveyard.iter().any(|id| game.object(*id).unwrap().name == "Discard"), paid); }
        if let Some(creature) = fodder { assert_eq!(game.battlefield.contains(&creature), !paid); assert_eq!(game.player(a).unwrap().graveyard.iter().any(|id| game.object(*id).unwrap().name == "Fodder"), paid); }
    } } }
    }
}

#[test]
fn gorge_group_candidate_keeps_optional_payments_and_rewards_inside_each_recipient() {
    use ironsmith::effects::{EffectContext, ResolvedTarget};
    struct Each { answers: Vec<bool>, next: usize }
    impl DecisionMaker for Each {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            let answer = self.answers.get(self.next).copied().unwrap_or(false); self.next += 1; answer
        }
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options.iter().find(|option| option.legal && option.description.to_lowercase().contains("discard"))
                .map(|option| vec![option.index]).unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(game, ctx))
        }
    }
    let program = definition("Group meal", "Type: Sorcery\nMana cost: {0}\nChoose up to two target creatures you control. For each creature chosen this way, you may discard a nonland card or sacrifice a creature other than that creature. If you do, put a +1/+1 counter on that creature and draw a card.");
    for count in [0, 2] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20); let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&program, a, Zone::Stack);
        let creature = definition("Recipient", "Type: Creature\nPower/Toughness: 2/2");
        let first = game.create_object_from_definition(&creature, a, Zone::Battlefield);
        let second = game.create_object_from_definition(&creature, a, Zone::Battlefield);
        for (name, body) in [("Payment one", "Type: Creature\nPower/Toughness: 2/2"), ("Payment two", "Type: Artifact")] { game.create_object_from_definition(&definition(name, body), a, Zone::Hand); }
        for name in ["Draw one", "Draw two"] { game.create_object_from_definition(&definition(name, "Type: Artifact"), a, Zone::Library); }
        let mut dm = Each { answers: vec![true, false], next: 0 };
        let targets = [first, second].into_iter().take(count).map(ResolvedTarget::Object).collect();
        let mut ctx = EffectContext::new(source, a, &mut dm).with_targets(targets);
        for effect in program.spell_effect.as_ref().unwrap().all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        drop(ctx);
        assert_eq!(game.counter_count(first, ironsmith::object::CounterType::PlusOnePlusOne), u32::from(count == 2));
        assert_eq!(game.counter_count(second, ironsmith::object::CounterType::PlusOnePlusOne), 0, "the declined second recipient cannot inherit the first payment's receipt");
        assert_eq!(game.player(a).unwrap().library.len(), if count == 0 { 2 } else { 1 });
        assert_eq!(game.player(a).unwrap().graveyard.len(), usize::from(count == 2));
        assert_eq!(dm.next, count, "zero recipients must not offer an optional payment");
    }
}

fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-gorge.json")).unwrap()}
#[test]
fn all_fifteen_gorge_sources_and_every_linked_face_compile_independently_without_loss(){
 let fixture=fixture();let mut count=0;let mut failures=vec![];
 for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){
  match source_support::linked_definitions(card){
   Err(error)=>failures.push(format!("{}: {error}",card["name"])),
   Ok(faces)=>for [native,artifact] in faces {
    let typed=|definition:ironsmith::CardDefinition|->Result<serde_json::Value,String>{
     let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error|format!("{error:?}"))?;
     serde_json::to_value(wire).map_err(|error|error.to_string())
    };
    match(typed(native),typed(artifact)){
     (Ok(left),Ok(right)) if left!=right=>failures.push(format!("{}: complete typed native/artifact source differs",card["name"])),
     (Err(error),_)|(_,Err(error))=>failures.push(format!("{}: typed source encoding {error}",card["name"])),
     _=>{},
    }
   },
  }count+=1;
 }}assert_eq!(count,15);assert!(failures.is_empty(),"{}",failures.join("\n"));
}
#[test]
fn exact_sanguine_source_reflexive_uses_completed_payment_and_post_counter_power(){
 let fixture=fixture();let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]=="Sanguine Liturgy").unwrap();
 struct Choices {accept:bool, victim:ObjectId, target_offers:usize}
 impl DecisionMaker for Choices {
  fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{self.accept}
  fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{c.options.iter().find(|o|o.legal&&o.description.to_lowercase().contains("discard")).map(|o|vec![o.index]).unwrap_or_else(||SelectFirstDecisionMaker.decide_options(g,c))}
  fn decide_targets(&mut self,_:&GameState,c:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{self.target_offers+=1;let victim=ironsmith::Target::Object(self.victim);assert!(c.requirements[0].legal_targets.contains(&victim),"counter must increase recipient power before the reflexive chooses its targets");vec![victim]}
 }
 for definition in source_support::definitions(card).unwrap(){for accept in [false,true]{
  let a=PlayerId(0);let b=PlayerId(1);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&definition,a,Zone::Stack);
  let recipient=game.create_object_from_definition(&super_definition("Recipient","Type: Creature\nPower/Toughness: 2/2"),a,Zone::Battlefield);
  let victim=game.create_object_from_definition(&super_definition("Victim","Type: Artifact\nMana cost: {3}"),b,Zone::Battlefield);
  game.create_object_from_definition(&super_definition("Payment","Type: Artifact"),a,Zone::Hand);game.create_object_from_definition(&super_definition("Draw","Type: Land"),a,Zone::Library);
  let mut dm=Choices{accept,victim,target_offers:0};let mut ctx=ironsmith::effects::EffectContext::new_default(source,a).with_decision_maker(&mut dm).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(recipient)]);
  let program=ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned()));
  ironsmith::effects::execute_effect(&mut game,&program,&mut ctx).unwrap();drop(ctx);
  assert_eq!(dm.target_offers,0,"reflexive targets are not chosen during original spell resolution");assert_eq!(game.counter_count(recipient,ironsmith::CounterType::PlusOnePlusOne),u32::from(accept));assert!(game.battlefield.contains(&victim));
  let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(queue.entries.len(),usize::from(accept));
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(dm.target_offers,usize::from(accept));
  if accept {ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert!(!game.battlefield.contains(&victim));}
 }}
}
fn super_definition(name:&str,text:&str)->ironsmith::CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
#[test]
fn exact_markov_effect_keeps_one_recipient_for_independent_repeat_payments(){
 let fixture=fixture();let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]=="Markov Imperator").unwrap();
 struct Choices {answers:[bool;2],next:usize}
 impl DecisionMaker for Choices {
  fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{let answer=self.answers[self.next];self.next+=1;answer}
  fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{c.options.iter().find(|o|o.legal&&o.description.to_lowercase().contains("discard")).map(|o|vec![o.index]).unwrap_or_else(||SelectFirstDecisionMaker.decide_options(g,c))}
 }
 for definition in source_support::definitions(card).unwrap(){for answers in [[true,true],[true,false],[false,true],[false,false]]{
  let a=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,a,Zone::Graveyard);let recipient=game.create_object_from_definition(&super_definition("Recipient","Type: Creature\nPower/Toughness: 2/2"),a,Zone::Battlefield);
  for name in ["First payment","Second payment"]{game.create_object_from_definition(&super_definition(name,"Type: Artifact"),a,Zone::Hand);}for name in ["First draw","Second draw"]{game.create_object_from_definition(&super_definition(name,"Type: Land"),a,Zone::Library);}
  let program=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Activated(ability)=>Some(ability.effects.all_effects_owned()),_=>None}).unwrap();let mut dm=Choices{answers,next:0};let mut ctx=ironsmith::effects::EffectContext::new_default(source,a).with_decision_maker(&mut dm).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(recipient)]);
  ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(program)),&mut ctx).unwrap();drop(ctx);let paid=answers.into_iter().filter(|answer|*answer).count();assert_eq!(dm.next,2);assert_eq!(game.counter_count(recipient,ironsmith::CounterType::PlusOnePlusOne),paid as u32);assert_eq!(game.player(a).unwrap().library.len(),2-paid);
 }}
}

#[test]
fn quoted_alternative_payment_does_not_hide_an_unknown_action_tail(){
 let input=serde_json::json!({"name":"Unknown price tail","executable_text":"Type: Sorcery\nYou may pay the cost \"Discard a nonland card or invent a creature\". If you do, draw a card."});
 assert!(source_support::definitions(&input).is_err(),"an unread alternative cost cannot become a paid discard-only program");
}
