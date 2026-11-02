//! Source Tether-to-artifact links to a noncreature permanent's exact graveyard departure.
use ironsmith::{CardDefinition,GameProgress,GameState,ObjectId,PlayerId,Zone};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
use ironsmith::triggers::TriggerQueue;
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices(bool);
impl ironsmith::decision::DecisionMaker for Choices {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{ironsmith::decision::DecisionMaker::decide_objects(&mut SelectFirstDecisionMaker,game,ctx)}
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None; game.turn.active_player = A; game.turn.priority_player = Some(A);
    game
}
fn cast(game: &mut GameState, definition: &CardDefinition, alternative: bool) -> ObjectId {
    cast_with_dm(game, definition, alternative, &mut SelectFirstDecisionMaker)
}
fn cast_with_dm(game: &mut GameState, definition: &CardDefinition, alternative: bool, dm: &mut impl ironsmith::decision::DecisionMaker) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell {spell_id, casting_method, ..} if *spell_id == source && matches!(casting_method, CastingMethod::Alternative(_)) == alternative)).expect("requested price available");
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
            return game.stack.first().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("cast unfinished");
}

fn definitions()->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-tether-next.json")).unwrap();let card=&fixture["mechanics"][1]["representative_cards"][0];
 source_support::definitions(card).unwrap()
}
fn flush(game:&mut GameState,dm:&mut Choices){let mut queue=TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();}
fn move_object(game:&mut GameState,source:ObjectId,zone:Zone){
 let outcome=ironsmith::effects::execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source),zone,false)),&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();for event in outcome.events{game.queue_trigger_event(Default::default(),event);}
}
#[test]
fn source_tether_artifact_is_optional_and_watches_its_chosen_noncreature_only(){
 for definition in definitions(){for accept in [false,true]{for host_exists in [false,true]{
  let mut game=setup();let artifact=ironsmith_compiler_runtime::compile_to_runtime_definition("Tether host","Type: Artifact",false).unwrap();
  let host=if host_exists{Some(game.create_object_from_definition(&artifact,A,Zone::Battlefield))}else{None};let unrelated=game.create_object_from_definition(&artifact,B,Zone::Battlefield);
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,3);
  let spell=cast(&mut game,&definition,false);let stable=game.object(spell).unwrap().stable_id;let mut dm=Choices(accept);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let current=game.find_object_by_stable_id(stable).unwrap();let linked=accept&&host_exists;assert_eq!(game.object(current).unwrap().zone,if linked{Zone::Exile}else{Zone::Graveyard});
  assert_eq!(game.effect_store.delayed_triggers.len(),usize::from(linked),"accept={accept} host_exists={host_exists}");
  if linked{move_object(&mut game,unrelated,Zone::Graveyard);flush(&mut game,&mut dm);assert!(game.stack.is_empty(),"an unrelated artifact cannot use the permission");move_object(&mut game,host.unwrap(),Zone::Graveyard);flush(&mut game,&mut dm);assert_eq!(game.stack.len(),1,"a noncreature artifact's graveyard departure triggers");ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.stack.len(),1,"the exact tethered card can be cast for free");assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Stack);}
 }}}
}
