use super::*;

#[test]
fn coordinated_draws_keep_both_players_and_counts() {
    for opponent_index in [1, 2] {
        let oracle = "Flying\nWhen this creature enters, target opponent draws a card and you draw three cards.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sphinx of Enlightenment")
            .card_types(vec![CardType::Creature]).parse_text(oracle).unwrap();
        let triggered = definition.abilities.iter().find_map(|a| if let AbilityKind::Triggered(t) = &a.kind { Some(t) } else { None }).unwrap();
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let players = game.players.iter().map(|p| p.id).collect::<Vec<_>>();
        for player in &players {
            for _ in 0..5 {
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Library fixture").build();
                game.create_object_from_card(&card, *player, Zone::Library);
            }
        }
        let source = game.create_object_from_definition(&definition, players[0], Zone::Battlefield);
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(&game, &triggered.effects, players[0], Some(source), None);
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].legal_targets, vec![crate::Target::Player(players[1]), crate::Target::Player(players[2])]);
        let mut ctx = crate::effects::EffectContext::new_default(source, players[0])
            .with_targets(vec![crate::effects::ResolvedTarget::Player(players[opponent_index])]);
        for segment in &triggered.effects.segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        for (index, player) in players.iter().enumerate() {
            assert_eq!(game.player(*player).unwrap().hand.len(), if index == 0 { 3 } else if index == opponent_index { 1 } else { 0 });
        }
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition), oracle.lines().collect::<Vec<_>>());
    }
}

#[test]
fn coordinated_draw_rendering_uses_counts_and_player_identity() {
    let oracle = "Target opponent draws two cards and you draw four cards.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cobalt Probe")
        .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition), [oracle]);
    let root = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects[0];
    let sequence = root.downcast_ref::<crate::effects::SequenceEffect>().unwrap();
    let mut effects = sequence.effects.clone();
    let last = effects.last().unwrap().downcast_ref::<crate::effects::DrawCardsEffect>().unwrap();
    let mut other_draw = last.clone();
    other_draw.player = PlayerFilter::Opponent;
    *effects.last_mut().unwrap() = Effect::new(other_draw);
    assert!(super::super::structural_bundles::describe_opponent_and_you_draw(&effects).is_none());
}
