use super::*;
use ironsmith::decisions::context::SelectionRevealPolicy;
struct Opening { pause: bool, pending: bool, requested: Vec<Vec<ObjectId>>, shown: Vec<ObjectId> }
impl DecisionMaker for Opening {
    fn decide_targets(&mut self, _: &GameState, _: &TargetsContext) -> Vec<Target> { vec![Target::Player(B)] }
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(context.reveal_policy, SelectionRevealPolicy::Public);
        assert_eq!(context.player, B);
        assert_eq!(context.min, 1);
        assert_eq!(context.max, Some(1));
        let chosen: Vec<_> = context.candidates.iter().map(|candidate| candidate.id).collect();
        assert_eq!(chosen.len(), 1, "the forced opening reveals only the random result");
        self.requested.push(chosen.clone());
        self.pending = self.pause;
        chosen
    }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn view_cards(&mut self, game: &GameState, _: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        assert!(context.public);
        assert!(cards.iter().all(|id| !game.is_hidden_card_placeholder(*id)));
        assert_eq!(game.player(B).unwrap().life, 20, "the opening precedes mana-value evaluation");
        self.shown = cards.to_vec();
    }
}
#[test]
fn ogre_real_entry_waits_for_owner_opening_on_owner_and_peer_then_loses_exact_life() {
    for definition in definitions("Singe-Mind Ogre") {
        for peer in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut identities = Vec::new();
            for (slot, mana) in [1, 5].into_iter().enumerate() {
                let card = compile_to_runtime_definition(&format!("Hand {slot}"), format!("Mana cost: {{{mana}}}\nType: Artifact"), false).unwrap();
                let id = game.create_hidden_card_placeholder(B, Zone::Hand, slot as u16, format!("hand-{slot}"));
                if !peer { game.reveal_hidden_card_with_definition(id, &card).unwrap(); }
                identities.push((id, card, mana));
            }
            let mut dm = Opening { pause: true, pending: false, requested: Vec::new(), shown: Vec::new() };
            let mut context = EffectContext::new(source, A, &mut dm);
            let entry = execute_effect(&mut game, &Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Battlefield, false), &mut context).unwrap();
            drop(context);
            for event in entry.events { game.queue_trigger_event(Default::default(), event); }
            put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
            game.queue_transcript_random_seeds([76543]);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(dm.pending);
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.player(B).unwrap().life, 20);
            assert_eq!(game.irreversible_random_count(), 0);
            assert!(dm.shown.is_empty());
            assert_eq!(dm.requested.len(), 1);
            let selected = dm.requested[0][0];
            let (_, card, mana) = identities.iter().find(|(id, _, _)| *id == selected).unwrap();
            if peer { game.reveal_hidden_card_with_definition(selected, card).unwrap(); }
            dm.pending = false;
            dm.pause = false;
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(B).unwrap().life, 20 - mana);
            assert_eq!(game.irreversible_random_count(), 1);
            assert_eq!(game.random_seed(), 76543);
            assert_eq!(dm.requested[0], dm.requested[1]);
            assert_eq!(dm.shown, vec![selected]);
            assert_eq!(game.player(B).unwrap().hand.len(), 2);
            for (id, _, _) in &identities {
                assert_eq!(game.is_publicly_revealed_hidden_card(*id), *id == selected);
                if peer && *id != selected { assert!(game.is_hidden_card_placeholder(*id)); }
            }
        }
    }
}
