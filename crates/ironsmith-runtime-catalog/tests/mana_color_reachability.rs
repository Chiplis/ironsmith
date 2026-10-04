use ironsmith::{GameState, PlayerId, Zone};
use ironsmith::mana_payment::{ManaPaymentRequest, plan_first_mana_payment, last_mana_payment_perf};

// Exact baked programs from the natural Pioneer menu failure. This is a
// program/board regression; the unchanged full gameplay prefix remains the
// production acceptance witness.
#[test]
fn unrelated_channel_reduction_preserves_missing_color_proof() {
    let load = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/ui/public/cards").join(format!("{name}.json"));
        let data: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let artifact = serde_json::from_value(data["artifacts"][0].clone()).unwrap();
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    for (name, owner) in [
        ("boseiju-who-endures", alice), ("botanical-sanctum", alice),
        ("botanical-sanctum", alice), ("nykthos-shrine-to-nyx", alice),
        ("badgermole-cub", alice), ("blood-crypt", bob), ("blightstep-pathway", bob),
    ] {
        game.create_object_from_definition(&load(name), owner, Zone::Battlefield);
    }
    let source = game.create_object_from_definition(&load("atraxa-grand-unifier"), alice, Zone::Hand);
    game.refresh_continuous_state().unwrap();
    let request = ManaPaymentRequest::new(alice, source,
        ironsmith::costs::PaymentReason::CastSpell,
        game.object(source).unwrap().mana_cost_owned().unwrap());
    assert!(plan_first_mana_payment(&game, &request).is_err());
    assert_eq!(last_mana_payment_perf().visited_nodes, 0,
        "missing white/black is proven before activation enumeration");
}
