//! Complete combat participant and declaration grouping bodies.
//! Source-authored scenarios, unrun during the implementation-first campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effects::{EffectContext as ExecutionContext, EffectExecutor};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_runtime_definition;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/combat_declaration_triggers.json.fixture"
    ))
    .unwrap()
}
fn compile_face(row: &serde_json::Value, other: Option<&str>) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let mut builder =
        ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name);
    if let Some(other) = other {
        builder = builder.other_face_name(other);
    }
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_builder_to_artifact(builder, text, false)
    });
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    compile_face(&row, row["other_face"]["name"].as_str())
}
fn linked_hezrou(game: &mut GameState, index: usize) -> CardDefinition {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == "Hezrou")
        .unwrap();
    let mut front = compile_face(&row, Some("Demonic Stench"))[index].clone();
    let mut back = compile_face(&row["other_face"], Some("Hezrou"))[index].clone();
    front.card.other_face = Some(back.card.id);
    back.card.other_face = Some(front.card.id);
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    front
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 30);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for p in [A, B, C] {
        for symbol in [
            ironsmith::mana::ManaSymbol::Red,
            ironsmith::mana::ManaSymbol::Green,
            ironsmith::mana::ManaSymbol::Blue,
            ironsmith::mana::ManaSymbol::Black,
            ironsmith::mana::ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(symbol, 30);
        }
    }
    g
}
fn object(g: &mut GameState, p: PlayerId, z: Zone, name: &str, text: &str) -> ObjectId {
    g.create_object_from_definition(
        &compile_to_runtime_definition(name, text, false).unwrap(),
        p,
        z,
    )
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    option: usize,
    accept: bool,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.target
            .filter(|id| {
                c.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&Target::Object(*id)))
            })
            .map(|id| vec![Target::Object(id)])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_targets(g, c))
    }
    fn decide_objects(&mut self, _g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        c.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(c.max.unwrap_or(c.candidates.len()))
            .map(|candidate| candidate.id)
            .collect()
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
    fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
        vec![self.option]
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.accept
    }
}
fn stack(g: &mut GameState, events: Vec<TriggerEvent>, dm: &mut Choices) -> usize {
    for event in events {
        g.queue_trigger_event(Default::default(), event);
    }
    put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
    g.stack.len()
}
fn settle(g: &mut GameState, dm: &mut Choices) {
    for _ in 0..32 {
        if g.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(g, dm).unwrap();
        put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
    }
    panic!("unsettled stack")
}
fn action(g: &mut GameState, p: PlayerId, action: LegalAction, dm: &mut Choices) {
    g.turn.priority_player = Some(p);
    let mut state = PriorityLoopState::new(g.players.len());
    let mut q = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(c) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &c, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
}

fn creature(g: &mut GameState, p: PlayerId, name: &str, extra: &str) -> ObjectId {
    let id = object(
        g,
        p,
        Zone::Battlefield,
        name,
        &format!("Type: Creature — Beast\nPower/Toughness: 2/10\n{extra}"),
    );
    g.remove_summoning_sickness(id);
    id
}
fn declare(g: &mut GameState, attacks: &[(ObjectId, AttackTarget)]) -> (CombatState, TriggerQueue) {
    g.turn.phase = ironsmith::Phase::Combat;
    g.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    g.mark_combat_phase_started();
    let mut combat = CombatState::default();
    let mut q = TriggerQueue::new();
    let declarations = attacks
        .iter()
        .map(|(creature, target)| AttackerDeclaration {
            creature: *creature,
            target: target.clone(),
        })
        .collect::<Vec<_>>();
    ironsmith::game_loop::apply_attacker_declarations(g, &mut combat, &mut q, &declarations)
        .unwrap();
    g.combat = Some(combat.clone());
    (combat, q)
}
fn blocks(
    g: &mut GameState,
    combat: &mut CombatState,
    q: &mut TriggerQueue,
    pairs: &[(ObjectId, ObjectId)],
) {
    g.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let declarations = pairs
        .iter()
        .map(|(blocker, attacker)| BlockerDeclaration {
            blocker: *blocker,
            blocking: *attacker,
        })
        .collect::<Vec<_>>();
    ironsmith::game_loop::apply_multiplayer_blocker_declarations(g, combat, q, &declarations)
        .unwrap();
    g.combat = Some(combat.clone());
}
fn put(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> usize {
    put_triggers_on_stack_with_dm(g, q, dm).unwrap();
    g.stack.len()
}
fn double(g: &GameState, id: ObjectId) -> bool {
    g.current_has_static_ability_id(
        id,
        ironsmith::static_abilities::StaticAbilityId::DoubleStrike,
    )
}
#[test]
fn four_exact_identities_and_both_adventure_faces_survive_artifact_transport() {
    assert_eq!(fixtures().len(), 4);
    for row in fixtures() {
        for d in compile_face(&row, row["other_face"]["name"].as_str()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
        }
        if row["other_face"].is_object() {
            for d in compile_face(&row["other_face"], row["name"].as_str()) {
                assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
            }
        }
    }
}
#[test]
fn hezrou_groups_one_completed_multiplayer_block_declaration_and_affects_current_blockers() {
    for d in definitions("Hezrou") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let a = creature(&mut g, A, "First attacker", "");
        let b = creature(&mut g, A, "Second attacker", "");
        let x = creature(&mut g, B, "First blocker", "");
        let y = creature(&mut g, C, "Second blocker", "");
        let idle = creature(&mut g, B, "Not blocking", "");
        let (mut combat, mut q) = declare(
            &mut g,
            &[(a, AttackTarget::Player(B)), (b, AttackTarget::Player(C))],
        );
        blocks(&mut g, &mut combat, &mut q, &[(x, a), (y, b)]);
        let mut dm = Choices::default();
        assert_eq!(put(&mut g, &mut q, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.current_power(x), Some(1));
        assert_eq!(g.current_power(y), Some(1));
        assert_eq!(g.current_power(idle), Some(2));
        assert_eq!(g.current_power(source), Some(6));
        ironsmith::turn::execute_cleanup_step(&mut g);
        assert_eq!(g.current_power(x), Some(2));
    }
}
#[test]
fn righteous_uses_each_blocking_pair_and_the_definite_blocker_survives_leaving_combat() {
    for d in definitions("Righteous Indignation") {
        for blink in [false, true] {
            let mut g = game();
            let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
            let red = object(
                &mut g,
                A,
                Zone::Battlefield,
                "Red attacker",
                "Mana cost: {R}\nType: Creature\nPower/Toughness: 2/10",
            );
            let black = object(
                &mut g,
                A,
                Zone::Battlefield,
                "Black attacker",
                "Mana cost: {B}\nType: Creature\nPower/Toughness: 2/10",
            );
            for id in [red, black] {
                g.remove_summoning_sickness(id);
            }
            let blocker = creature(
                &mut g,
                B,
                "Multiblocker",
                "This creature can block any number of creatures.",
            );
            let (mut combat, mut q) = declare(
                &mut g,
                &[
                    (red, AttackTarget::Player(B)),
                    (black, AttackTarget::Player(B)),
                ],
            );
            blocks(
                &mut g,
                &mut combat,
                &mut q,
                &[(blocker, red), (blocker, black)],
            );
            let mut dm = Choices::default();
            assert_eq!(put(&mut g, &mut q, &mut dm), 2);
            let remaining = if blink {
                let exile = g.move_object_by_effect(blocker, Zone::Exile).unwrap();
                g.move_object_by_effect(exile, Zone::Battlefield).unwrap()
            } else {
                ironsmith::effects::RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(
                    blocker,
                ))
                .execute(&mut g, &mut ExecutionContext::new_default(source, A))
                .unwrap();
                blocker
            };
            settle(&mut g, &mut dm);
            assert_eq!(g.current_power(remaining), Some(if blink { 2 } else { 4 }));
            assert_eq!(g.current_power(red), Some(2));
            assert_eq!(g.current_power(black), Some(2));
        }
    }
}
#[test]
fn neyith_unions_grouped_fight_and_blocked_events_without_duplicate_participants() {
    for d in definitions("Neyith of the Dire Hunt") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        for _ in 0..5 {
            object(&mut g, A, Zone::Library, "Drawn card", "Type: Land");
        }
        let a = creature(&mut g, A, "First fighter", "");
        let b = creature(&mut g, A, "Second fighter", "");
        let enemy = creature(&mut g, B, "Other fighter", "");
        let mut dm = Choices::default();
        let fight = ironsmith::effects::FightEffect::new(
            ChooseSpec::SpecificObject(a),
            ChooseSpec::SpecificObject(b),
        );
        let out = fight
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert_eq!(stack(&mut g, out.events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 1);
        let out = ironsmith::effects::FightEffect::new(
            ChooseSpec::SpecificObject(a),
            ChooseSpec::SpecificObject(enemy),
        )
        .execute(&mut g, &mut ExecutionContext::new(source, B, &mut dm))
        .unwrap();
        assert_eq!(stack(&mut g, out.events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 2);
        let second_enemy = creature(&mut g, C, "Other blocker", "");
        let (mut combat, mut q) = declare(
            &mut g,
            &[(a, AttackTarget::Player(B)), (b, AttackTarget::Player(C))],
        );
        blocks(
            &mut g,
            &mut combat,
            &mut q,
            &[(enemy, a), (second_enemy, b)],
        );
        assert_eq!(put(&mut g, &mut q, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 3);
    }
}
#[test]
fn neyith_paid_hybrid_clause_doubles_only_chosen_power_and_enforces_a_legal_block() {
    for d in definitions("Neyith of the Dire Hunt") {
        for accept in [false, true] {
            let mut g = game();
            let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
            let target = creature(&mut g, A, "Chosen attacker", "");
            let other = creature(&mut g, A, "Other attacker", "");
            let blocker = creature(&mut g, B, "Legal blocker", "");
            g.add_counters(target, ironsmith::CounterType::PlusOnePlusOne, 1);
            g.turn.phase = ironsmith::Phase::Combat;
            g.turn.step = Some(ironsmith::game_state::Step::BeginCombat);
            let mana = g.player(A).unwrap().mana_pool.total();
            let mut dm = Choices {
                target: Some(target),
                accept,
                ..Default::default()
            };
            assert_eq!(
                stack(
                    &mut g,
                    vec![TriggerEvent::new(
                        ironsmith::events::phase::BeginningOfCombatEvent::new(A),
                        Default::default()
                    )],
                    &mut dm
                ),
                1
            );
            settle(&mut g, &mut dm);
            assert_eq!(
                g.player(A).unwrap().mana_pool.total(),
                mana - if accept { 3 } else { 0 }
            );
            assert_eq!(g.current_power(target), Some(if accept { 6 } else { 3 }));
            assert_eq!(g.current_toughness(target), Some(11));
            assert_eq!(g.current_power(other), Some(2));
            let (mut combat, mut q) = declare(&mut g, &[(target, AttackTarget::Player(B))]);
            g.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
            let empty = ironsmith::game_loop::apply_multiplayer_blocker_declarations(
                &mut g,
                &mut combat,
                &mut q,
                &[],
            );
            assert_eq!(empty.is_err(), accept);
            if accept {
                blocks(&mut g, &mut combat, &mut q, &[(blocker, target)]);
            }
            ironsmith::turn::execute_cleanup_step(&mut g);
            assert_eq!(g.current_power(target), Some(3));
            assert_eq!(
                g.counter_count(target, ironsmith::CounterType::PlusOnePlusOne),
                1
            );
            let _ = source;
        }
    }
}
#[test]
fn yuriko_counts_only_other_declared_attackers_of_the_same_direct_player() {
    for d in definitions("Yuriko, Blade of the Mighty") {
        for same_player in [false, true] {
            let mut g = game();
            g.create_object_from_definition(&d, A, Zone::Battlefield);
            let first = creature(&mut g, A, "First attacker", "");
            let second = creature(&mut g, A, "Second attacker", "");
            let walker = object(
                &mut g,
                B,
                Zone::Battlefield,
                "Defending walker",
                "Type: Planeswalker\nLoyalty: 20",
            );
            let walker_attacker = creature(&mut g, A, "Walker attacker", "");
            let (_, mut q) = declare(
                &mut g,
                &[
                    (first, AttackTarget::Player(B)),
                    (
                        second,
                        AttackTarget::Player(if same_player { B } else { C }),
                    ),
                    (walker_attacker, AttackTarget::Planeswalker(walker)),
                ],
            );
            let mut dm = Choices::default();
            assert_eq!(
                put(&mut g, &mut q, &mut dm),
                if same_player { 0 } else { 2 }
            );
            g.combat = None;
            settle(&mut g, &mut dm);
            assert_eq!(double(&g, first), !same_player);
            assert_eq!(double(&g, second), !same_player);
            assert!(!double(&g, walker_attacker));
            ironsmith::turn::execute_cleanup_step(&mut g);
            assert!(!double(&g, first));
        }
    }
}
#[test]
fn yuriko_combat_restriction_preserves_mana_abilities_and_expires_with_the_phase() {
    for d in definitions("Yuriko, Blade of the Mighty") {
        let mut g = game();
        g.create_object_from_definition(&d, A, Zone::Battlefield);
        let spell = object(
            &mut g,
            A,
            Zone::Hand,
            "Instant probe",
            "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
        );
        let mana = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Mana artifact",
            "Type: Artifact\n{T}: Add {C}.",
        );
        let other = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Nonmana artifact",
            "Type: Artifact\n{0}: You gain 1 life.",
        );
        for combat in [true, false] {
            g.turn.phase = if combat {
                ironsmith::Phase::Combat
            } else {
                ironsmith::Phase::FirstMain
            };
            g.turn.step = if combat {
                Some(ironsmith::game_state::Step::BeginCombat)
            } else {
                None
            };
            let actions = ironsmith::decision::compute_legal_actions(&g, A).unwrap();
            assert_eq!(
                actions
                    .iter()
                    .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell)),
                !combat
            );
            assert_eq!(
                actions
                    .iter()
                    .any(|a| matches!(a,LegalAction::ActivateAbility{source,..} if *source==other)),
                !combat
            );
            assert!(
                actions.iter().any(
                    |a| matches!(a,LegalAction::ActivateManaAbility{source,..} if *source==mana)
                )
            );
        }
    }
}
#[test]
fn hezrou_adventure_uses_completed_block_history_and_exile_cast_permission() {
    for index in 0..2 {
        let mut g = game();
        let d = linked_hezrou(&mut g, index);
        let attacker = creature(&mut g, A, "Attacker", "");
        let blocker = creature(&mut g, B, "Prior blocker", "");
        let idle = creature(&mut g, B, "Never blocked", "");
        let (mut combat, mut q) = declare(&mut g, &[(attacker, AttackTarget::Player(B))]);
        blocks(&mut g, &mut combat, &mut q, &[(blocker, attacker)]);
        // The Adventure is an instant; it still finds the historical blocker
        // after combat, while a current-blocking filter would find nothing.
        g.combat = None;
        g.turn.phase = ironsmith::Phase::NextMain;
        g.turn.step = None;
        let card = g.create_object_from_definition(&d, A, Zone::Hand);
        let stable = g.object(card).unwrap().stable_id;
        let mut dm = Choices::default();
        action(
            &mut g,
            A,
            LegalAction::CastSpell {
                spell_id: card,
                from_zone: Zone::Hand,
                casting_method: ironsmith::alternative_cast::CastingMethod::SplitOtherHalf,
            },
            &mut dm,
        );
        settle(&mut g, &mut dm);
        assert_eq!(g.current_power(blocker), Some(1));
        assert_eq!(g.current_power(idle), Some(2));
        let exile = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(g.object(exile).unwrap().zone, Zone::Exile);
        assert_eq!(g.adventure_exiled_player(exile), Some(A));
        action(
            &mut g,
            A,
            LegalAction::CastSpell {
                spell_id: exile,
                from_zone: Zone::Exile,
                casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
            },
            &mut dm,
        );
        settle(&mut g, &mut dm);
        let permanent = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(g.object(permanent).unwrap().zone, Zone::Battlefield);
        assert_eq!(g.current_power(permanent), Some(6));
    }
}

#[test]
fn righteous_retains_its_untargeted_buff_on_a_blocker_that_stops_being_a_creature() {
    use ironsmith::continuous::{EffectTarget, Modification, PtSublayer};
    use ironsmith::effect::{Until, Value};
    let animate = |land| {
        ironsmith::effects::ApplyContinuousEffect::new(
            EffectTarget::Specific(land),
            Modification::AddCardTypes(vec![ironsmith::CardType::Creature]),
            Until::ThisLeavesTheBattlefield,
        )
        .with_additional_modification(Modification::SetPowerToughness {
            power: Value::Fixed(2),
            toughness: Value::Fixed(10),
            sublayer: PtSublayer::Setting,
        })
    };
    for d in definitions("Righteous Indignation") {
        let mut g = game();
        let observer = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let red = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Red attacker",
            "Mana cost: {R}\nType: Creature\nPower/Toughness: 2/10",
        );
        g.remove_summoning_sickness(red);
        let land = object(&mut g, B, Zone::Battlefield, "Animated land", "Type: Land");
        let animation_source = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Animation source",
            "Type: Artifact",
        );
        animate(land)
            .execute(
                &mut g,
                &mut ExecutionContext::new_default(animation_source, B),
            )
            .unwrap();
        let (mut combat, mut q) = declare(&mut g, &[(red, AttackTarget::Player(B))]);
        blocks(&mut g, &mut combat, &mut q, &[(land, red)]);
        let mut dm = Choices::default();
        assert_eq!(put(&mut g, &mut q, &mut dm), 1);
        g.move_object_by_effect(animation_source, Zone::Graveyard)
            .unwrap();
        g.refresh_continuous_state().unwrap();
        assert!(!g.current_is_creature(land));
        settle(&mut g, &mut dm);
        let second_source = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Later animation source",
            "Type: Artifact",
        );
        animate(land)
            .execute(&mut g, &mut ExecutionContext::new_default(second_source, B))
            .unwrap();
        assert_eq!(g.current_power(land), Some(3));
        assert_eq!(g.current_toughness(land), Some(11));
        ironsmith::turn::execute_cleanup_step(&mut g);
        assert_eq!(g.current_power(land), Some(2));
        let _ = observer;
    }
}
