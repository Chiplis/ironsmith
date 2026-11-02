//! Private exhaustive hypotheses for before-proposal copy-source choices.
//! Uses the real proposal owner on isolated clones; never answers live decisions.
use super::*;
use crate::decision::DecisionMaker;
#[derive(Clone)]
enum Answer {Option(usize),Boolean(bool)}
#[derive(Default)]
struct Hypothesis {answers:Vec<Answer>,cursor:usize,next:Option<Vec<Answer>>,unsupported:bool}
impl DecisionMaker for Hypothesis {
 fn awaiting_choice(&self)->bool{self.next.is_some()||self.unsupported}
 fn decide_options(&mut self,_:&GameState,ctx:&crate::decisions::context::SelectOptionsContext)->Vec<usize>{
  if ctx.min!=1||ctx.max!=1 {self.unsupported=true;return Vec::new();}
  if let Some(Answer::Option(index))=self.answers.get(self.cursor){
   self.cursor+=1;
   if ctx.options.iter().any(|option|option.legal&&option.index==*index){return vec![*index];}
   self.unsupported=true;return Vec::new();
  }
  self.next=Some(ctx.options.iter().filter(|option|option.legal).map(|option|Answer::Option(option.index)).collect());Vec::new()
 }
 fn decide_boolean(&mut self,_:&GameState,_:&crate::decisions::context::BooleanContext)->bool{
  if let Some(Answer::Boolean(value))=self.answers.get(self.cursor){self.cursor+=1;return *value;}
  self.next=Some(vec![Answer::Boolean(false),Answer::Boolean(true)]);false
 }
 fn decide_objects(&mut self,_:&GameState,_:&crate::decisions::context::SelectObjectsContext)->Vec<ObjectId>{self.unsupported=true;Vec::new()}
 fn decide_colors(&mut self,_:&GameState,_:&crate::decisions::context::ColorsContext)->Vec<crate::color::Color>{self.unsupported=true;Vec::new()}
}
/// Some only for copy casts, before a proposed stack incarnation exists.
/// A checked discovery failure is retained rather than guessed from one route.
pub(super) fn any_copy_cast_proposal(
 game:&GameState,player:PlayerId,spell:&crate::object::Object,method:&CastingMethod,
 mut admits:impl FnMut(&GameState,ObjectId)->bool,
)->Option<Result<bool,crate::game_loop::GameLoopError>>{
 if spell.zone==Zone::Stack{return None;}
 let native_copy=alternative_method_for_casting_method(game,player,spell,method)
  .is_some_and(|selected|selected.battlefield_instructions().cast_copy);
 if !native_copy&&spell.kind!=crate::object::ObjectKind::SpellCopy{return None;}
 let failure=|message:&str|crate::game_loop::GameLoopError::ExecutionFailed(crate::effects::ExecutionError::IncompleteEvidence(message.into()));
 let mut prefixes=vec![Vec::new()];let mut explored=0usize;
 while let Some(answers)=prefixes.pop(){
  explored+=1;
  if explored>4096{return Some(Err(failure("copy-source legality hypotheses exceeded bounded discovery")));}
  let mut preview=game.clone();
  let Some(candidate)=preview.object_mut(spell.id)else{return Some(Err(failure("copy-source legality donor is unavailable")));};
  *candidate=spell.clone();
  let mut chooser=Hypothesis{answers:answers.clone(),..Default::default()};
  let result=crate::game_loop::propose_spell_cast_with_dm(&mut preview,spell.id,spell.zone,player,method,&mut chooser);
  if chooser.unsupported{return Some(Err(failure("copy-source legality requires an unsupported choice shape")));}
  if let Some(next)=chooser.next{
   for answer in next {let mut path=answers.clone();path.push(answer);prefixes.push(path);}
   continue;
  }
  match result {
   Ok(Some(id))=>if admits(&preview,id){return Some(Ok(true));},
   Ok(None)=>return Some(Err(failure("copy-source legality suspended without a choice domain"))),
   Err(crate::game_loop::GameLoopError::ActionCancelled(_))=>{},
   Err(error)=>return Some(Err(error)),
  }
 }
 Some(Ok(false))
}
