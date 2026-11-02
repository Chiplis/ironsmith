//! Ordinary Compose uses exact successful-resolution arrivals; Function timing is excluded.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,compute_legal_actions,LegalAction};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-compose-source-complete.json")).unwrap()}
fn definitions(name:&str)->[CardDefinition;2]{let f=fixture();let card=f["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();source_support::definitions(card).unwrap_or_else(|error|panic!("{name}: {error}"))}
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;for symbol in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Red,ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Colorless]{g.player_mut(A).unwrap().mana_pool.add(symbol,30);}g}
fn ordinary(name:&str,text:&str)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn draw_cards(g:&mut GameState){let card=ordinary("Draw witness","Type: Land — Island");for _ in 0..5{g.create_object_from_definition(&card,A,Zone::Library);}}
fn cast(g:&mut GameState,id:ObjectId)->TriggerQueue{let action=compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id)).expect("actual source has a cast action");let mut state=PriorityLoopState::new(2);let mut q=TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;let mut progress=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();for _ in 0..64{if !state.has_pending_action(){return q}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("cast stopped outside a typed decision")};progress=apply_decision_context_with_dm(g,&mut q,&mut state,&ctx,&mut dm).unwrap();}panic!("cast did not finish")}
fn sagas(g:&GameState)->Vec<ObjectId>{g.battlefield.iter().copied().filter(|id|g.current_subtypes(*id).unwrap().contains(&ironsmith::Subtype::Saga)).collect()}
fn chapter_three(g:&mut GameState,saga:ObjectId){let effect=ironsmith::Effect::put_counters(ironsmith::CounterType::Lore,2,ironsmith::target::ChooseSpec::SpecificObject(saga));ironsmith::effects::execute_effect(g,&effect,&mut ironsmith::effects::EffectContext::new_default(saga,A)).unwrap();let mut q=TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut q,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut q,&mut SelectFirstDecisionMaker).unwrap();while !g.stack.is_empty(){resolve_stack_entry_with(g,&mut SelectFirstDecisionMaker).unwrap();}}
#[test]
fn all33ordinary_whole_sources_compile_on_independent_native_and_artifact_routes(){let f=fixture();let cards=f["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),33);let failures=cards.iter().filter_map(|card|source_support::definitions(card).err().map(|error|format!("{}: {error}",card["name"]))).collect::<Vec<_>>();assert!(failures.is_empty(),"{}",failures.join("\n"));let digital=&f["mechanics"][1]["representative_cards"][0];assert_eq!(digital["name"],"Digital Encores");assert_eq!(digital["excluded_dependent_clause"]["ordinary_compose_still_required"],true);}
#[test]
fn actual_inspect_plot_only_creates_saga_after_resolution_and_chapter_copies_exact_exiled_card(){for definition in definitions("Inspect the Plot"){let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);assert!(sagas(&g).is_empty());resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();let exiled=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);let tokens=sagas(&g);assert_eq!(tokens.len(),1);assert_eq!(g.counter_count(tokens[0],ironsmith::CounterType::Lore),1);assert_eq!(g.player(A).unwrap().hand.len(),1);chapter_three(&mut g,tokens[0]);assert_eq!(g.player(A).unwrap().hand.len(),2,"chapterIII casts a copy and its draw effect resolves");assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);assert!(sagas(&g).iter().all(|id|*id==tokens[0]),"the cast copy was not cast from hand and cannot create another Saga");}}
#[test]
fn actual_compose_countered_before_resolution_creates_no_saga(){for definition in definitions("Inspect the Plot"){let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);let stack=g.stack.last().unwrap().object_id;let counter=ironsmith::Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(stack)));ironsmith::effects::execute_effect(&mut g,&counter,&mut ironsmith::effects::EffectContext::new_default(stack,B)).unwrap();assert!(g.stack.is_empty());assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);assert!(sagas(&g).is_empty());assert!(g.player(A).unwrap().hand.is_empty());}}
#[test]
fn departed_and_returned_exile_incarnation_is_not_rebound_by_saga(){for definition in definitions("Inspect the Plot"){let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();let exiled=g.find_object_by_stable_id(stable).unwrap();let saga=sagas(&g)[0];let grave=g.move_object_by_effect(exiled,Zone::Graveyard).unwrap();let returned=g.move_object_by_effect(grave,Zone::Exile).unwrap();assert_ne!(returned,exiled);chapter_three(&mut g,saga);assert_eq!(g.player(A).unwrap().hand.len(),1,"stale link cannot cast another incarnation of the same physical card");assert_eq!(g.object(returned).unwrap().zone,Zone::Exile);}}

#[test]
fn actual_destroy_spell_with_all_targets_illegal_never_composes(){for definition in definitions("Defeat of the Greatest Evil"){let mut g=game();let creature=ordinary("Legal creature target","Type: Creature — Bear\nPower/Toughness: 2/2");let victim=g.create_object_from_definition(&creature,B,Zone::Battlefield);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);g.move_object_by_effect(victim,Zone::Graveyard).unwrap();resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert!(sagas(&g).is_empty());assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);}}
#[test]
fn actual_spell_cast_from_graveyard_resolves_normally_without_compose(){for definition in definitions("Inspect the Plot"){let mut g=game();draw_cards(&mut g);let permission=ordinary("Graveyard casting permission","Type: Enchantment\nYou may cast instant and sorcery spells from your graveyard.");g.create_object_from_definition(&permission,A,Zone::Battlefield);let id=g.create_object_from_definition(&definition,A,Zone::Graveyard);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(g.player(A).unwrap().hand.len(),1);assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);assert!(sagas(&g).is_empty());}}
#[test]
fn actual_paul_grants_compose_only_to_first_eligible_spell_from_hand(){for definition in definitions("Paul, Making Greatness"){let mut g=game();g.create_object_from_definition(&definition,A,Zone::Battlefield);let body=ordinary("Recipient spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");for expected in [1,1]{let spell=g.create_object_from_definition(&body,A,Zone::Hand);let mut queue=cast(&mut g,spell);ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut g,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g,&mut queue,&mut SelectFirstDecisionMaker).unwrap();while g.stack.len()>1{resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();}resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(sagas(&g).len(),expected);}}}

#[test]
fn multiple_instances_create_independent_sagas_linked_to_one_exile_arrival(){for mut definition in definitions("Inspect the Plot"){let extra=definition.abilities.iter().find(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if ability.canonical_model().is_some_and(|model|matches!(model.payload,ironsmith_core::StaticAbilityPayload::SpellResolutionDestinationProgram{..})))).expect("actual source has a typed terminal destination program").clone();definition.abilities.push(extra);let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();let exile=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(exile).unwrap().zone,Zone::Exile);let tokens=sagas(&g);assert_eq!(tokens.len(),2);for saga in tokens{chapter_three(&mut g,saga);}assert_eq!(g.player(A).unwrap().hand.len(),3,"both final chapters independently copy the same original exile incarnation");assert_eq!(g.object(exile).unwrap().zone,Zone::Exile);}}
#[test]
fn digital_encores_ordinary_compose_is_executable_without_its_excluded_function_clause(){let f=fixture();let card=&f["mechanics"][1]["representative_cards"][0];let original=card["original_text"].as_str().unwrap();let (prefix,_excluded)=original.split_once("Then if this spell wasn't cast from your hand, create a Function token").expect("frozen actual source excluded clause");assert!(prefix.ends_with("Draw two cards. "));let text=format!("{}\n{}",prefix.trim_end(),f["mechanics"][0]["executable_reminder"].as_str().unwrap());let scoped=serde_json::json!({"name":card["name"],"executable_text":text,"custom_keyword_identities":["compose"]});for definition in source_support::definitions(&scoped).unwrap(){let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(g.player(A).unwrap().hand.len(),2);assert_eq!(sagas(&g).len(),1);assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);}}

#[test]
fn replaced_exile_destination_cannot_create_saga_or_link_generic_child_movements(){for definition in definitions("Inspect the Plot"){let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;cast(&mut g,id);let stack=g.stack.last().unwrap().object_id;g.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(stack,A,ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(stack),Some(Zone::Stack),Some(Zone::Exile)),ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Graveyard)));resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(g.player(A).unwrap().hand.len(),1);assert!(sagas(&g).is_empty());assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);}}
struct PauseFollowup{pending:bool}
impl DecisionMaker for PauseFollowup{fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.pending=true;false}fn awaiting_choice(&self)->bool{self.pending}}
#[test]
fn private_resolution_replay_keeps_terminal_move_and_callback_prefix_atomic(){for mut definition in definitions("Inspect the Plot"){let ability=definition.abilities.iter_mut().find(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if ability.canonical_model().is_some_and(|model|matches!(model.payload,ironsmith_core::StaticAbilityPayload::SpellResolutionDestinationProgram{..})))).unwrap();let ironsmith::ability::AbilityKind::Static(static_ability)=&mut ability.kind else{unreachable!()};let mut model=static_ability.canonical_model().unwrap();let ironsmith_core::StaticAbilityPayload::SpellResolutionDestinationProgram{follow_up,..}=&mut model.payload else{unreachable!()};follow_up.extend(ironsmith::resolution::ResolutionProgram::from_effects(vec![ironsmith::Effect::new(ironsmith::effects::MayEffect::new_for_player(vec![ironsmith::Effect::gain_life(2)],ironsmith::target::PlayerFilter::You))]));*static_ability=ironsmith::static_abilities::StaticAbility::from_model(model);let mut checkpoint=game();draw_cards(&mut checkpoint);let id=checkpoint.create_object_from_definition(&definition,A,Zone::Hand);cast(&mut checkpoint,id);let mut private_attempt=checkpoint.clone();let mut pause=PauseFollowup{pending:false};resolve_stack_entry_with(&mut private_attempt,&mut pause).unwrap();assert!(pause.pending);assert!(sagas(&checkpoint).is_empty());assert_eq!(checkpoint.player(A).unwrap().hand.len(),0);assert_eq!(checkpoint.stack.len(),1,"the authoritative checkpoint is unchanged while its private proposal awaits a decision");let mut committed=checkpoint.clone();resolve_stack_entry_with(&mut committed,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(sagas(&committed).len(),1);assert_eq!(committed.player(A).unwrap().hand.len(),1);assert_eq!(committed.player(A).unwrap().life,22,"replaying an accepted callback runs its ordinary prefix once");}}


/// Choices are selected by their visible meaning, never by replacement IDs or
/// provider ordering. Buyback is announced and paid through the actual cast.
struct TerminalChoice { phrase: &'static str, saw_competition: bool }
impl DecisionMaker for TerminalChoice {
    fn decide_options(&mut self, g: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        if ctx.description == "Choose optional costs to pay" {
            return ctx.options.iter().filter(|o|o.legal && o.description.contains("Buyback")).map(|o|o.index).collect();
        }
        if ctx.description == "Choose which replacement effect to apply" {
            if let Some(choice) = ctx.options.iter().find(|o|o.legal && o.description.contains(self.phrase)) {
                self.saw_competition = true;
                return vec![choice.index];
            }
            // Freeze an outer Additionally application before selecting the
            // original draw/exile replacement in the phase-order regression.
            if let Some(choice)=ctx.options.iter().find(|o|o.legal && o.description.starts_with("Apply ")) {
                return vec![choice.index];
            }
        }
        SelectFirstDecisionMaker.decide_options(g,ctx)
    }
}
fn cast_with_terminal_choice<D: DecisionMaker>(g:&mut GameState,id:ObjectId,dm:&mut D){
    let action=compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id)).unwrap();
    let mut state=PriorityLoopState::new(2);let mut q=TriggerQueue::new();
    let mut progress=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
    for _ in 0..64 {if !state.has_pending_action(){return}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("untyped cast pause")};progress=apply_decision_context_with_dm(g,&mut q,&mut state,&ctx,dm).unwrap();}
    panic!("cast did not complete");
}
fn shuffle_count(g:&GameState)->usize{
    g.turn_store.turn_history.event_records.iter().chain(g.turn_store.turn_history.staged_event_records.iter())
        .filter(|r|r.event.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count()
}
#[test]
fn compose_and_rebound_compete_for_one_successful_terminal_move(){
    for choose_compose in [false,true] {for mut definition in definitions("Inspect the Plot") {
        definition.abilities.push(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::rebound()).in_zones(vec![Zone::Stack]));
        let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;
        let mut dm=TerminalChoice{phrase:if choose_compose{"resolution continuations"}else{"next upkeep"},saw_competition:false};
        cast_with_terminal_choice(&mut g,id,&mut dm);
        assert_eq!(g.object(g.stack.last().unwrap().object_id).unwrap().zone,Zone::Stack,"neither replacement exiles during casting");
        resolve_stack_entry_with(&mut g,&mut dm).unwrap();assert!(dm.saw_competition);
        let exiled=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);
        assert_eq!(sagas(&g).len(),usize::from(choose_compose));
        assert_eq!(g.effect_store.delayed_triggers.len(),usize::from(!choose_compose));
        if !choose_compose {assert_eq!(g.effect_store.delayed_triggers[0].target_objects,vec![exiled]);}
        assert_eq!(g.player(A).unwrap().hand.len(),1,"original source draw resolves once");
    }}
}
#[test]
fn paid_buyback_and_compose_offer_hand_or_exact_exile_with_saga(){
    for choose_compose in [false,true] {for mut definition in definitions("Inspect the Plot") {
        definition.optional_costs.push(ironsmith::cost::OptionalCost::buyback(ironsmith::cost::TotalCost::free()));
        let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;
        let mut dm=TerminalChoice{phrase:if choose_compose{"resolution continuations"}else{"owner's hand"},saw_competition:false};
        cast_with_terminal_choice(&mut g,id,&mut dm);assert!(g.stack.last().unwrap().optional_costs_paid.was_bought_back(),"real optional cost admission retained its paid receipt");
        resolve_stack_entry_with(&mut g,&mut dm).unwrap();assert!(dm.saw_competition);
        assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,if choose_compose{Zone::Exile}else{Zone::Hand});
        assert_eq!(sagas(&g).len(),usize::from(choose_compose));
        assert_eq!(g.player(A).unwrap().hand.len(),if choose_compose{1}else{2});
    }}
}
#[test]
fn native_omen_terminal_form_and_compose_offer_library_shuffle_or_saga(){
    // This supplements the actual Compose source with the existing engine Omen
    // terminal-form state, as the native Omen completion regressions do.
    for choose_compose in [false,true] {for mut definition in definitions("Inspect the Plot") {
        definition.card.subtypes.push(ironsmith::Subtype::Omen);
        let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;
        let mut dm=TerminalChoice{phrase:if choose_compose{"resolution continuations"}else{"owner's library"},saw_competition:false};
        cast_with_terminal_choice(&mut g,id,&mut dm);g.stack.last_mut().unwrap().casting_method=ironsmith::alternative_cast::CastingMethod::SplitOtherHalf;
        resolve_stack_entry_with(&mut g,&mut dm).unwrap();assert!(dm.saw_competition);
        assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,if choose_compose{Zone::Exile}else{Zone::Library});
        assert_eq!(sagas(&g).len(),usize::from(choose_compose));assert_eq!(shuffle_count(&g),usize::from(!choose_compose));
        assert_eq!(g.player(A).unwrap().hand.len(),1);
    }}
}


#[derive(Clone,Debug)]
struct OriginalCallbackBeforeAdded;
impl ironsmith::effects::EffectExecutor for OriginalCallbackBeforeAdded {
    fn execute(&self,g:&mut GameState,_:&mut ironsmith::effects::EffectContext<'_>)->Result<ironsmith::effect::EffectOutcome,ironsmith::effects::ExecutionError>{
        assert_eq!(sagas(g).len(),1,"the exact terminal original and Compose callback finish before added programs");
        assert_eq!(g.player(A).unwrap().hand.len(),2,"the replacement's original draw prefix also finishes before the callback");
        Ok(ironsmith::effect::EffectOutcome::count(0))
    }
}
#[test]
fn replacement_draw_then_exact_original_exile_retains_callback_before_added_programs(){
    for definition in definitions("Inspect the Plot") {
        let mut g=game();draw_cards(&mut g);let id=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;
        let mut dm=TerminalChoice{phrase:"resolution continuations",saw_competition:false};cast_with_terminal_choice(&mut g,id,&mut dm);
        let stack=g.stack.last().unwrap().object_id;
        let provider=g.create_object_from_definition(&ordinary("Exile observer","Type: Artifact"),A,Zone::Battlefield);
        let matcher=||ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(stack),Some(Zone::Stack),Some(Zone::Exile));
        g.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(provider,A,matcher(),ironsmith::replacement::ReplacementAction::Instead(vec![
            ironsmith::Effect::draw(1),ironsmith::Effect::new(ironsmith::effects::ExileEffect::with_spec(ironsmith::target::ChooseSpec::SpecificObject(stack))),
        ])));
        g.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(provider,A,matcher(),ironsmith::replacement::ReplacementAction::Additionally(vec![ironsmith::Effect::new(OriginalCallbackBeforeAdded)])));
        resolve_stack_entry_with(&mut g,&mut dm).unwrap();
        assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert_eq!(sagas(&g).len(),1);assert_eq!(g.player(A).unwrap().hand.len(),2);
    }
}
