//! Atomic as-transform characteristics from complete preserved source faces.
#[path="custom_final_next_wave_support.rs"] mod source_support;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition, GameState, ObjectId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::color::{Color, ColorSet};
use ironsmith::mana::ManaSymbol;

fn source(key:&str)->serde_json::Value {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
    fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap())
        .find(|card|card["source_card_key"]==key).unwrap().clone()
}
fn linked(game:&mut GameState,source:&serde_json::Value,route:usize)->CardDefinition {
    let faces=source_support::linked_definitions(source).unwrap();
    for pair in &faces {game.register_linked_face_definition(&pair[route]);}
    faces[0][route].clone()
}
fn transform(game:&mut GameState,id:ObjectId)->ironsmith::effect::EffectOutcome {
    execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::TransformEffect::source()),
        &mut EffectContext::new_default(id,play::A)).unwrap()
}

#[test]
fn exact_hayat_and_ameret_convert_every_existing_counter_before_transform_publication() {
    for route in 0..2 {for key in [
        "Hayat, Hungering Bakhtak//The All-Consuming Swarm_ROS",
        "The Lost and Seeking//Ameret, Shining and Proud_ROS",
    ] {
        let mut game=play::game();let definition=linked(&mut game,&source(key),route);
        let id=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);
        let object=game.object_mut(id).unwrap();
        object.counters.insert(CounterType::PlusOnePlusOne,2);
        object.counters.insert(CounterType::Charge,3);
        object.counters.insert(CounterType::Loyalty,1);
        let outcome=transform(&mut game,id);
        let counters=&game.object(id).unwrap().counters;
        assert_eq!(counters.len(),1);assert_eq!(counters.get(&CounterType::Loyalty),Some(&6));
        assert_eq!(outcome.events.len(),1,"counter kind conversion neither removes nor places counters");
        let event=outcome.events[0].downcast::<ironsmith::events::other::TransformedEvent>().unwrap();
        let snapshot=event.snapshot.as_ref().unwrap();
        assert_eq!(snapshot.counters.get(&CounterType::Loyalty),Some(&6));
        assert_eq!(snapshot.counters.len(),1,"transform observers see the atomic final kinds");
        transform(&mut game,id);
        assert_eq!(game.object(id).unwrap().counters.get(&CounterType::Loyalty),Some(&6),"turning back preserves counters and does not re-run the departed face's rider");
    }}
}

#[test]
fn exact_breathing_spellwork_reads_only_the_mana_paid_for_its_own_attain_activation() {
    for route in 0..2 {
        let mut card=source("Yearning Clockwork//Breathing Spellwork_HQZ");
        // The preserved original declares Attain. Its ordinary executable
        // activation receives that declaration's opaque ownership annotation.
        // The shared source adapter applies this same generic annotation.
        let front=card["executable_faces"][0]["executable_text"].as_str().unwrap();
        if !front.contains("This is the attain activated ability.") {
            let annotated=format!("{front} This is the attain activated ability.");
            card["executable_faces"][0]["executable_text"]=annotated.clone().into();
            card["executable_text"]=annotated.into();
        }
        let mut game=play::game();let front=linked(&mut game,&card,route);
        let id=game.create_object_from_definition(&front,play::A,Zone::Battlefield);
        for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green] {
            play::give_mana(&mut game,play::A,color,1);
        }
        let mut dm=play::Script::default();
        play::activate(&mut game,id,play::activated_at(&front,0),&mut dm);
        while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
        let expected:[Color;5]=[Color::White,Color::Blue,Color::Black,Color::Red,Color::Green];
        assert_eq!(game.current_colors(id).unwrap(),expected.into_iter().collect::<ColorSet>());
        assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0);
        transform(&mut game,id);transform(&mut game,id);
        assert_eq!(game.current_colors(id).unwrap(),expected.into_iter().collect::<ColorSet>(),"existing continuous color adjustment survives ordinary later flips");
        let unrelated=game.create_object_from_definition(&front,play::A,Zone::Battlefield);
        transform(&mut game,unrelated);
        assert!(game.current_colors(unrelated).unwrap().is_empty(),"an unrelated transform has no Attain payment receipt");
    }
}
