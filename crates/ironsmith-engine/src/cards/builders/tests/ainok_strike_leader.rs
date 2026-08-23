#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ATTACK_TRIGGER_LINE: &str = "Whenever you attack with this creature and/or your commander, for each opponent, create a 1/1 red Goblin creature token that's tapped and attacking that player.";

fn setup_game() -> (crate::game_state::GameState, ObjectId, ObjectId) {
    let definition = parse_oracle_card_definition("Ainok Strike Leader");
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let ainok = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let commander_definition = CardDefinitionBuilder::new(CardId::new(), "Strike Leader Commander")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let commander =
        game.create_object_from_definition(&commander_definition, alice, Zone::Battlefield);
    game.set_as_commander(commander, alice);
    game.remove_summoning_sickness(ainok);
    game.remove_summoning_sickness(commander);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    (game, ainok, commander)
}

fn goblin_attack_targets(
    game: &crate::game_state::GameState,
) -> Vec<crate::combat_state::AttackTarget> {
    let goblins = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == "Goblin"
                    && object.kind == crate::object::ObjectKind::Token
                    && game.controller_of(object) == PlayerId::from_index(0)
            })
        })
        .collect::<Vec<_>>();
    let combat = game.combat.as_ref().expect("combat should remain active");
    goblins
        .iter()
        .map(|goblin| {
            assert!(game.is_tapped(*goblin), "created Goblins must enter tapped");
            combat
                .attackers
                .iter()
                .find(|attacker| attacker.creature == *goblin)
                .map(|attacker| attacker.target.clone())
                .expect("each created Goblin must enter attacking")
        })
        .collect()
}

fn attack_and_resolve(
    game: &mut crate::game_state::GameState,
    declarations: &[crate::decision::AttackerDeclaration],
) {
    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut trigger_queue,
        declarations,
    )
    .expect("attack declarations should succeed");
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "attacking with either or both named creatures must produce one aggregate trigger"
    );
    crate::game_loop::put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Ainok Strike Leader's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(game)
        .expect("Ainok Strike Leader's trigger should resolve");
}

#[test]
fn ainok_strike_leader_triggers_for_commander_and_distributes_attack_targets() {
    let definition = parse_oracle_card_definition("Ainok Strike Leader");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == ATTACK_TRIGGER_LINE),
        "Ainok must retain both attack subjects and the per-opponent target: {rendered:#?}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let (mut commander_only_game, _ainok, commander) = setup_game();
    attack_and_resolve(
        &mut commander_only_game,
        &[crate::decision::AttackerDeclaration {
            creature: commander,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
    );
    let mut targets = goblin_attack_targets(&commander_only_game);
    targets.sort_by_key(|target| match target {
        crate::combat_state::AttackTarget::Player(player) => player.index(),
        _ => usize::MAX,
    });
    assert_eq!(
        targets,
        vec![
            crate::combat_state::AttackTarget::Player(bob),
            crate::combat_state::AttackTarget::Player(charlie),
        ],
        "one Goblin must attack each opponent rather than copying the commander's target"
    );

    let (mut combined_game, ainok, commander) = setup_game();
    attack_and_resolve(
        &mut combined_game,
        &[
            crate::decision::AttackerDeclaration {
                creature: ainok,
                target: crate::combat_state::AttackTarget::Player(bob),
            },
            crate::decision::AttackerDeclaration {
                creature: commander,
                target: crate::combat_state::AttackTarget::Player(charlie),
            },
        ],
    );
    assert_eq!(
        goblin_attack_targets(&combined_game).len(),
        2,
        "attacking with both must still create only one token per opponent"
    );

    assert_eq!(combined_game.turn.active_player, alice);
}
