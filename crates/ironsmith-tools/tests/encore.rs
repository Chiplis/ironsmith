//! Encore is executable graveyard text, including its multiplayer combat requirement.
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, AutoPassDecisionMaker, LegalAction, compute_legal_actions,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{AbilityKind, GameState, ObjectId, PlayerId, Zone};
use ironsmith_tools::parse_card_definition_with_runtime_builder;

fn setup(zone: Zone) -> (GameState, PlayerId, ObjectId) {
    let def = parse_card_definition_with_runtime_builder(
        "Encore test",
        "Type: Creature\nPower/Toughness: 2/2\nEncore {2}",
        false,
    )
    .unwrap();
    let ability = &def.abilities[0];
    assert!(matches!(ability.kind, AbilityKind::Activated(_)));
    assert_eq!(ability.functional_zones, [Zone::Graveyard]);
    assert_eq!(
        ironsmith::compiled_text::ability_surface_text(ability),
        "Encore {2}"
    );
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = game.players[0].id;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 10);
    let source = game.create_object_from_definition(&def, alice, zone);
    (game, alice, source)
}
fn activate(game: &mut GameState, alice: PlayerId, source: ObjectId) {
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source))
        .unwrap();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = AutoPassDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
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
            panic!("{progress:?}")
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.exile.len(), 1);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 8);
    assert!(game.battlefield.is_empty());
}
fn resolve() -> (GameState, Vec<AttackerDeclaration>) {
    let (mut game, alice, source) = setup(Zone::Graveyard);
    activate(&mut game, alice, source);
    // The exiled source can leave exile before resolution: the ability must use LKI.
    let exiled = game.exile[0];
    let new_identity = game
        .move_object(exiled, Zone::Hand, ironsmith::events::EventCause::effect())
        .unwrap();
    game.object_mut(new_identity).unwrap().base_power = Some(ironsmith::card::PtValue::Fixed(99));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.battlefield.len(), 2);
    assert_eq!(game.effect_store.delayed_triggers.len(), 2);
    let declarations = game
        .battlefield
        .iter()
        .map(|&id| {
            assert!(game.object_has_static_ability_id(id, StaticAbilityId::Haste));
            assert!(
                !game
                    .object(id)
                    .unwrap()
                    .has_static_ability_id(StaticAbilityId::Haste),
                "haste must not become a copiable characteristic"
            );
            assert_eq!(
                game.object(id).unwrap().power(),
                Some(2),
                "copy the departed source, not a later object"
            );
            assert!(!game.is_tapped(id));
            let players = game
                .required_attack_players_this_turn(id)
                .collect::<Vec<_>>();
            assert_eq!(players.len(), 1);
            AttackerDeclaration {
                creature: id,
                target: AttackTarget::Player(players[0]),
            }
        })
        .collect();
    (game, declarations)
}
#[test]
fn encore_activation_zone_and_timing() {
    for zone in [
        Zone::Battlefield,
        Zone::Hand,
        Zone::Library,
        Zone::Command,
        Zone::Exile,
        Zone::Graveyard,
    ] {
        let (mut game, alice, source) = setup(zone);
        let offered = |game: &GameState| {
            compute_legal_actions(game, alice).iter().any(
                |a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source),
            )
        };
        assert_eq!(offered(&game), zone == Zone::Graveyard);
        game.turn.phase = Phase::Combat;
        assert!(!offered(&game));
    }
}
#[test]
fn encore_multiplayer_copy_and_attack_requirements() {
    let (game, declarations) = resolve();
    assert_ne!(declarations[0].target, declarations[1].target);
    let check = |decls: &[AttackerDeclaration]| {
        apply_attacker_declarations(
            &mut game.clone(),
            &mut CombatState::default(),
            &mut ironsmith::triggers::TriggerQueue::new(),
            decls,
        )
    };
    assert!(check(&[]).is_err());
    let mut wrong = declarations.clone();
    wrong[0].target = declarations[1].target.clone();
    assert!(check(&wrong).is_err());
    assert!(check(&declarations).is_ok());
    let mut expired = game.clone();
    expired.turn.turn_number += 1;
    assert!(
        expired
            .required_attack_players_this_turn(declarations[0].creature)
            .next()
            .is_none()
    );
}
#[test]
fn encore_does_not_force_attack_taxes() {
    let (mut game, declarations) = resolve();
    let AttackTarget::Player(bob) = declarations[0].target else {
        unreachable!()
    };
    let tax =
        parse_card_definition_with_runtime_builder("Tax", "Type: Enchantment", false).unwrap();
    let tax = game.create_object_from_definition(&tax, bob, Zone::Battlefield);
    game.object_mut(tax)
        .unwrap()
        .abilities_mut()
        .push(ironsmith::Ability::static_ability(
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(1),
        ));
    game.refresh_continuous_state();
    let mut alternative = declarations.clone();
    alternative[0].target = declarations[1].target.clone();
    apply_attacker_declarations(
        &mut game.clone(),
        &mut CombatState::default(),
        &mut ironsmith::triggers::TriggerQueue::new(),
        &alternative,
    )
    .expect("a token may attack elsewhere instead of paying its assigned opponent's tax");

    apply_attacker_declarations(
        &mut game,
        &mut CombatState::default(),
        &mut ironsmith::triggers::TriggerQueue::new(),
        &declarations[1..],
    )
    .expect("taxed token may decline to attack");
}
#[test]
fn encore_cleanup_keeps_identity_and_controller() {
    let (mut game, declarations) = resolve();
    let stolen = declarations[0].creature;
    let bob = game.players[1].id;
    game.set_current_controller(stolen, bob);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::BeginningOfEndStepEvent::new(bob),
        Default::default(),
    );
    let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &event);
    assert_eq!(entries.len(), 2);
    assert!(game.effect_store.delayed_triggers.is_empty());
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    }
    assert!(
        game.object(stolen)
            .is_some_and(|o| o.zone == Zone::Battlefield)
    );
    assert!(!game.battlefield.contains(&declarations[1].creature));
}

#[test]
fn encore_compiles_as_an_activation_for_all_23_previously_compiled_candidates() {
    let names = [
        "Amphin Mutineer",
        "Angel of Indemnity",
        "Belonging",
        "Briarblade Adept",
        "Broodmate Tyrant",
        "Coastline Marauders",
        "Elvish Dreadlord",
        "Exquisite Huntmaster",
        "Fin-Clade Fugitives",
        "Impulsive Pilferer",
        "Impulsivity",
        "Jubilation",
        "Kangee's Lieutenant",
        "Kinsbaile Courier",
        "Kitesail Skirmisher",
        "Lamentation",
        "Mist Dancer",
        "Phyrexian Triniform",
        "Rakshasa Debaser",
        "Soul of Eternity",
        "Spellbinding Soprano",
        "Subterfuge",
        "Trove Tracker",
    ]
    .map(str::to_owned);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cards.json");
    let payloads =
        ironsmith_tools::load_card_payloads_by_names(path.to_str().unwrap(), &names).unwrap();
    for name in names {
        let faces = &payloads[&name];
        let mut found = false;
        for payload in faces {
            let def = ironsmith_tools::compile_definition_from_payload(payload)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            for ability in &def.abilities {
                if ironsmith::compiled_text::ability_surface_text(ability).starts_with("Encore ") {
                    found = true;
                    assert!(matches!(ability.kind, AbilityKind::Activated(_)), "{name}");
                    assert_eq!(ability.functional_zones, [Zone::Graveyard], "{name}");
                }
            }
        }
        assert!(found, "missing executable Encore for {name}");
    }
}

#[test]
fn encore_replacements_keep_each_opponents_assignment_and_cleanup() {
    let (mut game, alice, source) = setup(Zone::Graveyard);
    activate(&mut game, alice, source);
    let def =
        parse_card_definition_with_runtime_builder("Doubler", "Type: Enchantment", false).unwrap();
    let doubler = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.object_mut(doubler)
        .unwrap()
        .abilities_mut()
        .push(ironsmith::Ability::static_ability(
            StaticAbility::double_token_creation_replacement(
                ironsmith::target::PlayerFilter::You,
                "Double tokens".into(),
            ),
        ));
    game.refresh_continuous_state();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.battlefield.len(), 5);
    assert_eq!(game.effect_store.delayed_triggers.len(), 4);
    for opponent in [game.players[1].id, game.players[2].id] {
        let tokens = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.required_attack_players_this_turn(id)
                    .any(|p| p == opponent)
            })
            .collect::<Vec<_>>();
        assert_eq!(tokens.len(), 2);
        for &&id in &tokens {
            assert!(game.object_has_static_ability_id(id, StaticAbilityId::Haste));
        }
    }
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(game.effect_store.attack_player_requirements.is_empty());
}

#[test]
fn encore_countered_activation_does_not_refund_cost() {
    let (mut game, alice, source) = setup(Zone::Graveyard);
    activate(&mut game, alice, source);
    game.stack.clear();
    assert_eq!(game.exile.len(), 1);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 8);
    assert!(game.battlefield.is_empty());
    assert!(game.effect_store.delayed_triggers.is_empty());
}

#[test]
fn encore_ignores_departed_opponents_at_resolution_and_in_combat() {
    let (mut game, alice, source) = setup(Zone::Graveyard);
    activate(&mut game, alice, source);
    let departed = game.players[2].id;
    game.player_mut(departed).unwrap().has_left_game = true;
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.battlefield.len(), 1);
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);

    let (mut game, declarations) = resolve();
    let AttackTarget::Player(departed) = declarations[0].target else {
        unreachable!()
    };
    game.player_mut(departed).unwrap().has_left_game = true;
    apply_attacker_declarations(
        &mut game,
        &mut CombatState::default(),
        &mut ironsmith::triggers::TriggerQueue::new(),
        &declarations[1..],
    )
    .expect("a token need not attack a player who left");
}
