#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const AKRON_LEGIONNAIRE_ORACLE: &str = "Except for creatures named Akron Legionnaire and artifact creatures, creatures you control can't attack.";
const ARCHON_OF_ABSOLUTION_ORACLE: &str = "Flying\nProtection from white\nCreatures can't attack you or planeswalkers you control unless their controller pays {1} for each of those creatures.";
const LOOT_DISPUTE_ORACLE: &str = "When this enchantment enters, you take the initiative and create a Treasure token.\nWhenever you attack the player who has the initiative, create a Treasure token.\nLoud Ruckus — Whenever you complete a dungeon, create a 5/5 red Dragon creature token with flying.";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn planeswalker(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build()
}

fn token_count(game: &crate::GameState, controller: PlayerId, name: &str) -> usize {
    game.battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| {
            object.kind == crate::object::ObjectKind::Token
                && game.controller_of(object) == controller
                && object.name == name
        })
        .count()
}

fn trigger_event<E: crate::events::GameEventType + 'static>(
    event: E,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        event,
        crate::provenance::ProvNodeId::default(),
    )
}

fn source_triggers(
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn resolve_source_trigger(
    game: &mut crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) {
    let entries = source_triggers(game, source, event);
    assert_eq!(entries.len(), 1, "expected exactly one trigger from source");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("source trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(game).expect("source trigger should resolve");
}

#[test]
fn exact_score_cards_retain_their_complete_canonical_rules_surfaces() {
    for (name, oracle) in [
        ("Akron Legionnaire", AKRON_LEGIONNAIRE_ORACLE),
        ("Archon of Absolution", ARCHON_OF_ABSOLUTION_ORACLE),
        ("Loot Dispute", LOOT_DISPUTE_ORACLE),
    ] {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition).join("\n"),
            oracle,
            "{name} should render its exact Oracle surface: {definition:#?}"
        );
    }
}

#[test]
fn akron_legionnaire_restricts_only_its_controllers_ordinary_nonartifact_creatures() {
    let definition = parse_oracle_card_definition("Akron Legionnaire");
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let second_akron = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let artifact_creature_definition = CardDefinitionBuilder::new(CardId::new(), "Clockwork Ally")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let artifact_creature =
        game.create_object_from_definition(&artifact_creature_definition, alice, Zone::Battlefield);
    let alice_ordinary =
        game.create_object_from_definition(&creature("Alice Ordinary"), alice, Zone::Battlefield);
    let bob_ordinary =
        game.create_object_from_definition(&creature("Bob Ordinary"), bob, Zone::Battlefield);

    game.refresh_continuous_state();
    game.update_cant_effects();

    for exempt in [source, second_akron, artifact_creature] {
        assert!(
            game.can_attack(exempt),
            "Akron itself, another creature named Akron Legionnaire, and artifact creatures are exempt"
        );
    }
    assert!(
        !game.can_attack(alice_ordinary),
        "Alice's ordinary nonartifact creature should be unable to attack"
    );
    assert!(
        game.can_attack(bob_ordinary),
        "Akron must not restrict an opponent's ordinary creature"
    );
}

fn declare_attackers(
    game: &mut crate::GameState,
    declarations: &[crate::decision::AttackerDeclaration],
) -> Result<(), crate::game_loop::GameLoopError> {
    let mut combat = crate::combat_state::CombatState::default();
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, declarations)
}

#[test]
fn archon_of_absolution_taxes_attacks_at_its_controller_and_their_planeswalkers_only() {
    let definition = parse_oracle_card_definition("Archon of Absolution");
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let walker =
        game.create_object_from_definition(&planeswalker("Alice Walker"), alice, Zone::Battlefield);
    let siege_definition = CardDefinitionBuilder::new(CardId::new(), "Cara's Siege")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .defense(4)
        .build();
    let siege = game.create_object_from_definition(&siege_definition, cara, Zone::Battlefield);
    assert!(game.set_battle_protector(siege, alice));
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.refresh_continuous_state();

    let make_attacker = |game: &mut crate::GameState, name: &str| {
        let attacker = game.create_object_from_definition(&creature(name), bob, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);
        attacker
    };

    let at_alice = make_attacker(&mut game, "At Alice");
    assert!(
        declare_attackers(
            &mut game,
            &[crate::decision::AttackerDeclaration {
                creature: at_alice,
                target: crate::combat_state::AttackTarget::Player(alice),
            }],
        )
        .is_err(),
        "attacking Archon's controller without mana must fail"
    );

    let at_walker = make_attacker(&mut game, "At Alice Walker");
    assert!(
        declare_attackers(
            &mut game,
            &[crate::decision::AttackerDeclaration {
                creature: at_walker,
                target: crate::combat_state::AttackTarget::Planeswalker(walker),
            }],
        )
        .is_err(),
        "the historical planeswalker omission must remain fixed"
    );

    let at_siege = make_attacker(&mut game, "At Alice-Protected Siege");
    declare_attackers(
        &mut game,
        &[crate::decision::AttackerDeclaration {
            creature: at_siege,
            target: crate::combat_state::AttackTarget::Battle(siege),
        }],
    )
    .expect("Archon's player/planeswalker tax must not spill onto a Battle");

    let at_cara = make_attacker(&mut game, "At Cara");
    declare_attackers(
        &mut game,
        &[crate::decision::AttackerDeclaration {
            creature: at_cara,
            target: crate::combat_state::AttackTarget::Player(cara),
        }],
    )
    .expect("Archon must not tax attacks at another player");

    game.player_mut(bob)
        .expect("Bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let paid_player_attacker = make_attacker(&mut game, "Paid At Alice");
    let paid_walker_attacker = make_attacker(&mut game, "Paid At Alice Walker");
    declare_attackers(
        &mut game,
        &[
            crate::decision::AttackerDeclaration {
                creature: paid_player_attacker,
                target: crate::combat_state::AttackTarget::Player(alice),
            },
            crate::decision::AttackerDeclaration {
                creature: paid_walker_attacker,
                target: crate::combat_state::AttackTarget::Planeswalker(walker),
            },
        ],
    )
    .expect("two protected attackers should succeed after paying one mana apiece");
    assert_eq!(
        game.player(bob).expect("Bob exists").mana_pool.total(),
        0,
        "the attack declaration should spend exactly two mana"
    );
}

#[test]
fn loot_dispute_entry_takes_the_initiative_and_creates_exactly_one_treasure() {
    let definition = parse_oracle_card_definition("Loot Dispute");
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let in_hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let source = game
        .move_object_by_effect(in_hand, Zone::Battlefield)
        .expect("Loot Dispute should enter the battlefield");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue
            .entries
            .iter()
            .filter(|entry| entry.source == source)
            .count(),
        1,
        "Loot Dispute should queue one combined ETB trigger"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Loot Dispute's ETB trigger should go on the stack");
    while !game.stack.is_empty() {
        crate::game_loop::resolve_stack_entry(&mut game)
            .expect("Loot Dispute's ETB trigger should resolve");
    }
    assert_eq!(game.initiative, Some(alice));
    assert_eq!(token_count(&game, alice, "Treasure"), 1);
}

#[test]
fn loot_dispute_attack_trigger_tracks_the_live_initiative_holder_and_requires_the_player() {
    let definition = parse_oracle_card_definition("Loot Dispute");
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bob_walker =
        game.create_object_from_definition(&planeswalker("Bob Walker"), bob, Zone::Battlefield);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let run_attack = |game: &mut crate::GameState,
                      initiative: Option<PlayerId>,
                      target: crate::combat_state::AttackTarget,
                      label: &str| {
        game.set_initiative(initiative);
        let attacker =
            game.create_object_from_definition(&creature(label), alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);
        let before = token_count(game, alice, "Treasure");
        let mut combat = crate::combat_state::CombatState::default();
        let mut queue = crate::triggers::TriggerQueue::new();
        crate::game_loop::apply_attacker_declarations(
            game,
            &mut combat,
            &mut queue,
            &[crate::decision::AttackerDeclaration {
                creature: attacker,
                target,
            }],
        )
        .expect("probe attack should be legal");
        let source_entries = queue
            .entries
            .iter()
            .filter(|entry| entry.source == source)
            .count();
        if !queue.entries.is_empty() {
            crate::game_loop::put_triggers_on_stack(game, &mut queue)
                .expect("attack triggers should go on the stack");
            while !game.stack.is_empty() {
                crate::game_loop::resolve_stack_entry(game)
                    .expect("attack triggers should resolve");
            }
        }
        (
            source_entries,
            token_count(game, alice, "Treasure") - before,
        )
    };

    assert_eq!(
        run_attack(
            &mut game,
            Some(bob),
            crate::combat_state::AttackTarget::Player(bob),
            "Attack Current Holder",
        ),
        (1, 1),
        "attacking the current initiative-holding player should create one Treasure"
    );
    assert_eq!(
        run_attack(
            &mut game,
            Some(alice),
            crate::combat_state::AttackTarget::Player(bob),
            "Attack Former Holder",
        ),
        (0, 0),
        "the matcher must follow the live designation rather than a cached player"
    );
    assert_eq!(
        run_attack(
            &mut game,
            None,
            crate::combat_state::AttackTarget::Player(bob),
            "Attack Without Initiative",
        ),
        (0, 0),
        "there is no matching player while nobody has the initiative"
    );
    assert_eq!(
        run_attack(
            &mut game,
            Some(bob),
            crate::combat_state::AttackTarget::Planeswalker(bob_walker),
            "Attack Holder's Walker",
        ),
        (0, 0),
        "the Oracle text names the player, not a planeswalker they control"
    );
}

#[test]
fn loot_dispute_loud_ruckus_triggers_only_when_its_controller_completes_a_dungeon() {
    let definition = parse_oracle_card_definition("Loot Dispute");
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let bobs_completion = trigger_event(crate::events::KeywordActionEvent::new(
        crate::events::KeywordActionKind::CompleteDungeon,
        bob,
        source,
        1,
    ));
    assert!(
        source_triggers(&game, source, &bobs_completion).is_empty(),
        "another player's completed dungeon must not trigger Loot Dispute"
    );

    let alices_completion = trigger_event(crate::events::KeywordActionEvent::new(
        crate::events::KeywordActionKind::CompleteDungeon,
        alice,
        source,
        1,
    ));
    resolve_source_trigger(&mut game, source, &alices_completion);
    let dragons = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Token
                    && game.controller_of(object) == alice
                    && game.current_has_subtype(*id, Subtype::Dragon)
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(dragons.len(), 1);
    let dragon = dragons[0];
    assert_eq!(game.current_power(dragon), Some(5));
    assert_eq!(game.current_toughness(dragon), Some(5));
    assert_eq!(
        game.current_colors(dragon),
        Some(crate::color::ColorSet::RED)
    );
    assert!(game.current_has_static_ability_id(dragon, StaticAbilityId::Flying));
}
