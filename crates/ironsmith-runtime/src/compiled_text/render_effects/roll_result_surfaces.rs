use super::*;

/// Render a die-result backreference and a subsequent random attachment from
/// the effect graph that proves both relationships.
pub(super) fn describe_roll_result_damage_then_random_source_attachment(
    effects: &[Effect],
) -> Option<String> {
    let (roll_effect, damage_effect, attachment_target) = match effects {
        [
            roll_effect,
            damage_effect,
            choose_player_effect,
            attach_effect,
        ] => {
            let choose =
                choose_player_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
            if choose.chooser != PlayerFilter::You
                || !choose.random
                || !choose.excluded_tags.is_empty()
                || choose.remember_as_chosen_player
            {
                return None;
            }

            let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
            if !matches!(attach.objects.base(), ChooseSpec::Source)
                || !matches!(
                    attach.target.base(),
                    ChooseSpec::Player(PlayerFilter::TaggedPlayer(tag)) if tag == &choose.tag
                )
            {
                return None;
            }
            (
                roll_effect,
                damage_effect,
                describe_player_filter(&choose.filter),
            )
        }
        [roll_effect, damage_effect, attach_effect] => {
            let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
            let count = attach.target.count();
            let ChooseSpec::Player(filter) = attach.target.base() else {
                return None;
            };
            if !matches!(attach.objects.base(), ChooseSpec::Source)
                || !count.is_single()
                || !count.random
            {
                return None;
            }
            (roll_effect, damage_effect, describe_player_filter(filter))
        }
        _ => return None,
    };

    let roll_with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    roll_with_id
        .effect
        .downcast_ref::<crate::effects::RollDieEffect>()?;

    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || damage.unpreventable
        || !matches!(
            damage.amount.unhinted(),
            Value::EffectValue(id) if *id == roll_with_id.id
        )
    {
        return None;
    }

    let roll = describe_effect(roll_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let damage_target = describe_choose_spec(&damage.target);
    Some(format!(
        "{roll}. Deal damage to {damage_target} equal to the result. Then attach this source to {attachment_target} chosen at random"
    ))
}

/// The authored roll, damage, and attachment sentences lower into separate
/// resolution segments. Flatten only an otherwise plain program so the same
/// relationship proof also governs cross-segment rendering.
pub(in crate::compiled_text) fn describe_roll_result_damage_then_random_source_attachment_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let effects = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter().cloned())
        .collect::<Vec<_>>();
    describe_roll_result_damage_then_random_source_attachment(&effects)
}
