//! Production metadata shared by compact evaluation and authoritative credit.
//! Keep metadata on each event: a triggered bonus is a new production context,
//! even when its source happens to be the permanent that was just activated.
use crate::ability::{ActivatedAbility, ManaUsageRestriction, RestrictedManaUnit};
use crate::effects::ExecutionContext;
use crate::events::ManaAddedEvent;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::mana::ManaSymbol;

#[derive(Clone, Debug, Default)]
pub(crate) struct ManaCreditContext {
    pub restrictions: Vec<ManaUsageRestriction>,
    pub chosen_creature_type: Option<crate::types::Subtype>,
    pub retention: Option<ironsmith_core::ManaRetentionDuration>,
}

impl ManaCreditContext {
    pub fn from_execution(ctx: &ExecutionContext) -> Self {
        Self {
            restrictions: ctx.mana.mana_usage_restrictions.clone(),
            chosen_creature_type: ctx.mana.mana_source_chosen_creature_type,
            retention: ctx.mana.retention,
        }
    }

    pub fn from_activation(game: &GameState, source: ObjectId, ability: &ActivatedAbility) -> Self {
        Self {
            restrictions: ability.mana_usage_restrictions.clone(),
            chosen_creature_type: game.chosen_creature_type(source),
            retention: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ManaCredit {
    pub event: ManaAddedEvent,
    pub context: ManaCreditContext,
}

impl ManaCredit {
    fn restricted_unit(&self, symbol: ManaSymbol) -> RestrictedManaUnit {
        RestrictedManaUnit {
            source_controller: Some(self.event.controller),
            symbol,
            source: self.event.source,
            source_chosen_creature_type: self.context.chosen_creature_type,
            restrictions: self.context.restrictions.clone(),
        }
    }

    pub fn spendable_units(
        &self,
        game: &GameState,
        request: &super::ManaPaymentRequest,
    ) -> Vec<PaymentManaUnit> {
        if self.event.player != request.payer
            || !production_satisfies_cost(&request.cost, self.event.snapshot.as_ref())
        {
            return Vec::new();
        }
        let snow = self.event.snapshot.as_ref().map_or_else(
            || game.current_has_supertype(self.event.source, crate::types::Supertype::Snow),
            |snapshot| snapshot.supertypes.contains(&crate::types::Supertype::Snow),
        );
        self.event
            .mana
            .iter()
            .copied()
            .filter(|symbol| {
                self.context.restrictions.is_empty()
                    || game.restricted_mana_unit_is_payable_for_transaction(
                        &self.restricted_unit(*symbol),
                        Some(request.source),
                        request.reason,
                        Some(&request.cost),
                    )
            })
            .map(|symbol| PaymentManaUnit { symbol, snow })
            .collect()
    }

    /// The native event owner has already validated replacements. Both native
    /// execution and projected credits use this same provenance/context shape.
    pub fn commit(&self, game: &mut GameState) -> Result<(), crate::effects::ExecutionError> {
        let exact: u128 = game
            .players
            .iter()
            .map(|player| u128::from(player.mana_pool.total_wide()))
            .sum::<u128>()
            + self.event.mana.len() as u128;
        if exact > i32::MAX as u128 {
            return Err(crate::effects::ExecutionError::ResourceLimitExceeded {
                resource: "unspent mana scalar domain",
                requested: exact,
                maximum: i32::MAX as u128,
            });
        }
        crate::effects::composition::execute_world_error_transaction(
            game,
            |error| {
                matches!(
                    error,
                    crate::effects::ExecutionError::ContinuousDiscovery(_)
                )
            },
            |game| {
                game.with_player_mana_mut(self.event.player, |player| {
                    for &symbol in &self.event.mana {
                        if self.context.restrictions.is_empty() {
                            player.add_unrestricted_mana_with_retention(
                                symbol,
                                self.event.source,
                                self.event.snapshot.clone(),
                                self.context.retention,
                            );
                        } else {
                            player.add_restricted_mana_with_snapshot_and_retention(
                                self.restricted_unit(symbol),
                                self.event.snapshot.clone(),
                                self.context.retention,
                            );
                        }
                    }
                })
                .ok_or(crate::effects::ExecutionError::PlayerNotFound(
                    self.event.player,
                ))?;
                // A representable count may still exceed P/T after its source's base
                // stats and other layer-7 modifiers. Validate without publishing a
                // partially recalculated board or an impossible completed production.
                game.try_all_continuous_effects()
                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
                Ok(())
            },
        )
    }
}

/// Evaluate production characteristics, never the producer's later state.
/// Untracked pool units have no evidence and cannot satisfy a source condition.
pub(crate) fn production_satisfies_cost(
    cost: &crate::mana::ManaCost,
    snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> bool {
    fn matches(
        filter: &ironsmith_core::mana::ManaProducerFilter,
        snapshot: &crate::snapshot::ObjectSnapshot,
    ) -> bool {
        use ironsmith_core::mana::ManaProducerFilter;
        match filter {
            ManaProducerFilter::CardType(kind) => snapshot.card_types.contains(kind),
            ManaProducerFilter::Supertype(kind) => snapshot.supertypes.contains(kind),
            ManaProducerFilter::Subtype(kind) => snapshot.subtypes.contains(kind),
            ManaProducerFilter::All(parts) => parts.iter().all(|part| matches(part, snapshot)),
        }
    }
    cost.spending_restrictions().iter().all(|rule| match rule {
        // This restriction belongs to the final generic-pip allocation, not
        // to each unit offered for fixed/base/tax obligations.
        ironsmith_core::mana::ManaSpendingRestriction::OnX { .. } => true,
        ironsmith_core::mana::ManaSpendingRestriction::ProducedBy(filter) => {
            snapshot.is_some_and(|snapshot| {
                snapshot.zone == crate::zone::Zone::Battlefield && matches(filter, snapshot)
            })
        }
    })
}

/// A transaction-qualified unit. Full source/snapshot/restriction/retention
/// metadata stays on the owning credit or native pool while assignment uses
/// only these properties to match individual pips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PaymentManaUnit {
    pub symbol: ManaSymbol,
    pub snow: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::{CardType, Supertype};
    use crate::{CardBuilder, Zone};

    #[test]
    fn credit_preserves_recipient_retention_and_production_time_snow() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Snow credit source")
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Snow])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let credit = ManaCredit {
            event: ManaAddedEvent::new(source, alice, bob, vec![ManaSymbol::Blue])
                .with_snapshot(Some(snapshot)),
            context: ManaCreditContext {
                retention: Some(ironsmith_core::ManaRetentionDuration::EndOfTurn),
                ..Default::default()
            },
        };
        let request = super::super::ManaPaymentRequest::new(
            bob,
            source,
            crate::costs::PaymentReason::Effect,
            ManaCost::from_pips(vec![vec![ManaSymbol::Snow]]),
        );
        let projected = credit.spendable_units(&game, &request);
        assert_eq!(
            projected,
            vec![PaymentManaUnit {
                symbol: ManaSymbol::Blue,
                snow: true
            }]
        );
        let mut wrong_recipient = request.clone();
        wrong_recipient.payer = alice;
        assert!(credit.spendable_units(&game, &wrong_recipient).is_empty());
        credit.commit(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(game.payment_mana_units(&request), projected);
        let provenance = &game.player(bob).unwrap().mana_source_provenance[0];
        assert_eq!(provenance.source, source);
        assert_eq!(
            provenance.retention,
            Some(ironsmith_core::ManaRetentionDuration::EndOfTurn)
        );
        assert!(
            provenance
                .snapshot
                .as_ref()
                .unwrap()
                .supertypes
                .contains(&Supertype::Snow)
        );
    }
}

#[cfg(test)]
mod frozen_restriction_context_tests {
    use super::*;
    use crate::ability::{ManaPaymentPredicate, ManaPaymentPurpose};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::{CardBuilder, CardType, Zone};

    #[test]
    fn production_context_survives_departure_and_changed_control() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Mana context source")
            .card_types(vec![CardType::Land])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let spell = game.create_object_from_card(&card, bob, Zone::Stack);
        let credit = ManaCredit {
            event: ManaAddedEvent::new(source, alice, bob, vec![ManaSymbol::Blue]),
            context: ManaCreditContext {
                restrictions: vec![ManaUsageRestriction::PaymentTransaction {
                    restriction: Some(ManaPaymentPredicate::All(vec![
                        ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
                        ManaPaymentPredicate::SourceMatches(
                            ObjectFilter::default().owned_by(PlayerFilter::NotYou),
                        ),
                    ])),
                    on_spend: vec![],
                }],
                ..Default::default()
            },
        };
        let request = super::super::ManaPaymentRequest::new(
            bob,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::from_symbols(vec![ManaSymbol::Blue]),
        );
        credit.commit(&mut game).unwrap();
        let expected = vec![PaymentManaUnit {
            symbol: ManaSymbol::Blue,
            snow: false,
        }];
        assert_eq!(credit.spendable_units(&game, &request), expected);
        assert_eq!(game.payment_mana_units(&request), expected);
        game.set_current_controller(source, bob).unwrap();
        assert_eq!(
            credit.spendable_units(&game, &request),
            expected,
            "the producing ability was controlled by Alice, not Bob"
        );
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(
            game.payment_mana_units(&request),
            expected,
            "floating restricted mana remains usable after its source leaves"
        );
        assert_eq!(
            game.player(bob).unwrap().restricted_mana[0].source_controller,
            Some(alice)
        );
    }
}
