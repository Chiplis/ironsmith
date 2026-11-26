//! CR 117.4: completing a mana activation interrupts consecutive priority passes.
use ironsmith::decision::{AutoPassDecisionMaker, GameProgress, LegalAction};
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, PlayerId, Zone};

#[test]
fn paid_mana_activation_breaks_consecutive_passes() {
    paid_mana_priority(false);
}

#[test]
fn cancelled_mana_activation_preserves_consecutive_passes() {
    paid_mana_priority(true);
}

#[test]
fn nested_paid_mana_activation_returns_to_its_payment_parent() {
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = a;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(b);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::ids::CardId::new(),
        "Nested paid mana probe",
    )
    .card_types(vec![ironsmith::types::CardType::Artifact])
    .with_ability(ironsmith::ability::Ability::mana(
        ironsmith::cost::TotalCost::from_costs(vec![
            ironsmith::costs::Cost::mana(ManaCost::new().add_generic(1)),
            ironsmith::costs::Cost::tap(),
        ]),
        vec![ManaSymbol::Green],
    ))
    .build();
    let root = game.create_object_from_definition(&definition, a, Zone::Battlefield);
    let child = game.create_object_from_definition(&definition, a, Zone::Battlefield);
    game.player_mut(a)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = AutoPassDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority),
        &mut dm,
    )
    .unwrap();
    apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
            source: root,
            ability_index: 0,
        }),
        &mut dm,
    )
    .unwrap();
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Activate {
            source: child,
            ability_index: 0,
        }),
        &mut dm,
    )
    .unwrap();
    let GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ctx)) = progress else {
        panic!("child payment was not offered");
    };
    assert_eq!(ctx.source, child);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Confirm {
            plan_id: ctx.plan.id,
            request_hash: ctx.plan.request_hash,
        }),
        &mut dm,
    )
    .unwrap();
    let GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ctx)) = progress else {
        panic!("child escaped its parent payment");
    };
    assert_eq!(ctx.source, root);
    assert!(!game.is_tapped(root));
    assert!(game.is_tapped(child));
    apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Confirm {
            plan_id: ctx.plan.id,
            request_hash: ctx.plan.request_hash,
        }),
        &mut dm,
    )
    .unwrap();
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority),
        &mut dm,
    )
    .unwrap();
    assert!(
        matches!(progress,GameProgress::NeedsDecisionCtx(DecisionContext::Priority(ref ctx)) if ctx.player==b)
    );
}

fn paid_mana_priority(cancel: bool) {
    // CR 117.3c, 117.4: an activation separates passes even when its
    // authoritative mana payment is answered in a subsequent API call.
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = a;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(b);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::ids::CardId::new(),
        "Paid mana probe",
    )
    .card_types(vec![ironsmith::types::CardType::Artifact])
    .with_ability(ironsmith::ability::Ability::mana(
        ironsmith::cost::TotalCost::from_costs(vec![
            ironsmith::costs::Cost::mana(ManaCost::new().add_generic(1)),
            ironsmith::costs::Cost::tap(),
        ]),
        vec![ManaSymbol::Green],
    ))
    .build();
    let source = game.create_object_from_definition(&definition, a, Zone::Battlefield);
    game.player_mut(a)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    let mut queue = TriggerQueue::new();
    let mut priority = PriorityLoopState::new(2);
    let mut dm = AutoPassDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut priority,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority),
        &mut dm,
    )
    .unwrap();
    assert_eq!(game.turn.priority_player, Some(a));
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut priority,
        &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
            source,
            ability_index: 0,
        }),
        &mut dm,
    )
    .unwrap();
    let GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ctx)) = progress else {
        panic!("expected payment window");
    };
    assert!(ctx.plan.payable);
    apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut priority,
        &PriorityResponse::ManaPaymentPlan(if cancel {
            ManaPaymentResponse::Cancel
        } else {
            ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }),
        &mut dm,
    )
    .unwrap();
    assert_eq!(game.is_tapped(source), !cancel);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut priority,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority),
        &mut dm,
    )
    .unwrap();
    if cancel {
        assert!(
            matches!(progress, GameProgress::Continue),
            "a reversed announcement must not erase a prior pass: {progress:?}"
        );
        return;
    }
    assert!(
        matches!(progress,GameProgress::NeedsDecisionCtx(DecisionContext::Priority(ref ctx)) if ctx.player==b),
        "the activation must break the earlier pass sequence; got {progress:?}"
    );
}
