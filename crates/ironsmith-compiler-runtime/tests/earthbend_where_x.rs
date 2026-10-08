//! cf8/p07: `earthbend X, where X is ...` carries a dynamic counter amount
//! that is computed once as the instruction resolves (CR 701.66, CR 107.3a).
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::{EarthbendEffect, EffectContext as ExecutionContext, EffectExecutor};
use ironsmith::target::ChooseSpec;
use ironsmith::{CounterType, GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const FIXTURE: &str = include_str!("../../../fixtures/earthbend_where_x.json.fixture");
const A: PlayerId = PlayerId(0);

fn earthbends(definition: &ironsmith::cards::CardDefinition) -> Vec<EarthbendEffect> {
    let mut all = Vec::new();
    if definition.spell_effect.is_some() {
        all.extend(support::spell_effects(definition));
    }
    for triggered in support::triggered(definition) {
        all.extend(support::triggered_effects(triggered));
    }
    support::find::<EarthbendEffect>(&all)
}

#[test]
fn every_where_x_earthbend_keeps_its_dynamic_amount_on_both_routes() {
    let rows = support::rows(FIXTURE);
    assert_eq!(rows.len(), 5);
    for (name, oracle_id) in [
        ("Beifong's Bounty Hunters", "8561b088-56b3-4732-82e9-8da1e3affd55"),
        ("Bumi's Feast Lecture", "41f53bb5-8a62-4e6b-b9c3-5aa530a036bc"),
        ("The Boulder, Ready to Rumble", "d33ee141-a880-4368-a274-a6ae2f2c624b"),
        ("The Legend of Kyoshi // Avatar Kyoshi", "e41e8754-028c-4399-b530-de9e134d04b0"),
        ("Toph, Earthbending Master", "ad5d3e7e-bf8c-4fa3-a81e-1523aec2585f"),
    ] {
        let row = support::row(&rows, name);
        assert_eq!(row["oracle_id"], oracle_id);
        for definition in support::definitions(row) {
            let earthbends = earthbends(&definition);
            assert_eq!(earthbends.len(), 1, "{name}");
            let earthbend = &earthbends[0];
            assert!(!earthbend.awaken, "{name}");
            assert!(earthbend.target.is_target(), "{name}: earthbend targets a land you control");
            assert!(
                !matches!(earthbend.counters.unhinted(), Value::Fixed(_) | Value::X),
                "{name}: the where-X binding replaced X, got {:?}",
                earthbend.counters
            );
        }
    }
}

fn plain(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}

#[test]
fn bumi_earthbend_counts_foods_when_it_resolves() {
    let rows = support::rows(FIXTURE);
    for definition in support::definitions(support::row(&rows, "Bumi's Feast Lecture")) {
        let earthbend = earthbends(&definition).remove(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let land = game.create_object_from_definition(&plain("Plain Land", "Type: Land"), A, Zone::Battlefield);
        let food = plain("Food", "Type: Artifact — Food\n{2}, {T}, Sacrifice this artifact: You gain 3 life.");
        for _ in 0..2 {
            game.create_object_from_definition(&food, A, Zone::Battlefield);
        }
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        let resolved = EarthbendEffect { target: ChooseSpec::SpecificObject(land), ..earthbend };
        resolved
            .execute(&mut game, &mut ExecutionContext::new_default(source, A))
            .unwrap();
        // X is twice the number of Foods you control: 2 Foods -> 4 counters.
        assert_eq!(
            game.object(land).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied(),
            Some(4)
        );
    }
}
