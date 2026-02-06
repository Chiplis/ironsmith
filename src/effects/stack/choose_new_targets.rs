//! "Choose new targets" effect implementation.
//!
//! This effect supports text like "You may choose new targets for the copy."
//! by re-targeting stack objects produced by a prior effect.

use crate::decisions::context::{BooleanContext, TargetRequirementContext, TargetsContext};
use crate::effect::{ChoiceCount, EffectId, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry, Target};
use crate::target::ChooseSpec;
use crate::targeting::compute_legal_targets;
use crate::zone::Zone;

/// Effect that lets a player choose new targets for stack object(s).
///
/// The objects are read from a prior effect result (typically `CopySpellEffect`)
/// that stored `EffectResult::Objects`.
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNewTargetsEffect {
    /// Effect result ID that contains stack object IDs to retarget.
    pub from_effect: EffectId,
    /// Whether this is optional ("you may choose new targets").
    pub may: bool,
}

impl ChooseNewTargetsEffect {
    /// Create a new retargeting effect.
    pub fn new(from_effect: EffectId, may: bool) -> Self {
        Self { from_effect, may }
    }

    /// Optional retargeting ("you may choose new targets").
    pub fn may(from_effect: EffectId) -> Self {
        Self::new(from_effect, true)
    }

    /// Mandatory retargeting ("choose new targets").
    pub fn must(from_effect: EffectId) -> Self {
        Self::new(from_effect, false)
    }
}

fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => requires_target_selection(inner),
        ChooseSpec::AnyTarget | ChooseSpec::Player(_) | ChooseSpec::Object(_) => true,
        _ => false,
    }
}

fn effects_for_stack_entry(game: &GameState, entry: &StackEntry) -> Vec<crate::effect::Effect> {
    if let Some(ref effects) = entry.ability_effects {
        return effects.clone();
    }

    let Some(obj) = game.object(entry.object_id) else {
        return Vec::new();
    };

    if let Some(ref effects) = obj.spell_effect {
        return effects.clone();
    }

    Vec::new()
}

fn extract_requirements(
    game: &GameState,
    entry: &StackEntry,
) -> Option<Vec<TargetRequirementContext>> {
    let effects = effects_for_stack_entry(game, entry);
    let mut requirements = Vec::new();

    for effect in &effects {
        let Some(spec) = effect.0.get_target_spec() else {
            continue;
        };
        if !requires_target_selection(spec) {
            continue;
        }

        let count: ChoiceCount = effect.0.get_target_count().unwrap_or_default();
        let legal_targets =
            compute_legal_targets(game, spec, entry.controller, Some(entry.object_id));
        let has_enough = count.min == 0 || legal_targets.len() >= count.min;
        if !has_enough {
            return None;
        }

        requirements.push(TargetRequirementContext {
            description: effect.0.target_description().to_string(),
            legal_targets,
            min_targets: count.min,
            max_targets: count.max,
        });
    }

    Some(requirements)
}

fn normalize_target_choice(
    requirements: &[TargetRequirementContext],
    proposed: Vec<Target>,
) -> Option<Vec<Target>> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    for req in requirements {
        let mut selected = Vec::new();
        let max_for_req = req.max_targets.unwrap_or(req.min_targets.max(1));
        let end = (cursor + max_for_req).min(proposed.len());

        for target in &proposed[cursor..end] {
            if req.legal_targets.contains(target) {
                selected.push(*target);
            }
        }
        cursor = end;

        if selected.len() < req.min_targets {
            for legal in &req.legal_targets {
                if selected.len() >= req.min_targets {
                    break;
                }
                if !selected.contains(legal) {
                    selected.push(*legal);
                }
            }
        }

        if selected.len() < req.min_targets {
            return None;
        }

        out.extend(selected);
    }

    Some(out)
}

impl EffectExecutor for ChooseNewTargetsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = match ctx.get_result(self.from_effect) {
            Some(EffectResult::Objects(ids)) => ids.clone(),
            _ => return Ok(EffectOutcome::resolved()),
        };

        let mut changed = 0;

        for object_id in object_ids {
            let Some(stack_idx) = game.stack.iter().position(|e| e.object_id == object_id) else {
                continue;
            };

            if game
                .object(object_id)
                .is_none_or(|obj| obj.zone != Zone::Stack)
            {
                continue;
            }

            let entry = game.stack[stack_idx].clone();
            let Some(requirements) = extract_requirements(game, &entry) else {
                if self.may {
                    continue;
                }
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            };

            if requirements.is_empty() {
                continue;
            }

            if self.may {
                let source_name = game
                    .object(object_id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "copy".to_string());
                let choose = ctx.decision_maker.decide_boolean(
                    game,
                    &BooleanContext::new(
                        entry.controller,
                        Some(object_id),
                        format!("Choose new targets for {source_name}?"),
                    ),
                );
                if !choose {
                    continue;
                }
            }

            let targets_ctx = TargetsContext::new(
                entry.controller,
                object_id,
                "copy".to_string(),
                requirements.clone(),
            );
            let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
            let Some(new_targets) = normalize_target_choice(&requirements, proposed) else {
                if self.may {
                    continue;
                }
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            };

            if game.stack[stack_idx].targets != new_targets {
                game.stack[stack_idx].targets = new_targets;
                changed += 1;
            }
        }

        Ok(EffectOutcome::from_result(EffectResult::Count(changed)))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    struct AlwaysYesDm;

    impl DecisionMaker for AlwaysYesDm {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_targeted_spell_on_stack(
        game: &mut GameState,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), "Bolt")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();

        let id = game.create_object_from_card(&card, controller, Zone::Stack);
        if let Some(obj) = game.object_mut(id) {
            obj.spell_effect = Some(vec![Effect::deal_damage(3, ChooseSpec::AnyTarget)]);
        }

        let entry = StackEntry::new(id, controller);
        game.stack.push(entry);
        id
    }

    #[test]
    fn test_choose_new_targets_for_copy() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let spell_id = create_targeted_spell_on_stack(&mut game, alice);
        if let Some(entry) = game.stack.iter_mut().find(|e| e.object_id == spell_id) {
            entry.targets = vec![Target::Player(bob)];
        }

        let mut copy_ctx = ExecutionContext::new_default(source, alice);
        copy_ctx.targets = vec![ResolvedTarget::Object(spell_id)];
        let copy = crate::effects::CopySpellEffect::single(ChooseSpec::spell());
        let copied = copy.execute(&mut game, &mut copy_ctx).unwrap();
        copy_ctx.store_result(EffectId(7), copied.result);

        let mut dm = AlwaysYesDm;
        let mut ctx = copy_ctx.with_decision_maker(&mut dm);
        let retarget = ChooseNewTargetsEffect::may(EffectId(7));
        retarget.execute(&mut game, &mut ctx).unwrap();

        let copy_id = match ctx.get_result(EffectId(7)).unwrap() {
            EffectResult::Objects(ids) => ids[0],
            _ => panic!("expected copied object result"),
        };

        let copy_entry = game
            .stack
            .iter()
            .find(|e| e.object_id == copy_id)
            .expect("copy on stack");
        assert_eq!(copy_entry.targets, vec![Target::Player(alice)]);
    }

    #[test]
    fn test_choose_new_targets_declined_keeps_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let spell_id = create_targeted_spell_on_stack(&mut game, alice);
        if let Some(entry) = game.stack.iter_mut().find(|e| e.object_id == spell_id) {
            entry.targets = vec![Target::Player(bob)];
        }

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![ResolvedTarget::Object(spell_id)];
        let copy = crate::effects::CopySpellEffect::single(ChooseSpec::spell());
        let copied = copy.execute(&mut game, &mut ctx).unwrap();
        ctx.store_result(EffectId(8), copied.result);

        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);
        let retarget = ChooseNewTargetsEffect::may(EffectId(8));
        retarget.execute(&mut game, &mut ctx).unwrap();

        let copy_id = match ctx.get_result(EffectId(8)).unwrap() {
            EffectResult::Objects(ids) => ids[0],
            _ => panic!("expected copied object result"),
        };
        let copy_entry = game
            .stack
            .iter()
            .find(|e| e.object_id == copy_id)
            .expect("copy on stack");
        assert_eq!(copy_entry.targets, vec![Target::Player(bob)]);
    }
}
