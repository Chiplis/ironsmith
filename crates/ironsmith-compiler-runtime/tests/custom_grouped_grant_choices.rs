//! Composed granted abilities remain one resolution-time alternative.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,compute_legal_actions,LegalAction};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices {mode:usize,target:ObjectId,choices:usize,targets:usize}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self,_:&GameState,_:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{self.targets+=1;vec![ironsmith::Target::Object(self.target)]}
    fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
        if ctx.options.len()==2 && ctx.options.iter().any(|option|option.description.to_ascii_lowercase().contains("lifelink")) {self.choices+=1;vec![self.mode]}else{SelectFirstDecisionMaker.decide_options(game,ctx)}
    }
}
#[test]
fn full_source_chooses_one_complete_grant_after_casting_without_losing_target_or_followup() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-middle.json")).unwrap();
    let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]=="Wightlight").unwrap();
    for definition in source_support::definitions(card).unwrap() {for mode in [0,1] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary body","Type: Creature\nPower/Toughness: 2/8",false).unwrap();let target=game.create_object_from_definition(&body,B,Zone::Battlefield);let source=game.create_object_from_definition(&body,A,Zone::Battlefield);let untouched=game.create_object_from_definition(&body,B,Zone::Battlefield);
        let bottom=game.create_object_from_definition(&body,A,Zone::Library);let bottom_stable=game.object(bottom).unwrap().stable_id;
        let spell=game.create_object_from_definition(&definition,A,Zone::Hand);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,1);
        let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell {spell_id,..} if *spell_id==spell)).unwrap();
        let mut dm=Choices {mode,target,choices:0,targets:0};let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
        let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
        for _ in 0..32 {if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
        assert!(!state.has_pending_action());assert_eq!(dm.choices,0,"choice is made at resolution, after declaring the one common target");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(dm.choices,1);assert_eq!(dm.targets,1,"both alternatives use the one original target announcement");
        assert_eq!(game.current_has_static_ability_id(target,ironsmith_core::StaticAbilityId::Lifelink),mode==0);
        let marker=game.current_abilities(target).unwrap().iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(rule) if rule.compiled_model().is_some_and(|model|matches!(&model.payload,ironsmith_core::StaticAbilityPayload::CustomKeywordIdentity {name} if name.as_str()=="winnow"))));
        assert_eq!(marker,mode==1,"the chosen custom marker and rule form one indivisible alternative");
        let damage=ironsmith::Effect::new(ironsmith::effects::DealDamageEffect::new(2,ironsmith::target::ChooseSpec::SpecificObject(target)));ironsmith::effects::execute_effect(&mut game,&damage,&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();assert_eq!(game.player(B).unwrap().life,if mode==1 {18}else{20});
        assert!(!game.current_has_static_ability_id(untouched,ironsmith_core::StaticAbilityId::Lifelink));assert_eq!(game.object(game.find_object_by_stable_id(bottom_stable).unwrap()).unwrap().zone,Zone::Hand,"full source sibling instruction still executes");
    }}
}
