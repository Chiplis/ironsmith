//! Source767 uses singular planeswalker-inclusive attacks in static and timed forms.
#[path = "cf8_p10_support/mod.rs"]
mod support;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::effects::{CantEffect, EffectContext, execute_effect};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn attack(game: &mut GameState, attacker: ObjectId, target: AttackTarget) -> bool {
    game.turn.active_player = B;
    game.refresh_continuous_state().unwrap();
    let mut combat = CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut ironsmith::triggers::TriggerQueue::new(),
        &[AttackerDeclaration {
            creature: attacker,
            target,
        }],
    )
    .is_ok()
        && combat.attackers.len() == 1
}
#[test]
fn singular_planeswalker_phrase_preserves_controller_union_in_actual_source_faces() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-late-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "msem-767")
        .unwrap();
    let faces = row["representative_cards"][0]["executable_faces"]
        .as_array()
        .unwrap();
    for face in faces {
        for definition in support::definitions(
            face["name"].as_str().unwrap(),
            face["executable_text"].as_str().unwrap(),
        ) {
            for defender in 0..3 {
                for funded in [false, true] {
                    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
                    game.turn.phase = ironsmith::Phase::FirstMain;
                    game.turn.step = None;
                    let source =
                        game.create_object_from_definition(&definition, A, Zone::Battlefield);
                    if face["name"] == "Blinding Fiddler" {
                        let cants = support::find_all::<CantEffect>(&definition);
                        assert_eq!(cants.len(), 1);
                        assert_eq!(cants[0].duration, ironsmith::effect::Until::YourNextTurn);
                        execute_effect(
                            &mut game,
                            &ironsmith::Effect::new(cants[0].clone()),
                            &mut EffectContext::new_default(source, A),
                        )
                        .unwrap();
                        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                    }
                    let walker_definition =
                        ironsmith_compiler_runtime::compile_to_runtime_definition(
                            "Protected walker",
                            "Type: Planeswalker — Jace\nLoyalty: 3",
                            false,
                        )
                        .unwrap();
                    let walker = game.create_object_from_definition(
                        &walker_definition,
                        A,
                        Zone::Battlefield,
                    );
                    let attacker_definition =
                        ironsmith_compiler_runtime::compile_to_runtime_definition(
                            "Attacker",
                            "Type: Creature — Bear\nPower/Toughness: 2/2",
                            false,
                        )
                        .unwrap();
                    let attacker = game.create_object_from_definition(
                        &attacker_definition,
                        B,
                        Zone::Battlefield,
                    );
                    game.remove_summoning_sickness(attacker);
                    if funded {
                        game.player_mut(B)
                            .unwrap()
                            .mana_pool
                            .add(ironsmith::mana::ManaSymbol::Colorless, 2);
                    }
                    let target = match defender {
                        0 => AttackTarget::Player(A),
                        1 => AttackTarget::Planeswalker(walker),
                        _ => AttackTarget::Player(C),
                    };
                    assert_eq!(attack(&mut game, attacker, target), defender == 2 || funded);
                    assert_eq!(
                        game.player(B).unwrap().mana_pool.total(),
                        if funded && defender == 2 { 2 } else { 0 }
                    );
                }
            }
        }
    }
}
