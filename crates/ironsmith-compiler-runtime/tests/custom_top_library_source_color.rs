//! Ordinary permission on the exact linked Yearning Clockwork source, independent routes.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,compute_legal_actions};
use ironsmith::color::ColorSet;
use ironsmith::continuous::{EffectTarget,Modification};
use ironsmith::effects::{ApplyContinuousEffect,EffectContext,EffectExecutor};
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
fn source()->Vec<[CardDefinition;2]> {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
    let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["source_card_key"]=="Yearning Clockwork//Breathing Spellwork_HQZ").unwrap();
    assert_eq!(card["source_faces"].as_array().unwrap().len(),2);
    source_support::linked_definitions(card).unwrap()
}
fn setup(back:&CardDefinition)->(GameState,ObjectId) {
    let mut g=GameState::new(vec!["A".into(),"B".into()],20);
    g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;
    let source=g.create_object_from_definition(back,A,Zone::Battlefield);
    colors(&mut g,source,ColorSet::BLUE); (g,source)
}
fn colors(g:&mut GameState,id:ObjectId,colors:ColorSet) {
    ApplyContinuousEffect::new(EffectTarget::Specific(id),Modification::SetColors(colors),ironsmith::effect::Until::Forever)
        .execute(g,&mut EffectContext::new_default(id,A)).unwrap();
}
fn card(g:&mut GameState,owner:PlayerId,color:&str,kind:&str)->ObjectId {
    let indicator=if color.is_empty(){String::new()}else{format!("Color indicator: {color}\n")};
    let text=if kind=="Land" {"Type: Land".to_string()}else{format!("Mana cost: {{0}}\n{indicator}Type: {kind}\nYou gain 1 life.")};
    let d=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary candidate",&text,false).unwrap();
    g.create_object_from_definition(&d,owner,Zone::Library)
}
fn playable(g:&GameState,player:PlayerId,id:ObjectId)->bool {
    compute_legal_actions(g,player).unwrap().iter().any(|a|matches!(a,
       LegalAction::CastSpell{spell_id,..} if *spell_id==id) || matches!(a,LegalAction::PlayLand{land_id} if *land_id==id))
}
#[test]
fn actual_back_face_permission_checks_live_color_or_colorless_and_normal_timing() {
    let definitions=source(); assert_eq!(definitions.len(),2);
    for route in 0..2 {for (color,allowed) in [("",true),("Blue",true),("Red",false)] {
        let (mut g,id)=setup(&definitions[1][route]); let candidate=card(&mut g,A,color,"Sorcery");
        assert_eq!(playable(&g,A,candidate),allowed,"route{route} color{color}");
        if color=="Red" {colors(&mut g,id,ColorSet::RED);assert!(playable(&g,A,candidate),"relation follows source's current colors");}
        g.turn.active_player=B;assert!(!playable(&g,A,candidate),"ordinary permission does not grant sorcery flash");
        g.turn.active_player=A;g.move_object_by_effect(id,Zone::Exile).unwrap();assert!(!playable(&g,A,candidate),"source departure revokes permission");
    }}
}
#[test]
fn actual_permission_is_own_library_top_only_and_includes_land_play() {
    let definitions=source();
    for route in 0..2 {
        let (mut g,_)=setup(&definitions[1][route]);
        let lower=card(&mut g,A,"Blue","Sorcery");let upper=card(&mut g,A,"Red","Sorcery");
        assert_eq!(g.player(A).unwrap().library.last(),Some(&upper));
        assert!(!playable(&g,A,lower));assert!(!playable(&g,A,upper));
        g.move_object_by_effect(upper,Zone::Graveyard).unwrap();assert!(playable(&g,A,lower));
        let opposing=card(&mut g,B,"Blue","Sorcery");assert!(!playable(&g,A,opposing));
        let land=card(&mut g,A,"","Land");assert!(playable(&g,A,land));
        let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::PlayLand{land_id}if *land_id==land)).unwrap();
        let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::apply_priority_response_with_dm(&mut g,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(g.objects_in_zone(Zone::Battlefield).len(),2,"ordinary library land play enters battlefield beside source");
        let second=card(&mut g,A,"","Land");assert!(!playable(&g,A,second),"permission preserves ordinary land allowance");
    }
}

#[test]
fn permission_follows_source_controller_and_still_requires_printed_mana() {
    let definitions=source();
    for route in 0..2 {
        let (mut g,id)=setup(&definitions[1][route]);
        let a_card=card(&mut g,A,"Blue","Sorcery");let b_card=card(&mut g,B,"Blue","Sorcery");
        assert!(playable(&g,A,a_card));
        g.set_current_controller(id,B).unwrap();g.turn.active_player=B;g.turn.priority_player=Some(B);
        assert!(!playable(&g,A,a_card));assert!(playable(&g,B,b_card));
        let priced=ironsmith_compiler_runtime::compile_to_runtime_definition("Priced candidate","Mana cost: {3}\nColor indicator: Blue\nType: Sorcery\nYou gain 1 life.",false).unwrap();
        let priced=g.create_object_from_definition(&priced,B,Zone::Library);
        assert!(!playable(&g,B,priced),"ordinary permission does not waive printed mana cost");
    }
}
