//! Drive a real activation to cancellation/rejection, retaining native rollback.
use ironsmith::{GameProgress,GameState};
use ironsmith::decision::{DecisionMaker,LegalAction};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,GameLoopError,apply_priority_response_with_dm,apply_decision_context_with_dm};
use ironsmith::triggers::TriggerQueue;
pub fn reject_or_cancel<D:DecisionMaker>(game:&mut GameState,action:LegalAction,dm:&mut D)->bool {
 let mut queue=TriggerQueue::new();let mut state=PriorityLoopState::new(game.players.len());
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm);
 for _ in 0..64 {
  match progress {
   Err(GameLoopError::ActionCancelled(_))=>{assert!(!state.has_pending_action());assert!(game.stack.is_empty());return true;},
   Err(error)=>panic!("unexpected activation failure: {error:?}"),
   Ok(_)if !state.has_pending_action()=>{assert!(game.stack.is_empty(),"an unfunded activation must not commit");return true;},
   Ok(GameProgress::NeedsDecisionCtx(context))=>{progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,dm);},
   Ok(other)=>panic!("unfinished activation: {other:?}"),
  }
 }
 panic!("unfunded activation did not finish");
}
