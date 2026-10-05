//! Source-authored regressions. No execution until the campaign validation gate.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::effects::{
    EffectContext, EffectExecutor, GainLifeEffect, LoseLifeEffect, PayLifeEffect,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::static_abilities::StaticAbilityId;
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
        "../../../fixtures/life_change_triggers.json.fixture"
    ))
    .unwrap()
}
fn definitions_from(name: &str, text: &str) -> [CardDefinition; 2] {
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    definitions_from(name, row["text"].as_str().unwrap())
}
fn new_game() -> GameState {
    let mut game = GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Dana".into(),
        ],
        20,
    );
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn card(game: &mut GameState, player: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let def = compile_to_runtime_definition("Life test resource", text, false).unwrap();
    game.create_object_from_definition(&def, player, zone)
}
fn apply(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    effect: &dyn EffectExecutor,
) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, controller, &mut dm);
    let out = effect.execute(game, &mut ctx).unwrap();
    for event in out.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn gain(game: &mut GameState, source: ObjectId, player: PlayerId, amount: i32) {
    apply(
        game,
        source,
        A,
        &GainLifeEffect::with_filter(amount, PlayerFilter::Specific(player)),
    );
}
fn stack_pending(game: &mut GameState) -> usize {
    put_triggers_on_stack_with_dm(
        game,
        &mut TriggerQueue::new(),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState) {
    stack_pending(game);
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        stack_pending(game);
    }
    panic!("life triggers did not settle");
}
fn counters(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}
fn mana(game: &mut GameState, player: PlayerId) {
    for symbol in [ManaSymbol::White, ManaSymbol::Black, ManaSymbol::Colorless] {
        game.player_mut(player).unwrap().mana_pool.add(symbol, 1);
    }
}
fn activate(game: &mut GameState, source: ObjectId, ability_index: usize) {
    game.turn.priority_player = Some(A);
    let action=ironsmith::decision::compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index:index} if *id==source && *index==ability_index)).expect("paid activation legal");
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("activation lacks choice")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    assert!(!game.stack_is_empty());
    settle(game);
}

#[test]
fn five_exact_programs_round_trip_without_loss() {
    assert_eq!(rows().len(), 5);
    for row in rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}

#[test]
fn kavu_uses_actual_gain_amount_and_does_not_treat_teammates_or_zero_as_opponents() {
    for definition in definitions("Kavu Predator") {
        let mut game = new_game();
        game.set_teams(vec![vec![A, B], vec![C, D]]).unwrap();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for player in [A, B] {
            gain(&mut game, source, player, 4);
            assert_eq!(stack_pending(&mut game), 0);
        }
        gain(&mut game, source, C, 3);
        assert_eq!(stack_pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(counters(&game, source), 3);
        gain(&mut game, source, C, 0);
        assert_eq!(stack_pending(&mut game), 0);
        game.effect_store.cant_effects.add_cant_gain_life(D);
        gain(&mut game, source, D, 5);
        assert_eq!(stack_pending(&mut game), 0);
        let replacement = compile_to_runtime_definition(
            "Life addition",
            "Type: Enchantment\nIf you would gain life, you gain that much life plus 1 instead.",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&replacement, C, Zone::Battlefield);
        gain(&mut game, source, C, 2);
        assert_eq!(stack_pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(
            counters(&game, source),
            6,
            "the event retains the modified 3, not the proposal's 2"
        );
    }
}

#[test]
fn separate_opponent_gains_in_one_instruction_keep_independent_amounts_and_triggers() {
    for definition in definitions("Kavu Predator") {
        let mut game = new_game();
        game.set_teams(vec![vec![A, B], vec![C, D]]).unwrap();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        apply(
            &mut game,
            source,
            A,
            &ironsmith::effects::ForPlayersEffect::new(
                PlayerFilter::Opponent,
                vec![ironsmith::Effect::new(GainLifeEffect::with_filter(
                    2,
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
        );
        assert_eq!(stack_pending(&mut game), 2);
        settle(&mut game);
        assert_eq!(counters(&game, source), 4);
        assert_eq!(game.player(B).unwrap().life, 20);
        game.set_current_controller(source, C).unwrap();
        gain(&mut game, source, D, 2);
        assert_eq!(stack_pending(&mut game), 0);
        gain(&mut game, source, A, 1);
        settle(&mut game);
        assert_eq!(counters(&game, source), 5);
    }
}

#[test]
fn gain_or_loss_has_two_real_events_one_shared_turn_guard_and_one_shared_limit() {
    for name in ["Moonstone Harbinger", "Wax-Wane Witness"] {
        for definition in definitions(name) {
            let mut game = new_game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let initial = game.current_power(source).unwrap();
            let bat = card(
                &mut game,
                A,
                Zone::Battlefield,
                "Type: Creature — Bat\nPower/Toughness: 1/1",
            );
            let enemy_bat = card(
                &mut game,
                B,
                Zone::Battlefield,
                "Type: Creature — Bat\nPower/Toughness: 1/1",
            );
            gain(&mut game, source, B, 2);
            assert_eq!(stack_pending(&mut game), 0);
            gain(&mut game, source, A, 2);
            assert_eq!(stack_pending(&mut game), 1);
            settle(&mut game);
            assert_eq!(game.current_power(source), Some(initial + 1));
            apply(&mut game, source, A, &PayLifeEffect::you(1));
            assert_eq!(
                stack_pending(&mut game),
                usize::from(name == "Wax-Wane Witness"),
                "both arms share Moonstone's once-each-turn identity"
            );
            settle(&mut game);
            assert_eq!(
                game.current_power(source),
                Some(initial + if name == "Wax-Wane Witness" { 2 } else { 1 })
            );
            if name == "Moonstone Harbinger" {
                assert_eq!(game.current_power(bat), Some(2));
                assert!(game.current_has_static_ability_id(bat, StaticAbilityId::Deathtouch));
                assert!(
                    !game.current_has_static_ability_id(enemy_bat, StaticAbilityId::Deathtouch)
                );
            }
            ironsmith::turn::execute_cleanup_step(&mut game);
            game.next_turn();
            assert_eq!(game.current_power(source), Some(initial));
            apply(&mut game, source, A, &LoseLifeEffect::you(1));
            gain(&mut game, source, A, 2);
            assert_eq!(
                stack_pending(&mut game),
                0,
                "neither arm fires on an opponent's turn"
            );
            for _ in 0..3 {
                game.next_turn();
            }
            assert_eq!(game.turn.active_player, A);
            apply(&mut game, source, A, &LoseLifeEffect::you(1));
            assert_eq!(stack_pending(&mut game), 1);
            settle(&mut game);
            assert_eq!(game.current_power(source), Some(initial + 1));
        }
    }
}

#[test]
fn punishing_fire_functions_only_in_its_graveyard_and_pays_red_before_returning() {
    for definition in definitions("Punishing Fire") {
        for zone in [Zone::Hand, Zone::Graveyard, Zone::Exile] {
            let mut game = new_game();
            let source = game.create_object_from_definition(&definition, A, zone);
            gain(&mut game, source, A, 1);
            assert_eq!(stack_pending(&mut game), 0);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Red, 1);
            gain(&mut game, source, B, 3);
            assert_eq!(
                stack_pending(&mut game),
                usize::from(zone == Zone::Graveyard)
            );
            settle(&mut game);
            if zone == Zone::Graveyard {
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert_ne!(
                    game.player(A).unwrap().hand[0],
                    source,
                    "a return creates a new zone identity"
                );
            } else {
                assert_eq!(game.object(source).unwrap().zone, zone);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
            }
        }
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        gain(&mut game, source, B, 1);
        assert_eq!(stack_pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(
            game.object(source).unwrap().zone,
            Zone::Graveyard,
            "an unpaid optional return does not occur"
        );
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);
        gain(&mut game, source, B, 1);
        assert_eq!(stack_pending(&mut game), 1);
        let exile = game.move_object_by_effect(source, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exile, Zone::Graveyard).unwrap();
        settle(&mut game);
        assert_eq!(game.object(returned).unwrap().zone, Zone::Graveyard);
        assert!(
            game.player(A).unwrap().hand.is_empty(),
            "an old trigger cannot return the new graveyard incarnation"
        );
    }
}

#[test]
fn vizkopa_paid_listener_repeats_keeps_controller_after_departure_and_expires() {
    for definition in definitions("Vizkopa Guildmage") {
        let index = definition
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_)))
            .nth(1)
            .unwrap()
            .0;
        let mut game = new_game();
        game.set_teams(vec![vec![A, B], vec![C, D]]).unwrap();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        mana(&mut game, A);
        activate(&mut game, source, index);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        mana(&mut game, A);
        activate(&mut game, source, index);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let action_source = card(&mut game, A, Zone::Battlefield, "Type: Artifact");
        gain(&mut game, action_source, B, 3);
        assert_eq!(stack_pending(&mut game), 0);
        gain(&mut game, action_source, A, 3);
        assert_eq!(stack_pending(&mut game), 2);
        settle(&mut game);
        for player in [C, D] {
            assert_eq!(game.player(player).unwrap().life, 14);
        }
        assert_eq!(game.player(B).unwrap().life, 23);
        assert_eq!(game.player(A).unwrap().life, 23);
        gain(&mut game, action_source, A, 1);
        assert_eq!(stack_pending(&mut game), 2);
        settle(&mut game);
        assert_eq!(game.player(C).unwrap().life, 12);
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.next_turn();
        gain(&mut game, action_source, A, 2);
        assert_eq!(stack_pending(&mut game), 0);
        assert_eq!(game.player(C).unwrap().life, 12);
    }
}

#[test]
fn qualified_life_player_filters_fail_closed_and_keep_the_affected_players_turn() {
    for definition in definitions_from(
        "Affected life player",
        "Type: Enchantment\nWhenever an opponent gains life during their turn, that player draws a card.",
    ) {
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        card(&mut game, B, Zone::Library, "Type: Artifact");
        gain(&mut game, source, B, 1);
        assert_eq!(stack_pending(&mut game), 0);
        game.next_turn();
        gain(&mut game, source, C, 1);
        assert_eq!(stack_pending(&mut game), 0);
        gain(&mut game, source, B, 1);
        assert_eq!(stack_pending(&mut game), 1);
        settle(&mut game);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert!(game.player(A).unwrap().hand.is_empty());
    }
    for clause in [
        "an unknown opponent gains life",
        "you gain or lose life during an unknown phase",
    ] {
        assert!(
            compile_to_runtime_definition(
                "Bad life qualifier",
                &format!("Type: Enchantment\nWhenever {clause}, you draw a card."),
                false
            )
            .is_err()
        );
    }
}

#[test]
fn vizkopa_other_paid_ability_grants_working_lifelink_until_cleanup() {
    for definition in definitions("Vizkopa Guildmage") {
        let index = definition
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .unwrap();
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        mana(&mut game, A);
        activate(&mut game, source, index);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Lifelink));
        apply(
            &mut game,
            source,
            A,
            &ironsmith::effects::DealDamageEffect::new(
                2,
                ironsmith::target::ChooseSpec::SpecificPlayer(B),
            ),
        );
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(game.player(B).unwrap().life, 18);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(!game.current_has_static_ability_id(source, StaticAbilityId::Lifelink));
    }
}

struct TargetBob;
impl DecisionMaker for TargetBob {
    fn decide_targets(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let target = ironsmith::game_state::Target::Player(B);
        assert!(
            context
                .requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&target))
        );
        vec![target]
    }
}

#[test]
fn punishing_fire_keeps_its_real_paid_damage_spell_body() {
    for definition in definitions("Punishing Fire") {
        let mut game = new_game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        for symbol in [ManaSymbol::Red, ManaSymbol::Colorless] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
        }
        let action = ironsmith::decision::compute_legal_actions(&game, A).unwrap().into_iter()
            .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
        let mut state = PriorityLoopState::new(game.players.len());
        let mut queue = TriggerQueue::new();
        let mut dm = TargetBob;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..30 {
            if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("cast lacks requested choice");
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(B).unwrap().life, 18);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}
