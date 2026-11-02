//! A paid total has its own acknowledgement, independent of empty physical leaves.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{Effect,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectExecutor,MayEffect};
use ironsmith::effect::EffectPredicateRuntimeExt;
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
const A:PlayerId=PlayerId(0);
// SelectFirst deliberately selects a nonempty legal maximum. This test's zero
// payment scenario instead explicitly accepts while choosing zero resources.
struct PaymentChoices { choose_empty:bool }
impl DecisionMaker for PaymentChoices {
 fn decide_boolean(&mut self,g:&GameState,c:&ironsmith::decisions::context::BooleanContext)->bool {SelectFirstDecisionMaker.decide_boolean(g,c)}
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{SelectFirstDecisionMaker.decide_options(g,c)}
 fn decide_number(&mut self,g:&GameState,c:&ironsmith::decisions::context::NumberContext)->u32{SelectFirstDecisionMaker.decide_number(g,c)}
 fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{
  if self.choose_empty && c.min==0 {Vec::new()} else {SelectFirstDecisionMaker.decide_objects(g,c)}
 }
}

fn creature(mana:&str)->ironsmith::CardDefinition{
    ironsmith_compiler_runtime::compile_to_runtime_definition("Resource",&format!("Mana cost: {mana}\nType: Creature\nPower/Toughness: 2/2"),false).unwrap()
}
fn optional_owner(effect:&Effect)->MayEffect{
    let mut effect=effect;
    while let Some(child)=effect.transparent_child_effect(){effect=child;}
    effect.downcast_ref::<MayEffect>().expect("source compiles a typed optional cost owner").clone()
}
#[test]
fn optional_payment_acknowledges_zero_and_empty_terminal_leaves_in_ordinary_owner_and_composition(){
    for count in ["two","any number of"]{
        let text=format!("Type: Sorcery\nYou may pay the cost \"Choose {count} untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\". If you do, gain 1 life.");
        let definitions=source_support::definitions(&serde_json::json!({"name":"Optional cost receipt","executable_text":text})).unwrap();
        for definition in definitions{for composed in [false,true]{
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            let source=game.create_object_from_definition(&definition,A,Zone::Stack);
            let white=game.create_object_from_definition(&creature("{W}"),A,Zone::Battlefield);
            let blue=game.create_object_from_definition(&creature("{U}"),A,Zone::Battlefield);
            let effect=optional_owner(&definition.spell_effect.as_ref().unwrap().all_effects_owned()[0]);
            let mut dm=PaymentChoices{choose_empty:count=="any number of"};
            let mut ctx=ironsmith::effects::EffectContext::new_default(source,A).with_decision_maker(&mut dm);
            ctx.x_value=Some(99);
            let outcome=if composed{
                // Let ordinary dispatch negotiate the owner; a raw May proposal
                // is not supported when its complete price is not preparable.
                let program=Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned()));
                ironsmith::effects::execute_effect(&mut game,&program,&mut ctx).unwrap()
            }else{effect.execute(&mut game,&mut ctx).unwrap()};
            assert!(outcome.status.is_success(),"a fully paid total must not inherit an empty exile leaf's status: {outcome:?}");
            assert!(outcome.instruction_result().execution_facts.contains(&ironsmith::effect::ExecutionFact::Accepted),"the exact optional owner publishes acceptance: {outcome:?}");
            assert!(ironsmith::effect::EffectPredicate::Happened.evaluate_outcome(&outcome),"a subordinate zero-action status must not veto full paid acknowledgement: {outcome:?}");
            assert!(!outcome.instruction_result().execution_facts.iter().any(|fact|matches!(fact,ironsmith::effect::ExecutionFact::TargetInvalid|ironsmith::effect::ExecutionFact::Impossible|ironsmith::effect::ExecutionFact::Declined)),"the primary control-flow receipt belongs to the paid total");
            assert_eq!(game.player(A).unwrap().life,20+i32::from(composed),"the actual IfDo follow-up uses its owning payment receipt");
            if count=="two" {
                let tapped=outcome.instruction_result().execution_facts.iter().filter_map(|fact|match fact{
                    ironsmith::effect::ExecutionFact::ActionObjects{action:ironsmith::effect::PriorEffectAction::Tapped,objects,..}=>Some(objects),_=>None,
                }).flatten().map(|snapshot|snapshot.object_id).collect::<std::collections::HashSet<_>>();
                assert_eq!(tapped,std::collections::HashSet::from([white,blue]),"normalizing control flow must retain actual tap-action memory");
            }
            assert_eq!(ctx.x_value,Some(99));
            assert_eq!(game.is_tapped(white),count=="two");assert_eq!(game.is_tapped(blue),count=="two");
            assert!(game.exile.is_empty());
        }}
    }
}
