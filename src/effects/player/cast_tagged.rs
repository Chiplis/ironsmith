//! Cast a previously tagged card effect implementation.
//!
//! This effect is used for one-shot "You may cast it" patterns where a prior
//! effect tagged a specific card (often from exile). The cast is performed
//! immediately during resolution and returns an outcome that can be used by
//! subsequent "If you don't" clauses.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::tag::TagKey;
use crate::zone::Zone;

/// Effect that casts a tagged card immediately.
#[derive(Debug, Clone, PartialEq)]
pub struct CastTaggedEffect {
    pub tag: TagKey,
    pub allow_land: bool,
    pub as_copy: bool,
    pub without_paying_mana_cost: bool,
}

impl CastTaggedEffect {
    /// Create a new cast-tagged effect.
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self {
            tag: tag.into(),
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: false,
        }
    }

    /// Allow playing a tagged land (best-effort, ignores ownership restrictions).
    pub fn allow_land(mut self) -> Self {
        self.allow_land = true;
        self
    }

    /// Cast a copy of the tagged object instead of the tagged object itself.
    pub fn as_copy(mut self) -> Self {
        self.as_copy = true;
        self
    }

    /// Cast without paying mana cost.
    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }
}

impl EffectExecutor for CastTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::alternative_cast::CastingMethod;
        use crate::cost::OptionalCostsPaid;

        let Some(snapshot) = ctx.get_tagged(self.tag.as_str()) else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };

        let mut object_id = snapshot.object_id;
        if game.object(object_id).is_none() {
            if let Some(found) = game.find_object_by_stable_id(snapshot.stable_id) {
                object_id = found;
            } else {
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            }
        }

        let (is_land, mana_cost, from_zone, card_name, stable_id) = {
            let Some(obj) = game.object(object_id) else {
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            };
            (
                obj.is_land(),
                obj.mana_cost.clone(),
                obj.zone,
                obj.name.clone(),
                obj.stable_id,
            )
        };

        if self.as_copy {
            let caster = ctx.controller;
            let copy_id = game.new_object_id();

            let source_obj = match game.object(object_id) {
                Some(obj) => obj.clone(),
                None => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
            };
            let mut copy_obj = crate::object::Object::token_copy_of(&source_obj, copy_id, caster);
            copy_obj.controller = caster;

            if is_land {
                if !self.allow_land {
                    return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
                }
                copy_obj.zone = Zone::Battlefield;
                game.add_object(copy_obj);
                return Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                    copy_id,
                ])));
            }

            if !self.without_paying_mana_cost
                && let Some(cost) = mana_cost.as_ref()
            {
                if !game.can_pay_mana_cost(caster, None, cost, 0) {
                    return Ok(EffectOutcome::from_result(EffectResult::Impossible));
                }
                let _ = game.try_pay_mana_cost(caster, None, cost, 0);
            }

            copy_obj.zone = Zone::Stack;
            game.add_object(copy_obj);

            let mut stack_entry = StackEntry::new(copy_id, caster);
            stack_entry.source_stable_id = Some(stable_id);
            stack_entry.source_name = Some(card_name);
            game.push_to_stack(stack_entry);
            return Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                copy_id,
            ])));
        }

        if is_land {
            if !self.allow_land {
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            }

            let new_id = match game.move_object(object_id, Zone::Battlefield) {
                Some(id) => id,
                None => return Ok(EffectOutcome::from_result(EffectResult::Impossible)),
            };
            if let Some(new_obj) = game.object_mut(new_id) {
                new_obj.controller = ctx.controller;
            }
            return Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                new_id,
            ])));
        }

        let caster = ctx.controller;
        if !self.without_paying_mana_cost
            && let Some(cost) = mana_cost.as_ref()
        {
            if !game.can_pay_mana_cost(caster, None, cost, 0) {
                return Ok(EffectOutcome::from_result(EffectResult::Impossible));
            }
            let _ = game.try_pay_mana_cost(caster, None, cost, 0);
        }

        let Some(new_id) = game.move_object(object_id, Zone::Stack) else {
            return Ok(EffectOutcome::from_result(EffectResult::Impossible));
        };

        let casting_method = if from_zone == Zone::Hand {
            CastingMethod::Normal
        } else {
            CastingMethod::PlayFrom {
                source: ctx.source,
                zone: from_zone,
                use_alternative: None,
            }
        };

        let stack_entry = StackEntry {
            object_id: new_id,
            controller: caster,
            targets: vec![],
            x_value: None,
            ability_effects: None,
            is_ability: false,
            casting_method,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            saga_final_chapter_source: None,
            source_stable_id: Some(stable_id),
            source_snapshot: None,
            source_name: Some(card_name),
            triggering_event: None,
            intervening_if: None,
            keyword_payment_contributions: vec![],
            crew_contributors: vec![],
            chosen_modes: None,
        };

        game.push_to_stack(stack_entry);
        Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
            new_id,
        ])))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_tagged_effect_creation() {
        let effect = CastTaggedEffect::new("tag");
        assert_eq!(effect.tag.as_str(), "tag");
    }

    #[test]
    fn test_clone_box() {
        let effect = CastTaggedEffect::new("tag");
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CastTaggedEffect"));
    }
}
