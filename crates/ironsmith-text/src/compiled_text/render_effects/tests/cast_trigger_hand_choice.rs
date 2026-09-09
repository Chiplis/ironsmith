use super::*;

const TEXT: &str = "Whenever an opponent casts a green spell, you may pay {B}{B}. If you do, look at that player's hand and choose a card from it. The player discards that card.\n{B}{B}: Return this enchantment to its owner's hand.";

struct HandChoice {
    pay: bool,
    controller: crate::ids::PlayerId,
    hand: Vec<crate::ids::ObjectId>,
    chosen: crate::ids::ObjectId,
    views: usize,
    choices: usize,
}
impl crate::decision::DecisionMaker for HandChoice {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.pay
    }
    fn decide_objects(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        assert_eq!(ctx.player, self.controller);
        let mut candidates: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        candidates.sort();
        let mut expected = self.hand.clone();
        expected.sort();
        assert_eq!(
            candidates, expected,
            "only the caster's hand may be selected"
        );
        self.choices += 1;
        vec![self.chosen]
    }
    fn view_cards(
        &mut self,
        _: &crate::game_state::GameState,
        viewer: crate::ids::PlayerId,
        cards: &[crate::ids::ObjectId],
        _: &crate::decisions::context::ViewCardsContext,
    ) {
        assert_eq!(viewer, self.controller);
        assert_eq!(cards, self.hand);
        self.views += 1;
    }
}

#[test]
fn paid_cast_trigger_chooses_from_the_casters_hand_and_discards_only_that_card() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Leshrac's Sigil")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    for (pay, mana, spell_leaves) in [
        (true, 2, false),
        (false, 2, false),
        (true, 1, false),
        (true, 2, true),
    ] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Green Spell")
            .card_types(vec![CardType::Sorcery])
            .color_indicator(crate::color::ColorSet::GREEN)
            .build();
        for player in [alice, bob, carol] {
            for _ in 0..2 {
                game.create_object_from_card(&card, player, Zone::Hand);
            }
        }
        let hand = game.player(bob).unwrap().hand.to_vec();
        let chosen = hand[1];
        let spell = game.create_object_from_card(&card, bob, Zone::Stack);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new_with_snapshot(
                spell,
                bob,
                Zone::Hand,
                crate::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        if spell_leaves {
            game.move_object_by_effect(spell, Zone::Graveyard).unwrap();
        }
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Black, mana);
        let mut decisions = HandChoice {
            pay,
            controller: alice,
            hand,
            chosen,
            views: 0,
            choices: 0,
        };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_triggering_event(event)
            .with_decision_maker(&mut decisions);
        for effect in &triggers[0].ability.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        let paid = pay && mana == 2;
        assert_eq!(decisions.views, usize::from(paid));
        assert_eq!(decisions.choices, usize::from(paid));
        assert_eq!(
            game.player(bob).unwrap().hand.len(),
            if paid { 1 } else { 2 }
        );
        assert_eq!(game.player(bob).unwrap().hand.contains(&chosen), !paid);
        assert!(game.player(bob).unwrap().hand.contains(&decisions.hand[0]));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(carol).unwrap().hand.len(), 2);
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            if paid { 0 } else { mana }
        );
    }
}

#[test]
fn cast_trigger_hand_choice_renders_the_bound_hand_and_discard() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Leshrac's Sigil")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT,
        "{definition:#?}"
    );
}
