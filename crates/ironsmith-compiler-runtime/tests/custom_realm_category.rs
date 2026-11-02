//! Realm/Favor exact source: independent strict direct and compiled-artifact routes.
#[path = "custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{GameState, ObjectId, PlayerId, Zone, EffectContext};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
fn setup()->GameState {let mut g=GameState::new(vec!["A".into(),"B".into()],20); main(&mut g,A); g}
fn main(g:&mut GameState,player:PlayerId){g.turn.active_player=player;g.turn.priority_player=Some(player);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;}
fn legal(g:&GameState,source:ObjectId,index:usize,player:PlayerId)->Option<LegalAction>{compute_legal_actions(g,player).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}|LegalAction::ActivateManaAbility{source:id,ability_index} if *id==source&&*ability_index==index))}
fn favor(g:&GameState,source:ObjectId,player:PlayerId)->bool{ironsmith::condition_eval::evaluate_condition_resolution(g,&ironsmith_core::Condition::EachCurrentDesignatedAbilityActivatedByYou{designation:"realm".into()},&EffectContext::new_default(source,player)).unwrap()}
fn definitions(text:&str)->[ironsmith::CardDefinition;2]{source_support::definitions(&serde_json::json!({"name":"Realm scenario witness","executable_text":text})).unwrap()}
fn announce_action(
    g: &mut GameState,
    action: LegalAction,
    actor: PlayerId,
    resolve: bool,
    dm: &mut impl ironsmith::decision::DecisionMaker,
) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &ctx, dm).unwrap();
    }
    assert!(!state.has_pending_action());
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(g, &mut q, dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
    while resolve && !g.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(g, dm).unwrap();
    }
    g.turn.priority_player = Some(actor);
}

#[test]
fn all_three_exact_sources_once_sorcery_and_favor_bodies(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-realm-category.json")).unwrap();
 let rows=fixture["mechanics"][0]["representative_cards"].as_array().unwrap(); assert_eq!(rows.len(),3);
 for row in rows {for (route,definition) in source_support::definitions(row).unwrap().into_iter().enumerate(){
  let mut g=setup();let realm=g.create_object_from_definition(&definition,A,Zone::Battlefield);
  let elf=definitions("Mana cost: {0}\nType: Creature — Elf\nPower/Toughness: 2/2")[route].clone();
  let subject=g.create_object_from_definition(&elf,A,Zone::Battlefield);
  let forest=definitions("Type: Basic Land — Forest")[route].clone();
  for _ in 0..12 {g.create_object_from_definition(&forest,A,Zone::Library);g.create_object_from_definition(&elf,A,Zone::Library);}
  for symbol in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green]{g.player_mut(A).unwrap().mana_pool.add(symbol,20);}
  assert!(!favor(&g,realm,A),"{} route{route}",row["name"]);
  g.turn.phase=ironsmith::game_state::Phase::Combat; assert!(legal(&g,realm,0,A).is_none());main(&mut g,A);
  for index in 0..3 {
   assert_eq!(favor(&g,realm,A),false);
   let action=legal(&g,realm,index,A).expect("unspent realm ability");execute_action(&mut g,action,A,&mut SelectFirstDecisionMaker);
   assert!(legal(&g,realm,index,A).is_none(),"once per incarnation");
  }
  assert!(favor(&g,realm,A));assert!(!favor(&g,realm,B),"YOU means actual activator, not any player");
  match row["name"].as_str().unwrap(){
   "Hinode, Glorious Peak"=>{assert!(g.current_has_static_ability_id(subject,StaticAbilityId::Flying));assert!(g.current_has_static_ability_id(subject,StaticAbilityId::Lifelink));assert_eq!(g.counter_count(subject,ironsmith::object::CounterType::PlusOnePlusOne),2);},
   "Realms of the Goliaths"=>{assert!(g.current_has_static_ability_id(subject,StaticAbilityId::Hexproof));assert!(g.current_has_static_ability_id(subject,StaticAbilityId::Indestructible));assert!(!g.current_has_static_ability_id(realm,StaticAbilityId::Hexproof));},
   _=>{let before=g.player(A).unwrap().hand.len();let spell=g.create_object_from_definition(&elf,A,Zone::Hand);let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell)).unwrap();execute_action(&mut g,action,A,&mut SelectFirstDecisionMaker);assert_eq!(g.player(A).unwrap().hand.len(),before+1,"Elf cast draws from Favor");}
  }
  g.set_current_controller(realm,B).unwrap();main(&mut g,B);
  for symbol in [ManaSymbol::White,ManaSymbol::Green]{g.player_mut(B).unwrap().mana_pool.add(symbol,20);}
  assert!(!favor(&g,realm,B));for index in 0..3{assert!(legal(&g,realm,index,B).is_none(),"once survives controller changes");}
  g.set_current_controller(realm,A).unwrap();main(&mut g,A);assert!(favor(&g,realm,A));
  change(&mut g,realm,ironsmith::continuous::Modification::RemoveAllSubtypesOfFamily(ironsmith_core::SubtypeFamily::Enchantment));
  assert!(favor(&g,realm,A),"designation survives loss of Realm subtype");
  g.next_turn();main(&mut g,A);for symbol in [ManaSymbol::White,ManaSymbol::Green]{g.player_mut(A).unwrap().mana_pool.add(symbol,20);}assert!(favor(&g,realm,A));for index in 0..3{assert!(legal(&g,realm,index,A).is_none());}
  let fresh=g.create_object_from_definition(&definition,A,Zone::Battlefield);assert!(!favor(&g,fresh,A),"another incarnation has no activation history");
 }}
}
#[test]
fn empty_current_designated_set_is_vacuously_true(){for definition in definitions("Type: Artifact\nAs long as you have activated each of this permanent's activated abilities with designation realm, this permanent has hexproof."){let mut g=setup();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);assert!(favor(&g,source,A));assert!(g.current_has_static_ability_id(source,StaticAbilityId::Hexproof));}}

fn change(g:&mut GameState,source:ObjectId,modification:ironsmith::continuous::Modification){
 use ironsmith::effects::EffectExecutor;
 ironsmith::effects::ApplyContinuousEffect::with_spec(ironsmith::target::ChooseSpec::SpecificObject(source),modification,ironsmith::effect::Until::Forever)
  .execute(g,&mut EffectContext::new_default(source,A)).unwrap();
 g.refresh_continuous_state().unwrap();
}
#[test]
fn current_set_tracks_native_ability_loss_gain_and_reacquisition(){
 for (route,definition) in definitions("Type: Artifact").into_iter().enumerate(){
  let mut g=setup();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);
  let donor=definitions("Type: Artifact\n{0}: You gain 1 life. Activate only once. This activated ability has designation realm.")[route].clone();
  let ability=donor.abilities[0].clone();assert!(favor(&g,source,A));
  change(&mut g,source,ironsmith::continuous::Modification::AddAbilityGeneric(ability.clone()));
  assert!(!favor(&g,source,A));let action=legal(&g,source,0,A).unwrap();execute_action(&mut g,action,A,&mut SelectFirstDecisionMaker);assert!(favor(&g,source,A));
  change(&mut g,source,ironsmith::continuous::Modification::RemoveAllAbilities);assert!(favor(&g,source,A));
  change(&mut g,source,ironsmith::continuous::Modification::AddAbilityGeneric(ability));
  assert!(!favor(&g,source,A),"same definition newly acquired requires its own activation");
  let action=legal(&g,source,0,A).expect("new acquisition has its own once receipt");execute_action(&mut g,action,A,&mut SelectFirstDecisionMaker);assert!(favor(&g,source,A));
 }
}

fn execute_action(g:&mut GameState,action:LegalAction,actor:PlayerId,dm:&mut impl ironsmith::decision::DecisionMaker){announce_action(g,action,actor,true,dm);}
#[test]
fn countered_activation_still_satisfies_the_committed_activation_requirement(){
 use ironsmith::effects::EffectExecutor;
 for definition in definitions("Type: Artifact\n{0}: You gain 3 life. Activate only once. This activated ability has designation realm.\nAs long as you have activated each of this permanent's activated abilities with designation realm, this permanent has hexproof."){
  let mut g=setup();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);assert!(!favor(&g,source,A));
  g.refresh_continuous_state().unwrap();assert!(!g.current_has_static_ability_id(source,StaticAbilityId::Hexproof));
  let action=legal(&g,source,0,A).unwrap();announce_action(&mut g,action,A,false,&mut SelectFirstDecisionMaker);
  assert!(favor(&g,source,A));assert!(g.current_has_static_ability_id(source,StaticAbilityId::Hexproof),"free activation dirties the already-warm characteristic cache before resolution");assert_eq!(g.player(A).unwrap().life,20,"receipt is committed before the ability resolves");
  let ability=g.stack.last().unwrap().target_id();
  ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(ability)).execute(&mut g,&mut EffectContext::new_default(source,B)).unwrap();
  assert!(g.stack.is_empty());assert!(favor(&g,source,A));assert_eq!(g.player(A).unwrap().life,20);assert!(legal(&g,source,0,A).is_none());
 }
}

fn tracked_identity(g:&GameState,source:ObjectId)->(ironsmith::continuous::AbilityOrigin,ironsmith_core::LinkedExileDefinition){
 let chars=g.current_characteristics(source).unwrap();let origin=chars.abilities.origin(0).unwrap().clone();
 let ironsmith::ability::AbilityKind::Activated(activated)=&chars.abilities[0].kind else{panic!("expected designated activation")};
 assert_eq!(activated.effects.activation_designations,vec!["realm".to_string()]);
 (origin,activated.effects.activation_definition.expect("every designated occurrence must have a typed immutable definition"))
}
fn transform(g:&mut GameState,source:ObjectId){use ironsmith::effects::EffectExecutor;ironsmith::effects::TransformEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source)).execute(g,&mut EffectContext::new_default(source,A)).unwrap();g.refresh_continuous_state().unwrap();}
#[test]
fn same_printed_slot_new_definition_requires_and_permits_its_own_activation(){
 let row=serde_json::json!({"name":"Designated face witness","source_layout":"transform","executable_faces":[
  {"name":"First designated face","executable_text":"Type: Artifact\n{0}: You gain 1 life. Activate only once. This activated ability has designation realm.\nAs long as you have activated each of this permanent's activated abilities with designation realm, this permanent has hexproof."},
  {"name":"Second designated face","executable_text":"Type: Artifact\n{0}: You gain 2 life. Activate only once. This activated ability has designation realm.\nAs long as you have activated each of this permanent's activated abilities with designation realm, this permanent has hexproof."}
 ]});
 let faces=source_support::linked_definitions(&row).unwrap();assert_eq!(faces.len(),2);
 for route in 0..2{
  let mut g=setup();for face in &faces{g.register_linked_face_definition(&face[route]);}
  let source=g.create_object_from_definition(&faces[0][route],A,Zone::Battlefield);let first=tracked_identity(&g,source);
  let action=legal(&g,source,0,A).unwrap();execute_action(&mut g,action,A,&mut SelectFirstDecisionMaker);assert!(favor(&g,source,A));assert!(legal(&g,source,0,A).is_none());
  transform(&mut g,source);let second=tracked_identity(&g,source);
  assert_eq!(first.0,second.0,"regression requires exactly the reused Printed slot");assert_ne!(first.1,second.1,"new face retains a distinct typed definition");
  assert!(!favor(&g,source,A));assert!(!g.current_has_static_ability_id(source,StaticAbilityId::Hexproof));
  let action=legal(&g,source,0,A).expect("the new definition's once cap must not inherit the first face receipt");execute_action(&mut g,action,A,&mut SelectFirstDecisionMaker);assert!(favor(&g,source,A));assert_eq!(g.player(A).unwrap().life,23);
  transform(&mut g,source);assert_eq!(tracked_identity(&g,source),first);assert!(favor(&g,source,A));assert!(legal(&g,source,0,A).is_none(),"returning original face retains its original once receipt");
 }
}

#[test]
fn immediate_mana_activation_invalidates_warm_favor_characteristics(){
 for definition in definitions("Type: Artifact\n{0}: Add {G}. Activate only once. This activated ability has designation realm.\nAs long as you have activated each of this permanent's activated abilities with designation realm, this permanent has hexproof."){
  let mut g=setup();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);
  g.refresh_continuous_state().unwrap();assert!(!g.current_has_static_ability_id(source,StaticAbilityId::Hexproof));
  let action=legal(&g,source,0,A).unwrap();assert!(matches!(action,LegalAction::ActivateManaAbility{..}));
  execute_action(&mut g,action,A,&mut SelectFirstDecisionMaker);
  assert!(g.stack.is_empty());assert!(favor(&g,source,A));assert!(g.current_has_static_ability_id(source,StaticAbilityId::Hexproof));assert!(legal(&g,source,0,A).is_none());
 }
}
