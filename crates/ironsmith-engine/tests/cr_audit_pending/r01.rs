//! CR audit (reclassified rows, package r01): rules first classified as
//! non-engine because no printed card or app variant reaches them. They are
//! tracked as missing capabilities, so these assertions fail until supported.

#![allow(unused_imports, dead_code)]

use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::AttackersContext;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    AbilityKind, CardId, CardType, GameState, PlayerId, PowerToughness, StackEntry, Zone,
};

fn r01_compile(name: &str, text: &str) -> Result<CardDefinition, String> {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .map_err(|e| format!("{e:?}"))
}

fn r01_library(game: &mut GameState, player: PlayerId, count: usize) {
    let filler = CardDefinitionBuilder::new(CardId::new(), "R01 Filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..count {
        game.create_object_from_definition(&filler, player, Zone::Library);
    }
}

fn r01_resolve_with_triggers(game: &mut GameState, effect: ironsmith::Effect, controller: PlayerId) {
    let source = {
        let def = CardDefinitionBuilder::new(CardId::new(), "R01 Effect Source")
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_definition(&def, controller, Zone::Battlefield)
    };
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    game.stack.push(StackEntry::ability(source, controller, vec![effect]));
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(game, &mut queue).unwrap();
    for _ in 0..10 {
        if queue.entries.is_empty() && game.stack.is_empty() {
            break;
        }
        ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
        }
        ironsmith::game_loop::check_and_apply_sbas(game, &mut queue).unwrap();
    }
}

/// CR 113.6c: an ability that names the zones it doesn't function in works in
/// every other zone, including outside the game.
#[test]
fn cr_113_6c_ability_stating_excluded_zones_functions_everywhere_else() {
    let compiled = r01_compile(
        "R01 Zone Exclusion Witness",
        "Mana cost: {2}\nType: Creature — Spirit\nPower/Toughness: 1/1\nAt the beginning of your upkeep, you gain 1 life. This ability doesn't function on the battlefield.",
    );
    let zones = compiled.as_ref().ok().and_then(|definition| {
        (!ironsmith::cards::generated_definition_has_unimplemented_content(definition))
            .then(|| {
                definition
                    .abilities
                    .iter()
                    .find(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
                    .map(|ability| ability.functional_zones.clone())
            })
            .flatten()
    });
    let everywhere_but_battlefield = [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Command,
        Zone::OutsideGame,
    ];
    assert!(
        zones.as_ref().is_some_and(|zones| !zones.contains(&Zone::Battlefield)
            && everywhere_but_battlefield.iter().all(|zone| zones.contains(zone))),
        "CR 113.6c: the upkeep ability functions in every zone except the battlefield, even outside the game; compiled={:?} zones={zones:?}",
        compiled.as_ref().map(|definition| definition.card.name.clone())
    );
}

/// CR 603.1b: an instruction asking whether "all" trigger conditions happened
/// this turn counts events that happened before the ability could trigger.
#[test]
fn cr_603_1b_all_conditions_count_events_from_before_the_ability_existed() {
    let alice = PlayerId::from_index(0);
    let compiled = r01_compile(
        "R01 All Conditions Witness",
        "Mana cost: {2}\nType: Enchantment\nWhenever you gain life or lose life, if you've gained life and lost life this turn, draw a card.",
    );
    let definition = match compiled {
        Ok(definition)
            if !ironsmith::cards::generated_definition_has_unimplemented_content(&definition) =>
        {
            Some(definition)
        }
        _ => None,
    };
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    r01_library(&mut game, alice, 5);
    // Alice gains life before the enchantment is on the battlefield.
    r01_resolve_with_triggers(&mut game, ironsmith::Effect::gain_life(1), alice);
    if let Some(definition) = &definition {
        game.create_object_from_definition(definition, alice, Zone::Battlefield);
    }
    let hand_before = game.player(alice).unwrap().hand.len();
    r01_resolve_with_triggers(&mut game, ironsmith::Effect::lose_life(1), alice);
    let drawn = game.player(alice).unwrap().hand.len() - hand_before;
    assert_eq!(
        (definition.is_some(), drawn),
        (true, 1),
        "CR 603.1b: the earlier life gain counts toward \"all\" conditions even though the ability had not triggered on it"
    );
}

struct R01AttackTargets {
    seen: Vec<Vec<AttackTarget>>,
}

impl DecisionMaker for R01AttackTargets {
    fn decide_attackers(
        &mut self,
        _game: &GameState,
        ctx: &AttackersContext,
    ) -> Vec<ironsmith::decisions::spec::AttackerDeclaration> {
        self.seen
            .extend(ctx.attacker_options.iter().map(|option| option.valid_targets.clone()));
        Vec::new()
    }
}

/// CR 703.4h / 506.2a / 802.1: attacking multiple players is an option. In a
/// multiplayer game without it, the active player chooses one opponent at the
/// beginning of combat and only that player (and their permanents) can be attacked.
#[test]
fn cr_703_4h_without_attack_multiple_players_one_chosen_opponent_is_the_defending_player() {
    let players: Vec<PlayerId> = (0..3).map(PlayerId::from_index).collect();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    // CR 802.1: attacking multiple players is an option; this game does not use it.
    game.set_attack_multiple_players(false);
    for &player in &players {
        r01_library(&mut game, player, 10);
    }
    let bear = CardDefinitionBuilder::new(CardId::new(), "R01 Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker = game.create_object_from_definition(&bear, players[0], Zone::Battlefield);
    game.remove_summoning_sickness(attacker);
    game.turn.active_player = players[0];
    game.turn.turn_number = 2;
    let mut dm = R01AttackTargets { seen: Vec::new() };
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let _ = ironsmith::game_loop::execute_turn_with(&mut game, &mut combat, &mut queue, &mut dm);
    let defending: Vec<Vec<PlayerId>> = dm
        .seen
        .iter()
        .map(|targets| {
            let mut players: Vec<PlayerId> = targets
                .iter()
                .filter_map(|target| match target {
                    AttackTarget::Player(player) => Some(*player),
                    _ => None,
                })
                .collect();
            players.sort();
            players.dedup();
            players
        })
        .collect();
    assert!(
        !defending.is_empty() && defending.iter().all(|players| players.len() == 1),
        "CR 703.4h: a multiplayer game without the attack-multiple-players option has exactly one defending player; attackable players offered: {defending:?}"
    );
}
