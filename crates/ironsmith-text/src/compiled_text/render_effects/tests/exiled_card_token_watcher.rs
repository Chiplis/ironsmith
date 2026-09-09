use super::*;

#[derive(Default)]
struct ExiledViews(Vec<(crate::ids::PlayerId, Vec<crate::ids::ObjectId>, Zone)>);
impl crate::decision::DecisionMaker for ExiledViews {
    fn view_cards(&mut self, _game: &crate::game_state::GameState, viewer: crate::ids::PlayerId, cards: &[crate::ids::ObjectId], ctx: &crate::decisions::context::ViewCardsContext) {
        self.0.push((viewer, cards.to_vec(), ctx.zone));
    }
}

#[test]
fn exile_look_token_sequence_views_the_exiled_card_and_registers_a_watcher() {
    run_exile_token_lifecycle(false);
    run_exile_token_lifecycle(true);
}

fn run_exile_token_lifecycle(reenter_exile: bool) {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Exile watcher fixture")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile the top card of your library face down and look at it. Create a 2/2 colorless Spirit creature token. When that token leaves the battlefield, put the exiled card into your hand.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Library fixture").card_types(vec![CardType::Creature]).build();
    game.create_object_from_definition(&card, alice, Zone::Library);
    game.create_object_from_definition(&card, alice, Zone::Library);
    let mut decisions = ExiledViews::default();
    for _ in 0..2 {
        let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
    }
    assert_eq!(game.exile.len(), 2, "cards remain exiled until their tokens leave");
    assert_eq!(game.effect_store.delayed_triggers.len(), 2);
    assert_eq!(decisions.0.len(), 2);
    for (viewer, cards, zone) in &decisions.0 {
        assert_eq!(*viewer, alice);
        assert_eq!(*zone, Zone::Exile);
        assert_eq!(cards.len(), 1);
    }
    let first_card = decisions.0[0].1[0];
    let second_card = decisions.0[1].1[0];
    assert_ne!(first_card, second_card);
    let first_stable = game.object(first_card).unwrap().stable_id;
    game.move_object(source, Zone::Graveyard, crate::events::cause::EventCause::effect()).unwrap();
    let token = game.battlefield[0];
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(token).unwrap(), &game);
    let moved = game.move_object(token, Zone::Graveyard, crate::events::cause::EventCause::effect()).unwrap();
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_results(token, vec![moved], Zone::Battlefield, Zone::Graveyard,
            crate::events::cause::EventCause::effect(), Some(snapshot)),
        crate::provenance::ProvNodeId::default());
    let entries = crate::triggers::check_delayed_triggers(&mut game, &event);
    assert_eq!(entries.len(), 1);
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 1, "the watched token returns its exiled card");
    assert_eq!(game.exile, vec![second_card], "the other activation's card remains exiled");
    assert_eq!(game.object(game.player(alice).unwrap().hand[0]).unwrap().stable_id, first_stable);
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);

    // Once the second card leaves exile, its later object must not be returned.
    let second_stable = game.object(second_card).unwrap().stable_id;
    let moved_card = game.move_object(second_card, Zone::Graveyard, crate::events::cause::EventCause::effect()).unwrap();
    if reenter_exile {
        game.move_object(moved_card, Zone::Exile, crate::events::cause::EventCause::effect()).unwrap();
    }
    let token = game.battlefield[0];
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(token).unwrap(), &game);
    let moved = game.move_object(token, Zone::Exile, crate::events::cause::EventCause::effect()).unwrap();
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_results(token, vec![moved], Zone::Battlefield, Zone::Exile,
            crate::events::cause::EventCause::effect(), Some(snapshot)),
        crate::provenance::ProvNodeId::default());
    let entries = crate::triggers::check_delayed_triggers(&mut game, &event);
    assert_eq!(entries.len(), 1, "leaving for exile also triggers the watcher");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    let current = game.find_object_by_stable_id(second_stable).unwrap();
    assert_eq!(game.object(current).unwrap().zone, if reenter_exile { Zone::Exile } else { Zone::Graveyard });
    assert!(game.effect_store.delayed_triggers.is_empty());

}
