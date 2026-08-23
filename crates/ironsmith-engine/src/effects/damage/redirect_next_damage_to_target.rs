//! Redirect the next N damage from this source to a chosen target.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_player_from_spec, resolve_value,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::DamageTarget;
use crate::events::damage::matchers::DamageToSelfMatcher;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher};
use crate::game_state::GameState;
use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
use crate::target::ChooseSpec;

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectNextDamageDestination {
    Controller,
    TargetObject,
}

/// "The next N damage that would be dealt to this permanent this turn is dealt to target creature instead."
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextDamageToTargetEffect {
    pub amount: Value,
    pub protected_target: Option<ChooseSpec>,
    pub destination: RedirectNextDamageDestination,
    pub destination_target: Option<ChooseSpec>,
}

impl RedirectNextDamageToTargetEffect {
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            protected_target: None,
            destination: RedirectNextDamageDestination::TargetObject,
            destination_target: Some(target),
        }
    }

    pub fn to_controller(amount: impl Into<Value>, protected_target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            protected_target: Some(protected_target),
            destination: RedirectNextDamageDestination::Controller,
            destination_target: None,
        }
    }
}

#[derive(Debug, Clone)]
struct DamageToSpecificTargetMatcher {
    target: DamageTarget,
}

impl DamageToSpecificTargetMatcher {
    fn new(target: DamageTarget) -> Self {
        Self { target }
    }
}

impl ReplacementMatcher for DamageToSpecificTargetMatcher {
    fn matches_event(&self, event: &dyn GameEventType, _ctx: &crate::events::EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = crate::events::downcast_event::<crate::events::DamageEvent>(event)
        else {
            return false;
        };
        damage.target == self.target
    }

    fn display(&self) -> String {
        "When damage would be dealt to the chosen target".to_string()
    }
}

impl EffectExecutor for RedirectNextDamageToTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        if amount == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let redirect_target = match self.destination {
            RedirectNextDamageDestination::Controller => RedirectTarget::ToController,
            RedirectNextDamageDestination::TargetObject => {
                let target = self
                    .destination_target
                    .as_ref()
                    .ok_or(ExecutionError::InvalidTarget)?;
                let target = resolve_objects_for_effect(game, ctx, target)?
                    .into_iter()
                    .next()
                    .ok_or(ExecutionError::InvalidTarget)?;
                RedirectTarget::ToObject(target)
            }
        };

        let matcher: Box<dyn ReplacementMatcher> =
            if let Some(protected_target) = &self.protected_target {
                Box::new(DamageToSpecificTargetMatcher::new(
                    resolve_damage_target_for_effect(game, ctx, protected_target)?,
                ))
            } else {
                Box::new(DamageToSelfMatcher::new())
            };

        let replacement = ReplacementEffect::with_boxed_matcher(
            ctx.source,
            ctx.controller,
            matcher,
            ReplacementAction::RedirectDamageAmount {
                target: redirect_target,
                which: RedirectWhich::First,
                amount,
            },
        );
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.protected_target
            .as_ref()
            .or(self.destination_target.as_ref())
    }

    fn target_description(&self) -> &'static str {
        "damage redirection target"
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
    if matches!(
        spec.base(),
        ChooseSpec::Object(_) | ChooseSpec::SpecificObject(_)
    ) {
        return Err(ExecutionError::InvalidTarget);
    }

    resolve_player_from_spec(game, spec, ctx).map(DamageTarget::Player)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::ResolvedTarget;
    use crate::events::cause::EventCause;
    use crate::ids::{CardId, PlayerId};
    use crate::types::{CardType, Supertype};
    use crate::zone::Zone;

    fn create_creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        card_id: u32,
        supertypes: Vec<Supertype>,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Creature])
            .supertypes(supertypes)
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn vassals_duty_target_spec() -> ChooseSpec {
        ChooseSpec::target(ChooseSpec::Object(
            crate::target::ObjectFilter::creature()
                .with_supertype(Supertype::Legendary)
                .you_control(),
        ))
    }

    #[test]
    fn vassals_duty_redirects_only_next_one_damage_to_controller() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let vassals_duty = CardBuilder::new(CardId::from_raw(81_001), "Vassal's Duty")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_card(&vassals_duty, alice, Zone::Battlefield);
        let protected = create_creature(
            &mut game,
            "Protected Legend",
            alice,
            81_002,
            vec![Supertype::Legendary],
        );
        let damage_source = create_creature(&mut game, "Damage Source", bob, 81_003, Vec::new());

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        RedirectNextDamageToTargetEffect::to_controller(1, vassals_duty_target_spec())
            .execute(&mut game, &mut ctx)
            .expect("Vassal's Duty replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            damage_source,
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

        assert_eq!(protected_damage, 2);
        assert_eq!(controller_damage, 1);

        let second = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            damage_source,
            DamageTarget::Object(protected),
            2,
            false,
            EventCause::effect(),
        );
        let second_controller_damage: u32 = second
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();
        assert_eq!(second_controller_damage, 0, "Vassal's Duty is one-shot");
    }

    #[test]
    fn vassals_duty_does_not_redirect_damage_to_another_creature() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let vassals_duty = CardBuilder::new(CardId::from_raw(81_011), "Vassal's Duty")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_card(&vassals_duty, alice, Zone::Battlefield);
        let protected = create_creature(
            &mut game,
            "Protected Legend",
            alice,
            81_012,
            vec![Supertype::Legendary],
        );
        let other = create_creature(&mut game, "Other Creature", alice, 81_013, Vec::new());
        let damage_source = create_creature(&mut game, "Damage Source", bob, 81_014, Vec::new());

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        RedirectNextDamageToTargetEffect::to_controller(1, vassals_duty_target_spec())
            .execute(&mut game, &mut ctx)
            .expect("Vassal's Duty replacement should register");

        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            damage_source,
            DamageTarget::Object(other),
            3,
            false,
            EventCause::effect(),
        );
        let other_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Object(other))
            .map(|assignment| assignment.amount)
            .sum();
        let controller_damage: u32 = processed
            .assignments
            .iter()
            .filter(|assignment| assignment.target == DamageTarget::Player(alice))
            .map(|assignment| assignment.amount)
            .sum();

        assert_eq!(other_damage, 3);
        assert_eq!(controller_damage, 0);
    }

    #[test]
    fn vassals_duty_rejects_nonlegendary_protected_target() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let vassals_duty = CardBuilder::new(CardId::from_raw(81_021), "Vassal's Duty")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_card(&vassals_duty, alice, Zone::Battlefield);
        let nonlegendary = create_creature(
            &mut game,
            "Nonlegendary Creature",
            alice,
            81_022,
            Vec::new(),
        );

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(nonlegendary)]);
        let err = RedirectNextDamageToTargetEffect::to_controller(1, vassals_duty_target_spec())
            .execute(&mut game, &mut ctx)
            .expect_err("nonlegendary target should not satisfy Vassal's Duty");

        assert_eq!(err, ExecutionError::InvalidTarget);
    }
}
