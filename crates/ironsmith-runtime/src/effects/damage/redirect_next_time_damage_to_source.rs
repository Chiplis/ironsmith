//! Redirect-the-next-time damage replacement effect.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_from_spec};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::DamageTarget;
use crate::events::damage::matchers::{
    DamageFromSourceMatcher, DamageSourceConstraint, DamageToPlayerOrObjectMatcher,
};
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};

/// Matches damage events from a constrained source to a specific damage target.
#[derive(Debug, Clone)]
struct DamageSourceToSpecificTargetMatcher {
    source: DamageSourceConstraint,
    target: DamageTarget,
}

impl DamageSourceToSpecificTargetMatcher {
    fn new(source: DamageSourceConstraint, target: DamageTarget) -> Self {
        Self { source, target }
    }
}

impl ReplacementMatcher for DamageSourceToSpecificTargetMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &crate::events::EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = crate::events::downcast_event::<crate::events::DamageEvent>(event)
        else {
            return false;
        };
        if damage.target != self.target {
            return false;
        }
        match &self.source {
            DamageSourceConstraint::Specific(source) => damage.source == *source,
            DamageSourceConstraint::Filter(filter) => ctx
                .game
                .object(damage.source)
                .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game)),
        }
    }

    fn display(&self) -> String {
        "When the next chosen source would deal damage to that creature".to_string()
    }
}

/// How to constrain which source's damage is redirected.
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectNextTimeDamageSource {
    Choice,
    Filter(ObjectFilter),
    Target(ChooseSpec),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectNextTimeDamageDestination {
    SourceObject,
    Controller,
    SourceController,
    TargetObject,
}

/// "The next time a source of your choice would deal damage to target creature this turn,
/// that damage is dealt to this creature instead."
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextTimeDamageToSourceEffect {
    pub source: RedirectNextTimeDamageSource,
    pub target: Option<ChooseSpec>,
    pub destination: RedirectNextTimeDamageDestination,
    pub destination_target: Option<ChooseSpec>,
    pub all_this_turn: bool,
}

/// "All damage that would be dealt this turn to [a player/permanent set] is dealt to [target] instead."
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectAllDamageThisTurnToTargetEffect {
    pub player_filter: PlayerFilter,
    pub object_filter: ObjectFilter,
    pub target: ChooseSpec,
}

impl RedirectAllDamageThisTurnToTargetEffect {
    pub fn new(
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: ChooseSpec,
    ) -> Self {
        Self {
            player_filter,
            object_filter,
            target,
        }
    }
}

impl EffectExecutor for RedirectAllDamageThisTurnToTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let redirect_target = resolve_objects_for_effect(game, ctx, &self.target)?
            .into_iter()
            .next()
            .ok_or(ExecutionError::InvalidTarget)?;

        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            DamageToPlayerOrObjectMatcher::new(
                self.player_filter.clone(),
                self.object_filter.clone(),
            ),
            ReplacementAction::Redirect {
                target: RedirectTarget::ToObject(redirect_target),
                which: RedirectWhich::First,
            },
        );
        game.effect_store
            .replacement_effects
            .add_until_end_of_turn_effect(replacement);
        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "damage redirection target"
    }
}

impl RedirectNextTimeDamageToSourceEffect {
    pub fn new(source: RedirectNextTimeDamageSource, target: ChooseSpec) -> Self {
        Self {
            source,
            target: Some(target),
            destination: RedirectNextTimeDamageDestination::SourceObject,
            destination_target: None,
            all_this_turn: false,
        }
    }

    pub fn from_source_target(source: ChooseSpec) -> Self {
        Self {
            source: RedirectNextTimeDamageSource::Target(source),
            target: None,
            destination: RedirectNextTimeDamageDestination::SourceController,
            destination_target: None,
            all_this_turn: false,
        }
    }

    pub fn to_controller(mut self) -> Self {
        self.destination = RedirectNextTimeDamageDestination::Controller;
        self.destination_target = None;
        self
    }

    pub fn to_source_controller(mut self) -> Self {
        self.destination = RedirectNextTimeDamageDestination::SourceController;
        self.destination_target = None;
        self
    }

    pub fn to_target(mut self, target: ChooseSpec) -> Self {
        self.destination = RedirectNextTimeDamageDestination::TargetObject;
        self.destination_target = Some(target);
        self
    }

    pub fn all_this_turn(mut self) -> Self {
        self.all_this_turn = true;
        self
    }
}

impl EffectExecutor for RedirectNextTimeDamageToSourceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_constraint = match &self.source {
            RedirectNextTimeDamageSource::Filter(filter) => {
                DamageSourceConstraint::Filter(filter.clone())
            }
            RedirectNextTimeDamageSource::Target(spec) => {
                let source = resolve_objects_for_effect(game, ctx, spec)?
                    .into_iter()
                    .next()
                    .ok_or(ExecutionError::InvalidTarget)?;
                DamageSourceConstraint::Specific(source)
            }
            RedirectNextTimeDamageSource::Choice => {
                let mut candidates = Vec::new();
                candidates.extend(game.stack.iter().map(|entry| entry.object_id));
                candidates.extend(game.battlefield.iter().copied());
                candidates.sort_by_key(|id| id.0);
                candidates.dedup();

                if candidates.is_empty() {
                    return Ok(EffectOutcome::resolved());
                }

                let selectable = candidates
                    .iter()
                    .copied()
                    .map(|id| {
                        let name = game
                            .object(id)
                            .map(|object| object.name.to_string())
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
                DamageSourceConstraint::Specific(chosen)
            }
        };

        let redirect_target = match self.destination {
            RedirectNextTimeDamageDestination::SourceObject => RedirectTarget::ToObject(ctx.source),
            RedirectNextTimeDamageDestination::Controller => {
                RedirectTarget::ToPlayer(ctx.controller)
            }
            RedirectNextTimeDamageDestination::SourceController => {
                RedirectTarget::ToSourceController
            }
            RedirectNextTimeDamageDestination::TargetObject => {
                let target = self
                    .destination_target
                    .as_ref()
                    .ok_or(ExecutionError::InvalidTarget)?;
                let redirect_target = resolve_objects_for_effect(game, ctx, target)?
                    .into_iter()
                    .next()
                    .ok_or(ExecutionError::InvalidTarget)?;
                RedirectTarget::ToObject(redirect_target)
            }
        };

        let replacement = if let Some(target) = &self.target {
            let protected_target = resolve_damage_target_for_effect(game, ctx, target)?;
            ReplacementEffect::with_matcher(
                ctx.source,
                ctx.controller,
                DamageSourceToSpecificTargetMatcher::new(source_constraint, protected_target),
                ReplacementAction::Redirect {
                    target: redirect_target,
                    which: RedirectWhich::First,
                },
            )
        } else {
            let filter = match source_constraint {
                DamageSourceConstraint::Specific(source) => ObjectFilter::specific(source),
                DamageSourceConstraint::Filter(filter) => filter,
            };
            ReplacementEffect::with_matcher(
                ctx.source,
                ctx.controller,
                DamageFromSourceMatcher::new(filter),
                ReplacementAction::Redirect {
                    target: redirect_target,
                    which: RedirectWhich::First,
                },
            )
        };
        if self.all_this_turn {
            game.effect_store
                .replacement_effects
                .add_until_end_of_turn_effect(replacement);
        } else {
            game.effect_store
                .replacement_effects
                .add_one_shot_effect(replacement);
        }
        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.destination_target
            .as_ref()
            .or(self.target.as_ref())
            .or_else(|| match &self.source {
                RedirectNextTimeDamageSource::Target(spec) => Some(spec),
                _ => None,
            })
    }

    fn target_description(&self) -> &'static str {
        "damage source or protected target for redirection"
    }
}

fn resolve_damage_target_for_effect(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
) -> Result<DamageTarget, ExecutionError> {
    if let Ok(objects) = resolve_objects_for_effect(game, ctx, spec)
        && let Some(object) = objects.into_iter().next()
    {
        return Ok(DamageTarget::Object(object));
    }

    resolve_player_from_spec(game, spec, ctx).map(DamageTarget::Player)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::ExecutionContext;
    use crate::effects::ResolvedTarget;
    use crate::events::DamageTarget;
    use crate::events::cause::EventCause;
    use crate::ids::CardId;
    use crate::ids::PlayerId;
    use crate::types::CardType;
    use crate::zone::Zone;

    struct ChooseNamedSourceDecisionMaker {
        source_name: &'static str,
    }

    impl crate::decision::DecisionMaker for ChooseNamedSourceDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            if let Some(chosen) = ctx.candidates.iter().find_map(|candidate| {
                if !candidate.legal {
                    return None;
                }
                let matches = game
                    .object(candidate.id)
                    .is_some_and(|object| object.name == self.source_name);
                if matches { Some(candidate.id) } else { None }
            }) {
                vec![chosen]
            } else {
                crate::decision::AutoPassDecisionMaker.decide_objects(game, ctx)
            }
        }
    }

    fn create_creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        card_id: u32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_sorcery_on_stack(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        card_id: u32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Sorcery])
            .build();
        let object_id = game.create_object_from_card(&card, controller, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(object_id, controller));
        object_id
    }

    #[test]
    fn oracles_attendants_style_redirect_registers_until_end_of_turn_effect() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);

        let attendants = CardBuilder::new(CardId::from_raw(80_001), "Oracle's Attendants")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&attendants, alice, Zone::Battlefield);

        let bear = CardBuilder::new(CardId::from_raw(80_002), "Runeclaw Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let target = game.create_object_from_card(&bear, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::target_creature(),
        )
        .all_this_turn()
        .execute(&mut game, &mut ctx)
        .expect("Oracle's Attendants style replacement should register");

        assert_eq!(
            game.effect_store
                .replacement_effects
                .one_shot_effects_snapshot()
                .len(),
            0,
            "all-damage redirect should not register as one-shot"
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .until_end_of_turn_effects_snapshot()
                .len(),
            1,
            "all-damage redirect should register until end of turn"
        );
    }

    #[test]
    fn next_time_redirect_stays_one_shot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);

        let shaman = CardBuilder::new(CardId::from_raw(80_011), "Shaman en-Kor")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&shaman, alice, Zone::Battlefield);

        let target_creature = CardBuilder::new(CardId::from_raw(80_012), "Target Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let target = game.create_object_from_card(&target_creature, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::target_creature(),
        )
        .execute(&mut game, &mut ctx)
        .expect("next-time replacement should register");

        assert_eq!(
            game.effect_store
                .replacement_effects
                .one_shot_effects_snapshot()
                .len(),
            1,
            "next-time redirect should remain one-shot"
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .until_end_of_turn_effects_snapshot()
                .len(),
            0,
            "next-time redirect should not register until end of turn"
        );
    }

    #[test]
    fn reverberation_redirects_target_sorcery_spell_damage_to_that_spells_controller() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_sorcery = create_sorcery_on_stack(&mut game, "Target Sorcery", bob, 80_020);
        let reverberation = CardBuilder::new(CardId::from_raw(80_021), "Reverberation")
            .card_types(vec![CardType::Instant])
            .build();
        let reverberation_id = game.create_object_from_card(&reverberation, alice, Zone::Stack);

        let mut ctx = ExecutionContext::new_default(reverberation_id, alice)
            .with_targets(vec![ResolvedTarget::Object(target_sorcery)]);
        RedirectNextTimeDamageToSourceEffect::from_source_target(ChooseSpec::target(
            ChooseSpec::Object(crate::target::ObjectFilter::spell().with_type(CardType::Sorcery)),
        ))
        .all_this_turn()
        .execute(&mut game, &mut ctx)
        .expect("Reverberation replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            target_sorcery,
            DamageTarget::Player(alice),
            3,
            false,
            EventCause::effect(),
        );
        let alice_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        let bob_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(bob))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(alice_damage, 0);
        assert_eq!(bob_damage, 3);
        assert!(!processed.replacement_prevented);

        let second = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            target_sorcery,
            DamageTarget::Player(alice),
            2,
            false,
            EventCause::effect(),
        );
        let second_bob_damage: u32 = second
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(bob))
            .map(|assignment| assignment.amount)
            .sum();
        assert_eq!(second_bob_damage, 2, "effect should last for the turn");
    }

    #[test]
    fn reverberation_does_not_redirect_damage_from_untargeted_sorcery_spell() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_sorcery = create_sorcery_on_stack(&mut game, "Target Sorcery", bob, 80_025);
        let other_sorcery = create_sorcery_on_stack(&mut game, "Other Sorcery", bob, 80_026);
        let reverberation = CardBuilder::new(CardId::from_raw(80_027), "Reverberation")
            .card_types(vec![CardType::Instant])
            .build();
        let reverberation_id = game.create_object_from_card(&reverberation, alice, Zone::Stack);

        let mut ctx = ExecutionContext::new_default(reverberation_id, alice)
            .with_targets(vec![ResolvedTarget::Object(target_sorcery)]);
        RedirectNextTimeDamageToSourceEffect::from_source_target(ChooseSpec::target(
            ChooseSpec::Object(crate::target::ObjectFilter::spell().with_type(CardType::Sorcery)),
        ))
        .all_this_turn()
        .execute(&mut game, &mut ctx)
        .expect("Reverberation replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            other_sorcery,
            DamageTarget::Player(alice),
            4,
            false,
            EventCause::effect(),
        );
        let alice_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        let bob_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(bob))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(alice_damage, 4);
        assert_eq!(bob_damage, 0);
        assert!(!processed.replacement_prevented);
    }

    #[test]
    fn jade_monolith_redirects_chosen_source_damage_to_controller_once() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_030);
        let monolith_card = CardBuilder::new(CardId::from_raw(80_031), "Jade Monolith")
            .card_types(vec![CardType::Artifact])
            .build();
        let monolith = game.create_object_from_card(&monolith_card, alice, Zone::Battlefield);
        let protected = create_creature(&mut game, "Protected Creature", alice, 80_032);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(monolith, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::target_creature(),
        )
        .to_controller()
        .execute(&mut game, &mut ctx)
        .expect("Jade Monolith replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Object(protected),
            3,
            false,
            EventCause::effect(),
        );
        let protected_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(protected))
            .map(|assignment| assignment.amount)
            .sum();
        let controller_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        assert_eq!(protected_damage, 0);
        assert_eq!(controller_damage, 3);
        assert!(!processed.replacement_prevented);

        let second = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Object(protected),
            2,
            false,
            EventCause::effect(),
        );
        let second_protected_damage: u32 = second
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(protected))
            .map(|assignment| assignment.amount)
            .sum();
        let second_controller_damage: u32 = second
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        assert_eq!(second_protected_damage, 2);
        assert_eq!(second_controller_damage, 0);
    }

    #[test]
    fn jade_monolith_does_not_redirect_nonchosen_source_damage() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let _chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_040);
        let other_source = create_creature(&mut game, "Other Source", alice, 80_041);
        let monolith_card = CardBuilder::new(CardId::from_raw(80_042), "Jade Monolith")
            .card_types(vec![CardType::Artifact])
            .build();
        let monolith = game.create_object_from_card(&monolith_card, alice, Zone::Battlefield);
        let protected = create_creature(&mut game, "Protected Creature", alice, 80_043);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(monolith, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::target_creature(),
        )
        .to_controller()
        .execute(&mut game, &mut ctx)
        .expect("Jade Monolith replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            other_source,
            DamageTarget::Object(protected),
            4,
            false,
            EventCause::effect(),
        );
        let protected_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(protected))
            .map(|assignment| assignment.amount)
            .sum();
        let controller_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(protected_damage, 4);
        assert_eq!(controller_damage, 0);
        assert!(!processed.replacement_prevented);
    }

    #[test]
    fn generals_regalia_redirects_chosen_source_damage_from_you_to_target_creature_once() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_050);
        let regalia_card = CardBuilder::new(CardId::from_raw(80_051), "General's Regalia")
            .card_types(vec![CardType::Artifact])
            .build();
        let regalia = game.create_object_from_card(&regalia_card, alice, Zone::Battlefield);
        let recipient = create_creature(&mut game, "Shielded Ally", alice, 80_052);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(regalia, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(recipient)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::you(),
        )
        .to_target(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().you_control(),
        )))
        .execute(&mut game, &mut ctx)
        .expect("General's Regalia replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Player(alice),
            5,
            false,
            EventCause::effect(),
        );
        let alice_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        let recipient_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(recipient))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(alice_damage, 0);
        assert_eq!(recipient_damage, 5);
        assert!(!processed.replacement_prevented);

        let second = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Player(alice),
            2,
            false,
            EventCause::effect(),
        );
        let second_alice_damage: u32 = second
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        let second_recipient_damage: u32 = second
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(recipient))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(second_alice_damage, 2);
        assert_eq!(second_recipient_damage, 0);
    }

    #[test]
    fn generals_regalia_does_not_redirect_nonchosen_source_damage_to_you() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let _chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_060);
        let other_source = create_creature(&mut game, "Other Source", alice, 80_061);
        let regalia_card = CardBuilder::new(CardId::from_raw(80_062), "General's Regalia")
            .card_types(vec![CardType::Artifact])
            .build();
        let regalia = game.create_object_from_card(&regalia_card, alice, Zone::Battlefield);
        let recipient = create_creature(&mut game, "Shielded Ally", alice, 80_063);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(regalia, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(recipient)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::you(),
        )
        .to_target(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().you_control(),
        )))
        .execute(&mut game, &mut ctx)
        .expect("General's Regalia replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            other_source,
            DamageTarget::Player(alice),
            4,
            false,
            EventCause::effect(),
        );
        let alice_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        let recipient_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(recipient))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(alice_damage, 4);
        assert_eq!(recipient_damage, 0);
        assert!(!processed.replacement_prevented);
    }

    #[test]
    fn generals_regalia_only_redirects_damage_that_would_be_dealt_to_you() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_070);
        let regalia_card = CardBuilder::new(CardId::from_raw(80_071), "General's Regalia")
            .card_types(vec![CardType::Artifact])
            .build();
        let regalia = game.create_object_from_card(&regalia_card, alice, Zone::Battlefield);
        let recipient = create_creature(&mut game, "Shielded Ally", alice, 80_072);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(regalia, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(recipient)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::you(),
        )
        .to_target(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().you_control(),
        )))
        .execute(&mut game, &mut ctx)
        .expect("General's Regalia replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Player(bob),
            3,
            false,
            EventCause::effect(),
        );
        let bob_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(bob))
            .map(|assignment| assignment.amount)
            .sum();
        let recipient_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(recipient))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(bob_damage, 3);
        assert_eq!(recipient_damage, 0);
        assert!(!processed.replacement_prevented);
    }

    #[test]
    fn oracles_attendants_redirects_chosen_source_damage_for_rest_of_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_100);
        let attendants = create_creature(&mut game, "Oracle's Attendants", alice, 80_101);
        let protected = create_creature(&mut game, "Protected Creature", alice, 80_102);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(attendants, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::target_creature(),
        )
        .all_this_turn()
        .execute(&mut game, &mut ctx)
        .expect("oracle attendants replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            chosen_source,
            DamageTarget::Object(protected),
            3,
            false,
            EventCause::effect(),
        );
        let protected_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(protected))
            .map(|assignment| assignment.amount)
            .sum();
        let redirected_to_attendants: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(attendants))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(
            protected_damage, 0,
            "chosen-source damage should not stay on protected creature"
        );
        assert!(
            !processed.replacement_prevented,
            "redirect should not count as prevention"
        );
        assert_eq!(
            redirected_to_attendants, 3,
            "chosen-source damage should be redirected to this creature"
        );
    }

    #[test]
    fn oracles_attendants_does_not_redirect_other_sources() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let _chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_110);
        let attendants = create_creature(&mut game, "Oracle's Attendants", alice, 80_111);
        let protected = create_creature(&mut game, "Protected Creature", alice, 80_112);
        let other_source = create_creature(&mut game, "Other Source", alice, 80_113);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(attendants, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::target_creature(),
        )
        .all_this_turn()
        .execute(&mut game, &mut ctx)
        .expect("oracle attendants replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            other_source,
            DamageTarget::Object(protected),
            2,
            false,
            EventCause::effect(),
        );
        let protected_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(protected))
            .map(|assignment| assignment.amount)
            .sum();
        let redirected_to_attendants: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(attendants))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(
            protected_damage, 2,
            "non-chosen source should still damage the protected creature"
        );
        assert!(
            !processed.replacement_prevented,
            "unredirected damage should not be prevented"
        );
        assert_eq!(
            redirected_to_attendants, 0,
            "non-chosen source should not be redirected to this creature"
        );
    }

    #[test]
    fn oracles_attendants_redirect_expires_at_end_of_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let chosen_source = create_creature(&mut game, "Chosen Source", alice, 80_120);
        let attendants = create_creature(&mut game, "Oracle's Attendants", alice, 80_121);
        let protected = create_creature(&mut game, "Protected Creature", alice, 80_122);

        let mut decision_maker = ChooseNamedSourceDecisionMaker {
            source_name: "Chosen Source",
        };
        let mut ctx = ExecutionContext::new(attendants, alice, &mut decision_maker)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        RedirectNextTimeDamageToSourceEffect::new(
            RedirectNextTimeDamageSource::Choice,
            ChooseSpec::target_creature(),
        )
        .all_this_turn()
        .execute(&mut game, &mut ctx)
        .expect("oracle attendants replacement should register");

        game.effect_store
            .replacement_effects
            .clear_until_end_of_turn_effects();

        let (protected_damage, replacement_prevented) =
            crate::events::processing::process_damage_with_event(
                &mut game,
                chosen_source,
                DamageTarget::Object(protected),
                4,
                false,
                EventCause::effect(),
            );

        assert_eq!(
            protected_damage, 4,
            "chosen-source damage should stop redirecting after end of turn"
        );
        assert!(
            !replacement_prevented,
            "expired redirect should not prevent damage"
        );
    }
}
