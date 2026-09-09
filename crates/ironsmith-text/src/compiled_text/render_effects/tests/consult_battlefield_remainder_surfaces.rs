use super::*;

#[test]
fn smeagol_keeps_the_selected_land_and_revealed_remainder_partition() {
    let oracle = "At the beginning of your end step, if a creature died under your control this turn, the Ring tempts you.\nWhenever the Ring tempts you, target opponent reveals cards from the top of their library until they reveal a land card. Put that card onto the battlefield tapped under your control and the rest into their graveyard.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sméagol, Helpful Guide")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .expect("selected-land consult partition should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("battlefield_controller: You"), "{debug}");
    assert!(debug.contains("enters_tapped: true"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
fn vote_counted_reveal_moves_distinct_matches_from_one_traversal() {
    struct WildVotes { wild: usize, cast: usize }
    impl crate::decision::DecisionMaker for WildVotes {
        fn decide_options(&mut self, _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
            let option = usize::from(self.cast >= self.wild); self.cast += 1; vec![option]
        }
    }
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Council Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Starting with you, each player votes for wild or free. Reveal cards from the top of your library until you reveal a creature card for each wild vote. Put those creature cards onto the battlefield, then shuffle the rest into your library. You may put a permanent card from your hand onto the battlefield for each free vote.")
        .unwrap();
    for wild in 0..=2 {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Revealed Creature")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
    let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Revealed Land")
        .card_types(vec![CardType::Land]).build();
    let third = game.create_object_from_card(&creature, alice, Zone::Library);
    let second = game.create_object_from_card(&creature, alice, Zone::Library);
    let between = game.create_object_from_card(&land, alice, Zone::Library);
    let first = game.create_object_from_card(&creature, alice, Zone::Library);
    game.player_mut(alice).unwrap().library = vec![third, second, between, first].into();
    let before_shuffle = game.irreversible_random_count();
    let mut dm = WildVotes { wild, cast: 0 };
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm);
    for segment in &definition.spell_effect.as_ref().unwrap().segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|error| panic!("{error:?}: {definition:#?}"));
        }
    }
    assert_eq!(game.battlefield.len(), wild, "one distinct creature per wild vote");
    assert_eq!(game.player(alice).unwrap().library.len(), 4 - wild);
    assert_eq!(game.irreversible_random_count(), before_shuffle + 1);
    assert!(game.player(alice).unwrap().library.contains(&third), "stop before the third creature");
    assert!(game.player(alice).unwrap().library.contains(&between), "retain the revealed nonmatch without changing its identity");
    }
}

#[test]
fn counted_reveal_shuffle_compiles_and_renders_the_bound_collections() {
    for oracle in [
        "Exile all creatures you control, then reveal cards from the top of your library until you reveal that many creature cards. Put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
        "Council's dilemma — Starting with you, each player votes for wild or free. Reveal cards from the top of your library until you reveal a creature card for each wild vote. Put those creature cards onto the battlefield, then shuffle the rest into your library. You may put a permanent card from your hand onto the battlefield for each free vote.",
    ] {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Council Probe")
            .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered.trim_end_matches('.'), oracle.trim_end_matches('.'), "{definition:#?}");
    }
}

#[test]
fn exile_counted_reveal_keeps_new_creatures_and_shuffles_only_library_remainder() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Transformation Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile all creatures you control, then reveal cards from the top of your library until you reveal that many creature cards. Put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
    let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land")
        .card_types(vec![CardType::Land]).build();
    for _ in 0..2 { game.create_object_from_card(&creature, alice, Zone::Battlefield); }
    let third = game.create_object_from_card(&creature, alice, Zone::Library);
    let second = game.create_object_from_card(&creature, alice, Zone::Library);
    let between = game.create_object_from_card(&land, alice, Zone::Library);
    let first = game.create_object_from_card(&creature, alice, Zone::Library);
    game.player_mut(alice).unwrap().library = vec![third, second, between, first].into();
    let before = game.irreversible_random_count();
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for segment in &definition.spell_effect.as_ref().unwrap().segments {
        for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|err| panic!("{err:?}: {effect:#?}")); }
    }
    assert_eq!(game.battlefield.len(), 2);
    assert_eq!(game.exile.len(), 2);
    let library = &game.player(alice).unwrap().library;
    assert_eq!(library.len(), 2);
    assert!(library.contains(&third) && library.contains(&between));
    assert_eq!(game.irreversible_random_count(), before + 1);
}

#[test]
fn removed_creatures_controller_reveals_moves_and_shuffles_their_own_cards() {
    for text in [
        "Exile target creature. That creature's controller reveals cards from the top of their library until they reveal a creature card. That player puts that card onto the battlefield, then shuffles the rest into their library.",
        "Exile target creature an opponent controls. That player reveals cards from the top of their library until a creature card is revealed. The player puts that card onto the battlefield, then shuffles the rest into their library.",
        "Destroy target creature. It can't be regenerated. Its controller reveals cards from the top of their library until they reveal a creature card. The player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.",
    ] {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Transformation Probe")
            .card_types(vec![CardType::Sorcery]).parse_text(text).unwrap();
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join(" ");
        assert!(rendered.contains("puts that card onto the battlefield, then shuffles"), "{rendered}\n{definition:#?}");
        if text.contains("all other cards revealed this way") {
            assert!(rendered.contains("all other cards revealed this way into their library"), "{rendered}");
        } else {
            assert!(rendered.contains("the rest into their library"), "{rendered}");
        }
        for has_match in [true, false] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land")
            .card_types(vec![CardType::Land]).build();
        let target = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let untouched = game.create_object_from_card(&creature, alice, Zone::Library);
        let second = game.create_object_from_card(if has_match { &creature } else { &land }, bob, Zone::Library);
        let first = game.create_object_from_card(if has_match { &creature } else { &land }, bob, Zone::Library);
        let above = game.create_object_from_card(&land, bob, Zone::Library);
        game.player_mut(bob).unwrap().library = vec![second, first, above].into();
        let before = game.irreversible_random_count();
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.snapshot_targets(&game);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|err| panic!("{err:?}: {effect:#?}")); }
        }
        assert_eq!(game.battlefield.len(), usize::from(has_match), "{text}");
        if has_match { assert_eq!(game.controller_of(game.object(game.battlefield[0]).unwrap()), bob); }
        assert_eq!(game.player(alice).unwrap().library, vec![untouched]);
        let library = &game.player(bob).unwrap().library;
        assert_eq!(library.len(), if has_match { 2 } else { 3 });
        assert!(library.contains(&second) && library.contains(&above));
        assert_eq!(game.irreversible_random_count(), before + 1);
        }
    }
}

#[test]
fn chosen_type_consult_uses_resolution_choice_after_source_is_sacrificed() {
    struct ChooseDemon;
    impl crate::decision::DecisionMaker for ChooseDemon {
        fn decide_options(&mut self, _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
            vec![ctx.options.iter().find(|option| option.description.eq_ignore_ascii_case("demon")).unwrap().index]
        }
    }
    let text = "{2}{U}{U}, Sacrifice this creature: Choose a creature type. Reveal cards from the top of your library until you reveal a creature card of that type. Put that card onto the battlefield and shuffle the rest into your library.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Shapeshifter Probe")
        .card_types(vec![CardType::Creature]).parse_text(text).unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert_eq!(rendered.trim_end_matches('.'), text.trim_end_matches('.'));
    let crate::ability::AbilityKind::Activated(ability) = &definition.abilities[0].kind else { unreachable!(); };
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let demon = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Demon")
        .card_types(vec![CardType::Creature]).subtypes(vec![crate::types::Subtype::Demon])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
    let elf = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Elf")
        .card_types(vec![CardType::Creature]).subtypes(vec![crate::types::Subtype::Elf])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
    let later_demon = game.create_object_from_card(&demon, alice, Zone::Library);
    let first_demon = game.create_object_from_card(&demon, alice, Zone::Library);
    let first_elf = game.create_object_from_card(&elf, alice, Zone::Library);
    game.player_mut(alice).unwrap().library = vec![later_demon, first_demon, first_elf].into();
    game.move_object_by_effect(source, Zone::Graveyard);
    let mut dm = ChooseDemon;
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm);
    for segment in &ability.effects.segments {
        for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    }
    assert_eq!(game.battlefield.len(), 1);
    assert_eq!(game.object(game.battlefield[0]).unwrap().name.as_str(), "Demon");
    let library = &game.player(alice).unwrap().library;
    assert_eq!(library.len(), 2);
    assert!(library.contains(&later_demon) && library.contains(&first_elf));
}

#[test]
fn optional_consult_gates_movement_and_shuffle_on_acceptance() {
    struct AcceptReveal(bool);
    impl crate::decision::DecisionMaker for AcceptReveal {
        fn decide_boolean(&mut self, _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::BooleanContext) -> bool { self.0 }
    }
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Optional Transformation")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .parse_text("Kicker {1}{G}\nWhen this creature enters, if it was kicked, you may reveal cards from the top of your library until you reveal a creature card. If you do, put that card onto the battlefield and shuffle all other cards revealed this way into your library.").unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("If you do, put that card onto the battlefield and shuffle all other cards revealed this way into your library"), "{rendered}");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        crate::ability::AbilityKind::Triggered(trigger) => Some(trigger), _ => None,
    }).unwrap();
    assert!(format!("{:?}", trigger.intervening_if).contains("ThisSpellWasKicked"));
    for accept in [false, true] {
        for library_case in 0..3 {
            let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land")
                .card_types(vec![CardType::Land]).build();
            let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
                .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
            if library_case != 0 {
                game.create_object_from_card(if library_case == 2 { &creature } else { &land }, alice, Zone::Library);
                game.create_object_from_card(&land, alice, Zone::Library);
            }
            let original_library = game.player(alice).unwrap().library.clone();
            let before = game.irreversible_random_count();
            let mut dm = AcceptReveal(accept);
            let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm);
            for segment in &trigger.effects.segments {
                for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            }
            let moves = accept && library_case == 2;
            assert_eq!(game.battlefield.len(), 1 + usize::from(moves));
            assert_eq!(game.player(alice).unwrap().library.len(), original_library.len() - usize::from(moves));
            assert_eq!(game.irreversible_random_count(), before + u64::from(accept));
            if !accept { assert_eq!(game.player(alice).unwrap().library, original_library); }
        }
    }
}

#[test]
fn sacrificed_source_consult_keeps_gate_and_new_creature_damage() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ascending Creature")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text("When this creature deals combat damage to a player, sacrifice it. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield, then shuffle the rest into your library. If that creature is a Demon, it deals damage equal to its power to each opponent.").unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("sacrifice it. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield, then shuffle the rest into your library"), "{rendered}");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        crate::ability::AbilityKind::Triggered(trigger) => Some(trigger), _ => None,
    }).unwrap();
    for demon in [false, true] {
    for sacrifice_available in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Revealed Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![if demon { Subtype::Demon } else { Subtype::Elf }])
            .power_toughness(crate::card::PowerToughness::fixed(5, 5)).build();
        game.create_object_from_card(&creature, alice, Zone::Library);
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Revealed Land")
            .card_types(vec![CardType::Land]).build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Library);
        let event = crate::events::DamageEvent::with_cause(source, crate::events::DamageTarget::Player(bob), 3, true,
            crate::events::cause::EventCause::from_sba());
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        ctx.triggering_event = Some(crate::triggers::TriggerEvent::new_with_provenance(event, crate::provenance::ProvNodeId::default()));
        let before = game.irreversible_random_count();
        for segment in &trigger.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                if !sacrifice_available && effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>().is_some() {
                    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                }
            }
        }
        assert_eq!(game.battlefield.len(), usize::from(sacrifice_available));
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        if sacrifice_available {
            assert_eq!(game.player(alice).unwrap().library, vec![land_id]);
        } else {
            assert_eq!(game.player(alice).unwrap().library.len(), 2);
        }
        assert_eq!(game.irreversible_random_count(), before + 1);
        assert_eq!(game.players[0].life, 20);
        assert_eq!(game.players[1].life, if demon && sacrifice_available { 15 } else { 20 });
        assert_eq!(game.players[2].life, if demon && sacrifice_available { 15 } else { 20 });
    }
}
}
