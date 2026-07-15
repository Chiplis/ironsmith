//! Prevent-the-next-time damage replacement effect.
//!
//! Supports patterns like:
//! - "The next time a source of your choice would deal damage to you this turn, prevent that damage."
//! - "The next time a red source would deal damage to any target this turn, prevent that damage."

use crate::Value;
use crate::effect::{Effect, EffectOutcome, EventValueSpec};
use crate::effects::DealDamageEffect;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_players_from_spec};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::damage::matchers::{
    DamageSourceConstraint, DamageTargetConstraint, PreventableDamageConstraintMatcher,
};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter, ObjectRef, PlayerFilter};

/// How to constrain which source's damage is prevented.
#[derive(Debug, Clone, PartialEq)]
pub enum PreventNextTimeDamageSource {
    /// Choose a specific source as the effect resolves ("a source of your choice").
    Choice,
    /// Choose one source matching a filter as the effect resolves.
    ChoiceMatching(ObjectFilter),
    /// Use an already selected target as the source ("that creature").
    Target(ChooseSpec),
    /// Match any source that satisfies a filter ("a red source", "an artifact source", etc.).
    Filter(ObjectFilter),
}

/// How to constrain which target is protected.
#[derive(Debug, Clone, PartialEq)]
pub enum PreventNextTimeDamageTarget {
    /// Any damage target.
    AnyTarget,
    /// Any damage target, with no recipient phrase in the oracle text.
    Omitted,
    /// Only damage that would be dealt to you.
    You,
    /// Only damage that would be dealt to a chosen target.
    Target(ChooseSpec),
}

/// Register a one-shot replacement effect that prevents the next damage event matching constraints.
///
/// One-shot effects are cleaned up at end of turn by the cleanup step, and consumed after use.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventNextTimeDamageEffect {
    pub source: PreventNextTimeDamageSource,
    pub target: PreventNextTimeDamageTarget,
    pub reflect_damage_to_source_controller: bool,
    pub follow_up_effects: Vec<Effect>,
}

impl PreventNextTimeDamageEffect {
    pub fn new(source: PreventNextTimeDamageSource, target: PreventNextTimeDamageTarget) -> Self {
        Self {
            source,
            target,
            reflect_damage_to_source_controller: false,
            follow_up_effects: Vec::new(),
        }
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<Effect>) -> Self {
        self.follow_up_effects = effects;
        self
    }

    pub fn reflecting_to_source_controller(mut self) -> Self {
        self.reflect_damage_to_source_controller = true;
        self
    }
}

impl EffectExecutor for PreventNextTimeDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_constraint = match &self.target {
            PreventNextTimeDamageTarget::AnyTarget | PreventNextTimeDamageTarget::Omitted => {
                DamageTargetConstraint::Any
            }
            PreventNextTimeDamageTarget::You => DamageTargetConstraint::Player(ctx.controller),
            PreventNextTimeDamageTarget::Target(spec) => {
                if let Ok(objects) = resolve_objects_for_effect(game, ctx, spec)
                    && let Some(object_id) = objects.first()
                {
                    DamageTargetConstraint::Object(*object_id)
                } else if let Ok(players) = resolve_players_from_spec(game, spec, ctx)
                    && let Some(player_id) = players.first()
                {
                    DamageTargetConstraint::Player(*player_id)
                } else {
                    return Err(ExecutionError::InvalidTarget);
                }
            }
        };

        let mut chosen_source = None;
        let source_constraint = match &self.source {
            PreventNextTimeDamageSource::Target(spec) => {
                let Some(source) = resolve_objects_for_effect(game, ctx, spec)?
                    .into_iter()
                    .next()
                else {
                    return Ok(EffectOutcome::count(0));
                };
                chosen_source = Some(source);
                DamageSourceConstraint::Specific(source)
            }
            PreventNextTimeDamageSource::Filter(filter) => {
                DamageSourceConstraint::Filter(filter.clone())
            }
            choice @ (PreventNextTimeDamageSource::Choice
            | PreventNextTimeDamageSource::ChoiceMatching(_)) => {
                // Choose from a pragmatic union of likely "sources": stack objects + permanents.
                // This is not perfect MTG coverage, but it captures the primary gameplay cases.
                let mut candidates = Vec::new();
                candidates.extend(game.stack.iter().map(|e| e.object_id));
                candidates.extend(game.battlefield.iter().copied());
                candidates.sort_by_key(|id| id.0);
                candidates.dedup();
                if let PreventNextTimeDamageSource::ChoiceMatching(filter) = choice {
                    let filter_ctx = ctx.filter_context(game);
                    candidates.retain(|id| {
                        game.object(*id)
                            .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                    });
                }

                if candidates.is_empty() {
                    return Ok(EffectOutcome::resolved());
                }

                let selectable = candidates
                    .iter()
                    .copied()
                    .map(|id| {
                        let name = game
                            .object(id)
                            .map(|o| o.name.to_string())
                            .unwrap_or_else(|| format!("object {}", id.0));
                        crate::decisions::context::SelectableObject::new(id, name)
                    })
                    .collect::<Vec<_>>();
                let select_ctx = crate::decisions::context::SelectObjectsContext::new(
                    ctx.controller,
                    Some(ctx.source),
                    "Choose a source",
                    selectable,
                    1,
                    Some(1),
                );
                let chosen = ctx
                    .decision_maker
                    .decide_objects(game, &select_ctx)
                    .into_iter()
                    .find(|id| candidates.contains(id));
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let Some(chosen) = chosen else {
                    return Ok(EffectOutcome::count(0));
                };
                chosen_source = Some(chosen);
                DamageSourceConstraint::Specific(chosen)
            }
        };

        let matcher = PreventableDamageConstraintMatcher {
            source: source_constraint,
            target: target_constraint,
        };
        let replacement_action = if self.reflect_damage_to_source_controller
            && let Some(source) = chosen_source
        {
            let mut effects = vec![Effect::new(DealDamageEffect::new(
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::Specific(source))),
            ))];
            effects.extend(self.follow_up_effects.clone());
            ReplacementAction::Instead(effects)
        } else if !self.follow_up_effects.is_empty() {
            ReplacementAction::Instead(self.follow_up_effects.clone())
        } else {
            ReplacementAction::Prevent
        };
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            matcher,
            replacement_action,
        );

        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        match &self.target {
            PreventNextTimeDamageTarget::Target(spec) => Some(spec),
            _ => None,
        }
    }

    fn target_description(&self) -> &'static str {
        "damage prevention target"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::ResolvedTarget;
    use crate::events::DamageTarget;
    use crate::events::processing::process_damage_with_event;
    use crate::events::traits::ReplacementMatcher;
    use crate::ids::{ObjectId, PlayerId};
    use crate::{CardDefinitionBuilder, CardId, CardType, PowerToughness, Zone};

    #[test]
    fn prevent_next_time_damage_choice_any_target_prevents() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);

        // Make the source a permanent so it's in the candidate pool.
        game.battlefield.push(source);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let effect = PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::Choice,
            PreventNextTimeDamageTarget::AnyTarget,
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        let (final_damage, prevented) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        );
        assert_eq!(final_damage, 0);
        assert!(prevented);
    }

    #[test]
    fn prevent_next_time_damage_choice_to_you_only_affects_you() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);

        game.battlefield.push(source);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let effect = PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::Choice,
            PreventNextTimeDamageTarget::You,
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        // Damage to Alice is prevented.
        let (final_damage, prevented) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        );
        assert_eq!(final_damage, 0);
        assert!(prevented);

        // Damage to Bob is not prevented (replacement is consumed or doesn't match target).
        let (final_damage, prevented) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        );
        assert_eq!(final_damage, 3);
        assert!(!prevented);
    }

    #[test]
    fn prevent_next_time_damage_matcher_does_not_match_unpreventable_damage() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(10);

        let matcher = PreventableDamageConstraintMatcher::from_specific_source(
            source,
            DamageTargetConstraint::Player(alice),
        );
        let ctx = crate::events::EventContext::for_replacement_effect(alice, source, &game);

        let unpreventable = crate::events::DamageEvent::unpreventable_with_cause(
            source,
            DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
        assert!(
            !matcher.matches_event(&unpreventable, &ctx),
            "matcher should not match unpreventable damage"
        );
    }

    #[test]
    fn prevent_next_time_damage_target_object_only_prevents_matching_damage_once() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let chosen_source_def =
            CardDefinitionBuilder::new(CardId::from_raw(91_401), "Chosen Source")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
        let protected_def =
            CardDefinitionBuilder::new(CardId::from_raw(91_402), "Protected Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
        let other_target_def =
            CardDefinitionBuilder::new(CardId::from_raw(91_403), "Other Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
        let other_source_def = CardDefinitionBuilder::new(CardId::from_raw(91_404), "Other Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let chosen_source =
            game.create_object_from_definition(&chosen_source_def, bob, Zone::Battlefield);
        let protected =
            game.create_object_from_definition(&protected_def, alice, Zone::Battlefield);
        let other_target =
            game.create_object_from_definition(&other_target_def, alice, Zone::Battlefield);
        let other_source =
            game.create_object_from_definition(&other_source_def, bob, Zone::Battlefield);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(chosen_source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        let effect = PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::Choice,
            PreventNextTimeDamageTarget::Target(ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::default(),
            ))),
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        let (damage_to_other_target, prevented_other_target) = process_damage_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Object(other_target),
            3,
            false,
            crate::events::cause::EventCause::from_effect(chosen_source, alice),
        );
        assert_eq!(damage_to_other_target, 3);
        assert!(!prevented_other_target);

        let (damage_from_other_source, prevented_other_source) = process_damage_with_event(
            &mut game,
            other_source,
            DamageTarget::Object(protected),
            3,
            false,
            crate::events::cause::EventCause::from_effect(other_source, bob),
        );
        assert_eq!(damage_from_other_source, 3);
        assert!(!prevented_other_source);

        let (prevented_damage, prevented) = process_damage_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Object(protected),
            3,
            false,
            crate::events::cause::EventCause::from_effect(chosen_source, alice),
        );
        assert_eq!(prevented_damage, 0);
        assert!(prevented);

        let (second_damage, second_prevented) = process_damage_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Object(protected),
            3,
            false,
            crate::events::cause::EventCause::from_effect(chosen_source, alice),
        );
        assert_eq!(second_damage, 3);
        assert!(!second_prevented);
    }
}
