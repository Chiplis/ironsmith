//! Exact Mettalia source composes finite color choices and source-owned protection.
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::decision::DecisionMaker;
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
struct Decisions(bool);
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}
    fn decide_objects(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{ctx.candidates.iter().filter(|object|object.legal).take(ctx.min).map(|object|object.id).collect()}
}
fn pairs(name:&str,text:&str)->[CardDefinition;2]{
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
    assert!(!loss.is_lossy(),"{name}: {}",loss.reasons_text());let(artifact,direct)=result.unwrap();
    let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn body(name:&str,cost:&str,legendary:bool)->CardDefinition{pairs(name,&format!("Mana cost: {cost}\nType: {}Creature\nPower/Toughness: 2/2",if legendary{"Legendary "}else{""}))[0].clone()}
#[test]
fn actual_mettalia_source_selects_each_color_and_protection_follows_only_current_exact_members(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-collection-color-control.json")).unwrap();
    let card=&fixture["mechanics"][0]["representative_cards"][0];
    assert_eq!(card["source_card_key"],"Mettalia, Light from Above_EIA");
    assert!(card["original_text"].as_str().unwrap().contains("cards she's salvaged in exile"));
    let text=card["executable_text"].as_str().unwrap();assert!(!text.to_lowercase().contains("you may salvage"),"source action must actually expand");
    for definition in pairs("Mettalia, Light from Above",text){for accept in [false,true]{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);
        let source_stable=game.object(source).unwrap().stable_id;
        let other=pairs("Empty collection guardian","Type: Creature\nPower/Toughness: 1/1\nThis creature has protection from each color among cards in exile from the salvaged collection.")[0].clone();
        let other=game.create_object_from_definition(&other,A,Zone::Battlefield);
        let mut red_stable=None;
        for symbol in ["{W}","{U}","{B}","{R}","{G}"]{
            let id=game.create_object_from_definition(&body("Colored material",symbol,false),A,Zone::Graveyard);
            if symbol=="{R}"{red_stable=Some(game.object(id).unwrap().stable_id);}
        }
        let colorless=game.create_object_from_definition(&body("Colorless material","{1}",false),A,Zone::Graveyard);
        let legendary=game.create_object_from_definition(&body("Legendary material","{R}",true),A,Zone::Graveyard);
        let red=game.create_object_from_definition(&body("Unrelated red","{R}",false),B,Zone::Stack);
        let blue=game.create_object_from_definition(&body("Unrelated blue","{U}",false),B,Zone::Stack);
        game.create_object_from_definition(&body("Unrelated exile","{R}",false),A,Zone::Exile);
        // Enter through the ordinary zone-move owner, then announce and
        // resolve the real triggered program with its event references intact.
        let mut decisions=Decisions(accept);
        let outcome=execute_effect(&mut game, &ironsmith::Effect::move_to_zone(
            ironsmith::target::ChooseSpec::Source, Zone::Battlefield, false),
            &mut EffectContext::new(source,A,&mut decisions)).unwrap();
        for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
        let source=game.find_object_by_stable_id(source_stable).unwrap();
        let mut queue=ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut decisions).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut decisions).unwrap();
        assert_eq!(game.stack.len(),1,"the actual entry has exactly one collection trigger");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut decisions).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.source_collection_objects(source,"salvaged").len(),if accept{5}else{0});
        assert!(game.source_collection_objects(other,"salvaged").is_empty());
        assert_eq!(ironsmith::targeting::has_protection_from_source(&game,source,red),accept);
        assert!(!ironsmith::targeting::has_protection_from_source(&game,other,red));
        assert_eq!(game.object(colorless).unwrap().zone,Zone::Graveyard);assert_eq!(game.object(legendary).unwrap().zone,Zone::Graveyard);
        assert!(game.chosen_color(source).is_none(),"finite iteration must not mutate an unrelated permanent choice");
        if accept{
            let member=game.find_object_by_stable_id(red_stable.unwrap()).unwrap();let returned=game.move_object_by_effect(member,Zone::Hand).unwrap();game.move_object_by_effect(returned,Zone::Exile).unwrap();
            game.refresh_continuous_state().unwrap();
            assert!(!ironsmith::targeting::has_protection_from_source(&game,source,red),"leave-return cannot restore old collection membership");
            assert!(ironsmith::targeting::has_protection_from_source(&game,source,blue));
            game.set_current_controller(source,B).unwrap();game.refresh_continuous_state().unwrap();
            assert!(ironsmith::targeting::has_protection_from_source(&game,source,blue),"control change preserves source identity rather than borrowing another source's pool");
        }
    }}
}
