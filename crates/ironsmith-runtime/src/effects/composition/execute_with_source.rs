//! Execute an effect while temporarily treating another object as the source.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;

/// Executes an inner effect using a resolved object as `ctx.source`.
///
/// This is useful for patterns like "that creature deals damage" where the
/// effect sequence is still resolving from a spell or ability, but a different
/// object should be treated as the source of the inner effect.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecuteWithSourceEffect {
    pub source: ChooseSpec,
    pub effect: Box<Effect>,
}

impl ExecuteWithSourceEffect {
    pub fn new(source: ChooseSpec, effect: Effect) -> Self {
        Self {
            source,
            effect: Box::new(effect),
        }
    }
}

impl EffectExecutor for ExecuteWithSourceEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = match resolve_single_object_for_effect(game, ctx, &self.source) {
            Ok(source_id) => source_id,
            Err(_) => return Ok(EffectOutcome::target_invalid()),
        };
        let Some(source_obj) = game.object(source_id) else {
            return Ok(EffectOutcome::target_invalid());
        };
        let source_snapshot = Some(ObjectSnapshot::from_object(source_obj, game));

        let original_source = ctx.source;
        let original_source_snapshot = ctx.source_snapshot.clone();
        ctx.source = source_id;
        ctx.source_snapshot = source_snapshot;

        let outcome = execute_effect(game, &self.effect, ctx);

        ctx.source = original_source;
        ctx.source_snapshot = original_source_snapshot;
        outcome
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.effect.0.get_target_spec()
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
    use crate::events::DamageEvent;
    use crate::executor::ResolvedTarget;
    use crate::game_event::DamageTarget;
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
}
