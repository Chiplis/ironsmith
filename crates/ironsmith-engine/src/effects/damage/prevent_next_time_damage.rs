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
    /// Damage that would be dealt to you and/or permanents matching the
    /// filter ("you and/or creatures you control").
    YouAndPermanents(ObjectFilter),
    /// Damage that would be dealt to any player ("to one or more players").
    AnyPlayer,
    /// Only damage that would be dealt to a chosen target.
    Target(ChooseSpec),
}

/// "The next time ... would deal combat damage": the shield's constraints,
/// limited to combat damage events (CR 510.2).
#[derive(Debug, Clone)]
struct CombatDamageOnly(PreventableDamageConstraintMatcher);

impl crate::events::traits::ReplacementMatcher for CombatDamageOnly {
    fn may_match_event_kind(&self, kind: crate::events::EventKind) -> bool {
        self.0.may_match_event_kind(kind)
    }

    fn matches_prepared_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
        crate::events::downcast_event::<crate::events::DamageEvent>(event)
            .is_some_and(|damage| damage.is_combat)
            && self.0.matches_prepared_event(event, ctx)
    }

    fn display(&self) -> String {
        "When combat damage would be dealt (preventable)".to_string()
    }
}

/// The same source/recipient constraints for a replacement that prevents
/// nothing (Eye for an Eye): "damage can't be prevented" doesn't stop it.
#[derive(Debug, Clone)]
struct UnpreventableAlsoMatches(PreventableDamageConstraintMatcher);

impl crate::events::traits::ReplacementMatcher for UnpreventableAlsoMatches {
    fn may_match_event_kind(&self, kind: crate::events::EventKind) -> bool {
        self.0.may_match_event_kind(kind)
    }

    fn matches_prepared_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
        match crate::events::downcast_event::<crate::events::DamageEvent>(event) {
            Some(damage) if damage.is_unpreventable => {
                let mut preventable = damage.clone();
                preventable.is_unpreventable = false;
                self.0.matches_prepared_event(&preventable, ctx)
            }
            _ => self.0.matches_prepared_event(event, ctx),
        }
    }

    fn display(&self) -> String {
        "When damage would be dealt".to_string()
    }
}

/// Register a one-shot replacement effect that prevents the next damage event matching constraints.
///
/// One-shot effects are cleaned up at end of turn by the cleanup step, and consumed after use.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventNextTimeDamageEffect {
    pub source: PreventNextTimeDamageSource,
    pub target: PreventNextTimeDamageTarget,
    pub reflect_damage_to_source_controller: bool,
    /// Reflect only when the prevented damage's source matches this filter
    /// at the time the damage is prevented.
    pub reflect_source_filter: Option<ObjectFilter>,
    pub follow_up_effects: Vec<Effect>,
    /// The part of the next matching damage event the shield prevents.
    pub portion: ironsmith_core::NextTimeDamagePreventionPortion,
    /// Only a combat damage event uses the shield ("would deal combat damage").
    pub combat_only: bool,
}

impl PreventNextTimeDamageEffect {
    pub fn new(source: PreventNextTimeDamageSource, target: PreventNextTimeDamageTarget) -> Self {
        Self {
            source,
            target,
            reflect_damage_to_source_controller: false,
            reflect_source_filter: None,
            follow_up_effects: Vec::new(),
            portion: ironsmith_core::NextTimeDamagePreventionPortion::All,
            combat_only: false,
        }
    }

    pub fn with_portion(
        mut self,
        portion: ironsmith_core::NextTimeDamagePreventionPortion,
    ) -> Self {
        self.portion = portion;
        self
    }

    pub fn combat_damage_only(mut self) -> Self {
        self.combat_only = true;
        self
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<Effect>) -> Self {
        self.follow_up_effects = effects;
        self
    }

    pub fn reflecting_to_source_controller(mut self) -> Self {
        self.reflect_damage_to_source_controller = true;
        self
    }

    pub fn reflecting_only_from_source_matching(mut self, filter: ObjectFilter) -> Self {
        self.reflect_source_filter = Some(filter);
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
            PreventNextTimeDamageTarget::AnyPlayer => DamageTargetConstraint::AnyPlayer,
            PreventNextTimeDamageTarget::YouAndPermanents(filter) => {
                DamageTargetConstraint::PlayerOrPermanents {
                    player: ctx.controller,
                    filter: filter.clone(),
                }
            }
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
                use crate::effects::combat::prevention_helpers::{
                    SourceChoiceSelection, choose_source_of_your_choice,
                    choose_source_of_your_choice_matching_filter,
                };
                let selection = match choice {
                    PreventNextTimeDamageSource::ChoiceMatching(filter) => {
                        choose_source_of_your_choice_matching_filter(game, ctx, filter)
                    }
                    _ => choose_source_of_your_choice(game, ctx),
                };
                let chosen = match selection {
                    SourceChoiceSelection::Chosen(source) => source,
                    SourceChoiceSelection::NoAvailableSource => {
                        return Ok(EffectOutcome::resolved());
                    }
                    SourceChoiceSelection::NoChoiceMade => return Ok(EffectOutcome::count(0)),
                };
                chosen_source = Some(chosen);
                match choice {
                    // CR 615.9: the chosen source must still have the quality
                    // when it would deal the damage, or the shield doesn't
                    // apply (and isn't used up).
                    PreventNextTimeDamageSource::ChoiceMatching(filter) => {
                        DamageSourceConstraint::SpecificMatching {
                            source: chosen,
                            filter: filter.clone(),
                        }
                    }
                    _ => DamageSourceConstraint::Specific(chosen),
                }
            }
        };

        let matcher = PreventableDamageConstraintMatcher {
            source: source_constraint,
            target: target_constraint,
        };
        // "instead that source deals that much damage to you and this spell
        // deals that much damage to that source's controller" (Eye for an
        // Eye): a one-shot replacement that prevents nothing; the damage is
        // dealt and the reflection happens with the replaced amount.
        if self.portion == ironsmith_core::NextTimeDamagePreventionPortion::ReflectOnly {
            let Some(source) = chosen_source.filter(|_| self.reflect_damage_to_source_controller)
            else {
                return Err(ExecutionError::UnresolvableValue(
                    "a reflect-only next-time damage replacement needs a chosen source".into(),
                ));
            };
            let mut effects = vec![Effect::new(DealDamageEffect::new(
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::Specific(source))),
            ))];
            effects.extend(self.follow_up_effects.clone());
            let action = ReplacementAction::Additionally(effects);
            let replacement = if self.combat_only {
                ReplacementEffect::with_matcher(
                    ctx.source,
                    ctx.controller,
                    CombatDamageOnly(matcher),
                    action,
                )
            } else {
                ReplacementEffect::with_matcher(
                    ctx.source,
                    ctx.controller,
                    UnpreventableAlsoMatches(matcher),
                    action,
                )
            };
            game.effect_store
                .replacement_effects
                .add_next_damage_occurrence_effect(replacement);
            return Ok(EffectOutcome::resolved());
        }
        // A partial shield prevents only part of the next matching damage
        // event and is then used up (CR 615.1, 615.7); it has no "prevented
        // this way" rider in any printed form.
        if self.portion != ironsmith_core::NextTimeDamagePreventionPortion::All {
            if self.reflect_damage_to_source_controller || !self.follow_up_effects.is_empty() {
                return Err(ExecutionError::UnresolvableValue(
                    "a partial next-time prevention shield has no prevented-damage rider".into(),
                ));
            }
            let replacement_action = match &self.portion {
                ironsmith_core::NextTimeDamagePreventionPortion::HalfRoundedDown => {
                    ReplacementAction::PreventHalfDamage { round_up: false }
                }
                ironsmith_core::NextTimeDamagePreventionPortion::AllBut(remaining) => {
                    ReplacementAction::PreventDamageByRule(
                        ironsmith_core::StaticDamagePreventionAmount::AllBut(*remaining),
                    )
                }
                // The amount is fixed as the shield is created (CR 615.7):
                // "Prevent X of that damage" names X's value now.
                ironsmith_core::NextTimeDamagePreventionPortion::Exactly(amount) => {
                    let amount = crate::effects::helpers::resolve_value(game, amount, ctx)?.max(0);
                    ReplacementAction::PreventDamageByRule(
                        ironsmith_core::StaticDamagePreventionAmount::Amount(Value::Fixed(amount)),
                    )
                }
                ironsmith_core::NextTimeDamagePreventionPortion::All
                | ironsmith_core::NextTimeDamagePreventionPortion::ReflectOnly => {
                    ReplacementAction::Prevent
                }
            };
            let replacement = if self.combat_only {
                ReplacementEffect::with_matcher(
                    ctx.source,
                    ctx.controller,
                    CombatDamageOnly(matcher),
                    replacement_action,
                )
            } else {
                ReplacementEffect::with_matcher(
                    ctx.source,
                    ctx.controller,
                    matcher,
                    replacement_action,
                )
            };
            return crate::effects::ApplyReplacementEffect::one_shot(replacement)
                .execute_child(game, ctx);
        }
        // The shield prevents the damage (CR 615.1); what happens to the
        // prevented damage runs afterward as a prevention follow-up that reads
        // the prevented amount and the prevented damage event (CR 615.5).
        let replacement_action = if self.reflect_damage_to_source_controller
            && let Some(source) = chosen_source
        {
            let reflect = Effect::new(DealDamageEffect::new(
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::Specific(source))),
            ));
            // "If damage from a red source is prevented this way": the
            // prevented event's object is its source, checked as the damage
            // is prevented.
            let reflect = match &self.reflect_source_filter {
                Some(filter) => Effect::conditional_only(
                    crate::ConditionExpr::TaggedObjectMatches(
                        crate::TagKey::from("triggering"),
                        filter.clone(),
                    ),
                    vec![reflect],
                ),
                None => reflect,
            };
            let mut effects = vec![reflect];
            effects.extend(self.follow_up_effects.clone());
            ReplacementAction::PreventDamageThen(effects)
        } else if !self.follow_up_effects.is_empty() {
            ReplacementAction::PreventDamageThen(self.follow_up_effects.clone())
        } else {
            ReplacementAction::Prevent
        };
        let replacement = if self.combat_only {
            ReplacementEffect::with_matcher(
                ctx.source,
                ctx.controller,
                CombatDamageOnly(matcher),
                replacement_action,
            )
        } else {
            ReplacementEffect::with_matcher(ctx.source, ctx.controller, matcher, replacement_action)
        };

        // "The next time ... would deal damage" covers every part of that one
        // damage occurrence (e.g. trample damage to blockers and a player),
        // not just the first event committed from it.
        game.effect_store
            .replacement_effects
            .add_next_damage_occurrence_effect(replacement);
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
    use crate::events::processing::process_damage_summary_for_test;
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

        let (final_damage, prevented) = process_damage_summary_for_test(
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
    fn next_time_prevention_covers_every_part_of_one_damage_occurrence() {
        // Shadowbane against a trampler: combat damage to a blocker and to
        // you is one occurrence, so the shield prevents all of it.
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = |game: &mut GameState, owner, name: &str, id| {
            let definition = CardDefinitionBuilder::new(CardId::from_raw(id), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 20))
                .build();
            game.create_object_from_definition(&definition, owner, Zone::Battlefield)
        };
        let blocker = creature(&mut game, alice, "Blocker", 61_001);
        let trampler = creature(&mut game, bob, "Trampler", 61_002);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(61_003), alice, &mut dm);
        PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::ChoiceMatching(ObjectFilter::specific(trampler)),
            PreventNextTimeDamageTarget::YouAndPermanents(
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
            ),
        )
        .execute(&mut game, &mut ctx)
        .unwrap();

        let assignment = |target, amount| crate::events::processing::SimultaneousDamageEvent {
            source: trampler,
            target,
            amount,
            is_combat: true,
            unpreventable: false,
            cause: crate::events::cause::EventCause::combat_damage(trampler),
            source_snapshot: None,
        };
        let occurrence = vec![
            assignment(DamageTarget::Object(blocker), 20),
            assignment(DamageTarget::Player(alice), 5),
        ];
        crate::effects::damage::execute_damage_batch(&mut game, &mut ctx, occurrence.clone(), None)
            .unwrap();
        assert_eq!(game.damage_on(blocker), 0);
        assert_eq!(game.player(alice).unwrap().life, 20);

        // The shield is used up by that occurrence.
        crate::effects::damage::execute_damage_batch(&mut game, &mut ctx, occurrence, None)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 15);
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
        let (final_damage, prevented) = process_damage_summary_for_test(
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
        let (final_damage, prevented) = process_damage_summary_for_test(
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
            !matcher
                .matches_event(&unpreventable, &ctx)
                .expect("finite matcher fixture evaluates successfully"),
            "matcher should not match unpreventable damage"
        );
    }

    #[test]
    fn unpreventable_damage_neither_uses_up_nor_reflects_a_next_time_shield() {
        // CR 615.12 + 609.7b: the shield prevents nothing, so it isn't used up,
        // and Deflecting Palm's "if damage is prevented this way" rider does nothing.
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_def = CardDefinitionBuilder::new(CardId::from_raw(91_411), "Bob's Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let source = game.create_object_from_definition(&source_def, bob, Zone::Battlefield);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::Choice,
            PreventNextTimeDamageTarget::You,
        )
        .reflecting_to_source_controller()
        .execute(&mut game, &mut ctx)
        .unwrap();
        let hit = |unpreventable| crate::events::processing::SimultaneousDamageEvent {
            source,
            target: DamageTarget::Player(alice),
            amount: 3,
            is_combat: false,
            unpreventable,
            cause: crate::events::cause::EventCause::from_effect(source, bob),
            source_snapshot: None,
        };
        let results = crate::events::processing::process_simultaneous_damage_assignments_with_event(
            &mut game,
            &[hit(true)],
        )
        .unwrap();
        assert_eq!(results[0].assignments.iter().map(|a| a.amount).sum::<u32>(), 3);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.effect_store.replacement_effects.effects().len(), 1);
        // The retained shield still catches the next preventable damage.
        let results = crate::events::processing::process_simultaneous_damage_assignments_with_event(
            &mut game,
            &[hit(false)],
        )
        .unwrap();
        assert!(results[0].assignments.iter().all(|a| a.amount == 0));
        assert_eq!(game.player(bob).unwrap().life, 17);
        assert!(game.effect_store.replacement_effects.effects().is_empty());
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

        let (damage_to_other_target, prevented_other_target) = process_damage_summary_for_test(
            &mut game,
            chosen_source,
            DamageTarget::Object(other_target),
            3,
            false,
            crate::events::cause::EventCause::from_effect(chosen_source, alice),
        );
        assert_eq!(damage_to_other_target, 3);
        assert!(!prevented_other_target);

        let (damage_from_other_source, prevented_other_source) = process_damage_summary_for_test(
            &mut game,
            other_source,
            DamageTarget::Object(protected),
            3,
            false,
            crate::events::cause::EventCause::from_effect(other_source, bob),
        );
        assert_eq!(damage_from_other_source, 3);
        assert!(!prevented_other_source);

        let (prevented_damage, prevented) = process_damage_summary_for_test(
            &mut game,
            chosen_source,
            DamageTarget::Object(protected),
            3,
            false,
            crate::events::cause::EventCause::from_effect(chosen_source, alice),
        );
        assert_eq!(prevented_damage, 0);
        assert!(prevented);

        let (second_damage, second_prevented) = process_damage_summary_for_test(
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
    #[test]
    fn source_choice_empty_and_pending_do_not_register_and_replay_registers_once() {
        struct Pause(bool);
        impl crate::decision::DecisionMaker for Pause {
            fn decide_objects(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                self.0 = true;
                vec![]
            }
            fn awaiting_choice(&self) -> bool {
                self.0
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = game.new_object_id();
        let effect = PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::Choice,
            PreventNextTimeDamageTarget::AnyTarget,
        );
        effect
            .execute(
                &mut game,
                &mut ExecutionContext::new_default(source, PlayerId(0)),
            )
            .unwrap();
        assert!(game.effect_store.replacement_effects.effects().is_empty());
        game.battlefield.push(source);
        let mut dm = Pause(false);
        let mut ctx = ExecutionContext::new(source, PlayerId(0), &mut dm);
        let output = effect.execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(output.count_or_zero(), 0);
        assert!(game.effect_store.replacement_effects.effects().is_empty());
        effect
            .execute(
                &mut game,
                &mut ExecutionContext::new_default(source, PlayerId(0)),
            )
            .unwrap();
        assert_eq!(game.effect_store.replacement_effects.effects().len(), 1);
    }
}
