use super::*;
const TEXT: &str = "{T}, Sacrifice another creature or an artifact: For each opponent, you mill a card, then return that card from your graveyard to your hand unless that player pays 3 life.\nChoose a Background";
struct Payment {
    controller: crate::ids::PlayerId,
    opponents: Vec<crate::ids::PlayerId>,
    payments: u8,
    library_size: usize,
    calls: usize,
}
impl crate::decision::DecisionMaker for Payment {
    fn decide_boolean(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert_ne!(ctx.player, self.controller);
        assert_eq!(
            game.player(self.controller).unwrap().library.len(),
            self.library_size.saturating_sub(self.calls + 1),
            "each opponent decides after exactly its own mill"
        );
        assert_eq!(ctx.player, self.opponents[self.calls]);
        self.calls += 1;
        self.payments & (1 << (self.calls - 1)) != 0
    }
}
#[test]
fn per_opponent_mill_payment_keeps_each_card_with_its_opponents_decision() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sivriss, Nightmare Speaker")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| {
            if let AbilityKind::Activated(a) = &a.kind {
                Some(a)
            } else {
                None
            }
        })
        .unwrap();
    for library_size in [0usize, 1, 2, 4] {
        for payments in 0u8..4 {
            for active_index in 0..3 {
                let mut game = crate::game_state::GameState::new(
                    vec!["Alice".into(), "Bob".into(), "Carol".into()],
                    20,
                );
                let alice = game.players[0].id;
                game.turn.active_player = game.players[active_index].id;
                let opponents = (0..3)
                    .map(|offset| game.players[(active_index + offset) % 3].id)
                    .filter(|id| *id != alice)
                    .collect::<Vec<_>>();
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                for index in 0..library_size {
                    let card = crate::card::CardBuilder::new(
                        crate::ids::CardId::new(),
                        format!("Library Card {index}"),
                    )
                    .card_types(vec![CardType::Land])
                    .build();
                    game.create_object_from_card(&card, alice, Zone::Library);
                }
                let mut dm = Payment {
                    controller: alice,
                    opponents: opponents.clone(),
                    payments,
                    library_size,
                    calls: 0,
                };
                {
                    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm);
                    for effect in &ability.effects {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                }
                assert_eq!(dm.calls, 2);
                for (index, opponent) in opponents.iter().enumerate() {
                    assert_eq!(
                        game.player(*opponent).unwrap().life,
                        if payments & (1 << index) != 0 { 17 } else { 20 }
                    );
                }
                let player = game.player(alice).unwrap();
                assert_eq!(player.library.len(), library_size.saturating_sub(2));
                let mut expected_hand = Vec::new();
                let mut expected_graveyard = Vec::new();
                for index in 0..library_size.min(2) {
                    let name = format!("Library Card {}", library_size - 1 - index);
                    if payments & (1 << index) == 0 {
                        expected_hand.push(name);
                    } else {
                        expected_graveyard.push(name);
                    }
                }
                let names = |ids: &[crate::ids::ObjectId]| {
                    let mut names = ids
                        .iter()
                        .map(|id| game.object(*id).unwrap().name.clone())
                        .collect::<Vec<_>>();
                    names.sort();
                    names
                };
                expected_hand.sort();
                expected_graveyard.sort();
                assert_eq!(
                    names(&player.hand),
                    expected_hand,
                    "size={library_size}, payments={payments}, active={active_index}"
                );
                assert_eq!(names(&player.graveyard), expected_graveyard);
            }
        }
    }
}

#[test]
fn per_opponent_mill_payment_text_preserves_sequence() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sivriss, Nightmare Speaker")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        format!("{TEXT}.")
    );
}
