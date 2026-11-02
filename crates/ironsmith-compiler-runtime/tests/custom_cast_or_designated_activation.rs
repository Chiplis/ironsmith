//! One resolution-time alternative invokes one native cast or retained activation.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
fn definitions(name:&str,text:&str)->[CardDefinition;2]{source_routes::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap_or_else(|error|panic!("{name}: {error}"))}
const INSTRUCTION:&str="Type: Sorcery\nYou may cast an enchantment spell from your hand without paying its mana cost or activate an activated ability with designation sample of a permanent you control without paying its costs during sample actions.";
struct Decisions{mode:usize,accept:bool,offers:Vec<String>}
impl DecisionMaker for Decisions{
 fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
  self.offers.push(ctx.description.clone());
  if ctx.options.len()==2 && ctx.options.iter().any(|option|option.description.to_ascii_lowercase().contains("designation sample")){assert!(ctx.options.iter().any(|option|option.index==self.mode&&option.legal));vec![self.mode]}
  else{SelectFirstDecisionMaker.decide_options(game,ctx)}
 }
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.accept}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(game,ctx)}
 fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{SelectFirstDecisionMaker.decide_targets(game,ctx)}
}
#[test]
fn each_alternative_uses_one_real_announcement_and_decline_produces_none(){
 for (route,instruction) in definitions("Alternative instruction",INSTRUCTION).into_iter().enumerate(){for mode in [0,1]{for accept in [false,true]{
  let mut game=play::game();game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.priority_player=Some(play::B);
  let performer=definitions("Designated source","Type: Artifact\n{9}, {T}, Sacrifice this artifact: You gain 3 life. This activated ability has designation sample. Activate only as a sorcery or during sample actions.\n{0}: You gain 9 life.")[route].clone();
  let performer=game.create_object_from_definition(&performer,play::A,Zone::Battlefield);
  let enchantment=definitions("Free eligible spell","Mana cost: {9}\nType: Enchantment")[route].clone();let enchantment=game.create_object_from_definition(&enchantment,play::A,Zone::Hand);let enchantment_stable=game.object(enchantment).unwrap().stable_id;
  let excluded=play::vanilla("Excluded creature","{0}","Fox",1,1);let excluded=game.create_object_from_definition(&excluded,play::A,Zone::Hand);
  let source=game.create_object_from_definition(&instruction,play::A,Zone::Stack);let mut dm=Decisions{mode,accept,offers:vec![]};let priority=game.turn.priority_player;
  let effects=instruction.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,play::A).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(game.stack.len(),usize::from(accept),"one selected branch, never both");assert_eq!(game.turn.priority_player,priority);assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0);assert_eq!(game.object(performer).unwrap().zone,Zone::Battlefield);assert!(!game.is_tapped(performer),"waived tap/sacrifice prices stay waived");assert_eq!(game.object(excluded).unwrap().zone,Zone::Hand);assert_eq!(game.player(play::A).unwrap().life,20,"the inner action is announced, not resolved inline");
  let arrival=game.find_object_by_stable_id(enchantment_stable).unwrap();
  if !accept{assert_eq!(game.object(arrival).unwrap().zone,Zone::Hand);continue;}
  if mode==0{assert!(!game.stack[0].is_ability);assert_eq!(game.object(arrival).unwrap().zone,Zone::Stack);}else{assert!(game.stack[0].is_ability);assert_eq!(game.stack[0].ability_index,Some(0));assert_eq!(game.object(arrival).unwrap().zone,Zone::Hand);}
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  if mode==0{let arrival=game.find_object_by_stable_id(enchantment_stable).unwrap();assert_eq!(game.object(arrival).unwrap().zone,Zone::Battlefield);assert_eq!(game.player(play::A).unwrap().life,20);}else{assert_eq!(game.player(play::A).unwrap().life,23);}
 }}}
}
