//! Actual Lionize sources, positional grants and exact future exile casting.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn definitions(id: &str) -> [CardDefinition; 2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-lionize-next-wave.json")).unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let card = &row["representative_cards"][0];
    source_support::definitions(card).unwrap()
}
fn announce(game: &mut GameState, action: ironsmith::decision::LegalAction) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut q, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn settle(game: &mut GameState) {
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    for _ in 0..64 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut q, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut q, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("did not settle");
}
#[test]
fn lionize_grants_following_printed_abilities_until_end_of_turn_and_only_that_exile_incarnation_casts_later()
 {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::static_abilities::StaticAbilityId;
    for definition in definitions("msem-892") {
        for mode in ["live", "leave-before-resolution", "leave-after-grant"] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let body = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Lionize recipient",
                "Type: Creature — Human\nPower/Toughness: 2/2",
                false,
            )
            .unwrap();
            let recipient = game.create_object_from_definition(&body, A, Zone::Battlefield);
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Red, 1);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 3);
            let activation=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,..}if *id==source)).unwrap();
            announce(&mut game, activation);
            let mut exiled = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(exiled).unwrap().zone,
                Zone::Exile,
                "exile is paid before resolution"
            );
            if mode == "leave-before-resolution" {
                let hand = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
                exiled = game.move_object_by_effect(hand, Zone::Exile).unwrap();
            }
            settle(&mut game);
            assert_eq!(
                game.counter_count(recipient, ironsmith::object::CounterType::PlusOnePlusOne),
                1
            );
            assert!(game.object_has_static_ability_id(recipient, StaticAbilityId::Menace));
            assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,..}if *id==recipient)),"recipient never gains Lionize itself");
            game.queue_trigger_event(
                Default::default(),
                ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::DamageEvent::with_cause(
                        recipient,
                        ironsmith::events::DamageTarget::Player(PlayerId(1)),
                        3,
                        true,
                        ironsmith::events::EventCause::combat_damage(recipient),
                    ),
                    Default::default(),
                ),
            );
            settle(&mut game);
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter(|id| game
                        .calculated_subtypes(**id)
                        .contains(&ironsmith::Subtype::Treasure))
                    .count(),
                3,
                "following trigger belongs to recipient"
            );
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Red, 1);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 3);
            let can_cast = |g: &GameState, id| {
                compute_legal_actions(g, A).unwrap().into_iter().find(
                    |action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id),
                )
            };
            assert!(can_cast(&game, exiled).is_none(), "same turn is prohibited");
            if mode == "leave-after-grant" {
                let hand = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
                exiled = game.move_object_by_effect(hand, Zone::Exile).unwrap();
            }
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.turn.turn_number += 1;
            assert!(!game.object_has_static_ability_id(recipient, StaticAbilityId::Menace));
            let later = can_cast(&game, exiled);
            if mode == "live" {
                announce(
                    &mut game,
                    later.expect("original exile arrival remains castable on later turn"),
                );
                settle(&mut game);
                let arrived = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrived).unwrap().zone, Zone::Battlefield);
            } else {
                assert!(
                    later.is_none(),
                    "leaving and reentering exile never revives the exact permission"
                );
            }
        }
    }
}
#[test]
fn all_fourteen_exact_lionize_source_variants_compile_without_loss() {
    for n in 892..=905 {
        let _ = definitions(&format!("msem-{n}"));
    }
}
