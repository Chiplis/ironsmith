#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::mana::ManaSymbol;
use crate::special_actions::{SpecialAction, TurnFaceUpMethod};

const BUBBLE_SMUGGLER_TEXT: &str =
    "Disguise {5}{U}\nAs this creature is turned face up, put four +1/+1 counters on it.";
const HOODED_HYDRA_TEXT: &str = "This creature enters with X +1/+1 counters on it.\nWhen this creature dies, create a 1/1 green Snake creature token for each +1/+1 counter on it.\nMorph {3}{G}{G}\nAs this creature is turned face up, put five +1/+1 counters on it.";

fn assert_face_up_only_program(definition: &CardDefinition) {
    assert!(
        !format!("{:#?}", definition.spell_effect).contains("PutCountersEffect"),
        "the face-up instruction must not be lowered as an unconditional spell effect"
    );
    let (program, also_turns_face_up, turns_face_up_only) = definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
                program,
                also_turns_face_up,
                turns_face_up_only,
                ..
            } = &static_ability.compiled_model()?.payload
            else {
                return None;
            };
            Some((program, *also_turns_face_up, *turns_face_up_only))
        })
        .expect("named card should retain a typed face-up-only effect program");

    assert!(also_turns_face_up);
    assert!(turns_face_up_only);
    assert!(
        format!("{program:#?}").contains("PutCountersEffect"),
        "face-up-only program should retain the counter effect: {program:#?}"
    );
}

fn enter_normally(name: &str) -> (crate::GameState, PlayerId, ObjectId) {
    let definition = parse_oracle_card_definition(name);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut decisions = SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(card, Zone::Battlefield, &mut decisions)
        .unwrap_or_else(|| panic!("{name} should enter the battlefield normally"));
    (game, alice, entered.new_id)
}

fn turn_face_up(
    game: &mut crate::GameState,
    player: PlayerId,
    permanent: ObjectId,
    method: TurnFaceUpMethod,
) {
    assert!(game.set_face_down(permanent));
    game.turn.priority_player = Some(player);
    let mut decisions = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        SpecialAction::TurnFaceUp {
            permanent_id: permanent,
            method,
        },
        game,
        player,
        &mut decisions,
    )
    .expect("turn-face-up special action should succeed");
}

#[test]
fn named_cards_preserve_exact_face_up_only_text_and_structure() {
    let bubble = parse_oracle_card_definition("Bubble Smuggler");
    assert_eq!(
        canonical_compiled_lines(&bubble).join("\n"),
        BUBBLE_SMUGGLER_TEXT
    );
    assert_face_up_only_program(&bubble);

    let hydra = parse_oracle_card_definition("Hooded Hydra");
    assert_eq!(
        canonical_compiled_lines(&hydra).join("\n"),
        HOODED_HYDRA_TEXT
    );
    assert_face_up_only_program(&hydra);
}

#[test]
fn face_up_only_counter_programs_do_not_run_on_ordinary_entry() {
    for name in ["Bubble Smuggler", "Hooded Hydra"] {
        let (game, _alice, permanent) = enter_normally(name);
        assert_eq!(
            game.counter_count(permanent, CounterType::PlusOnePlusOne),
            0,
            "{name}'s face-up-only counter program must not run when it enters face up"
        );
    }
}

#[test]
fn bubble_smuggler_gets_four_counters_immediately_when_turned_face_up() {
    let (mut game, alice, permanent) = enter_normally("Bubble Smuggler");
    {
        let player = game.player_mut(alice).expect("Alice should exist");
        player.mana_pool.add(ManaSymbol::Colorless, 5);
        player.mana_pool.add(ManaSymbol::Blue, 1);
    }

    turn_face_up(
        &mut game,
        alice,
        permanent,
        TurnFaceUpMethod::DisguiseAbility,
    );

    assert_eq!(
        game.counter_count(permanent, CounterType::PlusOnePlusOne),
        4
    );
    assert!(
        game.stack_is_empty(),
        "an `As ... is turned face up` effect is immediate, not a queued trigger"
    );
}

#[test]
fn hooded_hydra_gets_five_counters_immediately_when_turned_face_up() {
    let (mut game, alice, permanent) = enter_normally("Hooded Hydra");
    {
        let player = game.player_mut(alice).expect("Alice should exist");
        player.mana_pool.add(ManaSymbol::Colorless, 3);
        player.mana_pool.add(ManaSymbol::Green, 2);
    }

    turn_face_up(
        &mut game,
        alice,
        permanent,
        TurnFaceUpMethod::TurnFaceUpAbility,
    );

    assert_eq!(
        game.counter_count(permanent, CounterType::PlusOnePlusOne),
        5
    );
    assert!(
        game.stack_is_empty(),
        "an `As ... is turned face up` effect is immediate, not a queued trigger"
    );
}
