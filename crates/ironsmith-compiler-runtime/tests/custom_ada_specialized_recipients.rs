//! Exact Ada source, independent strict adapters, native specialized recipient owner.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{Zone,Target};
#[test]
fn exact_ada_expands_heal_recipients_without_changing_the_amount_or_performer() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
    let ada=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap())
        .find(|card|card["source_card_key"]=="Ada, the Free_OPH").unwrap();
    let creatures=source_routes::definitions(&serde_json::json!({"name":"Marked recipient","executable_text":"Type: Creature\nPower/Toughness: 5/5"})).unwrap();
    let spells=source_routes::definitions(&serde_json::json!({"name":"Independent healing instruction","executable_text":"Mana cost: {0}\nType: Instant\nHeal 2 damage on target creature."})).unwrap();
    for provider in source_routes::definitions(ada).unwrap() {for creature in &creatures {for spell in &spells {
        let mut game=play::game();game.create_object_from_definition(&provider,play::A,Zone::Battlefield);
        let first=game.create_object_from_definition(creature,play::A,Zone::Battlefield);
        let second=game.create_object_from_definition(creature,play::B,Zone::Battlefield);
        let sentinel=game.create_object_from_definition(creature,play::A,Zone::Battlefield);
        game.mark_damage(first,4);game.mark_damage(second,1);game.mark_damage(sentinel,3);
        let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(first),Target::Object(second)],..Default::default()};
        play::cast(&mut game,play::A,spell,&mut dm);play::resolve_all(&mut game,&mut dm);
        assert_eq!((game.damage_on(first),game.damage_on(second),game.damage_on(sentinel)),(2,0,3));
    }}}
}

#[test]
fn exact_ada_expands_earthbend_recipients_and_art_sticker_recipients_on_both_routes() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
    let ada=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap())
        .find(|card|card["source_card_key"]=="Ada, the Free_OPH").unwrap();
    let land=source_routes::definitions(&serde_json::json!({"name":"Land recipient","executable_text":"Type: Land"})).unwrap()[0].clone();
    let artifact=source_routes::definitions(&serde_json::json!({"name":"Art recipient","executable_text":"Type: Artifact"})).unwrap()[0].clone();
    for provider in source_routes::definitions(ada).unwrap() {for (kind,text) in [
        ("earthbend","Mana cost: {0}\nType: Instant\nEarthbend 2."),
        ("sticker","Mana cost: {0}\nType: Instant\nPut an art sticker on target artifact you own."),
    ] {for spell in source_routes::definitions(&serde_json::json!({"name":"Independent specialized instruction","executable_text":text})).unwrap() {
        let mut game=play::game();game.create_object_from_definition(&provider,play::A,Zone::Battlefield);
        let recipient=if kind=="earthbend"{&land}else{&artifact};
        let ids=(0..3).map(|_|game.create_object_from_definition(recipient,play::A,Zone::Battlefield)).collect::<Vec<_>>();
        if kind=="sticker"{game.add_accessible_art_sticker(play::A);game.add_accessible_art_sticker(play::A);}
        let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(ids[0]),Target::Object(ids[1])],..Default::default()};
        play::cast(&mut game,play::A,&spell,&mut dm);play::resolve_all(&mut game,&mut dm);
        for (index,id) in ids.iter().copied().enumerate() {
            if kind=="earthbend" {
                assert_eq!(game.counter_count(id,ironsmith::object::CounterType::PlusOnePlusOne),if index<2{2}else{0});
                assert_eq!(game.current_card_types(id).unwrap().contains(&ironsmith::CardType::Creature),index<2);
            }else{
                assert_eq!(game.sticker_count_on_object(id,ironsmith::events::KeywordActionKind::ArtSticker,None),u32::from(index<2));
            }
        }
    }}}
}
