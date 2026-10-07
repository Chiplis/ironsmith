//! Exact frozen card bodies and native grouped/repeated coin instruction owners.
//! Authored from source only; builds and execution are deferred by the campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::{EffectOutcome, Until};
use ironsmith::effects::{EffectContext as ExecutionContext, EffectExecutor, FlipCoinEffect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm,
    generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{CoinFace, CounterType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const H: CoinFace = CoinFace::Heads;
const T: CoinFace = CoinFace::Tails;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/grouped_coin_flips.json.fixture")).unwrap()
}

fn program_definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|row| row["name"] == name).unwrap();
    program_definitions(name, row["text"].as_str().unwrap())
}

fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 30);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for player in [A, B] {
        for symbol in [
            ironsmith::ManaSymbol::Red,
            ironsmith::ManaSymbol::Blue,
            ironsmith::ManaSymbol::Colorless,
        ] {
            g.player_mut(player).unwrap().mana_pool.add(symbol, 30);
        }
    }
    g
}

fn object(g: &mut GameState, player: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    g.create_object_from_definition(
        &compile_to_runtime_definition(name, text, false).unwrap(), player, zone,
    )
}

#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    option: usize,
    accept: bool,
    booleans: Vec<bool>,
    boolean_players: Vec<PlayerId>,
    pause_on_boolean: Option<usize>,
    option_players: Vec<PlayerId>,
    pause_on_option: Option<usize>,
    number: Option<u32>,
    pause_on_number: bool,
    pending: bool,
}

impl DecisionMaker for Choices {
    fn decide_targets(&mut self, g: &GameState, context: &TargetsContext) -> Vec<Target> {
        if self.targets.is_empty() {
            return SelectFirstDecisionMaker.decide_targets(g, context);
        }
        self.targets.clone()
    }

    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        context.candidates.iter().filter(|candidate| candidate.legal)
            .take(context.max.unwrap_or(context.candidates.len()))
            .map(|candidate| candidate.id).collect()
    }

    fn decide_mana_payment(
        &mut self, _: &GameState, context: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: context.plan.id, request_hash: context.plan.request_hash,
        }
    }

    fn decide_options(&mut self, _: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        self.option_players.push(context.player);
        self.pending = self.pause_on_option == Some(self.option_players.len());
        vec![self.option]
    }

    fn decide_number(&mut self, _: &GameState, context: &ironsmith::decisions::context::NumberContext) -> u32 {
        self.pending = self.pause_on_number;
        self.number.unwrap_or(context.min)
    }

    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        self.boolean_players.push(context.player);
        self.pending = self.pause_on_boolean == Some(self.boolean_players.len());
        self.booleans.get(self.boolean_players.len() - 1).copied().unwrap_or(self.accept)
    }

    fn awaiting_choice(&self) -> bool {
        self.pending
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
    for _ in 0..64 {
        if g.stack.is_empty() { return; }
        resolve_stack_entry_with(g, dm).unwrap();
        assert!(!dm.pending, "settle requires completed decisions");
        put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
    }
    panic!("unsettled stack");
}

fn action(g: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    g.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(g.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        g, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm,
    ).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(g, &mut queue, dm).unwrap();
}

fn cast(g: &mut GameState, card: ObjectId, dm: &mut Choices) {
    action(g, A, LegalAction::CastSpell {
        spell_id: card, from_zone: Zone::Hand,
        casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
    }, dm);
}

fn attack(g: &mut GameState, attacker: ObjectId, dm: &mut Choices) {
    g.remove_summoning_sickness(attacker);
    g.turn.phase = ironsmith::Phase::Combat;
    g.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(g, &mut combat, &mut queue, &[AttackerDeclaration {
        creature: attacker, target: AttackTarget::Player(B),
    }]).unwrap();
    g.combat = Some(combat);
    put_triggers_on_stack_with_dm(g, &mut queue, dm).unwrap();
}

fn begin_combat(g: &mut GameState, player: PlayerId, dm: &mut Choices) -> usize {
    g.turn.active_player = player;
    g.turn.phase = ironsmith::Phase::Combat;
    g.turn.step = Some(ironsmith::game_state::Step::BeginCombat);
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(g, &mut queue);
    put_triggers_on_stack_with_dm(g, &mut queue, dm).unwrap();
    g.stack.len()
}

fn force(g: &mut GameState, faces: &[CoinFace]) {
    for &face in faces { g.force_next_coin_flip(face); }
}

fn flip(
    g: &mut GameState, source: ObjectId, player: PlayerId,
    count: u32, face_only: bool, repeat: bool, dm: &mut Choices,
) -> EffectOutcome {
    let mut effect = if face_only {
        FlipCoinEffect::face_only(PlayerFilter::Specific(player))
    } else {
        FlipCoinEffect::new(PlayerFilter::Specific(player))
    };
    effect.count = count;
    effect.repeat_until_loss = repeat;
    effect.execute(g, &mut ExecutionContext::new(source, B, dm)).unwrap()
}

fn library(g: &mut GameState, player: PlayerId, count: usize) {
    for _ in 0..count { object(g, player, Zone::Library, "Draw card", "Type: Land"); }
}

#[test]
fn eight_exact_entries_have_complete_direct_and_roundtripped_artifact_programs() {
    assert_eq!(fixtures().len(), 8);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
}

#[test]
fn firecat_cast_and_etb_count_only_wins_before_the_first_loss() {
    for definition in definitions("Crazed Firecat") {
        for (faces, wins) in [(vec![T], 0), (vec![H, H, T], 2)] {
            let mut g = game();
            let card = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(card).unwrap().stable_id;
            let mut dm = Choices::default();
            force(&mut g, &faces);
            cast(&mut g, card, &mut dm);
            settle(&mut g, &mut dm);
            let cat = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(g.object(cat).unwrap().zone, Zone::Battlefield);
            assert_eq!(g.counter_count(cat, CounterType::PlusOnePlusOne), wins);
            assert_eq!(g.current_power(cat), Some(4 + wins as i32));
            assert_eq!(dm.option_players, vec![A; faces.len()]);
            force(&mut g, &[H]);
            let out = flip(&mut g, cat, A, 1, false, false, &mut dm);
            stack(&mut g, out.events, &mut dm);
            settle(&mut g, &mut dm);
            assert_eq!(g.counter_count(cat, CounterType::PlusOnePlusOne), wins,
                "a later independent win cannot change the resolved ETB receipt");
        }
    }
}

#[test]
fn traprunner_creates_one_tapped_attacker_for_each_called_win() {
    for definition in definitions("Goblin Traprunner") {
        for (faces, wins) in [(vec![T, T, T], 0), (vec![H, T, H], 2), (vec![H, H, H], 3)] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut dm = Choices::default();
            force(&mut g, &faces);
            attack(&mut g, source, &mut dm);
            assert_eq!(g.stack.len(), 1);
            settle(&mut g, &mut dm);
            let tokens: Vec<_> = g.battlefield.iter().copied()
                .filter(|id| *id != source && g.object(*id).unwrap().name == "Goblin").collect();
            assert_eq!(tokens.len(), wins);
            assert_eq!(dm.option_players, vec![A; 3]);
            for token in tokens {
                assert!(g.is_tapped(token));
                assert_eq!(g.current_controller(token), Some(A));
                assert_eq!(g.current_power(token), Some(1));
                assert_eq!(g.current_toughness(token), Some(1));
                assert!(g.combat.as_ref().unwrap().attackers.iter().any(|attack| {
                    attack.creature == token && attack.target == AttackTarget::Player(B)
                }));
            }
        }
    }
}

#[test]
fn giant_tests_both_faces_of_its_own_instruction_and_expires_at_cleanup() {
    for definition in definitions("Two-Headed Giant") {
        for faces in [[H, H], [H, T], [T, H], [T, T]] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut dm = Choices::default();
            force(&mut g, &faces);
            attack(&mut g, source, &mut dm);
            settle(&mut g, &mut dm);
            assert!(dm.option_players.is_empty(), "face-only coins never ask for a call");
            assert_eq!(g.current_has_static_ability_id(source, StaticAbilityId::DoubleStrike), faces == [H, H]);
            assert_eq!(g.current_has_static_ability_id(source, StaticAbilityId::Menace), faces == [T, T]);
            ironsmith::turn::execute_cleanup_step(&mut g);
            assert!(!g.current_has_static_ability_id(source, StaticAbilityId::DoubleStrike));
            assert!(!g.current_has_static_ability_id(source, StaticAbilityId::Menace));
        }
    }
}

#[test]
fn ral_five_face_only_coins_pay_loyalty_and_schedule_exactly_the_heads_count() {
    for definition in definitions("Ral Zarek") {
        for (faces, turns) in [([T, T, T, T, T], 0), ([H, T, H, T, H], 3), ([H, H, H, H, H], 5)] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            g.add_counters(source, CounterType::Loyalty, 4);
            let indices: Vec<_> = definition.abilities.iter().enumerate().filter_map(|(index, ability)| {
                matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(index)
            }).collect();
            assert_eq!(indices.len(), 3);
            let loyalty = g.counter_count(source, CounterType::Loyalty);
            let mut dm = Choices::default();
            force(&mut g, &faces);
            action(&mut g, A, LegalAction::ActivateAbility { source, ability_index: indices[2] }, &mut dm);
            assert_eq!(g.counter_count(source, CounterType::Loyalty), loyalty - 7);
            settle(&mut g, &mut dm);
            assert_eq!(g.turn_store.extra_turns, vec![A; turns]);
            assert!(dm.option_players.is_empty());
        }
    }
}

#[test]
fn ral_other_two_loyalty_bodies_keep_distinct_targets_and_damage() {
    for definition in definitions("Ral Zarek") {
        for ability in [0, 1] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first = object(&mut g, B, Zone::Battlefield, "First permanent", "Type: Artifact");
            let second = object(&mut g, A, Zone::Battlefield, "Second permanent", "Type: Artifact");
            g.tap(second);
            let indices: Vec<_> = definition.abilities.iter().enumerate().filter_map(|(index, ability)| {
                matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(index)
            }).collect();
            let mut dm = Choices { targets: if ability == 0 {
                vec![Target::Object(first), Target::Object(second)]
            } else { vec![Target::Player(B)] }, ..Default::default() };
            let loyalty = g.counter_count(source, CounterType::Loyalty);
            action(&mut g, A, LegalAction::ActivateAbility { source, ability_index: indices[ability] }, &mut dm);
            settle(&mut g, &mut dm);
            if ability == 0 {
                assert!(g.is_tapped(first));
                assert!(!g.is_tapped(second));
                assert_eq!(g.counter_count(source, CounterType::Loyalty), loyalty + 1);
            } else {
                assert_eq!(g.player(B).unwrap().life, 27);
                assert_eq!(g.counter_count(source, CounterType::Loyalty), loyalty - 2);
            }
        }
    }
}

#[test]
fn each_partner_entry_searches_only_its_named_partner_through_its_cast_etb() {
    for (name, partner) in [
        ("Okaun, Eye of Chaos", "Zndrsplt, Eye of Wisdom"),
        ("Okaun, Eye of Chaos // Okaun, Eye of Chaos", "Zndrsplt, Eye of Wisdom"),
        ("Zndrsplt, Eye of Wisdom", "Okaun, Eye of Chaos"),
        ("Zndrsplt, Eye of Wisdom // Zndrsplt, Eye of Wisdom", "Okaun, Eye of Chaos"),
    ] {
        for definition in definitions(name) {
            for accept in [false, true] {
                let mut g = game();
                let card = g.create_object_from_definition(&definition, A, Zone::Hand);
                let stable = g.object(card).unwrap().stable_id;
                object(&mut g, B, Zone::Library, partner, "Type: Creature\nPower/Toughness: 1/1");
                object(&mut g, B, Zone::Library, "Unrelated creature", "Type: Creature\nPower/Toughness: 1/1");
                let mut dm = Choices { targets: vec![Target::Player(B)], accept, ..Default::default() };
                cast(&mut g, card, &mut dm);
                settle(&mut g, &mut dm);
                let source = g.find_object_by_stable_id(stable).unwrap();
                assert!(g.current_has_static_ability_id(source, StaticAbilityId::PartnerWith));
                assert_eq!(g.player(B).unwrap().hand.len(), usize::from(accept));
                assert_eq!(g.player(B).unwrap().library.len(), if accept { 1 } else { 2 });
                if accept {
                    assert_eq!(g.object(g.player(B).unwrap().hand[0]).unwrap().name, partner);
                }
            }
        }
    }
}

#[test]
fn partner_combat_runs_only_on_controllers_turn_and_each_win_gets_its_own_trigger() {
    for name in [
        "Okaun, Eye of Chaos", "Okaun, Eye of Chaos // Okaun, Eye of Chaos",
        "Zndrsplt, Eye of Wisdom", "Zndrsplt, Eye of Wisdom // Zndrsplt, Eye of Wisdom",
    ] {
        for definition in definitions(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            library(&mut g, A, 8);
            let mut dm = Choices::default();
            assert_eq!(begin_combat(&mut g, B, &mut dm), 0);
            force(&mut g, &[H, H, T]);
            assert_eq!(begin_combat(&mut g, A, &mut dm), 1);
            resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(stack(&mut g, vec![], &mut dm), 2,
                "the terminal loss is not a win; equal-looking wins remain separate occurrences");
            settle(&mut g, &mut dm);
            if name.starts_with("Okaun") {
                assert_eq!(g.current_power(source), Some(12));
                assert_eq!(g.current_toughness(source), Some(12));
            } else {
                assert_eq!(g.player(A).unwrap().hand.len(), 2);
            }
            // "A player" also observes an opponent's called win, but the
            // observer controller still receives the draw or P/T modification.
            force(&mut g, &[H]);
            let out = flip(&mut g, source, B, 1, false, false, &mut dm);
            assert_eq!(stack(&mut g, out.events, &mut dm), 1);
            settle(&mut g, &mut dm);
            if name.starts_with("Okaun") {
                assert_eq!(g.current_power(source), Some(24));
                assert_eq!(g.current_toughness(source), Some(24));
                ironsmith::turn::execute_cleanup_step(&mut g);
                assert_eq!(g.current_power(source), Some(3));
                assert_eq!(g.current_toughness(source), Some(3));
            } else {
                assert_eq!(g.player(A).unwrap().hand.len(), 3);
                assert!(g.player(B).unwrap().hand.is_empty());
            }
        }
    }
}

#[test]
fn grouped_receipt_preserves_called_outcomes_faces_players_ordinals_and_event_identity() {
    let mut g = game();
    let source = object(&mut g, B, Zone::Battlefield, "Foreign producer", "Type: Artifact");
    let mut dm = Choices::default();
    force(&mut g, &[H, T, H]);
    let out = flip(&mut g, source, A, 3, false, false, &mut dm);
    let receipt = out.coin_flip_results().unwrap();
    assert_eq!(receipt.len(), 3);
    assert_eq!(out.as_count(), Some(2));
    assert_eq!(dm.option_players, vec![A; 3], "the actual flipper calls, even for a foreign source");
    let mut provenance = std::collections::HashSet::new();
    let simultaneous_batch = out.events[0].simultaneous_batch();
    assert!(simultaneous_batch.is_some());
    for (index, (result, event)) in receipt.iter().zip(&out.events).enumerate() {
        let payload = event.downcast::<ironsmith::events::CoinFlippedEvent>().unwrap();
        assert_eq!(result.player, A);
        assert_eq!(result.face, [H, T, H][index]);
        assert_eq!(result.call, Some(H));
        assert_eq!(result.winner, (index != 1).then_some(A));
        assert_eq!(result.loser, (index == 1).then_some(A));
        assert_eq!(result.turn_ordinal, index as u32 + 1);
        assert_eq!(result.instruction_ordinal, index as u32 + 1);
        assert_eq!(payload.player, result.player);
        assert_eq!(payload.turn_ordinal, result.turn_ordinal);
        assert_eq!(payload.instruction_ordinal, result.instruction_ordinal);
        assert_eq!(event.simultaneous_batch(), simultaneous_batch);
        assert!(provenance.insert(event.provenance()), "each coin needs its own event provenance");
    }
    force(&mut g, &[T, H]);
    let face_only = flip(&mut g, source, A, 2, true, false, &mut dm);
    assert_eq!(dm.option_players.len(), 3);
    for (index, result) in face_only.coin_flip_results().unwrap().iter().enumerate() {
        assert_eq!(result.turn_ordinal, index as u32 + 4);
        assert_eq!(result.instruction_ordinal, index as u32 + 1);
        assert_eq!((result.call, result.winner, result.loser), (None, None, None));
    }
    force(&mut g, &[T]);
    let other = flip(&mut g, source, B, 1, false, false, &mut dm);
    assert_eq!(other.coin_flip_results().unwrap()[0].turn_ordinal, 1);
    g.turn_store.turn_history.clear_for_new_turn();
    g.turn.turn_number += 1;
    force(&mut g, &[H]);
    let next_turn = flip(&mut g, source, A, 1, false, false, &mut dm);
    assert_eq!(next_turn.coin_flip_results().unwrap()[0].turn_ordinal, 1);
}

#[test]
fn you_win_observer_uses_its_current_controller_and_never_face_only_heads() {
    for definition in program_definitions(
        "Own win observer", "Type: Artifact\nWhenever you win a coin flip, draw a card.",
    ) {
        let mut g = game();
        let observer = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = object(&mut g, B, Zone::Battlefield, "Foreign coin source", "Type: Artifact");
        library(&mut g, A, 4);
        library(&mut g, B, 4);
        let mut dm = Choices::default();
        for (player, face_only, triggers) in [(A, false, 2), (B, false, 0), (A, true, 0)] {
            force(&mut g, &[H, H]);
            let out = flip(&mut g, source, player, 2, face_only, false, &mut dm);
            assert_eq!(stack(&mut g, out.events, &mut dm), triggers);
            settle(&mut g, &mut dm);
        }
        assert_eq!(g.player(A).unwrap().hand.len(), 2);
        ironsmith::effects::GainControlEffect::new(ChooseSpec::SpecificObject(observer), Until::EndOfTurn)
            .execute(&mut g, &mut ExecutionContext::new(source, B, &mut dm)).unwrap();
        force(&mut g, &[H]);
        let out = flip(&mut g, source, B, 1, false, false, &mut dm);
        assert_eq!(stack(&mut g, out.events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(B).unwrap().hand.len(), 1);
    }
}

#[test]
fn incomplete_group_rolls_back_all_coins_and_replays_same_faces_after_native_restore() {
    for repeat in [false, true] {
        for pause in [1, 2, 3] {
            let mut g = game();
            let source = object(&mut g, A, Zone::Battlefield, "Pending producer", "Type: Artifact");
            force(&mut g, &[H, H, T]);
            let saved = g.clone();
            let random = g.irreversible_random_count();
            let ui_events = g.ui_effect_events().count();
            let mut dm = Choices { pause_on_option: Some(pause), ..Default::default() };
            let out = flip(&mut g, source, A, if repeat { 1 } else { 3 }, false, repeat, &mut dm);
            assert!(dm.pending);
            assert!(out.coin_flip_results().is_none());
            assert!(out.events.is_empty());
            assert!(out.execution_facts.is_empty());
            assert_eq!(g.irreversible_random_count(), random);
            assert_eq!(g.ui_effect_events().count(), ui_events);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
            assert!(g.take_pending_trigger_events().is_empty());
            dm.pending = false;
            dm.pause_on_option = None;
            let completed = flip(&mut g, source, A, if repeat { 1 } else { 3 }, false, repeat, &mut dm);
            let receipt = completed.coin_flip_results().unwrap().to_vec();
            assert_eq!(receipt.iter().map(|coin| coin.face).collect::<Vec<_>>(), [H, H, T]);
            assert_eq!(receipt.iter().map(|coin| coin.turn_ordinal).collect::<Vec<_>>(), [1, 2, 3]);
            assert_eq!(completed.as_count(), Some(2));
            let batches: std::collections::HashSet<_> = completed.events.iter()
                .map(|event| event.simultaneous_batch().unwrap()).collect();
            assert_eq!(batches.len(), if repeat { 3 } else { 1 },
                "repeated one-coin instructions remain separate simultaneous batches");
            g = saved;
            let restored = flip(&mut g, source, A, if repeat { 1 } else { 3 }, false, repeat, &mut dm);
            assert_eq!(restored.coin_flip_results().unwrap(), receipt.as_slice());
        }
    }
}

#[test]
fn typed_ordinal_failure_rolls_back_a_partial_repeat_and_keeps_forced_faces_for_retry() {
    use ironsmith::effects::ExecutionError;
    let mut g = game();
    let source = object(&mut g, A, Zone::Battlefield, "Capacity producer", "Type: Artifact");
    g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, i32::MAX as u32 - 1);
    force(&mut g, &[H, T]);
    let random = g.irreversible_random_count();
    let ui_events = g.ui_effect_events().count();
    let mut effect = FlipCoinEffect::new(PlayerFilter::You);
    effect.repeat_until_loss = true;
    let mut dm = Choices::default();
    let result = effect.execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm));
    assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded { .. })));
    assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), i32::MAX as u32 - 1);
    assert_eq!(g.irreversible_random_count(), random);
    assert_eq!(g.ui_effect_events().count(), ui_events);
    assert!(g.take_pending_trigger_events().is_empty());
    g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, 0);
    let completed = effect.execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm)).unwrap();
    assert_eq!(completed.coin_flip_results().unwrap().iter().map(|coin| coin.face).collect::<Vec<_>>(), [H, T]);
    assert_eq!(completed.as_count(), Some(1));
}

#[test]
fn thumb_ignored_coins_do_not_enter_receipts_history_or_native_win_triggers() {
    for definition in program_definitions(
        "Krark's Thumb",
        "Mana cost: {2}\nType: Legendary Artifact\nIf you would flip a coin, instead flip two coins and ignore one.",
    ) {
        let mut g = game();
        g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let producer = object(&mut g, B, Zone::Battlefield, "Foreign producer", "Type: Artifact");
        object(&mut g, A, Zone::Battlefield, "Retained win observer", "Type: Artifact\nWhenever you win a coin flip, draw a card.");
        library(&mut g, A, 4);
        // Choices calls heads and keeps the first of each pair. The ignored
        // second heads result must never become a second win trigger.
        force(&mut g, &[H, T, T, H]);
        let mut dm = Choices::default();
        let out = flip(&mut g, producer, A, 2, false, false, &mut dm);
        assert_eq!(out.coin_flip_results().unwrap().iter().map(|coin| coin.face).collect::<Vec<_>>(), [H, T]);
        assert_eq!(out.events.len(), 2);
        assert_eq!(out.as_count(), Some(1));
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 2);
        assert_eq!(stack(&mut g, out.events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 1);
        // The modifier is controlled by A; B flips only the authored coin.
        force(&mut g, &[H]);
        let out = flip(&mut g, producer, B, 1, false, false, &mut dm);
        assert_eq!(out.coin_flip_results().unwrap().len(), 1);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 1);
    }
}

#[test]
fn thumb_choice_after_physical_coins_suspends_without_publishing_ignored_or_kept_results() {
    for definition in program_definitions(
        "Krark's Thumb",
        "Mana cost: {2}\nType: Legendary Artifact\nIf you would flip a coin, instead flip two coins and ignore one.",
    ) {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        force(&mut g, &[H, T]);
        let random = g.irreversible_random_count();
        let mut dm = Choices { pause_on_option: Some(3), ..Default::default() };
        let out = flip(&mut g, source, A, 1, false, false, &mut dm);
        assert!(dm.pending);
        assert!(out.events.is_empty());
        assert!(out.coin_flip_results().is_none());
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
        assert_eq!(g.irreversible_random_count(), random);
        dm.pending = false;
        dm.pause_on_option = None;
        let out = flip(&mut g, source, A, 1, false, false, &mut dm);
        assert_eq!(out.coin_flip_results().unwrap()[0].face, H);
        assert_eq!(out.coin_flip_results().unwrap()[0].turn_ordinal, 1);
        assert_eq!(out.events.len(), 1);
    }
}

#[test]
fn edgar_first_fixed_batch_and_repeated_single_batches_have_different_boundaries() {
    for definition in program_definitions(
        "Edgar, King of Figaro",
        "Mana cost: {4}{U}{U}\nType: Legendary Creature — Human Artificer Noble\nPower/Toughness: 4/5\nWhen Edgar enters, draw a card for each artifact you control.\nTwo-Headed Coin — The first time you flip one or more coins each turn, those coins come up heads and you win those flips.",
    ) {
        for repeat in [false, true] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut dm = Choices::default();
            force(&mut g, if repeat { &[T, T] } else { &[T, T, T] });
            let out = flip(&mut g, source, A, if repeat { 1 } else { 3 }, false, repeat, &mut dm);
            let receipt = out.coin_flip_results().unwrap();
            if repeat {
                assert_eq!(receipt.iter().map(|coin| coin.face).collect::<Vec<_>>(), [H, T]);
                assert_eq!(out.as_count(), Some(1));
                assert_eq!(receipt[1].loser, Some(A));
            } else {
                assert_eq!(receipt.len(), 3);
                assert!(receipt.iter().all(|coin| coin.face == H && coin.winner == Some(A)));
                assert_eq!(out.as_count(), Some(3));
            }
            force(&mut g, &[T]);
            let later = flip(&mut g, source, A, 1, false, false, &mut dm);
            assert_eq!(later.coin_flip_results().unwrap()[0].loser, Some(A));
            g.turn_store.turn_history.clear_for_new_turn();
            g.turn.turn_number += 1;
            force(&mut g, &[T]);
            let next_turn = flip(&mut g, source, A, 1, true, false, &mut dm);
            let coin = next_turn.coin_flip_results().unwrap()[0];
            assert_eq!(coin.call, None);
            assert_eq!(coin.face, H);
            assert_eq!(coin.winner, Some(A), "Edgar explicitly awards the face-only win");
        }
    }
}

#[test]
fn pending_native_firecat_trigger_cannot_publish_partial_counters_or_win_triggers() {
    for definition in definitions("Crazed Firecat") {
        let mut g = game();
        let card = g.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = g.object(card).unwrap().stable_id;
        object(&mut g, A, Zone::Battlefield, "Win observer", "Type: Artifact\nWhenever you win a coin flip, draw a card.");
        library(&mut g, A, 4);
        let mut dm = Choices::default();
        cast(&mut g, card, &mut dm);
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(stack(&mut g, vec![], &mut dm), 1);
        let cat = g.find_object_by_stable_id(stable).unwrap();
        force(&mut g, &[H, H, T]);
        let saved = g.clone();
        dm.option_players.clear();
        dm.pause_on_option = Some(2);
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(g.stack.len(), 1, "the original ETB trigger stays on the stack");
        assert_eq!(g.counter_count(cat, CounterType::PlusOnePlusOne), 0);
        assert!(g.player(A).unwrap().hand.is_empty());
        assert!(g.take_pending_trigger_events().is_empty());
        for restored in [false, true] {
            if restored { g = saved.clone(); }
            dm.pending = false;
            dm.pause_on_option = None;
            settle(&mut g, &mut dm);
            assert_eq!(g.counter_count(cat, CounterType::PlusOnePlusOne), 2);
            assert_eq!(g.player(A).unwrap().hand.len(), 2);
        }
    }
}

#[test]
fn sequential_instruction_counts_do_not_leak_across_a_later_flip_or_unrelated_effect() {
    for definition in program_definitions(
        "Separate coin instructions",
        "Mana cost: {0}\nType: Sorcery\nFlip two coins. For each flip you win, you gain 1 life. Flip three coins. You gain 1 life. For each flip you win, draw a card.",
    ) {
        for faces in [[H, H, T, T, T], [T, T, H, T, H]] {
            let mut g = game();
            let card = g.create_object_from_definition(&definition, A, Zone::Hand);
            library(&mut g, A, 5);
            let mut dm = Choices::default();
            force(&mut g, &faces);
            cast(&mut g, card, &mut dm);
            settle(&mut g, &mut dm);
            let first_wins = faces[..2].iter().filter(|face| **face == H).count();
            let second_wins = faces[2..].iter().filter(|face| **face == H).count();
            assert_eq!(g.player(A).unwrap().life, 31 + first_wins as i32);
            assert_eq!(g.player(A).unwrap().hand.len(), second_wins);
        }
    }
}

#[test]
fn optional_group_decline_has_no_coin_outcome_and_cannot_reuse_an_earlier_receipt() {
    for definition in program_definitions(
        "Optional coin instruction",
        "Mana cost: {0}\nType: Sorcery\nFlip two coins. You gain 1 life. You may flip two coins. For each flip you win, draw a card.",
    ) {
        for accept in [false, true] {
            let mut g = game();
            let card = g.create_object_from_definition(&definition, A, Zone::Hand);
            library(&mut g, A, 4);
            let mut dm = Choices { accept, ..Default::default() };
            force(&mut g, &[H, H, H, T]);
            cast(&mut g, card, &mut dm);
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().life, 31);
            assert_eq!(g.player(A).unwrap().hand.len(), usize::from(accept));
            assert_eq!(dm.option_players.len(), if accept { 4 } else { 2 });
        }
    }
}

#[test]
fn unbound_grouped_coin_consumers_fail_closed() {
    for text in [
        "Type: Sorcery\nFlip that many coins.",
        "Type: Sorcery\nFor each flip you win, draw a card.",
        "Type: Sorcery\nFor each coin that comes up heads, you gain 1 life.",
        "Type: Sorcery\nIf both coins come up heads, draw a card.",
        "Type: Artifact\nWhenever you gain life, for each flip you won, draw a card.",
    ] {
        assert!(compile_to_artifact("Unbound coin receipt", text, false).is_err(), "{text}");
    }
}

#[test]
fn duplicate_thumb_occurrences_multiply_and_phasing_and_control_change_the_affected_player() {
    for definition in program_definitions(
        "Two independent coin replacements",
        "Type: Artifact\nIf you would flip a coin, instead flip two coins and ignore one.\nIf you would flip a coin, instead flip two coins and ignore one.",
    ) {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices::default();
        let before = g.irreversible_random_count();
        force(&mut g, &[H, T, T, H]);
        let out = flip(&mut g, source, A, 1, true, false, &mut dm);
        assert_eq!(g.irreversible_random_count(), before + 4);
        assert_eq!(out.coin_flip_results().unwrap().len(), 1);
        assert_eq!(out.coin_flip_results().unwrap()[0].face, H);
        assert_eq!(out.events.len(), 1);
        assert_eq!(dm.option_players, [A, A, A], "two nested keep choices and one outer keep choice");

        g.phase_out(source);
        let before = g.irreversible_random_count();
        force(&mut g, &[T]);
        let out = flip(&mut g, source, A, 1, true, false, &mut dm);
        assert_eq!(out.coin_flip_results().unwrap()[0].face, T);
        assert_eq!(g.irreversible_random_count(), before + 1);
        g.phase_in(source);
        ironsmith::effects::GainControlEffect::new(ChooseSpec::SpecificObject(source), Until::EndOfTurn)
            .execute(&mut g, &mut ExecutionContext::new_default(source, B)).unwrap();
        let before = g.irreversible_random_count();
        force(&mut g, &[H, H, H, H]);
        flip(&mut g, source, B, 1, true, false, &mut dm);
        assert_eq!(g.irreversible_random_count(), before + 4);
        let before = g.irreversible_random_count();
        force(&mut g, &[T]);
        flip(&mut g, source, A, 1, true, false, &mut dm);
        assert_eq!(g.irreversible_random_count(), before + 1);
    }
}

#[test]
fn a_known_empty_coin_receipt_is_zero_but_missing_or_unrelated_receipts_are_typed_errors() {
    use ironsmith::effect::{EffectId, EffectMetric, EffectMetricSource, Value};
    use ironsmith::effects::{ExecutionError, helpers::resolve_value};
    let mut g = game();
    let source = g.new_object_id();
    let value = Value::EffectMetric {
        effect_id: EffectId(0), source: EffectMetricSource::Outcome, metric: EffectMetric::CoinFlipsWon,
    };
    let mut dm = Choices::default();
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    assert!(matches!(resolve_value(&g, &value, &ctx), Err(ExecutionError::IncompleteEvidence(_))));
    ctx.store_outcome(EffectId(0), EffectOutcome::count(7));
    assert!(matches!(resolve_value(&g, &value, &ctx), Err(ExecutionError::IncompleteEvidence(_))));
    let mut empty = FlipCoinEffect::new(PlayerFilter::You);
    empty.count = 0;
    let out = empty.execute(&mut g, &mut ctx).unwrap();
    assert_eq!(out.coin_flip_results().unwrap(), &[]);
    ctx.store_outcome(EffectId(0), out);
    assert_eq!(resolve_value(&g, &value, &ctx).unwrap(), 0);
}

#[test]
fn live_readers_reject_unconsumed_symbols_and_punctuation_in_coin_clauses() {
    for text in [
        "Flip two coins {R}.",
        "Flip a coin until you lose: a flip.",
        "Flip two coins. For each flip {R} you win, draw a card.",
        "Flip two coins. If both coins come {R} up heads, draw a card.",
        "Flip two coins. Put a +1/+1 counter on this creature for each flip you {R} won.",
    ] {
        for full in [format!("Type: Sorcery\n{text}"), format!("Type: Creature\nPower/Toughness: 1/1\nWhen this creature enters, {text}")] {
            assert!(compile_to_artifact("Malformed coin instruction", &full, false).is_err(), "artifact: {full}");
            assert!(compile_to_runtime_definition("Malformed coin instruction", &full, false).is_err(), "direct: {full}");
        }
    }
}

#[test]
fn one_instruction_can_count_wins_and_heads_without_conflating_them() {
    for definition in program_definitions(
        "Called heads and wins",
        "Mana cost: {0}\nType: Sorcery\nFlip two coins. For each flip you win, you gain 1 life. You gain 1 life for each coin that comes up heads.",
    ) {
        let mut g = game();
        let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
        force(&mut g, &[T, T]);
        let mut dm = Choices { option: 1, ..Default::default() };
        cast(&mut g, spell, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(dm.option_players, [A, A]);
        assert_eq!(g.player(A).unwrap().life, 32, "two tails wins and zero heads");
    }
}

fn followup_definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/grouped_coin_followups.json.fixture")).unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    program_definitions(name, row["text"].as_str().unwrap())
}

#[test]
fn mirror_march_copies_exact_entry_lki_grants_noncopiable_haste_and_exiles_only_its_created_group() {
    for definition in followup_definitions("Mirror March") {
        for leaves in [false, true] {
            for wins in [0usize, 2] {
                let mut g = game();
                let mirror = g.create_object_from_definition(&definition, A, Zone::Battlefield);
                let card = object(&mut g, A, Zone::Hand, "March original", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 2/3\nVigilance");
                let stable = g.object(card).unwrap().stable_id;
                let mut dm = Choices::default();
                cast(&mut g, card, &mut dm);
                resolve_stack_entry_with(&mut g, &mut dm).unwrap();
                assert_eq!(stack(&mut g, vec![], &mut dm), 1);
                let original = g.find_object_by_stable_id(stable).unwrap();
                if leaves {
                    g.move_object_by_effect(original, Zone::Graveyard).unwrap();
                    g.move_object_by_effect(mirror, Zone::Graveyard).unwrap();
                }
                force(&mut g, &if wins == 0 { vec![T] } else { vec![H, H, T] });
                settle(&mut g, &mut dm);
                let tokens = g.battlefield.iter().copied().filter(|id| {
                    g.object(*id).is_some_and(|object| object.kind == ironsmith::object::ObjectKind::Token && object.name == "March original")
                }).collect::<Vec<_>>();
                assert_eq!(tokens.len(), wins);
                for &token in &tokens {
                    assert_eq!(g.current_power(token), Some(2));
                    assert_eq!(g.current_toughness(token), Some(3));
                    assert!(g.current_has_static_ability_id(token, StaticAbilityId::Vigilance));
                    assert!(g.current_has_static_ability_id(token, StaticAbilityId::Haste));
                }
                assert!(g.stack.is_empty(), "nontoken restriction must prevent recursive Mirror triggers");
                let recopy = tokens.first().map(|token| {
                    let outcome = ironsmith::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(*token))
                        .execute(&mut g, &mut ExecutionContext::new(mirror, A, &mut dm)).unwrap();
                    let copied = outcome.explicit_objects().unwrap()[0];
                    assert!(!g.current_has_static_ability_id(copied, StaticAbilityId::Haste), "a later copy does not inherit a separate haste grant");
                    copied
                });
                if let Some(&token) = tokens.first() {
                    ironsmith::effects::GainControlEffect::new(ChooseSpec::SpecificObject(token), Until::Forever)
                        .execute(&mut g, &mut ExecutionContext::new_default(mirror, B)).unwrap();
                }
                g.turn.phase = ironsmith::Phase::Ending;
                g.turn.step = Some(ironsmith::game_state::Step::End);
                let event = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfEndStepEvent::new(A), Default::default());
                stack(&mut g, vec![event], &mut dm);
                settle(&mut g, &mut dm);
                assert!(tokens.iter().all(|token| !g.battlefield.contains(token)));
                if let Some(copied) = recopy { assert!(g.battlefield.contains(&copied), "the extra copy was never part of Mirror's cleanup group"); }
                if !leaves { assert!(g.battlefield.contains(&original)); }
            }
        }
    }
}

fn multiplayer_game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into(), "Dan".into()], 30);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.priority_player = Some(A);
    for symbol in [ironsmith::ManaSymbol::Red, ironsmith::ManaSymbol::Blue, ironsmith::ManaSymbol::Colorless] {
        g.player_mut(A).unwrap().mana_pool.add(symbol, 30);
    }
    g
}

#[test]
fn mutalith_retains_each_opponents_coin_and_damages_only_the_losing_associations() {
    for definition in followup_definitions("Mutalith Vortex Beast") {
        for departed in [false, true] {
            let mut g = multiplayer_game();
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(spell).unwrap().stable_id;
            library(&mut g, A, 5);
            let mut dm = Choices::default();
            cast(&mut g, spell, &mut dm);
            resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(stack(&mut g, vec![], &mut dm), 1);
            let source = g.find_object_by_stable_id(stable).unwrap();
            assert!(g.current_has_static_ability_id(source, StaticAbilityId::Trample));
            if departed { g.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            force(&mut g, &[H, T, H]);
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().hand.len(), 2);
            assert_eq!(g.player(A).unwrap().life, 30);
            assert_eq!(g.player(B).unwrap().life, 30);
            assert_eq!(g.player(PlayerId(2)).unwrap().life, 27);
            assert_eq!(g.player(PlayerId(3)).unwrap().life, 30);
            assert_eq!(dm.option_players, [A, A, A], "the controller calls every coin; the associated opponents do not");
        }
    }
}

#[test]
fn opponent_batch_uses_actual_flipper_and_publishes_empty_or_exact_rosters_atomically() {
    let mut g = multiplayer_game();
    let source = object(&mut g, B, Zone::Battlefield, "Foreign opponent batch", "Type: Artifact");
    let tags = ironsmith_core::CoinFlipOpponentTags { won: "won_opponents".into(), lost: "lost_opponents".into() };
    let mut effect = FlipCoinEffect::new(PlayerFilter::Specific(A));
    effect.opponent_results = Some(tags.clone());
    force(&mut g, &[H, T, H]);
    let mut dm = Choices { pause_on_option: Some(2), ..Default::default() };
    let mut ctx = ExecutionContext::new(source, B, &mut dm);
    let pending = effect.execute(&mut g, &mut ctx).unwrap();
    assert!(pending.coin_flip_results().is_none());
    assert!(ctx.get_tagged_players(tags.won.as_str()).is_none());
    assert!(ctx.get_tagged_players(tags.lost.as_str()).is_none());
    drop(ctx);
    dm.pending = false;
    dm.pause_on_option = None;
    let mut ctx = ExecutionContext::new(source, B, &mut dm);
    let out = effect.execute(&mut g, &mut ctx).unwrap();
    assert_eq!(out.coin_flip_results().unwrap().iter().map(|flip| (flip.player, flip.associated_player)).collect::<Vec<_>>(),
        [(A, Some(B)), (A, Some(PlayerId(2))), (A, Some(PlayerId(3)))]);
    assert_eq!(ctx.get_tagged_players(tags.won.as_str()).unwrap(), &[B, PlayerId(3)]);
    assert_eq!(ctx.get_tagged_players(tags.lost.as_str()).unwrap(), &[PlayerId(2)]);
    force(&mut g, &[H, H, H]);
    effect.execute(&mut g, &mut ctx).unwrap();
    assert!(ctx.get_tagged_players(tags.lost.as_str()).unwrap().is_empty(), "a later all-win instruction replaces the losing roster");
}

#[test]
fn mirror_haste_occurs_after_entry_observation_for_original_and_replacement_added_tokens() {
    for definition in followup_definitions("Mirror March") {
        let mut g = game();
        let mirror = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        object(&mut g, A, Zone::Battlefield, "Additional Frog",
            "Type: Artifact\nIf one or more tokens would be created under your control, those tokens plus a 1/1 green Frog creature token are created instead.");
        let observer = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Haste entry observer")
            .card_types(vec![ironsmith::CardType::Enchantment])
            .with_ability(ironsmith::ability::Ability::triggered(
                ironsmith::triggers::Trigger::enters_battlefield(
                    ironsmith::target::ObjectFilter::creature().you_control().with_static_ability(StaticAbilityId::Haste), None),
                vec![ironsmith::effect::Effect::gain_life(7)],
            )).build();
        g.create_object_from_definition(&observer, A, Zone::Battlefield);
        let card = object(&mut g, A, Zone::Hand, "Unhurried original", "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2");
        let mut dm = Choices::default();
        force(&mut g, &[H, T]);
        cast(&mut g, card, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 30, "neither original nor added token entered with haste");
        let tokens = g.battlefield.iter().copied().filter(|id| g.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token).collect::<Vec<_>>();
        assert_eq!(tokens.len(), 2);
        assert!(tokens.iter().all(|id| g.current_has_static_ability_id(*id, StaticAbilityId::Haste)));
        // The supported inline copy exception remains copiable and qualifies
        // at entry, unlike Mirror's later grant.
        let mut inline = ironsmith::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(tokens[0]));
        inline.has_haste = true;
        let out = inline.execute(&mut g, &mut ExecutionContext::new(mirror, A, &mut dm)).unwrap();
        let inline_token = out.result_objects().unwrap()[0];
        stack(&mut g, out.events, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 37, "only the original inline-exception token enters with haste");
        let out = ironsmith::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(inline_token))
            .execute(&mut g, &mut ExecutionContext::new(mirror, A, &mut dm)).unwrap();
        let copied = out.result_objects().unwrap()[0];
        assert!(g.current_has_static_ability_id(copied, StaticAbilityId::Haste), "inline haste survives a subsequent copy");
    }
}

fn free_hand_action(g: &GameState, player: PlayerId, spell: ObjectId) -> Option<LegalAction> {
    ironsmith::decision::compute_legal_actions(g, player).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { spell_id, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::PlayFrom { use_alternative: Some(_), .. } }
            if *spell_id == spell))
}

#[test]
fn yusri_chosen_count_owns_wins_losses_and_only_five_wins_grant_temporary_free_hand_casts() {
    for definition in followup_definitions("Yusri, Fortune's Flame") {
        for faces in [vec![H], vec![H, T, H], vec![H, H, H, H, T], vec![H; 5]] {
            let mut g = game();
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(spell).unwrap().stable_id;
            library(&mut g, A, 8);
            let mut dm = Choices { number: Some(faces.len() as u32), ..Default::default() };
            cast(&mut g, spell, &mut dm);
            settle(&mut g, &mut dm);
            let source = g.find_object_by_stable_id(stable).unwrap();
            assert!(g.current_has_static_ability_id(source, StaticAbilityId::Flying));
            force(&mut g, &faces);
            attack(&mut g, source, &mut dm);
            assert_eq!(g.stack.len(), 1);
            settle(&mut g, &mut dm);
            let wins = faces.iter().filter(|face| **face == H).count();
            let losses = faces.len() - wins;
            assert_eq!(dm.option_players.len(), faces.len());
            assert_eq!(g.player(A).unwrap().hand.len(), wins);
            assert_eq!(g.player(A).unwrap().life, 30 - losses as i32 * 2);
            g.player_mut(A).unwrap().mana_pool = Default::default();
            let expensive = object(&mut g, A, Zone::Hand, "Later free instant", "Mana cost: {6}{B}\nType: Instant\nYou gain 1 life.");
            assert_eq!(free_hand_action(&g, A, expensive).is_some(), wins == 5);
            let sorcery = object(&mut g, A, Zone::Hand, "Timing still applies", "Mana cost: {6}{B}\nType: Sorcery\nYou gain 1 life.");
            assert!(free_hand_action(&g, A, sorcery).is_none(), "Yusri does not grant flash during combat");
            if wins == 5 {
                g.move_object_by_effect(source, Zone::Graveyard).unwrap();
                let free = free_hand_action(&g, A, expensive).unwrap();
                action(&mut g, A, free, &mut dm);
                settle(&mut g, &mut dm);
                assert_eq!(g.player(A).unwrap().life, 31);
                let additional = object(&mut g, A, Zone::Hand, "Required additional payment", "Mana cost: {6}{B}\nType: Instant\nAs an additional cost to cast this spell, pay 2 life.\nYou gain 1 life.");
                let free = free_hand_action(&g, A, additional).unwrap();
                action(&mut g, A, free, &mut dm);
                settle(&mut g, &mut dm);
                assert_eq!(g.player(A).unwrap().life, 30, "the free alternative does not waive an additional cost");
                let other = object(&mut g, B, Zone::Hand, "Opponent has no grant", "Mana cost: {6}{B}\nType: Instant\nYou gain 1 life.");
                assert!(free_hand_action(&g, B, other).is_none());
                g.turn.phase = ironsmith::Phase::NextMain;
                g.turn.step = None;
                assert!(free_hand_action(&g, A, sorcery).is_some());
                ironsmith::turn::execute_cleanup_step(&mut g);
                g.turn.phase = ironsmith::Phase::FirstMain;
                g.turn.step = None;
                assert!(free_hand_action(&g, A, sorcery).is_none(), "the resolving permission expires at cleanup");
            }
        }
    }
}

#[test]
fn yusri_pending_numeric_choice_publishes_no_flip_or_permission_and_can_resume() {
    for definition in followup_definitions("Yusri, Fortune's Flame") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        library(&mut g, A, 8);
        let mut dm = Choices { number: Some(5), pause_on_number: true, ..Default::default() };
        force(&mut g, &[H; 5]);
        attack(&mut g, source, &mut dm);
        let before = g.irreversible_random_count();
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(g.stack.len(), 1);
        assert_eq!(g.irreversible_random_count(), before);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
        assert!(g.player(A).unwrap().hand.is_empty());
        assert!(g.effect_store.grant_registry.grants.is_empty());
        dm.pending = false;
        dm.pause_on_number = false;
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 5);
        assert_eq!(dm.option_players.len(), 5);
    }
}

fn modifier_definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/grouped_coin_modifiers.json.fixture")).unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    program_definitions(name, row["text"].as_str().unwrap())
}

#[test]
fn both_frozen_thumb_entries_are_paid_artifacts_whose_ignored_coins_never_complete() {
    for name in ["Krark's Thumb", "Krark's Thumb // Krark's Thumb"] {
        for definition in modifier_definitions(name) {
            let mut g = game();
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(spell).unwrap().stable_id;
            let mut dm = Choices::default();
            cast(&mut g, spell, &mut dm);
            settle(&mut g, &mut dm);
            let source = g.find_object_by_stable_id(stable).unwrap();
            force(&mut g, &[H, T, T, H]);
            let out = flip(&mut g, source, A, 2, false, false, &mut dm);
            assert_eq!(out.coin_flip_results().unwrap().iter().map(|result| result.face).collect::<Vec<_>>(), [H, T]);
            assert_eq!(out.as_count(), Some(1));
            assert_eq!(out.events.len(), 2);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 2);
            g.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let before = g.irreversible_random_count();
            force(&mut g, &[T]);
            let out = flip(&mut g, source, A, 1, false, false, &mut dm);
            assert_eq!(out.coin_flip_results().unwrap().len(), 1);
            assert_eq!(g.irreversible_random_count(), before + 1);
        }
    }
}

#[test]
fn frozen_edgar_draws_for_current_controlled_artifacts_and_its_batch_rule_needs_the_live_source() {
    for definition in modifier_definitions("Edgar, King of Figaro") {
        for departed in [false, true] {
            let mut g = game();
            object(&mut g, A, Zone::Battlefield, "Controlled artifact one", "Type: Artifact");
            object(&mut g, A, Zone::Battlefield, "Controlled artifact two", "Type: Artifact Creature\nPower/Toughness: 1/1");
            object(&mut g, B, Zone::Battlefield, "Opponent artifact", "Type: Artifact");
            object(&mut g, A, Zone::Battlefield, "Not an artifact", "Type: Enchantment");
            library(&mut g, A, 7);
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(spell).unwrap().stable_id;
            let mut dm = Choices::default();
            cast(&mut g, spell, &mut dm);
            resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(stack(&mut g, vec![], &mut dm), 1);
            let source = g.find_object_by_stable_id(stable).unwrap();
            object(&mut g, A, Zone::Battlefield, "Artifact before resolution", "Type: Artifact");
            if departed { g.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().hand.len(), 3, "the ETB counts artifacts at resolution and excludes opponents");
            force(&mut g, &[T, T]);
            let out = flip(&mut g, source, A, 2, true, false, &mut dm);
            assert_eq!(out.coin_flip_results().unwrap().iter().map(|coin| (coin.face, coin.winner)).collect::<Vec<_>>(),
                if departed { vec![(T, None); 2] } else { vec![(H, Some(A)); 2] });
            assert!(dm.option_players.is_empty(), "Edgar gives face-only flips winners without requiring a call");
        }
    }
}

#[test]
fn required_coin_opponent_roster_distinguishes_missing_from_empty_and_rolls_back_partial_work() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{ExecutionError, ForEachTaggedPlayerEffect};
    let mut g = multiplayer_game();
    let source = g.new_object_id();
    let mut required = ForEachTaggedPlayerEffect::new("required_losses", vec![Effect::lose_life_player(2, PlayerFilter::IteratedPlayer)]);
    required.require_evidence = true;
    let mut dm = Choices::default();
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    assert!(matches!(required.execute(&mut g, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    ctx.set_tagged_players("required_losses", vec![]);
    assert_eq!(required.execute(&mut g, &mut ctx).unwrap().as_count(), Some(0));
    ctx.set_tagged_players("required_losses", vec![B, PlayerId(2)]);
    required.effects.push(Effect::new(ironsmith::effects::ChooseNumberEffect::new(PlayerFilter::You, 5, 1)));
    assert!(matches!(required.execute(&mut g, &mut ctx), Err(ExecutionError::Impossible(_))));
    assert_eq!(g.player(B).unwrap().life, 30);
    assert_eq!(g.player(PlayerId(2)).unwrap().life, 30);
    assert_eq!(ctx.iteration.iterated_player, None);
    assert_eq!(ctx.get_tagged_players("required_losses").unwrap(), &[B, PlayerId(2)]);
    drop(ctx);
    required.effects.pop();
    required.effects.push(Effect::flip_coin(PlayerFilter::IteratedPlayer));
    force(&mut g, &[H, T]);
    dm.pause_on_option = Some(2);
    let random = g.irreversible_random_count();
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    ctx.set_tagged_players("required_losses", vec![B, PlayerId(2)]);
    let pending = required.execute(&mut g, &mut ctx).unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert!(pending.events.is_empty());
    assert_eq!(g.player(B).unwrap().life, 30);
    assert_eq!(g.player(PlayerId(2)).unwrap().life, 30);
    assert_eq!(g.irreversible_random_count(), random);
    assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 0);
    assert_eq!(ctx.get_tagged_players("required_losses").unwrap(), &[B, PlayerId(2)]);
    drop(ctx);
    dm.pending = false;
    dm.pause_on_option = None;
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    let optional = ForEachTaggedPlayerEffect::new("unbound_optional_roster", vec![Effect::gain_life(9)]);
    assert_eq!(optional.execute(&mut g, &mut ctx).unwrap().as_count(), Some(0));
    assert_eq!(g.player(A).unwrap().life, 30);
}

#[test]
fn mutalith_counts_only_opponents_in_its_abilitys_range_of_influence() {
    for definition in followup_definitions("Mutalith Vortex Beast") {
        let mut g = multiplayer_game();
        g.enable_limited_range_of_influence(vec![A, B, PlayerId(2), PlayerId(3)], vec![1; 4]).unwrap();
        let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
        library(&mut g, A, 4);
        let mut dm = Choices::default();
        force(&mut g, &[H, T]);
        cast(&mut g, spell, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(dm.option_players, [A, A]);
        assert_eq!(g.player(A).unwrap().hand.len(), 1);
        assert_eq!(g.player(B).unwrap().life, 30);
        assert_eq!(g.player(PlayerId(2)).unwrap().life, 30, "the distant opponent supplies no coin");
        assert_eq!(g.player(PlayerId(3)).unwrap().life, 27);
    }
    let mut g = multiplayer_game();
    g.enable_limited_range_of_influence(vec![A, B, PlayerId(2), PlayerId(3)], vec![1; 4]).unwrap();
    let source = object(&mut g, B, Zone::Battlefield, "Different range controller", "Type: Artifact");
    let mut effect = FlipCoinEffect::new(PlayerFilter::Specific(A));
    effect.opponent_results = Some(ironsmith_core::CoinFlipOpponentTags { won: "range_wins".into(), lost: "range_losses".into() });
    force(&mut g, &[H, H]);
    let mut dm = Choices::default();
    let out = effect.execute(&mut g, &mut ExecutionContext::new(source, B, &mut dm)).unwrap();
    assert_eq!(out.coin_flip_results().unwrap().iter().map(|flip| flip.associated_player).collect::<Vec<_>>(),
        [Some(B), Some(PlayerId(2))], "opponent relation follows A while range follows the resolving controller B");
}

#[test]
fn chosen_coin_count_requires_exact_numeric_evidence_and_outer_failure_runs_no_followups() {
    use ironsmith::effect::{Effect, EffectId, ExecutionFact};
    use ironsmith::effects::{ExecutionError, SequenceEffect, execute_effect};
    let value = ironsmith_core::Value::PriorEffectMetric {
        effect_id: EffectId(19),
        query: ironsmith_core::PriorEffectMetricQuery::new(ironsmith_core::EffectMetricSource::Outcome, ironsmith_core::EffectMetric::Count)
            .with_action(ironsmith_core::PriorEffectAction::ChosenNumber),
    };
    let mut g = game();
    let source = object(&mut g, A, Zone::Battlefield, "Count receipt source", "Type: Artifact");
    library(&mut g, A, 5);
    let mut flip = FlipCoinEffect::new(PlayerFilter::You);
    flip.count_value = Some(value);
    let mut dm = Choices::default();
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    let random = g.irreversible_random_count();
    for outcome in [None, Some(EffectOutcome::count(5)), Some(EffectOutcome::count(5).with_execution_fact(ExecutionFact::ChosenNumber(4)))] {
        ctx.effect_outcomes.remove(&EffectId(19));
        if let Some(outcome) = outcome { ctx.store_outcome(EffectId(19), outcome); }
        assert!(matches!(flip.execute(&mut g, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
        let sequence = Effect::new(SequenceEffect::new(vec![Effect::gain_life(2), Effect::new(flip.clone()), Effect::draw(3)]));
        assert!(matches!(execute_effect(&mut g, &sequence, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
        assert_eq!(g.player(A).unwrap().life, 30);
        assert!(g.player(A).unwrap().hand.is_empty());
        assert_eq!(g.irreversible_random_count(), random);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
    }
    ctx.store_outcome(EffectId(19), EffectOutcome::count(0).with_execution_fact(ExecutionFact::ChosenNumber(0)));
    let empty = flip.execute(&mut g, &mut ctx).unwrap();
    assert!(empty.coin_flip_results().unwrap().is_empty(), "an authored completed choice of zero is valid evidence");
    ctx.store_outcome(EffectId(19), EffectOutcome::count(2).with_execution_fact(ExecutionFact::ChosenNumber(2)));
    force(&mut g, &[H, T]);
    let two = flip.execute(&mut g, &mut ctx).unwrap();
    assert_eq!(two.coin_flip_results().unwrap().len(), 2);
    assert_eq!(two.as_count(), Some(1), "the flip receipt does not mistake the chosen count for wins");
}

#[test]
fn required_player_iteration_shares_one_token_resource_transaction_across_participants() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{CreateTokenCopyEffect, ExecutionError, ForEachTaggedPlayerEffect};
    use ironsmith::effects::tokens::TokenCreationLimits;
    let mut g = multiplayer_game();
    let source = object(&mut g, A, Zone::Battlefield, "Roster transaction", "Type: Artifact");
    let model = object(&mut g, A, Zone::Battlefield, "Roster token model", "Type: Creature\nPower/Toughness: 1/1");
    let mut copy = CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(model));
    copy.controller = PlayerFilter::IteratedPlayer;
    let mut iteration = ForEachTaggedPlayerEffect::new("recipients", vec![Effect::new(copy)]);
    iteration.require_evidence = true;
    let before = g.battlefield.len();
    let mut dm = Choices::default();
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    ctx.set_tagged_players("recipients", vec![B, PlayerId(2)]);
    g.set_token_creation_limits(TokenCreationLimits { max_instructions: 1, ..Default::default() });
    let error = iteration.execute(&mut g, &mut ctx).unwrap_err();
    assert!(matches!(error, ExecutionError::ResourceLimitExceeded { resource: "token instruction work", .. }));
    assert_eq!(g.battlefield.len(), before, "the first participant's token is rolled back with the second failure");
    assert!(g.take_pending_trigger_events().is_empty());
    assert_eq!(ctx.iteration.iterated_player, None);
    g.set_token_creation_limits(TokenCreationLimits { max_instructions: 2, ..Default::default() });
    iteration.execute(&mut g, &mut ctx).unwrap();
    assert_eq!(g.battlefield.len(), before + 2);
    for recipient in [B, PlayerId(2)] {
        assert_eq!(g.battlefield.iter().filter(|id| g.object(**id).is_some_and(|object|
            object.kind == ironsmith::object::ObjectKind::Token && game_controller(&g, object.id) == recipient)).count(), 1);
    }
}

fn game_controller(g: &GameState, id: ObjectId) -> PlayerId {
    g.controller_of(g.object(id).unwrap())
}

#[path = "grouped_coin_flips/optional_loops.rs"]
mod optional_loops;
