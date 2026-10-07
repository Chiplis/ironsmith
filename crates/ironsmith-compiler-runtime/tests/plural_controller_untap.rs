//! Complete frozen untap bodies; authored but deliberately unexecuted.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, ManaPaymentContext, SelectObjectsContext, TargetsContext, ViewCardsContext};
use ironsmith::effects::{CantEffect, EffectContext, EffectExecutor};
use ironsmith::effect::{Restriction, Until};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with, apply_attacker_declarations};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/plural_controller_untap.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = artifact.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap(); assert_eq!(artifact, restored);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions { assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition)); }
    definitions
}
fn game() -> GameState {
    let mut g=GameState::new(vec!["A".into(),"B".into(),"C".into()],30);
    g.turn.phase=ironsmith::Phase::FirstMain; g.turn.step=None; g.turn.priority_player=Some(A);
    g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Blue,40);
    for _ in 0..12 { object(&mut g,A,Zone::Library,"Library land","Type: Land"); }
    g
}
fn object(g: &mut GameState,p:PlayerId,z:Zone,name:&str,text:&str)->ObjectId {
    g.create_object_from_definition(&compile_to_runtime_definition(name,text,false).unwrap(),p,z)
}
fn creature(g:&mut GameState,p:PlayerId)->ObjectId { object(g,p,Zone::Battlefield,"Witness","Type: Creature — Bear\nPower/Toughness: 2/6") }
#[derive(Default)]
struct Choices { targets:Vec<Target>, accept:bool, views:usize }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self,_:&GameState,_:&TargetsContext)->Vec<Target>{self.targets.clone()}
    fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{self.accept}
    fn decide_mana_payment(&mut self,_:&GameState,c:&ManaPaymentContext)->ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm{plan_id:c.plan.id,request_hash:c.plan.request_hash}
    }
    fn decide_objects(&mut self,_:&GameState,c:&SelectObjectsContext)->Vec<ObjectId> {
        c.candidates.iter().filter(|x|x.legal).take(c.min).map(|x|x.id).collect()
    }
    fn view_cards(&mut self,_:&GameState,_:PlayerId,_:&[ObjectId],_:&ViewCardsContext){self.views+=1;}
}
fn cast(g:&mut GameState,definition:&CardDefinition,dm:&mut Choices)->ObjectId {
    let hand=g.create_object_from_definition(definition,A,Zone::Hand);
    g.turn.priority_player=Some(A);
    let mut q=TriggerQueue::new();let mut state=PriorityLoopState::new(g.players.len());
    let mut progress=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id:hand,from_zone:Zone::Hand,casting_method:ironsmith::alternative_cast::CastingMethod::Normal,
    }),dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none(){break;}
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else {panic!("{progress:?}")};
        progress=apply_decision_context_with_dm(g,&mut q,&mut state,&ctx,dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(g,&mut q,dm).unwrap();
    g.stack.iter().find(|entry|!entry.is_ability).unwrap().object_id
}
fn settle(g:&mut GameState,dm:&mut Choices){
    for _ in 0..32 {if g.stack.is_empty(){return;}resolve_stack_entry_with(g,dm).unwrap();put_triggers_on_stack_with_dm(g,&mut TriggerQueue::new(),dm).unwrap();}
    panic!("unsettled stack");
}
fn untap(g:&mut GameState,p:PlayerId){
    g.turn.turn_number+=1;g.turn.active_player=p;g.turn.phase=ironsmith::Phase::Beginning;g.turn.step=Some(ironsmith::game_state::Step::Untap);
    ironsmith::turn::execute_untap_step_with(g,&mut SelectFirstDecisionMaker).unwrap();
}
#[test]
fn storm_preserves_zero_partial_and_all_illegal_targets_and_scry(){
    for definition in definitions("Sudden Storm") {for selected in 0..=2 {for removed in 0..=selected {
        let mut g=game();let first=creature(&mut g,B);let second=creature(&mut g,C);
        g.tap(first); // Already tapped remains part of the affected set.
        let mut dm=Choices{targets:[first,second].into_iter().take(selected).map(Target::Object).collect(),..Default::default()};
        cast(&mut g,&definition,&mut dm);
        for id in [first,second].into_iter().take(removed){g.move_object_by_effect(id,Zone::Graveyard).unwrap();}
        settle(&mut g,&mut dm);
        assert_eq!(dm.views,usize::from(selected==0 || removed<selected));
        for (index,id) in [first,second].into_iter().enumerate().take(selected).skip(removed){
            let p=if index==0{B}else{C};assert!(g.is_tapped(id));untap(&mut g,p);assert!(g.is_tapped(id));untap(&mut g,p);assert!(!g.is_tapped(id));
        }
        let late=creature(&mut g,B);g.tap(late);untap(&mut g,B);assert!(!g.is_tapped(late));
    }}}
}
#[test]
fn storm_tracks_new_controller_not_old_player_and_does_not_follow_blink(){
    for definition in definitions("Sudden Storm") {
        let mut g=game();let target=creature(&mut g,B);let mut dm=Choices{targets:vec![Target::Object(target)],..Default::default()};
        cast(&mut g,&definition,&mut dm);settle(&mut g,&mut dm);
        g.set_current_controller(target,C).unwrap();untap(&mut g,B);assert!(g.is_tapped(target));
        untap(&mut g,C);assert!(g.is_tapped(target));untap(&mut g,C);assert!(!g.is_tapped(target));
        g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;g.turn.active_player=A;
        cast(&mut g,&definition,&mut dm);settle(&mut g,&mut dm);
        let hand=g.move_object_by_effect(target,Zone::Hand).unwrap();let returned=g.move_object_by_effect(hand,Zone::Battlefield).unwrap();
        g.tap(returned);untap(&mut g,C);assert!(!g.is_tapped(returned));
    }
}
#[test]
fn dragon_flash_optional_target_and_all_illegal_trigger_are_distinct(){
    for definition in definitions("Dragon Turtle") {for mode in 0..3 {
        let mut g=game();g.turn.active_player=B;g.turn.phase=ironsmith::Phase::Ending;
        let target=creature(&mut g,B);let mut dm=Choices{targets:if mode==0{vec![]}else{vec![Target::Object(target)]},..Default::default()};
        let spell=cast(&mut g,&definition,&mut dm);resolve_stack_entry_with(&mut g,&mut dm).unwrap();
        let turtle=*g.battlefield.iter().find(|id|g.object(**id).unwrap().name=="Dragon Turtle").unwrap();assert_ne!(spell,turtle);
        put_triggers_on_stack_with_dm(&mut g,&mut TriggerQueue::new(),&mut dm).unwrap();
        if mode==2{g.move_object_by_effect(target,Zone::Graveyard).unwrap();}
        settle(&mut g,&mut dm);assert_eq!(g.is_tapped(turtle),mode!=2);
        if mode!=2{untap(&mut g,A);assert!(g.is_tapped(turtle));untap(&mut g,A);assert!(!g.is_tapped(turtle));}
        if mode==1{untap(&mut g,B);assert!(g.is_tapped(target));untap(&mut g,B);assert!(!g.is_tapped(target));}
    }}
}
#[test]
fn breaching_requires_a_hand_cast_and_locks_only_nonblue_original_creatures(){
    for definition in definitions("Breaching Leviathan") {for hand_cast in [false,true] {
        let mut g=game();let own=creature(&mut g,A);let enemy=creature(&mut g,B);
        let blue=object(&mut g,B,Zone::Battlefield,"Blue","Mana cost: {U}\nType: Creature — Drake\nPower/Toughness: 2/3");
        let mut dm=Choices::default();
        if hand_cast {cast(&mut g,&definition,&mut dm);settle(&mut g,&mut dm);} else {
            let grave=g.create_object_from_definition(&definition,A,Zone::Graveyard);
            let receipt=g.move_object_with_etb_processing_with_dm(grave,Zone::Battlefield,&mut dm).unwrap();assert!(!receipt.pending);
            put_triggers_on_stack_with_dm(&mut g,&mut TriggerQueue::new(),&mut dm).unwrap();settle(&mut g,&mut dm);
        }
        assert_eq!(g.is_tapped(own),hand_cast);assert_eq!(g.is_tapped(enemy),hand_cast);assert!(!g.is_tapped(blue));
        let late=creature(&mut g,B);g.tap(late);untap(&mut g,B);assert!(!g.is_tapped(late));assert_eq!(g.is_tapped(enemy),hand_cast);
    }}
}
#[test]
fn lorthos_attack_keeps_payment_and_targets_after_source_departure(){
    for definition in definitions("Lorthos, the Tidemaker") {for accept in [false,true] {
        let mut g=game();let lorthos=g.create_object_from_definition(&definition,A,Zone::Battlefield);let enemy=creature(&mut g,B);
        g.remove_summoning_sickness(lorthos);g.turn.phase=ironsmith::Phase::Combat;g.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat=ironsmith::combat_state::CombatState::default();let mut q=TriggerQueue::new();
        apply_attacker_declarations(&mut g,&mut combat,&mut q,&[AttackerDeclaration{creature:lorthos,target:ironsmith::combat_state::AttackTarget::Player(B)}]).unwrap();g.combat=Some(combat);
        let mut dm=Choices{targets:vec![Target::Object(enemy)],accept,..Default::default()};put_triggers_on_stack_with_dm(&mut g,&mut q,&mut dm).unwrap();
        g.move_object_by_effect(lorthos,Zone::Graveyard).unwrap();settle(&mut g,&mut dm);assert_eq!(g.is_tapped(enemy),accept);
        untap(&mut g,B);assert_eq!(g.is_tapped(enemy),accept);untap(&mut g,B);assert!(!g.is_tapped(enemy));
    }}
}
#[test]
fn cone_keeps_all_three_dice_bodies_and_timed_entry_restriction(){
    for definition in definitions("Cone of Cold") {for result in [1,10,20] {
        let mut g=game();let own=creature(&mut g,A);let enemy=creature(&mut g,B);let other=creature(&mut g,C);
        g.force_next_die_roll(result);let mut dm=Choices::default();cast(&mut g,&definition,&mut dm);settle(&mut g,&mut dm);
        assert!(!g.is_tapped(own));assert!(g.is_tapped(enemy));assert!(g.is_tapped(other));
        untap(&mut g,B);assert_eq!(g.is_tapped(enemy),result>=10);untap(&mut g,C);assert_eq!(g.is_tapped(other),result>=10);
        let def=compile_to_runtime_definition("Late","Type: Creature — Bear\nPower/Toughness: 2/3",false).unwrap();
        let hand=g.create_object_from_definition(&def,B,Zone::Hand);let receipt=g.move_object_with_etb_processing_with_dm(hand,Zone::Battlefield,&mut dm).unwrap();
        let late=receipt.original.into_result().unwrap().new_id;assert_eq!(g.is_tapped(late),result==20);
        untap(&mut g,A);
        let hand=g.create_object_from_definition(&def,B,Zone::Hand);let receipt=g.move_object_with_etb_processing_with_dm(hand,Zone::Battlefield,&mut dm).unwrap();
        assert!(!g.is_tapped(receipt.original.into_result().unwrap().new_id));
    }}
}
#[test]
fn fixed_your_step_and_exert_do_not_follow_new_controller(){
    let mut g=game();let target=creature(&mut g,A);g.tap(target);let source=creature(&mut g,A);
    CantEffect::new(Restriction::untap(ironsmith::target::ObjectFilter::specific(target)),Until::YourNextUntapStep)
        .execute(&mut g,&mut EffectContext::new(source,A,&mut SelectFirstDecisionMaker)).unwrap();
    g.set_current_controller(target,B).unwrap();untap(&mut g,B);assert!(!g.is_tapped(target));
    g.tap(target);untap(&mut g,A);untap(&mut g,B);assert!(!g.is_tapped(target));
    // The existing native exert owner is deliberately fixed to its actor.
    g.set_current_controller(target,A).unwrap();
    let exert=ironsmith::effects::ExertCostEffect::new("Exert this creature");
    exert.execute(&mut g,&mut EffectContext::new(target,A,&mut SelectFirstDecisionMaker)).unwrap();
    g.set_current_controller(target,B).unwrap();g.tap(target);untap(&mut g,B);assert!(!g.is_tapped(target));
}
#[test]
fn exact_step_binding_survives_native_clone_phasing_source_and_controller_departure(){
    for definition in definitions("Sudden Storm") {
        let mut g=game();let target=creature(&mut g,B);let mut dm=Choices{targets:vec![Target::Object(target)],..Default::default()};
        cast(&mut g,&definition,&mut dm);settle(&mut g,&mut dm);
        assert!(g.effect_store.restriction_effects.iter().any(|effect|effect.untap_step_object==Some(target)));
        let mut cloned=g.clone();cloned.set_current_controller(target,C).unwrap();
        untap(&mut cloned,B);assert!(cloned.is_tapped(target));untap(&mut cloned,C);assert!(cloned.is_tapped(target));
        // The spell's controller leaves; its already-resolved object-bound
        // restriction is not a duration tied to that departed player's turn.
        g.leave_game(A).unwrap();g.phase_out(target);untap(&mut g,B);
        assert!(!g.is_phased_out(target));assert!(g.is_tapped(target));untap(&mut g,B);assert!(!g.is_tapped(target));
    }
}
#[test]
fn known_empty_native_restriction_never_affects_later_permanents(){
    let mut g=game();let source=creature(&mut g,A);
    let mut dm=SelectFirstDecisionMaker;let mut ctx=EffectContext::new(source,A,&mut dm);
    ctx.tagged_objects.insert(ironsmith::tag::TagKey::from("empty_result"),vec![]);
    CantEffect::new(Restriction::untap(ironsmith::target::ObjectFilter::tagged("empty_result")),Until::ControllersNextUntapStep)
        .execute(&mut g,&mut ctx).unwrap();
    assert!(g.effect_store.restriction_effects.is_empty());
    let late=creature(&mut g,B);g.tap(late);untap(&mut g,B);assert!(!g.is_tapped(late));
}

#[test]
fn code_of_constraint_keeps_pump_draw_and_cast_time_addendum(){
    for definition in definitions("Code of Constraint") { for during_main in [false,true] {
        let mut g=game();let target=creature(&mut g,B);
        if !during_main {g.turn.phase=ironsmith::Phase::Combat;g.turn.step=Some(ironsmith::game_state::Step::BeginCombat);}
        let mut dm=Choices{targets:vec![Target::Object(target)],..Default::default()};
        cast(&mut g,&definition,&mut dm);
        let hand_before=g.player(A).unwrap().hand.len();
        // Change phase before resolution: the condition belongs to the cast,
        // not the phase in which its conditional followup resolves.
        g.turn.phase=if during_main{ironsmith::Phase::Combat}else{ironsmith::Phase::FirstMain};g.turn.step=None;
        settle(&mut g,&mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(),hand_before+1);
        assert_eq!(g.try_current_characteristics(target).unwrap().unwrap().power,Some(-2));
        assert_eq!(g.is_tapped(target),during_main);
        if during_main {untap(&mut g,B);assert!(g.is_tapped(target));untap(&mut g,B);assert!(!g.is_tapped(target));}
    }}
}

#[test]
fn draw_replacement_blink_cannot_move_later_tap_or_freeze_to_a_new_incarnation(){
    use ironsmith::Effect;
    use ironsmith::target::ChooseSpec;
    for definition in definitions("Code of Constraint") {
        let mut g=game();let target=creature(&mut g,B);
        let stable=g.object(target).unwrap().stable_id;
        let witness=creature(&mut g,A);
        g.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(witness,A,
                ironsmith::events::cards::matchers::WouldDrawCardMatcher::you(),
                ironsmith::replacement::ReplacementAction::Additionally(vec![
                    Effect::exile(ChooseSpec::SpecificObject(target)).tag("untap_blink"),
                    Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::Tagged("untap_blink".into()),Zone::Battlefield,false).under_owner_control()),
                ])));
        let mut dm=Choices{targets:vec![Target::Object(target)],..Default::default()};
        cast(&mut g,&definition,&mut dm);settle(&mut g,&mut dm);
        let returned=*g.battlefield.iter().find(|id|g.object(**id).unwrap().stable_id==stable).unwrap();
        assert_ne!(returned,target);assert!(!g.is_tapped(returned));
        assert!(!g.effect_store.restriction_effects.iter().any(|effect|effect.untap_step_object==Some(returned)));
        g.tap(returned);untap(&mut g,B);assert!(!g.is_tapped(returned));
    }
}

#[test]
fn native_retained_tap_set_keeps_its_exact_incarnation_before_registration(){
    let mut g=game();let source=creature(&mut g,A);let old=creature(&mut g,B);
    let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(g.object(old).unwrap(),&g);
    g.tap(old);
    let exile=g.move_object_by_effect(old,Zone::Exile).unwrap();let returned=g.move_object_by_effect(exile,Zone::Battlefield).unwrap();
    let mut dm=SelectFirstDecisionMaker;let mut ctx=EffectContext::new(source,A,&mut dm);
    ctx.tagged_objects.insert("original_tap".into(),vec![snapshot]);
    CantEffect::new(Restriction::untap(ironsmith::target::ObjectFilter::tagged("original_tap")),Until::ControllersNextUntapStep)
        .execute(&mut g,&mut ctx).unwrap();
    assert!(g.effect_store.restriction_effects.is_empty());
    g.tap(returned);untap(&mut g,B);assert!(!g.is_tapped(returned));
}
#[test]
fn a_skipped_untap_does_not_consume_the_object_bound_next_occurrence(){
    for definition in definitions("Sudden Storm") {
        let mut g=game();let target=creature(&mut g,B);let mut dm=Choices{targets:vec![Target::Object(target)],..Default::default()};
        cast(&mut g,&definition,&mut dm);settle(&mut g,&mut dm);
        let source=g.new_object_id();
        ironsmith::effects::SkipScheduledEffect{
            player:ironsmith::target::PlayerFilter::Specific(B),
            kind:ironsmith_core::ScheduledSkipKind::UntapStep,count:1,
        }.execute(&mut g,&mut EffectContext::new_default(source,A)).unwrap();
        g.next_turn();assert_eq!(g.turn.active_player,B);
        let mut runner=ironsmith::turn_runner::TurnRunner::new();
        assert!(matches!(runner.advance(&mut g,&mut TriggerQueue::new()).unwrap(),ironsmith::turn_runner::TurnAction::Continue));
        assert!(g.is_tapped(target));
        assert!(g.effect_store.restriction_effects.iter().any(|effect|effect.untap_step_object==Some(target)&&!effect.consumed_next_untap));
        untap(&mut g,B);assert!(g.is_tapped(target));untap(&mut g,B);assert!(!g.is_tapped(target));
    }
}
