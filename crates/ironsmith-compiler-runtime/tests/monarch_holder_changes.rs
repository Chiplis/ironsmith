//! Real designation changes, recorded player subjects and full monarch bodies.
//! Authored source regressions; unrun during the source-first campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::effects::{BecomeMonarchEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerFilter, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/monarch_holder_changes.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let (parsed, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = parsed.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    GameState::new(vec!["A".into(), "B".into(), "C".into()], 30)
}
fn creature(g: &mut GameState, p: PlayerId) -> ObjectId {
    let d = compile_to_runtime_definition(
        "Monarch resource",
        "Type: Creature — Beast\nPower/Toughness: 2/2",
        false,
    )
    .unwrap();
    g.create_object_from_definition(&d, p, Zone::Battlefield)
}
#[derive(Default)]
struct Choice {
    object: Option<ObjectId>,
    sacrifice: Option<ObjectId>,
    sacrifice_players: Vec<PlayerId>,
    legal: Vec<Target>,
}
impl DecisionMaker for Choice {
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.legal = c
            .requirements
            .iter()
            .flat_map(|r| r.legal_targets.iter().cloned())
            .collect();
        let target = self
            .object
            .map(Target::Object)
            .filter(|target| self.legal.contains(target))
            .or_else(|| {
                self.legal
                    .contains(&Target::Player(B))
                    .then_some(Target::Player(B))
            });
        target
            .map(|target| vec![target])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_targets(g, c))
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        self.sacrifice_players.push(c.player);
        self.sacrifice
            .filter(|id| {
                c.candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            })
            .map(|id| vec![id])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_objects(g, c))
    }
}
fn settle(g: &mut GameState, dm: &mut Choice) {
    let mut q = TriggerQueue::new();
    put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
    for _ in 0..32 {
        if g.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(g, dm).unwrap();
        put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
    }
    panic!("unsettled monarch triggers")
}
fn enter(g: &mut GameState, d: &CardDefinition, dm: &mut Choice) -> ObjectId {
    let old = g.create_object_from_definition(d, A, Zone::Hand);
    let receipt = g
        .move_object_with_etb_processing_with_dm(old, Zone::Battlefield, dm)
        .unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    let source = receipt.original.into_result().unwrap().new_id;
    settle(g, dm);
    source
}
fn make_monarch(g: &mut GameState, source: ObjectId, player: PlayerId) {
    BecomeMonarchEffect::new(PlayerFilter::Specific(player))
        .execute(g, &mut EffectContext::new_default(source, A))
        .unwrap();
}
#[test]
fn all_three_full_holder_bodies_round_trip_without_loss() {
    for row in rows() {
        for d in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
        }
    }
}
#[test]
fn lich_entry_becomes_monarch_then_selected_player_chooses_their_sacrifice_once() {
    for d in definitions("Custodi Lich") {
        let mut g = game();
        let first = creature(&mut g, B);
        let spare = creature(&mut g, B);
        let foreign = creature(&mut g, C);
        let mut dm = Choice {
            sacrifice: Some(spare),
            ..Default::default()
        };
        let source = enter(&mut g, &d, &mut dm);
        assert_eq!(g.monarch, Some(A));
        assert!(g.object(spare).is_none());
        assert!(g.object(first).is_some());
        assert!(g.object(foreign).is_some());
        assert!(dm.sacrifice_players.contains(&B));
        make_monarch(&mut g, source, A);
        put_triggers_on_stack_with_dm(&mut g, &mut TriggerQueue::new(), &mut dm).unwrap();
        assert!(g.stack.is_empty());
        make_monarch(&mut g, source, B);
        settle(&mut g, &mut dm);
        assert!(g.object(first).is_some());
        make_monarch(&mut g, source, A);
        settle(&mut g, &mut dm);
        assert!(g.object(first).is_none());
        assert!(g.object(source).is_some());
    }
}
#[test]
fn garland_targets_recorded_new_monarch_and_preserves_latched_control_and_owned_exclusion() {
    for d in definitions("Garland, Royal Kidnapper") {
        let mut g = game();
        let victim = creature(&mut g, B);
        let own = creature(&mut g, A);
        let foreign = creature(&mut g, C);
        let mut dm = Choice {
            object: Some(victim),
            ..Default::default()
        };
        let source = enter(&mut g, &d, &mut dm);
        assert_eq!(g.monarch, Some(B));
        assert_eq!(g.current_controller(victim), Some(A));
        assert_eq!(g.current_controller(foreign), Some(C));
        assert_eq!(g.calculated_characteristics(victim).unwrap().power, Some(4));
        assert_eq!(g.calculated_characteristics(own).unwrap().power, Some(2));
        assert!(!g.can_be_sacrificed_with_cause(victim, &ironsmith::events::EventCause::effect()));
        assert!(g.can_be_sacrificed_with_cause(own, &ironsmith::events::EventCause::effect()));
        assert!(dm.legal.contains(&Target::Object(victim)));
        assert!(!dm.legal.contains(&Target::Object(foreign)));
        g.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(g.current_controller(victim), Some(A));
        assert_eq!(g.calculated_characteristics(victim).unwrap().power, Some(2));
        make_monarch(&mut g, own, C);
        assert_eq!(g.current_controller(victim), Some(B));
        make_monarch(&mut g, own, B);
        assert_eq!(
            g.current_controller(victim),
            Some(B),
            "expired control must not revive when old holder returns"
        );
    }
}
#[test]
fn knights_use_the_turn_begin_holder_for_every_opponent_change_not_current_monarch() {
    for d in definitions("Knights of the Black Rose") {
        for started_as_monarch in [false, true] {
            let mut g = game();
            let mut dm = Choice::default();
            let source = enter(&mut g, &d, &mut dm);
            if started_as_monarch {
                g.next_turn();
            } else {
                assert_eq!(g.turn_store.turn_history.monarch_at_turn_start, None);
            }
            make_monarch(&mut g, source, B);
            settle(&mut g, &mut dm);
            assert_eq!(
                g.player(B).unwrap().life,
                if started_as_monarch { 28 } else { 30 }
            );
            assert_eq!(
                g.player(A).unwrap().life,
                if started_as_monarch { 32 } else { 30 }
            );
            make_monarch(&mut g, source, C);
            settle(&mut g, &mut dm);
            assert_eq!(
                g.player(C).unwrap().life,
                if started_as_monarch { 28 } else { 30 }
            );
            assert_eq!(
                g.player(A).unwrap().life,
                if started_as_monarch { 34 } else { 30 }
            );
        }
    }
}

#[test]
fn garland_target_announcement_and_revalidation_keep_each_recorded_holder() {
    for definition in definitions("Garland, Royal Kidnapper") {
        let mut g = game();
        // Install the full definition without firing its separate entry choice.
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let b = creature(&mut g, B);
        let c = creature(&mut g, C);
        make_monarch(&mut g, source, B);
        make_monarch(&mut g, source, C);
        let mut dm = Choice {
            object: Some(b),
            ..Default::default()
        };
        put_triggers_on_stack_with_dm(&mut g, &mut TriggerQueue::new(), &mut dm).unwrap();
        assert_eq!(g.stack.len(), 2);
        for entry in &g.stack {
            let changed = entry
                .triggering_event
                .as_ref()
                .unwrap()
                .downcast::<ironsmith::events::MonarchChangedEvent>()
                .unwrap();
            assert_eq!(
                entry.targets,
                vec![Target::Object(if changed.monarch == B { b } else { c })]
            );
        }
        settle(&mut g, &mut dm);
        assert_eq!(g.current_controller(c), Some(A));
        assert_eq!(
            g.current_controller(b),
            Some(B),
            "B's recorded target stays B, but its monarch duration is no longer true"
        );
    }
}
