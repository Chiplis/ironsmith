//! "A or B" triggers keep each arm's functional zones (CR 113.6, 603.6).
//! Source-authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
use ironsmith::triggers::{TriggerEvent, check_triggers};
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const ASTRAL_DRIFT: &str = "Mana cost: {2}{W}\nType: Enchantment\nWhenever you cycle this card or cycle another card while this enchantment is on the battlefield, you may exile target creature. If you do, return that card to the battlefield under its owner's control at the beginning of the next end step.\nCycling {2}{W} ({2}{W}, Discard this card: Draw a card.)";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game
}

fn cycled(game: &GameState, object: ObjectId) -> TriggerEvent {
    let stable = game.object(object).map(|o| o.stable_id.object_id()).unwrap_or(object);
    TriggerEvent::new_with_provenance(
        KeywordActionEvent::new(KeywordActionKind::Cycle, A, stable, 1),
        Default::default(),
    )
}

fn fired(game: &GameState, source: ObjectId, event: &TriggerEvent) -> usize {
    check_triggers(game, event).into_iter().filter(|entry| entry.source == source).count()
}

#[test]
fn astral_drift_other_cycle_arm_never_functions_from_the_graveyard() {
    for definition in definitions("Astral Drift", ASTRAL_DRIFT) {
        let trigger = definition
            .abilities
            .iter()
            .find(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
            .unwrap();
        assert!(trigger.functional_zones.contains(&Zone::Battlefield));
        assert!(trigger.functional_zones.contains(&Zone::Hand));
        let other = compile_to_runtime_definition("Cycler", "Type: Creature\nPower/Toughness: 1/1", false).unwrap();

        // Negative: Astral Drift in the graveyard, another card is cycled.
        let mut g = game();
        let drift = g.create_object_from_definition(&definition, A, Zone::Graveyard);
        let card = g.create_object_from_definition(&other, A, Zone::Graveyard);
        assert_eq!(fired(&g, drift, &cycled(&g, card)), 0, "the battlefield arm is gated");

        // Positive control: on the battlefield the other-card arm fires.
        let mut g = game();
        let drift = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = g.create_object_from_definition(&other, A, Zone::Graveyard);
        assert_eq!(fired(&g, drift, &cycled(&g, card)), 1);

        // The self-cycle arm looks back to the hand for every discard destination.
        for destination in [Zone::Graveyard, Zone::Exile, Zone::Library] {
            let mut g = game();
            let drift = g.create_object_from_definition(&definition, A, Zone::Hand);
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&g, drift).unwrap();
            g.move_object(drift, destination, ironsmith::events::cause::EventCause::from_game_rule()).unwrap();
            use ironsmith::effects::{EffectContext, EffectExecutor, EmitKeywordActionEffect};
            let mut ctx = EffectContext::new_default(drift, A).with_source_snapshot(snapshot);
            let outcome = EmitKeywordActionEffect::new(KeywordActionKind::Cycle, 1)
                .execute(&mut g, &mut ctx).unwrap();
            let event = outcome.events.first().unwrap();
            assert_eq!(fired(&g, drift, event), 1);
        }
    }
}
