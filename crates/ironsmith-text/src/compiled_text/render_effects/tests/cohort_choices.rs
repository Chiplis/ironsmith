use super::*;

const CHOICES: &str = "For each player, you choose from among the permanents that player controls an artifact, a creature, an enchantment, and a planeswalker. Then each player sacrifices all other nonland permanents they control.";

#[test]
fn cohort_choices_preserve_shared_multitype_kept_permanents_for_every_player() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(CHOICES)
        .unwrap();
    let hybrid = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Kept Hybrid")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .build();
    let artifact = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Extra Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Extra Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Kept Land")
        .card_types(vec![CardType::Land])
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let mut kept = Vec::new();
    for player in [alice, bob] {
        kept.push(game.create_object_from_definition(&hybrid, player, Zone::Battlefield));
        game.create_object_from_definition(&artifact, player, Zone::Battlefield);
        game.create_object_from_definition(&creature, player, Zone::Battlefield);
        kept.push(game.create_object_from_definition(&land, player, Zone::Battlefield));
    }
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for effect in spell
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.battlefield.len(), 4);
    for id in kept {
        assert!(game.battlefield.contains(&id));
    }
    for player in [alice, bob] {
        assert_eq!(game.player(player).unwrap().graveyard.len(), 2);
    }
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&spell).join("\n"),
        CHOICES
    );
}

#[test]
fn cohort_choices_render_optional_sacrifice_discard_and_evidence() {
    for (types, text) in [
        (
            vec![CardType::Sorcery],
            "You may sacrifice an artifact or discard a card. If you do, draw two cards.",
        ),
        (
            vec![CardType::Creature],
            "When this creature enters, you may sacrifice an artifact or discard a card. If you do, draw a card.",
        ),
        (
            vec![CardType::Creature],
            "Whenever this creature attacks, you may sacrifice an artifact or discard a card. If you do, draw a card and this creature gets +2/+0 until end of turn.",
        ),
        (
            vec![CardType::Creature],
            "Flying\nWhen this creature dies, you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.",
        ),
    ] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice Probe")
            .card_types(types)
            .parse_text(text)
            .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&card).join("\n"),
            text
        );
    }
}
