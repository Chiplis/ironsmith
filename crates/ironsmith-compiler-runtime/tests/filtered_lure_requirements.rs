//! "All creatures with flying able to block this creature do so." (Talruum
//! Piper) / "All Walls able to block this creature do so." (Marble Priest):
//! a CR 509.1c blocking requirement limited to a filtered set of blockers.
//! Source-authored, deliberately unrun.
use ironsmith::combat_state::{AttackTarget, CombatState, declare_blockers};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::game_loop::apply_attacker_declarations;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const TALRUUM_PIPER: &str = "Mana cost: {4}{R}\nType: Creature — Minotaur\nPower/Toughness: 3/3\nAll creatures with flying able to block this creature do so.";
const MARBLE_PRIEST: &str = "Mana cost: {5}\nType: Artifact Creature — Cleric\nPower/Toughness: 3/3\nAll Walls able to block this creature do so.\nPrevent all combat damage that would be dealt to this creature by Walls.";

fn creature(game: &mut GameState, controller: PlayerId, type_line: &str, text: &str) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Blocker",
        format!("Type: {type_line}\nPower/Toughness: 1/4\n{text}"),
        false,
    )
    .unwrap();
    let id = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn attacking(game: &mut GameState, attacker: ObjectId) -> CombatState {
    game.turn.active_player = A;
    game.refresh_continuous_state().unwrap();
    let mut combat = CombatState::default();
    apply_attacker_declarations(
        game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(B),
        }],
    )
    .unwrap();
    combat
}

#[test]
fn filtered_lure_bodies_compile_to_one_block_requirement() {
    for (name, text, fragment) in [
        ("Talruum Piper", TALRUUM_PIPER, "Flying"),
        ("Marble Priest", MARBLE_PRIEST, "Wall"),
    ] {
        for definition in compile::compile_both(name, text) {
            let restrictions = compile::statics(&definition, StaticAbilityId::RuleRestriction);
            let lure = restrictions
                .iter()
                .map(|ability| format!("{ability:?}"))
                .find(|debug| debug.contains("MustBlockSpecificAttacker"))
                .unwrap_or_else(|| panic!("{name}: lure requirement"));
            assert!(lure.contains(fragment), "{name}: {lure}");
        }
    }
}

#[test]
fn only_matching_blockers_are_required_to_block() {
    for (name, text, matching_type, matching_text) in [
        ("Talruum Piper", TALRUUM_PIPER, "Creature — Bird", "Flying"),
        ("Marble Priest", MARBLE_PRIEST, "Creature — Wall", "Defender"),
    ] {
        for definition in compile::compile_both(name, text) {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let attacker = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(attacker);
            let matching = creature(&mut game, B, matching_type, matching_text);
            let other = creature(&mut game, B, "Creature — Human", "");
            let combat = attacking(&mut game, attacker);

            let mut nobody = combat.clone();
            assert!(
                declare_blockers(&game, &mut nobody, vec![]).is_err(),
                "{name}: the matching creature must block"
            );
            let mut only_other = combat.clone();
            assert!(declare_blockers(&game, &mut only_other, vec![(other, attacker)]).is_err());
            let mut required = combat.clone();
            declare_blockers(&game, &mut required, vec![(matching, attacker)]).unwrap();
            game.tap(matching);
            let mut unable = combat.clone();
            declare_blockers(&game, &mut unable, vec![]).unwrap();
        }
    }
}
