//! Execute an effect while temporarily treating another object as the source.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_effect_source_with_lki;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
pub type ExecuteWithSourceEffect = ironsmith_core::ExecuteWithSourceEffect<Effect>;

/// Freeze the source and LKI used by the complete child action.
pub(super) fn resolve_source_binding(
    effect: &ExecuteWithSourceEffect, game: &mut GameState, ctx: &mut ExecutionContext,
) -> Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)> {
    let resolved = resolve_effect_source_with_lki(game, ctx, &effect.source);
    finish_source_binding(effect, game, ctx, resolved)
}

fn finish_source_binding(
    effect: &ExecuteWithSourceEffect, game: &GameState, ctx: &ExecutionContext,
    resolved: Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)>,
) -> Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)> {
    // The source can leave the battlefield before this effect runs: a
    // sacrifice and its own reflexive trigger are the printed case. The
    // ability still resolves from the source's last known information
    // (CR 608.2h), which the stack entry already carries, so rebinding is
    // simply a no-op there rather than a failure.
    let rebind_to_own_source_lki = matches!(effect.source.base(), ChooseSpec::Source)
        && ctx.source_snapshot.is_some()
        && game.object(ctx.source).is_none();
    let Some((source_id, tagged_snapshot)) = resolved
    else {
        if rebind_to_own_source_lki {
            return Some((ctx.source, ctx.source_snapshot.clone()));
        }
        return None;
    };
    let source_snapshot = match game.object(source_id) {
        // A tagged source keeps its tagged last known information even
        // after it left (CR 608.2h); that snapshot is authoritative.
        _ if tagged_snapshot.is_some() => tagged_snapshot,
        Some(source_obj) => match effect.source.base() {
            ChooseSpec::Source => ctx.source_snapshot.as_ref().and_then(|snapshot| {
                // Rebinding an effect to its own source must not replace the
                // stack entry's battlefield LKI with the counter-cleared card
                // object now in a graveyard (or another destination zone).
                (snapshot.stable_id == source_obj.stable_id
                    && (snapshot.object_id != source_obj.id || snapshot.zone != source_obj.zone))
                    .then(|| snapshot.clone())
            }),
            _ => None,
        }
        .or_else(|| Some(ObjectSnapshot::from_object(source_obj, game))),
        None if rebind_to_own_source_lki => return Some((ctx.source, ctx.source_snapshot.clone())),
        None => return None,
    };

    Some((source_id, source_snapshot))
}

pub(super) fn with_source_binding<T>(
    ctx: &mut ExecutionContext,
    binding: &(crate::ids::ObjectId, Option<ObjectSnapshot>),
    f: impl FnOnce(&mut ExecutionContext) -> T,
) -> T {
    let source = ctx.source;
    let snapshot = ctx.source_snapshot.clone();
    ctx.source = binding.0;
    ctx.source_snapshot = binding.1.clone();
    let result = f(ctx);
    ctx.source = source;
    ctx.source_snapshot = snapshot;
    result
}

#[derive(Debug)]
struct SourceProposal {
    binding: Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)>,
    inner: Option<Box<dyn crate::effects::SimultaneousEffectProposal>>,
}
impl crate::effects::SimultaneousEffectProposal for SourceProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (Some(binding), Some(inner)) = (self.binding, self.inner) else {
            return Ok(EffectOutcome::target_invalid());
        };
        with_source_binding(ctx, &binding, |ctx| inner.commit(game, ctx))
    }
}

impl EffectExecutor for ExecuteWithSourceEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        (self.source.is_target() || matches!(self.source.base(), ChooseSpec::Source | ChooseSpec::SpecificObject(_) | ChooseSpec::Tagged(_)))
            && self.effect.0.supports_simultaneous_player_action()
    }
    fn is_read_only_simultaneous_player_action(&self) -> bool {
        (self.source.is_target() || matches!(self.source.base(), ChooseSpec::Source | ChooseSpec::SpecificObject(_) | ChooseSpec::Tagged(_)))
            && self.effect.0.is_read_only_simultaneous_player_action()
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        if !self.supports_simultaneous_player_action() {
            return Err(ExecutionError::Impossible("chooser-bearing source scope requires the mutable action-program preparation owner".into()));
        }
        let resolved = crate::effects::helpers::resolve_effect_source_from_spec_with_lki(game, ctx, &self.source);
        let binding = finish_source_binding(self, game, ctx, resolved);
        let inner = match &binding {
            Some(binding) => Some(with_source_binding(ctx, binding, |ctx| {
                self.effect.0.prepare_simultaneous_player_action(game, ctx)
            })?),
            None => None,
        };
        Ok(Box::new(SourceProposal { binding, inner }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.effect);
    }

    fn transparent_child_effect(&self) -> Option<&Effect> {
        Some(&self.effect)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(binding) = resolve_source_binding(self, game, ctx) else {
            return Ok(EffectOutcome::target_invalid());
        };
        with_source_binding(ctx, &binding, |ctx| execute_effect(game, &self.effect, ctx))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.effect.0.get_target_spec()
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        self.effect.0.decision_related_object_specs()
    }

    fn target_description(&self) -> &'static str {
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.effect.0.get_target_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::events::DamageEvent;
    use crate::events::DamageTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn execute_with_source_uses_the_resolved_object_as_damage_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell_source = game.new_object_id();
        let creature = create_creature(&mut game, "Borrowed Source", alice);
        let mut ctx = ExecutionContext::new_default(spell_source, alice);

        let effect = ExecuteWithSourceEffect::new(
            ChooseSpec::SpecificObject(creature),
            Effect::deal_damage(2, ChooseSpec::AnyTarget),
        );
        let outcome = ctx
            .with_temp_targets(vec![ResolvedTarget::Player(bob)], |ctx| {
                effect.execute(&mut game, ctx)
            })
            .expect("wrapped effect should resolve");
        let events_debug = format!("{:?}", outcome.events);

        assert!(
            outcome.events.iter().any(|event| {
                event.downcast::<DamageEvent>().is_some_and(|damage| {
                    damage.source == creature
                        && damage.amount == 2
                        && matches!(damage.target, DamageTarget::Player(player) if player == bob)
                })
            }),
            "expected damage from wrapped source, got {events_debug}"
        );
    }

    #[test]
    fn execute_with_source_returns_target_invalid_when_source_is_missing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let spell_source = game.new_object_id();
        let missing = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(spell_source, alice);

        let outcome =
            ExecuteWithSourceEffect::new(ChooseSpec::SpecificObject(missing), Effect::gain_life(2))
                .execute(&mut game, &mut ctx)
                .expect("missing wrapped source should return an outcome");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn execute_with_source_preserves_source_lki_after_the_source_moves() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Countered Source", alice);
        game.add_counters(source, crate::object::CounterType::Charge, 3)
            .expect("source counters");
        let snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source should exist"), &game);
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should move");

        let mut ctx = ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
        ExecuteWithSourceEffect::new(
            ChooseSpec::Source,
            Effect::gain_life(crate::effect::Value::CountersOnSource(
                crate::object::CounterType::Charge,
            )),
        )
        .execute(&mut game, &mut ctx)
        .expect("wrapped source-LKI effect should resolve");

        assert_eq!(game.player(alice).expect("Alice").life, 23);
    }
}
