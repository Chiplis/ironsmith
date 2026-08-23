//! Heal marked damage from a permanent (CR 701.69a).

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::{
    TraitEventResult, process_trait_event_with_dm_and_applied_effects,
};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;

pub use ironsmith_core::HealDamageEffect;

fn execute_keyword_action_replacement_effects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: Vec<Effect>,
    effect_id: crate::replacement::ReplacementEffectId,
    action_snapshot: ObjectSnapshot,
) -> Result<EffectOutcome, ExecutionError> {
    let replacement_effect = game
        .effect_store
        .replacement_effects
        .get_effect(effect_id)
        .cloned();
    let (replacement_source, replacement_controller) = replacement_effect
        .as_ref()
        .map(|effect| (effect.source, effect.controller))
        .unwrap_or((ctx.source, ctx.controller));
    let replacement_key = replacement_effect
        .as_ref()
        .map(|effect| effect.application_key());

    let original_source = ctx.source;
    let original_controller = ctx.controller;
    let original_cause = ctx.cause.clone();
    let original_it = ctx.clear_object_tag("__it__");
    let original_plain_it = ctx.clear_object_tag("it");
    let was_suppressed = !ctx
        .replacement
        .suppressed_replacement_effects
        .insert(effect_id);
    let key_was_suppressed = if let Some(key) = replacement_key.as_ref() {
        !ctx.replacement
            .suppressed_replacement_effect_keys
            .insert(key.clone())
    } else {
        true
    };

    ctx.source = replacement_source;
    ctx.controller = replacement_controller;
    ctx.cause =
        crate::events::cause::EventCause::from_effect(replacement_source, replacement_controller);
    ctx.set_tagged_objects("__it__", vec![action_snapshot.clone()]);
    ctx.set_tagged_objects("it", vec![action_snapshot]);

    let result = (|| -> Result<EffectOutcome, ExecutionError> {
        let mut outcomes = Vec::new();
        for effect in effects {
            outcomes.push(crate::effects::execute_effect(game, &effect, ctx)?);
        }
        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    })();

    ctx.source = original_source;
    ctx.controller = original_controller;
    ctx.cause = original_cause;
    if !was_suppressed {
        ctx.replacement
            .suppressed_replacement_effects
            .remove(&effect_id);
    }
    if !key_was_suppressed && let Some(key) = replacement_key {
        ctx.replacement
            .suppressed_replacement_effect_keys
            .remove(&key);
    }
    match original_it {
        Some(snapshots) => ctx.set_tagged_objects("__it__", snapshots),
        None => {
            ctx.clear_object_tag("__it__");
        }
    }
    match original_plain_it {
        Some(snapshots) => ctx.set_tagged_objects("it", snapshots),
        None => {
            ctx.clear_object_tag("it");
        }
    }

    result
}

impl EffectExecutor for HealDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
        let Some(target) = game.object(target_id) else {
            return Ok(EffectOutcome::target_invalid());
        };
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(target, game);
        let controller = snapshot.controller;
        let marked = game.damage_on(target_id);
        let requested = match &self.amount {
            Some(amount) => resolve_value(game, amount, ctx)?.max(0) as u32,
            None => marked,
        };
        let healed = marked.min(requested);
        if healed == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let would_event = Event::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Heal, controller, target_id, healed)
                .with_snapshot(Some(snapshot.clone())),
            ctx.provenance,
        );
        let applied_effects = ctx.replacement.suppressed_replacement_effects.clone();
        let applied_effect_keys = ctx.replacement.suppressed_replacement_effect_keys.clone();
        if applied_effects.is_empty() && applied_effect_keys.is_empty() {
            game.update_replacement_effects();
        }
        match process_trait_event_with_dm_and_applied_effects(
            game,
            would_event,
            ctx.decision_maker,
            &applied_effects,
            &applied_effect_keys,
        ) {
            TraitEventResult::Replaced {
                effects, effect_id, ..
            } => {
                return execute_keyword_action_replacement_effects(
                    game, ctx, effects, effect_id, snapshot,
                );
            }
            TraitEventResult::Prevented => return Ok(EffectOutcome::prevented()),
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                return Ok(EffectOutcome::count(0));
            }
            TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {}
        }

        game.set_damage_marked(target_id, marked - healed);
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Heal, controller, target_id, healed)
                .with_snapshot(Some(snapshot)),
            ctx.provenance,
        );
        Ok(EffectOutcome::count(healed as i32)
            .with_affected_objects_from_game(game, vec![target_id])
            .with_event(event))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent with marked damage to heal"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_damaged_permanent(
        marked: u32,
    ) -> (GameState, crate::ids::ObjectId, PlayerId, PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::new(), "Heal Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        game.add_object(Object::from_card(id, &card, bob, Zone::Battlefield));
        game.mark_damage(id, marked);
        (game, id, alice, bob)
    }

    #[test]
    fn exact_heal_removes_only_the_requested_marked_damage() {
        let (mut game, target, alice, bob) = setup_damaged_permanent(5);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::exact(ChooseSpec::SpecificObject(target), 2)
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(game.damage_on(target), 3);
        assert_eq!(outcome.count_or_zero(), 2);
        let event = outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("heal should emit a typed keyword-action event");
        assert_eq!(event.action, KeywordActionKind::Heal);
        assert_eq!(event.player, bob, "the permanent's controller heals it");
        assert_eq!(event.source, target);
        assert_eq!(event.amount, 2);
    }

    #[test]
    fn exact_heal_saturates_at_the_damage_that_is_actually_marked() {
        let (mut game, target, alice, _) = setup_damaged_permanent(2);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::exact(ChooseSpec::SpecificObject(target), 7)
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(game.damage_on(target), 0);
        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(
            outcome.events[0]
                .downcast::<KeywordActionEvent>()
                .expect("heal event")
                .amount,
            2
        );
    }

    #[test]
    fn is_healed_surface_removes_all_marked_damage() {
        let (mut game, target, alice, _) = setup_damaged_permanent(4);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::all(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(game.damage_on(target), 0);
        assert_eq!(outcome.count_or_zero(), 4);
    }

    #[test]
    fn healing_when_no_damage_is_marked_is_a_no_op_without_an_action_event() {
        let (mut game, target, alice, _) = setup_damaged_permanent(0);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::all(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
    }
}
