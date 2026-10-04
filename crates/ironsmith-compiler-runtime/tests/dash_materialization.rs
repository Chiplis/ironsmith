use ironsmith::alternative_cast::{AlternativeCastingMethod, CastingMethod};
use ironsmith::cards::{CardDefinition, generated_definition_has_unimplemented_content};
use ironsmith::cost::{OptionalCostKind, OptionalCostRef};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::events::phase::{BeginningOfEndStepEvent, BeginningOfUpkeepEvent};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack, resolve_stack_entry,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_delayed_triggers};
use ironsmith::{ConditionExpr, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::StaticAbilityPayload;
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const PROBE: &str = "Mana cost: {2}{B}\nType: Creature — Warrior\nPower/Toughness: 2/2\nDash {B}";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let bytes = serde_json::to_vec(&artifact).unwrap();
    let restored: CompiledCardArtifact = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

fn assert_typed_dash(definition: &CardDefinition) {
    assert!(
        definition
            .alternative_casts
            .iter()
            .any(|method| matches!(method, AlternativeCastingMethod::Dash { .. }))
    );
    assert!(
        definition.abilities.iter().any(|ability| {
            let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind else {
                return false;
            };
            let Some(model) = ability.compiled_model() else {
                return false;
            };
            matches!(&model.payload,
            StaticAbilityPayload::Conditional {
                condition: ConditionExpr::ThisSpellPaidLabel(reference), ..
            } if reference.kind == OptionalCostKind::Dash)
        }),
        "{} must retain a typed Dash payment condition",
        definition.card.name
    );
    assert!(
        !generated_definition_has_unimplemented_content(definition),
        "{} retains an unsupported marker: {definition:#?}",
        definition.card.name
    );
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 3);
    game
}

fn cast(game: &mut GameState, definition: &CardDefinition, method: CastingMethod) -> ObjectId {
    let alice = PlayerId::from_index(0);
    let hand = game.create_object_from_definition(definition, alice, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: hand,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    let mut state = PriorityLoopState::new(2);
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
        if state.pending_cast.is_none() && !game.stack_is_empty() {
            break;
        }
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm)
                .unwrap();
        } else {
            break;
        }
    }
    assert!(state.pending_cast.is_none(), "casting must finish");
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry(game).unwrap();
    *game.battlefield.last().unwrap()
}

fn end_step(game: &mut GameState, player: PlayerId) -> usize {
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::End);
    game.turn.active_player = player;
    let event =
        TriggerEvent::new_with_provenance(BeginningOfEndStepEvent::new(player), Default::default());
    let entries = check_delayed_triggers(game, &event);
    let count = entries.len();
    let mut queue = TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    put_triggers_on_stack(game, &mut queue).unwrap();
    while !game.stack_is_empty() {
        resolve_stack_entry(game).unwrap();
    }
    count
}

#[test]
fn dash_materialization_all_21_frozen_failures_have_typed_cost_identity() {
    let cards: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/dash_materialization.json.fixture"
    ))
    .unwrap();
    let cards = cards.as_array().unwrap();
    assert_eq!(cards.len(), 21);
    for card in cards {
        let name = card["name"].as_str().unwrap();
        for definition in definitions(name, card["text"].as_str().unwrap()) {
            assert_typed_dash(&definition);
        }
    }
}

#[test]
fn dash_materialization_normal_and_dash_casts_pay_distinct_costs_and_gate_haste() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Dash payment probe", PROBE) {
        assert_typed_dash(&definition);
        for dashed in [false, true] {
            let mut game = game();
            let permanent = cast(
                &mut game,
                &definition,
                if dashed {
                    CastingMethod::Alternative(0)
                } else {
                    CastingMethod::Normal
                },
            );
            assert_eq!(
                game.player(alice).unwrap().mana_pool.total(),
                if dashed { 2 } else { 0 }
            );
            assert_eq!(
                game.object(permanent)
                    .unwrap()
                    .optional_costs_paid
                    .was_paid_label(OptionalCostRef::new(OptionalCostKind::Dash)),
                dashed
            );
            assert_eq!(
                game.current_has_static_ability_id(permanent, StaticAbilityId::Haste),
                dashed
            );
            assert_eq!(
                ironsmith::rules::combat::can_attack(game.object(permanent).unwrap(), &game),
                dashed
            );
            assert_eq!(
                game.effect_store.delayed_triggers.len(),
                usize::from(dashed)
            );
            assert_eq!(end_step(&mut game, alice), usize::from(dashed));
            assert_eq!(game.battlefield.contains(&permanent), !dashed);
            assert_eq!(game.player(alice).unwrap().hand.len(), usize::from(dashed));
            assert_eq!(end_step(&mut game, alice), 0, "return triggers only once");
        }
    }
}

#[test]
fn dash_materialization_keeps_sorcery_timing_and_does_not_return_at_upkeep() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Dash timing probe", PROBE) {
        let mut game = game();
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::BeginCombat);
        assert!(
            !compute_legal_actions(&game, alice)
                .unwrap()
                .iter()
                .any(|action| matches!(action,
            LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)),
            "dash does not grant flash"
        );
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(alice).unwrap().mana_pool.empty();
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        let actions = compute_legal_actions(&game, alice).unwrap();
        assert!(actions.iter().any(|action| matches!(action,
            LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == hand)));
        assert!(!actions.iter().any(|action| matches!(action,
            LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Normal, .. } if *spell_id == hand)));
        let permanent = cast(&mut game, &definition, CastingMethod::Alternative(0));
        let event = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(alice),
            Default::default(),
        );
        assert!(check_delayed_triggers(&mut game, &event).is_empty());
        assert!(game.battlefield.contains(&permanent));
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        assert_eq!(end_step(&mut game, alice), 1);
    }
}

#[test]
fn dash_materialization_returns_to_owner_at_next_end_step_after_control_changes() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let flash_probe = format!("{PROBE}\nFlash");
    for definition in definitions("Dash late cast probe", &flash_probe) {
        let mut game = game();
        assert_eq!(end_step(&mut game, alice), 0);
        // Flash permits this cast after the current end step has already begun.
        let permanent = cast(&mut game, &definition, CastingMethod::Alternative(0));
        game.set_current_controller(permanent, bob).unwrap();
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(
            game.battlefield.contains(&permanent),
            "not an end-of-turn duration"
        );
        assert!(game.current_has_static_ability_id(permanent, StaticAbilityId::Haste));
        game.turn.turn_number += 1;
        assert_eq!(
            end_step(&mut game, bob),
            1,
            "the next player's end step qualifies"
        );
        assert_eq!(game.player(alice).unwrap().hand.len(), 1, "return to owner");
        assert!(game.player(bob).unwrap().hand.is_empty());
    }
}

#[test]
fn dash_materialization_delayed_return_does_not_follow_a_blinked_object() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Dash blink probe", PROBE) {
        let mut game = game();
        let permanent = cast(&mut game, &definition, CastingMethod::Alternative(0));
        let exiled = game.move_object_by_effect(permanent, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(exiled, Zone::Battlefield)
            .unwrap();
        assert_ne!(permanent, returned);
        assert!(!game.current_has_static_ability_id(returned, StaticAbilityId::Haste));
        assert_eq!(end_step(&mut game, alice), 1);
        assert!(
            game.battlefield.contains(&returned),
            "new zone-change identity is not returned"
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
    }
}

#[test]
fn dash_materialization_does_not_relax_unknown_cost_validation() {
    let mut definition =
        compile_to_runtime_definition("Unknown payment probe", PROBE, false).unwrap();
    let ironsmith::ability::AbilityKind::Static(ability) = &mut definition.abilities[0].kind else {
        panic!("dash has a conditional static grant");
    };
    let mut model = ability.compiled_model().unwrap().clone();
    let StaticAbilityPayload::Conditional { condition, .. } = &mut model.payload else {
        panic!("dash haste is gated by payment");
    };
    *condition = ConditionExpr::ThisSpellPaidLabel("Unknown cost".into());
    *ability = StaticAbility::from_model(model);
    assert!(generated_definition_has_unimplemented_content(&definition));
}
