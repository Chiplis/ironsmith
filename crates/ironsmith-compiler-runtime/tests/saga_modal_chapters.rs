//! cf8/p07: "I, II — Choose one —" with bullet modes is one modal chapter
//! ability triggered by those chapters (CR 714.2b, CR 700.2b).
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::ChooseModeEffect;

const FIXTURE: &str = include_str!("../../../fixtures/saga_modal_chapters.json.fixture");

#[test]
fn modal_saga_chapters_are_chapter_triggered_mode_choices() {
    let rows = support::rows(FIXTURE);
    for (name, oracle_id, chapters, modes, random) in [
        (
            "Life of Toshiro Umezawa // Memory of Toshiro",
            "0a708566-2994-47b3-8d89-0547cc115e7c",
            vec![1u32, 2],
            3usize,
            false,
        ),
        (
            "Summon: Magus Sisters",
            "06f11ff8-cc99-411d-b00b-464ba4b7b87d",
            vec![1u32, 2, 3],
            3usize,
            true,
        ),
    ] {
        let row = support::row(&rows, name);
        assert_eq!(row["oracle_id"], oracle_id);
        for definition in support::definitions(row) {
            let modal: Vec<_> = support::triggered(&definition)
                .into_iter()
                .filter(|triggered| triggered.trigger.saga_chapters() == Some(chapters.as_slice()))
                .collect();
            assert_eq!(modal.len(), 1, "{name}: one modal chapter ability");
            let all = support::triggered_effects(modal[0]);
            let choices = support::find::<ChooseModeEffect>(&all);
            assert_eq!(choices.len(), 1, "{name}");
            assert_eq!(choices[0].modes.len(), modes, "{name}");
            assert_eq!(choices[0].choose_count, Value::Fixed(1), "{name}");
            assert_eq!(choices[0].random, random, "{name}");
            assert!(modal[0].intervening_if.is_none(), "{name}");
        }
    }
}
