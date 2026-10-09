//! Source-authored and deliberately unrun (cf8 p04): one-turn relaxations of
//! the loyalty-ability rule (CR 606.3) — an extra activation ("twice this turn
//! rather than only once", "as though none ... have been activated") and
//! instant-speed activation on any player's turn.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::effect::Effect;
use ironsmith::effects::{
    EffectContext, GrantLoyaltyActivationAllowanceEffect, LoyaltyActivationAllowance,
    LoyaltyActivationScope, execute_effect,
};
use ironsmith::game_state::Phase;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);

const COHORT: &[(&str, &str, &str)] = &[
    ("Jace's Machinations", "89bd056d-8f5b-4d0e-b80a-58a984e21100", "InstantSpeed"),
    ("Kaito, Dancing Shadow", "17fd42f6-f388-4ffc-8fec-ca9ada2506ee", "Source"),
    ("The Chain Veil", "fb88fb3d-6a85-4bb1-b316-7f759b45a0fa", "EachControlledPlaneswalkerNow"),
    ("Urza Assembles the Titans", "0d406496-e1c3-4e1c-976c-618d5ad67d0b", "ControlledPlaneswalkers"),
];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/loyalty_activation_allowances.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&decoded));
    [direct.unwrap(), decoded]
}

#[test]
fn frozen_bodies_carry_their_loyalty_allowance() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/loyalty_activation_allowances.json.fixture"
    ))
    .unwrap();
    for (name, oracle_id, marker) in COHORT {
        assert!(rows.iter().any(|row| row["name"] == *name && row["oracle_id"] == *oracle_id));
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("GrantLoyaltyActivationAllowanceEffect"), "{name}: {debug}");
            assert!(debug.contains(marker), "{name}: {debug}");
        }
    }
}

fn walker(game: &mut GameState) -> ObjectId {
    let definition = compile_to_runtime_definition(
        "Probe Jace",
        "Mana cost: {2}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 5\n+1: Draw a card.",
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, A, Zone::Battlefield)
}

fn can_activate(game: &GameState, source: ObjectId) -> bool {
    compute_legal_actions(game, A).unwrap().iter().any(|action| {
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)
    })
}

fn grant(game: &mut GameState, scope: LoyaltyActivationScope, allowance: LoyaltyActivationAllowance, source: ObjectId) {
    let effect = Effect::new(GrantLoyaltyActivationAllowanceEffect::new(scope, allowance));
    execute_effect(game, &effect, &mut EffectContext::new_default(source, A)).unwrap();
}

#[test]
fn extra_activation_allows_exactly_one_more_loyalty_activation_this_turn() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let jace = walker(&mut game);
    game.turn.active_player = A;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    assert!(can_activate(&game, jace));
    game.record_loyalty_ability_activation(jace);
    assert!(!can_activate(&game, jace), "once per turn by default");
    grant(
        &mut game,
        LoyaltyActivationScope::ControlledPlaneswalkers { subtype: None },
        LoyaltyActivationAllowance::ExtraActivation,
        jace,
    );
    assert!(can_activate(&game, jace), "twice this turn");
    game.record_loyalty_ability_activation(jace);
    assert!(!can_activate(&game, jace), "but not three times");
}

#[test]
fn instant_speed_allowance_covers_only_the_named_subtype() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let jace = walker(&mut game);
    game.turn.active_player = A;
    game.turn.phase = Phase::Combat;
    game.turn.priority_player = Some(A);
    assert!(!can_activate(&game, jace), "sorcery timing by default");
    grant(
        &mut game,
        LoyaltyActivationScope::ControlledPlaneswalkers {
            subtype: Some(ironsmith::Subtype::Kaito),
        },
        LoyaltyActivationAllowance::InstantSpeed,
        jace,
    );
    assert!(!can_activate(&game, jace), "a Kaito allowance doesn't cover Jace");
    grant(
        &mut game,
        LoyaltyActivationScope::ControlledPlaneswalkers {
            subtype: Some(ironsmith::Subtype::Jace),
        },
        LoyaltyActivationAllowance::InstantSpeed,
        jace,
    );
    assert!(can_activate(&game, jace));
}
