//! One-source damage with every recipient-local amount captured before results.
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::{
    SimultaneousDamageEvent, with_deferred_prevention_follow_up_outcome,
};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
pub use ironsmith_core::DealDamageEachEffect;
impl EffectExecutor for DealDamageEachEffect {
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = self.execute_captured(game, ctx);
        let pending = ctx.decision_maker.awaiting_choice();
        if result.is_err() || pending {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && pending);
            context_checkpoint.restore(ctx);
        }
        if pending && result.is_ok() {
            return Ok(EffectOutcome::count(0));
        }
        result
    }
}
trait ExecuteCapturedDamage {
    fn execute_captured(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}
impl ExecuteCapturedDamage for DealDamageEachEffect {
    fn execute_captured(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let matching = crate::effects::helpers::resolve_objects_from_spec(
            game,
            &ChooseSpec::All(self.filter.clone()),
            ctx,
        )?
        .into_iter()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                && !game.is_phased_out(*id)
                && super::deal_damage::object_can_be_dealt_damage(game, *id)
        })
        .filter_map(|id| {
            game.object(id).map(|object| {
                (
                    id,
                    ObjectSnapshot::from_object_with_calculated_characteristics(object, game),
                )
            })
        })
        .collect::<Vec<_>>();
        let source_snapshot = game
            .object(ctx.source)
            .filter(|_| !game.is_phased_out(ctx.source))
            .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
            .or_else(|| {
                game.turn_store
                    .turn_history
                    .source_last_known_snapshot(ctx.source)
                    .cloned()
            })
            .or_else(|| ctx.source_snapshot.clone());
        let tag = crate::tag::TagKey::from("__it__");
        let previous = ctx.tagged_objects.remove(&tag);
        let prepared = (|| {
            let mut events = Vec::new();
            for (recipient, snapshot) in matching {
                ctx.set_tagged_objects(tag.clone(), vec![snapshot.clone()]);
                let amount = ctx
                    .with_temp_iterated_object(Some(recipient), |ctx| {
                        ctx.with_temp_iterated_player(Some(snapshot.controller), |ctx| {
                            crate::effects::helpers::resolve_value(game, &self.amount, ctx)
                        })
                    })?
                    .max(0) as u32;
                if amount > 0 {
                    events.push(SimultaneousDamageEvent {
                        source: ctx.source,
                        target: crate::events::DamageTarget::Object(recipient),
                        amount,
                        is_combat: false,
                        unpreventable: false,
                        cause: ctx.cause.clone(),
                        source_snapshot: source_snapshot.clone(),
                    });
                }
            }
            Ok::<_, ExecutionError>(events)
        })();
        if let Some(previous) = previous {
            ctx.tagged_objects.insert(tag, previous);
        } else {
            ctx.tagged_objects.remove(&tag);
        }
        let events = prepared?;
        if events.is_empty() {
            return Ok(EffectOutcome::count(0));
        }
        let source = ctx.source;
        let controller = ctx.controller;
        let cause = ctx.cause.clone();
        let provenance = ctx.provenance;
        let replacement = ctx.replacement.clone();
        let batch = game.simultaneous_action_batch().unwrap_or_else(|| {
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::Damage)
        });
        with_deferred_prevention_follow_up_outcome(game, ctx.decision_maker, |game, dm| {
            let mut parent = ExecutionContext::new(source, controller, dm)
                .with_cause(cause)
                .with_provenance(provenance);
            parent.source_snapshot = source_snapshot;
            parent.replacement = replacement;
            super::multi_source_damage::commit_damage_batch(game, &mut parent, events, Some(batch))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::execute_effect;
    use crate::target::{ObjectFilter, PlayerFilter};
    #[test]
    fn recipient_local_values_and_global_values_share_one_pre_damage_frame() {
        for tagged in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let a = crate::PlayerId::from_index(0);
            let b = crate::PlayerId::from_index(1);
            let source = game.create_object_from_definition(
                &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Source")
                    .card_types(vec![crate::types::CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(1, 50))
                    .with_ability(crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::lifelink(),
                    ))
                    .build(),
                a,
                crate::zone::Zone::Battlefield,
            );
            let mut recipients = Vec::new();
            for (player, power) in [(a, 2), (b, 5)] {
                let id = game.create_object_from_definition(
                    &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Recipient")
                        .card_types(vec![crate::types::CardType::Creature])
                        .power_toughness(crate::card::PowerToughness::fixed(power, 50))
                        .build(),
                    player,
                    crate::zone::Zone::Battlefield,
                );
                recipients.push(id);
            }
            execute_effect(
                &mut game,
                &Effect::lose_life_player(17, PlayerFilter::You),
                &mut ExecutionContext::new_default(source, a),
            )
            .unwrap();
            let prior = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
            let mut ctx = ExecutionContext::new_default(source, a);
            ctx.set_tagged_objects("__it__", vec![prior]);
            let reference = if tagged {
                ChooseSpec::Tagged("__it__".into())
            } else {
                ChooseSpec::Iterated
            };
            let amount = crate::Value::Add(
                Box::new(crate::Value::PowerOf(Box::new(reference))),
                Box::new(crate::Value::LifeTotal(PlayerFilter::You)),
            );
            let outcome = execute_effect(
                &mut game,
                &Effect::new(DealDamageEachEffect {
                    amount,
                    filter: ObjectFilter::creature().other(),
                }),
                &mut ctx,
            )
            .unwrap();
            assert_eq!(game.damage_on(recipients[0]), 5);
            assert_eq!(
                game.damage_on(recipients[1]),
                8,
                "later amount must not include earlier lifelink"
            );
            assert_eq!(game.player(a).unwrap().life, 16);
            assert_eq!(ctx.get_tagged_all("__it__").unwrap()[0].object_id, source);
            assert!(ctx.iteration.iterated_object.is_none());
            assert!(ctx.iteration.iterated_player.is_none());
            let damage = outcome
                .events
                .iter()
                .filter(|event| event.downcast::<crate::events::DamageEvent>().is_some())
                .collect::<Vec<_>>();
            assert_eq!(damage.len(), 2);
            assert_eq!(
                damage[0].simultaneous_batch(),
                damage[1].simultaneous_batch()
            );
            assert_ne!(damage[0].provenance(), damage[1].provenance());
        }
    }
}
