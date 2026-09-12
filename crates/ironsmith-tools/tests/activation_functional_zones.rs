//! CR 113.6m: source-zone inference must survive compilation/materialization
//! and govern the actual priority menu, payment, and resolution.
use ironsmith::decision::{AutoPassDecisionMaker, LegalAction, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_tools::parse_card_definition_with_runtime_builder;

const CASES: &[(&str, &str, &[Zone])] = &[
    (
        "Slimefoot and Squee",
        "{1}{B}{R}{G}, Sacrifice a Saproling: Return this card and up to one other target creature card from your graveyard to the battlefield. Activate only as a sorcery.",
        &[Zone::Graveyard],
    ),
    (
        "Sandman, Shifting Scoundrel",
        "{3}{G}{G}: Return this card and target land card from your graveyard to the battlefield tapped.",
        &[Zone::Graveyard],
    ),
    (
        "Say Its Name",
        "Exile this card and two other cards named Say Its Name from your graveyard: Search your graveyard, hand, and/or library for a card named Altanak, the Thrice-Called and put it onto the battlefield. If you search your library this way, shuffle. Activate only as a sorcery.",
        &[Zone::Graveyard],
    ),
    (
        "Torrent Elemental",
        "{3}{B/G}{B/G}: Put this card from exile onto the battlefield tapped. Activate only as a sorcery.",
        &[Zone::Exile],
    ),
    (
        "Carrionette",
        "{2}{B}{B}: Exile this card and target creature unless that creature's controller pays {2}. Activate only if this card is in your graveyard.",
        &[Zone::Graveyard],
    ),
    (
        "Skyblade's Boon",
        "{2}{W}: Return Skyblade's Boon to its owner's hand. Activate only if Skyblade's Boon is on the battlefield or in your graveyard.",
        &[Zone::Battlefield, Zone::Graveyard],
    ),
    (
        "Glory",
        "{2}{W}: Choose a color. Creatures you control gain protection from the chosen color until end of turn. Activate only if this card is in your graveyard.",
        &[Zone::Graveyard],
    ),
    (
        "Loathsome Troll",
        "{3}{G}: Roll a d20. Activate only if this card is in your graveyard.\n1—9 | Put this card on top of your library.\n10—19 | Return this card to your hand.\n20 | Return this card to the battlefield tapped.",
        &[Zone::Graveyard],
    ),
    (
        "Kethis, the Hidden Hand",
        r#"Exile two legendary cards from your graveyard: Until end of turn, each legendary card in your graveyard gains "You may play this card from your graveyard.""#,
        &[Zone::Battlefield],
    ),
];

fn definition(name: &str, text: &str) -> CardDefinition {
    let metadata = match name {
        "Say Its Name" => "Type: Sorcery",
        "Skyblade's Boon" => "Type: Enchantment",
        _ => "Type: Creature\nPower/Toughness: 3/3",
    };
    parse_card_definition_with_runtime_builder(name, format!("{metadata}\n{text}"), false).unwrap()
}

fn setup(def: &CardDefinition, zone: Zone) -> (GameState, PlayerId, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    let source = game.create_object_from_definition(def, alice, zone);
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(alice).unwrap().mana_pool.add(symbol, 10);
    }
    let saproling = parse_card_definition_with_runtime_builder(
        "Saproling",
        "Type: Creature — Saproling\nPower/Toughness: 1/1",
        false,
    )
    .unwrap();
    game.create_object_from_definition(&saproling, alice, Zone::Battlefield);
    let land = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::ids::CardId::new(),
        "Land",
    )
    .card_types(vec![CardType::Land])
    .build();
    game.create_object_from_definition(&land, alice, Zone::Graveyard);
    if def.card.name == "Say Its Name" {
        game.create_object_from_definition(def, alice, Zone::Graveyard);
        game.create_object_from_definition(def, alice, Zone::Graveyard);
    }
    if def.card.name == "Kethis, the Hidden Hand" {
        let offering = parse_card_definition_with_runtime_builder(
            "Legendary Offering",
            "Type: Legendary Creature\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&offering, alice, Zone::Graveyard);
        game.create_object_from_definition(&offering, alice, Zone::Graveyard);
    }
    (game, alice, source)
}

#[test]
fn rule_113_6m_legal_menu_matches_source_zones() {
    for &(name, text, expected) in CASES {
        let def = definition(name, text);
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
            .unwrap();
        assert_eq!(ability.functional_zones, expected, "{name}");
        for zone in [
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Hand,
            Zone::Exile,
            Zone::Command,
            Zone::Library,
        ] {
            let (game, alice, source) = setup(&def, zone);
            let offered = compute_legal_actions(&game, alice).iter().any(
                |a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source),
            );
            assert_eq!(offered, expected.contains(&zone), "{name} in {zone:?}");
        }
    }
}

#[test]
fn rule_113_6m_activations_pay_and_resolve_from_the_required_zone() {
    for &(name, text, expected) in CASES {
        let def = definition(name, text);
        let start_zone = if name == "Skyblade's Boon" {
            Zone::Graveyard
        } else {
            expected[0]
        };
        let (mut game, alice, source) = setup(&def, start_zone);
        let stable_id = game.object(source).unwrap().stable_id;
        let before_mana = game.player(alice).unwrap().mana_pool.total();
        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
            .expect(name);
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = AutoPassDecisionMaker;
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..20 {
            if !game.stack.is_empty() {
                break;
            }
            let ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{name}: {progress:?}")
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1, "{name}");
        if name != "Say Its Name" && name != "Kethis, the Hidden Hand" {
            assert!(
                game.player(alice).unwrap().mana_pool.total() < before_mana,
                "{name} must pay mana"
            );
        } else if name == "Say Its Name" {
            assert_eq!(
                game.exile
                    .iter()
                    .filter(|id| game.object(**id).unwrap().name == name)
                    .count(),
                3
            );
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let current_id = game
            .find_object_by_stable_id(stable_id)
            .expect("source must still exist");
        let current_zone = game.object(current_id).unwrap().zone;
        let destinations: &[Zone] = match name {
            "Say Its Name" | "Carrionette" => &[Zone::Exile],
            "Skyblade's Boon" => &[Zone::Hand],
            "Glory" => &[Zone::Graveyard],
            "Loathsome Troll" => &[Zone::Library, Zone::Hand, Zone::Battlefield],
            _ => &[Zone::Battlefield],
        };
        assert!(
            destinations.contains(&current_zone),
            "{name} ended in {current_zone:?}"
        );
        if name == "Kethis, the Hidden Hand" {
            assert_eq!(
                game.exile
                    .iter()
                    .filter(|id| game.object(**id).unwrap().name == "Legendary Offering")
                    .count(),
                2
            );
        }
        if name == "Slimefoot and Squee" {
            assert!(
                !game
                    .battlefield
                    .iter()
                    .any(|id| game.object(*id).unwrap().name == "Saproling"),
                "the Saproling must be sacrificed"
            );
        }
        if name == "Sandman, Shifting Scoundrel" {
            assert!(
                game.battlefield
                    .iter()
                    .any(|id| game.object(*id).unwrap().name == "Land"),
                "the targeted land must return too"
            );
        }
    }
}

#[test]
fn compound_exile_cost_requires_two_other_matching_cards_in_your_graveyard() {
    let (name, text, _) = CASES[2];
    let def = definition(name, text);
    let (mut game, alice, source) = setup(&def, Zone::Graveyard);
    let other = *game
        .player(alice)
        .unwrap()
        .graveyard
        .iter()
        .find(|id| **id != source && game.object(**id).unwrap().name == name)
        .unwrap();
    game.move_object_by_effect(other, Zone::Hand).unwrap();
    assert!(
        !compute_legal_actions(&game, alice)
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source)),
        "the source is not one of the two other cards, and a copy in hand does not qualify"
    );
    let bob = game.players[1].id;
    game.create_object_from_definition(&def, bob, Zone::Graveyard);
    assert!(
        !compute_legal_actions(&game, alice)
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source)),
        "an opponent's copy cannot pay the cost"
    );
}

#[test]
fn carrionette_offers_payment_to_the_targets_controller_before_exiling_either_object() {
    struct PayDecision {
        payer: PlayerId,
        accept: bool,
        asked: bool,
    }
    impl ironsmith::decision::DecisionMaker for PayDecision {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            assert_eq!(ctx.player, self.payer);
            self.asked = true;
            self.accept
        }
    }
    for accept in [false, true] {
        let (name, text, _) = CASES[4];
        let def = definition(name, text);
        let (mut game, alice, source) = setup(&def, Zone::Graveyard);
        let source_stable = game.object(source).unwrap().stable_id;
        let bob = game.players[1].id;
        let victim = definition("Victim", "");
        let target = game.create_object_from_definition(&victim, bob, Zone::Battlefield);
        let target_stable = game.object(target).unwrap().stable_id;
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let AbilityKind::Activated(activated) = &def.abilities[0].kind else {
            panic!("activated ability")
        };
        game.push_to_stack(
            ironsmith::game_state::StackEntry::ability(source, alice, activated.effects.clone())
                .with_targets(vec![ironsmith::game_state::Target::Object(target)]),
        );
        let mut dm = PayDecision {
            payer: bob,
            accept,
            asked: false,
        };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.asked);
        let source_zone = game
            .object(game.find_object_by_stable_id(source_stable).unwrap())
            .unwrap()
            .zone;
        let target_zone = game
            .object(game.find_object_by_stable_id(target_stable).unwrap())
            .unwrap()
            .zone;
        assert_eq!(
            source_zone,
            if accept { Zone::Graveyard } else { Zone::Exile }
        );
        assert_eq!(
            target_zone,
            if accept {
                Zone::Battlefield
            } else {
                Zone::Exile
            }
        );
        assert_eq!(
            game.player(bob).unwrap().mana_pool.total(),
            if accept { 0 } else { 2 }
        );
    }
}
