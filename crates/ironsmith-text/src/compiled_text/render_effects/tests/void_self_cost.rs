use super::*;

const TEXT: &str = "Void — This spell costs {2} less to cast if a nonland permanent left the battlefield this turn or a spell was warped this turn.";

#[test]
fn labeled_self_cost_reduction_obeys_history_and_does_not_discount_other_spells() {
    let base = crate::mana::ManaCost::from_pips(vec![
        vec![crate::mana::ManaSymbol::Generic(2)],
        vec![crate::mana::ManaSymbol::Black],
    ]);
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "History Cost Fixture")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(base.clone())
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT.strip_prefix("Void — ").unwrap()
    );
    let unrelated = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unrelated Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(base.clone())
        .build();
    let permanent_discount =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Permanent Cost Fixture")
            .card_types(vec![CardType::Artifact])
            .mana_cost(base.clone())
            .parse_text(TEXT)
            .unwrap();
    for history in 0..4 {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let other = game.create_object_from_definition(&unrelated, alice, Zone::Hand);
        if history == 1 || history == 2 {
            let kind = if history == 1 {
                CardType::Land
            } else {
                CardType::Artifact
            };
            let permanent =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Departing Permanent")
                    .card_types(vec![kind])
                    .build();
            let object = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
            game.move_object_by_effect(object, Zone::Graveyard).unwrap();
        } else if history == 3 {
            game.turn_store.turn_history.spell_warped_this_turn = true;
        }
        let cost = crate::decision::calculate_effective_mana_cost(
            &game,
            alice,
            game.object(spell).unwrap(),
            &base,
        );
        assert_eq!(
            cost.to_oracle(),
            if history >= 2 { "{B}" } else { "{2}{B}" },
            "history={history}"
        );
        assert_eq!(
            crate::decision::calculate_effective_mana_cost(
                &game,
                alice,
                game.object(other).unwrap(),
                &base
            ),
            base
        );
        game.create_object_from_definition(&permanent_discount, alice, Zone::Battlefield);
        game.refresh_continuous_state();
        assert_eq!(
            crate::decision::calculate_effective_mana_cost(
                &game,
                alice,
                game.object(other).unwrap(),
                &base
            ),
            base,
            "a self-only discount must not become a battlefield-wide discount"
        );
    }
}
