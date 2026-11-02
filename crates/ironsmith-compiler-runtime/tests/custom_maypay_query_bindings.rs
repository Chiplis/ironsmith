//! Optional full-payment query uses the same bindings as public activation prices.
use ironsmith::{GameState,Zone,PlayerId,Effect};
use ironsmith::costs::{Cost,CostContext};
use ironsmith::effects::MayEffect;
fn optional(effect:&Effect)->Option<MayEffect>{
 if let Some(may)=effect.downcast_ref::<MayEffect>(){if may.pay_as_cost{return Some(may.clone());}}
 let mut found=None;
 effect.0.visit_child_effects(&mut |child|{if found.is_none(){found=optional(child);}});
 found
}
#[test]
fn full_optional_cost_is_affordable_when_only_the_battlefield_partition_is_nonempty(){
 const TEXT:&str="Type: Sorcery\nYou may pay the cost \"Choose two untapped creatures you control or cards from your graveyard, each representing a different color it has; Tap the chosen creatures on the battlefield; Exile the chosen cards from your graveyard\". If you do, draw a card.";
 let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Optional resource program",TEXT,false).unwrap();
 let may=definition.spell_effect.as_ref().unwrap().all_effects_owned().iter().find_map(optional).expect("owning MayPay");
 let cost=Cost::try_effects(may.effects.clone()).unwrap();
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let a=PlayerId(0);
 let source=game.create_object_from_definition(&definition,a,Zone::Stack);
 for mana in ["{W}","{U}"]{
 let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Resource",&format!("Mana cost: {mana}\nType: Creature\nPower/Toughness: 2/2"),false).unwrap();
 game.create_object_from_definition(&body,a,Zone::Battlefield);
 }
 let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
 let mut ctx=CostContext::new(source,a,&mut dm);ctx.reason=ironsmith::costs::PaymentReason::Effect;
 for component in cost.as_all().unwrap(){let result=component.can_pay(&game,&ctx);assert!(result.is_ok(),"optionalprice query {result:?}; {component:?}; owningMay={may:?}");}
}
