use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::continuous::{EffectTarget, Modification};
use crate::effects::{ApplyContinuousEffect, ExecutionContext, UnlessPaysEffect};
use crate::mana_payment::{ManaPaymentRequest, ManaPaymentResponse};
use crate::{
    Ability, AbilityKind, CardId, CardType, Effect, LegalAction, ManaCost, ManaSymbol,
    PlayerFilter, TotalCost, Until,
};

fn converter(effects: Vec<Effect>) -> Ability {
    let mut ability = Ability::activated(
        TotalCost::from_cost(crate::costs::Cost::mana(ManaCost::from_symbols(vec![
            ManaSymbol::Green,
        ]))),
        effects,
    );
    let AbilityKind::Activated(ref mut activated) = ability.kind else {
        unreachable!()
    };
    activated.mana_output = Some(vec![]);
    ability
}
fn setup(dummy: bool) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Distinct mana occurrences")
        .card_types(vec![CardType::Artifact]);
    if dummy {
        definition = definition.with_ability(Ability::activated(
            TotalCost::free(),
            vec![Effect::gain_life(1)],
        ));
    }
    definition = definition
        .with_ability(converter(vec![Effect::add_mana(vec![
            ManaSymbol::Green;
            2
        ])]))
        .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
    let source = game.create_object_from_definition(
        &definition.build(),
        PlayerId::from_index(0),
        Zone::Battlefield,
    );
    (game, source)
}
fn key(game: &mut GameState, source: ObjectId) -> crate::provenance::ProvNodeId {
    game.provenance_graph_mut()
        .alloc_root(crate::provenance::ProvenanceNodeKind::EffectExecution {
            source,
            controller: PlayerId::from_index(0),
        })
}
fn origin(game: &GameState, source: ObjectId, index: usize) -> crate::continuous::AbilityOrigin {
    game.current_characteristics(source)
        .unwrap()
        .abilities
        .origin(index)
        .unwrap()
        .clone()
}
fn choices(game: &GameState, source: ObjectId) -> Vec<(ObjectId, usize)> {
    crate::mana_payment::manual_mana_abilities(
        game,
        &ManaPaymentRequest::new(
            PlayerId::from_index(0),
            source,
            crate::costs::PaymentReason::Effect,
            ManaCost::from_symbols(vec![ManaSymbol::Green; 2]),
        ),
    )
}

#[test]
fn mana_occurrence_guard_survives_index_changes_and_distinguishes_an_identical_grant() {
    let (mut game, source) = setup(true);
    let original = origin(&game, source, 1);
    let announcement = key(&mut game, source);
    assert!(game.begin_mana_activation_guard(announcement, source, original.clone()));
    assert!(!choices(&game, source).contains(&(source, 1)));
    assert!(choices(&game, source).contains(&(source, 2)));
    let dummy = game.current_ability(source, 0).unwrap();
    let original_converter = game.current_ability(source, 1).unwrap();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut dm);
    crate::effects::execute_effect(
        &mut game,
        &Effect::new(ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::RemoveAbilityGeneric {
                ability: dummy,
                mode: ironsmith_core::AbilityLossMode::Lose,
            },
            Until::Forever,
        )),
        &mut ctx,
    )
    .unwrap();
    assert_eq!(origin(&game, source, 0), original);
    assert!(!choices(&game, source).contains(&(source, 0)));
    assert!(choices(&game, source).contains(&(source, 1)));
    crate::effects::execute_effect(
        &mut game,
        &Effect::new(ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::AddAbilityGeneric(original_converter),
            Until::Forever,
        )),
        &mut ctx,
    )
    .unwrap();
    assert_ne!(origin(&game, source, 2), original);
    assert!(
        choices(&game, source).contains(&(source, 2)),
        "a separately acquired identical ability remains available"
    );
    let mut branch = game.clone();
    branch.end_mana_activation_guard(announcement);
    assert!(choices(&branch, source).contains(&(source, 0)));
    assert!(!choices(&game, source).contains(&(source, 0)));
}

#[test]
fn mana_occurrence_guard_does_not_follow_a_new_zone_incarnation() {
    let (mut game, source) = setup(false);
    let announcement = key(&mut game, source);
    let old_origin = origin(&game, source, 0);
    assert!(game.begin_mana_activation_guard(announcement, source, old_origin.clone()));
    let exile = game
        .move_object(source, Zone::Exile, crate::events::EventCause::effect())
        .unwrap();
    let returned = game
        .move_object(
            exile,
            Zone::Battlefield,
            crate::events::EventCause::effect(),
        )
        .unwrap();
    assert_ne!(returned, source);
    assert_eq!(origin(&game, returned, 0), old_origin);
    assert!(choices(&game, returned).contains(&(returned, 0)));
    let new_key = key(&mut game, returned);
    assert!(game.begin_mana_activation_guard(new_key, returned, old_origin));
    game.end_mana_activation_guard(announcement);
    assert!(game.has_active_mana_ability_on(returned));
    game.end_mana_activation_guard(new_key);
    assert!(game.active_mana_ability_locks().is_empty());
}

#[test]
fn mana_occurrence_guard_lasts_through_its_own_resolution_payment() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Mana with an internal payment")
        .card_types(vec![CardType::Artifact])
        .with_ability(converter(vec![
            Effect::add_mana(vec![ManaSymbol::Green; 2]),
            Effect::new(UnlessPaysEffect::new(
                vec![Effect::gain_life(7)],
                PlayerFilter::You,
                vec![ManaSymbol::Green],
            )),
        ]))
        .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    struct Inspect {
        source: ObjectId,
        calls: usize,
    }
    impl crate::decision::DecisionMaker for Inspect {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.calls += 1;
            let available = crate::mana_payment::manual_mana_abilities(game, &ctx.request);
            assert!(
                !available.contains(&(self.source, 0)),
                "the original must stay locked while its own instructions resolve"
            );
            assert!(available.contains(&(self.source, 1)));
            assert!(ctx.plan.payable);
            ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }
    }
    let mut dm = Inspect { source, calls: 0 };
    crate::special_actions::perform_mana_ability_with_payment_outputs(
        &mut game,
        alice,
        source,
        0,
        None,
        Some(vec![]),
        vec![],
        &mut dm,
    )
    .unwrap();
    assert_eq!(dm.calls, 2);
    assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(game.active_mana_ability_locks().is_empty());
    assert!(choices(&game, source).contains(&(source, 0)));
}

#[test]
fn mana_occurrence_guard_priority_funding_and_cancel_keep_native_branches_separate() {
    let (mut game, source) = setup(false);
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::PriorityLoopState::new(game.players_in_game());
    struct Answers;
    impl crate::decision::DecisionMaker for Answers {}
    let mut dm = Answers;
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
            source,
            ability_index: 0,
        }),
        &mut dm,
    )
    .unwrap();
    assert!(state.pending_mana_ability.is_some());
    assert!(game.has_active_mana_ability_on(source));
    assert!(!choices(&game, source).contains(&(source, 0)));
    assert!(choices(&game, source).contains(&(source, 1)));
    let mut branch = game.clone();
    let mut branch_state = state.clone();
    let mut branch_queue = queue.clone();
    crate::game_loop::apply_priority_response_with_dm(
        &mut branch,
        &mut branch_queue,
        &mut branch_state,
        &crate::PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Cancel),
        &mut dm,
    )
    .unwrap();
    assert!(branch.active_mana_ability_locks().is_empty());
    assert!(game.has_active_mana_ability_on(source));
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Activate {
            source,
            ability_index: 1,
        }),
        &mut dm,
    )
    .unwrap();
    let payment = state
        .pending_mana_ability
        .as_ref()
        .unwrap()
        .pending_mana_payment
        .as_ref()
        .unwrap();
    let confirm = ManaPaymentResponse::Confirm {
        plan_id: payment.plan.id,
        request_hash: payment.plan.request_hash,
    };
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::PriorityResponse::ManaPaymentPlan(confirm),
        &mut dm,
    )
    .unwrap();
    assert!(game.active_mana_ability_locks().is_empty());
    assert!(state.pending_mana_ability.is_none());
    assert_eq!(
        game.player(PlayerId::from_index(0))
            .unwrap()
            .mana_pool
            .green,
        2
    );
    assert!(game.is_tapped(source));
    game.validate_zone_consistency().unwrap();
}
