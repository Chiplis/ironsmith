use super::*;

const RULES_TEXT: &str = "When this Siege enters, each player mills three cards, then each opponent discards a card and you draw a card.";

fn definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(920_002), "Player Scope Siege")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .parse_text(RULES_TEXT)
        .expect("explicit-player-scope fixture should parse")
}

fn add_cards(game: &mut GameState, owner: PlayerId, zone: Zone, count: usize) {
    for index in 0..count {
        let card = CardBuilder::new(CardId::new(), format!("Scope Card {owner:?} {index}"))
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, owner, zone);
    }
}

#[test]
fn explicit_player_scope_sequence_has_exact_compiled_surface() {
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition()),
        vec![RULES_TEXT.to_string()]
    );
}

#[test]
fn explicit_player_scope_tail_executes_once_instead_of_once_per_player() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_cards(&mut game, alice, Zone::Library, 4);
    add_cards(&mut game, bob, Zone::Library, 4);
    add_cards(&mut game, alice, Zone::Hand, 1);
    add_cards(&mut game, bob, Zone::Hand, 1);

    let definition = definition();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("fixture should contain its entry trigger");
    game.push_to_stack(StackEntry::ability(
        source,
        alice,
        triggered.effects.clone(),
    ));
    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("entry trigger should resolve");

    assert_eq!(game.player(alice).expect("alice exists").library.len(), 0);
    assert_eq!(game.player(bob).expect("bob exists").library.len(), 1);
    assert_eq!(game.player(alice).expect("alice exists").graveyard.len(), 3);
    assert_eq!(game.player(bob).expect("bob exists").graveyard.len(), 4);
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        2,
        "the controller draws exactly once after all players mill"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").hand.len(),
        0,
        "the opponent discards exactly once"
    );
}
