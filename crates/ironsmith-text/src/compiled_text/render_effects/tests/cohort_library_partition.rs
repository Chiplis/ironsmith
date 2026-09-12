use super::*;

const SACRIFICE: &str = "The controller of target artifact sacrifices it, then reveals cards from the top of their library until they reveal an artifact card. That player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.";
const OPPONENT: &str = "Reveal the top six cards of your library. An opponent exiles a nonland card from among them, then you put the rest into your hand. That opponent may cast the exiled card without paying its mana cost.";

#[test]
fn cohort_library_partition_renders_typed_exile_and_controller_consult() {
    for text in [SACRIFICE, OPPONENT] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Partition Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&card).join("\n"),
            text
        );
    }
}

#[test]
fn cohort_library_partition_consults_the_sacrificed_targets_controller_library() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Partition Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(SACRIFICE)
        .unwrap();
    let artifact = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Artifact Probe")
        .card_types(vec![CardType::Artifact])
        .build();
    let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Land Probe")
        .card_types(vec![CardType::Land])
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let target = game.create_object_from_definition(&artifact, bob, Zone::Battlefield);
    let own_card = game.create_object_from_definition(&artifact, alice, Zone::Library);
    let matching = game.create_object_from_definition(&artifact, bob, Zone::Library);
    let remaining = game.create_object_from_definition(&land, bob, Zone::Library);
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    // The stack-resolution entry point captures target LKI before executing
    // a spell, so controller references survive the sacrifice's zone change.
    ctx.snapshot_targets(&game);
    for effect in spell
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert!(game.player(alice).unwrap().library.contains(&own_card));
    assert_eq!(game.player(bob).unwrap().library.len(), 1);
    // Library moves can mint fresh IDs, so verify remaining card identity by name.
    assert_eq!(
        game.object(game.player(bob).unwrap().library[0])
            .unwrap()
            .name,
        "Land Probe"
    );
    assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
    assert!(game.battlefield.iter().any(|id| {
        game.object(*id)
            .is_some_and(|o| o.name == "Artifact Probe" && game.controller_of(o) == bob)
    }));
    let _ = (matching, remaining);
}

#[test]
fn cohort_library_partition_opponent_exiles_one_and_all_five_remaining_cards_enter_hand() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Partition Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(OPPONENT)
        .unwrap();
    let artifact = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Artifact Probe")
        .card_types(vec![CardType::Artifact])
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
    for _ in 0..7 {
        game.create_object_from_definition(&artifact, alice, Zone::Library);
    }
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);
    for effect in spell
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().hand.len(), 5);
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert!(game.player(bob).unwrap().hand.is_empty());
    assert_eq!(game.exile.len(), 1);
}

#[test]
fn cohort_library_partition_named_and_counted_reveals_accept_typed_exile() {
    for (name, text) in [
        (
            "Kindred Summons",
            "Choose a creature type. Reveal cards from the top of your library until you reveal X creature cards of the chosen type, where X is the number of creatures you control of that type. Put those cards onto the battlefield, then shuffle the rest of the revealed cards into your library.",
        ),
        (
            "Stitcher Geralf",
            "{2}{U}, {T}: Each player mills three cards. Exile up to two creature cards put into graveyards this way. Create an X/X blue Zombie creature token, where X is the total power of the cards exiled this way.",
        ),
        (
            "Demonic Consultation",
            "Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way.",
        ),
        (
            "Divining Witch",
            "{1}{B}, {T}, Discard a card: Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way.",
        ),
    ] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
            .parse_text(text)
            .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&card).join("\n"),
            text
        );
    }
}

#[test]
fn cohort_exile_and_nested_modifier_surfaces() {
    for (types, text) in [
        (
            vec![CardType::Sorcery],
            "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn.",
        ),
        (
            vec![CardType::Instant],
            "Until end of turn, target creature gets +2/+2, gains flying, and becomes a Horror enchantment creature in addition to its other types.",
        ),
        (
            vec![CardType::Instant],
            "Surface Probe deals X damage to target creature. When that creature dies this turn, exile a number of cards from the top of your library equal to its power, then choose a card exiled this way. Until the end of your next turn, you may play that card.",
        ),
        (
            vec![CardType::Enchantment],
            "At the beginning of your upkeep, put a despair counter on this enchantment, then each player exiles X permanents they control and/or cards from their hand, where X is the number of despair counters on this enchantment.",
        ),
        (
            vec![CardType::Sorcery],
            "Converge — Exile target nonland permanent if its mana value is less than or equal to the number of colors of mana spent to cast this spell.",
        ),
    ] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Surface Probe")
            .card_types(types)
            .parse_text(text)
            .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&card).join("\n"),
            text.strip_prefix("Converge — ").unwrap_or(text)
        );
    }
}

#[test]
fn cohort_library_partition_keeps_each_optional_searchers_cards_and_decisions() {
    struct SearchChoices {
        declining: crate::ids::PlayerId,
    }
    impl crate::decision::DecisionMaker for SearchChoices {
        fn decide_boolean(
            &mut self,
            _: &crate::game_state::GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            ctx.player != self.declining
        }
        fn decide_objects(
            &mut self,
            _: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .take(ctx.max.unwrap_or(ctx.min))
                .map(|c| c.id)
                .collect()
        }
    }
    let text = "Each opponent may search their library for up to three basic land cards. They each put one of those cards onto the battlefield tapped under your control and the rest onto the battlefield tapped under their control. Then each player who searched their library this way shuffles.";
    let triggered_text = format!("When this creature enters, {}", lowercase_first(text));
    let triggered = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search Trigger Probe")
        .card_types(vec![CardType::Creature]).parse_text(&triggered_text).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&triggered).join("\n"), triggered_text);
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .unwrap();
    let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Basic Probe")
        .card_types(vec![CardType::Land])
        .supertypes(vec![crate::types::Supertype::Basic])
        .build();
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Dana".into(),
        ],
        20,
    );
    let players = game.players.iter().map(|p| p.id).collect::<Vec<_>>();
    let alice = players[0];
    let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
    for player in &players {
        for _ in 0..4 {
            game.create_object_from_definition(&land, *player, Zone::Library);
        }
    }
    let mut choices = SearchChoices {
        declining: players[3],
    };
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut choices);
    for effect in spell
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.battlefield.len(), 6);
    for player in [players[1], players[2]] {
        assert_eq!(game.player(player).unwrap().library.len(), 1);
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.controller_of_id(**id) == Some(player))
                .count(),
            2
        );
    }
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|id| game.controller_of_id(**id) == Some(alice))
            .count(),
        2
    );
    assert_eq!(game.player(alice).unwrap().library.len(), 4);
    assert_eq!(game.player(players[3]).unwrap().library.len(), 4);
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&spell).join("\n"),
        text
    );
}
