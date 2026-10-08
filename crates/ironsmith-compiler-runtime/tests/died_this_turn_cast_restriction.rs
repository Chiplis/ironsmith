//! "Cast this spell only if a creature died this turn." (CR 601.3, 700.4).
//! Source-authored, deliberately unrun.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId(0);

const GRIM_WANDERER: &str = "Mana cost: {1}{B}\nType: Creature — Goblin Warlock\nPower/Toughness: 5/3\nFlash\nTragic Backstory — Cast this spell only if a creature died this turn.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

fn casts(game: &GameState, id: ObjectId) -> usize {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .count()
}

#[test]
fn grim_wanderer_is_castable_only_after_a_creature_died() {
    for definition in routes("Grim Wanderer", GRIM_WANDERER) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 5);
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert_eq!(casts(&game, card), 0, "no creature died yet");

        let bear = CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let bear = game.create_object_from_definition(&bear, A, Zone::Battlefield);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(bear).unwrap(), &game);
        game.move_object_by_effect(bear, Zone::Graveyard).unwrap();
        let event = ironsmith::events::RawEvent::new(
            ironsmith::events::ZoneChangeEvent::with_cause(
                bear,
                Zone::Battlefield,
                Zone::Graveyard,
                ironsmith::events::cause::EventCause::effect(),
                Some(snapshot.clone()),
            ),
            game.provenance_graph_mut()
                .alloc_root_event(ironsmith::events::EventKind::ZoneChange),
        );
        game.turn_store.turn_history.record_event(&event, Some(snapshot), None);
        assert!(casts(&game, card) > 0, "a creature died this turn");
    }
}
