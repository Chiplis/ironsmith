//! Per-ability selected alternative-cost receipts are independent of ordinary payments.
use ironsmith::{CardDefinition, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId=PlayerId(0);
const B: PlayerId=PlayerId(1);
const GRANT:&str="Give this artifact the energized status until end of turn. While this artifact has the energized status, you may pay {0} rather than pay the activation cost of each of its nonloyalty activated abilities once each turn.";
fn definitions() -> [CardDefinition;2] {
    let text=format!("Type: Artifact\n{{W}}, {{T}}: You gain 2 life.\n{{R}}, {{T}}, Pay 3 life, Sacrifice this artifact: You gain 5 life.\n{{0}}: {GRANT}\n{{2}}: You gain 1 life. Activate only as a sorcery.");
    materialized("Alternative witness", &text)
}
fn materialized(name:&str,text:&str)->[CardDefinition;2] {
    let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
struct Branch(bool);
impl DecisionMaker for Branch {
    fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
        if ctx.options.iter().any(|option|option.description.contains("once-per-turn alternative cost")) {
            return ctx.options.iter().filter(|option|option.legal&&option.description.contains("once-per-turn alternative cost")==self.0).take(1).map(|option|option.index).collect();
        }
        SelectFirstDecisionMaker.decide_options(game,ctx)
    }
}
fn game()->GameState {let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.turn.active_player=A;game.turn.priority_player=Some(A);game}
fn legal(game:&GameState,player:PlayerId,source:ObjectId,index:usize)->Option<LegalAction>{compute_legal_actions(game,player).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}|LegalAction::ActivateManaAbility{source:id,ability_index}if *id==source&&*ability_index==index))}
fn activate(game:&mut GameState,player:PlayerId,source:ObjectId,index:usize,free:bool){
    let action=legal(game,player,source,index).expect("requested activation is legal");let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=Branch(free);
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..64 {if !state.has_pending_action(){if !game.stack.is_empty(){resolve_stack_entry_with(game,&mut dm).unwrap();}return;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("activation did not finish")
}
#[test]
fn one_free_mana_use_and_one_normal_tap_can_fund_the_same_spell(){
    let text=format!("Type: Artifact\n{{T}}: Add {{U}}.\n{{0}}: {GRANT}");
    for definition in materialized("Funding alternative witness",&text){let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);activate(&mut game,A,source,1,false);
        let spell_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Two-blue funding witness","Mana cost: {U}{U}\nType: Instant\nYou gain 1 life.",false).unwrap();let spell=game.create_object_from_definition(&spell_definition,A,Zone::Hand);
        let cost=ironsmith::mana::ManaCost::from_pips(vec![vec![ManaSymbol::Blue],vec![ManaSymbol::Blue]]);
        let request=ironsmith::mana_payment::ManaPaymentRequest::new(A,spell,ironsmith::costs::PaymentReason::CastSpell,cost);
        eprintln!("potential={:?}",ironsmith::decision::compute_potential_mana(&game,A));
        eprintln!("inventory={:?}",ironsmith::mana_payment::mana_payment_activation_inventory_checked(&game,&request));
        eprintln!("plan={:?}",ironsmith::mana_payment::plan_first_mana_payment(&game,&request));
        let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).expect("both independent methods contribute to potential mana");
        let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=Branch(true);
        let mut progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
        for _ in 0..64 {if !state.has_pending_action(){break;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
        assert!(!state.has_pending_action());assert_eq!(game.stack.len(),1);assert!(game.is_tapped(source));assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,21);
    }
}

#[test]
fn actual_blooming_fireheart_pays_to_dynamize_then_reuses_each_whole_cost_once(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-dynamize-complete.json")).unwrap();let card=fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["name"]=="Blooming Fireheart").unwrap();assert_eq!(card["rewrite_status"],"frontend_materialized_unverified");
    for definition in materialized("Blooming Fireheart",card["executable_text"].as_str().unwrap()){let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red,1);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White,1);
        activate(&mut game,A,source,2,false);assert!(game.is_tapped(source));assert_eq!(game.player(A).unwrap().mana_pool.total(),0);assert!(game.has_custom_status(source,"dynamized"));
        activate(&mut game,A,source,1,true);assert_eq!(game.player(A).unwrap().life,22);
        activate(&mut game,A,source,0,true);assert!(game.battlefield.contains(&source),"free branch replaces the sacrifice as well as mana and tap");assert!(game.is_tapped(source));assert_eq!(game.player(A).unwrap().life+game.player(B).unwrap().life,40);
        activate(&mut game,A,source,2,true);for index in 0..3{assert!(legal(&game,A,source,index).is_none(),"redundant self-dynamize cannot replenish any consumed ability allowance");}
    }
}
