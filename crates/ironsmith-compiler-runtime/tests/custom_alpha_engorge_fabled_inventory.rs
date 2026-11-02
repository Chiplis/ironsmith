//! Complete physical inventory, independent source routes, and actual entry cases.
//! New inputs must be materialized by the parent before this target is executed.
#[path = "custom_final_next_wave_support.rs"]
mod source;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use serde_json::Value;
const A: PlayerId = PlayerId(0);
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-alpha-engorge-fabled-complete-inventory.json")).unwrap()
}
fn physical(key: &str) -> Value {
    fixture()["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["source_card_key"] == key).unwrap().clone()
}
fn definitions(key: &str) -> [CardDefinition; 2] {
    let mut faces = source::linked_definitions(&physical(key))
        .unwrap_or_else(|error| panic!("{key}: {error}"));
    assert_eq!(faces.len(), 1);
    faces.remove(0)
}
fn enter(game: &mut GameState, id: ObjectId, dm: &mut dyn DecisionMaker) -> ObjectId {
    let receipt=game.move_object_with_etb_processing_with_dm(id,Zone::Battlefield,dm).unwrap();
    assert!(!receipt.pending);receipt.original.into_result().unwrap().new_id
}

fn wire(definition: CardDefinition) -> Result<Value, String> {
    let definition=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition)
        .map_err(|error|format!("complete runtime encoding: {error:?}"))?;
    serde_json::to_value(definition).map_err(|error|error.to_string())
}
#[test]
fn all_ninety_four_physical_sources_and_one_hundred_twenty_eight_faces_compile_independently() {
    let data=fixture();let mut keys=std::collections::BTreeSet::new();let mut faces=0usize;let mut routes=0usize;
    let mut family_counts=std::collections::BTreeMap::new();let mut failures=Vec::new();
    for row in data["mechanics"].as_array().unwrap() {
        let family=row["keyword"].as_str().unwrap();
        *family_counts.entry(family.to_string()).or_insert(0usize)+=row["representative_cards"].as_array().unwrap().len();
        for card in row["representative_cards"].as_array().unwrap() {
            let key=card["source_card_key"].as_str().unwrap();assert!(keys.insert(key));
            let originals=card["source_faces"].as_array().unwrap();faces+=originals.len();
            let compiled=match source::linked_definitions(card) {
                Ok(compiled)=>compiled,Err(error)=>{failures.push(format!("{family} / {} / {key}: {error}",row["id"]));continue;}
            };
            if compiled.len()!=originals.len() {failures.push(format!("{key}: sibling face count {} != {}",compiled.len(),originals.len()));}
            for (original,[native,artifact]) in originals.iter().zip(compiled) {
                let expected=original["name"].as_str().unwrap();
                if native.card.name.as_str()!=expected || artifact.card.name.as_str()!=expected {failures.push(format!("{key}: exact face name/order mismatch for {expected}"));}
                routes+=2;
                match (wire(native),wire(artifact)) {
                    (Ok(native),Ok(artifact)) if native==artifact=>{},
                    (Ok(_),Ok(_))=>failures.push(format!("{key} / {expected}: independent routes differ in complete typed definition")),
                    (left,right)=>failures.push(format!("{key} / {expected}: canonical transport failure native={:?},artifact={:?}",left.err(),right.err())),
                }
            }
        }
    }
    assert_eq!(keys.len(),94);assert_eq!(faces,128);
    assert_eq!(family_counts,[("Alpha".to_string(),15),("Engorge".to_string(),12),("Fabled".to_string(),39),("Primal".to_string(),28)].into_iter().collect());
    assert!(!keys.contains("Mother of Monsters_MC25"));assert!(!keys.contains("Forgotten Library_KXP"));
    assert!(failures.is_empty(),"All source failures (none skipped):\n{}",failures.join("\n"));
    assert_eq!(routes,256,"every complete physical face must compile through both independent routes");
}

#[test]
fn actual_meadow_macaque_choice_adds_status_only_when_accepted_and_counter_removal_keeps_it() {
    struct Choice(bool);
    impl DecisionMaker for Choice {
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.0 }
    }
    for definition in definitions("Meadow Macaque_GNJ") {
        for accepted in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let id = game.create_object_from_definition(&definition, A, Zone::Hand);
            let entered = enter(&mut game, id, &mut Choice(accepted));
            assert_eq!(game.has_custom_status(entered, "alpha"), accepted);
            assert_eq!(game.object(entered).unwrap().counters.get(&ironsmith::object::CounterType::PlusOnePlusOne).copied().unwrap_or(0), u32::from(accepted));
            game.object_mut(entered).unwrap().remove_counters(ironsmith::object::CounterType::PlusOnePlusOne, 1);
            assert_eq!(game.has_custom_status(entered, "alpha"), accepted);
        }
    }
}
#[test]
fn actual_blood_servant_uses_its_entry_exile_type_and_not_an_unrelated_creature_in_exile() {
    for definition in definitions("Blood Servant_AFM") {
        for creature in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let witness = |name: &str, creature| ironsmith_compiler_runtime::compile_to_runtime_definition(name,
                if creature { "Type: Creature\nPower/Toughness: 1/1" } else { "Type: Artifact" }, false).unwrap();
            game.create_object_from_definition(&witness("Unrelated exiled creature", true), A, Zone::Exile);
            // One card makes the authored exile deterministic; verify trigger admission.
            game.create_object_from_definition(&witness("Entry witness", creature), A, Zone::Library);
            let id = game.create_object_from_definition(&definition, A, Zone::Hand);
            let entered = enter(&mut game, id, &mut SelectFirstDecisionMaker);
            let retained = &game.object(entered).unwrap().cast_tagged_objects[&ironsmith::tag::TagKey::from("__entry_action_exiled__")];
            assert_eq!(retained.len(), 1);
            assert_eq!(retained[0].card_types.contains(&ironsmith::CardType::Creature), creature);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(queue.entries.len(), usize::from(creature));
        }
    }
}

fn setup() -> GameState {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;
    game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;game
}
fn ordinary(name:&str,text:&str)->CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()
}
fn settle(game:&mut GameState,dm:&mut dyn DecisionMaker) {
    let mut queue=ironsmith::triggers::TriggerQueue::new();
    for _ in 0..64 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
        if game.stack.is_empty(){return;}
        ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();
    }
    panic!("source scenario did not settle");
}
fn announce(game:&mut GameState,action:ironsmith::decision::LegalAction,dm:&mut dyn DecisionMaker) {
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action(){return;}
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};
        progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();
    }
    panic!("source announcement did not finish");
}
fn cast_action(game:&GameState,id:ObjectId,alternative:bool)->Option<ironsmith::decision::LegalAction> {
    ironsmith::decision::compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,
        ironsmith::decision::LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==id
            && matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Alternative(_))==alternative))
}
#[test]
fn xiejuan_status_transition_observes_actual_entry_choice_and_other_creatures() {
    struct Choice(bool);impl DecisionMaker for Choice {
        fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}
    }
    for definition in definitions("Xiejuan, the Regal Beast_GNJ") {for accepted in [false,true] {
        let mut game=setup();let creature=ordinary("Independent companion","Type: Creature\nPower/Toughness: 1/1");
        for _ in 0..2{game.create_object_from_definition(&creature,A,Zone::Battlefield);}
        let land=ordinary("Independent library card","Type: Basic Land — Forest");
        for _ in 0..4{game.create_object_from_definition(&land,A,Zone::Library);}
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);let entered=enter(&mut game,source,&mut Choice(accepted));
        settle(&mut game,&mut Choice(accepted));
        assert_eq!(game.has_custom_status(entered,"alpha"),accepted);
        assert_eq!(game.player(A).unwrap().hand.len(),if accepted{2}else{0},"entry status event must use the actual resulting source and exclude itself from other creatures");
    }}
}
#[test]
fn deadland_mongrel_reads_only_its_entry_receipt_power_threshold() {
    for definition in definitions("Deadland Mongrel_AFM") {for power in [2,3] {
        let mut game=setup();let body=ordinary("Entry threshold witness",&format!("Type: Creature\nPower/Toughness: {power}/4"));
        let unrelated=ordinary("Unrelated exiled creature","Type: Creature\nPower/Toughness: 9/9");game.create_object_from_definition(&unrelated,A,Zone::Exile);
        game.create_object_from_definition(&body,A,Zone::Library);
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);let entered=enter(&mut game,source,&mut SelectFirstDecisionMaker);
        let base=physical("Deadland Mongrel_AFM")["power"].as_str().unwrap().parse::<i32>().unwrap();
        assert_eq!(game.calculated_power(entered),Some(base+i32::from(power>=3)));
    }}
}
#[test]
fn new_fabled_player_aura_attaches_to_the_actual_recipient_and_keeps_its_back_trigger() {
    const B:PlayerId=PlayerId(1);
    struct Opponent;impl DecisionMaker for Opponent {
        fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{
            let target=ironsmith::Target::Player(B);
            if ctx.requirements[0].legal_targets.contains(&target){vec![target]}
            else{ironsmith::decision::DecisionMaker::decide_targets(&mut SelectFirstDecisionMaker,game,ctx)}
        }
    }
    let faces=source::linked_definitions(&physical("Sinful Burial//Pious Judgment_GHQ")).unwrap();assert_eq!(faces.len(),2);
    for route in 0..2 {for selected in [false,true] {
        let mut game=setup();let body=ordinary("Independent recipient body","Type: Creature\nPower/Toughness: 2/2");
        game.create_object_from_definition(&body,B,Zone::Battlefield);
        let front=&faces[0][route];let source=game.create_object_from_definition(front,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Black,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,2);
        let action=cast_action(&game,source,selected).expect("requested Fabled price available");announce(&mut game,action,&mut Opponent);
        settle(&mut game,&mut Opponent);let current=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone,if selected{Zone::Battlefield}else{Zone::Graveyard});
        if selected {assert_eq!(game.object(current).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Player(B)));assert_eq!(game.object(current).unwrap().name,faces[1][route].card.name);}
        let own=game.create_object_from_definition(&body,A,Zone::Hand);enter(&mut game,own,&mut Opponent);settle(&mut game,&mut Opponent);
        assert_eq!(game.player(B).unwrap().life,20,"the unattached controller's creature does not trigger the Aura");
        let opponent=game.create_object_from_definition(&body,B,Zone::Hand);enter(&mut game,opponent,&mut Opponent);settle(&mut game,&mut Opponent);
        assert_eq!(game.player(B).unwrap().life,if selected{19}else{20});
    }}
}
#[test]
fn sunken_slasher_spends_a_land_allowance_without_falsely_recording_a_land_play() {
    for definition in definitions("Sunken Slasher_NVA") {for mode in ["primal","normal","normal-after-land"] {
        let selected=mode=="primal";let mut game=setup();
        if mode=="normal-after-land" {
            let land=ordinary("Actual land play","Type: Basic Land — Plains");let land=game.create_object_from_definition(&land,A,Zone::Hand);
            let play=ironsmith::decision::compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,ironsmith::decision::LegalAction::PlayLand{land_id}if *land_id==land)).unwrap();
            announce(&mut game,play,&mut SelectFirstDecisionMaker);assert_eq!(game.player(A).unwrap().lands_played_this_turn,1);
        }
        game.player_mut(A).unwrap().mana_pool.add(if selected{ironsmith::ManaSymbol::White}else{ironsmith::ManaSymbol::Red},if selected{2}else{1});
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,if selected{1}else{3});
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
        if mode=="normal-after-land" {assert!(cast_action(&game,source,true).is_none(),"an actual land consumed the one allowance");}
        let action=cast_action(&game,source,selected).expect("requested actual source price available");announce(&mut game,action,&mut SelectFirstDecisionMaker);
        assert_eq!(game.player(A).unwrap().nonland_land_play_usage_this_turn,u32::from(selected));
        assert_eq!(game.player(A).unwrap().lands_played_this_turn,u32::from(mode=="normal-after-land"));
        settle(&mut game,&mut SelectFirstDecisionMaker);let current=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.counter_count(current,ironsmith::CounterType::PlusOnePlusOne),u32::from(mode!="normal-after-land"));
    }}
}
#[test]
fn curio_connoisseur_reduces_fabled_spells_without_marking_itself_or_plain_spells() {
    let consumers=definitions("Curio Connoisseur_GHQ");let cards=source::linked_definitions(&physical("Seek Glory//Warforged Hero_TWR")).unwrap();
    for route in 0..2 {
        let mut game=setup();game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::White,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,2);
        let fabled=game.create_object_from_definition(&cards[0][route],A,Zone::Hand);
        assert!(cast_action(&game,fabled,true).is_none(),"the printed four-mana Fabled cost requires the reducer");
        assert!(!consumers[route].abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if ability.custom_keyword_name().is_some_and(|name|name.eq_ignore_ascii_case("fabled")))),"a keyword consumer is not a keyword declaration");
        game.create_object_from_definition(&consumers[route],A,Zone::Battlefield);game.refresh_continuous_state().unwrap();
        assert!(cast_action(&game,fabled,true).is_some(),"the actual Fabled identity qualifies for the generic reduction");
        let plain=ordinary("Unmarked plain spell","Mana cost: {3}{W}\nType: Sorcery\nYou gain 1 life.");let plain=game.create_object_from_definition(&plain,A,Zone::Hand);
        assert!(cast_action(&game,plain,false).is_none(),"the same mana value without Aura/Fabled does not qualify");
    }
}
