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
fn whole_cost_substitution_skips_tap_life_and_sacrifice_and_tracks_each_ability_independently(){
    for definition in definitions(){
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White,1);
        activate(&mut game,A,source,0,false);assert_eq!(game.player(A).unwrap().life,22);assert!(game.is_tapped(source));
        activate(&mut game,A,source,2,false);assert!(game.has_custom_status(source,"energized"));
        activate(&mut game,A,source,0,true);assert_eq!(game.player(A).unwrap().life,24);assert!(game.is_tapped(source));
        activate(&mut game,A,source,1,true);assert_eq!(game.player(A).unwrap().life,29,"free branch pays no life");assert!(game.object(source).is_some(),"free branch does not sacrifice source");
        assert!(legal(&game,A,source,0).is_none());assert!(legal(&game,A,source,1).is_none());
        activate(&mut game,A,source,2,false);assert!(legal(&game,A,source,0).is_none(),"reapplication never resets same-turn quota");
        game.set_current_controller(source,B).unwrap();game.turn.priority_player=Some(B);assert!(legal(&game,B,source,0).is_none(),"quota follows permanent, not activator");
        game.cleanup_custom_statuses_end_of_turn();game.turn.turn_number+=1;assert!(legal(&game,B,source,0).is_none(),"expired status disables retained grant");
        activate(&mut game,B,source,2,false);activate(&mut game,B,source,0,true);assert_eq!(game.player(B).unwrap().life,22,"new turn has a fresh allowance");
    }
}
#[test]
fn zero_cost_alternative_preserves_sorcery_timing_and_source_zone_restrictions(){
    for definition in definitions(){let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);activate(&mut game,A,source,2,false);
        game.turn.phase=ironsmith::game_state::Phase::Combat;assert!(legal(&game,A,source,3).is_none(),"alternate payment cannot bypass timing");
        game.turn.phase=ironsmith::game_state::Phase::FirstMain;assert!(legal(&game,A,source,3).is_some());
        let stable=game.object(source).unwrap().stable_id;let effect=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Graveyard,false));ironsmith::effects::execute_effect(&mut game,&effect,&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();
        let grave=game.find_object_by_stable_id(stable).unwrap();assert!(legal(&game,A,grave,0).is_none(),"battlefield abilities still cannot function in graveyard");
    }
}
#[test]
fn every_complete_dynamize_source_compiles_strictly_and_roundtrips(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-dynamize-complete.json")).unwrap();let cards=fixture["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),12);
    for card in cards{assert_eq!(card["rewrite_status"],"frontend_materialized_unverified","{}",card["name"]);let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap(),false)));assert!(!loss.is_lossy(),"{}: {}",card["name"],loss.reasons_text());let(artifact,_)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();}
}

#[test]
fn mana_ability_can_use_its_own_free_branch_while_tapped_and_only_once(){
    let text=format!("Type: Artifact\n{{T}}: Add {{U}}.\n{{0}}: {GRANT}");
    for definition in materialized("Mana alternative witness",&text){let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        activate(&mut game,A,source,0,false);assert!(game.is_tapped(source));assert_eq!(game.player(A).unwrap().mana_pool.total(),1);
        activate(&mut game,A,source,1,false);activate(&mut game,A,source,0,true);assert!(game.is_tapped(source));assert_eq!(game.player(A).unwrap().mana_pool.total(),2);
        assert!(legal(&game,A,source,0).is_none(),"a mana ability owns the same one-use allowance as a nonmana ability");
    }
}
#[test]
fn loyalty_is_excluded_and_off_zone_abilities_never_gain_battlefield_permission(){
    let text="Type: Planeswalker\nLoyalty: 3\n+1: You gain 7 life.\n{0}: Give this planeswalker the energized status until end of turn. While this planeswalker has the energized status, you may pay {0} rather than pay the activation cost of each of its nonloyalty activated abilities once each turn.";
    for definition in materialized("Loyalty alternative witness",text){let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);activate(&mut game,A,source,1,false);
        let ability=game.current_activated_ability(source,0).unwrap();let facts=ironsmith::decision::ActivationCostAbility::at(&game,A,source,0).unwrap();assert!(facts.loyalty_ability);
        let cost=ironsmith::decision::calculate_effective_activation_total_cost_for_ability(&game,A,source,&ability.mana_cost,&[],Some(&facts));assert!(cost.as_one_of().is_none(),"nonloyalty filter never offers a free loyalty branch");
    }
    let text=format!("Type: Artifact\nCycling {{2}}\n{{0}}: {GRANT}");
    for definition in materialized("Zone alternative witness",&text){let cycling=definition.abilities.iter().enumerate().find(|(_,a)|matches!(a.kind,ironsmith::ability::AbilityKind::Activated(_))&&a.functions_in(&Zone::Hand)&&!a.functions_in(&Zone::Battlefield)).map(|(i,_)|i).unwrap();let grant=definition.abilities.iter().enumerate().find(|(_,a)|format!("{a:?}").contains("GrantActivationCostAlternativeEffect")).map(|(i,_)|i).unwrap();let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);activate(&mut game,A,source,grant,false);assert!(legal(&game,A,source,cycling).is_none());}
}

#[test]
fn one_free_mana_use_and_one_normal_tap_can_fund_the_same_spell(){
    let text=format!("Type: Artifact\n{{T}}: Add {{U}}.\n{{0}}: {GRANT}");
    for definition in materialized("Funding alternative witness",&text){let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);activate(&mut game,A,source,1,false);
        let spell_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Two-blue funding witness","Mana cost: {U}{U}\nType: Instant\nYou gain 1 life.",false).unwrap();let spell=game.create_object_from_definition(&spell_definition,A,Zone::Hand);
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
