//! Source-authored and deliberately unrun (cf8 p04): temporary mana rules
//! created by a resolving instruction.
//! - "Until end of turn, whenever a player taps a Swamp for mana, that player
//!   adds an additional {B}." registers a delayed trigger for the turn; it is
//!   a triggered mana ability, so tapping a Swamp for mana adds the extra {B}
//!   immediately, without the stack (CR 605.1b).
//! - Chaos Moon's parity branches: "until end of turn, <anthem> and <mana
//!   trigger | mana rewrite>", with "that Mountain produces ..." accepted.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm};
use ironsmith::game_state::Phase;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);

fn chaos_moon() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/counted_number_references.json.fixture"
    ))
    .unwrap();
    let row = &rows[1];
    assert_eq!(row["name"], "Chaos Moon");
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

#[test]
fn chaos_moon_compiles_both_parity_branches() {
    for definition in chaos_moon() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("CountParity"), "{debug}");
        // Odd: a temporary tap-for-mana trigger adding {R}.
        assert!(debug.contains("PlayerTapsForMana"), "{debug}");
        // Even: a registered rewrite to colorless for Mountains.
        assert!(debug.contains("RegisterManaRewrite"), "{debug}");
        assert!(debug.contains("Mountain"), "{debug}");
        assert!(debug.contains("Colorless"), "{debug}");
    }
}

#[test]
fn temporary_tap_for_mana_trigger_fires_on_a_mana_ability_activation() {
    let muck = compile_to_runtime_definition(
        "Muck Probe",
        "Mana cost: {B}\nType: Instant\n\
         Until end of turn, whenever a player taps a Swamp for mana, that player adds an additional {B}.",
        false,
    )
    .unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&muck));
    let swamp =
        compile_to_runtime_definition("Swamp", "Type: Basic Land — Swamp", false).unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let land = game.create_object_from_definition(&swamp, A, Zone::Battlefield);
    let source = game.create_object_from_definition(&muck, A, Zone::Stack);
    {
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in muck.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    let action = compute_legal_actions(&game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::ActivateManaAbility { source, .. } if *source == land)
        })
        .expect("the Swamp's mana ability");
    let mut state = PriorityLoopState::new(2);
    apply_priority_response_with_dm(
        &mut game,
        &mut TriggerQueue::new(),
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    // {B} from the land plus the triggered mana ability's {B}, no stack use.
    assert_eq!(game.player(A).unwrap().mana_pool.black, 2);
    assert!(game.stack.is_empty());
}
