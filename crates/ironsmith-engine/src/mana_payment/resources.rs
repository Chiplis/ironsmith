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
            symbol,
            source: self.event.source,
            source_chosen_creature_type: self.context.chosen_creature_type,
            restrictions: self.context.restrictions.clone(),
        }
    }

    pub fn spendable_units(&self, game: &GameState, request: &super::ManaPaymentRequest) -> Vec<PaymentManaUnit> {
        if self.event.player != request.payer { return Vec::new(); }
        let snow = self.event.snapshot.as_ref().map_or_else(
            || game.current_has_supertype(self.event.source, crate::types::Supertype::Snow),
            |snapshot| snapshot.supertypes.contains(&crate::types::Supertype::Snow),
        );
        self.event.mana.iter().copied().filter(|symbol| {
            self.context.restrictions.is_empty() || game.restricted_mana_unit_is_payable_for_transaction(
                &self.restricted_unit(*symbol), Some(request.source), request.reason, Some(&request.cost),
            )
        }).map(|symbol| PaymentManaUnit { symbol, snow }).collect()
    }

    /// The native event owner has already validated replacements. Both native
    /// execution and projected credits use this same provenance/context shape.
    pub fn commit(&self, game: &mut GameState) -> Result<(), crate::effects::ExecutionError> {
        game.with_player_mana_mut(self.event.player, |player| {
            for &symbol in &self.event.mana {
                if self.context.restrictions.is_empty() {
                    player.add_unrestricted_mana_with_retention(symbol, self.event.source,
                        self.event.snapshot.clone(), self.context.retention);
                } else {
                    player.add_restricted_mana_with_snapshot_and_retention(self.restricted_unit(symbol),
                        self.event.snapshot.clone(), self.context.retention);
                }
            }
        }).ok_or(crate::effects::ExecutionError::PlayerNotFound(self.event.player))
    }
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
    use crate::{CardBuilder, Zone};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::{CardType, Supertype};

    #[test]
    fn credit_preserves_recipient_retention_and_production_time_snow() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Snow credit source")
            .card_types(vec![CardType::Land]).supertypes(vec![Supertype::Snow]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let credit = ManaCredit {
            event: ManaAddedEvent::new(source, alice, bob, vec![ManaSymbol::Blue])
                .with_snapshot(Some(snapshot)),
            context: ManaCreditContext {
                retention: Some(ironsmith_core::ManaRetentionDuration::EndOfTurn),
                ..Default::default()
            },
        };
        let request = super::super::ManaPaymentRequest::new(bob, source,
            crate::costs::PaymentReason::Effect, ManaCost::from_pips(vec![vec![ManaSymbol::Snow]]));
        let projected = credit.spendable_units(&game, &request);
        assert_eq!(projected, vec![PaymentManaUnit { symbol: ManaSymbol::Blue, snow: true }]);
        let mut wrong_recipient = request.clone();
        wrong_recipient.payer = alice;
        assert!(credit.spendable_units(&game, &wrong_recipient).is_empty());
        credit.commit(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(game.payment_mana_units(&request), projected);
        let provenance = &game.player(bob).unwrap().mana_source_provenance[0];
        assert_eq!(provenance.source, source);
        assert_eq!(provenance.retention, Some(ironsmith_core::ManaRetentionDuration::EndOfTurn));
        assert!(provenance.snapshot.as_ref().unwrap().supertypes.contains(&Supertype::Snow));
    }
}
