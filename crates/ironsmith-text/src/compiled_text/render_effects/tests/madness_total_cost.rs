use super::*;

struct MadnessChoice(bool);
impl crate::decision::DecisionMaker for MadnessChoice {
    fn decide_boolean(&mut self, _: &crate::game_state::GameState, _: &crate::decisions::context::BooleanContext) -> bool { self.0 }
}

#[test]
fn madness_total_cost_renders_mana_and_life() {
    for text in ["Madness—{2}{B}, Pay 8 life.", "Madness {1}{R}", "Madness—Pay three {C}."] {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Madness Cost Probe")
            .card_types(vec![CardType::Creature]).parse_text(text).unwrap();
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n").trim_end_matches('.'),text.trim_end_matches('.'));
    }
}

#[test]
fn madness_total_cost_discard_pays_all_components_or_none() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Madness Cost Probe")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(3,5)).parse_text("Madness—{2}{B}, Pay 8 life.").unwrap();
    for (life, mana, accept) in [(20,3,true),(8,3,true),(7,3,true),(20,2,true),(20,3,false)] {
        let mut game=crate::game_state::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let alice=game.players[0].id;
        game.player_mut(alice).unwrap().life=life;
        game.player_mut(alice).unwrap().mana_pool.add(crate::mana::ManaSymbol::Black,mana);
        let card=game.create_object_from_definition(&definition,alice,Zone::Hand);
        let mut decision=MadnessChoice(accept);
        let _result=crate::events::processing::execute_discard(&mut game,card,alice,
            crate::events::cause::EventCause::effect(),false,crate::provenance::ProvNodeId::default(),&mut decision);
        let succeeds=accept && life>=8 && mana>=3;
        assert_eq!(game.player(alice).unwrap().life,if succeeds {life-8} else {life});
        assert_eq!(game.player(alice).unwrap().mana_pool.total(),if succeeds {mana-3} else {mana});
        assert_eq!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name == "Madness Cost Probe"),succeeds);
    }
}
