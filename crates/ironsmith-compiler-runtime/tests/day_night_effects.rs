//! UNVALIDATED implementation-first coverage: "It becomes night/day" sets the
//! game's day/night designation (CR 731.2-731.3), including the toggle
//! "If it's night, it becomes day. Otherwise, it becomes night."
use ironsmith::effect::Effect;
use ironsmith::effects::{DayNightDesignation, EffectContext, SetDayNightEffect, execute_effect};
use ironsmith::{GameState, PlayerId};

#[path = "p09_common/mod.rs"]
mod common;

fn designations(definition: &ironsmith::cards::CardDefinition) -> Vec<DayNightDesignation> {
    common::all_effects(definition).iter()
        .filter_map(|effect| effect.downcast_ref::<SetDayNightEffect>().map(|set| set.designation))
        .collect()
}

#[test]
fn day_night_cards_compile_to_designation_effects() {
    let rows = common::rows(include_str!("../../../fixtures/day_night_effects.json.fixture"));
    assert_eq!(rows.len(), 3);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let found = designations(&definition);
            match name {
                "Into the Night" | "Unnatural Moonrise" => {
                    assert_eq!(found, vec![DayNightDesignation::Night], "{name}");
                }
                "The Celestus" => {
                    assert!(found.contains(&DayNightDesignation::Day), "{name}");
                    assert!(found.contains(&DayNightDesignation::Night), "{name}");
                    let debug = format!("{:?}", definition.abilities);
                    assert!(debug.contains("ItIsNight"), "toggle is gated on night: {debug}");
                }
                other => panic!("unexpected cohort member {other}"),
            }
        }
    }
}

#[test]
fn setting_day_night_changes_the_designation_once() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId(0);
    let source = game.new_object_id();
    let night = Effect::new(SetDayNightEffect::new(DayNightDesignation::Night));
    let mut ctx = EffectContext::new_default(source, alice);
    execute_effect(&mut game, &night, &mut ctx).unwrap();
    assert!(game.has_day_night());
    assert!(!game.is_daytime());
    let day = Effect::new(SetDayNightEffect::new(DayNightDesignation::Day));
    let mut ctx = EffectContext::new_default(source, alice);
    execute_effect(&mut game, &day, &mut ctx).unwrap();
    assert!(game.is_daytime());
}
