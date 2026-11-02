//! Unregistered native proof for the unapplied generic grant component/binder.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::object::{AttachmentTarget,CounterType};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(keyword:&str)->[CardDefinition;2] {
    let text=format!("Type: Artifact — Equipment\nEquipped creature has hexproof, custom keyword \"{keyword}\", and \"{{1}}: Put a +1/+1 counter on this permanent. This ability costs {{1}} more to activate for each +1/+1 counter on this permanent. Activate only as a sorcery.\"");
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Parameterized keyword grantor",&text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();assert!(direct.spell_effect.is_none(), "static recipient grants must be printed abilities: {direct:?}");assert!(!direct.abilities.is_empty());let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn identity(game:&GameState,object:ObjectId,name:&str)->bool {
    game.current_abilities(object).unwrap_or_default().iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(marker) if marker.custom_keyword_name().is_some_and(|actual|actual.eq_ignore_ascii_case(name))))
}
fn legal(game:&GameState,source:ObjectId,index:usize)->Option<LegalAction> {
    compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index} if *id==source&&*ability_index==index))
}
fn activate(game:&mut GameState,action:LegalAction) {
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..64 {if !state.has_pending_action(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut dm).unwrap();return;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("activation unfinished");
}
#[test]
fn identity_and_dynamic_price_belong_to_recipient_while_printed_abilities_remain_independent() {
    for keyword in ["improve","steady craft"] {for definition in definitions(keyword) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
        let gear=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.object_mut(gear).unwrap().counters.insert(CounterType::PlusOnePlusOne,9);
        let host_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient with independent activation","Type: Creature\nPower/Toughness: 2/2\n{0}: You gain 1 life.",false).unwrap();let host=game.create_object_from_definition(&host_definition,A,Zone::Battlefield);
        assert!(!identity(&game,gear,keyword));assert!(!identity(&game,host,keyword));assert!(game.attach_object_to_target(gear,AttachmentTarget::Object(host)));
        assert!(identity(&game,host,keyword));assert!(!identity(&game,gear,keyword));
        let activated=game.current_abilities(host).unwrap().iter().enumerate().filter_map(|(index,ability)|matches!(&ability.kind,ironsmith::ability::AbilityKind::Activated(_)).then_some(index)).collect::<Vec<_>>();assert_eq!(activated.len(),2);let printed=activated[0];let granted=activated[1];
        assert!(legal(&game,host,printed).is_some());assert!(legal(&game,host,granted).is_none());
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);let action=legal(&game,host,granted).expect("one mana pays recipient0counters, despite grantor9counters");activate(&mut game,action);assert_eq!(game.counter_count(host,CounterType::PlusOnePlusOne),1);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        assert!(legal(&game,host,printed).is_some(),"scoped cost increase cannot tax printed free ability");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);assert!(legal(&game,host,granted).is_none());game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);let action=legal(&game,host,granted).expect("recipient1counter raises next activation to2");activate(&mut game,action);assert_eq!(game.counter_count(host,CounterType::PlusOnePlusOne),2);
        assert!(game.detach_object_from_current_target(gear));assert!(!identity(&game,host,keyword));assert!(!identity(&game,gear,keyword));assert_eq!(game.current_abilities(host).unwrap().iter().filter(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Activated(_))).count(),1);
    }}
}
#[test]
fn granted_activation_prices_read_recipient_mana_cost_independently_of_grantor() {
 let text="Mana cost: {9}\nType: Artifact — Equipment\nEquipped creature has \"Pay this creature's mana cost, {1}: Put a +1/+1 counter on this creature.\"";
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Mana cost teacher",text,false).unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  let gear=game.create_object_from_definition(&definition,A,Zone::Battlefield);let host_def=ironsmith_compiler_runtime::compile_to_runtime_definition("Priced recipient","Mana cost: {1}{G}\nType: Creature\nPower/Toughness: 2/2",false).unwrap();let host=game.create_object_from_definition(&host_def,A,Zone::Battlefield);assert!(game.attach_object_to_target(gear,AttachmentTarget::Object(host)));
  let index=game.current_abilities(host).unwrap().iter().position(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Activated(_))).unwrap();
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);assert!(legal(&game,host,index).is_none(),"recipient mana cost plus one additional mana");game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);let action=legal_action(&game,host,index);activate(&mut game,action);assert_eq!(game.counter_count(host,CounterType::PlusOnePlusOne),1);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
 }
}
fn legal_action(game:&GameState,source:ObjectId,index:usize)->LegalAction {legal(game,source,index).expect("recipient dynamic mana price affordable")}

#[test]
fn composed_mana_prices_share_funding_without_borrowing_from_another_ability_or_branch(){
 use ironsmith::costs::Cost;use ironsmith::mana::{ManaCost,ManaSymbol};
 let one=||Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]));
 for counters in 0..=2 {for available in 0..=3 {
  let mut definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Composed intrinsic price","Type: Creature\nPower/Toughness: 2/2\n{1}: You gain 1 life.\n{0}: You gain 1 life.",false).unwrap();
  let dynamic=Cost::dynamic_mana(ironsmith_core::DynamicManaCost::generic_equal_to(ironsmith::effect::Value::CountersOnSource(CounterType::PlusOnePlusOne)));
  let ironsmith::ability::AbilityKind::Activated(ability)=&mut definition.abilities[0].kind else{panic!()};ability.mana_cost=ironsmith::cost::TotalCost::from_costs(vec![one(),dynamic]);
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,counters);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,available);
  assert_eq!(legal(&game,source,0).is_some(),available>=1+counters,"available={available} counters={counters}: one pool funds the whole dynamic price");assert!(legal(&game,source,1).is_some(),"independent free ability stays independent");
 }}
 for alternative in [false,true]{let mut definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Composed fixed price","Type: Artifact\n{1}: You gain 1 life.",false).unwrap();let split=ironsmith::cost::TotalCost::from_costs(vec![one(),one()]);let ironsmith::ability::AbilityKind::Activated(ability)=&mut definition.abilities[0].kind else{panic!()};ability.mana_cost=if alternative{ironsmith::cost::TotalCost::one_of(vec![split,ironsmith::cost::TotalCost::from_cost(one())])}else{split};let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);assert_eq!(legal(&game,source,0).is_some(),alternative,"split fixed amounts share funding, but a cheaper OneOf branch remains independent");}
}
