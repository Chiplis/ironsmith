//! Return from graveyard or exile to battlefield effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, PutOntoBattlefieldEffect};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};
pub use ironsmith_core::ReturnFromGraveyardOrExileToBattlefieldEffect;

/// Find a card in the graveyard or exile by stable identity.
fn find_by_stable_id(game: &GameState, owner: PlayerId, stable_id: StableId) -> Option<ObjectId> {
    let in_graveyard = game.player(owner).and_then(|p| {
        p.graveyard
            .iter()
            .find(|&&id| {
                game.object(id)
                    .is_some_and(|obj| obj.stable_id == stable_id)
            })
            .copied()
    });

    if in_graveyard.is_some() {
        return in_graveyard;
    }

    game.exile.iter().find_map(|&id| {
        game.object(id)
            .is_some_and(|obj| obj.stable_id == stable_id && obj.owner == owner)
            .then_some(id)
    })
}

impl EffectExecutor for ReturnFromGraveyardOrExileToBattlefieldEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_snapshot = game
            .object(ctx.source)
            .map(|source| (source.owner, source.stable_id));
        let event_snapshot = ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.snapshot())
            .map(|snapshot| (snapshot.owner, snapshot.stable_id));
        let Some((owner, stable_id)) = source_snapshot.or(event_snapshot) else {
            return Err(ExecutionError::Impossible(
                "graveyard-or-exile return requires source or triggering-object identity".into(),
            ));
        };

        let Some(target_id) = find_by_stable_id(game, owner, stable_id) else {
            return Ok(EffectOutcome::target_invalid());
        };

        let outcome = ctx.with_temp_targets(vec![ResolvedTarget::Object(target_id)], |ctx| {
            let put_effect = PutOntoBattlefieldEffect::new(
                ChooseSpec::SpecificObject(target_id),
                self.tapped,
                PlayerFilter::OwnerOf(ObjectRef::Specific(target_id)),
            );
            execute_effect(game, &Effect::new(put_effect), ctx)
        })?;

        let EffectOutcome {
            status,
            value,
            events,
            execution_facts,
        } = outcome;

        match status {
            // Preserve prior behavior: ETB prevented is treated as TargetInvalid.
            crate::effect::OutcomeStatus::Impossible => Ok(EffectOutcome::target_invalid()
                .with_events(events)
                .with_execution_facts(execution_facts)),
            other => Ok(EffectOutcome::with_details(
                other,
                value,
                events,
                execution_facts,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn source_return_executes_from_graveyard_and_exile_but_not_battlefield() {
        for (index, origin) in [Zone::Graveyard, Zone::Exile].into_iter().enumerate() {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let owner = PlayerId::from_index(0);
            let card = CardBuilder::new(CardId::from_raw(67_100 + index as u32), "Survivor")
                .card_types(vec![CardType::Creature])
                .build();
            let source = game.create_object_from_card(&card, owner, origin);
            let stable_id = game.object(source).expect("source").stable_id;
            let mut ctx = ExecutionContext::new_default(source, owner);

            let outcome = ReturnFromGraveyardOrExileToBattlefieldEffect::new(true)
                .execute(&mut game, &mut ctx)
                .expect("return should resolve");
            assert!(outcome.status.is_success(), "{origin:?}: {outcome:?}");
            let returned = game
                .battlefield
                .iter()
                .copied()
                .find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.stable_id == stable_id)
                })
                .expect("returned source");
            assert!(game.is_tapped(returned), "{origin:?}");
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let owner = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(67_199), "Resident")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&card, owner, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, owner);
        let outcome = ReturnFromGraveyardOrExileToBattlefieldEffect::new(true)
            .execute(&mut game, &mut ctx)
            .expect("near miss should resolve cleanly");
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::TargetInvalid);
    }
}
