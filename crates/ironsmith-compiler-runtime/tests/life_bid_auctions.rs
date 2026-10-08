//! Source-authored and deliberately unrun (cf8 p04): the five sentences of a
//! life auction for control (Illicit Auction) are one procedure, so the later
//! sentences no longer reach the verb parser on their own.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, NumberContext};
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/life_bid_auctions.json.fixture"))
            .unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["name"], "Illicit Auction");
    assert_eq!(row["oracle_id"], "dea8c725-455e-4668-877a-df388fe22255");
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let direct = direct.unwrap();
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        let debug = format!("{:?}", definition.spell_effect);
        assert_eq!(debug.matches("BidLifeEffect").count(), 1, "{debug}");
        assert!(debug.contains("Fixed(0)"), "{debug}");
        assert!(debug.contains("GainControlEffect"), "{debug}");
    }
    [direct, decoded]
}

/// Bob tops the opening bid once with 3 life; Alice then passes.
struct BobBidsThree;

impl DecisionMaker for BobBidsThree {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        ctx.player == B
    }
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        assert_eq!(ctx.player, B);
        ctx.min.max(3)
    }
}

fn auction(definition: &CardDefinition, dm: &mut impl DecisionMaker) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = game.create_object_from_definition(
        &compile_to_runtime_definition(
            "Prize",
            "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
            false,
        )
        .unwrap(),
        B,
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(definition, A, Zone::Stack);
    let requirements = extract_target_requirements_from_program_with_modes(
        &game,
        definition.spell_effect.as_ref().unwrap(),
        A,
        Some(spell),
        None,
    );
    assert_eq!(requirements.len(), 1);
    assert!(requirements[0].legal_targets.contains(&Target::Object(creature)));
    game.push_to_stack(
        StackEntry::new(spell, A)
            .with_targets(vec![Target::Object(creature)])
            .with_target_assignments(vec![TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..1,
            }]),
    );
    resolve_stack_entry_with(&mut game, dm).unwrap();
    (game, creature)
}

#[test]
fn unopposed_opening_bid_of_zero_wins_control_for_free() {
    for definition in definitions() {
        let (game, creature) = auction(&definition, &mut SelectFirstDecisionMaker);
        let object = game.object(creature).unwrap();
        assert_eq!(game.controller_of(object), A);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}

#[test]
fn the_high_bidder_pays_and_keeps_the_creature() {
    for definition in definitions() {
        let (game, creature) = auction(&definition, &mut BobBidsThree);
        let object = game.object(creature).unwrap();
        assert_eq!(game.controller_of(object), B);
        assert_eq!(game.player(B).unwrap().life, 17);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}
