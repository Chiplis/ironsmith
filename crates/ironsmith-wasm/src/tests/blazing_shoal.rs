use super::*;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::compute_legal_actions;
use ironsmith::game_state::Phase;
use ironsmith_registry_test::cards::definitions::{lightning_bolt, ornithopter};
use ironsmith_registry_test::compile_to_runtime_definition;

#[test]
fn blazing_shoal_casts_with_red_card_mana_value_or_normal_mana() {
    let _guard = crate::test_id_counter_guard();
    for alternative in [true, false] {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        let definition = compile_to_runtime_definition(
            "Blazing Shoal",
            "Mana cost: {X}{R}{R}\nType: Instant — Arcane\nYou may exile a red card with mana value X from your hand rather than pay this spell's mana cost.\nTarget creature gets +X/+0 until end of turn.",
            false,
        ).expect("Blazing Shoal should compile");
        let shoal = wasm
            .game
            .create_object_from_definition(&definition, alice, Zone::Hand);
        let bolt = wasm
            .game
            .create_object_from_definition(&lightning_bolt(), alice, Zone::Hand);
        let creature =
            wasm.game
                .create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);
        if !alternative {
            wasm.game
                .player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Red, 3);
        }
        let action = LegalAction::CastSpell {
            spell_id: shoal,
            from_zone: Zone::Hand,
            casting_method: if alternative {
                CastingMethod::Alternative(0)
            } else {
                CastingMethod::Normal
            },
        };
        assert!(
            compute_legal_actions(&wasm.game, alice)
                .unwrap()
                .contains(&action)
        );
        let mut root = ReplayRoot::Response(PriorityResponse::PriorityAction(action));
        let mut checkpoint = wasm.capture_replay_checkpoint();
        let mut answers = Vec::new();
        let mut announced_x = false;
        for _ in 0..12 {
            let (ctx, nested) = match wasm
                .execute_with_replay(&checkpoint, &root, &answers)
                .expect("Shoal casting should succeed")
            {
                ReplayOutcome::Complete(GameProgress::NeedsDecisionCtx(ctx)) => (ctx, false),
                ReplayOutcome::Complete(_) => break,
                ReplayOutcome::NeedsDecision(ctx) => (ctx, true),
            };
            let (response, answer) = match ctx {
                DecisionContext::Priority(_) => break,
                DecisionContext::SelectOptions(ctx)
                    if ctx.description.contains("casting method") =>
                {
                    let index = usize::from(alternative);
                    (
                        PriorityResponse::CastingMethodChoice(index),
                        ReplayDecisionAnswer::Options(vec![index]),
                    )
                }
                DecisionContext::Number(ctx) => {
                    assert_eq!(ctx.max, 1, "X must reflect the red resource, not zero");
                    announced_x = true;
                    (PriorityResponse::XValue(1), ReplayDecisionAnswer::Number(1))
                }
                DecisionContext::Targets(ctx) => {
                    assert!(
                        ctx.requirements[0]
                            .legal_targets
                            .contains(&Target::Object(creature))
                    );
                    let targets = vec![Target::Object(creature)];
                    (
                        PriorityResponse::Targets(targets.clone()),
                        ReplayDecisionAnswer::Targets(targets),
                    )
                }
                DecisionContext::SelectObjects(ctx) => {
                    assert!(alternative);
                    assert!(ctx.candidates.iter().any(|candidate| candidate.id == bolt));
                    (
                        PriorityResponse::CardCostChoice(bolt),
                        ReplayDecisionAnswer::Objects(vec![bolt]),
                    )
                }
                DecisionContext::ManaPayment(ctx) => {
                    let payment = ironsmith::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: ctx.plan.id,
                        request_hash: ctx.plan.request_hash,
                    };
                    (
                        PriorityResponse::ManaPaymentPlan(payment.clone()),
                        ReplayDecisionAnswer::ManaPayment(payment),
                    )
                }
                other => panic!("unexpected Shoal casting decision: {other:?}"),
            };
            if nested {
                answers.push(answer);
            } else {
                checkpoint = wasm.capture_replay_checkpoint();
                root = ReplayRoot::Response(response);
                answers.clear();
            }
        }
        assert!(announced_x);
        assert_eq!(wasm.game.stack.len(), 1, "Shoal should finish casting");
        assert_eq!(wasm.game.stack[0].x_value, Some(1));
        let bolt_zone = if alternative { Zone::Exile } else { Zone::Hand };
        assert!(
            wasm.game.objects_in_zone(bolt_zone).iter().any(|id| wasm
                .game
                .object(*id)
                .unwrap()
                .name
                == "Lightning Bolt")
        );
        for _ in 0..2 {
            let checkpoint = wasm.capture_replay_checkpoint();
            let outcome = wasm
                .execute_with_replay(
                    &checkpoint,
                    &ReplayRoot::Response(PriorityResponse::PriorityAction(
                        LegalAction::PassPriority,
                    )),
                    &[],
                )
                .expect("passing priority should resolve Shoal");
            assert!(matches!(outcome, ReplayOutcome::Complete(_)), "{outcome:?}");
        }
        assert!(wasm.game.stack.is_empty());
        assert_eq!(
            wasm.game
                .calculated_characteristics(creature)
                .unwrap()
                .power,
            Some(1)
        );
        assert_eq!(
            wasm.game
                .calculated_characteristics(creature)
                .unwrap()
                .toughness,
            Some(2)
        );
        wasm.game
            .effect_store
            .continuous_effects
            .cleanup_end_of_turn();
        assert_eq!(
            wasm.game
                .calculated_characteristics(creature)
                .unwrap()
                .power,
            Some(0)
        );
    }
}
