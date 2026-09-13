//! Reveal a source-linked secret subtype as a payment action.
use crate::effect::EffectOutcome;
use crate::effects::{
    CostExecutableEffect, CostValidationError, EffectExecutor, ExecutionContext, ExecutionError,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::types::Subtype;
pub use ironsmith_core::RevealChosenSubtypeEffect;

fn choice(game: &GameState, source: ObjectId, chooser: PlayerId) -> Option<Subtype> {
    if game.object(source).is_some() {
        return game.secret_chosen_subtype(source, chooser);
    }
    // Costs can be paid in either order. Sacrifice records the exact departed
    // object's snapshot; it must not redirect the choice to a new incarnation.
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .rev()
        .filter_map(|record| {
            record
                .event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
        })
        .flat_map(|event| event.snapshots())
        .find(|snapshot| snapshot.object_id == source)
        .and_then(|snapshot| snapshot.secret_chosen_subtype)
        .filter(|(player, _)| *player == chooser)
        .map(|(_, subtype)| subtype)
}
impl EffectExecutor for RevealChosenSubtypeEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let subtype = choice(game, ctx.source, ctx.controller).ok_or_else(|| {
            ExecutionError::Impossible(
                "no subtype secretly chosen by this player for this source".into(),
            )
        })?;
        game.set_chosen_subtype(ctx.source, subtype);
        Ok(EffectOutcome::count(1))
    }
}
impl CostExecutableEffect for RevealChosenSubtypeEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        choice(game, source, controller).map(|_| ()).ok_or_else(|| {
            CostValidationError::Other(
                "no subtype secretly chosen by this player for this source".into(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::ids::CardId;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn reveal_subtype_payment_preserves_chooser_and_departed_source_identity() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Secret choice fixture")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let effect = RevealChosenSubtypeEffect;
        assert!(CostExecutableEffect::can_execute_as_cost(&effect, &game, source, alice).is_err());
        game.set_secret_chosen_subtype(source, alice, Subtype::Human);
        game.set_secret_chosen_subtype(other, alice, Subtype::Goblin);
        assert!(CostExecutableEffect::can_execute_as_cost(&effect, &game, source, alice).is_ok());
        assert!(CostExecutableEffect::can_execute_as_cost(&effect, &game, source, bob).is_err());
        let departed = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        for event in game.take_pending_trigger_events() {
            game.turn_store
                .turn_history
                .record_event(&event, None, None);
        }
        assert_eq!(game.secret_chosen_subtype(departed, alice), None);
        assert!(CostExecutableEffect::can_execute_as_cost(&effect, &game, departed, alice).is_err());
        assert!(CostExecutableEffect::can_execute_as_cost(&effect, &game, source, bob).is_err());
        effect
            .execute(
                &mut game,
                &mut ExecutionContext::new(source, alice, &mut SelectFirstDecisionMaker),
            )
            .unwrap();
        assert_eq!(game.chosen_subtype(source), Some(Subtype::Human));
        assert_eq!(game.chosen_subtype(departed), None);
        assert_eq!(game.chosen_subtype(other), None);
        assert_eq!(
            game.secret_chosen_subtype(other, alice),
            Some(Subtype::Goblin)
        );
    }
}
