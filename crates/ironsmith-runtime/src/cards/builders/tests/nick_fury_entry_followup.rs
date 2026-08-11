#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const TRIGGER_LINE: &str = "Whenever a creature you control attacks alone, draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.";

fn candidate() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Field Agent")
        .card_types(vec![CardType::Creature])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Generic(3),
        ]))
        .power_toughness(PowerToughness::fixed(3, 3))
        .build()
}

fn filler() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Draw Filler")
        .card_types(vec![CardType::Land])
        .build()
}

fn attack_trigger_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:#?}", triggered.trigger).contains("AttacksAlone") =>
            {
                Some(&triggered.effects)
            }
            _ => None,
        })
        .expect("Nick Fury must retain the attack-alone trigger")
}

fn setup() -> (
    crate::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    crate::ids::StableId,
) {
    let mut definition = parse_oracle_card_definition("Nick Fury, Spymaster");
    definition.card.power_toughness = Some(PowerToughness::fixed(4, 4));
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let hand_card = game.create_object_from_definition(&candidate(), alice, Zone::Hand);
    let stable_id = game
        .object(hand_card)
        .expect("candidate should exist")
        .stable_id;
    game.create_object_from_definition(&filler(), alice, Zone::Library);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: source,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
        ..Default::default()
    });
    (game, alice, bob, source, stable_id)
}

fn current_id(game: &crate::GameState, stable_id: crate::ids::StableId) -> ObjectId {
    game.find_object_by_stable_id(stable_id)
        .expect("stable object should remain in the game")
}

fn resolve_attack_trigger(
    game: &mut crate::GameState,
    source: ObjectId,
    defender: PlayerId,
    decisions: &mut dyn crate::decision::DecisionMaker,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::with_total_attackers(
            source,
            crate::triggers::event::AttackEventTarget::Player(defender),
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_triggers(game, &event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "Nick Fury should trigger exactly once");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("Nick Fury trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(game, decisions)
        .expect("Nick Fury trigger should resolve");
}

#[test]
fn nick_fury_keeps_exact_linked_entry_followup_text_and_structure() {
    let definition = parse_oracle_card_definition("Nick Fury, Spymaster");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["First strike".to_string(), TRIGGER_LINE.to_string()]
    );

    let debug = format!("{:#?}", attack_trigger_program(&definition));
    assert!(debug.contains("enters_tapped: true"), "{debug}");
    assert!(debug.contains("enters_attacking: true"), "{debug}");
    assert!(debug.contains("ApplyContinuousEffect"), "{debug}");
    assert!(debug.contains("Indestructible"), "{debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
}

#[test]
fn declining_the_optional_deployment_leaves_the_candidate_in_hand() {
    let (mut game, _alice, bob, source, stable_id) = setup();
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    resolve_attack_trigger(&mut game, source, bob, &mut decisions);

    assert_eq!(
        game.object(current_id(&game, stable_id))
            .map(|object| object.zone),
        Some(Zone::Hand),
        "the entry modifiers must stay inside the declined may branch"
    );
}

#[test]
fn accepting_deploys_the_exact_card_tapped_attacking_and_temporarily_indestructible() {
    let (mut game, _alice, bob, source, stable_id) = setup();
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    resolve_attack_trigger(&mut game, source, bob, &mut decisions);

    let deployed = current_id(&game, stable_id);
    assert_eq!(
        game.object(deployed).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert!(game.is_tapped(deployed));
    assert!(
        game.combat.as_ref().is_some_and(|combat| {
            combat.attackers.iter().any(|attacker| {
                attacker.creature == deployed
                    && attacker.target == crate::combat_state::AttackTarget::Player(bob)
            })
        }),
        "the moved card must join the current combat attacking the defending player"
    );
    assert!(game.object_has_static_ability_id(deployed, StaticAbilityId::Indestructible));

    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert!(
        !game.object_has_static_ability_id(deployed, StaticAbilityId::Indestructible),
        "the temporary grant must expire during cleanup"
    );
}
