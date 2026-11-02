//! Announcement feasibility and tagged-consumer preflight share X cardinality.
use ironsmith::{Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{ChooseObjectsEffect, EffectExecutor};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
const A: PlayerId = PlayerId(0);
#[test]
fn scaled_count_value_admits_zero_before_announcement_and_enforces_full_bound_count() {
    for shell in [ironsmith::ChoiceCount::exactly(0), ironsmith::ChoiceCount::dynamic_x()] {
        for expression in [ironsmith::effect::Value::XTimes(2), ironsmith::effect::Value::Scaled(Box::new(ironsmith::effect::Value::X), 2)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Material", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
            let source = game.create_object_from_definition(&body, A, Zone::Battlefield);
            for index in 0..4 { game.create_object_from_definition(&body, A, if index < 2 { Zone::Hand } else { Zone::Graveyard }); }
            let mut filter = ObjectFilter::default();
            filter.any_of = vec![ObjectFilter::default().in_zone(Zone::Hand).owned_by(PlayerFilter::You), ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(PlayerFilter::You)];
            let mut choose = ChooseObjectsEffect::new(filter, shell, PlayerFilter::You, "payment_objects")
                .with_count_value_opt(Some(expression));
            choose.additional_zones = vec![Zone::Hand, Zone::Graveyard];
            assert!(choose.references_cost_x(), "count_value alone must request announcement");
            assert_eq!(choose.max_cost_x(&game, source, A), Some(2));
            let program = Effect::new(ironsmith::effects::SequenceEffect::new(vec![Effect::new(choose), Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::Tagged("payment_objects".into())))]));
            let cost = ironsmith::costs::Cost::try_effect(program).unwrap();
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let context = ironsmith::costs::CostContext::new(source, A, &mut dm);
            assert!(cost.can_pay(&game, &context).is_ok(), "unannounced 2X has feasible X=0, including tagged exile consumer");
            for x in [0, 1, 2, 3] {
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                let context = ironsmith::costs::CostContext::new(source, A, &mut dm).with_x(x);
                assert_eq!(cost.can_pay(&game, &context).is_ok(), x <= 2, "announced X={x} requires exactly {} materials", 2*x);
            }
            assert_eq!(game.player(A).unwrap().hand.len(), 2, "preflight must not pay or reveal material");
            assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
        }
    }
}
