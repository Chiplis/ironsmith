use super::*;
const TEXT: &str = "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card.";
struct RetargetChoice(bool);
impl crate::decision::DecisionMaker for RetargetChoice {
    fn decide_boolean(&mut self, _: &crate::game_state::GameState, _: &crate::decisions::context::BooleanContext) -> bool { self.0 }
}
#[test]
fn shared_type_copy_opponents_draw_independently_of_retarget_choice() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Gandalf, Westward Voyager")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(5, 5))
        .parse_text(TEXT).unwrap();
    for matching_player in [None, Some(1), Some(2)] {
        for retarget in [false, true] {
            let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let spell_def = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cast Spell")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(5)]))
                .parse_text("You gain 1 life.").unwrap();
            let spell = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
            game.stack.push(crate::game_state::StackEntry::new(spell, alice));
            let mut tops = Vec::new();
            for index in 0..3 {
                let player = game.players[index].id;
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Top Card")
                    .card_types(vec![if matching_player == Some(index) { CardType::Sorcery } else { CardType::Land }]).build();
                tops.push(game.create_object_from_card(&card, player, Zone::Library));
            }
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new(spell, alice, Zone::Hand), crate::provenance::ProvNodeId::default());
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            let mut dm = RetargetChoice(retarget);
            let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm).with_triggering_event(event);
            for segment in &triggers[0].ability.effects.segments {
                for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            }
            assert_eq!(game.stack.len(), if matching_player.is_some() { 2 } else { 1 });
            for (index, top) in tops.iter().enumerate() {
                let draws = if matching_player.is_some() { index != 0 } else { index == 0 };
                // Draw changes object identity; inspect the player's hand rather than the original ID.
                assert_eq!(game.players[index].hand.len(), usize::from(draws), "match={matching_player:?}, retarget={retarget}, player={index}");
                if !draws { assert_eq!(game.object(*top).unwrap().zone, Zone::Library); }
            }
        }
    }
}
#[test]
fn shared_type_copy_preserves_full_conditional_text() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Gandalf, Westward Voyager")
        .card_types(vec![CardType::Creature]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), TEXT);
}
