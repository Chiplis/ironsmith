//! cf8/p07: Balance's procedure re-applied "the same way" to further object
//! domains. Each step: every player chooses as many objects of the domain as
//! the player with the fewest has, then sacrifices (or, for cards in hand,
//! discards) the rest; steps run in the order written.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Value;
use ironsmith::effects::{ChooseObjectsEffect, EffectContext, execute_effect};
use ironsmith::game_state::Phase;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const FIXTURE: &str = include_str!("../../../fixtures/same_way_balance.json.fixture");
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn least_count_choices(effects: &[ironsmith::effect::Effect]) -> Vec<ChooseObjectsEffect> {
    support::find::<ChooseObjectsEffect>(effects)
        .into_iter()
        .filter(|choice| matches!(choice.count_value.as_ref(), Some(Value::LeastCount(_))))
        .collect()
}

fn assert_steps(effects: &[ironsmith::effect::Effect], expected_zones: &[Zone]) {
    let choices = least_count_choices(effects);
    assert_eq!(choices.len(), expected_zones.len(), "{effects:?}");
    for (choice, zone) in choices.iter().zip(expected_zones) {
        let Some(Value::LeastCount(counted)) = choice.count_value.as_ref() else {
            unreachable!()
        };
        assert_eq!(counted.zone, Some(*zone), "{choice:?}");
    }
    let debug = format!("{effects:?}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    if expected_zones.contains(&Zone::Hand) {
        assert!(debug.contains("Discard"), "{debug}");
    }
}

#[test]
fn balancing_act_discards_the_same_way_after_permanents() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Balancing Act");
    assert_eq!(row["oracle_id"], "4c68282b-776f-463a-9652-f611538c53ee");
    for definition in support::definitions(row) {
        let effects = support::spell_effects(&definition);
        assert_steps(&effects, &[Zone::Battlefield, Zone::Hand]);

        // Gameplay: three permanents and three cards against one and one.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        let land = compile_to_runtime_definition("Witness Plains", "Type: Basic Land — Plains", false)
            .unwrap();
        let card = compile_to_runtime_definition(
            "Witness Card",
            "Mana cost: {1}\nType: Sorcery\nDraw a card.",
            false,
        )
        .unwrap();
        for _ in 0..3 {
            game.create_object_from_definition(&land, A, Zone::Battlefield);
            game.create_object_from_definition(&card, A, Zone::Hand);
        }
        game.create_object_from_definition(&land, B, Zone::Battlefield);
        game.create_object_from_definition(&card, B, Zone::Hand);
        let spell = game.create_object_from_definition(&definition, A, Zone::Exile);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(spell, A, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let controlled = |game: &GameState, player| {
            game.battlefield
                .iter()
                .filter(|id| game.current_controller(**id) == Some(player))
                .count()
        };
        assert_eq!(controlled(&game, A), 1);
        assert_eq!(controlled(&game, B), 1);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
    }
}

#[test]
fn magus_of_the_balance_repeats_for_hands_then_creatures() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Magus of the Balance");
    assert_eq!(row["oracle_id"], "ed450f48-e69d-48d7-82cc-79cf89b2a1fb");
    for definition in support::definitions(row) {
        let activated = support::activated(&definition);
        assert_eq!(activated.len(), 1);
        let effects = support::activated_effects(activated[0]);
        assert_steps(&effects, &[Zone::Battlefield, Zone::Hand, Zone::Battlefield]);
    }
}

#[test]
fn restore_balance_repeats_for_creatures_then_hands() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Restore Balance");
    assert_eq!(row["oracle_id"], "ca79d130-6387-47b0-bde8-f45dd93f9221");
    for definition in support::definitions(row) {
        let effects = support::spell_effects(&definition);
        assert_steps(&effects, &[Zone::Battlefield, Zone::Battlefield, Zone::Hand]);
    }
}

#[test]
fn balance_counts_the_fewest_rather_than_choosing_one() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Balance");
    assert_eq!(row["oracle_id"], "17fa98cd-ed8f-483f-9525-7e989a82ebb2");
    for definition in support::definitions(row) {
        let effects = support::spell_effects(&definition);
        assert_steps(&effects, &[Zone::Battlefield, Zone::Hand, Zone::Battlefield]);
    }
}
