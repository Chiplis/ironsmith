//! "As this <permanent> is turned face up, <instruction>." (CR 702.37e /
//! CR 708.8 face-up replacement programs): the statement body is bound to the
//! turn-face-up event only. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const A: PlayerId = PlayerId::from_index(0);

const BUBBLE_SMUGGLER: &str = "Mana cost: {1}{U}\nType: Creature — Octopus Fish\nPower/Toughness: 2/1\nDisguise {5}{U} (You may cast this card face down for {3} as a 2/2 creature with ward {2}. Turn it face up any time for its disguise cost.)\nAs this creature is turned face up, put four +1/+1 counters on it.";
const HOODED_HYDRA: &str = "Mana cost: {X}{G}{G}\nType: Creature — Snake Hydra\nPower/Toughness: 0/0\nThis creature enters with X +1/+1 counters on it.\nWhen this creature dies, create a 1/1 green Snake creature token for each +1/+1 counter on it.\nMorph {3}{G}{G}\nAs this creature is turned face up, put five +1/+1 counters on it.";
const GIFT_OF_DOOM: &str = "Mana cost: {4}{B}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has deathtouch and indestructible.\nMorph—Sacrifice another creature. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)\nAs this Aura is turned face up, you may attach it to a creature.";

fn face_up_only_program(definition: &CardDefinition) -> String {
    definition
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
            (*also_turns_face_up && *turns_face_up_only).then(|| format!("{program:?}"))
        })
        .expect("the face-up instruction must lower to a face-up-only program")
}

#[test]
fn face_up_programs_compile_on_both_routes() {
    for (name, text, effect) in [
        ("Bubble Smuggler", BUBBLE_SMUGGLER, "PutCountersEffect"),
        ("Hooded Hydra", HOODED_HYDRA, "PutCountersEffect"),
        ("Gift of Doom", GIFT_OF_DOOM, "AttachToEffect"),
    ] {
        for definition in compile::compile_both(name, text) {
            let program = face_up_only_program(&definition);
            assert!(program.contains(effect), "{name}: {program}");
            assert!(
                !format!("{:?}", definition.spell_effect).contains(effect),
                "{name}: the face-up instruction is not a spell effect"
            );
        }
    }
}

fn enter_then_turn_face_down(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(
            hand,
            Zone::Battlefield,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
    let id = receipt.original.into_result().unwrap().new_id;
    assert!(game.set_face_down(id));
    id
}

#[test]
fn counters_arrive_only_when_turned_face_up() {
    for (name, text, method, mana, counters) in [
        (
            "Bubble Smuggler",
            BUBBLE_SMUGGLER,
            TurnFaceUpMethod::DisguiseAbility,
            [(ManaSymbol::Colorless, 5), (ManaSymbol::Blue, 1)],
            4,
        ),
        (
            "Hooded Hydra",
            HOODED_HYDRA,
            TurnFaceUpMethod::TurnFaceUpAbility,
            [(ManaSymbol::Colorless, 3), (ManaSymbol::Green, 2)],
            5,
        ),
    ] {
        for definition in compile::compile_both(name, text) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let permanent = enter_then_turn_face_down(&mut game, &definition);
            assert_eq!(game.counter_count(permanent, CounterType::PlusOnePlusOne), 0, "{name}");
            for (symbol, amount) in mana {
                game.player_mut(A).unwrap().mana_pool.add(symbol, amount);
            }
            game.turn.priority_player = Some(A);
            ironsmith::special_actions::perform(
                SpecialAction::TurnFaceUp {
                    permanent_id: permanent,
                    method,
                },
                &mut game,
                A,
                &mut SelectFirstDecisionMaker,
            )
            .expect("turning face up must succeed");
            assert_eq!(
                game.counter_count(permanent, CounterType::PlusOnePlusOne),
                counters,
                "{name}"
            );
            assert!(game.stack_is_empty(), "{name}: a replacement, not a trigger");
        }
    }
}
