//! UNVALIDATED implementation-first coverage (cf8 p09): "As this artifact
//! enters, choose two colors." records a set of chosen colors on the
//! permanent (single-color readers are unchanged). Seal of the Guildpact's
//! reduction counts the spell's colors among them; Tablet of the Guilds'
//! trigger checks "at least one of the chosen colors" and counts them.
use ironsmith::ability::AbilityKind;
use ironsmith::color::{Color, ColorSet};
use ironsmith::effect::Value;
use ironsmith::target::ChooseSpec;
use ironsmith::GameState;

#[path = "p09_common/mod.rs"]
mod common;

fn rows() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/multi_color_designations.json.fixture"))
}

fn static_models(definition: &ironsmith::cards::CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability
                .canonical_model()
                .map(|model| format!("{model:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn seal_of_the_guildpact_chooses_two_colors_and_reduces_per_chosen_color() {
    let rows = rows();
    for definition in common::definitions(common::row(&rows, "Seal of the Guildpact")) {
        let models = static_models(&definition);
        assert!(
            models
                .iter()
                .any(|model| model.contains("ChooseColorAsEnters") && model.contains("count: 2")),
            "{models:#?}"
        );
        assert!(
            models
                .iter()
                .any(|model| model.contains("against_source_chosen_colors: true")),
            "{models:#?}"
        );
    }
}

#[test]
fn tablet_of_the_guilds_counts_the_chosen_colors_of_the_spell() {
    let rows = rows();
    for definition in common::definitions(common::row(&rows, "Tablet of the Guilds")) {
        let models = static_models(&definition);
        assert!(
            models
                .iter()
                .any(|model| model.contains("ChooseColorAsEnters") && model.contains("count: 2")),
            "{models:#?}"
        );
        let debug = format!("{:?}", common::all_effects(&definition));
        assert!(debug.contains("ChosenColorsOf"), "{debug}");
        let triggers = format!("{:?}", definition.abilities);
        assert!(triggers.contains("chosen_color: true"), "{triggers}");
    }
}

#[test]
fn chosen_color_sets_union_with_single_choices() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let seal = game.new_object_id();
    assert_eq!(game.chosen_colors(seal), None);
    let two = ColorSet::from_color(Color::White).with(Color::Blue);
    game.set_chosen_colors(seal, two);
    assert_eq!(game.chosen_colors(seal), Some(two));
    // The single-color reader is untouched by a multi-color choice.
    assert_eq!(game.chosen_color(seal), None);
    let single = game.new_object_id();
    game.set_chosen_color(single, Color::Red);
    assert_eq!(game.chosen_colors(single), Some(ColorSet::from_color(Color::Red)));
    let _ = Value::ChosenColorsOf(Box::new(ChooseSpec::Source));
}
