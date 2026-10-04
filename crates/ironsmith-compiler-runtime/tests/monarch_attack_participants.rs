//! Exact adjacent monarch bodies using recorded attacking/defending players.
//! Source-authored direct/artifact scenarios; no execution during this campaign.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker,
};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effects::{BecomeMonarchEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/monarch_attack_participants.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|r| r["name"] == name).unwrap();
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 30);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for p in [A, B, C] {
        g.player_mut(p)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 20);
    }
    g
}
fn object(g: &mut GameState, p: PlayerId, z: Zone, text: &str) -> ObjectId {
    let d = compile_to_runtime_definition("Participant resource", text, false).unwrap();
    let id = g.create_object_from_definition(&d, p, z);
    g.remove_summoning_sickness(id);
    id
}
#[derive(Default)]
struct Choice {
    target: Option<ObjectId>,
    legal: Vec<Target>,
}
impl DecisionMaker for Choice {
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.legal = c
            .requirements
            .iter()
            .flat_map(|r| r.legal_targets.iter().cloned())
            .collect();
        if let Some(target) = self
            .target
            .filter(|id| self.legal.contains(&Target::Object(*id)))
        {
            vec![Target::Object(target)]
        } else {
            SelectFirstDecisionMaker.decide_targets(g, c)
        }
    }
    fn decide_mana_payment(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: c.plan.id,
            request_hash: c.plan.request_hash,
        }
    }
}
fn settle(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choice) {
    put_triggers_on_stack_with_dm(g, q, dm).unwrap();
    for _ in 0..32 {
        if g.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(g, dm).unwrap();
        put_triggers_on_stack_with_dm(g, q, dm).unwrap();
    }
    panic!("unsettled stack")
}
fn enter(g: &mut GameState, d: &CardDefinition, dm: &mut Choice) -> ObjectId {
    let old = g.create_object_from_definition(d, A, Zone::Hand);
    let receipt = g
        .move_object_with_etb_processing_with_dm(old, Zone::Battlefield, dm)
        .unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    let source = receipt.original.into_result().unwrap().new_id;
    settle(g, &mut TriggerQueue::new(), dm);
    source
}
fn make_monarch(g: &mut GameState, source: ObjectId, player: PlayerId) {
    BecomeMonarchEffect::new(ironsmith::PlayerFilter::Specific(player))
        .execute(g, &mut EffectContext::new_default(source, A))
        .unwrap();
}
fn attack(
    g: &mut GameState,
    active: PlayerId,
    declarations: &[(ObjectId, AttackTarget)],
) -> TriggerQueue {
    g.turn.active_player = active;
    g.turn.priority_player = Some(active);
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let declarations = declarations
        .iter()
        .map(|(creature, target)| AttackerDeclaration {
            creature: *creature,
            target: target.clone(),
        })
        .collect::<Vec<_>>();
    let mut queue = TriggerQueue::new();
    let mut combat = CombatState::default();
    apply_attacker_declarations(g, &mut combat, &mut queue, &declarations).unwrap();
    queue
}
fn equip(g: &mut GameState, source: ObjectId, host: ObjectId, dm: &mut Choice) {
    dm.target = Some(host);
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    let ability_index = g
        .current_abilities(source)
        .unwrap()
        .iter()
        .position(|a| matches!(&a.kind, AbilityKind::Activated(_)))
        .unwrap();
    let before = g.player(A).unwrap().mana_pool.total();
    let mut state = PriorityLoopState::new(g.players.len());
    let mut q = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source,
            ability_index,
        }),
        dm,
    )
    .unwrap();
    for _ in 0..32 {
        if state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending activation without prompt")
        };
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    settle(g, &mut q, dm);
    assert_eq!(g.player(A).unwrap().mana_pool.total(), before - 2);
    assert_eq!(
        g.object(source).unwrap().attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(host))
    );
}
#[test]
fn two_full_monarch_attack_bodies_round_trip_without_loss() {
    for row in rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
}
#[test]
fn captain_uses_one_recorded_opponent_declaration_and_current_hand_size_after_monarch_changes() {
    for d in definitions("Emberwilde Captain") {
        let mut g = game();
        let mut dm = Choice::default();
        let source = enter(&mut g, &d, &mut dm);
        assert_eq!(g.monarch, Some(A));
        for _ in 0..3 {
            object(&mut g, B, Zone::Hand, "Type: Artifact");
        }
        for _ in 0..7 {
            object(&mut g, C, Zone::Hand, "Type: Artifact");
        }
        let one = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2",
        );
        let two = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2",
        );
        let other = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2",
        );
        let mut q = attack(
            &mut g,
            B,
            &[
                (one, AttackTarget::Player(A)),
                (two, AttackTarget::Player(A)),
                (other, AttackTarget::Player(C)),
            ],
        );
        assert_eq!(q.entries.len(), 1);
        make_monarch(&mut g, source, C);
        g.set_current_controller(one, C).unwrap();
        object(&mut g, B, Zone::Hand, "Type: Artifact");
        settle(&mut g, &mut q, &mut dm);
        assert_eq!(g.player(B).unwrap().life, 26);
        assert_eq!(g.player(C).unwrap().life, 30);
    }
}
#[test]
fn captain_while_condition_is_event_time_and_never_retroactive() {
    for d in definitions("Emberwilde Captain") {
        let mut g = game();
        let mut dm = Choice::default();
        let source = enter(&mut g, &d, &mut dm);
        make_monarch(&mut g, source, C);
        let attacker = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2",
        );
        let q = attack(&mut g, B, &[(attacker, AttackTarget::Player(A))]);
        assert_eq!(q.entries.len(), 0);
        make_monarch(&mut g, source, A);
        assert_eq!(q.entries.len(), 0);
    }
}
#[test]
fn spear_paid_equip_and_attack_bind_original_defender_after_designation_and_attachment_change() {
    for d in definitions("The Spear of Bashenga") {
        let mut g = game();
        let mut dm = Choice::default();
        let source = enter(&mut g, &d, &mut dm);
        assert_eq!(g.monarch, Some(A));
        let host = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2",
        );
        equip(&mut g, source, host, &mut dm);
        assert_eq!(g.calculated_characteristics(host).unwrap().power, Some(4));
        assert!(
            g.current_abilities(host)
                .unwrap()
                .iter()
                .any(|ability| matches!(&ability.kind,AbilityKind::Static(s) if s.has_vigilance()))
        );
        make_monarch(&mut g, source, B);
        let victim = object(&mut g, B, Zone::Battlefield, "Type: Artifact");
        let wrong = object(&mut g, C, Zone::Battlefield, "Type: Artifact");
        g.tap(victim);
        g.tap(wrong);
        let mut q = attack(&mut g, A, &[(host, AttackTarget::Player(B))]);
        assert_eq!(q.entries.len(), 1);
        assert!(!g.is_tapped(host));
        make_monarch(&mut g, source, C);
        g.detach_object_from_current_target(source);
        dm.target = Some(victim);
        put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).unwrap();
        assert!(dm.legal.contains(&Target::Object(victim)));
        assert!(!dm.legal.contains(&Target::Object(wrong)));
        settle(&mut g, &mut q, &mut dm);
        assert!(g.object(victim).is_none());
        assert!(g.object(wrong).is_some());
    }
}
#[test]
fn spear_does_not_replace_existing_monarch_or_trigger_for_their_planeswalker() {
    for d in definitions("The Spear of Bashenga") {
        let mut g = game();
        let mut dm = Choice::default();
        let anchor = object(&mut g, A, Zone::Battlefield, "Type: Artifact");
        make_monarch(&mut g, anchor, B);
        let source = enter(&mut g, &d, &mut dm);
        assert_eq!(g.monarch, Some(B));
        let host = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2",
        );
        equip(&mut g, source, host, &mut dm);
        let planeswalker = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Type: Planeswalker — Jace\nLoyalty: 4",
        );
        let q = attack(
            &mut g,
            A,
            &[(host, AttackTarget::Planeswalker(planeswalker))],
        );
        assert_eq!(q.entries.len(), 0);
    }
}
