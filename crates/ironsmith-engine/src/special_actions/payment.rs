//! One payment contract for every paid special action. Eligibility and outcomes
//! stay mechanic-specific; affordability, prompting and rollback are shared.
use super::*;

pub(super) struct SpecialActionPayment {
    pub source: ObjectId,
    pub cost: crate::cost::TotalCost,
    pub reason: crate::costs::PaymentReason,
}

impl SpecialAction {
    /// No wildcard arm: adding an action requires declaring its payment route.
    pub(super) fn payment_spec(
        &self,
        game: &GameState,
        player: PlayerId,
    ) -> Result<Option<SpecialActionPayment>, ActionError> {
        use crate::cost::TotalCost;
        use crate::costs::PaymentReason;
        let (source, cost, reason) = match *self {
            Self::PlayLand { .. } | Self::PlayLandBackFace { .. } | Self::TurnConspiracyFaceUp { .. }
            // Mana abilities already own a nested, interactive cost transaction.
            | Self::ActivateManaAbility { .. } => return Ok(None),
            Self::Plot { card_id } => (card_id, TotalCost::mana(plot_cost(game.object(card_id).ok_or(ActionError::ObjectNotFound)?).ok_or(ActionError::NoSuchAbility)?), PaymentReason::Other),
            Self::Suspend { card_id } => (card_id, TotalCost::mana(suspend_spec(game.object(card_id).ok_or(ActionError::ObjectNotFound)?).ok_or(ActionError::NoSuchAbility)?.1), PaymentReason::Other),
            Self::Foretell { card_id } => (card_id, TotalCost::mana(foretell_special_action_quote(game, player, card_id)?.0), PaymentReason::Foretell),
            Self::Companion { card_id } => (card_id, TotalCost::mana(companion_action_cost()), PaymentReason::Other),
            Self::TurnFaceUp { permanent_id, method } => {
                let object = game.object(permanent_id).ok_or(ActionError::ObjectNotFound)?;
                let spec = turn_face_up_spec(game, object, method).map_err(|error| ActionError::ExecutionFailure {
                    source: permanent_id, error: crate::effects::ExecutionError::ContinuousDiscovery(error) })?
                    .ok_or(ActionError::NoSuchAbility)?;
                (permanent_id, adjusted_turn_face_up_cost(game, player, permanent_id, &spec), method.payment_reason())
            }
            Self::UnlockRoomDoor { room_id, door } => (room_id, adjusted_room_unlock_cost(game, player, room_id, door)?, PaymentReason::UnlockDoor),
            Self::RollPlanarDie => {
                let source = game.face_up_planar_objects().first().copied().ok_or(ActionError::InvalidTiming)?;
                (source, TotalCost::mana(planar_die_cost(game.planar_die_roll_cost(player).ok_or(ActionError::InvalidTiming)?)), PaymentReason::Other)
            }
            Self::IgnoreAttachedRestriction { source_id, .. } => (source_id, ignore_attached_restriction_cost(), PaymentReason::Other),
            Self::IgnoreSourceEffect { source_id, ability_index } => (source_id, TotalCost::mana(ignore_source_effect_mana_cost(game, source_id, ability_index)?), PaymentReason::Other),
            Self::PayDelayedTrigger { delayed_trigger_index } => {
                let payment = delayed_trigger_prepayment(game, delayed_trigger_index)?;
                (payment.source, payment.cost.clone(), PaymentReason::Other)
            }
            Self::PerformRepeatableManaPaymentAction { action_index } => {
                let action = repeatable_mana_payment_action(game, player, action_index)?;
                (action.source, TotalCost::mana(action.cost.clone()), PaymentReason::Other)
            }
        };
        Ok(Some(SpecialActionPayment {
            source,
            cost,
            reason,
        }))
    }
}

pub(super) fn check_special_action_payment(
    game: &GameState,
    player: PlayerId,
    payment: &SpecialActionPayment,
) -> Result<(), ActionError> {
    if let ironsmith_core::TotalCostKind::OneOf(branches) = payment.cost.kind() {
        for cost in branches {
            match check_special_action_payment(
                game,
                player,
                &SpecialActionPayment {
                    source: payment.source,
                    cost: cost.clone(),
                    reason: payment.reason,
                },
            ) {
                Ok(()) => return Ok(()),
                Err(error @ ActionError::ExecutionFailure { .. }) => return Err(error),
                Err(_) => {}
            }
        }
        return Err(ActionError::CantPayCost);
    }
    // Pure mana totals have a fast existential query. Combine components so a
    // single source cannot be counted independently for two mana costs.
    if let ironsmith_core::TotalCostKind::All(components) = payment.cost.kind() {
        let mana = components
            .iter()
            .map(|c| c.mana_cost_ref())
            .collect::<Option<Vec<_>>>();
        if let Some(costs) = mana {
            let mut combined = ManaCost::new();
            for cost in costs {
                let adjusted = game.adjust_mana_cost_for_payment_reason(
                    player,
                    Some(payment.source),
                    cost,
                    payment.reason,
                );
                combined = crate::decision::add_mana_cost(&combined, &adjusted);
            }
            let mut request = crate::mana_payment::ManaPaymentRequest::new(
                player,
                payment.source,
                payment.reason,
                combined,
            )
            .with_spend_policy(game.mana_spend_policy_for_reason(
                player,
                Some(payment.source),
                payment.reason,
            ));
            request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
                && game.player_can_pay_black_with_life_for_reason(
                    player,
                    Some(payment.source),
                    payment.reason,
                );
            return crate::mana_payment::check_mana_payment(game, &request).map_err(|error| {
                match error {
                    crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error) => {
                        game.record_token_resource_failure(&error);
                        ActionError::ExecutionFailure {
                            source: payment.source,
                            error,
                        }
                    }
                    _ => ActionError::CantPayCost,
                }
            });
        }
    }
    // Reuse the choice-aware total-cost interpreter on a clone for non-mana
    // components; earlier costs change what is available for later components.
    check_special_action_payment_with_snapshot(game, player, payment, None)
}

/// Preserve the existing sequential resource simulation while keeping a
/// prospective reveal distinct from completed public disclosure. This owner
/// creates the isolated game and never returns its hypothetical state.
pub(super) fn check_special_action_payment_with_snapshot(
    game: &GameState,
    player: PlayerId,
    payment: &SpecialActionPayment,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<(), ActionError> {
    let mut preview = game.clone();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let provenance = preview.provenance_graph_mut().alloc_root(
        crate::provenance::ProvenanceNodeKind::EffectExecution {
            source: payment.source,
            controller: player,
        },
    );
    let mut ctx = CostContext::new(payment.source, player, &mut dm)
        .with_reason(payment.reason)
        .with_provenance(provenance);
    ctx.source_snapshot = snapshot;
    ctx.interactive_mana_exclusions = Some(Vec::new());
    ctx.prospective_cost_payment = true;
    pay_total_cost_without_preflight_with_choice(
        &mut preview,
        &normalized_payment_cost(&payment.cost),
        &mut ctx,
    )
    .map(|_| ())
    .map_err(|error| cost_error_to_action_error(error, payment.source))
}

/// Pay a cost a resolving spell or ability demands (ward, for one). The payer
/// may activate mana abilities while paying (CR 605.3a), so the check counts
/// untapped sources and the payment goes through the interactive mana route.
pub(crate) fn pay_resolution_cost_with_snapshot(
    game: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    cost: &crate::cost::TotalCost,
    reason: crate::costs::PaymentReason,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    dm: &mut dyn DecisionMaker,
) -> bool {
    let result = pay_resolution_cost_with_outputs(game, player, source, cost, reason, snapshot, dm);
    if let Err(ActionError::ExecutionFailure { error, .. }) = &result {
        // This legacy boolean adapter participates in its caller's checked
        // execution scope. Unknown payment must not become an unpaid ward.
        game.record_token_resource_failure(error);
    }
    result.is_ok()
}

/// Resolution payment keeps the native owner's actual child receipts and typed
/// failures. A pending funding prefix is not a completed payment; callers must
/// inspect awaiting_choice before acknowledging or starting a consequence.
pub(crate) fn pay_resolution_cost_with_outputs(
    game: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    cost: &crate::cost::TotalCost,
    reason: crate::costs::PaymentReason,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    dm: &mut dyn DecisionMaker,
) -> Result<CompletedCostPayment, ActionError> {
    let payment = SpecialActionPayment {
        source,
        cost: cost.clone(),
        reason,
    };
    check_special_action_payment_with_snapshot(game, player, &payment, snapshot.clone())?;
    pay_special_action_payment_with_x_and_outputs(game, player, &payment, snapshot, None, dm)
}

/// Adjacent mana components are one payment, so paying a generic component
/// cannot consume the only color needed by the next component. Preserve the
/// order of non-mana costs and normalize alternative branches recursively.
fn normalized_payment_cost(cost: &crate::cost::TotalCost) -> crate::cost::TotalCost {
    use crate::cost::TotalCost;
    match cost.kind() {
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            TotalCost::one_of(branches.iter().map(normalized_payment_cost).collect())
        }
        ironsmith_core::TotalCostKind::All(components) => {
            let mut normalized = Vec::new();
            let mut mana = ManaCost::new();
            for component in components {
                if let Some(part) = component.mana_cost_ref() {
                    mana = crate::decision::add_mana_cost(&mana, part);
                } else {
                    if !mana.is_empty() {
                        normalized.push(crate::costs::Cost::mana(std::mem::take(&mut mana)));
                    }
                    normalized.push(component.clone());
                }
            }
            if !mana.is_empty() {
                normalized.push(crate::costs::Cost::mana(mana));
            }
            TotalCost::from_costs(normalized)
        }
    }
}

pub(super) fn pay_special_action_payment(
    game: &mut GameState,
    player: PlayerId,
    payment: &SpecialActionPayment,
    dm: &mut dyn DecisionMaker,
) -> Result<(), ActionError> {
    pay_special_action_payment_with_snapshot(game, player, payment, None, dm)
}

pub(super) fn pay_special_action_payment_with_snapshot(
    game: &mut GameState,
    player: PlayerId,
    payment: &SpecialActionPayment,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    dm: &mut dyn DecisionMaker,
) -> Result<(), ActionError> {
    pay_special_action_payment_with_x(game, player, payment, snapshot, None, dm)
}

/// The largest X the player could pay for this special action's mana cost,
/// or `None` when that cost has no X.
pub(super) fn special_action_payment_max_x(
    game: &GameState,
    player: PlayerId,
    payment: &SpecialActionPayment,
) -> Option<u32> {
    let cost = payment.cost.mana_cost()?;
    if !cost.has_x() {
        return None;
    }
    let policy = game.mana_spend_policy(player, Some(payment.source));
    let allow_black_life = crate::decision::mana_cost_has_black_symbol(cost)
        && game.player_can_pay_black_with_life_for_reason(
            player,
            Some(payment.source),
            payment.reason,
        );
    Some(
        crate::decision::compute_potential_mana(game, player)
            .max_x_for_cost_with_mana_spend_policy_and_black_life(cost, &policy, allow_black_life),
    )
}

pub(super) fn pay_special_action_payment_with_x(
    game: &mut GameState,
    player: PlayerId,
    payment: &SpecialActionPayment,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    x_value: Option<u32>,
    dm: &mut dyn DecisionMaker,
) -> Result<(), ActionError> {
    pay_special_action_payment_with_x_and_outputs(game, player, payment, snapshot, x_value, dm)
        .map(|_| ())
}

fn pay_special_action_payment_with_x_and_outputs(
    game: &mut GameState,
    player: PlayerId,
    payment: &SpecialActionPayment,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    x_value: Option<u32>,
    dm: &mut dyn DecisionMaker,
) -> Result<CompletedCostPayment, ActionError> {
    let provenance = game.provenance_graph_mut().alloc_root(
        crate::provenance::ProvenanceNodeKind::EffectExecution {
            source: payment.source,
            controller: player,
        },
    );
    let mut ctx = CostContext::new(payment.source, player, dm)
        .with_reason(payment.reason)
        .with_provenance(provenance);
    ctx.source_snapshot = snapshot;
    ctx.x_value = x_value;
    ctx.interactive_mana_exclusions = Some(Vec::new());
    pay_total_cost_without_preflight_with_outputs(
        game,
        &normalized_payment_cost(&payment.cost),
        &mut ctx,
    )
    .map_err(|error| cost_error_to_action_error(error, payment.source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::mana_payment::ManaPaymentResponse;

    fn setup(kind: usize) -> (GameState, PlayerId, ObjectId, SpecialAction) {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = player;
        game.turn.priority_player = Some(player);
        let builder = CardDefinitionBuilder::new(CardId::new(), "Payment Probe")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new().add_generic(1));
        let definition = match kind {
            0 => builder.plot(ManaCost::new().add_generic(1)),
            1 => builder.foretell(ManaCost::new().add_generic(1)),
            2 => builder.suspend(2, ManaCost::new().add_generic(1)),
            _ => builder,
        }
        .build();
        let card = game.create_object_from_definition(
            &definition,
            player,
            if kind == 3 {
                Zone::OutsideGame
            } else {
                Zone::Hand
            },
        );
        let action = match kind {
            0 => SpecialAction::Plot { card_id: card },
            1 => SpecialAction::Foretell { card_id: card },
            2 => SpecialAction::Suspend { card_id: card },
            _ => {
                game.player_mut(player).unwrap().companion = Some(card);
                SpecialAction::Companion { card_id: card }
            }
        };
        for _ in 0..3 {
            let land = CardDefinitionBuilder::new(CardId::new(), "Payment Land")
                .card_types(vec![CardType::Land])
                .build();
            let id = game.create_object_from_definition(&land, player, Zone::Battlefield);
            game.object_mut(id)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![ManaSymbol::Colorless],
                ));
        }
        (game, player, card, action)
    }

    #[derive(Default)]
    struct PaymentDm {
        prompts: usize,
        pending: bool,
        cancel_after_activation: bool,
    }
    impl DecisionMaker for PaymentDm {
        fn awaiting_choice(&self) -> bool {
            self.pending && self.prompts > 0
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.prompts += 1;
            if self.pending {
                return ManaPaymentResponse::Cancel;
            }
            if self.cancel_after_activation {
                if self.prompts == 1 {
                    let (source, ability_index) =
                        crate::mana_payment::manual_mana_abilities(game, &ctx.request)[0];
                    return ManaPaymentResponse::Activate {
                        source,
                        ability_index,
                    };
                }
                return ManaPaymentResponse::Cancel;
            }
            ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }
    }

    #[test]
    fn paid_special_actions_offer_and_pay_with_untapped_sources() {
        for kind in 0..4 {
            let (mut game, player, card, action) = setup(kind);
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
            let actions = crate::decision::compute_legal_actions(&game, player).expect("fixture has complete replacement state");
            assert!(
                actions.iter().any(
                    |a| matches!(a, crate::decision::LegalAction::SpecialAction(s) if s == &action)
                ),
                "kind {kind}: special action missing"
            );
            if kind < 3 {
                assert!(actions.iter().any(|a| matches!(a, crate::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == card)), "normal cast must remain a separate choice");
            }
            let mut dm = PaymentDm::default();
            perform(action, &mut game, player, &mut dm).unwrap();
            assert_eq!(dm.prompts, 1);
            assert!(
                game.object(card).is_none(),
                "zone move replaces the object id"
            );
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter(|id| game.is_tapped(**id))
                    .count(),
                [1, 2, 1, 3][kind]
            );
            if kind < 3 {
                assert_eq!(game.exile.len(), 1);
            } else {
                assert!(game.player(player).unwrap().companion_special_action_used);
            }
        }
    }

    #[test]
    fn pending_payment_and_cancellation_do_not_perform_the_special_action() {
        for kind in 0..4 {
            let (mut game, player, card, action) = setup(kind);
            let zone = game.object(card).unwrap().zone;
            let mut pending = PaymentDm {
                pending: true,
                ..Default::default()
            };
            let _ = perform(action.clone(), &mut game, player, &mut pending);
            assert!(pending.awaiting_choice());
            assert_eq!(game.object(card).unwrap().zone, zone);
            assert!(game.exile.is_empty());
            assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
            let mut cancel = PaymentDm {
                cancel_after_activation: true,
                ..Default::default()
            };
            assert_eq!(
                perform(action, &mut game, player, &mut cancel),
                Err(ActionError::Cancelled)
            );
            assert_eq!(cancel.prompts, 2);
            assert_eq!(game.object(card).unwrap().zone, zone);
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
            assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
        }
    }

    #[test]
    fn cancelled_payment_completes_the_priority_interaction_cleanly() {
        let (mut game, player, card, action) = setup(0);
        let mut queue = crate::triggers::TriggerQueue::new();
        let mut state = crate::game_loop::PriorityLoopState::new(2);
        let mut dm = PaymentDm {
            cancel_after_activation: true,
            ..Default::default()
        };
        crate::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &crate::game_loop::PriorityResponse::PriorityAction(
                crate::decision::LegalAction::SpecialAction(action),
            ),
            &mut dm,
        )
        .expect("cancel must return to priority, not report a replay failure");
        assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
    }

    #[test]
    fn non_mana_payment_may_sacrifice_the_previously_attached_object() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.priority_player = Some(bob);
        let creature = CardDefinitionBuilder::new(CardId::new(), "Attached Cost Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let attached = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let aura = CardDefinitionBuilder::new(CardId::new(), "Restriction Cost Probe")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
        game.object_mut(source).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::from_model(
                ironsmith_core::StaticAbility::attached_controller_may_sacrifice_permanent_to_ignore_source_effect_until_end_of_turn("Sacrifice a permanent to ignore this effect."),
            ),
        ));
        assert!(
            game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(attached))
        );
        let action = SpecialAction::IgnoreAttachedRestriction {
            source_id: source,
            ability_index: 0,
        };
        assert!(can_perform_check(&action, &game, bob).is_ok());
        perform(
            action,
            &mut game,
            bob,
            &mut crate::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(
            game.object(attached).is_none(),
            "the sole sacrifice candidate paid the cost"
        );
    }

    #[test]
    fn generic_payment_supports_new_cost_shapes_without_mechanic_dispatch() {
        let (mut game, player, card, _) = setup(0);
        let payment = SpecialActionPayment {
            source: card,
            reason: crate::costs::PaymentReason::Other,
            cost: crate::cost::TotalCost::one_of(vec![
                crate::cost::TotalCost::mana(ManaCost::new().add_generic(9)),
                crate::cost::TotalCost::from_costs(vec![
                    crate::costs::Cost::mana(ManaCost::new().add_generic(1)),
                    crate::costs::Cost::mana(ManaCost::new().add_generic(2)),
                ]),
            ]),
        };
        assert!(check_special_action_payment(&game, player, &payment).is_ok());
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
        let mut dm = PaymentDm::default();
        pay_special_action_payment(&mut game, player, &payment, &mut dm).unwrap();
        assert_eq!(dm.prompts, 1, "adjacent mana components share one payment");
        assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
        assert!(game.battlefield.iter().all(|id| game.is_tapped(*id)));
        assert!(check_special_action_payment(&game, player, &payment).is_err());
    }
}

#[cfg(test)]
mod reveal_admission_tests {
    use super::*;

    #[derive(Default)]
    struct RevealAnswer { chosen: Option<ObjectId>, views: usize }
    impl DecisionMaker for RevealAnswer {
        fn decide_objects(&mut self, _: &GameState, _: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            self.chosen.into_iter().collect()
        }
        fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[ObjectId], _: &crate::decisions::context::ViewCardsContext) {
            self.views += 1;
        }
    }

    #[test]
    fn compound_and_alternative_reveal_admission_does_not_open_or_complete_actual_payment() {
        for alternatives in [false, true] {
            let player = PlayerId(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Admission source")
                .card_types(vec![crate::CardType::Artifact]).build();
            let source = game.create_object_from_definition(&definition, player, Zone::Battlefield);
            let selected = game.create_hidden_card_placeholder(player, Zone::Hand, 0, "cost-admission".into());
            let reveal = crate::costs::Cost::reveal_from_hand_with_color_filter(1, None, Some(crate::color::ColorSet::RED));
            let branch = crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::life(2), reveal]);
            let cost = if alternatives { crate::cost::TotalCost::one_of(vec![branch,
                crate::cost::TotalCost::from_cost(crate::costs::Cost::life(30))]) } else { branch };
            let payment = SpecialActionPayment { source, cost: cost.clone(), reason: crate::costs::PaymentReason::TurnFaceUp };
            assert!(check_special_action_payment(&game, player, &payment).is_ok());
            assert_eq!(game.player(player).unwrap().life, 20);
            assert!(!game.is_publicly_revealed_hidden_card(selected));
            let mut dm = RevealAnswer { chosen: Some(selected), ..Default::default() };
            assert!(matches!(pay_special_action_payment(&mut game, player, &payment, &mut dm),
                Err(ActionError::ExecutionFailure { error: crate::effects::ExecutionError::IncompleteEvidence(_), .. })));
            assert_eq!(game.player(player).unwrap().life, 20);
            assert_eq!(dm.views, 0);
            assert!(!game.is_publicly_revealed_hidden_card(selected));
            // Ward uses the same cloned admission owner, then performs a real
            // cost. Prospective success must not masquerade as actual payment.
            assert!(!pay_resolution_cost_with_snapshot(&mut game, player, source, &cost,
                crate::costs::PaymentReason::Effect, None, &mut dm));
            assert_eq!(game.player(player).unwrap().life, 20);
            let opened = crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Opened red card")
                .card_types(vec![crate::CardType::Instant]).color_indicator(crate::color::ColorSet::RED).build();
            game.reveal_hidden_card_with_definition(selected, &opened).unwrap();
            assert!(pay_resolution_cost_with_snapshot(&mut game, player, source, &cost,
                crate::costs::PaymentReason::Effect, None, &mut dm));
            assert_eq!(game.player(player).unwrap().life, 18);
            assert_eq!(dm.views, 2, "only the actual payment discloses to both players");
            assert_eq!(game.object(selected).unwrap().zone, Zone::Hand);
        }
    }
}
