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
            Self::PlayLand { .. } | Self::TurnConspiracyFaceUp { .. }
            // Mana abilities already own a nested, interactive cost transaction.
            | Self::ActivateManaAbility { .. } => return Ok(None),
            Self::Plot { card_id } => (card_id, TotalCost::mana(plot_cost(game.object(card_id).ok_or(ActionError::ObjectNotFound)?).ok_or(ActionError::NoSuchAbility)?), PaymentReason::Other),
            Self::Suspend { card_id } => (card_id, TotalCost::mana(suspend_spec(game.object(card_id).ok_or(ActionError::ObjectNotFound)?).ok_or(ActionError::NoSuchAbility)?.1), PaymentReason::Other),
            Self::Foretell { card_id } => (card_id, TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])), PaymentReason::Other),
            Self::Companion { card_id } => (card_id, TotalCost::mana(companion_action_cost()), PaymentReason::Other),
            Self::TurnFaceUp { permanent_id, method } => {
                let object = game.object(permanent_id).ok_or(ActionError::ObjectNotFound)?;
                let spec = turn_face_up_spec(game, object, method).ok_or(ActionError::NoSuchAbility)?;
                (permanent_id, adjusted_turn_face_up_cost(game, player, permanent_id, &spec), PaymentReason::TurnFaceUp)
            }
            Self::UnlockRoomDoor { room_id } => (room_id, adjusted_room_unlock_cost(game, player, room_id)?, PaymentReason::UnlockDoor),
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
        return branches
            .iter()
            .any(|cost| {
                check_special_action_payment(
                    game,
                    player,
                    &SpecialActionPayment {
                        source: payment.source,
                        cost: cost.clone(),
                        reason: payment.reason,
                    },
                )
                .is_ok()
            })
            .then_some(())
            .ok_or(ActionError::CantPayCost);
    }
    // Pure mana totals have a fast existential query. Combine components so a
    // single source cannot be counted independently for two mana costs.
    if let ironsmith_core::TotalCostKind::All(components) = payment.cost.kind() {
        let mana = components
            .iter()
            .map(|c| c.mana_cost_ref())
            .collect::<Option<Vec<_>>>();
        if let Some(costs) = mana {
            let mut pips = Vec::new();
            for cost in costs {
                pips.extend(
                    game.adjust_mana_cost_for_payment_reason(
                        player,
                        Some(payment.source),
                        cost,
                        payment.reason,
                    )
                    .pips()
                    .iter()
                    .cloned(),
                );
            }
            let mut request = crate::mana_payment::ManaPaymentRequest::new(
                player,
                payment.source,
                payment.reason,
                ManaCost::from_pips(pips),
            )
            .with_spend_policy(game.mana_spend_policy(player, Some(payment.source)));
            request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
                && game.player_can_pay_black_with_life_for_reason(
                    player,
                    Some(payment.source),
                    payment.reason,
                );
            return crate::mana_payment::check_mana_payment(game, &request)
                .map_err(|_| ActionError::CantPayCost);
        }
    }
    // Reuse the choice-aware total-cost interpreter on a clone for non-mana
    // components; earlier costs change what is available for later components.
    pay_special_action_payment(
        &mut game.clone(),
        player,
        payment,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
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
            let mut pips = Vec::new();
            for component in components {
                if let Some(mana) = component.mana_cost_ref() {
                    pips.extend(mana.pips().iter().cloned());
                } else {
                    if !pips.is_empty() {
                        normalized.push(crate::costs::Cost::mana(ManaCost::from_pips(
                            std::mem::take(&mut pips),
                        )));
                    }
                    normalized.push(component.clone());
                }
            }
            if !pips.is_empty() {
                normalized.push(crate::costs::Cost::mana(ManaCost::from_pips(pips)));
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
    let provenance = game.provenance_graph_mut().alloc_root(
        crate::provenance::ProvenanceNodeKind::EffectExecution {
            source: payment.source,
            controller: player,
        },
    );
    let mut ctx = CostContext::new(payment.source, player, dm)
        .with_reason(payment.reason)
        .with_provenance(provenance);
    ctx.interactive_mana_exclusions = Some(Vec::new());
    pay_total_cost_without_preflight_with_choice(
        game,
        &normalized_payment_cost(&payment.cost),
        &mut ctx,
    )
    .map(|_| ())
    .map_err(cost_error_to_action_error)
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
            let actions = crate::decision::compute_legal_actions(&game, player);
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
