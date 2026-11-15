//! "Counter target spell if it would <act on> <objects>" (Equinox): the spell
//! is judged as if it resolved right now, and an action that only happens if
//! a player makes a choice doesn't count (Equinox rulings).
use super::*;

const EQUINOX: &str = "Counter target spell if it would destroy a land you control.";
const EXILE_GUARD: &str = "Counter target spell if it would exile a land you control.";
const BOUNCE_GUARD: &str =
    "Counter target spell if it would return a land you control to its owner's hand.";
const SACRIFICE_GUARD: &str = "Counter target spell if it would sacrifice a land you control.";

#[derive(Clone, Copy)]
struct Board {
    alice_lands: usize,
    bob_lands: usize,
    life: i32,
    caster_zombie: bool,
    /// Alice also controls a creature (a permanent that isn't a land).
    alice_creature: bool,
    /// The spell also targets Alice herself.
    target_alice: bool,
}

const BOARD: Board = Board {
    alice_lands: 7,
    bob_lands: 6,
    life: 20,
    caster_zombie: false,
    alice_creature: false,
    target_alice: false,
};

/// Bob casts `text` (targeting one of Alice's lands when `target_land`), and
/// Alice resolves `guard` against it. Returns whether it was countered.
fn guard_counters(
    guard: &str,
    text: &str,
    board: Board,
    target_land: bool,
    modes: Option<Vec<usize>>,
) -> bool {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], board.life);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Plains")
        .card_types(vec![CardType::Land])
        .build();
    let alice_lands: Vec<_> = (0..board.alice_lands)
        .map(|_| game.create_object_from_definition(&land, alice, Zone::Battlefield))
        .collect();
    for _ in 0..board.bob_lands {
        game.create_object_from_definition(&land, bob, Zone::Battlefield);
    }
    if board.caster_zombie {
        let zombie = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zombie")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![crate::types::Subtype::Zombie])
            .build();
        game.create_object_from_definition(&zombie, bob, Zone::Battlefield);
    }
    if board.alice_creature {
        let bear = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    }
    let spell_definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Land Spell")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .unwrap();
    let spell = game.create_object_from_definition(&spell_definition, bob, Zone::Stack);
    let mut targets = Vec::new();
    if target_land {
        targets.push(crate::game_state::Target::Object(alice_lands[0]));
    }
    if board.target_alice {
        targets.push(crate::game_state::Target::Player(alice));
    }
    let entry = crate::game_state::StackEntry::new(spell, bob).with_targets(targets);
    let mut entry = entry;
    entry.chosen_modes = modes;
    game.stack.push(entry);

    let equinox = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Guard Ability")
        .card_types(vec![CardType::Instant])
        .parse_text(guard)
        .unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&equinox).join("\n"), guard);
    let source = game.create_object_from_definition(&equinox, alice, Zone::Stack);
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(spell)]);
    for effect in equinox.spell_effect.as_ref().unwrap() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    !game.stack.iter().any(|entry| entry.object_id == spell)
}

fn equinox_counters(text: &str, board: Board, target_land: bool, modes: Option<Vec<usize>>) -> bool {
    guard_counters(EQUINOX, text, board, target_land, modes)
}

#[test]
fn equinox_counters_spells_that_destroy_your_land_without_a_choice() {
    assert!(equinox_counters("Destroy target land.", BOARD, true, None));
    assert!(equinox_counters("Destroy all lands.", BOARD, false, None));
    assert!(equinox_counters("For each land, destroy that land.", BOARD, false, None));
    let modal = "Choose one —\n• Destroy target artifact.\n• Destroy all lands.";
    assert!(equinox_counters(modal, BOARD, false, Some(vec![1])));
    assert!(!equinox_counters(modal, BOARD, false, Some(vec![0])), "unchosen mode");
    let gated = "If you control a Zombie, destroy target land.";
    assert!(!equinox_counters(gated, BOARD, true, None), "condition false right now");
    // The Zombie is Bob's: "you" is the spell's controller.
    assert!(equinox_counters(gated, Board { caster_zombie: true, ..BOARD }, true, None));
}

#[test]
fn equinox_ignores_destruction_a_choice_could_prevent() {
    assert!(!equinox_counters("Destroy target land unless its controller pays {3}.", BOARD, true, None));
    assert!(!equinox_counters(
        "For each land, destroy that land unless any player pays 1 life.",
        BOARD,
        false,
        None
    ));
}

#[test]
fn equinox_counters_cleansing_when_life_payments_cannot_save_every_land() {
    let cleansing = "For each land, destroy that land unless any player pays 1 life.";
    // Two players at 3 life can pay for six lands; Alice has seven.
    assert!(equinox_counters(cleansing, Board { life: 3, ..BOARD }, false, None));
    assert!(!equinox_counters(cleansing, Board { life: 3, alice_lands: 6, ..BOARD }, false, None));
}

#[test]
fn would_act_predicates_forecast_actions_other_than_destroy() {
    let exile = |text: &str, board: Board, target_land: bool| {
        guard_counters(EXILE_GUARD, text, board, target_land, None)
    };
    assert!(exile("Exile target land.", BOARD, true));
    assert!(!exile("Destroy target land.", BOARD, true), "a different action");
    assert!(!equinox_counters("Exile target land.", BOARD, true, None), "a different action");
    assert!(exile("For each land, exile that land.", BOARD, false));
    assert!(!exile("You may exile target land.", BOARD, true), "a choice");
    let cleansing = "For each land, exile that land unless any player pays 1 life.";
    assert!(!exile(cleansing, BOARD, false));
    assert!(exile(cleansing, Board { life: 3, ..BOARD }, false));
}

#[test]
fn would_act_predicates_forecast_returning_to_hand() {
    let bounce = |text: &str, board: Board, target_land: bool| {
        guard_counters(BOUNCE_GUARD, text, board, target_land, None)
    };
    assert!(bounce("Return target land to its owner's hand.", BOARD, true));
    assert!(bounce("Return all lands to their owners' hands.", BOARD, false));
    assert!(!bounce("Destroy target land.", BOARD, true), "a different action");
    assert!(!bounce("You may return target land to its owner's hand.", BOARD, true), "a choice");
}

#[test]
fn would_act_predicates_forecast_forced_sacrifices() {
    let sacrifice = |text: &str, board: Board| guard_counters(SACRIFICE_GUARD, text, board, false, None);
    let with_creature = Board { alice_creature: true, ..BOARD };
    assert!(sacrifice("Each player sacrifices a land.", BOARD));
    assert!(!sacrifice("Each player sacrifices a creature.", BOARD));
    assert!(!sacrifice("Sacrifice a land.", BOARD), "only the caster sacrifices");
    assert!(sacrifice("Target player sacrifices a land.", Board { target_alice: true, ..BOARD }));
    // Alice chooses which permanents: forced only if she has too few non-lands.
    assert!(sacrifice("Each player sacrifices a permanent.", BOARD));
    assert!(!sacrifice("Each player sacrifices a permanent.", with_creature));
    assert!(sacrifice("Each player sacrifices two permanents.", with_creature));
    assert!(!equinox_counters("Each player sacrifices a land.", BOARD, false, None), "a different action");
}

#[test]
fn would_act_predicates_forecast_who_is_caused_to_act() {
    // "Cause you to sacrifice": Alice must be the one sacrificing, so any
    // land she controls counts and Bob sacrificing his own doesn't.
    let you = |text: &str, board: Board| {
        guard_counters(
            "Counter target spell if it would cause you to sacrifice a land.",
            text,
            board,
            false,
            None,
        )
    };
    assert!(you("Each player sacrifices a land.", BOARD));
    assert!(you("Target player sacrifices a land.", Board { target_alice: true, ..BOARD }));
    assert!(!you("Sacrifice a land.", BOARD), "Bob sacrifices his own land");
    assert!(!you("Each player sacrifices a creature.", BOARD));
    let opponent = |text: &str, board: Board| {
        guard_counters(
            "Counter target spell if it would cause an opponent to sacrifice a land.",
            text,
            board,
            false,
            None,
        )
    };
    assert!(opponent("Sacrifice a land.", BOARD), "Bob is Alice's opponent");
    assert!(
        !opponent("Target player sacrifices a land.", Board { target_alice: true, ..BOARD }),
        "Alice isn't her own opponent"
    );
    // Other actions are performed by the spell's controller (Bob).
    let destroy = |guard: &str| guard_counters(guard, "Destroy target land.", BOARD, true, None);
    assert!(!destroy("Counter target spell if it would cause you to destroy a land."));
    assert!(destroy("Counter target spell if it would cause an opponent to destroy a land."));
}
