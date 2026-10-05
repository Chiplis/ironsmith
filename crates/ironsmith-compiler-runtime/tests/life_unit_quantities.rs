//! Per-unit life regressions, authored and unrun under the campaign gate.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::effects::{EffectContext, EffectExecutor, GainLifeEffect, LoseLifeEffect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{EventValueSpec, PlayerFilter, Value};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/life_unit_quantities.json.fixture"
    ))
    .unwrap()
}
fn defs_from(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let wire = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(wire, artifact);
    [direct, materialize_artifact(&wire).unwrap()]
}
fn defs(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    defs_from(name, row["text"].as_str().unwrap())
}
fn new_game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn card(game: &mut GameState, player: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Quantity resource", text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn creature(game: &mut GameState) -> ObjectId {
    card(
        game,
        A,
        Zone::Battlefield,
        "Type: Creature\nPower/Toughness: 2/2",
    )
}
fn apply(game: &mut GameState, source: ObjectId, effect: &dyn EffectExecutor) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn gain(game: &mut GameState, source: ObjectId, player: PlayerId, n: i32) {
    apply(
        game,
        source,
        &GainLifeEffect::with_filter(n, PlayerFilter::Specific(player)),
    );
}
fn lose(game: &mut GameState, source: ObjectId, n: i32) {
    apply(game, source, &LoseLifeEffect::you(n));
}
fn pending(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas_with(game, &mut queue, dm).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    game.stack.len()
}

fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) {
    pending(game, dm);
    for _ in 0..50 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        pending(game, dm);
    }
    panic!(
        "life program did not settle: life={}, active={}, pending={}",
        game.player(A).unwrap().life,
        game.player(A).unwrap().is_in_game(),
        game.stack.len()
    );
}
fn action(game: &mut GameState, action: LegalAction, dm: &mut impl DecisionMaker) {
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
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("missing choice")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    assert!(!game.stack_is_empty());
    settle(game, dm);
}
fn cast(game: &mut GameState, spell: ObjectId, dm: &mut impl DecisionMaker) {
    game.turn.priority_player = Some(A);
    let selected = ironsmith::decision::compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
        .unwrap();
    action(game, selected, dm);
}
fn counters(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    decline: bool,
    seen_pools: Vec<Vec<ObjectId>>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        !self.decline && ctx.can_accept
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.targets
            .iter()
            .filter(|target| {
                ctx.requirements
                    .iter()
                    .any(|req| req.legal_targets.contains(target))
            })
            .copied()
            .collect()
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|item| item.legal)
            .map(|item| item.id)
            .collect();
        self.seen_pools.push(legal.clone());
        let mut chosen: Vec<_> = self
            .objects
            .iter()
            .filter(|id| legal.contains(id))
            .copied()
            .collect();
        for id in legal {
            if !chosen.contains(&id) {
                chosen.push(id);
            }
        }
        chosen.truncate(ctx.max.unwrap_or(1));
        chosen
    }
}
#[test]
fn six_exact_complete_programs_round_trip() {
    assert_eq!(rows().len(), 6);
    for row in rows() {
        for definition in defs(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}
#[test]
fn cradle_uses_four_actual_life_not_two_paid_mana_or_later_life_changes() {
    for definition in defs("Cradle of Vitality") {
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = creature(&mut game);
        let unrelated = creature(&mut game);
        let mut dm = Choices {
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        let replacement = compile_to_runtime_definition(
            "Life addition",
            "Type: Enchantment\nIf you would gain life, you gain that much life plus 1 instead.",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&replacement, A, Zone::Battlefield);
        gain(&mut game, source, A, 3);
        assert_eq!(pending(&mut game, &mut dm), 1);
        // A later total is irrelevant to the already captured life event.
        game.player_mut(A).unwrap().life = 40;
        for symbol in [ManaSymbol::White, ManaSymbol::Colorless] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
        }
        settle(&mut game, &mut dm);
        assert_eq!(counters(&game, target), 4);
        assert_eq!(counters(&game, unrelated), 0);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        gain(&mut game, source, A, 1);
        dm.decline = true;
        settle(&mut game, &mut dm);
        assert_eq!(counters(&game, target), 4);
    }
}
#[test]
fn false_cure_registers_a_repeating_exact_player_amount_and_expires() {
    for definition in defs("False Cure") {
        let mut game = new_game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 2);
        cast(&mut game, spell, &mut SelectFirstDecisionMaker);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        let source = card(&mut game, A, Zone::Battlefield, "Type: Artifact");
        gain(&mut game, source, B, 4);
        assert_eq!(pending(&mut game, &mut SelectFirstDecisionMaker), 1);
        gain(&mut game, source, B, 1);
        assert_eq!(pending(&mut game, &mut SelectFirstDecisionMaker), 2);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(game.player(B).unwrap().life, 15);
        assert_eq!(game.player(A).unwrap().life, 20);
        gain(&mut game, source, C, 3);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(game.player(C).unwrap().life, 17);
        assert_eq!(game.player(B).unwrap().life, 15);
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.next_turn();
        gain(&mut game, source, B, 2);
        assert_eq!(pending(&mut game, &mut SelectFirstDecisionMaker), 0);
        assert_eq!(game.player(B).unwrap().life, 17);
    }
}
#[test]
fn lichs_tomb_selects_one_simultaneous_sacrifice_batch_and_protects_zero_life() {
    for definition in defs("Lich's Tomb") {
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let watcher = compile_to_runtime_definition(
            "Batch witness",
            "Type: Enchantment\nWhenever one or more creatures you control die, you gain 1 life.",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&watcher, A, Zone::Battlefield);
        let selected = vec![
            creature(&mut game),
            creature(&mut game),
            creature(&mut game),
        ];
        let untouched = creature(&mut game);
        let mut dm = Choices {
            objects: selected.clone(),
            ..Default::default()
        };
        lose(&mut game, source, 3);
        assert_eq!(pending(&mut game, &mut dm), 1);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(selected.iter().all(|id| !game.battlefield.contains(id)));
        assert!(game.battlefield.contains(&untouched));
        assert_eq!(
            pending(&mut game, &mut dm),
            1,
            "one grouped death, not three sequential sacrifices"
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 18);
        game.player_mut(A).unwrap().life = 0;
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert!(game.player(A).unwrap().is_in_game());
    }
}
#[test]
fn lichs_mastery_chooses_one_owned_or_controlled_cross_zone_batch() {
    for definition in defs("Lich's Mastery") {
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..5 {
            card(&mut game, A, Zone::Library, "Type: Artifact");
        }
        let first = creature(&mut game);
        let stolen = card(&mut game, B, Zone::Battlefield, "Type: Artifact");
        game.set_current_controller(stolen, A).unwrap();
        let hand = card(&mut game, A, Zone::Hand, "Type: Artifact");
        let grave = card(&mut game, A, Zone::Graveyard, "Type: Artifact");
        let foreign_hand = card(&mut game, B, Zone::Hand, "Type: Artifact");
        let foreign_grave = card(&mut game, B, Zone::Graveyard, "Type: Artifact");
        let enemy = card(&mut game, B, Zone::Battlefield, "Type: Artifact");
        let mut dm = Choices {
            objects: vec![first, stolen, hand, grave],
            ..Default::default()
        };
        lose(&mut game, source, 4);
        assert_eq!(pending(&mut game, &mut dm), 1);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.exile.len(), 4);
        assert!(game.battlefield.contains(&source));
        assert!(
            dm.seen_pools
                .iter()
                .all(|pool| !pool.contains(&foreign_hand)
                    && !pool.contains(&foreign_grave)
                    && !pool.contains(&enemy))
        );
        assert!(game.player(A).unwrap().hand.is_empty());
        gain(&mut game, source, A, 2);
        settle(&mut game, &mut dm);
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            2,
            "the independent life-gain draw body is retained"
        );
        game.player_mut(A).unwrap().life = -1;
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert!(game.player(A).unwrap().is_in_game());
    }
}
#[test]
fn mastery_self_exile_does_not_cancel_other_selected_cards_and_then_really_loses() {
    for definition in defs("Lich's Mastery") {
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand = card(&mut game, A, Zone::Hand, "Type: Artifact");
        let mut dm = Choices {
            objects: vec![source, hand],
            ..Default::default()
        };
        lose(&mut game, source, 2);
        assert_eq!(pending(&mut game, &mut dm), 1);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.exile.len(), 2);
        assert!(game.player(A).unwrap().hand.is_empty());
        settle(&mut game, &mut dm);
        assert!(!game.player(A).unwrap().is_in_game());
    }
}
#[test]
fn oath_keeps_individual_discard_or_sacrifice_decisions_and_its_other_activation() {
    for definition in defs("Oath of Lim-Dûl") {
        let index = definition
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .unwrap();
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = creature(&mut game);
        let two = creature(&mut game);
        let hand = card(&mut game, A, Zone::Hand, "Type: Artifact");
        let untouched = creature(&mut game);
        let mut dm = Choices {
            objects: vec![hand, one, two],
            ..Default::default()
        };
        lose(&mut game, source, 3);
        assert_eq!(pending(&mut game, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert!(game.battlefield.contains(&source));
        assert!(game.battlefield.contains(&untouched));
        assert!(!game.battlefield.contains(&one));
        assert!(!game.battlefield.contains(&two));
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
        card(&mut game, A, Zone::Library, "Type: Artifact");
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 2);
        game.turn.priority_player = Some(A);
        let selected=ironsmith::decision::compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index==index)).unwrap();
        action(&mut game, selected, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn transcendence_has_exact_loss_amount_and_real_twenty_life_state_trigger() {
    for definition in defs("Transcendence") {
        let mut game = new_game();
        game.player_mut(A).unwrap().life = 17;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        lose(&mut game, source, 3);
        assert_eq!(pending(&mut game, &mut SelectFirstDecisionMaker), 1);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert!(
            !game.player(A).unwrap().is_in_game(),
            "state trigger must really lose at twenty"
        );
        let mut game = new_game();
        game.player_mut(A).unwrap().life = 0;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert!(game.player(A).unwrap().is_in_game());
        lose(&mut game, source, 1);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(game.player(A).unwrap().life, 1);
        assert!(game.player(A).unwrap().is_in_game());
    }
}
#[test]
fn an_explicit_local_life_producer_wins_across_an_unrelated_payment_result() {
    let text = "Mana cost: {0}\nType: Sorcery\nYou gain 2 life. You may pay {1}. If you do, put a +1/+1 counter on target creature for each 1 life you gained.";
    for definition in defs_from("Local life quantity", text) {
        let mut game = new_game();
        let target = creature(&mut game);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        let mut dm = Choices {
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        cast(&mut game, spell, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(counters(&game, target), 2);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
    for text in [
        "Type: Enchantment\nWhenever you gain life, put a +1/+1 counter on target creature for each 1 life you lost.",
        "Type: Creature\nPower/Toughness: 2/2\nWhenever this creature deals damage, put a +1/+1 counter on it for each 1 life you gained.",
        "Type: Sorcery\nAn opponent gains 3 life. Put a +1/+1 counter on target creature for each 1 life you gained.",
    ] {
        assert!(compile_to_runtime_definition("Mismatched life quantity", text, false).is_err());
    }
}
#[test]
fn typed_runtime_life_quantity_rejects_wrong_event_direction_and_participant() {
    let game = new_game();
    let source = ObjectId(99);
    let mut ctx = EffectContext::new_default(source, A).with_triggering_event(
        ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::LifeGainEvent::new(B, 4),
            ironsmith::provenance::ProvNodeId::default(),
        ),
    );
    let value = |gained, for_controller| {
        Value::EventValue(EventValueSpec::LifeChange {
            gained,
            for_controller,
        })
    };
    assert!(ironsmith::effects::helpers::resolve_value(&game, &value(false, false), &ctx).is_err());
    assert!(ironsmith::effects::helpers::resolve_value(&game, &value(true, true), &ctx).is_err());
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value(true, false), &ctx).unwrap(),
        4
    );
    ctx = ctx.with_event_value_amount(99);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value(true, false), &ctx).unwrap(),
        4,
        "a generic count override is not a life event"
    );
}

#[test]
fn a_declined_optional_local_gain_exports_zero_without_an_ambient_fallback() {
    let text = "Mana cost: {0}\nType: Sorcery\nYou may gain 2 life. Put a +1/+1 counter on target creature for each 1 life you gained.";
    for definition in defs_from("Optional local life", text) {
        for decline in [false, true] {
            let mut game = new_game();
            let target = creature(&mut game);
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices {
                targets: vec![Target::Object(target)],
                decline,
                ..Default::default()
            };
            cast(&mut game, spell, &mut dm);
            assert_eq!(counters(&game, target), if decline { 0 } else { 2 });
            assert_eq!(game.player(A).unwrap().life, if decline { 20 } else { 22 });
        }
    }
}
