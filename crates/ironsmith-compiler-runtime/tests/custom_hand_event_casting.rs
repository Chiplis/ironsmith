//! Event-time hand reveal rights, followed by a separately resolving cast offer.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,compute_legal_actions,LegalAction};
use ironsmith::effects::{execute_effect,EffectContext};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(event:&str,price:&str)->[CardDefinition;2] {
    let text=format!("Mana cost: {{4}}{{U}}\nType: Sorcery\nYou may reveal this card from your hand as {event}. When you reveal this card this way, you may cast it by {price} rather than its mana cost.\nYou gain 2 life.");
    text_definitions("Event price",&text)
}
fn creature(game:&mut GameState,owner:PlayerId)->ObjectId {
    let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent creature","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
    game.create_object_from_definition(&definition,owner,Zone::Battlefield)
}
fn game()->GameState {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    game.turn.active_player=B;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareBlockers);game
}
struct Choices(bool);
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}
    fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(game,ctx)}
    fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{SelectFirstDecisionMaker.decide_targets(game,ctx)}
}
fn collect(game:&mut GameState,dm:&mut impl DecisionMaker) {
    let mut queue=ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
}
fn block(game:&mut GameState,owner:PlayerId) {
    let blocker=creature(game,owner);let attacker=creature(game,if owner==A{B}else{A});
    let event=ironsmith::events::CreatureBlockedEvent::with_snapshots(blocker,attacker,ironsmith::snapshot::ObjectSnapshot::from_object_id(game,blocker).unwrap(),ironsmith::snapshot::ObjectSnapshot::from_object_id(game,attacker).unwrap());
    game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new_with_provenance(event,Default::default()));
}
#[test]
fn block_reveal_right_is_frozen_before_late_arrivals_and_casts_sorcery_only_from_its_trigger() {
    for definition in definitions("a creature you control blocks or becomes blocked","paying {U}") {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
        assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==card)),"event price is not a priority alternative");
        block(&mut game,A);
        let late=game.create_object_from_definition(&definition,A,Zone::Hand);let late_stable=game.object(late).unwrap().stable_id;
        let mut dm=Choices(true);collect(&mut game,&mut dm);
        assert_eq!(game.stack.len(),1,"only the card actually in hand at the event reveals and triggers");
        assert_eq!(game.object(card).unwrap().zone,Zone::Hand,"reveal does not itself cast");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let cast=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(cast).unwrap().zone,Zone::Stack);
        assert!(game.object(cast).unwrap().optional_costs_paid.was_paid_label(definition.alternative_casts[0].name()),"captured named price is retained on the cast receipt");
        assert_eq!(game.stack.len(),1,"sorcery is cast during its trigger on the opponent's turn");
        assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,22);
        assert_eq!(game.object(game.find_object_by_stable_id(late_stable).unwrap()).unwrap().zone,Zone::Hand);
    }
}
#[test]
fn opponents_events_and_declined_reveals_never_grant_a_price() {
    for definition in definitions("a creature you control blocks or becomes blocked","paying {U}") {for (owner,accept) in [(B,true),(A,false)] {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
        block(&mut game,owner);collect(&mut game,&mut Choices(accept));assert!(game.stack.is_empty());assert_eq!(game.object(card).unwrap().zone,Zone::Hand);
        assert_eq!(game.player(A).unwrap().mana_pool.total(),1);
    }}
}
#[test]
fn revealed_hand_price_does_not_follow_a_new_hand_incarnation() {
    for definition in definitions("a creature you control blocks or becomes blocked","paying {U}") {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
        block(&mut game,A);let mut dm=Choices(true);collect(&mut game,&mut dm);assert_eq!(game.stack.len(),1);
        let grave=game.move_object_by_effect(card,Zone::Graveyard).unwrap();let _=game.move_object_by_effect(grave,Zone::Hand).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert!(game.stack.is_empty());assert_eq!(game.player(A).unwrap().mana_pool.total(),1);
    }
}
#[test]
fn leave_event_price_retains_nonmana_exile_payment_until_trigger_resolution() {
    for definition in definitions("a permanent you control leaves the battlefield","exiling a creature card from your graveyard") {
        let mut game=game();
        // Prepare the graveyard price before the hand-window authority exists.
        let payment=creature(&mut game,A);let payment=game.move_object_by_effect(payment,Zone::Graveyard).unwrap();let payment_stable=game.object(payment).unwrap().stable_id;
        let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;
        let leaving=creature(&mut game,A);
        let effects=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(leaving),Zone::Graveyard,false));
        let outcome=execute_effect(&mut game,&effects,&mut EffectContext::new_default(leaving,A).with_decision_maker(&mut Choices(true))).unwrap();
        for event in outcome.events {game.queue_trigger_event(event.provenance(),event);}
        let mut dm=Choices(true);collect(&mut game,&mut dm);assert_eq!(game.stack.len(),1);
        assert_eq!(game.object(game.find_object_by_stable_id(payment_stable).unwrap()).unwrap().zone,Zone::Graveyard);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Stack);
        assert!(game.exile.iter().any(|id|game.object(*id).unwrap().stable_id==payment_stable));
    }
}
#[test]
fn granted_leave_window_retains_full_price_when_its_own_grantor_leaves() {
    let text="Type: Enchantment\nNonland cards in your hand have custom keyword \"escape hatch\", and \"You may reveal this card from your hand as a permanent you control leaves the battlefield. When you reveal this card this way, you may cast it by paying {4} rather than its mana cost.\".";
    for grant in text_definitions("Permission source",text) {
        assert!(grant.spell_effect.is_none(),"this is a persistent hand permission");
        let mut game=game();let source=game.create_object_from_definition(&grant,A,Zone::Battlefield);
        let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary hand card","Mana cost: {4}{U}\nType: Sorcery\nYou gain 2 life.",false).unwrap();
        let card=game.create_object_from_definition(&spell,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,4);
        let mut dm=Choices(true);
        let effect=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Graveyard,false));
        let outcome=execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
        for event in outcome.events {game.queue_trigger_event(event.provenance(),event);}
        collect(&mut game,&mut dm);assert_eq!(game.stack.len(),1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let cast=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(cast).unwrap().zone,Zone::Stack);
        assert!(game.object(cast).unwrap().optional_costs_paid.was_paid_label("escape hatch"));
        assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
    }
}

struct PrivateOpening {pause:bool,pending:bool,requested:Vec<Vec<ObjectId>>}
impl DecisionMaker for PrivateOpening {
    fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool {true}
    fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
        if ctx.reveal_policy==ironsmith::decisions::context::SelectionRevealPolicy::Public && ctx.player==A {
            let chosen=ctx.candidates.iter().map(|candidate|candidate.id).collect::<Vec<_>>();self.requested.push(chosen.clone());self.pending=self.pause;chosen
        } else {SelectFirstDecisionMaker.decide_objects(game,ctx)}
    }
    fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{SelectFirstDecisionMaker.decide_targets(game,ctx)}
    fn awaiting_choice(&self)->bool {self.pending}
}
#[test]
fn private_filtered_grant_survives_its_own_departure_and_authenticated_opening_on_every_peer() {
    let text="Type: Enchantment\nCreature cards in your hand have custom keyword \"escape hatch\", and \"You may reveal this card from your hand as a permanent you control leaves the battlefield. When you reveal this card this way, you may cast it by paying {4} rather than its mana cost.\".";
    for grant in text_definitions("Private permission source",text) {for peer in [false,true] {for eligible in [false,true] {
        let mut game=game();let source=game.create_object_from_definition(&grant,A,Zone::Battlefield);
        let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Private event card",if eligible {"Mana cost: {8}\nType: Creature\nPower/Toughness: 1/1"}else{"Mana cost: {8}\nType: Sorcery\nYou gain 1 life."},false).unwrap();
        let card=game.create_hidden_card_placeholder(A,Zone::Hand,0,"event-hand-card".into());let stable=game.object(card).unwrap().stable_id;
        if !peer {game.reveal_hidden_card_with_definition(card,&definition).unwrap();}
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,4);
        game.move_object_by_effect(source,Zone::Graveyard).unwrap();
        let mut dm=PrivateOpening {pause:true,pending:false,requested:Vec::new()};let mut queue=ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert!(dm.pending);assert!(game.stack.is_empty());assert_eq!(game.player(A).unwrap().mana_pool.total(),4);
        assert_eq!(dm.requested,vec![vec![card]]);assert!(game.object(source).is_none(),"disclosure resumes after the already-completed departure");
        if peer {game.reveal_hidden_card_with_definition(card,&definition).unwrap();}
        dm.pause=false;dm.pending=false;collect(&mut game,&mut dm);
        assert_eq!(game.stack.len(),usize::from(eligible),"opening validates the frozen creature filter, including on a former placeholder");
        if eligible {
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();let cast=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(cast).unwrap().zone,Zone::Stack);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        } else {assert_eq!(game.object(card).unwrap().zone,Zone::Hand);assert_eq!(game.player(A).unwrap().mana_pool.total(),4);}
    }}}
}

#[path="custom_final_next_wave_support.rs"] mod source_support;
fn text_definitions(name:&str,text:&str)->[CardDefinition;2] {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text,"custom_keyword_identities":[],"custom_keyword_payments":[]})).unwrap()
}
fn source_definitions(name:&str)->[CardDefinition;2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-middle.json")).unwrap();
    let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();source_support::definitions(card).unwrap()
}
fn cast_normal(game:&mut GameState,card:ObjectId,dm:&mut Choices) {
    let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell {spell_id,casting_method:ironsmith::alternative_cast::CastingMethod::Normal,..} if *spell_id==card)).unwrap();
    let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
    let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),dm).unwrap();
    for _ in 0..32 {if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();}assert!(!state.has_pending_action());
}
#[test]
fn full_source_paranoia_price_receipt_controls_its_real_entry_draw_replacement() {
    for definition in source_definitions("Kofe-District Scholar") {for event_price in [false,true] {
        let mut game=game();if !event_price {game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;}
        let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;
        let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Library filler","Type: Artifact",false).unwrap();for _ in 0..2 {game.create_object_from_definition(&filler,A,Zone::Library);}
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,2);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);let mut dm=Choices(true);
        if event_price {let departing=creature(&mut game,A);game.move_object_by_effect(departing,Zone::Graveyard).unwrap();collect(&mut game,&mut dm);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
        else {cast_normal(&mut game,card,&mut dm);}
        let spell=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(spell).unwrap().zone,Zone::Stack);assert_eq!(game.object(spell).unwrap().optional_costs_paid.was_paid_label("paranoia"),event_price);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();collect(&mut game,&mut dm);while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
        assert_eq!(game.player(A).unwrap().hand.len(),if event_price {2}else{1});assert_eq!(game.player(A).unwrap().mana_pool.total(),if event_price {0}else{1});
    }}
}
#[test]
fn full_source_quickdraw_casts_its_sorcery_on_opponents_turn_for_the_full_selected_price() {
    for definition in source_definitions("Crossfire") {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);
        assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell {spell_id,..} if *spell_id==card)));
        block(&mut game,A);let participants=game.battlefield.clone();let mut dm=Choices(true);collect(&mut game,&mut dm);assert_eq!(game.stack.len(),1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();let spell=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(spell).unwrap().zone,Zone::Stack);assert!(game.object(spell).unwrap().optional_costs_paid.was_paid_label("quickdraw"));assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"the event permission costs2R even though the printed sorcery costsR");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();for participant in participants {assert_eq!(game.damage_on(participant),1);}
    }
}
