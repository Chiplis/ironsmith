use crate::cards::builders::CardTextError;
use crate::effect::{Condition, Effect};
use crate::target::{ChooseSpec, PlayerFilter};

use super::LoweredEffects;

fn validate_unbound_iterated_player(
    debug_repr: String,
    context: &str,
) -> Result<(), CardTextError> {
    if super::str_contains(debug_repr.as_str(), "IteratedPlayer") {
        return Err(CardTextError::InvariantViolation(format!(
            "{context} references PlayerFilter::IteratedPlayer without a trigger or loop that binds \"that player\": {debug_repr}"
        )));
    }
    Ok(())
}

fn validate_choose_specs_for_iterated_player(
    choices: &[ChooseSpec],
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if iterated_player_bound {
        return Ok(());
    }
    for choice in choices {
        validate_unbound_iterated_player(format!("{choice:?}"), context)?;
    }
    Ok(())
}

fn validate_condition_for_iterated_player(
    condition: &Condition,
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if iterated_player_bound {
        return Ok(());
    }
    validate_unbound_iterated_player(format!("{condition:?}"), context)
}

fn validate_effects_for_iterated_player(
    effects: &[Effect],
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        validate_effect_for_iterated_player(effect, iterated_player_bound, context)?;
    }
    Ok(())
}

fn validate_effect_for_iterated_player(
    effect: &Effect,
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return validate_effects_for_iterated_player(
            &sequence.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        if !iterated_player_bound && let Some(decider) = &may.decider {
            validate_unbound_iterated_player(format!("{decider:?}"), context)?;
        }
        return validate_effects_for_iterated_player(&may.effects, iterated_player_bound, context);
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", unless_pays.player), context)?;
        }
        return validate_effects_for_iterated_player(
            &unless_pays.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", unless_action.player), context)?;
        }
        validate_effects_for_iterated_player(
            &unless_action.effects,
            iterated_player_bound,
            context,
        )?;
        return validate_effects_for_iterated_player(
            &unless_action.alternative,
            iterated_player_bound,
            context,
        );
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", for_players.filter), context)?;
        }
        return validate_effects_for_iterated_player(&for_players.effects, true, context);
    }
    if let Some(for_each_object) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        if !iterated_player_bound {
            validate_unbound_iterated_player(format!("{:?}", for_each_object.filter), context)?;
        }
        return validate_effects_for_iterated_player(&for_each_object.effects, true, context);
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return validate_effects_for_iterated_player(&for_each_tagged.effects, true, context);
    }
    if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return validate_effects_for_iterated_player(&for_each_controller.effects, true, context);
    }
    if let Some(for_each_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        return validate_effects_for_iterated_player(&for_each_player.effects, true, context);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        validate_condition_for_iterated_player(
            &conditional.condition,
            iterated_player_bound,
            context,
        )?;
        validate_effects_for_iterated_player(&conditional.if_true, iterated_player_bound, context)?;
        return validate_effects_for_iterated_player(
            &conditional.if_false,
            iterated_player_bound,
            context,
        );
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        validate_effects_for_iterated_player(&if_effect.then, iterated_player_bound, context)?;
        return validate_effects_for_iterated_player(
            &if_effect.else_,
            iterated_player_bound,
            context,
        );
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return validate_effect_for_iterated_player(&tagged.effect, iterated_player_bound, context);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return validate_effect_for_iterated_player(
            &with_id.effect,
            iterated_player_bound,
            context,
        );
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        for mode in &choose_mode.modes {
            validate_effects_for_iterated_player(&mode.effects, iterated_player_bound, context)?;
        }
        return Ok(());
    }
    if let Some(vote) = effect.downcast_ref::<crate::effects::VoteEffect>() {
        if let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice {
            for option in options {
                validate_effects_for_iterated_player(
                    &option.effects_per_vote,
                    iterated_player_bound,
                    context,
                )?;
            }
        }
        return Ok(());
    }
    if let Some(reflexive) = effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
        validate_choose_specs_for_iterated_player(&reflexive.choices, false, context)?;
        return validate_effects_for_iterated_player(&reflexive.effects, false, context);
    }
    if let Some(schedule_delayed) =
        effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
    {
        if !iterated_player_bound {
            validate_unbound_iterated_player(
                format!("{:?}", schedule_delayed.controller),
                context,
            )?;
            if let Some(filter) = &schedule_delayed.target_filter {
                validate_unbound_iterated_player(format!("{filter:?}"), context)?;
            }
        }
        return validate_effects_for_iterated_player(&schedule_delayed.effects, false, context);
    }
    if let Some(schedule_when_leaves) =
        effect.downcast_ref::<crate::effects::ScheduleEffectsWhenTaggedLeavesEffect>()
    {
        if !iterated_player_bound {
            validate_unbound_iterated_player(
                format!("{:?}", schedule_when_leaves.controller),
                context,
            )?;
        }
        return validate_effects_for_iterated_player(&schedule_when_leaves.effects, false, context);
    }
    if let Some(haunt) = effect.downcast_ref::<crate::effects::HauntExileEffect>() {
        validate_choose_specs_for_iterated_player(&haunt.haunt_choices, false, context)?;
        return validate_effects_for_iterated_player(&haunt.haunt_effects, false, context);
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && !iterated_player_bound
        && matches!(choose.chooser, PlayerFilter::Target(_))
    {
        return Ok(());
    }

    if !iterated_player_bound {
        validate_unbound_iterated_player(format!("{effect:?}"), context)?;
    }
    Ok(())
}

pub(crate) fn validate_iterated_player_bindings_in_lowered_effects(
    lowered: &LoweredEffects,
    initial_iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    let iterated_player_bound = initial_iterated_player_bound || lowered.exports.iterated_player;
    validate_effects_for_iterated_player(&lowered.effects, iterated_player_bound, context)?;
    validate_choose_specs_for_iterated_player(&lowered.choices, iterated_player_bound, context)
}
