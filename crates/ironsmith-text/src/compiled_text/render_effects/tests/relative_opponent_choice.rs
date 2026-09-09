use super::*;

#[test]
fn relative_opponent_choice_selects_players_with_more_matching_permanents() {
    for (noun, counted) in [("lands", CardType::Land), ("creatures", CardType::Creature), ("artifacts", CardType::Artifact)] {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Relative choice fixture")
            .card_types(vec![CardType::Instant]).parse_text(&format!("Choose an opponent who controls more {noun} than you.")).unwrap();
        let program = definition.spell_effect.as_ref().unwrap();
        let root = &program.segments[0].default_effects[0];
        let choose = structural_unwrap_render_wrappers(root).downcast_ref::<crate::effects::ChoosePlayerEffect>()
            .expect("a player choice, not an object choice");
        assert_eq!(choose.chooser, PlayerFilter::You);
        for counts in [[2, 1, 3], [2, 2, 2], [3, 4, 5], [4, 1, 2]] {
            let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let players: Vec<_> = game.players.iter().map(|p| p.id).collect();
            game.turn.active_player = players[1];
            let source = game.create_object_from_definition(&definition, players[0], Zone::Stack);
            let permanent = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Count fixture").card_types(vec![counted]).build();
            for (i, player) in players.iter().enumerate() {
                for _ in 0..counts[i] { game.create_object_from_card(&permanent, *player, Zone::Battlefield); }
                for _ in 0..7 { game.create_object_from_card(&permanent, *player, Zone::Hand); }
            }
            game.refresh_continuous_state();
            assert!(crate::game_loop::extract_target_requirements_from_program_with_modes(&game, program, players[0], Some(source), None).is_empty());
            let mut ctx = crate::effects::EffectContext::new_default(source, players[0]);
            crate::effects::execute_effect(&mut game, root, &mut ctx).unwrap();
            let expected: Vec<_> = (1..3).filter(|i| counts[*i] > counts[0]).take(1).map(|i| players[i]).collect();
            assert_eq!(ctx.get_tagged_players(choose.tag.as_str()).cloned().unwrap_or_default(), expected, "{noun}: {counts:?}");
        }
    }
}

#[test]
fn relative_opponent_search_count_is_bound_to_the_chosen_player_difference() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Relative count fixture")
        .card_types(vec![CardType::Instant])
        .parse_text("Choose an opponent who controls more lands than you. Search your library for a number of Plains cards equal to the difference, reveal those cards, put them into your hand, then shuffle.").unwrap();
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Difference"), "{debug}");
    for counts in [[1, 3, 4], [3, 4, 5], [2, 2, 2]] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let players: Vec<_> = game.players.iter().map(|p| p.id).collect();
        game.turn.active_player = players[2];
        let source = game.create_object_from_definition(&definition, players[0], Zone::Stack);
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land").card_types(vec![CardType::Land]).build();
        for (i, player) in players.iter().enumerate() {
            for _ in 0..counts[i] { game.create_object_from_card(&land, *player, Zone::Battlefield); }
        }
        let plains = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Plains candidate")
            .card_types(vec![CardType::Land]).subtypes(vec![Subtype::Plains]).build();
        for _ in 0..4 { game.create_object_from_card(&plains, players[0], Zone::Library); }
        let other_library = game.create_object_from_card(&plains, players[1], Zone::Library);
        let expected = (1..3).find(|i| counts[*i] > counts[0]).map_or(0, |i| counts[i] - counts[0]);
        let mut dm = DifferenceSearchDecision { player: players[0], count: expected };
        let mut ctx = crate::effects::EffectContext::new_default(source, players[0]).with_decision_maker(&mut dm);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        assert_eq!(game.player(players[0]).unwrap().hand.len(), expected, "{counts:?}");
        assert_eq!(game.object(other_library).unwrap().zone, Zone::Library);
    }
}

struct DifferenceSearchDecision { player: crate::ids::PlayerId, count: usize }
impl crate::decision::DecisionMaker for DifferenceSearchDecision {
    fn decide_objects(&mut self, _: &crate::game_state::GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
        assert_eq!(ctx.player, self.player);
        assert_eq!(ctx.max, Some(self.count));
        ctx.candidates.iter().filter(|c| c.legal).take(self.count).map(|c| c.id).collect()
    }
}

struct SplitSearchDecision { player: crate::ids::PlayerId, selected: usize, calls: usize }
impl crate::decision::DecisionMaker for SplitSearchDecision {
    fn decide_objects(&mut self, _: &crate::game_state::GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
        assert_eq!(ctx.player, self.player);
        let legal: Vec<_> = ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).collect();
        self.calls += 1;
        if self.calls == 1 {
            assert!(ctx.max.is_some_and(|max| max <= 3));
            if self.selected == 3 { assert_eq!(ctx.max, Some(3)); }
            legal.into_iter().take(self.selected).collect()
        } else {
            assert_eq!(ctx.max, Some(1));
            assert_eq!(legal.len(), self.selected);
            legal.last().copied().into_iter().collect()
        }
    }
}

#[test]
fn relative_opponent_search_partitions_one_tapped_and_the_remainder_in_hand() {
    let oracle = "Choose an opponent who controls more lands than you. Search your library for a number of Plains cards equal to the difference, reveal those cards, put one of them onto the battlefield tapped and the rest into your hand, then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Split search fixture")
        .card_types(vec![CardType::Instant]).parse_text(oracle).unwrap();
    let effects = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects;
    let refs: Vec<_> = effects.iter().collect();
    assert!(describe_search_two_split_hand_graveyard_sequence(&refs).is_some(), "{effects:#?}");
    for mutation in 0..2 {
        let mut changed = effects.clone();
        let index = if mutation == 0 { 1 } else { 3 };
        let mut choose = changed[index].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap().clone();
        if mutation == 0 { choose.count_value = Some(Value::Fixed(3)); }
        else { choose.filter.subtypes.push(Subtype::Island); }
        changed[index] = Effect::new(choose);
        let refs: Vec<_> = changed.iter().collect();
        assert!(describe_search_two_split_hand_graveyard_sequence(&refs).is_none());
    }
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle);
    for (available, selected) in [(4, 3), (4, 1), (4, 0), (1, 1), (0, 0)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id;
        game.turn.active_player = bob;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let plains = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Plains fixture")
            .card_types(vec![CardType::Land]).subtypes(vec![Subtype::Plains]).build();
        game.create_object_from_card(&plains, alice, Zone::Battlefield);
        for _ in 0..4 { game.create_object_from_card(&plains, bob, Zone::Battlefield); }
        let mut stable = Vec::new();
        for _ in 0..available {
            let id = game.create_object_from_card(&plains, alice, Zone::Library);
            stable.push(game.object(id).unwrap().stable_id);
        }
        let mut dm = SplitSearchDecision { player: alice, selected, calls: 0 };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        let mut zones = [0, 0, 0];
        for id in stable {
            let object = game.object(game.find_object_by_stable_id(id).unwrap()).unwrap();
            assert_eq!(game.controller_of(object), alice);
            match object.zone {
                Zone::Battlefield => { zones[0] += 1; assert!(game.is_tapped(object.id)); }
                Zone::Hand => zones[1] += 1,
                Zone::Library => zones[2] += 1,
                zone => panic!("unexpected {zone:?}"),
            }
        }
        assert_eq!(zones, [usize::from(selected > 0), selected.saturating_sub(1), available-selected], "available={available},selected={selected}");
    }
}
