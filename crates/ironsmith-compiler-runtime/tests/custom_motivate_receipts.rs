//! Motivate source/recipient receipts compose with existing counter placement.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
const A:PlayerId=PlayerId::from_index(0);
const SOURCE:&str="Mana cost: {4}{R}{R}\nType: Legendary Creature — Human Knight\nPower/Toughness: 4/2\nWhen this creature enters, you may put 3 +1/+1 counters on target creature with power less than this creature's power. If you do, perform the motivate action with creatures given counters this way.\nWhen this permanent performs the motivate action with a creature, that creature gains first strike, trample, and haste until end of turn.";
fn definitions()->[CardDefinition;2]{
    let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),"General Merthilios").with_custom_keyword("motivate");
    let (artifact,direct)=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,SOURCE,false)).unwrap();
    let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
struct Choice(bool);
impl DecisionMaker for Choice{fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}}
fn drain(g:&mut GameState,dm:&mut impl DecisionMaker){
    let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack(g,&mut queue).unwrap();
}
fn enter(g:&mut GameState,id:ObjectId){
    let effect=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),Zone::Battlefield,false));
    let outcome=ironsmith::effects::execute_effect(g,&effect,&mut ironsmith::effects::EffectContext::new_default(id,A)).unwrap();for event in outcome.events{g.queue_trigger_event(Default::default(),event);}
}
#[test]
fn motivate_receipt_selects_the_counter_recipient_and_declining_emits_no_followup(){
    for definition in definitions(){for accept in [false,true]{
        let mut g=GameState::new(vec!["A".into(),"B".into()],20);
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Small","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
        let target=g.create_object_from_definition(&body,A,Zone::Battlefield);
        let source=g.create_object_from_definition(&definition,A,Zone::Hand);enter(&mut g,source);let mut dm=Choice(accept);drain(&mut g,&mut dm);
        assert_eq!(g.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut dm).unwrap();drain(&mut g,&mut dm);
        assert_eq!(g.stack.len(),usize::from(accept),"only completed motivation generates the linked followup");
        if accept{ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut dm).unwrap();}
        assert_eq!(g.calculated_power(target),Some(if accept{5}else{2}));
        for ability in [ironsmith::static_abilities::StaticAbility::first_strike(),ironsmith::static_abilities::StaticAbility::trample(),ironsmith::static_abilities::StaticAbility::haste()]{assert_eq!(g.object_has_ability(target,&ability),accept);}
        ironsmith::turn::execute_cleanup_step(&mut g);assert!(!g.object_has_ability(target,&ironsmith::static_abilities::StaticAbility::haste()));
    }}
}
#[test]
fn ordinary_counter_placement_is_not_a_motivation_receipt(){
    for definition in definitions(){
        let mut g=GameState::new(vec!["A".into(),"B".into()],20);let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);
        let effect=ironsmith::Effect::new(ironsmith::effects::PutCountersEffect::new(ironsmith::object::CounterType::PlusOnePlusOne,1,ironsmith::target::ChooseSpec::SpecificObject(source)));
        let outcome=ironsmith::effects::execute_effect(&mut g,&effect,&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();for event in outcome.events{g.queue_trigger_event(Default::default(),event);}drain(&mut g,&mut SelectFirstDecisionMaker);assert!(g.stack.is_empty());
    }
}
#[test]
fn fully_prevented_counter_placement_has_no_motivate_recipient(){
    for definition in definitions(){
        let mut g=GameState::new(vec!["A".into(),"B".into()],20);
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Cannot receive counters","Type: Creature\nPower/Toughness: 2/2\nCounters can't be put on this creature.",false).unwrap();
        let target=g.create_object_from_definition(&body,A,Zone::Battlefield);
        let source=g.create_object_from_definition(&definition,A,Zone::Hand);enter(&mut g,source);let mut dm=Choice(true);drain(&mut g,&mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut dm).unwrap();drain(&mut g,&mut dm);
        assert!(g.stack.is_empty(),"counter placement produced no recipient to motivate");assert_eq!(g.calculated_power(target),Some(2));assert!(!g.object_has_ability(target,&ironsmith::static_abilities::StaticAbility::haste()));
    }
}
