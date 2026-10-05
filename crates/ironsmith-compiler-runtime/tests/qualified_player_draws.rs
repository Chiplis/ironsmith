//! Exact-card source proposals; all scenarios are authored but unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::effects::{
    DestroyEffect, DrawCardsEffect, EffectContext, EffectExecutor, GainLifeEffect,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Step, Target};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::PlayerFilter;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const D: PlayerId = PlayerId(3);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/qualified_player_draws.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn resource(game: &mut GameState, player: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Draw resource", text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn library(game: &mut GameState, player: PlayerId, n: usize) {
    for _ in 0..n {
        resource(
            game,
            player,
            Zone::Library,
            "Mana cost: {9}\nType: Artifact",
        );
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: &dyn EffectExecutor) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: i32) {
    apply(
        game,
        source,
        &DrawCardsEffect::new(count, PlayerFilter::Specific(player)),
    );
}
fn pending(game: &mut GameState, dm: &mut dyn DecisionMaker) -> usize {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState, dm: &mut dyn DecisionMaker) {
    pending(game, dm);
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        pending(game, dm);
    }
    panic!("draw program did not settle");
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    decline: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        !self.decline && ctx.can_accept
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.targets
            .iter()
            .copied()
            .filter(|target| {
                ctx.requirements
                    .iter()
                    .any(|req| req.legal_targets.contains(target))
            })
            .collect()
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut dyn DecisionMaker) {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    for mana in [ManaSymbol::Blue, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(mana, 20);
    }
    game.turn.priority_player = Some(A);
    let action = ironsmith::decision::compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).expect("real cast is legal");
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_cast.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("missing cast choice");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none());
    settle(game, dm);
}
fn battlefield(game: &GameState, name: &str, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == name && game.controller_of(object) == controller
            })
        })
        .collect()
}
#[test]
fn four_exact_cards_have_lossless_direct_and_artifact_programs() {
    assert_eq!(rows().len(), 4);
    for row in rows() {
        definitions(row["name"].as_str().unwrap());
    }
}
#[test]
fn possession_tracks_the_enchanted_opponent_and_optional_draw_but_skips_its_own_draw_step() {
    for definition in definitions("Psychic Possession") {
        let mut game = game();
        for player in [A, B, C] {
            library(&mut game, player, 12);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "Psychic Possession", A)[0];
        draw(&mut game, source, C, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        draw(&mut game, source, B, 2);
        assert_eq!(pending(&mut game, &mut dm), 2);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        dm.decline = true;
        draw(&mut game, source, B, 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        assert!(ironsmith::turn::execute_draw_step_with(&mut game, &mut dm).is_empty());
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
    }
}
#[test]
fn watcher_enters_with_stun_and_qualifies_opponent_turns_then_resolves_both_death_targets() {
    for definition in definitions("The Watcher in the Water") {
        let mut game = game();
        game.set_teams(vec![vec![A, B], vec![C, D]]).unwrap();
        library(&mut game, A, 12);
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "The Watcher in the Water", A)[0];
        assert!(game.is_tapped(source));
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&CounterType::Stun),
            Some(&9)
        );
        for active in [A, B] {
            game.turn.active_player = active;
            draw(&mut game, source, A, 1);
            assert_eq!(pending(&mut game, &mut dm), 0);
        }
        game.turn.active_player = C;
        draw(&mut game, source, A, 2);
        assert_eq!(pending(&mut game, &mut dm), 2);
        settle(&mut game, &mut dm);
        let tentacles = battlefield(&game, "Tentacle", A);
        assert_eq!(tentacles.len(), 2);
        let kraken = resource(
            &mut game,
            A,
            Zone::Battlefield,
            "Type: Creature — Kraken\nPower/Toughness: 4/4",
        );
        game.tap(kraken);
        let artifact = resource(&mut game, C, Zone::Battlefield, "Type: Artifact");
        dm.targets = vec![Target::Object(kraken), Target::Object(artifact)];
        apply(
            &mut game,
            source,
            &DestroyEffect::with_spec(ChooseSpec::SpecificObject(tentacles[0])),
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert!(!game.is_tapped(kraken));
        assert_eq!(
            game.object(artifact)
                .unwrap()
                .counters
                .get(&CounterType::Stun),
            Some(&1)
        );
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&CounterType::Stun),
            Some(&9)
        );
    }
}
#[test]
fn wiretapping_hideaway_free_play_and_first_card_are_scoped_to_each_actual_draw_step() {
    for definition in definitions("Wiretapping") {
        let mut game = game();
        library(&mut game, A, 24);
        let mut dm = SelectFirstDecisionMaker;
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "Wiretapping", A)[0];
        assert_eq!(game.get_exiled_with_source_links(source).len(), 1);
        game.empty_mana_pools();
        for _ in 0..8 {
            resource(&mut game, A, Zone::Hand, "Type: Land");
        }
        // An earlier draw in upkeep must not consume the first draw of the draw step.
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        draw(&mut game, source, A, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.turn.step = Some(Step::Draw);
        for event in ironsmith::turn::execute_draw_step_with(&mut game, &mut dm) {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(
            battlefield(&game, "Draw resource", A).len(),
            1,
            "the hidden nine-mana artifact was played with no mana available"
        );
        assert_eq!(game.draw_step_context_for_player(A), (true, 2));
        draw(&mut game, source, A, 2);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.add_step_after(Step::Draw, Step::Draw);
        ironsmith::turn::advance_step(&mut game).unwrap();
        for event in ironsmith::turn::execute_draw_step_with(&mut game, &mut dm) {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.draw_step_context_for_player(A), (true, 2));
    }
}
#[test]
fn wedding_ring_copy_and_qualified_events_capture_participant_and_amount_before_resolution() {
    for definition in definitions("Wedding Ring") {
        let mut game = game();
        for player in [A, B, C] {
            library(&mut game, player, 12);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        let source = battlefield(&game, "Wedding Ring", A)[0];
        let partner = battlefield(&game, "Wedding Ring", B)[0];
        assert!(matches!(
            game.object(partner).unwrap().kind,
            ironsmith::object::ObjectKind::Token
        ));
        game.turn.active_player = C;
        draw(&mut game, source, C, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.turn.active_player = A;
        draw(&mut game, source, B, 1);
        assert_eq!(pending(&mut game, &mut dm), 0);
        game.turn.active_player = B;
        apply(
            &mut game,
            source,
            &GainLifeEffect::with_filter(4, PlayerFilter::Specific(B)),
        );
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 24);
        draw(&mut game, source, B, 2);
        assert_eq!(pending(&mut game, &mut dm), 2);
        apply(
            &mut game,
            source,
            &DestroyEffect::with_spec(ChooseSpec::SpecificObject(partner)),
        );
        settle(&mut game, &mut dm);
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            2,
            "who-controls is an event qualification, not an intervening-if"
        );
    }
}
