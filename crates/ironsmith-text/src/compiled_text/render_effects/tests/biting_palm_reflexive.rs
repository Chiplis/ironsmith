use super::*;
const TEXT: &str = "Ninjutsu {2}{B}\nThis creature enters with a menace counter on it.\nWhenever this creature deals combat damage to a player, you may remove a menace counter from it. When you do, that player reveals their hand and you choose a nonland card from it. Exile that card.";
struct NinjaChoices {
    player: crate::ids::PlayerId,
    chosen: crate::ids::ObjectId,
    accept: bool,
    choices: usize,
}
impl crate::decision::DecisionMaker for NinjaChoices {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert_eq!(ctx.player, self.player);
        self.accept
    }
    fn decide_objects(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        assert_eq!(ctx.player, self.player);
        let legal: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        assert_eq!(legal.len(), 2);
        assert!(legal.contains(&self.chosen));
        assert!(legal.iter().all(|id| game.object(*id).unwrap().owner
            == game.object(self.chosen).unwrap().owner
            && !game.current_has_card_type(*id, CardType::Land)));
        self.choices += 1;
        vec![self.chosen]
    }
}
#[test]
fn biting_palm_reflexive_requires_counter_removal_and_uses_damaged_hand() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Biting-Palm Ninja")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    for (initial, accept) in [(1, true), (1, false), (0, true)] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.remove_counters(source, CounterType::Menace, 100, None, None);
        game.add_counters(source, CounterType::Menace, initial);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Eligible Card")
            .card_types(vec![CardType::Instant])
            .build();
        let chosen = game.create_object_from_card(&card, bob, Zone::Hand);
        let untouched = game.create_object_from_card(&card, carol, Zone::Hand);
        let other = game.create_object_from_card(&card, bob, Zone::Hand);
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&land, bob, Zone::Hand);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(bob),
                2,
                true,
                crate::events::EventCause::combat_damage(source),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        let mut dm = NinjaChoices {
            player: alice,
            chosen,
            accept,
            choices: 0,
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let succeeds = initial > 0 && accept;
        assert_eq!(
            game.stack.len(),
            usize::from(succeeds),
            "initial={initial}, accept={accept}"
        );
        if succeeds {
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        assert_eq!(dm.choices, usize::from(succeeds));
        assert_eq!(game.object(chosen).is_none(), succeeds);
        assert_eq!(game.object(untouched).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(other).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(land).unwrap().zone, Zone::Hand);
        assert_eq!(game.exile.len(), usize::from(succeeds));
    }
}

#[test]
fn biting_palm_reflexive_preserves_the_coordinated_hand_sentence() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Biting-Palm Ninja")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
