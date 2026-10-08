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
const ORIGINAL_CANDIDATES: [(&str, &str); 5] = [
    ("Breaching Leviathan", "7d249b16-33ef-463e-9e5d-0b84d434f093"),
    ("Cone of Cold", "56fd0752-9fb0-4c05-90e5-c3b268b57c6f"),
    ("Dragon Turtle", "a3270e64-38d2-49e5-8a1e-a5e81205776b"),
    ("Lorthos, the Tidemaker", "d9fd4ff3-5ada-40f4-b949-6fe65624a0c4"),
    ("Sudden Storm", "32e915fb-3836-4bed-93b1-61c9c3951ad1"),
];
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/plural_controller_untap.json.fixture")).unwrap()
}
fn assert_metadata(row: &serde_json::Value, definition: &CardDefinition) {
    use ironsmith::{CardType, Subtype, Supertype};
    let name = row["name"].as_str().unwrap();
    assert_eq!(definition.card.name, name);
    assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), row["mana_cost"].as_str().unwrap());
    let (kind, subtypes, legendary) = match name {
        "Breaching Leviathan" => (CardType::Creature, vec![Subtype::Leviathan], false),
        "Cone of Cold" => (CardType::Sorcery, vec![], false),
        "Dragon Turtle" => (CardType::Creature, vec![Subtype::Dragon, Subtype::Turtle], false),
        "Lorthos, the Tidemaker" => (CardType::Creature, vec![Subtype::Octopus], true),
        "Sudden Storm" | "Code of Constraint" | "Send to Sleep" | "Icy Blast" => (CardType::Instant, vec![], false),
        _ => panic!("unexpected frozen body: {name}"),
    };
    assert_eq!(definition.card.card_types, vec![kind]);
    assert_eq!(definition.card.subtypes, subtypes);
    assert_eq!(definition.card.supertypes, if legendary { vec![Supertype::Legendary] } else { vec![] });
    if let Some(power) = row["power"].as_str() {
        let stats = definition.card.power_toughness.as_ref().unwrap();
        assert_eq!(stats.power.to_string(), power);
        assert_eq!(stats.toughness.to_string(), row["toughness"].as_str().unwrap());
    } else {
        assert!(definition.card.power_toughness.is_none());
    }
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
}

#[test]
fn all_five_original_bodies_compile_directly_with_complete_metadata() {
    let rows = fixtures();
    for (name, oracle_id) in ORIGINAL_CANDIDATES {
        let row = rows.iter().find(|row| row["oracle_id"] == oracle_id).unwrap();
        assert_eq!(row["name"], name);
        let mut source = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
        if let Some(power) = row["power"].as_str() {
            source.push_str(&format!("Power/Toughness: {power}/{}\n", row["toughness"].as_str().unwrap()));
        }
        source.push_str(row["oracle_text"].as_str().unwrap());
        assert_eq!(source, row["text"].as_str().unwrap(), "the original body may not be rewritten");
        let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, source, false));
        let direct = direct.unwrap_or_else(|error| panic!("{name} ({oracle_id}): {error}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        assert_metadata(row, &direct);
    }
}

#[test]
fn all_five_original_bodies_compile_independently_through_artifact_transport() {
    let rows = fixtures();
    for (name, oracle_id) in ORIGINAL_CANDIDATES {
        let row = rows.iter().find(|row| row["oracle_id"] == oracle_id).unwrap();
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, row["text"].as_str().unwrap(), false));
        // The returned companion definition is deliberately not called direct.
        let (artifact, _) = result.unwrap_or_else(|error| panic!("{name} ({oracle_id}): {error}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(artifact, restored);
        let definition = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
        assert_metadata(row, &definition);
    }
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows = fixtures();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = artifact.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap(); assert_eq!(artifact, restored);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    assert_eq!(definitions[0].canonical_text, definitions[1].canonical_text);
    assert_eq!(definitions[0].ability_labels, definitions[1].ability_labels);
    for definition in &definitions { assert_metadata(row, definition); }
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
struct Choices { x:u32, targets:Vec<Target>, accept:bool, views:usize, target_contexts:Vec<TargetsContext> }
impl DecisionMaker for Choices {
    fn decide_number(&mut self, g:&GameState, c:&ironsmith::decisions::context::NumberContext)->u32 {
        if c.is_x_value { self.x } else { SelectFirstDecisionMaker.decide_number(g,c) }
    }
    fn decide_targets(&mut self,_:&GameState,context:&TargetsContext)->Vec<Target>{self.target_contexts.push(context.clone());self.targets.clone()}
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
        let requirements=&dm.target_contexts.last().unwrap().requirements;
        assert_eq!(requirements.len(),1);assert_eq!(requirements[0].min_targets,0);assert_eq!(requirements[0].max_targets,Some(2));
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
        assert_eq!(g.current_controller(returned),Some(B));
        g.tap(returned);untap(&mut g,B);assert!(!g.is_tapped(returned));
    }
}
#[test]
fn dragon_flash_optional_target_and_all_illegal_trigger_are_distinct(){
    for definition in definitions("Dragon Turtle") {for mode in 0..3 {
        let mut g=game();g.turn.active_player=B;g.turn.phase=ironsmith::Phase::Ending;
        let target=creature(&mut g,B);let own=creature(&mut g,A);let land=object(&mut g,B,Zone::Battlefield,"Noncreature","Type: Land");
        let mut dm=Choices{targets:if mode==0{vec![]}else{vec![Target::Object(target)]},..Default::default()};
        let spell=cast(&mut g,&definition,&mut dm);resolve_stack_entry_with(&mut g,&mut dm).unwrap();
        let turtle=*g.battlefield.iter().find(|id|g.object(**id).unwrap().name=="Dragon Turtle").unwrap();assert_ne!(spell,turtle);
        put_triggers_on_stack_with_dm(&mut g,&mut TriggerQueue::new(),&mut dm).unwrap();
        let requirements=&dm.target_contexts.last().unwrap().requirements;
        assert_eq!(requirements.len(),1);assert_eq!(requirements[0].min_targets,0);assert_eq!(requirements[0].max_targets,Some(1));
        assert!(requirements[0].legal_targets.contains(&Target::Object(target)));
        for illegal in [own,land,turtle]{assert!(!requirements[0].legal_targets.contains(&Target::Object(illegal)));}
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
    for definition in definitions("Lorthos, the Tidemaker") {for (accept,mana) in [(false,40),(true,40),(true,7)] {for (selected,removed) in [(0,0),(1,0),(1,1),(8,0),(8,3),(8,8)] {
        let mut g=game();let lorthos=g.create_object_from_definition(&definition,A,Zone::Battlefield);
        g.player_mut(A).unwrap().mana_pool.blue=mana;
        let targets=(0..8).map(|index|if index%2==0{creature(&mut g,B)}else{object(&mut g,C,Zone::Battlefield,"Target land","Type: Land")}).collect::<Vec<_>>();
        g.tap(targets[0]); // Already-tapped permanents remain legal recipients.
        g.remove_summoning_sickness(lorthos);g.turn.phase=ironsmith::Phase::Combat;g.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat=ironsmith::combat_state::CombatState::default();let mut q=TriggerQueue::new();
        apply_attacker_declarations(&mut g,&mut combat,&mut q,&[AttackerDeclaration{creature:lorthos,target:ironsmith::combat_state::AttackTarget::Player(B)}]).unwrap();g.combat=Some(combat);
        let mut dm=Choices{targets:targets.iter().take(selected).copied().map(Target::Object).collect(),accept,..Default::default()};put_triggers_on_stack_with_dm(&mut g,&mut q,&mut dm).unwrap();
        let requirements=&dm.target_contexts.last().unwrap().requirements;
        assert_eq!(requirements.len(),1);assert_eq!(requirements[0].min_targets,0);assert_eq!(requirements[0].max_targets,Some(8));
        for id in &targets{assert!(requirements[0].legal_targets.contains(&Target::Object(*id)));}
        let mana_before=g.player(A).unwrap().mana_pool.total();
        for id in targets.iter().take(removed){g.move_object_by_effect(*id,Zone::Graveyard).unwrap();}
        g.move_object_by_effect(lorthos,Zone::Graveyard).unwrap();settle(&mut g,&mut dm);
        let resolves=selected==0 || removed<selected;
        let paid=accept && mana>=8 && resolves;
        assert_eq!(g.player(A).unwrap().mana_pool.total(),mana_before-if paid{8}else{0});
        for (index,id) in targets.iter().enumerate().skip(removed){assert_eq!(g.is_tapped(*id),index==0 || (paid && index<selected));}
        untap(&mut g,B);untap(&mut g,C);
        for (index,id) in targets.iter().enumerate().skip(removed){assert_eq!(g.is_tapped(*id),paid && index<selected);}
        untap(&mut g,B);untap(&mut g,C);
        for id in targets.iter().skip(removed){assert!(!g.is_tapped(*id));}
    }}}
}
#[test]
fn cone_keeps_all_three_dice_bodies_and_timed_entry_restriction(){
    for definition in definitions("Cone of Cold") {for result in [1,9,10,19,20] {
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

// Fresh measured regressions, distinct from the original five-card cohort.
const CONDITIONAL_CANDIDATES: [(&str, &str); 2] = [
    ("Send to Sleep", "9e3fd1e9-7db6-40de-b1de-cd8cc9f60590"),
    ("Icy Blast", "f49302c5-8510-4360-841c-a59f53f87e0b"),
];

fn set_condition(g: &mut GameState, name: &str, enabled: bool) -> Vec<ObjectId> {
    if !enabled { return vec![]; }
    if name == "Icy Blast" {
        vec![object(g,A,Zone::Battlefield,"Ferocious witness","Type: Creature — Bear\nPower/Toughness: 4/4")]
    } else {
        vec![
            object(g,A,Zone::Graveyard,"Instant witness","Type: Instant\nDraw a card."),
            object(g,A,Zone::Graveyard,"Sorcery witness","Type: Sorcery\nDraw a card."),
        ]
    }
}

#[test]
fn conditional_whole_bodies_preserve_metadata_and_both_transport_routes() {
    let rows = fixtures();
    for (name,id) in CONDITIONAL_CANDIDATES {
        let row=rows.iter().find(|row|row["oracle_id"]==id).unwrap();
        assert_eq!(row["name"],name);
        assert_eq!(row["text"],format!("Mana cost: {}\nType: Instant\n{}",row["mana_cost"].as_str().unwrap(),row["oracle_text"].as_str().unwrap()));
        // Independent direct compilation, artifact JSON validation and
        // materialization, with loss and unimplemented checks in definitions.
        for definition in definitions(name) {
            fn inspect(effect:&ironsmith::Effect, conditional:bool, counts:&mut (usize,usize)) {
                let is_condition=effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().is_some();
                if is_condition {counts.0+=1;}
                if let Some(cant)=effect.downcast_ref::<CantEffect>() {
                    assert!(conditional,"the freeze must not escape the condition");
                    assert!(matches!(&cant.restriction,Restriction::Untap(filter) if !filter.tagged_constraints.is_empty()));
                    assert_eq!(cant.duration,Until::ControllersNextUntapStep);
                    counts.1+=1;
                }
                effect.visit_child_effects(&mut |child|inspect(child,conditional||is_condition,counts));
            }
            let mut counts=(0,0);
            for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {
                inspect(effect,false,&mut counts);
            }
            assert_eq!(counts,(1,1),"{name}: the lowering must retain exactly one conditional freeze");
        }
    }
}

#[test]
fn conditional_freeze_is_checked_at_resolution_and_binds_only_legal_targets() {
    for (name,_) in CONDITIONAL_CANDIDATES { for definition in definitions(name) {
        for enabled_at_cast in [false,true] { for enabled_at_resolution in [false,true] {
            for selected in 0..=2 { for removed in 0..=selected {
                let mut g=game();
                let mut witnesses=set_condition(&mut g,name,enabled_at_cast);
                let first=creature(&mut g,B);let second=creature(&mut g,C);
                let unselected=creature(&mut g,B);
                let mut dm=Choices{x:selected as u32,targets:[first,second].into_iter().take(selected).map(Target::Object).collect(),..Default::default()};
                cast(&mut g,&definition,&mut dm);
                if selected > 0 || name == "Send to Sleep" {
                    let requirements=&dm.target_contexts.last().unwrap().requirements;
                    assert_eq!(requirements.len(),1);
                    assert_eq!(requirements[0].min_targets,if name=="Icy Blast"{selected}else{0});
                    assert_eq!(requirements[0].max_targets,Some(if name=="Icy Blast"{selected}else{2}));
                }
                if enabled_at_cast != enabled_at_resolution {
                    for witness in witnesses.drain(..) {g.move_object_by_effect(witness,Zone::Exile).unwrap();}
                    witnesses=set_condition(&mut g,name,enabled_at_resolution);
                }
                for id in [first,second].into_iter().take(removed){g.move_object_by_effect(id,Zone::Graveyard).unwrap();}
                settle(&mut g,&mut dm);
                assert!(!g.is_tapped(unselected),"{name}: empty, partial, and all-illegal selections must not tap an existing unselected creature");
                g.tap(unselected);
                for id in [first,second].into_iter().take(selected).skip(removed){assert!(g.is_tapped(id));}
                let late=creature(&mut g,B);g.tap(late);
                // Changing the condition after resolution must neither create
                // a missing freeze nor cancel an already-created one.
                for witness in witnesses {g.move_object_by_effect(witness,Zone::Exile).unwrap();}
                set_condition(&mut g,name,!enabled_at_resolution);
                untap(&mut g,B);untap(&mut g,C);
                assert!(!g.is_tapped(late));
                assert!(!g.is_tapped(unselected),"{name}: empty, partial, and all-illegal selections must not freeze an existing unselected creature");
                for id in [first,second].into_iter().take(selected).skip(removed){assert_eq!(g.is_tapped(id),enabled_at_resolution,"{name}");}
                untap(&mut g,B);untap(&mut g,C);
                for id in [first,second].into_iter().take(selected).skip(removed){assert!(!g.is_tapped(id));}
            }}
        }}
    }}
}

#[test]
fn malformed_conditional_bodies_cannot_be_accepted_losslessly() {
    for (name,_) in CONDITIONAL_CANDIDATES {
        let rows=fixtures();let row=rows.iter().find(|row|row["name"]==name).unwrap();
        for text in [
            row["text"].as_str().unwrap().replace("next untap steps","next two untap steps"),
            format!("{} Except on Tuesdays.",row["text"].as_str().unwrap()),
            row["text"].as_str().unwrap().replace("during their controllers' next untap steps","until their controllers' next untap steps"),
        ] {
            let (result,loss)=ironsmith_compiler::parse_loss::capture(||compile_to_runtime_definition(name,&text,false));
            assert!(result.is_err()||loss.is_lossy(),"{text}");
            let (result,loss)=ironsmith_compiler::parse_loss::capture(||compile_to_artifact(name,&text,false));
            assert!(result.is_err()||loss.is_lossy(),"{text}");
        }
    }
}

#[test]
fn spell_mastery_counts_qualifying_cards_in_only_your_graveyard() {
    for definition in definitions("Send to Sleep") {
        for (label, owner, zone, types, freezes) in [
            ("one instant", A, Zone::Graveyard, vec!["Instant"], false),
            ("one sorcery", A, Zone::Graveyard, vec!["Sorcery"], false),
            ("two instants", A, Zone::Graveyard, vec!["Instant", "Instant"], true),
            ("two sorceries", A, Zone::Graveyard, vec!["Sorcery", "Sorcery"], true),
            ("instant and sorcery", A, Zone::Graveyard, vec!["Instant", "Sorcery"], true),
            ("two nonqualifying cards", A, Zone::Graveyard, vec!["Land", "Artifact"], false),
            ("one qualifying and one nonqualifying", A, Zone::Graveyard, vec!["Instant", "Land"], false),
            ("opponent graveyard only", B, Zone::Graveyard, vec!["Instant", "Sorcery"], false),
            ("own qualifying cards in hand only", A, Zone::Hand, vec!["Instant", "Sorcery"], false),
            ("own qualifying cards in exile only", A, Zone::Exile, vec!["Instant", "Sorcery"], false),
        ] {
            let mut g = game();
            for kind in types {
                object(&mut g, owner, zone, "Graveyard witness", &format!("Type: {kind}"));
            }
            let target = creature(&mut g, B);
            let mut dm = Choices { targets: vec![Target::Object(target)], ..Default::default() };
            cast(&mut g, &definition, &mut dm);
            settle(&mut g, &mut dm);
            assert!(g.is_tapped(target), "the unconditional tap must survive: {label}");
            untap(&mut g, B);
            assert_eq!(g.is_tapped(target), freezes, "{label}");
            untap(&mut g, B);
            assert!(!g.is_tapped(target), "the freeze must expire: {label}");
        }
    }
}

#[test]
fn ferocious_checks_your_creatures_at_the_inclusive_power_boundary() {
    for definition in definitions("Icy Blast") {
        for (owner, power, freezes) in [(A, 3, false), (B, 4, false), (A, 4, true), (A, 5, true)] {
            let mut g = game();
            object(&mut g, owner, Zone::Battlefield, "Power witness", &format!("Type: Creature — Bear\nPower/Toughness: {power}/6"));
            let target = creature(&mut g, B);
            let mut dm = Choices { x: 1, targets: vec![Target::Object(target)], ..Default::default() };
            cast(&mut g, &definition, &mut dm);
            settle(&mut g, &mut dm);
            assert!(g.is_tapped(target));
            untap(&mut g, B);
            assert_eq!(g.is_tapped(target), freezes, "owner={owner:?}, power={power}");
            untap(&mut g, B);
            assert!(!g.is_tapped(target));
        }
    }
}

#[test]
fn conditional_taps_allow_any_creature_but_do_not_widen_to_unselected_objects() {
    for (name, _) in CONDITIONAL_CANDIDATES {
        for definition in definitions(name) {
            let mut g = game();
            set_condition(&mut g, name, true);
            let own = creature(&mut g, A);
            let opponent = creature(&mut g, B);
            let unselected = creature(&mut g, B);
            let other_opponent = creature(&mut g, C);
            let noncreature = object(&mut g, B, Zone::Battlefield, "Noncreature witness", "Type: Artifact");
            let mut dm = Choices { x: 2, targets: vec![Target::Object(own), Target::Object(opponent)], ..Default::default() };
            cast(&mut g, &definition, &mut dm);
            let requirements = &dm.target_contexts.last().unwrap().requirements;
            assert_eq!(requirements.len(), 1);
            let legal = &requirements[0].legal_targets;
            for id in [own, opponent, unselected, other_opponent] {
                assert!(legal.contains(&Target::Object(id)), "{name}: all controllers' creatures are legal");
            }
            assert!(!legal.contains(&Target::Object(noncreature)), "{name}: a noncreature must not be legal");
            settle(&mut g, &mut dm);
            assert!(g.is_tapped(own));
            assert!(g.is_tapped(opponent));
            for id in [unselected, other_opponent, noncreature] {
                assert!(!g.is_tapped(id), "{name}: an existing unselected object must not be tapped");
                // Tap it independently after resolution. This catches a freeze
                // captured over too broad a resolution-time object set.
                g.tap(id);
            }
            untap(&mut g, A);
            untap(&mut g, B);
            untap(&mut g, C);
            assert!(g.is_tapped(own));
            assert!(g.is_tapped(opponent));
            for id in [unselected, other_opponent, noncreature] {
                assert!(!g.is_tapped(id), "{name}: an existing unselected object must not be frozen");
            }
            untap(&mut g, A);
            untap(&mut g, B);
            assert!(!g.is_tapped(own));
            assert!(!g.is_tapped(opponent));
        }
    }
}

#[test]
fn icy_blast_x_three_is_not_capped_at_send_to_sleeps_two_targets() {
    for definition in definitions("Icy Blast") {
        let mut g = game();
        set_condition(&mut g, "Icy Blast", true);
        let targets = [creature(&mut g, A), creature(&mut g, B), creature(&mut g, C)];
        let mut dm = Choices { x: 3, targets: targets.into_iter().map(Target::Object).collect(), ..Default::default() };
        cast(&mut g, &definition, &mut dm);
        let requirements = &dm.target_contexts.last().unwrap().requirements;
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].min_targets, 3);
        assert_eq!(requirements[0].max_targets, Some(3));
        for id in targets { assert!(requirements[0].legal_targets.contains(&Target::Object(id))); }
        settle(&mut g, &mut dm);
        for id in targets { assert!(g.is_tapped(id)); }
        for player in [A, B, C] { untap(&mut g, player); }
        for id in targets { assert!(g.is_tapped(id)); }
        for player in [A, B, C] { untap(&mut g, player); }
        for id in targets { assert!(!g.is_tapped(id)); }
    }
}
