//! Source-authored and deliberately unrun (cf8 p04): one-shot casts drawn
//! from a named collection while an ability resolves (CR 608.2g, CR 601.2) —
//! "from among cards exiled this way", "from among the exiled cards", "from
//! among cards [you own] exiled with <this source>" (CR 607.2a), "cards
//! exiled with <this source>", and "from your graveyard".
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::resolution::ResolutionProgram;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const COHORT: &[(&str, &str)] = &[
    ("Hellcarver Demon", "90514aa5-84b6-4be7-b6d0-b05529c97140"),
    ("Izzet Chemister", "e4a3d6f3-36ba-4e6d-a254-351bc8886f20"),
    ("Kylox, Visionary Inventor", "37f6bbb8-2136-4f03-966c-85d72180e71f"),
    ("Doom Reigns Supreme", "f398a742-22c1-4da9-9d49-fdce2eb93d01"),
    ("Kaho, Minamo Historian", "395a7bd6-7c2d-448e-842c-ca53256d7008"),
    ("Krang & Shredder", "2385c8fb-9c38-4ff3-8f61-4e25a8c7d46b"),
    ("Shell of the Last Kappa", "a8b24b38-019c-45e7-a5f4-a1bc3014f7a2"),
    ("Boiling Rock Rioter", "5193172c-9024-409e-bd9e-0387971d65fe"),
    ("Jeleva, Nephalia's Scourge", "a014f283-c531-415c-ac00-e6773ea5d64d"),
    ("Summon: Esper Valigarmanda", "cecd2f49-8ccb-472d-9cf1-fd0f4d93546d"),
    ("Chandra Ablaze", "3fae28e6-2ffc-460e-9dd3-e321e8a53ba9"),
    ("Forger's Foundry", "118da256-d1ea-44e8-9026-317e49694d29"),
];

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/collection_casts.json.fixture")).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing fixture row {name}"));
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn frozen_bodies_compile_strictly_with_a_resolution_cast_choice() {
    let rows = rows();
    assert_eq!(rows.len(), COHORT.len());
    for (name, oracle_id) in COHORT {
        assert!(
            rows.iter().any(|row| row["name"] == *name && row["oracle_id"] == *oracle_id),
            "{name}"
        );
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("ChooseObjectsEffect"), "{name}: {debug}");
            assert!(debug.contains("CastTaggedEffect"), "{name}: {debug}");
        }
    }
}

#[test]
fn free_and_paid_casts_keep_their_authored_price_and_pool() {
    for (name, free, linked, owned) in [
        ("Hellcarver Demon", true, false, false),
        ("Kylox, Visionary Inventor", true, false, false),
        ("Doom Reigns Supreme", true, false, false),
        ("Izzet Chemister", true, true, false),
        ("Kaho, Minamo Historian", true, true, false),
        ("Krang & Shredder", true, true, false),
        ("Shell of the Last Kappa", true, true, false),
        ("Jeleva, Nephalia's Scourge", true, true, false),
        ("Forger's Foundry", true, true, false),
        ("Boiling Rock Rioter", false, true, true),
        ("Summon: Esper Valigarmanda", false, true, false),
        ("Chandra Ablaze", true, false, true),
    ] {
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert_eq!(
                debug.contains("without_paying_mana_cost: true"),
                free,
                "{name}: {debug}"
            );
            assert_eq!(debug.contains("__source_exiled__"), linked, "{name}: {debug}");
            if owned {
                assert!(debug.contains("owner: Some(You)"), "{name}: {debug}");
            }
        }
    }
    for definition in definitions("Summon: Esper Valigarmanda") {
        assert!(format!("{definition:?}").contains("AnyType"), "mana of any type");
    }
    for definition in definitions("Kaho, Minamo Historian") {
        assert!(format!("{definition:?}").contains("EqualExpr"), "mana value exactly X");
    }
}

/// Accepts every optional instruction and every legal candidate.
struct TakeEverything;

impl DecisionMaker for TakeEverything {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id);
        match ctx.max {
            Some(max) => legal.take(max).collect(),
            None => legal.collect(),
        }
    }
}

fn activated_program(definition: &CardDefinition, index: usize) -> ResolutionProgram {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability.effects.clone()),
            _ => None,
        })
        .nth(index)
        .expect("activated ability")
}

fn resolve_ability(
    game: &mut GameState,
    source: ObjectId,
    program: ResolutionProgram,
    targets: Vec<Target>,
    dm: &mut impl DecisionMaker,
) {
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        &program,
        A,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), targets.len());
    let assignments = requirements
        .iter()
        .enumerate()
        .map(|(index, requirement)| TargetAssignment {
            spec: requirement.spec.clone(),
            range: index..index + 1,
        })
        .collect();
    game.push_to_stack(
        StackEntry::ability(source, A, program)
            .with_targets(targets)
            .with_target_assignments(assignments),
    );
    resolve_stack_entry_with(game, dm).unwrap();
}

#[test]
fn izzet_chemister_casts_only_its_linked_exiles_for_free() {
    for definition in definitions("Izzet Chemister") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let chemister = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let insight = compile_to_runtime_definition(
            "Linked Insight",
            "Mana cost: {4}{U}\nType: Sorcery\nDraw a card.",
            false,
        )
        .unwrap();
        let in_graveyard = game.create_object_from_definition(&insight, A, Zone::Graveyard);
        // An unrelated exiled card is not part of the linked pool.
        let unrelated = game.create_object_from_definition(&insight, B, Zone::Exile);
        for n in 0..2 {
            game.create_object_from_definition(
                &compile_to_runtime_definition(&format!("Library {n}"), "Type: Land", false)
                    .unwrap(),
                A,
                Zone::Library,
            );
        }
        resolve_ability(
            &mut game,
            chemister,
            activated_program(&definition, 0),
            vec![Target::Object(in_graveyard)],
            &mut SelectFirstDecisionMaker,
        );
        let linked = game.get_exiled_with_source_links(chemister).to_vec();
        assert_eq!(linked.len(), 1);
        resolve_ability(
            &mut game,
            chemister,
            activated_program(&definition, 1),
            vec![],
            &mut TakeEverything,
        );
        assert_eq!(game.stack.len(), 1, "the linked card was cast");
        let cast = game.object(game.stack[0].object_id).unwrap();
        assert_eq!(cast.name, "Linked Insight");
        assert_eq!(cast.owner, A);
        assert_eq!(
            game.object(unrelated).map(|object| object.zone),
            Some(Zone::Exile),
            "an unlinked exiled card stays put"
        );
        let mana_before = game.player(A).unwrap().mana_pool.clone();
        let hand_before = game.player(A).unwrap().hand.len();
        resolve_stack_entry_with(&mut game, &mut TakeEverything).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before + 1);
        assert_eq!(game.player(A).unwrap().mana_pool, mana_before);
    }
}
