//! UNRUN: complete frozen Oracle bodies, independently compiled directly and
//! through serialized/restored artifacts. No execution evidence is claimed.
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState, declare_blockers};
use ironsmith::decision::{
    AttackerDeclaration, BlockerDeclaration, DecisionMaker, LegalAction,
    SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    ManaPaymentContext, PartitionContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations,
    apply_decision_context_with_dm, apply_multiplayer_blocker_declarations,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/source_must_be_blocked.json.fixture"
    )).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = format!(
        "Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(),
        row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap(),
    );
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "{name}: {}", direct_loss.reasons_text());
    // Do not reuse the convenience runtime result returned alongside the
    // artifact: this is an independent compilation and a real wire decode.
    let (compiled, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!artifact_loss.is_lossy(), "{name}: {}", artifact_loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert_eq!(definition.card.name, name);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn object(game: &mut GameState, player: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Scenario resource", text, false).unwrap();
    let id = game.create_object_from_definition(&definition, player, zone);
    game.remove_summoning_sickness(id);
    id
}

fn creature(game: &mut GameState, player: PlayerId, body: &str) -> ObjectId {
    object(game, player, Zone::Battlefield,
        &format!("Type: Creature — Beast\nPower/Toughness: 5/5\n{body}"))
}

fn source(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

#[derive(Default)]
struct Choices {
    mode: usize,
    target: Option<ObjectId>,
    target_pools: Vec<Vec<Target>>,
    partitions: Vec<(PlayerId, usize)>,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.options.len() == 2 && ctx.options.iter().all(|option|
            option.description.to_ascii_lowercase().contains("blocked")) {
            let option = &ctx.options[self.mode];
            assert!(option.legal);
            vec![option.index]
        } else {
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext)
        -> ironsmith::mana_payment::ManaPaymentResponse
    {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash,
        }
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for requirement in &ctx.requirements {
            self.target_pools.push(requirement.legal_targets.clone());
        }
        if let Some(target) = self.target {
            assert_eq!(ctx.requirements.len(), 1);
            assert!(ctx.requirements[0].legal_targets.contains(&Target::Object(target)));
            vec![Target::Object(target)]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    fn decide_partition(&mut self, _: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
        self.partitions.push((ctx.player, ctx.cards.len()));
        Vec::new()
    }
}

fn activation(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source))
}

fn activate(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = activation(game, source).expect("printed activation should be legal");
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_activation.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    assert_eq!(game.stack.len(), 1);
}

fn give_activation_mana(game: &mut GameState, name: &str) -> u32 {
    let pool = &mut game.player_mut(A).unwrap().mana_pool;
    if name == "Anzrag, the Quake-Mole" {
        pool.add(ManaSymbol::Colorless, 3);
        pool.add(ManaSymbol::Red, 2);
        pool.add(ManaSymbol::Green, 2);
        7
    } else {
        pool.add(ManaSymbol::Colorless, 2);
        pool.add(ManaSymbol::Green, 1);
        3
    }
}

fn attack(game: &mut GameState, entries: &[(ObjectId, PlayerId)]) -> CombatState {
    let phase_already_begun = game.turn.phase == Phase::Combat
        && game.turn.step == Some(ironsmith::game_state::Step::BeginCombat);
    game.combat = None;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    if !phase_already_begun { game.mark_combat_phase_started(); }
    game.refresh_continuous_state().unwrap();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let declarations = entries.iter().map(|(creature, player)| AttackerDeclaration {
        creature: *creature, target: AttackTarget::Player(*player),
    }).collect::<Vec<_>>();
    apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
    assert!(queue.entries.is_empty(), "none of this cohort triggers on attacking");
    game.combat = Some(combat.clone());
    combat
}

fn assert_blocks(game: &GameState, combat: &CombatState, pairs: &[(ObjectId, ObjectId)], legal: bool) {
    let mut declaration = combat.clone();
    let prior_blockers = declaration.blockers.clone();
    let prior_complete = declaration.block_declaration_complete;
    let result = declare_blockers(game, &mut declaration, pairs.to_vec());
    assert_eq!(result.is_ok(), legal, "pairs={pairs:?}, result={result:?}");
    if !legal {
        assert_eq!(declaration.blockers, prior_blockers, "rejection must not mutate declaration");
        assert_eq!(declaration.block_declaration_complete, prior_complete);
    }
}

fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..24 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("source-body stack did not settle");
}

fn scry(game: &mut GameState, player: PlayerId, count: u32, dm: &mut Choices) -> usize {
    assert!(game.stack_is_empty());
    let spell = object(game, player, Zone::Stack,
        &format!("Mana cost: {{0}}\nType: Instant\nScry {count}."));
    game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, player));
    resolve_stack_entry_with(game, dm).unwrap();
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}

#[test]
fn all_three_frozen_whole_bodies_have_independent_loss_free_materializations() {
    assert_eq!(rows().len(), 3);
    for row in rows() { definitions(row["name"].as_str().unwrap()); }
}

#[test]
fn paid_activations_require_only_the_source_to_be_blocked_and_expire_at_cleanup() {
    for name in ["Anzrag, the Quake-Mole", "Loathsome Catoblepas"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = source(&mut game, &definition);
            let ally = creature(&mut game, A, "");
            let first = creature(&mut game, B, "");
            let second = creature(&mut game, B, "");
            let foreign = creature(&mut game, C, "");
            assert!(activation(&game, source).is_none(), "the ability is not free");
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 100);
            assert!(activation(&game, source).is_none(), "generic mana cannot replace colored pips");
            game.player_mut(A).unwrap().mana_pool = Default::default();
            let paid = give_activation_mana(&mut game, name);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), paid);
            let mut dm = Choices::default();
            activate(&mut game, source, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(!game.is_tapped(source), "neither printed activation has a tap cost");
            // Announcing and paying do not apply the unresolved ability.
            let mut before_resolution = game.clone();
            let combat = attack(&mut before_resolution, &[(source, B), (ally, B)]);
            assert_blocks(&before_resolution, &combat, &[], true);
            settle(&mut game, &mut dm);
            let combat = attack(&mut game, &[(source, B), (ally, B)]);
            assert_blocks(&game, &combat, &[], false);
            assert_blocks(&game, &combat, &[(first, ally)], false);
            assert_blocks(&game, &combat, &[(first, source)], true);
            assert_blocks(&game, &combat, &[(second, source)], true);
            assert_blocks(&game, &combat, &[(first, source), (second, source)], true);
            assert_blocks(&game, &combat, &[(first, source), (second, ally)], true);
            assert_blocks(&game, &combat, &[(foreign, source)], false);
            // A resolved one-turn requirement does not disappear when the
            // source's printed abilities are subsequently removed.
            game.object_mut(source).unwrap().abilities_mut().clear();
            game.refresh_continuous_state().unwrap();
            assert_blocks(&game, &combat, &[], false);
            ironsmith::turn::execute_cleanup_step(&mut game);
            game.untap(source);
            game.untap(ally);
            let later = attack(&mut game, &[(source, B), (ally, B)]);
            assert_blocks(&game, &later, &[], true);
        }
    }
}

#[test]
fn source_obligation_respects_inability_defending_player_and_new_incarnations() {
    for name in ["Anzrag, the Quake-Mole", "Loathsome Catoblepas"] {
        for definition in definitions(name) {
            let mut base = game();
            let source = source(&mut base, &definition);
            let ally = creature(&mut base, A, "");
            let blocker = creature(&mut base, B, "");
            let foreign = creature(&mut base, C, "");
            give_activation_mana(&mut base, name);
            let mut dm = Choices::default();
            activate(&mut base, source, &mut dm);
            settle(&mut base, &mut dm);
            let mut tapped = base.clone();
            tapped.tap(blocker);
            let combat = attack(&mut tapped, &[(source, B), (ally, C)]);
            assert_blocks(&tapped, &combat, &[], true);
            assert_blocks(&tapped, &combat, &[(foreign, ally)], true);
            let mut cannot = base.clone();
            cannot.move_object_by_effect(blocker, Zone::Exile).unwrap();
            creature(&mut cannot, B, "This creature can't block.");
            let combat = attack(&mut cannot, &[(source, B)]);
            assert_blocks(&cannot, &combat, &[], true);
            let mut not_attacking = base.clone();
            let combat = attack(&mut not_attacking, &[(ally, B)]);
            assert_blocks(&not_attacking, &combat, &[], true);
            // Moving out and back creates a new object. The old source-only
            // effect must not migrate to that incarnation or any other attacker.
            let mut blinked = base.clone();
            let exile = blinked.move_object_by_effect(source, Zone::Exile).unwrap();
            let returned = blinked.move_object_by_effect(exile, Zone::Battlefield).unwrap();
            assert_ne!(source, returned);
            blinked.remove_summoning_sickness(returned);
            let combat = attack(&mut blinked, &[(returned, B), (ally, B)]);
            assert_blocks(&blinked, &combat, &[], true);
        }
    }
}

#[test]
fn anzrag_block_trigger_untaps_all_your_creatures_and_repeats_in_its_added_combat() {
    for definition in definitions("Anzrag, the Quake-Mole") {
        for blocker_count in [1, 2] {
            let mut game = game();
            let source = source(&mut game, &definition);
            let attacking_ally = creature(&mut game, A, "");
            let nonattacking_ally = creature(&mut game, A, "");
            let first = creature(&mut game, B, "");
            let second = creature(&mut game, B, "");
            let enemy = creature(&mut game, C, "");
            game.tap(nonattacking_ally);
            game.tap(enemy);
            let mut unrelated_block = game.clone();
            let mut unrelated_combat = attack(&mut unrelated_block,
                &[(source, B), (attacking_ally, B)]);
            let mut unrelated_queue = TriggerQueue::new();
            unrelated_block.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
            apply_multiplayer_blocker_declarations(&mut unrelated_block,
                &mut unrelated_combat, &mut unrelated_queue,
                &[BlockerDeclaration { blocker: first, blocking: attacking_ally }]).unwrap();
            assert!(unrelated_queue.entries.is_empty(), "another creature becoming blocked is not Anzrag");
            assert!(unrelated_block.is_tapped(source));
            assert!(unrelated_block.turn_store.additional_phases.is_empty());
            give_activation_mana(&mut game, "Anzrag, the Quake-Mole");
            let mut dm = Choices::default();
            activate(&mut game, source, &mut dm);
            settle(&mut game, &mut dm);
            let mut combat = attack(&mut game, &[(source, B), (attacking_ally, B)]);
            assert!(game.is_tapped(source) && game.is_tapped(attacking_ally));
            let mut queue = TriggerQueue::new();
            let declarations = [first, second].into_iter().take(blocker_count)
                .map(|blocker| BlockerDeclaration { blocker, blocking: source }).collect::<Vec<_>>();
            game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
            apply_multiplayer_blocker_declarations(&mut game, &mut combat, &mut queue, &declarations).unwrap();
            assert_eq!(queue.entries.len(), 1, "becomes blocked occurs once, not once per blocker");
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            settle(&mut game, &mut dm);
            for id in [source, attacking_ally, nonattacking_ally] { assert!(!game.is_tapped(id)); }
            assert!(game.is_tapped(enemy), "only the trigger controller's creatures untap");
            assert_eq!(game.turn_store.additional_phases, vec![Phase::Combat]);
            ironsmith::turn::advance_phase(&mut game).unwrap();
            assert_eq!(game.turn.phase, Phase::Combat);
            assert!(game.turn_store.additional_phases.is_empty());
            let mut additional = attack(&mut game, &[(source, B)]);
            assert_blocks(&game, &additional, &[], false);
            let mut queue = TriggerQueue::new();
            game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
            apply_multiplayer_blocker_declarations(&mut game, &mut additional, &mut queue,
                &[BlockerDeclaration { blocker: first, blocking: source }]).unwrap();
            assert_eq!(queue.entries.len(), 1, "the trigger has no once-per-turn limitation");
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            settle(&mut game, &mut dm);
            assert!(!game.is_tapped(source));
            assert_eq!(game.turn_store.additional_phases, vec![Phase::Combat]);
        }
    }
}

#[test]
fn glorfindel_each_scry_mode_pumps_once_and_preserves_its_distinct_block_rule() {
    for definition in definitions("Glorfindel, Dauntless Rescuer") {
        for mode in [0, 1] {
            let mut game = game();
            let source = source(&mut game, &definition);
            let ally = creature(&mut game, A, "");
            let first = creature(&mut game, B, "");
            let second = creature(&mut game, B, "");
            for player in [A, B] {
                for _ in 0..3 { object(&mut game, player, Zone::Library, "Type: Artifact"); }
            }
            let mut dm = Choices { mode, ..Default::default() };
            assert_eq!(scry(&mut game, B, 2, &mut dm), 0, "an opponent's scry is not yours");
            assert_eq!(game.current_power(source), Some(3));
            assert_eq!(scry(&mut game, A, 2, &mut dm), 1, "scry 2 still produces one trigger");
            assert_eq!(game.current_power(source), Some(3), "the common pump waits for resolution");
            settle(&mut game, &mut dm);
            assert_eq!(dm.partitions, vec![(B, 2), (A, 2)]);
            assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(4), Some(3)));
            assert_eq!(game.current_power(ally), Some(5));
            let combat = attack(&mut game, &[(source, B), (ally, B)]);
            assert_blocks(&game, &combat, &[], mode == 1);
            assert_blocks(&game, &combat, &[(first, ally)], mode == 1);
            assert_blocks(&game, &combat, &[(first, source)], true);
            assert_blocks(&game, &combat, &[(first, source), (second, source)], mode == 0);
            assert_blocks(&game, &combat, &[(first, source), (second, ally)], true);
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(3), Some(2)));
            game.untap(source);
            game.untap(ally);
            let later = attack(&mut game, &[(source, B), (ally, B)]);
            assert_blocks(&game, &later, &[], true);
            assert_blocks(&game, &later, &[(first, source), (second, source)], true);
        }
    }
}

#[test]
fn glorfindel_two_scry_triggers_combine_one_required_blocker_with_the_one_blocker_limit() {
    for definition in definitions("Glorfindel, Dauntless Rescuer") {
        let mut game = game();
        let source = source(&mut game, &definition);
        let first = creature(&mut game, B, "");
        let second = creature(&mut game, B, "");
        // A positive scry still happens with an empty library.
        let mut dm = Choices::default();
        assert_eq!(scry(&mut game, A, 1, &mut dm), 1);
        settle(&mut game, &mut dm);
        dm.mode = 1;
        assert_eq!(scry(&mut game, A, 1, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert!(dm.partitions.is_empty());
        assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(5), Some(4)));
        let combat = attack(&mut game, &[(source, B)]);
        assert_blocks(&game, &combat, &[], false);
        assert_blocks(&game, &combat, &[(first, source)], true);
        assert_blocks(&game, &combat, &[(second, source)], true);
        assert_blocks(&game, &combat, &[(first, source), (second, source)], false);
        game.tap(first);
        game.tap(second);
        assert_blocks(&game, &combat, &[], true);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(3), Some(2)));
    }
}

#[test]
fn catoblepas_death_uses_its_departed_source_and_targets_only_an_opponents_creature() {
    for definition in definitions("Loathsome Catoblepas") {
        for destination in [Zone::Graveyard, Zone::Exile] {
            let mut game = game();
            let source = source(&mut game, &definition);
            let friendly = creature(&mut game, A, "");
            let victim = creature(&mut game, B, "");
            let other_opponent = creature(&mut game, C, "");
            let noncreature = object(&mut game, B, Zone::Battlefield, "Type: Artifact");
            let unrelated = creature(&mut game, A, "");
            let mut dm = Choices { target: Some(victim), ..Default::default() };
            game.move_object_by_effect(unrelated, Zone::Graveyard).unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
            assert!(game.stack_is_empty(), "another creature dying is not this creature dying");
            let moved = game.move_object_by_effect(source, destination).unwrap();
            assert_ne!(source, moved);
            put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
            assert_eq!(game.stack.len(), usize::from(destination == Zone::Graveyard));
            if destination == Zone::Graveyard {
                assert!(!dm.target_pools.is_empty());
                for pool in &dm.target_pools {
                    assert!(pool.contains(&Target::Object(victim)));
                    assert!(pool.contains(&Target::Object(other_opponent)));
                    assert!(!pool.contains(&Target::Object(friendly)));
                    assert!(!pool.contains(&Target::Object(noncreature)));
                }
            }
            settle(&mut game, &mut dm);
            let expected = if destination == Zone::Graveyard { 2 } else { 5 };
            assert_eq!((game.current_power(victim), game.current_toughness(victim)), (Some(expected), Some(expected)));
            assert_eq!((game.current_power(friendly), game.current_toughness(friendly)), (Some(5), Some(5)));
            assert_eq!((game.current_power(other_opponent), game.current_toughness(other_opponent)), (Some(5), Some(5)));
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!((game.current_power(victim), game.current_toughness(victim)), (Some(5), Some(5)));
        }
    }
}
