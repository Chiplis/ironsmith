//! A pure transformation of one retained resolving modal owner.
use crate::{Effect, effect::Value};
use crate::effects::{ChooseModeEffect, ExecutionError, SequenceEffect, TaggedEffect, WithIdEffect};

pub(crate) fn first_then_other_mode(program: &[Effect], first: u32) -> Result<Vec<Effect>, ExecutionError> {
    let mut owners = 0;
    let transformed = program.iter().map(|effect| transform(effect, first as usize, &mut owners)).collect::<Result<Vec<_>, _>>()?;
    if owners != 1 { return Err(incomplete("operation modifier requires exactly one resolving modal owner")); }
    Ok(transformed)
}
fn incomplete(message: &str) -> ExecutionError { ExecutionError::IncompleteEvidence(message.into()) }
fn transform(effect: &Effect, first: usize, owners: &mut usize) -> Result<Effect, ExecutionError> {
    if let Some(wrapper) = effect.downcast_ref::<WithIdEffect>() {
        return Ok(Effect::new(WithIdEffect::new(wrapper.id, transform(&wrapper.effect, first, owners)?)));
    }
    if let Some(wrapper) = effect.downcast_ref::<TaggedEffect>() {
        return Ok(Effect::new(wrapper.with_effect(transform(&wrapper.effect, first, owners)?)));
    }
    if let Some(sequence) = effect.downcast_ref::<SequenceEffect>() {
        let mut sequence = sequence.clone();
        sequence.effects = sequence.effects.iter().map(|child| transform(child, first, owners)).collect::<Result<_, _>>()?;
        return Ok(Effect::new(sequence));
    }
    let Some(choice) = effect.downcast_ref::<ChooseModeEffect>() else { return Ok(effect.clone()); };
    *owners += 1;
    if choice.chooser.is_none() || choice.modes.len() < 2 || first >= choice.modes.len()
        || choice.min != Value::Fixed(1) || choice.max != Value::Fixed(1)
        || choice.choose_count != Value::Fixed(1) || choice.min_choose_count != Value::Fixed(1)
        || choice.faces_dilemma || choice.allow_repeat || choice.allow_repeated_modes || choice.random
        || choice.spree || choice.tiered || choice.descriptive_mode_costs
        || !choice.mode_additional_mana_costs.is_empty()
        || choice.mode_point_costs.iter().any(|cost| *cost != 1)
        || choice.common_suffix_effect_count != 0
        || choice.disallow_previously_chosen_modes || choice.disallow_previously_chosen_modes_this_turn
        || choice.distinct_player_targets_per_mode || choice.conditional_mode_range.is_some()
        || choice.endure || choice.cast_chooser.is_some() || choice.optional_choice
        || !choice.cost_choice_keywords.is_empty() || choice.announced_modes.is_some()
    { return Err(incomplete("operation mode modifier requires an ordinary resolving choose-one menu and valid original mode index")); }
    let mut remaining = choice.clone();
    let fixed = remaining.modes.remove(first);
    if !remaining.decision_labels.is_empty() {
        if remaining.decision_labels.len() != choice.modes.len() { return Err(incomplete("operation mode labels do not match original menu")); }
        remaining.decision_labels.remove(first);
    }
    if !remaining.mode_point_costs.is_empty() { remaining.mode_point_costs.remove(first); }
    let mut program = std::mem::take(&mut remaining.common_prefix_effects);
    program.extend(fixed.effects);
    program.push(Effect::new(remaining));
    Ok(Effect::new(SequenceEffect::new(program)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn menu() -> ChooseModeEffect {
        let modes = (1..=3).map(|n| crate::effect::EffectMode::new(format!("original {n}"), vec![Effect::gain_life(n)])).collect();
        let mut menu = ChooseModeEffect::new(modes, Value::Fixed(1), Value::Fixed(1), false);
        menu.chooser = Some(crate::target::PlayerFilter::You);
        menu.decision_labels = vec!["first".into(), "second".into(), "third".into()];
        menu
    }
    #[test]
    fn original_menu_is_retained_and_fixed_mode_removed_only_from_remaining_choice() {
        let original = menu();
        let effects = first_then_other_mode(&[Effect::new(original.clone())], 0).unwrap();
        let sequence = effects[0].downcast_ref::<SequenceEffect>().unwrap();
        assert_eq!(sequence.effects.len(), 2);
        let remaining = sequence.effects[1].downcast_ref::<ChooseModeEffect>().unwrap();
        assert_eq!(remaining.modes.iter().map(|mode| mode.source_text.as_str()).collect::<Vec<_>>(), vec!["original 2", "original 3"]);
        assert_eq!(remaining.decision_labels, vec!["second", "third"]);
        assert_eq!(original.modes.len(), 3);
    }
    #[test]
    fn missing_ambiguous_nonresolving_and_out_of_range_owners_fail_before_execution() {
        assert!(first_then_other_mode(&[], 0).is_err());
        assert!(first_then_other_mode(&[Effect::new(menu()), Effect::new(menu())], 0).is_err());
        assert!(first_then_other_mode(&[Effect::new(menu())], 3).is_err());
        let mut cast = menu(); cast.chooser = None;
        assert!(first_then_other_mode(&[Effect::new(cast)], 0).is_err());
        let mut dilemma = menu(); dilemma.faces_dilemma = true;
        assert!(first_then_other_mode(&[Effect::new(dilemma)], 0).is_err());
    }
}
